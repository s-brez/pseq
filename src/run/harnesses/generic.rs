use crate::error::AppError;
use crate::turn_settings::{REASONING_EFFORT_KEY, fragment_setting_label};

use super::super::process::run_turn_command;
use super::{HarnessTurnOutcome, HarnessTurnRequest};

pub(super) fn prepare_command(argv: &[String]) -> Vec<String> {
    argv.to_vec()
}

pub(super) fn validate_turn_settings(
    settings: crate::turn_settings::TurnRuntimeSettings,
    fragment: &crate::render::RenderedTurnFragment,
    argv: &[String],
) -> Result<(), AppError> {
    if !settings.is_empty() {
        return Err(AppError::InvalidRunInvocation {
            message: format!(
                "{} declares {REASONING_EFFORT_KEY}, but runner command {:?} is not a recognized adapter that supports it",
                fragment_setting_label(fragment),
                argv
            ),
        });
    }
    Ok(())
}

pub(super) fn run_turn(request: &HarnessTurnRequest<'_>) -> Result<HarnessTurnOutcome, AppError> {
    validate_turn_settings(request.settings, request.fragment, request.argv)?;
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
