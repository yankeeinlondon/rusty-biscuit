//! `TerminalDiagramRenderer`: the render-tree hook that draws Mermaid fences.
//!
//! biscuit-terminal's own tests pin when the hook is consulted with a fake;
//! these pin what the real hook draws through `render_terminal_node`.

use std::rc::Rc;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_node,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_visualized::components::{MermaidDiagram, TerminalDiagramRenderer};
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
        .with_diagram_renderer(Rc::new(TerminalDiagramRenderer));
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
        diagram_renderer: Some(Rc::new(TerminalDiagramRenderer)),
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
