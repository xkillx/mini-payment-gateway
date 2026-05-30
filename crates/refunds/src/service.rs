use crate::models::Refund;
use crate::repository::RefundRepository;
use uuid::Uuid;

pub trait RefundService: Send + Sync {
    async fn get_refund(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error>;
    async fn list_merchant_refunds(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error>;
}

pub struct DefaultRefundService<R: RefundRepository> {
    repo: R,
}

impl<R: RefundRepository> DefaultRefundService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: RefundRepository + 'static> RefundService for DefaultRefundService<R> {
    async fn get_refund(&self, id: Uuid) -> Result<Option<Refund>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn list_merchant_refunds(&self, merchant_id: Uuid) -> Result<Vec<Refund>, sqlx::Error> {
        self.repo.find_by_merchant(merchant_id).await
    }
}
