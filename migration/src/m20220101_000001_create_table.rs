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
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(pk_uuid(Messages::Id))
                    .col(date_time(Messages::CreatedAt))
                    .col(string(Messages::Content))
                    .col(integer(Messages::FromUserId))
                    .col(integer(Messages::ToUserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-messages-from_user_id")
                            .from(Messages::Table, Messages::FromUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-messages-to_user_id")
                            .from(Messages::Table, Messages::ToUserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
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
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Username,
    PasswordHash
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
    Uuid,
    FromUserId,
    ToUserId,
    Content,
    CreatedAt,
}
