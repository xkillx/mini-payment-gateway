use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use shared_auth::Actor;
use shared_http::error::AppError;
use sqlx::PgPool;
use validator::ValidationErrors;

use crate::models::{CreateRefundRequest, RefundResponse};
use crate::service::{create_refund, CreateRefundCommand, CreateRefundError, CreateRefundOutcome};

#[derive(Clone)]
pub struct RefundRouteState {
    pub pool: PgPool,
}

fn validation_error(field: &'static str, code: &'static str, message: &'static str) -> AppError {
    let mut errors = ValidationErrors::new();
    let mut ve = validator::ValidationError::new(code);
    ve.message = Some(std::borrow::Cow::Borrowed(message));
    errors.add(field, ve);
    AppError::Validation(errors)
}

fn extract_idempotency_key(headers: &HeaderMap) -> Result<String, AppError> {
    let key = headers
        .get("Idempotency-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if key.is_empty() {
        return Err(validation_error(
            "Idempotency-Key",
            "required",
            "Idempotency-Key header is required",
        ));
    }

    if key.len() > 128 {
        return Err(validation_error(
            "Idempotency-Key",
            "length",
            "Idempotency-Key must be at most 128 characters",
        ));
    }

    Ok(key)
}

async fn list_refunds() -> impl IntoResponse {
    AppError::NotImplemented("refunds.list")
}

async fn get_refund() -> impl IntoResponse {
    AppError::NotImplemented("refunds.get")
}

async fn create_refund_handler(
    State(state): State<RefundRouteState>,
    Extension(actor): Extension<Actor>,
    headers: HeaderMap,
    body: Result<Json<CreateRefundRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !actor.is_merchant() {
        return AppError::Forbidden("Only merchants can create refunds".into()).into_response();
    }

    let idempotency_key = match extract_idempotency_key(&headers) {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };

    let req = match body {
        Ok(b) => b.0,
        Err(_) => {
            return validation_error("body", "invalid_json", "Invalid JSON body").into_response();
        }
    };

    let merchant_id = actor.merchant_id().unwrap();

    let cmd = CreateRefundCommand {
        actor_id: actor.actor_id(),
        merchant_id,
        payment_id: req.payment_id,
        idempotency_key,
    };

    match create_refund(&state.pool, cmd).await {
        Ok(CreateRefundOutcome::Created(refund)) => (
            axum::http::StatusCode::CREATED,
            Json(RefundResponse::from(refund)),
        )
            .into_response(),
        Ok(CreateRefundOutcome::Replayed(refund)) => (
            axum::http::StatusCode::OK,
            Json(RefundResponse::from(refund)),
        )
            .into_response(),
        Err(CreateRefundError::NotFound) => {
            AppError::NotFound("Payment not found".into()).into_response()
        }
        Err(CreateRefundError::InvalidPaymentState(msg)) => AppError::Conflict(msg).into_response(),
        Err(CreateRefundError::DuplicateRefund(msg)) => AppError::Conflict(msg).into_response(),
        Err(CreateRefundError::Conflict(msg)) => AppError::Conflict(msg).into_response(),
        Err(CreateRefundError::Database(e)) => {
            tracing::error!(error = %e, "Database error creating refund");
            AppError::Internal("Failed to create refund".into()).into_response()
        }
        Err(CreateRefundError::Audit(e)) => {
            tracing::error!(error = %e, "Audit error creating refund");
            AppError::Internal("Failed to create refund".into()).into_response()
        }
    }
}

pub fn routes(pool: PgPool) -> Router {
    let state = RefundRouteState { pool };

    Router::new()
        .route(
            "/",
            axum::routing::get(list_refunds).post(create_refund_handler),
        )
        .route("/{id}", axum::routing::get(get_refund))
        .with_state(state)
}
