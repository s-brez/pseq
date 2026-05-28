use super::super::*;

pub(super) fn assert_fragment_payloads(store: &TestStore) {
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
        "first {{name}}\n",
    );
    assert_success(&first);
    let first_json = stdout_json(&first);
    assert_object_keys(&first_json, &["id", "name", "path", "git_commit"]);

    let fragment_list = pseq(&[
        "fragment",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&fragment_list);
    let fragment_list_json = stdout_json(&fragment_list);
    assert_object_keys(&fragment_list_json, &["fragments"]);
    assert_object_keys(
        first_array_item(&fragment_list_json, "fragments"),
        &["id", "name", "path"],
    );

    let fragment_show = pseq(&[
        "fragment",
        "show",
        "First",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&fragment_show);
    assert_object_keys(
        &stdout_json(&fragment_show),
        &["id", "name", "path", "body"],
    );

    let fragment_rename = pseq(&[
        "fragment",
        "rename",
        "First",
        "First Renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&fragment_rename);
    assert_object_keys(
        &stdout_json(&fragment_rename),
        &["id", "name", "path", "git_commit"],
    );

    let noop_editor = write_noop_editor(store.path());
    let editor = format!("sh {}", path_str(&noop_editor));
    let fragment_edit = pseq_with_env(
        &[
            "fragment",
            "edit",
            "First Renamed",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        &[("EDITOR", editor.as_str())],
    );
    let _ = fs::remove_file(&noop_editor);
    assert_success(&fragment_edit);
    assert_object_keys(&stdout_json(&fragment_edit), &["id", "name", "path"]);

    let fragment_mv = pseq(&[
        "fragment",
        "mv",
        "First Renamed",
        "contract/first-renamed",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&fragment_mv);
    assert_object_keys(
        &stdout_json(&fragment_mv),
        &["id", "name", "path", "git_commit"],
    );

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
}

pub(super) fn assert_fragment_remove_payload(store: &TestStore) {
    let fragment_rm = pseq(&[
        "fragment",
        "rm",
        "Second",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&fragment_rm);
    assert_object_keys(
        &stdout_json(&fragment_rm),
        &["id", "name", "path", "git_commit"],
    );
}
