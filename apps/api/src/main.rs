use axum::Router;
use shared_config::AppConfig;
use shared_db as db;
use shared_observability as observability;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use api::state::AppState;

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();
    observability::init(&config.log_level);

    let pool = db::connect(&config.database_url).await;
    let state = AppState {
        config: config.clone(),
        pool,
    };

    let app = Router::new()
        .merge(api::router::build(state))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(shared_http::middleware::request_id_layer())
        .layer(shared_http::middleware::propagate_request_id_layer());

    let addr = format!("0.0.0.0:{}", config.app_port);
    tracing::info!("API server starting on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
