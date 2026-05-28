use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub(crate) struct RenderSequence {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) fragments: Vec<RenderFragment>,
    pub(super) catalog: Vec<RenderFragment>,
}

#[derive(Debug, Clone)]
pub(super) struct RenderFragment {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) body: String,
}

#[derive(Debug, Clone)]
pub(super) struct FragmentIncludeFrame {
    pub(super) id: String,
    pub(super) label: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedRenderMetadata {
    pub(super) id: String,
    pub(super) sequence_id: String,
    pub(super) sequence_name: String,
    pub(super) sequence_path: String,
    pub(super) annotated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) history_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct HistoricalSequenceFile {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) fragments: Vec<String>,
}

#[derive(Debug)]
pub(super) struct HistoricalSequenceRecord {
    pub(super) data: HistoricalSequenceFile,
    pub(super) path: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct RenderFragmentFrontmatter {
    pub(super) id: String,
    pub(super) name: String,
}

#[derive(Debug)]
pub(super) enum RenderFragmentParseError {
    Frontmatter(String),
    Metadata(String),
}
