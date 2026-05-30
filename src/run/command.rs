use std::path::Path;

use crate::error::AppError;
use crate::render;
use crate::store;

use super::diagnostics::{
    should_write_diagnostics, write_runner_failure_diagnostic, write_turn_diagnostic,
};
use super::harnesses::{RunnerHarnessSession, prepare_runner_command};
use super::model::ProcessTurnOutput;
use super::options::{
    feedback_variable, load_base_variables, load_feedback_seed, output_mode, resolve_runner,
    validate_options,
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
    let output_mode = output_mode(&options);
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
            if should_write_diagnostics(options.capture_output, options.quiet) {
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
            let (command, process) = runner_session.run_turn(
                &command,
                &turn.text,
                output_mode,
                options.max_captured_output,
                has_later_turn,
            )?;
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
                command,
                &process,
                options.capture_output,
            ));

            if !process.success {
                failed_iteration = (options.iterations > 1).then_some(iteration);
                failed_turn = Some(turn.index);
                if should_write_diagnostics(options.capture_output, options.quiet) {
                    write_runner_failure_diagnostic(iteration, turn.index, &process);
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

fn run_turn_output(
    iterations: usize,
    iteration: usize,
    turn: &render::RenderedTurn,
    command: Vec<String>,
    process: &ProcessTurnOutput,
    capture_output: bool,
) -> RunTurnOutput {
    RunTurnOutput {
        iteration: (iterations > 1).then_some(iteration),
        index: turn.index,
        fragment: turn.fragment.clone(),
        command,
        pid: process.pid,
        termination: process.termination.as_str().to_owned(),
        exit_code: process.exit_code,
        signal: process.signal,
        signal_name: process.signal_name.map(str::to_owned),
        core_dumped: process.core_dumped,
        stdout: capture_output.then_some(process.stdout.clone()).flatten(),
        stderr: capture_output.then_some(process.stderr.clone()).flatten(),
        stdout_bytes: capture_output.then_some(process.stdout_bytes).flatten(),
        stderr_bytes: capture_output.then_some(process.stderr_bytes).flatten(),
        stdout_truncated: capture_output.then_some(process.stdout_truncated).flatten(),
        stderr_truncated: capture_output.then_some(process.stderr_truncated).flatten(),
    }
}
