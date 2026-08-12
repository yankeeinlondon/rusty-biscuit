//! Precedence logic for resolving prompt reporting configuration.
//!
//! The precedence chain (highest to lowest) is:
//! 1. CLI switches (`--silent`, `--quiet`, `--verbose`)
//! 2. `CLAUDINE_SYSTEM_PROMPT` environment variable
//! 3. Prompt length (< 10 lines → full body)
//! 4. Frontmatter `verbosity` property
//! 5. Default (`Summary`)

use super::types::{ReportMode, TruncationMode};

/// Resolve the [`ReportMode`] for a system prompt, applying the precedence
/// chain documented in the module-level description.
pub fn resolve_system_prompt_report_mode(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    env_verbosity: Option<ReportMode>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<ReportMode>,
    prompt_unchanged: bool,
) -> ReportMode {
    if cli_silent {
        return ReportMode::Silent;
    }
    if cli_quiet {
        return ReportMode::Summary;
    }
    if cli_verbose {
        return ReportMode::Full;
    }

    if let Some(mode) = env_verbosity {
        return mode;
    }

    if prompt_line_count < 10 {
        return ReportMode::Full;
    }

    if let Some(mode) = frontmatter_verbosity {
        return mode;
    }

    if prompt_unchanged {
        return ReportMode::Silent;
    }

    ReportMode::Summary
}

/// Resolve the [`ReportMode`] for a user (agent) prompt.
///
/// The agent prompt follows simpler rules than the system prompt:
/// - `--silent` suppresses everything ([`ReportMode::Silent`]).
/// - `--verbose` forces [`ReportMode::Full`].
/// - By default, the full body is shown when ≤ 40 lines, otherwise a
///   front/back truncated body ([`ReportMode::Partial`]).
///
/// `--quiet` is intentionally not a parameter here: it is a no-op for the
/// agent prompt (system-prompt-only control).
pub fn resolve_agent_prompt_report_mode(
    cli_silent: bool,
    cli_verbose: bool,
    prompt_line_count: usize,
) -> ReportMode {
    if cli_silent {
        return ReportMode::Silent;
    }
    if cli_verbose {
        return ReportMode::Full;
    }
    if prompt_line_count <= 40 {
        ReportMode::Full
    } else {
        ReportMode::Partial {
            truncation: TruncationMode::FrontBack,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- System prompt precedence tests ---

    #[test]
    fn cli_silent_wins_over_everything() {
        let mode = resolve_system_prompt_report_mode(
            true,
            false,
            false,
            Some(ReportMode::Full),
            5,
            Some(ReportMode::Full),
            false,
        );
        assert_eq!(mode, ReportMode::Silent);
    }

    #[test]
    fn cli_quiet_wins_over_env_and_frontmatter() {
        let mode = resolve_system_prompt_report_mode(
            false,
            true,
            false,
            Some(ReportMode::Full),
            5,
            Some(ReportMode::Full),
            false,
        );
        assert_eq!(mode, ReportMode::Summary);
    }

    #[test]
    fn cli_verbose_wins_over_env_and_frontmatter() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            true,
            Some(ReportMode::Summary),
            200,
            Some(ReportMode::Summary),
            false,
        );
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn env_verbose_used_when_no_cli_flags() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            Some(ReportMode::Full),
            200,
            Some(ReportMode::Summary),
            false,
        );
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn env_silent_used_when_no_cli_flags() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            Some(ReportMode::Silent),
            5,
            Some(ReportMode::Full),
            false,
        );
        assert_eq!(mode, ReportMode::Silent);
    }

    #[test]
    fn prompt_length_short_circuits_frontmatter() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            None,
            5,
            Some(ReportMode::Summary),
            false,
        );
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn frontmatter_used_when_no_other_hints() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            None,
            50,
            Some(ReportMode::Full),
            false,
        );
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn default_is_summary() {
        let mode =
            resolve_system_prompt_report_mode(false, false, false, None, 50, None, false);
        assert_eq!(mode, ReportMode::Summary);
    }

    #[test]
    fn unchanged_default_suppresses_header() {
        let mode = resolve_system_prompt_report_mode(false, false, false, None, 50, None, true);
        assert_eq!(mode, ReportMode::Silent);
    }

    #[test]
    fn unchanged_overridden_by_verbose() {
        let mode = resolve_system_prompt_report_mode(false, false, true, None, 50, None, true);
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn unchanged_overridden_by_env() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            Some(ReportMode::Summary),
            50,
            None,
            true,
        );
        assert_eq!(mode, ReportMode::Summary);
    }

    #[test]
    fn unchanged_overridden_by_frontmatter() {
        let mode = resolve_system_prompt_report_mode(
            false,
            false,
            false,
            None,
            50,
            Some(ReportMode::Full),
            true,
        );
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn long_prompt_with_no_hints_defaults_to_summary() {
        let mode =
            resolve_system_prompt_report_mode(false, false, false, None, 100, None, false);
        assert_eq!(mode, ReportMode::Summary);
    }

    // --- User prompt precedence tests ---

    #[test]
    fn user_silent_suppresses_all() {
        let mode = resolve_agent_prompt_report_mode(true, false, 10);
        assert_eq!(mode, ReportMode::Silent);
    }

    #[test]
    fn user_quiet_is_noop() {
        // Per spec: `--quiet` is a no-op for the user prompt; header
        // and body still render and length rules apply.
        let short = resolve_agent_prompt_report_mode(false, false, 10);
        assert_eq!(short, ReportMode::Full);

        let long = resolve_agent_prompt_report_mode(false, false, 100);
        assert_eq!(
            long,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack
            }
        );
    }

    #[test]
    fn user_verbose_forces_full() {
        let mode = resolve_agent_prompt_report_mode(false, true, 200);
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn user_short_prompt_shows_full() {
        let mode = resolve_agent_prompt_report_mode(false, false, 40);
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn user_long_prompt_uses_frontback() {
        let mode = resolve_agent_prompt_report_mode(false, false, 41);
        assert_eq!(
            mode,
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack
            }
        );
    }
}
