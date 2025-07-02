use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(pk_auto(Users::Id))
                    .col(string(Users::Username))
                    .col(string(Users::PasswordHash))
                    .index(
                        Index::create()
                            .name("idx-unique-user-name")
                            .col(Users::Username)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Rooms::Table)
                    .if_not_exists()
                    .col(pk_auto(Rooms::Id))
                    .col(string(Rooms::Name))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RoomsUsers::Table)
                    .if_not_exists()
                    .col(integer(RoomsUsers::RoomId))
                    .col(integer(RoomsUsers::UserId))
                    .primary_key(
                        Index::create()
                            .col(RoomsUsers::RoomId)
                            .col(RoomsUsers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-rooms-users-user_id")
                            .from(RoomsUsers::Table, RoomsUsers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-rooms-users-room_id")
                            .from(RoomsUsers::Table, RoomsUsers::RoomId)
                            .to(Rooms::Table, Rooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(pk_auto(Messages::Id))
                    .col(uuid(Messages::Uuid))
                    .col(date_time(Messages::CreatedAt))
                    .col(string(Messages::Content))
                    .col(integer(Messages::UserId))
                    .col(integer(Messages::RoomId))
                    .col(boolean(Messages::Read))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-messages-user_id")
                            .from(Messages::Table, Messages::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-messages-room_id")
                            .from(Messages::Table, Messages::RoomId)
                            .to(Rooms::Table, Rooms::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .name("idx-unique-message-uuid")
                            .col(Messages::Uuid)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(RoomsUsers::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Rooms::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash,
}

#[derive(DeriveIden)]
enum RoomsUsers {
    Table,
    RoomId,
    UserId,
}

#[derive(DeriveIden)]
enum Rooms {
    Table,
    Id,
    Name
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    Uuid,
    UserId,
    RoomId,
    Read,
    Content,
    CreatedAt,
}
