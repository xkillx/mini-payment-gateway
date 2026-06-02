use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use crate::models::{Payment, PaymentListFilter, PaymentStatus};

pub trait PaymentRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Payment>, sqlx::Error>;
    async fn find_by_merchant(&self, merchant_id: Uuid) -> Result<Vec<Payment>, sqlx::Error>;
    async fn insert(&self, payment: &Payment) -> Result<Payment, sqlx::Error>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<Payment, sqlx::Error>;
    async fn find_by_merchant_and_idempotency_key(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<Payment>, sqlx::Error>;
    async fn try_insert_idempotent(
        &self,
        payment: &Payment,
    ) -> Result<Option<Payment>, sqlx::Error>;
    async fn list(&self, filter: &PaymentListFilter) -> Result<Vec<Payment>, sqlx::Error>;
}

pub struct PostgresPaymentRepository {
    pool: PgPool,
}

impl PostgresPaymentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PaymentRepository for PostgresPaymentRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Payment>, sqlx::Error> {
        sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_merchant(&self, merchant_id: Uuid) -> Result<Vec<Payment>, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments WHERE merchant_id = $1 ORDER BY created_at DESC",
        )
        .bind(merchant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn insert(&self, payment: &Payment) -> Result<Payment, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            r#"
            INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, failure_reason, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(payment.id)
        .bind(payment.merchant_id)
        .bind(payment.amount_minor)
        .bind(&payment.currency)
        .bind(&payment.status)
        .bind(&payment.idempotency_key)
        .bind(&payment.metadata)
        .bind(&payment.failure_reason)
        .bind(payment.created_at)
        .bind(payment.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_status(&self, id: Uuid, status: &str) -> Result<Payment, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            r#"
            UPDATE payments SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    async fn find_by_merchant_and_idempotency_key(
        &self,
        merchant_id: Uuid,
        idempotency_key: &str,
    ) -> Result<Option<Payment>, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            "SELECT * FROM payments WHERE merchant_id = $1 AND idempotency_key = $2",
        )
        .bind(merchant_id)
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
    }

    async fn try_insert_idempotent(
        &self,
        payment: &Payment,
    ) -> Result<Option<Payment>, sqlx::Error> {
        sqlx::query_as::<_, Payment>(
            r#"
            INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, failure_reason, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(payment.id)
        .bind(payment.merchant_id)
        .bind(payment.amount_minor)
        .bind(&payment.currency)
        .bind(&payment.status)
        .bind(&payment.idempotency_key)
        .bind(&payment.metadata)
        .bind(&payment.failure_reason)
        .bind(payment.created_at)
        .bind(payment.updated_at)
        .fetch_optional(&self.pool)
        .await
    }

    async fn list(&self, filter: &PaymentListFilter) -> Result<Vec<Payment>, sqlx::Error> {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM payments WHERE 1=1");

        if let Some(merchant_id) = filter.merchant_id {
            qb.push(" AND merchant_id = ").push_bind(merchant_id);
        }

        if let Some(status) = &filter.status {
            qb.push(" AND status = ").push_bind(status.clone());
        }

        if filter.search.is_some() || filter.search_id.is_some() {
            qb.push(" AND (");
            let mut branch = false;

            if let Some(needle) = &filter.search {
                let pattern = escape_like_pattern(needle);
                let like_pattern = format!("%{}%", pattern);
                qb.push("(");
                qb.push("jsonb_typeof(metadata->'merchant_reference') = 'string'");
                qb.push(" AND metadata->>'merchant_reference' ILIKE ");
                qb.push_bind(like_pattern);
                qb.push(" ESCAPE '\\'");
                qb.push(")");
                branch = true;
            }

            if let Some(id) = filter.search_id {
                if branch {
                    qb.push(" OR ");
                }
                qb.push("id = ").push_bind(id);
            }

            qb.push(")");
        }

        qb.push(" ORDER BY created_at DESC, id DESC LIMIT ")
            .push_bind(filter.limit);
        qb.push(" OFFSET ").push_bind(filter.offset);

        qb.build_query_as::<Payment>().fetch_all(&self.pool).await
    }
}

fn escape_like_pattern(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out
}

pub async fn insert_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    payment: &Payment,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, failure_reason, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (merchant_id, idempotency_key) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(payment.id)
    .bind(payment.merchant_id)
    .bind(payment.amount_minor)
    .bind(&payment.currency)
    .bind(&payment.status)
    .bind(&payment.idempotency_key)
    .bind(&payment.metadata)
    .bind(&payment.failure_reason)
    .bind(payment.created_at)
    .bind(payment.updated_at)
    .fetch_optional(tx.as_mut())
    .await
}

pub async fn find_payment_by_merchant_and_idempotency_key_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    merchant_id: Uuid,
    idempotency_key: &str,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        "SELECT * FROM payments WHERE merchant_id = $1 AND idempotency_key = $2",
    )
    .bind(merchant_id)
    .bind(idempotency_key)
    .fetch_optional(tx.as_mut())
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>("SELECT * FROM payments WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn claim_pending_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    payment_id: Uuid,
    now: DateTime<Utc>,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        r#"
        UPDATE payments
        SET status = 'processing', failure_reason = NULL, updated_at = $2
        WHERE id = $1 AND status = 'pending'
        RETURNING *
        "#,
    )
    .bind(payment_id)
    .bind(now)
    .fetch_optional(tx.as_mut())
    .await
}

pub async fn claim_next_pending_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        r#"
        WITH next_pending AS (
            SELECT id
            FROM payments
            WHERE status = 'pending'
            ORDER BY created_at ASC, id ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
        )
        UPDATE payments
        SET status = 'processing', failure_reason = NULL, updated_at = $1
        FROM next_pending
        WHERE payments.id = next_pending.id
        RETURNING payments.*
        "#,
    )
    .bind(now)
    .fetch_optional(tx.as_mut())
    .await
}

pub async fn finalize_processing_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    payment_id: Uuid,
    final_status: PaymentStatus,
    failure_reason: Option<&str>,
    now: DateTime<Utc>,
) -> Result<Option<Payment>, sqlx::Error> {
    let status_str: &'static str = match final_status {
        PaymentStatus::Successful => "successful",
        PaymentStatus::Failed => "failed",
        _ => {
            return Err(sqlx::Error::Protocol(
                "finalize_processing_payment_in_tx only accepts Successful or Failed".into(),
            ));
        }
    };

    let failure_reason_param: Option<&str> = match final_status {
        PaymentStatus::Failed => failure_reason,
        PaymentStatus::Successful => None,
        _ => None,
    };

    sqlx::query_as::<_, Payment>(
        r#"
        UPDATE payments
        SET status = $2::payment_status,
            failure_reason = $3,
            updated_at = $4
        WHERE id = $1 AND status = 'processing'
        RETURNING *
        "#,
    )
    .bind(payment_id)
    .bind(status_str)
    .bind(failure_reason_param)
    .bind(now)
    .fetch_optional(tx.as_mut())
    .await
}
