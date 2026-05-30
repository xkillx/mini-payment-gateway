use uuid::Uuid;

use crate::models::ActorRecord;
use crate::repository::ActorRepository;

pub struct ActorService<R: ActorRepository> {
    repo: R,
}

impl<R: ActorRepository> ActorService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn get_active_actor(
        &self,
        actor_id: Uuid,
    ) -> Result<Option<ActorRecord>, sqlx::Error> {
        let actor = self.repo.find_by_id(actor_id).await?;
        match actor {
            Some(a) if a.is_active => Ok(Some(a)),
            _ => Ok(None),
        }
    }
}
