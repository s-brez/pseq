use super::*;

#[test]
fn render_out_annotate_and_save_write_requested_records() {
    let store = TestStore::initialized("render-output-options");
    create_single_fragment_sequence(&store, "Combo", "First", "A {{name}}\n");
    create_fragment(&store, "Second", "B");
    add_fragment_to_sequence(&store, "Combo", "Second");

    let out_path = store.path().with_extension("rendered.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--var",
        "name=Sam",
        "--annotate",
        "--out",
        path_str(&out_path),
        "--save",
        "--json",
    ]);
    assert_success(&rendered);
    let json = stdout_json(&rendered);
    let text = json["text"].as_str().unwrap();
    assert_eq!(json["annotated"], true);
    assert_eq!(json["out_path"], path_str(&out_path));
    assert!(text.contains("<!-- pseq fragment 1 begin: First"));
    assert!(text.contains("A Sam\n"));
    assert!(text.contains("<!-- pseq fragment 2 begin: Second"));
    assert!(text.ends_with("<!-- pseq fragment 2 end -->\n"));
    assert_eq!(fs::read_to_string(&out_path).unwrap(), text);

    let saved = &json["saved_render"];
    assert!(saved["id"].as_str().unwrap().starts_with("rnd_"));
    assert_eq!(saved["path"], "renders/combo.md");
    assert!(saved["git_commit"].is_string());
    let saved_content = fs::read_to_string(store.path().join("renders/combo.md")).unwrap();
    assert!(saved_content.starts_with("---\n"));
    assert!(saved_content.contains("sequence_name: Combo\n"));
    assert!(saved_content.contains("annotated: true\n"));
    assert!(saved_content.ends_with(text));

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    assert_eq!(stdout_json(&status)["counts"]["renders"], 1);

    let _ = fs::remove_file(&out_path);
    assert_git_clean(store.path());
}

#[test]
fn render_out_suppresses_non_json_stdout() {
    let store = TestStore::initialized("render-out-non-json");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");

    let out_path = store.path().with_extension("out.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
    ]);
    assert_success(&rendered);
    assert!(rendered.stdout.is_empty());
    assert!(rendered.stderr.is_empty());
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "body\n");

    let _ = fs::remove_file(&out_path);
    assert_git_clean(store.path());
}

#[test]
fn render_outside_store_does_not_create_store_history() {
    let store = TestStore::initialized("render-outside-store");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");
    let before_head = git_head(store.path());

    let out_path = store.path().with_extension("outside.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
        "--json",
    ]);
    assert_success(&rendered);
    let json = stdout_json(&rendered);
    assert_eq!(json["out_path"], path_str(&out_path));
    assert!(json["out_git_commit"].is_null());
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "body\n");
    assert_eq!(git_head(store.path()), before_head);

    let _ = fs::remove_file(&out_path);
    assert_git_clean(store.path());
}

#[test]
fn render_out_inside_store_versions_written_file() {
    let store = TestStore::initialized("render-out-versioned");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");

    let out_path = store.path().join("rendered.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
        "--json",
    ]);
    assert_success(&rendered);
    let json = stdout_json(&rendered);
    assert_eq!(json["out_path"], path_str(&out_path));
    assert!(json["out_git_commit"].is_string());
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "body\n");
    assert_git_clean(store.path());
}

#[test]
fn render_save_and_out_inside_store_are_versioned_together() {
    let store = TestStore::initialized("render-save-out-versioned");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");

    let out_path = store.path().join("rendered.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
        "--save",
        "--json",
    ]);
    assert_success(&rendered);
    let json = stdout_json(&rendered);
    let out_commit = json["out_git_commit"].as_str().unwrap();
    let saved_commit = json["saved_render"]["git_commit"].as_str().unwrap();
    assert_eq!(out_commit, saved_commit);
    assert_eq!(fs::read_to_string(&out_path).unwrap(), "body\n");
    assert!(store.path().join("renders/combo.md").is_file());
    assert_git_clean(store.path());
}

#[test]
fn render_save_and_out_same_destination_fail_before_writing() {
    let store = TestStore::initialized("render-save-out-collision");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");

    let out_path = store.path().join("renders/combo.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
        "--save",
        "--path",
        "renders/combo",
        "--json",
    ]);
    assert_eq!(rendered.status.code(), Some(1));
    assert!(rendered.stdout.is_empty());
    assert_eq!(
        stderr_json(&rendered)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(!out_path.exists());
    assert_git_clean(store.path());
}

#[test]
fn render_save_and_out_case_variant_same_destination_fail_before_writing() {
    let base = std::env::current_dir().unwrap().join("target");
    fs::create_dir_all(&base).unwrap();
    if !directory_is_case_insensitive(&base) {
        return;
    }

    let store = TestStore::initialized_under(&base, "render-save-out-case-collision");
    create_single_fragment_sequence(&store, "Combo", "Only", "body\n");

    let out_path = store.path().join("renders/combo.md");
    let rendered = pseq(&[
        "render",
        "Combo",
        "--store",
        path_str(store.path()),
        "--out",
        path_str(&out_path),
        "--save",
        "--path",
        "renders/Combo",
        "--json",
    ]);
    assert_eq!(rendered.status.code(), Some(1));
    assert!(rendered.stdout.is_empty());
    assert_eq!(
        stderr_json(&rendered)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(!out_path.exists());
    assert_git_clean(store.path());
}

fn directory_is_case_insensitive(directory: &std::path::Path) -> bool {
    let lower_name = format!(".pseq-test-case-probe-{}", std::process::id());
    let upper_name = lower_name.to_ascii_uppercase();
    let lower_path = directory.join(&lower_name);
    let upper_path = directory.join(&upper_name);
    if lower_path.exists() || upper_path.exists() {
        return false;
    }

    let Ok(file) = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lower_path)
    else {
        return false;
    };
    drop(file);

    let case_insensitive = upper_path.exists();
    let _ = fs::remove_file(&lower_path);
    case_insensitive
}
