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
