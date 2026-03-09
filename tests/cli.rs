use std::process::Command;

const AGENT_INSTRUCTIONS: &str = r#"## Work Tracking Instructions
### Overview
Pearls is a lightweight CLI for managing a task graph. Pearls tasks can be assigned parents, children, and priorities. Parent tasks block child tasks and must be completed and closed before child tasks are ready to be worked.
Database path defaults to ./pearls.db and can be overridden with PEARLS_DB.
Use --json on any command to emit machine-readable output.

Commands:
- pearls tasks list [--state ready,blocked,in_progress,closed]
- pearls tasks claim-next [--assignee <ASSIGNEE>]
- pearls tasks add --title <title> --description <desc> [--assignee <ASSIGNEE>] [--parent-of <id>] [--child-of <id>] [--priority <num>]
- pearls tasks update-metadata --id <id> [--title <title>] [--desc <desc>] [--priority <num>] [--state <state>] [--assignee <ASSIGNEE>] [--no-assignee]
- pearls tasks update-dependency --id <id> [--add-child <id> ...] [--remove-child <id> ...]

### Workflow
- claim the next ready task with `pearls tasks claim-next`
- when done, close the task with `pearls tasks update-metadata`
- if any new subtasks need to be created as a result of working your in progress task, create them with `pearls tasks add` and make sure to set their dependencies appropriately
"#;

#[test]
fn agent_instructions_outputs_expected_text() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    cmd.args(["agent", "instructions"]);
    let output = cmd.output().expect("run agent instructions");
    if !output.status.success() {
        panic!(
            "agent instructions failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    assert_eq!(String::from_utf8_lossy(&output.stdout), AGENT_INSTRUCTIONS);
}

#[test]
fn agent_instructions_supports_json_output() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    cmd.args(["--json", "agent", "instructions"]);
    let output = cmd.output().expect("run json agent instructions");
    if !output.status.success() {
        panic!(
            "json agent instructions failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json output");
    assert_eq!(payload["instructions"], AGENT_INSTRUCTIONS);
}

#[test]
fn json_output_for_add_and_list() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("pearls.db");

    let mut add = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    add.args([
        "--json",
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "add",
        "--title",
        "First",
        "--description",
        "Test task",
    ]);
    let output = add.output().expect("run add");
    if !output.status.success() {
        panic!("add failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let add_payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json add");
    assert_eq!(add_payload["title"], "First");
    assert_eq!(add_payload["desc"], "Test task");
    assert!(add_payload["assignee"].is_null());

    let mut list = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    list.args([
        "--json",
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "list",
        "--state",
        "ready",
    ]);
    let output = list.output().expect("run list");
    if !output.status.success() {
        panic!("list failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let list_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json list");
    assert_eq!(list_payload.as_array().map(|arr| arr.len()), Some(1));
    assert!(list_payload[0]["assignee"].is_null());
}

#[test]
fn human_list_shows_no_assignee_when_unassigned() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("pearls.db");

    let mut add = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    add.args([
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "add",
        "--title",
        "First",
        "--description",
        "Test task",
    ]);
    let output = add.output().expect("run add");
    if !output.status.success() {
        panic!("add failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let mut list = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    list.args([
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "list",
        "--state",
        "ready",
    ]);
    let output = list.output().expect("run list");
    if !output.status.success() {
        panic!("list failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("assignee=no assignee"));
}

#[test]
fn claim_next_selects_ready_task_and_updates_state() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("pearls.db");

    let mut add_low = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    add_low.args([
        "--json",
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "add",
        "--title",
        "Low",
        "--description",
        "Priority 2",
        "--priority",
        "2",
    ]);
    let output = add_low.output().expect("run add low");
    if !output.status.success() {
        panic!(
            "add low failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let mut add_high = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    add_high.args([
        "--json",
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "add",
        "--title",
        "High",
        "--description",
        "Priority 1",
        "--priority",
        "1",
    ]);
    let output = add_high.output().expect("run add high");
    if !output.status.success() {
        panic!(
            "add high failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let add_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json add high");
    let high_id = add_payload["id"].as_i64().expect("high id");

    let mut claim = Command::new(assert_cmd::cargo::cargo_bin!("pearls"));
    claim.args([
        "--json",
        "--db",
        db_path.to_str().expect("db path"),
        "tasks",
        "claim-next",
    ]);
    let output = claim.output().expect("run claim");
    if !output.status.success() {
        panic!("claim failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let claim_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json claim");
    assert_eq!(claim_payload["id"].as_i64(), Some(high_id));
    assert_eq!(claim_payload["state"], "in_progress");
}
