use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::codec;
use crate::collection;
use crate::paths;
use crate::store::{self, ValidationIssue};
use crate::yaml;

use super::model::SavedRenderMetadata;
use super::{ID_PREFIX, RENDERS_DIR, SEQUENCE_ID_PREFIX};

pub fn validate_saved_renders(store_path: &Path) -> Vec<ValidationIssue> {
    let renders_dir = store_path.join(RENDERS_DIR);
    if !renders_dir.is_dir() {
        return Vec::new();
    }

    let mut issues = collection::validate_structure(store_path, RENDERS_DIR);
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in collection::files_with_extension(store_path, RENDERS_DIR, "md") {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                issues.push(store::validation_issue(
                    "render_file_invalid",
                    format!("saved render file must be UTF-8 Markdown: {error}"),
                    Some(&path),
                ));
                continue;
            }
        };

        let (frontmatter, _) = match codec::split_yaml_frontmatter(&content) {
            Ok(parts) => parts,
            Err(message) => {
                issues.push(store::validation_issue(
                    "render_file_invalid",
                    format!("invalid saved render file {}: {message}", path.display()),
                    Some(&path),
                ));
                continue;
            }
        };

        let metadata: SavedRenderMetadata = match yaml::from_str(frontmatter) {
            Ok(metadata) => metadata,
            Err(error) => {
                issues.push(store::validation_issue(
                    "render_file_invalid",
                    format!("invalid saved render metadata {}: {error}", path.display()),
                    Some(&path),
                ));
                continue;
            }
        };

        if metadata.id.trim().is_empty() {
            issues.push(store::validation_issue(
                "render_id_missing",
                format!("saved render id is empty: {}", path.display()),
                Some(&path),
            ));
        } else if !store::is_valid_typed_id(&metadata.id, ID_PREFIX) {
            issues.push(store::validation_issue(
                "render_id_invalid",
                format!(
                    "saved render id must match {ID_PREFIX}<uuid>: {}",
                    metadata.id
                ),
                Some(&path),
            ));
        } else {
            ids.entry(metadata.id.clone())
                .or_default()
                .push(paths::store_relative(store_path, &path));
        }
        if metadata.sequence_id.trim().is_empty() {
            issues.push(store::validation_issue(
                "render_sequence_id_missing",
                format!("saved render sequence_id is empty: {}", path.display()),
                Some(&path),
            ));
        } else if !store::is_valid_typed_id(&metadata.sequence_id, SEQUENCE_ID_PREFIX) {
            issues.push(store::validation_issue(
                "render_sequence_id_invalid",
                format!(
                    "saved render sequence_id must match {SEQUENCE_ID_PREFIX}<uuid>: {}",
                    metadata.sequence_id
                ),
                Some(&path),
            ));
        }
        if metadata.sequence_name.trim().is_empty() {
            issues.push(store::validation_issue(
                "render_sequence_name_missing",
                format!("saved render sequence_name is empty: {}", path.display()),
                Some(&path),
            ));
        }
        if metadata.sequence_path.trim().is_empty() {
            issues.push(store::validation_issue(
                "render_sequence_path_missing",
                format!("saved render sequence_path is empty: {}", path.display()),
                Some(&path),
            ));
        } else if !is_valid_store_relative_sequence_path(&metadata.sequence_path) {
            issues.push(store::validation_issue(
                "render_sequence_path_invalid",
                format!(
                    "saved render sequence_path must be a forward-slash store-relative sequence path: {}",
                    metadata.sequence_path
                ),
                Some(&path),
            ));
        }
    }

    store::push_duplicate_id_issues(&mut issues, ids, "render_id_duplicate", "saved render");

    issues
}

fn is_valid_store_relative_sequence_path(path: &str) -> bool {
    path.starts_with("sequences/")
        && path.ends_with(".json")
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}
