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
        StackControl::Proxy { target } => ControlDispatch::Proxy {
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
mod tests {
    use super::*;

    fn retry(max: u32, backoff: RetryBackoff, delay: &str) -> StackControl {
        StackControl::Retry {
            max_attempts: max,
            backoff,
            delay: delay.to_string(),
        }
    }

    #[test]
    fn stop_skip_error_resolve_to_stop() {
        for control in [
            StackControl::Stop,
            StackControl::Skip,
            StackControl::Error { reason: None },
        ] {
            assert_eq!(
                decide_control(&control, 1, 0, true, true),
                ControlDispatch::Stop
            );
        }
    }

    #[test]
    fn retry_post_launch_reinvokes_provider() {
        let control = retry(2, RetryBackoff::Fixed, "0s");
        assert_eq!(
            decide_control(&control, 1, 0, false, true),
            ControlDispatch::Retry {
                delay: Duration::ZERO,
                reenter_preflight: false,
            }
        );
    }

    #[test]
    fn retry_pre_launch_re_enters_preflight() {
        let control = retry(1, RetryBackoff::Fixed, "0s");
        assert_eq!(
            decide_control(&control, 1, 0, false, false),
            ControlDispatch::Retry {
                delay: Duration::ZERO,
                reenter_preflight: true,
            }
        );
    }

    #[test]
    fn retry_honors_max_attempts_budget() {
        let control = retry(2, RetryBackoff::Fixed, "0s");
        // First firing at attempt 1 establishes budget = 1 + 2 = 3.
        let budget = control_budget_for(1, 2);
        assert_eq!(budget, 3);
        // attempt 1 and 2 retry; attempt 3 is exhausted.
        assert!(matches!(
            decide_control(&control, 1, budget, false, true),
            ControlDispatch::Retry { .. }
        ));
        assert!(matches!(
            decide_control(&control, 2, budget, false, true),
            ControlDispatch::Retry { .. }
        ));
        assert_eq!(
            decide_control(&control, 3, budget, false, true),
            ControlDispatch::Exhausted
        );
    }

    #[test]
    fn exponential_backoff_doubles_per_retry() {
        let base = Duration::from_secs(5);
        assert_eq!(
            compute_backoff_delay(base, RetryBackoff::Exponential, 0),
            Duration::from_secs(5)
        );
        assert_eq!(
            compute_backoff_delay(base, RetryBackoff::Exponential, 1),
            Duration::from_secs(10)
        );
        assert_eq!(
            compute_backoff_delay(base, RetryBackoff::Exponential, 2),
            Duration::from_secs(20)
        );
    }

    #[test]
    fn fixed_backoff_is_constant() {
        let base = Duration::from_secs(7);
        for index in 0..4 {
            assert_eq!(
                compute_backoff_delay(base, RetryBackoff::Fixed, index),
                base
            );
        }
    }

    #[test]
    fn exponential_retry_dispatch_applies_doubled_delay() {
        // budget = 3, max_attempts = 2, delay 4s exponential.
        // attempt 1: consumed 0 → 4s; attempt 2: consumed 1 → 8s.
        let control = retry(2, RetryBackoff::Exponential, "4s");
        assert_eq!(
            decide_control(&control, 1, 3, false, true),
            ControlDispatch::Retry {
                delay: Duration::from_secs(4),
                reenter_preflight: false,
            }
        );
        assert_eq!(
            decide_control(&control, 2, 3, false, true),
            ControlDispatch::Retry {
                delay: Duration::from_secs(8),
                reenter_preflight: false,
            }
        );
    }

    #[test]
    fn resume_with_session_resumes() {
        let control = StackControl::Resume {
            message: "fix it".to_string(),
            max_attempts: 1,
        };
        assert_eq!(
            decide_control(&control, 1, 0, true, true),
            ControlDispatch::Resume {
                message: "fix it".to_string(),
            }
        );
    }

    #[test]
    fn resume_without_session_errors() {
        let control = StackControl::Resume {
            message: "fix it".to_string(),
            max_attempts: 1,
        };
        assert_eq!(
            decide_control(&control, 1, 0, false, true),
            ControlDispatch::ResumeWithoutSession
        );
    }

    #[test]
    fn resume_honors_max_attempts() {
        let control = StackControl::Resume {
            message: "again".to_string(),
            max_attempts: 1,
        };
        let budget = control_budget_for(1, 1); // 2
        assert!(matches!(
            decide_control(&control, 1, budget, true, true),
            ControlDispatch::Resume { .. }
        ));
        assert_eq!(
            decide_control(&control, 2, budget, true, true),
            ControlDispatch::Exhausted
        );
    }

    #[test]
    fn proxy_dispatches_target() {
        let control = StackControl::Proxy {
            target: "@prompts/other.md".to_string(),
        };
        assert_eq!(
            decide_control(&control, 1, 0, false, true),
            ControlDispatch::Proxy {
                target: "@prompts/other.md".to_string(),
            }
        );
    }

    #[test]
    fn recovery_controls_dispatch_event_agnostically() {
        // `decide_control` is event-agnostic: placement (which control is valid
        // in which event) is the parse-time pre-scan's job, so every recovery
        // control dispatches here regardless of the originating event.
        assert!(matches!(
            decide_control(&retry(1, RetryBackoff::Fixed, "0s"), 1, 0, false, true),
            ControlDispatch::Retry {
                reenter_preflight: false,
                ..
            }
        ));

        let resume = StackControl::Resume {
            message: "continue".to_string(),
            max_attempts: 1,
        };
        assert!(matches!(
            decide_control(&resume, 1, 0, true, true),
            ControlDispatch::Resume { .. }
        ));

        let proxy = StackControl::Proxy {
            target: "@x.md".to_string(),
        };
        assert!(matches!(
            decide_control(&proxy, 1, 0, false, true),
            ControlDispatch::Proxy { .. }
        ));

        let requeue = StackControl::Defer {
            delay: "5m".to_string(),
            reason: None,
        };
        assert!(matches!(
            decide_control(&requeue, 1, 0, false, true),
            ControlDispatch::Defer { .. }
        ));
    }

    #[test]
    fn requeue_carries_delay_and_reason() {
        let control = StackControl::Defer {
            delay: "5m".to_string(),
            reason: Some("later".to_string()),
        };
        assert_eq!(
            decide_control(&control, 1, 0, false, true),
            ControlDispatch::Defer {
                delay: "5m".to_string(),
                reason: Some("later".to_string()),
            }
        );
    }

    #[test]
    fn parse_delay_handles_units_and_garbage() {
        assert_eq!(parse_delay("5m"), Duration::from_secs(300));
        assert_eq!(parse_delay("30s"), Duration::from_secs(30));
        assert_eq!(parse_delay("0s"), Duration::ZERO);
        assert_eq!(parse_delay(""), Duration::ZERO);
        assert_eq!(parse_delay("not-a-duration"), Duration::ZERO);
    }

    #[test]
    fn resolve_proxy_target_resolves_existing_relative_file() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("prompt.md");
        std::fs::write(&source, "---\n---\n").unwrap();
        let target = dir.path().join("other.md");
        std::fs::write(&target, "---\n---\n").unwrap();

        let resolved = resolve_proxy_target("other.md", &source, None).unwrap();
        assert_eq!(resolved, target);
    }

    #[test]
    fn resolve_proxy_target_resolves_repo_relative() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("sub/prompt.md");
        std::fs::create_dir_all(source.parent().unwrap()).unwrap();
        std::fs::write(&source, "---\n---\n").unwrap();
        let target = dir.path().join("prompts/next.md");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "---\n---\n").unwrap();

        let resolved =
            resolve_proxy_target("@prompts/next.md", &source, Some(dir.path())).unwrap();
        assert_eq!(resolved, target);
    }

    #[test]
    fn proxy_handoff_allowed_rejects_self_and_cycles() {
        let a = std::path::PathBuf::from("/p/a.md");
        let b = std::path::PathBuf::from("/p/b.md");

        // Empty chain: anything is allowed.
        assert!(proxy_handoff_allowed(&[], &a));
        // First hop recorded; re-proxying to the same doc is a self-cycle.
        assert!(!proxy_handoff_allowed(std::slice::from_ref(&a), &a));
        // A -> B is fine; A -> B -> A closes a cycle.
        assert!(proxy_handoff_allowed(std::slice::from_ref(&a), &b));
        assert!(!proxy_handoff_allowed(&[a.clone(), b.clone()], &a));
    }

    #[test]
    fn proxy_handoff_allowed_enforces_hop_limit() {
        // A chain at the hop limit rejects any further hand-off, even to a
        // never-seen document.
        let chain: Vec<std::path::PathBuf> = (0..MAX_PROXY_HOPS)
            .map(|i| std::path::PathBuf::from(format!("/p/{i}.md")))
            .collect();
        let fresh = std::path::PathBuf::from("/p/fresh.md");
        assert!(!proxy_handoff_allowed(&chain, &fresh));
        // One below the limit still allows a fresh target.
        assert!(proxy_handoff_allowed(&chain[..MAX_PROXY_HOPS - 1], &fresh));
    }

    #[test]
    fn resolve_proxy_target_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("prompt.md");
        std::fs::write(&source, "---\n---\n").unwrap();

        let err = resolve_proxy_target("nope.md", &source, None).unwrap_err();
        assert!(
            matches!(err, crate::harness::HarnessError::PathResolutionFailed { .. }),
            "unexpected variant: {err:?}"
        );
        assert!(
            err.to_string().contains("does not exist"),
            "unexpected: {err}"
        );
    }
}
