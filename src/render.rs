mod commands;
mod engine;
mod fragments;
mod history;
mod load;
mod model;
mod save;
mod types;
mod validation;
mod variables;

const RENDERS_DIR: &str = "renders";
const FRAGMENTS_DIR: &str = "fragments";
const SEQUENCES_DIR: &str = "sequences";
const ID_PREFIX: &str = "rnd_";
const SEQUENCE_ID_PREFIX: &str = "seq_";
const INCLUDE_PREFIX: &str = "pseq.fragment.";

pub use commands::{render, render_turns};
pub(crate) use engine::render_sequence_turns;
pub(crate) use load::load_current_sequence;
pub use types::{
    RenderOptions, RenderOutput, RenderTurnsOptions, RenderedSequenceTurns, RenderedTurn,
    RenderedTurnFragment, SavedRenderSummary,
};
pub use validation::validate_saved_renders;
pub(crate) use variables::{load_variables, validate_variable_name};
