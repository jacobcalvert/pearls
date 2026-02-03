use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "pearls")]
#[command(about = "Task manager for pearls", version)]
pub struct Cli {
    /// Path to the SQLite database
    #[arg(
        long,
        env = "PEARLS_DB",
        value_name = "PATH",
        default_value = "./pearls.db"
    )]
    db: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn db_path(&self) -> PathBuf {
        if let Some(path) = &self.db {
            return path.clone();
        }

        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("pearls.db")
    }

    pub fn command(&self) -> &Commands {
        &self.command
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Tasks(TasksCommand),
}

#[derive(Debug, Parser)]
pub struct TasksCommand {
    #[command(subcommand)]
    pub command: TaskSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskSubcommand {
    List {
        #[arg(long)]
        state: Option<TaskState>,
    },
    Ready,
    Add {
        #[arg(long)]
        title: String,
        #[arg(long, value_name = "DESC")]
        description: String,
        #[arg(long, value_name = "OTHER_ID")]
        parent_of: Option<u64>,
        #[arg(long, value_name = "OTHER_ID")]
        child_of: Option<u64>,
        #[arg(long, alias = "priorit")]
        priority: Option<i64>,
    },
    UpdateMetadata {
        #[arg(long, value_name = "ID")]
        id: u64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, value_name = "DESC")]
        desc: Option<String>,
        #[arg(long)]
        priority: Option<i64>,
        #[arg(long)]
        state: Option<TaskState>,
    },
    UpdateDependency {
        #[arg(long, value_name = "ID")]
        id: u64,
        #[arg(long, value_name = "PARENT_ID", action = ArgAction::Append)]
        add_parent: Vec<u64>,
        #[arg(long, value_name = "PARENT_ID", action = ArgAction::Append)]
        remove_parent: Vec<u64>,
        #[arg(long, value_name = "CHILD_ID", action = ArgAction::Append)]
        add_child: Vec<u64>,
        #[arg(long, value_name = "CHILD_ID", action = ArgAction::Append)]
        remove_child: Vec<u64>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum TaskState {
    Ready,
    Blocked,
    InProgress,
    Closed,
}
