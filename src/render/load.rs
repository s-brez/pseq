use std::path::Path;

use crate::error::AppError;
use crate::sequence;

use super::fragments::{read_current_fragments, resolve_sequence_fragments};
use super::model::RenderSequence;

pub(crate) fn load_current_sequence(
    store_path: &Path,
    sequence_reference: &str,
) -> Result<RenderSequence, AppError> {
    let sequence = sequence::render_source(store_path, sequence_reference)?;
    let catalog = read_current_fragments(store_path)?;
    let fragments = resolve_sequence_fragments(&catalog, &sequence.fragment_references)?;

    Ok(RenderSequence {
        id: sequence.id,
        name: sequence.name,
        path: sequence.path,
        fragments,
        catalog,
    })
}
