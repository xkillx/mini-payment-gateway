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
pub struct AdminDashboardSummaryResponse {
    pub configured_currency: String,
    pub generated_at: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub api_status: String,
    pub payment_overview: AdminPaymentOverview,
    pub refund_overview: AdminRefundOverview,
    pub notification_overview: AdminNotificationOverview,
    pub reconciliation_overview: AdminReconciliationOverview,
    pub audit_overview: AdminAuditOverview,
}

#[derive(Debug, Serialize)]
pub struct AdminPaymentOverview {
    pub status_counts: PaymentStatusCounts,
    pub recent_failed_payments: Vec<DashboardPaymentListItem>,
}

#[derive(Debug, Serialize)]
pub struct AdminRefundOverview {
    pub status_counts: RefundStatusCounts,
    pub recent_failed_refunds: Vec<DashboardRefundListItem>,
}

#[derive(Debug, Serialize)]
pub struct AdminNotificationOverview {
    pub status_counts: NotificationStatusCounts,
    pub recent_failed_notifications: Vec<AdminNotificationListItem>,
}

#[derive(Debug, Serialize)]
pub struct AdminReconciliationOverview {
    pub status_counts: ReconciliationStatusCounts,
    pub recent_attention_reconciliations: Vec<AdminReconciliationListItem>,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditOverview {
    pub recent_attention_audit_records: Vec<AdminAuditListItem>,
}

#[derive(Debug, Serialize)]
pub struct NotificationStatusCounts {
    pub pending: i64,
    pub processing: i64,
    pub delivered: i64,
    pub failed: i64,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationStatusCounts {
    pub matched: i64,
    pub mismatched: i64,
    pub error: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminNotificationListItem {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub payment_id: Uuid,
    pub destination_url: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminReconciliationListItem {
    pub id: Uuid,
    pub status: String,
    pub expected_total_minor: i64,
    pub actual_total_minor: i64,
    pub discrepancy_minor: i64,
    pub currency: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminAuditListItem {
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

pub fn default_notification_counts() -> NotificationStatusCounts {
    NotificationStatusCounts {
        pending: 0,
        processing: 0,
        delivered: 0,
        failed: 0,
    }
}

pub fn default_reconciliation_counts() -> ReconciliationStatusCounts {
    ReconciliationStatusCounts {
        matched: 0,
        mismatched: 0,
        error: 0,
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct NotificationStatusRow {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ReconciliationStatusRow {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AdminNotificationRow {
    pub id: Uuid,
    pub domain_event_id: Uuid,
    pub event_type: String,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub payment_id: Uuid,
    pub destination_url: String,
    pub status: String,
    pub attempt_count: i32,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<AdminNotificationRow> for AdminNotificationListItem {
    fn from(row: AdminNotificationRow) -> Self {
        Self {
            id: row.id,
            domain_event_id: row.domain_event_id,
            event_type: row.event_type,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            payment_id: row.payment_id,
            destination_url: row.destination_url,
            status: row.status,
            attempt_count: row.attempt_count,
            last_error: row.last_error,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct AdminReconciliationRow {
    pub id: Uuid,
    pub status: String,
    pub expected_total_minor: i64,
    pub actual_total_minor: i64,
    pub discrepancy_minor: i64,
    pub currency: String,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl From<AdminReconciliationRow> for AdminReconciliationListItem {
    fn from(row: AdminReconciliationRow) -> Self {
        Self {
            id: row.id,
            status: row.status,
            expected_total_minor: row.expected_total_minor,
            actual_total_minor: row.actual_total_minor,
            discrepancy_minor: row.discrepancy_minor,
            currency: row.currency,
            window_start: row.window_start,
            window_end: row.window_end,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct AdminAuditRow {
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

impl From<AdminAuditRow> for AdminAuditListItem {
    fn from(row: AdminAuditRow) -> Self {
        Self {
            id: row.id,
            actor_id: row.actor_id,
            actor_type: row.actor_type,
            action: row.action,
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            details: row.details,
            occurred_at: row.occurred_at,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct AdminFailedPaymentRow {
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

impl From<AdminFailedPaymentRow> for DashboardPaymentListItem {
    fn from(row: AdminFailedPaymentRow) -> Self {
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
pub struct AdminFailedRefundRow {
    pub id: Uuid,
    pub payment_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AdminFailedRefundRow> for DashboardRefundListItem {
    fn from(row: AdminFailedRefundRow) -> Self {
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
