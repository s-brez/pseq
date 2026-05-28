use std::fs;
use std::path::{Path, PathBuf};

use crate::commit;
use crate::error::AppError;
use crate::paths;
use crate::store;

use super::engine::{render_text, render_turns_with_variables};
use super::history::load_historical_sequence;
use super::load::load_current_sequence;
use super::save::{path_is_inside_store, save_destination, save_render};
use super::types::*;
use super::variables::load_variables;

pub fn render(
    store_path: &Path,
    sequence_reference: &str,
    options: RenderOptions<'_>,
) -> Result<RenderOutput, AppError> {
    let out_writes_store = options
        .out_path
        .is_some_and(|path| path_is_inside_store(store_path, path));
    let mutates_store = options.save || out_writes_store;
    if options.history_ref.is_some() && !mutates_store {
        store::require_valid_store_structure(store_path)?;
    } else {
        store::require_valid_store(store_path)?;
    }

    let sequence = if let Some(history_ref) = options.history_ref {
        load_historical_sequence(store_path, history_ref, sequence_reference)?
    } else {
        load_current_sequence(store_path, sequence_reference)?
    };

    let variables = load_variables(options.variables_file, options.variable_assignments)?;
    let text = render_text(
        &sequence.fragments,
        &sequence.catalog,
        &variables,
        options.annotate,
    )?;
    let save_destination = if options.save {
        Some(save_destination(
            store_path,
            &sequence,
            options.save_dir,
            options.save_path,
        )?)
    } else {
        None
    };
    if let (Some(out_path), Some(save_destination)) = (options.out_path, save_destination.as_ref())
        && crate::collection::same_destination(out_path, &save_destination.path)
    {
        return Err(AppError::InvalidCollectionPath {
            kind: "render destination".to_owned(),
            path: paths::display(out_path),
            message: "render --out and --save destination must not be the same path".to_owned(),
        });
    }
    let mut store_mutation_paths = Vec::new();

    let (out_path, out_is_store_path) = if let Some(path) = options.out_path {
        fs::write(path, &text).map_err(|source| AppError::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
        if out_writes_store {
            store_mutation_paths.push(path.to_path_buf());
        }
        (Some(paths::display(path)), out_writes_store)
    } else {
        (None, false)
    };

    let mut saved_render = if let Some(save_destination) = save_destination {
        let saved_render = save_render(
            &sequence,
            &text,
            options.annotate,
            options.history_ref,
            save_destination,
        )?;
        store_mutation_paths.push(PathBuf::from(&saved_render.path));
        Some(saved_render)
    } else {
        None
    };

    let git_commit = if store_mutation_paths.is_empty() {
        None
    } else {
        let message = if saved_render.is_some() {
            format!("Save render {}", sequence.name)
        } else {
            format!("Write render {}", sequence.name)
        };
        commit::maybe_commit_paths(
            options.commit_mode,
            store_path,
            &store_mutation_paths,
            &message,
        )?
    };

    if let Some(saved_render) = &mut saved_render {
        saved_render.git_commit = git_commit.clone();
    }
    let out_git_commit = if out_is_store_path { git_commit } else { None };

    Ok(RenderOutput {
        id: sequence.id,
        name: sequence.name,
        path: sequence.path,
        text,
        annotated: options.annotate,
        history_ref: options.history_ref.map(str::to_owned),
        out_path,
        out_git_commit,
        saved_render,
    })
}

pub fn render_turns(
    store_path: &Path,
    sequence_reference: &str,
    options: RenderTurnsOptions<'_>,
) -> Result<RenderedSequenceTurns, AppError> {
    store::require_valid_store(store_path)?;
    let variables = load_variables(options.variables_file, options.variable_assignments)?;
    render_turns_with_variables(store_path, sequence_reference, &variables)
}
