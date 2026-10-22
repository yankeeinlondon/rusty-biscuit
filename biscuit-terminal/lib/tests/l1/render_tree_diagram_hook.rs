//! Contract tests for the terminal tree renderer's `DiagramRenderer` hook.
//!
//! `biscuit-terminal` draws no diagrams itself: a `mermaid` fence reaches a
//! caller-installed hook only at `GraphicsMode::Rich` with
//! `TerminalMermaidMode::Image`, ahead of the `CodeRenderer` hook, and stays a
//! code block when no hook is installed. The real hook lives in
//! `biscuit-visualized` and is tested there.

use std::cell::RefCell;
use std::rc::Rc;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    DiagramRenderError, DiagramRenderRequest, DiagramRenderer, TerminalRenderOptions,
    render_terminal_node,
};
use biscuit_terminal::terminal::Terminal;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::color::TerminalCodeContext;
use renderable::tree::{
    CodeRenderer, DiagnosticKind, GraphicsMode, NodeAttrs, RenderError, RenderNode,
    RenderStrictness, TerminalMermaidMode,
};

const SOURCE: &str = "flowchart LR\n    A --> B";

/// What a [`RecordingDiagrams`] hook saw on one call.
#[derive(Debug, Clone, PartialEq)]
struct SeenRequest {
    source: String,
    node_id: Option<String>,
    available_width: u32,
    image_support: ImageSupport,
}

/// A hook that records each request and answers with a fixed result.
struct RecordingDiagrams {
    seen: RefCell<Vec<SeenRequest>>,
    answer: Result<&'static str, &'static str>,
}

impl RecordingDiagrams {
    fn answering(answer: Result<&'static str, &'static str>) -> Rc<Self> {
        Rc::new(Self { seen: RefCell::new(Vec::new()), answer })
    }

    fn seen(&self) -> Vec<SeenRequest> {
        self.seen.borrow().clone()
    }
}

impl DiagramRenderer for RecordingDiagrams {
    fn render_terminal_diagram(
        &self,
        request: &DiagramRenderRequest<'_>,
    ) -> Result<String, DiagramRenderError> {
        self.seen.borrow_mut().push(SeenRequest {
            source: request.source.to_string(),
            node_id: request.attrs.id.clone(),
            available_width: request.available_width,
            image_support: request.terminal.image_support(),
        });
        self.answer
            .map(str::to_string)
            .map_err(DiagramRenderError::new)
    }
}

/// A code hook that counts its calls and marks its output.
#[derive(Default)]
struct CountingCode {
    calls: RefCell<u32>,
}

impl CodeRenderer for CountingCode {
    fn render_terminal_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
        _context: TerminalCodeContext,
    ) -> Option<String> {
        *self.calls.borrow_mut() += 1;
        Some("CODE HOOK".to_string())
    }

    fn render_browser_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
    ) -> Option<BrowserFragment<Ready>> {
        None
    }
}

fn mermaid_fence() -> RenderNode {
    RenderNode::code(Some("mermaid".to_string()), None, SOURCE)
}

/// Options that request Mermaid promotion on a plain 80-column terminal.
fn promoting(strictness: RenderStrictness) -> TerminalRenderOptions {
    let term = Terminal::new_optimistic(80);
    let mut opts = TerminalRenderOptions::new(&term, strictness);
    opts.context.graphics_mode = GraphicsMode::Rich;
    opts.context.mermaid_mode = TerminalMermaidMode::Image;
    opts
}

#[test]
fn mermaid_fence_without_a_diagram_hook_renders_as_the_code_block() {
    let promoted = render_terminal_node(&mermaid_fence(), &promoting(RenderStrictness::Strict))
        .expect("a missing hook is not an error, even under Strict");

    let mut as_code = promoting(RenderStrictness::Strict);
    as_code.context.mermaid_mode = TerminalMermaidMode::Code;
    let code = render_terminal_node(&mermaid_fence(), &as_code).expect("render as code");

    assert_eq!(promoted.output, code.output, "no hook must render exactly like Mermaid-as-code");
    assert!(promoted.output.contains("A --> B"), "the fence source is shown: {:?}", promoted.output);
    assert!(promoted.diagnostics.is_empty(), "no hook is not lossy: {:?}", promoted.diagnostics);
}

#[test]
fn mermaid_fence_without_a_diagram_hook_still_reaches_the_code_hook() {
    let code = Rc::new(CountingCode::default());
    let opts = promoting(RenderStrictness::Warn).with_code_renderer(code.clone());

    let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

    assert_eq!(*code.calls.borrow(), 1);
    assert!(rendered.output.contains("CODE HOOK"), "{:?}", rendered.output);
}

#[test]
fn diagram_hook_is_consulted_before_the_code_hook() {
    let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
    let code = Rc::new(CountingCode::default());
    let opts = promoting(RenderStrictness::Warn)
        .with_code_renderer(code.clone())
        .with_diagram_renderer(diagrams.clone());

    let mut node = mermaid_fence();
    node.attrs.id = Some("fig-1".to_string());
    let rendered = render_terminal_node(&node, &opts).expect("render");

    assert!(rendered.output.contains("DIAGRAM"), "{:?}", rendered.output);
    assert!(!rendered.output.contains("CODE HOOK"), "{:?}", rendered.output);
    assert_eq!(*code.calls.borrow(), 0, "a drawn diagram never reaches the code hook");
    assert_eq!(
        diagrams.seen(),
        vec![SeenRequest {
            source: SOURCE.to_string(),
            node_id: Some("fig-1".to_string()),
            available_width: 80,
            image_support: ImageSupport::Kitty,
        }],
    );
}

#[test]
fn diagram_hook_is_not_consulted_unless_promotion_is_requested() {
    let cases: [(&str, Option<&str>, GraphicsMode, TerminalMermaidMode); 4] = [
        ("mermaid-as-code", Some("mermaid"), GraphicsMode::Rich, TerminalMermaidMode::Code),
        ("graphics off", Some("mermaid"), GraphicsMode::Off, TerminalMermaidMode::Image),
        ("other language", Some("rust"), GraphicsMode::Rich, TerminalMermaidMode::Image),
        ("no language", None, GraphicsMode::Rich, TerminalMermaidMode::Image),
    ];
    for (name, lang, graphics_mode, mermaid_mode) in cases {
        let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
        let mut opts = promoting(RenderStrictness::Warn).with_diagram_renderer(diagrams.clone());
        opts.context.graphics_mode = graphics_mode;
        opts.context.mermaid_mode = mermaid_mode;
        let node = RenderNode::code(lang.map(str::to_string), None, SOURCE);

        let rendered = render_terminal_node(&node, &opts).expect("render");

        assert!(diagrams.seen().is_empty(), "{name}: hook must not be consulted");
        assert!(!rendered.output.contains("DIAGRAM"), "{name}: {:?}", rendered.output);
    }
}

#[test]
fn diagram_hook_failure_follows_the_render_strictness() {
    // Strict: the hook's error rejects the render, carrying its message.
    let strict = promoting(RenderStrictness::Strict)
        .with_diagram_renderer(RecordingDiagrams::answering(Err("bad diagram")));
    match render_terminal_node(&mermaid_fence(), &strict) {
        Err(RenderError::LossyRejected { message }) => {
            assert_eq!(message, "Mermaid promotion failed: bad diagram");
        }
        other => panic!("Strict must reject a failed promotion; got {other:?}"),
    }

    // Warn: one lossy diagnostic, then the code hook renders the fence.
    let code = Rc::new(CountingCode::default());
    let warn = promoting(RenderStrictness::Warn)
        .with_code_renderer(code.clone())
        .with_diagram_renderer(RecordingDiagrams::answering(Err("bad diagram")));
    let rendered = render_terminal_node(&mermaid_fence(), &warn).expect("Warn falls back");
    assert!(rendered.output.contains("CODE HOOK"), "{:?}", rendered.output);
    assert_eq!(*code.calls.borrow(), 1);
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.diagnostics[0].kind, DiagnosticKind::Lossy);
    assert_eq!(
        rendered.diagnostics[0].message,
        "Mermaid rasterization failed, falling back to code block: bad diagram"
    );

    // Lossy: the same fallback, silently.
    let lossy = promoting(RenderStrictness::Lossy)
        .with_diagram_renderer(RecordingDiagrams::answering(Err("bad diagram")));
    let rendered = render_terminal_node(&mermaid_fence(), &lossy).expect("Lossy falls back");
    assert!(rendered.output.contains("A --> B"), "{:?}", rendered.output);
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

#[test]
fn diagram_hook_receives_the_forced_terminal_snapshot() {
    let term = Terminal::builder()
        .width(80)
        .image_support(ImageSupport::None)
        .is_tty(false)
        .build();
    let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
    let mut opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn)
        .with_diagram_renderer(diagrams.clone());
    opts.context.graphics_mode = GraphicsMode::Rich;
    opts.context.mermaid_mode = TerminalMermaidMode::Image;
    opts.context.force_graphics = true;

    render_terminal_node(&mermaid_fence(), &opts).expect("render");

    let seen = diagrams.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].image_support,
        ImageSupport::Kitty,
        "force_graphics upgrades a no-image terminal before the hook sees it"
    );
}

#[test]
fn nested_mermaid_fence_reaches_the_diagram_hook_at_its_narrowed_width() {
    let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
    let opts = promoting(RenderStrictness::Warn).with_diagram_renderer(diagrams.clone());
    // A block child of a list item renders through narrowed, cloned options.
    let list = RenderNode::list(
        false,
        None,
        vec![RenderNode::list_item(
            None,
            vec![RenderNode::paragraph(vec![RenderNode::text("item")]), mermaid_fence()],
        )],
    );

    let rendered = render_terminal_node(&list, &opts).expect("render");

    let seen = diagrams.seen();
    assert_eq!(seen.len(), 1, "the list item's child options keep the hook");
    assert!(
        seen[0].available_width < 80,
        "a nested fence is offered its narrowed width, got {}",
        seen[0].available_width
    );
    assert!(rendered.output.contains("DIAGRAM"), "{:?}", rendered.output);
}
