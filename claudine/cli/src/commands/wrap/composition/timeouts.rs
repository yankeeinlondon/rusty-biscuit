//! Timeout resolution and prompt-path helpers for the composition executor.
//!
//! These helpers are pure (no provider launch, no filesystem mutation) and
//! are shared by the composition executor, the harness loop, and the direct
//! wrapper path through the `composition::` re-exports.

use claudine::composition::SessionInteractivitySource;

/// Build a [`PromptTimingContext`] from a resolved prompt path, the
/// effective repo root (when any), and the optional warn thresholds
/// parsed from harness frontmatter.
///
/// `display_path` is resolved in the order repo root → CWD → `$HOME`
/// (falling back to the absolute path when none apply) per the feature
/// spec's "relative path" rules for the OSC8 link text.
pub(crate) fn build_prompt_timing_context(
    absolute_path: &std::path::Path,
    repo_root: Option<&std::path::Path>,
    timeout_warn: Option<std::time::Duration>,
    step_timeout_warn: Option<std::time::Duration>,
) -> claudine::stream::prompt_timing::PromptTimingContext {
    let display_path = resolve_prompt_display_path(absolute_path, repo_root);
    claudine::stream::prompt_timing::PromptTimingContext {
        absolute_path: absolute_path.to_path_buf(),
        display_path,
        timeout_warn,
        step_timeout_warn,
    }
}

/// Source-precedence input for [`resolve_single_timeout`].
///
/// `cli` is the value passed via `--timeout` / `--step-timeout`.
/// `frontmatter` is the value parsed from `HarnessPlan.timeout` /
/// `HarnessPlan.step_timeout`.
/// `env_var` is the env-var name to consult as the third-priority source.
/// `built_in` is the final fallback (e.g. `Some(30m)` for `step_timeout`,
/// `None` for `timeout`).
pub(crate) struct TimeoutResolutionInput<'a> {
    pub cli: Option<String>,
    pub frontmatter: Option<std::time::Duration>,
    pub env_var: &'a str,
    pub built_in: Option<std::time::Duration>,
}

/// Resolve a single timeout following the documented precedence chain:
///
///   CLI flag > frontmatter > env-var default > built-in default.
///
/// Env values use the same `parse_timeout` grammar as frontmatter
/// (`30s`, `5m`, `2h`). An env value of `0s` (or any zero duration via the
/// grammar) **disables** the rule for this run, returning `None` even if a
/// non-zero built-in default exists. Invalid env values are silently
/// ignored and the chain falls through to the next layer.
pub(crate) fn resolve_single_timeout(
    input: TimeoutResolutionInput<'_>,
) -> Option<std::time::Duration> {
    if let Some(raw) = input.cli {
        match claudine::harness::parse_timeout(&raw, std::path::Path::new("<cli>")) {
            Ok(d) => return Some(d),
            Err(_) => {
                // Invalid CLI value should have been caught earlier, but
                // fall through rather than panicking.
            }
        }
    }
    if let Some(d) = input.frontmatter {
        return Some(d);
    }
    match std::env::var(input.env_var) {
        Ok(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                input.built_in
            } else if is_zero_duration_literal(trimmed) {
                // Spec: env value of `0s` disables the rule (parse_timeout
                // itself rejects zero, so we recognise the literal here).
                None
            } else {
                match claudine::harness::parse_timeout(trimmed, std::path::Path::new("<env>")) {
                    Ok(d) => Some(d),
                    Err(_) => input.built_in,
                }
            }
        }
        Err(_) => input.built_in,
    }
}

/// Recognise env-var literals that the user means as "disable this rule".
///
/// Accepts plain `0`, `0s`, `0 seconds`, `0m`, `0h`, and decimal zeros like
/// `0.0s` — anything whose numeric component is exactly `0.0` regardless of
/// unit. A fractional value like `0.5s` is **not** a disable literal: the `.`
/// is part of the numeric prefix and `0.5` is non-zero.
fn is_zero_duration_literal(value: &str) -> bool {
    let trimmed = value.trim();
    let num_end = trimmed
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(trimmed.len());
    if num_end == 0 {
        return false;
    }
    let (num, _rest) = trimmed.split_at(num_end);
    num.parse::<f64>().is_ok_and(|n| n == 0.0)
}

/// Resolve `timeout` and `step_timeout` simultaneously and assemble a
/// [`TimeoutConfig`] for the watchdog ticker.
///
/// CLI > frontmatter > env > built-in. Built-ins are `None` for `timeout`
/// (no wall-clock kill unless opted in) and `30m` for `step_timeout`.
/// Supporting knobs (`kill_grace`, `interval`) are read from env via
/// [`super::super::subagent_watchdog::TimeoutConfig::resolve`].
pub(crate) fn resolve_timeouts(
    cli_timeout: Option<String>,
    plan_timeout: Option<std::time::Duration>,
    cli_step_timeout: Option<String>,
    plan_step_timeout: Option<std::time::Duration>,
) -> super::super::subagent_watchdog::TimeoutConfig {
    let timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_timeout,
        frontmatter: plan_timeout,
        env_var: "CLAUDINE_TIMEOUT",
        built_in: None,
    });
    let step_timeout = resolve_single_timeout(TimeoutResolutionInput {
        cli: cli_step_timeout,
        frontmatter: plan_step_timeout,
        env_var: "CLAUDINE_STEP_TIMEOUT",
        built_in: Some(std::time::Duration::from_secs(30 * 60)),
    });
    super::super::subagent_watchdog::TimeoutConfig::resolve(timeout, step_timeout)
}

/// Resolve the OpenCode stalled-generation backstop budget following the
/// documented precedence chain (CLI flag > env-var > built-in `10m`).
///
/// Each source is a clean three-state check against the canonical grammar
/// ([`claudine::harness::parse_timeout_allow_zero`]), disabling the guard only
/// on a true zero duration (`0s`, `0.0s`, `0m`) — a fractional value such as
/// `0.5s` arms the guard for 500ms:
///
/// - **CLI** (`--stall-timeout`): pre-validated upstream by
///   [`super::super::wrapper_stages::parse_cli_timeouts`] /
///   [`crate::commands::compose::SharedComposeArgs::stall_timeout_secs`], so an
///   `Err` here is unreachable; a zero returns `None`, a positive returns
///   `Some`, and a (defensive) `Err` falls through.
/// - **Env** (`CLAUDINE_OPENCODE_STALL_TIMEOUT`): a zero disables; a positive
///   is used; an invalid value falls through to the built-in.
/// - **Built-in**: `10m`.
///
/// ## Returns
///
/// `Some(duration)` when the guard is active for this run, or `None` when
/// the guard is disabled by an explicit zero at the CLI or env layer.
pub(crate) fn resolve_stall_timeout(cli: Option<String>) -> Option<std::time::Duration> {
    // CLI is pre-validated upstream, so the `Ok` arm is the real path; an
    // `Err` is unreachable and falls through rather than being relied upon.
    if let Some(raw) = cli
        && let Ok(d) =
            claudine::harness::parse_timeout_allow_zero(&raw, std::path::Path::new("<cli>"))
    {
        return if d.is_zero() { None } else { Some(d) };
    }
    if let Ok(raw) = std::env::var("CLAUDINE_OPENCODE_STALL_TIMEOUT") {
        let trimmed = raw.trim();
        if !trimmed.is_empty()
            && let Ok(d) =
                claudine::harness::parse_timeout_allow_zero(trimmed, std::path::Path::new("<env>"))
        {
            return if d.is_zero() { None } else { Some(d) };
        }
    }
    Some(std::time::Duration::from_secs(10 * 60))
}

pub(crate) fn resolve_prompt_display_path(
    path: &std::path::Path,
    repo_root: Option<&std::path::Path>,
) -> String {
    if let Some(root) = repo_root
        && let Ok(rel) = path.strip_prefix(root)
    {
        return biscuit_file::to_portable_string(rel);
    }
    if let Ok(cwd) = std::env::current_dir()
        && let Ok(rel) = path.strip_prefix(&cwd)
    {
        return biscuit_file::to_portable_string(rel);
    }
    if let Some(home) = dirs::home_dir()
        && let Ok(rel) = path.strip_prefix(&home)
    {
        return format!("~/{}", biscuit_file::to_portable_string(rel));
    }
    biscuit_file::to_portable_string(path)
}

/// Format the timeout-conflict error message, attributing the resolved
/// interactive mode to its source so users can tell a frontmatter-driven
/// conflict from a flag-driven one, and naming the conflicting timeout flag
/// (`--timeout` or `--step-timeout`).
pub(crate) fn format_interactive_timeout_conflict(
    source: SessionInteractivitySource,
    flag: &str,
) -> String {
    format!("interactive mode (from {source}) cannot be used with {flag}")
}

/// Extract a top-level frontmatter timeout duration (`timeout` /
/// `step_timeout`) for the resolved-interactive conflict check.
///
/// Returns `None` when the key is absent or its value is not a parseable
/// duration string. A malformed value is surfaced later by
/// [`claudine::harness::parse_harness_plan`], so swallowing the parse error
/// here is intentional — the syntax diagnostic takes precedence over the
/// interactive conflict.
pub(crate) fn frontmatter_timeout_duration(
    frontmatter: &serde_json::Value,
    key: &str,
    source_path: &std::path::Path,
) -> Option<std::time::Duration> {
    frontmatter
        .as_object()
        .and_then(|obj| obj.get(key))
        .and_then(|v| v.as_str())
        .and_then(|raw| claudine::harness::parse_timeout(raw, source_path).ok())
}

#[cfg(test)]
mod stall_timeout_tests {
    use super::*;
    use std::time::Duration;

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, original }
        }

        fn clear(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    const ENV: &str = "CLAUDINE_OPENCODE_STALL_TIMEOUT";

    #[test]
    #[serial_test::serial]
    fn env_zero_literal_disables_the_guard() {
        let _guard = EnvGuard::set(ENV, "0s");
        assert_eq!(resolve_stall_timeout(None), None);
    }

    #[test]
    #[serial_test::serial]
    fn unset_env_with_no_cli_resolves_to_built_in_10m() {
        let _guard = EnvGuard::clear(ENV);
        assert_eq!(
            resolve_stall_timeout(None),
            Some(Duration::from_secs(10 * 60))
        );
    }

    #[test]
    #[serial_test::serial]
    fn cli_beats_env() {
        let _guard = EnvGuard::set(ENV, "5m");
        assert_eq!(
            resolve_stall_timeout(Some("2m".to_string())),
            Some(Duration::from_secs(120))
        );
    }

    #[test]
    #[serial_test::serial]
    fn cli_zero_literal_disables_even_with_env_set() {
        let _guard = EnvGuard::set(ENV, "5m");
        assert_eq!(resolve_stall_timeout(Some("0s".to_string())), None);
    }

    #[test]
    #[serial_test::serial]
    fn cli_fractional_resolves_to_millis() {
        let _guard = EnvGuard::clear(ENV);
        assert_eq!(
            resolve_stall_timeout(Some("0.5s".to_string())),
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    #[serial_test::serial]
    fn env_fractional_resolves_to_millis() {
        let _guard = EnvGuard::set(ENV, "0.5s");
        assert_eq!(
            resolve_stall_timeout(None),
            Some(Duration::from_millis(500))
        );
    }

    #[test]
    #[serial_test::serial]
    fn env_zero_seconds_disables_the_guard() {
        let _guard = EnvGuard::set(ENV, "0s");
        assert_eq!(resolve_stall_timeout(None), None);
    }

    #[test]
    fn is_zero_duration_literal_recognizes_only_true_zeros() {
        assert!(is_zero_duration_literal("0"));
        assert!(is_zero_duration_literal("0s"));
        assert!(is_zero_duration_literal("0.0s"));
        assert!(!is_zero_duration_literal("0.5s"));
        assert!(!is_zero_duration_literal("5s"));
        assert!(!is_zero_duration_literal(""));
    }
}
