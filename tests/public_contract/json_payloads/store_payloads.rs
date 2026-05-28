use super::super::*;

pub(super) fn assert_store_and_discovery_payloads(store: &TestStore) {
    let init = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_success(&init);
    let init_json = stdout_json(&init);
    assert_object_keys(
        &init_json,
        &["store", "created", "already_initialized", "git_commit"],
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let status_json = stdout_json(&status);
    assert_object_keys(&status_json, &["store", "valid", "issues", "counts", "git"]);
    assert_object_keys(
        &status_json["counts"],
        &["fragments", "sequences", "captures", "renders"],
    );
    assert_object_keys(
        &status_json["git"],
        &[
            "repository",
            "branch",
            "head",
            "dirty",
            "changed_paths",
            "untracked_paths",
        ],
    );

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_success(&doctor);
    assert_object_keys(&stdout_json(&doctor), &["store", "valid", "issues"]);

    let config = pseq(&[
        "config",
        "show",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&config);
    assert_object_keys(&stdout_json(&config), &["path", "runner_count", "version"]);

    let sources = pseq(&[
        "capture",
        "sources",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sources);
    let sources_json = stdout_json(&sources);
    assert_object_keys(&sources_json, &["sources"]);
    let sources = sources_json["sources"]
        .as_array()
        .expect("sources should be an array");
    let source_names = sources
        .iter()
        .map(|source| source["name"].as_str().expect("source should have a name"))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        source_names,
        BTreeSet::from(["claude-code", "codex", "openhands", "opencode", "stdin"])
    );
    let stdin_source = sources
        .iter()
        .find(|source| source["name"] == "stdin")
        .expect("stdin source should be listed");
    assert_object_keys(
        stdin_source,
        &["name", "available", "description", "session_count"],
    );

    let probe = pseq(&[
        "capture",
        "probe",
        "--source",
        "stdin",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&probe);
    assert_object_keys(
        &stdout_json(&probe),
        &["source", "available", "message", "session_count"],
    );
}

pub(super) fn assert_history_payloads(store: &TestStore) {
    let log = pseq(&["log", "--store", path_str(store.path()), "--json"]);
    assert_success(&log);
    let log_json = stdout_json(&log);
    assert_object_keys(&log_json, &["entries"]);
    assert_object_keys(
        first_array_item(&log_json, "entries"),
        &[
            "commit",
            "short_commit",
            "author_name",
            "author_email",
            "timestamp",
            "summary",
        ],
    );

    let diff = pseq(&["diff", "--store", path_str(store.path()), "--json"]);
    assert_success(&diff);
    assert_object_keys(
        &stdout_json(&diff),
        &[
            "dirty",
            "changed_paths",
            "untracked_paths",
            "paths",
            "patch",
        ],
    );
}
