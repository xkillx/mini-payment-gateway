use std::env;
use std::fmt::Debug;

#[derive(Clone, serde::Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub app_port: u16,
    pub jwt_secret: String,
    pub log_level: String,
    pub worker_poll_interval_ms: u64,
    pub notification_max_attempts: u32,
    pub notification_retry_delays_secs: Vec<u64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: "postgres://payment:payment@localhost:5432/payment_gateway".into(),
            app_port: 4000,
            jwt_secret: "dev-secret-change-in-production".into(),
            log_level: "info".into(),
            worker_poll_interval_ms: 1000,
            notification_max_attempts: 5,
            notification_retry_delays_secs: vec![30, 120, 600, 1800, 7200],
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        Self {
            database_url: env_var("DATABASE_URL"),
            app_port: env_var("APP_PORT").parse().unwrap_or(4000),
            jwt_secret: env_var("JWT_SECRET"),
            log_level: env_var("LOG_LEVEL"),
            worker_poll_interval_ms: env_var("WORKER_POLL_INTERVAL_MS").parse().unwrap_or(1000),
            notification_max_attempts: env_var("NOTIFICATION_MAX_ATTEMPTS").parse().unwrap_or(5),
            notification_retry_delays_secs: env_var("NOTIFICATION_RETRY_DELAYS_SECS")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect(),
        }
    }
}

fn env_var(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| String::new())
}

impl Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"***")
            .field("app_port", &self.app_port)
            .field("jwt_secret", &"***")
            .field("log_level", &self.log_level)
            .field("worker_poll_interval_ms", &self.worker_poll_interval_ms)
            .field("notification_max_attempts", &self.notification_max_attempts)
            .field(
                "notification_retry_delays_secs",
                &self.notification_retry_delays_secs,
            )
            .finish()
    }
}
