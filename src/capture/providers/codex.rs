use std::fs;
use std::path::Path;

use crate::error::AppError;

use super::super::events::selectable_event_texts;
use super::super::model::{SourceEvent, SourceEventKind, SourceSession};
use super::super::payloads::{json_lines, shellish_value, structured_shell_command_text};
use super::super::source_files::file_modified_unix_nanos;

pub(in crate::capture) fn read_codex_session(
    path: &Path,
) -> Result<Option<SourceSession>, AppError> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut session_id = None;
    let mut session_timestamp = None;
    let mut event_events = Vec::new();
    let mut response_item_events = Vec::new();

    for value in json_lines(&content) {
        if value.get("type").and_then(serde_json::Value::as_str) == Some("session_meta")
            && let Some(payload) = value.get("payload")
        {
            session_id = session_id.or_else(|| {
                payload
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            });
            session_timestamp = session_timestamp.or_else(|| {
                payload
                    .get("timestamp")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            });
        }

        if let Some(event) = codex_event_source_event(&value) {
            event_events.push(event);
        } else if let Some(event) = codex_response_item_source_event(&value) {
            response_item_events.push(event);
        }
    }

    let events = if event_events
        .iter()
        .any(|event| event.kind == SourceEventKind::UserPrompt)
    {
        event_events
    } else {
        response_item_events
    };

    if selectable_event_texts(&events).is_empty() {
        return Ok(None);
    }

    Ok(Some(SourceSession {
        source: "codex".to_owned(),
        path: path.to_path_buf(),
        id: session_id,
        timestamp: session_timestamp,
        modified_unix_nanos: file_modified_unix_nanos(path),
        events,
    }))
}

fn codex_event_source_event(value: &serde_json::Value) -> Option<SourceEvent> {
    if value.get("type").and_then(serde_json::Value::as_str) != Some("event_msg") {
        return None;
    }

    let payload = value.get("payload")?;
    if let Some(command) = structured_shell_command_text(payload) {
        return Some(SourceEvent::shell_command(command));
    }

    match payload.get("type").and_then(serde_json::Value::as_str) {
        Some("user_message") => payload
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(SourceEvent::user_prompt),
        Some("agent_message" | "assistant_message") => payload
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(SourceEvent::assistant_message),
        Some("model_message") => payload
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(SourceEvent::model_message),
        Some(kind) if shellish_value(kind) => payload
            .get("message")
            .and_then(serde_json::Value::as_str)
            .map(SourceEvent::shell_command),
        _ => None,
    }
}

fn codex_response_item_source_event(value: &serde_json::Value) -> Option<SourceEvent> {
    if value.get("type").and_then(serde_json::Value::as_str) != Some("response_item") {
        return None;
    }

    let payload = value.get("payload")?;
    if let Some(command) = structured_shell_command_text(payload) {
        return Some(SourceEvent::shell_command(command));
    }

    if payload.get("type").and_then(serde_json::Value::as_str) != Some("message") {
        return None;
    }

    let text = response_item_message_text(payload.get("content")?)?;
    match payload.get("role").and_then(serde_json::Value::as_str) {
        Some("user") => Some(SourceEvent::user_prompt(text)),
        Some("assistant") => Some(SourceEvent::assistant_message(text)),
        Some("model") => Some(SourceEvent::model_message(text)),
        _ => None,
    }
}

fn response_item_message_text(value: &serde_json::Value) -> Option<String> {
    let parts = value
        .as_array()?
        .iter()
        .filter_map(|item| {
            matches!(
                item.get("type").and_then(serde_json::Value::as_str),
                Some("input_text" | "output_text" | "text")
            )
            .then(|| item.get("text").and_then(serde_json::Value::as_str))
            .flatten()
        })
        .collect::<Vec<_>>();

    (!parts.is_empty()).then(|| parts.join("\n\n"))
}
