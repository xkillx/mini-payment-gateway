use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentSummaryReport {
    pub total_count: i64,
    pub total_amount_minor: i64,
    pub currency: String,
}
