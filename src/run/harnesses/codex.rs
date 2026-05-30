use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;

use super::super::model::{OutputMode, ProcessTurnOutput};
use super::super::process::run_turn_command;
use super::{HarnessTurnOutcome, HarnessTurnRequest, RunnerHarness};

const INTERNAL_CODEX_JSON_CAPTURE_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct CodexSession {
    id: String,
    executable: String,
    resume_prefix_args: Vec<String>,
}

pub(super) fn matches_codex_exec_command(argv: &[String]) -> bool {
    is_codex_exec(argv)
}

pub(super) fn manages_codex_exec_session(argv: &[String]) -> bool {
    is_plain_codex_exec(argv) || is_codex_exec_resume(argv)
}

pub(super) fn prepare_command(argv: &[String], current_dir: &Path) -> Vec<String> {
    if codex_exec_has_sandbox_escape(argv) || codex_exec_sandbox_mode(argv) == Some("read-only") {
        return argv.to_vec();
    }

    let workspace_dir = codex_exec_workspace_dir(argv, current_dir);
    let Some(git_dirs) = git_metadata_dirs(&workspace_dir) else {
        return argv.to_vec();
    };

    let mut command = Vec::with_capacity(argv.len() + git_dirs.len() * 2);
    command.extend_from_slice(&argv[..2]);
    for git_dir in git_dirs {
        if !has_add_dir(argv, &git_dir, &workspace_dir) {
            command.push("--add-dir".to_owned());
            command.push(path_arg(&git_dir));
        }
    }
    command.extend_from_slice(&argv[2..]);
    command
}

pub(super) fn run_turn(
    session: Option<&CodexSession>,
    request: &HarnessTurnRequest<'_>,
) -> Result<HarnessTurnOutcome, AppError> {
    let output_path = temporary_output_path("pseq-codex-last-message");
    let command = if let Some(session) = session {
        codex_resume_command(
            &session.executable,
            &session.resume_prefix_args,
            &session.id,
            &output_path,
        )
    } else {
        codex_first_turn_command(request.argv, &output_path)
    };

    let internal_capture_limit = INTERNAL_CODEX_JSON_CAPTURE_LIMIT.max(request.max_captured_output);
    let process = run_turn_command(
        &command,
        request.prompt,
        OutputMode::Capture,
        internal_capture_limit,
    );
    let output = finalize_codex_turn_output(
        process,
        &command,
        &output_path,
        request.output_mode,
        request.max_captured_output,
    )?;

    let next_harness = if session.is_none() && output.process.success && request.needs_continuation
    {
        let session_id = codex_session_id_from_jsonl_stdout(output.internal_stdout.as_deref())
            .ok_or_else(|| AppError::InvalidRunInvocation {
                message: "Codex runner did not report a session id for continuation".to_owned(),
            })?;
        Some(RunnerHarness::Codex(CodexSession {
            id: session_id,
            executable: request
                .argv
                .first()
                .cloned()
                .unwrap_or_else(|| "codex".to_owned()),
            resume_prefix_args: codex_resume_prefix_args(request.argv),
        }))
    } else {
        None
    };

    Ok(HarnessTurnOutcome {
        command,
        process: output.process,
        next_harness,
    })
}

struct CodexFinalizedTurnOutput {
    process: ProcessTurnOutput,
    internal_stdout: Option<String>,
}

fn codex_first_turn_command(argv: &[String], output_path: &Path) -> Vec<String> {
    let mut command = argv.to_vec();
    insert_codex_json_output_args(&mut command, 2, output_path);
    command
}

fn codex_resume_command(
    executable: &str,
    resume_prefix_args: &[String],
    session_id: &str,
    output_path: &Path,
) -> Vec<String> {
    let mut command = vec![executable.to_owned(), "exec".to_owned()];
    command.extend_from_slice(resume_prefix_args);
    command.push("resume".to_owned());
    let insert_at = command.len();
    insert_codex_json_output_args(&mut command, insert_at, output_path);
    command.push(session_id.to_owned());
    command.push("-".to_owned());
    command
}

fn insert_codex_json_output_args(command: &mut Vec<String>, insert_at: usize, output_path: &Path) {
    remove_codex_output_last_message_args(command);
    let mut args = Vec::new();
    if !has_flag(command, "--json") {
        args.push("--json".to_owned());
    }
    args.push("--output-last-message".to_owned());
    args.push(output_path.to_string_lossy().into_owned());
    command.splice(insert_at..insert_at, args);
}

fn finalize_codex_turn_output(
    process: Result<ProcessTurnOutput, AppError>,
    command: &[String],
    output_path: &Path,
    output_mode: OutputMode,
    max_captured_output: usize,
) -> Result<CodexFinalizedTurnOutput, AppError> {
    let process = process?;
    let final_message = match fs::read_to_string(output_path) {
        Ok(text) => text,
        Err(source) if process.success => {
            let _ = fs::remove_file(output_path);
            return Err(AppError::RunnerReadOutput {
                command: command.to_vec(),
                stream: "codex final message",
                source,
            });
        }
        Err(_) => String::new(),
    };
    let _ = fs::remove_file(output_path);

    if output_mode != OutputMode::Capture {
        write_to_stdout(&final_message)?;
        if let Some(stderr) = &process.stderr {
            write_to_stderr(stderr)?;
        }
    }

    let stdout_capture = capture_for_mode(&final_message, output_mode, max_captured_output);
    let stderr_capture = process
        .stderr
        .as_deref()
        .map(|stderr| truncate_for_capture(stderr, max_captured_output));

    let output = ProcessTurnOutput {
        pid: process.pid,
        termination: process.termination,
        exit_code: process.exit_code,
        success: process.success,
        signal: process.signal,
        signal_name: process.signal_name,
        core_dumped: process.core_dumped,
        stdout: stdout_capture.as_ref().map(|capture| capture.text.clone()),
        stderr: stderr_capture.as_ref().map(|capture| capture.text.clone()),
        stdout_bytes: stdout_capture.as_ref().map(|capture| capture.bytes),
        stderr_bytes: stderr_capture.as_ref().map(|capture| capture.bytes),
        stdout_truncated: stdout_capture.as_ref().map(|capture| capture.truncated),
        stderr_truncated: stderr_capture.as_ref().map(|capture| capture.truncated),
    };

    Ok(CodexFinalizedTurnOutput {
        process: output,
        internal_stdout: process.stdout,
    })
}

fn capture_for_mode(
    text: &str,
    output_mode: OutputMode,
    max_captured_output: usize,
) -> Option<TruncatedTextCapture> {
    (output_mode != OutputMode::Inherit).then(|| truncate_for_capture(text, max_captured_output))
}

struct TruncatedTextCapture {
    text: String,
    bytes: usize,
    truncated: bool,
}

fn truncate_for_capture(text: &str, max_captured_output: usize) -> TruncatedTextCapture {
    let bytes = text.as_bytes();
    let kept = bytes.len().min(max_captured_output);
    TruncatedTextCapture {
        text: String::from_utf8_lossy(&bytes[..kept]).into_owned(),
        bytes: kept,
        truncated: bytes.len() > kept,
    }
}

fn codex_session_id_from_jsonl_stdout(stdout: Option<&str>) -> Option<String> {
    let stdout = stdout?;
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) == Some("session_meta")
            && let Some(id) = value
                .get("payload")
                .and_then(|payload| payload.get("id"))
                .and_then(Value::as_str)
        {
            return valid_codex_session_id(id);
        }
        if value.get("type").and_then(Value::as_str) == Some("thread.started")
            && let Some(id) = value.get("thread_id").and_then(Value::as_str)
        {
            return valid_codex_session_id(id);
        }
        if let Some(id) = value.get("session_id").and_then(Value::as_str) {
            return valid_codex_session_id(id);
        }
    }
    None
}

fn valid_codex_session_id(id: &str) -> Option<String> {
    let id = id.trim();
    (!id.is_empty() && !id.starts_with('-')).then(|| id.to_owned())
}

fn codex_resume_prefix_args(argv: &[String]) -> Vec<String> {
    let mut args = Vec::new();
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if matches!(arg.as_str(), "resume" | "review" | "help" | "-" | "--") {
            break;
        }
        if !arg.starts_with('-') {
            break;
        }
        if arg == "--json" {
            index += 1;
            continue;
        }
        if arg == "--output-last-message" || arg == "-o" {
            index += 2;
            continue;
        }
        if arg.starts_with("--output-last-message=") {
            index += 1;
            continue;
        }

        args.push(arg.clone());
        if codex_exec_option_takes_value(arg)
            && !arg.contains('=')
            && let Some(value) = argv.get(index + 1)
        {
            args.push(value.clone());
            index += 2;
            continue;
        }
        index += 1;
    }
    args
}

fn codex_exec_option_takes_value(arg: &str) -> bool {
    matches!(
        arg,
        "-c" | "--config"
            | "--enable"
            | "--disable"
            | "-i"
            | "--image"
            | "-m"
            | "--model"
            | "--local-provider"
            | "-p"
            | "--profile"
            | "--profile-v2"
            | "-s"
            | "--sandbox"
            | "-C"
            | "--cd"
            | "--add-dir"
            | "--output-schema"
            | "--color"
    )
}

fn codex_exec_has_sandbox_escape(argv: &[String]) -> bool {
    argv.iter()
        .any(|arg| arg == "--dangerously-bypass-approvals-and-sandbox")
}

fn codex_exec_sandbox_mode(argv: &[String]) -> Option<&str> {
    let mut mode = None;
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--sandbox" || arg == "-s" {
            if let Some(value) = argv.get(index + 1) {
                mode = Some(value.as_str());
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--sandbox=") {
            mode = Some(value);
        }
        index += 1;
    }
    mode
}

fn codex_exec_workspace_dir(argv: &[String], current_dir: &Path) -> PathBuf {
    let mut workspace_dir = current_dir.to_path_buf();
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if matches!(arg.as_str(), "resume" | "review" | "help" | "-" | "--") {
            break;
        }
        if arg == "-C" || arg == "--cd" {
            if let Some(value) = argv.get(index + 1) {
                workspace_dir = resolve_command_path(value, current_dir);
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--cd=") {
            workspace_dir = resolve_command_path(value, current_dir);
        }
        index += 1;
    }
    workspace_dir
}

fn git_metadata_dirs(workspace_dir: &Path) -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_dir)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let path = PathBuf::from(line);
        if !dirs
            .iter()
            .any(|existing| same_command_path(existing, &path))
        {
            dirs.push(path);
        }
    }
    (!dirs.is_empty()).then_some(dirs)
}

fn has_add_dir(argv: &[String], git_dir: &Path, workspace_dir: &Path) -> bool {
    let mut index = 0;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--add-dir" {
            if argv
                .get(index + 1)
                .map(|value| resolve_command_path(value, workspace_dir))
                .is_some_and(|path| same_command_path(&path, git_dir))
            {
                return true;
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--add-dir=")
            && same_command_path(&resolve_command_path(value, workspace_dir), git_dir)
        {
            return true;
        }
        index += 1;
    }
    false
}

fn resolve_command_path(value: &str, base: &Path) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn same_command_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> String {
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn is_plain_codex_exec(argv: &[String]) -> bool {
    is_codex_exec(argv)
        && !matches!(
            codex_exec_first_positional(argv),
            Some("resume" | "review" | "help")
        )
}

fn is_codex_exec_resume(argv: &[String]) -> bool {
    is_codex_exec(argv) && codex_exec_first_positional(argv) == Some("resume")
}

fn is_codex_exec(argv: &[String]) -> bool {
    let Some(command) = argv.first() else {
        return false;
    };
    is_command_name(command, "codex") && argv.get(1).is_some_and(|arg| arg == "exec")
}

fn codex_exec_first_positional(argv: &[String]) -> Option<&str> {
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--" {
            return None;
        }
        if !arg.starts_with('-') || arg == "-" {
            return Some(arg);
        }
        if codex_exec_option_takes_value(arg) && !arg.contains('=') {
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

fn has_flag(argv: &[String], flag: &str) -> bool {
    argv.iter().any(|arg| arg == flag)
}

fn remove_codex_output_last_message_args(command: &mut Vec<String>) {
    let mut index = 0;
    while index < command.len() {
        let arg = &command[index];
        if arg == "--output-last-message" || arg == "-o" {
            let end = (index + 2).min(command.len());
            command.drain(index..end);
            continue;
        }
        if arg.starts_with("--output-last-message=") {
            command.remove(index);
            continue;
        }
        index += 1;
    }
}

fn temporary_output_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.txt", Uuid::new_v4()))
}

fn is_command_name(command: &str, expected: &str) -> bool {
    let Some(file_name) = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return false;
    };
    let command_name = file_name.strip_suffix(".exe").unwrap_or(file_name);
    command_name.eq_ignore_ascii_case(expected)
}

fn write_to_stdout(text: &str) -> Result<(), AppError> {
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.flush())
        .map_err(|source| AppError::WriteOutput { source })
}

fn write_to_stderr(text: &str) -> Result<(), AppError> {
    let mut stderr = io::stderr().lock();
    stderr
        .write_all(text.as_bytes())
        .and_then(|_| stderr.flush())
        .map_err(|source| AppError::WriteOutput { source })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_session_id_from_jsonl_stdout_rejects_empty_and_option_like_ids() {
        assert_eq!(
            codex_session_id_from_jsonl_stdout(Some(r#"{"type":"thread.started","thread_id":""}"#)),
            None
        );
        assert_eq!(
            codex_session_id_from_jsonl_stdout(Some(
                r#"{"type":"thread.started","thread_id":"--last"}"#
            )),
            None
        );
    }

    #[test]
    fn codex_exec_subcommand_detection_skips_prepared_options() {
        let review = argv(&[
            "codex",
            "exec",
            "--add-dir",
            "/repo/.git",
            "--sandbox",
            "workspace-write",
            "review",
        ]);
        assert!(matches_codex_exec_command(&review));
        assert!(!is_plain_codex_exec(&review));
        assert!(!is_codex_exec_resume(&review));
        assert!(!manages_codex_exec_session(&review));

        let prompt = argv(&[
            "codex",
            "exec",
            "--add-dir",
            "/repo/.git",
            "--sandbox",
            "workspace-write",
            "-",
        ]);
        assert!(matches_codex_exec_command(&prompt));
        assert!(is_plain_codex_exec(&prompt));
        assert!(!is_codex_exec_resume(&prompt));
        assert!(manages_codex_exec_session(&prompt));
    }

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }
}
