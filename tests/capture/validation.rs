use super::*;

#[test]
fn doctor_rejects_capture_files_that_do_not_match_schema() {
    let store = TestStore::initialized("capture-schema");

    fs::write(
        store.path().join("captures/empty-events.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000001",
  "created_unix_seconds": 1,
  "origin": { "kind": "stdin" },
  "events": []
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("captures/missing-origin.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000002",
  "created_unix_seconds": 1,
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "hello" }
  ]
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("captures/unknown-field.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000003",
  "created_unix_seconds": 1,
  "origin": { "kind": "stdin" },
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "hello" }
  ],
  "unexpected": true
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("captures/bad-id.json"),
        r#"{
  "version": 1,
  "id": "not_a_capture_id",
  "created_unix_seconds": 1,
  "origin": { "kind": "stdin" },
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "hello" }
  ]
}
"#,
    )
    .unwrap();
    fs::write(
        store.path().join("captures/unsupported-source.json"),
        r#"{
  "version": 1,
  "id": "cap_00000000000000000000000000000004",
  "created_unix_seconds": 1,
  "origin": {
    "kind": "source",
    "source": "not-supported",
    "session": { "path": "/tmp/session.jsonl" }
  },
  "events": [
    { "index": 1, "kind": "user_prompt", "text": "hello" }
  ]
}
"#,
    )
    .unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    assert!(doctor.stderr.is_empty());
    let json = stdout_json(&doctor);
    assert_eq!(json["valid"], false);
    let issues = json["issues"].as_array().unwrap();
    let codes = issue_codes(&json);
    assert!(codes.contains(&"capture_file_invalid"));
    assert!(
        issues
            .iter()
            .any(|issue| issue["message"].as_str().unwrap().contains("unsupported"))
    );
}
