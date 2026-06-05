use axum::extract::{Extension, Json, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{http::StatusCode, Router};
use serde_json::json;
use shared_auth::Actor;
use shared_http::error::{AppError, ErrorEnvelope};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::RunReconciliationRequest;
use crate::service;

#[derive(Clone)]
struct RouteState {
    pool: PgPool,
    payment_currency: String,
}

pub fn routes(pool: PgPool, payment_currency: String) -> Router {
    let state = RouteState {
        pool,
        payment_currency,
    };

    Router::new()
        .route("/", get(list_reconciliations).post(run_reconciliation))
        .with_state(state)
}

async fn list_reconciliations() -> impl IntoResponse {
    AppError::NotImplemented("reconciliation.list")
}

async fn run_reconciliation(
    State(state): State<RouteState>,
    Extension(actor): Extension<Actor>,
    body: Result<Json<RunReconciliationRequest>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    if !actor.is_administrator() {
        let request_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorEnvelope::new(
                "FORBIDDEN",
                "Only administrators can run reconciliation",
                &request_id,
            )),
        )
            .into_response();
    }

    let body = match body {
        Ok(Json(req)) => req,
        Err(rejection) => {
            let request_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
            let (status, msg) = match &rejection {
                axum::extract::rejection::JsonRejection::JsonDataError(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body")
                }
                axum::extract::rejection::JsonRejection::JsonSyntaxError(_) => {
                    (StatusCode::UNPROCESSABLE_ENTITY, "Invalid JSON syntax")
                }
                axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Missing Content-Type: application/json",
                ),
                _ => (StatusCode::UNPROCESSABLE_ENTITY, "Invalid request body"),
            };
            let details = json!({
                "body": [msg],
            });
            return (
                status,
                Json(
                    ErrorEnvelope::new(
                        "VALIDATION_ERROR",
                        "Request validation failed",
                        &request_id,
                    )
                    .with_details(details),
                ),
            )
                .into_response();
        }
    };

    if body.currency != state.payment_currency {
        let request_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
        let details = json!({
            "currency": [format!(
                "Currency must be '{}'",
                state.payment_currency
            )],
        });
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                ErrorEnvelope::new("VALIDATION_ERROR", "Request validation failed", &request_id)
                    .with_details(details),
            ),
        )
            .into_response();
    }

    let window_start = body.window_start;
    let window_end = body.window_end;

    if window_start >= window_end {
        let request_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
        let details = json!({
            "window_end": ["window_end must be after window_start"],
        });
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                ErrorEnvelope::new("VALIDATION_ERROR", "Request validation failed", &request_id)
                    .with_details(details),
            ),
        )
            .into_response();
    }

    let command = service::RunManualReconciliationCommand {
        actor_id: actor.actor_id(),
        currency: body.currency,
        window_start: body.window_start,
        window_end: body.window_end,
        actual_total_minor: body.actual_total_minor,
        notes: body.notes,
    };

    match service::run_manual_reconciliation(&state.pool, command).await {
        Ok(reconciliation) => (StatusCode::CREATED, Json(reconciliation)).into_response(),
        Err(e) => {
            let request_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
            tracing::error!(error = %e, "Failed to run manual reconciliation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to run reconciliation",
                    &request_id,
                )),
            )
                .into_response()
        }
    }
}
