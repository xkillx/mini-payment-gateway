use uuid::Uuid;

use crate::models::Reconciliation;
use crate::repository::ReconciliationRepository;

pub trait ReconciliationService: Send + Sync {
    async fn get_reconciliation(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error>;
    async fn list_reconciliations(&self) -> Result<Vec<Reconciliation>, sqlx::Error>;
}

pub struct DefaultReconciliationService<R: ReconciliationRepository> {
    repo: R,
}

impl<R: ReconciliationRepository> DefaultReconciliationService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: ReconciliationRepository + 'static> ReconciliationService
    for DefaultReconciliationService<R>
{
    async fn get_reconciliation(&self, id: Uuid) -> Result<Option<Reconciliation>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn list_reconciliations(&self) -> Result<Vec<Reconciliation>, sqlx::Error> {
        self.repo.find_all().await
    }
}
