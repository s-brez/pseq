use std::path::Path;

use super::model::{SourceEvent, SourceEventKind, SourceSession};

pub(super) fn selectable_prompt_texts(session: &SourceSession) -> Vec<String> {
    selectable_event_texts(&session.events)
}

pub(super) fn selectable_event_texts(events: &[SourceEvent]) -> Vec<String> {
    events
        .iter()
        .filter(|event| event.kind == SourceEventKind::UserPrompt)
        .filter(|event| !is_current_pseq_capture_invocation(&event.text))
        .map(|event| event.text.clone())
        .collect()
}

fn is_current_pseq_capture_invocation(text: &str) -> bool {
    let mut command = text.trim();
    while let Some(stripped) = command.strip_prefix(['$', '>', '#']) {
        command = stripped.trim_start();
    }
    if command.is_empty() {
        return false;
    }

    let tokens = command
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '\'']))
        .collect::<Vec<_>>();
    let Some(first) = tokens.first() else {
        return false;
    };
    let mut command_index = 0;
    let first_executable = Path::new(first)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(first);
    if first_executable == "rtk" {
        command_index = 1;
    }

    let Some(command) = tokens.get(command_index) else {
        return false;
    };
    let executable = Path::new(command)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(command)
        .trim_end_matches(".exe");

    if executable != "pseq" {
        return false;
    }

    let mut index = command_index + 1;
    while let Some(token) = tokens.get(index) {
        match *token {
            "capture" | "cap" => return true,
            "--json" | "--quiet" | "--no-pager" => index += 1,
            "--store" | "-C" => index += 2,
            value if value.starts_with("--store=") => index += 1,
            value if value.starts_with("-C") && value.len() > 2 => index += 1,
            _ => return false,
        }
    }

    false
}
