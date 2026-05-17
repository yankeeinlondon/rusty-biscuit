//! Pass-through tests for the terminal tree renderer's `CodeRenderer` hook.
//!
//! These verify the call-site contract in
//! `biscuit-terminal/lib/src/render_tree/render.rs`: that `render_code_node`
//! builds a [`TerminalCodeContext`] from the [`TerminalRenderContext`] and
//! passes it to a `CodeRenderer` faithfully — using `available_width` (not
//! root `width`), mapping color depth and color mode through the boundary
//! `From` impls, and never re-detecting capabilities from the ambient
//! environment.
//!
//! A plain `RenderNode::code(...)` node is constructed directly so the test
//! exercises the `biscuit-terminal` renderer contract without depending on a
//! component (e.g. `YamlBlock`) from another crate.

use std::rc::Rc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use biscuit_terminal::discovery::detection::{ColorDepth, ColorMode};
use biscuit_terminal::render_tree::{
    TerminalRenderContext, TerminalRenderOptions, render_terminal_node,
};
use biscuit_terminal::terminal::Terminal;
use renderable::color::{
    ColorDepth as RenderColorDepth, ColorMode as RenderColorMode, TerminalCodeContext,
};
use renderable::tree::{CodeRenderer, NodeAttrs, RenderNode, RenderStrictness};
use serial_test::serial;

/// A stub [`CodeRenderer`] that captures the [`TerminalCodeContext`] it
/// receives so a test can assert on the call-site contract.
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
        self.captured_width
            .store(context.width(), Ordering::Relaxed);
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

/// Builds a plain code node for the renderer to fold.
fn code_node() -> RenderNode {
    RenderNode::code(Some("yaml".to_string()), None, "foo: 1")
}

/// Builds render options from an explicit context plus a capturing renderer.
fn options_with(
    context: TerminalRenderContext,
    renderer: Rc<CapturingCodeRenderer>,
) -> TerminalRenderOptions {
    let mut opts = TerminalRenderOptions::new(&context.terminal, RenderStrictness::Warn);
    opts.context = context;
    opts.with_code_renderer(renderer)
}

/// Test (a): the hook receives `available_width`, not root `width`.
#[test]
fn context_passes_available_width_not_root_width() {
    let term = Terminal::new_optimistic(120);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.available_width = 80; // Differs from root width (120).

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = options_with(context, renderer.clone());

    let _ = render_terminal_node(&code_node(), &opts).expect("render should succeed");

    assert_eq!(
        renderer.captured_width(),
        80,
        "the hook should receive available_width (80), not root width (120)"
    );
}

/// Test (b): the configured `ColorDepth` is mapped through and passed.
#[test]
fn context_passes_configured_color_depth() {
    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_depth = ColorDepth::Enhanced;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = options_with(context, renderer.clone());

    let _ = render_terminal_node(&code_node(), &opts).expect("render should succeed");

    assert_eq!(
        renderer.captured_color_depth(),
        Some(RenderColorDepth::Enhanced),
        "ColorDepth::Enhanced should map to renderable's ColorDepth::Enhanced"
    );
}

/// Test (c): the configured `ColorMode` is mapped through, including `Unknown`.
#[test]
fn context_passes_configured_color_mode_including_unknown() {
    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_mode = ColorMode::Unknown;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = options_with(context, renderer.clone());

    let _ = render_terminal_node(&code_node(), &opts).expect("render should succeed");

    assert_eq!(
        renderer.captured_color_mode(),
        Some(RenderColorMode::Unknown),
        "ColorMode::Unknown should map to renderable's ColorMode::Unknown"
    );
}

/// Restores a set of environment variables to their prior values on drop.
struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(vars: &[(&'static str, &str)]) -> Self {
        let saved = vars
            .iter()
            .map(|(key, value)| {
                let prev = std::env::var(key).ok();
                // SAFETY: the test is `#[serial]`, so no other thread is
                // concurrently reading or writing the environment.
                unsafe { std::env::set_var(key, value) };
                (*key, prev)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, prev) in &self.saved {
            // SAFETY: see `EnvGuard::set`.
            match prev {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

/// Test (d): conflicting ambient environment variables do not influence the
/// context the hook receives — the explicitly built `TerminalRenderContext`
/// wins.
///
/// This is the regression the spec (§9d) targets: accidental re-detection via
/// `COLORTERM` / `NO_COLOR` / `COLORFGBG` would override the render context.
#[test]
#[serial]
fn context_ignores_conflicting_ambient_environment() {
    // Ambient environment claims true-color on a dark background, and also
    // sets NO_COLOR — all conflicting with the explicit context below.
    let _env = EnvGuard::set(&[
        ("COLORTERM", "truecolor"),
        ("NO_COLOR", "1"),
        ("COLORFGBG", "15;0"),
    ]);

    let term = Terminal::new_optimistic(80);
    let mut context = TerminalRenderContext::from_terminal(&term);
    context.color_depth = ColorDepth::Basic;
    context.color_mode = ColorMode::Light;

    let renderer = Rc::new(CapturingCodeRenderer::new());
    let opts = options_with(context, renderer.clone());

    let _ = render_terminal_node(&code_node(), &opts).expect("render should succeed");

    assert_eq!(
        renderer.captured_color_depth(),
        Some(RenderColorDepth::Basic),
        "explicit ColorDepth::Basic must win over ambient COLORTERM/NO_COLOR"
    );
    assert_eq!(
        renderer.captured_color_mode(),
        Some(RenderColorMode::Light),
        "explicit ColorMode::Light must win over ambient COLORFGBG"
    );
}
