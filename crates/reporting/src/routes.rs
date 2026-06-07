use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared_http::error::AppError;
use sqlx::PgPool;
use validator::ValidationErrors;

use crate::service::{self, PaymentReportingPeriodError};

#[derive(Debug, Deserialize)]
struct PaymentSummaryQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

#[derive(Clone)]
struct RouteState {
    pool: PgPool,
    configured_currency: String,
}

pub fn routes(pool: PgPool, configured_currency: String) -> Router {
    let state = RouteState {
        pool,
        configured_currency,
    };

    Router::new()
        .route("/payment-summary", get(payment_summary))
        .with_state(state)
}

async fn payment_summary(
    State(state): State<RouteState>,
    query: Result<Query<PaymentSummaryQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<crate::models::PaymentSummaryReport>, AppError> {
    let query = query.map_err(|_| {
        let mut errors = ValidationErrors::new();
        errors.add("query", validator::ValidationError::new("invalid_query_parameters"));
        AppError::Validation(errors)
    })?;

    let period = service::resolve_period(query.from, query.to, Utc::now())
        .map_err(|e| {
            let mut errors = ValidationErrors::new();
            match e {
                PaymentReportingPeriodError::InvalidRange => {
                    errors.add("to", validator::ValidationError::new("to_must_be_after_from"));
                }
                PaymentReportingPeriodError::PeriodTooLong => {
                    errors.add("period", validator::ValidationError::new("period_must_not_exceed_366_days"));
                }
            }
            AppError::Validation(errors)
        })?;

    let report = service::payment_summary(&state.pool, &state.configured_currency, period)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load payment report");
            AppError::Internal("Failed to load payment report".into())
        })?;

    Ok(Json(report))
}
