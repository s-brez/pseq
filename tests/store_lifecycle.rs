#[path = "common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;

use common::{
    TestStore, assert_git_clean, assert_stdout_contains, assert_success, git, git_head, git_status,
    issue_codes, path_str, pseq, pseq_with_env_changes, stderr_json, stdout_json,
};

#[test]
fn init_creates_valid_git_backed_store() {
    let store = TestStore::new("init");

    let output = pseq(&["init", "--store", path_str(store.path())]);
    assert_success(&output);
    assert_stdout_contains(&output, "initialized store:");
    assert!(store.path().join("fragments").is_dir());
    assert!(store.path().join("sequences").is_dir());
    assert!(store.path().join("captures").is_dir());
    assert!(store.path().join("renders").is_dir());
    assert!(store.path().join("config.toml").is_file());
    assert!(store.path().join(".git").exists());

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_success(&doctor);
    let doctor_json = stdout_json(&doctor);
    assert_eq!(doctor_json["valid"], true);
    assert_eq!(doctor_json["issues"].as_array().unwrap().len(), 0);

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let status_json = stdout_json(&status);
    assert_eq!(status_json["valid"], true);
    assert_eq!(status_json["counts"]["fragments"], 0);
    assert_eq!(status_json["counts"]["sequences"], 0);
    assert_eq!(status_json["counts"]["captures"], 0);
    assert_eq!(status_json["counts"]["renders"], 0);
    assert_eq!(status_json["git"]["repository"], true);
    assert_eq!(status_json["git"]["dirty"], false);
    assert_eq!(status_json["git"]["changed_paths"], 0);

    assert_git_clean(store.path());
}

#[test]
fn init_creates_nested_store_as_its_own_git_repository() {
    let parent = TestStore::new("parent-repo");
    fs::create_dir_all(parent.path()).unwrap();
    assert_success(&git(parent.path(), &["init", "--quiet"]));

    let store_path = parent.path().join("nested-store");
    let output = pseq(&["init", "--store", path_str(&store_path), "--json"]);
    assert_success(&output);
    assert!(store_path.join(".git").exists());

    let doctor = pseq(&["doctor", "--store", path_str(&store_path), "--json"]);
    assert_success(&doctor);
    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], true);

    let root = git(&store_path, &["rev-parse", "--show-toplevel"]);
    assert_success(&root);
    let actual_root = String::from_utf8(root.stdout).unwrap();
    assert_eq!(
        actual_root.trim().replace('\\', "/"),
        path_str(&store_path).replace('\\', "/")
    );
}

#[test]
fn init_adopts_existing_git_repo_root_without_committing_unrelated_state() {
    let store = TestStore::new("existing-repo");
    fs::create_dir_all(store.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));

    fs::write(store.path().join("tracked.txt"), "original\n").unwrap();
    assert_success(&git(store.path(), &["add", "tracked.txt"]));
    assert_success(&git(
        store.path(),
        &[
            "-c",
            "user.name=pseq-test",
            "-c",
            "user.email=pseq-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "Initial project commit",
        ],
    ));

    fs::write(store.path().join("tracked.txt"), "modified\n").unwrap();
    fs::write(store.path().join("staged.txt"), "staged\n").unwrap();
    assert_success(&git(store.path(), &["add", "staged.txt"]));
    fs::write(store.path().join("manual-note.txt"), "manual\n").unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["created"], true);
    assert_eq!(json["already_initialized"], false);
    assert!(json["git_commit"].as_str().is_some());

    assert!(store.path().join("fragments").is_dir());
    assert!(store.path().join("sequences").is_dir());
    assert!(store.path().join("captures").is_dir());
    assert!(store.path().join("renders").is_dir());
    assert!(store.path().join("config.toml").is_file());

    let status = git_status(store.path());
    assert!(status.contains(" M tracked.txt"), "{status}");
    assert!(status.contains("A  staged.txt"), "{status}");
    assert!(status.contains("?? manual-note.txt"), "{status}");
    assert!(!status.contains("config.toml"), "{status}");
    assert!(!status.contains("fragments/.gitkeep"), "{status}");
    assert!(!status.contains("sequences/.gitkeep"), "{status}");
    assert!(!status.contains("captures/.gitkeep"), "{status}");
    assert!(!status.contains("renders/.gitkeep"), "{status}");

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_success(&doctor);
}

#[test]
fn init_adopts_existing_git_worktree_root_with_git_file_metadata() {
    let source = TestStore::new("linked-worktree-source");
    let store = TestStore::new("linked-worktree-store");
    fs::create_dir_all(source.path()).unwrap();
    assert_success(&git(source.path(), &["init", "--quiet"]));
    fs::write(source.path().join("tracked.txt"), "source\n").unwrap();
    common::git_commit_all(source.path(), "Initial source commit");

    assert_success(&git(
        source.path(),
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "pseq-linked-worktree",
            path_str(store.path()),
            "HEAD",
        ],
    ));
    assert!(store.path().join(".git").is_file());

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["created"], true);
    assert!(json["git_commit"].as_str().is_some());

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_success(&doctor);
    assert!(store.path().join(".git").is_file());
    assert_git_clean(store.path());
}

#[test]
fn init_refuses_non_empty_subdir_of_parent_git_repo() {
    let parent = TestStore::new("parent-with-subdir");
    fs::create_dir_all(parent.path()).unwrap();
    assert_success(&git(parent.path(), &["init", "--quiet"]));

    let store_path = parent.path().join("not-root");
    fs::create_dir_all(&store_path).unwrap();
    fs::write(store_path.join("unrelated.txt"), "manual\n").unwrap();

    let output = pseq(&["init", "--store", path_str(&store_path), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_not_empty");
    assert!(!store_path.join("config.toml").exists());
}

#[test]
fn init_refuses_existing_git_repo_root_with_incompatible_pseq_paths() {
    let store = TestStore::new("existing-conflict");
    fs::create_dir_all(store.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    fs::write(store.path().join("fragments"), "not a directory\n").unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_conflict");
    assert!(!store.path().join("config.toml").exists());
}

#[test]
fn init_refuses_scaffold_path_conflict_without_scaffold_mutation() {
    let store = TestStore::new("existing-gitkeep-conflict");
    fs::create_dir_all(store.path().join("renders/.gitkeep")).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_conflict");
    assert!(!store.path().join("config.toml").exists());
    assert!(!store.path().join("fragments").exists());
    assert!(!store.path().join("sequences").exists());
    assert!(!store.path().join("captures").exists());
    assert!(store.path().join("renders/.gitkeep").is_dir());
}

#[cfg(unix)]
#[test]
fn init_refuses_symlinked_scaffold_path_without_external_mutation() {
    use std::os::unix::fs::symlink;

    let store = TestStore::new("existing-symlinked-scaffold");
    let external = TestStore::new("external-scaffold-target");
    fs::create_dir_all(store.path()).unwrap();
    fs::create_dir_all(external.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    fs::write(external.path().join("bad.md"), "not frontmatter\n").unwrap();
    symlink(external.path(), store.path().join("fragments")).unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_conflict");
    assert!(!external.path().join(".gitkeep").exists());
    assert!(!store.path().join("config.toml").exists());
    assert!(!store.path().join("sequences").exists());
    assert!(!store.path().join("captures").exists());
    assert!(!store.path().join("renders").exists());

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    let doctor_json = stdout_json(&doctor);
    let codes = issue_codes(&doctor_json);
    assert!(codes.contains(&"required_path_symlink"));
    assert!(!codes.contains(&"fragment_file_invalid"));
}

#[cfg(unix)]
#[test]
fn init_refuses_symlinked_config_path_without_external_mutation() {
    use std::os::unix::fs::symlink;

    let store = TestStore::new("existing-symlinked-config");
    let external = TestStore::new("external-config-target");
    fs::create_dir_all(store.path()).unwrap();
    fs::create_dir_all(external.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    let external_config = external.path().join("config.toml");
    fs::write(&external_config, "version = 1\n").unwrap();
    symlink(&external_config, store.path().join("config.toml")).unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_conflict");
    assert_eq!(
        fs::read_to_string(&external_config).unwrap(),
        "version = 1\n"
    );
    assert!(!store.path().join("fragments").exists());
    assert!(!store.path().join("sequences").exists());
    assert!(!store.path().join("captures").exists());
    assert!(!store.path().join("renders").exists());
}

#[test]
fn init_refuses_existing_git_repo_root_with_incompatible_config() {
    let store = TestStore::new("existing-bad-config");
    fs::create_dir_all(store.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    fs::write(
        store.path().join("config.toml"),
        "store = '/tmp/elsewhere'\n",
    )
    .unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "invalid_config");
    assert!(!store.path().join("fragments").exists());
}

#[test]
fn init_refuses_existing_git_repo_root_with_invalid_pseq_content_without_scaffold_mutation() {
    let store = TestStore::new("existing-invalid-content");
    fs::create_dir_all(store.path().join("fragments")).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    fs::write(store.path().join("fragments/bad.md"), "not frontmatter\n").unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_conflict");
    assert!(store.path().join("fragments/bad.md").is_file());
    assert!(!store.path().join("fragments/.gitkeep").exists());
    assert!(!store.path().join("captures").exists());
    assert!(!store.path().join("sequences").exists());
    assert!(!store.path().join("renders").exists());
    assert!(!store.path().join("config.toml").exists());

    let status = git_status(store.path());
    assert!(status.contains("?? fragments/bad.md"), "{status}");
    assert!(!status.contains("config.toml"), "{status}");
    assert!(!status.contains(".gitkeep"), "{status}");
}

#[test]
fn init_rolls_back_created_scaffold_when_versioning_fails() {
    let store = TestStore::new("existing-ignored-scaffold");
    fs::create_dir_all(store.path()).unwrap();
    assert_success(&git(store.path(), &["init", "--quiet"]));
    fs::write(
        store.path().join(".gitignore"),
        "config.toml\n**/.gitkeep\n",
    )
    .unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "git_failed");
    assert!(!store.path().join("config.toml").exists());
    assert!(!store.path().join("fragments").exists());
    assert!(!store.path().join("sequences").exists());
    assert!(!store.path().join("captures").exists());
    assert!(!store.path().join("renders").exists());

    let status = git_status(store.path());
    assert!(status.contains("?? .gitignore"), "{status}");
    assert!(!status.contains("config.toml"), "{status}");
    assert!(!status.contains(".gitkeep"), "{status}");
}

#[cfg(unix)]
#[test]
fn init_ignores_template_pre_commit_hook() {
    use std::os::unix::fs::PermissionsExt;

    let root = TestStore::new("template-hook-root");
    let template = TestStore::new("template-hook");
    let marker = root.path().join("hook-ran");
    let hook = template.path().join("hooks/pre-commit");
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    fs::write(
        &hook,
        format!("#!/bin/sh\nprintf ran > '{}'\nexit 1\n", path_str(&marker)),
    )
    .unwrap();
    let mut permissions = fs::metadata(&hook).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&hook, permissions).unwrap();

    let store_path = root.path().join("nested/child/store");
    let output = pseq_with_env_changes(
        &["init", "--store", path_str(&store_path), "--json"],
        &[("GIT_TEMPLATE_DIR", path_str(template.path()))],
        &[],
    );
    assert_success(&output);
    assert!(!marker.exists());
    assert!(store_path.join("config.toml").is_file());
    assert_git_clean(&store_path);
}

#[test]
fn doctor_reports_missing_store_as_json_with_nonzero_exit() {
    let store = TestStore::new("missing");

    let output = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());

    let json = stdout_json(&output);
    assert_eq!(json["valid"], false);
    assert_eq!(json["issues"][0]["code"], "store_missing");
}

#[test]
fn init_refuses_non_empty_non_store_with_json_error_envelope() {
    let store = TestStore::new("non-empty");
    fs::create_dir_all(store.path()).unwrap();
    fs::write(store.path().join("unrelated.txt"), "").unwrap();

    let output = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "init_target_not_empty");
}

#[test]
fn doctor_detects_invalid_capture_and_saved_render_files() {
    let store = TestStore::initialized("record-validation");

    fs::write(store.path().join("captures/broken.json"), "not json").unwrap();
    fs::write(store.path().join("captures/not-object.json"), "[]").unwrap();
    fs::write(store.path().join("renders/broken.md"), [0xff, 0xfe]).unwrap();
    fs::write(
        store.path().join("renders/invalid-ids.md"),
        r#"---
id: not_a_render_id
sequence_id: not_a_sequence_id
sequence_name: Invalid IDs
sequence_path: sequences/invalid.json
annotated: false
---
rendered
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("renders/unknown-field.md"),
        r#"---
id: rnd_unknown
sequence_id: seq_unknown
sequence_name: Unknown
sequence_path: sequences/unknown.json
annotated: false
unexpected: true
---
rendered
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("renders/bad-sequence-path.md"),
        r#"---
id: rnd_00000000000000000000000000000001
sequence_id: seq_00000000000000000000000000000001
sequence_name: Bad Path
sequence_path: sequences\bad.json
annotated: false
---
rendered
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("renders/duplicate-a.md"),
        r#"---
id: rnd_00000000000000000000000000000002
sequence_id: seq_00000000000000000000000000000001
sequence_name: Duplicate A
sequence_path: sequences/duplicate-a.json
annotated: false
---
rendered
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("renders/duplicate-b.md"),
        r#"---
id: rnd_00000000000000000000000000000002
sequence_id: seq_00000000000000000000000000000001
sequence_name: Duplicate B
sequence_path: sequences/duplicate-b.json
annotated: false
---
rendered
"#,
    )
    .unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(doctor.stderr.is_empty());

    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], false);
    let codes = issue_codes(&json);
    assert!(codes.contains(&"capture_file_invalid"));
    assert!(codes.contains(&"render_file_invalid"));
    assert!(codes.contains(&"render_id_invalid"));
    assert!(codes.contains(&"render_sequence_id_invalid"));
    assert!(codes.contains(&"render_sequence_path_invalid"));
    assert!(codes.contains(&"render_id_duplicate"));
}

#[test]
fn config_show_reports_config_as_non_json_and_json() {
    let store = TestStore::initialized("config-show");

    let json_output = pseq(&[
        "config",
        "show",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&json_output);
    let json = stdout_json(&json_output);
    assert_eq!(json["path"], "config.toml");
    assert_eq!(json["version"], 1);
    assert_eq!(json["runner_count"], 0);

    let non_json_output = pseq(&["config", "show", "--store", path_str(store.path())]);
    assert_success(&non_json_output);
    assert_stdout_contains(&non_json_output, "path: config.toml");
    assert_stdout_contains(&non_json_output, "version: 1");
    assert_stdout_contains(&non_json_output, "runners: 0");
    assert_git_clean(store.path());
}

#[test]
fn blank_fragment_and_sequence_names_fail_before_mutation() {
    let store = TestStore::initialized("blank-names");

    let blank_fragment = pseq(&[
        "fragment",
        "new",
        "   ",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(blank_fragment.status.code(), Some(1));
    assert!(blank_fragment.stdout.is_empty());
    assert_eq!(
        stderr_json(&blank_fragment)["error"]["code"],
        "invalid_fragment_name"
    );

    let blank_sequence = pseq(&[
        "sequence",
        "new",
        "   ",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(blank_sequence.status.code(), Some(1));
    assert!(blank_sequence.stdout.is_empty());
    assert_eq!(
        stderr_json(&blank_sequence)["error"]["code"],
        "invalid_sequence_name"
    );

    assert_success(&pseq(&[
        "fragment",
        "new",
        "Valid Fragment",
        "--store",
        path_str(store.path()),
    ]));
    let invalid_fragment_rename = pseq(&[
        "fragment",
        "rename",
        "Valid Fragment",
        "   ",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(invalid_fragment_rename.status.code(), Some(1));
    assert_eq!(
        stderr_json(&invalid_fragment_rename)["error"]["code"],
        "invalid_fragment_name"
    );

    assert_success(&pseq(&[
        "sequence",
        "new",
        "Valid Sequence",
        "--store",
        path_str(store.path()),
    ]));
    let invalid_sequence_rename = pseq(&[
        "sequence",
        "rename",
        "Valid Sequence",
        "   ",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(invalid_sequence_rename.status.code(), Some(1));
    assert_eq!(
        stderr_json(&invalid_sequence_rename)["error"]["code"],
        "invalid_sequence_name"
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let json = stdout_json(&status);
    assert_eq!(json["counts"]["fragments"], 1);
    assert_eq!(json["counts"]["sequences"], 1);
    assert_git_clean(store.path());
}

#[test]
fn quiet_suppresses_non_json_success_output() {
    let store = TestStore::new("quiet");

    let output = pseq(&["init", "--store", path_str(store.path()), "--quiet"]);
    assert_success(&output);
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(store.path().join("config.toml").is_file());
}

#[test]
fn store_alias_c_works() {
    let store = TestStore::new("alias");

    let output = pseq(&["-C", path_str(store.path()), "init"]);
    assert_success(&output);

    let doctor = pseq(&["-C", path_str(store.path()), "doctor", "--json"]);
    assert_success(&doctor);
    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], true);
}

#[test]
fn user_config_default_store_is_used() {
    let config_home = TestStore::new("config-home");
    let store = TestStore::new("config-store");
    write_user_config(
        config_home.path(),
        &format!("store = '{}'\n", path_str(store.path())),
    );

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[("XDG_CONFIG_HOME", path_str(config_home.path()))],
        &["PSEQ_STORE"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(store.path()));
    assert!(store.path().join("config.toml").is_file());
}

#[test]
fn store_env_overrides_user_config() {
    let config_home = TestStore::new("config-env-home");
    let config_store = TestStore::new("config-env-store");
    let env_store = TestStore::new("env-store");
    write_user_config(
        config_home.path(),
        &format!("store = '{}'\n", path_str(config_store.path())),
    );

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[
            ("XDG_CONFIG_HOME", path_str(config_home.path())),
            ("PSEQ_STORE", path_str(env_store.path())),
        ],
        &[],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(env_store.path()));
    assert!(env_store.path().join("config.toml").is_file());
    assert!(!config_store.path().exists());
}

#[test]
fn explicit_store_overrides_env_and_user_config() {
    let config_home = TestStore::new("config-explicit-home");
    let config_store = TestStore::new("config-explicit-store");
    let env_store = TestStore::new("explicit-env-store");
    let explicit_store = TestStore::new("explicit-store");
    write_user_config(
        config_home.path(),
        &format!("store = '{}'\n", path_str(config_store.path())),
    );

    let output = pseq_with_env_changes(
        &["init", "--store", path_str(explicit_store.path()), "--json"],
        &[
            ("XDG_CONFIG_HOME", path_str(config_home.path())),
            ("PSEQ_STORE", path_str(env_store.path())),
        ],
        &[],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(explicit_store.path()));
    assert!(explicit_store.path().join("config.toml").is_file());
    assert!(!env_store.path().exists());
    assert!(!config_store.path().exists());
}

#[test]
fn store_alias_overrides_env_and_user_config() {
    let config_home = TestStore::new("config-alias-home");
    let config_store = TestStore::new("config-alias-store");
    let env_store = TestStore::new("alias-env-store");
    let alias_store = TestStore::new("alias-store");
    write_user_config(
        config_home.path(),
        &format!("store = '{}'\n", path_str(config_store.path())),
    );

    let output = pseq_with_env_changes(
        &["-C", path_str(alias_store.path()), "init", "--json"],
        &[
            ("XDG_CONFIG_HOME", path_str(config_home.path())),
            ("PSEQ_STORE", path_str(env_store.path())),
        ],
        &[],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(alias_store.path()));
    assert!(alias_store.path().join("config.toml").is_file());
    assert!(!env_store.path().exists());
    assert!(!config_store.path().exists());
}

#[test]
fn invalid_user_config_is_ignored_when_store_is_selected_earlier() {
    let config_home = TestStore::new("invalid-ignored-home");
    let env_store = TestStore::new("invalid-ignored-env");
    let explicit_store = TestStore::new("invalid-ignored-explicit");
    write_user_config(config_home.path(), "store = ''\nunexpected = true\n");

    let env_output = pseq_with_env_changes(
        &["init", "--json"],
        &[
            ("XDG_CONFIG_HOME", path_str(config_home.path())),
            ("PSEQ_STORE", path_str(env_store.path())),
        ],
        &[],
    );
    assert_success(&env_output);
    assert_eq!(
        stdout_json(&env_output)["store"],
        path_str(env_store.path())
    );

    let explicit_output = pseq_with_env_changes(
        &["init", "--store", path_str(explicit_store.path()), "--json"],
        &[("XDG_CONFIG_HOME", path_str(config_home.path()))],
        &["PSEQ_STORE"],
    );
    assert_success(&explicit_output);
    assert_eq!(
        stdout_json(&explicit_output)["store"],
        path_str(explicit_store.path())
    );
}

#[test]
fn invalid_user_config_fails_when_consulted() {
    let config_home = TestStore::new("invalid-config-home");
    write_user_config(config_home.path(), "store = ''\n");

    let output = pseq_with_env_changes(
        &["status", "--json"],
        &[("XDG_CONFIG_HOME", path_str(config_home.path()))],
        &["PSEQ_STORE"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "invalid_user_config");
}

#[test]
fn user_config_relative_store_is_resolved_from_config_directory() {
    let config_home = TestStore::new("relative-config-home");
    let reported_store = config_home.path().join("pseq").join("../store-relative");
    let filesystem_store = config_home.path().join("store-relative");
    write_user_config(config_home.path(), "store = '../store-relative'\n");

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[("XDG_CONFIG_HOME", path_str(config_home.path()))],
        &["PSEQ_STORE"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(&reported_store));
    assert!(filesystem_store.join("config.toml").is_file());
}

#[test]
fn missing_user_config_falls_back_to_builtin_default_store() {
    let home = TestStore::new("default-home");
    fs::create_dir_all(home.path()).unwrap();
    let expected_store = home.path().join(".pseq");

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[("HOME", path_str(home.path()))],
        &["PSEQ_STORE", "XDG_CONFIG_HOME", "USERPROFILE"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(&expected_store));
    assert!(expected_store.join("config.toml").is_file());
}

#[test]
fn empty_home_uses_userprofile_for_builtin_default_store() {
    let profile = TestStore::new("default-userprofile");
    fs::create_dir_all(profile.path()).unwrap();
    let expected_store = profile.path().join(".pseq");

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[("HOME", ""), ("USERPROFILE", path_str(profile.path()))],
        &["PSEQ_STORE", "XDG_CONFIG_HOME"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(&expected_store));
    assert!(expected_store.join("config.toml").is_file());
}

#[test]
fn empty_home_uses_userprofile_for_user_config_location() {
    let profile = TestStore::new("config-userprofile");
    let store = TestStore::new("config-userprofile-store");
    write_user_config(
        &profile.path().join(".config"),
        &format!("store = '{}'\n", path_str(store.path())),
    );

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[("HOME", ""), ("USERPROFILE", path_str(profile.path()))],
        &["PSEQ_STORE", "XDG_CONFIG_HOME"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(store.path()));
    assert!(store.path().join("config.toml").is_file());
}

#[test]
fn home_precedes_userprofile_for_user_config_location() {
    let home = TestStore::new("config-home-priority");
    let profile = TestStore::new("config-userprofile-priority");
    let home_store = TestStore::new("config-home-priority-store");
    let profile_store = TestStore::new("config-userprofile-priority-store");
    write_user_config(
        &home.path().join(".config"),
        &format!("store = '{}'\n", path_str(home_store.path())),
    );
    write_user_config(
        &profile.path().join(".config"),
        &format!("store = '{}'\n", path_str(profile_store.path())),
    );

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[
            ("HOME", path_str(home.path())),
            ("USERPROFILE", path_str(profile.path())),
        ],
        &["PSEQ_STORE", "XDG_CONFIG_HOME"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(home_store.path()));
    assert!(home_store.path().join("config.toml").is_file());
    assert!(!profile_store.path().exists());
}

#[test]
fn empty_home_without_userprofile_cannot_select_builtin_default_store() {
    let output = pseq_with_env_changes(
        &["status", "--json"],
        &[("HOME", "")],
        &["PSEQ_STORE", "XDG_CONFIG_HOME", "USERPROFILE"],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "default_store_unavailable");
}

#[test]
fn user_config_without_store_falls_back_to_builtin_default_store() {
    let home = TestStore::new("empty-config-home");
    let config_home = TestStore::new("empty-config-xdg");
    fs::create_dir_all(home.path()).unwrap();
    write_user_config(config_home.path(), "\n");
    let expected_store = home.path().join(".pseq");

    let output = pseq_with_env_changes(
        &["init", "--json"],
        &[
            ("HOME", path_str(home.path())),
            ("XDG_CONFIG_HOME", path_str(config_home.path())),
        ],
        &["PSEQ_STORE", "USERPROFILE"],
    );
    assert_success(&output);
    let json = stdout_json(&output);
    assert_eq!(json["store"], path_str(&expected_store));
    assert!(expected_store.join("config.toml").is_file());
}

#[test]
fn read_commands_do_not_create_version_history_entries() {
    let store = TestStore::initialized("read-no-mutation");
    assert_success(&pseq(&[
        "fragment",
        "new",
        "Read Fragment",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Read Sequence",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Read Sequence",
        "Read Fragment",
        "--store",
        path_str(store.path()),
    ]));

    let head = git_head(store.path());
    for args in [
        vec!["doctor", "--store", path_str(store.path())],
        vec!["status", "--store", path_str(store.path())],
        vec!["log", "--store", path_str(store.path())],
        vec!["diff", "--store", path_str(store.path())],
        vec!["config", "show", "--store", path_str(store.path())],
        vec!["runner", "list", "--store", path_str(store.path())],
        vec!["fragment", "list", "--store", path_str(store.path())],
        vec![
            "fragment",
            "show",
            "Read Fragment",
            "--store",
            path_str(store.path()),
        ],
        vec!["sequence", "list", "--store", path_str(store.path())],
        vec![
            "sequence",
            "show",
            "Read Sequence",
            "--store",
            path_str(store.path()),
        ],
        vec!["render", "Read Sequence", "--store", path_str(store.path())],
    ] {
        assert_success(&pseq(&args));
        assert_eq!(git_head(store.path()), head);
    }

    assert_git_clean(store.path());
}

#[test]
fn mutating_commands_commit_only_paths_they_touch() {
    let store = TestStore::initialized("path-scoped-commit");
    fs::write(store.path().join("manual-note.txt"), "manual\n").unwrap();

    let created = pseq(&[
        "fragment",
        "new",
        "Committed Fragment",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&created);

    let status = git_status(store.path());
    assert!(status.contains("?? manual-note.txt"));
    assert!(!status.contains("fragments/committed-fragment.md"));
}

#[test]
fn json_cli_parse_errors_use_error_envelope() {
    let output = pseq(&["--json", "render"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    let json = stderr_json(&output);
    assert_eq!(json["error"]["code"], "cli_parse_failed");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("<SEQUENCE>")
    );
    assert_eq!(json["error"]["details"]["kind"], "MissingRequiredArgument");
}

fn write_user_config(config_home: &Path, content: &str) {
    let config_dir = config_home.join("pseq");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.toml"), content).unwrap();
}
