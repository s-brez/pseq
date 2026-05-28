use super::super::*;

pub(super) fn assert_capture_payloads(store: &TestStore) {
    let capture_import = pseq_with_stdin(
        &[
            "capture",
            "import",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "captured prompt\n",
    );
    assert_success(&capture_import);
    let capture_import_json = stdout_json(&capture_import);
    assert_capture_import_shape(&capture_import_json);
    let capture_id = capture_import_json["id"].as_str().unwrap().to_owned();

    let capture_list = pseq(&[
        "capture",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&capture_list);
    let capture_list_json = stdout_json(&capture_list);
    assert_object_keys(&capture_list_json, &["captures"]);
    assert_capture_summary_shape(first_array_item(&capture_list_json, "captures"));

    let capture_show = pseq(&[
        "capture",
        "show",
        &capture_id,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&capture_show);
    let capture_show_json = stdout_json(&capture_show);
    assert_object_keys(&capture_show_json, &["id", "path", "origin", "events"]);
    assert_object_keys(
        first_array_item(&capture_show_json, "events"),
        &["index", "kind", "text"],
    );

    let capture_mv = pseq(&[
        "capture",
        "mv",
        &capture_id,
        "contract/captured",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&capture_mv);
    let capture_mv_json = stdout_json(&capture_mv);
    assert_object_keys(
        &capture_mv_json,
        &["id", "path", "origin", "event_count", "git_commit"],
    );

    let capture_promote = pseq(&[
        "capture",
        "promote",
        &capture_id,
        "--as-sequence",
        "Captured Seq",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&capture_promote);
    assert_capture_promote_shape(&stdout_json(&capture_promote));

    let capture_last = pseq_with_stdin(
        &[
            "capture",
            "last",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "last prompt\n",
    );
    assert_success(&capture_last);
    assert_capture_import_shape(&stdout_json(&capture_last));

    let capture_range = pseq_with_stdin(
        &[
            "capture",
            "range",
            "1..1",
            "--as-sequence",
            "Range Seq",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "range prompt\n",
    );
    assert_success(&capture_range);
    assert_capture_promote_shape(&stdout_json(&capture_range));
}
