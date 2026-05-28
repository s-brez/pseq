use super::*;

#[test]
fn sequence_remove_move_rename_and_rm_end_to_end() {
    let store = TestStore::initialized("sequence-maintenance");

    for (name, body) in [
        ("First", "first\n"),
        ("Second", "second\n"),
        ("Third", "third\n"),
    ] {
        assert_success(&pseq_with_stdin(
            &[
                "fragment",
                "new",
                name,
                "--stdin",
                "--store",
                path_str(store.path()),
            ],
            body,
        ));
    }

    assert_success(&pseq(&[
        "sequence",
        "new",
        "Combo",
        "--store",
        path_str(store.path()),
    ]));
    for name in ["First", "Second", "Third"] {
        assert_success(&pseq(&[
            "sequence",
            "add",
            "Combo",
            name,
            "--store",
            path_str(store.path()),
        ]));
    }

    let moved = pseq(&[
        "sequence",
        "move",
        "Combo",
        "3",
        "1",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&moved);
    let moved_json = stdout_json(&moved);
    assert_eq!(moved_json["from_index"], 3);
    assert_eq!(moved_json["to_index"], 1);

    let show = pseq(&[
        "sequence",
        "show",
        "Combo",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["fragments"][0]["name"], "Third");
    assert_eq!(show_json["fragments"][1]["name"], "First");
    assert_eq!(show_json["fragments"][2]["name"], "Second");

    let removed_by_index = pseq(&[
        "sequence",
        "remove",
        "Combo",
        "2",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&removed_by_index);
    assert_eq!(
        stdout_json(&removed_by_index)["removed_fragment"]["name"],
        "First"
    );

    let removed_by_ref = pseq(&[
        "sequence",
        "remove",
        "Combo",
        "Second",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&removed_by_ref);
    assert_eq!(
        stdout_json(&removed_by_ref)["removed_fragment"]["name"],
        "Second"
    );

    let renamed = pseq(&[
        "sequence",
        "rename",
        "Combo",
        "Renamed Combo",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&renamed);
    let renamed_json = stdout_json(&renamed);
    assert_eq!(renamed_json["name"], "Renamed Combo");
    assert_eq!(renamed_json["path"], "sequences/combo.json");

    let rendered = pseq(&["render", "Renamed Combo", "--store", path_str(store.path())]);
    assert_success(&rendered);
    assert_eq!(String::from_utf8(rendered.stdout).unwrap(), "third\n");

    let removed = pseq(&[
        "sequence",
        "rm",
        "Renamed Combo",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&removed);
    assert_eq!(stdout_json(&removed)["path"], "sequences/combo.json");
    assert!(!store.path().join("sequences/combo.json").exists());

    let list = pseq(&[
        "sequence",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    assert_eq!(stdout_json(&list)["sequences"].as_array().unwrap().len(), 0);
    assert_git_clean(store.path());
}

#[test]
fn sequence_remove_by_repeated_fragment_reference_fails_closed() {
    let store = TestStore::initialized("sequence-remove-ambiguity");
    assert_success(&pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Repeated Fragment",
            "--stdin",
            "--store",
            path_str(store.path()),
        ],
        "body",
    ));
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Repeated Sequence",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Repeated Sequence",
        "Repeated Fragment",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Repeated Sequence",
        "Repeated Fragment",
        "--store",
        path_str(store.path()),
    ]));

    let removed = pseq(&[
        "sequence",
        "remove",
        "Repeated Sequence",
        "Repeated Fragment",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(removed.status.code(), Some(1));
    assert!(removed.stdout.is_empty());
    assert_eq!(
        stderr_json(&removed)["error"]["code"],
        "sequence_fragment_reference_ambiguous"
    );

    let removed_by_index = pseq(&[
        "sequence",
        "remove",
        "Repeated Sequence",
        "1",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&removed_by_index);
    assert_eq!(stdout_json(&removed_by_index)["fragment_count"], 1);
    assert_git_clean(store.path());
}
