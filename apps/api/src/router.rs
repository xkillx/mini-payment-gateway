use axum::{routing::get, Extension, Router};
use shared_config::AppConfig;

use crate::auth_layer;

async fn health() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

pub fn build(config: AppConfig) -> Router {
    let v1_router = Router::new()
        .nest("/payments", payments::routes::routes())
        .nest("/refunds", refunds::routes::routes())
        .nest("/notifications", notifications::routes::routes())
        .nest("/reconciliation", reconciliation::routes::routes())
        .nest("/audit", audit::routes::routes())
        .nest("/reporting", reporting::routes::routes())
        .nest("/admin", admin::routes::routes());

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", v1_router)
        .layer(axum::middleware::from_fn(auth_layer::auth_middleware))
        .layer(Extension(config))
}
