#[path = "common/mod.rs"]
mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use common::{
    TestStore, assert_git_clean, assert_success, git_status, issue_codes, path_str, pseq,
    pseq_with_env, pseq_with_env_removed, pseq_with_stdin, stderr_json, stdout_json,
};

const DEFAULT_CAPTURE_SOURCE_NAMES: [&str; 5] =
    ["stdin", "codex", "claude-code", "openhands", "opencode"];

#[path = "capture/import.rs"]
mod import;
#[path = "capture/promote.rs"]
mod promote;
#[path = "capture/selection.rs"]
mod selection;
#[path = "capture/sources.rs"]
mod sources;
#[path = "capture/validation.rs"]
mod validation;

fn assert_source_capture(
    store_path: &Path,
    source: &str,
    env: (&str, &str),
    expected_prompts: &[&str],
) {
    let captured = pseq_with_env(
        &[
            "capture",
            "range",
            "1..2",
            "--source",
            source,
            "--store",
            path_str(store_path),
            "--json",
        ],
        &[env],
    );
    assert_success(&captured);
    let captured_json = stdout_json(&captured);
    assert_eq!(captured_json["origin"]["kind"], "source");
    assert_eq!(captured_json["origin"]["source"], source);
    assert_eq!(captured_json["event_count"], 2);

    let capture_id = captured_json["id"].as_str().unwrap();
    let show = pseq(&[
        "capture",
        "show",
        capture_id,
        "--store",
        path_str(store_path),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["events"][0]["text"], expected_prompts[0]);
    assert_eq!(show_json["events"][1]["text"], expected_prompts[1]);
}

fn write_codex_session(codex_home: &Path, timestamp: &str, id: &str, prompts: &[&str]) -> PathBuf {
    let dir = codex_home.join("sessions/2026/05/23");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("rollout-{timestamp}-{id}.jsonl"));
    let mut file = fs::File::create(&path).unwrap();

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": timestamp,
            "type": "session_meta",
            "payload": {
                "id": id,
                "timestamp": timestamp,
                "cwd": ".",
                "originator": "test",
                "cli_version": "test",
                "source": "cli",
                "model_provider": "test-provider"
            }
        }),
    );

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": timestamp,
            "type": "event_msg",
            "payload": {
                "type": "agent_message",
                "message": "assistant text must be ignored"
            }
        }),
    );

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": timestamp,
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [
                    {
                        "type": "input_text",
                        "text": "response item fallback should be ignored when events exist"
                    }
                ]
            }
        }),
    );

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": timestamp,
            "type": "event_msg",
            "payload": {
                "type": "user_message",
                "message": "cargo test",
                "command": "cargo test"
            }
        }),
    );

    for prompt in prompts {
        write_json_line(
            &mut file,
            serde_json::json!({
                "timestamp": timestamp,
                "type": "event_msg",
                "payload": {
                    "type": "user_message",
                    "message": prompt,
                    "kind": "plain"
                }
            }),
        );
    }

    path
}

fn write_claude_code_session(claude_home: &Path, id: &str, prompts: &[&str]) -> PathBuf {
    let dir = claude_home.join("projects/test-project");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{id}.jsonl"));
    let mut file = fs::File::create(&path).unwrap();

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": "2026-05-23T14:00:00.000Z",
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    { "type": "text", "text": "assistant text must be ignored" }
                ]
            }
        }),
    );

    write_json_line(
        &mut file,
        serde_json::json!({
            "timestamp": "2026-05-23T14:00:01.000Z",
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "content": "tool result must be ignored"
                    }
                ]
            }
        }),
    );

    for prompt in prompts {
        write_json_line(
            &mut file,
            serde_json::json!({
                "timestamp": "2026-05-23T14:00:02.000Z",
                "type": "user",
                "message": {
                    "role": "user",
                    "content": [
                        { "type": "text", "text": prompt }
                    ]
                }
            }),
        );
    }

    path
}

fn write_json_line(file: &mut fs::File, value: serde_json::Value) {
    writeln!(file, "{value}").unwrap();
}

fn write_openhands_trajectory(openhands_home: &Path, id: &str, prompts: &[&str]) -> PathBuf {
    let dir = openhands_home.join("trajectories");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{id}.json"));

    let mut entries = vec![
        serde_json::json!({
            "id": 1,
            "timestamp": "2026-05-23T15:00:00.000Z",
            "source": "agent",
            "action": "run",
            "message": "Running command: cargo test",
            "args": { "command": "cargo test" }
        }),
        serde_json::json!({
            "id": 2,
            "timestamp": "2026-05-23T15:00:01.000Z",
            "source": "user",
            "observation": "null",
            "message": "No observation",
            "content": ""
        }),
    ];
    for (index, prompt) in prompts.iter().enumerate() {
        entries.push(serde_json::json!({
            "id": index + 3,
            "timestamp": "2026-05-23T15:00:02.000Z",
            "source": "user",
            "action": "message",
            "message": prompt,
            "args": {
                "content": prompt,
                "wait_for_response": false
            }
        }));
    }

    fs::write(&path, serde_json::to_string_pretty(&entries).unwrap()).unwrap();
    path
}

fn write_opencode_messages(
    opencode_home: &Path,
    session_id: &str,
    prompts: &[&str],
) -> Vec<PathBuf> {
    let dir = opencode_home.join("storage/message/test-project");
    fs::create_dir_all(&dir).unwrap();
    let mut paths = Vec::new();

    let assistant_path = dir.join(format!("{session_id}-assistant.json"));
    fs::write(
        &assistant_path,
        serde_json::to_string_pretty(&serde_json::json!({
            "id": "msg_assistant",
            "sessionID": session_id,
            "role": "assistant",
            "time": { "created": 1 },
            "parts": [
                { "type": "text", "text": "assistant text must be ignored" },
                {
                    "type": "tool",
                    "tool": "bash",
                    "state": {
                        "status": "completed",
                        "input": { "command": "cargo test" },
                        "output": "shell output must be ignored"
                    }
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    paths.push(assistant_path);

    for (index, prompt) in prompts.iter().enumerate() {
        let path = dir.join(format!("{session_id}-user-{index}.json"));
        fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::json!({
                "id": format!("msg_user_{index}"),
                "sessionID": session_id,
                "role": "user",
                "time": { "created": index + 2 },
                "parts": [
                    { "type": "text", "text": prompt },
                    { "type": "text", "text": "ignored synthetic prompt", "synthetic": true }
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        paths.push(path);
    }

    paths
}

fn assert_default_capture_source_names(json: &serde_json::Value) {
    let source_names = json["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(source_names.as_slice(), &DEFAULT_CAPTURE_SOURCE_NAMES[..]);
}
