use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{AuditRecord, AuditRecordFilter};
use crate::service;

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    actor_id: Option<Uuid>,
    actor_type: Option<String>,
    resource_type: Option<String>,
    resource_id: Option<String>,
    action: Option<String>,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    items: Vec<AuditRecord>,
    limit: i64,
    offset: i64,
}

fn invalid_param(field: &'static str) -> shared_http::error::AppError {
    let mut errors = validator::ValidationErrors::new();
    errors.add(field, validator::ValidationError::new("invalid"));
    shared_http::error::AppError::Validation(errors)
}

async fn list_audit_records(
    State(pool): State<PgPool>,
    Query(query): Query<AuditListQuery>,
) -> Result<Json<AuditListResponse>, shared_http::error::AppError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    if limit < 1 || limit > 200 {
        return Err(invalid_param("limit"));
    }
    if offset < 0 {
        return Err(invalid_param("offset"));
    }

    let filter = AuditRecordFilter {
        actor_id: query.actor_id,
        actor_type: query.actor_type,
        resource_type: query.resource_type,
        resource_id: query.resource_id,
        action: query.action,
        occurred_from: query.occurred_from,
        occurred_to: query.occurred_to,
        limit,
        offset,
    };

    let items = service::list_audit_records(&pool, &filter)
        .await
        .map_err(|e| shared_http::error::AppError::Internal(e.to_string()))?;

    Ok(Json(AuditListResponse {
        items,
        limit,
        offset,
    }))
}

async fn get_audit_record(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<AuditRecord>, shared_http::error::AppError> {
    let record = service::get_audit_record(&pool, id)
        .await
        .map_err(|e| shared_http::error::AppError::Internal(e.to_string()))?;

    match record {
        Some(r) => Ok(Json(r)),
        None => Err(shared_http::error::AppError::NotFound(
            "Audit record not found".into(),
        )),
    }
}

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(list_audit_records))
        .route("/{id}", get(get_audit_record))
        .with_state(pool)
}
