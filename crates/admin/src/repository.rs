use sqlx::PgPool;
use uuid::Uuid;

use crate::models::ActorRecord;

pub trait ActorRepository: Send + Sync {
    async fn find_by_id(&self, actor_id: Uuid) -> Result<Option<ActorRecord>, sqlx::Error>;
}

pub struct PostgresActorRepository {
    pool: PgPool,
}

impl PostgresActorRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ActorRepository for PostgresActorRepository {
    async fn find_by_id(&self, actor_id: Uuid) -> Result<Option<ActorRecord>, sqlx::Error> {
        sqlx::query_as::<_, ActorRecord>("SELECT * FROM actors WHERE id = $1")
            .bind(actor_id)
            .fetch_optional(&self.pool)
            .await
    }
}
