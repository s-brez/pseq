use std::path::Path;

use crate::collection;
use crate::error::AppError;
use crate::store;

use super::SEQUENCES_DIR;
use super::records::{
    read_sequences, resolve_fragment_summaries, resolve_sequence_index, sequence_summary,
};
use super::types::*;

pub fn list(
    store_path: &Path,
    prefix: Option<&Path>,
    tree: bool,
) -> Result<SequenceListOutput, AppError> {
    store::require_valid_store(store_path)?;
    collection::validate_prefix(SEQUENCES_DIR, prefix)?;
    let mut sequences = Vec::new();
    for sequence in read_sequences(store_path)? {
        if collection::prefix_matches(&sequence.store_relative_path, SEQUENCES_DIR, "json", prefix)?
        {
            sequences.push(sequence_summary(sequence));
        }
    }

    Ok(SequenceListOutput { sequences, tree })
}

pub fn show(store_path: &Path, reference: &str) -> Result<SequenceShowOutput, AppError> {
    store::require_valid_store(store_path)?;
    let sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &sequences[index];

    Ok(SequenceShowOutput {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        fragments: resolve_fragment_summaries(store_path, &sequence.data.fragments)?,
        variables: sequence.data.variables.clone(),
        metadata: sequence.data.metadata.clone(),
    })
}

pub(crate) fn render_source(
    store_path: &Path,
    reference: &str,
) -> Result<SequenceRenderSource, AppError> {
    store::require_valid_store(store_path)?;
    let sequences = read_sequences(store_path)?;
    let index = resolve_sequence_index(&sequences, reference)?;
    let sequence = &sequences[index];

    Ok(SequenceRenderSource {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.store_relative_path.clone(),
        fragment_references: sequence.data.fragments.clone(),
    })
}

pub(crate) fn sequences_referencing_fragment(
    store_path: &Path,
    fragment_id: &str,
) -> Result<Vec<SequenceSummary>, AppError> {
    Ok(read_sequences(store_path)?
        .into_iter()
        .filter(|sequence| {
            sequence
                .data
                .fragments
                .iter()
                .any(|fragment_reference| fragment_reference == fragment_id)
        })
        .map(sequence_summary)
        .collect())
}
