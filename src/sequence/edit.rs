use std::fs;
use std::path::{Path, PathBuf};

use crate::commit::{self, CommitMode};
use crate::editor;
use crate::error::AppError;
use crate::store;

use super::records::{parse_sequence_content, read_sequences, resolve_sequence_index};
use super::types::*;
use super::validation::validate_edited_sequence;

pub fn edit(
    store_path: &Path,
    reference: &str,
    commit_mode: CommitMode,
) -> Result<SequenceEditOutput, AppError> {
    store::require_valid_store(store_path)?;
    let sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &sequences[index];
    let original = fs::read_to_string(&sequence.path).map_err(|source| AppError::ReadFile {
        path: sequence.path.clone(),
        source,
    })?;

    let Some(edited) = editor::edit_text(&original, "json")? else {
        return Ok(SequenceEditOutput {
            id: sequence.data.id.clone(),
            name: sequence.data.name.clone(),
            path: sequence.store_relative_path.clone(),
            fragment_count: sequence.data.fragments.len(),
            git_commit: None,
        });
    };

    let edited_sequence = parse_sequence_content(store_path, &sequence.path, &edited)
        .map_err(|message| AppError::InvalidEditedSequence { message })?;
    validate_edited_sequence(store_path, sequence, &edited_sequence)?;

    fs::write(&sequence.path, edited).map_err(|source| AppError::WriteFile {
        path: sequence.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&edited_sequence.store_relative_path)],
        &format!("Edit sequence {}", edited_sequence.data.name),
    )?;

    Ok(SequenceEditOutput {
        id: edited_sequence.data.id,
        name: edited_sequence.data.name,
        path: edited_sequence.store_relative_path,
        fragment_count: edited_sequence.data.fragments.len(),
        git_commit,
    })
}
