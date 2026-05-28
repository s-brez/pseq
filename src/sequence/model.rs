use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SequenceFile {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) fragments: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) variables: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(super) struct SequenceRecord {
    pub(super) data: SequenceFile,
    pub(super) path: PathBuf,
    pub(super) store_relative_path: String,
}
