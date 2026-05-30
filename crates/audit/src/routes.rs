use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_audit_records() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("audit.list")
    }

    Router::new().route("/", axum::routing::get(list_audit_records))
}
