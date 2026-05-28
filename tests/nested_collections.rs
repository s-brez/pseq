#[path = "common/mod.rs"]
mod common;

use std::fs;

#[cfg(unix)]
use common::issue_codes;
use common::{
    TestStore, assert_git_clean, assert_success, git_head, path_str, pseq, pseq_bin,
    pseq_with_stdin, stderr_json, stdout_json,
};

#[test]
fn nested_fragments_and_sequences_resolve_by_paths_and_folded_aliases() {
    let store = TestStore::initialized("nested-fragments-sequences");

    let nested = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Nested DCQ",
            "--stdin",
            "--path",
            "omega/dcq-productization-review",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "nested body\n",
    );
    assert_success(&nested);
    let nested_json = stdout_json(&nested);
    let nested_id = nested_json["id"].as_str().unwrap();
    assert_eq!(
        nested_json["path"],
        "fragments/omega/dcq-productization-review.md"
    );

    let flat = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Flat DCQ",
            "--stdin",
            "--path",
            "omega-dcq-productization-review",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "flat body\n",
    );
    assert_success(&flat);
    assert_eq!(
        stdout_json(&flat)["path"],
        "fragments/omega-dcq-productization-review.md"
    );

    let show_nested = pseq(&[
        "fragment",
        "show",
        "omega/dcq-productization-review",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&show_nested);
    assert_eq!(
        String::from_utf8(show_nested.stdout).unwrap(),
        "nested body\n"
    );

    let show_flat = pseq(&[
        "fragment",
        "show",
        "omega-dcq-productization-review.md",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&show_flat);
    assert_eq!(String::from_utf8(show_flat.stdout).unwrap(), "flat body\n");

    let ambiguous = pseq(&[
        "fragment",
        "show",
        "omega-dcq-productization-review",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(ambiguous.status.code(), Some(1));
    assert_eq!(
        stderr_json(&ambiguous)["error"]["code"],
        "fragment_reference_ambiguous"
    );

    let sequence = pseq(&[
        "sequence",
        "new",
        "DCQ Productization Review",
        "--dir",
        "sequences/omega",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence);
    assert_eq!(
        stdout_json(&sequence)["path"],
        "sequences/omega/dcq-productization-review.json"
    );

    assert_success(&pseq(&[
        "sequence",
        "add",
        "omega/dcq-productization-review",
        nested_id,
        "--store",
        path_str(store.path()),
    ]));

    let render = pseq(&[
        "render",
        "omega-dcq-productization-review",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&render);
    assert_eq!(String::from_utf8(render.stdout).unwrap(), "nested body\n");

    let run_sink = TestStore::initialized("nested-run-sink");
    let run = pseq(&[
        "run",
        "omega-dcq-productization-review",
        "--store",
        path_str(store.path()),
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(run_sink.path()),
    ]);
    assert_success(&run);
    let run_captures = pseq(&[
        "capture",
        "list",
        "--store",
        path_str(run_sink.path()),
        "--json",
    ]);
    assert_success(&run_captures);
    assert_eq!(
        stdout_json(&run_captures)["captures"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let moved = pseq(&[
        "fragment",
        "mv",
        "omega/dcq-productization-review",
        "omega/review/dcq-productization-review",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&moved);
    assert_eq!(
        stdout_json(&moved)["path"],
        "fragments/omega/review/dcq-productization-review.md"
    );

    let render_after_move = pseq(&[
        "render",
        "omega-dcq-productization-review",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&render_after_move);
    assert_eq!(
        String::from_utf8(render_after_move.stdout).unwrap(),
        "nested body\n"
    );

    let list = pseq(&[
        "fragment",
        "list",
        "--prefix",
        "omega",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    let fragments = stdout_json(&list)["fragments"].as_array().unwrap().clone();
    assert_eq!(fragments.len(), 1);
    assert_eq!(
        fragments[0]["path"],
        "fragments/omega/review/dcq-productization-review.md"
    );

    assert_git_clean(store.path());
}

#[test]
fn nested_captures_and_saved_renders_are_discovered_and_validated() {
    let store = TestStore::initialized("nested-captures-renders");

    let capture = pseq_with_stdin(
        &[
            "capture",
            "import",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "captured prompt\n",
    );
    assert_success(&capture);
    let capture_id = stdout_json(&capture)["id"].as_str().unwrap().to_owned();

    let moved_capture = pseq(&[
        "capture",
        "mv",
        &capture_id,
        "codex/manual",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&moved_capture);
    assert_eq!(
        stdout_json(&moved_capture)["path"],
        "captures/codex/manual.json"
    );

    let shown_capture = pseq(&[
        "capture",
        "show",
        "captures/codex/manual",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&shown_capture);
    assert_eq!(
        stdout_json(&shown_capture)["events"][0]["text"],
        "captured prompt\n"
    );

    let capture_list = pseq(&[
        "capture",
        "list",
        "--prefix",
        "captures/codex",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&capture_list);
    let captures = stdout_json(&capture_list)["captures"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0]["path"], "captures/codex/manual.json");

    let fragment = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Render Source",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "rendered prompt\n",
    );
    assert_success(&fragment);
    let fragment_id = stdout_json(&fragment)["id"].as_str().unwrap().to_owned();

    let sequence = pseq(&[
        "sequence",
        "new",
        "Omega Misc",
        "--path",
        "omega/misc",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&sequence);
    let sequence_path = stdout_json(&sequence)["path"].as_str().unwrap().to_owned();
    assert_eq!(sequence_path, "sequences/omega/misc.json");

    assert_success(&pseq(&[
        "sequence",
        "add",
        "omega-misc",
        &fragment_id,
        "--store",
        path_str(store.path()),
    ]));
    let historical_head = git_head(store.path());

    let saved = pseq(&[
        "render",
        "omega/misc",
        "--save",
        "--dir",
        "renders/omega",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&saved);
    let saved_path = stdout_json(&saved)["saved_render"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(saved_path, "renders/omega/omega-misc.md");

    let explicit_saved = pseq(&[
        "render",
        "omega/misc",
        "--save",
        "--path",
        "renders/omega/custom",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&explicit_saved);
    assert_eq!(
        stdout_json(&explicit_saved)["saved_render"]["path"],
        "renders/omega/custom.md"
    );

    assert_success(&pseq(&[
        "sequence",
        "mv",
        "omega/misc",
        "archive/misc",
        "--store",
        path_str(store.path()),
    ]));
    let historical_render = pseq(&[
        "render",
        "omega/misc",
        "--at",
        &historical_head,
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&historical_render);
    assert_eq!(
        String::from_utf8(historical_render.stdout).unwrap(),
        "rendered prompt\n"
    );
    let saved_content = fs::read_to_string(store.path().join(&saved_path)).unwrap();
    assert!(
        saved_content.contains("sequence_path: sequences/omega/misc.json"),
        "{saved_content}"
    );

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_success(&status);
    let status_json = stdout_json(&status);
    assert_eq!(status_json["counts"]["captures"], 1);
    assert_eq!(status_json["counts"]["renders"], 2);

    assert_success(&pseq(&["doctor", "--store", path_str(store.path())]));
    assert_git_clean(store.path());
}

#[test]
fn invalid_nested_destinations_fail_before_writing() {
    let store = TestStore::initialized("nested-invalid-destinations");

    let traversal = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Bad",
            "--stdin",
            "--path",
            "../bad",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "bad\n",
    );
    assert_eq!(traversal.status.code(), Some(1));
    assert_eq!(
        stderr_json(&traversal)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(!store.path().join("bad.md").exists());

    let bad_extension = pseq(&[
        "sequence",
        "new",
        "Bad",
        "--path",
        "bad.txt",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(bad_extension.status.code(), Some(1));
    assert_eq!(
        stderr_json(&bad_extension)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(!store.path().join("sequences/bad.txt").exists());

    assert_git_clean(store.path());
}

#[test]
fn mismatched_collection_prefix_destinations_fail_before_writing() {
    let store = TestStore::initialized("nested-mismatched-prefixes");

    let fragment = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Bad Fragment",
            "--stdin",
            "--path",
            "sequences/bad-fragment",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "bad\n",
    );
    assert_eq!(fragment.status.code(), Some(1));
    assert_eq!(
        stderr_json(&fragment)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(
        !store
            .path()
            .join("fragments/sequences/bad-fragment.md")
            .exists()
    );

    let sequence = pseq(&[
        "sequence",
        "new",
        "Bad Sequence",
        "--path",
        "fragments/bad-sequence",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(sequence.status.code(), Some(1));
    assert_eq!(
        stderr_json(&sequence)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(
        !store
            .path()
            .join("sequences/fragments/bad-sequence.json")
            .exists()
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
        "captured\n",
    );
    assert_success(&capture);
    let capture_id = stdout_json(&capture)["id"].as_str().unwrap().to_owned();
    let capture_move = pseq(&[
        "capture",
        "mv",
        &capture_id,
        "renders/bad-capture",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(capture_move.status.code(), Some(1));
    assert_eq!(
        stderr_json(&capture_move)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(
        !store
            .path()
            .join("captures/renders/bad-capture.json")
            .exists()
    );

    let render_fragment = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Render Fragment",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "rendered\n",
    );
    assert_success(&render_fragment);
    let render_fragment_id = stdout_json(&render_fragment)["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_success(&pseq(&[
        "sequence",
        "new",
        "Render Sequence",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Render Sequence",
        &render_fragment_id,
        "--store",
        path_str(store.path()),
    ]));
    let saved_render = pseq(&[
        "render",
        "Render Sequence",
        "--save",
        "--path",
        "captures/bad-render",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(saved_render.status.code(), Some(1));
    assert_eq!(
        stderr_json(&saved_render)["error"]["code"],
        "invalid_collection_path"
    );
    assert!(!store.path().join("renders/captures/bad-render.md").exists());

    assert_git_clean(store.path());
}

#[cfg(unix)]
#[test]
fn doctor_reports_symlinked_entries_inside_managed_collections() {
    use std::os::unix::fs::symlink;

    let store = TestStore::initialized("nested-symlink");
    let external = TestStore::new("nested-symlink-target");
    fs::create_dir_all(external.path()).unwrap();
    symlink(external.path(), store.path().join("fragments/link")).unwrap();

    let doctor = pseq(&["doctor", "--store", path_str(store.path()), "--json"]);
    assert_eq!(doctor.status.code(), Some(1));
    let json = stdout_json(&doctor);
    let codes = issue_codes(&json);
    assert!(codes.contains(&"collection_entry_symlink"), "{json}");
}

#[cfg(unix)]
#[test]
fn status_counts_do_not_follow_symlinked_collection_roots() {
    use std::os::unix::fs::symlink;

    let store = TestStore::initialized("nested-symlink-root-count");
    let external = TestStore::new("nested-symlink-root-target");
    fs::create_dir_all(external.path()).unwrap();
    fs::write(
        external.path().join("external.md"),
        "---\nid: frg_00000000000000000000000000000000\nname: External\n---\nexternal\n",
    )
    .unwrap();
    fs::remove_dir_all(store.path().join("fragments")).unwrap();
    symlink(external.path(), store.path().join("fragments")).unwrap();

    let status = pseq(&["status", "--store", path_str(store.path()), "--json"]);
    assert_eq!(status.status.code(), Some(1));
    let json = stdout_json(&status);
    let codes = issue_codes(&json);
    assert!(codes.contains(&"required_path_symlink"), "{json}");
    assert_eq!(json["counts"]["fragments"], 0);
}
