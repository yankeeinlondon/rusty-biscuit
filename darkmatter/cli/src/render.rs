//! Terminal and page-style rendering entry points for the `md` CLI.
//!
//! Lowers a render's prepared style onto a [`DarkmatterPage`] and drives
//! terminal rendering. Page builders read only prepared values: the
//! effective style, the request's presentation defaults and document
//! context, and the output's terminal; never the parsed command line.

use crate::args::Feature;
use crate::io::OpenedArgument;
use crate::request::MdRequest;
use biscuit_file::FileResolutionContext;
use biscuit_terminal::output::OutputTarget;
use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::MermaidFallbackPolicy;
use color_eyre::eyre::{Result, eyre};
use darkmatter::layout::DarkmatterPage;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::MermaidMode;
use darkmatter::markdown::output::terminal::TerminalImageMode;
use darkmatter::style::{
    PreparedStyle, PresentationDefaults, StyleAssetResolver, StylePreparationPolicy, apply_prepared_style, prepare_style_layers,
};
use std::path::Path;

/// What a page build takes from the request rather than from the document.
#[derive(Debug, Clone)]
pub struct PageInputs {
    /// The opened document's own context, or the launch context for standard
    /// input: the context its `style:` layer is prepared in, so a document
    /// `style.page.stylesheet` resolves there.
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

/// What a page shows besides the document: its effective style and the
/// render's behavior switches.
#[derive(Debug, Clone, Copy)]
pub struct PagePresentation<'a> {
    pub style: &'a PreparedStyle,
    pub features: &'a [Feature],
    /// `md render --mermaid`: automatic Mermaid mode.
    pub mermaid: bool,
}

/// Build the page that renders terminal output on `term` (the output's own
/// terminal, `Terminal::from_output_options`), with page-relative assets
/// beside `input_path`.
///
/// ## Errors
///
/// The style cannot be applied.
pub fn terminal_page(
    input_path: Option<&Path>,
    inputs: &PageInputs,
    presentation: &PagePresentation<'_>,
    term: &Terminal,
) -> Result<DarkmatterPage> {
    // The color mode comes from the `Terminal` that will render the page, so
    // the page surface and the nested code-block panel share one source.
    let mut page = DarkmatterPage::new(term)
        .with_color_mode(term.color_mode())
        .with_image_mode(terminal_image_mode_from_env())
        .with_mermaid_mode(if presentation.mermaid { MermaidMode::Automatic } else { MermaidMode::Off });

    if let Some(path) = input_path
        && path.to_str() != Some("-")
    {
        page = page.with_base_path(path.parent().map(Path::to_path_buf).unwrap_or_default());
    }

    let page = apply_presentation(page, inputs, presentation, OutputTarget::Terminal)?;
    Ok(apply_features(page, presentation.features))
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

/// Lowers `presentation`'s effective style onto `page` for `target`.
///
/// Theme, code-block mode, and line numbers resolve over the merged style
/// from the request's presentation defaults. A local stylesheet is read only
/// for HTML and MarkdownPlus, in the context of the layer that supplied it.
///
/// ## Errors
///
/// A style value the page cannot apply, or a stylesheet that cannot be read.
pub fn apply_presentation(
    page: DarkmatterPage,
    inputs: &PageInputs,
    presentation: &PagePresentation<'_>,
    target: OutputTarget,
) -> Result<DarkmatterPage> {
    apply_prepared_style(page, presentation.style, StyleAssetResolver::for_target(target), &inputs.presentation)
        .map_err(|error| eyre!("Failed to apply the style: {error}"))
}

/// The document's own style, with no CLI layer: what `md clean` and
/// `md set` render their result with. Its warnings are left for the caller.
///
/// ## Errors
///
/// The document's `style:` cannot be prepared.
pub fn document_style(md: &Markdown, path: Option<&Path>, context: FileResolutionContext) -> Result<PreparedStyle> {
    let layer = crate::render_style::document_style_layer(md, path, context);
    prepare_style_layers(&[layer], None, StylePreparationPolicy::render()).map_err(|error| eyre!("Failed to prepare the style: {error}"))
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

    #[test]
    fn deny_defaults_to_nothing_and_takes_style() {
        let parse = |args: &[&str]| crate::args::Cli::try_parse_from(args).expect("CLI should parse");
        assert!(parse(&["md", "doc.md"]).render.deny.is_empty());
        let cli = parse(&["md", "doc.md", "--deny", "style"]);
        let policy = crate::args::policy::resolve_for(crate::args::policy::PolicyCommand::Render, &[], &cli.render.deny, &[], None)
            .expect("resolves");
        assert!(policy.denies("style"));
    }

    fn test_inputs() -> PageInputs {
        PageInputs::new(FileResolutionContext::new(std::env::temp_dir()), PresentationDefaults::default())
    }

    fn html_page_of(raw: &str) -> Result<DarkmatterPage> {
        let md = Markdown::try_from_content(raw).unwrap();
        let style = document_style(&md, None, FileResolutionContext::new(std::env::temp_dir()))?;
        let presentation = PagePresentation { style: &style, features: &[], mermaid: false };
        crate::artifact::html_page(&test_inputs(), &presentation)
    }

    #[test]
    fn a_remote_stylesheet_in_the_document_reaches_the_html_page() {
        let raw = "---\nstyle:\n    page:\n        stylesheet: https://example.com/main.css\n---\n\n# Doc\n";
        let page = html_page_of(raw).expect("apply");
        assert!(page.stylesheet().is_some(), "the stylesheet is applied");
    }
}
