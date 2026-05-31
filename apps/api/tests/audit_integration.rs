use axum::Router;
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;

use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use api::router;

const ADMIN_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000002";
const MERCHANT_ACTOR_ID: &str = "00000000-0000-0000-0000-000000000001";
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

#[tokio::test]
async fn admin_list_audit_records_returns_200() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit?limit=5")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(body["items"].is_array());
    assert_eq!(body["limit"], 5);
    assert_eq!(body["offset"], 0);
}

#[tokio::test]
async fn admin_get_audit_record_by_id_returns_200() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let record_id = new_uuid_v7();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(record_id)
    .bind(Uuid::parse_str(ADMIN_ACTOR_ID).unwrap())
    .bind("administrator")
    .bind("test.action")
    .bind("test_resource")
    .bind("res-123")
    .bind(serde_json::Value::Object(serde_json::Map::new()))
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/audit/{record_id}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["id"], record_id.to_string());
    assert_eq!(body["action"], "test.action");
    assert_eq!(body["actor_type"], "administrator");
}

#[tokio::test]
async fn unknown_audit_record_id_returns_404() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let unknown_id = "00000000-0000-0000-0000-000000000000";
    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/audit/{unknown_id}"))
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn merchant_cannot_list_audit_records() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn merchant_cannot_get_audit_record() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit/00000000-0000-0000-0000-000000000000")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn audit_without_auth_returns_401() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn audit_filters_work() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let now = Utc::now();
    let target_id = new_uuid_v7();
    let resource_id = "filter-test-res-1";

    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(target_id)
    .bind::<Option<Uuid>>(None)
    .bind("system")
    .bind("filter.test.action")
    .bind("filter_resource")
    .bind(resource_id)
    .bind::<Option<serde_json::Value>>(None)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let token = admin_token();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/audit?resource_type=filter_resource&resource_id={resource_id}&action=filter.test.action"))
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty(), "Expected at least one filter match");
    assert_eq!(items[0]["id"], target_id.to_string());
}

#[tokio::test]
async fn invalid_pagination_returns_422() {
    let pool = setup_db().await;
    let token = admin_token();

    for uri in &[
        "/api/v1/audit?limit=0",
        "/api/v1/audit?limit=201",
        "/api/v1/audit?offset=-1",
    ] {
        let app = build_app(pool.clone()).await;
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri(*uri)
                    .header("Authorization", format!("Bearer {token}"))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}

#[tokio::test]
async fn pagination_defaults() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = admin_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(body["limit"], 50);
    assert_eq!(body["offset"], 0);
}

#[tokio::test]
async fn append_only_trigger_rejects_update() {
    let pool = setup_db().await;

    let record_id = new_uuid_v7();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(record_id)
    .bind::<Option<Uuid>>(None)
    .bind("system")
    .bind("append_only_test")
    .bind("test")
    .bind("test")
    .bind::<Option<serde_json::Value>>(None)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let result = sqlx::query("UPDATE audit_records SET action = 'updated' WHERE id = $1")
        .bind(record_id)
        .execute(&pool)
        .await;

    assert!(
        result.is_err(),
        "UPDATE should be rejected by append-only trigger"
    );
}

#[tokio::test]
async fn append_only_trigger_rejects_delete() {
    let pool = setup_db().await;

    let record_id = new_uuid_v7();
    let now = Utc::now();
    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(record_id)
    .bind::<Option<Uuid>>(None)
    .bind("system")
    .bind("append_only_delete_test")
    .bind("test")
    .bind("test")
    .bind::<Option<serde_json::Value>>(None)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let result = sqlx::query("DELETE FROM audit_records WHERE id = $1")
        .bind(record_id)
        .execute(&pool)
        .await;

    assert!(
        result.is_err(),
        "DELETE should be rejected by append-only trigger"
    );
}

#[tokio::test]
async fn auth_failure_audit_records_still_written() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);

    let audit: Vec<(String, String)> = sqlx::query_as(
        "SELECT action, actor_type FROM audit_records WHERE resource_type = 'auth' AND action = 'auth.authentication_failed' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].0, "auth.authentication_failed");
    assert_eq!(audit[0].1, "unknown");
}

#[tokio::test]
async fn authorization_failure_audit_still_written() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let token = merchant_token();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/audit")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);

    let audit: Vec<(String, String)> = sqlx::query_as(
        "SELECT action, actor_type FROM audit_records WHERE resource_type = 'auth' AND action = 'auth.authorization_failed' ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].0, "auth.authorization_failed");
    assert_eq!(audit[0].1, "merchant");
}

#[tokio::test]
async fn audit_results_ordered_by_occurred_at_desc() {
    let pool = setup_db().await;
    let run_id = new_uuid_v7();
    let action1 = format!("order_test_1_{run_id}");
    let action2 = format!("order_test_2_{run_id}");

    let now = Utc::now();
    let earlier = now - chrono::Duration::hours(2);

    let id1 = new_uuid_v7();
    let id2 = new_uuid_v7();

    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(id1)
    .bind::<Option<Uuid>>(None)
    .bind("system")
    .bind(&action1)
    .bind("order_test")
    .bind("order-1")
    .bind::<Option<serde_json::Value>>(None)
    .bind(earlier)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"INSERT INTO audit_records (id, actor_id, actor_type, action, resource_type, resource_id, details, occurred_at, created_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(id2)
    .bind::<Option<Uuid>>(None)
    .bind("system")
    .bind(&action2)
    .bind("order_test")
    .bind("order-2")
    .bind::<Option<serde_json::Value>>(None)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let token = admin_token();
    let filter = format!("/api/v1/audit?action={action1}&limit=10");
    let response = build_app(pool.clone())
        .await
        .oneshot(
            axum::http::Request::builder()
                .uri(&filter)
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "Should find the earlier record");
    assert_eq!(items[0]["id"], id1.to_string());

    let filter = format!("/api/v1/audit?action={action2}&limit=10");
    let response = build_app(pool.clone())
        .await
        .oneshot(
            axum::http::Request::builder()
                .uri(&filter)
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "Should find the later record");
    assert_eq!(items[0]["id"], id2.to_string());
}
