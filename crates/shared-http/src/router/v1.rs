use axum::{routing::get, Router};

use crate::error::AppError;

fn not_implemented(module: &'static str) -> impl axum::response::IntoResponse {
    AppError::NotImplemented(module)
}

async fn health() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

async fn handle_payments() -> impl axum::response::IntoResponse {
    not_implemented("payments")
}

async fn handle_refunds() -> impl axum::response::IntoResponse {
    not_implemented("refunds")
}

async fn handle_notifications() -> impl axum::response::IntoResponse {
    not_implemented("notifications")
}

async fn handle_reconciliation() -> impl axum::response::IntoResponse {
    not_implemented("reconciliation")
}

async fn handle_audit() -> impl axum::response::IntoResponse {
    not_implemented("audit")
}

async fn handle_reporting() -> impl axum::response::IntoResponse {
    not_implemented("reporting")
}

async fn handle_admin() -> impl axum::response::IntoResponse {
    not_implemented("admin")
}

pub fn routes() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/payments", get(handle_payments).post(handle_payments))
        .route("/refunds", get(handle_refunds).post(handle_refunds))
        .route(
            "/notifications",
            get(handle_notifications).post(handle_notifications),
        )
        .route(
            "/reconciliation",
            get(handle_reconciliation).post(handle_reconciliation),
        )
        .route("/audit", get(handle_audit).post(handle_audit))
        .route("/reporting", get(handle_reporting).post(handle_reporting))
        .route("/admin", get(handle_admin).post(handle_admin))
}
