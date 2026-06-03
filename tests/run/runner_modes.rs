use super::*;

#[test]
fn run_with_ad_hoc_command_feeds_each_fragment_as_one_turn() {
    let store = TestStore::initialized("run-ad-hoc");
    let sink = TestStore::initialized("run-ad-hoc-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);
    let before_head = git_head(store.path());

    let output = pseq(&[
        "--store",
        path_str(store.path()),
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
    assert_eq!(git_head(store.path()), before_head);
    assert!(String::from_utf8_lossy(&output.stdout).contains("created capture:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("\npseq: running turn 1/2"));

    let texts = capture_texts(&sink);
    assert_eq!(texts.len(), 2);
    assert!(texts.contains(&"A\n".to_owned()));
    assert!(texts.contains(&"B\n".to_owned()));
    assert_git_clean(store.path());
    assert_git_clean(sink.path());
}

#[test]
fn run_uses_named_generic_runner_first_then_next_commands() {
    let store = TestStore::initialized("run-named");
    let first_sink = TestStore::initialized("run-first-sink");
    let next_sink = TestStore::initialized("run-next-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

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

    let output = pseq(&["--store", path_str(store.path()), "run", "Workflow"]);
    assert_success(&output);

    assert_eq!(capture_texts(&first_sink), vec!["A\n".to_owned()]);
    assert_eq!(capture_texts(&next_sink), vec!["B\n".to_owned()]);
    assert_git_clean(store.path());
}

#[test]
fn run_can_reset_named_generic_runner_first_next_per_iteration() {
    let store = TestStore::initialized("run-named-session-scope-iteration");
    let first_sink = TestStore::initialized("run-named-session-scope-iteration-first-sink");
    let next_sink = TestStore::initialized("run-named-session-scope-iteration-next-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

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
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--session-scope",
        "iteration",
    ]);
    assert_success(&output);

    assert_eq!(
        capture_texts(&first_sink),
        vec!["A\n".to_owned(), "A\n".to_owned()]
    );
    assert_eq!(
        capture_texts(&next_sink),
        vec!["B\n".to_owned(), "B\n".to_owned()]
    );
    assert_git_clean(store.path());
    assert_git_clean(first_sink.path());
    assert_git_clean(next_sink.path());
}

#[test]
fn run_uses_first_command_for_every_turn_when_named_generic_runner_has_no_next_command() {
    let store = TestStore::initialized("run-named-first-only");
    let sink = TestStore::initialized("run-named-first-only-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

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
        path_str(sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    let output = pseq(&["--store", path_str(store.path()), "run", "Workflow"]);
    assert_success(&output);

    assert_eq!(
        capture_texts(&sink),
        vec!["A\n".to_owned(), "B\n".to_owned()]
    );
    assert_git_clean(store.path());
    assert_git_clean(sink.path());
}

#[test]
fn run_refuses_changed_store_runner_until_trusted() {
    let store = TestStore::initialized("run-runner-trust");
    let sink = TestStore::initialized("run-runner-trust-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n")]);

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "--version",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    fs::write(
        store.path().join("config.toml"),
        format!(
            "version = 1\ndefault_runner = \"local\"\n\n[runners.local]\nfirst = [{:?}, \"capture\", \"import\", \"--stdin\", \"--store\", {:?}]\n",
            pseq_bin(),
            path_str(sink.path())
        ),
    )
    .unwrap();

    let rejected = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
    ]);
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(
        stderr_json(&rejected)["error"]["code"],
        "runner_not_trusted"
    );
    assert!(capture_texts(&sink).is_empty());

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "trust",
        "local",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
    ]));
    assert_eq!(capture_texts(&sink), vec!["A\n".to_owned()]);
}
