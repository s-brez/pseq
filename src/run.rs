mod adapters;
mod command;
mod diagnostics;
mod model;
mod options;
mod process;
mod session;
mod types;

pub const DEFAULT_MAX_CAPTURED_OUTPUT: usize = 1024 * 1024;
pub const DEFAULT_FEEDBACK_VARIABLE: &str = "pseq_feedback";

pub use command::run_sequence;
pub use types::{
    FeedbackFrom, RunOptions, RunOutput, RunRunnerSummary, RunSequenceSummary, RunTurnOutput,
    SessionScope,
};
