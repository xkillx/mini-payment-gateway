use axum::{
    extract::{Path, Query, State},
    Extension, Router,
};
use serde::Deserialize;
use shared_http::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{NotificationListFilter, NotificationListResponse};
use crate::repository::{NotificationRepository, PostgresNotificationRepository};

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", axum::routing::get(list_notifications))
        .route("/{id}", axum::routing::get(get_notification_detail))
        .route("/{id}/retry", axum::routing::post(retry_notification))
        .with_state(pool)
}

#[derive(Deserialize)]
struct ListQueryParams {
    status: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list_notifications(
    State(pool): State<PgPool>,
    Query(params): Query<ListQueryParams>,
) -> Result<axum::Json<NotificationListResponse>, AppError> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    if !(1..=200).contains(&limit) {
        return Err(AppError::Validation(validation_error(
            "limit",
            "range",
            "limit must be between 1 and 200",
        )));
    }
    if offset < 0 {
        return Err(AppError::Validation(validation_error(
            "offset",
            "range",
            "offset must be >= 0",
        )));
    }

    let status = match params.status.as_deref() {
        None => None,
        Some("pending") => Some(crate::models::NotificationStatus::Pending),
        Some("processing") => Some(crate::models::NotificationStatus::Processing),
        Some("delivered") => Some(crate::models::NotificationStatus::Delivered),
        Some("failed") => Some(crate::models::NotificationStatus::Failed),
        Some(_) => {
            return Err(AppError::Validation(validation_error(
                "status",
                "invalid",
                "status must be one of: pending, processing, delivered, failed",
            )));
        }
    };

    let filter = NotificationListFilter {
        status,
        limit,
        offset,
    };

    let repo = PostgresNotificationRepository::new(pool);
    let items = repo
        .list_details(&filter)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

    Ok(axum::Json(NotificationListResponse {
        items,
        limit,
        offset,
    }))
}

async fn get_notification_detail(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<axum::Json<crate::models::NotificationDeliveryRecordDetailResponse>, AppError> {
    let repo = PostgresNotificationRepository::new(pool);
    let detail = repo
        .find_detail_by_id(id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

    detail
        .map(axum::Json)
        .ok_or_else(|| AppError::NotFound(format!("Notification {} not found", id)))
}

async fn retry_notification(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Extension(actor): Extension<shared_auth::Actor>,
) -> Result<axum::Json<crate::models::NotificationDeliveryRecordDetailResponse>, AppError> {
    let admin_actor_id = match actor {
        shared_auth::Actor::Administrator { actor_id } => actor_id,
        _ => return Err(AppError::Forbidden("Administrator role required".into())),
    };

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

    let updated = PostgresNotificationRepository::retry_failed_in_tx(&mut tx, id)
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

    let record = match updated {
        Some(r) => r,
        None => {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM notification_delivery_records WHERE id = $1)",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

            if !exists {
                return Err(AppError::NotFound(format!("Notification {} not found", id)));
            }
            return Err(AppError::Conflict(
                "Notification is not in failed status".into(),
            ));
        }
    };

    let audit_record = audit::service::new_audit_record(
        Some(admin_actor_id),
        audit::models::ActorType::ADMINISTRATOR,
        audit::models::actions::NOTIFICATION_RETRY_REQUESTED,
        "notification_delivery_record",
        id.to_string(),
        Some(serde_json::json!({
            "previous_status": "failed",
            "previous_attempt_count": record.attempt_count,
            "previous_retry_generation": record.retry_generation - 1,
            "new_retry_generation": record.retry_generation,
            "previous_last_error": record.last_error,
            "previous_last_attempt_at": record.last_attempt_at,
            "previous_next_retry_at": record.next_retry_at,
        })),
    );

    audit::service::record_required_in_tx(&mut tx, audit_record)
        .await
        .map_err(|e| AppError::Internal(format!("Audit error: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| AppError::Internal(format!("Database error: {e}")))?;

    Ok(axum::Json(record))
}

fn validation_error(
    field: &'static str,
    code: &'static str,
    message: &'static str,
) -> validator::ValidationErrors {
    let mut errors = validator::ValidationErrors::new();
    let mut ve = validator::ValidationError::new(code);
    ve.message = Some(std::borrow::Cow::Borrowed(message));
    errors.add(field, ve);
    errors
}
