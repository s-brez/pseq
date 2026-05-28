use std::fs;
use std::path::Path;

use crate::error::AppError;

use super::super::events::selectable_event_texts;
use super::super::model::{SourceEvent, SourceSession};
use super::super::payloads::{json_lines, structured_shell_command_text};
use super::super::source_files::file_modified_unix_nanos;

pub(in crate::capture) fn read_openhands_session(
    path: &Path,
) -> Result<Option<SourceSession>, AppError> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let values = if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value == "jsonl")
    {
        json_lines(&content).collect::<Vec<_>>()
    } else {
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(value) => openhands_events(&value).into_iter().cloned().collect(),
            Err(_) => Vec::new(),
        }
    };

    let mut timestamp = None;
    let mut events = Vec::new();
    for value in values {
        timestamp = timestamp.or_else(|| {
            value
                .get("timestamp")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });

        if let Some(event) = openhands_source_event(&value) {
            events.push(event);
        }
    }

    if selectable_event_texts(&events).is_empty() {
        return Ok(None);
    }

    Ok(Some(SourceSession {
        source: "openhands".to_owned(),
        path: path.to_path_buf(),
        id: path
            .file_stem()
            .map(|value| value.to_string_lossy().into_owned()),
        timestamp,
        modified_unix_nanos: file_modified_unix_nanos(path),
        events,
    }))
}

fn openhands_events(value: &serde_json::Value) -> Vec<&serde_json::Value> {
    if let Some(entries) = value.as_array() {
        return entries.iter().collect();
    }

    for key in ["entries", "history", "events", "trajectory"] {
        if let Some(entries) = value.get(key).and_then(serde_json::Value::as_array) {
            return entries.iter().collect();
        }
    }

    value.as_object().map(|_| vec![value]).unwrap_or_default()
}

fn openhands_source_event(value: &serde_json::Value) -> Option<SourceEvent> {
    if let Some(command) = structured_shell_command_text(value) {
        return Some(SourceEvent::shell_command(command));
    }

    let source = value.get("source").and_then(serde_json::Value::as_str);
    let actor_type = value.get("actorType").and_then(serde_json::Value::as_str);
    let event_type = value.get("type").and_then(serde_json::Value::as_str);
    let action = value.get("action").and_then(serde_json::Value::as_str);

    let is_user_message = (source == Some("user") && action == Some("message"))
        || (actor_type == Some("User") && event_type == Some("message"));
    let text = value
        .get("args")
        .and_then(|args| args.get("content"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("content").and_then(serde_json::Value::as_str))
        .or_else(|| value.get("message").and_then(serde_json::Value::as_str))?;

    if text == "No observation" && value.get("observation").is_some() {
        return None;
    }

    if is_user_message {
        Some(SourceEvent::user_prompt(text))
    } else if source == Some("agent") || actor_type == Some("Agent") {
        Some(SourceEvent::assistant_message(text))
    } else {
        None
    }
}
