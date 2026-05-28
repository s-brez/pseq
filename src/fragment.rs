use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec;
use crate::collection;
use crate::commit::{self, CommitMode};
use crate::editor;
use crate::error::AppError;
use crate::paths;
use crate::resolve;
use crate::sequence;
use crate::store::{self, ValidationIssue};
use crate::yaml;

const FRAGMENTS_DIR: &str = "fragments";
const ID_PREFIX: &str = "frg_";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FragmentFrontmatter {
    id: String,
    name: String,
    #[serde(flatten)]
    metadata: BTreeMap<String, yaml::Value>,
}

#[derive(Debug, Clone)]
struct FragmentRecord {
    metadata: FragmentFrontmatter,
    path: PathBuf,
    store_relative_path: String,
    body: String,
}

#[derive(Debug, Serialize)]
pub struct FragmentNewOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FragmentListOutput {
    pub fragments: Vec<FragmentSummary>,
    #[serde(skip)]
    pub tree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FragmentSummary {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct FragmentShowOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, yaml::Value>,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct FragmentEditOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FragmentRenameOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FragmentMoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FragmentRemoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

pub(crate) fn resolve_summary(
    store_path: &Path,
    reference: &str,
) -> Result<FragmentSummary, AppError> {
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;

    Ok(FragmentSummary {
        id: fragment.metadata.id.clone(),
        name: fragment.metadata.name.clone(),
        path: fragment.store_relative_path.clone(),
    })
}

pub(crate) fn fragment_id_counts(store_path: &Path) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for path in collection::files_with_extension(store_path, FRAGMENTS_DIR, "md") {
        let Ok(fragment) = read_fragment_file(store_path, &path) else {
            continue;
        };
        if fragment.metadata.id.trim().is_empty() {
            continue;
        }
        *counts.entry(fragment.metadata.id).or_insert(0) += 1;
    }
    counts
}

pub fn create(
    store_path: &Path,
    name: String,
    from_file: Option<&Path>,
    read_stdin: bool,
    dir: Option<&Path>,
    path: Option<&Path>,
    commit_mode: CommitMode,
) -> Result<FragmentNewOutput, AppError> {
    store::require_valid_store(store_path)?;

    let body = match (from_file, read_stdin) {
        (Some(path), false) => fs::read_to_string(path).map_err(|source| AppError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?,
        (None, true) => {
            let mut input = String::new();
            io::stdin()
                .read_to_string(&mut input)
                .map_err(|source| AppError::ReadStdin { source })?;
            input
        }
        _ => String::new(),
    };

    let fragment = create_uncommitted_placed(store_path, name, &body, dir, path)?;
    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&fragment.path)],
        &format!("Add fragment {}", fragment.name),
    )?;

    Ok(FragmentNewOutput {
        id: fragment.id,
        name: fragment.name,
        path: fragment.path,
        git_commit,
    })
}

pub(crate) fn create_uncommitted(
    store_path: &Path,
    name: String,
    body: &str,
) -> Result<FragmentSummary, AppError> {
    create_uncommitted_placed(store_path, name, body, None, None)
}

fn create_uncommitted_placed(
    store_path: &Path,
    name: String,
    body: &str,
    dir: Option<&Path>,
    path: Option<&Path>,
) -> Result<FragmentSummary, AppError> {
    validate_name(&name)?;

    let id = format!("{ID_PREFIX}{}", Uuid::new_v4().simple());
    let destination = match (dir, path) {
        (Some(_), Some(_)) => {
            return Err(AppError::InvalidCollectionPath {
                kind: "fragment placement".to_owned(),
                path: "--dir/--path".to_owned(),
                message: "--dir and --path are mutually exclusive".to_owned(),
            });
        }
        (Some(dir), None) => collection::destination_file_in_directory(
            store_path,
            FRAGMENTS_DIR,
            "md",
            dir,
            &name,
            "fragment",
        )?,
        (None, Some(path)) => collection::destination_file(store_path, FRAGMENTS_DIR, "md", path)?,
        (None, None) => {
            collection::default_destination_file(store_path, FRAGMENTS_DIR, &name, "fragment", "md")
        }
    };
    let metadata = FragmentFrontmatter {
        id: id.clone(),
        name: name.clone(),
        metadata: BTreeMap::new(),
    };
    let content = codec::encode_yaml_frontmatter(&metadata, body)?;
    collection::create_parent_dir(&destination)?;
    fs::write(&destination.path, content).map_err(|source| AppError::WriteFile {
        path: destination.path.clone(),
        source,
    })?;

    Ok(FragmentSummary {
        id,
        name,
        path: destination.store_relative_path,
    })
}

pub fn list(
    store_path: &Path,
    prefix: Option<&Path>,
    tree: bool,
) -> Result<FragmentListOutput, AppError> {
    store::require_valid_store(store_path)?;
    collection::validate_prefix(FRAGMENTS_DIR, prefix)?;
    let mut fragments = Vec::new();
    for fragment in read_fragments(store_path)? {
        if collection::prefix_matches(&fragment.store_relative_path, FRAGMENTS_DIR, "md", prefix)? {
            fragments.push(FragmentSummary {
                id: fragment.metadata.id,
                name: fragment.metadata.name,
                path: fragment.store_relative_path,
            });
        }
    }

    Ok(FragmentListOutput { fragments, tree })
}

pub fn show(store_path: &Path, reference: &str) -> Result<FragmentShowOutput, AppError> {
    store::require_valid_store(store_path)?;
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;

    Ok(FragmentShowOutput {
        id: fragment.metadata.id.clone(),
        name: fragment.metadata.name.clone(),
        path: fragment.store_relative_path.clone(),
        metadata: fragment.metadata.metadata.clone(),
        body: fragment.body.clone(),
    })
}

pub fn edit(
    store_path: &Path,
    reference: &str,
    commit_mode: CommitMode,
) -> Result<FragmentEditOutput, AppError> {
    store::require_valid_store(store_path)?;
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;
    let original = fs::read_to_string(&fragment.path).map_err(|source| AppError::ReadFile {
        path: fragment.path.clone(),
        source,
    })?;

    let Some(edited) = editor::edit_text(&original, "md")? else {
        return Ok(FragmentEditOutput {
            id: fragment.metadata.id.clone(),
            name: fragment.metadata.name.clone(),
            path: fragment.store_relative_path.clone(),
            git_commit: None,
        });
    };

    let edited_fragment = parse_fragment_content(store_path, &fragment.path, &edited)
        .map_err(|message| AppError::InvalidEditedFragment { message })?;
    validate_edited_fragment(fragment, &edited_fragment)?;

    fs::write(&fragment.path, edited).map_err(|source| AppError::WriteFile {
        path: fragment.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&edited_fragment.store_relative_path)],
        &format!("Edit fragment {}", edited_fragment.metadata.name),
    )?;

    Ok(FragmentEditOutput {
        id: edited_fragment.metadata.id,
        name: edited_fragment.metadata.name,
        path: edited_fragment.store_relative_path,
        git_commit,
    })
}

pub fn rename(
    store_path: &Path,
    reference: &str,
    name: String,
    commit_mode: CommitMode,
) -> Result<FragmentRenameOutput, AppError> {
    validate_name(&name)?;
    store::require_valid_store(store_path)?;
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;
    let metadata = FragmentFrontmatter {
        id: fragment.metadata.id.clone(),
        name: name.clone(),
        metadata: fragment.metadata.metadata.clone(),
    };
    let content = codec::encode_yaml_frontmatter(&metadata, &fragment.body)?;

    fs::write(&fragment.path, content).map_err(|source| AppError::WriteFile {
        path: fragment.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&fragment.store_relative_path)],
        &format!("Rename fragment {name}"),
    )?;

    Ok(FragmentRenameOutput {
        id: metadata.id,
        name,
        path: fragment.store_relative_path.clone(),
        git_commit,
    })
}

pub fn move_file(
    store_path: &Path,
    reference: &str,
    destination: &Path,
    commit_mode: CommitMode,
) -> Result<FragmentMoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;
    let destination = collection::destination_file(store_path, FRAGMENTS_DIR, "md", destination)?;

    collection::create_parent_dir(&destination)?;
    fs::rename(&fragment.path, &destination.path).map_err(|source| AppError::MoveFile {
        from: fragment.path.clone(),
        to: destination.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[
            PathBuf::from(&fragment.store_relative_path),
            PathBuf::from(&destination.store_relative_path),
        ],
        &format!("Move fragment {}", fragment.metadata.name),
    )?;

    Ok(FragmentMoveOutput {
        id: fragment.metadata.id.clone(),
        name: fragment.metadata.name.clone(),
        path: destination.store_relative_path,
        git_commit,
    })
}

pub(crate) fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        Err(AppError::InvalidFragmentName {
            name: name.to_owned(),
        })
    } else {
        Ok(())
    }
}

pub fn remove(
    store_path: &Path,
    reference: &str,
    commit_mode: CommitMode,
) -> Result<FragmentRemoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    let fragments = read_fragments(store_path)?;
    let fragment = resolve_fragment(&fragments, reference)?;
    let usages = sequence::sequences_referencing_fragment(store_path, &fragment.metadata.id)?;
    if !usages.is_empty() {
        let sequences = usages
            .iter()
            .map(|sequence| sequence.path.clone())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::FragmentInUse {
            reference: reference.to_owned(),
            sequences,
        });
    }

    fs::remove_file(&fragment.path).map_err(|source| AppError::RemoveFile {
        path: fragment.path.clone(),
        source,
    })?;

    let git_commit = commit::maybe_commit_paths(
        commit_mode,
        store_path,
        &[PathBuf::from(&fragment.store_relative_path)],
        &format!("Remove fragment {}", fragment.metadata.name),
    )?;

    Ok(FragmentRemoveOutput {
        id: fragment.metadata.id.clone(),
        name: fragment.metadata.name.clone(),
        path: fragment.store_relative_path.clone(),
        git_commit,
    })
}

pub fn validate_fragments(store_path: &Path) -> Vec<ValidationIssue> {
    let fragments_dir = store_path.join(FRAGMENTS_DIR);
    if !fragments_dir.is_dir() {
        return Vec::new();
    }

    let mut issues = collection::validate_structure(store_path, FRAGMENTS_DIR);
    let mut ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in collection::files_with_extension(store_path, FRAGMENTS_DIR, "md") {
        match read_fragment_file(store_path, &path) {
            Ok(fragment) => {
                if fragment.metadata.id.trim().is_empty() {
                    issues.push(store::validation_issue(
                        "fragment_id_missing",
                        format!("fragment id is empty: {}", path.display()),
                        Some(&path),
                    ));
                } else if !store::is_valid_typed_id(&fragment.metadata.id, ID_PREFIX) {
                    issues.push(store::validation_issue(
                        "fragment_id_invalid",
                        format!(
                            "fragment id must match {ID_PREFIX}<uuid>: {}",
                            fragment.metadata.id
                        ),
                        Some(&path),
                    ));
                } else {
                    ids.entry(fragment.metadata.id)
                        .or_default()
                        .push(fragment.store_relative_path);
                }

                if fragment.metadata.name.trim().is_empty() {
                    issues.push(store::validation_issue(
                        "fragment_name_missing",
                        format!("fragment name is empty: {}", path.display()),
                        Some(&path),
                    ));
                }
            }
            Err(message) => issues.push(store::validation_issue(
                "fragment_file_invalid",
                format!("invalid fragment file {}: {message}", path.display()),
                Some(&path),
            )),
        }
    }

    store::push_duplicate_id_issues(&mut issues, ids, "fragment_id_duplicate", "fragment");

    issues
}

fn read_fragments(store_path: &Path) -> Result<Vec<FragmentRecord>, AppError> {
    let mut fragments = Vec::new();
    for path in collection::files_with_extension(store_path, FRAGMENTS_DIR, "md") {
        match read_fragment_file(store_path, &path) {
            Ok(fragment) => fragments.push(fragment),
            Err(_) => return Err(store::invalid_store(store_path)),
        }
    }

    fragments.sort_by(|left, right| left.store_relative_path.cmp(&right.store_relative_path));
    Ok(fragments)
}

fn read_fragment_file(store_path: &Path, path: &Path) -> Result<FragmentRecord, String> {
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_fragment_content(store_path, path, &content)
}

fn parse_fragment_content(
    store_path: &Path,
    path: &Path,
    content: &str,
) -> Result<FragmentRecord, String> {
    let (frontmatter, body) = codec::split_yaml_frontmatter(content)?;
    let metadata: FragmentFrontmatter =
        yaml::from_str(frontmatter).map_err(|error| error.to_string())?;

    Ok(FragmentRecord {
        metadata,
        path: path.to_path_buf(),
        store_relative_path: paths::store_relative(store_path, path),
        body: body.to_owned(),
    })
}

fn validate_edited_fragment(
    original: &FragmentRecord,
    edited: &FragmentRecord,
) -> Result<(), AppError> {
    if edited.metadata.id.trim().is_empty() {
        return Err(AppError::InvalidEditedFragment {
            message: "fragment id is empty".to_owned(),
        });
    }
    if edited.metadata.id != original.metadata.id {
        return Err(AppError::InvalidEditedFragment {
            message: "fragment id is stable and cannot be changed by edit".to_owned(),
        });
    }
    if edited.metadata.name.trim().is_empty() {
        return Err(AppError::InvalidEditedFragment {
            message: "fragment name is empty".to_owned(),
        });
    }

    Ok(())
}

fn resolve_fragment<'a>(
    fragments: &'a [FragmentRecord],
    reference: &str,
) -> Result<&'a FragmentRecord, AppError> {
    let mut matches: BTreeMap<String, &'a FragmentRecord> = BTreeMap::new();

    for fragment in fragments {
        let id = &fragment.metadata.id;
        let id_matches = !reference.is_empty() && id.starts_with(reference);

        if id_matches
            || fragment.metadata.name == reference
            || collection::matches_explicit_path_reference(
                &fragment.store_relative_path,
                FRAGMENTS_DIR,
                "md",
                reference,
            )
        {
            matches.insert(fragment.store_relative_path.clone(), fragment);
        }
    }

    if !matches.is_empty() {
        return resolve::single_match(
            matches,
            || unreachable!("matches is not empty"),
            |matches| AppError::FragmentReferenceAmbiguous {
                reference: reference.to_owned(),
                matches,
            },
        );
    }

    let mut folded_matches: BTreeMap<String, &'a FragmentRecord> = BTreeMap::new();
    for fragment in fragments {
        if collection::folded_path_alias(&fragment.store_relative_path, FRAGMENTS_DIR, "md")
            .as_deref()
            == Some(reference)
        {
            folded_matches.insert(fragment.store_relative_path.clone(), fragment);
        }
    }

    resolve::single_match(
        folded_matches,
        || AppError::FragmentNotFound {
            reference: reference.to_owned(),
        },
        |matches| AppError::FragmentReferenceAmbiguous {
            reference: reference.to_owned(),
            matches,
        },
    )
}
