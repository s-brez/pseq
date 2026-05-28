use serde::{Deserialize, Serialize};

use crate::fragment::FragmentSummary;
use crate::sequence::SequenceSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum CaptureOrigin {
    Stdin,
    File {
        path: String,
    },
    Source {
        source: String,
        session: CaptureSourceSession,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureSourceSession {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaptureSourcesOutput {
    pub sources: Vec<CaptureSourceSummary>,
}

#[derive(Debug, Serialize)]
pub struct CaptureSourceSummary {
    pub name: String,
    pub available: bool,
    pub description: String,
    pub session_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<CaptureSourceSessionSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureSourceSessionSummary {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    pub prompt_count: usize,
}

#[derive(Debug, Serialize)]
pub struct CaptureProbeOutput {
    pub source: String,
    pub available: bool,
    pub message: String,
    pub session_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<CaptureSourceSessionSummary>,
}

#[derive(Debug, Serialize)]
pub struct CaptureImportOutput {
    pub id: String,
    pub path: String,
    pub origin: CaptureOrigin,
    pub event_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CaptureSelectionOutput {
    Capture(CaptureImportOutput),
    Promoted(CapturePromoteOutput),
}

#[derive(Debug, Serialize)]
pub struct CaptureListOutput {
    pub captures: Vec<CaptureSummary>,
    #[serde(skip)]
    pub tree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureSummary {
    pub id: String,
    pub path: String,
    pub origin: CaptureOrigin,
    pub event_count: usize,
}

#[derive(Debug, Serialize)]
pub struct CaptureShowOutput {
    pub id: String,
    pub path: String,
    pub origin: CaptureOrigin,
    pub events: Vec<CaptureEventOutput>,
}

#[derive(Debug, Serialize)]
pub struct CaptureMoveOutput {
    pub id: String,
    pub path: String,
    pub origin: CaptureOrigin,
    pub event_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CapturePromoteOutput {
    pub capture: CaptureSummary,
    pub sequence: SequenceSummary,
    pub fragments: Vec<FragmentSummary>,
    pub event_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CaptureEventOutput {
    pub index: usize,
    pub kind: String,
    pub text: String,
}
