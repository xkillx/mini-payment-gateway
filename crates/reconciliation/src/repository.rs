use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{IncludedPaymentRecord, IncludedRefundRecord, Reconciliation};

pub trait ReconciliationRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error>;
    async fn find_all(&self) -> Result<Vec<Reconciliation>, sqlx::Error>;
    async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<Reconciliation>, sqlx::Error>;
    async fn insert(&self, record: &Reconciliation) -> Result<Reconciliation, sqlx::Error>;
}

pub struct PostgresReconciliationRepository {
    pool: PgPool,
}

impl PostgresReconciliationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        record: &Reconciliation,
    ) -> Result<Reconciliation, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>(
            r#"
            INSERT INTO reconciliations (id, status, expected_total_minor, actual_total_minor, discrepancy_minor, currency, window_start, window_end, notes, run_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(&record.status)
        .bind(record.expected_total_minor)
        .bind(record.actual_total_minor)
        .bind(record.discrepancy_minor)
        .bind(&record.currency)
        .bind(record.window_start)
        .bind(record.window_end)
        .bind(&record.notes)
        .bind(record.run_at)
        .bind(record.created_at)
        .fetch_one(tx.as_mut())
        .await
    }

    pub async fn calculate_expected_total(
        pool: &PgPool,
        currency: &str,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let row: (Option<i64>,) = sqlx::query_as(
            r#"
            SELECT COALESCE(
                (SELECT SUM(p.amount_minor)::BIGINT
                 FROM domain_events e
                 JOIN payments p ON e.aggregate_id = p.id
                 WHERE e.event_type = 'payment.successful'
                   AND e.aggregate_type = 'payment'
                   AND p.currency = $1
                   AND e.created_at >= $2
                   AND e.created_at < $3), 0)
              - COALESCE(
                (SELECT SUM(r.amount_minor)::BIGINT
                 FROM domain_events e
                 JOIN refunds r ON e.aggregate_id = r.id
                 WHERE e.event_type = 'refund.completed'
                   AND e.aggregate_type = 'refund'
                   AND r.currency = $1
                   AND e.created_at >= $2
                   AND e.created_at < $3), 0) AS expected_total
            "#,
        )
        .bind(currency)
        .bind(window_start)
        .bind(window_end)
        .fetch_one(pool)
        .await?;

        Ok(row.0.unwrap_or(0))
    }
}

impl ReconciliationRepository for PostgresReconciliationRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>("SELECT * FROM reconciliations WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_all(&self) -> Result<Vec<Reconciliation>, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>(
            "SELECT * FROM reconciliations ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<Reconciliation>, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>(
            "SELECT * FROM reconciliations ORDER BY created_at DESC, id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    async fn insert(&self, record: &Reconciliation) -> Result<Reconciliation, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>(
            r#"
            INSERT INTO reconciliations (id, status, expected_total_minor, actual_total_minor, discrepancy_minor, currency, window_start, window_end, notes, run_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(&record.status)
        .bind(record.expected_total_minor)
        .bind(record.actual_total_minor)
        .bind(record.discrepancy_minor)
        .bind(&record.currency)
        .bind(record.window_start)
        .bind(record.window_end)
        .bind(&record.notes)
        .bind(record.run_at)
        .bind(record.created_at)
        .fetch_one(&self.pool)
        .await
    }
}

impl PostgresReconciliationRepository {
    pub async fn find_included_payments(
        pool: &PgPool,
        currency: &str,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<Vec<IncludedPaymentRecord>, sqlx::Error> {
        sqlx::query_as::<_, IncludedPaymentRecord>(
            r#"
            SELECT e.id AS domain_event_id,
                   p.id AS payment_id,
                   p.merchant_id,
                   p.amount_minor,
                   p.currency,
                   e.created_at AS occurred_at,
                   p.metadata
            FROM domain_events e
            JOIN payments p ON e.aggregate_id = p.id
            WHERE e.event_type = 'payment.successful'
              AND e.aggregate_type = 'payment'
              AND p.currency = $1
              AND e.created_at >= $2
              AND e.created_at < $3
            ORDER BY e.created_at ASC, e.id ASC
            "#,
        )
        .bind(currency)
        .bind(window_start)
        .bind(window_end)
        .fetch_all(pool)
        .await
    }

    pub async fn find_included_refunds(
        pool: &PgPool,
        currency: &str,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> Result<Vec<IncludedRefundRecord>, sqlx::Error> {
        sqlx::query_as::<_, IncludedRefundRecord>(
            r#"
            SELECT e.id AS domain_event_id,
                   r.id AS refund_id,
                   r.payment_id,
                   r.merchant_id,
                   r.amount_minor,
                   r.currency,
                   e.created_at AS occurred_at
            FROM domain_events e
            JOIN refunds r ON e.aggregate_id = r.id
            WHERE e.event_type = 'refund.completed'
              AND e.aggregate_type = 'refund'
              AND r.currency = $1
              AND e.created_at >= $2
              AND e.created_at < $3
            ORDER BY e.created_at ASC, e.id ASC
            "#,
        )
        .bind(currency)
        .bind(window_start)
        .bind(window_end)
        .fetch_all(pool)
        .await
    }
}
