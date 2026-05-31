use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{AuditRecord, AuditRecordFilter, NewAuditRecord};
use crate::repository::{AuditRepository, PostgresAuditRepository};

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

pub async fn get_audit_record(pool: &PgPool, id: Uuid) -> Result<Option<AuditRecord>, AuditError> {
    let repo = PostgresAuditRepository::new(pool.clone());
    Ok(repo.find_by_id(id).await?)
}

pub async fn list_audit_records(
    pool: &PgPool,
    filter: &AuditRecordFilter,
) -> Result<Vec<AuditRecord>, AuditError> {
    let repo = PostgresAuditRepository::new(pool.clone());
    Ok(repo.list(filter).await?)
}

pub async fn record_required(
    pool: &PgPool,
    record: NewAuditRecord,
) -> Result<AuditRecord, AuditError> {
    let repo = PostgresAuditRepository::new(pool.clone());
    Ok(repo.insert(&record).await?)
}

pub async fn record_best_effort(pool: &PgPool, record: NewAuditRecord) {
    let repo = PostgresAuditRepository::new(pool.clone());
    if let Err(e) = repo.insert(&record).await {
        tracing::error!(
            error = %e,
            action = %record.action,
            "Failed to write best-effort audit record"
        );
    }
}

pub async fn record_required_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    record: NewAuditRecord,
) -> Result<AuditRecord, AuditError> {
    let result = sqlx::query_as::<_, AuditRecord>(
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
    .fetch_one(tx.as_mut())
    .await?;
    Ok(result)
}

pub fn new_audit_record(
    actor_id: Option<Uuid>,
    actor_type: impl Into<String>,
    action: impl Into<String>,
    resource_type: impl Into<String>,
    resource_id: impl Into<String>,
    details: Option<serde_json::Value>,
) -> NewAuditRecord {
    let now = Utc::now();
    let id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
    NewAuditRecord {
        id,
        actor_id,
        actor_type: actor_type.into(),
        action: action.into(),
        resource_type: resource_type.into(),
        resource_id: resource_id.into(),
        details,
        occurred_at: now,
        created_at: now,
    }
}
