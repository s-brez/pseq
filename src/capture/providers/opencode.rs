use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AppError;

use super::super::events::selectable_event_texts;
use super::super::model::{OpenCodeEvent, OpenCodeSessionBuilder, SourceEvent, SourceSession};
use super::super::payloads::{structured_shell_command_text, text_from_message_content};
use super::super::source_files::file_modified_unix_nanos;

pub(in crate::capture) fn read_opencode_event_file(
    path: &Path,
) -> Result<Option<(String, OpenCodeEvent)>, AppError> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Ok(None);
    };

    let Some((session_id, event, order, timestamp)) = opencode_source_event(&value, path) else {
        return Ok(None);
    };

    Ok(Some((
        session_id,
        OpenCodeEvent {
            event,
            path: path.to_path_buf(),
            order,
            timestamp,
        },
    )))
}

pub(in crate::capture) fn opencode_session_from_builder(
    mut builder: OpenCodeSessionBuilder,
) -> Option<SourceSession> {
    builder.events.sort_by(|left, right| {
        left.order
            .cmp(&right.order)
            .then_with(|| left.path.cmp(&right.path))
    });
    let path = builder
        .events
        .last()
        .map(|event| event.path.clone())
        .unwrap_or_else(|| PathBuf::from(&builder.id));
    let timestamp = builder
        .events
        .iter()
        .rev()
        .find_map(|event| event.timestamp.clone());
    let events = builder
        .events
        .into_iter()
        .map(|event| event.event)
        .collect::<Vec<_>>();
    if selectable_event_texts(&events).is_empty() {
        return None;
    }

    Some(SourceSession {
        source: "opencode".to_owned(),
        path,
        id: Some(builder.id),
        timestamp,
        modified_unix_nanos: builder.modified_unix_nanos,
        events,
    })
}

fn opencode_source_event(
    value: &serde_json::Value,
    path: &Path,
) -> Option<(String, SourceEvent, u128, Option<String>)> {
    let data = value
        .get("data")
        .filter(|data| data.is_object())
        .unwrap_or(value);
    let session_id = data
        .get("sessionID")
        .or_else(|| data.get("session_id"))
        .or_else(|| value.get("sessionID"))
        .or_else(|| value.get("session_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            path.parent()
                .and_then(Path::file_name)
                .map(|value| value.to_string_lossy().into_owned())
        })?;

    let event = if let Some(command) =
        structured_shell_command_text(data).or_else(|| structured_shell_command_text(value))
    {
        SourceEvent::shell_command(command)
    } else {
        let text = data
            .get("parts")
            .or_else(|| value.get("parts"))
            .and_then(opencode_parts_text)
            .or_else(|| {
                data.get("content")
                    .or_else(|| data.get("message"))
                    .or_else(|| data.get("text"))
                    .and_then(text_from_message_content)
            })?;
        match data.get("role").and_then(serde_json::Value::as_str) {
            Some("user") => SourceEvent::user_prompt(text),
            Some("assistant") => SourceEvent::assistant_message(text),
            Some("model") => SourceEvent::model_message(text),
            _ => return None,
        }
    };
    let order = opencode_message_order(data, value, path);
    let timestamp = opencode_message_timestamp(data).or_else(|| opencode_message_timestamp(value));

    Some((session_id, event, order, timestamp))
}

fn opencode_parts_text(value: &serde_json::Value) -> Option<String> {
    let parts = value.as_array()?;
    let texts = parts
        .iter()
        .filter(|part| {
            part.get("type").and_then(serde_json::Value::as_str) == Some("text")
                && part.get("ignored").and_then(serde_json::Value::as_bool) != Some(true)
                && part.get("synthetic").and_then(serde_json::Value::as_bool) != Some(true)
        })
        .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    (!texts.is_empty()).then(|| texts.join("\n\n"))
}

fn opencode_message_order(
    data: &serde_json::Value,
    value: &serde_json::Value,
    path: &Path,
) -> u128 {
    numeric_time_field(data)
        .or_else(|| numeric_time_field(value))
        .unwrap_or_else(|| file_modified_unix_nanos(path).unwrap_or_default())
}

fn opencode_message_timestamp(value: &serde_json::Value) -> Option<String> {
    value
        .get("time")
        .and_then(|time| {
            time.get("created")
                .or_else(|| time.get("updated"))
                .and_then(json_scalar_string)
        })
        .or_else(|| {
            value
                .get("createdAt")
                .or_else(|| value.get("updatedAt"))
                .or_else(|| value.get("time_created"))
                .or_else(|| value.get("time_updated"))
                .and_then(json_scalar_string)
        })
}

fn numeric_time_field(value: &serde_json::Value) -> Option<u128> {
    value
        .get("time")
        .and_then(|time| {
            time.get("created")
                .or_else(|| time.get("updated"))
                .and_then(json_number_as_u128)
        })
        .or_else(|| {
            value
                .get("createdAt")
                .or_else(|| value.get("updatedAt"))
                .or_else(|| value.get("time_created"))
                .or_else(|| value.get("time_updated"))
                .and_then(json_number_as_u128)
        })
}

fn json_number_as_u128(value: &serde_json::Value) -> Option<u128> {
    value
        .as_u64()
        .map(u128::from)
        .or_else(|| value.as_i64().and_then(|value| u128::try_from(value).ok()))
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0)
                .map(|value| value as u128)
        })
        .or_else(|| value.as_str().and_then(|value| value.parse::<u128>().ok()))
}

fn json_scalar_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_i64().map(|value| value.to_string()))
}
