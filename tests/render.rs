#[path = "common/mod.rs"]
mod common;

use std::fs;
use std::process::Output;

use common::{
    TestStore, assert_git_clean, assert_success, git_commit_all, git_head, git_status, path_str,
    pseq, pseq_with_stdin, stderr_json, stdout_json,
};

#[path = "render/errors.rs"]
mod errors;
#[path = "render/history.rs"]
mod history;
#[path = "render/inclusions.rs"]
mod inclusions;
#[path = "render/rendering.rs"]
mod rendering;
#[path = "render/save_out.rs"]
mod save_out;
#[path = "render/variables.rs"]
mod variables;

fn create_fragment(store: &TestStore, fragment_name: &str, body: &str) -> serde_json::Value {
    let created = pseq_with_stdin(
        &[
            "fragment",
            "new",
            fragment_name,
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        body,
    );
    assert_success(&created);
    stdout_json(&created)
}

fn create_sequence(store: &TestStore, sequence_name: &str) -> serde_json::Value {
    let created = pseq(&[
        "sequence",
        "new",
        sequence_name,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&created);
    stdout_json(&created)
}

fn add_fragment_to_sequence(store: &TestStore, sequence_name: &str, fragment_reference: &str) {
    assert_success(&pseq(&[
        "sequence",
        "add",
        sequence_name,
        fragment_reference,
        "--store",
        path_str(store.path()),
    ]));
}

fn create_single_fragment_sequence(
    store: &TestStore,
    sequence_name: &str,
    fragment_name: &str,
    body: &str,
) {
    create_fragment(store, fragment_name, body);
    create_sequence(store, sequence_name);
    add_fragment_to_sequence(store, sequence_name, fragment_name);
}

fn assert_render_json_error(store: &TestStore, sequence_reference: &str, code: &str) {
    let output = pseq(&[
        "render",
        sequence_reference,
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_json_error(&output, code);
}

fn assert_json_error(output: &Output, code: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_json(output)["error"]["code"], code);
}
