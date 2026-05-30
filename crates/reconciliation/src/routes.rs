use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_reconciliations() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("reconciliation.list")
    }
    async fn run_reconciliation() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("reconciliation.run")
    }

    Router::new().route(
        "/",
        axum::routing::get(list_reconciliations).post(run_reconciliation),
    )
}
