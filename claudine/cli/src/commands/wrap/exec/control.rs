//! Retained-stdin control sessions for the semantic spawn path.
//!
//! Most providers take their task as a one-shot stdin seed. A provider whose
//! managed launch is a bidirectional protocol (Pi's `--mode rpc`) instead keeps
//! stdin open for the whole run: the task, steering commands, and answers to
//! the provider's own requests all travel over it. A [`StdioControl`] owns
//! that stdin, sees every stdout line before the semantic parser does, submits
//! the task once the child proves it is ready, and ends a one-shot run by
//! closing stdin when the provider has settled.
//!
//! [`run_child_stream_semantic`](super::spawn::run_child_stream_semantic)
//! builds the parser only after readiness. If the child cannot become ready
//! before the task is submitted, the spawn path kills it and, when the session
//! offers a [`FallbackLaunch`], runs that launch with the same parser builder
//! and the task as an ordinary stdin seed. Nothing was submitted, so nothing
//! is replayed; once submission has been attempted there is no fallback.

use std::process::ChildStdin;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use claudine::steering::controller::{SteeringController, SteeringExecutor};
use claudine::stream::logs::EarlyTermination;

use super::termination::CompletionTermination;

/// Whether the child has proved it can take the task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Readiness {
    Pending,
    Ready,
    /// Not ready, and the task was never submitted.
    Failed(String),
}

/// Channels a session uses to end the run it controls.
pub(crate) struct ControlChannels {
    /// Fails the run: the child is terminated and the reason reported.
    pub(crate) early: Sender<EarlyTermination>,
    /// Completes the run: the child is terminated and the run reported as
    /// completed, for a provider that does not exit once stdin is closed.
    pub(crate) completion: Sender<CompletionTermination>,
}

/// The alternative a session offers when its child is not ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FallbackLaunch {
    /// Complete provider argv for the alternative interface.
    pub(crate) args: Vec<String>,
    /// Shown on STDERR before the alternative starts: why the preferred
    /// interface is unavailable, what runs instead, and what is lost.
    pub(crate) warning: String,
}

/// One provider child's retained-stdin control session.
///
/// The spawn path calls [`open`](Self::open) once after spawning,
/// [`observe`](Self::observe) for every stdout line (from the reader thread),
/// [`submit`](Self::submit) at most once after [`Readiness::Ready`], and
/// [`finish`](Self::finish) once the child has exited.
pub(crate) trait StdioControl: Send + Sync {
    /// Takes ownership of the child's stdin and starts the readiness check.
    fn open(&self, stdin: ChildStdin, child_pid: u32, channels: ControlChannels) -> std::io::Result<()>;

    /// Reacts to one stdout line.
    fn observe(&self, line: &str);

    /// Whether `line` answers a command the session itself sent (readiness,
    /// task, steering, settlement). Such a line is control traffic, not
    /// agent progress, so it does not refresh the stream-silence clock.
    fn is_control_reply(&self, _line: &str) -> bool {
        false
    }

    /// The current readiness; it leaves [`Readiness::Pending`] exactly once.
    fn readiness(&self) -> Readiness;

    /// Gives up on readiness. Returns `false` when the child became ready
    /// first, in which case the run proceeds as ready.
    fn abandon_readiness(&self, reason: &str) -> bool;

    /// How long the spawn path waits for readiness.
    fn readiness_deadline(&self) -> Duration;

    /// Submits the task. Called only after [`Readiness::Ready`].
    fn submit(&self, task: &str) -> std::io::Result<()>;

    /// The child has exited: release stdin and fail anything still waiting
    /// on it.
    fn finish(&self);

    /// The alternative launch for a child that was not ready, if one
    /// preserves the execution's settings.
    fn fallback(&self) -> Option<FallbackLaunch>;

    /// The researched steering launch profile this session runs, when its
    /// protocol can carry steering.
    fn steering_profile(&self) -> Option<&'static str> {
        None
    }

    /// The adapter that delivers steering over this session.
    fn steering_executor(&self) -> Option<Arc<dyn SteeringExecutor>> {
        None
    }

    /// Gives the session the execution's steering controller, so provider
    /// state, conversation, and process identity reach it. Called before
    /// [`open`](Self::open).
    fn bind_controller(&self, _controller: SteeringController) {}
}
