use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_payments() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("payments.list")
    }
    async fn create_payment() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("payments.create")
    }
    async fn get_payment() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("payments.get")
    }

    Router::new()
        .route("/", axum::routing::get(list_payments).post(create_payment))
        .route("/{id}", axum::routing::get(get_payment))
}
