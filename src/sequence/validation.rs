use std::collections::BTreeMap;
use std::path::Path;

use crate::collection;
use crate::error::AppError;
use crate::fragment;
use crate::store::{self, ValidationIssue};

use super::model::SequenceRecord;
use super::records::read_sequence_file;
use super::{FRAGMENT_ID_PREFIX, ID_PREFIX, SEQUENCES_DIR};

pub(crate) fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        Err(AppError::InvalidSequenceName {
            name: name.to_owned(),
        })
    } else {
        Ok(())
    }
}

pub fn validate_sequences(store_path: &Path) -> Vec<ValidationIssue> {
    let sequences_dir = store_path.join(SEQUENCES_DIR);
    if !sequences_dir.is_dir() {
        return Vec::new();
    }

    let mut issues = collection::validate_structure(store_path, SEQUENCES_DIR);
    let fragment_id_counts = fragment::fragment_id_counts(store_path);
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in collection::files_with_extension(store_path, SEQUENCES_DIR, "json") {
        match read_sequence_file(store_path, &path) {
            Ok(sequence) => {
                validate_sequence_record(&sequence, &fragment_id_counts, &mut issues);
                if !sequence.data.id.trim().is_empty() {
                    ids.entry(sequence.data.id.clone())
                        .or_default()
                        .push(sequence.store_relative_path.clone());
                }
            }
            Err(message) => issues.push(store::validation_issue(
                "sequence_file_invalid",
                format!("invalid sequence file {}: {message}", path.display()),
                Some(&path),
            )),
        }
    }

    store::push_duplicate_id_issues(&mut issues, ids, "sequence_id_duplicate", "sequence");

    issues
}

pub(super) fn validate_edited_sequence(
    store_path: &Path,
    original: &SequenceRecord,
    edited: &SequenceRecord,
) -> Result<(), AppError> {
    if edited.data.id != original.data.id {
        return Err(AppError::InvalidEditedSequence {
            message: "sequence id is stable and cannot be changed by edit".to_owned(),
        });
    }

    let mut issues = Vec::new();
    let fragment_id_counts = fragment::fragment_id_counts(store_path);
    validate_sequence_record(edited, &fragment_id_counts, &mut issues);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(AppError::InvalidEditedSequence {
            message: issues
                .into_iter()
                .map(|issue| format!("{}: {}", issue.code, issue.message))
                .collect::<Vec<_>>()
                .join("; "),
        })
    }
}

fn validate_sequence_record(
    sequence: &SequenceRecord,
    fragment_id_counts: &BTreeMap<String, usize>,
    issues: &mut Vec<ValidationIssue>,
) {
    if sequence.data.id.trim().is_empty() {
        issues.push(store::validation_issue(
            "sequence_id_missing",
            format!("sequence id is empty: {}", sequence.path.display()),
            Some(&sequence.path),
        ));
    } else if !store::is_valid_typed_id(&sequence.data.id, ID_PREFIX) {
        issues.push(store::validation_issue(
            "sequence_id_invalid",
            format!(
                "sequence id must match {ID_PREFIX}<uuid>: {}",
                sequence.data.id
            ),
            Some(&sequence.path),
        ));
    }

    if sequence.data.name.trim().is_empty() {
        issues.push(store::validation_issue(
            "sequence_name_missing",
            format!("sequence name is empty: {}", sequence.path.display()),
            Some(&sequence.path),
        ));
    }

    for (index, fragment_ref) in sequence.data.fragments.iter().enumerate() {
        let position = index + 1;
        if fragment_ref.trim().is_empty() {
            issues.push(store::validation_issue(
                "sequence_fragment_ref_missing",
                format!(
                    "sequence {} has an empty fragment reference at position {position}",
                    sequence.store_relative_path
                ),
                Some(&sequence.path),
            ));
            continue;
        }
        if !store::is_valid_typed_id(fragment_ref, FRAGMENT_ID_PREFIX) {
            issues.push(store::validation_issue(
                "sequence_fragment_ref_invalid",
                format!(
                    "sequence {} has invalid fragment id {fragment_ref} at position {position}; expected {FRAGMENT_ID_PREFIX}<uuid>",
                    sequence.store_relative_path
                ),
                Some(&sequence.path),
            ));
            continue;
        }

        match fragment_id_counts.get(fragment_ref) {
            Some(1) => {}
            Some(count) => issues.push(store::validation_issue(
                "sequence_fragment_ref_ambiguous",
                format!(
                    "sequence {} references fragment id {fragment_ref} at position {position}, but {count} fragments share that id",
                    sequence.store_relative_path
                ),
                Some(&sequence.path),
            )),
            None => issues.push(store::validation_issue(
                "sequence_fragment_missing",
                format!(
                    "sequence {} references missing fragment id {fragment_ref} at position {position}",
                    sequence.store_relative_path
                ),
                Some(&sequence.path),
            )),
        }
    }
}
