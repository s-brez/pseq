use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::codec;
use crate::collection;
use crate::error::AppError;
use crate::fragment::{self, FragmentSummary};
use crate::paths;
use crate::resolve;
use crate::store;

use super::SEQUENCES_DIR;
use super::model::{SequenceFile, SequenceRecord};
use super::types::*;

pub(super) fn read_sequences(store_path: &Path) -> Result<Vec<SequenceRecord>, AppError> {
    let mut sequences = Vec::new();
    for path in collection::files_with_extension(store_path, SEQUENCES_DIR, "json") {
        match read_sequence_file(store_path, &path) {
            Ok(sequence) => sequences.push(sequence),
            Err(_) => return Err(store::invalid_store(store_path)),
        }
    }

    sequences.sort_by(|left, right| left.store_relative_path.cmp(&right.store_relative_path));
    Ok(sequences)
}

pub(super) fn read_sequence_file(store_path: &Path, path: &Path) -> Result<SequenceRecord, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_sequence_content(store_path, path, &content)
}

pub(super) fn parse_sequence_content(
    store_path: &Path,
    path: &Path,
    content: &str,
) -> Result<SequenceRecord, String> {
    let data: SequenceFile = serde_json::from_str(content).map_err(|error| error.to_string())?;

    Ok(SequenceRecord {
        data,
        path: path.to_path_buf(),
        store_relative_path: paths::store_relative(store_path, path),
    })
}

pub(super) fn write_sequence_file(sequence: &SequenceRecord) -> Result<(), AppError> {
    let content = codec::encode_json(&sequence.data)?;
    fs::write(&sequence.path, content).map_err(|source| AppError::WriteFile {
        path: sequence.path.clone(),
        source,
    })
}

pub(super) fn checked_sequence_index(index: usize, len: usize) -> Result<usize, AppError> {
    if (1..=len).contains(&index) {
        Ok(index - 1)
    } else {
        Err(AppError::InvalidSequenceIndex { index, len })
    }
}

pub(super) fn checked_sequence_insert_index(index: usize, len: usize) -> Result<usize, AppError> {
    let upper_bound = len.saturating_add(1);
    if (1..=upper_bound).contains(&index) {
        Ok(index - 1)
    } else {
        Err(AppError::InvalidSequenceIndex {
            index,
            len: upper_bound,
        })
    }
}

pub(super) fn resolve_sequence_index(
    sequences: &[SequenceRecord],
    reference: &str,
) -> Result<usize, AppError> {
    let mut matches: BTreeMap<String, usize> = BTreeMap::new();

    for (index, sequence) in sequences.iter().enumerate() {
        let id = &sequence.data.id;
        let id_matches = !reference.is_empty() && id.starts_with(reference);

        if id_matches
            || sequence.data.name == reference
            || collection::matches_explicit_path_reference(
                &sequence.store_relative_path,
                SEQUENCES_DIR,
                "json",
                reference,
            )
        {
            matches.insert(sequence.store_relative_path.clone(), index);
        }
    }

    if !matches.is_empty() {
        return resolve::single_match(
            matches,
            || unreachable!("matches is not empty"),
            |matches| AppError::SequenceReferenceAmbiguous {
                reference: reference.to_owned(),
                matches,
            },
        );
    }

    let mut folded_matches: BTreeMap<String, usize> = BTreeMap::new();
    for (index, sequence) in sequences.iter().enumerate() {
        if collection::folded_path_alias(&sequence.store_relative_path, SEQUENCES_DIR, "json")
            .as_deref()
            == Some(reference)
        {
            folded_matches.insert(sequence.store_relative_path.clone(), index);
        }
    }

    resolve::single_match(
        folded_matches,
        || AppError::SequenceNotFound {
            reference: reference.to_owned(),
        },
        |matches| AppError::SequenceReferenceAmbiguous {
            reference: reference.to_owned(),
            matches,
        },
    )
}

pub(super) fn resolve_fragment_summaries(
    store_path: &Path,
    fragment_references: &[String],
) -> Result<Vec<FragmentSummary>, AppError> {
    let mut fragments = Vec::new();
    for fragment_reference in fragment_references {
        fragments.push(fragment::resolve_summary(store_path, fragment_reference)?);
    }
    Ok(fragments)
}

pub(super) fn sequence_summary(sequence: SequenceRecord) -> SequenceSummary {
    SequenceSummary {
        id: sequence.data.id,
        name: sequence.data.name,
        path: sequence.store_relative_path,
        fragment_count: sequence.data.fragments.len(),
    }
}
