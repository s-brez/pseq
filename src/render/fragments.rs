use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::codec;
use crate::collection;
use crate::error::AppError;
use crate::git;
use crate::paths;
use crate::resolve;
use crate::store;
use crate::yaml;

use super::FRAGMENTS_DIR;
use super::model::{RenderFragment, RenderFragmentFrontmatter, RenderFragmentParseError};

pub(super) fn resolve_sequence_fragments(
    catalog: &[RenderFragment],
    references: &[String],
) -> Result<Vec<RenderFragment>, AppError> {
    references
        .iter()
        .map(|reference| resolve_render_fragment(catalog, reference).cloned())
        .collect()
}

pub(super) fn read_current_fragments(store_path: &Path) -> Result<Vec<RenderFragment>, AppError> {
    let mut fragments = Vec::new();
    for path in collection::files_with_extension(store_path, FRAGMENTS_DIR, "md") {
        let content = fs::read_to_string(&path).map_err(|source| AppError::ReadFile {
            path: path.clone(),
            source,
        })?;
        let fragment = parse_render_fragment(&content, paths::store_relative(store_path, &path))
            .map_err(|error| match error {
                RenderFragmentParseError::Frontmatter(_) => AppError::InvalidStore {
                    path: store_path.to_path_buf(),
                    issues: 1,
                },
                RenderFragmentParseError::Metadata(_) => AppError::InvalidStore {
                    path: store_path.to_path_buf(),
                    issues: store::validate_store(store_path).issues.len().max(1),
                },
            })?;
        fragments.push(fragment);
    }
    fragments.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(fragments)
}

pub(super) fn read_historical_fragments(
    store_path: &Path,
    history_ref: &str,
) -> Result<Vec<RenderFragment>, AppError> {
    let mut fragments = Vec::new();
    for path in git::list_files_at_ref(store_path, history_ref, &[FRAGMENTS_DIR])? {
        if !path.ends_with(".md") {
            continue;
        }
        let content = git::show_text_at_ref(store_path, history_ref, &path)?;
        let fragment = parse_render_fragment(&content, path.clone()).map_err(|error| {
            AppError::HistoricalFileInvalid {
                reference: history_ref.to_owned(),
                path: path.clone(),
                message: match error {
                    RenderFragmentParseError::Frontmatter(message)
                    | RenderFragmentParseError::Metadata(message) => message,
                },
            }
        })?;
        fragments.push(fragment);
    }
    fragments.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(fragments)
}

pub(super) fn resolve_render_fragment<'a>(
    fragments: &'a [RenderFragment],
    reference: &str,
) -> Result<&'a RenderFragment, AppError> {
    let mut matches: BTreeMap<String, &'a RenderFragment> = BTreeMap::new();

    for fragment in fragments {
        let id_matches = !reference.is_empty() && fragment.id.starts_with(reference);
        if id_matches
            || fragment.name == reference
            || collection::matches_explicit_path_reference(
                &fragment.path,
                FRAGMENTS_DIR,
                "md",
                reference,
            )
        {
            matches.insert(fragment.path.clone(), fragment);
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

    let mut folded_matches: BTreeMap<String, &'a RenderFragment> = BTreeMap::new();
    for fragment in fragments {
        if collection::folded_path_alias(&fragment.path, FRAGMENTS_DIR, "md").as_deref()
            == Some(reference)
        {
            folded_matches.insert(fragment.path.clone(), fragment);
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

fn parse_render_fragment(
    content: &str,
    path: String,
) -> Result<RenderFragment, RenderFragmentParseError> {
    let (frontmatter, body) =
        codec::split_yaml_frontmatter(content).map_err(RenderFragmentParseError::Frontmatter)?;
    let metadata: RenderFragmentFrontmatter = yaml::from_str(frontmatter)
        .map_err(|error| RenderFragmentParseError::Metadata(error.to_string()))?;
    Ok(RenderFragment {
        id: metadata.id,
        name: metadata.name,
        path,
        body: body.to_owned(),
    })
}
