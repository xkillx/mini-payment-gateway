use sqlx::PgPool;
use uuid::Uuid;

pub const MERCHANT_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const ADMIN_ID: &str = "00000000-0000-0000-0000-000000000002";

pub async fn seed_actors(pool: &PgPool) {
    let merchant_id = Uuid::parse_str(MERCHANT_ID).unwrap();
    let admin_id = Uuid::parse_str(ADMIN_ID).unwrap();

    sqlx::query(
        r#"
        INSERT INTO actors (id, name, email, role, is_active)
        VALUES ($1, 'Merchant One', 'merchant@example.com', 'merchant', true)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(merchant_id)
    .execute(pool)
    .await
    .expect("Failed to seed merchant actor");

    sqlx::query(
        r#"
        INSERT INTO actors (id, name, email, role, is_active)
        VALUES ($1, 'Admin One', 'admin@example.com', 'administrator', true)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(admin_id)
    .execute(pool)
    .await
    .expect("Failed to seed admin actor");

    tracing::info!("Seed data inserted");
    tracing::info!("Merchant ID: {MERCHANT_ID}");
    tracing::info!("Admin ID: {ADMIN_ID}");
    tracing::info!("Example JWT payload for merchant: {{\"sub\":\"{MERCHANT_ID}\",\"role\":\"merchant\",\"merchant_id\":\"{MERCHANT_ID}\"}}");
    tracing::info!(
        "Example JWT payload for admin: {{\"sub\":\"{ADMIN_ID}\",\"role\":\"administrator\"}}"
    );
}
