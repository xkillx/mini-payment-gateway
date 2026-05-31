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
}
