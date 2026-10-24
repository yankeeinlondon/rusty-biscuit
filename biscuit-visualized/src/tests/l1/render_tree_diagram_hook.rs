//! `TerminalDiagramRenderer`: the render-tree hook that draws Mermaid fences.
//!
//! biscuit-terminal's own tests pin when the hook is consulted with a fake;
//! these pin what the real hook draws through `render_terminal_node`.

use std::rc::Rc;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_node,
};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use biscuit_visualized::components::{
    MermaidDiagram, MermaidFallbackPolicy, PARTIAL_NOTE_ID, TerminalDiagramRenderer, TextFallback,
};
use renderable::tree::{
    DiagnosticKind, GraphicsMode, RenderError, RenderNode, RenderStrictness, TerminalMermaidMode,
};

const SOURCE: &str = "flowchart LR\n    A --> B";

fn mermaid_fence(source: &str) -> RenderNode {
    RenderNode::code(Some("mermaid".to_string()), None, source)
}

/// Options that request Mermaid promotion with the real hook installed.
fn promoting(term: &Terminal, strictness: RenderStrictness) -> TerminalRenderOptions {
    let mut opts = TerminalRenderOptions::new(term, strictness)
        .with_diagram_renderer(Rc::new(TerminalDiagramRenderer::default()));
    opts.context.graphics_mode = GraphicsMode::Rich;
    opts.context.mermaid_mode = TerminalMermaidMode::Image;
    opts
}

fn no_image_terminal() -> Terminal {
    Terminal::builder()
        .width(80)
        .image_support(ImageSupport::None)
        .is_tty(false)
        .build()
}

/// Mermaid code blocks under `GraphicsMode::Rich` **with the Mermaid opt-in**
/// bypass the `CodeRenderer` hook and route to `MermaidDiagram::try_render`.
/// When the diagram cannot be drawn as an image the renderer records a lossy
/// diagnostic and falls back to the built-in plain fallback.
#[test]
fn mermaid_rich_mode_with_opt_in_bypasses_code_renderer() {
    use renderable::tree::{GraphicsMode, TerminalMermaidMode};

    let mermaid_node = RenderNode::code(
        Some("mermaid".to_string()),
        None,
        "flowchart LR\n    A --> B",
    );

    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.graphics_mode = GraphicsMode::Rich;
    context.mermaid_mode = TerminalMermaidMode::Image;

    // No code_renderer set — the fallback path is the built-in plain renderer.
    let opts = TerminalRenderOptions {
        context,
        strictness: RenderStrictness::Warn,
        code_renderer: None,
        diagram_renderer: Some(Rc::new(TerminalDiagramRenderer::default())),
    };

    let result = render_terminal_node(&mermaid_node, &opts).expect("render should succeed");

    // The output must contain the Mermaid source (plain fallback) when the
    // image render fails, OR an image string when it succeeds.
    // Either way, the render must not panic.
    assert!(
        !result.output.is_empty(),
        "Mermaid render must produce non-empty output"
    );
}

#[test]
fn hook_draws_the_fence_exactly_as_the_component_does() {
    let term = Terminal::new_optimistic(80);

    let rendered = render_terminal_node(&mermaid_fence(SOURCE), &promoting(&term, RenderStrictness::Strict))
        .expect("an optimistic Kitty terminal draws the diagram");
    let direct = MermaidDiagram::new(SOURCE).try_render(&term).expect("direct render");

    // Every Kitty upload has a fresh image ID; nothing else may differ.
    assert_eq!(
        without_kitty_id(&rendered.output),
        without_kitty_id(&direct.output),
        "the hook adds nothing to the component's output"
    );
    assert!(rendered.output.contains("\x1b_G"), "a Kitty image, not code: {:?}", rendered.output);
    assert!(!rendered.output.contains("A --> B"), "{:?}", rendered.output);
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

/// `output` with the digits of its one Kitty image ID removed.
fn without_kitty_id(output: &str) -> String {
    let start = output.find("\x1b_Gq=2,i=").expect("a Kitty upload") + "\x1b_Gq=2,i=".len();
    let digits = output[start..].find(',').expect("more parameters follow the ID");
    format!("{}{}", &output[..start], &output[start + digits..])
}

/// Finding 7 (review-1): under `RenderStrictness::Strict`, a Mermaid
/// promotion that cannot complete must escalate to a `RenderError` rather
/// than silently degrading to a code block. A terminal with no image
/// support guarantees the promotion fails (no protocol to target, or no
/// rasterization deps) regardless of host capability.
#[test]
fn mermaid_promotion_failure_rejects_under_strict() {
    let result = render_terminal_node(
        &mermaid_fence(SOURCE),
        &promoting(&no_image_terminal(), RenderStrictness::Strict),
    );
    assert!(
        matches!(result, Err(RenderError::LossyRejected { .. })),
        "strict Mermaid promotion failure must reject; got {result:?}",
    );
}

#[test]
fn failed_promotion_warns_with_the_component_error_and_renders_code() {
    let term = no_image_terminal();
    let expected = MermaidDiagram::new(SOURCE)
        .try_render(&term)
        .expect_err("no image support cannot draw a diagram");

    let rendered = render_terminal_node(&mermaid_fence(SOURCE), &promoting(&term, RenderStrictness::Warn))
        .expect("Warn falls back");

    assert!(rendered.output.contains("A --> B"), "the fence source is shown: {:?}", rendered.output);
    assert!(!rendered.output.contains("\x1b_G"), "{:?}", rendered.output);
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.diagnostics[0].kind, DiagnosticKind::Lossy);
    assert_eq!(
        rendered.diagnostics[0].message,
        format!("Mermaid rasterization failed, falling back to code block: {expected}")
    );
}

#[test]
fn invalid_mermaid_source_falls_back_to_code_under_warn() {
    let term = Terminal::new_optimistic(80);
    let source = "this is not valid mermaid syntax";

    let rendered = render_terminal_node(&mermaid_fence(source), &promoting(&term, RenderStrictness::Warn))
        .expect("Warn falls back");

    assert!(rendered.output.contains(source), "{:?}", rendered.output);
    assert!(!rendered.output.contains("\x1b_G"), "{:?}", rendered.output);
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.diagnostics[0].kind, DiagnosticKind::Lossy);
}

/// The request's ceiling binds the real hook: on an image-capable terminal, a
/// `Vector` render never carries an image upload.
#[test]
fn hook_never_draws_an_image_below_the_rich_ceiling() {
    let term = Terminal::new_optimistic(80);
    let mut opts = promoting(&term, RenderStrictness::Strict);
    opts.context.mermaid_mode = TerminalMermaidMode::Automatic;
    opts.context.graphics_mode = GraphicsMode::Vector;

    let rendered = render_terminal_node(&mermaid_fence(SOURCE), &opts)
        .expect("a Vector render never fails on an image route");

    assert!(!rendered.output.contains("\x1b_G"), "no Kitty upload at Vector: {:?}", rendered.output);
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

// ── Automatic mode (D5, AC-F8) ───────────────────────────────────────────

/// Fits 80 columns canonically; its text drawing is partial.
const FLOWCHART: &str = "flowchart LR\n    A[Write spec] --> B{Approved?}\n    B -- yes --> C[Plan]";
/// Parses as a flowchart header, then fails.
const BROKEN_FLOWCHART: &str = "flowchart LR\n    A[unclosed --> B";
const PIE: &str = "pie\n    \"Dogs\" : 50";

/// The hook with text allowed explicitly, so `MERMAID_FALLBACK` in the
/// developer's environment cannot change these tests.
fn text_allowed() -> TerminalDiagramRenderer {
    TerminalDiagramRenderer::default()
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed))
}

/// Automatic promotion at `graphics_mode` with `hook` installed.
fn automatic_with(
    term: &Terminal,
    strictness: RenderStrictness,
    graphics_mode: GraphicsMode,
    hook: TerminalDiagramRenderer,
) -> TerminalRenderOptions {
    let mut opts = TerminalRenderOptions::new(term, strictness).with_diagram_renderer(Rc::new(hook));
    opts.context.graphics_mode = graphics_mode;
    opts.context.mermaid_mode = TerminalMermaidMode::Automatic;
    opts
}

/// `source` rendered as the ordinary code block `opts` would print.
fn as_code(source: &str, opts: &TerminalRenderOptions) -> String {
    let mut code = opts.clone();
    code.context.mermaid_mode = TerminalMermaidMode::Code;
    code.diagram_renderer = None;
    render_terminal_node(&mermaid_fence(source), &code).expect("code block").output
}

#[test]
fn automatic_mode_draws_an_image_at_rich_with_image_support() {
    let term = Terminal::new_optimistic(80);
    let opts = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, text_allowed());

    let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("image");

    assert!(rendered.output.contains("\x1b_G"), "a Kitty image: {:?}", rendered.output);
    assert!(!rendered.output.contains("```"), "{:?}", rendered.output);
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

/// Text at `Vector` on an image-capable terminal, and at `Rich` on one
/// without images.
#[test]
fn automatic_mode_draws_text_below_rich_or_without_images() {
    for (term, graphics_mode) in
        [(Terminal::new_optimistic(80), GraphicsMode::Vector), (no_image_terminal(), GraphicsMode::Rich)]
    {
        let opts = automatic_with(&term, RenderStrictness::Strict, graphics_mode, text_allowed());

        let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("text");

        let plain = strip_escape_codes(rendered.output.as_str());
        assert!(plain.starts_with("```txt\n┌"), "{graphics_mode:?}: {plain}");
        assert!(plain.contains("│ Write spec ├"), "{graphics_mode:?}: {plain}");
        assert!(plain.contains("│ Note: ") && plain.contains("mermaid-partial-warning]."), "the note: {plain}");
        assert!(!rendered.output.contains("\x1b_G"), "{graphics_mode:?}");
        assert!(!plain.contains("A[Write spec]"), "not the source: {plain}");
        assert!(rendered.diagnostics.is_empty(), "{graphics_mode:?}: {:?}", rendered.diagnostics);
    }
}

#[test]
fn automatic_mode_at_graphics_off_renders_code() {
    let opts = automatic_with(&Terminal::new_optimistic(80), RenderStrictness::Strict, GraphicsMode::Off, text_allowed());

    let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("code");

    assert_eq!(rendered.output, as_code(FLOWCHART, &opts));
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

/// Declined: an ineligible family renders as code with no diagnostic under
/// every strictness, at `Rich` without images and at `Vector`.
#[test]
fn an_ineligible_fence_renders_as_code_without_a_diagnostic() {
    for strictness in [RenderStrictness::Strict, RenderStrictness::Warn, RenderStrictness::Lossy] {
        for (term, graphics_mode) in
            [(no_image_terminal(), GraphicsMode::Rich), (Terminal::new_optimistic(80), GraphicsMode::Vector)]
        {
            let opts = automatic_with(&term, strictness, graphics_mode, text_allowed());

            let rendered = render_terminal_node(&mermaid_fence(PIE), &opts).expect("declined is not an error");

            assert_eq!(rendered.output, as_code(PIE, &opts), "{strictness:?} {graphics_mode:?}");
            assert!(rendered.diagnostics.is_empty(), "{strictness:?} {graphics_mode:?}: {:?}", rendered.diagnostics);
        }
    }
}

#[test]
fn a_parse_error_follows_the_render_strictness() {
    let term = no_image_terminal();
    let strict = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Vector, text_allowed());
    let result = render_terminal_node(&mermaid_fence(BROKEN_FLOWCHART), &strict);
    assert!(matches!(result, Err(RenderError::LossyRejected { .. })), "{result:?}");

    let warn = automatic_with(&term, RenderStrictness::Warn, GraphicsMode::Vector, text_allowed());
    let rendered = render_terminal_node(&mermaid_fence(BROKEN_FLOWCHART), &warn).expect("Warn falls back");
    assert_eq!(rendered.output, as_code(BROKEN_FLOWCHART, &warn));
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.diagnostics[0].kind, DiagnosticKind::Lossy);
    assert!(rendered.diagnostics[0].message.contains("parse error"), "{}", rendered.diagnostics[0].message);

    let lossy = automatic_with(&term, RenderStrictness::Lossy, GraphicsMode::Vector, text_allowed());
    let rendered = render_terminal_node(&mermaid_fence(BROKEN_FLOWCHART), &lossy).expect("Lossy falls back");
    assert_eq!(rendered.output, as_code(BROKEN_FLOWCHART, &lossy));
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

#[test]
fn automatic_mode_without_the_hook_and_code_mode_render_code() {
    let term = no_image_terminal();
    let mut no_hook = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Vector, text_allowed());
    no_hook.diagram_renderer = None;
    let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &no_hook).expect("code");
    assert_eq!(rendered.output, as_code(FLOWCHART, &no_hook));

    let mut code_mode = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, text_allowed());
    code_mode.context.mermaid_mode = TerminalMermaidMode::Code;
    let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &code_mode).expect("code");
    assert_eq!(rendered.output, as_code(FLOWCHART, &code_mode));
}

/// `Image` mode keeps meaning image-only: on a terminal without images it
/// fails rather than drawing text (the strict rejection is pinned above).
#[test]
fn image_mode_never_answers_with_text() {
    let rendered = render_terminal_node(
        &mermaid_fence(FLOWCHART),
        &promoting(&no_image_terminal(), RenderStrictness::Lossy),
    )
    .expect("Lossy falls back");

    assert_eq!(strip_escape_codes(rendered.output.as_str()), format!("```mermaid\n{FLOWCHART}\n```"));
}

/// The policy set on the hook's builder: never text declines silently even
/// under strict; a hidden note leaves the drawing alone.
#[test]
fn the_hooks_fallback_policy_applies() {
    let term = no_image_terminal();
    let never = TerminalDiagramRenderer::default()
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Never));
    assert_eq!(never.fallback_policy().text_fallback(), Some(TextFallback::Never));
    let opts = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, never);
    let rendered = render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("declined");
    assert_eq!(rendered.output, as_code(FLOWCHART, &opts));
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);

    let no_note = TerminalDiagramRenderer::default().with_fallback_policy(
        MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed).with_show_partial_note(false),
    );
    let opts = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, no_note);
    let plain = strip_escape_codes(render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("text").output);
    assert!(plain.starts_with("```txt\n") && plain.ends_with("\n```"), "{plain}");
    assert!(!plain.contains(PARTIAL_NOTE_ID), "{plain}");
}

/// The hook's text, note included, is exactly what the component prints at
/// the same width.
#[test]
fn hook_text_is_exactly_the_components() {
    let term = no_image_terminal();
    let opts = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, text_allowed());

    let hook = render_terminal_node(&mermaid_fence(FLOWCHART), &opts).expect("text").output;
    let component = MermaidDiagram::new(FLOWCHART)
        .with_fallback_policy(MermaidFallbackPolicy::default().with_text_fallback(TextFallback::Allowed))
        .render(&term);

    assert_eq!(hook, component);
    assert!(hook.contains("```txt"), "{hook:?}");
}

/// A fence in a list item is drawn within the item's width: the drawing
/// that fits the page declines there when the item is too narrow for it.
#[test]
fn a_nested_fence_is_drawn_within_its_own_width() {
    // 66 cells canonically, 58 compactly: it needs 60 columns with the code
    // block's padding.
    let chain = "flowchart LR\n    A[Alpha] --> B[Beta] --> C[Gamma] --> D[Delta] --> E[Epsilon]";
    let item = |source: &str| RenderNode::list(false, None, vec![RenderNode::list_item(None, vec![mermaid_fence(source)])]);
    let at = |width: u32| {
        let term = Terminal::builder().width(width).image_support(ImageSupport::None).is_tty(false).build();
        let opts = automatic_with(&term, RenderStrictness::Strict, GraphicsMode::Rich, text_allowed());
        strip_escape_codes(render_terminal_node(&item(chain), &opts).expect("render").output)
    };

    let top_level = Terminal::builder().width(60).image_support(ImageSupport::None).is_tty(false).build();
    let page = automatic_with(&top_level, RenderStrictness::Strict, GraphicsMode::Rich, text_allowed());
    let page = strip_escape_codes(render_terminal_node(&mermaid_fence(chain), &page).expect("render").output);
    assert!(page.contains("```txt"), "fits 60 columns at the top level: {page}");
    assert!(at(60).contains("```mermaid"), "narrower inside the item: {}", at(60));
    assert!(at(80).contains("```txt"), "{}", at(80));
}
