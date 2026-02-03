use std::path::Path;

use sea_orm_migration::MigratorTrait;
use sea_orm_migration::sea_orm::{Database, DatabaseConnection, DbErr};

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
    Ok(conn)
}
