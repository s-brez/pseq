use super::super::*;

#[test]
fn run_iterations_feed_final_stdout_into_configured_feedback_variable() {
    let store = TestStore::initialized("run-feedback-loop");
    let first_sink = TestStore::initialized("run-feedback-loop-first-sink");
    let next_sink = TestStore::initialized("run-feedback-loop-next-sink");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            ("First", "Previous update:\n{{loop_feedback}}\n"),
            ("Final", "emit update\n"),
        ],
    );
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(first_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "next",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(next_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

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
        "--feedback-var",
        "loop_feedback",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["iterations"], 2);
    assert_eq!(json["turn_count"], 4);
    assert_eq!(json["completed_turns"], 4);
    assert_eq!(json["turns"][0]["iteration"], 1);
    assert_eq!(json["turns"][2]["iteration"], 2);

    assert_eq!(
        capture_texts(&first_sink),
        vec!["Previous update:\n\n".to_owned()]
    );
    let texts = capture_texts(&next_sink);
    assert_eq!(texts.len(), 3);
    assert!(
        texts
            .iter()
            .any(|text| text.starts_with("Previous update:\ncreated capture: cap_")),
        "second iteration first prompt should include prior final-turn stdout; got {texts:#?}"
    );
    assert_git_clean(store.path());
    assert_git_clean(first_sink.path());
    assert_git_clean(next_sink.path());
}

#[test]
fn run_feedback_seed_initializes_first_iteration_then_loop_feedback_replaces_it() {
    let store = TestStore::initialized("run-feedback-seed-literal");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            ("First", "feedback={{pseq_feedback}}\n"),
            ("Final", "final\n"),
        ],
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
        "seed",
        "--",
        "sh",
        "-c",
        "cat",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turns"][0]["stdout"], "feedback=seed\n");
    assert_eq!(json["turns"][1]["stdout"], "final\n");
    assert_eq!(json["turns"][2]["stdout"], "feedback=final\n\n");
    assert_git_clean(store.path());
}

#[test]
fn run_feedback_seed_uses_configured_feedback_variable() {
    let store = TestStore::initialized("run-feedback-seed-custom-var");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            ("First", "feedback={{loop_feedback}}\n"),
            ("Final", "final\n"),
        ],
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
        "--feedback-var",
        "loop_feedback",
        "--feedback-seed",
        "seed",
        "--",
        "sh",
        "-c",
        "cat",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turns"][0]["stdout"], "feedback=seed\n");
    assert_eq!(json["turns"][2]["stdout"], "feedback=final\n\n");
    assert_git_clean(store.path());
}

#[test]
fn run_feedback_seed_can_read_from_stdin() {
    let store = TestStore::initialized("run-feedback-seed-stdin");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("Only", "feedback={{pseq_feedback}}\n")],
    );

    let output = pseq_with_stdin(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--feedback-from",
            "final-stdout",
            "--feedback-seed",
            "@-",
            "--",
            "sh",
            "-c",
            "cat",
        ],
        "stdin seed\n",
    );
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turns"][0]["stdout"], "feedback=stdin seed\n\n");
    assert_git_clean(store.path());
}

#[test]
fn run_feedback_seed_can_read_from_file() {
    let store = TestStore::initialized("run-feedback-seed-file");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("Only", "feedback={{pseq_feedback}}\n")],
    );
    let seed_path = store.path().with_extension("seed.txt");
    std::fs::write(&seed_path, "file seed\n").unwrap();

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--feedback-from",
        "final-stdout",
        "--feedback-seed",
        &format!("@{}", path_str(&seed_path)),
        "--",
        "sh",
        "-c",
        "cat",
    ]);
    let _ = std::fs::remove_file(&seed_path);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turns"][0]["stdout"], "feedback=file seed\n\n");
    assert_git_clean(store.path());
}

#[test]
fn run_feedback_human_mode_tees_output_while_retaining_feedback() {
    let store = TestStore::initialized("run-feedback-human-tee");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            ("First", "feedback={{pseq_feedback}}\n"),
            ("Final", "final prompt\n"),
        ],
    );

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--",
        "sh",
        "-c",
        r#"input=$(cat); case "$input" in *"feedback=UPDATE"*) printf 'SAW-FEEDBACK\n' ;; *"final prompt"*) printf 'UPDATE\n' ;; *) printf 'EMPTY\n' ;; esac"#,
    ]);
    assert_success(&output);

    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "EMPTY\nUPDATE\nSAW-FEEDBACK\nUPDATE\n"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("pseq: running iteration 2/2 turn 1/2")
    );
    assert_git_clean(store.path());
}

#[test]
fn run_iterations_without_feedback_render_once_before_execution() {
    let store = TestStore::initialized("run-iterations-render-once");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    let sequence = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "sequence",
        "show",
        "Workflow",
    ]);
    assert_success(&sequence);
    let sequence_path = store.path().join(
        stdout_json(&sequence)["path"]
            .as_str()
            .expect("sequence path should be a string"),
    );

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--",
        "sh",
        "-c",
        "cat >/dev/null; rm -f \"$1\"",
        "sh",
        path_str(&sequence_path),
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turn_count"], 2);
    assert_eq!(json["completed_turns"], 2);
    assert_eq!(json["success"], true);
}

#[test]
fn run_feedback_iterations_use_initial_sequence_snapshot() {
    let store = TestStore::initialized("run-feedback-snapshot");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "{{pseq_feedback}}\n")]);
    let sequence = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "sequence",
        "show",
        "Workflow",
    ]);
    assert_success(&sequence);
    let sequence_path = store.path().join(
        stdout_json(&sequence)["path"]
            .as_str()
            .expect("sequence path should be a string"),
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
        "--",
        "sh",
        "-c",
        "cat >/dev/null; rm -f \"$1\"; printf 'feedback\\n'",
        "sh",
        path_str(&sequence_path),
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["turn_count"], 2);
    assert_eq!(json["completed_turns"], 2);
    assert_eq!(json["success"], true);
}
