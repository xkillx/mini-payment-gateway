use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_notifications() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("notifications.list")
    }
    async fn retry_notification() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("notifications.retry")
    }

    Router::new()
        .route("/", axum::routing::get(list_notifications))
        .route("/{id}/retry", axum::routing::post(retry_notification))
}
