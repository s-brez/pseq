use super::*;

#[test]
fn capture_last_and_range_from_codex_select_user_prompts_in_chronological_order() {
    let store = TestStore::initialized("capture-codex-select");
    let codex_home = TestStore::new("codex-home-select");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T12-00-00",
        "select-one",
        &["first\n", "second\n", "third\n"],
    );

    let last = pseq_with_env(
        &[
            "capture",
            "last",
            "2",
            "--source",
            "codex",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&last);
    let last_json = stdout_json(&last);
    let capture_id = last_json["id"].as_str().unwrap();
    assert_eq!(last_json["origin"]["kind"], "source");
    assert_eq!(last_json["origin"]["source"], "codex");
    assert_eq!(last_json["event_count"], 2);

    let show = pseq(&[
        "capture",
        "show",
        capture_id,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["events"][0]["index"], 1);
    assert_eq!(show_json["events"][0]["text"], "second\n");
    assert_eq!(show_json["events"][1]["index"], 2);
    assert_eq!(show_json["events"][1]["text"], "third\n");

    let range = pseq_with_env(
        &[
            "capture",
            "range",
            "-2..-1",
            "--source",
            "codex",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&range);
    let range_json = stdout_json(&range);
    assert_eq!(range_json["event_count"], 2);

    let range_show = pseq(&[
        "capture",
        "show",
        range_json["id"].as_str().unwrap(),
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&range_show);
    let range_show_json = stdout_json(&range_show);
    assert_eq!(range_show_json["events"][0]["text"], "second\n");
    assert_eq!(range_show_json["events"][1]["text"], "third\n");
    assert_git_clean(store.path());
}

#[test]
fn capture_last_can_select_a_source_session_candidate() {
    let store = TestStore::initialized("capture-codex-session-select");
    let codex_home = TestStore::new("codex-home-session-select");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T12-00-00",
        "older-session",
        &["older prompt\n"],
    );
    write_codex_session(
        codex_home.path(),
        "2026-05-23T13-00-00",
        "newer-session",
        &["newer prompt\n"],
    );

    let captured = pseq_with_env(
        &[
            "capture",
            "last",
            "--source",
            "codex",
            "--session",
            "older-session",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&captured);
    let capture_id = stdout_json(&captured)["id"].as_str().unwrap().to_owned();

    let show = pseq(&[
        "capture",
        "show",
        &capture_id,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let json = stdout_json(&show);
    assert_eq!(json["origin"]["session"]["id"], "older-session");
    assert_eq!(json["events"][0]["text"], "older prompt\n");
    assert_git_clean(store.path());
}

#[test]
fn capture_range_from_codex_can_create_sequence_in_one_command() {
    let store = TestStore::initialized("capture-codex-sequence");
    let codex_home = TestStore::new("codex-home-sequence");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T13-00-00",
        "sequence-one",
        &["alpha\n", "beta\n", "gamma\n"],
    );

    let promoted = pseq_with_env(
        &[
            "capture",
            "range",
            "1..2",
            "--source",
            "codex",
            "--as-sequence",
            "Codex Pair",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&promoted);
    let promoted_json = stdout_json(&promoted);
    assert_eq!(promoted_json["capture"]["origin"]["kind"], "source");
    assert_eq!(promoted_json["capture"]["origin"]["source"], "codex");
    assert_eq!(promoted_json["event_count"], 2);
    assert_eq!(promoted_json["sequence"]["name"], "Codex Pair");
    assert_eq!(promoted_json["sequence"]["fragment_count"], 2);
    assert!(promoted_json["git_commit"].is_string());

    let rendered = pseq(&["render", "Codex Pair", "--store", path_str(store.path())]);
    assert_success(&rendered);
    assert_eq!(String::from_utf8(rendered.stdout).unwrap(), "alpha\nbeta\n");
    assert_git_clean(store.path());
}

#[test]
fn capture_as_sequence_rejects_blank_sequence_name_before_writing_capture() {
    let store = TestStore::initialized("capture-invalid-sequence-name");
    let codex_home = TestStore::new("codex-home-invalid-sequence-name");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T13-30-00",
        "invalid-sequence-name",
        &["first\n", "second\n"],
    );

    let captured = pseq_with_env(
        &[
            "capture",
            "last",
            "2",
            "--source",
            "codex",
            "--as-sequence",
            "   ",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_eq!(captured.status.code(), Some(1));
    assert!(captured.stdout.is_empty());
    assert_eq!(
        stderr_json(&captured)["error"]["code"],
        "invalid_sequence_name"
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let status_json = stdout_json(&status);
    assert_eq!(status_json["counts"]["captures"], 0);
    assert_eq!(status_json["counts"]["fragments"], 0);
    assert_eq!(status_json["counts"]["sequences"], 0);
    assert_git_clean(store.path());
}
