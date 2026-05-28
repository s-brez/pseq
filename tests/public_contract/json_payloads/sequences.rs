use super::super::*;

pub(super) fn assert_sequence_payloads(store: &TestStore) {
    let sequence_new = pseq(&[
        "sequence",
        "new",
        "Seq",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_new);
    assert_object_keys(
        &stdout_json(&sequence_new),
        &["id", "name", "path", "git_commit"],
    );

    let sequence_list = pseq(&[
        "sequence",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_list);
    let sequence_list_json = stdout_json(&sequence_list);
    assert_object_keys(&sequence_list_json, &["sequences"]);
    assert_object_keys(
        first_array_item(&sequence_list_json, "sequences"),
        &["id", "name", "path", "fragment_count"],
    );

    let sequence_show_empty = pseq(&[
        "sequence",
        "show",
        "Seq",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_show_empty);
    assert_object_keys(
        &stdout_json(&sequence_show_empty),
        &["id", "name", "path", "fragments"],
    );

    let sequence_add = pseq(&[
        "sequence",
        "add",
        "Seq",
        "First Renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_add);
    let sequence_add_json = stdout_json(&sequence_add);
    assert_object_keys(
        &sequence_add_json,
        &[
            "id",
            "name",
            "path",
            "fragment",
            "index",
            "fragment_count",
            "git_commit",
        ],
    );
    assert_object_keys(&sequence_add_json["fragment"], &["id", "name", "path"]);

    assert_success(&pseq(&[
        "sequence",
        "add",
        "Seq",
        "Second",
        "--store",
        path_str(store.path()),
    ]));

    let sequence_move = pseq(&[
        "sequence",
        "move",
        "Seq",
        "2",
        "1",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_move);
    assert_object_keys(
        &stdout_json(&sequence_move),
        &[
            "id",
            "name",
            "path",
            "from_index",
            "to_index",
            "fragment_count",
            "git_commit",
        ],
    );

    let sequence_remove = pseq(&[
        "sequence",
        "remove",
        "Seq",
        "1",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_remove);
    let sequence_remove_json = stdout_json(&sequence_remove);
    assert_object_keys(
        &sequence_remove_json,
        &[
            "id",
            "name",
            "path",
            "removed_fragment",
            "fragment_count",
            "git_commit",
        ],
    );
    assert_object_keys(
        &sequence_remove_json["removed_fragment"],
        &["id", "name", "path"],
    );

    let sequence_rename = pseq(&[
        "sequence",
        "rename",
        "Seq",
        "Seq Renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_rename);
    assert_object_keys(
        &stdout_json(&sequence_rename),
        &["id", "name", "path", "fragment_count", "git_commit"],
    );

    let noop_editor = write_noop_editor(store.path());
    let editor = format!("sh {}", path_str(&noop_editor));
    let sequence_edit = pseq_with_env(
        &[
            "sequence",
            "edit",
            "Seq Renamed",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&noop_editor);
    assert_success(&sequence_edit);
    assert_object_keys(
        &stdout_json(&sequence_edit),
        &["id", "name", "path", "fragment_count"],
    );

    let sequence_mv = pseq(&[
        "sequence",
        "mv",
        "Seq Renamed",
        "contract/seq-renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_mv);
    assert_object_keys(
        &stdout_json(&sequence_mv),
        &["id", "name", "path", "fragment_count", "git_commit"],
    );
}

pub(super) fn assert_sequence_remove_payload(store: &TestStore) {
    let sequence_rm = pseq(&[
        "sequence",
        "rm",
        "Seq Renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence_rm);
    assert_object_keys(
        &stdout_json(&sequence_rm),
        &["id", "name", "path", "fragment_count", "git_commit"],
    );
}
