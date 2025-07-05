use sea_orm::prelude::async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, DatabaseTransaction, EntityTrait,
    IntoActiveModel, JoinType, PaginatorTrait, QueryFilter, QuerySelect, RelationTrait,
    TransactionTrait,
};
use sea_orm::{ModelTrait, PrimaryKeyTrait, QueryOrder};
use serde::Deserialize;
use tokio::sync::OnceCell;

use crate::config;

pub mod entity;
pub mod repository;

#[async_trait]
pub trait DbConnectionContainerTrait {
    async fn get_connection(&self) -> anyhow::Result<&DatabaseConnection>;

    // TODO: add macro
    async fn begin_transaction(&self) -> anyhow::Result<DatabaseTransaction> {
        let connection = self.get_connection().await?;
        let transaction = connection.begin().await?;
        Ok(transaction)
    }

    async fn end_transaction(&self, txn: DatabaseTransaction) -> anyhow::Result<()> {
        txn.commit().await?;

        Ok(())
    }
}

type Identifier = nultr_shared_lib::request::Identifier;

#[async_trait]
pub trait RepositoryTrait<E>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E> + Send,
    E::Model: IntoActiveModel<E::ActiveModel>,
    <E::PrimaryKey as PrimaryKeyTrait>::ValueType: From<Identifier>,
    Self: DbConnectionContainerTrait,
{
    async fn exists_by_id(&self, id: Identifier) -> anyhow::Result<bool> {
        let connection = self.get_connection().await?;
        let exists = E::find_by_id(id).one(connection).await?.is_some();

        Ok(exists)
    }

    async fn get_by_id(&self, id: Identifier) -> anyhow::Result<Option<E::Model>> {
        let connection = self.get_connection().await?;
        let user = E::find_by_id(id).one(connection).await?;

        Ok(user)
    }

    async fn update(&self, model: E::ActiveModel) -> anyhow::Result<()> {
        let connection = self.get_connection().await?;
        model.update(connection).await?;

        Ok(())
    }

    async fn insert(&self, active_model: E::ActiveModel) -> anyhow::Result<E::Model> {
        let connection = self.get_connection().await?;
        let model = active_model.insert(connection).await?;

        Ok(model)
    }

    async fn delete(&self, model: E::ActiveModel) -> anyhow::Result<()> {
        let connection = self.get_connection().await?;
        model.delete(connection).await?;

        Ok(())
    }

    async fn get_all(&self) -> anyhow::Result<Vec<E::Model>> {
        let connection = self.get_connection().await?;
        let models = E::find().all(connection).await?;

        Ok(models)
    }
}

pub struct LazyConnector {
    pub db_url: String,
    pub db_pool: OnceCell<DatabaseConnection>,
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
    async fn get_connection(&self) -> anyhow::Result<&DatabaseConnection> {
        self.db_pool
            .get_or_try_init(|| async {
                let db = sea_orm::Database::connect(&self.db_url).await?;

                Ok(db)
            })
            .await
    }
}

#[derive(Deserialize)]
pub struct Pagination {
    pub page: u64,
    pub page_size: u64,
}
