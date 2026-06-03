use std::path::Path;
use std::time::Duration;

use crate::error::AppError;
use crate::render;
use crate::store;

use super::diagnostics::{
    should_write_diagnostics, write_runner_failure_diagnostic, write_runner_retry_diagnostic,
    write_turn_diagnostic,
};
use super::harnesses::{RunnerHarnessSession, prepare_runner_command};
use super::model::{OutputMode, ProcessTermination, ProcessTurnOutput};
use super::options::{
    RunSettings, feedback_variable, load_base_variables, load_feedback_seed, output_mode,
    resolve_run_settings, resolve_runner, validate_options,
};
use super::types::*;

pub fn run_sequence(
    store_path: &Path,
    sequence_reference: &str,
    options: RunOptions<'_>,
) -> Result<(i32, RunOutput), AppError> {
    store::require_valid_store(store_path)?;
    validate_options(&options)?;

    let feedback_variable = feedback_variable(&options)?;
    let run_settings = resolve_run_settings(store_path, &options)?;
    let runner = resolve_runner(store_path, &options)?;
    let base_variables = load_base_variables(&options, feedback_variable.as_deref())?;
    let mut previous_feedback = load_feedback_seed(&options)?;
    let render_sequence = render::load_current_sequence(store_path, sequence_reference)?;

    let mut first_iteration_variables = base_variables.clone();
    if let Some(variable) = &feedback_variable {
        first_iteration_variables.insert(variable.clone(), previous_feedback.clone());
    }
    let first_sequence =
        render::render_sequence_turns(&render_sequence, &first_iteration_variables)?;
    if options.feedback_from.is_some() && first_sequence.turns.is_empty() {
        return Err(AppError::InvalidRunInvocation {
            message: "feedback requires a sequence with at least one turn".to_owned(),
        });
    }

    let sequence_summary = RunSequenceSummary {
        id: first_sequence.id.clone(),
        name: first_sequence.name.clone(),
        path: first_sequence.path.clone(),
    };
    let turns_per_iteration = first_sequence.turns.len();
    let total_turns = turns_per_iteration * options.iterations;
    let output_mode = output_mode(&options, run_settings);
    let include_output =
        options.capture_output || run_settings.preserve_output || options.feedback_from.is_some();
    let write_diagnostics = should_write_diagnostics(options.capture_output, options.quiet);
    let mut run_session = RunnerHarnessSession::new();

    let mut turns = Vec::new();
    let mut failed_turn = None;
    let mut failed_iteration = None;
    'iterations: for iteration in 1..=options.iterations {
        let mut iteration_session = RunnerHarnessSession::new();
        let sequence = if iteration == 1 || options.feedback_from.is_none() {
            first_sequence.clone()
        } else {
            let mut variables = base_variables.clone();
            if let Some(variable) = &feedback_variable {
                variables.insert(variable.clone(), previous_feedback.clone());
            }
            render::render_sequence_turns(&render_sequence, &variables)?
        };

        let mut iteration_feedback = None;
        for turn in &sequence.turns {
            let global_turn_index = turns.len() + 1;
            let scoped_turn_index = match options.session_scope {
                SessionScope::Run => global_turn_index,
                SessionScope::Iteration => turn.index,
            };
            let has_later_turn = match options.session_scope {
                SessionScope::Run => global_turn_index < total_turns,
                SessionScope::Iteration => turn.index < sequence.turns.len(),
            };
            let command = prepare_runner_command(runner.command_for_turn(scoped_turn_index))?;
            if write_diagnostics {
                write_turn_diagnostic(
                    iteration,
                    options.iterations,
                    turn.index,
                    sequence.turns.len(),
                    &runner.label(),
                    &turn.fragment.name,
                );
            }

            let runner_session = match options.session_scope {
                SessionScope::Run => &mut run_session,
                SessionScope::Iteration => &mut iteration_session,
            };
            let attempts = run_turn_attempts(
                runner_session,
                RunAttemptRequest {
                    command: &command,
                    prompt: &turn.text,
                    output_mode,
                    max_captured_output: options.max_captured_output,
                    has_later_turn,
                    settings: run_settings,
                    diagnostics: RetryDiagnosticContext {
                        enabled: write_diagnostics,
                        iteration,
                        turn: turn.index,
                    },
                },
            )?;
            let final_attempt = attempts
                .last()
                .expect("run_turn_attempts always returns one attempt");
            let process = &final_attempt.process;
            let is_feedback_turn =
                options.feedback_from.is_some() && turn.index == sequence.turns.len();
            let feedback_stdout = if is_feedback_turn {
                process.stdout.clone()
            } else {
                None
            };
            let stdout_truncated = process.stdout_truncated;

            turns.push(run_turn_output(
                options.iterations,
                iteration,
                turn,
                &final_attempt.command,
                process,
                include_output,
                &attempts,
            ));

            if !process.success {
                failed_iteration = (options.iterations > 1).then_some(iteration);
                failed_turn = Some(turn.index);
                if write_diagnostics {
                    write_runner_failure_diagnostic(iteration, turn.index, process, attempts.len());
                }
                break 'iterations;
            }

            if is_feedback_turn {
                if iteration < options.iterations && stdout_truncated == Some(true) {
                    return Err(AppError::InvalidRunInvocation {
                        message: "feedback from final stdout exceeded --max-captured-output; increase the limit before continuing".to_owned(),
                    });
                }
                iteration_feedback = feedback_stdout;
            }
        }

        if options.feedback_from.is_some() && iteration < options.iterations {
            previous_feedback = iteration_feedback.unwrap_or_default();
        }
    }

    let completed_turns = turns.iter().filter(|turn| turn.exit_code == 0).count();
    let success = failed_turn.is_none();
    let exit_code = if success { 0 } else { 1 };

    Ok((
        exit_code,
        RunOutput {
            sequence: sequence_summary,
            runner: RunRunnerSummary {
                mode: if runner.name.is_some() {
                    "named".to_owned()
                } else {
                    "ad-hoc".to_owned()
                },
                name: runner.name,
            },
            iterations: options.iterations,
            turn_count: turns_per_iteration * options.iterations,
            completed_turns,
            success,
            failed_iteration,
            failed_turn,
            turns,
        },
    ))
}

#[derive(Debug)]
struct RunAttemptRecord {
    attempt: usize,
    command: Vec<String>,
    process: ProcessTurnOutput,
    retryable: bool,
}

#[derive(Debug, Clone, Copy)]
struct RetryDiagnosticContext {
    enabled: bool,
    iteration: usize,
    turn: usize,
}

struct RunAttemptRequest<'a> {
    command: &'a [String],
    prompt: &'a str,
    output_mode: OutputMode,
    max_captured_output: usize,
    has_later_turn: bool,
    settings: RunSettings,
    diagnostics: RetryDiagnosticContext,
}

fn run_turn_attempts(
    runner_session: &mut RunnerHarnessSession,
    request: RunAttemptRequest<'_>,
) -> Result<Vec<RunAttemptRecord>, AppError> {
    let settings = request.settings;
    let max_attempts = settings.retries.saturating_add(1);
    let mut attempts = Vec::new();

    for attempt in 1..=max_attempts {
        let (attempt_command, process) = runner_session.run_turn(
            request.command,
            request.prompt,
            request.output_mode,
            request.max_captured_output,
            request.has_later_turn,
        )?;
        let retryable = is_retryable_runner_failure(&process);
        let success = process.success;
        attempts.push(RunAttemptRecord {
            attempt,
            command: attempt_command,
            process,
            retryable,
        });

        if success || attempt == max_attempts || !retryable {
            break;
        }

        if request.diagnostics.enabled {
            write_runner_retry_diagnostic(
                request.diagnostics.iteration,
                request.diagnostics.turn,
                attempt,
                max_attempts,
                settings.retry_delay_ms,
                &attempts.last().expect("attempt was just pushed").process,
            );
        }
        if settings.retry_delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(settings.retry_delay_ms));
        }
    }

    Ok(attempts)
}

fn is_retryable_runner_failure(process: &ProcessTurnOutput) -> bool {
    !process.success
        && matches!(
            process.termination,
            ProcessTermination::Exit | ProcessTermination::Unknown
        )
}

fn run_turn_output(
    iterations: usize,
    iteration: usize,
    turn: &render::RenderedTurn,
    command: &[String],
    process: &ProcessTurnOutput,
    include_output: bool,
    attempts: &[RunAttemptRecord],
) -> RunTurnOutput {
    RunTurnOutput {
        iteration: (iterations > 1).then_some(iteration),
        index: turn.index,
        fragment: turn.fragment.clone(),
        command: command.to_vec(),
        pid: process.pid,
        termination: process.termination.as_str().to_owned(),
        exit_code: process.exit_code,
        signal: process.signal,
        signal_name: process.signal_name.map(str::to_owned),
        core_dumped: process.core_dumped,
        attempt_count: (attempts.len() > 1).then_some(attempts.len()),
        attempts: (attempts.len() > 1).then(|| run_attempt_outputs(attempts, include_output)),
        stdout: include_output.then_some(process.stdout.clone()).flatten(),
        stderr: include_output.then_some(process.stderr.clone()).flatten(),
        stdout_bytes: include_output.then_some(process.stdout_bytes).flatten(),
        stderr_bytes: include_output.then_some(process.stderr_bytes).flatten(),
        stdout_truncated: include_output.then_some(process.stdout_truncated).flatten(),
        stderr_truncated: include_output.then_some(process.stderr_truncated).flatten(),
    }
}

fn run_attempt_outputs(
    attempts: &[RunAttemptRecord],
    include_output: bool,
) -> Vec<RunAttemptOutput> {
    attempts
        .iter()
        .map(|attempt| RunAttemptOutput {
            attempt: attempt.attempt,
            command: attempt.command.clone(),
            pid: attempt.process.pid,
            termination: attempt.process.termination.as_str().to_owned(),
            exit_code: attempt.process.exit_code,
            signal: attempt.process.signal,
            signal_name: attempt.process.signal_name.map(str::to_owned),
            core_dumped: attempt.process.core_dumped,
            retryable: attempt.retryable,
            stdout: include_output
                .then_some(attempt.process.stdout.clone())
                .flatten(),
            stderr: include_output
                .then_some(attempt.process.stderr.clone())
                .flatten(),
            stdout_bytes: include_output
                .then_some(attempt.process.stdout_bytes)
                .flatten(),
            stderr_bytes: include_output
                .then_some(attempt.process.stderr_bytes)
                .flatten(),
            stdout_truncated: include_output
                .then_some(attempt.process.stdout_truncated)
                .flatten(),
            stderr_truncated: include_output
                .then_some(attempt.process.stderr_truncated)
                .flatten(),
        })
        .collect()
}
