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

async fn get_rejected_audit(
    pool: &PgPool,
    payment_id: Uuid,
) -> Vec<(String, Option<String>, String, serde_json::Value)> {
    sqlx::query_as(
        r#"SELECT action, actor_id::text, actor_type, details
           FROM audit_records
           WHERE action = 'refund.rejected'
             AND resource_type = 'payment'
             AND resource_id = $1
           ORDER BY created_at"#,
    )
    .bind(payment_id.to_string())
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn count_rejected_audit(pool: &PgPool, payment_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM audit_records
           WHERE action = 'refund.rejected'
             AND resource_type = 'payment'
             AND resource_id = $1"#,
    )
    .bind(payment_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn count_refund_domain_events_for_payment(pool: &PgPool, payment_id: Uuid) -> i64 {
    sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM domain_events
           WHERE aggregate_type = 'refund'
           AND payload->>'payment_id' = $1"#,
    )
    .bind(payment_id.to_string())
    .fetch_one(pool)
    .await
    .unwrap()
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

    let rejected_count: i64 = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
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

    let rejected = get_rejected_audit(&pool, payment_b).await;
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, "refund.rejected");
    assert_eq!(rejected[0].1.as_deref(), Some(MERCHANT_ACTOR_ID));
    assert_eq!(rejected[0].2, "merchant");
    assert_eq!(rejected[0].3["rejection_code"], "idempotency_key_conflict");
    assert_eq!(rejected[0].3["existing_payment_id"], payment_a.to_string());

    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_a).await,
        1
    );
    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_b).await,
        0
    );
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

    let rejected = get_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, "refund.rejected");
    assert_eq!(rejected[0].3["rejection_code"], "duplicate_refund");
    assert_eq!(
        rejected[0].3["rejection_reason"],
        "Payment already has a refund"
    );
    assert!(!rejected[0].3["existing_refund_id"]
        .as_str()
        .unwrap()
        .is_empty());

    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_id).await,
        1
    );
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

    let rejected = get_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, "refund.rejected");
    assert_eq!(rejected[0].3["rejection_code"], "payment_not_found");
    assert_eq!(rejected[0].3["rejection_reason"], "Payment not found");
}

#[tokio::test]
async fn refund_wrong_merchant_payment_no_domain_event() {
    let pool = setup_db().await;

    let merchant2_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors WHERE id = '10000000-0000-0000-0000-000000000001')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    if !merchant2_exists {
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

    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_id).await,
        0
    );
}

#[tokio::test]
async fn refund_missing_payment_returns_404() {
    let pool = setup_db().await;
    let payment_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

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

    let rejected = get_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, "refund.rejected");
    assert_eq!(rejected[0].3["rejection_code"], "payment_not_found");

    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_id).await,
        0
    );
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

    let payment_uuid = Uuid::parse_str(payment_id).unwrap();
    let rejected = get_rejected_audit(&pool, payment_uuid).await;
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].0, "refund.rejected");
    assert_eq!(rejected[0].3["rejection_code"], "invalid_payment_status");
    assert_eq!(rejected[0].3["payment_status"], "pending");

    assert_eq!(
        count_refund_domain_events_for_payment(&pool, payment_uuid).await,
        0
    );
}

#[tokio::test]
async fn admin_cannot_create_refund_returns_403() {
    let pool = setup_db().await;
    let payment_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &admin_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);

    let rejected_count = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
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

    let rejected_count = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
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

    let rejected_count = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
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

    let rejected_count = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
}

#[tokio::test]
async fn missing_auth_returns_401() {
    let pool = setup_db().await;
    let payment_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));

    let app = build_app(pool.clone()).await;
    let builder = Request::builder()
        .method(axum::http::Method::POST)
        .uri("/api/v1/refunds")
        .header("Content-Type", "application/json")
        .header("Idempotency-Key", new_idempotency_key());

    let payload = json!({
        "payment_id": payment_id.to_string()
    });
    let req = builder
        .body(Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

    let rejected_count = count_rejected_audit(&pool, payment_id).await;
    assert_eq!(rejected_count, 0);
}

async fn ensure_merchant2_actor(pool: &PgPool) {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors WHERE id = '10000000-0000-0000-0000-000000000001')",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    if !exists {
        sqlx::query(
            "INSERT INTO actors (id, name, email, role, merchant_id, is_active) VALUES ($1, 'merchant2', 'merchant2@test.invalid', 'merchant', $2, true)",
        )
        .bind(Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap())
        .bind(Uuid::parse_str("10000000-0000-0000-0000-000000000001").unwrap())
        .execute(pool)
        .await
        .unwrap();
    }
}

async fn _create_refund(pool: &PgPool, token: &str) -> (Uuid, Uuid, serde_json::Value) {
    let payment_id = create_successful_payment_for_merchant(pool, token).await;
    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        token,
        Some(&idem_key),
        Some(json!({
            "payment_id": payment_id.to_string()
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let refund_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    (refund_id, payment_id, body)
}

#[tokio::test]
async fn merchant_list_refunds_returns_own_refunds() {
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
    let refund_id = body["id"].as_str().unwrap().to_string();

    let (status, list_body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(list_body["limit"], 50);
    assert_eq!(list_body["offset"], 0);
    assert!(list_body["items"].is_array());

    let items = list_body["items"].as_array().unwrap();
    let found = items.iter().any(|item| item["id"] == refund_id);
    assert!(found, "merchant list should include own refund");

    let refund_item = items.iter().find(|item| item["id"] == refund_id).unwrap();
    assert_eq!(refund_item["payment_id"], payment_id.to_string());
    assert_eq!(refund_item["amount_minor"], 1000);
    assert_eq!(refund_item["currency"], "USD");
    assert_eq!(refund_item["status"], "pending");
    assert!(refund_item["merchant_id"].is_string());
    assert!(refund_item.get("idempotency_key").is_none());
    assert!(refund_item.get("created_at").is_some());
    assert!(refund_item.get("updated_at").is_some());
}

#[tokio::test]
async fn admin_list_refunds_returns_refunds_across_merchants() {
    let pool = setup_db().await;
    ensure_merchant2_actor(&pool).await;

    let payment1_id = create_successful_payment(&pool).await;
    let payment2_id = create_successful_payment_for_merchant(&pool, &merchant2_token()).await;

    let mut app1 = build_app(pool.clone()).await;
    let idem_key1 = new_idempotency_key();
    let (s1, _b1) = send_request(
        &mut app1,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key1),
        Some(json!({"payment_id": payment1_id.to_string()})),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let idem_key2 = new_idempotency_key();
    let (s2, b2) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant2_token(),
        Some(&idem_key2),
        Some(json!({"payment_id": payment2_id.to_string()})),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CREATED);
    let _merchant2_refund_id = b2["id"].as_str().unwrap().to_string();
}
#[tokio::test]
async fn admin_list_refunds_filters_by_merchant_id() {
    let pool = setup_db().await;
    ensure_merchant2_actor(&pool).await;

    let payment1_id = create_successful_payment(&pool).await;
    let payment2_id = create_successful_payment_for_merchant(&pool, &merchant2_token()).await;

    let mut app1 = build_app(pool.clone()).await;
    let (s1, _b1) = send_request(
        &mut app1,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment1_id.to_string()})),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let idem_key2 = new_idempotency_key();
    let (s2, b2) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant2_token(),
        Some(&idem_key2),
        Some(json!({"payment_id": payment2_id.to_string()})),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CREATED);
    let _merchant2_refund_id = b2["id"].as_str().unwrap().to_string();

    let mut app = build_app(pool.clone()).await;
    let (status, list_body) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/refunds?merchant_id={}", MERCHANT_ACTOR_ID),
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    for item in items.iter() {
        assert_eq!(
            item["merchant_id"], MERCHANT_ACTOR_ID,
            "filtered list should only contain merchant1 refunds"
        );
    }
}

#[tokio::test]
async fn merchant_list_with_merchant_id_returns_403() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds?merchant_id=00000000-0000-0000-0000-000000000001",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn merchant_list_with_malformed_merchant_id_returns_403() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds?merchant_id=not-a-uuid",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_refunds_pagination_orders_newest_first_and_respects_limit_offset() {
    let pool = setup_db().await;

    let payment_id1 = create_successful_payment(&pool).await;
    let payment_id2 = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let mut refund_ids = Vec::new();

    for (_idx, payment_id) in [payment_id1, payment_id2].iter().enumerate() {
        let (s, b) = send_request(
            &mut app,
            axum::http::Method::POST,
            "/api/v1/refunds",
            &merchant_token(),
            Some(&new_idempotency_key()),
            Some(json!({"payment_id": payment_id.to_string()})),
        )
        .await;
        assert_eq!(s, axum::http::StatusCode::CREATED);
        refund_ids.push(b["id"].as_str().unwrap().to_string());
    }

    sqlx::query("UPDATE refunds SET created_at = $1 WHERE id = $2")
        .bind(chrono::Utc::now() - chrono::Duration::seconds(60))
        .bind(Uuid::parse_str(&refund_ids[0]).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE refunds SET created_at = $1 WHERE id = $2")
        .bind(chrono::Utc::now())
        .bind(Uuid::parse_str(&refund_ids[1]).unwrap())
        .execute(&pool)
        .await
        .unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (status, list_body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        "/api/v1/refunds?limit=50&offset=0",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();

    let refund_positions: Vec<usize> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| refund_ids.contains(&item["id"].as_str().unwrap().to_string()))
        .map(|(i, _)| i)
        .collect();
    assert!(
        refund_positions[0] < refund_positions[1],
        "newer refund should come before older refund in sorted output"
    );
}

#[tokio::test]
async fn list_refunds_status_filter_returns_matching_status() {
    let pool = setup_db().await;

    let payment_id = create_successful_payment(&pool).await;
    let mut app = build_app(pool.clone()).await;

    let (s, b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::CREATED);
    let refund_id = b["id"].as_str().unwrap().to_string();
    let refund_uuid = Uuid::parse_str(&refund_id).unwrap();

    sqlx::query("UPDATE refunds SET status = 'completed', updated_at = NOW() WHERE id = $1")
        .bind(refund_uuid)
        .execute(&pool)
        .await
        .unwrap();

    let (status, list_body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds?status=completed",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    for item in items.iter() {
        assert_eq!(
            item["status"], "completed",
            "status filter should only return completed refunds"
        );
    }
}

#[tokio::test]
async fn list_refunds_invalid_limit_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    for limit in &["0", "201", "abc"] {
        let (status, _) = send_request(
            &mut app,
            axum::http::Method::GET,
            &format!("/api/v1/refunds?limit={limit}"),
            &merchant_token(),
            None,
            None,
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[tokio::test]
async fn list_refunds_invalid_offset_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds?offset=-1",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn list_refunds_invalid_status_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/refunds?status=unknown",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn merchant_get_refund_detail_returns_200() {
    let pool = setup_db().await;

    let payment_id = create_successful_payment(&pool).await;
    let mut app = build_app(pool.clone()).await;

    let (s, b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::CREATED);
    let refund_id = b["id"].as_str().unwrap().to_string();

    let (status, detail) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/refunds/{refund_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(detail["id"], refund_id);
    assert_eq!(detail["payment_id"], payment_id.to_string());
    assert_eq!(detail["amount_minor"], 1000);
    assert_eq!(detail["currency"], "USD");
    assert_eq!(detail["status"], "pending");
    assert_eq!(detail["merchant_id"], MERCHANT_ACTOR_ID);
    assert!(detail.get("idempotency_key").is_none());
    assert!(detail.get("created_at").is_some());
    assert!(detail.get("updated_at").is_some());
}

#[tokio::test]
async fn admin_get_refund_detail_returns_200() {
    let pool = setup_db().await;

    let payment_id = create_successful_payment(&pool).await;
    let mut app = build_app(pool.clone()).await;

    let (s, b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::CREATED);
    let refund_id = b["id"].as_str().unwrap().to_string();

    let (status, detail) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/refunds/{refund_id}"),
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(detail["id"], refund_id);
    assert_eq!(detail["merchant_id"], MERCHANT_ACTOR_ID);
}

#[tokio::test]
async fn merchant_get_refund_detail_other_merchant_returns_404() {
    let pool = setup_db().await;
    ensure_merchant2_actor(&pool).await;

    let payment_id = create_successful_payment_for_merchant(&pool, &merchant2_token()).await;
    let mut app = build_app(pool.clone()).await;

    let (s, b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant2_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::CREATED);
    let refund_id = b["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &format!("/api/v1/refunds/{refund_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_refund_missing_id_returns_404() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let missing_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/refunds/{missing_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn refund_detail_response_includes_merchant_id_and_excludes_idempotency_key() {
    let pool = setup_db().await;

    let payment_id = create_successful_payment(&pool).await;
    let mut app = build_app(pool.clone()).await;

    let (s, b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(s, axum::http::StatusCode::CREATED);
    let refund_id = b["id"].as_str().unwrap().to_string();

    let (status, detail) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/refunds/{refund_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(detail.get("merchant_id").is_some());
    assert!(detail.get("idempotency_key").is_none());
    assert!(
        detail.get("status").is_some(),
        "read status is the Refund Status"
    );
}

#[tokio::test]
async fn direct_sql_duplicate_payment_id_refund_fails_on_unique_index() {
    let pool = setup_db().await;
    let payment_id = create_successful_payment(&pool).await;

    let refund_id_1 = new_uuid_v7();
    let refund_id_2 = new_uuid_v7();
    let now = chrono::Utc::now();
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let key1 = new_idempotency_key();
    let key2 = new_idempotency_key();

    sqlx::query(
        r#"INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8)"#,
    )
    .bind(refund_id_1)
    .bind(payment_id)
    .bind(merchant_uuid)
    .bind(1000_i64)
    .bind("USD")
    .bind(&key1)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let result = sqlx::query(
        r#"INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, 'pending', $6, $7, $8)"#,
    )
    .bind(refund_id_2)
    .bind(payment_id)
    .bind(merchant_uuid)
    .bind(1000_i64)
    .bind("USD")
    .bind(&key2)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "second direct INSERT for same payment_id must fail on unique index"
    );
}
