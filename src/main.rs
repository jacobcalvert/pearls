mod cli;
mod db;

use clap::Parser;
use filelock::FileLock;
use serde::Serialize;
use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = cli::Cli::parse();
    let db_path = cli.db_path();
    let conn = db::conn::connect(&db_path)
        .await
        .unwrap_or_else(|err| panic!("failed to open db at {}: {err}", db_path.display()));
    let command = cli.command();
    let json_output = cli.json();
    let lock_path = db_path.with_extension("lock");
    let lock_path = lock_path.to_string_lossy().to_string();
    let mut lock = FileLock::new(&lock_path);

    match command {
        cli::Commands::Tasks(tasks) => match &tasks.command {
            cli::TaskSubcommand::List {
                state,
                offset,
                limit,
            } => {
                match db::tasks::list_tasks_paginated(&conn, state, *offset, *limit).await {
                    Ok(rows) => {
                        if json_output {
                            print_json(&rows);
                        } else {
                            for row in rows {
                                println!("{}", row.display_line());
                            }
                        }
                    }
                    Err(err) => {
                        eprintln!("failed to list tasks: {err}");
                    }
                }
            }
            cli::TaskSubcommand::ClaimNext => {
                let _guard = match lock.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
                    }
                };

                match db::tasks::claim_next(&conn).await {
                    Ok(Some(task)) => {
                        if json_output {
                            print_json(&task);
                        } else {
                            println!("{}", task.display_line());
                        }
                    }
                    Ok(None) => {
                        if json_output {
                            print_json(&json!({ "status": "no_ready_tasks" }));
                        } else {
                            println!("no ready tasks");
                        }
                    }
                    Err(err) => {
                        eprintln!("failed to claim next task: {err}");
                    }
                }
            }
            cli::TaskSubcommand::Add {
                title,
                description,
                parent_of,
                child_of,
                priority,
            } => {
                let _guard = match lock.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
                    }
                };

                let task = match db::tasks::add_task(&conn, title, description, *priority).await {
                    Ok(task) => task,
                    Err(err) => {
                        eprintln!("failed to add task: {err}");
                        return;
                    }
                };

                let mut dep_errors = Vec::new();
                if let Some(other) = *parent_of
                    && let Err(err) =
                        db::tasks::add_dependency(&conn, task.id, other as i64).await
                {
                    dep_errors.push(err);
                }
                if let Some(other) = *child_of
                    && let Err(err) =
                        db::tasks::add_dependency(&conn, other as i64, task.id).await
                {
                    dep_errors.push(err);
                }

                let has_deps = parent_of.is_some() || child_of.is_some();
                if !dep_errors.is_empty() {
                    eprintln!("task added but failed to update dependencies");
                    for err in dep_errors {
                        eprintln!("  - {err}");
                    }
                }

                if json_output {
                    if has_deps {
                        match db::tasks::get_task_by_id(&conn, task.id).await {
                            Ok(updated) => print_json(&updated),
                            Err(err) => eprintln!("task added but failed to load: {err}"),
                        }
                    } else {
                        print_json(&task);
                    }
                } else {
                    println!("added task #{}", task.id);
                }
            }
            cli::TaskSubcommand::UpdateMetadata {
                id,
                title,
                desc,
                priority,
                state,
            } => {
                let _guard = match lock.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
                    }
                };

                match db::tasks::update_metadata(
                    &conn,
                    *id as i64,
                    title.as_deref(),
                    desc.as_deref(),
                    *priority,
                    *state,
                )
                .await
                {
                    Ok(0) => {
                        if json_output {
                            print_json(&json!({ "status": "no_changes" }));
                        } else {
                            eprintln!("no fields to update");
                        }
                    }
                    Ok(_) => match db::tasks::get_task_by_id(&conn, *id as i64).await {
                        Ok(task) => {
                            if json_output {
                                print_json(&task);
                            } else {
                                println!("updated task #{id}");
                            }
                        }
                        Err(err) => {
                            eprintln!("task updated but failed to load: {err}");
                        }
                    },
                    Err(err) => {
                        eprintln!("failed to update task: {err}");
                    }
                }
            }
            cli::TaskSubcommand::UpdateDependency {
                id,
                add_child,
                remove_child,
            } => {
                let _guard = match lock.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        eprintln!("{err}");
                        return;
                    }
                };

                let add_child: Vec<i64> = add_child.iter().map(|v| *v as i64).collect();
                let remove_child: Vec<i64> = remove_child.iter().map(|v| *v as i64).collect();

                match db::tasks::update_dependency(
                    &conn,
                    *id as i64,
                    &add_child,
                    &remove_child,
                )
                .await
                {
                    Ok(()) => match db::tasks::get_task_by_id(&conn, *id as i64).await {
                        Ok(task) => {
                            if json_output {
                                print_json(&task);
                            } else {
                                println!("updated dependencies for #{id}");
                            }
                        }
                        Err(err) => {
                            eprintln!("dependencies updated but failed to load task: {err}");
                        }
                    },
                    Err(err) => eprintln!("failed to update dependencies: {err}"),
                }
            }
        },
    }
}

fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(payload) => println!("{payload}"),
        Err(err) => eprintln!("failed to serialize json: {err}"),
    }
}
