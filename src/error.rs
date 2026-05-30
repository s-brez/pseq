use std::io::{self, Write};
use std::path::PathBuf;
use std::string::FromUtf8Error;
use std::time::SystemTimeError;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to resolve the current directory: {source}")]
    CurrentDir { source: io::Error },

    #[error("failed to resolve the default store path: HOME or USERPROFILE is not set")]
    DefaultStoreUnavailable,

    #[error("store path exists and is not a directory: {path}")]
    StorePathNotDirectory { path: PathBuf },

    #[error(
        "refusing to initialize a non-empty directory that is not already a valid pseq store: {path}"
    )]
    InitTargetNotEmpty { path: PathBuf },

    #[error("cannot initialize pseq store at {path}: {message}")]
    InitTargetConflict { path: PathBuf, message: String },

    #[error("failed to create directory {path}: {source}")]
    CreateDir { path: PathBuf, source: io::Error },

    #[error("failed to write file {path}: {source}")]
    WriteFile { path: PathBuf, source: io::Error },

    #[error("failed to remove file {path}: {source}")]
    RemoveFile { path: PathBuf, source: io::Error },

    #[error("failed to move file {from} to {to}: {source}")]
    MoveFile {
        from: PathBuf,
        to: PathBuf,
        source: io::Error,
    },

    #[error("failed to read file {path}: {source}")]
    ReadFile { path: PathBuf, source: io::Error },

    #[error("failed to read stdin: {source}")]
    ReadStdin { source: io::Error },

    #[error("no editor configured; set VISUAL or EDITOR")]
    EditorUnavailable,

    #[error("failed to run editor {editor:?}: {source}")]
    EditorSpawn { editor: String, source: io::Error },

    #[error("editor {editor:?} exited unsuccessfully: {status}")]
    EditorFailed { editor: String, status: String },

    #[error("edited fragment is invalid: {message}")]
    InvalidEditedFragment { message: String },

    #[error("edited sequence is invalid: {message}")]
    InvalidEditedSequence { message: String },

    #[error("failed to resolve system time: {source}")]
    SystemTime { source: SystemTimeError },

    #[error("invalid variable assignment {assignment:?}; expected key=value")]
    InvalidVariableAssignment { assignment: String },

    #[error(
        "invalid variable name {name:?}; expected ASCII letters, digits, or underscores, not starting with a digit"
    )]
    InvalidVariableName { name: String },

    #[error("missing required variable: {name}")]
    MissingVariable { name: String },

    #[error("invalid variable placeholder {placeholder:?}; expected {{{{variable_name}}}}")]
    InvalidVariablePlaceholder { placeholder: String },

    #[error("invalid variables file {path}: {message}")]
    InvalidVariablesFile { path: PathBuf, message: String },

    #[error("invalid config file {path}: {message}")]
    InvalidConfig { path: PathBuf, message: String },

    #[error("invalid user config file {path}: {message}")]
    InvalidUserConfig { path: PathBuf, message: String },

    #[error("invalid {kind} {path:?}: {message}")]
    InvalidCollectionPath {
        kind: String,
        path: String,
        message: String,
    },

    #[error(
        "invalid runner name {name:?}; expected ASCII letters, digits, underscores, or hyphens, not starting with a hyphen"
    )]
    InvalidRunnerName { name: String },

    #[error("runner command is empty: {context}")]
    RunnerCommandEmpty { context: String },

    #[error("runner not found: {name}")]
    RunnerNotFound { name: String },

    #[error("no default runner configured; use `pseq runner default <name>` or pass a runner name")]
    DefaultRunnerMissing,

    #[error(
        "runner {name:?} from store {store} is not trusted on this machine; run `pseq runner trust {name}` to trust the current command"
    )]
    RunnerNotTrusted { name: String, store: String },

    #[error("failed to resolve trusted runner file: HOME or USERPROFILE is not set")]
    RunnerTrustUnavailable,

    #[error("trusted runner file is locked: {path}")]
    RunnerTrustLocked { path: PathBuf },

    #[error("invalid run invocation: {message}")]
    InvalidRunInvocation { message: String },

    #[error("failed to spawn runner command {command:?}: {source}")]
    RunnerSpawn {
        command: Vec<String>,
        source: io::Error,
    },

    #[error("failed to open stdin for runner command {command:?}")]
    RunnerStdinUnavailable { command: Vec<String> },

    #[error("failed to write prompt to runner command {command:?}: {source}")]
    RunnerWriteStdin {
        command: Vec<String>,
        source: io::Error,
    },

    #[error("failed to open {stream} for runner command {command:?}")]
    RunnerOutputUnavailable {
        command: Vec<String>,
        stream: &'static str,
    },

    #[error("failed to read {stream} from runner command {command:?}: {source}")]
    RunnerReadOutput {
        command: Vec<String>,
        stream: &'static str,
        source: io::Error,
    },

    #[error("failed to wait for runner command {command:?}: {source}")]
    RunnerWait {
        command: Vec<String>,
        source: io::Error,
    },

    #[error("failed to install runner signal forwarding: {message}")]
    RunnerSignalForwarding { message: String },

    #[error("store is invalid: {path} ({issues} issue(s)); run `pseq doctor --store {path}`")]
    InvalidStore { path: PathBuf, issues: usize },

    #[error("invalid fragment name {name:?}; expected at least one non-whitespace character")]
    InvalidFragmentName { name: String },

    #[error("fragment not found: {reference}")]
    FragmentNotFound { reference: String },

    #[error("fragment reference is ambiguous: {reference}; matches: {matches}")]
    FragmentReferenceAmbiguous { reference: String, matches: String },

    #[error(
        "invalid fragment include placeholder {placeholder:?}; expected {{{{pseq.fragment.<fragment-ref>}}}}"
    )]
    InvalidFragmentInclude { placeholder: String },

    #[error("fragment inclusion cycle detected at {reference}; chain: {chain}")]
    FragmentIncludeCycle { reference: String, chain: String },

    #[error(
        "fragment is used by sequence(s) and cannot be removed: {reference}; sequences: {sequences}"
    )]
    FragmentInUse {
        reference: String,
        sequences: String,
    },

    #[error("sequence not found: {reference}")]
    SequenceNotFound { reference: String },

    #[error("invalid sequence name {name:?}; expected at least one non-whitespace character")]
    InvalidSequenceName { name: String },

    #[error("sequence reference is ambiguous: {reference}; matches: {matches}")]
    SequenceReferenceAmbiguous { reference: String, matches: String },

    #[error("sequence fragment reference not found: {reference}")]
    SequenceFragmentNotFound { reference: String },

    #[error("sequence fragment reference is ambiguous: {reference}; positions: {positions}")]
    SequenceFragmentReferenceAmbiguous {
        reference: String,
        positions: String,
    },

    #[error("invalid sequence index {index}; expected an index between 1 and {len}")]
    InvalidSequenceIndex { index: usize, len: usize },

    #[error("capture source is not supported: {name}")]
    CaptureSourceUnsupported { name: String },

    #[error("capture source is unavailable: {name}; {reason}")]
    CaptureSourceUnavailable { name: String, reason: String },

    #[error(
        "capture source session selection is ambiguous for {source_name}; candidates: {sessions}"
    )]
    CaptureSessionAmbiguous {
        source_name: String,
        sessions: String,
    },

    #[error("capture source session not found for {source_name}: {reference}")]
    CaptureSessionNotFound {
        source_name: String,
        reference: String,
    },

    #[error(
        "capture source session reference is ambiguous for {source_name}: {reference}; candidates: {sessions}"
    )]
    CaptureSessionReferenceAmbiguous {
        source_name: String,
        reference: String,
        sessions: String,
    },

    #[error("invalid capture count {count}; expected a positive integer")]
    InvalidCaptureCount { count: usize },

    #[error("invalid capture range {selector:?}; expected A..B with nonzero integer endpoints")]
    InvalidCaptureRange { selector: String },

    #[error("capture selection {selector:?} is outside available prompt range 1..{available}")]
    CaptureSelectionOutOfRange { selector: String, available: usize },

    #[error("capture not found: {reference}")]
    CaptureNotFound { reference: String },

    #[error("capture reference is ambiguous: {reference}; matches: {matches}")]
    CaptureReferenceAmbiguous { reference: String, matches: String },

    #[error("failed to serialize fragment frontmatter: {source}")]
    SerializeYaml { source: crate::yaml::Error },

    #[error("failed to run git: {source}")]
    GitSpawn { source: io::Error },

    #[error("git command failed: {command}: {stderr}")]
    GitFailed { command: String, stderr: String },

    #[error("git file at {reference}:{path} is not valid UTF-8: {source}")]
    GitFileNotUtf8 {
        reference: String,
        path: String,
        source: FromUtf8Error,
    },

    #[error("invalid file at {reference}:{path}: {message}")]
    HistoricalFileInvalid {
        reference: String,
        path: String,
        message: String,
    },

    #[error("pseq initialized an invalid store at {path}")]
    InitProducedInvalidStore { path: PathBuf },

    #[error("failed to serialize JSON: {source}")]
    SerializeJson { source: serde_json::Error },

    #[error("failed to serialize TOML: {source}")]
    SerializeToml { source: toml::ser::Error },

    #[error("failed to write output: {source}")]
    WriteOutput { source: io::Error },
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::CurrentDir { .. } => "current_dir_failed",
            Self::DefaultStoreUnavailable => "default_store_unavailable",
            Self::StorePathNotDirectory { .. } => "store_path_not_directory",
            Self::InitTargetNotEmpty { .. } => "init_target_not_empty",
            Self::InitTargetConflict { .. } => "init_target_conflict",
            Self::CreateDir { .. } => "create_dir_failed",
            Self::WriteFile { .. } => "write_file_failed",
            Self::RemoveFile { .. } => "remove_file_failed",
            Self::MoveFile { .. } => "move_file_failed",
            Self::ReadFile { .. } => "read_file_failed",
            Self::ReadStdin { .. } => "read_stdin_failed",
            Self::EditorUnavailable => "editor_unavailable",
            Self::EditorSpawn { .. } => "editor_spawn_failed",
            Self::EditorFailed { .. } => "editor_failed",
            Self::InvalidEditedFragment { .. } => "invalid_edited_fragment",
            Self::InvalidEditedSequence { .. } => "invalid_edited_sequence",
            Self::SystemTime { .. } => "system_time_failed",
            Self::InvalidVariableAssignment { .. } => "invalid_variable_assignment",
            Self::InvalidVariableName { .. } => "invalid_variable_name",
            Self::MissingVariable { .. } => "missing_variable",
            Self::InvalidVariablePlaceholder { .. } => "invalid_variable_placeholder",
            Self::InvalidVariablesFile { .. } => "invalid_variables_file",
            Self::InvalidConfig { .. } => "invalid_config",
            Self::InvalidUserConfig { .. } => "invalid_user_config",
            Self::InvalidCollectionPath { .. } => "invalid_collection_path",
            Self::InvalidRunnerName { .. } => "invalid_runner_name",
            Self::RunnerCommandEmpty { .. } => "runner_command_empty",
            Self::RunnerNotFound { .. } => "runner_not_found",
            Self::DefaultRunnerMissing => "default_runner_missing",
            Self::RunnerNotTrusted { .. } => "runner_not_trusted",
            Self::RunnerTrustUnavailable => "runner_trust_unavailable",
            Self::RunnerTrustLocked { .. } => "runner_trust_locked",
            Self::InvalidRunInvocation { .. } => "invalid_run_invocation",
            Self::RunnerSpawn { .. } => "runner_spawn_failed",
            Self::RunnerStdinUnavailable { .. } => "runner_stdin_unavailable",
            Self::RunnerWriteStdin { .. } => "runner_write_stdin_failed",
            Self::RunnerOutputUnavailable { .. } => "runner_output_unavailable",
            Self::RunnerReadOutput { .. } => "runner_read_output_failed",
            Self::RunnerWait { .. } => "runner_wait_failed",
            Self::RunnerSignalForwarding { .. } => "runner_signal_forwarding_failed",
            Self::InvalidStore { .. } => "invalid_store",
            Self::InvalidFragmentName { .. } => "invalid_fragment_name",
            Self::FragmentNotFound { .. } => "fragment_not_found",
            Self::FragmentReferenceAmbiguous { .. } => "fragment_reference_ambiguous",
            Self::InvalidFragmentInclude { .. } => "invalid_fragment_include",
            Self::FragmentIncludeCycle { .. } => "fragment_include_cycle",
            Self::FragmentInUse { .. } => "fragment_in_use",
            Self::InvalidSequenceName { .. } => "invalid_sequence_name",
            Self::SequenceNotFound { .. } => "sequence_not_found",
            Self::SequenceReferenceAmbiguous { .. } => "sequence_reference_ambiguous",
            Self::SequenceFragmentNotFound { .. } => "sequence_fragment_not_found",
            Self::SequenceFragmentReferenceAmbiguous { .. } => {
                "sequence_fragment_reference_ambiguous"
            }
            Self::InvalidSequenceIndex { .. } => "invalid_sequence_index",
            Self::CaptureSourceUnsupported { .. } => "capture_source_unsupported",
            Self::CaptureSourceUnavailable { .. } => "capture_source_unavailable",
            Self::CaptureSessionAmbiguous { .. } => "capture_session_ambiguous",
            Self::CaptureSessionNotFound { .. } => "capture_session_not_found",
            Self::CaptureSessionReferenceAmbiguous { .. } => "capture_session_reference_ambiguous",
            Self::InvalidCaptureCount { .. } => "invalid_capture_count",
            Self::InvalidCaptureRange { .. } => "invalid_capture_range",
            Self::CaptureSelectionOutOfRange { .. } => "capture_selection_out_of_range",
            Self::CaptureNotFound { .. } => "capture_not_found",
            Self::CaptureReferenceAmbiguous { .. } => "capture_reference_ambiguous",
            Self::SerializeYaml { .. } => "serialize_yaml_failed",
            Self::GitSpawn { .. } => "git_unavailable",
            Self::GitFailed { .. } => "git_failed",
            Self::GitFileNotUtf8 { .. } => "git_file_not_utf8",
            Self::HistoricalFileInvalid { .. } => "historical_file_invalid",
            Self::InitProducedInvalidStore { .. } => "init_produced_invalid_store",
            Self::SerializeJson { .. } => "serialize_json_failed",
            Self::SerializeToml { .. } => "serialize_toml_failed",
            Self::WriteOutput { .. } => "write_output_failed",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::GitSpawn { .. } => 3,
            _ => 1,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope<'a> {
    error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
struct ErrorBody<'a> {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a serde_json::Value>,
}

pub fn write_error(error: &AppError, json: bool) -> io::Result<()> {
    if json {
        return write_error_envelope(error.code(), error.to_string(), None);
    }

    let mut stderr = io::stderr().lock();
    writeln!(stderr, "error: {error}")
}

pub fn write_cli_error(error: &clap::Error) -> io::Result<()> {
    let details = serde_json::json!({
        "kind": format!("{:?}", error.kind()),
    });
    write_error_envelope("cli_parse_failed", error.to_string(), Some(&details))
}

fn write_error_envelope(
    code: &'static str,
    message: String,
    details: Option<&serde_json::Value>,
) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    let envelope = ErrorEnvelope {
        error: ErrorBody {
            code,
            message,
            details,
        },
    };
    serde_json::to_writer_pretty(&mut stderr, &envelope)?;
    writeln!(stderr)
}
