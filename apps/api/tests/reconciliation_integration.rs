use std::sync::OnceLock;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use serde_json::json;
use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};
use tower::ServiceExt;
use uuid::Uuid;

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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, value)
}

static TEST_DAY: AtomicU32 = AtomicU32::new(0);
static EPOCH: OnceLock<u64> = OnceLock::new();

fn next_test_day() -> u32 {
    TEST_DAY.fetch_add(1, Ordering::SeqCst) + 1
}

fn epoch() -> u64 {
    *EPOCH.get_or_init(|| (chrono::Utc::now().timestamp_millis() % 6000) as u64)
}

fn epoch_year() -> u64 {
    2100 + epoch()
}

fn test_window(day: u32) -> (String, String) {
    let year = epoch_year() + day as u64;
    (
        format!("{}-01-01T00:00:00Z", year),
        format!("{}-01-02T00:00:00Z", year),
    )
}

fn test_event_at(_day: u32) -> String {
    let year = epoch_year() + _day as u64;
    format!("{}-01-01T12:00:00Z", year)
}

async fn seed_payment_successful(
    pool: &PgPool,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    event_created_at: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'successful', $5, NOW(), NOW())
        "#,
    )
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(format!("ik-{}", payment_id))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, 'payment.successful', 'payment', $2, $3, 1, $4)
        "#,
    )
    .bind(new_uuid_v7())
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

async fn seed_payment_failed(
    pool: &PgPool,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    event_created_at: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'failed', $5, NOW(), NOW())
        "#,
    )
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(format!("ik-{}", payment_id))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, 'payment.failed', 'payment', $2, $3, 1, $4)
        "#,
    )
    .bind(new_uuid_v7())
    .bind(payment_id)
    .bind(
        json!({
            "payment_id": payment_id.to_string(),
            "merchant_id": MERCHANT_ACTOR_ID,
            "amount_minor": amount_minor,
            "currency": currency,
            "failure_reason": "test failure",
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

async fn seed_payment_pending(pool: &PgPool, payment_id: Uuid, amount_minor: i64, currency: &str) {
    sqlx::query(
        r#"
        INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'pending', $5, NOW(), NOW())
        "#,
    )
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(format!("ik-{}", payment_id))
    .execute(pool)
    .await
    .unwrap();
}

async fn seed_refund_completed(
    pool: &PgPool,
    refund_id: Uuid,
    payment_id: Uuid,
    amount_minor: i64,
    currency: &str,
    event_created_at: &str,
) {
    sqlx::query(
        r#"
        INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'completed', $6, NOW(), NOW())
        "#,
    )
    .bind(refund_id)
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(amount_minor)
    .bind(currency)
    .bind(format!("ik-{}", refund_id))
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, 'refund.completed', 'refund', $2, $3, 1, $4)
        "#,
    )
    .bind(new_uuid_v7())
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

async fn has_audit_record_for_reconciliation(pool: &PgPool, reconciliation_id: &str) -> bool {
    let count: (Option<i64>,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM audit_records
           WHERE resource_id = $1 AND action = 'reconciliation.executed'"#,
    )
    .bind(reconciliation_id)
    .fetch_one(pool)
    .await
    .unwrap();
    count.0.unwrap_or(0) > 0
}

async fn reconciliation_exists(pool: &PgPool, id: &str) -> bool {
    let count: (Option<i64>,) =
        sqlx::query_as("SELECT COUNT(*) FROM reconciliations WHERE id::text = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    count.0.unwrap_or(0) > 0
}

fn r(s: &str) -> String {
    s.to_string()
}

#[tokio::test]
async fn admin_matched_reconciliation_returns_201() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["status"], "matched");
    assert_eq!(body["expected_total_minor"], 1000);
    assert_eq!(body["actual_total_minor"], 1000);
    assert_eq!(body["discrepancy_minor"], 0);
    assert_eq!(body["currency"], "USD");
    assert!(body["id"].as_str().unwrap().len() > 0);
    assert!(body["run_at"].as_str().unwrap().len() > 0);
    assert!(body["created_at"].as_str().unwrap().len() > 0);

    let reconciliation_id = r(body["id"].as_str().unwrap());
    assert!(reconciliation_exists(&pool, &reconciliation_id).await);
    assert!(has_audit_record_for_reconciliation(&pool, &reconciliation_id).await);
}

#[tokio::test]
async fn admin_mismatched_reconciliation_returns_201() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1500
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["status"], "mismatched");
    assert_eq!(body["expected_total_minor"], 1000);
    assert_eq!(body["actual_total_minor"], 1500);
    assert_eq!(body["discrepancy_minor"], 500);

    let reconciliation_id = r(body["id"].as_str().unwrap());

    let audit_rows: Vec<(String, String, String, String, serde_json::Value)> = sqlx::query_as(
        r#"SELECT action, resource_type, resource_id, actor_type, details
           FROM audit_records
           WHERE resource_id = $1 AND action = 'reconciliation.executed'"#,
    )
    .bind(&reconciliation_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audit_rows.len(), 1);
    assert_eq!(audit_rows[0].0, "reconciliation.executed");
    assert_eq!(audit_rows[0].1, "reconciliation");
    assert_eq!(audit_rows[0].2, reconciliation_id);
    assert_eq!(audit_rows[0].3, "administrator");
    assert_eq!(audit_rows[0].4["discrepancy_minor"], 500);
}

#[tokio::test]
async fn payment_at_window_start_is_included() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 500, "USD", &ws).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 500
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 500);
}

#[tokio::test]
async fn payment_at_window_end_is_excluded() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 500, "USD", &we).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 0);
    assert_eq!(body["status"], "matched");
}

#[tokio::test]
async fn completed_refund_in_window_is_subtracted() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 2000, "USD", &test_event_at(day)).await;

    let refund_id = new_uuid_v7();
    seed_refund_completed(
        &pool,
        refund_id,
        payment_id,
        2000,
        "USD",
        &test_event_at(day),
    )
    .await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 0);
    assert_eq!(body["status"], "matched");
}

#[tokio::test]
async fn refund_only_window_produces_negative_expected_total() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 2000, "USD", "1999-01-01T12:00:00Z").await;

    let refund_id = new_uuid_v7();
    seed_refund_completed(
        &pool,
        refund_id,
        payment_id,
        2000,
        "USD",
        &test_event_at(day),
    )
    .await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": -2000
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], -2000);
    assert_eq!(body["status"], "matched");
}

#[tokio::test]
async fn refunded_payment_counted_by_success_event() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    sqlx::query("UPDATE payments SET status = 'refunded' WHERE id = $1")
        .bind(payment_id)
        .execute(&pool)
        .await
        .unwrap();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 1000);
}

#[tokio::test]
async fn pending_and_failed_payments_are_ignored() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let pending_id = new_uuid_v7();
    seed_payment_pending(&pool, pending_id, 500, "USD").await;

    let failed_id = new_uuid_v7();
    seed_payment_failed(&pool, failed_id, 300, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 0);
}

#[tokio::test]
async fn merchant_token_returns_403() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &merchant_token(),
        Some(json!({
            "currency": "USD",
            "window_start": "2100-06-05T00:00:00Z",
            "window_end": "2100-06-06T00:00:00Z",
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn invalid_currency_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "EUR",
            "window_start": "2100-06-05T00:00:00Z",
            "window_end": "2100-06-06T00:00:00Z",
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["details"]["currency"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn window_start_not_before_window_end_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": "2100-06-06T00:00:00Z",
            "window_end": "2100-06-05T00:00:00Z",
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "VALIDATION_ERROR");
    assert!(body["details"]["window_end"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn window_start_equal_window_end_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": "2100-06-05T00:00:00Z",
            "window_end": "2100-06-05T00:00:00Z",
            "actual_total_minor": 0
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn duplicate_run_creates_two_reconciliations() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status1, body1) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000
        })),
    )
    .await;

    assert_eq!(status1, axum::http::StatusCode::CREATED);

    let (status2, body2) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 2000
        })),
    )
    .await;

    assert_eq!(status2, axum::http::StatusCode::CREATED);
    assert_ne!(body1["id"], body2["id"]);

    let id1 = r(body1["id"].as_str().unwrap());
    let id2 = r(body2["id"].as_str().unwrap());
    assert!(reconciliation_exists(&pool, &id1).await);
    assert!(reconciliation_exists(&pool, &id2).await);
    assert!(has_audit_record_for_reconciliation(&pool, &id1).await);
    assert!(has_audit_record_for_reconciliation(&pool, &id2).await);
}

#[tokio::test]
async fn response_status_is_lowercase() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    let status_str = body["status"].as_str().unwrap();
    assert!(
        status_str == "matched" || status_str == "mismatched" || status_str == "error",
        "expected lowercase status, got: {status_str}"
    );
}

#[tokio::test]
async fn negative_actual_total_is_accepted() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 500, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": -100
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["actual_total_minor"], -100);
    assert_eq!(body["discrepancy_minor"], -600);
}

#[tokio::test]
async fn notes_appear_in_response() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": "2100-06-05T00:00:00Z",
            "window_end": "2100-06-06T00:00:00Z",
            "actual_total_minor": 0,
            "notes": "External statement reconciliation"
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["notes"], "External statement reconciliation");
}

#[tokio::test]
async fn refund_created_event_not_counted_only_refund_completed() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 2000, "USD", &test_event_at(day)).await;

    let refund_id = new_uuid_v7();
    sqlx::query(
        r#"
        INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'pending', $6, NOW(), NOW())
        "#,
    )
    .bind(refund_id)
    .bind(payment_id)
    .bind(Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap())
    .bind(2000i64)
    .bind("USD")
    .bind(format!("ik-{}", refund_id))
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, 'refund.created', 'refund', $2, $3, 1, $4)
        "#,
    )
    .bind(new_uuid_v7())
    .bind(refund_id)
    .bind(
        json!({
            "refund_id": refund_id.to_string(),
            "payment_id": payment_id.to_string(),
            "merchant_id": MERCHANT_ACTOR_ID,
            "amount_minor": 2000,
            "currency": "USD",
        }),
    )
    .bind(
        chrono::DateTime::parse_from_rfc3339(&test_event_at(day))
            .unwrap()
            .with_timezone(&chrono::Utc),
    )
    .execute(&pool)
    .await
    .unwrap();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 2000
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 2000);
}

#[tokio::test]
async fn multiple_payments_in_window_are_summed() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    for _ in 0..3 {
        let payment_id = new_uuid_v7();
        seed_payment_successful(&pool, payment_id, 500, "USD", &test_event_at(day)).await;
    }

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1500
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 1500);
    assert_eq!(body["status"], "matched");
}

#[tokio::test]
async fn missing_body_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn unknown_request_fields_are_ignored() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000,
            "unknown_field": "should be ignored"
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["expected_total_minor"], 1000);
}

#[tokio::test]
async fn reconciliation_audit_record_has_correct_details() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let day = next_test_day();
    let (ws, we) = test_window(day);

    let payment_id = new_uuid_v7();
    seed_payment_successful(&pool, payment_id, 1000, "USD", &test_event_at(day)).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/reconciliation",
        &admin_token(),
        Some(json!({
            "currency": "USD",
            "window_start": ws,
            "window_end": we,
            "actual_total_minor": 1000,
            "notes": "Q1 audit"
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    let reconciliation_id = r(body["id"].as_str().unwrap());

    let audit_rows: Vec<(Option<Uuid>, String, String, String, serde_json::Value)> =
        sqlx::query_as(
            r#"SELECT actor_id, actor_type, action, resource_type, details
               FROM audit_records
               WHERE resource_id = $1 AND action = 'reconciliation.executed'"#,
        )
        .bind(&reconciliation_id)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(audit_rows.len(), 1);
    assert_eq!(
        audit_rows[0].0.unwrap().to_string(),
        "00000000-0000-0000-0000-000000000002"
    );
    assert_eq!(audit_rows[0].1, "administrator");
    assert_eq!(audit_rows[0].2, "reconciliation.executed");
    assert_eq!(audit_rows[0].3, "reconciliation");
    assert_eq!(audit_rows[0].4["currency"], "USD");
    assert_eq!(audit_rows[0].4["expected_total_minor"], 1000);
    assert_eq!(audit_rows[0].4["actual_total_minor"], 1000);
    assert_eq!(audit_rows[0].4["discrepancy_minor"], 0);
    assert_eq!(audit_rows[0].4["notes"], "Q1 audit");
}
