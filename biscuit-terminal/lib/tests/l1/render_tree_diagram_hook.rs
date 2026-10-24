//! Contract tests for the terminal tree renderer's `DiagramRenderer` hook.
//!
//! `biscuit-terminal` draws no diagrams itself: a `mermaid` fence reaches a
//! caller-installed hook, ahead of the `CodeRenderer` hook, with
//! `TerminalMermaidMode::Image` at `GraphicsMode::Rich` and with
//! `TerminalMermaidMode::Automatic` at `Rich` or `Vector`. It stays a code
//! block when no hook is installed or the hook declines. The real hook lives
//! in `biscuit-visualized` and is tested there.

use std::cell::RefCell;
use std::rc::Rc;

use biscuit_terminal::discovery::detection::ImageSupport;
use biscuit_terminal::render_tree::{
    DiagramOutcome, DiagramRenderError, DiagramRenderRequest, DiagramRenderer,
    TerminalRenderOptions, render_terminal_node,
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
    graphics_mode: GraphicsMode,
}

/// A hook that records each request and answers with a fixed result.
struct RecordingDiagrams {
    seen: RefCell<Vec<SeenRequest>>,
    answer: Result<DiagramOutcome, &'static str>,
}

impl RecordingDiagrams {
    /// A hook whose `Ok` answer is pre-formatted output.
    fn answering(answer: Result<&'static str, &'static str>) -> Rc<Self> {
        Self::outcome(answer.map(|output| DiagramOutcome::Rendered(output.to_string())))
    }

    fn outcome(answer: Result<DiagramOutcome, &'static str>) -> Rc<Self> {
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
    ) -> Result<DiagramOutcome, DiagramRenderError> {
        self.seen.borrow_mut().push(SeenRequest {
            source: request.source.to_string(),
            node_id: request.attrs.id.clone(),
            available_width: request.available_width,
            image_support: request.terminal.image_support(),
            graphics_mode: request.graphics_mode,
        });
        self.answer.clone().map_err(DiagramRenderError::new)
    }
}

/// A code hook that counts its calls, records what it was given, and marks
/// its output.
#[derive(Default)]
struct CountingCode {
    calls: RefCell<u32>,
    /// `(lang, value, available width)` per call.
    seen: RefCell<Vec<(Option<String>, String, u32)>>,
}

impl CodeRenderer for CountingCode {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        *self.calls.borrow_mut() += 1;
        self.seen.borrow_mut().push((lang.map(str::to_string), value.to_string(), context.width()));
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
            graphics_mode: GraphicsMode::Rich,
        }],
    );
}

#[test]
fn diagram_hook_is_not_consulted_unless_promotion_is_requested() {
    let cases: [(&str, Option<&str>, GraphicsMode, TerminalMermaidMode); 8] = [
        ("mermaid-as-code", Some("mermaid"), GraphicsMode::Rich, TerminalMermaidMode::Code),
        ("mermaid-as-code at vector", Some("mermaid"), GraphicsMode::Vector, TerminalMermaidMode::Code),
        ("graphics off", Some("mermaid"), GraphicsMode::Off, TerminalMermaidMode::Image),
        ("image at vector", Some("mermaid"), GraphicsMode::Vector, TerminalMermaidMode::Image),
        ("automatic, graphics off", Some("mermaid"), GraphicsMode::Off, TerminalMermaidMode::Automatic),
        ("automatic, other language", Some("rust"), GraphicsMode::Rich, TerminalMermaidMode::Automatic),
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

/// Options that request automatic Mermaid promotion at `graphics_mode`.
fn automatic(strictness: RenderStrictness, graphics_mode: GraphicsMode) -> TerminalRenderOptions {
    let mut opts = promoting(strictness);
    opts.context.graphics_mode = graphics_mode;
    opts.context.mermaid_mode = TerminalMermaidMode::Automatic;
    opts
}

/// The output of the same fence rendered as an ordinary code block.
fn as_code_block(opts: &TerminalRenderOptions) -> String {
    let mut as_code = opts.clone();
    as_code.context.mermaid_mode = TerminalMermaidMode::Code;
    render_terminal_node(&mermaid_fence(), &as_code).expect("render as code").output
}

#[test]
fn automatic_mode_offers_the_fence_at_rich_and_vector_with_the_ceiling() {
    for graphics_mode in [GraphicsMode::Rich, GraphicsMode::Vector] {
        let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
        let opts = automatic(RenderStrictness::Strict, graphics_mode)
            .with_diagram_renderer(diagrams.clone());

        let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

        assert_eq!(rendered.output, "DIAGRAM", "{graphics_mode:?}: Rendered prints verbatim");
        assert!(rendered.diagnostics.is_empty(), "{graphics_mode:?}: {:?}", rendered.diagnostics);
        let seen = diagrams.seen();
        assert_eq!(seen.len(), 1, "{graphics_mode:?}: exactly one hook call");
        assert_eq!(seen[0].graphics_mode, graphics_mode, "the request carries the ceiling");
        assert_eq!(seen[0].source, SOURCE);
    }
}

#[test]
fn automatic_mode_at_graphics_off_renders_the_code_block_without_the_hook() {
    let diagrams = RecordingDiagrams::answering(Ok("DIAGRAM"));
    let opts = automatic(RenderStrictness::Strict, GraphicsMode::Off)
        .with_diagram_renderer(diagrams.clone());

    let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

    assert!(diagrams.seen().is_empty(), "Off never consults the hook");
    assert_eq!(rendered.output, as_code_block(&opts));
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

#[test]
fn automatic_mode_without_a_diagram_hook_renders_the_code_block() {
    for graphics_mode in [GraphicsMode::Rich, GraphicsMode::Vector] {
        let opts = automatic(RenderStrictness::Strict, graphics_mode);

        let rendered = render_terminal_node(&mermaid_fence(), &opts)
            .expect("a missing hook is not an error, even under Strict");

        assert_eq!(rendered.output, as_code_block(&opts), "{graphics_mode:?}");
        assert!(rendered.output.contains("A --> B"), "{:?}", rendered.output);
        assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
    }
}

#[test]
fn declined_renders_the_code_block_silently_under_every_strictness() {
    for strictness in [RenderStrictness::Strict, RenderStrictness::Warn, RenderStrictness::Lossy] {
        for (mode, graphics_mode) in [
            (TerminalMermaidMode::Automatic, GraphicsMode::Rich),
            (TerminalMermaidMode::Automatic, GraphicsMode::Vector),
            (TerminalMermaidMode::Image, GraphicsMode::Rich),
        ] {
            let case = format!("{strictness:?}/{mode:?}/{graphics_mode:?}");
            let diagrams = RecordingDiagrams::outcome(Ok(DiagramOutcome::Declined));
            let mut opts = promoting(strictness).with_diagram_renderer(diagrams.clone());
            opts.context.mermaid_mode = mode;
            opts.context.graphics_mode = graphics_mode;

            let rendered = render_terminal_node(&mermaid_fence(), &opts)
                .unwrap_or_else(|e| panic!("{case}: a declined fence never fails the render: {e}"));

            assert_eq!(diagrams.seen().len(), 1, "{case}: the hook was asked");
            assert_eq!(rendered.output, as_code_block(&opts), "{case}");
            assert!(rendered.diagnostics.is_empty(), "{case}: {:?}", rendered.diagnostics);
        }
    }
}

#[test]
fn declined_fence_still_reaches_the_code_hook() {
    let code = Rc::new(CountingCode::default());
    let opts = automatic(RenderStrictness::Strict, GraphicsMode::Vector)
        .with_code_renderer(code.clone())
        .with_diagram_renderer(RecordingDiagrams::outcome(Ok(DiagramOutcome::Declined)));

    let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

    assert_eq!(rendered.output, "CODE HOOK");
    assert_eq!(
        *code.seen.borrow(),
        vec![(Some("mermaid".to_string()), SOURCE.to_string(), 80)],
    );
}

#[test]
fn automatic_mode_hook_failure_follows_the_render_strictness() {
    // Strict rejects, carrying the hook's message.
    let strict = automatic(RenderStrictness::Strict, GraphicsMode::Vector)
        .with_diagram_renderer(RecordingDiagrams::answering(Err("parse error")));
    match render_terminal_node(&mermaid_fence(), &strict) {
        Err(RenderError::LossyRejected { message }) => {
            assert_eq!(message, "Mermaid promotion failed: parse error");
        }
        other => panic!("Strict must reject a failed promotion; got {other:?}"),
    }

    // Warn records one lossy diagnostic and renders the code block.
    let warn = automatic(RenderStrictness::Warn, GraphicsMode::Vector)
        .with_diagram_renderer(RecordingDiagrams::answering(Err("parse error")));
    let rendered = render_terminal_node(&mermaid_fence(), &warn).expect("Warn falls back");
    assert_eq!(rendered.output, as_code_block(&warn));
    assert_eq!(rendered.diagnostics.len(), 1, "{:?}", rendered.diagnostics);
    assert_eq!(rendered.diagnostics[0].kind, DiagnosticKind::Lossy);

    // Lossy falls back silently.
    let lossy = automatic(RenderStrictness::Lossy, GraphicsMode::Vector)
        .with_diagram_renderer(RecordingDiagrams::answering(Err("parse error")));
    let rendered = render_terminal_node(&mermaid_fence(), &lossy).expect("Lossy falls back");
    assert_eq!(rendered.output, as_code_block(&lossy));
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

/// A text drawing as a `txt` code node, then a source fallback as a `mermaid`
/// code node: the shapes a diagram hook's fragment carries.
fn fragment() -> Vec<RenderNode> {
    vec![
        RenderNode::code(Some("txt".to_string()), None, "+---+\n| A |\n+---+"),
        RenderNode::code(Some("mermaid".to_string()), None, SOURCE),
    ]
}

#[test]
fn nodes_outcome_lays_out_its_code_nodes_through_the_code_hook() {
    let code = Rc::new(CountingCode::default());
    let diagrams = RecordingDiagrams::outcome(Ok(DiagramOutcome::Nodes(fragment())));
    let opts = automatic(RenderStrictness::Strict, GraphicsMode::Vector)
        .with_code_renderer(code.clone())
        .with_diagram_renderer(diagrams.clone());

    let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

    // Both fragment nodes reach the code hook, in order and at the fence's
    // width; the `mermaid` node in the fragment is not offered to the diagram
    // hook a second time.
    assert_eq!(
        *code.seen.borrow(),
        vec![
            (Some("txt".to_string()), "+---+\n| A |\n+---+".to_string(), 80),
            (Some("mermaid".to_string()), SOURCE.to_string(), 80),
        ],
    );
    assert_eq!(diagrams.seen().len(), 1, "the fragment never re-enters the diagram hook");
    assert_eq!(rendered.output, "CODE HOOK\n\nCODE HOOK", "blocks are separated as siblings");
    assert!(rendered.diagnostics.is_empty(), "{:?}", rendered.diagnostics);
}

#[test]
fn nodes_outcome_without_a_code_hook_renders_plain_code_blocks() {
    let diagrams = RecordingDiagrams::outcome(Ok(DiagramOutcome::Nodes(fragment())));
    let opts = automatic(RenderStrictness::Strict, GraphicsMode::Rich)
        .with_diagram_renderer(diagrams.clone());

    let rendered = render_terminal_node(&mermaid_fence(), &opts).expect("render");

    let expected = render_terminal_node(&RenderNode::root(fragment()), &as_code_opts(&opts))
        .expect("render the fragment directly")
        .output;
    assert_eq!(rendered.output, expected);
    assert!(rendered.output.contains("| A |"), "{:?}", rendered.output);
    assert!(rendered.output.contains("A --> B"), "{:?}", rendered.output);
    assert_eq!(diagrams.seen().len(), 1);
}

#[test]
fn nodes_outcome_in_a_list_item_is_laid_out_at_the_items_indent_and_width() {
    let code = Rc::new(CountingCode::default());
    let diagrams = RecordingDiagrams::outcome(Ok(DiagramOutcome::Nodes(vec![RenderNode::code(
        Some("txt".to_string()),
        None,
        "DRAWING",
    )])));
    let opts = automatic(RenderStrictness::Strict, GraphicsMode::Vector)
        .with_code_renderer(code.clone())
        .with_diagram_renderer(diagrams.clone());
    let list = RenderNode::list(
        false,
        None,
        vec![RenderNode::list_item(
            None,
            vec![RenderNode::paragraph(vec![RenderNode::text("item")]), mermaid_fence()],
        )],
    );

    let rendered = render_terminal_node(&list, &opts).expect("render");

    let fence_width = diagrams.seen()[0].available_width;
    assert!(fence_width < 80, "the fence is offered its narrowed width, got {fence_width}");
    assert_eq!(code.seen.borrow()[0].2, fence_width, "the fragment renders at the fence's width");
    let column_of = |needle: &str| {
        rendered.output.lines().find_map(|line| line.find(needle)).expect(needle)
    };
    assert!(column_of("item") > 0, "{:?}", rendered.output);
    assert_eq!(
        column_of("CODE HOOK"),
        column_of("item"),
        "the fragment sits at the item's content indent: {:?}",
        rendered.output
    );
}

/// `opts` with Mermaid promotion off.
fn as_code_opts(opts: &TerminalRenderOptions) -> TerminalRenderOptions {
    let mut as_code = opts.clone();
    as_code.context.mermaid_mode = TerminalMermaidMode::Code;
    as_code
}
