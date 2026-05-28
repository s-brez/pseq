use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::fs_walk;

use super::super::model::{OpenCodeSessionBuilder, SourceDefinition, SourceSession};
use super::super::providers;
use super::super::source_files::{file_modified_unix_nanos, max_optional_u128, path_has_component};

pub(super) fn source_sessions(source: &SourceDefinition) -> Result<Vec<SourceSession>, AppError> {
    match source.name {
        "codex" => codex_source_sessions(),
        "claude-code" => claude_code_source_sessions(),
        "openhands" => openhands_source_sessions(),
        "opencode" => opencode_source_sessions(),
        _ => Ok(Vec::new()),
    }
}

fn source_sessions_from_files(
    root: PathBuf,
    extensions: &[&str],
    mut read_session: impl FnMut(&Path) -> Result<Option<SourceSession>, AppError>,
) -> Result<Vec<SourceSession>, AppError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();
    for path in fs_walk::regular_files_with_extensions(&root, extensions) {
        if let Some(session) = read_session(&path)? {
            sessions.push(session);
        }
    }

    Ok(sessions)
}

fn codex_source_sessions() -> Result<Vec<SourceSession>, AppError> {
    source_sessions_from_files(
        codex_home()?.join("sessions"),
        &["jsonl"],
        providers::read_codex_session,
    )
}

fn claude_code_source_sessions() -> Result<Vec<SourceSession>, AppError> {
    source_sessions_from_files(claude_code_home()?.join("projects"), &["jsonl"], |path| {
        if path_has_component(path, "subagents") {
            Ok(None)
        } else {
            providers::read_claude_code_session(path)
        }
    })
}

fn openhands_source_sessions() -> Result<Vec<SourceSession>, AppError> {
    source_sessions_from_files(
        openhands_home()?,
        &["json", "jsonl"],
        providers::read_openhands_session,
    )
}

fn opencode_source_sessions() -> Result<Vec<SourceSession>, AppError> {
    let root = opencode_data_dir()?;
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut sessions = BTreeMap::<String, OpenCodeSessionBuilder>::new();
    for path in fs_walk::regular_files_with_extensions(&root, &["json"]) {
        let Some((session_id, event)) = providers::read_opencode_event_file(&path)? else {
            continue;
        };

        let builder =
            sessions
                .entry(session_id.clone())
                .or_insert_with(|| OpenCodeSessionBuilder {
                    id: session_id,
                    events: Vec::new(),
                    modified_unix_nanos: None,
                });
        builder.modified_unix_nanos =
            max_optional_u128(builder.modified_unix_nanos, file_modified_unix_nanos(&path));
        builder.events.push(event);
    }

    Ok(sessions
        .into_values()
        .filter_map(providers::opencode_session_from_builder)
        .collect())
}

fn codex_home() -> Result<PathBuf, AppError> {
    configured_data_path(&["CODEX_HOME"], &[".codex"], "codex")
}

fn claude_code_home() -> Result<PathBuf, AppError> {
    configured_data_path(
        &["CLAUDE_CONFIG_DIR", "CLAUDE_HOME"],
        &[".claude"],
        "claude-code",
    )
}

fn openhands_home() -> Result<PathBuf, AppError> {
    configured_data_path(
        &["OPENHANDS_HOME", "OPENHANDS_DATA_DIR"],
        &[".openhands"],
        "openhands",
    )
}

fn opencode_data_dir() -> Result<PathBuf, AppError> {
    configured_data_path(
        &["OPENCODE_DATA_DIR"],
        &[".local", "share", "opencode"],
        "opencode",
    )
}

fn configured_data_path(
    env_names: &[&str],
    home_suffix: &[&str],
    source_name: &str,
) -> Result<PathBuf, AppError> {
    for name in env_names {
        if let Some(path) = env::var_os(name).filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| AppError::CaptureSourceUnavailable {
            name: source_name.to_owned(),
            reason: format!("HOME, USERPROFILE, and {} are not set", env_names.join("/")),
        })?;

    let mut path = PathBuf::from(home);
    for component in home_suffix {
        path.push(component);
    }
    Ok(path)
}
