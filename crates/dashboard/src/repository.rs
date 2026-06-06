use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{DashboardPaymentRow, DashboardRefundRow, PaymentStatusCount, RefundStatusRow};

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
