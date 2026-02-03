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
