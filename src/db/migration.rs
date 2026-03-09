use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250203_000001_create_tables::Migration),
            Box::new(m20260309_000002_add_assignee_to_tasks::Migration),
        ]
    }
}

mod m20250203_000001_create_tables {
    use sea_orm_migration::prelude::*;
    use sea_query::{ColumnDef, Index, Table};

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20250203_000001_create_tables"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .create_table(
                    Table::create()
                        .table(Task::Table)
                        .if_not_exists()
                        .col(ColumnDef::new(Task::Id).integer().not_null().primary_key())
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
                        .col(ColumnDef::new(Dependency::ParentId).integer().not_null())
                        .col(ColumnDef::new(Dependency::ChildId).integer().not_null())
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

mod m20260309_000002_add_assignee_to_tasks {
    use sea_orm_migration::prelude::*;
    use sea_orm_migration::sea_orm::{DbBackend, Statement};
    use sea_query::{ColumnDef, Table};

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m20260309_000002_add_assignee_to_tasks"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
            manager
                .alter_table(
                    Table::alter()
                        .table(Task::Table)
                        .add_column(ColumnDef::new(Task::Assignee).text())
                        .to_owned(),
                )
                .await?;

            Ok(())
        }

        async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
            let conn = _manager.get_connection();

            let rows = conn
                .query_all(Statement::from_string(
                    DbBackend::Sqlite,
                    "PRAGMA table_info(task)".to_owned(),
                ))
                .await?;

            let mut has_assignee = false;
            for row in rows {
                let name: String = row.try_get_by_index(1)?;
                if name == "assignee" {
                    has_assignee = true;
                    break;
                }
            }

            if has_assignee {
                conn.execute(Statement::from_string(
                    DbBackend::Sqlite,
                    "ALTER TABLE task DROP COLUMN assignee".to_owned(),
                ))
                .await?;
            }

            Ok(())
        }
    }

    #[derive(DeriveIden)]
    enum Task {
        Table,
        Assignee,
    }
}
