use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Refund, RefundStatus};
use crate::repository::{self, RefundRepository};

use audit::models::{actions, ActorType, NewAuditRecord};
use audit::service::{record_required_in_tx, AuditError};

struct NewDomainEvent {
    event_type: &'static str,
    aggregate_type: &'static str,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    occurred_at: chrono::DateTime<Utc>,
    version: i32,
}

async fn record_domain_event_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: NewDomainEvent,
) -> Result<Uuid, sqlx::Error> {
    let event_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(event_id)
    .bind(event.event_type)
    .bind(event.aggregate_type)
    .bind(event.aggregate_id)
    .bind(event.payload)
    .bind(event.version)
    .bind(event.occurred_at)
    .execute(tx.as_mut())
    .await?;
    Ok(event_id)
}

pub trait RefundService: Send + Sync {
    async fn get_refund(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error>;
    async fn list_merchant_refunds(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error>;
}

pub struct DefaultRefundService<R: RefundRepository> {
    repo: R,
}

impl<R: RefundRepository> DefaultRefundService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: RefundRepository + 'static> RefundService for DefaultRefundService<R> {
    async fn get_refund(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn list_merchant_refunds(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error> {
        self.repo.find_by_merchant(merchant_id).await
    }
}

#[derive(Debug)]
pub struct CreateRefundCommand {
    pub actor_id: Uuid,
    pub merchant_id: Uuid,
    pub payment_id: Uuid,
    pub idempotency_key: String,
}

#[derive(Debug)]
pub enum CreateRefundOutcome {
    Created(Refund),
    Replayed(Refund),
}

#[derive(Debug, thiserror::Error)]
pub enum CreateRefundError {
    #[error("Payment not found")]
    NotFound,
    #[error("{0}")]
    InvalidPaymentState(String),
    #[error("{0}")]
    DuplicateRefund(String),
    #[error("{0}")]
    Conflict(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Audit error: {0}")]
    Audit(#[from] AuditError),
}

impl From<sqlx::Error> for CreateRefundError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub async fn create_refund(
    pool: &PgPool,
    cmd: CreateRefundCommand,
) -> Result<CreateRefundOutcome, CreateRefundError> {
    let mut tx = pool.begin().await?;

    let existing_by_key = repository::find_refund_by_merchant_and_idempotency_key_in_tx(
        &mut tx,
        cmd.merchant_id,
        &cmd.idempotency_key,
    )
    .await?;

    if let Some(ref existing) = existing_by_key {
        if existing.payment_id == cmd.payment_id {
            tx.commit().await?;
            return Ok(CreateRefundOutcome::Replayed(existing.clone()));
        } else {
            drop(tx);
            return Err(CreateRefundError::Conflict(
                "Idempotency key replay with different parameters".into(),
            ));
        }
    }

    let payment =
        repository::find_payment_for_refund_in_tx(&mut tx, cmd.payment_id, cmd.merchant_id).await?;

    let payment = match payment {
        Some(p) => p,
        None => {
            drop(tx);
            return Err(CreateRefundError::NotFound);
        }
    };

    if payment.status != "successful" {
        drop(tx);
        return Err(CreateRefundError::InvalidPaymentState(format!(
            "payment status is {}; only successful payments can be refunded",
            payment.status
        )));
    }

    let existing_by_payment =
        repository::find_refund_by_payment_id_in_tx(&mut tx, cmd.payment_id).await?;

    if existing_by_payment.is_some() {
        drop(tx);
        return Err(CreateRefundError::DuplicateRefund(
            "Payment already has a refund".into(),
        ));
    }

    let now = Utc::now();
    let refund_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

    let refund = Refund {
        id: refund_id,
        payment_id: cmd.payment_id,
        merchant_id: cmd.merchant_id,
        amount_minor: payment.amount_minor,
        currency: payment.currency.clone(),
        status: RefundStatus::Pending,
        idempotency_key: cmd.idempotency_key.clone(),
        created_at: now,
        updated_at: now,
    };

    let inserted = repository::insert_refund_in_tx(&mut tx, &refund).await?;

    let created = match inserted {
        Some(c) => c,
        None => {
            let existing = repository::find_refund_by_merchant_and_idempotency_key_in_tx(
                &mut tx,
                cmd.merchant_id,
                &cmd.idempotency_key,
            )
            .await?
            .ok_or_else(|| {
                CreateRefundError::Conflict(
                    "Idempotency key conflict but no existing refund found".into(),
                )
            })?;

            if existing.payment_id == cmd.payment_id {
                tx.commit().await?;
                return Ok(CreateRefundOutcome::Replayed(existing));
            } else {
                drop(tx);
                return Err(CreateRefundError::Conflict(
                    "Idempotency key replay with different parameters".into(),
                ));
            }
        }
    };

    let audit_record = NewAuditRecord {
        id: Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
        actor_id: Some(cmd.actor_id),
        actor_type: ActorType::MERCHANT.into(),
        action: actions::REFUND_CREATED.into(),
        resource_type: "refund".into(),
        resource_id: created.id.to_string(),
        details: Some(json!({
            "merchant_id": cmd.merchant_id.to_string(),
            "payment_id": cmd.payment_id.to_string(),
            "amount_minor": payment.amount_minor,
            "currency": payment.currency,
            "idempotency_key": cmd.idempotency_key,
            "status": "pending",
        })),
        occurred_at: now,
        created_at: now,
    };
    record_required_in_tx(&mut tx, audit_record).await?;

    record_domain_event_in_tx(
        &mut tx,
        NewDomainEvent {
            event_type: "refund.created",
            aggregate_type: "refund",
            aggregate_id: created.id,
            payload: json!({
                "refund_id": created.id.to_string(),
                "payment_id": cmd.payment_id.to_string(),
                "amount_minor": payment.amount_minor,
                "currency": payment.currency,
                "created_at": now,
            }),
            occurred_at: now,
            version: 1,
        },
    )
    .await?;

    tx.commit().await?;
    Ok(CreateRefundOutcome::Created(created))
}
