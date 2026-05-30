use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Refund;

pub trait RefundRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error>;
    async fn find_by_merchant(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error>;
    async fn insert(&self, refund: &Refund) -> Result<Refund, sqlx::Error>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<Refund, sqlx::Error>;
}

pub struct PostgresRefundRepository {
    pool: PgPool,
}

impl PostgresRefundRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl RefundRepository for PostgresRefundRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error> {
        sqlx::query_as::<_, Refund>("SELECT * FROM refunds WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_by_merchant(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error> {
        sqlx::query_as::<_, Refund>(
            "SELECT * FROM refunds WHERE merchant_id = $1 ORDER BY created_at DESC",
        )
        .bind(merchant_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn insert(&self, refund: &Refund) -> Result<Refund, sqlx::Error> {
        sqlx::query_as::<_, Refund>(
            r#"
            INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(refund.id)
        .bind(refund.payment_id)
        .bind(refund.merchant_id)
        .bind(refund.amount_minor)
        .bind(&refund.currency)
        .bind(&refund.status)
        .bind(&refund.idempotency_key)
        .bind(refund.created_at)
        .bind(refund.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_status(&self, id: Uuid, status: &str) -> Result<Refund, sqlx::Error> {
        sqlx::query_as::<_, Refund>(
            r#"
            UPDATE refunds SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }
}
