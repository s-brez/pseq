use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::git;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitMode {
    Commit,
    NoCommit,
}

impl CommitMode {
    pub fn from_no_commit(no_commit: bool) -> Self {
        if no_commit {
            Self::NoCommit
        } else {
            Self::Commit
        }
    }
}

pub(crate) fn maybe_commit_paths(
    mode: CommitMode,
    store_path: &Path,
    paths: &[PathBuf],
    message: &str,
) -> Result<Option<String>, AppError> {
    match mode {
        CommitMode::Commit => git::commit_paths_if_changed(store_path, paths, message),
        CommitMode::NoCommit => Ok(None),
    }
}
