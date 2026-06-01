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

use crate::models::{parse_payment_status, PaymentListFilter, PaymentResponse, PaymentStatus};
use crate::service::{
    create_payment, get_payment_detail, list_payments as list_payments_service,
    CreatePaymentCommand, CreatePaymentError, CreatePaymentOutcome, GetPaymentDetailError,
    ListPaymentsError,
};

#[derive(Clone)]
pub struct PaymentRouteState {
    pub pool: PgPool,
    pub payment_currency: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatePaymentRequest {
    pub amount_minor: i64,
    pub currency: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

pub fn routes(pool: PgPool, payment_currency: String) -> Router {
    let state = PaymentRouteState {
        pool,
        payment_currency,
    };

    Router::new()
        .route(
            "/",
            axum::routing::get(list_payments).post(create_payment_handler),
        )
        .route("/{id}", axum::routing::get(get_payment))
        .with_state(state)
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ListPaymentsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    search: Option<String>,
    #[serde(default)]
    merchant_id: Option<String>,
    #[serde(default)]
    limit: Option<String>,
    #[serde(default)]
    offset: Option<String>,
}

const SEARCH_MAX_LEN: usize = 128;
const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

async fn list_payments(
    State(state): State<PaymentRouteState>,
    Extension(actor): Extension<Actor>,
    Query(query): Query<ListPaymentsQuery>,
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

    let status: Option<PaymentStatus> = match query.status.as_deref() {
        None => None,
        Some(raw) => match parse_payment_status(raw) {
            Some(s) => Some(s),
            None => {
                return validation_error(
                    "status",
                    "invalid",
                    "status must be one of pending, processing, successful, failed, refunded",
                )
                .into_response();
            }
        },
    };

    let search = match query.search.as_deref() {
        None => None,
        Some(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                if trimmed.chars().count() > SEARCH_MAX_LEN {
                    return validation_error_string(
                        "search",
                        "invalid",
                        format!("search must be at most {SEARCH_MAX_LEN} characters"),
                    )
                    .into_response();
                }
                Some(trimmed.to_string())
            }
        }
    };

    let search_id = search.as_deref().and_then(|s| Uuid::parse_str(s).ok());

    if actor.is_merchant() && query.merchant_id.is_some() {
        return AppError::Forbidden("Merchants cannot filter payments by merchant_id".into())
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

    let filter = PaymentListFilter {
        merchant_id,
        status,
        search,
        search_id,
        limit,
        offset,
    };

    match list_payments_service(&state.pool, filter).await {
        Ok(response) => Json(response).into_response(),
        Err(ListPaymentsError::Database(e)) => {
            tracing::error!(error = %e, "Database error listing payments");
            AppError::Internal("Failed to list payments".into()).into_response()
        }
    }
}

async fn get_payment(
    State(state): State<PaymentRouteState>,
    Extension(actor): Extension<Actor>,
    Path(id): Path<Uuid>,
) -> Response {
    let viewer_merchant_id = actor.merchant_id();

    match get_payment_detail(&state.pool, id, viewer_merchant_id).await {
        Ok(detail) => Json(detail).into_response(),
        Err(GetPaymentDetailError::NotFound) => {
            AppError::NotFound("Payment not found".into()).into_response()
        }
        Err(GetPaymentDetailError::Database(e)) => {
            tracing::error!(payment_id = %id, error = %e, "Database error getting payment");
            AppError::Internal("Failed to get payment".into()).into_response()
        }
    }
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

async fn create_payment_handler(
    State(state): State<PaymentRouteState>,
    Extension(actor): Extension<Actor>,
    headers: HeaderMap,
    body: Result<Json<CreatePaymentRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    if !actor.is_merchant() {
        return AppError::Forbidden("Only merchants can create payments".into()).into_response();
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

    if req.amount_minor <= 0 {
        return validation_error("amount_minor", "gt", "amount_minor must be > 0").into_response();
    }

    if req.currency != state.payment_currency {
        return validation_error_string(
            "currency",
            "invalid_currency",
            format!(
                "Unsupported currency: {}. Only {} is accepted",
                req.currency, state.payment_currency
            ),
        )
        .into_response();
    }

    let metadata = match req.metadata {
        Some(val) => {
            if !val.is_object() {
                return validation_error(
                    "metadata",
                    "invalid_metadata",
                    "metadata must be a JSON object",
                )
                .into_response();
            }
            let serialized = serde_json::to_vec(&val).unwrap_or_default();
            if serialized.len() > 4096 {
                return validation_error(
                    "metadata",
                    "metadata_too_large",
                    "metadata serialized size must be at most 4096 bytes",
                )
                .into_response();
            }
            val
        }
        None => serde_json::Value::Object(serde_json::Map::new()),
    };

    let merchant_id = actor.merchant_id().unwrap();

    let cmd = CreatePaymentCommand {
        actor_id: actor.actor_id(),
        merchant_id,
        amount_minor: req.amount_minor,
        currency: req.currency,
        metadata,
        idempotency_key,
    };

    match create_payment(&state.pool, cmd).await {
        Ok(CreatePaymentOutcome::Created(payment)) => (
            axum::http::StatusCode::CREATED,
            Json(PaymentResponse::from(payment)),
        )
            .into_response(),
        Ok(CreatePaymentOutcome::Replayed(payment)) => (
            axum::http::StatusCode::OK,
            Json(PaymentResponse::from(payment)),
        )
            .into_response(),
        Err(CreatePaymentError::Conflict(msg)) => AppError::Conflict(msg).into_response(),
        Err(CreatePaymentError::Database(e)) => {
            tracing::error!(error = %e, "Database error creating payment");
            AppError::Internal("Failed to create payment".into()).into_response()
        }
        Err(CreatePaymentError::Audit(e)) => {
            tracing::error!(error = %e, "Audit error creating payment");
            AppError::Internal("Failed to create payment".into()).into_response()
        }
    }
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
