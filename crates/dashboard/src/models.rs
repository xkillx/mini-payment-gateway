use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct MerchantDashboardSummaryResponse {
    pub configured_currency: String,
    pub generated_at: DateTime<Utc>,
    pub payment_overview: MerchantPaymentOverview,
    pub refund_overview: MerchantRefundOverview,
}

#[derive(Debug, Serialize)]
pub struct MerchantPaymentOverview {
    pub status_counts: PaymentStatusCounts,
    pub recent_payments: Vec<DashboardPaymentListItem>,
}

#[derive(Debug, Serialize)]
pub struct MerchantRefundOverview {
    pub status_counts: RefundStatusCounts,
    pub recent_refunds: Vec<DashboardRefundListItem>,
}

#[derive(Debug, Serialize)]
pub struct PaymentStatusCounts {
    pub pending: i64,
    pub processing: i64,
    pub successful: i64,
    pub failed: i64,
    pub refunded: i64,
}

#[derive(Debug, Serialize)]
pub struct RefundStatusCounts {
    pub pending: i64,
    pub processing: i64,
    pub completed: i64,
    pub failed: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardPaymentListItem {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DashboardRefundListItem {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PaymentStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct RefundStatusRow {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct DashboardPaymentRow {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub metadata: serde_json::Value,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DashboardPaymentRow> for DashboardPaymentListItem {
    fn from(row: DashboardPaymentRow) -> Self {
        Self {
            id: row.id,
            merchant_id: row.merchant_id,
            amount_minor: row.amount_minor,
            currency: row.currency,
            status: row.status,
            metadata: row.metadata,
            failure_reason: row.failure_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct DashboardRefundRow {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DashboardRefundRow> for DashboardRefundListItem {
    fn from(row: DashboardRefundRow) -> Self {
        Self {
            id: row.id,
            payment_id: row.payment_id,
            merchant_id: row.merchant_id,
            amount_minor: row.amount_minor,
            currency: row.currency,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

pub fn default_payment_counts() -> PaymentStatusCounts {
    PaymentStatusCounts {
        pending: 0,
        processing: 0,
        successful: 0,
        failed: 0,
        refunded: 0,
    }
}

pub fn default_refund_counts() -> RefundStatusCounts {
    RefundStatusCounts {
        pending: 0,
        processing: 0,
        completed: 0,
        failed: 0,
    }
}
