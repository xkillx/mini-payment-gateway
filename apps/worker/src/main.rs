use notifications::repository::{NotificationRepository, PostgresNotificationRepository};
use payments::service::{process_next_pending_payment, PaymentProcessingOutcome};
use shared_config::AppConfig;
use shared_db as db;
use shared_observability as observability;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

const NOTIFICATION_PROJECTION_BATCH_SIZE: i64 = 100;

#[derive(sqlx::FromRow)]
struct PendingNotification {
    id: Uuid,
    destination_url: String,
    attempt_count: i32,
}

async fn poll_notifications(pool: &PgPool, config: &AppConfig) {
    loop {
        if let Err(e) = try_process_payment(pool).await {
            tracing::error!("Payment processing error: {e}");
        }
        if let Err(e) = try_project_notifications(pool).await {
            tracing::error!("Notification projection error: {e}");
        }
        if let Err(e) = try_process_notifications(pool, config).await {
            tracing::error!("Notification processing error: {e}");
        }
        tokio::time::sleep(Duration::from_millis(config.worker_poll_interval_ms)).await;
    }
}

async fn try_project_notifications(pool: &PgPool) -> Result<u64, anyhow::Error> {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let inserted = repo
        .project_domain_events(NOTIFICATION_PROJECTION_BATCH_SIZE)
        .await?;
    if inserted > 0 {
        tracing::info!(inserted, "Projected notification delivery records");
    } else {
        tracing::debug!("No new notification delivery records to project");
    }
    Ok(inserted)
}

async fn try_process_payment(pool: &PgPool) -> Result<(), anyhow::Error> {
    match process_next_pending_payment(pool, PaymentProcessingOutcome::simulated_success()).await {
        Ok(Some(payment)) => {
            tracing::info!(
                payment_id = %payment.id,
                status = ?payment.status,
                "Payment processed"
            );
        }
        Ok(None) => {
            tracing::debug!("No pending payments to process");
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to process pending payment");
        }
    }
    Ok(())
}

async fn try_process_notifications(pool: &PgPool, config: &AppConfig) -> Result<(), anyhow::Error> {
    let pending = sqlx::query_as::<_, PendingNotification>(
        r#"
        UPDATE notification_delivery_records
        SET status = 'processing', updated_at = NOW()
        WHERE id = (
            SELECT id FROM notification_delivery_records
            WHERE status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= NOW())
            ORDER BY created_at ASC
            LIMIT 1
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, destination_url, attempt_count
        "#,
    )
    .fetch_optional(pool)
    .await?;

    if let Some(record) = pending {
        let id = record.id;
        let url = record.destination_url;
        let attempt_count = record.attempt_count;

        tracing::info!(notification_id = %id, url = %url, attempt = attempt_count + 1, "Processing notification");

        let success = deliver_notification(&url).await;

        if success {
            sqlx::query(
                r#"
                UPDATE notification_delivery_records
                SET status = 'delivered', attempt_count = attempt_count + 1, last_attempt_at = NOW(), updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(id)
            .execute(pool)
            .await?;
            tracing::info!(notification_id = %id, "Notification delivered");
        } else {
            let new_attempt = attempt_count + 1;
            if new_attempt >= config.notification_max_attempts as i32 {
                sqlx::query(
                    r#"
                    UPDATE notification_delivery_records
                    SET status = 'failed', attempt_count = $2, last_attempt_at = NOW(), updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(id)
                .bind(new_attempt)
                .execute(pool)
                .await?;
                tracing::warn!(notification_id = %id, attempts = new_attempt, "Notification failed after max attempts");
            } else {
                let delay_secs = config
                    .notification_retry_delays_secs
                    .get((new_attempt - 1) as usize)
                    .copied()
                    .unwrap_or(3600);
                let next_retry = chrono::Utc::now() + chrono::Duration::seconds(delay_secs as i64);
                sqlx::query(
                    r#"
                    UPDATE notification_delivery_records
                    SET status = 'pending', attempt_count = $2, last_attempt_at = NOW(), next_retry_at = $3, updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(id)
                .bind(new_attempt)
                .bind(next_retry)
                .execute(pool)
                .await?;
                tracing::info!(notification_id = %id, next_retry = %next_retry, "Notification scheduled for retry");
            }
        }
    }

    Ok(())
}

async fn deliver_notification(url: &str) -> bool {
    match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => match client
            .post(url)
            .json(&serde_json::json!({"event": "ping"}))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                tracing::warn!(url = %url, error = %e, "Notification delivery failed");
                false
            }
        },
        Err(e) => {
            tracing::error!("Failed to build HTTP client: {e}");
            false
        }
    }
}

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();
    observability::init(&config.log_level);

    let pool = db::connect(&config.database_url).await;

    tracing::info!(
        "Worker starting with poll interval {}ms",
        config.worker_poll_interval_ms
    );

    poll_notifications(&pool, &config).await;
}
