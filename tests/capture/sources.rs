use super::*;

#[test]
fn capture_sources_and_probe_report_source_availability() {
    let codex_home = TestStore::new("codex-home-sources");
    let claude_home = TestStore::new("claude-home-sources");
    let openhands_home = TestStore::new("openhands-home-sources");
    let opencode_home = TestStore::new("opencode-home-sources");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T12-00-00",
        "source-one",
        &["hello"],
    );
    write_claude_code_session(claude_home.path(), "claude-source", &["hello claude"]);
    write_openhands_trajectory(
        openhands_home.path(),
        "openhands-source",
        &["hello openhands"],
    );
    write_opencode_messages(opencode_home.path(), "opencode-source", &["hello opencode"]);

    let sources = pseq_with_env(
        &["capture", "sources", "--json"],
        &[
            ("CODEX_HOME", path_str(codex_home.path())),
            ("CLAUDE_CONFIG_DIR", path_str(claude_home.path())),
            ("OPENHANDS_HOME", path_str(openhands_home.path())),
            ("OPENCODE_DATA_DIR", path_str(opencode_home.path())),
        ],
    );
    assert_success(&sources);
    let json = stdout_json(&sources);
    assert_default_capture_source_names(&json);
    for source in json["sources"].as_array().unwrap() {
        assert_eq!(source["available"], true);
    }

    let stdin_probe = pseq(&["capture", "probe", "--source", "stdin", "--json"]);
    assert_success(&stdin_probe);
    let stdin_json = stdout_json(&stdin_probe);
    assert_eq!(stdin_json["source"], "stdin");
    assert_eq!(stdin_json["available"], true);

    let codex_probe = pseq_with_env(
        &["capture", "probe", "--source", "codex", "--json"],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&codex_probe);
    assert!(codex_probe.stderr.is_empty());
    let codex_json = stdout_json(&codex_probe);
    assert_eq!(codex_json["source"], "codex");
    assert_eq!(codex_json["available"], true);
    assert_eq!(codex_json["session_count"], 1);
    assert_eq!(codex_json["sessions"][0]["id"], "source-one");
    assert_eq!(codex_json["sessions"][0]["prompt_count"], 1);

    let empty_codex_home = TestStore::new("codex-home-empty");
    let unavailable_codex_probe = pseq_with_env(
        &["capture", "probe", "--source", "codex", "--json"],
        &[("CODEX_HOME", path_str(empty_codex_home.path()))],
    );
    assert_eq!(unavailable_codex_probe.status.code(), Some(1));
    assert!(unavailable_codex_probe.stderr.is_empty());
    assert_eq!(stdout_json(&unavailable_codex_probe)["available"], false);

    let unsupported = pseq(&["capture", "probe", "--source", "unknown", "--json"]);
    assert_eq!(unsupported.status.code(), Some(1));
    assert!(unsupported.stdout.is_empty());
    assert_eq!(
        stderr_json(&unsupported)["error"]["code"],
        "capture_source_unsupported"
    );
}

#[test]
fn capture_discovery_commands_do_not_require_a_store_path() {
    let removed_env = [
        "HOME",
        "USERPROFILE",
        "PSEQ_STORE",
        "CODEX_HOME",
        "CLAUDE_CONFIG_DIR",
        "CLAUDE_HOME",
        "OPENHANDS_HOME",
        "OPENHANDS_DATA_DIR",
        "OPENCODE_DATA_DIR",
    ];

    let sources = pseq_with_env_removed(&["capture", "sources", "--json"], &removed_env);
    assert_success(&sources);
    assert!(sources.stderr.is_empty());
    let sources_json = stdout_json(&sources);
    assert_default_capture_source_names(&sources_json);

    let stdin_probe = pseq_with_env_removed(
        &["capture", "probe", "--source", "stdin", "--json"],
        &removed_env,
    );
    assert_success(&stdin_probe);
    assert!(stdin_probe.stderr.is_empty());
    let stdin_json = stdout_json(&stdin_probe);
    assert_eq!(stdin_json["source"], "stdin");
    assert_eq!(stdin_json["available"], true);

    let codex_probe = pseq_with_env_removed(
        &["capture", "probe", "--source", "codex", "--json"],
        &removed_env,
    );
    assert_eq!(codex_probe.status.code(), Some(1));
    assert!(codex_probe.stderr.is_empty());
    let codex_json = stdout_json(&codex_probe);
    assert_eq!(codex_json["source"], "codex");
    assert_eq!(codex_json["available"], false);
}

#[test]
fn capture_sources_exclude_current_pseq_capture_invocation() {
    let store = TestStore::initialized("capture-current-invocation");
    let codex_home = TestStore::new("codex-home-current-invocation");
    write_codex_session(
        codex_home.path(),
        "2026-05-23T12-30-00",
        "current-invocation",
        &[
            "first real prompt\n",
            "pseq capture last --source codex\n",
            "rtk ./target/debug/pseq capture last --source codex\n",
            "pseq --store /tmp/pseq-store capture last --source codex\n",
            "pseq --json --quiet capture range -1..-1 --source codex\n",
            "pseq -C /tmp/pseq-store cap last --source codex\n",
            "rtk ./target/debug/pseq --no-pager capture sources\n",
            "second real prompt\n",
        ],
    );

    let captured = pseq_with_env(
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
    assert_eq!(json["events"][0]["text"], "first real prompt\n");
    assert_eq!(json["events"][1]["text"], "second real prompt\n");
    assert_git_clean(store.path());
}

#[cfg(unix)]
#[test]
fn capture_source_discovery_skips_symlinked_directories() {
    use std::os::unix::fs::symlink;

    let codex_home = TestStore::new("codex-home-symlink");
    let external_home = TestStore::new("codex-external-symlink");
    write_codex_session(
        external_home.path(),
        "2026-05-23T13-00-00",
        "symlinked-session",
        &["should not be discovered"],
    );
    let sessions_dir = codex_home.path().join("sessions");
    fs::create_dir_all(&sessions_dir).unwrap();
    symlink(
        external_home.path().join("sessions"),
        sessions_dir.join("external"),
    )
    .unwrap();

    let probe = pseq_with_env(
        &["capture", "probe", "--source", "codex", "--json"],
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_eq!(probe.status.code(), Some(1));
    let json = stdout_json(&probe);
    assert_eq!(json["available"], false);
    assert_eq!(json["session_count"], 0);
}

#[test]
fn capture_last_from_supported_harness_sources_filters_to_user_prompts() {
    let store = TestStore::initialized("capture-supported-sources");
    let claude_home = TestStore::new("claude-home-select");
    let openhands_home = TestStore::new("openhands-home-select");
    let opencode_home = TestStore::new("opencode-home-select");
    write_claude_code_session(
        claude_home.path(),
        "claude-select",
        &["claude first\n", "claude second\n"],
    );
    write_openhands_trajectory(
        openhands_home.path(),
        "openhands-select",
        &["openhands first\n", "openhands second\n"],
    );
    write_opencode_messages(
        opencode_home.path(),
        "opencode-select",
        &["opencode first\n", "opencode second\n"],
    );

    assert_source_capture(
        store.path(),
        "claude-code",
        ("CLAUDE_CONFIG_DIR", path_str(claude_home.path())),
        &["claude first\n", "claude second\n"],
    );
    assert_source_capture(
        store.path(),
        "openhands",
        ("OPENHANDS_HOME", path_str(openhands_home.path())),
        &["openhands first\n", "openhands second\n"],
    );
    assert_source_capture(
        store.path(),
        "opencode",
        ("OPENCODE_DATA_DIR", path_str(opencode_home.path())),
        &["opencode first\n", "opencode second\n"],
    );
    assert_git_clean(store.path());
}

#[test]
fn capture_last_fails_before_mutation_when_source_is_unavailable() {
    let store = TestStore::initialized("capture-source-unavailable");
    let claude_home = TestStore::new("claude-home-empty");

    let captured = pseq_with_env(
        &[
            "capture",
            "last",
            "--source",
            "claude-code",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("CLAUDE_CONFIG_DIR", path_str(claude_home.path()))],
    );
    assert_eq!(captured.status.code(), Some(1));
    assert!(captured.stdout.is_empty());
    assert_eq!(
        stderr_json(&captured)["error"]["code"],
        "capture_source_unavailable"
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let status_json = stdout_json(&status);
    assert_eq!(status_json["counts"]["captures"], 0);
    assert_eq!(status_json["counts"]["fragments"], 0);
    assert_eq!(status_json["counts"]["sequences"], 0);
    assert_git_clean(store.path());
}
