use super::*;

#[test]
fn run_rejects_mixed_named_runner_and_ad_hoc_command_before_execution() {
    let store = TestStore::initialized("run-mixed-invocation");
    let sink = TestStore::initialized("run-mixed-invocation-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "codex",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(sink.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "invalid_run_invocation"
    );
    assert!(capture_texts(&sink).is_empty());
}

#[test]
fn run_stops_after_first_unsuccessful_runner_exit() {
    let store = TestStore::initialized("run-failure");
    let missing = TestStore::new("run-missing-store");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "doctor",
        "--store",
        path_str(missing.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["success"], false);
    assert!(
        json.get("failed_iteration").is_none(),
        "single-iteration failures should preserve the original JSON shape"
    );
    assert_eq!(json["failed_turn"], 1);
    assert_eq!(json["completed_turns"], 0);
    assert_eq!(json["turns"].as_array().unwrap().len(), 1);
    assert_eq!(json["turns"][0]["exit_code"], 1);
}

#[test]
fn run_failure_uses_pseq_failure_exit_code_and_reports_runner_exit_code() {
    let store = TestStore::initialized("run-failure-exit-code");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "not-a-command",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["success"], false);
    assert!(
        json.get("failed_iteration").is_none(),
        "single-iteration failures should preserve the original JSON shape"
    );
    assert_eq!(json["failed_turn"], 1);
    assert_eq!(json["turns"][0]["exit_code"], 2);
}

#[test]
fn run_fails_before_executing_when_turn_rendering_fails() {
    let store = TestStore::initialized("run-render-fail");
    let sink = TestStore::initialized("run-render-fail-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("Template", "{{missing}}\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(sink.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_json(&output)["error"]["code"], "missing_variable");
    assert!(capture_texts(&sink).is_empty());
}

#[test]
fn run_requires_resolvable_runner() {
    let store = TestStore::initialized("run-missing-runner");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let missing_default = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
    ]);
    assert_eq!(missing_default.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_default)["error"]["code"],
        "default_runner_missing"
    );

    let missing_named = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "codex",
    ]);
    assert_eq!(missing_named.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_named)["error"]["code"],
        "runner_not_found"
    );
}
