use notifications::repository::{NotificationRepository, PostgresNotificationRepository};
use notifications::service::{
    DefaultNotificationService, NotificationDeliverySettings, ReqwestNotificationTransport,
};
use payments::service::{process_next_pending_payment, PaymentProcessingOutcome};
use shared_config::AppConfig;
use shared_db as db;
use shared_observability as observability;
use sqlx::PgPool;
use std::time::Duration;

const NOTIFICATION_PROJECTION_BATCH_SIZE: i64 = 100;

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
    let repo = PostgresNotificationRepository::new(pool.clone());
    let service = DefaultNotificationService::new(repo);
    let transport = ReqwestNotificationTransport::new();
    let settings = NotificationDeliverySettings::new(
        config.notification_max_attempts,
        config.notification_retry_delays_secs.clone(),
    );

    match service
        .process_next_due_delivery(&transport, &settings)
        .await
    {
        Ok(Some(outcome)) => match outcome.status {
            notifications::service::DeliveryStatus::Delivered => {
                tracing::info!(
                    notification_id = %outcome.record_id,
                    event_type = %outcome.event_type,
                    destination_url = %outcome.destination_url,
                    attempt = outcome.attempt_number,
                    "Notification delivered"
                );
            }
            notifications::service::DeliveryStatus::PendingRetry => {
                tracing::info!(
                    notification_id = %outcome.record_id,
                    event_type = %outcome.event_type,
                    destination_url = %outcome.destination_url,
                    attempt = outcome.attempt_number,
                    next_retry_at = ?outcome.next_retry_at,
                    last_error = ?outcome.last_error,
                    "Notification scheduled for retry"
                );
            }
            notifications::service::DeliveryStatus::TerminalFailed => {
                tracing::warn!(
                    notification_id = %outcome.record_id,
                    event_type = %outcome.event_type,
                    destination_url = %outcome.destination_url,
                    attempt = outcome.attempt_number,
                    last_error = ?outcome.last_error,
                    "Notification failed after max attempts"
                );
            }
        },
        Ok(None) => {
            tracing::debug!("No due notification delivery records");
        }
        Err(e) => {
            tracing::error!(error = %e, "Notification delivery service error");
        }
    }

    Ok(())
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
