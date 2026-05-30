use axum::Router;

pub mod v1;

pub fn build_router() -> Router {
    Router::new().nest("/api/v1", v1::routes())
}
