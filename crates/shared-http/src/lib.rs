pub mod error;
pub mod middleware;
pub mod router;

use axum::routing::MethodRouter;
use axum::Router;

pub fn module_routes(prefix: &str, routes: Vec<(&'static str, MethodRouter)>) -> Router {
    let mut builder = Router::new();
    for (path, handler) in routes {
        builder = builder.route(path, handler);
    }
    Router::new().nest(prefix, builder)
}
