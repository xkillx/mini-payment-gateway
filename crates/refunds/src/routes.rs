use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_refunds() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("refunds.list")
    }
    async fn create_refund() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("refunds.create")
    }
    async fn get_refund() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("refunds.get")
    }

    Router::new()
        .route("/", axum::routing::get(list_refunds).post(create_refund))
        .route("/{id}", axum::routing::get(get_refund))
}
