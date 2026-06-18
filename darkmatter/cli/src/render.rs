//! Terminal and page-style rendering entry points for the `md` CLI.
//!
//! Applies CLI layout claims and `style:` frontmatter to a [`DarkmatterPage`]
//! and drives terminal rendering.

use crate::args::Cli;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::block::scan_inline_hr_warnings;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair, detect_code_theme, detect_prose_theme};
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::style::{
    StyleWarning, StyleWarningKind, apply_bespoke_style, apply_cli_claims, apply_color_style,
    apply_component_style, apply_disclosure_style, apply_hr_style, apply_list_style,
    apply_page_style, bespoke_style_overrides_from_claims,
    component_style_overrides_from_claims, disclosure_style_overrides_from_claims,
    from_frontmatter, hr_style_overrides_from_claims, into_strict,
    list_style_overrides_from_claims, page_style_overrides_from_claims,
};
use std::io::{self, Write};
use std::path::PathBuf;

/// Resolved theme configuration for an output path that needs one.
///
/// The page surface and the nested code-block panel must share a single
/// source of truth. The `color_mode` is always taken from the [`Terminal`]
/// that will actually render the page, so the prose/code themes and the
/// layout context cannot drift.
#[derive(Debug)]
pub(crate) struct ResolvedTheme {
    pub(crate) prose: ThemePair,
    pub(crate) code: ThemePair,
    pub(crate) color_mode: ColorMode,
}

impl ResolvedTheme {
    pub(crate) fn from_cli(cli: &Cli, terminal: &Terminal) -> Self {
        let prose = cli.theme.unwrap_or_else(detect_prose_theme);
        let code = cli.code_theme.unwrap_or_else(|| detect_code_theme(prose));
        let color_mode = terminal.color_mode();
        Self {
            prose,
            code,
            color_mode,
        }
    }
}

/// Render Markdown to the terminal using CLI flags and `style:` frontmatter.
pub fn render_terminal_output(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    term: Terminal,
) -> Result<()> {
    // Resolve the theme from the same `Terminal` that will render the page.
    // This keeps the page surface and the nested code-block panel aligned on
    // a single `color_mode` source of truth.
    let theme = ResolvedTheme::from_cli(cli, &term);

    let mut page = DarkmatterPage::new(&term)
        .with_prose_theme(theme.prose.kebab_name())
        .with_code_theme(theme.code.kebab_name())
        .with_color_mode(theme.color_mode)
        .with_code_block_mode(cli.code_block.into())
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if cli.mermaid {
            MermaidMode::Image
        } else {
            MermaidMode::Off
        });

    if let Some(path) = input_path
        && path.to_str() != Some("-")
    {
        page = page.with_base_path(path.parent().map(|p| p.to_path_buf()).unwrap_or_default());
    }

    // Apply layout flags from CLI.
    page = apply_cli_layout_flags(page, cli);

    // Apply page-level frontmatter style after CLI flags so CLI wins on
    // overlapping fields via PageStyleOverrides.
    page = apply_style_frontmatter(page, md, cli, input_path)?;

    // Handle line numbers: CLI flag overrides default.
    if let Some(on) = cli.line_numbers {
        page = page.with_line_numbers(on);
    }

    let output = page
        .render(md)
        .context("Failed to render markdown for terminal")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(output.as_bytes())
        .context("Failed to write terminal output")?;

    Ok(())
}

/// Apply CLI layout flags to a [`DarkmatterPage`].
///
/// This delegates to [`darkmatter::style::apply_cli_claims`] after lowering the
/// parsed CLI into a neutral [`CliStyleClaims`] value. Precedence is:
/// margin/padding shorthand → axis → side; alignment/fill global →
/// component-specific.
pub fn apply_cli_layout_flags(page: DarkmatterPage, cli: &Cli) -> DarkmatterPage {
    let claims = crate::style_claims::cli_style_claims(cli);
    apply_cli_claims(page, &claims)
}

/// Parse the `style:` frontmatter (if any), promote schema warnings to errors
/// when `--strict-style` is set, log remaining warnings, and apply the
/// page-level subset to `page` using the CLI's override summary.
pub fn apply_style_frontmatter(
    page: DarkmatterPage,
    md: &Markdown,
    cli: &Cli,
    input_path: Option<&PathBuf>,
) -> Result<DarkmatterPage> {
    let (style, all_warnings) =
        from_frontmatter(md.frontmatter()).context("Failed to parse `style:` frontmatter")?;

    // `--strict-style` promotes schema issues (UnknownKey / Deprecated) to
    // errors, but informational `KnownButInactive` warnings must still flow
    // through `log_style_warnings` so `RUST_LOG=darkmatter=info` users see
    // future-phase keys regardless of strict mode.
    let (style, warnings) = if cli.strict_style {
        let (mut schema, informational): (Vec<_>, Vec<_>) = all_warnings
            .into_iter()
            .partition(StyleWarning::is_schema_issue);
        // Also reject inline HR deprecation warnings in strict mode.
        let inline_hr_warnings = scan_inline_hr_warnings(md.content());
        schema.extend(inline_hr_warnings);
        let style = into_strict((style, schema))
            .context("`style:` frontmatter rejected by --strict-style")?;
        (style, informational)
    } else {
        (style, all_warnings)
    };

    log_style_warnings(&warnings);

    let claims = crate::style_claims::cli_style_claims(cli);

    let page = apply_page_style(page, &style, page_style_overrides_from_claims(&claims))
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_component_style(page, &style, component_style_overrides_from_claims(&claims))
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_list_style(page, &style, list_style_overrides_from_claims(&claims))
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_hr_style(page, &style, hr_style_overrides_from_claims(&claims))
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_disclosure_style(page, &style, disclosure_style_overrides_from_claims(&claims))
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let page = apply_color_style(page, &style)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))?;

    let bespoke_overrides = bespoke_style_overrides_from_claims(&claims);
    let source_path = input_path
        .filter(|p| p.to_str() != Some("-"))
        .map(|p| p.as_path());
    apply_bespoke_style(page, &style, bespoke_overrides, source_path)
        .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))
}

/// Log non-fatal style warnings via `tracing`.
///
/// `KnownButInactive` is informational; `UnknownKey` and `Deprecated` are
/// warnings (but only when not promoted to errors by `--strict-style`).
fn log_style_warnings(warnings: &[StyleWarning]) {
    for w in warnings {
        match &w.kind {
            StyleWarningKind::UnknownKey => {
                tracing::warn!(path = %w.path, "unknown style key");
            }
            StyleWarningKind::Deprecated { replacement } => {
                tracing::warn!(
                    path = %w.path,
                    replacement = %replacement,
                    "deprecated style key",
                );
            }
            StyleWarningKind::KnownButInactive { sub_spec } => {
                tracing::info!(
                    path = %w.path,
                    sub_spec = sub_spec,
                    "style key parsed but not yet wired",
                );
            }
        }
    }
}

/// Resolve the `TERMINAL_IMAGES` environment variable into a
/// [`TerminalImageMode`].
pub fn terminal_image_mode_from_env() -> TerminalImageMode {
    let Ok(raw) = std::env::var("TERMINAL_IMAGES") else {
        return TerminalImageMode::Auto;
    };

    match parse_bool_env(&raw) {
        Some(true) => TerminalImageMode::Force,
        Some(false) => TerminalImageMode::Never,
        None => {
            tracing::warn!(value = %raw, "Invalid TERMINAL_IMAGES value; falling back to auto mode");
            TerminalImageMode::Auto
        }
    }
}

/// Parse a boolean-ish environment variable value.
pub fn parse_bool_env(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "y" => Some(true),
        "0" | "false" | "no" | "off" | "n" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parse_bool_env_supports_truthy_values() {
        for value in ["1", "true", "TRUE", "yes", "on", "y"] {
            assert_eq!(parse_bool_env(value), Some(true), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_supports_falsy_values() {
        for value in ["0", "false", "FALSE", "no", "off", "n"] {
            assert_eq!(parse_bool_env(value), Some(false), "value: {value}");
        }
    }

    #[test]
    fn parse_bool_env_rejects_unknown_values() {
        for value in ["", "maybe", "2", "enable", "disable"] {
            assert_eq!(parse_bool_env(value), None, "value: {value}");
        }
    }

    fn cli_from(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("CLI should parse")
    }

    #[test]
    fn strict_style_flag_defaults_false() {
        let cli = cli_from(&["md", "doc.md"]);
        assert!(!cli.strict_style);
    }

    #[test]
    fn strict_style_flag_parses() {
        let cli = cli_from(&["md", "doc.md", "--strict-style"]);
        assert!(cli.strict_style);
    }

    // -----------------------------------------------------------------
    // Strict-style preserves informational `KnownButInactive` warnings
    // -----------------------------------------------------------------

    /// Build a `DarkmatterPage` at a deterministic width for warning-log tests.
    fn test_page() -> DarkmatterPage {
        DarkmatterPage::new(&Terminal::new_optimistic(80))
    }

    #[test]
    #[tracing_test::traced_test]
    fn strict_style_succeeds_on_schema_clean_sub_spec_7_key() {
        // After sub-spec #7, `page.stylesheet` is wired and produces no
        // `KnownButInactive` warning. `--strict-style` must still succeed
        // because the document is schema-clean.
        let raw = "---\n\
style:\n\
\x20   page:\n\
\x20       stylesheet: https://example.com/main.css\n\
---\n\n# Doc\n";
        let md = Markdown::try_from_content(raw).unwrap();
        let cli = cli_from(&["md", "doc.md", "--strict-style"]);
        apply_style_frontmatter(test_page(), &md, &cli, None)
            .expect("strict-style must succeed on schema-clean wired key");
    }

    #[test]
    #[tracing_test::traced_test]
    fn non_strict_style_applies_sub_spec_7_key_silently() {
        // After sub-spec #7, `page.stylesheet` is wired and silent.
        let raw = "---\n\
style:\n\
\x20   page:\n\
\x20       stylesheet: https://example.com/main.css\n\
---\n\n# Doc\n";
        let md = Markdown::try_from_content(raw).unwrap();
        let cli = cli_from(&["md", "doc.md"]);
        let page = apply_style_frontmatter(test_page(), &md, &cli, None).expect("apply");
        assert!(page.stylesheet().is_some(), "sub-spec #7 key should be applied");
    }
}
