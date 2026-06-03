use serde::Serialize;
use std::path::Path;

use crate::commit::CommitMode;
use crate::turn_settings::TurnRuntimeSettings;

#[derive(Debug)]
pub struct RenderOptions<'a> {
    pub variables_file: Option<&'a Path>,
    pub variable_assignments: &'a [String],
    pub save: bool,
    pub save_dir: Option<&'a Path>,
    pub save_path: Option<&'a Path>,
    pub out_path: Option<&'a Path>,
    pub annotate: bool,
    pub history_ref: Option<&'a str>,
    pub commit_mode: CommitMode,
}

#[derive(Debug)]
pub struct RenderTurnsOptions<'a> {
    pub variables_file: Option<&'a Path>,
    pub variable_assignments: &'a [String],
}

#[derive(Debug, Serialize)]
pub struct RenderOutput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub text: String,
    pub annotated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_git_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saved_render: Option<SavedRenderSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedSequenceTurns {
    pub id: String,
    pub name: String,
    pub path: String,
    pub turns: Vec<RenderedTurn>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedTurn {
    pub index: usize,
    pub fragment: RenderedTurnFragment,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedTurnFragment {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedSequenceRuntimeTurns {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) turns: Vec<RenderedRuntimeTurn>,
}

#[derive(Debug, Clone)]
pub(crate) struct RenderedRuntimeTurn {
    pub(crate) index: usize,
    pub(crate) fragment: RenderedTurnFragment,
    pub(crate) settings: TurnRuntimeSettings,
    pub(crate) text: String,
}

#[derive(Debug, Serialize)]
pub struct SavedRenderSummary {
    pub id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}
