use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::models::{ClaimedNotificationDelivery, NotificationDeliveryRecord};
use crate::repository::NotificationRepository;

pub trait NotificationService: Send + Sync {
    async fn get_notification(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecord>, sqlx::Error>;
    async fn create_notification(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn project_domain_events(&self, limit: i64) -> Result<u64, sqlx::Error>;
}

pub struct DefaultNotificationService<R: NotificationRepository> {
    repo: R,
}

impl<R: NotificationRepository> DefaultNotificationService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: NotificationRepository + 'static> NotificationService for DefaultNotificationService<R> {
    async fn get_notification(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecord>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn create_notification(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        self.repo.insert(record).await
    }

    async fn project_domain_events(&self, limit: i64) -> Result<u64, sqlx::Error> {
        self.repo.project_domain_events(limit).await
    }
}

// ---------------------------------------------------------------------------
// Delivery payload, settings, transport contracts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct NotificationDeliveryPayload {
    pub event_id: Uuid,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub schema_version: i32,
    pub payload: serde_json::Value,
}

impl NotificationDeliveryPayload {
    pub fn from_claimed(claimed: &ClaimedNotificationDelivery) -> Self {
        Self {
            event_id: claimed.domain_event_id,
            event_type: claimed.event_type.clone(),
            occurred_at: claimed.event_created_at,
            resource_type: claimed.aggregate_type.clone(),
            resource_id: claimed.aggregate_id,
            schema_version: claimed.version,
            payload: claimed.payload.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NotificationDeliverySettings {
    pub max_attempts: u32,
    pub retry_delays_secs: Vec<u64>,
    pub stale_processing_after: Duration,
}

impl NotificationDeliverySettings {
    pub fn new(max_attempts: u32, retry_delays_secs: Vec<u64>) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            retry_delays_secs,
            stale_processing_after: Duration::minutes(5),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeliveryStatus {
    Delivered,
    PendingRetry,
    TerminalFailed,
}

#[derive(Debug, Clone)]
pub struct NotificationDeliveryOutcome {
    pub record_id: Uuid,
    pub destination_url: String,
    pub event_type: String,
    pub attempt_number: u32,
    pub status: DeliveryStatus,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationDeliveryServiceError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("No claim available")]
    NoClaimAvailable,
}

impl From<sqlx::Error> for NotificationDeliveryServiceError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

fn build_last_error(http_status: Option<u16>, error_message: Option<&str>) -> String {
    let raw = if let Some(code) = http_status {
        format!("http_status:{}", code)
    } else if let Some(msg) = error_message {
        if msg.contains("timeout") || msg.contains("timed out") {
            "timeout".to_string()
        } else {
            format!("transport:{}", msg)
        }
    } else {
        "transport:unknown error".to_string()
    };
    truncate_str(&raw, 500)
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len).collect()
    }
}

pub trait NotificationTransport: Send + Sync {
    fn deliver(
        &self,
        destination_url: &str,
        payload: &NotificationDeliveryPayload,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send + '_>>;
}

pub struct ReqwestNotificationTransport {
    client: reqwest::Client,
}

impl ReqwestNotificationTransport {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("ReqwestNotificationTransport client build should succeed");
        Self { client }
    }
}

impl Default for ReqwestNotificationTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationTransport for ReqwestNotificationTransport {
    fn deliver(
        &self,
        destination_url: &str,
        payload: &NotificationDeliveryPayload,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send + '_>> {
        let client = self.client.clone();
        let url = destination_url.to_string();
        let body = serde_json::to_vec(payload)
            .expect("NotificationDeliveryPayload serialization should succeed");

        Box::pin(async move {
            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            Ok(response.status().as_u16())
        })
    }
}

// ---------------------------------------------------------------------------
// Delivery service: process_next_due_delivery
// ---------------------------------------------------------------------------

impl<R: NotificationRepository + 'static> DefaultNotificationService<R> {
    pub async fn process_next_due_delivery<T: NotificationTransport>(
        &self,
        transport: &T,
        settings: &NotificationDeliverySettings,
    ) -> Result<Option<NotificationDeliveryOutcome>, NotificationDeliveryServiceError> {
        let stale_before = Utc::now() - settings.stale_processing_after;

        let claimed = self.repo.claim_due_for_delivery(stale_before).await?;
        let Some(claimed) = claimed else {
            return Ok(None);
        };

        let payload = NotificationDeliveryPayload::from_claimed(&claimed);

        let delivery_result = transport.deliver(&claimed.destination_url, &payload).await;

        let new_attempt = (claimed.attempt_count + 1) as u32;
        let is_success = delivery_result
            .as_ref()
            .map(|status| (200..300).contains(status))
            .unwrap_or(false);

        if is_success {
            let _record = self.repo.mark_delivery_delivered(claimed.id).await?;
            return Ok(Some(NotificationDeliveryOutcome {
                record_id: claimed.id,
                destination_url: claimed.destination_url,
                event_type: claimed.event_type,
                attempt_number: new_attempt,
                status: DeliveryStatus::Delivered,
                next_retry_at: None,
                last_error: None,
            }));
        }

        let last_error = match &delivery_result {
            Ok(status) => build_last_error(Some(*status), None),
            Err(msg) => build_last_error(None, Some(msg)),
        };

        if new_attempt >= settings.max_attempts {
            let _record = self
                .repo
                .mark_delivery_failed(claimed.id, &last_error)
                .await?;
            return Ok(Some(NotificationDeliveryOutcome {
                record_id: claimed.id,
                destination_url: claimed.destination_url,
                event_type: claimed.event_type,
                attempt_number: new_attempt,
                status: DeliveryStatus::TerminalFailed,
                next_retry_at: None,
                last_error: Some(last_error),
            }));
        }

        let delay_secs = settings
            .retry_delays_secs
            .get((new_attempt - 1) as usize)
            .copied()
            .unwrap_or(3600);
        let next_retry_at = Utc::now() + Duration::seconds(delay_secs as i64);

        let _record = self
            .repo
            .mark_delivery_pending_retry(claimed.id, next_retry_at, &last_error)
            .await?;

        Ok(Some(NotificationDeliveryOutcome {
            record_id: claimed.id,
            destination_url: claimed.destination_url,
            event_type: claimed.event_type,
            attempt_number: new_attempt,
            status: DeliveryStatus::PendingRetry,
            next_retry_at: Some(next_retry_at),
            last_error: Some(last_error),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn payload_serialization_shape() {
        let payload = NotificationDeliveryPayload {
            event_id: Uuid::nil(),
            event_type: "payment.created".into(),
            occurred_at: Utc::now(),
            resource_type: "payment".into(),
            resource_id: Uuid::nil(),
            schema_version: 1,
            payload: json!({"payment_id": "test"}),
        };
        let value = serde_json::to_value(&payload).unwrap();
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["event_type"], "payment.created");
        assert_eq!(value["resource_type"], "payment");
        assert_eq!(value["schema_version"], 1);
        assert!(value["occurred_at"].as_str().is_some());
        assert!(value["payload"].is_object());
    }

    #[test]
    fn payload_from_claimed_maps_fields() {
        let claimed = ClaimedNotificationDelivery {
            id: Uuid::nil(),
            domain_event_id: Uuid::parse_str("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d").unwrap(),
            destination_url: "https://example.com/webhook".into(),
            attempt_count: 0,
            event_type: "refund.created".into(),
            aggregate_type: "refund".into(),
            aggregate_id: Uuid::parse_str("b2c3d4e5-f6a5-4b9c-8d0e-1f2a3b4c5d6e").unwrap(),
            payload: json!({"refund_id": "123"}),
            event_created_at: Utc::now(),
            version: 1,
        };
        let payload = NotificationDeliveryPayload::from_claimed(&claimed);
        assert_eq!(payload.event_type, "refund.created");
        assert_eq!(payload.resource_type, "refund");
        assert_eq!(payload.schema_version, 1);
        assert_eq!(payload.payload, json!({"refund_id": "123"}));
    }

    #[test]
    fn build_last_error_http_status() {
        let result = build_last_error(Some(500), None);
        assert_eq!(result, "http_status:500");
    }

    #[test]
    fn build_last_error_timeout() {
        let result = build_last_error(None, Some("request timed out"));
        assert_eq!(result, "timeout");
    }

    #[test]
    fn build_last_error_transport() {
        let result = build_last_error(None, Some("connection refused"));
        assert_eq!(result, "transport:connection refused");
    }

    #[test]
    fn truncate_long_errors() {
        let long = "x".repeat(600);
        let msg = format!("transport:{}", long);
        let result = build_last_error(None, Some(&msg));
        assert!(result.len() <= 500);
        assert!(result.starts_with("transport:"));
    }

    #[test]
    fn settings_normalizes_zero_max_attempts_to_one() {
        let settings = NotificationDeliverySettings::new(0, vec![30]);
        assert_eq!(settings.max_attempts, 1);
    }

    #[test]
    fn settings_preserves_valid_max_attempts() {
        let settings = NotificationDeliverySettings::new(5, vec![30, 60]);
        assert_eq!(settings.max_attempts, 5);
    }
}
