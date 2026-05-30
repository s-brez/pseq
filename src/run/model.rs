#[derive(Debug)]
pub(super) struct ProcessTurnOutput {
    pub(super) pid: u32,
    pub(super) termination: ProcessTermination,
    pub(super) exit_code: i32,
    pub(super) success: bool,
    pub(super) signal: Option<i32>,
    pub(super) signal_name: Option<&'static str>,
    pub(super) core_dumped: Option<bool>,
    pub(super) stdout: Option<String>,
    pub(super) stderr: Option<String>,
    pub(super) stdout_bytes: Option<usize>,
    pub(super) stderr_bytes: Option<usize>,
    pub(super) stdout_truncated: Option<bool>,
    pub(super) stderr_truncated: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProcessTermination {
    Exit,
    Signal,
    Unknown,
}

impl ProcessTermination {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Exit => "exit",
            Self::Signal => "signal",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug)]
pub(super) struct CapturedStream {
    pub(super) text: String,
    pub(super) bytes: usize,
    pub(super) truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OutputMode {
    Inherit,
    Capture,
    Tee,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TeeStream {
    Stdout,
    Stderr,
}

impl OutputMode {
    pub(super) fn tee_stream(self, stream: TeeStream) -> Option<TeeStream> {
        (self == Self::Tee).then_some(stream)
    }
}
