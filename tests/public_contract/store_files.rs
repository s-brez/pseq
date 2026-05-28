use super::*;

#[test]
fn store_file_schemas_are_public_contract() {
    let store = TestStore::initialized("schema-contract");
    assert_eq!(
        fs::read_to_string(store.path().join("config.toml")).unwrap(),
        "version = 1\n"
    );

    let fragment = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Schema Fragment",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "schema body\n",
    );
    assert_success(&fragment);
    let fragment_json = stdout_json(&fragment);
    let fragment_id = fragment_json["id"].as_str().unwrap();
    let fragment_path = fragment_json["path"].as_str().unwrap();
    let fragment_content = fs::read_to_string(store.path().join(fragment_path)).unwrap();
    let (fragment_frontmatter, fragment_body) = split_frontmatter(&fragment_content);
    let fragment_metadata: pseq::yaml::Value = pseq::yaml::from_str(fragment_frontmatter).unwrap();
    assert_eq!(fragment_metadata["id"], fragment_id);
    assert_eq!(fragment_metadata["name"], "Schema Fragment");
    assert_eq!(fragment_body, "schema body\n");

    let sequence = pseq(&[
        "sequence",
        "new",
        "Schema Seq",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence);
    let sequence_json = stdout_json(&sequence);
    let sequence_id = sequence_json["id"].as_str().unwrap().to_owned();
    let sequence_path = sequence_json["path"].as_str().unwrap().to_owned();
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Schema Seq",
        fragment_id,
        "--store",
        path_str(store.path()),
    ]));
    let sequence_file: Value =
        serde_json::from_str(&fs::read_to_string(store.path().join(&sequence_path)).unwrap())
            .unwrap();
    assert_object_keys(&sequence_file, &["id", "name", "fragments"]);
    assert_eq!(sequence_file["id"], sequence_id);
    assert_eq!(sequence_file["name"], "Schema Seq");
    assert_eq!(sequence_file["fragments"][0], fragment_id);
    assert!(
        !fs::read_to_string(store.path().join(&sequence_path))
            .unwrap()
            .contains("schema body"),
        "sequence files must reference fragments, not duplicate bodies"
    );

    let capture = pseq_with_stdin(
        &[
            "capture",
            "import",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "captured schema prompt\n",
    );
    assert_success(&capture);
    let capture_json = stdout_json(&capture);
    let capture_path = capture_json["path"].as_str().unwrap();
    let capture_file: Value =
        serde_json::from_str(&fs::read_to_string(store.path().join(capture_path)).unwrap())
            .unwrap();
    assert_object_keys(
        &capture_file,
        &["version", "id", "created_unix_seconds", "origin", "events"],
    );
    assert_eq!(capture_file["version"], 1);
    assert_eq!(capture_file["origin"]["kind"], "stdin");
    assert_eq!(capture_file["events"][0]["index"], 1);
    assert_eq!(capture_file["events"][0]["kind"], "user_prompt");
    assert_eq!(
        capture_file["events"][0]["text"],
        "captured schema prompt\n"
    );

    let saved_render = pseq(&[
        "render",
        "Schema Seq",
        "--save",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&saved_render);
    let saved_render_json = stdout_json(&saved_render);
    let saved_render_path = saved_render_json["saved_render"]["path"].as_str().unwrap();
    let saved_render_content = fs::read_to_string(store.path().join(saved_render_path)).unwrap();
    let (render_frontmatter, render_body) = split_frontmatter(&saved_render_content);
    let render_metadata: pseq::yaml::Value = pseq::yaml::from_str(render_frontmatter).unwrap();
    assert!(render_metadata["id"].as_str().unwrap().starts_with("rnd_"));
    assert_eq!(render_metadata["sequence_id"], sequence_id);
    assert_eq!(render_metadata["sequence_name"], "Schema Seq");
    assert_eq!(render_metadata["sequence_path"], sequence_path);
    assert_eq!(render_metadata["annotated"], false);
    assert_eq!(render_body, "schema body\n");

    assert_git_clean(store.path());
}
