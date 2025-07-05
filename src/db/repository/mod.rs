pub mod implementation;

use super::{
    DbConnectionContainerTrait, Identifier, LazyConnector, Pagination,
    entity::{messages, rooms, rooms_users, users},
};
use nultr_shared_lib::request::UuidIdentifier;
use sea_orm::{
    ColumnTrait, DbBackend, EntityTrait, FromQueryResult, JoinType,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Statement,
    prelude::Expr,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    pub lazy_connector: Arc<LazyConnector>,
}

impl UserRepository {
    pub async fn get_by_username(&self, username: String) -> anyhow::Result<Option<users::Model>> {
        let connection = self.get_connection().await?;
        let filter = users::Column::Username.eq(username);
        let user = users::Entity::find().filter(filter).one(connection).await?;

        Ok(user)
    }
}

#[derive(Clone)]
pub struct RoomRepository {
    pub lazy_connector: Arc<LazyConnector>,
}

#[derive(Debug, FromQueryResult)]
pub struct PersonalizedRoomData {
    pub id: Identifier,
    pub name: String,
}

impl RoomRepository {
    pub async fn get_for_user(
        &self,
        user_id: Identifier,
    ) -> anyhow::Result<Vec<PersonalizedRoomData>> {
        let connection = self.get_connection().await?;
        let query = r#"
            SELECT r.id as id, COALESCE(ru.generated_room_name, r.name, '#' || CAST(r.id as TEXT)) as name
            FROM rooms r
            INNER JOIN rooms_users ru ON r.id = ru.room_id
            WHERE ru.user_id = ?
        "#;

        let rooms: Vec<PersonalizedRoomData> = PersonalizedRoomData::find_by_statement(
            Statement::from_sql_and_values(DbBackend::Sqlite, query, vec![user_id.into()]),
        )
        .all(connection)
        .await?;

        Ok(rooms)
    }

    pub async fn insert_rooms_users(
        &self,
        models: Vec<rooms_users::ActiveModel>,
    ) -> anyhow::Result<()> {
        let connection = self.get_connection().await?;
        rooms_users::Entity::insert_many(models)
            .exec(connection)
            .await?;

        Ok(())
    }

    pub async fn get_users_by_room(
        &self,
        room_id: Identifier,
    ) -> anyhow::Result<Vec<users::Model>> {
        let connection = self.get_connection().await?;
        let users = users::Entity::find()
            .join(JoinType::InnerJoin, users::Relation::RoomsUsers.def())
            .join(JoinType::InnerJoin, rooms_users::Relation::Rooms.def())
            .filter(rooms::Column::Id.eq(room_id))
            .all(connection)
            .await?;

        Ok(users)
    }
}

#[derive(Clone)]
pub struct MessageRepository {
    pub lazy_connector: Arc<LazyConnector>,
}

impl MessageRepository {
    pub async fn get_message_by_uuid(&self, uuid: Uuid) -> anyhow::Result<Option<messages::Model>> {
        let model = messages::Entity::find()
            .filter(messages::Column::Uuid.eq(uuid))
            .one(self.get_connection().await?)
            .await?;

        Ok(model)
    }

    pub async fn get_messages_by_room(
        &self,
        room_id: Identifier,
        pagination: Pagination,
    ) -> anyhow::Result<Vec<messages::Model>> {
        let connection = self.get_connection().await?;
        let paginator = messages::Entity::find()
            .filter(messages::Column::RoomId.eq(room_id))
            .order_by_desc(messages::Column::CreatedAt)
            .paginate(connection, pagination.page_size);

        let messages = paginator.fetch_page(pagination.page).await?;

        Ok(messages)
    }

    pub async fn mark_messages_read(&self, message_uuids: Vec<UuidIdentifier>) -> anyhow::Result<()> {
        let connection = self.get_connection().await?;

        messages::Entity::update_many()
            .col_expr(messages::Column::Read, Expr::value(true))
            .filter(Expr::col(messages::Column::Uuid).is_in(message_uuids))
            .exec(connection)
            .await?;

        Ok(())
    }
}
