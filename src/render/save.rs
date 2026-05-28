use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::codec;
use crate::collection::{self, Destination};
use crate::error::AppError;

use super::model::{RenderSequence, SavedRenderMetadata};
use super::types::*;
use super::{ID_PREFIX, RENDERS_DIR};

pub(super) fn save_render(
    sequence: &RenderSequence,
    text: &str,
    annotated: bool,
    history_ref: Option<&str>,
    destination: Destination,
) -> Result<SavedRenderSummary, AppError> {
    let id = format!("{ID_PREFIX}{}", Uuid::new_v4().simple());
    let metadata = SavedRenderMetadata {
        id: id.clone(),
        sequence_id: sequence.id.clone(),
        sequence_name: sequence.name.clone(),
        sequence_path: sequence.path.clone(),
        annotated,
        history_ref: history_ref.map(str::to_owned),
    };
    let content = codec::encode_yaml_frontmatter(&metadata, text)?;
    collection::create_parent_dir(&destination)?;
    fs::write(&destination.path, content).map_err(|source| AppError::WriteFile {
        path: destination.path.clone(),
        source,
    })?;

    Ok(SavedRenderSummary {
        id,
        path: destination.store_relative_path,
        git_commit: None,
    })
}

pub(super) fn save_destination(
    store_path: &Path,
    sequence: &RenderSequence,
    dir: Option<&Path>,
    path: Option<&Path>,
) -> Result<Destination, AppError> {
    match (dir, path) {
        (Some(_), Some(_)) => Err(AppError::InvalidCollectionPath {
            kind: "render save placement".to_owned(),
            path: "--dir/--path".to_owned(),
            message: "--dir and --path are mutually exclusive".to_owned(),
        }),
        (Some(dir), None) => collection::destination_file_in_directory(
            store_path,
            RENDERS_DIR,
            "md",
            dir,
            &sequence.name,
            "render",
        ),
        (None, Some(path)) => collection::destination_file(store_path, RENDERS_DIR, "md", path),
        (None, None) => Ok(collection::default_destination_file(
            store_path,
            RENDERS_DIR,
            &sequence.name,
            "render",
            "md",
        )),
    }
}

pub(super) fn path_is_inside_store(store_path: &Path, path: &Path) -> bool {
    let store_path = store_path
        .canonicalize()
        .unwrap_or_else(|_| store_path.to_path_buf());
    let path = absolutize(path);
    let comparable_path = path
        .canonicalize()
        .ok()
        .or_else(|| path.parent().and_then(|parent| parent.canonicalize().ok()))
        .unwrap_or(path);

    comparable_path.starts_with(store_path)
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
