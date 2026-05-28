mod discovery;
mod sessions;

use crate::error::AppError;

use super::model::{SourceAvailability, SourceDefinition};

pub(super) use sessions::resolve_source_session;

pub(super) const SOURCES: &[SourceDefinition] = &[
    SourceDefinition {
        name: "stdin",
        description: "standard input",
    },
    SourceDefinition {
        name: "codex",
        description: "Codex transcript source",
    },
    SourceDefinition {
        name: "claude-code",
        description: "Claude Code transcript source",
    },
    SourceDefinition {
        name: "openhands",
        description: "OpenHands transcript source",
    },
    SourceDefinition {
        name: "opencode",
        description: "OpenCode transcript source",
    },
];

pub(super) fn is_supported_source(source: &str) -> bool {
    SOURCES.iter().any(|candidate| candidate.name == source)
}

pub(super) fn resolve_source(source: &str) -> Result<&'static SourceDefinition, AppError> {
    SOURCES
        .iter()
        .find(|candidate| candidate.name == source)
        .ok_or_else(|| AppError::CaptureSourceUnsupported {
            name: source.to_owned(),
        })
}

pub(super) fn source_availability(source: &SourceDefinition) -> SourceAvailability {
    match source.name {
        "stdin" => SourceAvailability {
            available: true,
            unavailable_reason: None,
            sessions: Vec::new(),
        },
        _ => match discovery::source_sessions(source) {
            Ok(sessions) if !sessions.is_empty() => {
                let sessions = sessions::sorted_source_sessions(sessions)
                    .iter()
                    .map(sessions::session_summary)
                    .collect();
                SourceAvailability {
                    available: true,
                    unavailable_reason: None,
                    sessions,
                }
            }
            Ok(_) => SourceAvailability {
                available: false,
                unavailable_reason: Some(format!(
                    "no {} sessions with user prompts found",
                    source.description
                )),
                sessions: Vec::new(),
            },
            Err(error) => SourceAvailability {
                available: false,
                unavailable_reason: Some(error.to_string()),
                sessions: Vec::new(),
            },
        },
    }
}
