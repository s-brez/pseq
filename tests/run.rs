#[path = "common/mod.rs"]
mod common;

use std::fs;

use common::{
    TestStore, assert_git_clean, assert_success, git, git_head, path_str, pseq, pseq_bin,
    pseq_in_dir_with_env, pseq_with_stdin, stderr_json, stdout_json,
};
#[cfg(unix)]
use common::{pseq_command_in_own_process_group, pseq_in_own_process_group};

#[path = "run/capture_output.rs"]
mod capture_output;
#[path = "run/codex_harness.rs"]
mod codex_harness;
#[path = "run/failures.rs"]
mod failures;
#[path = "run/feedback.rs"]
mod feedback;
#[path = "run/reasoning_effort.rs"]
mod reasoning_effort;
#[path = "run/runner_modes.rs"]
mod runner_modes;

fn create_sequence_with_fragments(store: &TestStore, sequence: &str, fragments: &[(&str, &str)]) {
    for (name, body) in fragments {
        assert_success(&pseq_with_stdin(
            &[
                "--store",
                path_str(store.path()),
                "fragment",
                "new",
                name,
                "--stdin",
            ],
            body,
        ));
    }
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "sequence",
        "new",
        sequence,
    ]));
    for (name, _) in fragments {
        assert_success(&pseq(&[
            "--store",
            path_str(store.path()),
            "sequence",
            "add",
            sequence,
            name,
        ]));
    }
}

fn capture_texts(store: &TestStore) -> Vec<String> {
    let mut texts = Vec::new();
    for entry in fs::read_dir(store.path().join("captures")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        texts.extend(
            value["events"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| event["text"].as_str().unwrap().to_owned()),
        );
    }
    texts.sort();
    texts
}
