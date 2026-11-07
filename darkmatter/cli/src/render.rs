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
use darkmatter::markdown::compose::policy::{STYLE_DEPRECATED_CODE, STYLE_UNKNOWN_KEY_CODE};
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::style::{
    PresentationDefaults, StyleAssetResolver, StyleFrontmatter, StyleWarning, StyleWarningKind, apply_bespoke_style, apply_cli_claims, apply_color_style,
    apply_component_style, apply_disclosure_style, apply_hr_style, apply_list_style,
    apply_page_style, bespoke_style_overrides_from_claims,
    component_style_overrides_from_claims, disclosure_style_overrides_from_claims,
    from_frontmatter, hr_style_overrides_from_claims, into_strict,
    list_style_overrides_from_claims, page_style_overrides_from_claims,
};
use crate::io::OpenedArgument;
use crate::request::MdRequest;
use biscuit_file::FileResolutionContext;
use biscuit_terminal::output::OutputTarget;
use std::path::{Path, PathBuf};

/// What a page build takes from the request rather than from the document.
#[derive(Debug, Clone)]
pub struct PageInputs {
    /// The context the document's `style.page.stylesheet` resolves in: the
    /// opened document's own, or the launch context for standard input.
    pub document_context: FileResolutionContext,
    /// The `THEME` and `CODE_THEME` values the request captured.
    pub presentation: PresentationDefaults,
}

impl PageInputs {
    pub fn new(document_context: FileResolutionContext, presentation: PresentationDefaults) -> Self {
        Self { document_context, presentation }
    }

    /// The inputs of a document opened through `opened`, or of standard input.
    ///
    /// ## Errors
    ///
    /// The document or launch context cannot be built.
    pub fn for_opened(request: &MdRequest, opened: Option<&OpenedArgument>) -> Result<Self> {
        let document_context = match opened {
            Some(opened) => opened.document_context(request)?,
            None => request.launch_context()?.clone(),
        };
        Ok(Self::new(document_context, PresentationDefaults::from_snapshot(request.snapshot())))
    }

    /// The inputs of the document at `resolved`, a file the request already
    /// opened (its opening spelling is gone), or of standard input.
    ///
    /// ## Errors
    ///
    /// The document or launch context cannot be built.
    pub fn for_resolved(request: &MdRequest, resolved: Option<&Path>) -> Result<Self> {
        let document_context = match resolved {
            Some(path) => request.document_context(None, path)?,
            None => request.launch_context()?.clone(),
        };
        Ok(Self::new(document_context, PresentationDefaults::from_snapshot(request.snapshot())))
    }
}

/// Build the page that renders terminal output: CLI flags, `style:`
/// frontmatter, then `features`. `mermaid` (`md render --mermaid`) selects
/// automatic Mermaid mode.
#[allow(clippy::too_many_arguments, reason = "the render dispatch rewrite replaces these inputs with prepared values")]
pub fn terminal_page(
    md: &Markdown,
    input_path: Option<&PathBuf>,
    inputs: &PageInputs,
    cli: &Cli,
    term: &Terminal,
    strict_style: bool,
    features: &[Feature],
    mermaid: bool,
) -> Result<DarkmatterPage> {
    // The color mode comes from the `Terminal` that will render the page, so
    // the page surface and the nested code-block panel share one source.
    let mut page = DarkmatterPage::new(term)
        .with_color_mode(term.color_mode())
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if mermaid {
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
    page = apply_style_frontmatter(page, md, cli, inputs, OutputTarget::Terminal, strict_style)?;

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
    inputs: &PageInputs,
    target: OutputTarget,
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

    // Theme, code-block mode, and line numbers resolve after the flags and
    // the document merge, so a flag still wins and an omitted one leaves the
    // document's value.
    let page = inputs.presentation.resolve(&with_cli_presentation(&style, cli)).apply_to(page);

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
    apply_bespoke_style(
        page,
        &style,
        bespoke_overrides,
        StyleAssetResolver::for_target(target),
        &inputs.document_context,
    )
    .map_err(|e| eyre!("Failed to apply `style:` frontmatter: {e}"))
}

/// `style` with the presentation flags (`--theme`, `--code-theme`,
/// `--line-numbers`) written over it, as the top layer.
fn with_cli_presentation(style: &StyleFrontmatter, cli: &Cli) -> StyleFrontmatter {
    let mut style = style.clone();
    let page = style.page.get_or_insert_with(Default::default);
    if let Some(theme) = cli.theme {
        page.theme = Some(theme);
    }
    if let Some(theme) = cli.code_theme {
        page.code.get_or_insert_with(Default::default).theme = Some(theme);
    }
    if let Some(on) = cli.line_numbers {
        style.code_block.get_or_insert_with(Default::default).line_numbers = Some(on);
    }
    style
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

/// The stable code and message of a warning a reader should see, or `None` for
/// a notice that stays in the debug log.
fn style_warning_report(warning: &StyleWarning) -> Option<(&'static str, String)> {
    match &warning.kind {
        StyleWarningKind::UnknownKey => Some((
            STYLE_UNKNOWN_KEY_CODE,
            match &warning.suggestion {
                Some(key) => format!("unknown style key `{}`; use `{key}`", warning.path),
                None => format!("unknown style key `{}`", warning.path),
            },
        )),
        StyleWarningKind::Deprecated { replacement } => Some((
            STYLE_DEPRECATED_CODE,
            format!("deprecated style key `{}`; use `{replacement}`", warning.path),
        )),
        StyleWarningKind::KnownButInactive { .. } => None,
    }
}

/// Fails when `--deny <code>` named the code of one of `md`'s `style:` warnings.
///
/// `--deny style` is the whole family and is judged where the page's style is
/// read; this judges a single code. Unparseable `style:` frontmatter reports
/// nothing here, as in [`style_diagnostics`].
///
/// ## Errors
///
/// The first denied warning.
pub fn stop_if_style_code_denied(md: &Markdown, blocking: &darkmatter::markdown::compose::policy::BlockingPolicy) -> Result<()> {
    use darkmatter::markdown::compose::expression::Severity;
    let Ok((_, warnings)) = from_frontmatter(md.frontmatter()) else {
        return Ok(());
    };
    let (issues, _) = denied_style_issues(warnings, md);
    match issues
        .iter()
        .filter_map(style_warning_report)
        .find(|(code, _)| blocking.blocks(Some(code), Severity::Warning))
    {
        Some((code, message)) => Err(eyre!("`style:` frontmatter rejected by --deny {code}: {message}")),
        None => Ok(()),
    }
}

/// The `style:` warnings of `md` as a data output's diagnostics.
///
/// A data output builds no page, but it reports what a page would. Unparseable
/// `style:` frontmatter is no diagnostic here: the document it carries is the
/// source text, which never reads it.
///
/// ## Errors
///
/// `strict_style` (`--deny style`) refuses a schema issue, as a page does.
pub fn style_diagnostics(md: &Markdown, strict_style: bool) -> Result<Vec<darkmatter::markdown::compose::expression::Diagnostic>> {
    let Ok((style, warnings)) = from_frontmatter(md.frontmatter()) else {
        return Ok(Vec::new());
    };
    if strict_style {
        let (schema, _) = denied_style_issues(warnings.clone(), md);
        into_strict((style, schema)).context("`style:` frontmatter rejected by --deny style")?;
    }
    use darkmatter::markdown::compose::expression::Severity;
    Ok(warnings
        .iter()
        .filter_map(style_warning_report)
        .map(|(code, message)| crate::output::output_diagnostic(code, Severity::Warning, &message))
        .collect())
}

/// Reports non-fatal style warnings on stderr; `KnownButInactive` notices stay
/// in the debug log. Warnings `--deny style` promoted never reach here.
fn log_style_warnings(warnings: &[StyleWarning]) {
    for w in warnings {
        if let Some((_, message)) = style_warning_report(w) {
            crate::output::stderr_warning(&message);
        }
        match &w.kind {
            StyleWarningKind::UnknownKey | StyleWarningKind::Deprecated { .. } => {}
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

    #[test]
    fn unknown_style_key_report_names_the_suggested_key() {
        let renamed = StyleWarning::new("style.page.left-margin", StyleWarningKind::UnknownKey)
            .with_suggestion("page.margin-left");
        let (code, message) = style_warning_report(&renamed).expect("reported");
        assert_eq!(code, STYLE_UNKNOWN_KEY_CODE);
        assert_eq!(message, "unknown style key `style.page.left-margin`; use `page.margin-left`");

        let typo = StyleWarning::new("style.page.bogus", StyleWarningKind::UnknownKey);
        assert_eq!(style_warning_report(&typo).expect("reported").1, "unknown style key `style.page.bogus`");
    }

    // -----------------------------------------------------------------
    // Strict-style preserves informational `KnownButInactive` warnings
    // -----------------------------------------------------------------

    /// Build a `DarkmatterPage` at a deterministic width for warning-log tests.
    fn test_page() -> DarkmatterPage {
        DarkmatterPage::new(&Terminal::new_optimistic(80))
    }

    fn test_inputs() -> PageInputs {
        PageInputs::new(FileResolutionContext::new(std::env::temp_dir()), PresentationDefaults::default())
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
        apply_style_frontmatter(test_page(), &md, &cli, &test_inputs(), OutputTarget::Html, true)
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
        let page = apply_style_frontmatter(test_page(), &md, &cli, &test_inputs(), OutputTarget::Html, false).expect("apply");
        assert!(page.stylesheet().is_some(), "sub-spec #7 key should be applied");
    }
}
