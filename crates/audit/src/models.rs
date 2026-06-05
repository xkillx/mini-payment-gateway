use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditRecord {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_type: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct NewAuditRecord {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_type: String,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub details: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub struct AuditRecordFilter {
    pub actor_id: Option<Uuid>,
    pub actor_type: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub action: Option<String>,
    pub occurred_from: Option<DateTime<Utc>>,
    pub occurred_to: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

pub struct ActorType;

impl ActorType {
    pub const MERCHANT: &'static str = "merchant";
    pub const ADMINISTRATOR: &'static str = "administrator";
    pub const SYSTEM: &'static str = "system";
    pub const UNKNOWN: &'static str = "unknown";
}

pub mod actions {
    pub const AUTH_AUTHENTICATION_FAILED: &str = "auth.authentication_failed";
    pub const AUTH_AUTHORIZATION_FAILED: &str = "auth.authorization_failed";
    pub const PAYMENT_CREATED: &str = "payment.created";
    pub const PAYMENT_PROCESSING: &str = "payment.processing";
    pub const PAYMENT_SUCCESSFUL: &str = "payment.successful";
    pub const PAYMENT_FAILED: &str = "payment.failed";
    pub const REFUND_REQUESTED: &str = "refund.requested";
    pub const REFUND_CREATED: &str = "refund.created";
    pub const REFUND_REJECTED: &str = "refund.rejected";
    pub const REFUND_COMPLETED: &str = "refund.completed";
    pub const RECONCILIATION_EXECUTED: &str = "reconciliation.executed";
    pub const NOTIFICATION_RETRY_REQUESTED: &str = "notification.retry_requested";
}
