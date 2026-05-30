use crate::error::AppError;

use super::super::process::run_turn_command;
use super::{HarnessTurnOutcome, HarnessTurnRequest};

pub(super) fn prepare_command(argv: &[String]) -> Vec<String> {
    argv.to_vec()
}

pub(super) fn run_turn(request: &HarnessTurnRequest<'_>) -> Result<HarnessTurnOutcome, AppError> {
    let process = run_turn_command(
        request.argv,
        request.prompt,
        request.output_mode,
        request.max_captured_output,
    )?;
    Ok(HarnessTurnOutcome {
        command: request.argv.to_vec(),
        process,
        next_harness: None,
    })
}
