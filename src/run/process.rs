use std::io::{self, Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use crate::error::AppError;

use super::model::*;

#[derive(Debug)]
struct ClassifiedProcessStatus {
    termination: ProcessTermination,
    exit_code: i32,
    signal: Option<i32>,
    signal_name: Option<&'static str>,
    core_dumped: Option<bool>,
}

pub(super) fn run_turn_command(
    argv: &[String],
    prompt: &str,
    output_mode: OutputMode,
    max_captured_output: usize,
) -> Result<ProcessTurnOutput, AppError> {
    let command_label = argv.to_vec();
    let executable = argv.first().ok_or_else(|| AppError::RunnerCommandEmpty {
        context: "runner command".to_owned(),
    })?;
    let mut command = Command::new(executable);
    command.args(&argv[1..]);
    command.stdin(Stdio::piped());
    if output_mode == OutputMode::Inherit {
        command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    }

    let mut child = command.spawn().map_err(|source| AppError::RunnerSpawn {
        command: command_label.clone(),
        source,
    })?;
    let pid = child.id();
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::RunnerStdinUnavailable {
            command: command_label.clone(),
        })?;
    let stdin_writer = spawn_stdin_writer(stdin, prompt.as_bytes().to_vec(), command_label.clone());

    let stdout_reader = if output_mode != OutputMode::Inherit {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::RunnerOutputUnavailable {
                command: command_label.clone(),
                stream: "stdout",
            })?;
        Some(spawn_output_reader(
            stdout,
            max_captured_output,
            command_label.clone(),
            "stdout",
            output_mode.tee_stream(TeeStream::Stdout),
        ))
    } else {
        None
    };
    let stderr_reader = if output_mode != OutputMode::Inherit {
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::RunnerOutputUnavailable {
                command: command_label.clone(),
                stream: "stderr",
            })?;
        Some(spawn_output_reader(
            stderr,
            max_captured_output,
            command_label.clone(),
            "stderr",
            output_mode.tee_stream(TeeStream::Stderr),
        ))
    } else {
        None
    };

    let status = child.wait().map_err(|source| AppError::RunnerWait {
        command: command_label.clone(),
        source,
    })?;
    stdin_writer
        .join()
        .map_err(|_| AppError::RunnerWriteStdin {
            command: command_label.clone(),
            source: io::Error::other("stdin writer thread panicked"),
        })??;
    let stdout = stdout_reader.map(join_output_reader).transpose()?;
    let stderr = stderr_reader.map(join_output_reader).transpose()?;
    let status = classify_process_status(status);

    Ok(ProcessTurnOutput {
        pid,
        termination: status.termination,
        exit_code: status.exit_code,
        success: status.termination == ProcessTermination::Exit && status.exit_code == 0,
        signal: status.signal,
        signal_name: status.signal_name,
        core_dumped: status.core_dumped,
        stdout: stdout.as_ref().map(|output| output.text.clone()),
        stderr: stderr.as_ref().map(|output| output.text.clone()),
        stdout_bytes: stdout.as_ref().map(|output| output.bytes),
        stderr_bytes: stderr.as_ref().map(|output| output.bytes),
        stdout_truncated: stdout.as_ref().map(|output| output.truncated),
        stderr_truncated: stderr.as_ref().map(|output| output.truncated),
    })
}

fn spawn_stdin_writer(
#[cfg(unix)]
fn classify_process_status(status: ExitStatus) -> ClassifiedProcessStatus {
    use std::os::unix::process::ExitStatusExt;

    if let Some(exit_code) = status.code() {
        return ClassifiedProcessStatus {
            termination: ProcessTermination::Exit,
            exit_code,
            signal: None,
            signal_name: None,
            core_dumped: None,
        };
    }

    if let Some(signal) = status.signal() {
        return ClassifiedProcessStatus {
            termination: ProcessTermination::Signal,
            exit_code: signal.checked_add(128).unwrap_or(1),
            signal: Some(signal),
            signal_name: unix_signal_name(signal),
            core_dumped: Some(status.core_dumped()),
        };
    }

    ClassifiedProcessStatus {
        termination: ProcessTermination::Unknown,
        exit_code: 1,
        signal: None,
        signal_name: None,
        core_dumped: None,
    }
}

#[cfg(not(unix))]
fn classify_process_status(status: ExitStatus) -> ClassifiedProcessStatus {
    if let Some(exit_code) = status.code() {
        return ClassifiedProcessStatus {
            termination: ProcessTermination::Exit,
            exit_code,
            signal: None,
            signal_name: None,
            core_dumped: None,
        };
    }

    ClassifiedProcessStatus {
        termination: ProcessTermination::Unknown,
        exit_code: 1,
        signal: None,
        signal_name: None,
        core_dumped: None,
    }
}

#[cfg(unix)]
fn unix_signal_name(signal: i32) -> Option<&'static str> {
    match signal {
        1 => Some("SIGHUP"),
        2 => Some("SIGINT"),
        3 => Some("SIGQUIT"),
        4 => Some("SIGILL"),
        5 => Some("SIGTRAP"),
        6 => Some("SIGABRT"),
        8 => Some("SIGFPE"),
        9 => Some("SIGKILL"),
        11 => Some("SIGSEGV"),
        13 => Some("SIGPIPE"),
        14 => Some("SIGALRM"),
        15 => Some("SIGTERM"),
        _ => None,
    }
}

    mut stdin: impl Write + Send + 'static,
    prompt: Vec<u8>,
    command: Vec<String>,
) -> thread::JoinHandle<Result<(), AppError>> {
    thread::spawn(move || {
        if let Err(source) = stdin.write_all(&prompt)
            && source.kind() != io::ErrorKind::BrokenPipe
        {
            return Err(AppError::RunnerWriteStdin { command, source });
        }
        Ok(())
    })
}

fn spawn_output_reader(
    mut output: impl Read + Send + 'static,
    max_capture: usize,
    command: Vec<String>,
    stream: &'static str,
    tee: Option<TeeStream>,
) -> thread::JoinHandle<Result<CapturedStream, AppError>> {
    thread::spawn(move || {
        let mut captured = Vec::new();
        let mut truncated = false;
        let mut buffer = [0; 8192];

        loop {
            let read = output
                .read(&mut buffer)
                .map_err(|source| AppError::RunnerReadOutput {
                    command: command.clone(),
                    stream,
                    source,
                })?;
            if read == 0 {
                break;
            }

            if let Some(tee) = tee {
                write_tee_stream(tee, &buffer[..read])?;
            }

            let remaining = max_capture.saturating_sub(captured.len());
            if remaining > 0 {
                let kept = remaining.min(read);
                captured.extend_from_slice(&buffer[..kept]);
            }
            if read > remaining {
                truncated = true;
            }
        }

        Ok(CapturedStream {
            bytes: captured.len(),
            text: String::from_utf8_lossy(&captured).into_owned(),
            truncated,
        })
    })
}

fn join_output_reader(
    reader: thread::JoinHandle<Result<CapturedStream, AppError>>,
) -> Result<CapturedStream, AppError> {
    reader.join().map_err(|_| AppError::RunnerReadOutput {
        command: Vec::new(),
        stream: "output",
        source: io::Error::other("output reader thread panicked"),
    })?
}

fn write_tee_stream(stream: TeeStream, bytes: &[u8]) -> Result<(), AppError> {
    match stream {
        TeeStream::Stdout => {
            let mut stdout = io::stdout().lock();
            stdout
                .write_all(bytes)
                .and_then(|_| stdout.flush())
                .map_err(|source| AppError::WriteOutput { source })
        }
        TeeStream::Stderr => {
            let mut stderr = io::stderr().lock();
            stderr
                .write_all(bytes)
                .and_then(|_| stderr.flush())
                .map_err(|source| AppError::WriteOutput { source })
        }
    }
}
