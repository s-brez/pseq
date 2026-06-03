use super::*;

#[test]
fn run_rejects_reasoning_effort_for_unrecognized_runner_before_execution() {
    let store = TestStore::initialized("run-reasoning-generic-reject");
    let scratch = TestStore::new("run-reasoning-generic-reject-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let marker = scratch.path().join("runner-started");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    add_fragment_pseq_run_reasoning_effort(&store, "Only", "high");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "printf started > \"$1\"",
        "sh",
        path_str(&marker),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!marker.exists(), "runner command should not be spawned");

    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("fragment \"Only\""));
    assert!(message.contains("pseq.run.reasoning_effort"));
    assert!(message.contains("not a recognized adapter"));
}

#[test]
fn run_rejects_later_reasoning_effort_for_unrecognized_runner_before_any_execution() {
    let store = TestStore::initialized("run-reasoning-later-generic-reject");
    let scratch = TestStore::new("run-reasoning-later-generic-reject-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let marker = scratch.path().join("runner-started");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("First", "first\n"), ("Second", "second\n")],
    );
    add_fragment_pseq_run_reasoning_effort(&store, "Second", "high");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "printf started > \"$1\"",
        "sh",
        path_str(&marker),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!marker.exists(), "no runner turn should be spawned");

    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("fragment \"Second\""));
    assert!(message.contains("pseq.run.reasoning_effort"));
    assert!(message.contains("not a recognized adapter"));
}

#[test]
fn run_rejects_invalid_reasoning_effort_before_execution() {
    let store = TestStore::initialized("run-reasoning-invalid-value");
    let scratch = TestStore::new("run-reasoning-invalid-value-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let marker = scratch.path().join("runner-started");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    add_fragment_pseq_run_reasoning_effort(&store, "Only", "turbo");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "printf started > \"$1\"",
        "sh",
        path_str(&marker),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!marker.exists(), "runner command should not be spawned");

    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("fragment \"Only\""));
    assert!(message.contains("pseq.run.reasoning_effort"));
    assert!(message.contains("unsupported value \"turbo\""));
    assert!(message.contains("minimal, low, medium, high, xhigh, max"));
}

#[test]
fn run_rejects_non_string_reasoning_effort_before_execution() {
    let store = TestStore::initialized("run-reasoning-non-string-value");
    let scratch = TestStore::new("run-reasoning-non-string-value-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let marker = scratch.path().join("runner-started");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    add_fragment_frontmatter_block(&store, "Only", "pseq:\n  run:\n    reasoning_effort: 7\n");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "printf started > \"$1\"",
        "sh",
        path_str(&marker),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!marker.exists(), "runner command should not be spawned");

    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("fragment \"Only\""));
    assert!(message.contains("pseq.run.reasoning_effort must be a string value"));
}

#[test]
fn run_rejects_literal_dotted_reasoning_effort_key_before_execution() {
    let store = TestStore::initialized("run-reasoning-literal-dotted-key");
    let scratch = TestStore::new("run-reasoning-literal-dotted-key-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let marker = scratch.path().join("runner-started");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    add_fragment_frontmatter_block(&store, "Only", "pseq.run.reasoning_effort: high\n");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "printf started > \"$1\"",
        "sh",
        path_str(&marker),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(!marker.exists(), "runner command should not be spawned");

    let error = stderr_json(&output);
    assert_eq!(error["error"]["code"], "invalid_run_invocation");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(message.contains("fragment \"Only\""));
    assert!(message.contains("literal frontmatter key \"pseq.run.reasoning_effort\""));
    assert!(message.contains("pseq -> run -> reasoning_effort"));
}

#[test]
fn run_ignores_unrelated_pseq_frontmatter_metadata() {
    let store = TestStore::initialized("run-reasoning-unrelated-pseq");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    add_fragment_frontmatter_block(&store, "Only", "pseq: note\n");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "cat",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 1);
    assert_eq!(json["turns"][0]["stdout"], "body\n");
}

#[test]
fn run_ignores_reasoning_effort_on_included_fragments() {
    let store = TestStore::initialized("run-reasoning-include-ignored");
    assert_success(&pseq_with_stdin(
        &[
            "--store",
            path_str(store.path()),
            "fragment",
            "new",
            "Top",
            "--stdin",
        ],
        "{{pseq.fragment.Reusable}}",
    ));
    assert_success(&pseq_with_stdin(
        &[
            "--store",
            path_str(store.path()),
            "fragment",
            "new",
            "Reusable",
            "--stdin",
        ],
        "included\n",
    ));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "sequence",
        "new",
        "Workflow",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "sequence",
        "add",
        "Workflow",
        "Top",
    ]));
    add_fragment_pseq_run_reasoning_effort(&store, "Reusable", "high");

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "cat",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 1);
    assert_eq!(json["turns"][0]["stdout"], "included\n");
}

#[ignore = "boots the real Codex CLI and spends model/tool time"]
#[test]
fn run_with_real_codex_applies_fragment_reasoning_effort_per_turn() {
    assert_success(
        &std::process::Command::new("codex")
            .arg("--version")
            .output()
            .expect("real codex CLI should be installed for this ignored test"),
    );

    let store = TestStore::initialized("run-real-codex-reasoning-store");
    let codex_home = isolated_codex_home("run-real-codex-reasoning-codex-home");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            (
                "Low",
                "Reply exactly with this token and no other text: PSEQ-REASONING-LOW\n",
            ),
            (
                "Default",
                "Reply exactly with this token and no other text: PSEQ-REASONING-DEFAULT\n",
            ),
            (
                "High",
                "Reply exactly with this token and no other text: PSEQ-REASONING-HIGH\n",
            ),
        ],
    );
    add_fragment_pseq_run_reasoning_effort(&store, "Low", "low");
    add_fragment_pseq_run_reasoning_effort(&store, "High", "high");

    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--max-captured-output",
            "2000000",
            "--",
            "codex",
            "exec",
            "-m",
            "gpt-5.4-mini",
            "-c",
            "model_reasoning_effort=\"medium\"",
            "--ignore-user-config",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "-",
        ],
        store.path(),
        &[("CODEX_HOME", path_str(codex_home.path()))],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 3);
    assert_codex_turn_reasoning_effort(&json, 0, "low");
    assert_codex_turn_reasoning_effort(&json, 1, "medium");
    assert_codex_turn_reasoning_effort(&json, 2, "high");
    assert_eq!(
        codex_session_turn_efforts(&codex_home),
        vec!["low".to_owned(), "medium".to_owned(), "high".to_owned()]
    );

    for turn_index in 1..=2 {
        let command = json["turns"][turn_index]["command"].as_array().unwrap();
        assert!(
            command.iter().any(|arg| arg == "resume"),
            "expected later Codex turn to resume the active session, got {command:?}"
        );
    }

    assert!(
        json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-REASONING-LOW")
    );
    assert!(
        json["turns"][1]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-REASONING-DEFAULT")
    );
    assert!(
        json["turns"][2]["stdout"]
            .as_str()
            .unwrap()
            .contains("PSEQ-REASONING-HIGH")
    );
}

fn add_fragment_pseq_run_reasoning_effort(store: &TestStore, name: &str, effort: &str) {
    add_fragment_frontmatter_block(
        store,
        name,
        &format!("pseq:\n  run:\n    reasoning_effort: {effort}\n"),
    );
}

fn add_fragment_frontmatter_block(store: &TestStore, name: &str, block: &str) {
    let path = fragment_path_by_name(store, name);
    let content = fs::read_to_string(&path).unwrap();
    let rest = content
        .strip_prefix("---\n")
        .expect("fragment should start with YAML frontmatter");
    let delimiter = rest
        .find("\n---\n")
        .expect("fragment should close YAML frontmatter");
    let frontmatter = &rest[..delimiter];
    let body = &rest[delimiter + "\n---\n".len()..];
    fs::write(&path, format!("---\n{frontmatter}\n{block}---\n{body}")).unwrap();
}

fn fragment_path_by_name(store: &TestStore, name: &str) -> std::path::PathBuf {
    let expected_line = format!("name: {name}");
    let mut matches = Vec::new();
    for entry in fs::read_dir(store.path().join("fragments")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("md") {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap();
        if content.lines().any(|line| line == expected_line) {
            matches.push(path);
        }
    }
    assert_eq!(matches.len(), 1, "expected one fragment named {name:?}");
    matches.pop().unwrap()
}

fn isolated_codex_home(name: &str) -> TestStore {
    let codex_home = TestStore::new(name);
    fs::create_dir_all(codex_home.path()).unwrap();
    fs::copy(
        default_codex_home().join("auth.json"),
        codex_home.path().join("auth.json"),
    )
    .expect("real Codex integration tests require Codex auth");
    codex_home
}

fn default_codex_home() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os("CODEX_HOME") {
        return path.into();
    }
    if let Some(home) = std::env::var_os("HOME") {
        return std::path::PathBuf::from(home).join(".codex");
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE") {
        return std::path::PathBuf::from(userprofile).join(".codex");
    }
    panic!("real Codex integration tests require CODEX_HOME, HOME, or USERPROFILE");
}

fn codex_session_turn_efforts(codex_home: &TestStore) -> Vec<String> {
    let mut records = Vec::new();
    collect_codex_session_turn_efforts(&codex_home.path().join("sessions"), &mut records);
    records.sort_by(|left, right| left.0.cmp(&right.0));
    records
        .into_iter()
        .map(|(_, effort)| effort)
        .collect::<Vec<_>>()
}

fn collect_codex_session_turn_efforts(path: &std::path::Path, records: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_codex_session_turn_efforts(&path, records);
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        for line in fs::read_to_string(&path).unwrap().lines() {
            let value: serde_json::Value = serde_json::from_str(line).unwrap();
            if value["type"] == "turn_context" {
                records.push((
                    value["timestamp"].as_str().unwrap().to_owned(),
                    value["payload"]["effort"].as_str().unwrap().to_owned(),
                ));
            }
        }
    }
}

fn assert_codex_turn_reasoning_effort(json: &serde_json::Value, turn_index: usize, effort: &str) {
    let command = json["turns"][turn_index]["command"].as_array().unwrap();
    let expected = format!("model_reasoning_effort=\"{effort}\"");
    assert!(
        command
            .windows(2)
            .any(|args| args[0] == "-c" && args[1] == expected),
        "expected turn {} Codex command to set reasoning effort to {effort:?}, got {command:?}",
        turn_index + 1
    );
}
