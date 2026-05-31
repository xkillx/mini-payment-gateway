use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Payment, PaymentStatus};
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
