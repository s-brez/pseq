use std::collections::BTreeMap;
use std::path::Path;

use crate::collection;
use crate::error::AppError;
use crate::git;
use crate::resolve;

use super::SEQUENCES_DIR;
use super::fragments::{read_historical_fragments, resolve_sequence_fragments};
use super::model::{HistoricalSequenceFile, HistoricalSequenceRecord, RenderSequence};

pub(super) fn load_historical_sequence(
    store_path: &Path,
    history_ref: &str,
    sequence_reference: &str,
) -> Result<RenderSequence, AppError> {
    let sequences = read_historical_sequences(store_path, history_ref)?;
    let sequence = resolve_historical_sequence(&sequences, sequence_reference)?;
    let catalog = read_historical_fragments(store_path, history_ref)?;
    let fragments = resolve_sequence_fragments(&catalog, &sequence.data.fragments)?;

    Ok(RenderSequence {
        id: sequence.data.id.clone(),
        name: sequence.data.name.clone(),
        path: sequence.path.clone(),
        fragments,
        catalog,
    })
}

fn read_historical_sequences(
    store_path: &Path,
    history_ref: &str,
) -> Result<Vec<HistoricalSequenceRecord>, AppError> {
    let mut sequences = Vec::new();
    for path in git::list_files_at_ref(store_path, history_ref, &[SEQUENCES_DIR])? {
        if !path.ends_with(".json") {
            continue;
        }
        let content = git::show_text_at_ref(store_path, history_ref, &path)?;
        let data: HistoricalSequenceFile =
            serde_json::from_str(&content).map_err(|source| AppError::HistoricalFileInvalid {
                reference: history_ref.to_owned(),
                path: path.clone(),
                message: source.to_string(),
            })?;
        sequences.push(HistoricalSequenceRecord { data, path });
    }
    sequences.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sequences)
}

fn resolve_historical_sequence<'a>(
    sequences: &'a [HistoricalSequenceRecord],
    reference: &str,
) -> Result<&'a HistoricalSequenceRecord, AppError> {
    let mut matches: BTreeMap<String, &'a HistoricalSequenceRecord> = BTreeMap::new();

    for sequence in sequences {
        let id_matches = !reference.is_empty() && sequence.data.id.starts_with(reference);
        if id_matches
            || sequence.data.name == reference
            || collection::matches_explicit_path_reference(
                &sequence.path,
                SEQUENCES_DIR,
                "json",
                reference,
            )
        {
            matches.insert(sequence.path.clone(), sequence);
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

    let mut folded_matches: BTreeMap<String, &'a HistoricalSequenceRecord> = BTreeMap::new();
    for sequence in sequences {
        if collection::folded_path_alias(&sequence.path, SEQUENCES_DIR, "json").as_deref()
            == Some(reference)
        {
            folded_matches.insert(sequence.path.clone(), sequence);
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
