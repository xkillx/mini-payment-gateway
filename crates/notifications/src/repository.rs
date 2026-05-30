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
}

pub struct PostgresNotificationRepository {
    pool: PgPool,
}

impl PostgresNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
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
}
