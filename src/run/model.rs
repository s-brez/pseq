#[derive(Debug)]
pub(super) struct ProcessTurnOutput {
    pub(super) exit_code: i32,
    pub(super) success: bool,
    pub(super) stdout: Option<String>,
    pub(super) stderr: Option<String>,
    pub(super) stdout_bytes: Option<usize>,
    pub(super) stderr_bytes: Option<usize>,
    pub(super) stdout_truncated: Option<bool>,
    pub(super) stderr_truncated: Option<bool>,
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
