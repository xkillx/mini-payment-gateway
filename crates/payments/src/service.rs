use crate::models::Payment;
use crate::repository::PaymentRepository;
use uuid::Uuid;

pub trait PaymentService: Send + Sync {
    async fn get_payment(&self, id: Uuid) -> Result<Option<Payment>, sqlx::Error>;
    async fn list_merchant_payments(&self, merchant_id: Uuid) -> Result<Vec<Payment>, sqlx::Error>;
}

pub struct DefaultPaymentService<R: PaymentRepository> {
    repo: R,
}

impl<R: PaymentRepository> DefaultPaymentService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: PaymentRepository + 'static> PaymentService for DefaultPaymentService<R> {
    async fn get_payment(&self, id: Uuid) -> Result<Option<Payment>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn list_merchant_payments(&self, merchant_id: Uuid) -> Result<Vec<Payment>, sqlx::Error> {
        self.repo.find_by_merchant(merchant_id).await
    }
}
