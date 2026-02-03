use std::path::Path;

use sea_orm_migration::MigratorTrait;
use sea_orm_migration::sea_orm::{Database, DatabaseConnection};

use crate::db::migration::Migrator;

use eyre::Result;

pub async fn connect(path: &Path) -> Result<DatabaseConnection> {
    if let Some(parent) = path.parent()
        && let Err(err) = std::fs::create_dir_all(parent)
    {
        return Err(eyre::eyre!(
            "failed to create db directory {}: {err}",
            parent.display()
        ));
    }

    if !path.exists() {
        std::fs::File::create(path)?;
    }

    let url = format!("sqlite://{}", path.display());
    let conn = Database::connect(&url).await?;
    Migrator::up(&conn, None).await?;
    Ok(conn)
}
