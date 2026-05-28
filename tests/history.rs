#[path = "common/mod.rs"]
mod common;

use std::fs;

use common::{
    TestStore, assert_git_clean, assert_stdout_contains, assert_success, git_status, path_str,
    pseq, pseq_with_stdin, stdout_json,
};

#[test]
fn log_exposes_git_backed_history_as_human_and_json() {
    let store = TestStore::initialized("history-log");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "History",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "body\n",
    ));

    let json_output = pseq(&["log", "--store", path_str(store.path()), "--json"]);
    assert_success(&json_output);
    let json = stdout_json(&json_output);
    let entries = json["entries"].as_array().unwrap();
    assert!(entries.len() >= 2);
    assert_eq!(entries[0]["summary"], "Add fragment History");
    assert!(entries[0]["commit"].as_str().unwrap().len() >= 40);
    assert!(entries[0]["short_commit"].is_string());
    assert!(entries[0]["timestamp"].is_string());
    assert!(
        entries
            .iter()
            .any(|entry| entry["summary"] == "Initialize pseq store")
    );

    let human_output = pseq(&["log", "--store", path_str(store.path())]);
    assert_success(&human_output);
    assert_stdout_contains(&human_output, "Add fragment History");
    assert_stdout_contains(&human_output, "Initialize pseq store");

    assert_git_clean(store.path());
}

#[test]
fn diff_exposes_tracked_and_untracked_store_differences_without_mutating() {
    let store = TestStore::initialized("history-diff");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Tracked",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "old body\n",
    ));

    let fragment_path = store.path().join("fragments/tracked.md");
    let fragment = fs::read_to_string(&fragment_path).unwrap();
    let updated_fragment = fragment.replace("old body\n", "new body\n");
    fs::write(&fragment_path, updated_fragment).unwrap();
    fs::write(
        store.path().join("captures/manual.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000001",
  "created_unix_seconds": 1,
  "origin": { "kind": "stdin" },
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "manual\n" }
  ]
}
"#,
    )
    .unwrap();

    let expected_status = git_status(store.path());
    assert!(expected_status.contains(" M fragments/tracked.md"));
    assert!(expected_status.contains("?? captures/manual.json"));

    let json_output = pseq(&["diff", "--store", path_str(store.path()), "--json"]);
    assert_success(&json_output);
    let json = stdout_json(&json_output);
    assert_eq!(json["dirty"], true);
    assert_eq!(json["changed_paths"], 2);
    assert_eq!(json["untracked_paths"], 1);
    assert!(
        json["patch"]
            .as_str()
            .unwrap()
            .contains("fragments/tracked.md")
    );
    assert!(json["patch"].as_str().unwrap().contains("-old body"));
    assert!(json["patch"].as_str().unwrap().contains("+new body"));
    assert!(
        json["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| { path["status"] == "??" && path["path"] == "captures/manual.json" })
    );

    let human_output = pseq(&["diff", "--store", path_str(store.path())]);
    assert_success(&human_output);
    assert_stdout_contains(&human_output, "fragments/tracked.md");
    assert_stdout_contains(&human_output, "-old body");
    assert_stdout_contains(&human_output, "+new body");
    assert_stdout_contains(&human_output, "?? captures/manual.json");

    assert_eq!(git_status(store.path()), expected_status);
}
