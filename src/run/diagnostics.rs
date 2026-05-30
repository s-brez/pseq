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
        eprintln!("pseq: running turn {turn}/{turns} with {runner}: {fragment}");
    } else {
        eprintln!(
            "pseq: running iteration {iteration}/{iterations} turn {turn}/{turns} with {runner}: {fragment}"
        );
    }
}

pub(super) fn write_runner_failure_diagnostic(
    iteration: usize,
    turn: usize,
    process: &ProcessTurnOutput,
) {
    match process.termination {
        ProcessTermination::Exit => {
            eprintln!(
                "pseq: runner exited unsuccessfully at iteration {iteration} turn {turn} with exit code {} (pid {})",
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
                "pseq: runner terminated by signal {signal_label} at iteration {iteration} turn {turn} (pid {}, exit code {})",
                process.pid, process.exit_code
            );
        }
        ProcessTermination::Unknown => {
            eprintln!(
                "pseq: runner ended without an exit code or signal at iteration {iteration} turn {turn} (pid {}, exit code {})",
                process.pid, process.exit_code
            );
        }
    }
}
