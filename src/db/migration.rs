use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250203_000001_create_tables::Migration)]
    }
}

mod m20250203_000001_create_tables {
    use sea_orm_migration::prelude::*;
    use sea_query::{ColumnDef, Index, Table};

    #[derive(DeriveMigrationName)]
    pub struct Migration;

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Task::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Task::Id)
                                .integer()
                                .not_null()
                                .primary_key(),
                        )
                        .col(ColumnDef::new(Task::Title).text())
                        .col(ColumnDef::new(Task::Desc).text())
                        .col(
                            ColumnDef::new(Task::Priority)
                                .integer()
                                .not_null()
                                .default(1),
                        )
                        .col(
                            ColumnDef::new(Task::State)
                                .text()
                                .not_null()
                                .default("ready"),
                        )
                        .to_owned(),
                )
                .await?;

            manager
                .create_table(
                    Table::create()
                        .table(Dependency::Table)
                        .if_not_exists()
                        .col(
                            ColumnDef::new(Dependency::ParentId)
                                .integer()
                                .not_null(),
                        )
                        .col(
                            ColumnDef::new(Dependency::ChildId)
                                .integer()
                                .not_null(),
                        )
                        .primary_key(
                            Index::create()
                                .col(Dependency::ParentId)
                                .col(Dependency::ChildId),
                        )
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .drop_table(Table::drop().table(Dependency::Table).to_owned())
                .await?;
            manager
                .drop_table(Table::drop().table(Task::Table).to_owned())
                .await?;
            Ok(())
        }
    }

    #[derive(DeriveIden)]
    enum Task {
        Table,
        Id,
        Title,
        Desc,
        Priority,
        State,
    }

    #[derive(DeriveIden)]
    enum Dependency {
        Table,
        ParentId,
        ChildId,
    }
}
