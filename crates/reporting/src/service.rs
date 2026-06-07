use chrono::{DateTime, TimeDelta, Utc};
use sqlx::PgPool;

use crate::models::{
    PaymentReportTotals, PaymentSummaryReport, PaymentTrendBucket, RefundReportActivity,
};
use crate::repository;

#[derive(Debug)]
pub struct PaymentReportingPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
pub enum PaymentReportingPeriodError {
    InvalidRange,
    PeriodTooLong,
}

pub fn resolve_period(
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<PaymentReportingPeriod, PaymentReportingPeriodError> {
    let end = to.unwrap_or(now);
    let start = match from {
        Some(f) => f,
        None => end - TimeDelta::days(30),
    };

    if start >= end {
        return Err(PaymentReportingPeriodError::InvalidRange);
    }

    if (end - start) > TimeDelta::days(366) {
        return Err(PaymentReportingPeriodError::PeriodTooLong);
    }

    Ok(PaymentReportingPeriod { start, end })
}

pub async fn payment_summary(
    pool: &PgPool,
    configured_currency: &str,
    period: PaymentReportingPeriod,
) -> Result<PaymentSummaryReport, sqlx::Error> {
    let created = repository::fetch_created_payment_totals(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let successful = repository::fetch_successful_payment_totals(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let failed = repository::fetch_failed_payment_totals(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let completed_refund = repository::fetch_completed_refund_totals(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let failed_refund = repository::fetch_failed_refund_count(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let created_trend = repository::fetch_created_trend(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let successful_trend = repository::fetch_successful_trend(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let failed_trend = repository::fetch_failed_trend(
        pool,
        configured_currency,
        period.start,
        period.end,
    )
    .await?;

    let trend = build_trend_buckets(period.start, period.end, created_trend, successful_trend, failed_trend);

    Ok(PaymentSummaryReport {
        configured_currency: configured_currency.to_string(),
        generated_at: Utc::now(),
        period_start: period.start,
        period_end: period.end,
        payment_totals: PaymentReportTotals {
            created_count: created.count.unwrap_or(0),
            created_amount_minor: created.amount.unwrap_or(0),
            successful_count: successful.count.unwrap_or(0),
            successful_amount_minor: successful.amount.unwrap_or(0),
            failed_count: failed.count.unwrap_or(0),
            failed_attempted_amount_minor: failed.amount.unwrap_or(0),
        },
        refund_activity: RefundReportActivity {
            completed_count: completed_refund.count.unwrap_or(0),
            completed_amount_minor: completed_refund.amount.unwrap_or(0),
            failed_count: failed_refund.count.unwrap_or(0),
        },
        trend,
    })
}

fn build_trend_buckets(
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
    created: Vec<crate::models::TrendRow>,
    successful: Vec<crate::models::TrendRow>,
    failed: Vec<crate::models::TrendRow>,
) -> Vec<PaymentTrendBucket> {
    let mut buckets: Vec<PaymentTrendBucket> = Vec::new();

    let last_date = (period_end - chrono::TimeDelta::microseconds(1)).date_naive();
    let mut current = period_start.date_naive();

    while current <= last_date {
        let created_count = created
            .iter()
            .find(|r| r.bucket_date == current)
            .and_then(|r| r.count)
            .unwrap_or(0);

        let successful_row = successful.iter().find(|r| r.bucket_date == current);

        let successful_count = successful_row.and_then(|r| r.count).unwrap_or(0);
        let successful_amount = successful_row.and_then(|r| r.amount).unwrap_or(0);

        let failed_count = failed
            .iter()
            .find(|r| r.bucket_date == current)
            .and_then(|r| r.count)
            .unwrap_or(0);

        buckets.push(PaymentTrendBucket {
            bucket_date: current,
            created_count,
            successful_count,
            failed_count,
            successful_amount_minor: successful_amount,
        });

        current = current
            .succ_opt()
            .unwrap_or(current);
    }

    buckets
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn resolve_period_both_missing_defaults_to_30_days() {
        let now = dt("2026-06-07T12:00:00Z");
        let period = resolve_period(None, None, now).unwrap();
        assert_eq!(period.start, dt("2026-05-08T12:00:00Z"));
        assert_eq!(period.end, now);
    }

    #[test]
    fn resolve_period_only_from_uses_now_as_end() {
        let now = dt("2026-06-07T12:00:00Z");
        let from = dt("2026-06-01T00:00:00Z");
        let period = resolve_period(Some(from), None, now).unwrap();
        assert_eq!(period.start, from);
        assert_eq!(period.end, now);
    }

    #[test]
    fn resolve_period_only_to_uses_to_minus_30_days_as_start() {
        let now = dt("2026-06-07T12:00:00Z");
        let to = dt("2026-06-07T00:00:00Z");
        let period = resolve_period(None, Some(to), now).unwrap();
        assert_eq!(period.start, dt("2026-05-08T00:00:00Z"));
        assert_eq!(period.end, to);
    }

    #[test]
    fn resolve_period_rejects_start_equal_to_end() {
        let now = dt("2026-06-07T12:00:00Z");
        let result = resolve_period(Some(now), Some(now), now);
        assert_eq!(result.unwrap_err(), PaymentReportingPeriodError::InvalidRange);
    }

    #[test]
    fn resolve_period_rejects_start_after_end() {
        let now = dt("2026-06-07T12:00:00Z");
        let from = dt("2026-06-08T00:00:00Z");
        let to = dt("2026-06-07T00:00:00Z");
        let result = resolve_period(Some(from), Some(to), now);
        assert_eq!(result.unwrap_err(), PaymentReportingPeriodError::InvalidRange);
    }

    #[test]
    fn resolve_period_rejects_span_greater_than_366_days() {
        let now = dt("2026-06-07T12:00:00Z");
        let from = dt("2025-06-01T00:00:00Z");
        let to = dt("2026-06-07T00:00:00Z");
        let result = resolve_period(Some(from), Some(to), now);
        assert_eq!(result.unwrap_err(), PaymentReportingPeriodError::PeriodTooLong);
    }

    #[test]
    fn build_trend_buckets_zero_fills_missing_dates() {
        let start = dt("2026-06-01T00:00:00Z");
        let end = dt("2026-06-04T00:00:00Z");
        let created: Vec<crate::models::TrendRow> = vec![];
        let successful: Vec<crate::models::TrendRow> = vec![];
        let failed: Vec<crate::models::TrendRow> = vec![];

        let buckets = build_trend_buckets(start, end, created, successful, failed);

        assert_eq!(buckets.len(), 3);
        assert_eq!(buckets[0].bucket_date.to_string(), "2026-06-01");
        assert_eq!(buckets[0].created_count, 0);
        assert_eq!(buckets[1].bucket_date.to_string(), "2026-06-02");
        assert_eq!(buckets[2].bucket_date.to_string(), "2026-06-03");
    }
}
