use std::collections::BTreeMap;
use std::path::Path;

use crate::collection;
use crate::store::{self, ValidationIssue};

use super::model::CaptureFile;
use super::records::read_capture_file;
use super::sources::is_supported_source;
use super::types::CaptureOrigin;
use super::{CAPTURE_VERSION, CAPTURES_DIR, ID_PREFIX};

pub fn validate_captures(store_path: &Path) -> Vec<ValidationIssue> {
    let captures_dir = store_path.join(CAPTURES_DIR);
    if !captures_dir.is_dir() {
        return Vec::new();
    }

    let mut issues = collection::validate_structure(store_path, CAPTURES_DIR);
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in collection::files_with_extension(store_path, CAPTURES_DIR, "json") {
        match read_capture_file(store_path, &path) {
            Ok(capture) => {
                if capture.data.id.trim().is_empty() {
                    issues.push(store::validation_issue(
                        "capture_id_missing",
                        format!("capture id is empty: {}", path.display()),
                        Some(&path),
                    ));
                } else {
                    ids.entry(capture.data.id.clone())
                        .or_default()
                        .push(capture.store_relative_path.clone());
                }

                if let Err(message) = validate_capture_schema(&capture.data) {
                    issues.push(store::validation_issue(
                        "capture_file_invalid",
                        format!("invalid capture file {}: {message}", path.display()),
                        Some(&path),
                    ));
                }
            }
            Err(message) => issues.push(store::validation_issue(
                "capture_file_invalid",
                format!("invalid capture file {}: {message}", path.display()),
                Some(&path),
            )),
        }
    }

    store::push_duplicate_id_issues(&mut issues, ids, "capture_id_duplicate", "capture");

    issues
}

fn validate_capture_schema(capture: &CaptureFile) -> Result<(), String> {
    if capture.version != CAPTURE_VERSION {
        return Err(format!(
            "unsupported capture version {}; expected {CAPTURE_VERSION}",
            capture.version
        ));
    }

    if !store::is_valid_typed_id(&capture.id, ID_PREFIX) {
        return Err(format!("capture id must match {ID_PREFIX}<uuid>"));
    }

    match &capture.origin {
        CaptureOrigin::Stdin => {}
        CaptureOrigin::File { path } if !path.trim().is_empty() => {}
        CaptureOrigin::File { .. } => return Err("file capture origin path is empty".to_owned()),
        CaptureOrigin::Source { source, session } => {
            if source.trim().is_empty() {
                return Err("source capture origin source is empty".to_owned());
            }
            if !is_supported_source(source) {
                return Err(format!(
                    "source capture origin source is unsupported: {source}"
                ));
            }
            if session.path.trim().is_empty() {
                return Err("source capture origin session path is empty".to_owned());
            }
        }
    }

    if capture.events.is_empty() {
        return Err("capture must contain at least one event".to_owned());
    }

    for (position, event) in capture.events.iter().enumerate() {
        let expected = position + 1;
        if event.index != expected {
            return Err(format!(
                "capture event index {} must be {expected}",
                event.index
            ));
        }
    }

    Ok(())
}
