use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "pseq")]
#[command(version)]
#[command(about = "Build, render, and run local prompt sequences")]
#[command(long_about = None)]
pub struct Cli {
    #[arg(
        short = 'C',
        long,
        global = true,
        value_name = "PATH",
        display_order = 900,
        help = "Use prompt store at PATH"
    )]
    pub store: Option<PathBuf>,

    #[arg(long, global = true, display_order = 902, help = "Print JSON")]
    pub json: bool,

    #[arg(
        long,
        global = true,
        display_order = 903,
        help = "Suppress success messages"
    )]
    pub quiet: bool,

    #[arg(long, global = true, display_order = 904, help = "Disable pager")]
    pub no_pager: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a prompt store.
    Init,
    /// Manage prompt fragments.
    #[command(alias = "frag")]
    Fragment {
        #[command(subcommand)]
        command: FragmentCommand,
    },
    /// Manage prompt sequences.
    #[command(alias = "seq")]
    Sequence {
        #[command(subcommand)]
        command: SequenceCommand,
    },
    /// Render a sequence.
    Render {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "SEQUENCE")]
        sequence_reference: String,

        #[arg(
            long = "var",
            value_name = "KEY=VALUE",
            help = "Set variable; KEY=@FILE reads a file"
        )]
        variables: Vec<String>,

        #[arg(
            long = "vars",
            value_name = "PATH",
            help = "Read variables from JSON, TOML, or YAML"
        )]
        variables_file: Option<PathBuf>,

        #[arg(long, help = "Save rendered output")]
        save: bool,

        #[arg(
            long,
            value_name = "PATH",
            requires = "save",
            conflicts_with = "save_path",
            help = "Save under directory"
        )]
        dir: Option<PathBuf>,

        #[arg(
            long = "path",
            value_name = "PATH",
            requires = "save",
            conflicts_with = "dir",
            help = "Save at store path"
        )]
        save_path: Option<PathBuf>,

        #[arg(long, value_name = "PATH", help = "Write output to PATH")]
        out: Option<PathBuf>,

        #[arg(long, help = "Show fragment boundaries")]
        annotate: bool,

        #[arg(long, value_name = "REF", help = "Render from Git revision")]
        at: Option<String>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Run sequence turns.
    Run {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "SEQUENCE")]
        sequence_reference: String,

        /// Named runner to use.
        #[arg(value_name = "RUNNER")]
        runner_name: Option<String>,

        #[arg(
            long = "var",
            value_name = "KEY=VALUE",
            help = "Set variable; KEY=@FILE reads a file"
        )]
        variables: Vec<String>,

        #[arg(
            long = "vars",
            value_name = "PATH",
            help = "Read variables from JSON, TOML, or YAML"
        )]
        variables_file: Option<PathBuf>,

        #[arg(long, value_name = "BYTES", help = "Limit captured output bytes")]
        max_captured_output: Option<usize>,

        #[arg(long, value_name = "N", help = "Run N iterations")]
        iterations: Option<usize>,

        #[arg(
            long,
            value_name = "N",
            conflicts_with = "no_retry",
            help = "Retry each failed runner turn N times"
        )]
        retries: Option<usize>,

        #[arg(
            long,
            conflicts_with = "retries",
            help = "Do not retry failed runner turns"
        )]
        no_retry: bool,

        #[arg(
            long,
            value_name = "MS",
            help = "Delay MS milliseconds between runner retries"
        )]
        retry_delay_ms: Option<u64>,

        #[arg(
            long,
            help = "Inherit runner stdout/stderr without preserving bounded copies"
        )]
        no_preserve_output: bool,

        #[arg(
            long,
            value_enum,
            value_name = "SCOPE",
            help = "Set runner session scope (default: run)"
        )]
        session_scope: Option<SessionScopeArg>,

        #[arg(
            long,
            value_enum,
            value_name = "SOURCE",
            help = "Feed output into the next iteration"
        )]
        feedback_from: Option<FeedbackFromArg>,

        #[arg(long, value_name = "NAME", help = "Feedback variable name")]
        feedback_var: Option<String>,

        #[arg(
            long,
            value_name = "VALUE",
            help = "Initial feedback; @FILE or @- reads input"
        )]
        feedback_seed: Option<String>,

        /// Command to run after `--`.
        #[arg(
            value_name = "COMMAND",
            allow_hyphen_values = true,
            last = true,
            num_args = 1..
        )]
        command: Vec<String>,
    },
    /// Capture prompt text.
    #[command(alias = "cap")]
    Capture {
        #[command(subcommand)]
        command: CaptureCommand,
    },
    /// Manage runner commands.
    Runner {
        #[command(subcommand)]
        command: RunnerCommand,
    },
    /// Show store status.
    Status,
    /// Show uncommitted changes.
    Diff,
    /// Show store history.
    Log,
    /// Check store health.
    Doctor,
    /// Show configuration.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum FragmentCommand {
    /// Create a fragment.
    New {
        /// Fragment name.
        #[arg(value_name = "NAME")]
        name: String,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "stdin",
            help = "Read text from a file"
        )]
        from_file: Option<PathBuf>,

        #[arg(long, help = "Read text from stdin")]
        stdin: bool,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "path",
            help = "Create under directory"
        )]
        dir: Option<PathBuf>,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "dir",
            help = "Create at store path"
        )]
        path: Option<PathBuf>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// List fragments.
    List {
        #[arg(long, value_name = "PATH", help = "List only this path prefix")]
        prefix: Option<PathBuf>,

        #[arg(long, help = "Print as a tree")]
        tree: bool,
    },
    /// Show a fragment.
    Show {
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
    },
    /// Edit a fragment.
    Edit {
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Rename a fragment.
    Rename {
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
        /// New fragment name.
        #[arg(value_name = "NAME")]
        name: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Move a fragment.
    Mv {
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
        /// Destination path under fragments/.
        #[arg(value_name = "PATH")]
        path: PathBuf,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Remove a fragment.
    Rm {
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SequenceCommand {
    /// Create a sequence.
    New {
        /// Sequence name.
        #[arg(value_name = "NAME")]
        name: String,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "path",
            help = "Create under directory"
        )]
        dir: Option<PathBuf>,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "dir",
            help = "Create at store path"
        )]
        path: Option<PathBuf>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// List sequences.
    List {
        #[arg(long, value_name = "PATH", help = "List only this path prefix")]
        prefix: Option<PathBuf>,

        #[arg(long, help = "Print as a tree")]
        tree: bool,
    },
    /// Show a sequence.
    Show {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
    },
    /// Edit a sequence.
    Edit {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Add a fragment.
    Add {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "SEQUENCE")]
        sequence_reference: String,
        /// Fragment name, path, id, or unique id prefix.
        #[arg(value_name = "FRAGMENT")]
        fragment_reference: String,

        #[arg(
            long = "at",
            value_name = "INDEX",
            help = "Insert at this 1-based position"
        )]
        at: Option<usize>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Remove a fragment.
    Remove {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "SEQUENCE")]
        sequence_reference: String,
        /// Fragment reference or 1-based position.
        #[arg(value_name = "FRAGMENT|INDEX")]
        fragment_reference_or_index: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Move a fragment.
    Move {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "SEQUENCE")]
        sequence_reference: String,
        /// Current 1-based position.
        #[arg(value_name = "FROM")]
        from_index: usize,
        /// New 1-based position.
        #[arg(value_name = "TO")]
        to_index: usize,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Rename a sequence.
    Rename {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
        /// New sequence name.
        #[arg(value_name = "NAME")]
        name: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Move a sequence.
    Mv {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
        /// Destination path under sequences/.
        #[arg(value_name = "PATH")]
        path: PathBuf,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Remove a sequence.
    Rm {
        /// Sequence name, path, id, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum CaptureCommand {
    /// List capture sources.
    Sources,
    /// Check a capture source.
    Probe {
        #[arg(long, value_name = "SOURCE", help = "Capture source")]
        source: String,
    },
    /// Capture recent prompts.
    Last {
        /// Number of prompts to capture.
        #[arg(value_name = "N")]
        count: Option<usize>,

        #[arg(long, value_name = "SOURCE", help = "Capture source")]
        source: Option<String>,

        #[arg(long, value_name = "SESSION", help = "Source session")]
        session: Option<String>,

        #[arg(
            long = "as-sequence",
            value_name = "NAME",
            help = "Also create sequence"
        )]
        as_sequence: Option<String>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Capture a prompt range.
    Range {
        /// Range selector, such as `-5..-1`.
        #[arg(value_name = "SELECTOR", allow_hyphen_values = true)]
        selector: String,

        #[arg(long, value_name = "SOURCE", help = "Capture source")]
        source: Option<String>,

        #[arg(long, value_name = "SESSION", help = "Source session")]
        session: Option<String>,

        #[arg(
            long = "as-sequence",
            value_name = "NAME",
            help = "Also create sequence"
        )]
        as_sequence: Option<String>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Import prompt text.
    Import {
        #[arg(
            long,
            conflicts_with = "file",
            required_unless_present = "file",
            help = "Read text from stdin"
        )]
        stdin: bool,

        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "stdin",
            required_unless_present = "stdin",
            help = "Read text from a file"
        )]
        file: Option<PathBuf>,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// List captures.
    List {
        #[arg(long, value_name = "PATH", help = "List only this path prefix")]
        prefix: Option<PathBuf>,
    },
    /// Show a capture.
    Show {
        /// Capture id, path, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
    },
    /// Move a capture.
    Mv {
        /// Capture id, path, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,
        /// Destination path under captures/.
        #[arg(value_name = "PATH")]
        path: PathBuf,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
    /// Promote a capture to a sequence.
    Promote {
        /// Capture id, path, or unique id prefix.
        #[arg(value_name = "REF")]
        reference: String,

        #[arg(
            long = "as-sequence",
            value_name = "NAME",
            help = "Sequence name to create"
        )]
        as_sequence: String,

        #[arg(long, help = "Do not commit store changes")]
        no_commit: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum RunnerCommand {
    /// Set a runner command.
    Set {
        /// Runner name.
        #[arg(value_name = "NAME")]
        name: String,
        /// Runner slot to set.
        #[arg(value_name = "SLOT")]
        slot: RunnerSlotArg,

        /// Command to run after `--`.
        #[arg(
            value_name = "COMMAND",
            required = true,
            allow_hyphen_values = true,
            last = true,
            num_args = 1..
        )]
        command: Vec<String>,
    },
    /// Set the default runner.
    Default {
        /// Runner name.
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// List runners.
    List,
    /// Show a runner.
    Show {
        /// Runner name.
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Trust a runner command on this machine.
    Trust {
        /// Runner name.
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Remove a runner.
    Rm {
        /// Runner name.
        #[arg(value_name = "NAME")]
        name: String,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RunnerSlotArg {
    First,
    Next,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum FeedbackFromArg {
    FinalStdout,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SessionScopeArg {
    Run,
    Iteration,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show resolved configuration.
    Show,
}
