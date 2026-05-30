use sqlx::PgPool;
use uuid::Uuid;

use crate::models::AuditRecord;

pub trait AuditRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuditRecord>, sqlx::Error>;
    async fn find_all(&self) -> Result<Vec<AuditRecord>, sqlx::Error>;
    async fn insert(&self, record: &AuditRecord) -> Result<AuditRecord, sqlx::Error>;
}

pub struct PostgresAuditRepository {
    pool: PgPool,
}

impl PostgresAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuditRepository for PostgresAuditRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuditRecord>, sqlx::Error> {
        sqlx::query_as::<_, AuditRecord>("SELECT * FROM audit_records WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    async fn find_all(&self) -> Result<Vec<AuditRecord>, sqlx::Error> {
        sqlx::query_as::<_, AuditRecord>("SELECT * FROM audit_records ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    async fn insert(&self, record: &AuditRecord) -> Result<AuditRecord, sqlx::Error> {
        sqlx::query_as::<_, AuditRecord>(
            r#"
            INSERT INTO audit_records (id, actor_id, action, resource_type, resource_id, details, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(record.actor_id)
        .bind(&record.action)
        .bind(&record.resource_type)
        .bind(&record.resource_id)
        .bind(&record.details)
        .bind(record.created_at)
        .fetch_one(&self.pool)
        .await
    }
}
