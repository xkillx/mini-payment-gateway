use axum::{routing::get, Router};
use shared_auth::Role;

use crate::auth_layer;
use crate::state::AppState;

async fn health() -> impl axum::response::IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": "0.1.0"
    }))
}

fn role_guard(router: Router, roles: Vec<Role>) -> Router {
    router.layer(axum::middleware::from_fn(move |req, next| {
        let roles = roles.clone();
        auth_layer::require_role(req, next, roles)
    }))
}

fn merchant_scoped_guard(router: Router) -> Router {
    router.layer(axum::middleware::from_fn(auth_layer::merchant_write_guard))
}

pub fn build(state: AppState) -> Router {
    let merchant_or_admin = vec![Role::Merchant, Role::Administrator];
    let admin_only = vec![Role::Administrator];

    let payments = merchant_scoped_guard(payments::routes::routes(
        state.pool.clone(),
        state.config.payment_currency.clone(),
    ));
    let payments = role_guard(payments, merchant_or_admin.clone());

    let refunds = merchant_scoped_guard(refunds::routes::routes(state.pool.clone()));
    let refunds = role_guard(refunds, merchant_or_admin);

    let admin_routes = role_guard(admin::routes::routes(), admin_only.clone());
    let notifications_routes = role_guard(notifications::routes::routes(), admin_only.clone());
    let reconciliation_routes = role_guard(reconciliation::routes::routes(), admin_only.clone());
    let audit_routes = role_guard(
        audit::routes::routes(state.pool.clone()),
        admin_only.clone(),
    );
    let reporting_routes = role_guard(reporting::routes::routes(), admin_only);

    let v1_router = Router::new()
        .nest("/payments", payments)
        .nest("/refunds", refunds)
        .nest("/notifications", notifications_routes)
        .nest("/reconciliation", reconciliation_routes)
        .nest("/audit", audit_routes)
        .nest("/reporting", reporting_routes)
        .nest("/admin", admin_routes)
        .layer(axum::middleware::from_fn(auth_layer::auth_middleware))
        .layer(axum::Extension(state));

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", v1_router)
}
