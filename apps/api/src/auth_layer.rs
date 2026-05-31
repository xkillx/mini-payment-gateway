use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use shared_auth::{parse_authorization_header, parse_claims, Actor, AuthError, Role};
use shared_http::error::ErrorEnvelope;
use shared_http::middleware::get_request_id;
use uuid::Uuid;

use crate::state::AppState;

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let state = req.extensions().get::<AppState>().cloned();
    let Some(state) = state else {
        return internal_error("AppState not available");
    };

    let request_id = get_request_id(req.headers());

    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let Some(auth_header) = auth_header else {
        return auth_failure(
            &state.pool,
            &request_id,
            "missing_token",
            None,
            None,
            req.method().to_string(),
            req.uri().path().to_string(),
        )
        .await;
    };

    let token_str = match parse_authorization_header(auth_header) {
        Ok(t) => t,
        Err(AuthError::MalformedToken) => {
            return auth_failure(
                &state.pool,
                &request_id,
                "malformed_token",
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
        Err(e) => {
            return auth_failure(
                &state.pool,
                &request_id,
                &e.to_string(),
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
    };

    let parsed = match parse_claims(token_str, &state.config.jwt_secret) {
        Ok(c) => c,
        Err(AuthError::ExpiredToken) => {
            return auth_failure(
                &state.pool,
                &request_id,
                "expired_token",
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
        Err(AuthError::InvalidToken(msg)) => {
            return auth_failure(
                &state.pool,
                &request_id,
                &msg,
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
        Err(AuthError::InvalidClaims(msg)) => {
            return auth_failure(
                &state.pool,
                &request_id,
                &msg,
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
        Err(e) => {
            return auth_failure(
                &state.pool,
                &request_id,
                &e.to_string(),
                None,
                None,
                req.method().to_string(),
                req.uri().path().to_string(),
            )
            .await;
        }
    };

    let actor_id = parsed.actor_id;
    let claimed_merchant_id = parsed.merchant_id;

    let actor_record = match sqlx::query_as::<_, admin::models::ActorRecord>(
        "SELECT * FROM actors WHERE id = $1",
    )
    .bind(actor_id)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(path = %req.uri().path(), error = %e, "Database error during actor lookup");
            return internal_error("Database error during authentication");
        }
    };

    let Some(record) = actor_record else {
        return auth_failure(
            &state.pool,
            &request_id,
            "actor_not_found",
            Some(actor_id),
            claimed_merchant_id,
            req.method().to_string(),
            req.uri().path().to_string(),
        )
        .await;
    };

    if !record.is_active {
        return auth_failure(
            &state.pool,
            &request_id,
            "actor_inactive",
            Some(actor_id),
            claimed_merchant_id,
            req.method().to_string(),
            req.uri().path().to_string(),
        )
        .await;
    }

    if record.role
        != match parsed.role {
            Role::Merchant => "merchant",
            Role::Administrator => "administrator",
        }
    {
        return auth_failure(
            &state.pool,
            &request_id,
            "role_mismatch",
            Some(actor_id),
            claimed_merchant_id,
            req.method().to_string(),
            req.uri().path().to_string(),
        )
        .await;
    }

    if parsed.role == Role::Merchant && claimed_merchant_id != record.merchant_id {
        return auth_failure(
            &state.pool,
            &request_id,
            "merchant_id_mismatch",
            Some(actor_id),
            claimed_merchant_id,
            req.method().to_string(),
            req.uri().path().to_string(),
        )
        .await;
    }

    let actor = match parsed.role {
        Role::Merchant => Actor::Merchant {
            actor_id,
            merchant_id: claimed_merchant_id.unwrap(),
        },
        Role::Administrator => Actor::Administrator { actor_id },
    };

    req.extensions_mut().insert(actor);
    next.run(req).await
}

pub async fn require_role(req: Request, next: Next, allowed_roles: Vec<Role>) -> Response {
    let state = req.extensions().get::<AppState>().cloned();
    let request_id = get_request_id(req.headers());

    let actor = req.extensions().get::<Actor>().cloned();

    let Some(actor) = actor else {
        return unauthorized_response(&request_id, "No authenticated actor");
    };

    let actor_role = actor.role();
    if allowed_roles.contains(&actor_role) {
        return next.run(req).await;
    }

    let role_names: Vec<&str> = allowed_roles
        .iter()
        .map(|r| match r {
            Role::Merchant => "merchant",
            Role::Administrator => "administrator",
        })
        .collect();

    let authenticated_role = match actor_role {
        Role::Merchant => "merchant",
        Role::Administrator => "administrator",
    };

    let actor_type = match actor_role {
        Role::Merchant => audit::models::ActorType::MERCHANT,
        Role::Administrator => audit::models::ActorType::ADMINISTRATOR,
    };

    let details = serde_json::json!({
        "request_method": req.method().to_string(),
        "request_path": req.uri().path().to_string(),
        "required_roles": role_names,
        "authenticated_role": authenticated_role,
    });

    if let Some(state) = state {
        let record = audit::service::new_audit_record(
            Some(actor.actor_id()),
            actor_type,
            audit::models::actions::AUTH_AUTHORIZATION_FAILED,
            "auth",
            &request_id,
            Some(details),
        );
        audit::service::record_best_effort(&state.pool, record).await;
    }

    let envelope = ErrorEnvelope::new("FORBIDDEN", "Insufficient permissions", &request_id);
    (StatusCode::FORBIDDEN, Json(envelope)).into_response()
}

pub async fn merchant_write_guard(req: Request, next: Next) -> Response {
    if req.method() == axum::http::Method::POST {
        return require_role(req, next, vec![Role::Merchant]).await;
    }
    next.run(req).await
}

async fn auth_failure(
    pool: &sqlx::PgPool,
    request_id: &str,
    failure_kind: &str,
    claimed_actor_id: Option<Uuid>,
    claimed_merchant_id: Option<Uuid>,
    request_method: String,
    request_path: String,
) -> Response {
    let mut details = serde_json::json!({
        "request_method": request_method,
        "request_path": request_path,
        "failure_kind": failure_kind,
    });
    if let Some(id) = claimed_actor_id {
        details["claimed_actor_id"] = serde_json::json!(id.to_string());
    }
    if let Some(id) = claimed_merchant_id {
        details["claimed_merchant_id"] = serde_json::json!(id.to_string());
    }
    details["request_id"] = serde_json::json!(request_id);

    let record = audit::service::new_audit_record(
        None,
        audit::models::ActorType::UNKNOWN,
        audit::models::actions::AUTH_AUTHENTICATION_FAILED,
        "auth",
        request_id,
        Some(details),
    );
    audit::service::record_best_effort(pool, record).await;

    let envelope = ErrorEnvelope::new("UNAUTHORIZED", "Authentication failed", request_id);
    (StatusCode::UNAUTHORIZED, Json(envelope)).into_response()
}

fn internal_error(msg: &str) -> Response {
    let request_id = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
    let envelope = ErrorEnvelope::new("INTERNAL_ERROR", msg, &request_id);
    (StatusCode::INTERNAL_SERVER_ERROR, Json(envelope)).into_response()
}

fn unauthorized_response(request_id: &str, msg: &str) -> Response {
    let envelope = ErrorEnvelope::new("UNAUTHORIZED", msg, request_id);
    (StatusCode::UNAUTHORIZED, Json(envelope)).into_response()
}
