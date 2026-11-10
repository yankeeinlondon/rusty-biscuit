//! Pages built in process the way `md` renders one: the arguments go through
//! clap and render preparation, the `style…=VALUE` setters and `--style` files
//! are prepared over the document's own `style:`, and the effective style is
//! lowered with `render::apply_presentation`.
//!
//! Every request is anchored in a fresh temporary directory, so no ambient
//! schema, home, or theme variable reaches the page.

use biscuit_terminal::output::OutputTarget;
use biscuit_terminal::terminal::Terminal;
use clap::Parser;
use darkmatter::layout::{DarkmatterPage, PageComponent};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::style::{PreparedStyle, PresentationDefaults, StylePreparationPolicy};
use darkmatter_cli::Cli;
use darkmatter_cli::args::{Command, Feature, PreparedRenderOptions, RenderOptions};
use darkmatter_cli::render::{PageInputs, PagePresentation, apply_presentation};
use darkmatter_cli::render_style::{document_style_layer, prepare_cli_style_inputs};
use darkmatter_cli::request::MdRequest;

/// The style bucket each [`PageComponent`] reads: the ten targets a page-wide
/// broadcast reaches.
pub const COMPONENT_BUCKETS: [(PageComponent, &str); 10] = [
    (PageComponent::Images, "images"),
    (PageComponent::BlockQuotes, "block-quote"),
    (PageComponent::Tables, "table"),
    (PageComponent::CodeBlocks, "code-block"),
    (PageComponent::Ul, "ul"),
    (PageComponent::Ol, "ol"),
    (PageComponent::Li, "li"),
    (PageComponent::Hyperlinks, "hyperlinks"),
    (PageComponent::Hr, "hr"),
    (PageComponent::Disclosure, "disclosure"),
];

/// The bucket name of `component`.
pub fn bucket(component: PageComponent) -> &'static str {
    COMPONENT_BUCKETS
        .iter()
        .find_map(|(each, name)| (*each == component).then_some(*name))
        .expect("every PageComponent has a bucket")
}

/// A `style=OBJECT` setter giving each of `buckets` the JSON5 `members`
/// (`{ "max-width": 40 }`), which is how a broadcast flag is spelled now.
pub fn style_object(buckets: &[&str], members: &str) -> String {
    let fields: Vec<String> = buckets.iter().map(|bucket| format!("\"{bucket}\": {members}")).collect();
    format!("style={{ {} }}", fields.join(", "))
}

/// [`style_object`] over every bucket a page-wide broadcast reached.
pub fn every_component(members: &str) -> String {
    let buckets: Vec<&str> = COMPONENT_BUCKETS.iter().map(|(_, name)| *name).collect();
    style_object(&buckets, members)
}

pub fn parse_cli(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("md").chain(args.iter().copied())).expect("CLI args must parse")
}

/// The render options `md` dispatches for `cli`: the root's, or `render`'s own.
pub fn render_options(cli: &Cli) -> RenderOptions {
    match cli.command_or_render() {
        Command::Render(options) => options,
        other => panic!("expected a render, got {other:?}"),
    }
}

/// Page inputs with no document file: the stylesheet context is a scratch
/// directory and no theme variable is set.
pub fn page_inputs() -> PageInputs {
    PageInputs::new(biscuit_file::FileResolutionContext::new(std::env::temp_dir()), PresentationDefaults::default())
}

/// `doc` (a whole Markdown document) and its effective style under `args`'
/// setters and `--style` files, with the prepared render options.
///
/// ## Panics
///
/// The arguments do not parse or the style does not prepare.
pub fn prepared(doc: &str, args: &[&str]) -> (Markdown, PreparedStyle, PreparedRenderOptions) {
    try_prepared(doc, args).unwrap_or_else(|error| panic!("style must prepare for {args:?}: {error:#}"))
}

/// [`prepared`], returning a preparation failure.
pub fn try_prepared(
    doc: &str,
    args: &[&str],
) -> color_eyre::Result<(Markdown, PreparedStyle, PreparedRenderOptions)> {
    let options = PreparedRenderOptions::prepare(&render_options(&parse_cli(args))).expect("render options prepare");
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::create_dir_all(root.join("home")).expect("home");
    let request = MdRequest::new(RequestSnapshot::new(&root).with_home(Some(root.join("home"))));
    let md = Markdown::try_from_content(doc).expect("document parses");
    let inputs = prepare_cli_style_inputs(&md, None, &options.styles, &options.setters, &request)?;
    let document = document_style_layer(&md, None, request.launch_context()?.clone());
    let style = inputs.prepare(Some(document), StylePreparationPolicy::render())?;
    Ok((md, style, options))
}

/// Calls `f` with the presentation a render builds from `style`.
pub fn with_presentation<R>(
    style: &PreparedStyle,
    features: &[Feature],
    mermaid: bool,
    f: impl FnOnce(&PagePresentation<'_>) -> R,
) -> R {
    f(&PagePresentation { style, features, mermaid })
}

/// `page` with `style` lowered for `target`.
pub fn present(page: DarkmatterPage, style: &PreparedStyle, target: OutputTarget) -> DarkmatterPage {
    with_presentation(style, &[], false, |presentation| {
        apply_presentation(page, &page_inputs(), presentation, target).expect("style applies")
    })
}

/// The terminal page `md` builds for `doc` and `args` on a `width`-column
/// optimistic terminal, and the document.
pub fn styled(doc: &str, args: &[&str], width: u32) -> (DarkmatterPage, Markdown) {
    let (md, style, _) = prepared(doc, args);
    let term = Terminal::new_optimistic(width);
    (present(DarkmatterPage::new(&term), &style, OutputTarget::Terminal), md)
}

/// [`styled`]'s page alone.
pub fn styled_page(doc: &str, args: &[&str], width: u32) -> DarkmatterPage {
    styled(doc, args, width).0
}
