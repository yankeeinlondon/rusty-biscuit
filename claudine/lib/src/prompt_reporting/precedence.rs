//! Precedence logic for resolving prompt reporting configuration.
//!
//! The precedence chain (highest to lowest) is:
//! 1. CLI switches (`--silent`, `--quiet`, `--verbose`)
//! 2. `CLAUDINE_SYSTEM_PROMPT` environment variable
//! 3. Prompt length (< 10 lines → `FullPrompt`)
//! 4. Frontmatter `verbosity` property
//! 5. Default (`Summary`)

use super::types::{PromptReportFormat, PromptVerbosity, SystemPromptReportConfig, TruncationMode};

/// Resolve the reporting configuration for a system prompt.
///
/// Applies the precedence chain documented in the module-level description.
///
/// ## Arguments
///
/// - `cli_silent`: whether `--silent` was passed.
/// - `cli_quiet`: whether `--quiet` was passed.
/// - `cli_verbose`: whether `--verbose` was passed (any count > 0).
/// - `env_verbosity`: value of `CLAUDINE_SYSTEM_PROMPT`, if set and valid.
/// - `prompt_line_count`: number of lines in the composed prompt.
/// - `frontmatter_verbosity`: value parsed from the prompt file's frontmatter.
///
/// ## Returns
///
/// A [`SystemPromptReportConfig`] describing what to render.
///
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::{
///     resolve_system_prompt_report_config, PromptReportFormat, SystemPromptReportConfig,
///     TruncationMode,
/// };
///
/// let config = resolve_system_prompt_report_config(
///     false, // silent
///     false, // quiet
///     true,  // verbose
///     None,
///     5,
///     None,
/// );
/// assert_eq!(config.format, PromptReportFormat::FullPrompt);
/// ```
pub fn resolve_system_prompt_report_config(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    env_verbosity: Option<PromptVerbosity>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<PromptVerbosity>,
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
    env_verbosity: Option<PromptVerbosity>,
    prompt_line_count: usize,
    frontmatter_verbosity: Option<PromptVerbosity>,
    prompt_unchanged: bool,
) -> SystemPromptReportConfig {
    // 1. CLI switches always win.
    if cli_silent {
        return SystemPromptReportConfig {
            show_header: false,
            show_summary: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        };
    }
    if cli_quiet {
        return SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        };
    }
    if cli_verbose {
        return SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
    }

    // 2. Environment variable.
    if let Some(verbosity) = env_verbosity {
        return config_from_verbosity(verbosity);
    }

    // 3. Prompt length heuristic.
    if prompt_line_count < 10 {
        return SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
    }

    // 4. Frontmatter hint.
    if let Some(verbosity) = frontmatter_verbosity {
        return config_from_verbosity(verbosity);
    }

    // 5. Default.
    //
    // Per spec: when no override has been selected, suppress the entire
    // report if the composed prompt has not changed since the last
    // observed run at this scope.
    if prompt_unchanged {
        return SystemPromptReportConfig {
            show_header: false,
            show_summary: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        };
    }

    SystemPromptReportConfig {
        show_header: true,
        show_summary: true,
        format: PromptReportFormat::Summary,
        truncation: TruncationMode::FrontBack,
    }
}

/// Convert a [`PromptVerbosity`] into a full [`SystemPromptReportConfig`].
fn config_from_verbosity(verbosity: PromptVerbosity) -> SystemPromptReportConfig {
    match verbosity {
        PromptVerbosity::Silent => SystemPromptReportConfig {
            show_header: false,
            show_summary: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        PromptVerbosity::Quiet => SystemPromptReportConfig {
            show_header: true,
            show_summary: true,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        },
        PromptVerbosity::Verbose => SystemPromptReportConfig {
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
/// - `--quiet` suppresses everything.
/// - `--verbose` forces the full body.
/// - By default, the full body is shown when ≤ 40 lines.
/// - When > 40 lines, `FrontBack` truncation is used.
pub fn resolve_user_prompt_report_config(
    cli_silent: bool,
    cli_quiet: bool,
    cli_verbose: bool,
    prompt_line_count: usize,
) -> super::types::UserPromptReportConfig {
    use super::types::UserPromptReportConfig;

    if cli_silent || cli_quiet {
        return UserPromptReportConfig {
            show_header: false,
            show_body: false,
            format: PromptReportFormat::Summary,
            truncation: TruncationMode::FrontBack,
        };
    }

    if cli_verbose {
        return UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        };
    }

    if prompt_line_count <= 40 {
        UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::FullPrompt,
            truncation: TruncationMode::FrontBack,
        }
    } else {
        UserPromptReportConfig {
            show_header: true,
            show_body: true,
            format: PromptReportFormat::PartialPrompt,
            truncation: TruncationMode::FrontBack,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::PromptVerbosity;

    // --- System prompt precedence tests ---

    #[test]
    fn cli_silent_wins_over_everything() {
        let config = resolve_system_prompt_report_config(
            true,
            false,
            false,
            Some(PromptVerbosity::Verbose),
            5,
            Some(PromptVerbosity::Verbose),
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
            Some(PromptVerbosity::Verbose),
            5,
            Some(PromptVerbosity::Verbose),
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
            Some(PromptVerbosity::Quiet),
            200,
            Some(PromptVerbosity::Quiet),
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn env_verbose_used_when_no_cli_flags() {
        let config = resolve_system_prompt_report_config(
            false, false, false,
            Some(PromptVerbosity::Verbose),
            200,
            Some(PromptVerbosity::Quiet),
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn env_silent_used_when_no_cli_flags() {
        let config = resolve_system_prompt_report_config(
            false, false, false,
            Some(PromptVerbosity::Silent),
            5,
            Some(PromptVerbosity::Verbose),
        );
        assert!(!config.show_header);
    }

    #[test]
    fn prompt_length_short_circuits_frontmatter() {
        // Even with frontmatter verbose, a short prompt (< 10 lines)
        // should trigger FullPrompt … wait, no: env comes before prompt
        // length, but frontmatter comes AFTER prompt length.
        let config = resolve_system_prompt_report_config(
            false, false, false,
            None,
            5,
            Some(PromptVerbosity::Quiet),
        );
        // Prompt length (< 10) should win over frontmatter.
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn frontmatter_used_when_no_other_hints() {
        let config = resolve_system_prompt_report_config(
            false, false, false,
            None,
            50,
            Some(PromptVerbosity::Verbose),
        );
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn default_is_summary() {
        let config = resolve_system_prompt_report_config(
            false, false, false,
            None,
            50,
            None,
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    #[test]
    fn unchanged_default_suppresses_header() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, false,
            None,
            50,
            None,
            true, // unchanged
        );
        assert!(!config.show_header);
        assert!(!config.show_summary);
    }

    #[test]
    fn unchanged_overridden_by_verbose() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, true,
            None,
            50,
            None,
            true, // unchanged but verbose flag wins
        );
        assert!(config.show_header);
        assert_eq!(config.format, PromptReportFormat::FullPrompt);
    }

    #[test]
    fn unchanged_overridden_by_env() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, false,
            Some(PromptVerbosity::Quiet),
            50,
            None,
            true,
        );
        assert!(config.show_header);
    }

    #[test]
    fn unchanged_overridden_by_frontmatter() {
        let config = resolve_system_prompt_report_config_with_change(
            false, false, false,
            None,
            50,
            Some(PromptVerbosity::Verbose),
            true,
        );
        assert!(config.show_header);
    }

    #[test]
    fn long_prompt_with_no_hints_defaults_to_summary() {
        let config = resolve_system_prompt_report_config(
            false, false, false,
            None,
            100,
            None,
        );
        assert_eq!(config.format, PromptReportFormat::Summary);
    }

    // --- User prompt tests ---

    #[test]
    fn user_silent_suppresses_all() {
        let config = resolve_user_prompt_report_config(true, false, false, 10);
        assert!(!config.show_header);
        assert!(!config.show_body);
    }

    #[test]
    fn user_quiet_suppresses_all() {
        let config = resolve_user_prompt_report_config(false, true, false, 10);
        assert!(!config.show_header);
        assert!(!config.show_body);
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
}
