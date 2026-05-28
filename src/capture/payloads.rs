pub(super) fn json_lines(content: &str) -> impl Iterator<Item = serde_json::Value> + '_ {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
}

pub(super) fn structured_shell_command_text(value: &serde_json::Value) -> Option<String> {
    if let Some(command) = value
        .get("command")
        .or_else(|| value.get("cmd"))
        .and_then(serde_json::Value::as_str)
    {
        return Some(command.to_owned());
    }

    for key in ["args", "input", "tool_input"] {
        if let Some(command) = value.get(key).and_then(|nested| {
            nested
                .get("command")
                .or_else(|| nested.get("cmd"))
                .and_then(serde_json::Value::as_str)
        }) {
            return Some(command.to_owned());
        }
    }

    if let Some(command) = value
        .get("state")
        .and_then(|state| state.get("input"))
        .and_then(|input| {
            input
                .get("command")
                .or_else(|| input.get("cmd"))
                .and_then(serde_json::Value::as_str)
        })
    {
        return Some(command.to_owned());
    }

    for key in ["parts", "content"] {
        if let Some(command) = value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .and_then(|items| items.iter().find_map(structured_shell_command_text))
        {
            return Some(command);
        }
    }

    let event_type = value
        .get("type")
        .or_else(|| value.get("action"))
        .or_else(|| value.get("tool"))
        .or_else(|| value.get("name"))
        .and_then(serde_json::Value::as_str);
    let message = value
        .get("message")
        .or_else(|| value.get("content"))
        .and_then(serde_json::Value::as_str);

    match (event_type, message) {
        (Some(event_type), Some(message)) if shellish_value(event_type) => Some(message.to_owned()),
        _ => None,
    }
}

pub(super) fn shellish_value(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    ["bash", "shell", "command", "exec", "run"]
        .iter()
        .any(|needle| value.contains(needle))
}

pub(super) fn text_from_message_content(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_owned());
    }

    let texts = value
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(serde_json::Value::as_str) == Some("text"))
        .filter_map(|part| part.get("text").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();

    (!texts.is_empty()).then(|| texts.join("\n\n"))
}
