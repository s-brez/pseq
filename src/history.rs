use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::git;
use crate::store;

#[derive(Debug, Serialize)]
pub struct LogOutput {
    pub entries: Vec<LogEntry>,
}

#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub commit: String,
    pub short_commit: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct DiffOutput {
    pub dirty: bool,
    pub changed_paths: usize,
    pub untracked_paths: usize,
    pub paths: Vec<DiffPath>,
    pub patch: String,
}

#[derive(Debug, Serialize)]
pub struct DiffPath {
    pub status: String,
    pub path: String,
}

pub fn log(store_path: &Path) -> Result<LogOutput, AppError> {
    store::require_valid_store(store_path)?;
    Ok(LogOutput {
        entries: git::log_entries(store_path)?
            .into_iter()
            .map(|entry| LogEntry {
                commit: entry.commit,
                short_commit: entry.short_commit,
                author_name: entry.author_name,
                author_email: entry.author_email,
                timestamp: entry.timestamp,
                summary: entry.summary,
            })
            .collect(),
    })
}

pub fn diff(store_path: &Path) -> Result<DiffOutput, AppError> {
    store::require_valid_store(store_path)?;
    let paths = git::status_entries(store_path)?
        .into_iter()
        .map(|path| DiffPath {
            status: path.status,
            path: path.path,
        })
        .collect::<Vec<_>>();
    let patch = git::diff_patch(store_path)?;
    let untracked_paths = paths.iter().filter(|path| path.status == "??").count();

    Ok(DiffOutput {
        dirty: !paths.is_empty() || !patch.is_empty(),
        changed_paths: paths.len(),
        untracked_paths,
        paths,
        patch,
    })
}
