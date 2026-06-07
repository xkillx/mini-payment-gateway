use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    AdminAuditRow, AdminFailedPaymentRow, AdminFailedRefundRow, AdminNotificationRow,
    AdminReconciliationRow, DashboardPaymentRow, DashboardRefundRow, NotificationStatusRow,
    OperationalNotificationFailureRow, OperationalPaymentFailureRow,
    OperationalReconciliationAttentionRow, OperationalRefundFailureRow, PaymentOutcomeCountRow,
    PaymentStatusCount, ReconciliationStatusRow, RefundStatusRow,
};

pub async fn get_payment_status_counts(
    pool: &PgPool,
    merchant_id: Uuid,
) -> Result<Vec<PaymentStatusCount>, sqlx::Error> {
    sqlx::query_as::<_, PaymentStatusCount>(
        "SELECT status::text AS status, COUNT(*) AS count FROM payments WHERE merchant_id = $1 GROUP BY status",
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await
}

pub async fn get_recent_payments(
    pool: &PgPool,
    merchant_id: Uuid,
) -> Result<Vec<DashboardPaymentRow>, sqlx::Error> {
    sqlx::query_as::<_, DashboardPaymentRow>(
        "SELECT id, merchant_id, amount_minor, currency, status::text AS status, metadata, failure_reason, created_at, updated_at FROM payments WHERE merchant_id = $1 ORDER BY created_at DESC, id DESC LIMIT 10",
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await
}

pub async fn get_refund_status_counts(
    pool: &PgPool,
    merchant_id: Uuid,
) -> Result<Vec<RefundStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, RefundStatusRow>(
        "SELECT status::text AS status, COUNT(*) AS count FROM refunds WHERE merchant_id = $1 GROUP BY status",
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await
}

pub async fn get_recent_refunds(
    pool: &PgPool,
    merchant_id: Uuid,
) -> Result<Vec<DashboardRefundRow>, sqlx::Error> {
    sqlx::query_as::<_, DashboardRefundRow>(
        "SELECT id, payment_id, merchant_id, amount_minor, currency, status::text AS status, created_at, updated_at FROM refunds WHERE merchant_id = $1 ORDER BY created_at DESC, id DESC LIMIT 10",
    )
    .bind(merchant_id)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_payment_status_counts_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<PaymentStatusCount>, sqlx::Error> {
    sqlx::query_as::<_, PaymentStatusCount>(
        "SELECT status::text AS status, COUNT(*) AS count FROM payments WHERE updated_at >= $1 GROUP BY status",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_failed_payments(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<AdminFailedPaymentRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminFailedPaymentRow>(
        "SELECT id, merchant_id, amount_minor, currency, status::text AS status, metadata, failure_reason, created_at, updated_at FROM payments WHERE status = 'failed' AND updated_at >= $1 ORDER BY updated_at DESC, id DESC LIMIT 5",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_refund_status_counts_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<RefundStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, RefundStatusRow>(
        "SELECT status::text AS status, COUNT(*) AS count FROM refunds WHERE updated_at >= $1 GROUP BY status",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_failed_refunds(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<AdminFailedRefundRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminFailedRefundRow>(
        "SELECT id, payment_id, merchant_id, amount_minor, currency, status::text AS status, created_at, updated_at FROM refunds WHERE status = 'failed' AND updated_at >= $1 ORDER BY updated_at DESC, id DESC LIMIT 5",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_notification_status_counts(
    pool: &PgPool,
) -> Result<Vec<NotificationStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, NotificationStatusRow>(
        "SELECT status::text AS status, COUNT(*) AS count FROM notification_delivery_records GROUP BY status",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_failed_notifications(
    pool: &PgPool,
) -> Result<Vec<AdminNotificationRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminNotificationRow>(
        "SELECT ndr.id, ndr.domain_event_id, de.event_type, de.aggregate_type AS resource_type, de.aggregate_id AS resource_id, de.aggregate_id AS payment_id, ndr.destination_url, ndr.status::text AS status, ndr.attempt_count, ndr.last_error, ndr.updated_at FROM notification_delivery_records ndr JOIN domain_events de ON ndr.domain_event_id = de.id WHERE ndr.status = 'failed' ORDER BY ndr.updated_at DESC, ndr.id DESC LIMIT 5",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_admin_reconciliation_status_counts_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<ReconciliationStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, ReconciliationStatusRow>(
        "SELECT status::text AS status, COUNT(*) AS count FROM reconciliations WHERE created_at >= $1 GROUP BY status",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_attention_reconciliations(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<AdminReconciliationRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminReconciliationRow>(
        "SELECT id, status::text AS status, expected_total_minor, actual_total_minor, discrepancy_minor, currency, window_start, window_end, created_at FROM reconciliations WHERE status IN ('mismatched', 'error') AND created_at >= $1 ORDER BY created_at DESC, id DESC LIMIT 5",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_attention_audit_records(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<AdminAuditRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminAuditRow>(
        "SELECT id, actor_id, actor_type::text AS actor_type, action, resource_type, resource_id, details, occurred_at, created_at FROM audit_records WHERE occurred_at >= $1 AND action IN ('auth.authentication_failed', 'auth.authorization_failed', 'payment.failed', 'refund.rejected') ORDER BY occurred_at DESC, created_at DESC, id DESC LIMIT 5",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_payment_outcome_counts_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<PaymentOutcomeCountRow>, sqlx::Error> {
    sqlx::query_as::<_, PaymentOutcomeCountRow>(
        "SELECT event_type, COUNT(*) AS count FROM domain_events WHERE aggregate_type = 'payment' AND event_type IN ('payment.successful', 'payment.failed') AND created_at >= $1 GROUP BY event_type",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_notification_status_counts_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<NotificationStatusRow>, sqlx::Error> {
    sqlx::query_as::<_, NotificationStatusRow>(
        "SELECT status::text AS status, COUNT(*) AS count FROM notification_delivery_records WHERE updated_at >= $1 GROUP BY status",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_recent_failed_notifications_window(
    pool: &PgPool,
    window_start: DateTime<Utc>,
) -> Result<Vec<AdminNotificationRow>, sqlx::Error> {
    sqlx::query_as::<_, AdminNotificationRow>(
        "SELECT ndr.id, ndr.domain_event_id, de.event_type, de.aggregate_type AS resource_type, de.aggregate_id AS resource_id, de.aggregate_id AS payment_id, ndr.destination_url, ndr.status::text AS status, ndr.attempt_count, ndr.last_error, ndr.updated_at FROM notification_delivery_records ndr JOIN domain_events de ON ndr.domain_event_id = de.id WHERE ndr.status = 'failed' AND ndr.updated_at >= $1 ORDER BY ndr.updated_at DESC, ndr.id DESC LIMIT 5",
    )
    .bind(window_start)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_operational_payment_failures(
    pool: &PgPool,
    window_start: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<OperationalPaymentFailureRow>, sqlx::Error> {
    sqlx::query_as::<_, OperationalPaymentFailureRow>(
        "SELECT p.id, p.merchant_id, p.amount_minor, p.currency, p.status::text AS status, p.failure_reason, de.created_at AS occurred_at, de.event_type FROM payments p JOIN domain_events de ON de.aggregate_type = 'payment' AND de.aggregate_id = p.id WHERE de.event_type = 'payment.failed' AND de.created_at >= $1 ORDER BY de.created_at DESC, p.id DESC LIMIT $2",
    )
    .bind(window_start)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_operational_refund_failures(
    pool: &PgPool,
    window_start: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<OperationalRefundFailureRow>, sqlx::Error> {
    sqlx::query_as::<_, OperationalRefundFailureRow>(
        "SELECT id, payment_id, merchant_id, amount_minor, currency, status::text AS status, updated_at AS occurred_at FROM refunds WHERE status = 'failed' AND updated_at >= $1 ORDER BY updated_at DESC, id DESC LIMIT $2",
    )
    .bind(window_start)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_operational_notification_failures(
    pool: &PgPool,
    window_start: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<OperationalNotificationFailureRow>, sqlx::Error> {
    sqlx::query_as::<_, OperationalNotificationFailureRow>(
        "SELECT ndr.id, de.aggregate_id AS payment_id, de.event_type, de.aggregate_type AS resource_type, de.aggregate_id AS resource_id, ndr.status::text AS status, ndr.last_error, ndr.updated_at AS occurred_at FROM notification_delivery_records ndr JOIN domain_events de ON ndr.domain_event_id = de.id WHERE ndr.status = 'failed' AND ndr.updated_at >= $1 ORDER BY ndr.updated_at DESC, ndr.id DESC LIMIT $2",
    )
    .bind(window_start)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_admin_operational_reconciliation_attention(
    pool: &PgPool,
    window_start: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<OperationalReconciliationAttentionRow>, sqlx::Error> {
    sqlx::query_as::<_, OperationalReconciliationAttentionRow>(
        "SELECT id, status::text AS status, expected_total_minor, actual_total_minor, discrepancy_minor, currency, created_at AS occurred_at FROM reconciliations WHERE status IN ('mismatched', 'error') AND created_at >= $1 ORDER BY created_at DESC, id DESC LIMIT $2",
    )
    .bind(window_start)
    .bind(limit)
    .fetch_all(pool)
    .await
}
