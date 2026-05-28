use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

use crate::error::AppError;
use crate::paths;

use super::super::events::selectable_prompt_texts;
use super::super::model::{SourceDefinition, SourceEvent, SourceSession};
use super::super::types::CaptureSourceSessionSummary;
use super::discovery::source_sessions;
use super::resolve_source;

pub(in crate::capture) fn resolve_source_session(
    source: &str,
    session_reference: Option<&str>,
) -> Result<SourceSession, AppError> {
    let source = resolve_source(source)?;
    match source.name {
        "stdin" => stdin_source_session(),
        _ => {
            let sessions = source_sessions(source)?;
            if let Some(reference) = session_reference {
                resolve_harness_session(source, sessions, reference)
            } else {
                latest_harness_session(source, sessions)
            }
        }
    }
}

fn stdin_source_session() -> Result<SourceSession, AppError> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Err(AppError::CaptureSourceUnavailable {
            name: "stdin".to_owned(),
            reason: "stdin source requires piped input; use `pseq capture import --stdin` or pipe text to `pseq capture last --source stdin`".to_owned(),
        });
    }

    let mut input = String::new();
    stdin
        .lock()
        .read_to_string(&mut input)
        .map_err(|source| AppError::ReadStdin { source })?;
    if input.is_empty() {
        return Err(AppError::CaptureSourceUnavailable {
            name: "stdin".to_owned(),
            reason: "stdin source requires piped input; received empty stdin".to_owned(),
        });
    }

    Ok(SourceSession {
        source: "stdin".to_owned(),
        path: PathBuf::from("<stdin>"),
        id: None,
        timestamp: None,
        modified_unix_nanos: None,
        events: vec![SourceEvent::user_prompt(input)],
    })
}

fn latest_harness_session(
    source: &SourceDefinition,
    sessions: Vec<SourceSession>,
) -> Result<SourceSession, AppError> {
    let mut sessions = sorted_source_sessions(sessions);
    if sessions.is_empty() {
        return Err(AppError::CaptureSourceUnavailable {
            name: source.name.to_owned(),
            reason: format!("no {} sessions with user prompts found", source.description),
        });
    }

    if sessions.get(1).is_some_and(|session| {
        source_session_order_key(session) == source_session_order_key(&sessions[0])
    }) {
        let sessions = sessions
            .iter()
            .take_while(|session| {
                source_session_order_key(session) == source_session_order_key(&sessions[0])
            })
            .map(|session| paths::display(&session.path))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::CaptureSessionAmbiguous {
            source_name: source.name.to_owned(),
            sessions,
        });
    }

    Ok(sessions.remove(0))
}

fn resolve_harness_session(
    source: &SourceDefinition,
    sessions: Vec<SourceSession>,
    reference: &str,
) -> Result<SourceSession, AppError> {
    let mut matches = sessions
        .into_iter()
        .filter(|session| source_session_matches(session, reference))
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(AppError::CaptureSessionNotFound {
            source_name: source.name.to_owned(),
            reference: reference.to_owned(),
        }),
        1 => Ok(matches.remove(0)),
        _ => Err(AppError::CaptureSessionReferenceAmbiguous {
            source_name: source.name.to_owned(),
            reference: reference.to_owned(),
            sessions: matches
                .iter()
                .map(|session| paths::display(&session.path))
                .collect::<Vec<_>>()
                .join(", "),
        }),
    }
}

pub(super) fn sorted_source_sessions(mut sessions: Vec<SourceSession>) -> Vec<SourceSession> {
    sessions.sort_by(|left, right| {
        source_session_order_key(right)
            .cmp(&source_session_order_key(left))
            .then_with(|| right.path.cmp(&left.path))
    });
    sessions
}

fn source_session_matches(session: &SourceSession, reference: &str) -> bool {
    if session.id.as_deref() == Some(reference) {
        return true;
    }

    let normalized_reference = paths::normalize_reference(reference);
    let normalized_path = paths::normalize(&session.path);
    if normalized_path == normalized_reference {
        return true;
    }

    session
        .path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|file_name| file_name == reference)
}

pub(super) fn session_summary(session: &SourceSession) -> CaptureSourceSessionSummary {
    CaptureSourceSessionSummary {
        path: paths::display(&session.path),
        id: session.id.clone(),
        timestamp: session.timestamp.clone(),
        prompt_count: selectable_prompt_texts(session).len(),
    }
}

fn source_session_order_key(session: &SourceSession) -> (u128, String) {
    (
        session.modified_unix_nanos.unwrap_or_default(),
        session.timestamp.clone().unwrap_or_default(),
    )
}
