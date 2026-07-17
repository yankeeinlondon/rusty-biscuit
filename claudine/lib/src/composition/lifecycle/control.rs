//! Pure runtime-dispatch decisions for lifecycle [`StackControl`] outcomes.
//!
//! The lifecycle stack engine ([`super::lifecycle_executor`]) resolves a
//! matched control action into a [`StackControl`]. The composition runtime
//! then has to translate that into concrete control flow: retry the loop,
//! resume the provider session, hand off to another prompt, requeue for
//! later, or stop.
//!
//! This module isolates the *decision* — given the terminal event, the
//! control, and the current attempt state, what should the runtime do? —
//! from the *effect* (re-entering the harness loop, mutating prompt state,
//! sleeping). Keeping the decision pure means the four control branches can
//! be unit-tested without spawning a provider.

use std::time::Duration;

use super::actions::RetryBackoff;
use super::executor::StackControl;

/// The concrete runtime action a terminal-event [`StackControl`] resolves to.
///
/// This is what the harness loop acts on after a `blocked`/`failure` (or a
/// downgraded terminal) event reports its control. Each variant maps to one
/// branch of the loop's control flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlDispatch {
    /// Re-enter the loop for another attempt. The runtime should sleep for
    /// `delay` (already adjusted for backoff), then `continue` the loop.
    ///
    /// `reenter_preflight` re-enters the pre-flight/start path (no provider
    /// invocation existed yet); otherwise the provider is invoked again.
    Retry {
        /// How long to wait before the next attempt (post-backoff).
        delay: Duration,
        /// True when the provider had **not** launched this iteration (re-run
        /// pre-flight/start); false when it had (re-invoke the provider).
        /// Derived from launch state, not the event — so a `retry` recovers
        /// uniformly from `blocked` (pre-launch) or `failure`/`finalize`
        /// (post-launch).
        reenter_preflight: bool,
    },

    /// Resume the provider session with a follow-up message. The runtime
    /// must have a session id; absence is a hard error (see
    /// [`ControlDispatch::ResumeWithoutSession`]).
    Resume {
        /// The follow-up prompt to deliver on resume.
        message: String,
    },

    /// The control requested resume but no session id is available — a hard
    /// error the runtime surfaces rather than silently dropping.
    ResumeWithoutSession,

    /// Hand off to another prompt document, entering at its own
    /// `initialize` (a fresh prompt run including pre-flight).
    Proxy {
        /// The target prompt reference (e.g. `@prompts/foo.md`).
        target: String,
    },

    /// Push this prompt onto the deferred-execution (`rendezvous`) queue,
    /// then exit this run.
    Defer {
        /// Evaluated delay duration string.
        delay: String,
        /// Optional human-readable reason.
        reason: Option<String>,
    },

    /// The retry/resume attempt ceiling has been reached; no further
    /// attempts. The runtime falls through to its normal terminal handling.
    Exhausted,

    /// End the event cleanly with the outcome unchanged (`Stop`, or any
    /// control that has no runtime effect here).
    Stop,
}

/// Decide what the runtime should do for a lifecycle-event control.
///
/// `attempt` is the 1-based current attempt counter, and `control_budget` is
/// the per-control retry/resume ceiling already chosen for this control's first
/// firing (`1 + max_attempts`, in absolute attempt-counter terms). When
/// `attempt` has reached the budget the dispatch is
/// [`ControlDispatch::Exhausted`].
///
/// `has_session` reports whether a provider session id is available (only
/// matters for `Resume`). `provider_launched` reports whether the provider
/// child had launched this iteration; it decides whether a `Retry` re-enters
/// pre-flight (pre-launch) or re-invokes the provider (post-launch).
///
/// ## Uniformity
///
/// This decision is **event-agnostic**: every lifecycle event's stack
/// dispatches a control the same way. Which control is *valid* in which event
/// is the parse-time pre-scan's job ([`super::lifecycle_actions::LifecycleControlAction::is_valid_for`]
/// → `LifecycleActionPlacement`), not this function's — so there is no
/// per-signal gate here. `Stop`/`Skip`/`Error` carry no runtime recovery
/// effect and resolve to [`ControlDispatch::Stop`] (their terminal-outcome
/// effects are applied by the caller at the event boundary).
pub fn decide_control(
    control: &StackControl,
    attempt: u32,
    control_budget: u32,
    has_session: bool,
    provider_launched: bool,
) -> ControlDispatch {
    match control {
        StackControl::Stop | StackControl::Skip | StackControl::Error { .. } => {
            ControlDispatch::Stop
        }
        StackControl::Retry {
            max_attempts,
            backoff,
            delay,
        } => {
            // `control_budget` carries the absolute attempt ceiling derived
            // from `max_attempts` when the control first fired. When this is
            // the first firing (budget not yet established) derive it here.
            let budget = if control_budget == 0 {
                attempt.saturating_add(*max_attempts)
            } else {
                control_budget
            };
            if attempt >= budget {
                return ControlDispatch::Exhausted;
            }
            let base = parse_delay(delay);
            // `retry_index` is how many retries this control has already
            // consumed: the first retry (attempt == budget - max_attempts)
            // applies the base delay, each subsequent one doubles under
            // exponential backoff.
            let consumed = attempt.saturating_sub(budget.saturating_sub(*max_attempts));
            let adjusted = compute_backoff_delay(base, *backoff, consumed);
            ControlDispatch::Retry {
                delay: adjusted,
                reenter_preflight: !provider_launched,
            }
        }
        StackControl::Resume {
            message,
            max_attempts,
        } => {
            let budget = if control_budget == 0 {
                attempt.saturating_add(*max_attempts)
            } else {
                control_budget
            };
            if attempt >= budget {
                return ControlDispatch::Exhausted;
            }
            if !has_session {
                return ControlDispatch::ResumeWithoutSession;
            }
            ControlDispatch::Resume {
                message: message.clone(),
            }
        }
        StackControl::Proxy { target, .. } => ControlDispatch::Proxy {
            target: target.clone(),
        },
        StackControl::Defer { delay, reason } => ControlDispatch::Defer {
            delay: delay.clone(),
            reason: reason.clone(),
        },
    }
}

/// Compute the absolute attempt ceiling for a control's first firing.
///
/// `attempt` is the attempt at which the control fired; `max_attempts` is
/// the additional attempts requested. The ceiling is the attempt counter
/// value at which no further retries are permitted.
pub fn control_budget_for(attempt: u32, max_attempts: u32) -> u32 {
    attempt.saturating_add(max_attempts)
}

/// Apply the backoff strategy to a base delay for the `retry_index`-th retry
/// (0-based: the first retry uses the base delay unchanged).
///
/// Under [`RetryBackoff::Exponential`] the delay doubles per retry; under
/// [`RetryBackoff::Fixed`] it is constant. Doubling saturates at
/// [`Duration::MAX`] rather than overflowing.
pub fn compute_backoff_delay(
    base: Duration,
    backoff: RetryBackoff,
    retry_index: u32,
) -> Duration {
    match backoff {
        RetryBackoff::Fixed => base,
        RetryBackoff::Exponential => {
            let factor = 2u64.checked_pow(retry_index).unwrap_or(u64::MAX);
            base.checked_mul(u32::try_from(factor).unwrap_or(u32::MAX))
                .unwrap_or(Duration::MAX)
        }
    }
}

/// The maximum number of `proxy(...)` hand-offs permitted in a single run.
///
/// A `proxy` chain longer than this — even without a repeated document — is
/// treated as runaway control flow and stopped with a typed error.
pub const MAX_PROXY_HOPS: usize = 16;

/// Whether a `proxy` hand-off to `target` is permitted given the chain of
/// documents already proxied to in this run.
///
/// `chain` is the ordered list of resolved documents already visited by proxy
/// in this run. A hand-off is rejected when `target` already appears in the
/// chain (a self-proxy or an A→B→A cycle) or when accepting it would exceed
/// [`MAX_PROXY_HOPS`].
///
/// This is the pure decision used by the harness loop's `Proxy` arm; the
/// effectful swap (re-materialize, re-parse lifecycle, reset guard) only runs
/// when this returns `true`.
pub fn proxy_handoff_allowed(chain: &[std::path::PathBuf], target: &std::path::Path) -> bool {
    if chain.len() >= MAX_PROXY_HOPS {
        return false;
    }
    !chain.iter().any(|p| p == target)
}

/// Resolve a `Proxy` target reference to an existing prompt file.
///
/// Wraps [`crate::harness::resolve_harness_path`] (which handles `@repo/…`,
/// relative, and absolute forms) with an existence check so a hand-off to a
/// missing document fails loudly rather than producing an empty/garbage run.
///
/// ## Errors
///
/// Returns a [`crate::harness::HarnessError`] when the reference cannot be
/// resolved (e.g. an `@`-prefixed reference with no `repo_root`, propagated
/// directly from [`crate::harness::resolve_harness_path`]) or when the resolved
/// path is not an existing file.
pub fn resolve_proxy_target(
    target: &str,
    source_path: &std::path::Path,
    repo_root: Option<&std::path::Path>,
) -> Result<std::path::PathBuf, crate::harness::HarnessError> {
    let ctx = crate::harness::HarnessResolutionContext {
        source_path,
        repo_root,
    };
    let resolved = crate::harness::resolve_harness_path(target, &ctx)?;
    if !resolved.is_file() {
        return Err(crate::harness::HarnessError::PathResolutionFailed {
            raw: target.to_string(),
            detail: format!("proxy target does not exist: {}", resolved.display()),
        });
    }
    Ok(resolved)
}

/// Parse a lifecycle delay string (e.g. `"5m"`, `"0s"`, `"30 sec"`) into a
/// [`Duration`].
///
/// Reuses the harness timeout grammar. An unparseable or empty value yields
/// [`Duration::ZERO`] — a delay is advisory pacing, not a correctness gate,
/// so a malformed value degrades to "no wait" rather than aborting the run.
pub fn parse_delay(raw: &str) -> Duration {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Duration::ZERO;
    }
    crate::harness::parse_timeout(trimmed, std::path::Path::new("<lifecycle delay>"))
        .unwrap_or(Duration::ZERO)
}

#[cfg(test)]
mod tests;
