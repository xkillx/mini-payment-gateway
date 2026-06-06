use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use chrono::Utc;
use shared_auth::Actor;
use shared_http::error::AppError;
use sqlx::PgPool;

use crate::models::{
    default_payment_counts, default_refund_counts, DashboardPaymentListItem,
    DashboardRefundListItem, MerchantDashboardSummaryResponse, MerchantPaymentOverview,
    MerchantRefundOverview,
};
use crate::repository;

#[derive(Clone)]
pub struct DashboardRouteState {
    pub pool: PgPool,
    pub configured_currency: String,
}

async fn get_merchant_dashboard(
    State(state): State<DashboardRouteState>,
    Extension(actor): Extension<Actor>,
) -> Response {
    let merchant_id = match actor.merchant_id() {
        Some(id) => id,
        None => {
            return AppError::Forbidden("Merchant access required".into()).into_response();
        }
    };

    let payment_counts = match repository::get_payment_status_counts(&state.pool, merchant_id).await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(merchant_id = %merchant_id, error = %e, "Database error fetching payment status counts");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let recent_payments = match repository::get_recent_payments(&state.pool, merchant_id).await {
        Ok(payments) => payments,
        Err(e) => {
            tracing::error!(merchant_id = %merchant_id, error = %e, "Database error fetching recent payments");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let refund_counts = match repository::get_refund_status_counts(&state.pool, merchant_id).await {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(merchant_id = %merchant_id, error = %e, "Database error fetching refund status counts");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let recent_refunds = match repository::get_recent_refunds(&state.pool, merchant_id).await {
        Ok(refunds) => refunds,
        Err(e) => {
            tracing::error!(merchant_id = %merchant_id, error = %e, "Database error fetching recent refunds");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let mut payment_status_counts = default_payment_counts();
    for row in payment_counts {
        match row.status.as_str() {
            "pending" => payment_status_counts.pending = row.count,
            "processing" => payment_status_counts.processing = row.count,
            "successful" => payment_status_counts.successful = row.count,
            "failed" => payment_status_counts.failed = row.count,
            "refunded" => payment_status_counts.refunded = row.count,
            _ => {}
        }
    }

    let mut refund_status_counts = default_refund_counts();
    for row in refund_counts {
        match row.status.as_str() {
            "pending" => refund_status_counts.pending = row.count,
            "processing" => refund_status_counts.processing = row.count,
            "completed" => refund_status_counts.completed = row.count,
            "failed" => refund_status_counts.failed = row.count,
            _ => {}
        }
    }

    let response = MerchantDashboardSummaryResponse {
        configured_currency: state.configured_currency,
        generated_at: Utc::now(),
        payment_overview: MerchantPaymentOverview {
            status_counts: payment_status_counts,
            recent_payments: recent_payments
                .into_iter()
                .map(DashboardPaymentListItem::from)
                .collect(),
        },
        refund_overview: MerchantRefundOverview {
            status_counts: refund_status_counts,
            recent_refunds: recent_refunds
                .into_iter()
                .map(DashboardRefundListItem::from)
                .collect(),
        },
    };

    Json(response).into_response()
}

pub fn merchant_routes(pool: PgPool, configured_currency: String) -> Router {
    let state = DashboardRouteState {
        pool,
        configured_currency,
    };
    Router::new()
        .route("/merchant", axum::routing::get(get_merchant_dashboard))
        .with_state(state)
}
