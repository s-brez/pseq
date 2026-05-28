use std::path::{Path, PathBuf};

use crate::commit::{self, CommitMode};
use crate::error::AppError;
use crate::paths;
use crate::sequence;

use super::model::SourceSession;
use super::promote::promote_capture_record_uncommitted;
use super::records::{capture_import_output, save_capture_uncommitted};
use super::types::*;

pub(super) fn resolve_range(selector: &str, available: usize) -> Result<(usize, usize), AppError> {
    let Some((start, end)) = selector.split_once("..") else {
        return Err(AppError::InvalidCaptureRange {
            selector: selector.to_owned(),
        });
    };
    let start = parse_range_endpoint(selector, start)?;
    let end = parse_range_endpoint(selector, end)?;
    if start == 0 || end == 0 {
        return Err(AppError::InvalidCaptureRange {
            selector: selector.to_owned(),
        });
    }

    let start = resolve_range_endpoint(selector, start, available)?;
    let end = resolve_range_endpoint(selector, end, available)?;
    if start > end {
        return Err(AppError::InvalidCaptureRange {
            selector: selector.to_owned(),
        });
    }

    Ok((start - 1, end))
}

fn parse_range_endpoint(selector: &str, endpoint: &str) -> Result<isize, AppError> {
    endpoint
        .parse::<isize>()
        .map_err(|_| AppError::InvalidCaptureRange {
            selector: selector.to_owned(),
        })
}

fn resolve_range_endpoint(
    selector: &str,
    endpoint: isize,
    available: usize,
) -> Result<usize, AppError> {
    let resolved = if endpoint > 0 {
        endpoint
    } else {
        available as isize + endpoint + 1
    };

    if resolved < 1 || resolved as usize > available {
        return Err(AppError::CaptureSelectionOutOfRange {
            selector: selector.to_owned(),
            available,
        });
    }

    Ok(resolved as usize)
}

pub(super) fn save_source_selection(
    store_path: &Path,
    session: &SourceSession,
    texts: Vec<String>,
    as_sequence: Option<String>,
    commit_mode: CommitMode,
) -> Result<CaptureSelectionOutput, AppError> {
    if let Some(sequence_name) = as_sequence.as_deref() {
        sequence::validate_name(sequence_name)?;
    }

    let origin = CaptureOrigin::Source {
        source: session.source.to_owned(),
        session: CaptureSourceSession {
            path: paths::display(&session.path),
            id: session.id.clone(),
            timestamp: session.timestamp.clone(),
        },
    };
    let capture = save_capture_uncommitted(store_path, origin, texts)?;

    if let Some(sequence_name) = as_sequence {
        let mut output =
            promote_capture_record_uncommitted(store_path, &capture, sequence_name.clone())?;
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
                "Capture {} prompts as sequence {sequence_name}",
                session.source
            ),
        )?;
        output.git_commit = git_commit;
        Ok(CaptureSelectionOutput::Promoted(output))
    } else {
        let git_commit = commit::maybe_commit_paths(
            commit_mode,
            store_path,
            &[PathBuf::from(&capture.store_relative_path)],
            &format!("Capture {} prompts {}", session.source, capture.data.id),
        )?;
        Ok(CaptureSelectionOutput::Capture(capture_import_output(
            &capture, git_commit,
        )))
    }
}
