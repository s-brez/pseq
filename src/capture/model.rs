use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::types::{CaptureOrigin, CaptureSourceSessionSummary};

#[derive(Debug)]
pub(super) struct SourceDefinition {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CaptureFile {
    pub(super) version: u32,
    pub(super) id: String,
    pub(super) created_unix_seconds: u64,
    pub(super) origin: CaptureOrigin,
    pub(super) events: Vec<CaptureEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CaptureEvent {
    pub(super) index: usize,
    pub(super) kind: CaptureEventKind,
    pub(super) text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CaptureEventKind {
    UserPrompt,
}

#[derive(Debug)]
pub(super) struct CaptureRecord {
    pub(super) data: CaptureFile,
    pub(super) path: PathBuf,
    pub(super) store_relative_path: String,
}

#[derive(Debug)]
pub(super) struct SourceAvailability {
    pub(super) available: bool,
    pub(super) unavailable_reason: Option<String>,
    pub(super) sessions: Vec<CaptureSourceSessionSummary>,
}

#[derive(Debug)]
pub(super) struct SourceSession {
    pub(super) source: String,
    pub(super) path: PathBuf,
    pub(super) id: Option<String>,
    pub(super) timestamp: Option<String>,
    pub(super) modified_unix_nanos: Option<u128>,
    pub(super) events: Vec<SourceEvent>,
}

#[derive(Debug, Clone)]
pub(super) struct SourceEvent {
    pub(super) kind: SourceEventKind,
    pub(super) text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceEventKind {
    UserPrompt,
    AssistantMessage,
    ModelMessage,
    ShellCommand,
}

#[derive(Debug)]
pub(super) struct OpenCodeEvent {
    pub(super) event: SourceEvent,
    pub(super) path: PathBuf,
    pub(super) order: u128,
    pub(super) timestamp: Option<String>,
}

#[derive(Debug)]
pub(super) struct OpenCodeSessionBuilder {
    pub(super) id: String,
    pub(super) events: Vec<OpenCodeEvent>,
    pub(super) modified_unix_nanos: Option<u128>,
}

impl SourceEvent {
    pub(super) fn user_prompt(text: impl Into<String>) -> Self {
        Self {
            kind: SourceEventKind::UserPrompt,
            text: text.into(),
        }
    }

    pub(super) fn assistant_message(text: impl Into<String>) -> Self {
        Self {
            kind: SourceEventKind::AssistantMessage,
            text: text.into(),
        }
    }

    pub(super) fn model_message(text: impl Into<String>) -> Self {
        Self {
            kind: SourceEventKind::ModelMessage,
            text: text.into(),
        }
    }

    pub(super) fn shell_command(text: impl Into<String>) -> Self {
        Self {
            kind: SourceEventKind::ShellCommand,
            text: text.into(),
        }
    }
}
