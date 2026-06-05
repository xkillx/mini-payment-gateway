use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "notification_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
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
    pub retry_generation: i32,
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
    pub retry_generation: i32,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub payload: serde_json::Value,
    pub event_created_at: DateTime<Utc>,
    pub version: i32,
    pub attempt_id: Uuid,
    pub attempt_number: i32,
    pub generation_attempt_number: u32,
}

// ---------------------------------------------------------------------------
// Attempt History
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(
    type_name = "notification_delivery_attempt_status",
    rename_all = "lowercase"
)]
#[serde(rename_all = "lowercase")]
pub enum NotificationDeliveryAttemptStatus {
    Processing,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NotificationDeliveryAttempt {
    pub id: Uuid,
    pub notification_delivery_record_id: Uuid,
    pub retry_generation: i32,
    pub attempt_number: i32,
    pub status: NotificationDeliveryAttemptStatus,
    pub http_status_code: Option<i32>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// API Response Structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct NotificationDeliveryAttemptResponse {
    pub id: Uuid,
    pub notification_delivery_record_id: Uuid,
    pub retry_generation: i32,
    pub attempt_number: i32,
    pub status: NotificationDeliveryAttemptStatus,
    pub http_status_code: Option<i32>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<NotificationDeliveryAttempt> for NotificationDeliveryAttemptResponse {
    fn from(a: NotificationDeliveryAttempt) -> Self {
        Self {
            id: a.id,
            notification_delivery_record_id: a.notification_delivery_record_id,
            retry_generation: a.retry_generation,
            attempt_number: a.attempt_number,
            status: a.status,
            http_status_code: a.http_status_code,
            error: a.error,
            started_at: a.started_at,
            finished_at: a.finished_at,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationDeliveryRecordDetailResponse {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub destination_url: String,
    pub status: NotificationStatus,
    pub attempt_count: i32,
    pub retry_generation: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub attempts: Vec<NotificationDeliveryAttemptResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotificationListResponse {
    pub items: Vec<NotificationDeliveryRecordDetailResponse>,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Query Filters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NotificationListFilter {
    pub status: Option<NotificationStatus>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for NotificationListFilter {
    fn default() -> Self {
        Self {
            status: None,
            limit: 50,
            offset: 0,
        }
    }
}
