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

async fn record_rejected_refund_attempt_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cmd: &CreateRefundCommand,
    rejection_code: &str,
    rejection_reason: &str,
    context: Option<serde_json::Value>,
) -> Result<(), AuditError> {
    let now = Utc::now();
    let mut details = json!({
        "merchant_id": cmd.merchant_id.to_string(),
        "payment_id": cmd.payment_id.to_string(),
        "idempotency_key": cmd.idempotency_key,
        "rejection_code": rejection_code,
        "rejection_reason": rejection_reason,
    });
    if let Some(ctx) = context {
        if let Some(obj) = details.as_object_mut() {
            for (k, v) in ctx.as_object().into_iter().flat_map(|o| o.iter()) {
                obj.insert(k.clone(), v.clone());
            }
        }
    }
    let audit_record = NewAuditRecord {
        id: Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)),
        actor_id: Some(cmd.actor_id),
        actor_type: ActorType::MERCHANT.into(),
        action: actions::REFUND_REJECTED.into(),
        resource_type: "payment".into(),
        resource_id: cmd.payment_id.to_string(),
        details: Some(details),
        occurred_at: now,
        created_at: now,
    };
    record_required_in_tx(tx, audit_record).await?;
    Ok(())
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
            record_rejected_refund_attempt_in_tx(
                &mut tx,
                &cmd,
                "idempotency_key_conflict",
                "Idempotency key replay with different parameters",
                Some(json!({
                    "existing_payment_id": existing.payment_id.to_string(),
                })),
            )
            .await?;
            tx.commit().await?;
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
            record_rejected_refund_attempt_in_tx(
                &mut tx,
                &cmd,
                "payment_not_found",
                "Payment not found",
                None,
            )
            .await?;
            tx.commit().await?;
            return Err(CreateRefundError::NotFound);
        }
    };

    if payment.status != "successful" {
        let rejection_reason = format!(
            "payment status is {}; only successful payments can be refunded",
            payment.status
        );
        record_rejected_refund_attempt_in_tx(
            &mut tx,
            &cmd,
            "invalid_payment_status",
            &rejection_reason,
            Some(json!({
                "payment_status": payment.status,
            })),
        )
        .await?;
        tx.commit().await?;
        return Err(CreateRefundError::InvalidPaymentState(rejection_reason));
    }

    let existing_by_payment =
        repository::find_refund_by_payment_id_in_tx(&mut tx, cmd.payment_id).await?;

    if let Some(ref existing) = existing_by_payment {
        record_rejected_refund_attempt_in_tx(
            &mut tx,
            &cmd,
            "duplicate_refund",
            "Payment already has a refund",
            Some(json!({
                "existing_refund_id": existing.id.to_string(),
            })),
        )
        .await?;
        tx.commit().await?;
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
            .await?;

            if let Some(existing) = existing {
                if existing.payment_id == cmd.payment_id {
                    tx.commit().await?;
                    return Ok(CreateRefundOutcome::Replayed(existing));
                }

                record_rejected_refund_attempt_in_tx(
                    &mut tx,
                    &cmd,
                    "idempotency_key_conflict",
                    "Idempotency key replay with different parameters",
                    Some(json!({
                        "existing_payment_id": existing.payment_id.to_string(),
                    })),
                )
                .await?;
                tx.commit().await?;
                return Err(CreateRefundError::Conflict(
                    "Idempotency key replay with different parameters".into(),
                ));
            }

            let by_payment =
                repository::find_refund_by_payment_id_in_tx(&mut tx, cmd.payment_id).await?;

            if let Some(existing) = by_payment {
                record_rejected_refund_attempt_in_tx(
                    &mut tx,
                    &cmd,
                    "duplicate_refund",
                    "Payment already has a refund",
                    Some(json!({
                        "existing_refund_id": existing.id.to_string(),
                    })),
                )
                .await?;
                tx.commit().await?;
                return Err(CreateRefundError::DuplicateRefund(
                    "Payment already has a refund".into(),
                ));
            }

            return Err(CreateRefundError::Database(
                "Insert conflict but no matching refund found".into(),
            ));
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

#[derive(Debug, thiserror::Error)]
pub enum ListRefundsError {
    #[error("Database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for ListRefundsError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub async fn list_refunds(
    pool: &PgPool,
    filter: crate::models::RefundListFilter,
) -> Result<crate::models::RefundListResponse, ListRefundsError> {
    use crate::models::{RefundListResponse, RefundReadResponse};
    use crate::repository::{PostgresRefundRepository, RefundRepository};

    let repo = PostgresRefundRepository::new(pool.clone());
    let rows = repo.list(&filter).await?;
    let limit = filter.limit;
    let offset = filter.offset;
    Ok(RefundListResponse {
        items: rows.into_iter().map(RefundReadResponse::from).collect(),
        limit,
        offset,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum GetRefundDetailError {
    #[error("Refund not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for GetRefundDetailError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

pub async fn get_refund_detail(
    pool: &PgPool,
    refund_id: Uuid,
    viewer_merchant_id: Option<Uuid>,
) -> Result<crate::models::RefundReadResponse, GetRefundDetailError> {
    use crate::models::RefundReadResponse;
    use crate::repository::{PostgresRefundRepository, RefundRepository};

    let repo = PostgresRefundRepository::new(pool.clone());
    let refund = repo
        .find_by_id(refund_id)
        .await?
        .ok_or(GetRefundDetailError::NotFound)?;

    if let Some(merchant_id) = viewer_merchant_id {
        if refund.merchant_id != merchant_id {
            return Err(GetRefundDetailError::NotFound);
        }
    }

    Ok(RefundReadResponse::from(refund))
}
