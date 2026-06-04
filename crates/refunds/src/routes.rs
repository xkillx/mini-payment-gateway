use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use shared_auth::Actor;
use shared_http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
use validator::ValidationErrors;

use crate::models::{parse_refund_status, CreateRefundRequest, RefundListFilter, RefundResponse};
use crate::service::{
    create_refund, get_refund_detail, list_refunds as list_refunds_service, CreateRefundCommand,
    CreateRefundError, CreateRefundOutcome, GetRefundDetailError, ListRefundsError,
};

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

fn validation_error_string(field: &'static str, code: &'static str, message: String) -> AppError {
    let mut errors = ValidationErrors::new();
    let mut ve = validator::ValidationError::new(code);
    ve.message = Some(std::borrow::Cow::Owned(message));
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

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListRefundsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    merchant_id: Option<String>,
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    offset: Option<String>,
}

async fn list_refunds(
    State(state): State<RefundRouteState>,
    Extension(actor): Extension<Actor>,
    Query(query): Query<ListRefundsQuery>,
) -> Response {
    let limit = match query.limit.as_deref() {
        None => DEFAULT_LIMIT,
        Some(raw) => match raw.parse::<i64>() {
            Ok(value) if (1..=MAX_LIMIT).contains(&value) => value,
            Ok(value) if value < 1 => {
                return validation_error(
                    "limit",
                    "invalid",
                    "limit must be greater than or equal to 1",
                )
                .into_response();
            }
            Ok(value) if value > MAX_LIMIT => {
                return validation_error(
                    "limit",
                    "invalid",
                    "limit must be less than or equal to 200",
                )
                .into_response();
            }
            _ => {
                return validation_error_string(
                    "limit",
                    "invalid",
                    format!("limit must be an integer between 1 and {MAX_LIMIT}"),
                )
                .into_response();
            }
        },
    };

    let offset = match query.offset.as_deref() {
        None => 0,
        Some(raw) => match raw.parse::<i64>() {
            Ok(value) if value >= 0 => value,
            Ok(_) => {
                return validation_error(
                    "offset",
                    "invalid",
                    "offset must be greater than or equal to 0",
                )
                .into_response();
            }
            _ => {
                return validation_error_string(
                    "offset",
                    "invalid",
                    "offset must be a non-negative integer".to_string(),
                )
                .into_response();
            }
        },
    };

    let status = match query.status.as_deref() {
        None => None,
        Some(raw) => match parse_refund_status(raw) {
            Some(s) => Some(s),
            None => {
                return validation_error(
                    "status",
                    "invalid",
                    "status must be one of pending, processing, completed, failed",
                )
                .into_response();
            }
        },
    };

    if actor.is_merchant() && query.merchant_id.is_some() {
        return AppError::Forbidden("Merchants cannot filter refunds by merchant_id".into())
            .into_response();
    }

    let merchant_id: Option<Uuid> = if actor.is_merchant() {
        actor.merchant_id()
    } else {
        match query.merchant_id.as_deref() {
            None => None,
            Some(raw) => match Uuid::parse_str(raw) {
                Ok(value) => Some(value),
                Err(_) => {
                    return validation_error(
                        "merchant_id",
                        "invalid",
                        "merchant_id must be a valid UUID",
                    )
                    .into_response();
                }
            },
        }
    };

    let filter = RefundListFilter {
        merchant_id,
        status,
        limit,
        offset,
    };

    match list_refunds_service(&state.pool, filter).await {
        Ok(response) => Json(response).into_response(),
        Err(ListRefundsError::Database(e)) => {
            tracing::error!(error = %e, "Database error listing refunds");
            AppError::Internal("Failed to list refunds".into()).into_response()
        }
    }
}

async fn get_refund(
    State(state): State<RefundRouteState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<Uuid>,
) -> Response {
    let viewer_merchant_id = actor.merchant_id();

    match get_refund_detail(&state.pool, id, viewer_merchant_id).await {
        Ok(detail) => Json(detail).into_response(),
        Err(GetRefundDetailError::NotFound) => {
            AppError::NotFound("Refund not found".into()).into_response()
        }
        Err(GetRefundDetailError::Database(e)) => {
            tracing::error!(refund_id = %id, error = %e, "Database error getting refund");
            AppError::Internal("Failed to get refund".into()).into_response()
        }
    }
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
