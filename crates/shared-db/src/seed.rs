use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub const MERCHANT_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const ADMIN_ID: &str = "00000000-0000-0000-0000-000000000002";

pub async fn seed_actors(pool: &PgPool) {
    let merchant_id = Uuid::parse_str(MERCHANT_ID).unwrap();
    let admin_id = Uuid::parse_str(ADMIN_ID).unwrap();

    sqlx::query(
        r#"
        INSERT INTO actors (id, name, email, role, is_active, merchant_id)
        VALUES ($1, 'Merchant One', 'merchant@example.com', 'merchant', true, $2)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            email = EXCLUDED.email,
            role = EXCLUDED.role,
            is_active = EXCLUDED.is_active,
            merchant_id = EXCLUDED.merchant_id
        "#,
    )
    .bind(merchant_id)
    .bind(merchant_id)
    .execute(pool)
    .await
    .expect("Failed to seed merchant actor");

    sqlx::query(
        r#"
        INSERT INTO actors (id, name, email, role, is_active, merchant_id)
        VALUES ($1, 'Admin One', 'admin@example.com', 'administrator', true, NULL)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            email = EXCLUDED.email,
            role = EXCLUDED.role,
            is_active = EXCLUDED.is_active,
            merchant_id = EXCLUDED.merchant_id
        "#,
    )
    .bind(admin_id)
    .execute(pool)
    .await
    .expect("Failed to seed admin actor");

    tracing::info!("Seed actors inserted");
    tracing::info!("Merchant Actor ID (sub): {MERCHANT_ID}");
    tracing::info!("Merchant Account ID (merchant_id): {MERCHANT_ID}");
    tracing::info!("Admin Actor ID (sub): {ADMIN_ID}");
}

fn uu(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap()
}

const NOTIF_DEST_ID: &str = "00000000-0000-0000-0000-000000000010";

const P_IDS: &[&str] = &[
    "00000000-0000-0000-0000-000000000011",
    "00000000-0000-0000-0000-000000000012",
    "00000000-0000-0000-0000-000000000013",
    "00000000-0000-0000-0000-000000000014",
    "00000000-0000-0000-0000-000000000015",
    "00000000-0000-0000-0000-000000000016",
    "00000000-0000-0000-0000-000000000017",
    "00000000-0000-0000-0000-000000000018",
    "00000000-0000-0000-0000-000000000019",
    "00000000-0000-0000-0000-00000000001a",
    "00000000-0000-0000-0000-00000000001b",
    "00000000-0000-0000-0000-00000000001c",
    "00000000-0000-0000-0000-00000000001d",
    "00000000-0000-0000-0000-00000000001e",
    "00000000-0000-0000-0000-00000000001f",
    "00000000-0000-0000-0000-000000000020",
    "00000000-0000-0000-0000-000000000021",
    "00000000-0000-0000-0000-000000000022",
    "00000000-0000-0000-0000-000000000023",
    "00000000-0000-0000-0000-000000000024",
    "00000000-0000-0000-0000-000000000025",
    "00000000-0000-0000-0000-000000000026",
    "00000000-0000-0000-0000-000000000027",
    "00000000-0000-0000-0000-000000000028",
    "00000000-0000-0000-0000-000000000029",
];

const R_IDS: &[&str] = &[
    "00000000-0000-0000-0000-000000000030",
    "00000000-0000-0000-0000-000000000031",
    "00000000-0000-0000-0000-000000000032",
    "00000000-0000-0000-0000-000000000033",
    "00000000-0000-0000-0000-000000000034",
];

const DE_IDS: &[&str] = &[
    "00000000-0000-0000-0000-000000000040",
    "00000000-0000-0000-0000-000000000041",
    "00000000-0000-0000-0000-000000000042",
    "00000000-0000-0000-0000-000000000043",
    "00000000-0000-0000-0000-000000000044",
    "00000000-0000-0000-0000-000000000045",
    "00000000-0000-0000-0000-000000000046",
    "00000000-0000-0000-0000-000000000047",
    "00000000-0000-0000-0000-000000000048",
    "00000000-0000-0000-0000-000000000049",
    "00000000-0000-0000-0000-00000000004a",
    "00000000-0000-0000-0000-00000000004b",
    "00000000-0000-0000-0000-00000000004c",
    "00000000-0000-0000-0000-00000000004d",
    "00000000-0000-0000-0000-00000000004e",
    "00000000-0000-0000-0000-00000000004f",
    "00000000-0000-0000-0000-000000000050",
    "00000000-0000-0000-0000-000000000051",
    "00000000-0000-0000-0000-000000000052",
    "00000000-0000-0000-0000-000000000053",
    "00000000-0000-0000-0000-000000000054",
    "00000000-0000-0000-0000-000000000055",
    "00000000-0000-0000-0000-000000000056",
    "00000000-0000-0000-0000-000000000057",
    "00000000-0000-0000-0000-000000000058",
    "00000000-0000-0000-0000-000000000059",
    "00000000-0000-0000-0000-00000000005a",
    "00000000-0000-0000-0000-00000000005b",
    "00000000-0000-0000-0000-00000000005c",
    "00000000-0000-0000-0000-00000000005d",
    "00000000-0000-0000-0000-00000000005e",
    "00000000-0000-0000-0000-00000000005f",
    "00000000-0000-0000-0000-000000000060",
    "00000000-0000-0000-0000-000000000061",
    "00000000-0000-0000-0000-000000000062",
    "00000000-0000-0000-0000-000000000063",
    "00000000-0000-0000-0000-000000000064",
    "00000000-0000-0000-0000-000000000065",
    "00000000-0000-0000-0000-000000000066",
    "00000000-0000-0000-0000-000000000067",
    "00000000-0000-0000-0000-000000000068",
    "00000000-0000-0000-0000-000000000069",
    "00000000-0000-0000-0000-00000000006a",
    "00000000-0000-0000-0000-00000000006b",
    "00000000-0000-0000-0000-00000000006c",
    "00000000-0000-0000-0000-00000000006d",
    "00000000-0000-0000-0000-00000000006e",
    "00000000-0000-0000-0000-00000000006f",
    "00000000-0000-0000-0000-000000000070",
    "00000000-0000-0000-0000-000000000071",
    "00000000-0000-0000-0000-000000000072",
    "00000000-0000-0000-0000-000000000073",
    "00000000-0000-0000-0000-000000000074",
    "00000000-0000-0000-0000-000000000075",
    "00000000-0000-0000-0000-000000000076",
    "00000000-0000-0000-0000-000000000077",
    "00000000-0000-0000-0000-000000000078",
    "00000000-0000-0000-0000-000000000079",
    "00000000-0000-0000-0000-00000000007a",
    "00000000-0000-0000-0000-00000000007b",
    "00000000-0000-0000-0000-00000000007c",
    "00000000-0000-0000-0000-00000000007d",
    "00000000-0000-0000-0000-00000000007e",
    "00000000-0000-0000-0000-00000000007f",
    "00000000-0000-0000-0000-000000000080",
    "00000000-0000-0000-0000-000000000081",
    "00000000-0000-0000-0000-000000000082",
    "00000000-0000-0000-0000-000000000083",
    "00000000-0000-0000-0000-000000000084",
    "00000000-0000-0000-0000-000000000085",
    "00000000-0000-0000-0000-000000000086",
    "00000000-0000-0000-0000-000000000087",
    "00000000-0000-0000-0000-000000000088",
    "00000000-0000-0000-0000-000000000089",
    "00000000-0000-0000-0000-00000000008a",
    "00000000-0000-0000-0000-00000000008b",
    "00000000-0000-0000-0000-00000000008c",
    "00000000-0000-0000-0000-00000000008d",
    "00000000-0000-0000-0000-00000000008e",
    "00000000-0000-0000-0000-00000000008f",
    "00000000-0000-0000-0000-000000000090",
    "00000000-0000-0000-0000-000000000091",
    "00000000-0000-0000-0000-000000000092",
    "00000000-0000-0000-0000-000000000093",
    "00000000-0000-0000-0000-000000000094",
    "00000000-0000-0000-0000-000000000095",
    "00000000-0000-0000-0000-000000000096",
    "00000000-0000-0000-0000-000000000097",
    "00000000-0000-0000-0000-000000000098",
    "00000000-0000-0000-0000-000000000099",
    "00000000-0000-0000-0000-00000000009a",
    "00000000-0000-0000-0000-00000000009b",
    "00000000-0000-0000-0000-00000000009c",
    "00000000-0000-0000-0000-00000000009d",
    "00000000-0000-0000-0000-00000000009e",
    "00000000-0000-0000-0000-00000000009f",
    "00000000-0000-0000-0000-0000000000a0",
    "00000000-0000-0000-0000-0000000000a1",
    "00000000-0000-0000-0000-0000000000a2",
    "00000000-0000-0000-0000-0000000000a3",
    "00000000-0000-0000-0000-0000000000a4",
    "00000000-0000-0000-0000-0000000000a5",
    "00000000-0000-0000-0000-0000000000a6",
    "00000000-0000-0000-0000-0000000000a7",
    "00000000-0000-0000-0000-0000000000a8",
    "00000000-0000-0000-0000-0000000000a9",
    "00000000-0000-0000-0000-0000000000aa",
    "00000000-0000-0000-0000-0000000000ab",
    "00000000-0000-0000-0000-0000000000ac",
    "00000000-0000-0000-0000-0000000000ad",
    "00000000-0000-0000-0000-0000000000ae",
    "00000000-0000-0000-0000-0000000000af",
    "00000000-0000-0000-0000-0000000000b0",
    "00000000-0000-0000-0000-0000000000b1",
    "00000000-0000-0000-0000-0000000000b2",
    "00000000-0000-0000-0000-0000000000b3",
    "00000000-0000-0000-0000-0000000000b4",
    "00000000-0000-0000-0000-0000000000b5",
    "00000000-0000-0000-0000-0000000000b6",
    "00000000-0000-0000-0000-0000000000b7",
    "00000000-0000-0000-0000-0000000000b8",
    "00000000-0000-0000-0000-0000000000b9",
    "00000000-0000-0000-0000-0000000000ba",
    "00000000-0000-0000-0000-0000000000bb",
    "00000000-0000-0000-0000-0000000000bc",
    "00000000-0000-0000-0000-0000000000bd",
    "00000000-0000-0000-0000-0000000000be",
    "00000000-0000-0000-0000-0000000000bf",
    "00000000-0000-0000-0000-0000000000c0",
    "00000000-0000-0000-0000-0000000000c1",
    "00000000-0000-0000-0000-0000000000c2",
    "00000000-0000-0000-0000-0000000000c3",
    "00000000-0000-0000-0000-0000000000c4",
    "00000000-0000-0000-0000-0000000000c5",
    "00000000-0000-0000-0000-0000000000c6",
    "00000000-0000-0000-0000-0000000000c7",
    "00000000-0000-0000-0000-0000000000c8",
    "00000000-0000-0000-0000-0000000000c9",
    "00000000-0000-0000-0000-0000000000ca",
    "00000000-0000-0000-0000-0000000000cb",
    "00000000-0000-0000-0000-0000000000cc",
    "00000000-0000-0000-0000-0000000000cd",
    "00000000-0000-0000-0000-0000000000ce",
    "00000000-0000-0000-0000-0000000000cf",
    "00000000-0000-0000-0000-0000000000d0",
];

const NR_IDS: &[&str] = &[
    "00000000-0000-0000-0000-0000000000d1",
    "00000000-0000-0000-0000-0000000000d2",
    "00000000-0000-0000-0000-0000000000d3",
    "00000000-0000-0000-0000-0000000000d4",
    "00000000-0000-0000-0000-0000000000d5",
];

const RECON_IDS: &[&str] = &[
    "00000000-0000-0000-0000-0000000000e0",
    "00000000-0000-0000-0000-0000000000e1",
    "00000000-0000-0000-0000-0000000000e2",
];

const A_IDS: &[&str] = &[
    "00000000-0000-0000-0000-0000000000f0",
    "00000000-0000-0000-0000-0000000000f1",
    "00000000-0000-0000-0000-0000000000f2",
    "00000000-0000-0000-0000-0000000000f3",
    "00000000-0000-0000-0000-0000000000f4",
    "00000000-0000-0000-0000-0000000000f5",
    "00000000-0000-0000-0000-0000000000f6",
    "00000000-0000-0000-0000-0000000000f7",
    "00000000-0000-0000-0000-0000000000f8",
    "00000000-0000-0000-0000-0000000000f9",
];

struct PSeed {
    amount: i64,
    ref_val: &'static str,
    status: &'static str,
    fail_reason: Option<&'static str>,
    hours_ago: i64,
}

static PS: &[PSeed] = &[
    PSeed { amount: 10000,  ref_val: "INV-001", status: "successful", fail_reason: None, hours_ago: 312 },
    PSeed { amount: 25000,  ref_val: "INV-002", status: "successful", fail_reason: None, hours_ago: 280 },
    PSeed { amount: 5000,   ref_val: "INV-003", status: "successful", fail_reason: None, hours_ago: 240 },
    PSeed { amount: 75000,  ref_val: "INV-004", status: "successful", fail_reason: None, hours_ago: 200 },
    PSeed { amount: 15000,  ref_val: "INV-005", status: "successful", fail_reason: None, hours_ago: 168 },
    PSeed { amount: 50000,  ref_val: "INV-006", status: "successful", fail_reason: None, hours_ago: 120 },
    PSeed { amount: 3500,   ref_val: "INV-007", status: "successful", fail_reason: None, hours_ago: 72 },
    PSeed { amount: 20000,  ref_val: "INV-008", status: "successful", fail_reason: None, hours_ago: 48 },
    PSeed { amount: 8500,   ref_val: "INV-009", status: "successful", fail_reason: None, hours_ago: 24 },
    PSeed { amount: 12000,  ref_val: "INV-010", status: "successful", fail_reason: None, hours_ago: 12 },
    PSeed { amount: 45000,  ref_val: "INV-011", status: "successful", fail_reason: None, hours_ago: 6 },
    PSeed { amount: 3000,   ref_val: "INV-012", status: "successful", fail_reason: None, hours_ago: 2 },
    PSeed { amount: 8000,   ref_val: "INV-013", status: "failed",    fail_reason: Some("insufficient_funds"), hours_ago: 96 },
    PSeed { amount: 60000,  ref_val: "INV-014", status: "failed",    fail_reason: Some("card_declined"),      hours_ago: 48 },
    PSeed { amount: 22000,  ref_val: "INV-015", status: "failed",    fail_reason: Some("processor_timeout"),  hours_ago: 12 },
    PSeed { amount: 9500,   ref_val: "INV-016", status: "failed",    fail_reason: Some("invalid_cvv"),         hours_ago: 3 },
    PSeed { amount: 18000,  ref_val: "INV-017", status: "refunded",  fail_reason: None, hours_ago: 336 },
    PSeed { amount: 42000,  ref_val: "INV-018", status: "refunded",  fail_reason: None, hours_ago: 264 },
    PSeed { amount: 7500,   ref_val: "INV-019", status: "refunded",  fail_reason: None, hours_ago: 144 },
    PSeed { amount: 33000,  ref_val: "INV-020", status: "pending",   fail_reason: None, hours_ago: 1 },
    PSeed { amount: 16000,  ref_val: "INV-021", status: "pending",   fail_reason: None, hours_ago: 1 },
    PSeed { amount: 2800,   ref_val: "INV-022", status: "pending",   fail_reason: None, hours_ago: 1 },
    PSeed { amount: 55000,  ref_val: "INV-023", status: "processing", fail_reason: None, hours_ago: 2 },
    PSeed { amount: 11000,  ref_val: "INV-024", status: "processing", fail_reason: None, hours_ago: 2 },
    PSeed { amount: 38000,  ref_val: "INV-025", status: "processing", fail_reason: None, hours_ago: 2 },
];

pub async fn seed_all(pool: &PgPool) {
    seed_actors(pool).await;
    let merchant_id = uu(MERCHANT_ID);
    let now = Utc::now();

    seed_notif_dest(pool, merchant_id, now).await;
    let successful_events = seed_payments_and_events(pool, merchant_id, now).await;
    seed_refunds_and_extra(pool, merchant_id, now, &successful_events).await;
    seed_reconciliations(pool, now).await;
    seed_audit_records(pool, merchant_id, now).await;

    tracing::info!("Dummy data inserted");
    tracing::info!("  25 payments, 5 refunds, ~100 domain events");
    tracing::info!("  5 notification records, 3 reconciliations, 10 audit records");
}

async fn seed_notif_dest(pool: &PgPool, merchant_id: Uuid, now: DateTime<Utc>) {
    sqlx::query(
        r#"
        INSERT INTO notification_destinations (id, merchant_id, destination_url, is_active, created_at, updated_at)
        VALUES ($1, $2, 'https://webhook.example.com/merchant-one', true, $3, $3)
        ON CONFLICT (id) DO UPDATE SET
            merchant_id = EXCLUDED.merchant_id,
            destination_url = EXCLUDED.destination_url,
            is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(uu(NOTIF_DEST_ID))
    .bind(merchant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to seed notification destination");
}

fn de_idx(start: usize, offset: usize) -> Uuid {
    uu(DE_IDS[start + offset])
}

async fn seed_payments_and_events(pool: &PgPool, merchant_id: Uuid, now: DateTime<Utc>) -> Vec<(Uuid, Uuid)> {
    let mut di = 0usize;
    let mut successful_events = Vec::new();

    for (i, ps) in PS.iter().enumerate() {
        let pid = uu(P_IDS[i]);
        let created = now - Duration::hours(ps.hours_ago);
        let terminal = matches!(ps.status, "successful" | "failed" | "refunded");
        let updated = if terminal { created + Duration::minutes(5) } else { created };

        let meta = serde_json::json!({"merchant_reference": ps.ref_val});
        let ik = format!("seed-ik-pay-{:02}", i + 1);

        sqlx::query(
            r#"
            INSERT INTO payments (id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at, metadata, failure_reason)
            VALUES ($1, $2, $3, 'USD', $4::text::payment_status, $5, $6, $7, $8::jsonb, $9)
            ON CONFLICT (id) DO UPDATE SET
                merchant_id = EXCLUDED.merchant_id, amount_minor = EXCLUDED.amount_minor,
                currency = EXCLUDED.currency, status = EXCLUDED.status,
                idempotency_key = EXCLUDED.idempotency_key, created_at = EXCLUDED.created_at,
                updated_at = EXCLUDED.updated_at, metadata = EXCLUDED.metadata,
                failure_reason = EXCLUDED.failure_reason
            "#,
        )
        .bind(pid)
        .bind(merchant_id)
        .bind(ps.amount)
        .bind(ps.status)
        .bind(&ik)
        .bind(created)
        .bind(updated)
        .bind(&meta)
        .bind(ps.fail_reason)
        .execute(pool)
        .await
        .expect(&format!("Failed to seed payment {}", i + 1));

        let event_list = match ps.status {
            "successful" => vec![("payment.created", ps.hours_ago), ("payment.processing", ps.hours_ago - 1), ("payment.successful", ps.hours_ago - 2)],
            "failed" => vec![("payment.created", ps.hours_ago), ("payment.processing", ps.hours_ago - 1), ("payment.failed", ps.hours_ago - 2)],
            "refunded" => vec![("payment.created", ps.hours_ago), ("payment.processing", ps.hours_ago - 1), ("payment.successful", ps.hours_ago - 2), ("payment.refunded", ps.hours_ago - 3)],
            "processing" => vec![("payment.created", ps.hours_ago), ("payment.processing", ps.hours_ago - 1)],
            "pending" => vec![("payment.created", ps.hours_ago)],
            _ => unreachable!(),
        };

        for (evt, ha) in &event_list {
            let eid = de_idx(0, di);
            di += 1;
            let ea = now - Duration::hours(*ha);
            let pl = serde_json::json!({
                "payment_id": pid.to_string(),
                "amount_minor": ps.amount,
                "currency": "USD",
                "merchant_reference": ps.ref_val,
            });

            sqlx::query(
                r#"
                INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
                VALUES ($1, $2, 'payment', $3, $4::jsonb, 1, $5)
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(eid)
            .bind(evt)
            .bind(pid)
            .bind(&pl)
            .bind(ea)
            .execute(pool)
            .await
            .expect(&format!("Failed to seed event {} for payment {}", evt, i + 1));

            if *evt == "payment.successful" && successful_events.len() < 5 {
                successful_events.push((pid, eid));
            }
        }
    }

    tracing::info!("Seeded {} payments using {} domain events", PS.len(), di);
    successful_events
}

async fn seed_refunds_and_extra(pool: &PgPool, merchant_id: Uuid, now: DateTime<Utc>, successful_events: &[(Uuid, Uuid)]) {
    let base = 100usize;
    let mut di = 0usize;

    let refund_data: [(usize, i64, &str, i64); 5] = [
        (16, 18000, "completed", 332),
        (17, 42000, "completed", 260),
        (18, 7500, "completed", 140),
        (19, 12000, "failed", 10),
        (13, 8000, "failed", 8),
    ];

    for (ri, &(pi, amt, status, ha)) in refund_data.iter().enumerate() {
        let rid = uu(R_IDS[ri]);
        let pid = uu(P_IDS[pi]);
        let created = now - Duration::hours(ha);
        let updated = created + Duration::minutes(3);
        let ik = format!("seed-ik-ref-{:02}", ri + 1);

        sqlx::query(
            r#"
            INSERT INTO refunds (id, payment_id, merchant_id, amount_minor, currency, status, idempotency_key, created_at, updated_at)
            VALUES ($1, $2, $3, $4, 'USD', $5::text::refund_status, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                payment_id = EXCLUDED.payment_id, merchant_id = EXCLUDED.merchant_id,
                amount_minor = EXCLUDED.amount_minor, currency = EXCLUDED.currency,
                status = EXCLUDED.status, idempotency_key = EXCLUDED.idempotency_key,
                created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(rid)
        .bind(pid)
        .bind(merchant_id)
        .bind(amt)
        .bind(status)
        .bind(&ik)
        .bind(created)
        .bind(updated)
        .execute(pool)
        .await
        .expect(&format!("Failed to seed refund {}", ri + 1));

        let revents: &[(&str, i64)] = if status == "completed" {
            &[("refund.created", ha), ("refund.processing", ha - 1), ("refund.completed", ha - 2)]
        } else {
            &[("refund.created", ha), ("refund.processing", ha - 1), ("refund.failed", ha - 2)]
        };

        for (evt, ha2) in revents {
            let eid = de_idx(base, di);
            di += 1;
            let ea = now - Duration::hours(*ha2);
            let pl = serde_json::json!({
                "refund_id": rid.to_string(),
                "payment_id": pid.to_string(),
                "amount_minor": amt,
                "currency": "USD",
            });

            sqlx::query(
                r#"
                INSERT INTO domain_events (id, event_type, aggregate_type, aggregate_id, payload, version, created_at)
                VALUES ($1, $2, 'refund', $3, $4::jsonb, 1, $5)
                ON CONFLICT (id) DO NOTHING
                "#,
            )
            .bind(eid)
            .bind(evt)
            .bind(rid)
            .bind(&pl)
            .bind(ea)
            .execute(pool)
            .await
            .expect(&format!("Failed to seed refund event {}", evt));
        }
    }

    for (ndi, &(_pid, eid)) in successful_events.iter().enumerate() {
        let nid = uu(NR_IDS[ndi]);
        let delivered = ndi < 4;
        let status = if delivered { "delivered" } else { "failed" };
        let error = if delivered { None } else { Some("connection_timeout") };
        let nca = now - Duration::hours(ndi as i64 * 12 + 1);

        sqlx::query(
            r#"
            INSERT INTO notification_delivery_records (id, domain_event_id, destination_url, status, attempt_count, last_attempt_at, last_error, created_at, updated_at)
            VALUES ($1, $2, 'https://webhook.example.com/merchant-one', $3::text::notification_status, $4, $5, $6, $7, $8)
            ON CONFLICT (domain_event_id, destination_url) DO UPDATE SET
                id = EXCLUDED.id,
                status = EXCLUDED.status, attempt_count = EXCLUDED.attempt_count,
                last_attempt_at = EXCLUDED.last_attempt_at, last_error = EXCLUDED.last_error,
                created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(nid)
        .bind(eid)
        .bind(status)
        .bind(if delivered { 1i32 } else { 3i32 })
        .bind(nca)
        .bind(error)
        .bind(nca)
        .bind(nca + Duration::minutes(1))
        .execute(pool)
        .await
        .expect(&format!("Failed to seed notification record {}", ndi));
    }

    tracing::info!("Seeded 5 refunds, ~15 refund events, 5 notification records");
}

async fn seed_reconciliations(pool: &PgPool, now: DateTime<Utc>) {
    let recs = [
        ("matched", 250000i64, 250000i64, 0i64, 336i64, None as Option<&str>),
        ("mismatched", 150000i64, 142000i64, 8000i64, 168i64, Some("Unmatched transaction ID INV-014")),
        ("error", 0i64, 0i64, 0i64, 12i64, Some("Processor API unavailable during reconciliation window")),
    ];

    for (i, (status, exp, act, disc, ha, notes)) in recs.iter().enumerate() {
        let we = now - Duration::hours(*ha);
        let ws = we - Duration::hours(24);
        let ra = we + Duration::minutes(30);

        sqlx::query(
            r#"
            INSERT INTO reconciliations (id, status, expected_total_minor, actual_total_minor, discrepancy_minor, currency, notes, window_start, window_end, run_at, created_at)
            VALUES ($1, $2::text::reconciliation_status, $3, $4, $5, 'USD', $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                status = EXCLUDED.status, expected_total_minor = EXCLUDED.expected_total_minor,
                actual_total_minor = EXCLUDED.actual_total_minor, discrepancy_minor = EXCLUDED.discrepancy_minor,
                currency = EXCLUDED.currency, notes = EXCLUDED.notes,
                window_start = EXCLUDED.window_start, window_end = EXCLUDED.window_end,
                run_at = EXCLUDED.run_at, created_at = EXCLUDED.created_at
            "#,
        )
        .bind(uu(RECON_IDS[i]))
        .bind(status)
        .bind(exp)
        .bind(act)
        .bind(disc)
        .bind(notes)
        .bind(ws)
        .bind(we)
        .bind(ra)
        .bind(ra)
        .execute(pool)
        .await
        .expect(&format!("Failed to seed reconciliation {}", i + 1));
    }

    tracing::info!("Seeded 3 reconciliations");
}

async fn seed_audit_records(pool: &PgPool, merchant_id: Uuid, now: DateTime<Utc>) {
    let admin_id = uu(ADMIN_ID);
    let pids: Vec<Uuid> = P_IDS.iter().map(|s| uu(s)).collect();
    let rids: Vec<Uuid> = R_IDS.iter().map(|s| uu(s)).collect();
    let recon_ids: Vec<Uuid> = RECON_IDS.iter().map(|s| uu(s)).collect();

    struct ARec {
        actor_type: &'static str,
        actor_id: Option<Uuid>,
        action: &'static str,
        resource_type: &'static str,
        resource_id: String,
        details: serde_json::Value,
        hours_ago: i64,
    }

    let recs = vec![
        ARec { actor_type: "merchant", actor_id: Some(merchant_id), action: "payment.created", resource_type: "payment", resource_id: pids[0].to_string(), details: serde_json::json!({"amount_minor": PS[0].amount}), hours_ago: 312 },
        ARec { actor_type: "system", actor_id: None, action: "payment.successful", resource_type: "payment", resource_id: pids[0].to_string(), details: serde_json::json!({"amount_minor": PS[0].amount}), hours_ago: 310 },
        ARec { actor_type: "merchant", actor_id: Some(merchant_id), action: "payment.created", resource_type: "payment", resource_id: pids[12].to_string(), details: serde_json::json!({"amount_minor": PS[12].amount}), hours_ago: 96 },
        ARec { actor_type: "system", actor_id: None, action: "payment.failed", resource_type: "payment", resource_id: pids[12].to_string(), details: serde_json::json!({"failure_reason": "insufficient_funds"}), hours_ago: 94 },
        ARec { actor_type: "merchant", actor_id: Some(merchant_id), action: "refund.created", resource_type: "refund", resource_id: rids[0].to_string(), details: serde_json::json!({"payment_id": pids[16].to_string(), "amount_minor": 18000}), hours_ago: 334 },
        ARec { actor_type: "system", actor_id: None, action: "refund.completed", resource_type: "refund", resource_id: rids[0].to_string(), details: serde_json::json!({"payment_id": pids[16].to_string()}), hours_ago: 332 },
        ARec { actor_type: "merchant", actor_id: Some(merchant_id), action: "refund.created", resource_type: "refund", resource_id: rids[3].to_string(), details: serde_json::json!({"payment_id": pids[19].to_string(), "amount_minor": 12000}), hours_ago: 11 },
        ARec { actor_type: "system", actor_id: None, action: "refund.failed", resource_type: "refund", resource_id: rids[3].to_string(), details: serde_json::json!({"payment_id": pids[19].to_string()}), hours_ago: 10 },
        ARec { actor_type: "administrator", actor_id: Some(admin_id), action: "reconciliation.run", resource_type: "reconciliation", resource_id: recon_ids[1].to_string(), details: serde_json::json!({"expected_total_minor": 150000, "actual_total_minor": 142000}), hours_ago: 168 },
        ARec { actor_type: "unknown", actor_id: None, action: "auth.authentication_failed", resource_type: "session", resource_id: "unknown".to_string(), details: serde_json::json!({"email": "unknown@example.com"}), hours_ago: 2 },
    ];

    for (i, r) in recs.iter().enumerate() {
        let oa = now - Duration::hours(r.hours_ago);
        sqlx::query(
            r#"
            INSERT INTO audit_records (id, actor_id, action, resource_type, resource_id, details, actor_type, occurred_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::text, $8, $9)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(uu(A_IDS[i]))
        .bind(r.actor_id)
        .bind(r.action)
        .bind(r.resource_type)
        .bind(&r.resource_id)
        .bind(&r.details)
        .bind(r.actor_type)
        .bind(oa)
        .bind(oa)
        .execute(pool)
        .await
        .expect(&format!("Failed to seed audit record {}", i + 1));
    }

    tracing::info!("Seeded 10 audit records");
}
