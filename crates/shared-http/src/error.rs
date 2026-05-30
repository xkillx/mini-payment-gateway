use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;
use validator::{ValidationErrors, ValidationErrorsKind};

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorEnvelope {
    pub fn new(code: impl Into<String>, message: impl Into<String>, request_id: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            request_id: request_id.to_owned(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("Validation error: {0}")]
    Validation(ValidationErrors),
    #[error("Authentication error: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let request_id = uuid::Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();
        let (status, envelope) = match self {
            AppError::NotImplemented(msg) => (
                StatusCode::NOT_IMPLEMENTED,
                ErrorEnvelope::new("NOT_IMPLEMENTED", msg, &request_id),
            ),
            AppError::Validation(errs) => {
                let details = validation_errors_to_map(&errs);
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    ErrorEnvelope::new(
                        "VALIDATION_ERROR",
                        "Request validation failed",
                        &request_id,
                    )
                    .with_details(details),
                )
            }
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorEnvelope::new("UNAUTHORIZED", msg, &request_id),
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorEnvelope::new("NOT_FOUND", msg, &request_id),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                ErrorEnvelope::new("CONFLICT", msg, &request_id),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorEnvelope::new("INTERNAL_ERROR", msg, &request_id),
            ),
        };
        (status, Json(envelope)).into_response()
    }
}

fn validation_errors_to_map(errs: &ValidationErrors) -> serde_json::Value {
    let mut map = HashMap::new();
    for (field, kind) in errs.errors() {
        match kind {
            ValidationErrorsKind::Field(errs) => {
                let msgs: Vec<String> = errs
                    .iter()
                    .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                    .collect();
                map.insert(field.to_string(), msgs);
            }
            ValidationErrorsKind::Struct(_) | ValidationErrorsKind::List(_) => {
                map.insert(field.to_string(), vec!["Invalid".to_string()]);
            }
        }
    }
    serde_json::to_value(map).unwrap_or(serde_json::Value::Null)
}
