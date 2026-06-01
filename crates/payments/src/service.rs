use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    Payment, PaymentDetailResponse, PaymentNotificationDeliveryRecordResponse,
    PaymentRefundSummary, PaymentStatus, PaymentStatusHistoryEntry,
};
use crate::repository;
use audit::models::{actions, ActorType, NewAuditRecord};
use audit::service::record_required_in_tx;
use audit::service::AuditError;

#[derive(Debug)]
pub struct CreatePaymentCommand {
    pub actor_id: Uuid,
    pub merchant_id: Uuid,
    pub amount_minor: i64,
    pub currency: String,
    pub metadata: serde_json::Value,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub enum CreatePaymentOutcome {
    Created(Payment),
    Replayed(Payment),
}

#[derive(Debug, thiserror::Error)]
pub enum CreatePaymentError {
    #[error("Idempotency conflict: {0}")]
    Conflict(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Audit error: {0}")]
    Audit(#[from] AuditError),
}

impl From<sqlx::Error> for CreatePaymentError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub async fn create_payment(
    pool: &PgPool,
    cmd: CreatePaymentCommand,
) -> Result<CreatePaymentOutcome, CreatePaymentError> {
    let mut tx = pool.begin().await?;

    let now = Utc::now();
    let payment_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

    let payment = Payment {
        id: payment_id,
        merchant_id: cmd.merchant_id,
        amount_minor: cmd.amount_minor,
        currency: cmd.currency.clone(),
        status: PaymentStatus::Pending,
        idempotency_key: cmd.idempotency_key.clone(),
        metadata: cmd.metadata.clone(),
        created_at: now,
        updated_at: now,
    };

    let inserted = repository::insert_payment_in_tx(&mut tx, &payment).await?;

    if let Some(created) = inserted {
        let audit_record = NewAuditRecord {
            id: Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
            actor_id: Some(cmd.actor_id),
            actor_type: ActorType::MERCHANT.into(),
            action: actions::PAYMENT_CREATED.into(),
            resource_type: "payment".into(),
            resource_id: created.id.to_string(),
            details: Some(json!({
                "merchant_id": cmd.merchant_id.to_string(),
                "amount_minor": cmd.amount_minor,
                "currency": cmd.currency,
                "idempotency_key": cmd.idempotency_key,
                "metadata": cmd.metadata,
            })),
            occurred_at: now,
            created_at: now,
        };
        record_required_in_tx(&mut tx, audit_record).await?;

        let domain_event_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        sqlx::query(
            r#"
            INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(domain_event_id)
        .bind("payment.created")
        .bind("payment")
        .bind(created.id)
        .bind(json!({
            "payment_id": created.id.to_string(),
            "merchant_id": cmd.merchant_id.to_string(),
            "amount_minor": cmd.amount_minor,
            "currency": cmd.currency,
            "metadata": cmd.metadata,
            "idempotency_key": cmd.idempotency_key,
            "created_at": created.created_at,
        }))
        .bind(1i32)
        .bind(now)
        .execute(tx.as_mut())
        .await?;

        tx.commit().await?;
        Ok(CreatePaymentOutcome::Created(created))
    } else {
        let existing = repository::find_payment_by_merchant_and_idempotency_key_in_tx(
            &mut tx,
            cmd.merchant_id,
            &cmd.idempotency_key,
        )
        .await?
        .ok_or_else(|| {
            CreatePaymentError::Conflict(
                "Idempotency key conflict but no existing payment found".into(),
            )
        })?;

        if existing.amount_minor == cmd.amount_minor
            && existing.currency == cmd.currency
            && existing.metadata == cmd.metadata
        {
            tx.commit().await?;
            Ok(CreatePaymentOutcome::Replayed(existing))
        } else {
            drop(tx);
            Err(CreatePaymentError::Conflict(
                "Idempotency key replay with different parameters".into(),
            ))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GetPaymentDetailError {
    #[error("Payment not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for GetPaymentDetailError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct RefundRow {
    id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    currency: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct DomainEventRow {
    id: Uuid,
    event_type: String,
    #[allow(dead_code)]
    aggregate_type: String,
    #[allow(dead_code)]
    aggregate_id: Uuid,
    payload: serde_json::Value,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct NotificationRow {
    id: Uuid,
    domain_event_id: Uuid,
    event_type: String,
    destination_url: String,
    status: String,
    attempt_count: i32,
    last_attempt_at: Option<DateTime<Utc>>,
    next_retry_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub async fn get_payment_detail(
    pool: &PgPool,
    payment_id: Uuid,
    viewer_merchant_id: Option<Uuid>,
) -> Result<PaymentDetailResponse, GetPaymentDetailError> {
    let payment = repository::find_by_id(pool, payment_id)
        .await?
        .ok_or(GetPaymentDetailError::NotFound)?;

    if let Some(merchant_id) = viewer_merchant_id {
        if payment.merchant_id != merchant_id {
            return Err(GetPaymentDetailError::NotFound);
        }
    }

    let refunds = sqlx::query_as::<_, RefundRow>(
        r#"
        SELECT id, payment_id, amount_minor, currency, status::text AS status, created_at, updated_at
        FROM refunds
        WHERE payment_id = $1
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(payment_id)
    .fetch_all(pool)
    .await?;

    let refund_ids: Vec<Uuid> = refunds.iter().map(|r| r.id).collect();

    let domain_events = if refund_ids.is_empty() {
        sqlx::query_as::<_, DomainEventRow>(
            r#"
            SELECT id, event_type, aggregate_type, aggregate_id, payload, created_at
            FROM domain_events
            WHERE aggregate_type = 'payment' AND aggregate_id = $1
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(payment_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, DomainEventRow>(
            r#"
            SELECT id, event_type, aggregate_type, aggregate_id, payload, created_at
            FROM domain_events
            WHERE (aggregate_type = 'payment' AND aggregate_id = $1)
               OR (aggregate_type = 'refund' AND aggregate_id = ANY($2))
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(payment_id)
        .bind(&refund_ids)
        .fetch_all(pool)
        .await?
    };

    let status_history = build_status_history(&domain_events);

    let notification_delivery_records = if domain_events.is_empty() {
        Vec::new()
    } else {
        let event_ids: Vec<Uuid> = domain_events.iter().map(|e| e.id).collect();
        sqlx::query_as::<_, NotificationRow>(
            r#"
            SELECT n.id, n.domain_event_id, e.event_type, n.destination_url,
                   n.status::text AS status, n.attempt_count, n.last_attempt_at,
                   n.next_retry_at, n.created_at, n.updated_at
            FROM notification_delivery_records n
            JOIN domain_events e ON e.id = n.domain_event_id
            WHERE n.domain_event_id = ANY($1)
            ORDER BY n.created_at ASC, n.id ASC
            "#,
        )
        .bind(&event_ids)
        .fetch_all(pool)
        .await?
    };

    Ok(PaymentDetailResponse {
        id: payment.id,
        merchant_id: payment.merchant_id,
        amount_minor: payment.amount_minor,
        currency: payment.currency,
        status: payment.status,
        metadata: payment.metadata,
        created_at: payment.created_at,
        updated_at: payment.updated_at,
        status_history,
        refunds: refunds
            .into_iter()
            .map(|r| PaymentRefundSummary {
                id: r.id,
                payment_id: r.payment_id,
                amount_minor: r.amount_minor,
                currency: r.currency,
                status: r.status,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect(),
        notification_delivery_records: notification_delivery_records
            .into_iter()
            .map(|n| PaymentNotificationDeliveryRecordResponse {
                id: n.id,
                domain_event_id: n.domain_event_id,
                event_type: n.event_type,
                destination_url: n.destination_url,
                status: n.status,
                attempt_count: n.attempt_count,
                last_attempt_at: n.last_attempt_at,
                next_retry_at: n.next_retry_at,
                created_at: n.created_at,
                updated_at: n.updated_at,
            })
            .collect(),
    })
}

pub fn map_event_to_status(event_type: &str) -> Option<(PaymentStatus, Option<serde_json::Value>)> {
    match event_type {
        "payment.created" => Some((PaymentStatus::Pending, None)),
        "payment.processing" => Some((PaymentStatus::Processing, None)),
        "payment.successful" => Some((PaymentStatus::Successful, None)),
        "payment.refunded" => Some((PaymentStatus::Refunded, None)),
        "refund.completed" => Some((PaymentStatus::Refunded, None)),
        "payment.failed" => Some((PaymentStatus::Failed, None)),
        _ => None,
    }
}

pub(crate) fn build_status_history(events: &[DomainEventRow]) -> Vec<PaymentStatusHistoryEntry> {
    let mut history = Vec::new();
    for event in events {
        if event.event_type == "refund.created" {
            continue;
        }
        let Some((status, default_details)) = map_event_to_status(&event.event_type) else {
            continue;
        };
        let details = if event.event_type == "payment.failed" {
            event
                .payload
                .get("failure_reason")
                .map(|v| serde_json::json!({ "failure_reason": v }))
        } else {
            default_details
        };
        history.push(PaymentStatusHistoryEntry {
            status,
            source_event_type: event.event_type.clone(),
            domain_event_id: event.id,
            occurred_at: event.created_at,
            details,
        });
    }
    history
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;

    fn event_row(event_type: &str, payload: serde_json::Value) -> DomainEventRow {
        DomainEventRow {
            id: Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
            event_type: event_type.into(),
            aggregate_type: "payment".into(),
            aggregate_id: Uuid::nil(),
            payload,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn maps_payment_created_to_pending() {
        let entry = build_status_history(&[event_row("payment.created", json!({}))])[0].clone();
        assert_eq!(entry.status, PaymentStatus::Pending);
        assert_eq!(entry.source_event_type, "payment.created");
        assert!(entry.details.is_none());
    }

    #[test]
    fn maps_payment_successful_to_successful() {
        let entry = build_status_history(&[event_row("payment.successful", json!({}))])[0].clone();
        assert_eq!(entry.status, PaymentStatus::Successful);
    }

    #[test]
    fn maps_payment_failed_with_failure_reason_details() {
        let entry = build_status_history(&[event_row(
            "payment.failed",
            json!({"failure_reason": "insufficient_funds"}),
        )])[0]
            .clone();
        assert_eq!(entry.status, PaymentStatus::Failed);
        assert_eq!(
            entry.details.unwrap()["failure_reason"],
            "insufficient_funds"
        );
    }

    #[test]
    fn maps_payment_failed_without_failure_reason_has_no_details() {
        let entry = build_status_history(&[event_row("payment.failed", json!({}))])[0].clone();
        assert!(entry.details.is_none());
    }

    #[test]
    fn maps_refund_completed_to_refunded() {
        let entry = build_status_history(&[event_row("refund.completed", json!({}))])[0].clone();
        assert_eq!(entry.status, PaymentStatus::Refunded);
    }

    #[test]
    fn ignores_refund_created() {
        let entries = build_status_history(&[event_row("refund.created", json!({}))]);
        assert!(entries.is_empty());
    }

    #[test]
    fn ignores_unknown_event_types() {
        let entries = build_status_history(&[event_row("payment.unknown", json!({}))]);
        assert!(entries.is_empty());
    }

    #[test]
    fn preserves_order_oldest_to_newest() {
        let mut first = event_row("payment.created", json!({}));
        first.created_at = Utc::now() - chrono::Duration::seconds(10);
        let mut second = event_row("payment.successful", json!({}));
        second.created_at = Utc::now();
        let entries = build_status_history(&[first, second]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, PaymentStatus::Pending);
        assert_eq!(entries[1].status, PaymentStatus::Successful);
    }
}
