use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::collection;
use crate::commit::{self, CommitMode};
use crate::error::AppError;
use crate::paths;
use crate::store;

use super::events::selectable_prompt_texts;
use super::promote::promote_capture_record_uncommitted;
use super::records::{capture_summary, read_captures, resolve_capture, save_capture};
use super::selection::{resolve_range, save_source_selection};
use super::sources::{SOURCES, resolve_source, resolve_source_session, source_availability};
use super::types::*;

pub fn sources() -> CaptureSourcesOutput {
    CaptureSourcesOutput {
        sources: SOURCES
            .iter()
            .map(|source| {
                let availability = source_availability(source);
                CaptureSourceSummary {
                    name: source.name.to_owned(),
                    available: availability.available,
                    description: source.description.to_owned(),
                    session_count: availability.sessions.len(),
                    sessions: availability.sessions,
                    unavailable_reason: availability.unavailable_reason,
                }
            })
            .collect(),
    }
}

pub fn probe(source: &str) -> Result<CaptureProbeOutput, AppError> {
    let source = resolve_source(source)?;
    let availability = source_availability(source);
    let message = if availability.available {
        "source is available".to_owned()
    } else {
        availability
            .unavailable_reason
            .unwrap_or_else(|| "capture source is unavailable".to_owned())
    };

    Ok(CaptureProbeOutput {
        source: source.name.to_owned(),
        available: availability.available,
        message,
        session_count: availability.sessions.len(),
        sessions: availability.sessions,
    })
}

pub fn last(
    store_path: &Path,
    source: &str,
    count: Option<usize>,
    session_reference: Option<&str>,
    as_sequence: Option<String>,
    commit_mode: CommitMode,
) -> Result<CaptureSelectionOutput, AppError> {
    store::require_valid_store(store_path)?;
    let count = count.unwrap_or(1);
    if count == 0 {
        return Err(AppError::InvalidCaptureCount { count });
    }

    let session = resolve_source_session(source, session_reference)?;
    let prompt_texts = selectable_prompt_texts(&session);
    let available = prompt_texts.len();
    if count > available {
        return Err(AppError::CaptureSelectionOutOfRange {
            selector: format!("last {count}"),
            available,
        });
    }

    let start = available - count;
    save_source_selection(
        store_path,
        &session,
        prompt_texts[start..available].to_vec(),
        as_sequence,
        commit_mode,
    )
}

pub fn range(
    store_path: &Path,
    source: &str,
    selector: &str,
    session_reference: Option<&str>,
    as_sequence: Option<String>,
    commit_mode: CommitMode,
) -> Result<CaptureSelectionOutput, AppError> {
    store::require_valid_store(store_path)?;
    let session = resolve_source_session(source, session_reference)?;
    let prompt_texts = selectable_prompt_texts(&session);
    let available = prompt_texts.len();
    let (start, end_exclusive) = resolve_range(selector, available)?;

    save_source_selection(
        store_path,
        &session,
        prompt_texts[start..end_exclusive].to_vec(),
        as_sequence,
        commit_mode,
    )
}

pub fn import_stdin(
    store_path: &Path,
    commit_mode: CommitMode,
) -> Result<CaptureImportOutput, AppError> {
    store::require_valid_store(store_path)?;

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|source| AppError::ReadStdin { source })?;

    save_capture(
        store_path,
        CaptureOrigin::Stdin,
        vec![input],
        "Import capture",
        commit_mode,
    )
}

pub fn import_file(
    store_path: &Path,
    path: &Path,
    commit_mode: CommitMode,
) -> Result<CaptureImportOutput, AppError> {
    store::require_valid_store(store_path)?;
    let text = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    save_capture(
        store_path,
        CaptureOrigin::File {
            path: paths::display(path),
        },
        vec![text],
        "Import capture",
        commit_mode,
    )
}

pub fn list(store_path: &Path, prefix: Option<&Path>) -> Result<CaptureListOutput, AppError> {
    store::require_valid_store(store_path)?;
    collection::validate_prefix(super::CAPTURES_DIR, prefix)?;
    let mut captures = Vec::new();
    for capture in read_captures(store_path)? {
        if collection::prefix_matches(
            &capture.store_relative_path,
            super::CAPTURES_DIR,
            "json",
            prefix,
        )? {
            captures.push(capture_summary(capture));
        }
    }

    Ok(CaptureListOutput {
        captures,
        tree: false,
    })
}

pub fn show(store_path: &Path, reference: &str) -> Result<CaptureShowOutput, AppError> {
    store::require_valid_store(store_path)?;
    let captures = read_captures(store_path)?;
    let capture = resolve_capture(&captures, reference)?;

    Ok(CaptureShowOutput {
        id: capture.data.id.clone(),
        path: capture.store_relative_path.clone(),
        origin: capture.data.origin.clone(),
        events: capture
            .data
            .events
            .iter()
            .map(|event| CaptureEventOutput {
                index: event.index,
                kind: "user_prompt".to_owned(),
                text: event.text.clone(),
            })
            .collect(),
    })
}

pub fn move_file(
    store_path: &Path,
    reference: &str,
    destination: &Path,
    commit_mode: CommitMode,
) -> Result<CaptureMoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let captures = read_captures(store_path)?;
    let capture = resolve_capture(&captures, reference)?;
    let destination =
        collection::destination_file(store_path, super::CAPTURES_DIR, "json", destination)?;

    collection::create_parent_dir(&destination)?;
    fs::rename(&capture.path, &destination.path).map_err(|source| AppError::MoveFile {
        from: capture.path.clone(),
        to: destination.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[
            PathBuf::from(&capture.store_relative_path),
            PathBuf::from(&destination.store_relative_path),
        ],
        &format!("Move capture {}", capture.data.id),
    )?;

    Ok(CaptureMoveOutput {
        id: capture.data.id.clone(),
        path: destination.store_relative_path,
        origin: capture.data.origin.clone(),
        event_count: capture.data.events.len(),
        git_commit,
    })
}

pub fn promote(
    store_path: &Path,
    reference: &str,
    sequence_name: String,
    commit_mode: CommitMode,
) -> Result<CapturePromoteOutput, AppError> {
    store::require_valid_store(store_path)?;
    let captures = read_captures(store_path)?;
    let capture = resolve_capture(&captures, reference)?;
    let mut output =
        promote_capture_record_uncommitted(store_path, capture, sequence_name.clone())?;
    let paths = output
        .fragments
        .iter()
        .map(|fragment| PathBuf::from(&fragment.path))
        .chain([
            PathBuf::from(&output.capture.path),
            PathBuf::from(&output.sequence.path),
        ])
        .collect::<Vec<_>>();
    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &paths,
        &format!(
            "Promote capture {} to sequence {sequence_name}",
            capture.data.id
        ),
    )?;
    output.git_commit = git_commit;

    Ok(output)
}
