use std::collections::BTreeMap;

use serde::Serialize;

use crate::fragment::FragmentSummary;

#[derive(Debug, Serialize)]
pub struct SequenceNewOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceListOutput {
    pub sequences: Vec<SequenceSummary>,
    #[serde(skip)]
    pub tree: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SequenceSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SequenceShowOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragments: Vec<FragmentSummary>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct SequenceEditOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceAddOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment: FragmentSummary,
    pub index: usize,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceFragmentRemoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub removed_fragment: FragmentSummary,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceMoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub from_index: usize,
    pub to_index: usize,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceRenameOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequencePathMoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SequenceRemoveOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SequenceRenderSource {
    pub id: String,
    pub name: String,
    pub path: String,
    pub fragment_references: Vec<String>,
}
