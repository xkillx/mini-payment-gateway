use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Successful,
    Failed,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub idempotency_key: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Payment> for PaymentResponse {
    fn from(p: Payment) -> Self {
        Self {
            id: p.id,
            amount_minor: p.amount_minor,
            currency: p.currency,
            status: p.status,
            metadata: p.metadata,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentDetailResponse {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status_history: Vec<PaymentStatusHistoryEntry>,
    pub refunds: Vec<PaymentRefundSummary>,
    pub notification_delivery_records: Vec<PaymentNotificationDeliveryRecordResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentStatusHistoryEntry {
    pub status: PaymentStatus,
    pub source_event_type: String,
    pub domain_event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentRefundSummary {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentNotificationDeliveryRecordResponse {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub event_type: String,
    pub destination_url: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentListItemResponse {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Payment> for PaymentListItemResponse {
    fn from(p: Payment) -> Self {
        Self {
            id: p.id,
            merchant_id: p.merchant_id,
            amount_minor: p.amount_minor,
            currency: p.currency,
            status: p.status,
            metadata: p.metadata,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentListResponse {
    pub items: Vec<PaymentListItemResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct PaymentListFilter {
    pub merchant_id: Option<Uuid>,
    pub status: Option<PaymentStatus>,
    pub search: Option<String>,
    pub search_id: Option<Uuid>,
    pub limit: i64,
    pub offset: i64,
}

pub fn parse_payment_status(value: &str) -> Option<PaymentStatus> {
    match value {
        "pending" => Some(PaymentStatus::Pending),
        "processing" => Some(PaymentStatus::Processing),
        "successful" => Some(PaymentStatus::Successful),
        "failed" => Some(PaymentStatus::Failed),
        "refunded" => Some(PaymentStatus::Refunded),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_value(PaymentStatus::Pending).unwrap(),
            "pending"
        );
        assert_eq!(
            serde_json::to_value(PaymentStatus::Processing).unwrap(),
            "processing"
        );
        assert_eq!(
            serde_json::to_value(PaymentStatus::Successful).unwrap(),
            "successful"
        );
        assert_eq!(
            serde_json::to_value(PaymentStatus::Failed).unwrap(),
            "failed"
        );
        assert_eq!(
            serde_json::to_value(PaymentStatus::Refunded).unwrap(),
            "refunded"
        );
    }

    #[test]
    fn payment_status_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<PaymentStatus>("\"pending\"").unwrap(),
            PaymentStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<PaymentStatus>("\"processing\"").unwrap(),
            PaymentStatus::Processing
        );
        assert_eq!(
            serde_json::from_str::<PaymentStatus>("\"successful\"").unwrap(),
            PaymentStatus::Successful
        );
        assert_eq!(
            serde_json::from_str::<PaymentStatus>("\"failed\"").unwrap(),
            PaymentStatus::Failed
        );
        assert_eq!(
            serde_json::from_str::<PaymentStatus>("\"refunded\"").unwrap(),
            PaymentStatus::Refunded
        );
    }

    #[test]
    fn payment_response_excludes_sensitive_fields() {
        let payment = Payment {
            id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            status: PaymentStatus::Pending,
            idempotency_key: "test-key".into(),
            metadata: serde_json::json!({"foo": "bar"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response: PaymentResponse = payment.into();
        let value = serde_json::to_value(response).unwrap();
        assert!(value.get("merchant_id").is_none());
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["metadata"]["foo"], "bar");
    }

    #[test]
    fn metadata_jsonb_equality_key_order_independent() {
        let a = serde_json::json!({"a": 1, "b": 2});
        let b = serde_json::json!({"b": 2, "a": 1});
        // serde_json::Value equality for objects is order-independent
        assert_eq!(a, b);
    }

    #[test]
    fn payment_detail_response_includes_merchant_id_and_excludes_idempotency_key() {
        let detail = PaymentDetailResponse {
            id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            status: PaymentStatus::Pending,
            metadata: serde_json::json!({"foo": "bar"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status_history: vec![],
            refunds: vec![],
            notification_delivery_records: vec![],
        };
        let value = serde_json::to_value(detail).unwrap();
        assert!(value.get("merchant_id").is_some());
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["status_history"], serde_json::json!([]));
        assert_eq!(value["refunds"], serde_json::json!([]));
        assert_eq!(
            value["notification_delivery_records"],
            serde_json::json!([])
        );
    }

    #[test]
    fn payment_status_history_entry_omits_details_when_none() {
        let entry = PaymentStatusHistoryEntry {
            status: PaymentStatus::Pending,
            source_event_type: "payment.created".into(),
            domain_event_id: Uuid::nil(),
            occurred_at: Utc::now(),
            details: None,
        };
        let value = serde_json::to_value(entry).unwrap();
        assert!(value.get("details").is_none());
    }

    #[test]
    fn payment_status_history_entry_includes_details_when_some() {
        let entry = PaymentStatusHistoryEntry {
            status: PaymentStatus::Failed,
            source_event_type: "payment.failed".into(),
            domain_event_id: Uuid::nil(),
            occurred_at: Utc::now(),
            details: Some(serde_json::json!({"failure_reason": "insufficient_funds"})),
        };
        let value = serde_json::to_value(entry).unwrap();
        assert_eq!(value["details"]["failure_reason"], "insufficient_funds");
    }

    #[test]
    fn payment_refund_summary_excludes_idempotency_key() {
        let summary = PaymentRefundSummary {
            id: Uuid::nil(),
            payment_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            status: "completed".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let value = serde_json::to_value(summary).unwrap();
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["status"], "completed");
    }

    #[test]
    fn payment_notification_delivery_record_response_excludes_idempotency_key() {
        let record = PaymentNotificationDeliveryRecordResponse {
            id: Uuid::nil(),
            domain_event_id: Uuid::nil(),
            event_type: "payment.created".into(),
            destination_url: "https://example.com/webhook".into(),
            status: "pending".into(),
            attempt_count: 0,
            last_attempt_at: None,
            next_retry_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let value = serde_json::to_value(record).unwrap();
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["event_type"], "payment.created");
    }

    #[test]
    fn payment_list_item_response_includes_merchant_id_and_excludes_idempotency_key() {
        let payment = Payment {
            id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            status: PaymentStatus::Pending,
            idempotency_key: "test-key".into(),
            metadata: serde_json::json!({"merchant_reference": "ORD-123"}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let item: PaymentListItemResponse = payment.into();
        let value = serde_json::to_value(item).unwrap();
        assert!(value.get("merchant_id").is_some());
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["amount_minor"], 1000);
        assert_eq!(value["status"], "pending");
        assert_eq!(value["metadata"]["merchant_reference"], "ORD-123");
    }

    #[test]
    fn payment_list_response_serializes_items_limit_offset() {
        let response = PaymentListResponse {
            items: vec![],
            limit: 50,
            offset: 0,
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value["items"], serde_json::json!([]));
        assert_eq!(value["limit"], 50);
        assert_eq!(value["offset"], 0);
    }

    #[test]
    fn parse_payment_status_matches_lowercase_values() {
        assert_eq!(
            parse_payment_status("pending"),
            Some(PaymentStatus::Pending)
        );
        assert_eq!(
            parse_payment_status("processing"),
            Some(PaymentStatus::Processing)
        );
        assert_eq!(
            parse_payment_status("successful"),
            Some(PaymentStatus::Successful)
        );
        assert_eq!(parse_payment_status("failed"), Some(PaymentStatus::Failed));
        assert_eq!(
            parse_payment_status("refunded"),
            Some(PaymentStatus::Refunded)
        );
        assert_eq!(parse_payment_status("PENDING"), None);
        assert_eq!(parse_payment_status(""), None);
        assert_eq!(parse_payment_status("paid"), None);
    }
}
