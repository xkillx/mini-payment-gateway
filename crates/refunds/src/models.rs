use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "refund_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Refund {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: RefundStatus,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRefundRequest {
    pub payment_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RefundResponse {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: RefundStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Refund> for RefundResponse {
    fn from(r: Refund) -> Self {
        Self {
            id: r.id,
            payment_id: r.payment_id,
            amount_minor: r.amount_minor,
            currency: r.currency,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refund_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_value(RefundStatus::Pending).unwrap(),
            "pending"
        );
        assert_eq!(
            serde_json::to_value(RefundStatus::Processing).unwrap(),
            "processing"
        );
        assert_eq!(
            serde_json::to_value(RefundStatus::Completed).unwrap(),
            "completed"
        );
        assert_eq!(
            serde_json::to_value(RefundStatus::Failed).unwrap(),
            "failed"
        );
    }

    #[test]
    fn refund_response_excludes_sensitive_fields() {
        let refund = Refund {
            id: Uuid::nil(),
            payment_id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 4000,
            currency: "USD".into(),
            status: RefundStatus::Pending,
            idempotency_key: "test-key".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = RefundResponse::from(refund);
        let value = serde_json::to_value(response).unwrap();
        assert!(value.get("merchant_id").is_none());
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["status"], "pending");
        assert_eq!(value["amount_minor"], 4000);
        assert_eq!(value["currency"], "USD");
    }
}
