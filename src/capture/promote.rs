use std::path::Path;

use crate::error::AppError;
use crate::fragment;
use crate::sequence;

use super::model::CaptureRecord;
use super::records::capture_summary_ref;
use super::types::CapturePromoteOutput;

pub(super) fn promote_capture_record_uncommitted(
    store_path: &Path,
    capture: &CaptureRecord,
    sequence_name: String,
) -> Result<CapturePromoteOutput, AppError> {
    sequence::validate_name(&sequence_name)?;

    let event_count = capture.data.events.len();
    let mut promoted_fragments = Vec::with_capacity(event_count);
    for event in &capture.data.events {
        let fragment_name = promoted_fragment_name(&sequence_name, event.index, event_count);
        let fragment = fragment::create_uncommitted(store_path, fragment_name, &event.text)?;
        promoted_fragments.push(fragment);
    }

    let sequence = sequence::create_uncommitted(
        store_path,
        sequence_name,
        promoted_fragments
            .iter()
            .map(|fragment| fragment.id.clone())
            .collect(),
    )?;

    Ok(CapturePromoteOutput {
        capture: capture_summary_ref(capture),
        sequence,
        fragments: promoted_fragments,
        event_count,
        git_commit: None,
    })
}

fn promoted_fragment_name(sequence_name: &str, event_index: usize, event_count: usize) -> String {
    if event_count == 1 {
        sequence_name.to_owned()
    } else {
        format!("{sequence_name} {event_index}")
    }
}
