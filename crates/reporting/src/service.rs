use sqlx::PgPool;
use uuid::Uuid;

use crate::models::PaymentSummaryReport;

pub trait ReportingService: Send + Sync {
    async fn payment_summary(&self, merchant_id: Uuid)
        -> Result<PaymentSummaryReport, sqlx::Error>;
}

pub struct DefaultReportingService {
    pool: PgPool,
}

impl DefaultReportingService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ReportingService for DefaultReportingService {
    async fn payment_summary(
        &self,
        merchant_id: Uuid,
    ) -> Result<PaymentSummaryReport, sqlx::Error> {
        sqlx::query_as::<_, PaymentSummaryReport>(
            r#"
            SELECT
                COUNT(*) as total_count,
                COALESCE(SUM(amount_minor), 0) as total_amount_minor,
                'USD' as currency
            FROM payments
            WHERE merchant_id = $1
            "#,
        )
        .bind(merchant_id)
        .fetch_one(&self.pool)
        .await
    }
}
