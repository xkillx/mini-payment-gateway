use sqlx::PgPool;
use uuid::Uuid;

use crate::models::NotificationDeliveryRecord;

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
}

pub struct PostgresNotificationRepository {
    pool: PgPool,
}

impl PostgresNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ProjectionPair {
    domain_event_id: Uuid,
    destination_url: String,
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
            INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, next_retry_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(record.id)
        .bind(record.domain_event_id)
        .bind(&record.destination_url)
        .bind(&record.status)
        .bind(record.attempt_count)
        .bind(record.last_attempt_at)
        .bind(record.next_retry_at)
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
                INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, next_retry_at, created_at, updated_at)
                VALUES ($1, $2, $3, 'pending', 0, NULL, NULL, NOW(), NOW())
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
}
