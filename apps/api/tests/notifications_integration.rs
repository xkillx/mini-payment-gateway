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
use notifications::repository::{NotificationRepository, PostgresNotificationRepository};

const MERCHANT_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
const SECRET: &str = "dev-secret-change-in-production";
const PROJECTION_BATCH: i64 = 100;

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

fn unique_dest_url() -> String {
    format!("https://test-{}.example/webhook", new_uuid_v7())
}

async fn deactivate_all_destinations(pool: &PgPool, merchant_id: Uuid) {
    sqlx::query(
        "UPDATE notification_destinations SET is_active = false WHERE merchant_id = $1 AND is_active = true",
    )
    .bind(merchant_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn set_active_destination(pool: &PgPool, merchant_id: Uuid, destination_url: &str) {
    deactivate_all_destinations(pool, merchant_id).await;
    let id = new_uuid_v7();
    sqlx::query(
        "INSERT INTO notification_destinations (id, merchant_id, destination_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, true, NOW(), NOW())",
    )
    .bind(id)
    .bind(merchant_id)
    .bind(destination_url)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_inactive_destination(pool: &PgPool, merchant_id: Uuid, destination_url: &str) {
    let id = new_uuid_v7();
    sqlx::query(
        "INSERT INTO notification_destinations (id, merchant_id, destination_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, false, NOW(), NOW())",
    )
    .bind(id)
    .bind(merchant_id)
    .bind(destination_url)
    .execute(pool)
    .await
    .unwrap();
}

async fn drain_projection(pool: &PgPool) -> u64 {
    let repo = PostgresNotificationRepository::new(pool.clone());
    let mut total = 0u64;
    loop {
        let inserted = repo.project_domain_events(PROJECTION_BATCH).await.unwrap();
        if inserted == 0 {
            break;
        }
        total += inserted;
    }
    total
}

async fn project_once(pool: &PgPool) -> u64 {
    let repo = PostgresNotificationRepository::new(pool.clone());
    repo.project_domain_events(PROJECTION_BATCH).await.unwrap()
}

async fn create_pending_payment(pool: &PgPool, amount_minor: i64) -> (String, Uuid) {
    let mut app = build_app(pool.clone()).await;
    let idem = new_idempotency_key();
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &merchant_token(),
        Some(&idem),
        Some(json!({
            "amount_minor": amount_minor,
            "currency": "USD",
            "metadata": {"order_ref": format!("NOTIF-{}", new_uuid_v7())}
        })),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let id = body["id"].as_str().unwrap().to_string();
    let uuid = Uuid::parse_str(&id).unwrap();
    (id, uuid)
}

async fn create_successful_payment(pool: &PgPool) -> Uuid {
    use payments::service::{process_payment, PaymentProcessingOutcome};
    let (_payment_id, payment_uuid) = create_pending_payment(pool, 1000).await;
    process_payment(
        pool,
        payment_uuid,
        PaymentProcessingOutcome::simulated_success(),
    )
    .await
    .expect("process_payment should succeed");
    payment_uuid
}

async fn get_domain_events(
    pool: &PgPool,
    aggregate_type: &str,
    aggregate_id: Uuid,
) -> Vec<(Uuid, String, serde_json::Value)> {
    sqlx::query_as(
        "SELECT id, event_type, payload FROM domain_events WHERE aggregate_type = $1 AND aggregate_id = $2 ORDER BY created_at ASC, id ASC",
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn count_notif_for_event_dest(pool: &PgPool, event_id: Uuid, destination_url: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM notification_delivery_records WHERE domain_event_id = $1 AND destination_url = $2",
    )
    .bind(event_id)
    .bind(destination_url)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn assert_pending_record(pool: &PgPool, event_id: Uuid, dest_url: &str) {
    let records: Vec<(String, String, i32)> = sqlx::query_as(
        "SELECT destination_url, status::text, attempt_count FROM notification_delivery_records WHERE domain_event_id = $1 AND destination_url = $2",
    )
    .bind(event_id)
    .bind(dest_url)
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        records.len(),
        1,
        "expected exactly one record for event {event_id} -> {dest_url}"
    );
    assert_eq!(records[0].0, dest_url);
    assert_eq!(records[0].1, "pending");
    assert_eq!(records[0].2, 0);
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

// -------------------------------------------------------------------------
// MPG-013 Notification Event Pipeline Tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn projection_creates_record_for_payment_created() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    let (_payment_id, payment_uuid) = create_pending_payment(&pool, 1000).await;

    let total = drain_projection(&pool).await;
    assert!(total > 0, "should have projected at least one record");

    let events = get_domain_events(&pool, "payment", payment_uuid).await;
    let created_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_pending_record(&pool, created_event.0, &dest_url).await;
}

#[tokio::test]
async fn projection_creates_for_successful_skips_processing() {
    use payments::service::{process_payment, PaymentProcessingOutcome};

    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    let (_payment_id, payment_uuid) = create_pending_payment(&pool, 2000).await;
    process_payment(
        &pool,
        payment_uuid,
        PaymentProcessingOutcome::simulated_success(),
    )
    .await
    .expect("process_payment should succeed");

    drain_projection(&pool).await;

    let events = get_domain_events(&pool, "payment", payment_uuid).await;

    let processing_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.processing")
        .unwrap();
    assert_eq!(
        count_notif_for_event_dest(&pool, processing_event.0, &dest_url).await,
        0,
        "payment.processing must not create Notification Delivery Records"
    );

    let created_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_pending_record(&pool, created_event.0, &dest_url).await;

    let successful_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.successful")
        .unwrap();
    assert_pending_record(&pool, successful_event.0, &dest_url).await;
}

#[tokio::test]
async fn projection_creates_for_failed_payment() {
    use payments::service::{process_payment, PaymentProcessingOutcome};

    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    let (_payment_id, payment_uuid) = create_pending_payment(&pool, 3000).await;
    process_payment(
        &pool,
        payment_uuid,
        PaymentProcessingOutcome::Failed {
            failure_reason: "processor_decline".into(),
        },
    )
    .await
    .expect("process_payment should succeed for failure");

    drain_projection(&pool).await;

    let events = get_domain_events(&pool, "payment", payment_uuid).await;

    let processing_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.processing")
        .unwrap();
    assert_eq!(
        count_notif_for_event_dest(&pool, processing_event.0, &dest_url).await,
        0
    );

    let created_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_pending_record(&pool, created_event.0, &dest_url).await;

    let failed_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.failed")
        .unwrap();
    assert_pending_record(&pool, failed_event.0, &dest_url).await;
}

#[tokio::test]
async fn projection_creates_record_for_refund_created() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();

    let payment_id = create_successful_payment(&pool).await;

    let mut app = build_app(pool.clone()).await;
    let idem_key = new_idempotency_key();
    let (status, body) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/refunds",
        &merchant_token(),
        Some(&idem_key),
        Some(json!({"payment_id": payment_id.to_string()})),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    let refund_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    drain_projection(&pool).await;

    let refund_events = get_domain_events(&pool, "refund", refund_id).await;
    let refund_created = refund_events
        .iter()
        .find(|(_, t, _)| t == "refund.created")
        .unwrap();
    assert_pending_record(&pool, refund_created.0, &dest_url).await;
}

#[tokio::test]
async fn projection_creates_record_for_refund_completed() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();

    let payment_id = create_successful_payment(&pool).await;
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
    .bind(payment_id)
    .bind(merchant_uuid)
    .bind(1000i64)
    .bind("USD")
    .bind(&refund_idem)
    .bind(now)
    .execute(&pool)
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
    .bind(serde_json::json!({"refund_id": refund_id.to_string(), "payment_id": payment_id.to_string()}))
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    drain_projection(&pool).await;

    assert_pending_record(&pool, refund_completed_event_id, &dest_url).await;
}

#[tokio::test]
async fn projection_is_idempotent() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let dest_url = unique_dest_url();
    set_active_destination(&pool, merchant_uuid, &dest_url).await;

    create_pending_payment(&pool, 4000).await;

    drain_projection(&pool).await;

    let total_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notification_delivery_records WHERE destination_url = $1",
    )
    .bind(&dest_url)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(total_before > 0);

    let second = project_once(&pool).await;
    assert_eq!(second, 0, "second projection should insert 0 records");
}

#[tokio::test]
async fn no_active_destination_creates_no_records() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();

    deactivate_all_destinations(&pool, merchant_uuid).await;

    let dest_url = unique_dest_url();
    insert_inactive_destination(&pool, merchant_uuid, &dest_url).await;

    let (_payment_id, payment_uuid) = create_pending_payment(&pool, 5000).await;

    let inserted = drain_projection(&pool).await;
    assert_eq!(
        inserted, 0,
        "inactive destination should not produce records"
    );

    let events = get_domain_events(&pool, "payment", payment_uuid).await;
    let created_event = events
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_eq!(
        count_notif_for_event_dest(&pool, created_event.0, &dest_url).await,
        0
    );
}

#[tokio::test]
async fn one_active_destination_per_merchant_uniqueness() {
    let pool = setup_db().await;
    let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();

    set_active_destination(&pool, merchant_uuid, "https://first.example/webhook").await;

    let second_id = new_uuid_v7();
    let result = sqlx::query(
        "INSERT INTO notification_destinations (id, merchant_id, destination_url, is_active, created_at, updated_at) VALUES ($1, $2, $3, true, NOW(), NOW())",
    )
    .bind(second_id)
    .bind(merchant_uuid)
    .bind("https://second.example/webhook")
    .execute(&pool)
    .await;
    assert!(
        result.is_err(),
        "second active destination for same merchant must fail on unique index"
    );

    insert_inactive_destination(&pool, merchant_uuid, "https://third.example/webhook").await;
}

#[tokio::test]
async fn merchant_scoping_events_only_create_records_for_own_destinations() {
    let pool = setup_db().await;
    let merchant_a = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
    let dest_a = unique_dest_url();
    set_active_destination(&pool, merchant_a, &dest_a).await;

    let actor_b = second_merchant_actor_id();
    insert_second_merchant_actor(&pool, &actor_b).await;

    let (_payment_a_id, payment_a_uuid) = create_pending_payment(&pool, 6000).await;

    let token_b = second_merchant_token(&actor_b);
    let mut app = build_app(pool.clone()).await;
    let idem_b = new_idempotency_key();
    let (status_b, body_b) = send_request(
        &mut app,
        axum::http::Method::POST,
        "/api/v1/payments",
        &token_b,
        Some(&idem_b),
        Some(json!({"amount_minor": 7000, "currency": "USD"})),
    )
    .await;
    assert_eq!(status_b, axum::http::StatusCode::CREATED);
    let payment_b_uuid = Uuid::parse_str(body_b["id"].as_str().unwrap()).unwrap();

    drain_projection(&pool).await;

    let events_a = get_domain_events(&pool, "payment", payment_a_uuid).await;
    let created_a = events_a
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_eq!(
        count_notif_for_event_dest(&pool, created_a.0, &dest_a).await,
        1,
        "merchant A event should create record for merchant A destination"
    );

    let events_b = get_domain_events(&pool, "payment", payment_b_uuid).await;
    let created_b = events_b
        .iter()
        .find(|(_, t, _)| t == "payment.created")
        .unwrap();
    assert_eq!(
        count_notif_for_event_dest(&pool, created_b.0, &dest_a).await,
        0,
        "merchant B event should NOT create record for merchant A destination"
    );
}
