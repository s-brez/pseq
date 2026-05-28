use super::*;

#[test]
fn render_substitutes_variables_from_direct_values_files_and_vars_file() {
    let store = TestStore::initialized("render-vars");
    create_single_fragment_sequence(
        &store,
        "Greeting",
        "Template",
        "Hello {{name}}\n{{task}}\n{{note}}",
    );

    let vars_path = store.path().with_extension("vars.toml");
    fs::write(
        &vars_path,
        r#"name = "from vars file"
task = "Review this patch."
note = "from vars file"
"#,
    )
    .unwrap();
    let note_path = store.path().with_extension("note.txt");
    fs::write(&note_path, "from file\n").unwrap();
    let note_assignment = format!("note=@{}", path_str(&note_path));

    let rendered = pseq(&[
        "render",
        "Greeting",
        "--store",
        path_str(store.path()),
        "--vars",
        path_str(&vars_path),
        "--var",
        "name=Sam",
        "--var",
        note_assignment.as_str(),
        "--json",
    ]);
    let _ = fs::remove_file(&vars_path);
    let _ = fs::remove_file(&note_path);

    assert_success(&rendered);
    let json = stdout_json(&rendered);
    assert_eq!(json["text"], "Hello Sam\nReview this patch.\nfrom file\n");
    assert_git_clean(store.path());
}

#[test]
fn render_fails_when_required_variable_is_missing() {
    let store = TestStore::initialized("render-missing-var");
    create_single_fragment_sequence(&store, "Greeting", "Template", "Hello {{name}}\n");

    assert_render_json_error(&store, "Greeting", "missing_variable");
}

#[test]
fn render_rejects_expression_like_variable_placeholders() {
    let store = TestStore::initialized("render-invalid-placeholder");
    create_single_fragment_sequence(&store, "Greeting", "Template", "Hello {{name | upper}}\n");

    let rendered = pseq(&[
        "render",
        "Greeting",
        "--store",
        path_str(store.path()),
        "--var",
        "name=Sam",
        "--json",
    ]);
    assert_json_error(&rendered, "invalid_variable_placeholder");
}

#[test]
fn render_rejects_invalid_variable_inputs() {
    let store = TestStore::initialized("render-invalid-vars");
    create_single_fragment_sequence(&store, "Greeting", "Template", "Hello {{name}}\n");

    let invalid_assignment = pseq(&[
        "render",
        "Greeting",
        "--store",
        path_str(store.path()),
        "--var",
        "name",
        "--json",
    ]);
    assert_json_error(&invalid_assignment, "invalid_variable_assignment");

    let invalid_name = pseq(&[
        "render",
        "Greeting",
        "--store",
        path_str(store.path()),
        "--var",
        "1name=Sam",
        "--json",
    ]);
    assert_json_error(&invalid_name, "invalid_variable_name");
}
