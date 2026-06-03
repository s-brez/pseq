use super::*;

#[test]
fn run_rejects_mixed_named_runner_and_ad_hoc_command_before_execution() {
    let store = TestStore::initialized("run-mixed-invocation");
    let sink = TestStore::initialized("run-mixed-invocation-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "codex",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(sink.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        stderr_json(&output)["error"]["code"],
        "invalid_run_invocation"
    );
    assert!(capture_texts(&sink).is_empty());
}

#[test]
fn run_stops_after_unsuccessful_runner_turn_exhausts_retries() {
    let store = TestStore::initialized("run-failure");
    let missing = TestStore::new("run-missing-store");
    create_sequence_with_fragments(&store, "Workflow", &[("First", "A\n"), ("Second", "B\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "doctor",
        "--store",
        path_str(missing.path()),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["success"], false);
    assert!(
        json.get("failed_iteration").is_none(),
        "single-iteration failures should preserve the original JSON shape"
    );
    assert_eq!(json["failed_turn"], 1);
    assert_eq!(json["completed_turns"], 0);
    assert_eq!(json["turns"].as_array().unwrap().len(), 1);
    assert_eq!(json["turns"][0]["exit_code"], 1);
}

#[test]
fn run_retries_failed_runner_turn_by_default() {
    let store = TestStore::initialized("run-default-retry");
    let scratch = TestStore::new("run-default-retry-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let counter = scratch.path().join("attempts");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--retry-delay-ms",
        "0",
        "--",
        "sh",
        "-c",
        r#"counter=$1
count=$(cat "$counter" 2>/dev/null || printf 0)
count=$((count + 1))
printf '%s\n' "$count" > "$counter"
printf 'stdout attempt %s\n' "$count"
printf 'stderr attempt %s\n' "$count" >&2
if [ "$count" -lt 3 ]; then
  exit 17
fi
"#,
        "sh",
        path_str(&counter),
    ]);
    assert_success(&output);
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_to_string(&counter).unwrap().trim(), "3");

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], true);
    assert_eq!(json["completed_turns"], 1);
    assert_eq!(turn["exit_code"], 0);
    assert_eq!(turn["attempt_count"], 3);
    assert_eq!(turn["attempts"].as_array().unwrap().len(), 3);
    assert_eq!(turn["attempts"][0]["exit_code"], 17);
    assert_eq!(turn["attempts"][0]["retryable"], true);
    assert_eq!(turn["attempts"][0]["stdout"], "stdout attempt 1\n");
    assert_eq!(turn["attempts"][0]["stderr"], "stderr attempt 1\n");
    assert_eq!(turn["attempts"][2]["exit_code"], 0);
    assert_eq!(turn["attempts"][2]["retryable"], false);
    assert_eq!(turn["stdout"], "stdout attempt 3\n");
    assert_eq!(turn["stderr"], "stderr attempt 3\n");
}

#[test]
fn run_no_retry_preserves_one_shot_failure_behavior() {
    let store = TestStore::initialized("run-no-retry");
    let scratch = TestStore::new("run-no-retry-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let counter = scratch.path().join("attempts");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--no-retry",
        "--",
        "sh",
        "-c",
        r#"counter=$1
count=$(cat "$counter" 2>/dev/null || printf 0)
count=$((count + 1))
printf '%s\n' "$count" > "$counter"
printf 'failed once\n'
exit 9
"#,
        "sh",
        path_str(&counter),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read_to_string(&counter).unwrap().trim(), "1");

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(json["completed_turns"], 0);
    assert_eq!(turn["exit_code"], 9);
    assert!(turn.get("attempt_count").is_none());
    assert!(turn.get("attempts").is_none());
    assert_eq!(turn["stdout"], "failed once\n");
}

#[test]
fn run_uses_configured_retry_defaults() {
    let store = TestStore::initialized("run-config-retry");
    let scratch = TestStore::new("run-config-retry-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let counter = scratch.path().join("attempts");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);
    fs::write(
        store.path().join("config.toml"),
        "version = 1\n\n[run]\nretries = 1\nretry_delay_ms = 0\n",
    )
    .unwrap();

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        r#"counter=$1
count=$(cat "$counter" 2>/dev/null || printf 0)
count=$((count + 1))
printf '%s\n' "$count" > "$counter"
exit 12
"#,
        "sh",
        path_str(&counter),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&counter).unwrap().trim(), "2");

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(turn["attempt_count"], 2);
    assert_eq!(turn["attempts"].as_array().unwrap().len(), 2);
    assert_eq!(turn["attempts"][0]["retryable"], true);
    assert_eq!(turn["attempts"][1]["retryable"], true);
}

#[test]
fn run_retry_diagnostics_are_factual_and_concise() {
    let store = TestStore::initialized("run-retry-diagnostic");
    let scratch = TestStore::new("run-retry-diagnostic-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let counter = scratch.path().join("attempts");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
        "--retries",
        "1",
        "--retry-delay-ms",
        "0",
        "--",
        "sh",
        "-c",
        r#"counter=$1
count=$(cat "$counter" 2>/dev/null || printf 0)
count=$((count + 1))
printf '%s\n' "$count" > "$counter"
if [ "$count" -eq 1 ]; then
  exit 6
fi
"#,
        "sh",
        path_str(&counter),
    ]);
    assert_success(&output);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("\npseq: runner attempt 1/2 failed at iteration 1 turn 1 with exit code 6"),
        "stderr should include factual retry status, got {stderr:?}"
    );
    assert!(
        stderr.contains("retrying in 0ms"),
        "stderr should include retry delay, got {stderr:?}"
    );
}

#[test]
fn run_failure_uses_pseq_failure_exit_code_and_reports_runner_exit_code() {
    let store = TestStore::initialized("run-failure-exit-code");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        pseq_bin(),
        "not-a-command",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    assert_eq!(json["success"], false);
    assert!(
        json.get("failed_iteration").is_none(),
        "single-iteration failures should preserve the original JSON shape"
    );
    assert_eq!(json["failed_turn"], 1);
    assert_eq!(json["turns"][0]["exit_code"], 2);
}

#[cfg(unix)]
#[test]
fn run_does_not_retry_signal_terminated_runner_process() {
    let store = TestStore::initialized("run-signal-no-retry");
    let scratch = TestStore::new("run-signal-no-retry-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let counter = scratch.path().join("attempts");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--retry-delay-ms",
        "0",
        "--",
        "sh",
        "-c",
        r#"counter=$1
count=$(cat "$counter" 2>/dev/null || printf 0)
count=$((count + 1))
printf '%s\n' "$count" > "$counter"
kill -TERM $$
"#,
        "sh",
        path_str(&counter),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(fs::read_to_string(&counter).unwrap().trim(), "1");

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["signal"], 15);
    assert!(turn.get("attempt_count").is_none());
    assert!(turn.get("attempts").is_none());
}

#[cfg(unix)]
#[test]
fn run_reports_signal_terminated_runner_process() {
    let store = TestStore::initialized("run-signal-termination");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "kill -TERM $$",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(json["failed_turn"], 1);
    assert!(turn["pid"].as_u64().unwrap() > 0);
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["exit_code"], 143);
    assert_eq!(turn["signal"], 15);
    assert_eq!(turn["signal_name"], "SIGTERM");
    assert_eq!(turn["core_dumped"], false);
}

#[cfg(unix)]
#[test]
fn run_isolates_runner_process_group_signals() {
    let store = TestStore::initialized("run-process-group-signal");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq_in_own_process_group(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "kill -TERM 0",
    ]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "pseq should survive runner process-group SIGTERM and report a runner failure"
    );
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(json["failed_turn"], 1);
    assert!(turn["pid"].as_u64().unwrap() > 0);
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["exit_code"], 143);
    assert_eq!(turn["signal"], 15);
    assert_eq!(turn["signal_name"], "SIGTERM");
}

#[cfg(unix)]
#[test]
fn run_forwards_interrupt_to_isolated_runner_process_group() {
    let store = TestStore::initialized("run-forward-interrupt");
    let scratch = TestStore::new("run-forward-interrupt-scratch");
    fs::create_dir_all(scratch.path()).unwrap();
    let runner_pid_path = scratch.path().join("runner.pid");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let runner_script = format!(
        "printf '%s\\n' $$ > {}; sleep 5",
        path_str(&runner_pid_path)
    );
    let child = pseq_command_in_own_process_group(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        &runner_script,
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("pseq binary should spawn");

    let runner_pid = wait_for_pid_file(&runner_pid_path);
    send_signal_to_process_group(child.id(), "INT");

    let output = child.wait_with_output().expect("pseq binary should finish");
    if process_is_alive(runner_pid) {
        send_signal_to_process_group(runner_pid, "TERM");
    }

    assert_eq!(
        output.status.code(),
        Some(1),
        "pseq should survive SIGINT long enough to report the runner failure"
    );
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["exit_code"], 130);
    assert_eq!(turn["signal"], 2);
    assert_eq!(turn["signal_name"], "SIGINT");
    assert!(
        !process_is_alive(runner_pid),
        "runner process should not outlive pseq after forwarded SIGINT"
    );
}

#[cfg(unix)]
#[test]
fn run_failure_diagnostic_reports_signal_status() {
    let store = TestStore::initialized("run-signal-diagnostic");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "run",
        "Workflow",
        "--",
        "sh",
        "-c",
        "kill -TERM $$",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("runner terminated by signal SIGTERM (15) at iteration 1 turn 1"),
        "stderr should report signal termination, got {stderr:?}"
    );
    assert!(
        stderr.contains("exit code 143"),
        "stderr should report shell-compatible exit code, got {stderr:?}"
    );
    assert!(
        stderr.contains("pid "),
        "stderr should include the direct runner pid, got {stderr:?}"
    );
}

#[cfg(unix)]
#[test]
fn run_codex_wrapper_preserves_signal_status() {
    use std::os::unix::fs::PermissionsExt;

    let store = TestStore::initialized("run-codex-signal-status");
    let bin_dir = TestStore::new("run-codex-signal-status-bin");
    fs::create_dir_all(bin_dir.path()).unwrap();
    let fake_codex = bin_dir.path().join("codex");
    fs::write(
        &fake_codex,
        r#"#!/bin/sh
kill -TERM $$
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
            "-",
        ],
        store.path(),
        &[("PATH", &path)],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["exit_code"], 143);
    assert_eq!(turn["signal"], 15);
    assert_eq!(turn["signal_name"], "SIGTERM");
}

#[cfg(unix)]
#[test]
#[ignore = "boots real Codex CLI, requires auth, and signal-terminates a live Codex run"]
fn run_with_real_codex_forwards_sigquit_to_runner_process_group() {
    assert_success(
        &std::process::Command::new("codex")
            .arg("--version")
            .output()
            .expect("real codex CLI should be installed for this ignored test"),
    );

    let store = TestStore::initialized("run-real-codex-forward-quit");
    create_sequence_with_fragments(
        &store,
        "Workflow",
        &[(
            "Only",
            "Automated pseq interrupt-forwarding test. Wait briefly, then reply exactly: SHOULD_NOT_REACH_THIS_RESPONSE\n",
        )],
    );

    let child = pseq_command_in_own_process_group(&[
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
        "--ephemeral",
        "--ignore-user-config",
        "--ignore-rules",
        "--cd",
        path_str(store.path()),
        "--sandbox",
        "read-only",
        "--color",
        "never",
        "-",
    ])
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .expect("pseq binary should spawn");

    let codex_process = wait_for_descendant_process_matching_command(child.id(), "codex exec");
    std::thread::sleep(std::time::Duration::from_secs(2));
    send_signal_to_process_group(child.id(), "QUIT");

    let output = wait_for_child_output_or_kill_process_group(
        child,
        std::time::Duration::from_secs(45),
        "real Codex SIGQUIT forwarding",
        &[codex_process.pgid],
    );
    if process_is_alive(codex_process.pid) {
        try_send_signal_to_process(codex_process.pid, "TERM");
    }

    assert_eq!(
        output.status.code(),
        Some(1),
        "pseq should survive SIGQUIT long enough to report the real Codex runner failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "expected JSON run report on stdout and no stderr, got stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json = stdout_json(&output);
    let turn = &json["turns"][0];
    assert_eq!(json["success"], false);
    assert_eq!(turn["termination"], "signal");
    assert_eq!(turn["exit_code"], 131);
    assert_eq!(turn["signal"], 3);
    assert_eq!(turn["signal_name"], "SIGQUIT");
    assert!(
        wait_for_process_group_to_empty(turn["pid"].as_u64().unwrap() as u32),
        "real Codex runner process group should be empty after pseq reports the interrupted run"
    );
}

#[cfg(unix)]
fn wait_for_pid_file(path: &std::path::Path) -> u32 {
    for _ in 0..100 {
        if let Ok(pid) = fs::read_to_string(path)
            && let Ok(pid) = pid.trim().parse()
        {
            return pid;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    panic!("timed out waiting for runner pid at {}", path.display());
}

#[cfg(unix)]
fn wait_for_descendant_process_matching_command(root_pid: u32, needle: &str) -> PsProcess {
    for _ in 0..300 {
        for process in read_ps_process_table() {
            if process.pid != root_pid
                && process.command.contains(needle)
                && process_has_ancestor(process.pid, root_pid)
            {
                return process;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("timed out waiting for descendant process containing {needle:?}");
}

#[cfg(unix)]
#[derive(Clone, Debug)]
struct PsProcess {
    pid: u32,
    ppid: u32,
    pgid: u32,
    command: String,
}

#[cfg(unix)]
fn read_ps_process_table() -> Vec<PsProcess> {
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid=,ppid=,pgid=,args="])
        .output()
        .expect("ps should run");
    assert_success(&output);

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse().ok()?;
            let ppid = parts.next()?.parse().ok()?;
            let pgid = parts.next()?.parse().ok()?;
            Some(PsProcess {
                pid,
                ppid,
                pgid,
                command: parts.collect::<Vec<_>>().join(" "),
            })
        })
        .collect()
}

#[cfg(unix)]
fn process_has_ancestor(pid: u32, ancestor: u32) -> bool {
    let processes = read_ps_process_table();
    let mut current = pid;
    for _ in 0..64 {
        let Some(process) = processes.iter().find(|process| process.pid == current) else {
            return false;
        };
        if process.ppid == ancestor {
            return true;
        }
        if process.ppid == 0 || process.ppid == current {
            return false;
        }
        current = process.ppid;
    }
    false
}

#[cfg(unix)]
fn wait_for_child_output_or_kill_process_group(
    mut child: std::process::Child,
    timeout: std::time::Duration,
    label: &str,
    extra_pgids_to_kill: &[u32],
) -> std::process::Output {
    let pgid = child.id();
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if child
            .try_wait()
            .expect("pseq wait should succeed")
            .is_some()
        {
            return child
                .wait_with_output()
                .expect("pseq output should be readable");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    try_send_signal_to_process_group(pgid, "TERM");
    for extra_pgid in extra_pgids_to_kill {
        try_send_signal_to_process_group(*extra_pgid, "TERM");
    }
    let cleanup_started = std::time::Instant::now();
    while cleanup_started.elapsed() < std::time::Duration::from_secs(2) {
        if child
            .try_wait()
            .expect("pseq cleanup wait should succeed")
            .is_some()
        {
            for extra_pgid in extra_pgids_to_kill {
                try_send_signal_to_process_group(*extra_pgid, "KILL");
            }
            let output = child
                .wait_with_output()
                .expect("pseq output should be readable after timeout cleanup");
            panic!(
                "{label} timed out\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    try_send_signal_to_process_group(pgid, "KILL");
    for extra_pgid in extra_pgids_to_kill {
        try_send_signal_to_process_group(*extra_pgid, "KILL");
    }
    let kill_started = std::time::Instant::now();
    while kill_started.elapsed() < std::time::Duration::from_secs(2) {
        if child
            .try_wait()
            .expect("pseq kill wait should succeed")
            .is_some()
        {
            let output = child
                .wait_with_output()
                .expect("pseq output should be readable after kill cleanup");
            panic!(
                "{label} timed out and required SIGKILL cleanup\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    panic!("{label} timed out and pseq did not exit after SIGTERM/SIGKILL cleanup");
}

#[cfg(unix)]
fn wait_for_process_group_to_empty(pgid: u32) -> bool {
    for _ in 0..50 {
        if !read_ps_process_table()
            .into_iter()
            .any(|process| process.pgid == pgid)
        {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

#[cfg(unix)]
fn send_signal_to_process_group(pgid: u32, signal: &str) {
    let group = format!("-{pgid}");
    let output = std::process::Command::new("kill")
        .args(["-s", signal, "--", &group])
        .output()
        .expect("kill should run");
    assert_success(&output);
}

#[cfg(unix)]
fn try_send_signal_to_process_group(pgid: u32, signal: &str) {
    let group = format!("-{pgid}");
    let _ = std::process::Command::new("kill")
        .args(["-s", signal, "--", &group])
        .status();
}

#[cfg(unix)]
fn try_send_signal_to_process(pid: u32, signal: &str) {
    let _ = std::process::Command::new("kill")
        .args(["-s", signal, "--", &pid.to_string()])
        .status();
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn run_fails_before_executing_when_turn_rendering_fails() {
    let store = TestStore::initialized("run-render-fail");
    let sink = TestStore::initialized("run-render-fail-sink");
    create_sequence_with_fragments(&store, "Workflow", &[("Template", "{{missing}}\n")]);

    let output = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
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
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(stderr_json(&output)["error"]["code"], "missing_variable");
    assert!(capture_texts(&sink).is_empty());
}

#[test]
fn run_requires_resolvable_runner() {
    let store = TestStore::initialized("run-missing-runner");
    create_sequence_with_fragments(&store, "Workflow", &[("Only", "body\n")]);

    let missing_default = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
    ]);
    assert_eq!(missing_default.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_default)["error"]["code"],
        "default_runner_missing"
    );

    let missing_named = pseq(&[
        "--store",
        path_str(store.path()),
        "--json",
        "run",
        "Workflow",
        "codex",
    ]);
    assert_eq!(missing_named.status.code(), Some(1));
    assert_eq!(
        stderr_json(&missing_named)["error"]["code"],
        "runner_not_found"
    );
}
