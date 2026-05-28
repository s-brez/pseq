#[path = "common/mod.rs"]
mod common;

use std::fs;

use common::{
    TestStore, assert_git_clean, assert_success, issue_codes, path_str, pseq, pseq_with_env,
    pseq_with_stdin, stderr_json, stdout_json,
};

#[test]
fn fragment_create_list_and_show_from_stdin_and_file() {
    let store = TestStore::initialized("fragments");

    let stdin_body = "alpha\nbeta\n";
    let created_from_stdin = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Stdin Prompt",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        stdin_body,
    );
    assert_success(&created_from_stdin);
    let stdin_json = stdout_json(&created_from_stdin);
    let stdin_id = stdin_json["id"].as_str().unwrap();
    assert!(stdin_id.starts_with("frg_"));
    assert_eq!(stdin_json["path"], "fragments/stdin-prompt.md");

    let source_path = store.path().with_extension("source.md");
    let file_body = "file body\nwith second line";
    fs::write(&source_path, file_body).unwrap();
    let created_from_file = pseq(&[
        "frag",
        "new",
        "File Prompt",
        "--from-file",
        path_str(&source_path),
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    let _ = fs::remove_file(&source_path);
    assert_success(&created_from_file);
    let file_json = stdout_json(&created_from_file);
    assert_eq!(file_json["path"], "fragments/file-prompt.md");

    let list = pseq(&[
        "fragment",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    let list_json = stdout_json(&list);
    assert_eq!(list_json["fragments"].as_array().unwrap().len(), 2);
    assert_eq!(
        list_json["fragments"][0]["path"],
        "fragments/file-prompt.md"
    );
    assert_eq!(
        list_json["fragments"][1]["path"],
        "fragments/stdin-prompt.md"
    );

    let show_by_name = pseq(&[
        "fragment",
        "show",
        "Stdin Prompt",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&show_by_name);
    assert_eq!(String::from_utf8(show_by_name.stdout).unwrap(), stdin_body);

    let short_id = &stdin_id[..12];
    let show_by_short_id = pseq(&[
        "fragment",
        "show",
        short_id,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show_by_short_id);
    assert_eq!(stdout_json(&show_by_short_id)["body"], stdin_body);

    let show_by_path = pseq(&[
        "fragment",
        "show",
        "fragments/file-prompt.md",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show_by_path);
    assert_eq!(stdout_json(&show_by_path)["body"], file_body);

    assert_git_clean(store.path());
}

#[test]
fn ambiguous_fragment_name_and_short_id_fail_closed() {
    let store = TestStore::initialized("fragment-ambiguity");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Repeated",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "first",
    ));
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Repeated",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "second",
    ));

    let by_name = pseq(&[
        "fragment",
        "show",
        "Repeated",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(by_name.status.code(), Some(1));
    assert!(by_name.stdout.is_empty());
    assert_eq!(
        stderr_json(&by_name)["error"]["code"],
        "fragment_reference_ambiguous"
    );

    let by_short_id = pseq(&[
        "fragment",
        "show",
        "frg_",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(by_short_id.status.code(), Some(1));
    assert!(by_short_id.stdout.is_empty());
    assert_eq!(
        stderr_json(&by_short_id)["error"]["code"],
        "fragment_reference_ambiguous"
    );
}

#[test]
fn fragment_rename_preserves_body_and_remove_deletes_unused_fragment() {
    let store = TestStore::initialized("fragment-rename-remove");

    let body = "keep this body exactly\n{{placeholder}}\n";
    let created = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Original Name",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        body,
    );
    assert_success(&created);
    let id = stdout_json(&created)["id"].as_str().unwrap().to_owned();

    let renamed = pseq(&[
        "fragment",
        "rename",
        &id,
        "Renamed Fragment",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&renamed);
    let renamed_json = stdout_json(&renamed);
    assert_eq!(renamed_json["id"], id);
    assert_eq!(renamed_json["name"], "Renamed Fragment");

    let show = pseq(&[
        "fragment",
        "show",
        "Renamed Fragment",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&show);
    assert_eq!(String::from_utf8(show.stdout).unwrap(), body);

    let removed = pseq(&[
        "fragment",
        "rm",
        "Renamed Fragment",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&removed);
    assert_eq!(stdout_json(&removed)["id"], id);
    assert!(!store.path().join("fragments/original-name.md").exists());

    let list = pseq(&[
        "fragment",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    assert_eq!(stdout_json(&list)["fragments"].as_array().unwrap().len(), 0);
    assert_git_clean(store.path());
}

#[test]
fn fragment_edit_uses_editor_preserves_metadata_and_versions_change() {
    let store = TestStore::initialized("fragment-edit");

    let created = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Editable",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "original\n",
    );
    assert_success(&created);
    let id = stdout_json(&created)["id"].as_str().unwrap().to_owned();

    let script = store.path().with_extension("fragment-editor.sh");
    fs::write(
        &script,
        format!(
            r#"cat > "$1" <<'EOF'
---
id: {id}
name: Edited Fragment
audience: agents
priority: 2
---
edited body
EOF
"#
        ),
    )
    .unwrap();
    let editor = format!("sh {}", path_str(&script));

    let edited = pseq_with_env(
        &[
            "fragment",
            "edit",
            "Editable",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&script);
    assert_success(&edited);
    let edited_json = stdout_json(&edited);
    assert_eq!(edited_json["id"], id);
    assert_eq!(edited_json["name"], "Edited Fragment");
    assert!(edited_json["git_commit"].is_string());

    let shown = pseq(&[
        "fragment",
        "show",
        "Edited Fragment",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&shown);
    let shown_json = stdout_json(&shown);
    assert_eq!(shown_json["body"], "edited body\n");
    assert_eq!(shown_json["metadata"]["audience"], "agents");
    assert_eq!(shown_json["metadata"]["priority"], 2);
    assert_git_clean(store.path());
}

#[cfg(unix)]
#[test]
fn fragment_edit_uses_owner_private_temp_file() {
    let store = TestStore::initialized("fragment-edit-private-temp");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Private Temp",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "original\n",
    ));

    let marker = store.path().with_extension("temp-modes.txt");
    let script = store.path().with_extension("temp-mode-editor.sh");
    fs::write(
        &script,
        format!(
            r#"printf "%s %s" "$(stat -c '%a' "$(dirname "$1")")" "$(stat -c '%a' "$1")" > '{}'
"#,
            path_str(&marker)
        ),
    )
    .unwrap();
    let editor = format!("sh {}", path_str(&script));

    let edited = pseq_with_env(
        &[
            "fragment",
            "edit",
            "Private Temp",
            "--store",
            path_str(store.path()),
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&script);
    assert_success(&edited);
    assert_eq!(fs::read_to_string(&marker).unwrap(), "700 600");
    let _ = fs::remove_file(&marker);
    assert_git_clean(store.path());
}

#[test]
fn fragment_edit_rejects_id_changes_before_mutating_store() {
    let store = TestStore::initialized("fragment-edit-id");

    let created = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Stable",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "original\n",
    );
    assert_success(&created);
    let path = store.path().join("fragments/stable.md");
    let original = fs::read_to_string(&path).unwrap();

    let script = store.path().with_extension("fragment-bad-editor.sh");
    fs::write(
        &script,
        r#"cat > "$1" <<'EOF'
---
id: frg_changed
name: Stable
---
changed
EOF
"#,
    )
    .unwrap();
    let editor = format!("sh {}", path_str(&script));

    let edited = pseq_with_env(
        &[
            "fragment",
            "edit",
            "Stable",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&script);
    assert_eq!(edited.status.code(), Some(1));
    assert!(edited.stdout.is_empty());
    assert_eq!(
        stderr_json(&edited)["error"]["code"],
        "invalid_edited_fragment"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert_git_clean(store.path());
}

#[test]
fn fragment_remove_fails_when_fragment_is_used_by_a_sequence() {
    let store = TestStore::initialized("fragment-rm-used");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Used",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "body",
    ));
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Uses Fragment",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Uses Fragment",
        "Used",
        "--store",
        path_str(store.path()),
    ]));

    let removed = pseq(&[
        "fragment",
        "rm",
        "Used",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(removed.status.code(), Some(1));
    assert!(removed.stdout.is_empty());
    assert_eq!(stderr_json(&removed)["error"]["code"], "fragment_in_use");
    assert!(store.path().join("fragments/used.md").exists());
    assert_git_clean(store.path());
}

#[test]
fn doctor_detects_invalid_fragment_files_and_duplicate_ids() {
    let store = TestStore::initialized("fragment-doctor");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Original",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "body",
    ));

    let original = store.path().join("fragments/original.md");
    let duplicate = store.path().join("fragments/duplicate.md");
    fs::copy(&original, &duplicate).unwrap();
    fs::write(store.path().join("fragments/broken.md"), "not frontmatter").unwrap();
    fs::write(
        store.path().join("fragments/bad-id.md"),
        "---\nid: not_a_fragment_id\nname: Bad ID\n---\nbody",
    )
    .unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(doctor.stderr.is_empty());

    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], false);
    let codes = issue_codes(&json);
    assert!(codes.contains(&"fragment_file_invalid"));
    assert!(codes.contains(&"fragment_id_invalid"));
    assert!(codes.contains(&"fragment_id_duplicate"));

    let list = pseq(&[
        "fragment",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(list.status.code(), Some(1));
    assert!(list.stdout.is_empty());
    assert_eq!(stderr_json(&list)["error"]["code"], "invalid_store");
}
