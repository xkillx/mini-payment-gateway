use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PaymentSummaryReport {
    pub configured_currency: String,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub payment_totals: PaymentReportTotals,
    pub refund_activity: RefundReportActivity,
    pub trend: Vec<PaymentTrendBucket>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentReportTotals {
    pub created_count: i64,
    pub created_amount_minor: i64,
    pub successful_count: i64,
    pub successful_amount_minor: i64,
    pub failed_count: i64,
    pub failed_attempted_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefundReportActivity {
    pub completed_count: i64,
    pub completed_amount_minor: i64,
    pub failed_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentTrendBucket {
    pub bucket_date: NaiveDate,
    pub created_count: i64,
    pub successful_count: i64,
    pub failed_count: i64,
    pub successful_amount_minor: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct CountAmountRow {
    pub count: Option<i64>,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct SimpleCountRow {
    pub count: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct TrendRow {
    pub bucket_date: NaiveDate,
    pub count: Option<i64>,
    pub amount: Option<i64>,
}
