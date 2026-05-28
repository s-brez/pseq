#[path = "common/mod.rs"]
mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use common::{
    TestStore, assert_git_clean, assert_stdout_contains, assert_success, path_str, pseq, pseq_bin,
    pseq_with_env, pseq_with_stdin, stderr_json, stdout_json,
};

#[path = "public_contract/cli.rs"]
mod cli;
#[path = "public_contract/errors.rs"]
mod errors;
#[path = "public_contract/json_payloads.rs"]
mod json_payloads;
#[path = "public_contract/store_files.rs"]
mod store_files;

fn assert_object_keys(value: &Value, expected: &[&str]) {
    let object = value.as_object().unwrap_or_else(|| {
        panic!("expected JSON object, got:\n{value:#}");
    });
    let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "unexpected JSON object keys:\n{value:#}");
}

fn first_array_item<'a>(value: &'a Value, field: &str) -> &'a Value {
    value[field]
        .as_array()
        .unwrap_or_else(|| panic!("{field} should be an array"))
        .first()
        .unwrap_or_else(|| panic!("{field} should not be empty"))
}

fn assert_capture_import_shape(value: &Value) {
    assert_object_keys(
        value,
        &["id", "path", "origin", "event_count", "git_commit"],
    );
    assert!(value["id"].as_str().unwrap().starts_with("cap_"));
    assert!(value["event_count"].as_u64().unwrap() >= 1);
}

fn assert_capture_promote_shape(value: &Value) {
    assert_object_keys(
        value,
        &[
            "capture",
            "sequence",
            "fragments",
            "event_count",
            "git_commit",
        ],
    );
    assert_capture_summary_shape(&value["capture"]);
    assert_object_keys(
        &value["sequence"],
        &["id", "name", "path", "fragment_count"],
    );
    assert_object_keys(
        first_array_item(value, "fragments"),
        &["id", "name", "path"],
    );
}

fn assert_capture_summary_shape(value: &Value) {
    assert_object_keys(value, &["id", "path", "origin", "event_count"]);
    assert!(value["id"].as_str().unwrap().starts_with("cap_"));
}

fn write_noop_editor(store_path: &Path) -> PathBuf {
    let path = store_path.with_extension("noop-editor.sh");
    fs::write(&path, "exit 0\n").unwrap();
    path
}

fn split_frontmatter(content: &str) -> (&str, &str) {
    let content = content
        .strip_prefix("---\n")
        .expect("file should start with frontmatter");
    let (frontmatter, body) = content
        .split_once("\n---\n")
        .expect("frontmatter should have a closing delimiter");
    (frontmatter, body)
}
