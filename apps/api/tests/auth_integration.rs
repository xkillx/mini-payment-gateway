use axum::Router;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use tower::ServiceExt;

use api::router;

const MERCHANT_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const ADMIN_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000002";
const SECRET: &str = "dev-secret-change-in-production";

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    role: String,
    merchant_id: Option<String>,
    exp: usize,
}

fn make_token(claims: TestClaims) -> String {
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap()
}

fn merchant_token() -> String {
    make_token(TestClaims {
        sub: MERCHANT_ACTOR_ID.into(),
        role: "merchant".into(),
        merchant_id: Some(MERCHANT_ACTOR_ID.into()),
        exp: 9999999999,
    })
}

fn admin_token() -> String {
    make_token(TestClaims {
        sub: ADMIN_ACTOR_ID.into(),
        role: "administrator".into(),
        merchant_id: None,
        exp: 9999999999,
    })
}

fn expired_merchant_token() -> String {
    make_token(TestClaims {
        sub: MERCHANT_ACTOR_ID.into(),
        role: "merchant".into(),
        merchant_id: Some(MERCHANT_ACTOR_ID.into()),
        exp: 1000000000,
    })
}

fn unknown_actor_token() -> String {
    make_token(TestClaims {
        sub: "99999999-9999-9999-9999-999999999999".into(),
        role: "merchant".into(),
        merchant_id: Some("99999999-9999-9999-9999-999999999999".into()),
        exp: 9999999999,
    })
}

async fn build_app(pool: PgPool) -> Router {
    let config = AppConfig {
        jwt_secret: SECRET.into(),
        ..AppConfig::default()
    };
    let state = api::state::AppState { config, pool };
    router::build(state)
}

async fn setup_db() -> PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://mac@localhost:5432/mpg_testdb".into());
    let pool = db::connect(&database_url).await;
    pool
}

#[tokio::test]
async fn health_succeeds_without_auth() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn protected_route_without_auth_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/payments")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM audit_records WHERE action = 'auth.authentication_failed' AND resource_type = 'auth')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        exists,
        "Expected an auth.authentication_failed audit record to exist"
    );
}

#[tokio::test]
async fn malformed_token_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/payments")
                .header("Authorization", "NotBearer sometoken")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_actor_token_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = unknown_actor_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/payments")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn expired_token_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = expired_merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/payments")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn merchant_on_admin_route_returns_403() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/admin/actors")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM audit_records WHERE action = 'auth.authorization_failed' AND resource_type = 'auth')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        exists,
        "Expected an auth.authorization_failed audit record to exist"
    );
}

#[tokio::test]
async fn merchant_get_payments_reaches_handler_501() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/payments")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn admin_get_admin_actors_reaches_handler_501() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/admin/actors")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn admin_post_payments_returns_403() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/payments")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn merchant_post_payments_reaches_handler_501() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/payments")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
}
