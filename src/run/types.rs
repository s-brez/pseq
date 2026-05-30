use std::path::Path;

use serde::Serialize;

use crate::render::RenderedTurnFragment;

#[derive(Debug, Clone, Copy)]
pub enum FeedbackFrom {
    FinalStdout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScope {
    Run,
    Iteration,
}

#[derive(Debug)]
pub struct RunOptions<'a> {
    pub runner_name: Option<&'a str>,
    pub ad_hoc_command: &'a [String],
    pub variables_file: Option<&'a Path>,
    pub variable_assignments: &'a [String],
    pub capture_output: bool,
    pub max_captured_output: usize,
    pub iterations: usize,
    pub session_scope: SessionScope,
    pub feedback_from: Option<FeedbackFrom>,
    pub feedback_var: Option<&'a str>,
    pub feedback_seed: Option<&'a str>,
    pub quiet: bool,
}

#[derive(Debug, Serialize)]
pub struct RunOutput {
    pub sequence: RunSequenceSummary,
    pub runner: RunRunnerSummary,
    #[serde(skip_serializing_if = "is_one")]
    pub iterations: usize,
    pub turn_count: usize,
    pub completed_turns: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_iteration: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_turn: Option<usize>,
    pub turns: Vec<RunTurnOutput>,
}

#[derive(Debug, Serialize)]
pub struct RunSequenceSummary {
    pub id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct RunRunnerSummary {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunTurnOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iteration: Option<usize>,
    pub index: usize,
    pub fragment: RenderedTurnFragment,
    pub command: Vec<String>,
    pub pid: u32,
    pub termination: String,
    pub exit_code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core_dumped: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_truncated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_truncated: Option<bool>,
}

fn is_one(value: &usize) -> bool {
    *value == 1
}
