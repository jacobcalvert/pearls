mod cli;
mod db;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    let db_path = cli.db_path();
    let _conn = db::conn::connect(&db_path)
        .await
        .unwrap_or_else(|err| panic!("failed to open db at {}: {err}", db_path.display()));
    let _command = cli.command();
}
