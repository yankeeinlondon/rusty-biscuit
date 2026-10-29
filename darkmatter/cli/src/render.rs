//! Terminal and page-style rendering entry points for the `md` CLI.
//!
//! Applies CLI layout claims and `style:` frontmatter to a [`DarkmatterPage`]
//! and drives terminal rendering.

use crate::args::{Cli, Feature};
use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::MermaidFallbackPolicy;
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

/// Render Markdown to the terminal using CLI flags, `style:` frontmatter, and
/// `features`.
pub fn render_terminal_output(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    term: Terminal,
    strict_style: bool,
    features: &[Feature],
) -> Result<()> {
    let output = terminal_page(md, input_path, cli, &term, strict_style, features)?
        .render(md)
        .context("Failed to render markdown for terminal")?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(output.as_bytes())
        .context("Failed to write terminal output")?;

    Ok(())
}

/// Build the page that renders terminal output: CLI flags (`--mermaid` selects
/// automatic Mermaid mode), `style:` frontmatter, then `features`.
pub fn terminal_page(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    cli: &Cli,
    term: &Terminal,
    strict_style: bool,
    features: &[Feature],
) -> Result<DarkmatterPage> {
    // Resolve the theme from the same `Terminal` that will render the page.
    // This keeps the page surface and the nested code-block panel aligned on
    // a single `color_mode` source of truth.
    let theme = ResolvedTheme::from_cli(cli, term);

    let mut page = DarkmatterPage::new(term)
        .with_prose_theme(theme.prose.kebab_name())
        .with_code_theme(theme.code.kebab_name())
        .with_color_mode(theme.color_mode)
        .with_code_block_mode(cli.code_block.into())
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if cli.mermaid {
            MermaidMode::Automatic
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
    page = apply_style_frontmatter(page, md, cli, input_path, strict_style)?;

    // Handle line numbers: CLI flag overrides default.
    if let Some(on) = cli.line_numbers {
        page = page.with_line_numbers(on);
    }

    Ok(apply_features(page, features))
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

/// Apply each `--feature` to `page`.
///
/// Order and repetition among features do not matter. Call this after the
/// page's Mermaid mode is set: `no-mermaid-rendering` replaces it with code.
/// The fallback features fold into the page's Mermaid fallback policy, which
/// only automatic mode (`--mermaid`) consults; with none of them the policy
/// stays unset, so `MERMAID_FALLBACK` applies.
pub fn apply_features(page: DarkmatterPage, features: &[Feature]) -> DarkmatterPage {
    let policy = MermaidFallbackPolicy::from_features(
        features.iter().filter_map(|feature| feature.mermaid_fallback()),
    );
    features
        .iter()
        .fold(page, |page, feature| match feature {
            Feature::EmbedMermaidFont => page.with_embed_mermaid_font(true),
            Feature::NoMermaidRendering => page.with_mermaid_mode(MermaidMode::Text),
            Feature::NoMermaidFallbacks
            | Feature::NoMermaidPartialFallbacks
            | Feature::NoMermaidPartialWarning => page,
        })
        .with_mermaid_fallback_policy(policy)
}

/// Parse the `style:` frontmatter (if any), promote schema warnings to errors
/// when `strict_style` (`--deny style` or `--deny all`) is set, log remaining
/// warnings, and apply the page-level subset to `page` using the CLI's
/// override summary.
pub fn apply_style_frontmatter(
    page: DarkmatterPage,
    md: &Markdown,
    cli: &Cli,
    input_path: Option<&PathBuf>,
    strict_style: bool,
) -> Result<DarkmatterPage> {
    let (style, all_warnings) =
        from_frontmatter(md.frontmatter()).context("Failed to parse `style:` frontmatter")?;

    // Denying `style` promotes schema issues (UnknownKey / Deprecated) to
    // errors, but informational `KnownButInactive` warnings must still flow
    // through `log_style_warnings` so `RUST_LOG=darkmatter=info` users see
    // future-phase keys regardless of strict mode.
    let (style, warnings) = if strict_style {
        let (schema, informational) = denied_style_issues(all_warnings, md);
        let style = into_strict((style, schema))
            .context("`style:` frontmatter rejected by --deny style")?;
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

/// Splits `warnings` under a denied `style` into the issues that block (the
/// schema issues plus the body's inline horizontal-rule deprecations) and the
/// informational notices, which never block.
fn denied_style_issues(warnings: Vec<StyleWarning>, md: &Markdown) -> (Vec<StyleWarning>, Vec<StyleWarning>) {
    let (mut schema, informational): (Vec<_>, Vec<_>) =
        warnings.into_iter().partition(StyleWarning::is_schema_issue);
    schema.extend(scan_inline_hr_warnings(md.content()));
    (schema, informational)
}

/// Log non-fatal style warnings via `tracing`.
///
/// `KnownButInactive` is informational; `UnknownKey` and `Deprecated` are
/// warnings (but only when not promoted to errors by `--deny style`).
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
    fn deny_defaults_to_nothing_and_takes_style() {
        assert!(cli_from(&["md", "doc.md"]).render.deny.is_empty());
        let cli = cli_from(&["md", "doc.md", "--deny", "style"]);
        let policy = crate::args::policy::resolve_for(crate::args::policy::PolicyCommand::Render, &[], &cli.render.deny, &[], None)
            .expect("resolves");
        assert!(policy.denies("style"));
    }

    #[test]
    fn a_denied_style_blocks_schema_issues_and_inline_rules_but_never_a_notice() {
        let md = Markdown::try_from_content("# Doc\n\n--- { style: waves }\n").unwrap();
        let warnings = vec![
            StyleWarning::new("style.page.bogus", StyleWarningKind::UnknownKey),
            StyleWarning::new("style.page.later", StyleWarningKind::KnownButInactive { sub_spec: 99 }),
        ];
        let (blocking, informational) = denied_style_issues(warnings, &md);
        assert!(blocking.iter().any(|w| w.path == "style.page.bogus"), "{blocking:?}");
        assert!(blocking.iter().any(|w| matches!(w.kind, StyleWarningKind::Deprecated { .. })), "{blocking:?}");
        assert_eq!(informational.len(), 1);
        assert!(matches!(informational[0].kind, StyleWarningKind::KnownButInactive { .. }));
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
        // `KnownButInactive` warning. `--deny style` must still succeed
        // because the document is schema-clean.
        let raw = "---\n\
style:\n\
\x20   page:\n\
\x20       stylesheet: https://example.com/main.css\n\
---\n\n# Doc\n";
        let md = Markdown::try_from_content(raw).unwrap();
        let cli = cli_from(&["md", "doc.md"]);
        apply_style_frontmatter(test_page(), &md, &cli, None, true)
            .expect("a denied style must succeed on a schema-clean wired key");
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
        let page = apply_style_frontmatter(test_page(), &md, &cli, None, false).expect("apply");
        assert!(page.stylesheet().is_some(), "sub-spec #7 key should be applied");
    }
}
