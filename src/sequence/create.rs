use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::codec;
use crate::collection;
use crate::commit::{self, CommitMode};
use crate::error::AppError;
use crate::store;

use super::model::SequenceFile;
use super::types::*;
use super::validation::validate_name;
use super::{ID_PREFIX, SEQUENCES_DIR};

pub fn create(
    store_path: &Path,
    name: String,
    dir: Option<&Path>,
    path: Option<&Path>,
    commit_mode: CommitMode,
) -> Result<SequenceNewOutput, AppError> {
    store::require_valid_store(store_path)?;

    let sequence = create_uncommitted_placed(store_path, name, Vec::new(), dir, path)?;
    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.path)],
        &format!("Add sequence {}", sequence.name),
    )?;

    Ok(SequenceNewOutput {
        id: sequence.id,
        name: sequence.name,
        path: sequence.path,
        git_commit,
    })
}

pub(crate) fn create_uncommitted(
    store_path: &Path,
    name: String,
    fragments: Vec<String>,
) -> Result<SequenceSummary, AppError> {
    create_uncommitted_placed(store_path, name, fragments, None, None)
}

fn create_uncommitted_placed(
    store_path: &Path,
    name: String,
    fragments: Vec<String>,
    dir: Option<&Path>,
    path: Option<&Path>,
) -> Result<SequenceSummary, AppError> {
    validate_name(&name)?;

    let id = format!("{ID_PREFIX}{}", Uuid::new_v4().simple());
    let destination = match (dir, path) {
        (Some(_), Some(_)) => {
            return Err(AppError::InvalidCollectionPath {
                kind: "sequence placement".to_owned(),
                path: "--dir/--path".to_owned(),
                message: "--dir and --path are mutually exclusive".to_owned(),
            });
        }
        (Some(dir), None) => collection::destination_file_in_directory(
            store_path,
            SEQUENCES_DIR,
            "json",
            dir,
            &name,
            "sequence",
        )?,
        (None, Some(path)) => {
            collection::destination_file(store_path, SEQUENCES_DIR, "json", path)?
        }
        (None, None) => collection::default_destination_file(
            store_path,
            SEQUENCES_DIR,
            &name,
            "sequence",
            "json",
        ),
    };
    let data = SequenceFile {
        id: id.clone(),
        name: name.clone(),
        fragments,
        variables: BTreeMap::new(),
        metadata: BTreeMap::new(),
    };
    let content = codec::encode_json(&data)?;
    collection::create_parent_dir(&destination)?;
    fs::write(&destination.path, content).map_err(|source| AppError::WriteFile {
        path: destination.path.clone(),
        source,
    })?;

    Ok(SequenceSummary {
        id,
        name,
        path: destination.store_relative_path,
        fragment_count: data.fragments.len(),
    })
}
