#[path = "common/mod.rs"]
mod common;

use std::fs;

use common::{
    TestStore, assert_git_clean, assert_stdout_contains, assert_success, issue_codes, path_str,
    pseq, pseq_bin, stderr_json, stdout_json,
};

#[test]
fn runner_commands_manage_named_argv_runners_in_config() {
    let store = TestStore::initialized("runner-registry");

    let set_first = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "exec",
        "-",
    ]);
    assert_success(&set_first);
    assert_stdout_contains(&set_first, "configured runner local first command");

    let set_next = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "next",
        "--",
        pseq_bin(),
        "exec",
        "follow-up",
        "-",
    ]);
    assert_success(&set_next);

    let default = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]);
    assert_success(&default);

    let config = fs::read_to_string(store.path().join("config.toml")).unwrap();
    assert!(config.contains("default_runner = \"local\""));
    assert!(config.contains("[runners.local]"));
    assert!(config.contains("first = ["));
    assert!(config.contains("next = ["));

    let list = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "list",
        "--json",
    ]);
    assert_success(&list);
    let list_json = stdout_json(&list);
    assert_eq!(list_json["default_runner"], "local");
    assert_eq!(list_json["runners"][0]["name"], "local");
    assert_eq!(list_json["runners"][0]["has_next"], true);
    assert_eq!(list_json["runners"][0]["is_default"], true);

    let show = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "show",
        "local",
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["name"], "local");
    assert_eq!(show_json["is_default"], true);
    assert_eq!(show_json["first"][0], pseq_bin());
    assert_eq!(show_json["next"][1], "exec");

    let config_show = pseq(&[
        "--store",
        path_str(store.path()),
        "config",
        "show",
        "--json",
    ]);
    assert_success(&config_show);
    let config_json = stdout_json(&config_show);
    assert_eq!(config_json["default_runner"], "local");
    assert_eq!(config_json["runner_count"], 1);

    assert_git_clean(store.path());
}

#[test]
fn runner_remove_clears_default_runner() {
    let store = TestStore::initialized("runner-remove");
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "codex",
        "first",
        "--",
        pseq_bin(),
        "exec",
        "-",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "codex",
    ]));

    let removed = pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "rm",
        "codex",
        "--json",
    ]);
    assert_success(&removed);
    let json = stdout_json(&removed);
    assert_eq!(json["name"], "codex");
    assert_eq!(json["was_default"], true);

    let config = fs::read_to_string(store.path().join("config.toml")).unwrap();
    assert!(!config.contains("default_runner"));
    assert!(!config.contains("[runners.codex]"));
    assert_git_clean(store.path());
}

#[test]
fn runner_commands_validate_names_commands_and_missing_references() {
    let store = TestStore::initialized("runner-validation");

    let invalid_name = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "runner",
        "set",
        "bad.name",
        "first",
        "--",
        pseq_bin(),
    ]);
    assert_eq!(invalid_name.status.code(), Some(1));
    assert_eq!(
        stderr_json(&invalid_name)["error"]["code"],
        "invalid_runner_name"
    );

    let missing_next = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "runner",
        "set",
        "codex",
        "next",
        "--",
        pseq_bin(),
    ]);
    assert_eq!(missing_next.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_next)["error"]["code"],
        "runner_not_found"
    );

    let missing_default = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "runner",
        "default",
        "codex",
    ]);
    assert_eq!(missing_default.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_default)["error"]["code"],
        "runner_not_found"
    );

    assert_git_clean(store.path());
}

#[test]
fn runner_config_rejects_names_that_start_with_hyphen() {
    let store = TestStore::initialized("runner-leading-hyphen");
    fs::write(
        store.path().join("config.toml"),
        "version = 1\n\n[runners.\"-bad\"]\nfirst = [\"true\"]\n",
    )
    .unwrap();

    let doctor = pseq(&["--store", path_str(store.path()), "--json", "doctor"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(issue_codes(&stdout_json(&doctor)).contains(&"config_file_invalid"));
}

#[test]
fn runner_config_rejects_missing_default_runner_target() {
    let store = TestStore::initialized("runner-missing-default-target");
    fs::write(
        store.path().join("config.toml"),
        "version = 1\ndefault_runner = \"codex\"\n",
    )
    .unwrap();

    let doctor = pseq(&["--store", path_str(store.path()), "--json", "doctor"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(issue_codes(&stdout_json(&doctor)).contains(&"config_file_invalid"));
}
