use super::*;

#[test]
fn run_with_ad_hoc_command_feeds_each_fragment_as_one_turn() {
    let store = TestStore::initialized("run-ad-hoc");
    let sink = TestStore::initialized("run-ad-hoc-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);
    let before_head = git_head(store.path());

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(sink.path()),
    ]);
    assert_success(&output);
    assert_eq!(git_head(store.path()), before_head);
    assert!(String::from_utf8_lossy(&output.stdout).contains("created capture:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("pseq: running turn 1/2"));

    let texts = capture_texts(&sink);
    assert_eq!(texts.len(), 2);
    assert!(texts.contains(&"A\n".to_owned()));
    assert!(texts.contains(&"B\n".to_owned()));
    assert_git_clean(store.path());
    assert_git_clean(sink.path());
}

#[cfg(unix)]
#[test]
fn run_adds_git_metadata_writable_roots_for_sandboxed_agent_runner() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-agent-git-roots");
    let workspace = TestStore::initialized("run-agent-git-roots-workspace");
    let bin_dir = TestStore::new("run-agent-git-roots-bin");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
args="$*"
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message|-o)
      shift
      out="$1"
      ;;
  esac
  shift
done
if [ -n "$out" ]; then
  printf '%s\n' "$args" > "$out"
else
  printf '%s\n' "$args"
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--",
            "codex",
            "exec",
            "--sandbox",
            "workspace-write",
            "--color",
            "never",
            "-",
        ],
        workspace.path(),
        &[("PATH", &path)],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    let command = json["turns"][0]["command"].as_array().unwrap();
    let git_dir_path = workspace.path().join(".git");
    let git_dir = path_str(&git_dir_path);
    assert!(
        command
            .windows(2)
            .any(|args| args[0] == "--add-dir" && args[1] == git_dir),
        "expected command to add git metadata as writable root, got {command:?}"
    );
    let stdout = json["turns"][0]["stdout"].as_str().unwrap();
    assert!(
        stdout.contains(git_dir),
        "fake runner should receive the prepared command argv, got {stdout:?}"
    );
}

#[cfg(unix)]
#[test]
fn run_does_not_session_wrap_codex_exec_review_after_prepared_options() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-review-subcommand-store");
    let bin_dir = TestStore::new("run-codex-review-subcommand-bin");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' "$*"
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--",
            "codex",
            "exec",
            "--sandbox",
            "workspace-write",
            "review",
        ],
        store.path(),
        &[("PATH", &path)],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    let command = json["turns"][0]["command"].as_array().unwrap();
    let git_dir_path = store.path().join(".git");
    let git_dir = path_str(&git_dir_path);
    assert!(
        command
            .windows(2)
            .any(|args| args[0] == "--add-dir" && args[1] == git_dir),
        "expected prepared command to keep writable Git metadata root, got {command:?}"
    );
    assert!(
        command.iter().any(|arg| arg == "review"),
        "expected Codex review subcommand to remain in command, got {command:?}"
    );
    assert!(
        !command.iter().any(|arg| arg == "--output-last-message"),
        "expected Codex review subcommand not to be session wrapped, got {command:?}"
    );
    assert!(
        json["turns"][0]["stdout"]
            .as_str()
            .unwrap()
            .contains("review"),
        "expected generic fake Codex stdout to be captured"
    );
}

#[cfg(unix)]
#[test]
fn run_resumes_exact_codex_session_for_later_sequence_turns() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-session-fake-store");
    let bin_dir = TestStore::new("run-codex-session-fake-bin");
    let log_path = bin_dir.path().join("codex.log");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
args="$*"
out=""
resume=0
session=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message|-o)
      shift
      out="$1"
      ;;
    resume)
      resume=1
      ;;
    --json|--color|--sandbox|-m|-s)
      if [ "$1" != "--json" ]; then
        shift
      fi
      ;;
    --*)
      ;;
    -)
      ;;
    *)
      if [ "$resume" = "1" ] && [ -z "$session" ]; then
        session="$1"
      fi
      ;;
  esac
  shift
done
input=$(cat)
if [ "$resume" = "1" ]; then
  printf 'resume session=%s input=%s\n' "$session" "$input" >> "$PSEQ_FAKE_CODEX_LOG"
  printf 'resumed %s\n' "$session" > "$out"
else
  printf 'first input=%s args=%s\n' "$input" "$args" >> "$PSEQ_FAKE_CODEX_LOG"
  printf '{"type":"thread.started","thread_id":"fake-session-123"}\n'
  printf 'started fake-session-123\n' > "$out"
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("First", "first prompt\n"), ("Second", "second prompt\n")],
    );

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let log_path_arg = path_str(&log_path);
    let ignored_output_path = bin_dir.path().join("ignored-output.txt");
    let ignored_output_path_arg = path_str(&ignored_output_path);
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--",
            "codex",
            "exec",
            "--sandbox",
            "workspace-write",
            "--color",
            "never",
            "--output-last-message",
            ignored_output_path_arg,
            "-",
        ],
        store.path(),
        &[
            ("PATH", path.as_str()),
            ("PSEQ_FAKE_CODEX_LOG", log_path_arg),
        ],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 2);
    assert_eq!(json["turns"][0]["stdout"], "started fake-session-123\n");
    assert_eq!(json["turns"][1]["stdout"], "resumed fake-session-123\n");

    let turn_2_command = json["turns"][1]["command"].as_array().unwrap();
    let turn_1_command = json["turns"][0]["command"].as_array().unwrap();
    assert!(
        !turn_1_command
            .iter()
            .any(|arg| arg == ignored_output_path_arg),
        "expected pseq to own Codex output capture path, got {turn_1_command:?}"
    );
    assert!(
        turn_2_command.iter().any(|arg| arg == "resume"),
        "expected second turn command to use Codex resume, got {turn_2_command:?}"
    );
    assert!(
        turn_2_command.iter().any(|arg| arg == "fake-session-123"),
        "expected second turn command to resume exact session id, got {turn_2_command:?}"
    );
    assert!(
        !turn_2_command.iter().any(|arg| arg == "--last"),
        "expected exact session resume, got {turn_2_command:?}"
    );
    let git_dir_path = store.path().join(".git");
    let git_dir = path_str(&git_dir_path);
    assert!(
        turn_2_command
            .windows(2)
            .any(|args| args[0] == "--add-dir" && args[1] == git_dir),
        "expected resumed Codex command to preserve writable Git metadata roots, got {turn_2_command:?}"
    );

    let log = fs::read_to_string(log_path).unwrap();
    assert!(log.contains("first input=first prompt"));
    assert!(log.contains("resume session=fake-session-123 input=second prompt"));
}

#[cfg(unix)]
#[test]
fn run_can_reset_codex_session_per_iteration_while_carrying_feedback() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-session-scope-iteration-store");
    let bin_dir = TestStore::new("run-codex-session-scope-iteration-bin");
    let counter_path = bin_dir.path().join("counter.txt");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
out=""
resume=0
session=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message|-o)
      shift
      out="$1"
      ;;
    resume)
      resume=1
      ;;
    --json|--color|--sandbox|-m|-s)
      if [ "$1" != "--json" ]; then
        shift
      fi
      ;;
    --*)
      ;;
    -)
      ;;
    *)
      if [ "$resume" = "1" ] && [ -z "$session" ]; then
        session="$1"
      fi
      ;;
  esac
  shift
done
input=$(cat)
if [ "$resume" = "1" ]; then
  printf 'resumed %s\n' "$session" > "$out"
else
  count=0
  if [ -f "$PSEQ_FAKE_CODEX_COUNTER" ]; then
    count=$(cat "$PSEQ_FAKE_CODEX_COUNTER")
  fi
  count=$((count + 1))
  printf '%s\n' "$count" > "$PSEQ_FAKE_CODEX_COUNTER"
  session="iteration-session-$count"
  saw=none
  case "$input" in
    *PSEQ-SEED*) saw=seed ;;
    *"resumed iteration-session-1"*) saw=feedback ;;
  esac
  printf '{"type":"thread.started","thread_id":"%s"}\n' "$session"
  printf 'started %s saw=%s\n' "$session" "$saw" > "$out"
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            ("First", "feedback={{loop_feedback}}\n"),
            ("Final", "final prompt\n"),
        ],
    );

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let counter_path_arg = path_str(&counter_path);
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--iterations",
            "2",
            "--session-scope",
            "iteration",
            "--feedback-from",
            "final-stdout",
            "--feedback-var",
            "loop_feedback",
            "--feedback-seed",
            "PSEQ-SEED",
            "--",
            "codex",
            "exec",
            "--sandbox",
            "workspace-write",
            "--color",
            "never",
            "-",
        ],
        store.path(),
        &[
            ("PATH", &path),
            ("PSEQ_FAKE_CODEX_COUNTER", counter_path_arg),
        ],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 4);
    assert_eq!(
        json["turns"][0]["stdout"],
        "started iteration-session-1 saw=seed\n"
    );
    assert_eq!(json["turns"][1]["stdout"], "resumed iteration-session-1\n");
    assert_eq!(
        json["turns"][2]["stdout"],
        "started iteration-session-2 saw=feedback\n"
    );
    assert_eq!(json["turns"][3]["stdout"], "resumed iteration-session-2\n");

    let iteration_1_final = json["turns"][1]["command"].as_array().unwrap();
    let iteration_2_first = json["turns"][2]["command"].as_array().unwrap();
    let iteration_2_final = json["turns"][3]["command"].as_array().unwrap();
    assert!(
        iteration_1_final.iter().any(|arg| arg == "resume")
            && iteration_1_final
                .iter()
                .any(|arg| arg == "iteration-session-1"),
        "expected first iteration final turn to resume iteration-session-1, got {iteration_1_final:?}"
    );
    assert!(
        !iteration_2_first.iter().any(|arg| arg == "resume"),
        "expected second iteration first turn to start a fresh session, got {iteration_2_first:?}"
    );
    assert!(
        iteration_2_final.iter().any(|arg| arg == "resume")
            && iteration_2_final
                .iter()
                .any(|arg| arg == "iteration-session-2"),
        "expected second iteration final turn to resume iteration-session-2, got {iteration_2_final:?}"
    );
}

#[cfg(unix)]
#[test]
fn run_named_codex_runner_uses_session_continuation_instead_of_configured_next() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-named-codex-session-store");
    let bin_dir = TestStore::new("run-named-codex-session-bin");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
out=""
resume=0
session=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message|-o)
      shift
      out="$1"
      ;;
    resume)
      resume=1
      ;;
    --json|--color)
      ;;
    -)
      ;;
    *)
      if [ "$resume" = "1" ] && [ -z "$session" ]; then
        session="$1"
      fi
      ;;
  esac
  shift
done
cat >/dev/null
if [ "$resume" = "1" ]; then
  printf 'resumed %s\n' "$session" > "$out"
else
  printf '{"type":"thread.started","thread_id":"named-session-456"}\n'
  printf 'started named-session-456\n' > "$out"
fi
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("First", "first prompt\n"), ("Second", "second prompt\n")],
    );

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "codex",
        "first",
        "--",
        "codex",
        "exec",
        "--color",
        "never",
        "-",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "codex",
        "next",
        "--",
        pseq_bin(),
        "--version",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "codex",
    ]));

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
        ],
        store.path(),
        &[("PATH", &path)],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 2);
    assert_eq!(json["turns"][1]["stdout"], "resumed named-session-456\n");
    let turn_2_command = json["turns"][1]["command"].as_array().unwrap();
    assert!(
        turn_2_command.iter().any(|arg| arg == "resume"),
        "expected named Codex runner to use session continuation, got {turn_2_command:?}"
    );
    assert!(
        turn_2_command.iter().any(|arg| arg == "named-session-456"),
        "expected named Codex runner to resume exact session id, got {turn_2_command:?}"
    );
    assert!(
        !turn_2_command
            .iter()
            .any(|arg| arg.as_str() == Some(pseq_bin())),
        "expected configured next command to be ignored for active Codex session, got {turn_2_command:?}"
    );
}

#[cfg(unix)]
#[test]
fn run_fails_when_codex_session_id_is_missing_before_later_turns() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-missing-session-store");
    let bin_dir = TestStore::new("run-codex-missing-session-bin");
    let log_path = bin_dir.path().join("codex.log");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --output-last-message|-o)
      shift
      out="$1"
      ;;
  esac
  shift
done
input=$(cat)
printf '%s\n' "$input" >> "$PSEQ_FAKE_CODEX_LOG"
printf 'no session id\n' > "$out"
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[("First", "first prompt\n"), ("Second", "second prompt\n")],
    );

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let log_path_arg = path_str(&log_path);
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--",
            "codex",
            "exec",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "-",
        ],
        store.path(),
        &[
            ("PATH", path.as_str()),
            ("PSEQ_FAKE_CODEX_LOG", log_path_arg),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "invalid_run_invocation"
    );
    assert_eq!(fs::read_to_string(log_path).unwrap(), "first prompt\n");
}

#[cfg(unix)]
#[test]
fn run_fails_when_successful_codex_does_not_write_final_message() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-missing-final-message-store");
    let bin_dir = TestStore::new("run-codex-missing-final-message-bin");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
cat >/dev/null
printf '{"type":"thread.started","thread_id":"fake-session-123"}\n'
"#,
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let path = format!(
        "{}:{}",
        path_str(bin_dir.path()),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = pseq_in_dir_with_env(
        &[
            "--store",
            path_str(store.path()),
            "--json",
            "run",
            "Workflow",
            "--",
            "codex",
            "exec",
            "--color",
            "never",
            "-",
        ],
        store.path(),
        &[("PATH", &path)],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "runner_read_output_failed"
    );
}

#[ignore = "boots the real Codex CLI and spends model/tool time"]
#[test]
fn run_with_real_codex_can_commit_inside_workspace_write_sandbox() {
    const MARKER_FILE: &str = "pseq-real-codex-git-marker.txt";
    const MARKER_TEXT: &str = "pseq real codex git metadata write check\n";
    const COMMIT_MESSAGE: &str = "pseq real codex git write check";

    assert_success(
        &std::process::Command::new("codex")
            .arg("--version")
            .output()
            .expect("real codex CLI should be installed for this ignored test"),
    );

    let store = TestStore::initialized("run-real-codex-git-store");
    let workspace = TestStore::new("run-real-codex-git-workspace");
    fs::create_dir_all(workspace.path()).unwrap();
    assert_success(&git(workspace.path(), &["init", "--quiet"]));
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[(
            "Only",
            &format!(
                "\
Automated pseq integration test.

Do exactly this in the current Git repository:
1. Write the file `{MARKER_FILE}` with exactly this single line:
{}
2. Run:
git add {MARKER_FILE}
git -c user.name=pseq-real-codex-test -c user.email=pseq-real-codex-test@example.invalid commit -m {COMMIT_MESSAGE:?}
3. Do not modify any other files.
4. Final response: committed {COMMIT_MESSAGE}
",
                MARKER_TEXT.trim_end()
            ),
        )],
    );

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
            "--sandbox",
            "workspace-write",
            "--color",
            "never",
            "-",
        ],
        workspace.path(),
        &[],
    );
    assert_success(&output);

    let json = stdout_json(&output);
    let command = json["turns"][0]["command"].as_array().unwrap();
    let git_dir_path = workspace.path().join(".git");
    let git_dir = path_str(&git_dir_path);
    assert!(
        command
            .windows(2)
            .any(|args| args[0] == "--add-dir" && args[1] == git_dir),
        "expected pseq to add git metadata as a writable root, got {command:?}"
    );

    let committed_file = git(workspace.path(), &["show", &format!("HEAD:{MARKER_FILE}")]);
    assert_success(&committed_file);
    assert_eq!(
        String::from_utf8(committed_file.stdout).unwrap(),
        MARKER_TEXT
    );

    let commit_subject = git(workspace.path(), &["log", "-1", "--pretty=%s"]);
    assert_success(&commit_subject);
    assert_eq!(
        String::from_utf8(commit_subject.stdout).unwrap().trim(),
        COMMIT_MESSAGE
    );
    assert_git_clean(workspace.path());
}

#[ignore = "boots the real Codex CLI and spends model/tool time"]
#[test]
fn run_with_real_codex_keeps_sequence_turns_in_one_session() {
    const TOKEN: &str = "PSEQ-CODEX-SESSION-CONTINUITY-1779625000";

    assert_success(
        &std::process::Command::new("codex")
            .arg("--version")
            .output()
            .expect("real codex CLI should be installed for this ignored test"),
    );

    let store = TestStore::initialized("run-real-codex-session-store");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[
            (
                "Remember",
                &format!(
                    "\
Remember this exact continuity token for the next prompt:
{TOKEN}

Reply exactly:
stored
"
                ),
            ),
            (
                "Recall",
                "\
Without reading files, running shell commands, or using external state, reply with the exact continuity token I asked you to remember in the previous prompt.
Do not include anything except the token.
",
            ),
        ],
    );

    let output = pseq(&[
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
        "--skip-git-repo-check",
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "-",
    ]);
    assert_success(&output);

    let json = stdout_json(&output);
    assert_eq!(json["completed_turns"], 2);
    let turn_2_command = json["turns"][1]["command"].as_array().unwrap();
    assert!(
        turn_2_command.iter().any(|arg| arg == "resume"),
        "expected second Codex turn to resume the first session, got {turn_2_command:?}"
    );
    assert!(
        !turn_2_command.iter().any(|arg| arg == "--last"),
        "expected pseq to resume an exact Codex session id, got {turn_2_command:?}"
    );
    let turn_2_stdout = json["turns"][1]["stdout"].as_str().unwrap();
    assert!(
        turn_2_stdout.contains(TOKEN),
        "expected second turn to recall {TOKEN}, got {turn_2_stdout:?}"
    );
}

#[test]
fn run_uses_named_generic_runner_first_then_next_commands() {
    let store = TestStore::initialized("run-named");
    let first_sink = TestStore::initialized("run-first-sink");
    let next_sink = TestStore::initialized("run-next-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(first_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "next",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(next_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    let output = pseq(&["--store", path_str(store.path()), "run", "Workflow"]);
    assert_success(&output);

    assert_eq!(capture_texts(&first_sink), vec!["A\n".to_owned()]);
    assert_eq!(capture_texts(&next_sink), vec!["B\n".to_owned()]);
    assert_git_clean(store.path());
}

#[test]
fn run_can_reset_named_generic_runner_first_next_per_iteration() {
    let store = TestStore::initialized("run-named-session-scope-iteration");
    let first_sink = TestStore::initialized("run-named-session-scope-iteration-first-sink");
    let next_sink = TestStore::initialized("run-named-session-scope-iteration-next-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(first_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "next",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(next_sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
        "--iterations",
        "2",
        "--session-scope",
        "iteration",
    ]);
    assert_success(&output);

    assert_eq!(
        capture_texts(&first_sink),
        vec!["A\n".to_owned(), "A\n".to_owned()]
    );
    assert_eq!(
        capture_texts(&next_sink),
        vec!["B\n".to_owned(), "B\n".to_owned()]
    );
    assert_git_clean(store.path());
    assert_git_clean(first_sink.path());
    assert_git_clean(next_sink.path());
}

#[test]
fn run_uses_first_command_for_every_turn_when_named_generic_runner_has_no_next_command() {
    let store = TestStore::initialized("run-named-first-only");
    let sink = TestStore::initialized("run-named-first-only-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(sink.path()),
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    let output = pseq(&["--store", path_str(store.path()), "run", "Workflow"]);
    assert_success(&output);

    assert_eq!(
        capture_texts(&sink),
        vec!["A\n".to_owned(), "B\n".to_owned()]
    );
    assert_git_clean(store.path());
    assert_git_clean(sink.path());
}

#[test]
fn run_refuses_changed_store_runner_until_trusted() {
    let store = TestStore::initialized("run-runner-trust");
    let sink = TestStore::initialized("run-runner-trust-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n")]);

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "set",
        "local",
        "first",
        "--",
        pseq_bin(),
        "--version",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "default",
        "local",
    ]));

    fs::write(
        store.path().join("config.toml"),
        format!(
            "version = 1\ndefault_runner = \"local\"\n\n[runners.local]\nfirst = [{:?}, \"capture\", \"import\", \"--stdin\", \"--store\", {:?}]\n",
            pseq_bin(),
            path_str(sink.path())
        ),
    )
    .unwrap();

    let rejected = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
    ]);
    assert_eq!(rejected.status.code(), Some(1));
    assert_eq!(
        stderr_json(&rejected)["error"]["code"],
        "runner_not_trusted"
    );
    assert!(capture_texts(&sink).is_empty());

    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "runner",
        "trust",
        "local",
    ]));
    assert_success(&pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
    ]));
    assert_eq!(capture_texts(&sink), vec!["A\n".to_owned()]);
}
