use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn payment_summary() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("reporting.payment_summary")
    }

    Router::new().route("/payment-summary", axum::routing::get(payment_summary))
}
