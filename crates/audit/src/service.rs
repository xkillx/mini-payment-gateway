use uuid::Uuid;

use crate::models::AuditRecord;
use crate::repository::AuditRepository;

pub trait AuditService: Send + Sync {
    async fn get_audit_record(&self, id: Uuid) -> Result<Option<AuditRecord>, sqlx::Error>;
    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, sqlx::Error>;
}

pub struct DefaultAuditService<R: AuditRepository> {
    repo: R,
}

impl<R: AuditRepository> DefaultAuditService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: AuditRepository + 'static> AuditService for DefaultAuditService<R> {
    async fn get_audit_record(&self, id: Uuid) -> Result<Option<AuditRecord>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, sqlx::Error> {
        self.repo.find_all().await
    }
}
