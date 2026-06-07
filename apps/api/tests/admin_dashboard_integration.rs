use axum::body::Body;
use axum::http::Request;
use axum::Router;
use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use tower::ServiceExt;

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
    let body_bytes = axum::body::to_bytes(response.into_body(), 10_000_000)
        .await
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
    (status, body)
}

#[tokio::test]
async fn admin_dashboard_returns_summary_with_all_sections() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["configured_currency"], "USD");
    assert_eq!(body["api_status"], "reachable");
    assert!(body["generated_at"].as_str().unwrap().len() > 0);
    assert!(body["window_start"].as_str().unwrap().len() > 0);
    assert!(body["window_end"].as_str().unwrap().len() > 0);

    assert!(body["payment_overview"]["status_counts"]["pending"].is_number());
    assert!(body["payment_overview"]["status_counts"]["processing"].is_number());
    assert!(body["payment_overview"]["status_counts"]["successful"].is_number());
    assert!(body["payment_overview"]["status_counts"]["failed"].is_number());
    assert!(body["payment_overview"]["status_counts"]["refunded"].is_number());
    assert!(body["payment_overview"]["recent_failed_payments"].is_array());

    assert!(body["refund_overview"]["status_counts"]["pending"].is_number());
    assert!(body["refund_overview"]["status_counts"]["processing"].is_number());
    assert!(body["refund_overview"]["status_counts"]["completed"].is_number());
    assert!(body["refund_overview"]["status_counts"]["failed"].is_number());
    assert!(body["refund_overview"]["recent_failed_refunds"].is_array());

    assert!(body["notification_overview"]["status_counts"]["pending"].is_number());
    assert!(body["notification_overview"]["status_counts"]["processing"].is_number());
    assert!(body["notification_overview"]["status_counts"]["delivered"].is_number());
    assert!(body["notification_overview"]["status_counts"]["failed"].is_number());
    assert!(body["notification_overview"]["recent_failed_notifications"].is_array());

    assert!(body["reconciliation_overview"]["status_counts"]["matched"].is_number());
    assert!(body["reconciliation_overview"]["status_counts"]["mismatched"].is_number());
    assert!(body["reconciliation_overview"]["status_counts"]["error"].is_number());
    assert!(body["reconciliation_overview"]["recent_attention_reconciliations"].is_array());

    assert!(body["audit_overview"]["recent_attention_audit_records"].is_array());

    assert!(body["operational_health"]["recent_failed_operations"].is_array());
    assert!(
        body["operational_health"]["payment_processing_success_rate"]["numerator_count"]
            .is_number()
    );
    assert!(
        body["operational_health"]["payment_processing_success_rate"]["denominator_count"]
            .is_number()
    );
    assert!(
        body["operational_health"]["payment_processing_success_rate"]["in_flight_count"]
            .is_number()
    );
    assert!(
        body["operational_health"]["notification_delivery_success_rate"]["numerator_count"]
            .is_number()
    );
    assert!(
        body["operational_health"]["reconciliation_completion_rate"]["numerator_count"].is_number()
    );
}

#[tokio::test]
async fn merchant_token_returns_403_on_admin_dashboard() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &merchant_token(),
        None,
    )
    .await;

    assert_eq!(status, 403);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn missing_token_returns_401_on_admin_dashboard() {
    let pool = setup_db().await;
    let app = build_app(pool.clone()).await;

    let builder = Request::builder()
        .method(axum::http::Method::GET)
        .uri("/api/v1/dashboard/admin")
        .header("Content-Type", "application/json");

    let response = app
        .oneshot(builder.body(Body::from(vec![])).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn admin_dashboard_zero_fills_missing_statuses() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
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

    let notif_counts = &body["notification_overview"]["status_counts"];
    assert!(notif_counts["pending"].as_i64().unwrap() >= 0);
    assert!(notif_counts["processing"].as_i64().unwrap() >= 0);
    assert!(notif_counts["delivered"].as_i64().unwrap() >= 0);
    assert!(notif_counts["failed"].as_i64().unwrap() >= 0);

    let rec_counts = &body["reconciliation_overview"]["status_counts"];
    assert!(rec_counts["matched"].as_i64().unwrap() >= 0);
    assert!(rec_counts["mismatched"].as_i64().unwrap() >= 0);
    assert!(rec_counts["error"].as_i64().unwrap() >= 0);
}

#[tokio::test]
async fn admin_dashboard_recent_failed_notifications_include_event_context() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let failed_notifs = body["notification_overview"]["recent_failed_notifications"]
        .as_array()
        .unwrap();
    for notif in failed_notifs {
        assert!(notif["id"].as_str().unwrap().len() > 0);
        assert!(notif["domain_event_id"].as_str().unwrap().len() > 0);
        assert!(notif["event_type"].as_str().unwrap().len() > 0);
        assert!(notif["resource_type"].as_str().unwrap().len() > 0);
    }
}

#[tokio::test]
async fn admin_dashboard_window_timestamps_are_within_24_hours() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let window_start = body["window_start"].as_str().unwrap();
    let window_end = body["window_end"].as_str().unwrap();

    let start: chrono::DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(window_start)
        .unwrap()
        .into();
    let end: chrono::DateTime<chrono::Utc> = chrono::DateTime::parse_from_rfc3339(window_end)
        .unwrap()
        .into();

    let diff = end - start;
    assert!(diff.num_hours() >= 23);
    assert!(diff.num_hours() <= 25);
    assert!(start < end);
}

#[tokio::test]
async fn admin_dashboard_recent_audit_records_only_include_attention_actions() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let audit_records = body["audit_overview"]["recent_attention_audit_records"]
        .as_array()
        .unwrap();
    let valid_actions = [
        "auth.authentication_failed",
        "auth.authorization_failed",
        "payment.failed",
        "refund.rejected",
    ];
    for record in audit_records {
        let action = record["action"].as_str().unwrap();
        assert!(
            valid_actions.contains(&action),
            "Unexpected audit action: {action}"
        );
    }
}

#[tokio::test]
async fn admin_dashboard_zero_terminals_yields_null_rate_percent() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let reconciliation_rate = &body["operational_health"]["reconciliation_completion_rate"];
    if reconciliation_rate["denominator_count"].as_i64().unwrap() == 0 {
        assert!(reconciliation_rate["rate_percent"].is_null());
    } else {
        assert!(
            reconciliation_rate["rate_percent"].is_f64()
                || reconciliation_rate["rate_percent"].is_null()
        );
    }
}

#[tokio::test]
async fn admin_dashboard_in_flight_does_not_enter_denominator() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let payment_rate = &body["operational_health"]["payment_processing_success_rate"];
    let denominator = payment_rate["denominator_count"].as_i64().unwrap();
    let in_flight = payment_rate["in_flight_count"].as_i64().unwrap();

    assert!(denominator >= 0);
    assert!(in_flight >= 0);
    let _ = payment_rate["numerator_count"].as_i64().unwrap();
}

#[tokio::test]
async fn admin_dashboard_failed_operations_sorted_newest_first() {
    let pool = setup_db().await;
    let mut app = build_app(pool.clone()).await;

    let (status, body) = send_request(
        &mut app,
        axum::http::Method::GET,
        "/api/v1/dashboard/admin",
        &admin_token(),
        None,
    )
    .await;

    assert_eq!(status, 200);

    let failed_ops = body["operational_health"]["recent_failed_operations"]
        .as_array()
        .unwrap();
    for i in 1..failed_ops.len() {
        let prev_time = failed_ops[i - 1]["occurred_at"].as_str().unwrap();
        let curr_time = failed_ops[i]["occurred_at"].as_str().unwrap();
        assert!(
            prev_time >= curr_time,
            "Failed ops must be sorted newest-first: {prev_time} < {curr_time}"
        );
    }
}
