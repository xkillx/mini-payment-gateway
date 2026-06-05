use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::models::{
    ClaimedNotificationDelivery, NotificationDeliveryAttempt, NotificationDeliveryAttemptResponse,
    NotificationDeliveryRecord, NotificationDeliveryRecordDetailResponse, NotificationListFilter,
};

pub trait NotificationRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid)
        -> Result<Option<NotificationDeliveryRecord>, sqlx::Error>;
    async fn find_pending(
        &self,
        limit: i64,
    ) -> Result<Vec<NotificationDeliveryRecord>, sqlx::Error>;
    async fn insert(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn update_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn project_domain_events(&self, limit: i64) -> Result<u64, sqlx::Error>;
    async fn claim_due_for_delivery(
        &self,
        stale_before: DateTime<Utc>,
        max_attempts: u32,
    ) -> Result<Option<ClaimedNotificationDelivery>, sqlx::Error>;
    async fn mark_delivery_attempt_delivered(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn mark_delivery_attempt_failed_pending_retry(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
        last_error: &str,
        next_retry_at: DateTime<Utc>,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn mark_delivery_attempt_failed_terminal(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
        last_error: &str,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
    async fn list_details(
        &self,
        filter: &NotificationListFilter,
    ) -> Result<Vec<NotificationDeliveryRecordDetailResponse>, sqlx::Error>;
    async fn find_detail_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecordDetailResponse>, sqlx::Error>;
}

pub struct PostgresNotificationRepository {
    pool: PgPool,
}

impl PostgresNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn retry_failed_in_tx(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecordDetailResponse>, sqlx::Error> {
        #[derive(Debug, sqlx::FromRow)]
        struct RetryRow {
            id: Uuid,
            domain_event_id: Uuid,
            destination_url: String,
            status: crate::models::NotificationStatus,
            attempt_count: i32,
            retry_generation: i32,
            last_attempt_at: Option<DateTime<Utc>>,
            next_retry_at: Option<DateTime<Utc>>,
            last_error: Option<String>,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            event_type: String,
            resource_type: String,
            resource_id: Uuid,
            payment_id: Uuid,
        }

        let row = sqlx::query_as::<_, RetryRow>(
            r#"
            UPDATE notification_delivery_records
            SET status = 'pending',
                retry_generation = retry_generation + 1,
                next_retry_at = NULL,
                updated_at = NOW()
            WHERE id = $1 AND status = 'failed'
            RETURNING
                id, domain_event_id, destination_url, status, attempt_count,
                retry_generation, last_attempt_at, next_retry_at, last_error,
                created_at, updated_at,
                (SELECT event_type FROM domain_events de WHERE de.id = domain_event_id) AS event_type,
                (SELECT aggregate_type FROM domain_events de WHERE de.id = domain_event_id) AS resource_type,
                (SELECT aggregate_id FROM domain_events de WHERE de.id = domain_event_id) AS resource_id,
                COALESCE(
                    (SELECT aggregate_id FROM domain_events de WHERE de.id = domain_event_id AND de.aggregate_type = 'payment'),
                    (SELECT r.payment_id FROM domain_events de JOIN refunds r ON de.aggregate_type = 'refund' AND r.id = de.aggregate_id WHERE de.id = domain_event_id)
                ) AS payment_id
            "#,
        )
        .bind(id)
        .fetch_optional(tx.as_mut())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let attempts = load_attempts_for_record(tx.as_mut(), row.id).await?;

        Ok(Some(build_detail(
            row.id,
            row.domain_event_id,
            row.event_type,
            row.resource_type,
            row.resource_id,
            row.payment_id,
            row.destination_url,
            row.status,
            row.attempt_count,
            row.retry_generation,
            row.last_attempt_at,
            row.next_retry_at,
            row.last_error,
            row.created_at,
            row.updated_at,
            attempts,
        )))
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectionPair {
    domain_event_id: Uuid,
    destination_url: String,
}

#[derive(Debug, sqlx::FromRow)]
struct NotificationDetailRow {
    id: Uuid,
    domain_event_id: Uuid,
    destination_url: String,
    status: crate::models::NotificationStatus,
    attempt_count: i32,
    retry_generation: i32,
    last_attempt_at: Option<DateTime<Utc>>,
    next_retry_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    event_type: String,
    resource_type: String,
    resource_id: Uuid,
    payment_id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct ClaimRow {
    id: Uuid,
    domain_event_id: Uuid,
    destination_url: String,
    attempt_count: i32,
    retry_generation: i32,
    event_type: String,
    aggregate_type: String,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    event_created_at: DateTime<Utc>,
    version: i32,
}

fn build_detail(
    id: Uuid,
    domain_event_id: Uuid,
    event_type: String,
    resource_type: String,
    resource_id: Uuid,
    payment_id: Uuid,
    destination_url: String,
    status: crate::models::NotificationStatus,
    attempt_count: i32,
    retry_generation: i32,
    last_attempt_at: Option<DateTime<Utc>>,
    next_retry_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    attempts: Vec<NotificationDeliveryAttemptResponse>,
) -> NotificationDeliveryRecordDetailResponse {
    let resource_api_path = match resource_type.as_str() {
        "payment" => format!("/api/v1/payments/{resource_id}"),
        "refund" => format!("/api/v1/refunds/{resource_id}"),
        _ => format!("/api/v1/{}s/{resource_id}", resource_type),
    };
    let payment_api_path = format!("/api/v1/payments/{payment_id}");

    let (refund_id, refund_api_path) = if resource_type == "refund" {
        (
            Some(resource_id),
            Some(format!("/api/v1/refunds/{resource_id}")),
        )
    } else {
        (None, None)
    };

    NotificationDeliveryRecordDetailResponse {
        id,
        domain_event_id,
        event_type,
        resource_type,
        resource_id,
        resource_api_path,
        payment_id,
        payment_api_path,
        refund_id,
        refund_api_path,
        destination_url,
        status,
        attempt_count,
        retry_generation,
        last_attempt_at,
        next_retry_at,
        last_error,
        created_at,
        updated_at,
        attempts,
    }
}

async fn load_attempts_for_record(
    executor: impl sqlx::Executor<'_, Database = Postgres>,
    record_id: Uuid,
) -> Result<Vec<NotificationDeliveryAttemptResponse>, sqlx::Error> {
    let rows = sqlx::query_as::<_, NotificationDeliveryAttempt>(
        "SELECT * FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1 ORDER BY attempt_number ASC",
    )
    .bind(record_id)
    .fetch_all(executor)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

impl NotificationRepository for PostgresNotificationRepository {
    async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecord>, sqlx::Error> {
        sqlx::query_as::<_, NotificationDeliveryRecord>(
            "SELECT * FROM notification_delivery_records WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_pending(
        &self,
        limit: i64,
    ) -> Result<Vec<NotificationDeliveryRecord>, sqlx::Error> {
        sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            SELECT * FROM notification_delivery_records
            WHERE status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= NOW())
            ORDER BY created_at ASC
            LIMIT $1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    async fn insert(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, retry_generation, last_attempt_at, next_retry_at, last_error, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(record.domain_event_id)
        .bind(&record.destination_url)
        .bind(&record.status)
        .bind(record.attempt_count)
        .bind(record.retry_generation)
        .bind(record.last_attempt_at)
        .bind(record.next_retry_at)
        .bind(&record.last_error)
        .bind(record.created_at)
        .bind(record.updated_at)
        .fetch_one(&self.pool)
        .await
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: &str,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            UPDATE notification_delivery_records SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
    }

    async fn project_domain_events(&self, limit: i64) -> Result<u64, sqlx::Error> {
        let candidates = sqlx::query_as::<_, ProjectionPair>(
            r#"
            WITH resolved AS (
                SELECT
                    e.id AS event_id,
                    e.created_at,
                    CASE
                        WHEN e.event_type LIKE 'payment.%' THEN
                            COALESCE(
                                CASE WHEN e.payload->>'merchant_id' ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                                     THEN (e.payload->>'merchant_id')::uuid
                                END,
                                p.merchant_id
                            )
                        WHEN e.event_type LIKE 'refund.%' THEN
                            COALESCE(
                                CASE WHEN e.payload->>'merchant_id' ~ '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
                                     THEN (e.payload->>'merchant_id')::uuid
                                END,
                                r.merchant_id
                            )
                    END AS merchant_id
                FROM domain_events e
                LEFT JOIN payments p ON e.aggregate_type = 'payment' AND e.aggregate_id = p.id
                LEFT JOIN refunds r ON e.aggregate_type = 'refund' AND e.aggregate_id = r.id
                WHERE e.event_type IN ('payment.created', 'payment.successful', 'payment.failed', 'refund.created', 'refund.completed')
            )
            SELECT r.event_id AS domain_event_id, d.destination_url
            FROM resolved r
            JOIN notification_destinations d ON d.merchant_id = r.merchant_id AND d.is_active = true
            WHERE r.merchant_id IS NOT NULL
              AND NOT EXISTS (
                  SELECT 1 FROM notification_delivery_records ndr
                  WHERE ndr.domain_event_id = r.event_id
                    AND ndr.destination_url = d.destination_url
              )
            ORDER BY r.created_at ASC, r.event_id ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut inserted: u64 = 0;
        for pair in candidates {
            let id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
            let result = sqlx::query(
                r#"
                INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, next_retry_at, last_error, created_at, updated_at)
                VALUES ($1, $2, $3, 'pending', 0, NULL, NULL, NULL, NOW(), NOW())
                ON CONFLICT (domain_event_id, destination_url) DO NOTHING
                "#,
            )
            .bind(id)
            .bind(pair.domain_event_id)
            .bind(&pair.destination_url)
            .execute(&self.pool)
            .await?;
            inserted += result.rows_affected();
        }

        Ok(inserted)
    }

    async fn claim_due_for_delivery(
        &self,
        stale_before: DateTime<Utc>,
        max_attempts: u32,
    ) -> Result<Option<ClaimedNotificationDelivery>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let candidate: Option<ClaimRow> = sqlx::query_as::<_, ClaimRow>(
            r#"
            SELECT id, domain_event_id, destination_url, attempt_count, retry_generation,
                   (SELECT de.event_type FROM domain_events de WHERE de.id = ndr.domain_event_id) AS event_type,
                   (SELECT de.aggregate_type FROM domain_events de WHERE de.id = ndr.domain_event_id) AS aggregate_type,
                   (SELECT de.aggregate_id FROM domain_events de WHERE de.id = ndr.domain_event_id) AS aggregate_id,
                   (SELECT de.payload FROM domain_events de WHERE de.id = ndr.domain_event_id) AS payload,
                   (SELECT de.created_at FROM domain_events de WHERE de.id = ndr.domain_event_id) AS event_created_at,
                   (SELECT de.version FROM domain_events de WHERE de.id = ndr.domain_event_id) AS version
            FROM notification_delivery_records ndr
            WHERE
                (ndr.status = 'pending' AND (ndr.next_retry_at IS NULL OR ndr.next_retry_at <= NOW()))
                OR (ndr.status = 'processing' AND ndr.last_attempt_at <= $1)
            ORDER BY ndr.created_at ASC, ndr.id ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
            "#,
        )
        .bind(stale_before)
        .fetch_optional(tx.as_mut())
        .await?;

        let Some(candidate) = candidate else {
            tx.commit().await?;
            return Ok(None);
        };

        let was_stale = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1 AND status = 'processing')",
        )
        .bind(candidate.id)
        .fetch_one(tx.as_mut())
        .await?;

        if was_stale {
            sqlx::query(
                r#"
                UPDATE notification_delivery_attempts
                SET status = 'failed',
                    error = 'stale_processing_reclaimed',
                    finished_at = NOW(),
                    updated_at = NOW()
                WHERE notification_delivery_record_id = $1 AND status = 'processing'
                "#,
            )
            .bind(candidate.id)
            .execute(tx.as_mut())
            .await?;
        }

        let gen_attempts: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM notification_delivery_attempts
            WHERE notification_delivery_record_id = $1 AND retry_generation = $2
            "#,
        )
        .bind(candidate.id)
        .bind(candidate.retry_generation)
        .fetch_one(tx.as_mut())
        .await?;

        if gen_attempts >= max_attempts as i64 {
            sqlx::query(
                r#"
                UPDATE notification_delivery_records
                SET status = 'failed',
                    last_error = 'stale_processing_reclaimed',
                    next_retry_at = NULL,
                    updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(candidate.id)
            .execute(tx.as_mut())
            .await?;
            tx.commit().await?;
            return Ok(None);
        }

        let new_attempt_count = candidate.attempt_count + 1;
        let new_attempt_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let started_at = Utc::now();
        let generation_attempt_number = (gen_attempts + 1) as u32;

        sqlx::query(
            r#"
            INSERT INTO notification_delivery_attempts
                (id, notification_delivery_record_id, retry_generation, attempt_number, status, started_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'processing', $5, $5, $5)
            "#,
        )
        .bind(new_attempt_id)
        .bind(candidate.id)
        .bind(candidate.retry_generation)
        .bind(new_attempt_count)
        .bind(started_at)
        .execute(tx.as_mut())
        .await?;

        sqlx::query(
            r#"
            UPDATE notification_delivery_records
            SET status = 'processing',
                attempt_count = $2,
                last_attempt_at = $3,
                next_retry_at = NULL,
                updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(candidate.id)
        .bind(new_attempt_count)
        .bind(started_at)
        .execute(tx.as_mut())
        .await?;

        tx.commit().await?;

        Ok(Some(ClaimedNotificationDelivery {
            id: candidate.id,
            domain_event_id: candidate.domain_event_id,
            destination_url: candidate.destination_url,
            attempt_count: new_attempt_count,
            retry_generation: candidate.retry_generation,
            event_type: candidate.event_type,
            aggregate_type: candidate.aggregate_type,
            aggregate_id: candidate.aggregate_id,
            payload: candidate.payload,
            event_created_at: candidate.event_created_at,
            version: candidate.version,
            attempt_id: new_attempt_id,
            attempt_number: new_attempt_count,
            generation_attempt_number,
        }))
    }

    async fn mark_delivery_attempt_delivered(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE notification_delivery_attempts
            SET status = 'delivered',
                http_status_code = $2,
                error = NULL,
                finished_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(attempt_id)
        .bind(http_status_code)
        .execute(tx.as_mut())
        .await?;

        let record = sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            UPDATE notification_delivery_records
            SET status = 'delivered',
                next_retry_at = NULL,
                last_error = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(record_id)
        .fetch_one(tx.as_mut())
        .await?;

        tx.commit().await?;
        Ok(record)
    }

    async fn mark_delivery_attempt_failed_pending_retry(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
        last_error: &str,
        next_retry_at: DateTime<Utc>,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE notification_delivery_attempts
            SET status = 'failed',
                http_status_code = $2,
                error = $3,
                finished_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(attempt_id)
        .bind(http_status_code)
        .bind(last_error)
        .execute(tx.as_mut())
        .await?;

        let record = sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            UPDATE notification_delivery_records
            SET status = 'pending',
                next_retry_at = $2,
                last_error = $3,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(record_id)
        .bind(next_retry_at)
        .bind(last_error)
        .fetch_one(tx.as_mut())
        .await?;

        tx.commit().await?;
        Ok(record)
    }

    async fn mark_delivery_attempt_failed_terminal(
        &self,
        record_id: Uuid,
        attempt_id: Uuid,
        http_status_code: Option<i32>,
        last_error: &str,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE notification_delivery_attempts
            SET status = 'failed',
                http_status_code = $2,
                error = $3,
                finished_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(attempt_id)
        .bind(http_status_code)
        .bind(last_error)
        .execute(tx.as_mut())
        .await?;

        let record = sqlx::query_as::<_, NotificationDeliveryRecord>(
            r#"
            UPDATE notification_delivery_records
            SET status = 'failed',
                next_retry_at = NULL,
                last_error = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(record_id)
        .bind(last_error)
        .fetch_one(tx.as_mut())
        .await?;

        tx.commit().await?;
        Ok(record)
    }

    async fn list_details(
        &self,
        filter: &NotificationListFilter,
    ) -> Result<Vec<NotificationDeliveryRecordDetailResponse>, sqlx::Error> {
        let mut qb: sqlx::QueryBuilder<'_, Postgres> = sqlx::QueryBuilder::new(
            r#"SELECT ndr.id, ndr.domain_event_id, ndr.destination_url, ndr.status, ndr.attempt_count, ndr.retry_generation, ndr.last_attempt_at, ndr.next_retry_at, ndr.last_error, ndr.created_at, ndr.updated_at, de.event_type, de.aggregate_type AS resource_type, de.aggregate_id AS resource_id,
            COALESCE(
                CASE WHEN de.aggregate_type = 'payment' THEN de.aggregate_id END,
                r.payment_id
            ) AS payment_id
            FROM notification_delivery_records ndr
            JOIN domain_events de ON de.id = ndr.domain_event_id
            LEFT JOIN refunds r ON de.aggregate_type = 'refund' AND r.id = de.aggregate_id
            WHERE 1=1"#,
        );

        if let Some(ref status) = filter.status {
            qb.push(" AND ndr.status = ");
            qb.push_bind(status.clone());
        }

        qb.push(" ORDER BY ndr.created_at DESC, ndr.id DESC LIMIT ")
            .push_bind(filter.limit);
        qb.push(" OFFSET ").push_bind(filter.offset);

        let rows = qb
            .build_query_as::<NotificationDetailRow>()
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let attempts = load_attempts_for_record(&self.pool, row.id).await?;
            results.push(build_detail(
                row.id,
                row.domain_event_id,
                row.event_type,
                row.resource_type,
                row.resource_id,
                row.payment_id,
                row.destination_url,
                row.status,
                row.attempt_count,
                row.retry_generation,
                row.last_attempt_at,
                row.next_retry_at,
                row.last_error,
                row.created_at,
                row.updated_at,
                attempts,
            ));
        }
        Ok(results)
    }

    async fn find_detail_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecordDetailResponse>, sqlx::Error> {
        let row = sqlx::query_as::<_, NotificationDetailRow>(
            r#"
            SELECT ndr.id, ndr.domain_event_id, ndr.destination_url, ndr.status, ndr.attempt_count, ndr.retry_generation, ndr.last_attempt_at, ndr.next_retry_at, ndr.last_error, ndr.created_at, ndr.updated_at, de.event_type, de.aggregate_type AS resource_type, de.aggregate_id AS resource_id,
            COALESCE(
                CASE WHEN de.aggregate_type = 'payment' THEN de.aggregate_id END,
                r.payment_id
            ) AS payment_id
            FROM notification_delivery_records ndr
            JOIN domain_events de ON de.id = ndr.domain_event_id
            LEFT JOIN refunds r ON de.aggregate_type = 'refund' AND r.id = de.aggregate_id
            WHERE ndr.id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let attempts = load_attempts_for_record(&self.pool, row.id).await?;
        Ok(Some(build_detail(
            row.id,
            row.domain_event_id,
            row.event_type,
            row.resource_type,
            row.resource_id,
            row.payment_id,
            row.destination_url,
            row.status,
            row.attempt_count,
            row.retry_generation,
            row.last_attempt_at,
            row.next_retry_at,
            row.last_error,
            row.created_at,
            row.updated_at,
            attempts,
        )))
    }
}
