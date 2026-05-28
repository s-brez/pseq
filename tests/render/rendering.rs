use super::*;

#[test]
fn render_outputs_sequence_fragment_bodies_in_order() {
    let store = TestStore::initialized("render");

    let first_id = create_fragment(&store, "First", "A\n")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    create_fragment(&store, "Second", "B\n");

    let sequence_json = create_sequence(&store, "Combo");
    let sequence_id = sequence_json["id"].as_str().unwrap();

    add_fragment_to_sequence(&store, "Combo", &first_id);
    add_fragment_to_sequence(&store, "Combo", "Second");

    let rendered = pseq(&["render", "Combo", "--store", path_str(store.path())]);
    assert_success(&rendered);
    assert_eq!(String::from_utf8(rendered.stdout).unwrap(), "A\nB\n");
    assert!(rendered.stderr.is_empty());

    let rendered_json = pseq(&[
        "render",
        "sequences/combo.json",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&rendered_json);
    let json = stdout_json(&rendered_json);
    assert_eq!(json["id"], sequence_id);
    assert_eq!(json["name"], "Combo");
    assert_eq!(json["path"], "sequences/combo.json");
    assert_eq!(json["text"], "A\nB\n");

    assert_git_clean(store.path());
}

#[test]
fn render_annotations_mark_sequence_fragments_not_transitive_inclusions() {
    let store = TestStore::initialized("render-include-annotate");

    create_fragment(&store, "Included", "included\n");
    create_single_fragment_sequence(
        &store,
        "Wrapper",
        "Wrapper",
        "before\n{{pseq.fragment.Included}}after\n",
    );

    let rendered = pseq(&[
        "render",
        "Wrapper",
        "--store",
        path_str(store.path()),
        "--annotate",
    ]);
    assert_success(&rendered);
    let text = String::from_utf8(rendered.stdout).unwrap();
    assert!(text.contains("<!-- pseq fragment 1 begin: Wrapper"));
    assert!(text.contains("before\nincluded\nafter\n"));
    assert_eq!(text.matches(" begin: ").count(), 1);
    assert!(!text.contains("begin: Included"));
    assert_git_clean(store.path());
}
