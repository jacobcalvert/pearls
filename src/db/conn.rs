use std::path::Path;

use sea_orm_migration::MigratorTrait;
use sea_orm_migration::prelude::ConnectionTrait;
use sea_orm_migration::sea_orm::{Database, DatabaseConnection, DbBackend, DbErr, Statement};

use crate::db::migration::Migrator;

pub async fn connect(path: &Path) -> Result<DatabaseConnection, DbErr> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(DbErr::Custom(format!(
            "failed to create db directory {}: {err}",
            parent.display()
        )));
    }

    let mut url = if path.is_absolute() {
        let path_str = path.display().to_string();
        let trimmed = path_str.strip_prefix('/').unwrap_or(&path_str);
        format!("sqlite:///{}", trimmed)
    } else {
        format!("sqlite://{}", path.display())
    };
    if !url.contains('?') {
        url.push_str("?mode=rwc");
    }
    let conn = Database::connect(&url).await?;
    Migrator::up(&conn, None).await?;
    ensure_assignee_column(&conn).await?;
    Ok(conn)
}

async fn ensure_assignee_column(conn: &DatabaseConnection) -> Result<(), DbErr> {
    let rows = conn
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA table_info(task)".to_owned(),
        ))
        .await?;

    let mut has_assignee = false;
    for row in rows {
        let name: String = row.try_get_by_index::<String>(1)?;
        if name == "assignee" {
            has_assignee = true;
            break;
        }
    }

    if has_assignee {
        return Ok(());
    }

    conn.execute(Statement::from_string(
        DbBackend::Sqlite,
        "ALTER TABLE task ADD COLUMN assignee TEXT".to_owned(),
    ))
    .await?;

    Ok(())
}
