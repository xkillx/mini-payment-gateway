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

#[derive(Debug, Serialize)]
pub struct RefundReadResponse {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: RefundStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Refund> for RefundReadResponse {
    fn from(r: Refund) -> Self {
        Self {
            id: r.id,
            payment_id: r.payment_id,
            merchant_id: r.merchant_id,
            amount_minor: r.amount_minor,
            currency: r.currency,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RefundListResponse {
    pub items: Vec<RefundReadResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone)]
pub struct RefundListFilter {
    pub merchant_id: Option<Uuid>,
    pub status: Option<RefundStatus>,
    pub limit: i64,
    pub offset: i64,
}

pub fn parse_refund_status(value: &str) -> Option<RefundStatus> {
    match value {
        "pending" => Some(RefundStatus::Pending),
        "processing" => Some(RefundStatus::Processing),
        "completed" => Some(RefundStatus::Completed),
        "failed" => Some(RefundStatus::Failed),
        _ => None,
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

    #[test]
    fn refund_read_response_includes_merchant_id_and_excludes_idempotency_key() {
        let refund = Refund {
            id: Uuid::nil(),
            payment_id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            status: RefundStatus::Pending,
            idempotency_key: "test-key".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let response = RefundReadResponse::from(refund);
        let value = serde_json::to_value(response).unwrap();
        assert!(value.get("merchant_id").is_some());
        assert!(value.get("idempotency_key").is_none());
        assert_eq!(value["amount_minor"], 1000);
        assert_eq!(value["status"], "pending");
    }

    #[test]
    fn refund_list_response_serializes_items_limit_offset() {
        let response = RefundListResponse {
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
    fn parse_refund_status_matches_lowercase_values() {
        assert_eq!(parse_refund_status("pending"), Some(RefundStatus::Pending));
        assert_eq!(
            parse_refund_status("processing"),
            Some(RefundStatus::Processing)
        );
        assert_eq!(
            parse_refund_status("completed"),
            Some(RefundStatus::Completed)
        );
        assert_eq!(parse_refund_status("failed"), Some(RefundStatus::Failed));
        assert_eq!(parse_refund_status("PENDING"), None);
        assert_eq!(parse_refund_status(""), None);
        assert_eq!(parse_refund_status("unknown"), None);
    }

    #[test]
    fn refund_status_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<RefundStatus>("\"pending\"").unwrap(),
            RefundStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<RefundStatus>("\"processing\"").unwrap(),
            RefundStatus::Processing
        );
        assert_eq!(
            serde_json::from_str::<RefundStatus>("\"completed\"").unwrap(),
            RefundStatus::Completed
        );
        assert_eq!(
            serde_json::from_str::<RefundStatus>("\"failed\"").unwrap(),
            RefundStatus::Failed
        );
    }
}
