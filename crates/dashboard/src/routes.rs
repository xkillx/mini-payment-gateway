use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use chrono::{Duration, Utc};
use shared_auth::Actor;
use shared_http::error::AppError;
use sqlx::PgPool;

use crate::models::{
    default_notification_counts, default_payment_counts, default_reconciliation_counts,
    default_refund_counts, AdminAuditListItem, AdminAuditOverview, AdminDashboardSummaryResponse,
    AdminNotificationListItem, AdminNotificationOverview, AdminOperationalHealth,
    AdminPaymentOverview, AdminReconciliationListItem, AdminReconciliationOverview,
    AdminRefundOverview, DashboardPaymentListItem, DashboardRefundListItem,
    MerchantDashboardSummaryResponse, MerchantPaymentOverview, MerchantRefundOverview,
    NotificationStatusCounts, OperationalHealthFailedOperation, OperationalHealthRate,
    OperationalNotificationFailureRow, OperationalPaymentFailureRow,
    OperationalReconciliationAttentionRow, OperationalRefundFailureRow, PaymentOutcomeCountRow,
    PaymentStatusCounts, ReconciliationStatusCounts,
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

async fn get_admin_dashboard(State(state): State<DashboardRouteState>) -> Response {
    let window_end = Utc::now();
    let window_start = window_end - Duration::hours(24);

    let payment_counts =
        match repository::get_admin_payment_status_counts_window(&state.pool, window_start).await {
            Ok(counts) => counts,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin payment status counts");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let recent_failed_payments =
        match repository::get_admin_recent_failed_payments(&state.pool, window_start).await {
            Ok(payments) => payments,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin recent failed payments");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let refund_counts =
        match repository::get_admin_refund_status_counts_window(&state.pool, window_start).await {
            Ok(counts) => counts,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin refund status counts");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let recent_failed_refunds =
        match repository::get_admin_recent_failed_refunds(&state.pool, window_start).await {
            Ok(refunds) => refunds,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin recent failed refunds");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let notification_counts = match repository::get_admin_notification_status_counts_window(
        &state.pool,
        window_start,
    )
    .await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching admin notification status counts");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let recent_failed_notifications =
        match repository::get_admin_recent_failed_notifications_window(&state.pool, window_start)
            .await
        {
            Ok(notifications) => notifications,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin recent failed notifications");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let reconciliation_counts = match repository::get_admin_reconciliation_status_counts_window(
        &state.pool,
        window_start,
    )
    .await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching admin reconciliation status counts");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let recent_attention_reconciliations =
        match repository::get_admin_recent_attention_reconciliations(&state.pool, window_start)
            .await
        {
            Ok(reconciliations) => reconciliations,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching admin recent attention reconciliations");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let recent_attention_audit_records = match repository::get_admin_recent_attention_audit_records(
        &state.pool,
        window_start,
    )
    .await
    {
        Ok(records) => records,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching admin recent attention audit records");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let payment_outcome_counts = match repository::get_admin_payment_outcome_counts_window(
        &state.pool,
        window_start,
    )
    .await
    {
        Ok(counts) => counts,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching admin payment outcome counts");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let op_payment_failures =
        match repository::get_admin_operational_payment_failures(&state.pool, window_start, 10)
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching operational payment failures");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
                        &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                    )),
                )
                    .into_response();
            }
        };

    let op_refund_failures = match repository::get_admin_operational_refund_failures(
        &state.pool,
        window_start,
        10,
    )
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching operational refund failures");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let op_notification_failures = match repository::get_admin_operational_notification_failures(
        &state.pool,
        window_start,
        10,
    )
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching operational notification failures");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(shared_http::error::ErrorEnvelope::new(
                    "INTERNAL_ERROR",
                    "Failed to load administrator dashboard summary",
                    &uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string(),
                )),
            )
                .into_response();
        }
    };

    let op_reconciliation_attention =
        match repository::get_admin_operational_reconciliation_attention(
            &state.pool,
            window_start,
            10,
        )
        .await
        {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "Database error fetching operational reconciliation attention");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(shared_http::error::ErrorEnvelope::new(
                        "INTERNAL_ERROR",
                        "Failed to load administrator dashboard summary",
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

    let mut notif_status_counts = default_notification_counts();
    for row in notification_counts {
        match row.status.as_str() {
            "pending" => notif_status_counts.pending = row.count,
            "processing" => notif_status_counts.processing = row.count,
            "delivered" => notif_status_counts.delivered = row.count,
            "failed" => notif_status_counts.failed = row.count,
            _ => {}
        }
    }

    let mut rec_status_counts = default_reconciliation_counts();
    for row in reconciliation_counts {
        match row.status.as_str() {
            "matched" => rec_status_counts.matched = row.count,
            "mismatched" => rec_status_counts.mismatched = row.count,
            "error" => rec_status_counts.error = row.count,
            _ => {}
        }
    }

    let operational_health = build_operational_health(
        &payment_outcome_counts,
        &notif_status_counts,
        &rec_status_counts,
        &payment_status_counts,
        op_payment_failures,
        op_refund_failures,
        op_notification_failures,
        op_reconciliation_attention,
    );

    let response = AdminDashboardSummaryResponse {
        configured_currency: state.configured_currency.clone(),
        generated_at: Utc::now(),
        window_start,
        window_end,
        api_status: "reachable".into(),
        payment_overview: AdminPaymentOverview {
            status_counts: payment_status_counts,
            recent_failed_payments: recent_failed_payments
                .into_iter()
                .map(DashboardPaymentListItem::from)
                .collect(),
        },
        refund_overview: AdminRefundOverview {
            status_counts: refund_status_counts,
            recent_failed_refunds: recent_failed_refunds
                .into_iter()
                .map(DashboardRefundListItem::from)
                .collect(),
        },
        notification_overview: AdminNotificationOverview {
            status_counts: notif_status_counts,
            recent_failed_notifications: recent_failed_notifications
                .into_iter()
                .map(AdminNotificationListItem::from)
                .collect(),
        },
        reconciliation_overview: AdminReconciliationOverview {
            status_counts: rec_status_counts,
            recent_attention_reconciliations: recent_attention_reconciliations
                .into_iter()
                .map(AdminReconciliationListItem::from)
                .collect(),
        },
        audit_overview: AdminAuditOverview {
            recent_attention_audit_records: recent_attention_audit_records
                .into_iter()
                .map(AdminAuditListItem::from)
                .collect(),
        },
        operational_health,
    };

    Json(response).into_response()
}

fn rate_percent(numerator: i64, denominator: i64) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some((numerator as f64 / denominator as f64) * 100.0)
    }
}

fn build_operational_health(
    payment_outcome_counts: &[PaymentOutcomeCountRow],
    notif_status_counts: &NotificationStatusCounts,
    rec_status_counts: &ReconciliationStatusCounts,
    payment_status_counts: &PaymentStatusCounts,
    payment_failures: Vec<OperationalPaymentFailureRow>,
    refund_failures: Vec<OperationalRefundFailureRow>,
    notification_failures: Vec<OperationalNotificationFailureRow>,
    reconciliation_attention: Vec<OperationalReconciliationAttentionRow>,
) -> AdminOperationalHealth {
    let mut payment_successful: i64 = 0;
    let mut payment_failed: i64 = 0;
    for row in payment_outcome_counts {
        match row.event_type.as_str() {
            "payment.successful" => payment_successful = row.count,
            "payment.failed" => payment_failed = row.count,
            _ => {}
        }
    }

    let payment_numerator = payment_successful;
    let payment_denominator = payment_successful + payment_failed;
    let payment_in_flight = payment_status_counts.pending + payment_status_counts.processing;

    let notif_numerator = notif_status_counts.delivered;
    let notif_denominator = notif_status_counts.delivered + notif_status_counts.failed;
    let notif_in_flight = notif_status_counts.pending + notif_status_counts.processing;

    let rec_numerator = rec_status_counts.matched + rec_status_counts.mismatched;
    let rec_denominator =
        rec_status_counts.matched + rec_status_counts.mismatched + rec_status_counts.error;
    let rec_in_flight: i64 = 0;

    let mut failed_operations: Vec<OperationalHealthFailedOperation> = Vec::new();

    for row in payment_failures {
        failed_operations.push(OperationalHealthFailedOperation {
            kind: "payment".into(),
            id: row.id,
            occurred_at: row.occurred_at,
            status: row.status,
            merchant_id: Some(row.merchant_id),
            payment_id: Some(row.id),
            amount_minor: Some(row.amount_minor),
            currency: Some(row.currency),
            reason: row.failure_reason,
            event_type: Some(row.event_type),
            resource_type: Some("payment".into()),
            resource_id: Some(row.id),
            discrepancy_minor: None,
        });
    }

    for row in refund_failures {
        failed_operations.push(OperationalHealthFailedOperation {
            kind: "refund".into(),
            id: row.id,
            occurred_at: row.occurred_at,
            status: row.status,
            merchant_id: Some(row.merchant_id),
            payment_id: Some(row.payment_id),
            amount_minor: Some(row.amount_minor),
            currency: Some(row.currency),
            reason: None,
            event_type: None,
            resource_type: Some("refund".into()),
            resource_id: Some(row.id),
            discrepancy_minor: None,
        });
    }

    for row in notification_failures {
        failed_operations.push(OperationalHealthFailedOperation {
            kind: "notification_delivery_record".into(),
            id: row.id,
            occurred_at: row.occurred_at,
            status: row.status,
            merchant_id: None,
            payment_id: Some(row.payment_id),
            amount_minor: None,
            currency: None,
            reason: row.last_error,
            event_type: Some(row.event_type),
            resource_type: Some(row.resource_type),
            resource_id: Some(row.resource_id),
            discrepancy_minor: None,
        });
    }

    for row in reconciliation_attention {
        failed_operations.push(OperationalHealthFailedOperation {
            kind: "reconciliation".into(),
            id: row.id,
            occurred_at: row.occurred_at,
            status: row.status,
            merchant_id: None,
            payment_id: None,
            amount_minor: None,
            currency: Some(row.currency),
            reason: None,
            event_type: None,
            resource_type: Some("reconciliation".into()),
            resource_id: Some(row.id),
            discrepancy_minor: Some(row.discrepancy_minor),
        });
    }

    failed_operations.sort_by(|a, b| {
        b.occurred_at
            .cmp(&a.occurred_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    failed_operations.truncate(10);

    AdminOperationalHealth {
        payment_processing_success_rate: OperationalHealthRate {
            numerator_count: payment_numerator,
            denominator_count: payment_denominator,
            in_flight_count: payment_in_flight,
            rate_percent: rate_percent(payment_numerator, payment_denominator),
        },
        notification_delivery_success_rate: OperationalHealthRate {
            numerator_count: notif_numerator,
            denominator_count: notif_denominator,
            in_flight_count: notif_in_flight,
            rate_percent: rate_percent(notif_numerator, notif_denominator),
        },
        reconciliation_completion_rate: OperationalHealthRate {
            numerator_count: rec_numerator,
            denominator_count: rec_denominator,
            in_flight_count: rec_in_flight,
            rate_percent: rate_percent(rec_numerator, rec_denominator),
        },
        recent_failed_operations: failed_operations,
    }
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

pub fn admin_routes(pool: PgPool, configured_currency: String) -> Router {
    let state = DashboardRouteState {
        pool,
        configured_currency,
    };
    Router::new()
        .route("/admin", axum::routing::get(get_admin_dashboard))
        .with_state(state)
}
