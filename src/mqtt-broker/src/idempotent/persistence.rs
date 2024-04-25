use axum::async_trait;
use dashmap::DashMap;

use super::Idempotent;

pub struct IdempotentPersistence {}

impl IdempotentPersistence {
    pub fn new() -> Self {
        return IdempotentPersistence {};
    }
}

#[async_trait]
impl Idempotent for IdempotentPersistence {
    async fn save_idem_data(&self, topic_id: String, pkid: u16) {}

    async fn delete_idem_data(&self, topic_id: String, pkid: u16) {}

    async fn idem_data_exists(&self, topic_id: String, pkid: u16) -> bool {
        return false;
    }

    async fn idem_data(&self) -> DashMap<String, u64> {
        return DashMap::with_capacity(256);
    }
}
