use super::*;

#[test]
fn capture_promote_creates_fragment_sequence_and_renderable_prompt() {
    let store = TestStore::initialized("capture-promote");

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
    let capture_id = stdout_json(&imported)["id"].as_str().unwrap().to_owned();

    let promoted = pseq(&[
        "capture",
        "promote",
        &capture_id,
        "--as-sequence",
        "Promoted Prompt",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&promoted);
    let promoted_json = stdout_json(&promoted);
    assert_eq!(promoted_json["capture"]["id"], capture_id);
    assert_eq!(promoted_json["sequence"]["name"], "Promoted Prompt");
    assert_eq!(promoted_json["sequence"]["fragment_count"], 1);
    assert_eq!(promoted_json["fragments"][0]["name"], "Promoted Prompt");
    assert_eq!(promoted_json["event_count"], 1);
    assert!(promoted_json["git_commit"].is_string());

    let fragment_id = promoted_json["fragments"][0]["id"].as_str().unwrap();
    let fragment = pseq(&[
        "fragment",
        "show",
        fragment_id,
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&fragment);
    assert_eq!(String::from_utf8(fragment.stdout).unwrap(), prompt);

    let sequence = pseq(&[
        "sequence",
        "show",
        "Promoted Prompt",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence);
    let sequence_json = stdout_json(&sequence);
    assert_eq!(sequence_json["fragments"][0]["id"], fragment_id);

    let rendered = pseq(&[
        "render",
        "Promoted Prompt",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&rendered);
    assert_eq!(String::from_utf8(rendered.stdout).unwrap(), prompt);
    assert_git_clean(store.path());
}

#[test]
fn capture_promote_preserves_multi_event_order() {
    let store = TestStore::initialized("capture-promote-multi");
    fs::write(
        store.path().join("captures/multi.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000004",
  "created_unix_seconds": 1,
  "origin": { "kind": "stdin" },
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "first\n" },
    { "index": 2, "kind": "user_prompt", "text": "second" }
  ]
}
"#,
    )
    .unwrap();

    let promoted = pseq(&[
        "capture",
        "promote",
        "multi.json",
        "--as-sequence",
        "Multi Capture",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&promoted);
    let promoted_json = stdout_json(&promoted);
    assert_eq!(promoted_json["sequence"]["fragment_count"], 2);
    assert_eq!(promoted_json["fragments"][0]["name"], "Multi Capture 1");
    assert_eq!(promoted_json["fragments"][1]["name"], "Multi Capture 2");

    let rendered = pseq(&["render", "Multi Capture", "--store", path_str(store.path())]);
    assert_success(&rendered);
    assert_eq!(String::from_utf8(rendered.stdout).unwrap(), "first\nsecond");
    assert_git_clean(store.path());
}

#[test]
fn capture_promote_fails_before_mutation_when_capture_ref_is_ambiguous() {
    let store = TestStore::initialized("capture-promote-ambiguous");
    for prompt in ["one", "two"] {
        assert_success(&pseq_with_stdin(
            &[
                "capture",
                "import",
                "--stdin",
                "--store",
                path_str(store.path()),
            ],
            prompt,
        ));
    }

    let promoted = pseq(&[
        "capture",
        "promote",
        "cap_",
        "--as-sequence",
        "Should Not Exist",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(promoted.status.code(), Some(1));
    assert!(promoted.stdout.is_empty());
    assert_eq!(
        stderr_json(&promoted)["error"]["code"],
        "capture_reference_ambiguous"
    );

    let sequences = pseq(&[
        "sequence",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequences);
    assert_eq!(
        stdout_json(&sequences)["sequences"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_git_clean(store.path());
}
