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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationListResponse {
    pub items: Vec<Reconciliation>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationReportResponse {
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
    pub included_payment_total_minor: i64,
    pub included_refund_total_minor: i64,
    pub included_record_count: i64,
    pub included_payments: Vec<IncludedPaymentRecord>,
    pub included_refunds: Vec<IncludedRefundRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IncludedPaymentRecord {
    pub domain_event_id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IncludedRefundRecord {
    pub domain_event_id: Uuid,
    pub refund_id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub occurred_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_status_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&ReconciliationStatus::Matched).unwrap(),
            r#""matched""#
        );
        assert_eq!(
            serde_json::to_string(&ReconciliationStatus::Mismatched).unwrap(),
            r#""mismatched""#
        );
        assert_eq!(
            serde_json::to_string(&ReconciliationStatus::Error).unwrap(),
            r#""error""#
        );
    }

    #[test]
    fn reconciliation_status_deserializes_lowercase() {
        assert_eq!(
            serde_json::from_str::<ReconciliationStatus>(r#""matched""#).unwrap(),
            ReconciliationStatus::Matched
        );
        assert_eq!(
            serde_json::from_str::<ReconciliationStatus>(r#""mismatched""#).unwrap(),
            ReconciliationStatus::Mismatched
        );
        assert_eq!(
            serde_json::from_str::<ReconciliationStatus>(r#""error""#).unwrap(),
            ReconciliationStatus::Error
        );
    }

    #[test]
    fn reconciliation_list_response_serialization() {
        let resp = ReconciliationListResponse {
            items: vec![],
            limit: 50,
            offset: 0,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["limit"], 50);
        assert_eq!(json["offset"], 0);
        assert!(json["items"].is_array());
    }

    #[test]
    fn included_payment_record_serialization() {
        let record = IncludedPaymentRecord {
            domain_event_id: Uuid::nil(),
            payment_id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 1000,
            currency: "USD".into(),
            occurred_at: chrono::DateTime::parse_from_rfc3339("2100-01-01T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            metadata: serde_json::json!({"merchant_reference": "ORD-123"}),
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["amount_minor"], 1000);
        assert_eq!(json["currency"], "USD");
        assert_eq!(json["metadata"]["merchant_reference"], "ORD-123");
    }

    #[test]
    fn included_refund_record_serialization() {
        let record = IncludedRefundRecord {
            domain_event_id: Uuid::nil(),
            refund_id: Uuid::nil(),
            payment_id: Uuid::nil(),
            merchant_id: Uuid::nil(),
            amount_minor: 500,
            currency: "USD".into(),
            occurred_at: chrono::DateTime::parse_from_rfc3339("2100-01-01T12:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let json = serde_json::to_value(&record).unwrap();
        assert_eq!(json["amount_minor"], 500);
        assert_eq!(json["currency"], "USD");
    }

    #[test]
    fn reconciliation_report_response_serialization() {
        let window_start = chrono::DateTime::parse_from_rfc3339("2100-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let window_end = chrono::DateTime::parse_from_rfc3339("2100-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let resp = ReconciliationReportResponse {
            id: Uuid::nil(),
            status: ReconciliationStatus::Matched,
            expected_total_minor: 1000,
            actual_total_minor: 1000,
            discrepancy_minor: 0,
            currency: "USD".into(),
            window_start,
            window_end,
            notes: None,
            run_at: window_end,
            created_at: window_end,
            included_payment_total_minor: 1000,
            included_refund_total_minor: 0,
            included_record_count: 1,
            included_payments: vec![],
            included_refunds: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["status"], "matched");
        assert_eq!(json["expected_total_minor"], 1000);
        assert_eq!(json["included_payment_total_minor"], 1000);
        assert_eq!(json["included_record_count"], 1);
    }
}
