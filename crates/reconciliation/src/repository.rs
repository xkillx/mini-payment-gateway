use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Reconciliation;

pub trait ReconciliationRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error>;
    async fn find_all(&self) -> Result<Vec<Reconciliation>, sqlx::Error>;
    async fn insert(&self, record: &Reconciliation) -> Result<Reconciliation, sqlx::Error>;
}

pub struct PostgresReconciliationRepository {
    pool: PgPool,
}

impl PostgresReconciliationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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

    async fn insert(&self, record: &Reconciliation) -> Result<Reconciliation, sqlx::Error> {
        sqlx::query_as::<_, Reconciliation>(
            r#"
            INSERT INTO reconciliations (id, status, expected_total_minor, actual_total_minor, currency, notes, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(&record.status)
        .bind(record.expected_total_minor)
        .bind(record.actual_total_minor)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(record.created_at)
        .fetch_one(&self.pool)
        .await
    }
}
