use std::sync::Arc;

use entity::{messages, users};
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter};
use tokio::sync::OnceCell;

use crate::config;

pub mod entity;

#[derive(Clone)]
pub struct UserRepository {
    pub lazy_connector: Arc<LazyConnector>,
}

impl UserRepository {
    pub async fn get_all(&self) -> anyhow::Result<Vec<users::Model>> {
        let connection = self.lazy_connector.get_connection().await?;

        let users = users::Entity::find().all(connection).await?;

        Ok(users)
    }

    pub async fn get_by_username(&self, username: String) -> anyhow::Result<Option<users::Model>> {
        let connection = self.lazy_connector.get_connection().await?;

        let filter = users::Column::Username.eq(username);
        let user = users::Entity::find().filter(filter).one(connection).await?;

        Ok(user)
    }

    pub async fn get_by_id(&self, id: i32) -> anyhow::Result<Option<users::Model>> {
        let connection = self.lazy_connector.get_connection().await?;

        let user = users::Entity::find_by_id(id).one(connection).await?;

        Ok(user)
    }

    pub async fn insert(&self, model: users::ActiveModel) -> anyhow::Result<()> {
        let connection = self.lazy_connector.get_connection().await?;

        model.insert(connection).await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct MessageRepository {
    pub lazy_connector: Arc<LazyConnector>,
}

impl MessageRepository {
    pub async fn insert(&self, model: messages::ActiveModel) -> anyhow::Result<()> {
        let connection = self.lazy_connector.get_connection().await?;

        model.insert(connection).await?;

        Ok(())
    }

    pub async fn get_messages_between_users(
        &self,
        first_user_id: i32,
        second_user_id: i32,
    ) -> anyhow::Result<Vec<messages::Model>> {
        let connection = self.lazy_connector.get_connection().await?;

        let filter = Condition::any()
            .add(
                Condition::all()
                    .add(messages::Column::FromUserId.eq(first_user_id))
                    .add(messages::Column::ToUserId.eq(second_user_id)),
            )
            .add(
                Condition::all()
                    .add(messages::Column::FromUserId.eq(second_user_id))
                    .add(messages::Column::ToUserId.eq(first_user_id)),
            );

        let messages = messages::Entity::find()
            .filter(filter)
            .all(connection)
            .await?;

        Ok(messages)
    }
}

pub struct LazyConnector {
    pub db_url: String,
    pub db_pool: OnceCell<sea_orm::DatabaseConnection>,
}

impl Default for LazyConnector {
    fn default() -> Self {
        Self {
            db_url: config::DATABASE_URL.clone(),
            db_pool: OnceCell::new(),
        }
    }
}

impl LazyConnector {
    async fn get_connection(&self) -> anyhow::Result<&sea_orm::DatabaseConnection> {
        self.db_pool
            .get_or_try_init(|| async {
                let db = sea_orm::Database::connect(&self.db_url).await?;

                Ok(db)
            })
            .await
    }
}
