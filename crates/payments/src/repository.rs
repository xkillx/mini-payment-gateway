use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::Payment;

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
            INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
            INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
        .bind(payment.created_at)
        .bind(payment.updated_at)
        .fetch_optional(&self.pool)
        .await
    }
}

pub async fn insert_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    payment: &Payment,
) -> Result<Option<Payment>, sqlx::Error> {
    sqlx::query_as::<_, Payment>(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
