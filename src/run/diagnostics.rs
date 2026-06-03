use super::model::{ProcessTermination, ProcessTurnOutput};

pub(super) fn should_write_diagnostics(capture_output: bool, quiet: bool) -> bool {
    !capture_output && !quiet
}

pub(super) fn write_turn_diagnostic(
    iteration: usize,
    iterations: usize,
    turn: usize,
    turns: usize,
    runner: &str,
    fragment: &str,
) {
    if iterations == 1 {
        eprintln!("\npseq: running turn {turn}/{turns} with {runner}: {fragment}");
    } else {
        eprintln!(
            "\npseq: running iteration {iteration}/{iterations} turn {turn}/{turns} with {runner}: {fragment}"
        );
    }
}

pub(super) fn write_runner_failure_diagnostic(
    iteration: usize,
    turn: usize,
    process: &ProcessTurnOutput,
    attempts: usize,
) {
    let attempt_context = attempt_context(attempts);
    match process.termination {
        ProcessTermination::Exit => {
            eprintln!(
                "\npseq: runner exited unsuccessfully{attempt_context} at iteration {iteration} turn {turn} with exit code {} (pid {})",
                process.exit_code, process.pid
            );
        }
        ProcessTermination::Signal => {
            let signal = process
                .signal
                .map(|signal| signal.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let signal_label = process
                .signal_name
                .map(|name| format!("{name} ({signal})"))
                .unwrap_or(signal);
            eprintln!(
                "\npseq: runner terminated by signal {signal_label}{attempt_context} at iteration {iteration} turn {turn} (pid {}, exit code {})",
                process.pid, process.exit_code
            );
        }
        ProcessTermination::Unknown => {
            eprintln!(
                "\npseq: runner ended without an exit code or signal{attempt_context} at iteration {iteration} turn {turn} (pid {}, exit code {})",
                process.pid, process.exit_code
            );
        }
    }
}

fn attempt_context(attempts: usize) -> String {
    match attempts {
        0 | 1 => String::new(),
        2 => " after 2 attempts".to_owned(),
        attempts => format!(" after {attempts} attempts"),
    }
}

pub(super) fn write_runner_retry_diagnostic(
    iteration: usize,
    turn: usize,
    attempt: usize,
    attempts: usize,
    retry_delay_ms: u64,
    process: &ProcessTurnOutput,
) {
    eprintln!(
        "\npseq: runner attempt {attempt}/{attempts} failed at iteration {iteration} turn {turn} with {}; retrying in {retry_delay_ms}ms",
        process_status(process)
    );
}

fn process_status(process: &ProcessTurnOutput) -> String {
    match process.termination {
        ProcessTermination::Exit => {
            format!("exit code {} (pid {})", process.exit_code, process.pid)
        }
        ProcessTermination::Signal => {
            let signal = process
                .signal
                .map(|signal| signal.to_string())
                .unwrap_or_else(|| "unknown".to_owned());
            let signal_label = process
                .signal_name
                .map(|name| format!("{name} ({signal})"))
                .unwrap_or(signal);
            format!(
                "signal {signal_label} (pid {}, exit code {})",
                process.pid, process.exit_code
            )
        }
        ProcessTermination::Unknown => {
            format!(
                "unknown process status (pid {}, exit code {})",
                process.pid, process.exit_code
            )
        }
    }
}
