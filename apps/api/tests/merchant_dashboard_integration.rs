use axum::body::Body;
use axum::http::Request;
use axum::Router;
use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use tower::ServiceExt;

use api::router;

const MERCHANT_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const SECRET: &str = "dev-secret-change-in-production";

#[derive(serde::Serialize)]
struct TestClaims {
    sub: String,
    role: String,
    merchant_id: Option<String>,
    exp: usize,
}

fn make_token(claims: TestClaims) -> String {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
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

fn merchant2_token() -> String {
    make_token(TestClaims {
        sub: "10000000-0000-0000-0000-000000000001".into(),
        role: "merchant".into(),
        merchant_id: Some("10000000-0000-0000-0000-000000000001".into()),
        exp: 9999999999,
    })
}

fn admin_token() -> String {
    make_token(TestClaims {
        sub: "00000000-0000-0000-0000-000000000002".into(),
        role: "administrator".into(),
        merchant_id: None,
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
    db::connect(&database_url).await
}

async fn send_request(
    app: &mut Router,
    method: axum::http::Method,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json");

    let body_bytes = body
        .map(|b| serde_json::to_vec(&b).unwrap())
        .unwrap_or_default();

    let response = app
        .oneshot(builder.body(Body::from(body_bytes)).unwrap())
        .await
        .unwrap();

    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), 10_000_000)
        .await
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

#[tokio::test]
async fn merchant_dashboard_returns_summary_with_counts_and_recent_items() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert!(body.get("configured_currency").is_some());
    assert!(body.get("generated_at").is_some());

    let payment_overview = body.get("payment_overview").unwrap();
    let status_counts = payment_overview.get("status_counts").unwrap();
    assert!(status_counts.get("pending").is_some());
    assert!(status_counts.get("processing").is_some());
    assert!(status_counts.get("successful").is_some());
    assert!(status_counts.get("failed").is_some());
    assert!(status_counts.get("refunded").is_some());

    assert!(payment_overview.get("recent_payments").unwrap().is_array());

    let refund_overview = body.get("refund_overview").unwrap();
    let refund_counts = refund_overview.get("status_counts").unwrap();
    assert!(refund_counts.get("pending").is_some());
    assert!(refund_counts.get("processing").is_some());
    assert!(refund_counts.get("completed").is_some());
    assert!(refund_counts.get("failed").is_some());

    assert!(refund_overview.get("recent_refunds").unwrap().is_array());
}

#[tokio::test]
async fn merchant_dashboard_zero_fills_missing_statuses() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let status_counts = &body["payment_overview"]["status_counts"];
    assert!(status_counts["pending"].as_i64().unwrap() >= 0);
    assert!(status_counts["processing"].as_i64().unwrap() >= 0);
    assert!(status_counts["successful"].as_i64().unwrap() >= 0);
    assert!(status_counts["failed"].as_i64().unwrap() >= 0);
    assert!(status_counts["refunded"].as_i64().unwrap() >= 0);

    let refund_counts = &body["refund_overview"]["status_counts"];
    assert!(refund_counts["pending"].as_i64().unwrap() >= 0);
    assert!(refund_counts["processing"].as_i64().unwrap() >= 0);
    assert!(refund_counts["completed"].as_i64().unwrap() >= 0);
    assert!(refund_counts["failed"].as_i64().unwrap() >= 0);
}

#[tokio::test]
async fn merchant_dashboard_excludes_other_merchant_data() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let recent_payments = body["payment_overview"]["recent_payments"]
        .as_array()
        .unwrap();
    for payment in recent_payments {
        if let Some(mid) = payment.get("merchant_id").and_then(|v| v.as_str()) {
            assert_eq!(mid, MERCHANT_ACTOR_ID);
        }
    }

    let recent_refunds = body["refund_overview"]["recent_refunds"]
        .as_array()
        .unwrap();
    for refund in recent_refunds {
        if let Some(mid) = refund.get("merchant_id").and_then(|v| v.as_str()) {
            assert_eq!(mid, MERCHANT_ACTOR_ID);
        }
    }
}

#[tokio::test]
async fn administrator_token_returns_403() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 403);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn missing_token_returns_401() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let builder = Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/dashboard/merchant")
        .header("Content-Type", "application/json");

    let response = app
        .oneshot(builder.body(Body::from(vec![])).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn merchant_dashboard_shows_configured_currency() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["configured_currency"], "USD");
}

#[tokio::test]
async fn merchant_dashboard_has_generated_at_timestamp() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    let generated_at = body["generated_at"].as_str().unwrap();
    assert!(!generated_at.is_empty());
}

#[tokio::test]
async fn different_merchants_see_only_their_data() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status_m1, _body_m1) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant_token(),
        None,
    )
    .await;
    assert_eq!(status_m1, 200);

    let (status_m2, body_m2) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/merchant",
        &merchant2_token(),
        None,
    )
    .await;
    assert_eq!(status_m2, 200);

    let m2_payments = body_m2["payment_overview"]["recent_payments"]
        .as_array()
        .unwrap();
    for payment in m2_payments {
        let mid = payment["merchant_id"].as_str().unwrap();
        assert_eq!(mid, "10000000-0000-0000-0000-000000000001");
    }
}
