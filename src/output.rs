use std::io::{self, Write};

use serde::Serialize;

use crate::capture::{
    CaptureImportOutput, CaptureListOutput, CaptureMoveOutput, CaptureProbeOutput,
    CapturePromoteOutput, CaptureSelectionOutput, CaptureShowOutput, CaptureSourcesOutput,
};
use crate::collection;
use crate::config::ConfigShowOutput;
use crate::error::AppError;
use crate::fragment::{
    FragmentEditOutput, FragmentListOutput, FragmentMoveOutput, FragmentNewOutput,
    FragmentRemoveOutput, FragmentRenameOutput, FragmentShowOutput,
};
use crate::history::{DiffOutput, LogOutput};
use crate::render::RenderOutput;
use crate::run::RunOutput;
use crate::runner::{
    RunnerDefaultOutput, RunnerListOutput, RunnerRemoveOutput, RunnerSetOutput, RunnerShowOutput,
    RunnerTrustOutput,
};
use crate::sequence::{
    SequenceAddOutput, SequenceEditOutput, SequenceFragmentRemoveOutput, SequenceListOutput,
    SequenceMoveOutput, SequenceNewOutput, SequencePathMoveOutput, SequenceRemoveOutput,
    SequenceRenameOutput, SequenceShowOutput,
};
use crate::store::{InitOutput, StatusOutput, ValidationReport};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Payload {
    Init(InitOutput),
    Doctor(ValidationReport),
    Status(StatusOutput),
    Log(LogOutput),
    Diff(DiffOutput),
    FragmentNew(FragmentNewOutput),
    FragmentList(FragmentListOutput),
    FragmentShow(FragmentShowOutput),
    FragmentEdit(FragmentEditOutput),
    FragmentRename(FragmentRenameOutput),
    FragmentMove(FragmentMoveOutput),
    FragmentRemove(FragmentRemoveOutput),
    SequenceNew(SequenceNewOutput),
    SequenceList(SequenceListOutput),
    SequenceShow(SequenceShowOutput),
    SequenceEdit(SequenceEditOutput),
    SequenceAdd(SequenceAddOutput),
    SequenceFragmentRemove(SequenceFragmentRemoveOutput),
    SequenceMove(SequenceMoveOutput),
    SequenceRename(SequenceRenameOutput),
    SequencePathMove(SequencePathMoveOutput),
    SequenceRemove(SequenceRemoveOutput),
    Render(RenderOutput),
    Run(RunOutput),
    RunnerSet(RunnerSetOutput),
    RunnerDefault(RunnerDefaultOutput),
    RunnerList(RunnerListOutput),
    RunnerShow(RunnerShowOutput),
    RunnerTrust(RunnerTrustOutput),
    RunnerRemove(RunnerRemoveOutput),
    CaptureSources(CaptureSourcesOutput),
    CaptureProbe(CaptureProbeOutput),
    CaptureLast(CaptureSelectionOutput),
    CaptureRange(CaptureSelectionOutput),
    CaptureImport(CaptureImportOutput),
    CaptureList(CaptureListOutput),
    CaptureShow(CaptureShowOutput),
    CaptureMove(CaptureMoveOutput),
    CapturePromote(CapturePromoteOutput),
    ConfigShow(ConfigShowOutput),
}

trait WriteOutputResult<T> {
    fn write_output(self) -> Result<T, AppError>;
}

impl<T> WriteOutputResult<T> for io::Result<T> {
    fn write_output(self) -> Result<T, AppError> {
        self.map_err(|source| AppError::WriteOutput { source })
    }
}

impl Payload {
    pub fn write_to_stdout(&self, json: bool, quiet: bool) -> Result<(), AppError> {
        if quiet && !json {
            return Ok(());
        }

        let mut stdout = io::stdout().lock();
        if json {
            serde_json::to_writer_pretty(&mut stdout, self)
                .map_err(|source| AppError::SerializeJson { source })?;
            writeln!(stdout).write_output()?;
        } else {
            match self {
                Self::Init(output) => write_init(&mut stdout, output)?,
                Self::Doctor(report) => write_doctor(&mut stdout, report)?,
                Self::Status(status) => write_status(&mut stdout, status)?,
                Self::Log(output) => write_log(&mut stdout, output)?,
                Self::Diff(output) => write_diff(&mut stdout, output)?,
                Self::FragmentNew(output) => write_fragment_new(&mut stdout, output)?,
                Self::FragmentList(output) => write_fragment_list(&mut stdout, output)?,
                Self::FragmentShow(output) => write_fragment_show(&mut stdout, output)?,
                Self::FragmentEdit(output) => write_fragment_edit(&mut stdout, output)?,
                Self::FragmentRename(output) => write_fragment_rename(&mut stdout, output)?,
                Self::FragmentMove(output) => write_fragment_move(&mut stdout, output)?,
                Self::FragmentRemove(output) => write_fragment_remove(&mut stdout, output)?,
                Self::SequenceNew(output) => write_sequence_new(&mut stdout, output)?,
                Self::SequenceList(output) => write_sequence_list(&mut stdout, output)?,
                Self::SequenceShow(output) => write_sequence_show(&mut stdout, output)?,
                Self::SequenceEdit(output) => write_sequence_edit(&mut stdout, output)?,
                Self::SequenceAdd(output) => write_sequence_add(&mut stdout, output)?,
                Self::SequenceFragmentRemove(output) => {
                    write_sequence_fragment_remove(&mut stdout, output)?
                }
                Self::SequenceMove(output) => write_sequence_move(&mut stdout, output)?,
                Self::SequenceRename(output) => write_sequence_rename(&mut stdout, output)?,
                Self::SequencePathMove(output) => write_sequence_path_move(&mut stdout, output)?,
                Self::SequenceRemove(output) => write_sequence_remove(&mut stdout, output)?,
                Self::Render(output) => write_render(&mut stdout, output)?,
                Self::Run(_output) => {}
                Self::RunnerSet(output) => write_runner_set(&mut stdout, output)?,
                Self::RunnerDefault(output) => write_runner_default(&mut stdout, output)?,
                Self::RunnerList(output) => write_runner_list(&mut stdout, output)?,
                Self::RunnerShow(output) => write_runner_show(&mut stdout, output)?,
                Self::RunnerTrust(output) => write_runner_trust(&mut stdout, output)?,
                Self::RunnerRemove(output) => write_runner_remove(&mut stdout, output)?,
                Self::CaptureSources(output) => write_capture_sources(&mut stdout, output)?,
                Self::CaptureProbe(output) => write_capture_probe(&mut stdout, output)?,
                Self::CaptureLast(output) => write_capture_selection(&mut stdout, output)?,
                Self::CaptureRange(output) => write_capture_selection(&mut stdout, output)?,
                Self::CaptureImport(output) => write_capture_import(&mut stdout, output)?,
                Self::CaptureList(output) => write_capture_list(&mut stdout, output)?,
                Self::CaptureShow(output) => write_capture_show(&mut stdout, output)?,
                Self::CaptureMove(output) => write_capture_move(&mut stdout, output)?,
                Self::CapturePromote(output) => write_capture_promote(&mut stdout, output)?,
                Self::ConfigShow(output) => write_config_show(&mut stdout, output)?,
            }
        }
        Ok(())
    }
}

fn write_init(mut stdout: impl Write, output: &InitOutput) -> Result<(), AppError> {
    if output.already_initialized {
        writeln!(stdout, "store already initialized: {}", output.store).write_output()?;
    } else {
        writeln!(stdout, "initialized store: {}", output.store).write_output()?;
    }

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_fragment_new(mut stdout: impl Write, output: &FragmentNewOutput) -> Result<(), AppError> {
    writeln!(stdout, "created fragment: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_log(mut stdout: impl Write, output: &LogOutput) -> Result<(), AppError> {
    if output.entries.is_empty() {
        writeln!(stdout, "no history").write_output()?;
        return Ok(());
    }

    for entry in &output.entries {
        writeln!(
            stdout,
            "{}  {}  {}",
            entry.short_commit, entry.timestamp, entry.summary
        )
        .write_output()?;
    }

    Ok(())
}

fn write_diff(mut stdout: impl Write, output: &DiffOutput) -> Result<(), AppError> {
    if output.patch.is_empty() {
        write_untracked_paths(&mut stdout, output)?;
        return Ok(());
    }

    stdout.write_all(output.patch.as_bytes()).write_output()?;
    if !output.patch.ends_with('\n') {
        writeln!(stdout).write_output()?;
    }
    write_untracked_paths(&mut stdout, output)
}

fn write_untracked_paths(mut stdout: impl Write, output: &DiffOutput) -> Result<(), AppError> {
    for path in output.paths.iter().filter(|path| path.status == "??") {
        writeln!(stdout, "{} {}", path.status, path.path).write_output()?;
    }
    Ok(())
}

fn write_git_commit(mut stdout: impl Write, commit: Option<&str>) -> Result<(), AppError> {
    if let Some(commit) = commit {
        writeln!(stdout, "git commit: {commit}").write_output()?;
    }
    Ok(())
}

fn write_path(mut stdout: impl Write, path: &str) -> Result<(), AppError> {
    writeln!(stdout, "path: {path}").write_output()
}

fn write_tree_line(
    mut stdout: impl Write,
    store_relative_path: &str,
    directory: &str,
    detail: &str,
) -> Result<(), AppError> {
    let relative = collection::collection_relative_path(store_relative_path, directory)
        .unwrap_or(store_relative_path);
    let depth = relative.matches('/').count();
    let name = relative.rsplit('/').next().unwrap_or(relative);
    let indent = "  ".repeat(depth);
    writeln!(stdout, "{indent}{name}  {detail}").write_output()
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn write_fragment_list(
    mut stdout: impl Write,
    output: &FragmentListOutput,
) -> Result<(), AppError> {
    if output.fragments.is_empty() {
        writeln!(stdout, "no fragments").write_output()?;
        return Ok(());
    }

    for fragment in &output.fragments {
        let short_id = short_id(&fragment.id);
        if output.tree {
            write_tree_line(
                &mut stdout,
                &fragment.path,
                "fragments",
                &format!("{short_id}  {}", fragment.name),
            )?;
        } else {
            writeln!(stdout, "{short_id}  {}  {}", fragment.name, fragment.path).write_output()?;
        }
    }

    Ok(())
}

fn write_fragment_show(
    mut stdout: impl Write,
    output: &FragmentShowOutput,
) -> Result<(), AppError> {
    stdout.write_all(output.body.as_bytes()).write_output()
}

fn write_fragment_edit(
    mut stdout: impl Write,
    output: &FragmentEditOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "edited fragment: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_fragment_rename(
    mut stdout: impl Write,
    output: &FragmentRenameOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "renamed fragment: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_fragment_move(
    mut stdout: impl Write,
    output: &FragmentMoveOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "moved fragment: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_fragment_remove(
    mut stdout: impl Write,
    output: &FragmentRemoveOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "removed fragment: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_new(mut stdout: impl Write, output: &SequenceNewOutput) -> Result<(), AppError> {
    writeln!(stdout, "created sequence: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_list(
    mut stdout: impl Write,
    output: &SequenceListOutput,
) -> Result<(), AppError> {
    if output.sequences.is_empty() {
        writeln!(stdout, "no sequences").write_output()?;
        return Ok(());
    }

    for sequence in &output.sequences {
        let short_id = short_id(&sequence.id);
        let detail = format!("{}  fragments={}", sequence.name, sequence.fragment_count);
        if output.tree {
            write_tree_line(
                &mut stdout,
                &sequence.path,
                "sequences",
                &format!("{short_id}  {detail}"),
            )?;
        } else {
            writeln!(
                stdout,
                "{short_id}  {}  fragments={}  {}",
                sequence.name, sequence.fragment_count, sequence.path
            )
            .write_output()?;
        }
    }

    Ok(())
}

fn write_sequence_show(
    mut stdout: impl Write,
    output: &SequenceShowOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "{} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    if output.fragments.is_empty() {
        writeln!(stdout, "fragments: none").write_output()?;
        return Ok(());
    }

    writeln!(stdout, "fragments:").write_output()?;
    for (index, fragment) in output.fragments.iter().enumerate() {
        let position = index + 1;
        let short_id = short_id(&fragment.id);
        writeln!(
            stdout,
            "{position}. {short_id}  {}  {}",
            fragment.name, fragment.path
        )
        .write_output()?;
    }

    Ok(())
}

fn write_sequence_edit(
    mut stdout: impl Write,
    output: &SequenceEditOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "edited sequence: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;
    writeln!(stdout, "fragment count: {}", output.fragment_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_add(mut stdout: impl Write, output: &SequenceAddOutput) -> Result<(), AppError> {
    writeln!(
        stdout,
        "added fragment: {} ({})",
        output.fragment.name, output.fragment.id
    )
    .write_output()?;
    writeln!(stdout, "sequence: {} ({})", output.name, output.id).write_output()?;
    writeln!(stdout, "position: {}", output.index).write_output()?;
    writeln!(stdout, "fragment count: {}", output.fragment_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_fragment_remove(
    mut stdout: impl Write,
    output: &SequenceFragmentRemoveOutput,
) -> Result<(), AppError> {
    writeln!(
        stdout,
        "removed fragment: {} ({})",
        output.removed_fragment.name, output.removed_fragment.id
    )
    .write_output()?;
    writeln!(stdout, "sequence: {} ({})", output.name, output.id).write_output()?;
    writeln!(stdout, "fragment count: {}", output.fragment_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_move(
    mut stdout: impl Write,
    output: &SequenceMoveOutput,
) -> Result<(), AppError> {
    writeln!(
        stdout,
        "moved fragment in sequence: {} ({})",
        output.name, output.id
    )
    .write_output()?;
    writeln!(stdout, "from: {}", output.from_index).write_output()?;
    writeln!(stdout, "to: {}", output.to_index).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_rename(
    mut stdout: impl Write,
    output: &SequenceRenameOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "renamed sequence: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_path_move(
    mut stdout: impl Write,
    output: &SequencePathMoveOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "moved sequence: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;
    writeln!(stdout, "fragment count: {}", output.fragment_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_sequence_remove(
    mut stdout: impl Write,
    output: &SequenceRemoveOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "removed sequence: {} ({})", output.name, output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_render(mut stdout: impl Write, output: &RenderOutput) -> Result<(), AppError> {
    if output.out_path.is_some() {
        return Ok(());
    }

    stdout.write_all(output.text.as_bytes()).write_output()
}

fn write_runner_set(mut stdout: impl Write, output: &RunnerSetOutput) -> Result<(), AppError> {
    writeln!(
        stdout,
        "configured runner {} {} command",
        output.name, output.slot
    )
    .write_output()?;
    writeln!(stdout, "command: {}", output.command.join(" ")).write_output()?;
    write_git_commit(&mut stdout, output.git_commit.as_deref())
}

fn write_runner_default(
    mut stdout: impl Write,
    output: &RunnerDefaultOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "default runner: {}", output.name).write_output()?;
    write_git_commit(&mut stdout, output.git_commit.as_deref())
}

fn write_runner_list(mut stdout: impl Write, output: &RunnerListOutput) -> Result<(), AppError> {
    if output.runners.is_empty() {
        writeln!(stdout, "no runners").write_output()?;
        return Ok(());
    }

    for runner in &output.runners {
        let default = if runner.is_default { " default" } else { "" };
        let next = if runner.has_next { " next" } else { "" };
        writeln!(stdout, "{}{default}{next}", runner.name).write_output()?;
    }
    Ok(())
}

fn write_runner_show(mut stdout: impl Write, output: &RunnerShowOutput) -> Result<(), AppError> {
    writeln!(
        stdout,
        "{}{}",
        output.name,
        if output.is_default { " (default)" } else { "" }
    )
    .write_output()?;
    writeln!(stdout, "first: {}", output.first.join(" ")).write_output()?;
    if let Some(next) = &output.next {
        writeln!(stdout, "next: {}", next.join(" ")).write_output()?;
    }
    Ok(())
}

fn write_runner_trust(mut stdout: impl Write, output: &RunnerTrustOutput) -> Result<(), AppError> {
    writeln!(stdout, "trusted runner: {}", output.name).write_output()
}

fn write_runner_remove(
    mut stdout: impl Write,
    output: &RunnerRemoveOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "removed runner: {}", output.name).write_output()?;
    if output.was_default {
        writeln!(stdout, "default runner cleared").write_output()?;
    }
    write_git_commit(&mut stdout, output.git_commit.as_deref())
}

fn write_capture_sources(
    mut stdout: impl Write,
    output: &CaptureSourcesOutput,
) -> Result<(), AppError> {
    for source in &output.sources {
        let state = if source.available {
            "available"
        } else {
            "unavailable"
        };
        writeln!(stdout, "{}  {state}  {}", source.name, source.description).write_output()?;
    }
    Ok(())
}

fn write_capture_probe(
    mut stdout: impl Write,
    output: &CaptureProbeOutput,
) -> Result<(), AppError> {
    let state = if output.available {
        "available"
    } else {
        "unavailable"
    };
    writeln!(stdout, "{}: {state}", output.source).write_output()?;
    writeln!(stdout, "{}", output.message).write_output()?;
    for session in &output.sessions {
        let id = session.id.as_deref().unwrap_or("-");
        writeln!(
            stdout,
            "session: {}  id={}  prompts={}",
            session.path, id, session.prompt_count
        )
        .write_output()?;
    }
    Ok(())
}

fn write_capture_import(
    mut stdout: impl Write,
    output: &CaptureImportOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "created capture: {}", output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;
    writeln!(stdout, "events: {}", output.event_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_capture_selection(
    mut stdout: impl Write,
    output: &CaptureSelectionOutput,
) -> Result<(), AppError> {
    match output {
        CaptureSelectionOutput::Capture(output) => write_capture_import(&mut stdout, output),
        CaptureSelectionOutput::Promoted(output) => write_capture_promote(&mut stdout, output),
    }
}

fn write_capture_list(mut stdout: impl Write, output: &CaptureListOutput) -> Result<(), AppError> {
    if output.captures.is_empty() {
        writeln!(stdout, "no captures").write_output()?;
        return Ok(());
    }

    for capture in &output.captures {
        let short_id = short_id(&capture.id);
        if output.tree {
            write_tree_line(
                &mut stdout,
                &capture.path,
                "captures",
                &format!("{short_id}  events={}", capture.event_count),
            )?;
        } else {
            writeln!(
                stdout,
                "{short_id}  events={}  {}",
                capture.event_count, capture.path
            )
            .write_output()?;
        }
    }

    Ok(())
}

fn write_capture_show(mut stdout: impl Write, output: &CaptureShowOutput) -> Result<(), AppError> {
    writeln!(stdout, "capture: {}", output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;

    for event in &output.events {
        writeln!(stdout, "{}. {}", event.index, event.kind).write_output()?;
        stdout.write_all(event.text.as_bytes()).write_output()?;
        if !event.text.ends_with('\n') {
            writeln!(stdout).write_output()?;
        }
    }

    Ok(())
}

fn write_capture_move(mut stdout: impl Write, output: &CaptureMoveOutput) -> Result<(), AppError> {
    writeln!(stdout, "moved capture: {}", output.id).write_output()?;
    write_path(&mut stdout, &output.path)?;
    writeln!(stdout, "events: {}", output.event_count).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_capture_promote(
    mut stdout: impl Write,
    output: &CapturePromoteOutput,
) -> Result<(), AppError> {
    writeln!(stdout, "promoted capture: {}", output.capture.id).write_output()?;
    writeln!(
        stdout,
        "created sequence: {} ({})",
        output.sequence.name, output.sequence.id
    )
    .write_output()?;
    write_path(&mut stdout, &output.sequence.path)?;
    writeln!(stdout, "fragments: {}", output.fragments.len()).write_output()?;

    write_git_commit(&mut stdout, output.git_commit.as_deref())?;

    Ok(())
}

fn write_doctor(mut stdout: impl Write, report: &ValidationReport) -> Result<(), AppError> {
    if report.valid {
        writeln!(stdout, "ok: {}", report.store).write_output()?;
    } else {
        writeln!(stdout, "invalid: {}", report.store).write_output()?;
        for issue in &report.issues {
            writeln!(stdout, "- {}: {}", issue.code, issue.message).write_output()?;
        }
    }
    Ok(())
}

fn write_status(mut stdout: impl Write, status: &StatusOutput) -> Result<(), AppError> {
    writeln!(stdout, "store: {}", status.store).write_output()?;
    writeln!(stdout, "valid: {}", if status.valid { "yes" } else { "no" }).write_output()?;

    if !status.issues.is_empty() {
        writeln!(stdout, "issues:").write_output()?;
        for issue in &status.issues {
            writeln!(stdout, "- {}: {}", issue.code, issue.message).write_output()?;
        }
    }

    writeln!(
        stdout,
        "counts: fragments={}, sequences={}, captures={}, renders={}",
        status.counts.fragments,
        status.counts.sequences,
        status.counts.captures,
        status.counts.renders
    )
    .write_output()?;

    if status.git.repository {
        let branch = status.git.branch.as_deref().unwrap_or("detached");
        let head = status.git.head.as_deref().unwrap_or("no commits");
        let state = if status.git.dirty { "dirty" } else { "clean" };
        writeln!(stdout, "git: {state} on {branch} @ {head}").write_output()?;
    } else if let Some(error) = &status.git.error {
        writeln!(stdout, "git: unavailable ({error})").write_output()?;
    } else {
        writeln!(stdout, "git: unavailable").write_output()?;
    }

    Ok(())
}

fn write_config_show(mut stdout: impl Write, output: &ConfigShowOutput) -> Result<(), AppError> {
    write_path(&mut stdout, &output.path)?;
    writeln!(stdout, "version: {}", output.version).write_output()?;
    if let Some(default_runner) = &output.default_runner {
        writeln!(stdout, "default runner: {default_runner}").write_output()?;
    }
    writeln!(stdout, "runners: {}", output.runner_count).write_output()
}
