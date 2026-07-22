//! Launch-workspace resolution helpers for the composition executor.
//!
//! [`select_launch_workspace`] is the W0 hot path: it returns the precomputed
//! `prep` value when present, falling back to the legacy
//! `env::resolve_launch_workspace_context` walk only for library callers that
//! don't thread a `CompositionPrepContext` (none in the production CLI).

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use claudine::diagnostics::{DiagnosticSnapshot, RestoredDiagnostic};
use color_eyre::eyre::Result;

use super::env;

/// W0 instrumentation counter: increments every time
/// [`select_launch_workspace`] falls back to the legacy
/// `env::resolve_launch_workspace_context` call.
///
/// The fallback path performs a fresh `detect_git` + `detect_repo`
/// filesystem scan, which is exactly the redundancy W0 was designed to
/// remove. The counter is process-global so a regression test can
/// observe it across whatever spawn / fixture machinery the test uses
/// without having to thread an injectable counter through the entire
/// composition request type. Tests reset it via
/// [`reset_launch_workspace_fallbacks_for_tests`].
static LAUNCH_WORKSPACE_FALLBACK_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Choose the launch workspace context for the executor.
///
/// Returns the precomputed `prep` value when present (the W0 hot path).
/// Falls back to the legacy `env::resolve_launch_workspace_context` walk
/// only for library callers that don't thread a `CompositionPrepContext`
/// (none in the production CLI).
///
/// The fallback branch increments [`LAUNCH_WORKSPACE_FALLBACK_COUNT`] so
/// regression tests can prove the production hot path stays on the
/// no-walk branch even after future refactors.
pub(crate) fn select_launch_workspace(
    prep: Option<&env::LaunchWorkspaceContext>,
    launch_cwd: &Path,
    source_repo_root: Option<&Path>,
) -> env::LaunchWorkspaceContext {
    if let Some(p) = prep {
        return p.clone();
    }
    LAUNCH_WORKSPACE_FALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    env::resolve_launch_workspace_context(launch_cwd, source_repo_root)
}

/// Test-only: snapshot of the fallback counter.
#[cfg(test)]
pub(crate) fn launch_workspace_fallback_count_for_tests() -> usize {
    LAUNCH_WORKSPACE_FALLBACK_COUNT.load(Ordering::SeqCst)
}

/// Test-only: reset the fallback counter so an isolated test can
/// observe a clean baseline.
#[cfg(test)]
pub(crate) fn reset_launch_workspace_fallbacks_for_tests() {
    LAUNCH_WORKSPACE_FALLBACK_COUNT.store(0, Ordering::SeqCst);
}

/// Enforce the `--repo` legacy hard-fail contract when prep-time
/// launch-context detection failed.
///
/// `CompositionPrepContext` runs a single shared `sniff::detect_with_plan`
/// scan and falls back to a default `LaunchContext` on failure so best-
/// effort consumers can keep going. `--repo` is not a best-effort
/// consumer: it requires real repo detection. When the prep scan failed
/// **and** `--repo` is set, surface the captured sniff error as a hard
/// run abort, matching the behavior of the legacy non-prep path that
/// called `LaunchContext::from_cwd` directly.
///
/// The captured failure arrives as a [`DiagnosticSnapshot`] because the prep
/// record is `Clone` and a concrete error is not. Restoring it — rather than
/// lifting `snapshot.message` into an `eyre!` string — is what keeps the
/// original `code`, `category`, `disposition`, `origin`, `detail`, and cause
/// available to effective-diagnostic selection, `err.*`, and `StatusBlock`
/// rendering at the point the run actually aborts.
pub(super) fn enforce_repo_launch_detection(
    repo: bool,
    prep_launch_detection_error: Option<&DiagnosticSnapshot>,
) -> Result<()> {
    if repo && let Some(captured) = prep_launch_detection_error {
        return Err(RestoredDiagnostic::new(captured.clone())
            .with_context(REPO_LAUNCH_DETECTION_CONTEXT)
            .into());
    }
    Ok(())
}

/// The `--repo` framing prefixed onto the restored message.
///
/// A user-visible surface predating the typed restoration; the text is held
/// byte-identical to the prose the previous `eyre!` produced.
const REPO_LAUNCH_DETECTION_CONTEXT: &str =
    "--repo requires startup repo detection, but launch-context detection failed";
