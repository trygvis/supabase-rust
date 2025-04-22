use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Example::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Example::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Example::Name).string().not_null())
                    .col(ColumnDef::new(Example::Description).text())
                    .col(
                        ColumnDef::new(Example::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Example::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Example {
    Table,
    Id,
    Name,
    Description,
    CreatedAt,
}
