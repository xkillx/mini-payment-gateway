use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AuditRecord, AuditRecordFilter, NewAuditRecord};

pub trait AuditRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AuditRecord>, sqlx::Error>;
    async fn list(&self, filter: &AuditRecordFilter) -> Result<Vec<AuditRecord>, sqlx::Error>;
    async fn insert(&self, record: &NewAuditRecord) -> Result<AuditRecord, sqlx::Error>;
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

    async fn list(&self, filter: &AuditRecordFilter) -> Result<Vec<AuditRecord>, sqlx::Error> {
        let mut builder = sqlx::QueryBuilder::new(
            "SELECT id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at FROM audit_records WHERE 1=1",
        );

        if let Some(actor_id) = filter.actor_id {
            builder.push(" AND actor_id = ").push_bind(actor_id);
        }
        if let Some(ref actor_type) = filter.actor_type {
            builder.push(" AND actor_type = ").push_bind(actor_type);
        }
        if let Some(ref resource_type) = filter.resource_type {
            builder
                .push(" AND resource_type = ")
                .push_bind(resource_type);
        }
        if let Some(ref resource_id) = filter.resource_id {
            builder.push(" AND resource_id = ").push_bind(resource_id);
        }
        if let Some(ref action) = filter.action {
            builder.push(" AND action = ").push_bind(action);
        }
        if let Some(occurred_from) = filter.occurred_from {
            builder
                .push(" AND occurred_at >= ")
                .push_bind(occurred_from);
        }
        if let Some(occurred_to) = filter.occurred_to {
            builder.push(" AND occurred_at <= ").push_bind(occurred_to);
        }

        builder.push(" ORDER BY occurred_at DESC, created_at DESC, id DESC");
        builder.push(" LIMIT ").push_bind(filter.limit);
        builder.push(" OFFSET ").push_bind(filter.offset);

        builder
            .build_query_as::<AuditRecord>()
            .fetch_all(&self.pool)
            .await
    }

    async fn insert(&self, record: &NewAuditRecord) -> Result<AuditRecord, sqlx::Error> {
        sqlx::query_as::<_, AuditRecord>(
            r#"
            INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(record.actor_id)
        .bind(&record.actor_type)
        .bind(&record.action)
        .bind(&record.resource_type)
        .bind(&record.resource_id)
        .bind(&record.details)
        .bind(record.occurred_at)
        .bind(record.created_at)
        .fetch_one(&self.pool)
        .await
    }
}
