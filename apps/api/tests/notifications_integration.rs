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
use notifications::service::{
    DefaultNotificationService, NotificationDeliveryPayload, NotificationDeliverySettings,
    NotificationTransport,
};
use std::sync::{Arc, Mutex};

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

// -------------------------------------------------------------------------
// MPG-014 Notification Delivery Tests
// -------------------------------------------------------------------------

struct FakeTransport {
    response_code: Arc<Mutex<u16>>,
    captured_payloads: Arc<Mutex<Vec<NotificationDeliveryPayload>>>,
}

impl FakeTransport {
    fn new(response_code: u16) -> Self {
        Self {
            response_code: Arc::new(Mutex::new(response_code)),
            captured_payloads: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl NotificationTransport for FakeTransport {
    fn deliver(
        &self,
        _destination_url: &str,
        payload: &NotificationDeliveryPayload,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send + '_>> {
        let code = *self.response_code.lock().unwrap();
        self.captured_payloads.lock().unwrap().push(payload.clone());
        Box::pin(async move { Ok(code) })
    }
}

struct FailingTransport {
    error: String,
}

impl NotificationTransport for FailingTransport {
    fn deliver(
        &self,
        _destination_url: &str,
        _payload: &NotificationDeliveryPayload,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u16, String>> + Send + '_>> {
        let msg = self.error.clone();
        Box::pin(async move { Err(msg) })
    }
}

mod delivery_tests {
    use super::*;
    use notifications::repository::PostgresNotificationRepository;
    use notifications::service::{DefaultNotificationService, NotificationDeliverySettings};

    #[tokio::test]
    async fn successful_delivery_sends_full_payload_and_marks_delivered() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let (_payment_id, payment_uuid) = create_pending_payment(&pool, 1000).await;
        let events = get_domain_events(&pool, "payment", payment_uuid).await;
        let created_event = events
            .iter()
            .find(|(_, t, _)| t == "payment.created")
            .unwrap();

        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE domain_event_id = $1 AND destination_url = $2",
        )
        .bind(created_event.0)
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query(
            "DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1",
        )
        .bind(notif_id)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        let transport = FakeTransport::new(200);
        let repo = PostgresNotificationRepository::new(pool.clone());
        let service = DefaultNotificationService::new(repo);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should have claimed a record");

        assert_eq!(outcome.record_id.to_string().len(), 36);
        assert_eq!(outcome.destination_url, dest_url);
        assert_eq!(outcome.event_type, "payment.created");
        assert_eq!(outcome.attempt_number, 1);

        let captured = transport.captured_payloads.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].event_id, created_event.0);
        assert_eq!(captured[0].event_type, "payment.created");
        assert_eq!(captured[0].resource_type, "payment");
        assert_eq!(captured[0].resource_id, payment_uuid);
        assert_eq!(captured[0].schema_version, 1);
        assert!(captured[0].payload.get("payment_id").is_some());

        let record: (String, i32, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, last_error, next_retry_at::text FROM notification_delivery_records WHERE id = $1",
        )
        .bind(outcome.record_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "delivered");
        assert_eq!(record.1, 1);
        assert!(
            record.2.is_none(),
            "last_error should be null after success"
        );
        assert!(
            record.3.is_none(),
            "next_retry_at should be null after success"
        );
    }

    #[tokio::test]
    async fn non_2xx_failure_records_http_status_and_schedules_retry() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 2000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
        )
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        let transport = FakeTransport::new(500);
        let repo = PostgresNotificationRepository::new(pool.clone());
        let service = DefaultNotificationService::new(repo);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should have claimed a record");

        let record: (String, i32, String, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, last_error, next_retry_at::text FROM notification_delivery_records WHERE id = $1",
        )
        .bind(outcome.record_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "pending");
        assert_eq!(record.1, 1);
        assert_eq!(record.2, "http_status:500");
        assert!(record.3.is_some(), "next_retry_at should be set for retry");
    }

    #[tokio::test]
    async fn transport_error_records_bounded_last_error() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 3000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
        )
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        let transport = FailingTransport {
            error: "request timed out".into(),
        };
        let repo = PostgresNotificationRepository::new(pool.clone());
        let service = DefaultNotificationService::new(repo);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should have claimed a record");

        let record: (String, i32, String, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, last_error, next_retry_at::text FROM notification_delivery_records WHERE id = $1",
        )
        .bind(outcome.record_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "pending");
        assert_eq!(record.1, 1);
        assert_eq!(record.2, "timeout");
    }

    #[tokio::test]
    async fn max_attempts_marks_terminal_failed() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 4000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
        )
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        let transport = FakeTransport::new(503);
        let repo = PostgresNotificationRepository::new(pool.clone());
        let service = DefaultNotificationService::new(repo);
        let settings = NotificationDeliverySettings::new(1, vec![]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should deliver one attempt that fails");

        let record: (String, i32, String, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, last_error, next_retry_at::text FROM notification_delivery_records WHERE id = $1",
        )
        .bind(outcome.record_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "failed");
        assert_eq!(record.1, 1);
        assert_eq!(record.2, "http_status:503");
        assert!(
            record.3.is_none(),
            "next_retry_at should be null on terminal failure"
        );
    }

    #[tokio::test]
    async fn stale_processing_record_is_reclaimed() {
        let pool = setup_db().await;
        sqlx::query("DELETE FROM notification_delivery_attempts")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records")
            .execute(&pool)
            .await
            .unwrap();

        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 5000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE status = 'pending' LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let stale_time = chrono::Utc::now() - chrono::Duration::minutes(10);
        sqlx::query(
            "UPDATE notification_delivery_records SET status = 'processing', last_attempt_at = $2 WHERE id = $1",
        )
        .bind(notif_id)
        .bind(stale_time)
        .execute(&pool)
        .await
        .unwrap();

        let transport = FakeTransport::new(200);
        let repo = PostgresNotificationRepository::new(pool.clone());
        let service = DefaultNotificationService::new(repo);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("stale processing record should be reclaimed");

        let record: (String, i32, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, last_error, next_retry_at::text FROM notification_delivery_records WHERE id = $1",
        )
        .bind(outcome.record_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "delivered");
        assert!(record.1 >= 1, "attempt_count should be at least 1");
        assert!(record.2.is_none());
        assert!(record.3.is_none());
    }
}

// -------------------------------------------------------------------------
// MPG-015 Admin Notification Tests
// -------------------------------------------------------------------------

const ADMIN_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000002";

fn admin_token() -> String {
    make_token(TestClaims {
        sub: ADMIN_ACTOR_ID.into(),
        role: "administrator".into(),
        merchant_id: None,
        exp: 9999999999,
    })
}

async fn force_record_to_failed(pool: &PgPool, dest_url: &str) -> Uuid {
    create_pending_payment(pool, 1000).await;
    drain_projection(pool).await;

    let notif_id: Uuid = sqlx::query_scalar(
        "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
    )
    .bind(dest_url)
    .fetch_one(pool)
    .await
    .unwrap();

    sqlx::query(
        "DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1",
    )
    .bind(notif_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
        .bind(notif_id)
        .execute(pool)
        .await
        .unwrap();

    let repo = PostgresNotificationRepository::new(pool.clone());
    let service = DefaultNotificationService::new(repo);
    let settings = NotificationDeliverySettings::new(1, vec![]);
    let transport = FakeTransport::new(503);

    let outcome = service
        .process_next_due_delivery(&transport, &settings)
        .await
        .unwrap()
        .expect("should have claimed and failed");

    assert!(
        matches!(
            outcome.status,
            notifications::service::DeliveryStatus::TerminalFailed
        ),
        "expected terminal failure"
    );

    notif_id
}

mod admin_tests {
    use super::*;
    use notifications::repository::PostgresNotificationRepository;
    use notifications::service::{DefaultNotificationService, NotificationDeliverySettings};

    #[tokio::test]
    async fn admin_list_notifications_returns_records_with_attempts() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            "/api/v1/notifications",
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        let items = body["items"].as_array().unwrap();
        assert!(!items.is_empty(), "should return at least one notification");

        let target = items
            .iter()
            .find(|i| i["id"].as_str().unwrap() == notif_id.to_string())
            .expect("target notification should be in list");
        assert_eq!(target["status"], "failed");
        assert!(target["attempt_count"].as_i64().unwrap() >= 1);
        assert!(target["retry_generation"].as_i64().unwrap() >= 0);
        assert!(target["last_error"].as_str().unwrap_or("").contains("503"));
        assert!(target["destination_url"]
            .as_str()
            .unwrap()
            .contains("webhook"));

        let attempts = target["attempts"].as_array().unwrap();
        assert!(!attempts.is_empty(), "should have attempt history");
        let attempt = &attempts[0];
        assert_eq!(attempt["status"], "failed");
        assert!(attempt["error"].as_str().unwrap_or("").contains("503"));
    }

    #[tokio::test]
    async fn admin_list_notifications_filters_by_status() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        force_record_to_failed(&pool, &dest_url).await;

        let mut app = build_app(pool.clone()).await;

        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            "/api/v1/notifications?status=failed",
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        let items = body["items"].as_array().unwrap();
        for item in items {
            assert_eq!(item["status"], "failed");
        }

        let (status2, body2) = send_request(
            &mut app,
            axum::http::Method::GET,
            "/api/v1/notifications?status=delivered",
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status2, axum::http::StatusCode::OK);
        for item in body2["items"].as_array().unwrap() {
            assert_eq!(item["status"], "delivered");
        }
    }

    #[tokio::test]
    async fn admin_get_notification_detail_returns_attempt_history() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            &format!("/api/v1/notifications/{notif_id}"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["id"], notif_id.to_string());
        assert_eq!(body["status"], "failed");
        assert!(body["event_type"].as_str().is_some());
        assert!(body["resource_type"].as_str().is_some());
        assert!(body["destination_url"].as_str().is_some());
        assert!(body["attempts"].as_array().unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn admin_retry_failed_notification_requeues_and_audits() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let before: (String, i32, i32, Option<String>) = sqlx::query_as(
            "SELECT status::text, attempt_count, retry_generation, last_error FROM notification_delivery_records WHERE id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(before.0, "failed");
        let prev_count = before.1;
        let prev_gen = before.2;

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["id"], notif_id.to_string());
        assert_eq!(body["status"], "pending");
        assert_eq!(
            body["retry_generation"].as_i64().unwrap(),
            (prev_gen + 1) as i64
        );
        assert_eq!(body["attempt_count"].as_i64().unwrap(), prev_count as i64);
        assert!(body["last_error"].as_str().is_some());

        let after: (String, i32, i32) = sqlx::query_as(
            "SELECT status::text, attempt_count, retry_generation FROM notification_delivery_records WHERE id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(after.0, "pending");
        assert_eq!(after.2, prev_gen + 1);

        let audit: (String, String) = sqlx::query_as(
            "SELECT action, resource_id FROM audit_records WHERE action = 'notification.retry_requested' AND resource_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(notif_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(audit.0, "notification.retry_requested");
        assert_eq!(audit.1, notif_id.to_string());
    }

    #[tokio::test]
    async fn admin_retry_non_failed_notification_returns_409() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 1000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
        )
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        let before: (String,) =
            sqlx::query_as("SELECT status::text FROM notification_delivery_records WHERE id = $1")
                .bind(notif_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(before.0, "pending");

        let mut app = build_app(pool.clone()).await;
        let (status, _body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::CONFLICT);

        let after: (String,) =
            sqlx::query_as("SELECT status::text FROM notification_delivery_records WHERE id = $1")
                .bind(notif_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(after.0, "pending", "status should not change on conflict");
    }

    #[tokio::test]
    async fn admin_retry_unknown_notification_returns_404() {
        let pool = setup_db().await;
        let mut app = build_app(pool.clone()).await;

        let fake_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let (status, _body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{fake_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn merchant_cannot_list_get_or_retry_notifications() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let mut app = build_app(pool.clone()).await;

        let (list_status, _) = send_request(
            &mut app,
            axum::http::Method::GET,
            "/api/v1/notifications",
            &merchant_token(),
            None,
            None,
        )
        .await;
        assert_eq!(list_status, axum::http::StatusCode::FORBIDDEN);

        let (detail_status, _) = send_request(
            &mut app,
            axum::http::Method::GET,
            &format!("/api/v1/notifications/{notif_id}"),
            &merchant_token(),
            None,
            None,
        )
        .await;
        assert_eq!(detail_status, axum::http::StatusCode::FORBIDDEN);

        let (retry_status, _) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &merchant_token(),
            None,
            None,
        )
        .await;
        assert_eq!(retry_status, axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn retry_request_does_not_create_attempt_row_until_worker_runs() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let attempts_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        let mut app = build_app(pool.clone()).await;
        let (status, _body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);

        let attempts_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            attempts_before, attempts_after,
            "retry request should not create new attempt rows"
        );
    }

    #[tokio::test]
    async fn manual_retry_generation_gets_fresh_budget_without_resetting_attempt_count() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let mut app = build_app(pool.clone()).await;
        let (status, _body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);

        let record: (i32, i32) = sqlx::query_as(
            "SELECT attempt_count, retry_generation FROM notification_delivery_records WHERE id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, 1, "attempt_count should not reset");
        assert_eq!(record.1, 1, "retry_generation should be 1");

        let service =
            DefaultNotificationService::new(PostgresNotificationRepository::new(pool.clone()));
        let transport = FakeTransport::new(200);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should claim requeued record");

        assert_eq!(outcome.attempt_number, 2);
        assert!(
            matches!(
                outcome.status,
                notifications::service::DeliveryStatus::Delivered
            ),
            "should deliver in new generation"
        );
    }

    #[tokio::test]
    async fn success_after_retry_clears_last_error_but_preserves_failed_attempt_rows() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let notif_id = force_record_to_failed(&pool, &dest_url).await;

        let failed_attempt_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1 AND status = 'failed'",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(failed_attempt_count >= 1);

        let mut app = build_app(pool.clone()).await;
        let (status, _body) = send_request(
            &mut app,
            axum::http::Method::POST,
            &format!("/api/v1/notifications/{notif_id}/retry"),
            &admin_token(),
            None,
            None,
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);

        let service =
            DefaultNotificationService::new(PostgresNotificationRepository::new(pool.clone()));
        let transport = FakeTransport::new(200);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("should deliver");

        let record: (String, Option<String>) = sqlx::query_as(
            "SELECT status::text, last_error FROM notification_delivery_records WHERE id = $1",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(record.0, "delivered");
        assert!(
            record.1.is_none(),
            "last_error should be cleared on success"
        );

        let failed_still_there: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM notification_delivery_attempts WHERE notification_delivery_record_id = $1 AND status = 'failed'",
        )
        .bind(notif_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            failed_still_there, failed_attempt_count,
            "prior failed attempt rows should be preserved"
        );
    }

    #[tokio::test]
    async fn stale_processing_attempt_is_marked_failed_before_reclaim() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        create_pending_payment(&pool, 1000).await;
        drain_projection(&pool).await;

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE destination_url = $1 LIMIT 1",
        )
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM notification_delivery_attempts WHERE notification_delivery_record_id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("DELETE FROM notification_delivery_records WHERE id != $1")
            .bind(notif_id)
            .execute(&pool)
            .await
            .unwrap();

        let attempt_id = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let stale_time = chrono::Utc::now() - chrono::Duration::minutes(10);
        sqlx::query(
            "INSERT INTO notification_delivery_attempts (id, notification_delivery_record_id, retry_generation, attempt_number, status, started_at, created_at, updated_at) VALUES ($1, $2, 0, 1, 'processing', $3, $3, $3)",
        )
        .bind(attempt_id)
        .bind(notif_id)
        .bind(stale_time)
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "UPDATE notification_delivery_records SET status = 'processing', attempt_count = 1, last_attempt_at = $2, retry_generation = 0 WHERE id = $1",
        )
        .bind(notif_id)
        .bind(stale_time)
        .execute(&pool)
        .await
        .unwrap();

        let service =
            DefaultNotificationService::new(PostgresNotificationRepository::new(pool.clone()));
        let transport = FakeTransport::new(200);
        let settings = NotificationDeliverySettings::new(3, vec![1, 2]);

        let outcome = service
            .process_next_due_delivery(&transport, &settings)
            .await
            .unwrap()
            .expect("stale record should be reclaimed");

        let stale_attempt: (String, Option<String>) = sqlx::query_as(
            "SELECT status::text, error FROM notification_delivery_attempts WHERE id = $1",
        )
        .bind(attempt_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(stale_attempt.0, "failed");
        assert_eq!(
            stale_attempt.1.as_deref(),
            Some("stale_processing_reclaimed"),
            "stale attempt should be marked failed"
        );

        assert_eq!(outcome.attempt_number, 2);
        assert!(
            matches!(
                outcome.status,
                notifications::service::DeliveryStatus::Delivered
            ),
            "new attempt should deliver"
        );
    }

    // -------------------------------------------------------------------------
    // MPG-016 Navigation Field Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn payment_event_notification_includes_payment_navigation_fields() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let (_payment_id, payment_uuid) = create_pending_payment(&pool, 1000).await;
        drain_projection(&pool).await;

        let events = get_domain_events(&pool, "payment", payment_uuid).await;
        let created_event = events
            .iter()
            .find(|(_, t, _)| t == "payment.created")
            .unwrap();

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE domain_event_id = $1 AND destination_url = $2",
        )
        .bind(created_event.0)
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            &format!("/api/v1/notifications/{notif_id}"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["resource_type"], "payment");
        assert_eq!(body["resource_id"], payment_uuid.to_string());
        assert_eq!(
            body["resource_api_path"],
            format!("/api/v1/payments/{payment_uuid}")
        );
        assert_eq!(body["payment_id"], payment_uuid.to_string());
        assert_eq!(
            body["payment_api_path"],
            format!("/api/v1/payments/{payment_uuid}")
        );
        assert!(body.get("refund_id").map_or(true, |v| v.is_null()));
        assert!(body.get("refund_api_path").map_or(true, |v| v.is_null()));
    }

    #[tokio::test]
    async fn refund_event_notification_includes_refund_and_payment_navigation_fields() {
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

        let notif_id: Uuid = sqlx::query_scalar(
            "SELECT id FROM notification_delivery_records WHERE domain_event_id = $1 AND destination_url = $2",
        )
        .bind(refund_completed_event_id)
        .bind(&dest_url)
        .fetch_one(&pool)
        .await
        .unwrap();

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            &format!("/api/v1/notifications/{notif_id}"),
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body["resource_type"], "refund");
        assert_eq!(body["resource_id"], refund_id.to_string());
        assert_eq!(
            body["resource_api_path"],
            format!("/api/v1/refunds/{refund_id}")
        );
        assert_eq!(body["payment_id"], payment_id.to_string());
        assert_eq!(
            body["payment_api_path"],
            format!("/api/v1/payments/{payment_id}")
        );
        assert_eq!(body["refund_id"], refund_id.to_string());
        assert_eq!(
            body["refund_api_path"],
            format!("/api/v1/refunds/{refund_id}")
        );
    }

    #[tokio::test]
    async fn notification_list_items_include_navigation_fields() {
        let pool = setup_db().await;
        let merchant_uuid = Uuid::parse_str(MERCHANT_ACTOR_ID).unwrap();
        let dest_url = unique_dest_url();
        set_active_destination(&pool, merchant_uuid, &dest_url).await;

        let (_payment_id, payment_uuid) = create_pending_payment(&pool, 1000).await;
        drain_projection(&pool).await;

        let mut app = build_app(pool.clone()).await;
        let (status, body) = send_request(
            &mut app,
            axum::http::Method::GET,
            "/api/v1/notifications",
            &admin_token(),
            None,
            None,
        )
        .await;

        assert_eq!(status, axum::http::StatusCode::OK);
        let items = body["items"].as_array().unwrap();
        assert!(!items.is_empty());

        let payment_items: Vec<_> = items
            .iter()
            .filter(|i| {
                i["resource_type"] == "payment" && i["resource_id"] == payment_uuid.to_string()
            })
            .collect();
        assert!(
            !payment_items.is_empty(),
            "should find payment event items in list"
        );

        for item in &payment_items {
            assert!(item.get("resource_api_path").is_some());
            assert!(item.get("payment_id").is_some());
            assert!(item.get("payment_api_path").is_some());
            assert!(item.get("refund_id").map_or(true, |v| v.is_null()));
            assert!(item.get("refund_api_path").map_or(true, |v| v.is_null()));
        }
    }
}
