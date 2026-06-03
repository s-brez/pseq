mod codex;
mod generic;

use std::env;
use std::path::Path;

use crate::error::AppError;
use crate::render::RenderedTurnFragment;
use crate::turn_settings::TurnRuntimeSettings;

use super::model::{OutputMode, ProcessTurnOutput};

#[derive(Debug, Default)]
pub(super) struct RunnerHarnessSession {
    active: RunnerHarness,
}

#[derive(Debug, Default)]
enum RunnerHarness {
    #[default]
    Generic,
    Codex(codex::CodexSession),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerHarnessKind {
    Generic,
    Codex,
}

impl RunnerHarnessKind {
    fn detect(argv: &[String]) -> Self {
        if codex::matches_codex_exec_command(argv) {
            Self::Codex
        } else {
            Self::Generic
        }
    }

    fn prepare_command(self, argv: &[String], current_dir: &Path) -> Vec<String> {
        match self {
            Self::Generic => generic::prepare_command(argv),
            Self::Codex => codex::prepare_command(argv, current_dir),
        }
    }
}

pub(super) fn prepare_runner_command(argv: &[String]) -> Result<Vec<String>, AppError> {
    let current_dir = env::current_dir().map_err(|source| AppError::CurrentDir { source })?;
    Ok(RunnerHarnessKind::detect(argv).prepare_command(argv, &current_dir))
}

pub(super) fn runner_command_supports_turn_settings(argv: &[String]) -> bool {
    RunnerHarnessKind::detect(argv) == RunnerHarnessKind::Codex
        && codex::manages_codex_exec_session(argv)
}

pub(super) fn validate_command_turn_settings(
    argv: &[String],
    settings: TurnRuntimeSettings,
    fragment: &RenderedTurnFragment,
) -> Result<(), AppError> {
    if runner_command_supports_turn_settings(argv) {
        codex::validate_turn_settings(settings, fragment)
    } else {
        generic::validate_turn_settings(settings, fragment, argv)
    }
}

pub(super) fn validate_active_codex_turn_settings(
    settings: TurnRuntimeSettings,
    fragment: &RenderedTurnFragment,
) -> Result<(), AppError> {
    codex::validate_turn_settings(settings, fragment)
}

impl RunnerHarnessSession {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn run_turn(
        &mut self,
        request: HarnessTurnRequest<'_>,
    ) -> Result<(Vec<String>, ProcessTurnOutput), AppError> {
        let harness_kind = RunnerHarnessKind::detect(request.argv);
        let outcome = match &self.active {
            RunnerHarness::Codex(session) => codex::run_turn(Some(session), &request),
            RunnerHarness::Generic
                if harness_kind == RunnerHarnessKind::Codex
                    && codex::manages_codex_exec_session(request.argv) =>
            {
                codex::run_turn(None, &request)
            }
            RunnerHarness::Generic => generic::run_turn(&request),
        }?;

        if outcome.process.success
            && let Some(next_harness) = outcome.next_harness
        {
            self.active = next_harness;
        }

        Ok((outcome.command, outcome.process))
    }
}

pub(super) struct HarnessTurnRequest<'a> {
    pub(super) argv: &'a [String],
    pub(super) prompt: &'a str,
    pub(super) fragment: &'a RenderedTurnFragment,
    pub(super) settings: TurnRuntimeSettings,
    pub(super) output_mode: OutputMode,
    pub(super) max_captured_output: usize,
    pub(super) needs_continuation: bool,
}

struct HarnessTurnOutcome {
    command: Vec<String>,
    process: ProcessTurnOutput,
    next_harness: Option<RunnerHarness>,
}
