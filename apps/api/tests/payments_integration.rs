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

#[tokio::test]
async fn merchant_creates_payment_returns_201() {
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
            "currency": "USD",
            "metadata": {"order_ref": "ORD-123"}
        })),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(body["amount_minor"], 1000);
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["metadata"]["order_ref"], "ORD-123");
    assert!(body["id"].as_str().unwrap().len() > 0);
    assert!(body["created_at"].as_str().unwrap().len() > 0);
    assert!(body["updated_at"].as_str().unwrap().len() > 0);
    assert!(body.get("merchant_id").is_none());
    assert!(body.get("idempotency_key").is_none());

    let payment_id = body["id"].as_str().unwrap().to_string();
    let payment_uuid = Uuid::parse_str(&payment_id).unwrap();

    let row: (Uuid,) = sqlx::query_as("SELECT merchant_id FROM payments WHERE id = $1")
        .bind(payment_uuid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0.to_string(), MERCHANT_ACTOR_ID);

    let audit_rows: Vec<(String, String, String)> = sqlx::query_as(
        r#"SELECT action, resource_type, resource_id FROM audit_records
           WHERE resource_type = 'payment' AND resource_id = $1"#,
    )
    .bind(&payment_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audit_rows.len(), 1);
    assert_eq!(audit_rows[0].0, "payment.created");
    assert_eq!(audit_rows[0].1, "payment");
    assert_eq!(audit_rows[0].2, payment_id);

    let event_rows: Vec<(String, String, String, i32)> = sqlx::query_as(
        r#"SELECT event_type, aggregate_type, aggregate_id::text, version FROM domain_events
           WHERE aggregate_type = 'payment' AND aggregate_id = $1"#,
    )
    .bind(payment_uuid)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(event_rows.len(), 1);
    assert_eq!(event_rows[0].0, "payment.created");
    assert_eq!(event_rows[0].1, "payment");
    assert_eq!(event_rows[0].2, payment_id);
    assert_eq!(event_rows[0].3, 1);
}

#[tokio::test]
async fn exact_idempotent_replay_returns_200() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (status1, body1) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 500,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(status1, axum::http::StatusCode::CREATED);
    let payment_id = body1["id"].as_str().unwrap().to_string();

    let payment_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM payments WHERE idempotency_key = $1")
            .bind(&idem_key)
            .fetch_one(&pool)
            .await
            .unwrap();

    let audit_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_records WHERE resource_id = $1")
            .bind(&payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let event_count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM domain_events WHERE aggregate_id::text = $1")
            .bind(&payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (status2, body2) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 500,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::OK);
    assert_eq!(body2["id"], payment_id);
    assert_eq!(body2["amount_minor"], 500);

    let payment_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM payments WHERE idempotency_key = $1")
            .bind(&idem_key)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(payment_count_after, payment_count_before);

    let audit_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_records WHERE resource_id = $1")
            .bind(&payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(audit_count_after, audit_count_before);

    let event_count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM domain_events WHERE aggregate_id::text = $1")
            .bind(&payment_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(event_count_after, event_count_before);
}

#[tokio::test]
async fn same_key_different_metadata_returns_409() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (s1, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"ref": "A"}
        })),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let (s2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"ref": "B"}
        })),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn same_key_different_amount_returns_409() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (s1, _) = send_request(
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
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let (s2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 2000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn missing_idempotency_key_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        None,
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn amount_minor_zero_or_negative_returns_422() {
    let pool = setup_db().await;

    for amount in &[0i64, -1] {
        let mut app = build_app(pool.clone()).await;
        let (status, _) = send_request(
            &mut app,
            axum::http::Method::POST,
            "/api/v1/payments",
            &merchant_token(),
            Some(&new_idempotency_key()),
            Some(json!({
                "amount_minor": amount,
                "currency": "USD"
            })),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "amount_minor={} should be rejected",
            amount
        );
    }
}

#[tokio::test]
async fn unsupported_currency_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "EUR"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn unknown_body_field_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "amount": 10.00
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn metadata_as_array_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": [1, 2, 3]
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn metadata_scalar_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": "not-an-object"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn oversized_metadata_returns_422() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let large_value = "x".repeat(5000);
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"data": large_value}
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn administrator_create_payment_returns_403() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &admin_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn metadata_key_order_does_not_affect_replay() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (s1, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"a": 1, "b": 2}
        })),
    )
    .await;
    assert_eq!(s1, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let (s2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(serde_json::json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"b": 2, "a": 1}
        })),
    )
    .await;
    assert_eq!(s2, axum::http::StatusCode::OK);
}

fn second_merchant_actor_id() -> String {
    new_uuid_v7().to_string()
}

fn second_merchant_token(actor_id: &str) -> String {
    make_token(TestClaims {
        sub: actor_id.into(),
        role: "merchant".into(),
        merchant_id: Some(actor_id.into()),
        exp: 9999999999,
    })
}

async fn insert_second_merchant_actor(pool: &PgPool, actor_id: &str) {
    let id = Uuid::parse_str(actor_id).unwrap();
    sqlx::query(
        r#"
        INSERT INTO actors (id, name, email, role, is_active, merchant_id)
        VALUES ($1, $2, $3, 'merchant', true, $4)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            email = EXCLUDED.email,
            role = EXCLUDED.role,
            is_active = EXCLUDED.is_active,
            merchant_id = EXCLUDED.merchant_id
        "#,
    )
    .bind(id)
    .bind(format!("Merchant {actor_id}"))
    .bind(format!("merchant-{actor_id}@example.com"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn merchant_gets_own_payment_detail() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (create_status, create_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1500,
            "currency": "USD",
            "metadata": {"order_ref": "ORD-1"}
        })),
    )
    .await;
    assert_eq!(create_status, axum::http::StatusCode::CREATED);
    let payment_id = create_body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{payment_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    assert_eq!(body["id"], payment_id);
    assert_eq!(body["merchant_id"], MERCHANT_ACTOR_ID);
    assert_eq!(body["amount_minor"], 1500);
    assert_eq!(body["currency"], "USD");
    assert_eq!(body["status"], "pending");
    assert_eq!(body["metadata"]["order_ref"], "ORD-1");
    assert!(body["idempotency_key"].is_null());

    let history = body["status_history"].as_array().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["status"], "pending");
    assert_eq!(history[0]["source_event_type"], "payment.created");
    assert_eq!(history[0]["domain_event_id"].as_str().unwrap().len(), 36);
    assert!(history[0]["occurred_at"].as_str().is_some());

    assert_eq!(body["refunds"].as_array().unwrap().len(), 0);
    assert_eq!(
        body["notification_delivery_records"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn administrator_gets_any_payment_detail() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (create_status, create_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 2000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(create_status, axum::http::StatusCode::CREATED);
    let payment_id = create_body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{payment_id}"),
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["id"], payment_id);
    assert_eq!(body["merchant_id"], MERCHANT_ACTOR_ID);
}

#[tokio::test]
async fn missing_payment_returns_404_for_merchant() {
    let pool = setup_db().await;
    let missing_id = new_uuid_v7();

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{missing_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_payment_returns_404_for_administrator() {
    let pool = setup_db().await;
    let missing_id = new_uuid_v7();

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{missing_id}"),
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn cross_merchant_payment_returns_404() {
    let pool = setup_db().await;
    let other_actor_id = second_merchant_actor_id();
    insert_second_merchant_actor(&pool, &other_actor_id).await;

    let idem_key = new_idempotency_key();
    let other_token = second_merchant_token(&other_actor_id);

    let mut app = build_app(pool.clone()).await;
    let (create_status, create_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &other_token,
        Some(&idem_key),
        Some(json!({
            "amount_minor": 3000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(create_status, axum::http::StatusCode::CREATED);
    let payment_id = create_body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{payment_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn detail_includes_refunds_refunded_history_and_notifications() {
    let pool = setup_db().await;
    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (create_status, create_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 4000,
            "currency": "USD",
            "metadata": {"order_ref": "ORD-DETAIL"}
        })),
    )
    .await;
    assert_eq!(create_status, axum::http::StatusCode::CREATED);
    let payment_id = create_body["id"].as_str().unwrap().to_string();
    let payment_uuid = Uuid::parse_str(&payment_id).unwrap();

    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let refund_id = new_uuid_v7();
    let refund_idem = format!("refund-idem-{}", new_uuid_v7());
    let now = chrono::Utc::now();
    sqlx::query(
        r#"
        INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7, $7)
        "#,
    )
    .bind(refund_id)
    .bind(payment_uuid)
    .bind(merchant_uuid)
    .bind(4000i64)
    .bind("USD")
    .bind(&refund_idem)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let payment_event_id: Uuid =
        sqlx::query_scalar("SELECT id FROM domain_events WHERE aggregate_type = 'payment' AND aggregate_id = $1 LIMIT 1")
            .bind(payment_uuid)
            .fetch_one(&pool)
            .await
            .unwrap();

    let refund_completed_event_id = new_uuid_v7();
    sqlx::query(
        r#"
        INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
        VALUES ($1, 'refund.completed', 'refund', $2, $3, 1, $4)
        "#,
    )
    .bind(refund_completed_event_id)
    .bind(refund_id)
    .bind(serde_json::json!({"refund_id": refund_id.to_string(), "payment_id": payment_id}))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let notif_id_for_payment = new_uuid_v7();
    let notif_id_for_refund = new_uuid_v7();
    sqlx::query(
        r#"
        INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, next_retry_at, created_at, updated_at)
        VALUES ($1, $2, 'https://merchant.example/webhook', 'pending', 0, NULL, NULL, $3, $3)
        "#,
    )
    .bind(notif_id_for_payment)
    .bind(payment_event_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, next_retry_at, created_at, updated_at)
        VALUES ($1, $2, 'https://merchant.example/webhook', 'pending', 0, NULL, NULL, $3, $3)
        "#,
    )
    .bind(notif_id_for_refund)
    .bind(refund_completed_event_id)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &format!("/api/v1/payments/{payment_id}"),
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let refunds = body["refunds"].as_array().unwrap();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0]["id"], refund_id.to_string());
    assert_eq!(refunds[0]["payment_id"], payment_id);
    assert_eq!(refunds[0]["amount_minor"], 4000);
    assert_eq!(refunds[0]["currency"], "USD");
    assert_eq!(refunds[0]["status"], "completed");
    assert!(refunds[0]["idempotency_key"].is_null());

    let history = body["status_history"].as_array().unwrap();
    let statuses: Vec<&str> = history
        .iter()
        .map(|e| e["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, vec!["pending", "refunded"]);
    let sources: Vec<&str> = history
        .iter()
        .map(|e| e["source_event_type"].as_str().unwrap())
        .collect();
    assert_eq!(sources, vec!["payment.created", "refund.completed"]);

    let notifs = body["notification_delivery_records"].as_array().unwrap();
    assert_eq!(notifs.len(), 2);
    let event_types: Vec<&str> = notifs
        .iter()
        .map(|n| n["event_type"].as_str().unwrap())
        .collect();
    assert!(event_types.contains(&"payment.created"));
    assert!(event_types.contains(&"refund.completed"));
    for n in notifs {
        assert!(n["idempotency_key"].is_null());
        assert_eq!(n["destination_url"], "https://merchant.example/webhook");
        assert_eq!(n["status"], "pending");
    }
}

#[tokio::test]
async fn merchant_list_defaults_returns_only_own_payments() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (own_status, own_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"merchant_reference": "OWN-REF-1"}
        })),
    )
    .await;
    assert_eq!(own_status, axum::http::StatusCode::CREATED);
    let own_payment_id = own_body["id"].as_str().unwrap().to_string();

    let other_actor_id = second_merchant_actor_id();
    insert_second_merchant_actor(&pool, &other_actor_id).await;
    let other_token = second_merchant_token(&other_actor_id);

    let mut app2 = build_app(pool.clone()).await;
    let (other_status, other_body) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &other_token,
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 2000,
            "currency": "USD"
        })),
    )
    .await;
    assert_eq!(other_status, axum::http::StatusCode::CREATED);
    let other_payment_id = other_body["id"].as_str().unwrap().to_string();

    let mut app3 = build_app(pool.clone()).await;
    let (list_status, list_body) = send_request(
        &mut app3,
        axum::http::Method::GET,
        "/api/v1/payments",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);

    assert_eq!(list_body["limit"], 50);
    assert_eq!(list_body["offset"], 0);
    let items = list_body["items"].as_array().unwrap();
    let returned_ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(returned_ids.contains(&own_payment_id.as_str()));
    assert!(!returned_ids.contains(&other_payment_id.as_str()));
    for item in items {
        assert_eq!(item["merchant_id"], MERCHANT_ACTOR_ID);
        assert!(item.get("idempotency_key").is_none());
    }
}

#[tokio::test]
async fn administrator_list_returns_payments_across_merchants() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (own_status, own_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 1000, "currency": "USD"})),
    )
    .await;
    assert_eq!(own_status, axum::http::StatusCode::CREATED);
    let own_payment_id = own_body["id"].as_str().unwrap().to_string();

    let other_actor_id = second_merchant_actor_id();
    insert_second_merchant_actor(&pool, &other_actor_id).await;
    let other_token = second_merchant_token(&other_actor_id);

    let mut app2 = build_app(pool.clone()).await;
    let (other_status, other_body) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &other_token,
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 2000, "currency": "USD"})),
    )
    .await;
    assert_eq!(other_status, axum::http::StatusCode::CREATED);
    let other_payment_id = other_body["id"].as_str().unwrap().to_string();

    let mut app3 = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app3,
        axum::http::Method::GET,
        "/api/v1/payments",
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let items = body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&own_payment_id.as_str()));
    assert!(ids.contains(&other_payment_id.as_str()));
}

#[tokio::test]
async fn administrator_list_filters_by_merchant_id() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (own_status, own_body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 1000, "currency": "USD"})),
    )
    .await;
    assert_eq!(own_status, axum::http::StatusCode::CREATED);
    let own_payment_id = own_body["id"].as_str().unwrap().to_string();

    let other_actor_id = second_merchant_actor_id();
    insert_second_merchant_actor(&pool, &other_actor_id).await;
    let other_token = second_merchant_token(&other_actor_id);

    let mut app2 = build_app(pool.clone()).await;
    let (other_status, other_body) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &other_token,
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 2000, "currency": "USD"})),
    )
    .await;
    assert_eq!(other_status, axum::http::StatusCode::CREATED);
    let other_payment_id = other_body["id"].as_str().unwrap().to_string();

    let mut app3 = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?merchant_id={MERCHANT_ACTOR_ID}");
    let (status, body) = send_request(
        &mut app3,
        axum::http::Method::GET,
        &uri,
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let items = body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&own_payment_id.as_str()));
    assert!(!ids.contains(&other_payment_id.as_str()));
    for item in items {
        assert_eq!(item["merchant_id"], MERCHANT_ACTOR_ID);
    }
}

#[tokio::test]
async fn merchant_list_with_merchant_id_query_returns_403() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?merchant_id={MERCHANT_ACTOR_ID}");
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);

    let mut app2 = build_app(pool.clone()).await;
    let uri2 = "/api/v1/payments?merchant_id=not-a-uuid";
    let (status2, _) = send_request(
        &mut app2,
        axum::http::Method::GET,
        uri2,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_pagination_orders_newest_first_and_respects_limit_offset() {
    let pool = setup_db().await;

    let base = chrono::Utc::now() + chrono::Duration::seconds(3600);
    let mut created_ids: Vec<String> = Vec::new();
    for i in 0..3 {
        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::POST,
            "/api/v1/payments",
            &merchant_token(),
            Some(&new_idempotency_key()),
            Some(json!({
                "amount_minor": 1000 + i as i64,
                "currency": "USD"
            })),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::CREATED);
        let id = body["id"].as_str().unwrap().to_string();
        created_ids.push(id.clone());

        let created_at = base + chrono::Duration::milliseconds(i as i64 * 10);
        let id_uuid = Uuid::parse_str(&id).unwrap();
        sqlx::query("UPDATE payments SET created_at = $1, updated_at = $1 WHERE id = $2")
            .bind(created_at)
            .bind(id_uuid)
            .execute(&pool)
            .await
            .unwrap();
    }

    let mut app = build_app(pool.clone()).await;
    let uri = "/api/v1/payments?limit=1&offset=1";
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["limit"], 1);
    assert_eq!(body["offset"], 1);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], created_ids[1]);
}

#[tokio::test]
async fn list_status_filter_returns_only_matching_status() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status1, body1) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 1000, "currency": "USD"})),
    )
    .await;
    assert_eq!(status1, axum::http::StatusCode::CREATED);
    let pending_id = body1["id"].as_str().unwrap().to_string();

    let (status2, body2) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 2000, "currency": "USD"})),
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::CREATED);
    let successful_id = body2["id"].as_str().unwrap().to_string();
    let successful_uuid = Uuid::parse_str(&successful_id).unwrap();

    sqlx::query("UPDATE payments SET status = 'successful', updated_at = NOW() WHERE id = $1")
        .bind(successful_uuid)
        .execute(&pool)
        .await
        .unwrap();

    let mut app2 = build_app(pool.clone()).await;
    let (list_status, list_body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        "/api/v1/payments?status=successful",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&successful_id.as_str()));
    assert!(!ids.contains(&pending_id.as_str()));
    for item in items {
        assert_eq!(item["status"], "successful");
    }
}

#[tokio::test]
async fn list_search_by_exact_payment_id_returns_that_payment() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({"amount_minor": 1000, "currency": "USD"})),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let target_id = body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?search={target_id}");
    let (list_status, list_body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], target_id);
}

#[tokio::test]
async fn list_search_by_case_insensitive_merchant_reference() {
    let pool = setup_db().await;

    let unique = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext))
        .to_string()
        .replace('-', "");
    let upper_ref = format!("ORD-{unique}").to_uppercase();
    let lower_search = format!("ord-{unique}").to_lowercase();

    let mut app = build_app(pool.clone()).await;
    let (status1, body1) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"merchant_reference": upper_ref}
        })),
    )
    .await;
    assert_eq!(status1, axum::http::StatusCode::CREATED);
    let target_id = body1["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let (status2, _) = send_request(
        &mut app2,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 2000,
            "currency": "USD",
            "metadata": {"merchant_reference": "OTHER-REF"}
        })),
    )
    .await;
    assert_eq!(status2, axum::http::StatusCode::CREATED);

    let mut app3 = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?search={lower_search}");
    let (list_status, list_body) = send_request(
        &mut app3,
        axum::http::Method::GET,
        &uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&target_id.as_str()));
}

#[tokio::test]
async fn list_search_with_uuid_text_matches_merchant_reference() {
    let pool = setup_db().await;

    let uuid_like_ref = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)).to_string();

    let mut app = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {"merchant_reference": uuid_like_ref}
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let target_id = body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?search={uuid_like_ref}");
    let (list_status, list_body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&target_id.as_str()));
}

#[tokio::test]
async fn list_search_ignores_idempotency_key_and_arbitrary_metadata() {
    let pool = setup_db().await;

    let idem_key = new_idempotency_key();

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {
                "merchant_reference": "REAL-REF-001",
                "order_ref": "IDEMKEY123",
                "tag": "value"
            }
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    let mut app2 = build_app(pool.clone()).await;
    let uri_idem = format!("/api/v1/payments?search={idem_key}");
    let (status_idem, body_idem) = send_request(
        &mut app2,
        axum::http::Method::GET,
        &uri_idem,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status_idem, axum::http::StatusCode::OK);
    assert_eq!(body_idem["items"].as_array().unwrap().len(), 0);

    let mut app3 = build_app(pool.clone()).await;
    let uri_arb = "/api/v1/payments?search=IDEMKEY123";
    let (status_arb, body_arb) = send_request(
        &mut app3,
        axum::http::Method::GET,
        uri_arb,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status_arb, axum::http::StatusCode::OK);
    assert_eq!(body_arb["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_search_ignores_non_string_merchant_reference() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&new_idempotency_key()),
        Some(json!({
            "amount_minor": 1000,
            "currency": "USD",
            "metadata": {
                "merchant_reference": 12345,
                "real_ref": "ABCXYZ"
            }
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let target_id = body["id"].as_str().unwrap().to_string();

    let mut app2 = build_app(pool.clone()).await;
    let uri = "/api/v1/payments?search=12345";
    let (list_status, list_body) = send_request(
        &mut app2,
        axum::http::Method::GET,
        uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status, axum::http::StatusCode::OK);
    let items = list_body["items"].as_array().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(!ids.contains(&target_id.as_str()));

    let mut app3 = build_app(pool.clone()).await;
    let uri2 = "/api/v1/payments?search=ABCXYZ";
    let (list_status2, list_body2) = send_request(
        &mut app3,
        axum::http::Method::GET,
        uri2,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(list_status2, axum::http::StatusCode::OK);
    let items2 = list_body2["items"].as_array().unwrap();
    let ids2: Vec<&str> = items2.iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert!(!ids2.contains(&target_id.as_str()));
}

#[tokio::test]
async fn list_invalid_pagination_returns_422() {
    let pool = setup_db().await;

    for (uri, label) in [
        ("/api/v1/payments?limit=0", "limit=0"),
        ("/api/v1/payments?limit=201", "limit=201"),
        ("/api/v1/payments?offset=-1", "offset=-1"),
        ("/api/v1/payments?limit=abc", "limit=abc"),
    ] {
        let mut app = build_app(pool.clone()).await;
        let (status, _) = send_request(
            &mut app,
            axum::http::Method::GET,
            uri,
            &merchant_token(),
            None,
            None,
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "{label} should be rejected"
        );
    }
}

#[tokio::test]
async fn list_invalid_status_returns_422() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/payments?status=paid",
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn list_search_too_long_returns_422() {
    let pool = setup_db().await;

    let too_long = "a".repeat(129);
    let mut app = build_app(pool.clone()).await;
    let uri = format!("/api/v1/payments?search={too_long}");
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        &uri,
        &merchant_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn administrator_list_with_malformed_merchant_id_returns_422() {
    let pool = setup_db().await;

    let mut app = build_app(pool.clone()).await;
    let (status, _) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/payments?merchant_id=not-a-uuid",
        &admin_token(),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}
