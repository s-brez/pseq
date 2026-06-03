use super::super::*;

#[test]
#[ignore = "requires the real Codex CLI binary"]
fn run_retries_real_codex_cli_failure_and_preserves_stderr() {
    let store = TestStore::initialized("run-codex-retry-failure");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--retry-delay-ms",
        "0",
        "--",
        "codex",
        "exec",
        "--pseq-invalid-option",
        "-",
    ]);
    assert_eq!(output.status.code(), Some(1));

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(turn["attempt_count"], 3);
    assert_eq!(turn["attempts"].as_array().unwrap().len(), 3);
    assert!(
        turn["attempts"][0]["stderr"]
            .as_str()
            .is_some_and(|stderr| !stderr.trim().is_empty()),
        "real Codex parse failure should preserve stderr"
    );
}

#[test]
#[ignore = "requires Codex CLI auth and spends model tokens"]
fn run_feedback_loop_with_real_codex_cli() {
    let store = TestStore::initialized("run-feedback-codex");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[(
            "LoopPrompt",
            "Previous feedback:\n{{pseq_feedback}}\n\nRules:\n- If Previous feedback is empty, reply exactly: PSEQ-FIRST-EMPTY\n- If Previous feedback contains PSEQ-FIRST-EMPTY, reply exactly: PSEQ-SECOND-SAW-FIRST\n- Do not include anything else.\n",
        )],
    );

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
        "200000",
        "--",
        "codex",
        "exec",
        "-m",
        "gpt-5.4-mini",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "-",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["iterations"], 2);
    assert_eq!(json["turns"][0]["iteration"], 1);
    assert_eq!(json["turns"][1]["iteration"], 2);
    assert!(
        json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-FIRST-EMPTY")
    );
    assert!(
        json["turns"][1]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-SECOND-SAW-FIRST")
    );
}

#[test]
#[ignore = "requires Codex CLI auth and spends model tokens"]
fn run_feedback_seed_loop_with_real_codex_cli() {
    let store = TestStore::initialized("run-feedback-seed-codex");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[(
            "LoopPrompt",
            "Previous feedback:\n{{pseq_feedback}}\n\nRules:\n- If Previous feedback contains PSEQ-SEED-INITIAL, reply exactly: PSEQ-FIRST-SAW-SEED\n- If Previous feedback contains PSEQ-FIRST-SAW-SEED, reply exactly: PSEQ-SECOND-SAW-FIRST\n- If Previous feedback is empty, reply exactly: PSEQ-UNEXPECTED-EMPTY\n- Do not include anything else.\n",
        )],
    );

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
        "--feedback-seed",
        "PSEQ-SEED-INITIAL",
        "--max-captured-output",
        "200000",
        "--",
        "codex",
        "exec",
        "-m",
        "gpt-5.4-mini",
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "-",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["iterations"], 2);
    assert_eq!(json["turns"][0]["iteration"], 1);
    assert_eq!(json["turns"][1]["iteration"], 2);
    assert!(
        json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-FIRST-SAW-SEED")
    );
    assert!(
        !json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-UNEXPECTED-EMPTY")
    );
    assert!(
        json["turns"][1]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-SECOND-SAW-FIRST")
    );
}
