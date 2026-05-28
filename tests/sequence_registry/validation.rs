use super::*;

#[test]
fn ambiguous_sequence_name_and_short_id_fail_closed() {
    let store = TestStore::initialized("sequence-ambiguity");
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Repeated",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Repeated",
        "--store",
        path_str(store.path()),
    ]));

    let by_name = pseq(&[
        "sequence",
        "show",
        "Repeated",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(by_name.status.code(), Some(1));
    assert!(by_name.stdout.is_empty());
    assert_eq!(
        stderr_json(&by_name)["error"]["code"],
        "sequence_reference_ambiguous"
    );

    let by_short_id = pseq(&[
        "sequence",
        "show",
        "seq_",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(by_short_id.status.code(), Some(1));
    assert!(by_short_id.stdout.is_empty());
    assert_eq!(
        stderr_json(&by_short_id)["error"]["code"],
        "sequence_reference_ambiguous"
    );
}

#[test]
fn doctor_detects_invalid_sequences_and_bad_fragment_references() {
    let store = TestStore::initialized("sequence-doctor");

    let fragment = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Original",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "body",
    );
    assert_success(&fragment);
    let fragment_id = stdout_json(&fragment)["id"].as_str().unwrap().to_owned();

    assert_success(&pseq(&[
        "sequence",
        "new",
        "Original Sequence",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Original Sequence",
        "Original",
        "--store",
        path_str(store.path()),
    ]));

    fs::copy(
        store.path().join("fragments/original.md"),
        store.path().join("fragments/duplicate.md"),
    )
    .unwrap();
    fs::copy(
        store.path().join("sequences/original-sequence.json"),
        store.path().join("sequences/duplicate.json"),
    )
    .unwrap();
    fs::write(store.path().join("sequences/broken.json"), "not json").unwrap();
    fs::write(
        store.path().join("sequences/bad-id.json"),
        r#"{
  "id": "not_a_sequence_id",
  "name": "Bad ID",
  "fragments": []
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("sequences/missing-fragment.json"),
        r#"{
  "id": "seq_00000000000000000000000000000001",
  "name": "Missing Fragment",
  "fragments": ["frg_00000000000000000000000000000002"]
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("sequences/invalid-fragment-ref.json"),
        r#"{
  "id": "seq_00000000000000000000000000000002",
  "name": "Invalid Fragment Reference",
  "fragments": ["not_a_fragment_id"]
}
"#,
    )
    .unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(doctor.stderr.is_empty());

    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], false);
    let codes = issue_codes(&json);
    assert!(codes.contains(&"fragment_id_duplicate"));
    assert!(codes.contains(&"sequence_file_invalid"));
    assert!(codes.contains(&"sequence_id_invalid"));
    assert!(codes.contains(&"sequence_id_duplicate"));
    assert!(codes.contains(&"sequence_fragment_missing"));
    assert!(codes.contains(&"sequence_fragment_ref_invalid"));
    assert!(codes.contains(&"sequence_fragment_ref_ambiguous"));
    assert!(
        json["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["message"].as_str().unwrap().contains(&fragment_id))
    );

    let list = pseq(&[
        "sequence",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(list.status.code(), Some(1));
    assert!(list.stdout.is_empty());
    assert_eq!(stderr_json(&list)["error"]["code"], "invalid_store");
}

#[test]
fn doctor_rejects_sequence_files_with_unsupported_fields() {
    let store = TestStore::initialized("sequence-unsupported-fields");
    fs::write(
        store.path().join("sequences/tool-call.json"),
        r#"{
  "id": "seq_tool",
  "name": "Tool Call",
  "fragments": [],
  "tools": []
}
"#,
    )
    .unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(doctor.stderr.is_empty());
    let json = stdout_json(&doctor);
    assert!(
        json["issues"].as_array().unwrap().iter().any(|issue| {
            issue["code"] == "sequence_file_invalid"
                && issue["message"].as_str().unwrap().contains("unknown field")
        }),
        "expected sequence_file_invalid unknown field issue, got {json}"
    );
}
