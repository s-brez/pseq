mod create;
mod edit;
mod model;
mod mutations;
mod queries;
mod records;
mod types;
mod validation;

const SEQUENCES_DIR: &str = "sequences";
const ID_PREFIX: &str = "seq_";
const FRAGMENT_ID_PREFIX: &str = "frg_";

pub use create::create;
pub(crate) use create::create_uncommitted;
pub use edit::edit;
pub use mutations::{add, move_file, move_fragment, remove, remove_fragment, rename};
pub use queries::{list, show};
pub(crate) use queries::{render_source, sequences_referencing_fragment};
pub use types::{
    SequenceAddOutput, SequenceEditOutput, SequenceFragmentRemoveOutput, SequenceListOutput,
    SequenceMoveOutput, SequenceNewOutput, SequencePathMoveOutput, SequenceRemoveOutput,
    SequenceRenameOutput, SequenceShowOutput, SequenceSummary,
};
pub(crate) use validation::validate_name;
pub use validation::validate_sequences;
