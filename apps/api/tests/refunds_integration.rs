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
use payments::service::{process_payment, PaymentProcessingOutcome};

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

fn new_uuid_v7() -> Uuid {
    Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
}

fn new_idempotency_key() -> String {
    format!("test-key-{}", new_uuid_v7())
}

async fn send_request(
    app: &mut Router,
    method: axum::http::Method,
    uri: &str,
    token: &str,
    idempotency_key: Option<&str>,
    body: Option<serde_json::Value>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json");

    if let Some(key) = idempotency_key {
        builder = builder.header("Idempotency-Key", key);
    }

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

async fn create_successful_payment(pool: &PgPool) -> Uuid {
    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD"
        })),
    )
    .await;

    assert_eq!(
        status,
        axum::http::StatusCode::CREATED,
        "failed to create payment for test setup: {body}"
    );

    let payment_id_str = body["id"].as_str().unwrap();
    let payment_uuid = Uuid::parse_str(payment_id_str).unwrap();

    process_payment(
        pool,
        payment_uuid,
        PaymentProcessingOutcome::simulated_success(),
    )
    .await
    .expect("process_payment should succeed");

    payment_uuid
}

async fn create_successful_payment_for_merchant(pool: &PgPool, token: &str) -> Uuid {
    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        token,
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD"
        })),
    )
    .await;

    assert_eq!(
        status,
        axum::http::StatusCode::CREATED,
        "failed to create payment for test setup: {body}"
    );

    let payment_id_str = body["id"].as_str().unwrap();
    let payment_uuid = Uuid::parse_str(payment_id_str).unwrap();

    process_payment(
        pool,
        payment_uuid,
        PaymentProcessingOutcome::simulated_success(),
    )
    .await
    .expect("process_payment should succeed");

    payment_uuid
}

#[tokio::test]
async fn merchant_creates_refund_for_successful_payment_returns_201() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["payment_id"], payment_id.to_string());
    assert_eq!(body["amount_minor"], 1000);
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["status"], "pending");
    assert!(!body["id"].as_str().unwrap().is_empty());
    assert!(!body["created_at"].as_str().unwrap().is_empty());
    assert!(!body["updated_at"].as_str().unwrap().is_empty());
    assert!(body.get("merchant_id").is_none());
    assert!(body.get("idempotency_key").is_none());

    let refund_id = body["id"].as_str().unwrap().to_string();
    let refund_uuid = Uuid::parse_str(&refund_id).unwrap();

    let row: (Uuid, Uuid, Uuid, i64, String, String, String) = sqlx::query_as(
        r#"SELECT id, payment_id, merchant_id, amount_minor, currency, status::text AS status, idempotency_key
           FROM refunds WHERE id = $1"#,
    )
    .bind(refund_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0.to_string(), refund_id);
    assert_eq!(row.1, payment_id);
    assert_eq!(row.2.to_string(), MERCHANT_ACTOR_ID);
    assert_eq!(row.3, 1000);
    assert_eq!(row.4, "USD");
    assert_eq!(row.5, "pending");
    assert_eq!(row.6, idem_key);

    let audit_rows: Vec<(String, String, String, Option<String>, String)> = sqlx::query_as(
        r#"SELECT action, resource_type, resource_id, actor_id::text, actor_type FROM audit_records
           WHERE resource_type = 'refund' AND resource_id = $1"#,
    )
    .bind(&refund_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audit_rows.len(), 1);
    assert_eq!(audit_rows[0].0, "refund.created");
    assert_eq!(audit_rows[0].1, "refund");
    assert_eq!(audit_rows[0].2, refund_id);
    assert_eq!(audit_rows[0].3.as_deref(), Some(MERCHANT_ACTOR_ID));
    assert_eq!(audit_rows[0].4, "merchant");

    let event_rows: Vec<(String, String, String, i32)> = sqlx::query_as(
        r#"SELECT event_type, aggregate_type, aggregate_id::text, version FROM domain_events
           WHERE aggregate_type = 'refund' AND aggregate_id = $1"#,
    )
    .bind(refund_uuid)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(event_rows.len(), 1);
    assert_eq!(event_rows[0].0, "refund.created");
    assert_eq!(event_rows[0].1, "refund");
    assert_eq!(event_rows[0].2, refund_id);
    assert_eq!(event_rows[0].3, 1);

    let (payload, row_created_at): (serde_json::Value, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as(
            r#"SELECT payload, created_at FROM domain_events
               WHERE aggregate_type = 'refund' AND aggregate_id = $1"#,
        )
        .bind(refund_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    let payload_obj = payload.as_object().expect("payload must be an object");
    let mut payload_keys: Vec<&str> = payload_obj.keys().map(|s| s.as_str()).collect();
    payload_keys.sort();
    assert_eq!(
        payload_keys,
        vec![
            "amount_minor",
            "created_at",
            "currency",
            "payment_id",
            "refund_id"
        ],
        "refund.created payload must contain exactly the documented schema v1 fields"
    );
    assert_eq!(payload["refund_id"], refund_id);
    assert_eq!(payload["payment_id"], payment_id.to_string());
    assert_eq!(payload["amount_minor"], 1000);
    assert_eq!(payload["currency"], "USD");
    let payload_created_at = chrono::DateTime::parse_from_rfc3339(
        payload["created_at"]
            .as_str()
            .expect("created_at must be a string"),
    )
    .expect("payload.created_at must be RFC3339")
    .with_timezone(&chrono::Utc);
    assert!(
        (payload_created_at - row_created_at)
            .num_milliseconds()
            .abs()
            <= 1,
        "payload.created_at must equal event row created_at within 1ms"
    );

    let payment_status: String =
        sqlx::query_scalar("SELECT status::text FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(payment_status, "successful");

    let notification_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notification_delivery_records WHERE domain_event_id IN (SELECT id FROM domain_events WHERE aggregate_type = 'refund' AND aggregate_id = $1)",
    )
    .bind(refund_uuid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(notification_count, 0);
}

#[tokio::test]
async fn create_refund_replay_returns_200_without_side_effects() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (status1, body1) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status1, axum::http::StatusCode::CREATED);
    let refund_id = body1["id"].as_str().unwrap().to_string();

    let refund_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refunds WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let audit_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_records WHERE resource_id = $1")
            .bind(&refund_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let event_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM domain_events WHERE aggregate_id::text = $1")
            .bind(&refund_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (status2, body2) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::OK);
    assert_eq!(body2["id"], refund_id);

    let refund_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refunds WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refund_count_after, refund_count_before);

    let audit_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_records WHERE resource_id = $1")
            .bind(&refund_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit_count_after, audit_count_before);

    let event_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM domain_events WHERE aggregate_id::text = $1")
            .bind(&refund_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(event_count_after, event_count_before);
}

#[tokio::test]
async fn same_idempotency_key_different_payment_returns_409() {
    let pool = setup_db().await;
    let payment_a = create_successful_payment(&pool).await;
    let payment_b = create_successful_payment(&pool).await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (s1, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_a.to_string()
        })),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let (s2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_b.to_string()
        })),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CONFLICT);

    let refund_b_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refunds WHERE payment_id = $1")
            .bind(payment_b)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refund_b_count, 0);
}

#[tokio::test]
async fn same_payment_different_idempotency_key_returns_409() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let (s1, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let (s2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CONFLICT);

    let refund_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM refunds WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(refund_count, 1);
}

#[tokio::test]
async fn refund_wrong_merchant_payment_returns_404() {
    let pool = setup_db().await;

    let merchant2_pool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM actors WHERE id = '10000000-0000-0000-0000-000000000001')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    if !merchant2_pool {
        sqlx::query(
            "INSERT INTO actors (id, name, email, role, merchant_id, is_active) VALUES ($1, 'merchant2', 'merchant2@test.invalid', 'merchant', $2, true)",
        )
        .bind(Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap())
        .bind(Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap())
        .execute(&pool)
        .await
        .unwrap();
    }

    let payment_id = create_successful_payment_for_merchant(&pool, &merchant2_token()).await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refund_missing_payment_returns_404() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refund_non_successful_payment_returns_409() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let payment_id = body["id"].as_str().unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (status2, body2) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::CONFLICT);
    assert!(body2["message"]
        .as_str()
        .unwrap()
        .contains("only successful payments can be refunded"));
}

#[tokio::test]
async fn admin_cannot_create_refund_returns_403() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &admin_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn missing_idempotency_key_returns_422() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        None,
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn unknown_body_field_returns_422() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string(),
            "amount_minor": 500
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn refund_currency_unknown_field_returns_422() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string(),
            "currency": "EUR"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn missing_auth_returns_401() {
    let pool = setup_db().await;

    let app = build_app(pool.clone()).await;
    let builder = Request::builder()
        .method(axum::http::Method::POST)
        .uri("/api/v1/refunds")
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", new_idempotency_key());

    let payload = json!({
        "payment_id": Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string()
    });
    let req = builder
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_refund_returns_not_implemented() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!(
            "/api/v1/refunds/{}",
            Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
        ),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn list_refunds_returns_not_implemented() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_IMPLEMENTED);
}
