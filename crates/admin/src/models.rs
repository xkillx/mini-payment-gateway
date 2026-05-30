use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActorRecord {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub merchant_id: Option<Uuid>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
