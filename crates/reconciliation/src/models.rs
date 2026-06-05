use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "reconciliation_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
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
    pub discrepancy_minor: i64,
    pub currency: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub notes: Option<String>,
    pub run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunReconciliationRequest {
    pub currency: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub actual_total_minor: i64,
    pub notes: Option<String>,
}
