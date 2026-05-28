use super::*;

#[test]
fn render_expands_inline_fragment_inclusions() {
    let store = TestStore::initialized("render-includes");

    let reusable = create_fragment(&store, "Reusable Block", "included {{topic}}\n");
    let reusable_id = reusable["id"].as_str().unwrap();
    let reusable_short_id = &reusable_id[..12];
    let reusable_path = reusable["path"].as_str().unwrap();

    create_fragment(
        &store,
        "Nested",
        &format!("nested {{{{pseq.fragment.{reusable_path}}}}}"),
    );
    create_single_fragment_sequence(
        &store,
        "Wrapper",
        "Wrapper",
        &format!(
            "before\n{{{{pseq.fragment.Nested}}}}again {{{{pseq.fragment.{reusable_short_id}}}}}after\n"
        ),
    );

    let rendered = pseq(&[
        "render",
        "Wrapper",
        "--store",
        path_str(store.path()),
        "--var",
        "topic=docs",
    ]);
    assert_success(&rendered);
    assert_eq!(
        String::from_utf8(rendered.stdout).unwrap(),
        "before\nnested included docs\nagain included docs\nafter\n"
    );

    assert_git_clean(store.path());
}

#[test]
fn render_fails_closed_for_invalid_fragment_inclusions() {
    let store = TestStore::initialized("render-include-missing");
    create_single_fragment_sequence(
        &store,
        "Missing",
        "Missing",
        "{{pseq.fragment.Does Not Exist}}",
    );

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_success(&doctor);

    assert_render_json_error(&store, "Missing", "fragment_not_found");

    create_single_fragment_sequence(&store, "Invalid", "Invalid", "{{pseq.fragment.}}");
    assert_render_json_error(&store, "Invalid", "invalid_fragment_include");

    create_fragment(&store, "Duplicate", "first\n");
    create_fragment(&store, "Duplicate", "second\n");
    create_single_fragment_sequence(
        &store,
        "Ambiguous",
        "Ambiguous",
        "{{pseq.fragment.Duplicate}}",
    );
    assert_render_json_error(&store, "Ambiguous", "fragment_reference_ambiguous");

    create_fragment(&store, "Cycle A", "{{pseq.fragment.Cycle B}}");
    create_fragment(&store, "Cycle B", "{{pseq.fragment.Cycle A}}");
    create_single_fragment_sequence(&store, "Cycle", "Cycle", "{{pseq.fragment.Cycle A}}");
    assert_render_json_error(&store, "Cycle", "fragment_include_cycle");
}

#[test]
fn render_fails_when_sequence_references_missing_fragment() {
    let store = TestStore::initialized("render-missing-fragment");
    fs::write(
        store.path().join("sequences/broken.json"),
        r#"{
  "id": "seq_00000000000000000000000000000001",
  "name": "Broken",
  "fragments": ["frg_00000000000000000000000000000001"]
}
"#,
    )
    .unwrap();

    assert_render_json_error(&store, "Broken", "invalid_store");
}
