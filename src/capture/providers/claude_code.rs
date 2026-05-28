use std::fs;
use std::path::Path;

use crate::error::AppError;

use super::super::events::selectable_event_texts;
use super::super::model::{SourceEvent, SourceSession};
use super::super::payloads::{
    json_lines, structured_shell_command_text, text_from_message_content,
};
use super::super::source_files::file_modified_unix_nanos;

pub(in crate::capture) fn read_claude_code_session(
    path: &Path,
) -> Result<Option<SourceSession>, AppError> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let mut timestamp = None;
    let mut events = Vec::new();

    for value in json_lines(&content) {
        timestamp = timestamp.or_else(|| {
            value
                .get("timestamp")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });

        if let Some(event) = claude_code_source_event(&value) {
            events.push(event);
        }
    }

    if selectable_event_texts(&events).is_empty() {
        return Ok(None);
    }

    Ok(Some(SourceSession {
        source: "claude-code".to_owned(),
        path: path.to_path_buf(),
        id: path
            .file_stem()
            .map(|value| value.to_string_lossy().into_owned()),
        timestamp,
        modified_unix_nanos: file_modified_unix_nanos(path),
        events,
    }))
}

fn claude_code_source_event(value: &serde_json::Value) -> Option<SourceEvent> {
    if let Some(command) = structured_shell_command_text(value) {
        return Some(SourceEvent::shell_command(command));
    }

    let message = value.get("message")?;
    if let Some(command) = structured_shell_command_text(message) {
        return Some(SourceEvent::shell_command(command));
    }

    let message_role = message.get("role").and_then(serde_json::Value::as_str);
    let top_level_type = value.get("type").and_then(serde_json::Value::as_str);
    let text = text_from_message_content(message.get("content")?)?;

    match (message_role, top_level_type) {
        (Some("user"), _) | (_, Some("user")) => Some(SourceEvent::user_prompt(text)),
        (Some("assistant"), _) | (_, Some("assistant")) => {
            Some(SourceEvent::assistant_message(text))
        }
        (Some("model"), _) | (_, Some("model")) => Some(SourceEvent::model_message(text)),
        _ => None,
    }
}
