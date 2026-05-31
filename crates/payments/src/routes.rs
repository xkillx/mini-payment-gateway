use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use shared_auth::Actor;
use shared_http::error::AppError;
use sqlx::PgPool;
use validator::ValidationErrors;

use crate::models::PaymentResponse;
use crate::service::{
    create_payment, CreatePaymentCommand, CreatePaymentError, CreatePaymentOutcome,
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

async fn list_payments() -> impl axum::response::IntoResponse {
    AppError::NotImplemented("payments.list")
}

async fn get_payment() -> impl axum::response::IntoResponse {
    AppError::NotImplemented("payments.get")
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
