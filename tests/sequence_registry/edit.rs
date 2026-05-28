use super::*;

#[test]
fn sequence_edit_uses_editor_validates_and_preserves_optional_data() {
    let store = TestStore::initialized("sequence-edit");

    let first = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "First",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "first\n",
    );
    assert_success(&first);
    let first_id = stdout_json(&first)["id"].as_str().unwrap().to_owned();

    let second = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Second",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "second\n",
    );
    assert_success(&second);
    let second_id = stdout_json(&second)["id"].as_str().unwrap().to_owned();

    let created = pseq(&[
        "sequence",
        "new",
        "Editable Sequence",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&created);
    let sequence_id = stdout_json(&created)["id"].as_str().unwrap().to_owned();
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Editable Sequence",
        "First",
        "--store",
        path_str(store.path()),
    ]));

    let script = store.path().with_extension("sequence-editor.sh");
    fs::write(
        &script,
        format!(
            r#"cat > "$1" <<'EOF'
{{
  "id": "{sequence_id}",
  "name": "Edited Sequence",
  "fragments": [
    "{second_id}",
    "{first_id}"
  ],
  "variables": {{
    "subject": "required"
  }},
  "metadata": {{
    "purpose": "test"
  }}
}}
EOF
"#
        ),
    )
    .unwrap();
    let editor = format!("sh {}", path_str(&script));

    let edited = pseq_with_env(
        &[
            "sequence",
            "edit",
            "Editable Sequence",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&script);
    assert_success(&edited);
    let edited_json = stdout_json(&edited);
    assert_eq!(edited_json["id"], sequence_id);
    assert_eq!(edited_json["name"], "Edited Sequence");
    assert_eq!(edited_json["fragment_count"], 2);
    assert!(edited_json["git_commit"].is_string());

    let show = pseq(&[
        "sequence",
        "show",
        "Edited Sequence",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["fragments"][0]["id"], second_id);
    assert_eq!(show_json["fragments"][1]["id"], first_id);
    assert_eq!(show_json["variables"]["subject"], "required");
    assert_eq!(show_json["metadata"]["purpose"], "test");

    let rendered = pseq(&[
        "render",
        "Edited Sequence",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&rendered);
    assert_eq!(
        String::from_utf8(rendered.stdout).unwrap(),
        "second\nfirst\n"
    );
    assert_git_clean(store.path());
}

#[test]
fn sequence_edit_rejects_missing_fragment_before_mutating_store() {
    let store = TestStore::initialized("sequence-edit-invalid");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Only",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "only\n",
    ));
    let created = pseq(&[
        "sequence",
        "new",
        "Stable Sequence",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&created);
    let sequence_id = stdout_json(&created)["id"].as_str().unwrap().to_owned();
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Stable Sequence",
        "Only",
        "--store",
        path_str(store.path()),
    ]));
    let path = store.path().join("sequences/stable-sequence.json");
    let original = fs::read_to_string(&path).unwrap();

    let script = store.path().with_extension("sequence-bad-editor.sh");
    fs::write(
        &script,
        format!(
            r#"cat > "$1" <<'EOF'
{{
  "id": "{sequence_id}",
  "name": "Stable Sequence",
  "fragments": ["frg_00000000000000000000000000000001"]
}}
EOF
"#
        ),
    )
    .unwrap();
    let editor = format!("sh {}", path_str(&script));

    let edited = pseq_with_env(
        &[
            "sequence",
            "edit",
            "Stable Sequence",
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
        "invalid_edited_sequence"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert_git_clean(store.path());
}
