use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::codec;
use crate::collection;
use crate::commit::{self, CommitMode};
use crate::error::AppError;
use crate::paths;
use crate::resolve;
use crate::store;

use super::model::{CaptureEvent, CaptureEventKind, CaptureFile, CaptureRecord};
use super::types::*;
use super::{CAPTURE_VERSION, CAPTURES_DIR, ID_PREFIX};

pub(super) fn save_capture(
    store_path: &Path,
    origin: CaptureOrigin,
    texts: Vec<String>,
    commit_prefix: &str,
    commit_mode: CommitMode,
) -> Result<CaptureImportOutput, AppError> {
    let record = save_capture_uncommitted(store_path, origin, texts)?;
    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&record.store_relative_path)],
        &format!("{commit_prefix} {}", record.data.id),
    )?;

    Ok(capture_import_output(&record, git_commit))
}

pub(super) fn save_capture_uncommitted(
    store_path: &Path,
    origin: CaptureOrigin,
    texts: Vec<String>,
) -> Result<CaptureRecord, AppError> {
    let id = format!("{ID_PREFIX}{}", Uuid::new_v4().simple());
    let file_path = store_path.join(CAPTURES_DIR).join(format!("{id}.json"));
    let data = CaptureFile {
        version: CAPTURE_VERSION,
        id: id.clone(),
        created_unix_seconds: unix_seconds()?,
        origin: origin.clone(),
        events: texts
            .into_iter()
            .enumerate()
            .map(|(index, text)| CaptureEvent {
                index: index + 1,
                kind: CaptureEventKind::UserPrompt,
                text,
            })
            .collect(),
    };
    let content = codec::encode_json(&data)?;
    fs::write(&file_path, content).map_err(|source| AppError::WriteFile {
        path: file_path.clone(),
        source,
    })?;

    Ok(CaptureRecord {
        data,
        path: file_path.clone(),
        store_relative_path: paths::store_relative(store_path, &file_path),
    })
}

pub(super) fn capture_import_output(
    record: &CaptureRecord,
    git_commit: Option<String>,
) -> CaptureImportOutput {
    CaptureImportOutput {
        id: record.data.id.clone(),
        path: record.store_relative_path.clone(),
        origin: record.data.origin.clone(),
        event_count: record.data.events.len(),
        git_commit,
    }
}

pub(super) fn read_captures(store_path: &Path) -> Result<Vec<CaptureRecord>, AppError> {
    let mut captures = Vec::new();
    for path in collection::files_with_extension(store_path, CAPTURES_DIR, "json") {
        match read_capture_file(store_path, &path) {
            Ok(capture) => captures.push(capture),
            Err(_) => return Err(store::invalid_store(store_path)),
        }
    }

    captures.sort_by(|left, right| left.store_relative_path.cmp(&right.store_relative_path));
    Ok(captures)
}

pub(super) fn read_capture_file(store_path: &Path, path: &Path) -> Result<CaptureRecord, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let data: CaptureFile = serde_json::from_str(&content).map_err(|error| error.to_string())?;

    Ok(CaptureRecord {
        data,
        path: path.to_path_buf(),
        store_relative_path: paths::store_relative(store_path, path),
    })
}

pub(super) fn resolve_capture<'a>(
    captures: &'a [CaptureRecord],
    reference: &str,
) -> Result<&'a CaptureRecord, AppError> {
    let mut matches: BTreeMap<String, &'a CaptureRecord> = BTreeMap::new();

    for capture in captures {
        let id_matches = !reference.is_empty() && capture.data.id.starts_with(reference);
        if id_matches
            || collection::matches_explicit_path_reference(
                &capture.store_relative_path,
                CAPTURES_DIR,
                "json",
                reference,
            )
        {
            matches.insert(capture.store_relative_path.clone(), capture);
        }
    }

    resolve::single_match(
        matches,
        || AppError::CaptureNotFound {
            reference: reference.to_owned(),
        },
        |matches| AppError::CaptureReferenceAmbiguous {
            reference: reference.to_owned(),
            matches,
        },
    )
}

pub(super) fn capture_summary(capture: CaptureRecord) -> CaptureSummary {
    CaptureSummary {
        id: capture.data.id,
        path: capture.store_relative_path,
        origin: capture.data.origin,
        event_count: capture.data.events.len(),
    }
}

pub(super) fn capture_summary_ref(capture: &CaptureRecord) -> CaptureSummary {
    CaptureSummary {
        id: capture.data.id.clone(),
        path: capture.store_relative_path.clone(),
        origin: capture.data.origin.clone(),
        event_count: capture.data.events.len(),
    }
}

fn unix_seconds() -> Result<u64, AppError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| AppError::SystemTime { source })?
        .as_secs())
}
