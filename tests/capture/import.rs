use super::*;

#[test]
fn capture_import_stdin_list_show_and_status_preserve_prompt_text() {
    let store = TestStore::initialized("capture-stdin");

    let prompt = "first line\nsecond line\n";
    let imported = pseq_with_stdin(
        &[
            "capture",
            "import",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        prompt,
    );
    assert_success(&imported);
    let imported_json = stdout_json(&imported);
    let capture_id = imported_json["id"].as_str().unwrap();
    assert!(capture_id.starts_with("cap_"));
    assert_eq!(imported_json["origin"]["kind"], "stdin");
    assert_eq!(imported_json["event_count"], 1);
    assert!(imported_json["git_commit"].is_string());

    let capture_path = store.path().join(imported_json["path"].as_str().unwrap());
    let capture_file: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(capture_path).unwrap()).unwrap();
    assert_eq!(capture_file["version"], 1);
    assert_eq!(capture_file["events"][0]["kind"], "user_prompt");
    assert_eq!(capture_file["events"][0]["text"], prompt);

    let list = pseq(&[
        "capture",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    let list_json = stdout_json(&list);
    assert_eq!(list_json["captures"][0]["id"], capture_id);
    assert_eq!(list_json["captures"][0]["event_count"], 1);

    let show = pseq(&[
        "capture",
        "show",
        capture_id,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["events"][0]["text"], prompt);

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    assert_eq!(stdout_json(&status)["counts"]["captures"], 1);
    assert_git_clean(store.path());
}

#[test]
fn capture_import_no_commit_leaves_capture_uncommitted() {
    let store = TestStore::initialized("capture-no-commit");

    let imported = pseq_with_stdin(
        &[
            "capture",
            "import",
            "--stdin",
            "--no-commit",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "secret prompt\n",
    );
    assert_success(&imported);
    let json = stdout_json(&imported);
    assert!(json.get("git_commit").is_none());
    assert!(store.path().join(json["path"].as_str().unwrap()).is_file());

    let status = git_status(store.path());
    assert!(status.contains("?? captures/"), "{status}");
}

#[test]
fn capture_stdin_source_selects_piped_prompt_text() {
    let store = TestStore::initialized("capture-stdin-source");

    let prompt = "source prompt\nsecond line\n";
    let captured = pseq_with_stdin(
        &[
            "capture",
            "last",
            "--source",
            "stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        prompt,
    );
    assert_success(&captured);
    let captured_json = stdout_json(&captured);
    assert_eq!(captured_json["origin"]["kind"], "source");
    assert_eq!(captured_json["origin"]["source"], "stdin");
    assert_eq!(captured_json["origin"]["session"]["path"], "<stdin>");
    assert_eq!(captured_json["event_count"], 1);

    let show = pseq(&[
        "capture",
        "show",
        captured_json["id"].as_str().unwrap(),
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    assert_eq!(stdout_json(&show)["events"][0]["text"], prompt);

    let default_source = pseq_with_stdin(
        &[
            "capture",
            "last",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "default source prompt\n",
    );
    assert_success(&default_source);
    let default_source_json = stdout_json(&default_source);
    assert_eq!(default_source_json["origin"]["kind"], "source");
    assert_eq!(default_source_json["origin"]["source"], "stdin");
    assert_eq!(default_source_json["event_count"], 1);

    let ranged = pseq_with_stdin(
        &[
            "capture",
            "range",
            "-1..-1",
            "--source",
            "stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "range prompt\n",
    );
    assert_success(&ranged);
    assert_eq!(stdout_json(&ranged)["event_count"], 1);

    let default_source_range = pseq_with_stdin(
        &[
            "capture",
            "range",
            "-1..-1",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "default range prompt\n",
    );
    assert_success(&default_source_range);
    let default_source_range_json = stdout_json(&default_source_range);
    assert_eq!(default_source_range_json["origin"]["kind"], "source");
    assert_eq!(default_source_range_json["origin"]["source"], "stdin");
    assert_eq!(default_source_range_json["event_count"], 1);
    assert_git_clean(store.path());
}

#[test]
fn capture_stdin_source_rejects_empty_input_before_mutation() {
    let store = TestStore::initialized("capture-empty-stdin-source");

    let captured = pseq_with_stdin(
        &[
            "capture",
            "last",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "",
    );
    assert_eq!(captured.status.code(), Some(1));
    assert!(captured.stdout.is_empty());
    assert_eq!(
        stderr_json(&captured)["error"]["code"],
        "capture_source_unavailable"
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    assert_eq!(stdout_json(&status)["counts"]["captures"], 0);
    assert_git_clean(store.path());
}

#[test]
fn capture_import_file_can_be_shown_by_store_relative_path() {
    let store = TestStore::initialized("capture-file");
    let input_path = store.path().with_extension("prompt.txt");
    fs::write(&input_path, "from file\n").unwrap();

    let imported = pseq(&[
        "capture",
        "import",
        "--file",
        path_str(&input_path),
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    let _ = fs::remove_file(&input_path);
    assert_success(&imported);
    let imported_json = stdout_json(&imported);
    assert_eq!(imported_json["origin"]["kind"], "file");
    assert_eq!(imported_json["event_count"], 1);

    let capture_path = imported_json["path"].as_str().unwrap();
    let capture_file_name = capture_path.strip_prefix("captures/").unwrap();
    let show = pseq(&[
        "cap",
        "show",
        capture_file_name,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    assert_eq!(stdout_json(&show)["events"][0]["text"], "from file\n");
    assert_git_clean(store.path());
}
