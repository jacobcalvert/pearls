use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use eyre::Result;
use filelock::FileLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::cli::TaskState;
use crate::db;
use sea_orm_migration::sea_orm::DatabaseConnection;

const PROTOCOL_VERSION: &str = "2024-11-05";

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub async fn serve_stdio(db_path: PathBuf) -> Result<()> {
    let conn = db::conn::connect(&db_path).await?;
    let lock_path = db_path.with_extension("lock");
    let lock_path = lock_path.to_string_lossy().to_string();
    let mut lock = FileLock::new(&lock_path);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                writeln!(stdout, "failed to read stdin: {err}")?;
                stdout.flush()?;
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(request) => request,
            Err(err) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32600,
                        message: format!("invalid request: {err}"),
                        data: None,
                    }),
                };
                write_response(&mut stdout, &response)?;
                continue;
            }
        };

        if let Some(version) = request.jsonrpc.as_deref()
            && version != "2.0"
            && request.id.is_some()
        {
            let response = json_rpc_error(
                request.id.clone().unwrap_or(Value::Null),
                -32600,
                "invalid jsonrpc version",
                None,
            );
            write_response(&mut stdout, &response)?;
            continue;
        }

        let Some(id) = request.id.clone() else {
            // Notification: no response.
            continue;
        };

        let response = match request.method.as_str() {
            "initialize" => handle_initialize(id),
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, request.params, &conn, &mut lock).await,
            "shutdown" => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: Some(Value::Null),
                error: None,
            },
            _ => JsonRpcResponse {
                jsonrpc: "2.0",
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: "method not found".to_string(),
                    data: None,
                }),
            },
        };

        write_response(&mut stdout, &response)?;
    }

    Ok(())
}

fn write_response(stdout: &mut io::Stdout, response: &JsonRpcResponse) -> Result<()> {
    let payload = serde_json::to_string(response)?;
    writeln!(stdout, "{payload}")?;
    stdout.flush()?;
    Ok(())
}

fn handle_initialize(id: Value) -> JsonRpcResponse {
    let result = json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {},
        },
        "serverInfo": {
            "name": "pearls",
            "version": env!("CARGO_PKG_VERSION"),
        }
    });

    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn handle_tools_list(id: Value) -> JsonRpcResponse {
    let tools = vec![
        json!({
            "name": "tasks.list",
            "description": "List tasks, optionally filtering by state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "state": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["ready", "blocked", "in_progress", "closed"]
                        },
                        "description": "States to include (defaults to ready, blocked, in_progress)."
                    }
                }
            }
        }),
        json!({
            "name": "tasks.claim_next",
            "description": "Claim the highest-priority ready task.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "tasks.add",
            "description": "Add a task with title, description, and optional relationships.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "parent_of": { "type": "integer" },
                    "child_of": { "type": "integer" },
                    "priority": { "type": "integer" }
                },
                "required": ["title", "description"]
            }
        }),
        json!({
            "name": "tasks.update_metadata",
            "description": "Update a task's title, description, priority, or state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "title": { "type": "string" },
                    "desc": { "type": "string" },
                    "priority": { "type": "integer" },
                    "state": {
                        "type": "string",
                        "enum": ["ready", "blocked", "in_progress", "closed"]
                    }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "tasks.update_dependency",
            "description": "Update child dependencies for a task.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "integer" },
                    "add_child": { "type": "array", "items": { "type": "integer" } },
                    "remove_child": { "type": "array", "items": { "type": "integer" } }
                },
                "required": ["id"]
            }
        }),
    ];

    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

async fn handle_tools_call(
    id: Value,
    params: Option<Value>,
    conn: &DatabaseConnection,
    lock: &mut FileLock,
) -> JsonRpcResponse {
    let Some(params) = params else {
        return json_rpc_error(id, -32602, "missing params", None);
    };
    let name = params.get("name").and_then(Value::as_str);
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    let Some(name) = name else {
        return json_rpc_error(id, -32602, "missing tool name", None);
    };

    match name {
        "tasks.list" => {
            let states = match parse_states(&args) {
                Ok(states) => states,
                Err(err) => return json_rpc_error(id, -32602, &err, None),
            };
            match db::tasks::list_tasks(conn, &states).await {
                Ok(rows) => json_rpc_ok(id, json!({ "content": [text_content(&rows)] })),
                Err(err) => tool_error(id, &format!("failed to list tasks: {err}")),
            }
        }
        "tasks.claim_next" => {
            let _guard = match lock.lock() {
                Ok(guard) => guard,
                Err(err) => return tool_error(id, &format!("{err}")),
            };
            match db::tasks::claim_next(conn).await {
                Ok(Some(task)) => json_rpc_ok(id, json!({ "content": [text_content(&task)] })),
                Ok(None) => json_rpc_ok(
                    id,
                    json!({ "content": [text_content(&json!({ "status": "no_ready_tasks" }))] }),
                ),
                Err(err) => tool_error(id, &format!("failed to claim next task: {err}")),
            }
        }
        "tasks.add" => {
            let title = match args.get("title").and_then(Value::as_str) {
                Some(title) => title,
                None => return json_rpc_error(id, -32602, "missing title", None),
            };
            let description = match args.get("description").and_then(Value::as_str) {
                Some(description) => description,
                None => return json_rpc_error(id, -32602, "missing description", None),
            };
            let parent_of = args.get("parent_of").and_then(Value::as_i64);
            let child_of = args.get("child_of").and_then(Value::as_i64);
            let priority = args.get("priority").and_then(Value::as_i64);

            let _guard = match lock.lock() {
                Ok(guard) => guard,
                Err(err) => return tool_error(id, &format!("{err}")),
            };

            let task = match db::tasks::add_task(conn, title, description, priority).await {
                Ok(task) => task,
                Err(err) => return tool_error(id, &format!("failed to add task: {err}")),
            };

            let mut dep_errors = Vec::new();
            if let Some(other) = parent_of
                && let Err(err) = db::tasks::add_dependency(conn, task.id, other).await
            {
                dep_errors.push(err);
            }
            if let Some(other) = child_of
                && let Err(err) = db::tasks::add_dependency(conn, other, task.id).await
            {
                dep_errors.push(err);
            }

            if !dep_errors.is_empty() {
                return tool_error(id, "task added but failed to update dependencies");
            }

            if parent_of.is_some() || child_of.is_some() {
                match db::tasks::get_task_by_id(conn, task.id).await {
                    Ok(updated) => json_rpc_ok(id, json!({ "content": [text_content(&updated)] })),
                    Err(err) => tool_error(id, &format!("task added but failed to load: {err}")),
                }
            } else {
                json_rpc_ok(id, json!({ "content": [text_content(&task)] }))
            }
        }
        "tasks.update_metadata" => {
            let id_value = match args.get("id").and_then(Value::as_i64) {
                Some(id_value) => id_value,
                None => return json_rpc_error(id, -32602, "missing id", None),
            };
            let title = args.get("title").and_then(Value::as_str);
            let desc = args.get("desc").and_then(Value::as_str);
            let priority = args.get("priority").and_then(Value::as_i64);
            let state = match args.get("state").and_then(Value::as_str) {
                Some(value) => match parse_state(value) {
                    Ok(state) => Some(state),
                    Err(err) => return json_rpc_error(id, -32602, &err, None),
                },
                None => None,
            };

            let _guard = match lock.lock() {
                Ok(guard) => guard,
                Err(err) => return tool_error(id, &format!("{err}")),
            };

            match db::tasks::update_metadata(conn, id_value, title, desc, priority, state).await {
                Ok(0) => json_rpc_ok(
                    id,
                    json!({ "content": [text_content(&json!({ "status": "no_changes" }))] }),
                ),
                Ok(_) => match db::tasks::get_task_by_id(conn, id_value).await {
                    Ok(task) => json_rpc_ok(id, json!({ "content": [text_content(&task)] })),
                    Err(err) => tool_error(id, &format!("task updated but failed to load: {err}")),
                },
                Err(err) => tool_error(id, &format!("failed to update task: {err}")),
            }
        }
        "tasks.update_dependency" => {
            let id_value = match args.get("id").and_then(Value::as_i64) {
                Some(id_value) => id_value,
                None => return json_rpc_error(id, -32602, "missing id", None),
            };
            let add_child = match args.get("add_child") {
                Some(Value::Array(values)) => match values_to_ids(values) {
                    Ok(values) => values,
                    Err(err) => return json_rpc_error(id, -32602, &err, None),
                },
                Some(_) => return json_rpc_error(id, -32602, "add_child must be an array", None),
                None => Vec::new(),
            };
            let remove_child = match args.get("remove_child") {
                Some(Value::Array(values)) => match values_to_ids(values) {
                    Ok(values) => values,
                    Err(err) => return json_rpc_error(id, -32602, &err, None),
                },
                Some(_) => {
                    return json_rpc_error(id, -32602, "remove_child must be an array", None);
                }
                None => Vec::new(),
            };

            let _guard = match lock.lock() {
                Ok(guard) => guard,
                Err(err) => return tool_error(id, &format!("{err}")),
            };

            match db::tasks::update_dependency(conn, id_value, &add_child, &remove_child).await {
                Ok(()) => match db::tasks::get_task_by_id(conn, id_value).await {
                    Ok(task) => json_rpc_ok(id, json!({ "content": [text_content(&task)] })),
                    Err(err) => tool_error(
                        id,
                        &format!("dependencies updated but failed to load task: {err}"),
                    ),
                },
                Err(err) => tool_error(id, &format!("failed to update dependencies: {err}")),
            }
        }
        _ => json_rpc_error(id, -32601, "tool not found", None),
    }
}

fn parse_states(args: &Value) -> Result<Vec<TaskState>, String> {
    let Some(value) = args.get("state") else {
        return Ok(default_states());
    };

    let Some(items) = value.as_array() else {
        return Err("state must be an array of strings".to_string());
    };

    let mut states = Vec::with_capacity(items.len());
    for item in items {
        let Some(value) = item.as_str() else {
            return Err("state values must be strings".to_string());
        };
        states.push(parse_state(value)?);
    }
    Ok(states)
}

fn parse_state(value: &str) -> Result<TaskState, String> {
    match value {
        "ready" => Ok(TaskState::Ready),
        "blocked" => Ok(TaskState::Blocked),
        "in_progress" => Ok(TaskState::InProgress),
        "closed" => Ok(TaskState::Closed),
        _ => Err(format!("unknown state: {value}")),
    }
}

fn default_states() -> Vec<TaskState> {
    vec![TaskState::Ready, TaskState::Blocked, TaskState::InProgress]
}

fn values_to_ids(values: &Vec<Value>) -> Result<Vec<i64>, String> {
    let mut ids = Vec::with_capacity(values.len());
    for value in values {
        let Some(id) = value.as_i64() else {
            return Err("dependency ids must be integers".to_string());
        };
        ids.push(id);
    }
    Ok(ids)
}

fn text_content<T: Serialize>(value: &T) -> Value {
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    json!({
        "type": "text",
        "text": payload
    })
}

fn json_rpc_ok(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn json_rpc_error(id: Value, code: i64, message: &str, data: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data,
        }),
    }
}

fn tool_error(id: Value, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(json!({
            "content": [
                {
                    "type": "text",
                    "text": message
                }
            ],
            "isError": true
        })),
        error: None,
    }
}
