use std::process::Command;

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
        panic!(
            "add failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let add_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json add");
    assert_eq!(add_payload["title"], "First");
    assert_eq!(add_payload["desc"], "Test task");

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
        panic!(
            "list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let list_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json list");
    assert_eq!(list_payload.as_array().map(|arr| arr.len()), Some(1));
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
        panic!(
            "claim failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let claim_payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("json claim");
    assert_eq!(claim_payload["id"].as_i64(), Some(high_id));
    assert_eq!(claim_payload["state"], "in_progress");
}
