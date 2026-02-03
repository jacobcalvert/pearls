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

    /// Output JSON instead of text
    #[arg(long, global = true)]
    json: bool,

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

    pub fn json(&self) -> bool {
        self.json
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
    /// List all tasks matching a given filter (or all tasks by default)
    List {
        #[arg(
            long,
            value_name = "STATES",
            value_delimiter = ',',
            default_value = "ready,blocked,in_progress",
            help = "Comma-separated states to include"
        )]
        state: Vec<TaskState>,
    },
    /// List ready tasks
    Ready,
    /// Add a task with a given title, description, and optional priority, parent, and child
    Add {
        #[arg(long, value_name = "TITLE", help = "Task title")]
        title: String,
        #[arg(long, value_name = "DESC", help = "Task description")]
        description: String,
        #[arg(
            long,
            value_name = "OTHER_ID",
            help = "Make this task the parent of the given task id"
        )]
        parent_of: Option<u64>,
        #[arg(
            long,
            value_name = "OTHER_ID",
            help = "Make this task the child of the given task id"
        )]
        child_of: Option<u64>,
        #[arg(
            long,
            value_name = "NUM",
            help = "Task priority (lower is more important)"
        )]
        priority: Option<i64>,
    },
    /// Update the metadata associated with a given task ID
    UpdateMetadata {
        #[arg(long, value_name = "ID", help = "Task id to update")]
        id: u64,
        #[arg(long, value_name = "TITLE", help = "New title (optional)")]
        title: Option<String>,
        #[arg(long, value_name = "DESC", help = "New description (optional)")]
        desc: Option<String>,
        #[arg(long, value_name = "NUM", help = "New priority (optional)")]
        priority: Option<i64>,
        #[arg(long, value_name = "STATE", help = "New state (optional)")]
        state: Option<TaskState>,
    },
    /// Update dependency relationships for a given task ID
    UpdateDependency {
        #[arg(long, value_name = "ID", help = "Task id to update")]
        id: u64,
        #[arg(long, value_name = "PARENT_ID", action = ArgAction::Append, help = "Add parent dependency (repeatable)")]
        add_parent: Vec<u64>,
        #[arg(long, value_name = "PARENT_ID", action = ArgAction::Append, help = "Remove parent dependency (repeatable)")]
        remove_parent: Vec<u64>,
        #[arg(long, value_name = "CHILD_ID", action = ArgAction::Append, help = "Add child dependency (repeatable)")]
        add_child: Vec<u64>,
        #[arg(long, value_name = "CHILD_ID", action = ArgAction::Append, help = "Remove child dependency (repeatable)")]
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
