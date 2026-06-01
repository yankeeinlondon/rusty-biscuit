//! Precedence logic for resolving prompt reporting configuration.
//!
//! The precedence chain (highest to lowest) is:
//! 1. CLI switches (`--silent`, `--quiet`, `--verbose`)
//! 2. `CLAUDINE_SYSTEM_PROMPT` environment variable
//! 3. Prompt length (< 10 lines → full body)
//! 4. Frontmatter `verbosity` property
//! 5. Default (`Summary`)

use super::types::{
    PromptReportFormat, ReportMode, SystemPromptReportConfig, TruncationMode,
    UserPromptReportConfig,
};

/// Resolve the reporting configuration for a system prompt, applying the
/// precedence chain documented in the module-level description.
pub fn resolve_system_prompt_report_config(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    env_verbosity: Option<ReportMode>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<ReportMode>,
) -> SystemPromptReportConfig {
    resolve_system_prompt_report_config_with_change(
        cli_silent,
        cli_quiet,
        cli_verbose,
        env_verbosity,
        prompt_line_count,
        frontmatter_verbosity,
        false,
    )
}

/// Like [`resolve_system_prompt_report_config`] but with a `prompt_unchanged`
/// hint that suppresses the header in the default precedence branch.
///
/// Per spec: when no CLI flag, ENV variable, or frontmatter selects a body
/// mode and the composed prompt is unchanged from the previous session at
/// this scope, the header (and therefore the entire report) is suppressed.
pub fn resolve_system_prompt_report_config_with_change(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    env_verbosity: Option<ReportMode>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<ReportMode>,
    prompt_unchanged: bool,
) -> SystemPromptReportConfig {
    let mode = resolve_system_prompt_report_mode(
        cli_silent,
        cli_quiet,
        cli_verbose,
        env_verbosity,
        prompt_line_count,
        frontmatter_verbosity,
        prompt_unchanged,
    );
    config_from_mode(mode)
}

/// Resolve the [`ReportMode`] for a system prompt, applying the precedence
/// chain documented in the module-level description.
///
/// This is the encapsulated successor to
/// [`resolve_system_prompt_report_config_with_change`]; it returns a single
/// [`ReportMode`] rather than the legacy boolean-bag config.
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

/// Convert a [`ReportMode`] into the legacy [`SystemPromptReportConfig`].
///
/// This preserves the visual output of the legacy CLI call sites until they
/// are migrated to [`super::types::ReportMode`]-aware report types in a
/// later phase.
fn config_from_mode(mode: ReportMode) -> SystemPromptReportConfig {
    match mode {
        ReportMode::Silent => SystemPromptReportConfig {
            show_header: false,
            show_summary: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        ReportMode::Summary => SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        ReportMode::Partial { truncation } => SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::PartialPrompt,
            truncation,
        },
        ReportMode::Full => SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        },
    }
}

/// Resolve the reporting configuration for a user (agent) prompt.
///
/// The user prompt follows simpler rules than the system prompt:
/// - `--silent` suppresses everything.
/// - `--quiet` is a **no-op** for the user prompt — header and body are
///   rendered using the same rules as the default case. (`--quiet` is a
///   system-prompt-only control.)
/// - `--verbose` forces the full body.
/// - By default, the full body is shown when ≤ 40 lines.
/// - When > 40 lines, `FrontBack` truncation is used.
pub fn resolve_user_prompt_report_config(
    cli_silent: bool,
    _cli_quiet: bool,
    cli_verbose: bool,
    prompt_line_count: usize,
) -> UserPromptReportConfig {
    let mode = resolve_agent_prompt_report_mode(cli_silent, cli_verbose, prompt_line_count);
    user_config_from_mode(mode)
}

fn user_config_from_mode(mode: ReportMode) -> UserPromptReportConfig {
    match mode {
        ReportMode::Silent => UserPromptReportConfig {
            show_header: false,
            show_body: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        ReportMode::Summary => UserPromptReportConfig {
            show_header: true,
            show_body: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        ReportMode::Partial { truncation } => UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::PartialPrompt,
            truncation,
        },
        ReportMode::Full => UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- System prompt precedence tests (legacy config shape) ---

    #[test]
    fn cli_silent_wins_over_everything() {
        let config = resolve_system_prompt_report_config(
            true,
            false,
            false,
            Some(ReportMode::Full),
            5,
            Some(ReportMode::Full),
        );
        assert!(!config.show_header);
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    #[test]
    fn cli_quiet_wins_over_env_and_frontmatter() {
        let config = resolve_system_prompt_report_config(
            false,
            true,
            false,
            Some(ReportMode::Full),
            5,
            Some(ReportMode::Full),
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    #[test]
    fn cli_verbose_wins_over_env_and_frontmatter() {
        let config = resolve_system_prompt_report_config(
            false,
            false,
            true,
            Some(ReportMode::Summary),
            200,
            Some(ReportMode::Summary),
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn env_verbose_used_when_no_cli_flags() {
        let config = resolve_system_prompt_report_config(
            false,
            false,
            false,
            Some(ReportMode::Full),
            200,
            Some(ReportMode::Summary),
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn env_silent_used_when_no_cli_flags() {
        let config = resolve_system_prompt_report_config(
            false,
            false,
            false,
            Some(ReportMode::Silent),
            5,
            Some(ReportMode::Full),
        );
        assert!(!config.show_header);
    }

    #[test]
    fn prompt_length_short_circuits_frontmatter() {
        let config = resolve_system_prompt_report_config(
            false,
            false,
            false,
            None,
            5,
            Some(ReportMode::Summary),
        );
        // Prompt length (< 10) should win over frontmatter.
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn frontmatter_used_when_no_other_hints() {
        let config = resolve_system_prompt_report_config(
            false,
            false,
            false,
            None,
            50,
            Some(ReportMode::Full),
        );
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn default_is_summary() {
        let config = resolve_system_prompt_report_config(false, false, false, None, 50, None);
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    #[test]
    fn unchanged_default_suppresses_header() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, false, None, 50, None, true,
        );
        assert!(!config.show_header);
        assert!(!config.show_summary);
    }

    #[test]
    fn unchanged_overridden_by_verbose() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, true, None, 50, None, true,
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn unchanged_overridden_by_env() {
        let config = resolve_system_prompt_report_config_with_change(
            false,
            false,
            false,
            Some(ReportMode::Summary),
            50,
            None,
            true,
        );
        assert!(config.show_header);
    }

    #[test]
    fn unchanged_overridden_by_frontmatter() {
        let config = resolve_system_prompt_report_config_with_change(
            false,
            false,
            false,
            None,
            50,
            Some(ReportMode::Full),
            true,
        );
        assert!(config.show_header);
    }

    #[test]
    fn long_prompt_with_no_hints_defaults_to_summary() {
        let config = resolve_system_prompt_report_config(false, false, false, None, 100, None);
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    // --- System prompt resolver returning `ReportMode` ---

    #[test]
    fn mode_cli_silent_wins() {
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
    fn mode_cli_quiet_is_summary() {
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
    fn mode_cli_verbose_is_full() {
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
    fn mode_env_used_when_no_cli() {
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
    fn mode_short_prompt_is_full() {
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
    fn mode_frontmatter_used_when_no_other_hints() {
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
    fn mode_default_is_summary() {
        let mode =
            resolve_system_prompt_report_mode(false, false, false, None, 50, None, false);
        assert_eq!(mode, ReportMode::Summary);
    }

    #[test]
    fn mode_unchanged_default_is_silent() {
        let mode = resolve_system_prompt_report_mode(false, false, false, None, 50, None, true);
        assert_eq!(mode, ReportMode::Silent);
    }

    #[test]
    fn mode_unchanged_overridden_by_verbose() {
        let mode = resolve_system_prompt_report_mode(false, false, true, None, 50, None, true);
        assert_eq!(mode, ReportMode::Full);
    }

    #[test]
    fn mode_unchanged_overridden_by_env() {
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
    fn mode_unchanged_overridden_by_frontmatter() {
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

    // --- User prompt tests (legacy config shape) ---

    #[test]
    fn user_silent_suppresses_all() {
        let config = resolve_user_prompt_report_config(true, false, false, 10);
        assert!(!config.show_header);
        assert!(!config.show_body);
    }

    #[test]
    fn user_quiet_is_noop() {
        // Per spec 6.3: `--quiet` is a no-op for the user prompt; header
        // and body still render and length rules apply.
        let short = resolve_user_prompt_report_config(false, true, false, 10);
        assert!(short.show_header);
        assert!(short.show_body);
        assert_eq!(short.format, PromptReportFormat::FullPrompt);

        let long = resolve_user_prompt_report_config(false, true, false, 100);
        assert!(long.show_header);
        assert!(long.show_body);
        assert_eq!(long.format, PromptReportFormat::PartialPrompt);
    }

    #[test]
    fn user_verbose_forces_full() {
        let config = resolve_user_prompt_report_config(false, false, true, 200);
        assert!(config.show_header);
        assert!(config.show_body);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn user_short_prompt_shows_full() {
        let config = resolve_user_prompt_report_config(false, false, false, 40);
        assert!(config.show_header);
        assert!(config.show_body);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn user_long_prompt_uses_frontback() {
        let config = resolve_user_prompt_report_config(false, false, false, 41);
        assert!(config.show_header);
        assert!(config.show_body);
        assert_eq!(config.format, PromptReportFormat::PartialPrompt);
        assert_eq!(config.truncation, TruncationMode::FrontBack);
    }

    // --- Agent prompt resolver returning `ReportMode` ---

    #[test]
    fn agent_mode_silent() {
        assert_eq!(
            resolve_agent_prompt_report_mode(true, false, 10),
            ReportMode::Silent,
        );
    }

    #[test]
    fn agent_mode_verbose_forces_full() {
        assert_eq!(
            resolve_agent_prompt_report_mode(false, true, 200),
            ReportMode::Full,
        );
    }

    #[test]
    fn agent_mode_short_prompt_is_full() {
        assert_eq!(
            resolve_agent_prompt_report_mode(false, false, 40),
            ReportMode::Full,
        );
    }

    #[test]
    fn agent_mode_long_prompt_is_partial_frontback() {
        assert_eq!(
            resolve_agent_prompt_report_mode(false, false, 41),
            ReportMode::Partial {
                truncation: TruncationMode::FrontBack
            },
        );
    }
}
