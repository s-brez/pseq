use super::super::*;

pub(super) fn assert_render_payload(store: &TestStore) {
    let rendered = pseq(&[
        "render",
        "Seq Renamed",
        "--var",
        "name=Contract",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&rendered);
    assert_object_keys(
        &stdout_json(&rendered),
        &["id", "name", "path", "text", "annotated"],
    );
}

pub(super) fn assert_run_payloads(store: &TestStore) {
    let run = pseq(&[
        "run",
        "Seq Renamed",
        "local",
        "--var",
        "name=Contract",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&run);
    let run_json = stdout_json(&run);
    assert_object_keys(
        &run_json,
        &[
            "sequence",
            "runner",
            "turn_count",
            "completed_turns",
            "success",
            "turns",
        ],
    );
    assert_object_keys(&run_json["sequence"], &["id", "name", "path"]);
    assert_object_keys(&run_json["runner"], &["mode", "name"]);
    assert_object_keys(
        first_array_item(&run_json, "turns"),
        &[
            "index",
            "fragment",
            "command",
            "pid",
            "termination",
            "exit_code",
            "stdout",
            "stderr",
            "stdout_bytes",
            "stderr_bytes",
            "stdout_truncated",
            "stderr_truncated",
        ],
    );
    assert_object_keys(
        &first_array_item(&run_json, "turns")["fragment"],
        &["id", "name", "path"],
    );

    let looped_run = pseq(&[
        "run",
        "Seq Renamed",
        "local",
        "--var",
        "name=Contract",
        "--iterations",
        "2",
        "--feedback-from",
        "final-stdout",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&looped_run);
    let looped_run_json = stdout_json(&looped_run);
    assert_object_keys(
        &looped_run_json,
        &[
            "sequence",
            "runner",
            "iterations",
            "turn_count",
            "completed_turns",
            "success",
            "turns",
        ],
    );
    assert_object_keys(
        first_array_item(&looped_run_json, "turns"),
        &[
            "iteration",
            "index",
            "fragment",
            "command",
            "pid",
            "termination",
            "exit_code",
            "stdout",
            "stderr",
            "stdout_bytes",
            "stderr_bytes",
            "stdout_truncated",
            "stderr_truncated",
        ],
    );
}

pub(super) fn assert_saved_render_payload(store: &TestStore) {
    let out_path = store.path().join("manual-output.md");
    let rendered_saved = pseq(&[
        "render",
        "Seq Renamed",
        "--var",
        "name=Contract",
        "--save",
        "--out",
        path_str(&out_path),
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&rendered_saved);
    let rendered_saved_json = stdout_json(&rendered_saved);
    assert_object_keys(
        &rendered_saved_json,
        &[
            "id",
            "name",
            "path",
            "text",
            "annotated",
            "out_path",
            "out_git_commit",
            "saved_render",
        ],
    );
    assert_object_keys(
        &rendered_saved_json["saved_render"],
        &["id", "path", "git_commit"],
    );
}
