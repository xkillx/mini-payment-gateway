use axum::body::Body;
use axum::http::Request;
use axum::Router;
use serde_json::json;
use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use api::router;

const ADMIN_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000002";
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

fn admin_token() -> String {
    make_token(TestClaims {
        sub: ADMIN_ACTOR_ID.into(),
        role: "administrator".into(),
        merchant_id: None,
        exp: 9999999999,
    })
}

fn merchant_token() -> String {
    make_token(TestClaims {
        sub: MERCHANT_ACTOR_ID.into(),
        role: "merchant".into(),
        merchant_id: Some(MERCHANT_ACTOR_ID.into()),
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

fn new_uuid_v7() -> Uuid {
    Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
}

async fn send_request(
    app: &mut Router,
    method: axum::http::Method,
    uri: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let builder = Request::builder()
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
    let bytes = axum::body::to_bytes(response.into_body(), 10_000_000)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

fn test_timestamps() -> (String, String) {
    (
        "2040-01-01T00:00:00Z".to_string(),
        "2040-01-04T00:00:00Z".to_string(),
    )
}

fn inside_period_at() -> String {
    "2040-01-01T12:00:00Z".to_string()
}

fn outside_period() -> String {
    "2039-12-31T12:00:00Z".to_string()
}

async fn seed_payment_with_event(
    pool: &PgPool,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    event_type: &str,
    event_created_at: &str,
    payment_created_at: &str,
    status: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        "#,
    )
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(status)
    .bind(format!("ik-{}", payment_id))
    .bind(
        chrono::DateTime::parse_from_rfc3339(payment_created_at)
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, $2, 'payment', $3, $4, 1, $5)
        "#,
    )
    .bind(new_uuid_v7())
    .bind(event_type)
    .bind(payment_id)
    .bind(
        json!({
            "payment_id": payment_id.to_string(),
            "merchant_id": MERCHANT_ACTOR_ID,
            "amount_minor": amount_minor,
            "currency": currency,
        }),
    )
    .bind(
        chrono::DateTime::parse_from_rfc3339(event_created_at)
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_refund_with_event(
    pool: &PgPool,
    refund_id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    status: &str,
    event_type: &str,
    event_created_at: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        "#,
    )
    .bind(refund_id)
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(status)
    .bind(format!("ik-{}", refund_id))
    .bind(
        chrono::DateTime::parse_from_rfc3339(event_created_at)
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, $2, 'refund', $3, $4, 1, $5)
        "#,
    )
    .bind(new_uuid_v7())
    .bind(event_type)
    .bind(refund_id)
    .bind(
        json!({
            "refund_id": refund_id.to_string(),
            "payment_id": payment_id.to_string(),
            "merchant_id": MERCHANT_ACTOR_ID,
            "amount_minor": amount_minor,
            "currency": currency,
        }),
    )
    .bind(
        chrono::DateTime::parse_from_rfc3339(event_created_at)
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_refund_without_event(
    pool: &PgPool,
    refund_id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    status: &str,
    updated_at: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        "#,
    )
    .bind(refund_id)
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(status)
    .bind(format!("ik-{}", refund_id))
    .bind(
        chrono::DateTime::parse_from_rfc3339(updated_at)
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn admin_gets_payment_report_with_all_metrics() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (from, to) = test_timestamps();

    let created_payment = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        created_payment,
        5000,
        "USD",
        "payment.created",
        &inside_period_at(),
        &inside_period_at(),
        "pending",
    )
    .await;

    let successful_payment = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        successful_payment,
        10000,
        "USD",
        "payment.successful",
        &inside_period_at(),
        &inside_period_at(),
        "successful",
    )
    .await;

    let failed_payment = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        failed_payment,
        2000,
        "USD",
        "payment.failed",
        &inside_period_at(),
        &inside_period_at(),
        "failed",
    )
    .await;

    let outside_payment = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        outside_payment,
        9999,
        "USD",
        "payment.successful",
        &outside_period(),
        &outside_period(),
        "successful",
    )
    .await;

    let completed_refund = new_uuid_v7();
    seed_refund_with_event(
        &pool,
        completed_refund,
        successful_payment,
        10000,
        "USD",
        "completed",
        "refund.completed",
        &inside_period_at(),
    )
    .await;

    let failed_refund = new_uuid_v7();
    seed_refund_without_event(
        &pool,
        failed_refund,
        successful_payment,
        0,
        "USD",
        "failed",
        &inside_period_at(),
    )
    .await;

    let uri = format!("/api/v1/reporting/payment-summary?from={from}&to={to}");
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        &uri,
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["configured_currency"], "USD");
    assert_eq!(body["period_start"], from);
    assert_eq!(body["period_end"], to);
    assert!(body["generated_at"].as_str().unwrap().len() > 0);

    let totals = &body["payment_totals"];
    assert_eq!(totals["created_count"], 3, "created payments inside period");
    assert_eq!(totals["created_amount_minor"], 17000);
    assert_eq!(totals["successful_count"], 1);
    assert_eq!(totals["successful_amount_minor"], 10000);
    assert_eq!(totals["failed_count"], 1);
    assert_eq!(totals["failed_attempted_amount_minor"], 2000);

    let refunds = &body["refund_activity"];
    assert_eq!(refunds["completed_count"], 1);
    assert_eq!(refunds["completed_amount_minor"], 10000);
    assert_eq!(refunds["failed_count"], 1);

    let trend = body["trend"].as_array().unwrap();
    assert!(
        trend.len() >= 3,
        "expected at least 3 trend buckets, got {}",
        trend.len()
    );
    for bucket in trend {
        assert!(bucket["bucket_date"].as_str().unwrap().len() > 0);
        assert!(bucket["created_count"].as_i64().is_some());
        assert!(bucket["successful_count"].as_i64().is_some());
        assert!(bucket["failed_count"].as_i64().is_some());
        assert!(bucket["successful_amount_minor"].as_i64().is_some());
    }
}

#[tokio::test]
async fn merchant_receives_403() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/reporting/payment-summary",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 403);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn missing_token_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let builder = Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/reporting/payment-summary")
        .header("Content-Type", "application/json");

    let response = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
    let bytes = axum::body::to_bytes(response.into_body(), 10_000_000)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn default_period_returns_report() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/reporting/payment-summary",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["configured_currency"], "USD");
    assert!(body["period_start"].as_str().unwrap().len() > 0);
    assert!(body["period_end"].as_str().unwrap().len() > 0);
    assert!(body["trend"].as_array().is_some());
}

#[tokio::test]
async fn empty_period_returns_zeros() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let uri = "/api/v1/reporting/payment-summary?from=2040-06-01T00:00:00Z&to=2040-06-04T00:00:00Z";
    let (status, body) =
        send_request(&mut app, axum::http::Method::GET, uri, &admin_token(), None).await;

    assert_eq!(status, 200);
    assert_eq!(body["payment_totals"]["created_count"], 0);
    assert_eq!(body["payment_totals"]["created_amount_minor"], 0);
    assert_eq!(body["payment_totals"]["successful_count"], 0);
    assert_eq!(body["payment_totals"]["failed_count"], 0);
    assert_eq!(body["refund_activity"]["completed_count"], 0);
    assert_eq!(body["refund_activity"]["failed_count"], 0);

    let trend = body["trend"].as_array().unwrap();
    assert_eq!(trend.len(), 3);
    for bucket in trend {
        assert_eq!(bucket["created_count"], 0);
        assert_eq!(bucket["successful_count"], 0);
        assert_eq!(bucket["failed_count"], 0);
    }
}

#[tokio::test]
async fn from_equal_to_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let uri = "/api/v1/reporting/payment-summary?from=2040-06-01T00:00:00Z&to=2040-06-01T00:00:00Z";
    let (status, body) =
        send_request(&mut app, axum::http::Method::GET, uri, &admin_token(), None).await;

    assert_eq!(status, 422);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn from_after_to_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let uri = "/api/v1/reporting/payment-summary?from=2040-06-02T00:00:00Z&to=2040-06-01T00:00:00Z";
    let (status, body) =
        send_request(&mut app, axum::http::Method::GET, uri, &admin_token(), None).await;

    assert_eq!(status, 422);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn period_over_366_days_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let uri = "/api/v1/reporting/payment-summary?from=2040-01-01T00:00:00Z&to=2041-01-02T00:00:00Z";
    let (status, body) =
        send_request(&mut app, axum::http::Method::GET, uri, &admin_token(), None).await;

    assert_eq!(status, 422);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn malformed_timestamp_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let uri = "/api/v1/reporting/payment-summary?from=not-a-date&to=2040-06-01T00:00:00Z";
    let (status, body) =
        send_request(&mut app, axum::http::Method::GET, uri, &admin_token(), None).await;

    assert_eq!(status, 422);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn refund_without_event_uses_updated_at_fallback() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (from, to) = test_timestamps();

    let payment_id = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        payment_id,
        5000,
        "USD",
        "payment.successful",
        &inside_period_at(),
        &inside_period_at(),
        "successful",
    )
    .await;

    let refund_id = new_uuid_v7();
    seed_refund_without_event(
        &pool,
        refund_id,
        payment_id,
        5000,
        "USD",
        "completed",
        &inside_period_at(),
    )
    .await;

    let uri = format!("/api/v1/reporting/payment-summary?from={from}&to={to}");
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        &uri,
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["refund_activity"]["completed_count"], 1);
    assert_eq!(body["refund_activity"]["completed_amount_minor"], 5000);
}

#[tokio::test]
async fn refund_outside_period_not_counted() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (from, to) = test_timestamps();

    let payment_id = new_uuid_v7();
    seed_payment_with_event(
        &pool,
        payment_id,
        5000,
        "USD",
        "payment.successful",
        &inside_period_at(),
        &inside_period_at(),
        "successful",
    )
    .await;

    let refund_id = new_uuid_v7();
    seed_refund_with_event(
        &pool,
        refund_id,
        payment_id,
        5000,
        "USD",
        "completed",
        "refund.completed",
        &outside_period(),
    )
    .await;

    let uri = format!("/api/v1/reporting/payment-summary?from={from}&to={to}");
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        &uri,
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["refund_activity"]["completed_count"], 0);
}
