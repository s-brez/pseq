use super::super::*;

#[test]
fn run_feedback_options_fail_closed() {
    let store = TestStore::initialized("run-feedback-validation");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "{{pseq_feedback}}\n")]);

    let zero_iterations = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "0",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(store.path()),
    ]);
    assert_eq!(zero_iterations.status.code(), Some(1));
    assert_eq!(
        stderr_json(&zero_iterations)["error"]["code"],
        "invalid_run_invocation"
    );

    let feedback_var_without_source = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--feedback-var",
        "pseq_feedback",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(store.path()),
    ]);
    assert_eq!(feedback_var_without_source.status.code(), Some(1));
    assert_eq!(
        stderr_json(&feedback_var_without_source)["error"]["code"],
        "invalid_run_invocation"
    );

    let feedback_seed_without_source = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--feedback-seed",
        "seed",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(store.path()),
    ]);
    assert_eq!(feedback_seed_without_source.status.code(), Some(1));
    assert_eq!(
        stderr_json(&feedback_seed_without_source)["error"]["code"],
        "invalid_run_invocation"
    );

    let feedback_var_conflict = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--var",
        "pseq_feedback=manual",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(store.path()),
    ]);
    assert_eq!(feedback_var_conflict.status.code(), Some(1));
    assert_eq!(
        stderr_json(&feedback_var_conflict)["error"]["code"],
        "invalid_run_invocation"
    );

    let vars_path = store.path().with_extension("vars.toml");
    std::fs::write(&vars_path, "pseq_feedback = \"manual\"\n").unwrap();
    let feedback_vars_conflict = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--vars",
        path_str(&vars_path),
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(store.path()),
    ]);
    let _ = std::fs::remove_file(&vars_path);
    assert_eq!(feedback_vars_conflict.status.code(), Some(1));
    assert_eq!(
        stderr_json(&feedback_vars_conflict)["error"]["code"],
        "invalid_run_invocation"
    );
}

#[test]
fn run_feedback_truncation_fails_before_next_iteration() {
    let store = TestStore::initialized("run-feedback-truncated");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "{{pseq_feedback}}\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--max-captured-output",
        "4",
        "--",
        "sh",
        "-c",
        "cat >/dev/null; printf abcdefghij",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("exceeded --max-captured-output")
    );
}

#[test]
fn run_feedback_final_turn_failure_takes_precedence_over_truncation() {
    let store = TestStore::initialized("run-feedback-failure-precedence");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "{{pseq_feedback}}\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--max-captured-output",
        "4",
        "--",
        "sh",
        "-c",
        "cat >/dev/null; printf abcdefghij; exit 7",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["success"], false);
    assert_eq!(json["failed_iteration"], 1);
    assert_eq!(json["failed_turn"], 1);
    assert_eq!(json["turns"][0]["exit_code"], 7);
    assert_eq!(json["turns"][0]["stdout_truncated"], true);
}
