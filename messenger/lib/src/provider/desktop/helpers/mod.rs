//! Third-party desktop notification helper abstraction.
//!
//! Each `HelperBackend` adapts a CLI like `dunstify`, `notify-send`,
//! `terminal-notifier`, `alerter`, `snoretoast`, or `BurntToast` into the
//! same interface that the platform [`DesktopBackend`](super::DesktopBackend)
//! consumes. Helpers are elected at send time (see [`election`]) and the
//! native backend remains the universal fallback.
//!
//! The trait is intentionally `pub(crate)` — helpers are an implementation
//! detail of the desktop provider. The shared identifier
//! [`HelperName`] is re-exported from `sniff` so consumers can name helpers
//! through the public CLI/config surface without messenger redefining the
//! catalog.
//!
//! ## Notes
//!
//! - Helper construction takes only what the helper needs to invoke its
//!   binary (path, optional version, daemon hints). Sniff detection happens
//!   in the platform backend's `new()`.
//! - `score()` returns `0` when the helper cannot serve a request; the
//!   election skips score-zero helpers.
//! - Errors from `send()`/`replace()` are converted into a fallback signal
//!   by the platform backend; only `Unsupported` and `Parse` are propagated
//!   directly to the caller.

// Phase 2 wires only the Linux backend to these adapters; Phases 3 and 4
// extend the macOS and Windows backends. Until then, suppress the dead-code
// lint on non-Linux to keep CI quiet without per-item annotations.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

pub(crate) mod election;
pub(crate) mod process;

#[cfg(target_os = "linux")]
pub(crate) mod dunstify;
#[cfg(target_os = "linux")]
pub(crate) mod notify_send;

use async_trait::async_trait;

use super::request::{DesktopNotificationReceipt, DesktopNotificationRequest};

#[cfg(target_os = "linux")]
pub(crate) use election::{HelperAttempt, elect_helpers};

/// Public identifier for a notification helper.
///
/// Re-exported from `sniff` so the messenger CLI, config, and library all
/// share the same catalog and string forms.
pub type HelperName = sniff::programs::NotificationHelper;

/// Capability flags advertised by a [`HelperBackend`].
///
/// Used by [`elect_helpers`] to filter helpers that cannot serve a request
/// shape (interactive vs notice-only, image present, replace requested).
#[derive(Debug, Clone, Copy)]
pub(crate) struct HelperCapabilities {
    pub actions: bool,
    pub reply: bool,
    pub image: bool,
    pub sound: bool,
    pub replace: bool,
    pub group: bool,
    /// `true` when the helper blocks until the user dismisses or activates
    /// the notification. Used by election to prefer non-blocking helpers
    /// for notice-only sends.
    pub blocking: bool,
}

/// Errors returned from a [`HelperBackend`] invocation.
///
/// Most variants are converted to a fallback signal by the platform backend.
/// `Unsupported` and `Parse` propagate directly so callers see precise
/// failures instead of a silent downgrade.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HelperError {
    /// The helper binary disappeared between detection and send time.
    #[error("helper not present on PATH")]
    NotPresent,
    /// The helper does not support the requested operation.
    #[error("helper does not support {0}")]
    Unsupported(&'static str),
    /// The helper exited with a non-zero status.
    #[error("helper exited with status {status}: {stderr}")]
    Exited { status: i32, stderr: String },
    /// The helper invocation timed out.
    #[error("helper invocation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    /// The helper output could not be parsed.
    #[error("helper output unparseable: {0}")]
    Parse(String),
    /// An I/O error occurred spawning or talking to the helper.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl HelperError {
    /// Produce a short tag suitable for `helper_fallbacks` metadata.
    pub(crate) fn summary_tag(&self) -> &'static str {
        match self {
            Self::NotPresent => "not_present",
            Self::Unsupported(_) => "unsupported",
            Self::Exited { .. } => "exited",
            Self::Timeout { .. } => "timeout",
            Self::Parse(_) => "parse",
            Self::Io(_) => "io",
        }
    }

    /// Whether the platform backend should fall through to the next helper
    /// (or native backend) on this error.
    pub(crate) fn is_fallback_eligible(&self) -> bool {
        matches!(
            self,
            Self::NotPresent | Self::Exited { .. } | Self::Timeout { .. } | Self::Io(_)
        )
    }
}

/// Trait implemented by every notification helper adapter.
///
/// `score()` is the single hook the election uses to filter helpers per
/// dispatch shape. Helpers that cannot serve a given request return `0`.
#[async_trait]
pub(crate) trait HelperBackend: Send + Sync {
    /// Stable identifier used in receipts (`helper_used` metadata) and the
    /// election trace (`helper_fallbacks`).
    fn name(&self) -> HelperName;

    /// What the helper can deliver on the host.
    fn capabilities(&self) -> HelperCapabilities;

    /// Per-send fitness for this dispatch.
    ///
    /// Returns `0` when the helper cannot serve the request (missing
    /// capability, daemon mismatch, malformed input). Higher values are
    /// preferred during election.
    fn score(&self, request: &DesktopNotificationRequest) -> u8;

    /// Deliver the request via the helper binary.
    async fn send(
        &self,
        request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError>;

    /// Replace an existing notification by helper-specific id.
    ///
    /// Default implementation reports the operation as unsupported; helpers
    /// that genuinely support replacement override this method.
    async fn replace(
        &self,
        _id: &str,
        _request: &DesktopNotificationRequest,
    ) -> Result<DesktopNotificationReceipt, HelperError> {
        Err(HelperError::Unsupported("replace"))
    }
}
