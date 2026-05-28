use super::*;

#[test]
fn render_at_expands_included_fragments_from_requested_history() {
    let store = TestStore::initialized("render-include-at");

    let child = create_fragment(&store, "Child", "old\n");
    create_single_fragment_sequence(&store, "Wrapper", "Wrapper", "{{pseq.fragment.Child}}");
    let old_head = git_head(store.path());

    let child_path = store.path().join(child["path"].as_str().unwrap());
    let old_child = fs::read_to_string(&child_path).unwrap();
    fs::write(&child_path, old_child.replace("old\n", "new\n")).unwrap();
    git_commit_all(store.path(), "Update child body");

    let current = pseq(&["render", "Wrapper", "--store", path_str(store.path())]);
    assert_success(&current);
    assert_eq!(String::from_utf8(current.stdout).unwrap(), "new\n");

    let historical = pseq(&[
        "render",
        "Wrapper",
        "--store",
        path_str(store.path()),
        "--at",
        &old_head,
        "--json",
    ]);
    assert_success(&historical);
    let json = stdout_json(&historical);
    assert_eq!(json["text"], "old\n");
    assert_eq!(json["history_ref"], old_head);
    assert_git_clean(store.path());
}

#[test]
fn render_at_uses_requested_store_history_ref() {
    let store = TestStore::initialized("render-at");
    create_single_fragment_sequence(&store, "History", "Old", "old\n");
    let old_head = git_head(store.path());

    create_fragment(&store, "New", "new\n");
    add_fragment_to_sequence(&store, "History", "New");

    let current = pseq(&["render", "History", "--store", path_str(store.path())]);
    assert_success(&current);
    assert_eq!(String::from_utf8(current.stdout).unwrap(), "old\nnew\n");

    let historical = pseq(&[
        "render",
        "History",
        "--store",
        path_str(store.path()),
        "--at",
        &old_head,
        "--json",
    ]);
    assert_success(&historical);
    let json = stdout_json(&historical);
    assert_eq!(json["text"], "old\n");
    assert_eq!(json["history_ref"], old_head);
    assert_git_clean(store.path());
}

#[test]
fn render_at_reads_history_even_when_current_catalog_is_invalid() {
    let store = TestStore::initialized("render-at-current-invalid");
    create_single_fragment_sequence(&store, "History", "Old", "old\n");
    let old_head = git_head(store.path());

    fs::write(
        store.path().join("sequences/history.json"),
        r#"{
  "id": "seq_00000000000000000000000000000002",
  "name": "History",
  "fragments": ["frg_00000000000000000000000000000003"]
}
"#,
    )
    .unwrap();
    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(
        stdout_json(&doctor)["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "sequence_fragment_missing")
    );

    let before_status = git_status(store.path());
    let historical = pseq(&[
        "render",
        "History",
        "--store",
        path_str(store.path()),
        "--at",
        &old_head,
        "--json",
    ]);
    assert_success(&historical);
    let json = stdout_json(&historical);
    assert_eq!(json["text"], "old\n");
    assert_eq!(json["history_ref"], old_head);
    assert_eq!(git_status(store.path()), before_status);
}
