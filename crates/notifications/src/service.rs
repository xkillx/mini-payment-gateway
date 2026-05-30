use uuid::Uuid;

use crate::models::NotificationDeliveryRecord;
use crate::repository::NotificationRepository;

pub trait NotificationService: Send + Sync {
    async fn get_notification(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecord>, sqlx::Error>;
    async fn create_notification(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error>;
}

pub struct DefaultNotificationService<R: NotificationRepository> {
    repo: R,
}

impl<R: NotificationRepository> DefaultNotificationService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R: NotificationRepository + 'static> NotificationService for DefaultNotificationService<R> {
    async fn get_notification(
        &self,
        id: Uuid,
    ) -> Result<Option<NotificationDeliveryRecord>, sqlx::Error> {
        self.repo.find_by_id(id).await
    }

    async fn create_notification(
        &self,
        record: &NotificationDeliveryRecord,
    ) -> Result<NotificationDeliveryRecord, sqlx::Error> {
        self.repo.insert(record).await
    }
}
