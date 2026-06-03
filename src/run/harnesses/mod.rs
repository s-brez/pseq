mod codex;
mod generic;

use std::env;
use std::path::Path;

use crate::error::AppError;

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

impl RunnerHarnessSession {
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
        let request = HarnessTurnRequest {
            argv,
            prompt,
            output_mode,
            max_captured_output,
            needs_continuation,
        };
        let harness_kind = RunnerHarnessKind::detect(argv);
        let outcome = match &self.active {
            RunnerHarness::Codex(session) => codex::run_turn(Some(session), &request),
            RunnerHarness::Generic
                if harness_kind == RunnerHarnessKind::Codex
                    && codex::manages_codex_exec_session(argv) =>
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

struct HarnessTurnRequest<'a> {
    argv: &'a [String],
    prompt: &'a str,
    output_mode: OutputMode,
    max_captured_output: usize,
    needs_continuation: bool,
}

struct HarnessTurnOutcome {
    command: Vec<String>,
    process: ProcessTurnOutput,
    next_harness: Option<RunnerHarness>,
}
