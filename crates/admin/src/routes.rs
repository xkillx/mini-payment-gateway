use axum::Router;
use shared_http::error::AppError;

pub fn routes() -> Router {
    async fn list_actors() -> impl axum::response::IntoResponse {
        AppError::NotImplemented("admin.actors.list")
    }

    Router::new().route("/actors", axum::routing::get(list_actors))
}
