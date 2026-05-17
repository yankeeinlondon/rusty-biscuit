//! Parity tests for the `YamlBlock` component's tree projection.
//!
//! These tests verify that `YamlBlock::render_tree_node()` produces a
//! well-formed `NodeKind::Code` tree tagged with the `yaml` language, that the
//! tree validates cleanly, that plain tree rendering preserves the YAML body,
//! that an optional `CodeRenderer` hook is consulted, and that Markdown output
//! preserves the YAML fence.

use std::rc::Rc;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::prelude::strip_escape_codes;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;
use renderable::color::TerminalCodeContext;
use renderable::tree::{
    CodeRenderer, MarkdownRenderOptions, NodeAttrs, NodeKind, RenderStrictness, ValidationMode,
    render_markdown_node, validate,
};

/// Renders a render-tree node to a terminal string with no code-render hook.
fn render_tree_plain(node: &renderable::tree::RenderNode) -> String {
    let term = Terminal::new_optimistic(80);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
    render_terminal_node(node, &opts)
        .expect("tree render should succeed")
        .output
}

// ---------------------------------------------------------------------------
// Structural snapshot
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_produces_code_kind_with_yaml_lang() {
    let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
    let node = block
        .render_tree_node()
        .expect("YamlBlock should produce a tree node");

    match &node.kind {
        NodeKind::Code { lang, meta, value } => {
            assert_eq!(lang.as_deref(), Some("yaml"), "lang should be yaml");
            assert_eq!(*meta, None, "no meta info-string is set");
            assert_eq!(value, "foo: 1\nbar: 2", "value is the raw YAML body");
        }
        other => panic!("expected NodeKind::Code, got {other:?}"),
    }

    // The code hints request a header row, a yaml label, and highlighting.
    let hints = node.attrs.code_hints();
    assert!(hints.header_row, "header_row hint should be set");
    assert_eq!(hints.language_label.as_deref(), Some("yaml"));
    assert!(hints.highlight, "highlight hint should be set");
}

// ---------------------------------------------------------------------------
// Validity
// ---------------------------------------------------------------------------

#[test]
fn render_tree_node_validates_cleanly() {
    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    let report = validate(&node, ValidationMode::Full);
    assert!(
        report.findings.is_empty(),
        "projected YAML tree should validate with zero findings, got: {:?}",
        report.findings
    );
}

// ---------------------------------------------------------------------------
// Body parity (plain tree rendering, no hook)
// ---------------------------------------------------------------------------

#[test]
fn plain_tree_rendering_preserves_body_and_label() {
    let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
    let node = block.render_tree_node().expect("tree node");

    let output = render_tree_plain(&node);
    let plain = strip_escape_codes(&output);

    assert!(plain.contains("foo: 1"), "body line preserved: {plain:?}");
    assert!(plain.contains("bar: 2"), "body line preserved: {plain:?}");
    assert!(plain.contains("yaml"), "yaml label preserved: {plain:?}");
}

// ---------------------------------------------------------------------------
// Empty YAML
// ---------------------------------------------------------------------------

#[test]
fn empty_yaml_projects_and_renders_with_label() {
    let block = YamlBlock::new("").unwrap();
    let node = block.render_tree_node().expect("empty YAML still projects");

    match &node.kind {
        NodeKind::Code { lang, value, .. } => {
            assert_eq!(lang.as_deref(), Some("yaml"));
            assert_eq!(value, "", "empty YAML projects an empty body");
        }
        other => panic!("expected NodeKind::Code, got {other:?}"),
    }

    let output = render_tree_plain(&node);
    let plain = strip_escape_codes(&output);
    assert!(
        !plain.trim().is_empty(),
        "empty YAML should still render non-empty output"
    );
    assert!(plain.contains("yaml"), "yaml label preserved: {plain:?}");
}

// ---------------------------------------------------------------------------
// Hook parity — a stub CodeRenderer is consulted
// ---------------------------------------------------------------------------

/// A stub [`CodeRenderer`] that returns a recognizable marker string so a
/// test can confirm the tree renderer consulted the hook.
struct StubCodeRenderer;

impl CodeRenderer for StubCodeRenderer {
    fn render_terminal_code(
        &self,
        lang: Option<&str>,
        value: &str,
        attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        let hints = attrs.code_hints();
        Some(format!(
            "STUB[lang={};width={};header={};body={}]",
            lang.unwrap_or("?"),
            context.width(),
            hints.header_row,
            value,
        ))
    }

    fn render_browser_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _attrs: &NodeAttrs,
    ) -> Option<renderable::browser::fragment::BrowserFragment<renderable::browser::fragment::Ready>>
    {
        None
    }
}

#[test]
fn code_renderer_hook_is_used_when_set() {
    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    let term = Terminal::new_optimistic(80);
    let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn)
        .with_code_renderer(Rc::new(StubCodeRenderer));
    let output = render_terminal_node(&node, &opts)
        .expect("tree render should succeed")
        .output;

    // The stub's marker proves the hook was consulted; the hints and body
    // were threaded through to it.
    assert!(
        output.contains("STUB[lang=yaml;width=80;header=true;body=foo: 1]"),
        "code renderer hook should be consulted: {output:?}"
    );
}

#[test]
fn code_renderer_hook_absent_falls_back_to_plain() {
    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    // No hook: built-in plain rendering preserves the body verbatim.
    let output = render_tree_plain(&node);
    let plain = strip_escape_codes(&output);
    assert!(!plain.contains("STUB["), "no hook output should appear");
    assert!(plain.contains("foo: 1"), "plain body preserved: {plain:?}");
}

// ---------------------------------------------------------------------------
// Markdown output — the YAML fence is preserved
// ---------------------------------------------------------------------------

#[test]
fn render_markdown_node_preserves_yaml_fence() {
    let block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
    let node = block.render_tree_node().expect("tree node");

    let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
        .expect("markdown render should succeed");

    assert!(
        rendered.output.contains("```yaml"),
        "markdown output should open a yaml fence: {:?}",
        rendered.output
    );
    assert!(
        rendered.output.contains("foo: 1") && rendered.output.contains("bar: 2"),
        "markdown output should preserve the YAML body: {:?}",
        rendered.output
    );
    assert!(
        rendered.output.trim_end().ends_with("```"),
        "markdown output should close the fence: {:?}",
        rendered.output
    );
}

// ---------------------------------------------------------------------------
// TerminalCodeContext pass-through tests
// ---------------------------------------------------------------------------

use biscuit_terminal::discovery::detection::{ColorDepth, ColorMode};
use renderable::color::{ColorDepth as RenderColorDepth, ColorMode as RenderColorMode};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// A stub [`CodeRenderer`] that captures the context it receives.
struct CapturingCodeRenderer {
    captured_width: AtomicU32,
    captured_color_depth: Mutex<Option<RenderColorDepth>>,
    captured_color_mode: Mutex<Option<RenderColorMode>>,
}

impl CapturingCodeRenderer {
    fn new() -> Self {
        Self {
            captured_width: AtomicU32::new(0),
            captured_color_depth: Mutex::new(None),
            captured_color_mode: Mutex::new(None),
        }
    }

    fn captured_width(&self) -> u32 {
        self.captured_width.load(Ordering::Relaxed)
    }

    fn captured_color_depth(&self) -> Option<RenderColorDepth> {
        *self.captured_color_depth.lock().unwrap()
    }

    fn captured_color_mode(&self) -> Option<RenderColorMode> {
        *self.captured_color_mode.lock().unwrap()
    }
}

impl CodeRenderer for CapturingCodeRenderer {
    fn render_terminal_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        self.captured_width.store(context.width(), Ordering::Relaxed);
        *self.captured_color_depth.lock().unwrap() = Some(context.color_depth());
        *self.captured_color_mode.lock().unwrap() = Some(context.color_mode());
        Some("CAPTURED".to_string())
    }

    fn render_browser_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _attrs: &NodeAttrs,
    ) -> Option<renderable::browser::fragment::BrowserFragment<renderable::browser::fragment::Ready>>
    {
        None
    }
}

/// Test (a): available_width is passed through, NOT root width.
#[test]
fn context_passes_available_width_not_root_width() {
    use biscuit_terminal::render_tree::TerminalRenderContext;

    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    // Build a context where available_width differs from root width
    let term = Terminal::new_optimistic(120);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.available_width = 80; // Different from root width (120)

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = TerminalRenderOptions::from_context(context, RenderStrictness::Warn)
        .with_code_renderer(renderer.clone());

    let _ = render_terminal_node(&node, &opts).expect("render");

    assert_eq!(
        renderer.captured_width(),
        80,
        "should pass available_width (80), not root width (120)"
    );
}

/// Test (b): ColorDepth is mapped correctly from terminal context.
#[test]
fn context_passes_color_depth() {
    use biscuit_terminal::render_tree::TerminalRenderContext;

    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    // Manually set color_depth to Enhanced
    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_depth = ColorDepth::Enhanced;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = TerminalRenderOptions::from_context(context, RenderStrictness::Warn)
        .with_code_renderer(renderer.clone());

    let _ = render_terminal_node(&node, &opts).expect("render");

    assert_eq!(
        renderer.captured_color_depth(),
        Some(RenderColorDepth::Enhanced),
        "ColorDepth::Enhanced should map to renderable's ColorDepth::Enhanced"
    );
}

/// Test (c): ColorMode is mapped correctly, including Unknown.
#[test]
fn context_passes_color_mode_including_unknown() {
    use biscuit_terminal::render_tree::TerminalRenderContext;

    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    // Test Unknown specifically
    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_mode = ColorMode::Unknown;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = TerminalRenderOptions::from_context(context, RenderStrictness::Warn)
        .with_code_renderer(renderer.clone());

    let _ = render_terminal_node(&node, &opts).expect("render");

    assert_eq!(
        renderer.captured_color_mode(),
        Some(RenderColorMode::Unknown),
        "ColorMode::Unknown should map to renderable's ColorMode::Unknown"
    );
}

/// Test (d): No ambient influence — explicit context values override environment.
#[test]
fn context_ignores_ambient_environment() {
    use biscuit_terminal::render_tree::TerminalRenderContext;

    let block = YamlBlock::new("foo: 1").unwrap();
    let node = block.render_tree_node().expect("tree node");

    // Set explicit context to None (no color)
    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_depth = ColorDepth::None;
    context.color_mode = ColorMode::Light;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = TerminalRenderOptions::from_context(context, RenderStrictness::Warn)
        .with_code_renderer(renderer.clone());

    // Even if environment has conflicting values, the explicit context should win.
    // (We can't easily set env vars in a test without affecting other tests,
    // but the point is that the context is passed directly without re-detection.)
    let _ = render_terminal_node(&node, &opts).expect("render");

    assert_eq!(
        renderer.captured_color_depth(),
        Some(RenderColorDepth::None),
        "explicit ColorDepth::None should be passed, not ambient detection"
    );
    assert_eq!(
        renderer.captured_color_mode(),
        Some(RenderColorMode::Light),
        "explicit ColorMode::Light should be passed, not ambient detection"
    );
}
