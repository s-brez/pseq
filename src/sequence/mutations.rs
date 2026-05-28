use std::fs;
use std::path::{Path, PathBuf};

use crate::codec;
use crate::collection;
use crate::commit::{self, CommitMode};
use crate::error::AppError;
use crate::fragment;
use crate::store;

use super::records::{
    checked_sequence_index, checked_sequence_insert_index, read_sequences, resolve_sequence_index,
    write_sequence_file,
};
use super::types::*;
use super::validation::validate_name;

pub fn add(
    store_path: &Path,
    sequence_reference: &str,
    fragment_reference: &str,
    insert_at: Option<usize>,
    commit_mode: CommitMode,
) -> Result<SequenceAddOutput, AppError> {
    store::require_valid_store(store_path)?;
    let fragment = fragment::resolve_summary(store_path, fragment_reference)?;

    let mut sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, sequence_reference)?;
    let sequence = &mut sequences[index];
    let insert_index = insert_at
        .map(|index| checked_sequence_insert_index(index, sequence.data.fragments.len()))
        .transpose()?
        .unwrap_or(sequence.data.fragments.len());
    sequence
        .data
        .fragments
        .insert(insert_index, fragment.id.clone());

    let content = codec::encode_json(&sequence.data)?;
    fs::write(&sequence.path, content).map_err(|source| AppError::WriteFile {
        path: sequence.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.store_relative_path)],
        &format!(
            "Add fragment {} to sequence {}",
            fragment.name, sequence.data.name
        ),
    )?;

    Ok(SequenceAddOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        fragment,
        index: insert_index + 1,
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}

pub fn remove_fragment(
    store_path: &Path,
    sequence_reference: &str,
    fragment_reference_or_index: &str,
    commit_mode: CommitMode,
) -> Result<SequenceFragmentRemoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let mut sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, sequence_reference)?;

    let (remove_index, removed_fragment) =
        if let Ok(parsed_index) = fragment_reference_or_index.parse::<usize>() {
            let sequence = &sequences[index];
            let remove_index = checked_sequence_index(parsed_index, sequence.data.fragments.len())?;
            let fragment =
                fragment::resolve_summary(store_path, &sequence.data.fragments[remove_index])?;
            (remove_index, fragment)
        } else {
            let fragment = fragment::resolve_summary(store_path, fragment_reference_or_index)?;
            let sequence = &sequences[index];
            let positions = sequence
                .data
                .fragments
                .iter()
                .enumerate()
                .filter_map(|(index, fragment_reference)| {
                    (fragment_reference == &fragment.id).then_some(index)
                })
                .collect::<Vec<_>>();

            match positions.as_slice() {
                [] => {
                    return Err(AppError::SequenceFragmentNotFound {
                        reference: fragment_reference_or_index.to_owned(),
                    });
                }
                [position] => (*position, fragment),
                _ => {
                    let positions = positions
                        .iter()
                        .map(|position| (position + 1).to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    return Err(AppError::SequenceFragmentReferenceAmbiguous {
                        reference: fragment_reference_or_index.to_owned(),
                        positions,
                    });
                }
            }
        };

    let sequence = &mut sequences[index];
    sequence.data.fragments.remove(remove_index);
    write_sequence_file(sequence)?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.store_relative_path)],
        &format!(
            "Remove fragment {} from sequence {}",
            removed_fragment.name, sequence.data.name
        ),
    )?;

    Ok(SequenceFragmentRemoveOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        removed_fragment,
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}

pub fn move_fragment(
    store_path: &Path,
    sequence_reference: &str,
    from_index: usize,
    to_index: usize,
    commit_mode: CommitMode,
) -> Result<SequenceMoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let mut sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, sequence_reference)?;
    let sequence = &mut sequences[index];
    let len = sequence.data.fragments.len();
    let from = checked_sequence_index(from_index, len)?;
    let to = checked_sequence_index(to_index, len)?;

    let fragment_reference = sequence.data.fragments.remove(from);
    sequence.data.fragments.insert(to, fragment_reference);
    write_sequence_file(sequence)?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.store_relative_path)],
        &format!("Move fragment in sequence {}", sequence.data.name),
    )?;

    Ok(SequenceMoveOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        from_index,
        to_index,
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}

pub fn rename(
    store_path: &Path,
    reference: &str,
    name: String,
    commit_mode: CommitMode,
) -> Result<SequenceRenameOutput, AppError> {
    validate_name(&name)?;
    store::require_valid_store(store_path)?;
    let mut sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &mut sequences[index];
    sequence.data.name = name.clone();
    write_sequence_file(sequence)?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.store_relative_path)],
        &format!("Rename sequence {name}"),
    )?;

    Ok(SequenceRenameOutput {
        id: sequence.data.id.clone(),
        name,
        path: sequence.store_relative_path.clone(),
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}

pub fn move_file(
    store_path: &Path,
    reference: &str,
    destination: &Path,
    commit_mode: CommitMode,
) -> Result<SequencePathMoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &sequences[index];
    let destination =
        collection::destination_file(store_path, super::SEQUENCES_DIR, "json", destination)?;

    collection::create_parent_dir(&destination)?;
    fs::rename(&sequence.path, &destination.path).map_err(|source| AppError::MoveFile {
        from: sequence.path.clone(),
        to: destination.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[
            PathBuf::from(&sequence.store_relative_path),
            PathBuf::from(&destination.store_relative_path),
        ],
        &format!("Move sequence {}", sequence.data.name),
    )?;

    Ok(SequencePathMoveOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: destination.store_relative_path,
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}

pub fn remove(
    store_path: &Path,
    reference: &str,
    commit_mode: CommitMode,
) -> Result<SequenceRemoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &sequences[index];

    fs::remove_file(&sequence.path).map_err(|source| AppError::RemoveFile {
        path: sequence.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&sequence.store_relative_path)],
        &format!("Remove sequence {}", sequence.data.name),
    )?;

    Ok(SequenceRemoveOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        fragment_count: sequence.data.fragments.len(),
        git_commit,
    })
}
