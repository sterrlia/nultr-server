use sea_orm::{DatabaseConnection, prelude::async_trait::async_trait};

use crate::db::{entity::{messages, rooms, users}, DbConnectionContainerTrait, RepositoryTrait};

use super::{MessageRepository, RoomRepository, UserRepository};

#[async_trait]
impl DbConnectionContainerTrait for UserRepository {
    async fn get_connection(&self) -> anyhow::Result<&DatabaseConnection> {
        self.lazy_connector.get_connection().await
    }
}

impl RepositoryTrait<users::Entity> for UserRepository {}

#[async_trait]
impl DbConnectionContainerTrait for RoomRepository {
    async fn get_connection(&self) -> anyhow::Result<&DatabaseConnection> {
        self.lazy_connector.get_connection().await
    }
}

impl RepositoryTrait<rooms::Entity> for RoomRepository {}

#[async_trait]
impl DbConnectionContainerTrait for MessageRepository {
    async fn get_connection(&self) -> anyhow::Result<&DatabaseConnection> {
        self.lazy_connector.get_connection().await
    }
}

impl RepositoryTrait<messages::Entity> for MessageRepository {}

