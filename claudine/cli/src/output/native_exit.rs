//! The evidence a provider's process exit leaves for the user-facing failure
//! report: exit code, termination, and bounded tails of both output streams.
//!
//! Every launch path builds one [`NativeExit`] from what it already observes;
//! a path that inherits a stream (interactive or passthrough launches) leaves
//! that tail absent, and an absent tail can never classify a failure.

use claudine::harness::ProcessTermination;
use claudine::signals::{EXIT_STDERR_TAIL_LINES, EXIT_STDOUT_TAIL_LINES, tail_lines};

/// The last lines of one output stream.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct StreamTail {
    pub(crate) text: String,
    /// The stream already reached the user's terminal verbatim (streamed
    /// live or echoed), so a report must not repeat its lines.
    pub(crate) shown: bool,
}

/// A provider process exit, as the failure report classifies it.
///
/// `Debug` prints only tail lengths: the tails are provider output and may
/// echo a forwarded secret.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct NativeExit {
    pub(crate) exit_code: i32,
    pub(crate) termination: ProcessTermination,
    pub(crate) stdout: Option<StreamTail>,
    pub(crate) stderr: Option<StreamTail>,
    /// A failure headline already shown for this exit (the composition
    /// path's per-attempt failure status). A diagnostic line it contains is
    /// not repeated.
    pub(crate) shown_headline: Option<String>,
}

impl NativeExit {
    /// An exit with no captured stream evidence.
    pub(crate) fn new(exit_code: i32, termination: ProcessTermination) -> Self {
        Self {
            exit_code,
            termination,
            stdout: None,
            stderr: None,
            shown_headline: None,
        }
    }

    /// Record a failure headline the user has already seen.
    pub(crate) fn with_shown_headline(mut self, headline: &str) -> Self {
        self.shown_headline = Some(headline.to_string());
        self
    }

    /// Attach stdout, keeping its last [`EXIT_STDOUT_TAIL_LINES`] lines.
    pub(crate) fn with_stdout(mut self, text: &str, shown: bool) -> Self {
        self.stdout = bounded(text, EXIT_STDOUT_TAIL_LINES, shown);
        self
    }

    /// Attach stderr, keeping its last [`EXIT_STDERR_TAIL_LINES`] lines.
    pub(crate) fn with_stderr(mut self, text: &str, shown: bool) -> Self {
        self.stderr = bounded(text, EXIT_STDERR_TAIL_LINES, shown);
        self
    }

    /// Whether the provider ended in failure: a non-zero exit or any
    /// termination other than normal completion.
    pub(crate) fn failed(&self) -> bool {
        self.exit_code != 0 || self.termination != ProcessTermination::Completed
    }

    /// The captured tails, stderr first.
    pub(crate) fn tails(&self) -> impl Iterator<Item = &StreamTail> {
        [self.stderr.as_ref(), self.stdout.as_ref()].into_iter().flatten()
    }
}

fn bounded(text: &str, keep: usize, shown: bool) -> Option<StreamTail> {
    (!text.trim().is_empty()).then(|| StreamTail {
        text: tail_lines(text, keep),
        shown,
    })
}

impl std::fmt::Debug for StreamTail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamTail")
            .field("bytes", &self.text.len())
            .field("shown", &self.shown)
            .finish()
    }
}

impl std::fmt::Debug for NativeExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeExit")
            .field("exit_code", &self.exit_code)
            .field("termination", &self.termination)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .field("shown_headline", &self.shown_headline.as_ref().map(String::len))
            .finish()
    }
}
