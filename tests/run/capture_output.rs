use super::*;

#[test]
fn run_json_captures_runner_output_without_mixing_stdout() {
    let store = TestStore::initialized("run-json");
    let sink = TestStore::initialized("run-json-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

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
    assert_success(&output);
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["sequence"]["name"], "Workflow");
    assert_eq!(json["runner"]["mode"], "ad-hoc");
    assert_eq!(json["turn_count"], 1);
    assert_eq!(json["completed_turns"], 1);
    assert_eq!(json["success"], true);
    assert_eq!(json["turns"][0]["fragment"]["name"], "Only");
    assert!(json["turns"][0]["pid"].as_u64().unwrap() > 0);
    assert_eq!(json["turns"][0]["termination"], "exit");
    assert!(
        json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("created capture:")
    );
    assert_eq!(capture_texts(&sink), vec!["body\n".to_owned()]);
}

#[test]
fn run_json_bounds_captured_runner_output_and_reports_truncation() {
    let store = TestStore::initialized("run-json-bounded-output");
    let missing = TestStore::new("run-json-bounded-output-missing");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--max-captured-output",
        "8",
        "--",
        pseq_bin(),
        "doctor",
        "--store",
        path_str(missing.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(turn["stdout"].as_str().unwrap().len(), 8);
    assert_eq!(turn["stdout_bytes"], 8);
    assert_eq!(turn["stdout_truncated"], true);
    assert_eq!(turn["stderr"], "");
    assert_eq!(turn["stderr_bytes"], 0);
    assert_eq!(turn["stderr_truncated"], false);
}
