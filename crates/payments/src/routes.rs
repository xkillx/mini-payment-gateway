use axum::Router;
use shared_http::error::AppError;

async fn list_payments() -> impl axum::response::IntoResponse {
    AppError::NotImplemented("payments.list")
}

async fn create_payment() -> impl axum::response::IntoResponse {
    AppError::NotImplemented("payments.create")
}

async fn get_payment() -> impl axum::response::IntoResponse {
    AppError::NotImplemented("payments.get")
}

pub fn routes() -> Router {
    Router::new()
        .route("/", axum::routing::get(list_payments).post(create_payment))
        .route("/{id}", axum::routing::get(get_payment))
}
