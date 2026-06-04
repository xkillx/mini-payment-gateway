use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "notification_status", rename_all = "lowercase")]
pub enum NotificationStatus {
    Pending,
    Processing,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationDeliveryRecord {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub destination_url: String,
    pub status: NotificationStatus,
    pub attempt_count: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationDestination {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub destination_url: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ClaimedNotificationDelivery {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub destination_url: String,
    pub attempt_count: i32,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub payload: serde_json::Value,
    pub event_created_at: DateTime<Utc>,
    pub version: i32,
}
