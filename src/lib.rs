pub mod capture;
pub mod cli;
mod codec;
mod collection;
pub mod commit;
pub mod config;
mod editor;
pub mod error;
pub mod fragment;
mod fs_walk;
mod git;
pub mod history;
pub mod output;
mod paths;
pub mod render;
mod resolve;
pub mod run;
pub mod runner;
pub mod sequence;
pub mod store;
mod trust;
mod turn_settings;
mod user_config;
pub mod yaml;

use cli::{
    CaptureCommand, Cli, Command, ConfigCommand, FeedbackFromArg, FragmentCommand, RunnerCommand,
    RunnerSlotArg, SequenceCommand, SessionScopeArg,
};
use error::AppError;
use output::Payload;

pub struct CommandResult {
    pub exit_code: i32,
    pub payload: Payload,
}

fn command_result(exit_code: i32, payload: Payload) -> Result<CommandResult, AppError> {
    Ok(CommandResult { exit_code, payload })
}

fn success(payload: Payload) -> Result<CommandResult, AppError> {
    command_result(0, payload)
}

pub fn run(cli: Cli) -> Result<CommandResult, AppError> {
    let Cli {
        store,
        json,
        quiet,
        no_pager: _,
        command,
    } = cli;

    let command = match command {
        Command::Capture {
            command: CaptureCommand::Sources,
        } => return success(Payload::CaptureSources(capture::sources())),
        Command::Capture {
            command: CaptureCommand::Probe { source },
        } => {
            let output = capture::probe(&source)?;
            let exit_code = if output.available { 0 } else { 1 };
            return command_result(exit_code, Payload::CaptureProbe(output));
        }
        command => command,
    };

    let store_path = store::resolve_store_path(store.as_deref(), None)?;

    match command {
        Command::Init => {
            let output = store::init_store(&store_path)?;
            success(Payload::Init(output))
        }
        Command::Doctor => {
            let report = store::validate_store(&store_path);
            let exit_code = if report.valid { 0 } else { 1 };
            command_result(exit_code, Payload::Doctor(report))
        }
        Command::Status => {
            let status = store::status(&store_path);
            let exit_code = if status.valid { 0 } else { 1 };
            command_result(exit_code, Payload::Status(status))
        }
        Command::Log => {
            let output = history::log(&store_path)?;
            success(Payload::Log(output))
        }
        Command::Diff => {
            let output = history::diff(&store_path)?;
            success(Payload::Diff(output))
        }
        Command::Render {
            sequence_reference,
            variables,
            variables_file,
            save,
            dir,
            save_path,
            out,
            annotate,
            at,
            no_commit,
        } => {
            let options = render::RenderOptions {
                variables_file: variables_file.as_deref(),
                variable_assignments: &variables,
                save,
                save_dir: dir.as_deref(),
                save_path: save_path.as_deref(),
                out_path: out.as_deref(),
                annotate,
                history_ref: at.as_deref(),
                commit_mode: commit::CommitMode::from_no_commit(no_commit),
            };
            let output = render::render(&store_path, &sequence_reference, options)?;
            success(Payload::Render(output))
        }
        Command::Run {
            sequence_reference,
            runner_name,
            variables,
            variables_file,
            max_captured_output,
            iterations,
            retries,
            no_retry,
            retry_delay_ms,
            no_preserve_output,
            session_scope,
            feedback_from,
            feedback_var,
            feedback_seed,
            command,
        } => {
            let (exit_code, output) = run::run_sequence(
                &store_path,
                &sequence_reference,
                run::RunOptions {
                    runner_name: runner_name.as_deref(),
                    ad_hoc_command: &command,
                    variables_file: variables_file.as_deref(),
                    variable_assignments: &variables,
                    capture_output: json,
                    max_captured_output: max_captured_output
                        .unwrap_or(run::DEFAULT_MAX_CAPTURED_OUTPUT),
                    iterations: iterations.unwrap_or(1),
                    retries,
                    no_retry,
                    retry_delay_ms,
                    preserve_output: if no_preserve_output {
                        Some(false)
                    } else {
                        None
                    },
                    session_scope: session_scope
                        .map(session_scope_arg)
                        .unwrap_or(run::SessionScope::Run),
                    feedback_from: feedback_from.map(feedback_from_source),
                    feedback_var: feedback_var.as_deref(),
                    feedback_seed: feedback_seed.as_deref(),
                    quiet,
                },
            )?;
            command_result(exit_code, Payload::Run(output))
        }
        Command::Runner { command } => match command {
            RunnerCommand::Set {
                name,
                slot,
                command,
            } => {
                let output = runner::set(&store_path, name, runner_slot(slot), command)?;
                success(Payload::RunnerSet(output))
            }
            RunnerCommand::Default { name } => {
                let output = runner::set_default(&store_path, name)?;
                success(Payload::RunnerDefault(output))
            }
            RunnerCommand::List => {
                let output = runner::list(&store_path)?;
                success(Payload::RunnerList(output))
            }
            RunnerCommand::Show { name } => {
                let output = runner::show(&store_path, &name)?;
                success(Payload::RunnerShow(output))
            }
            RunnerCommand::Trust { name } => {
                let output = runner::trust(&store_path, &name)?;
                success(Payload::RunnerTrust(output))
            }
            RunnerCommand::Rm { name } => {
                let output = runner::remove(&store_path, &name)?;
                success(Payload::RunnerRemove(output))
            }
        },
        Command::Fragment { command } => match command {
            FragmentCommand::New {
                name,
                from_file,
                stdin,
                dir,
                path,
                no_commit,
            } => {
                let output = fragment::create(
                    &store_path,
                    name,
                    from_file.as_deref(),
                    stdin,
                    dir.as_deref(),
                    path.as_deref(),
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::FragmentNew(output))
            }
            FragmentCommand::List { prefix, tree } => {
                let output = fragment::list(&store_path, prefix.as_deref(), tree)?;
                success(Payload::FragmentList(output))
            }
            FragmentCommand::Show { reference } => {
                let output = fragment::show(&store_path, &reference)?;
                success(Payload::FragmentShow(output))
            }
            FragmentCommand::Edit {
                reference,
                no_commit,
            } => {
                let output = fragment::edit(
                    &store_path,
                    &reference,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::FragmentEdit(output))
            }
            FragmentCommand::Rename {
                reference,
                name,
                no_commit,
            } => {
                let output = fragment::rename(
                    &store_path,
                    &reference,
                    name,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::FragmentRename(output))
            }
            FragmentCommand::Mv {
                reference,
                path,
                no_commit,
            } => {
                let output = fragment::move_file(
                    &store_path,
                    &reference,
                    &path,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::FragmentMove(output))
            }
            FragmentCommand::Rm {
                reference,
                no_commit,
            } => {
                let output = fragment::remove(
                    &store_path,
                    &reference,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::FragmentRemove(output))
            }
        },
        Command::Sequence { command } => match command {
            SequenceCommand::New {
                name,
                dir,
                path,
                no_commit,
            } => {
                let output = sequence::create(
                    &store_path,
                    name,
                    dir.as_deref(),
                    path.as_deref(),
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceNew(output))
            }
            SequenceCommand::List { prefix, tree } => {
                let output = sequence::list(&store_path, prefix.as_deref(), tree)?;
                success(Payload::SequenceList(output))
            }
            SequenceCommand::Show { reference } => {
                let output = sequence::show(&store_path, &reference)?;
                success(Payload::SequenceShow(output))
            }
            SequenceCommand::Edit {
                reference,
                no_commit,
            } => {
                let output = sequence::edit(
                    &store_path,
                    &reference,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceEdit(output))
            }
            SequenceCommand::Add {
                sequence_reference,
                fragment_reference,
                at,
                no_commit,
            } => {
                let output = sequence::add(
                    &store_path,
                    &sequence_reference,
                    &fragment_reference,
                    at,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceAdd(output))
            }
            SequenceCommand::Remove {
                sequence_reference,
                fragment_reference_or_index,
                no_commit,
            } => {
                let output = sequence::remove_fragment(
                    &store_path,
                    &sequence_reference,
                    &fragment_reference_or_index,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceFragmentRemove(output))
            }
            SequenceCommand::Move {
                sequence_reference,
                from_index,
                to_index,
                no_commit,
            } => {
                let output = sequence::move_fragment(
                    &store_path,
                    &sequence_reference,
                    from_index,
                    to_index,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceMove(output))
            }
            SequenceCommand::Rename {
                reference,
                name,
                no_commit,
            } => {
                let output = sequence::rename(
                    &store_path,
                    &reference,
                    name,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceRename(output))
            }
            SequenceCommand::Mv {
                reference,
                path,
                no_commit,
            } => {
                let output = sequence::move_file(
                    &store_path,
                    &reference,
                    &path,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequencePathMove(output))
            }
            SequenceCommand::Rm {
                reference,
                no_commit,
            } => {
                let output = sequence::remove(
                    &store_path,
                    &reference,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::SequenceRemove(output))
            }
        },
        Command::Capture { command } => match command {
            CaptureCommand::Sources | CaptureCommand::Probe { .. } => {
                unreachable!("capture discovery commands returned before store resolution")
            }
            CaptureCommand::Last {
                count,
                source,
                session,
                as_sequence,
                no_commit,
            } => {
                let source = source.as_deref().unwrap_or("stdin");
                let output = capture::last(
                    &store_path,
                    source,
                    count,
                    session.as_deref(),
                    as_sequence,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::CaptureLast(output))
            }
            CaptureCommand::Range {
                selector,
                source,
                session,
                as_sequence,
                no_commit,
            } => {
                let source = source.as_deref().unwrap_or("stdin");
                let output = capture::range(
                    &store_path,
                    source,
                    &selector,
                    session.as_deref(),
                    as_sequence,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::CaptureRange(output))
            }
            CaptureCommand::Import {
                stdin,
                file,
                no_commit,
            } => {
                let output = if stdin {
                    capture::import_stdin(
                        &store_path,
                        commit::CommitMode::from_no_commit(no_commit),
                    )?
                } else if let Some(file) = file {
                    capture::import_file(
                        &store_path,
                        &file,
                        commit::CommitMode::from_no_commit(no_commit),
                    )?
                } else {
                    unreachable!("clap requires either --stdin or --file")
                };
                success(Payload::CaptureImport(output))
            }
            CaptureCommand::List { prefix } => {
                let output = capture::list(&store_path, prefix.as_deref())?;
                success(Payload::CaptureList(output))
            }
            CaptureCommand::Show { reference } => {
                let output = capture::show(&store_path, &reference)?;
                success(Payload::CaptureShow(output))
            }
            CaptureCommand::Mv {
                reference,
                path,
                no_commit,
            } => {
                let output = capture::move_file(
                    &store_path,
                    &reference,
                    &path,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::CaptureMove(output))
            }
            CaptureCommand::Promote {
                reference,
                as_sequence,
                no_commit,
            } => {
                let output = capture::promote(
                    &store_path,
                    &reference,
                    as_sequence,
                    commit::CommitMode::from_no_commit(no_commit),
                )?;
                success(Payload::CapturePromote(output))
            }
        },
        Command::Config { command } => match command {
            ConfigCommand::Show => {
                let output = config::show(&store_path)?;
                success(Payload::ConfigShow(output))
            }
        },
    }
}

fn runner_slot(slot: RunnerSlotArg) -> runner::RunnerSlot {
    match slot {
        RunnerSlotArg::First => runner::RunnerSlot::First,
        RunnerSlotArg::Next => runner::RunnerSlot::Next,
    }
}

fn feedback_from_source(source: FeedbackFromArg) -> run::FeedbackFrom {
    match source {
        FeedbackFromArg::FinalStdout => run::FeedbackFrom::FinalStdout,
    }
}

fn session_scope_arg(scope: SessionScopeArg) -> run::SessionScope {
    match scope {
        SessionScopeArg::Run => run::SessionScope::Run,
        SessionScopeArg::Iteration => run::SessionScope::Iteration,
    }
}
