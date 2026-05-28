use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;

use super::model::{OutputMode, ProcessTurnOutput};
use super::process::run_turn_command;

const INTERNAL_CODEX_JSON_CAPTURE_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct RunnerSession {
    codex: Option<CodexSession>,
}

#[derive(Debug)]
struct CodexSession {
    id: String,
    executable: String,
    resume_prefix_args: Vec<String>,
}

impl RunnerSession {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn run_turn(
        &mut self,
        argv: &[String],
        prompt: &str,
        output_mode: OutputMode,
        max_captured_output: usize,
        needs_continuation: bool,
    ) -> Result<(Vec<String>, ProcessTurnOutput), AppError> {
        if is_plain_codex_exec(argv) || is_codex_exec_resume(argv) || self.codex.is_some() {
            return self.run_codex_turn(
                argv,
                prompt,
                output_mode,
                max_captured_output,
                needs_continuation,
            );
        }

        let process = run_turn_command(argv, prompt, output_mode, max_captured_output)?;
        Ok((argv.to_vec(), process))
    }

    fn run_codex_turn(
        &mut self,
        argv: &[String],
        prompt: &str,
        output_mode: OutputMode,
        max_captured_output: usize,
        needs_continuation: bool,
    ) -> Result<(Vec<String>, ProcessTurnOutput), AppError> {
        let output_path = temporary_output_path("pseq-codex-last-message");
        let command = if let Some(session) = &self.codex {
            codex_resume_command(
                &session.executable,
                &session.resume_prefix_args,
                &session.id,
                &output_path,
            )
        } else {
            codex_first_turn_command(argv, &output_path)
        };

        let internal_capture_limit = INTERNAL_CODEX_JSON_CAPTURE_LIMIT.max(max_captured_output);
        let process = run_turn_command(
            &command,
            prompt,
            OutputMode::Capture,
            internal_capture_limit,
        );
        let output = finalize_codex_output(
            process,
            &command,
            &output_path,
            output_mode,
            max_captured_output,
        )?;

        if self.codex.is_none() && output.process.success && needs_continuation {
            let session_id =
                codex_session_id(output.internal_stdout.as_deref()).ok_or_else(|| {
                    AppError::InvalidRunInvocation {
                        message: "Codex runner did not report a session id for continuation"
                            .to_owned(),
                    }
                })?;
            self.codex = Some(CodexSession {
                id: session_id,
                executable: argv.first().cloned().unwrap_or_else(|| "codex".to_owned()),
                resume_prefix_args: codex_resume_prefix_args(argv),
            });
        }

        Ok((command, output.process))
    }
}

struct CodexTurnOutput {
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

fn finalize_codex_output(
    process: Result<ProcessTurnOutput, AppError>,
    command: &[String],
    output_path: &Path,
    output_mode: OutputMode,
    max_captured_output: usize,
) -> Result<CodexTurnOutput, AppError> {
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
        exit_code: process.exit_code,
        success: process.success,
        stdout: stdout_capture.as_ref().map(|capture| capture.text.clone()),
        stderr: stderr_capture.as_ref().map(|capture| capture.text.clone()),
        stdout_bytes: stdout_capture.as_ref().map(|capture| capture.bytes),
        stderr_bytes: stderr_capture.as_ref().map(|capture| capture.bytes),
        stdout_truncated: stdout_capture.as_ref().map(|capture| capture.truncated),
        stderr_truncated: stderr_capture.as_ref().map(|capture| capture.truncated),
    };

    Ok(CodexTurnOutput {
        process: output,
        internal_stdout: process.stdout,
    })
}

fn capture_for_mode(
    text: &str,
    output_mode: OutputMode,
    max_captured_output: usize,
) -> Option<Capture> {
    (output_mode != OutputMode::Inherit).then(|| truncate_for_capture(text, max_captured_output))
}

struct Capture {
    text: String,
    bytes: usize,
    truncated: bool,
}

fn truncate_for_capture(text: &str, max_captured_output: usize) -> Capture {
    let bytes = text.as_bytes();
    let kept = bytes.len().min(max_captured_output);
    Capture {
        text: String::from_utf8_lossy(&bytes[..kept]).into_owned(),
        bytes: kept,
        truncated: bytes.len() > kept,
    }
}

fn codex_session_id(stdout: Option<&str>) -> Option<String> {
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
    fn codex_session_id_rejects_empty_and_option_like_ids() {
        assert_eq!(
            codex_session_id(Some(r#"{"type":"thread.started","thread_id":""}"#)),
            None
        );
        assert_eq!(
            codex_session_id(Some(r#"{"type":"thread.started","thread_id":"--last"}"#)),
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
        assert!(!is_plain_codex_exec(&review));
        assert!(!is_codex_exec_resume(&review));

        let prompt = argv(&[
            "codex",
            "exec",
            "--add-dir",
            "/repo/.git",
            "--sandbox",
            "workspace-write",
            "-",
        ]);
        assert!(is_plain_codex_exec(&prompt));
        assert!(!is_codex_exec_resume(&prompt));
    }

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }
}
