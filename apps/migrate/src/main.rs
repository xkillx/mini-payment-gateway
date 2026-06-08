use shared_config::AppConfig;
use shared_db as db;
use sqlx::PgPool;
use std::fs;
use std::path::PathBuf;

const MIGRATION_DIRS: &[&str] = &[
    "crates/payments/migrations",
    "crates/refunds/migrations",
    "crates/notifications/migrations",
    "crates/reconciliation/migrations",
    "crates/audit/migrations",
    "crates/admin/migrations",
];

#[derive(Debug)]
struct MigrationFile {
    filename: String,
    path: PathBuf,
    sort_key: String,
}

fn discover_migrations() -> Vec<MigrationFile> {
    let mut migrations = Vec::new();
    for dir in MIGRATION_DIRS {
        let path = PathBuf::from(dir);
        if !path.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "sql") {
                    let filename = path.file_name().unwrap().to_string_lossy().to_string();
                    migrations.push(MigrationFile {
                        sort_key: filename.clone(),
                        filename,
                        path,
                    });
                }
            }
        }
    }
    migrations.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));
    migrations
}

async fn create_tracking_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _migrations (
            filename TEXT PRIMARY KEY,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn run_migrations(pool: &PgPool) -> Result<(), anyhow::Error> {
    create_tracking_table(pool).await?;
    let migrations = discover_migrations();
    tracing::info!("Found {} migration files", migrations.len());

    for m in &migrations {
        let already_applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _migrations WHERE filename = $1)")
                .bind(&m.filename)
                .fetch_one(pool)
                .await?;

        if already_applied {
            tracing::info!("Skipping already-applied migration: {}", m.filename);
            continue;
        }

        let sql = fs::read_to_string(&m.path)?;
        tracing::info!("Applying migration: {}", m.filename);

        sqlx::raw_sql(&sql).execute(pool).await?;

        sqlx::query("INSERT INTO _migrations (filename) VALUES ($1)")
            .bind(&m.filename)
            .execute(pool)
            .await?;

        tracing::info!("Applied migration: {}", m.filename);
    }

    tracing::info!("All migrations applied");
    Ok(())
}

async fn check_migrations(pool: &PgPool) -> Result<(), anyhow::Error> {
    create_tracking_table(pool).await?;
    let migrations = discover_migrations();
    let applied: Vec<String> =
        sqlx::query_scalar("SELECT filename FROM _migrations ORDER BY filename")
            .fetch_all(pool)
            .await?;

    let mut all_applied = true;
    for m in &migrations {
        if !applied.contains(&m.filename) {
            tracing::warn!("Pending migration: {}", m.filename);
            all_applied = false;
        }
    }

    if all_applied {
        tracing::info!("All {} migrations applied", migrations.len());
    } else {
        tracing::warn!("Some migrations have not been applied");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = AppConfig::from_env();
    let pool = db::connect(&config.database_url).await;

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("up");

    match command {
        "up" => {
            tracing::info!("Running migrations...");
            run_migrations(&pool).await?;
            tracing::info!("Seeding data...");
            shared_db::seed::seed_actors(&pool).await;
        }
        "--check" | "check" => {
            check_migrations(&pool).await?;
        }
        "seed" => {
            shared_db::seed::seed_all(&pool).await;
        }
        other => {
            anyhow::bail!("Unknown command: {other}. Use 'up', 'check', or 'seed'.");
        }
    }

    Ok(())
}
