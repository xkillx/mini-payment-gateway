use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::models::{CountAmountRow, SimpleCountRow, TrendRow};

pub(crate) async fn fetch_created_payment_totals(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<CountAmountRow, sqlx::Error> {
    sqlx::query_as::<_, CountAmountRow>(
        r#"
        SELECT
            COUNT(*)::BIGINT as count,
            COALESCE(SUM(amount_minor), 0)::BIGINT as amount
        FROM payments
        WHERE currency = $1
          AND created_at >= $2
          AND created_at < $3
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub(crate) async fn fetch_successful_payment_totals(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<CountAmountRow, sqlx::Error> {
    sqlx::query_as::<_, CountAmountRow>(
        r#"
        SELECT
            COUNT(*)::BIGINT as count,
            COALESCE(SUM(p.amount_minor), 0)::BIGINT as amount
        FROM domain_events e
        JOIN payments p ON e.aggregate_id = p.id
        WHERE e.event_type = 'payment.successful'
          AND e.aggregate_type = 'payment'
          AND p.currency = $1
          AND e.created_at >= $2
          AND e.created_at < $3
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub(crate) async fn fetch_failed_payment_totals(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<CountAmountRow, sqlx::Error> {
    sqlx::query_as::<_, CountAmountRow>(
        r#"
        SELECT
            COUNT(*)::BIGINT as count,
            COALESCE(SUM(p.amount_minor), 0)::BIGINT as amount
        FROM domain_events e
        JOIN payments p ON e.aggregate_id = p.id
        WHERE e.event_type = 'payment.failed'
          AND e.aggregate_type = 'payment'
          AND p.currency = $1
          AND e.created_at >= $2
          AND e.created_at < $3
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub(crate) async fn fetch_completed_refund_totals(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<CountAmountRow, sqlx::Error> {
    sqlx::query_as::<_, CountAmountRow>(
        r#"
        WITH refund_outcomes AS (
            SELECT r.id, r.amount_minor,
                   COALESCE(
                     (SELECT e.created_at FROM domain_events e
                      WHERE e.event_type = 'refund.completed'
                        AND e.aggregate_type = 'refund'
                        AND e.aggregate_id = r.id
                      LIMIT 1),
                     r.updated_at
                   ) AS outcome_at
            FROM refunds r
            WHERE r.status = 'completed' AND r.currency = $1
        )
        SELECT
            COUNT(*)::BIGINT as count,
            COALESCE(SUM(amount_minor), 0)::BIGINT as amount
        FROM refund_outcomes
        WHERE outcome_at >= $2 AND outcome_at < $3
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub(crate) async fn fetch_failed_refund_count(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<SimpleCountRow, sqlx::Error> {
    sqlx::query_as::<_, SimpleCountRow>(
        r#"
        WITH refund_outcomes AS (
            SELECT r.id,
                   COALESCE(
                     (SELECT e.created_at FROM domain_events e
                      WHERE e.event_type = 'refund.failed'
                        AND e.aggregate_type = 'refund'
                        AND e.aggregate_id = r.id
                      LIMIT 1),
                     r.updated_at
                   ) AS outcome_at
            FROM refunds r
            WHERE r.status = 'failed' AND r.currency = $1
        )
        SELECT COUNT(*)::BIGINT as count
        FROM refund_outcomes
        WHERE outcome_at >= $2 AND outcome_at < $3
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_one(pool)
    .await
}

pub(crate) async fn fetch_created_trend(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<TrendRow>, sqlx::Error> {
    sqlx::query_as::<_, TrendRow>(
        r#"
        SELECT
            (created_at AT TIME ZONE 'UTC')::date as bucket_date,
            COUNT(*)::BIGINT as count,
            0::BIGINT as amount
        FROM payments
        WHERE currency = $1
          AND created_at >= $2
          AND created_at < $3
        GROUP BY bucket_date
        ORDER BY bucket_date
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(pool)
    .await
}

pub(crate) async fn fetch_successful_trend(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<TrendRow>, sqlx::Error> {
    sqlx::query_as::<_, TrendRow>(
        r#"
        SELECT
            (e.created_at AT TIME ZONE 'UTC')::date as bucket_date,
            COUNT(*)::BIGINT as count,
            COALESCE(SUM(p.amount_minor), 0)::BIGINT as amount
        FROM domain_events e
        JOIN payments p ON e.aggregate_id = p.id
        WHERE e.event_type = 'payment.successful'
          AND e.aggregate_type = 'payment'
          AND p.currency = $1
          AND e.created_at >= $2
          AND e.created_at < $3
        GROUP BY bucket_date
        ORDER BY bucket_date
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(pool)
    .await
}

pub(crate) async fn fetch_failed_trend(
    pool: &PgPool,
    currency: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<Vec<TrendRow>, sqlx::Error> {
    sqlx::query_as::<_, TrendRow>(
        r#"
        SELECT
            (e.created_at AT TIME ZONE 'UTC')::date as bucket_date,
            COUNT(*)::BIGINT as count,
            0::BIGINT as amount
        FROM domain_events e
        JOIN payments p ON e.aggregate_id = p.id
        WHERE e.event_type = 'payment.failed'
          AND e.aggregate_type = 'payment'
          AND p.currency = $1
          AND e.created_at >= $2
          AND e.created_at < $3
        GROUP BY bucket_date
        ORDER BY bucket_date
        "#,
    )
    .bind(currency)
    .bind(period_start)
    .bind(period_end)
    .fetch_all(pool)
    .await
}
