use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "reconciliation_status", rename_all = "lowercase")]
pub enum ReconciliationStatus {
    Matched,
    Mismatched,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reconciliation {
    pub id: Uuid,
    pub status: ReconciliationStatus,
    pub expected_total_minor: i64,
    pub actual_total_minor: i64,
    pub currency: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}
