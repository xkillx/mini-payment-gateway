use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Reconciliation, ReconciliationStatus};
use crate::repository::{PostgresReconciliationRepository, ReconciliationRepository};

pub trait ReconciliationService: Send + Sync {
    async fn get_reconciliation(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error>;
    async fn list_reconciliations(&self) -> Result<Vec<Reconciliation>, sqlx::Error>;
}

pub struct DefaultReconciliationService {
    pool: PgPool,
}

impl DefaultReconciliationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ReconciliationService for DefaultReconciliationService {
    async fn get_reconciliation(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error> {
        PostgresReconciliationRepository::new(self.pool.clone())
            .find_by_id(id)
            .await
    }

    async fn list_reconciliations(&self) -> Result<Vec<Reconciliation>, sqlx::Error> {
        PostgresReconciliationRepository::new(self.pool.clone())
            .find_all()
            .await
    }
}

pub struct RunManualReconciliationCommand {
    pub actor_id: Uuid,
    pub currency: String,
    pub window_start: chrono::DateTime<chrono::Utc>,
    pub window_end: chrono::DateTime<chrono::Utc>,
    pub actual_total_minor: i64,
    pub notes: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReconciliationError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Audit error: {0}")]
    Audit(String),
}

pub async fn run_manual_reconciliation(
    pool: &PgPool,
    command: RunManualReconciliationCommand,
) -> Result<Reconciliation, ReconciliationError> {
    let now = Utc::now();
    let id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

    let expected_total_minor = match PostgresReconciliationRepository::calculate_expected_total(
        pool,
        &command.currency,
        command.window_start,
        command.window_end,
    )
    .await
    {
        Ok(total) => total,
        Err(e) => {
            return insert_error_reconciliation(
                pool,
                id,
                now,
                &command,
                format!("Expected total calculation failed: {e}"),
            )
            .await;
        }
    };

    let discrepancy_minor = command.actual_total_minor - expected_total_minor;

    let status = if discrepancy_minor == 0 {
        ReconciliationStatus::Matched
    } else {
        ReconciliationStatus::Mismatched
    };

    let reconciliation = Reconciliation {
        id,
        status,
        expected_total_minor,
        actual_total_minor: command.actual_total_minor,
        discrepancy_minor,
        currency: command.currency.clone(),
        window_start: command.window_start,
        window_end: command.window_end,
        notes: command.notes.clone(),
        run_at: now,
        created_at: now,
    };

    let details = serde_json::json!({
        "status": serde_json::to_value(&reconciliation.status).unwrap_or_default(),
        "currency": reconciliation.currency,
        "window_start": reconciliation.window_start.to_rfc3339(),
        "window_end": reconciliation.window_end.to_rfc3339(),
        "expected_total_minor": reconciliation.expected_total_minor,
        "actual_total_minor": reconciliation.actual_total_minor,
        "discrepancy_minor": reconciliation.discrepancy_minor,
    });

    let mut details_map = match details {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    if let Some(ref notes) = command.notes {
        details_map.insert(
            "notes".to_string(),
            serde_json::Value::String(notes.clone()),
        );
    }

    let audit_record = audit::service::new_audit_record(
        Some(command.actor_id),
        audit::models::ActorType::ADMINISTRATOR,
        audit::models::actions::RECONCILIATION_EXECUTED,
        "reconciliation",
        reconciliation.id.to_string(),
        Some(serde_json::Value::Object(details_map)),
    );

    let mut tx = pool.begin().await?;

    let stored = PostgresReconciliationRepository::insert_in_tx(&mut tx, &reconciliation).await?;

    audit::service::record_required_in_tx(&mut tx, audit_record)
        .await
        .map_err(|e| ReconciliationError::Audit(format!("Failed to insert audit record: {e}")))?;

    tx.commit().await?;

    Ok(stored)
}

async fn insert_error_reconciliation(
    pool: &PgPool,
    id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
    command: &RunManualReconciliationCommand,
    error_message: String,
) -> Result<Reconciliation, ReconciliationError> {
    let notes = match &command.notes {
        Some(n) => format!("Error: {}. {}", error_message, n),
        None => error_message,
    };

    let reconciliation = Reconciliation {
        id,
        status: ReconciliationStatus::Error,
        expected_total_minor: 0,
        actual_total_minor: command.actual_total_minor,
        discrepancy_minor: command.actual_total_minor,
        currency: command.currency.clone(),
        window_start: command.window_start,
        window_end: command.window_end,
        notes: Some(notes.clone()),
        run_at: now,
        created_at: now,
    };

    let details = serde_json::json!({
        "status": serde_json::to_value(&reconciliation.status).unwrap_or_default(),
        "currency": reconciliation.currency,
        "window_start": reconciliation.window_start.to_rfc3339(),
        "window_end": reconciliation.window_end.to_rfc3339(),
        "expected_total_minor": reconciliation.expected_total_minor,
        "actual_total_minor": reconciliation.actual_total_minor,
        "discrepancy_minor": reconciliation.discrepancy_minor,
        "notes": notes,
    });

    let audit_record = audit::service::new_audit_record(
        Some(command.actor_id),
        audit::models::ActorType::ADMINISTRATOR,
        audit::models::actions::RECONCILIATION_EXECUTED,
        "reconciliation",
        reconciliation.id.to_string(),
        Some(details),
    );

    let mut tx = pool.begin().await?;

    let stored = PostgresReconciliationRepository::insert_in_tx(&mut tx, &reconciliation).await?;

    audit::service::record_required_in_tx(&mut tx, audit_record)
        .await
        .map_err(|e| ReconciliationError::Audit(format!("Failed to insert audit record: {e}")))?;

    tx.commit().await?;

    Ok(stored)
}
