//! Projection layer for converting terminal content to render tree nodes.
//!
//! This module provides [`TreeProjectionContext`] and the
//! [`to_tree_nodes`](RenderableTerminalContent::to_tree_nodes) method for
//! projecting [`RenderableTerminalContent`] into the canonical render tree.
//!
//! ## Behavior
//!
//! - `String(s)` projects to a [`RenderNode::text`].
//! - `Component(c)` calls [`render_tree_node`](TerminalRenderable::render_tree_node):
//!   - `Some(node)` — included directly in the result.
//!   - `None` — behavior depends on [`RenderStrictness`]:
//!     - `Strict` — produces an [`Unsupported`](renderable::tree::NodeKind::Unsupported) node with an error diagnostic.
//!     - `Warn` — renders a fallback (ANSI-stripped text) with a warning diagnostic.
//!     - `Lossy` — renders a fallback silently.
//!
//! Recursion depth is guarded; overflow produces an [`Unsupported`](renderable::tree::NodeKind::Unsupported)
//! node and a structural diagnostic.
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::components::renderable::RenderableTerminalContent;
//! use biscuit_terminal::render_tree::projection::{TreeProjectionContext, ProjectionResult};
//!
//! let content = RenderableTerminalContent::String("Hello".into());
//! let mut ctx = TreeProjectionContext::default();
//! let result = content.to_tree_nodes(&mut ctx);
//!
//! assert_eq!(result.nodes.len(), 1);
//! assert!(result.diagnostics.is_empty());
//! ```

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use renderable::tree::{
    Diagnostic, DiagnosticKind, NodeKind, RenderNode, RenderStrictness, Severity,
};
use tracing::{debug, warn};

use crate::components::prose::Prose;
use crate::components::renderable::RenderableTerminalContent;
use crate::discovery::eval::strip_ansi_codes;
use crate::terminal::Terminal;

/// Set of component type names that have already produced a `Warn`-mode
/// fallback diagnostic. The first occurrence for any given type emits
/// `tracing::warn!`; subsequent occurrences emit `tracing::debug!` so the
/// signal stays loud without flooding logs in long-running processes.
fn warned_types() -> &'static Mutex<HashSet<&'static str>> {
    static WARNED_TYPES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    WARNED_TYPES.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Emits a fallback observability event for a component that produced `None`
/// from `render_tree_node` under [`RenderStrictness::Warn`].
///
/// The first call for a given `type_name` emits `tracing::warn!`. Subsequent
/// calls for the same type emit `tracing::debug!` so the warn signal stays
/// useful in long-running processes without being silenced.
fn emit_fallback_event(type_name: &'static str) {
    let is_first = {
        let mut guard = warned_types().lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(type_name)
    };
    if is_first {
        warn!(
            component = type_name,
            "component does not implement render_tree_node; falling back to ANSI-stripped text"
        );
    } else {
        debug!(
            component = type_name,
            "component does not implement render_tree_node; falling back to ANSI-stripped text"
        );
    }
}

/// Test-only helper to reset the warn-once cache so tests are not
/// order-dependent on the global state.
#[cfg(test)]
fn reset_warned_types_for_test() {
    if let Ok(mut guard) = warned_types().lock() {
        guard.clear();
    }
}

/// Context for projecting terminal content to render tree nodes.
///
/// The context tracks recursion depth and strictness policy. The default
/// strictness is [`RenderStrictness::Warn`] with a maximum depth of 100.
#[derive(Debug, Clone)]
pub struct TreeProjectionContext {
    /// How strictly to handle unsupported content.
    pub strictness: RenderStrictness,
    /// Maximum recursion depth to prevent infinite loops.
    pub max_depth: usize,
    /// Current recursion depth.
    pub current_depth: usize,
}

impl Default for TreeProjectionContext {
    fn default() -> Self {
        Self {
            strictness: RenderStrictness::Warn,
            max_depth: 100,
            current_depth: 0,
        }
    }
}

impl TreeProjectionContext {
    /// Creates a new context with the given strictness.
    #[must_use]
    pub fn with_strictness(strictness: RenderStrictness) -> Self {
        Self {
            strictness,
            ..Default::default()
        }
    }

    /// Returns `true` if the current depth has reached the maximum.
    #[must_use]
    pub fn is_depth_exceeded(&self) -> bool {
        self.current_depth >= self.max_depth
    }

    /// Increments the recursion depth and returns `true` if the new depth
    /// exceeds the maximum.
    pub fn enter(&mut self) -> bool {
        self.current_depth += 1;
        self.is_depth_exceeded()
    }

    /// Decrements the recursion depth.
    pub fn exit(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }
}

/// Result of projecting content to tree nodes.
#[derive(Debug, Clone)]
pub struct ProjectionResult {
    /// The projected nodes.
    pub nodes: Vec<RenderNode>,
    /// Diagnostics encountered during projection.
    pub diagnostics: Vec<Diagnostic>,
}

impl ProjectionResult {
    /// Creates an empty result with no nodes or diagnostics.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Creates a result with a single node and no diagnostics.
    #[must_use]
    pub fn single(node: RenderNode) -> Self {
        Self {
            nodes: vec![node],
            diagnostics: Vec::new(),
        }
    }

    /// Creates a result with a single node and a single diagnostic.
    #[must_use]
    pub fn with_diagnostic(node: RenderNode, diagnostic: Diagnostic) -> Self {
        Self {
            nodes: vec![node],
            diagnostics: vec![diagnostic],
        }
    }

    /// Merges another result into this one.
    pub fn merge(&mut self, other: ProjectionResult) {
        self.nodes.extend(other.nodes);
        self.diagnostics.extend(other.diagnostics);
    }
}

/// Mode for [`project_renderable_content`] controlling how a non-`Prose`
/// component is projected.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ProjectionMode<'a> {
    /// Allow block-level projections through the canonical render tree.
    ///
    /// When a non-`Prose` component implements
    /// [`render_tree_node`](crate::components::renderable::TerminalRenderable::render_tree_node),
    /// its returned node is used directly (block or inline). When the
    /// component has no tree projection and `terminal_hint` is set, the
    /// terminal is used to render an ANSI-stripped `Text` fallback. Used by
    /// containers whose children may be block-level (e.g. `OrderedList`,
    /// `UnorderedList`, `Compose`).
    Structural { terminal_hint: Option<&'a Terminal> },

    /// Force every non-`Prose` component to flatten into a single
    /// ANSI-stripped `Text` node, regardless of whether the component
    /// implements `render_tree_node`. Used by containers whose children must
    /// be inline (e.g. `BlockQuote` wrapping content in a `Paragraph`).
    InlineOnly,
}

/// Shared projection helper for container components.
///
/// This consolidates the Prose-downcast pattern used by container components
/// ([`BlockQuote`](crate::components::block_quote::BlockQuote),
/// [`Compose`](crate::components::compose::Compose),
/// [`OrderedList`](crate::components::list::OrderedList),
/// [`UnorderedList`](crate::components::list::UnorderedList)) when projecting
/// their child [`RenderableTerminalContent`] into render-tree nodes.
///
/// ## Behavior
///
/// - [`RenderableTerminalContent::String`] becomes a single
///   [`RenderNode::text`] regardless of mode.
/// - [`RenderableTerminalContent::Component`]:
///   - If the component downcasts to [`Prose`], its inline structure is
///     projected through [`Prose::to_render_nodes`] so bold/italic/colored
///     runs survive as structured inline nodes
///     (`Strong` / `Emphasis` / styled `Span`) — preserving the terminal SGR
///     lowering through the tree renderer.
///   - Otherwise, behavior depends on `mode`:
///     - [`ProjectionMode::Structural`] — preferred for block-level-capable
///       containers. If the component has no canonical `render_tree_node`
///       and a `terminal_hint` is provided, the component is rendered
///       through that terminal and the ANSI-stripped output is wrapped as a
///       single `Text` node. Threading the actual terminal keeps
///       capability-sensitive fallbacks (for example a text-only terminal
///       staying on `HorizontalRule`'s Unicode tier instead of jumping to
///       the Kitty image tier) honest. Otherwise falls back to
///       [`RenderableTerminalContent::to_tree_nodes`] under
///       [`RenderStrictness::Warn`].
///     - [`ProjectionMode::InlineOnly`] — forces every non-`Prose`
///       component into a single ANSI-stripped `Text` node via the
///       component's optimistic render. This preserves the historical
///       `BlockQuote` flattening that keeps a `Paragraph`'s children
///       inline.
///
/// Diagnostics from the structural fallback path are intentionally swallowed
/// silently — this helper has no [`Diagnostic`] sink in its return type, and
/// container callers (`OrderedList`, `UnorderedList`, `Compose`, `BlockQuote`)
/// project many child nodes per render and have no clear place to surface
/// per-child diagnostics in their user-facing output. Callers that need
/// diagnostics should call [`RenderableTerminalContent::to_tree_nodes`]
/// directly.
///
/// ## Wrapping
///
/// This helper deliberately does **not** wrap the returned nodes in a
/// `Paragraph`, `ListItem`, or other container. Wrapping is a
/// caller-dependent decision:
///
/// - `BlockQuote::paragraph_children` returns inline nodes; its caller wraps
///   them in a single `Paragraph` so all siblings share one block.
/// - `Compose::project_part` does not wrap because Compose's outer container
///   is a `Root` whose children are explicit sequence siblings.
/// - `OrderedList` / `UnorderedList` wrap the returned nodes in a
///   `Paragraph` *inside* a `ListItem` so the list renderer's prefix attaches
///   to a single inline block instead of misclassifying sibling inline nodes
///   as block children.
///
/// ## Examples
///
/// ```ignore
/// // `project_renderable_content` and `ProjectionMode` are `pub(crate)`;
/// // this snippet illustrates the in-crate usage pattern and is therefore
/// // not runnable from outside the crate.
/// use biscuit_terminal::components::renderable::RenderableTerminalContent;
/// use biscuit_terminal::render_tree::projection::{
///     project_renderable_content, ProjectionMode,
/// };
///
/// let content = RenderableTerminalContent::String("Hello".into());
/// let nodes = project_renderable_content(
///     &content,
///     ProjectionMode::Structural { terminal_hint: None },
/// );
/// assert_eq!(nodes.len(), 1);
/// ```
#[must_use]
pub(crate) fn project_renderable_content(
    content: &RenderableTerminalContent,
    mode: ProjectionMode<'_>,
) -> Vec<RenderNode> {
    match content {
        RenderableTerminalContent::String(s) => vec![RenderNode::text(s)],
        RenderableTerminalContent::Component(component) => {
            if let Some(prose) = component.as_any().downcast_ref::<Prose>() {
                return prose.to_render_nodes();
            }
            match mode {
                ProjectionMode::InlineOnly => {
                    let stripped = strip_ansi_codes(&component.render_optimistic(None));
                    vec![RenderNode::text(stripped)]
                }
                ProjectionMode::Structural { terminal_hint } => {
                    // For bespoke-only components, prefer rendering through
                    // the caller's actual terminal so capability-sensitive
                    // fallbacks (e.g. HR text tier vs. image tier) reflect
                    // the real target. The warn-once fallback event still
                    // fires here so a component that forgets to override
                    // `render_tree_node` is observable in logs and CI even
                    // when the projector takes this terminal-hint
                    // short-circuit instead of routing through
                    // `to_tree_nodes`'s strictness ladder.
                    if component.render_tree_node().is_none()
                        && let Some(term) = terminal_hint
                    {
                        emit_fallback_event(component.type_name());
                        let rendered = component.render(term);
                        let stripped = strip_ansi_codes(&rendered);
                        return vec![RenderNode::text(stripped)];
                    }
                    let mut ctx = TreeProjectionContext::default();
                    content.to_tree_nodes(&mut ctx).nodes
                }
            }
        }
    }
}

/// Folds a flat sequence of [`Prose`]-projected nodes into block-level
/// children.
///
/// Contiguous inline nodes are flushed into a single
/// [`Paragraph`](renderable::tree::NodeKind::Paragraph); each block-level
/// [`Code`](renderable::tree::NodeKind::Code) node — the only block-level node
/// the Prose parser emits — is preserved as a block-level sibling.
///
/// Containers that embed `Prose` via [`Prose::to_render_nodes`] must fold the
/// sequence this way. Wrapping the whole sequence in one `Paragraph` nests a
/// block-level `Code` inside phrasing content, which render-tree validation
/// rejects (the terminal renderer then emits empty output and the Markdown /
/// HTML renderers surface a validation error).
///
/// This is the shared producer behind [`Prose`]'s own
/// [`render_tree`](TreeRenderable::render_tree) split and the `BlockQuote` /
/// list-item container projections, so all three stay in lockstep. When the
/// sequence is purely inline (the common case) it returns a single
/// `Paragraph`, matching the historical container shape exactly.
///
/// [`Prose`]: crate::components::prose::Prose
/// [`Prose::to_render_nodes`]: crate::components::prose::Prose::to_render_nodes
/// [`TreeRenderable::render_tree`]: renderable::tree::TreeRenderable::render_tree
#[must_use]
pub(crate) fn fold_prose_nodes_into_blocks(nodes: Vec<RenderNode>) -> Vec<RenderNode> {
    let mut blocks: Vec<RenderNode> = Vec::new();
    let mut paragraph: Vec<RenderNode> = Vec::new();
    for node in nodes {
        if matches!(node.kind, NodeKind::Code { .. }) {
            if !paragraph.is_empty() {
                blocks.push(RenderNode::paragraph(std::mem::take(&mut paragraph)));
            }
            blocks.push(node);
        } else {
            paragraph.push(node);
        }
    }
    if !paragraph.is_empty() {
        blocks.push(RenderNode::paragraph(paragraph));
    }
    blocks
}

impl RenderableTerminalContent {
    /// Projects this content to render tree nodes.
    ///
    /// ## Behavior
    ///
    /// - `String(s)` produces a single [`RenderNode::text(s)`].
    /// - `Component(c)` calls [`render_tree_node`](TerminalRenderable::render_tree_node):
    ///   - `Some(node)` — the node is included directly.
    ///   - `None` — fallback behavior depends on [`RenderStrictness`]:
    ///     - `Strict` — an [`Unsupported`](renderable::tree::NodeKind::Unsupported) node
    ///       is produced with an error diagnostic.
    ///     - `Warn` — ANSI-stripped fallback text is produced with a warning diagnostic.
    ///     - `Lossy` — ANSI-stripped fallback text is produced silently.
    ///
    /// Recursion depth is tracked by the context. If the maximum depth is
    /// exceeded, an [`Unsupported`](renderable::tree::NodeKind::Unsupported)
    /// node is produced with a structural diagnostic.
    ///
    /// All projected nodes use [`SourceSpan::synthetic()`](renderable::tree::SourceSpan::synthetic).
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_terminal::components::renderable::RenderableTerminalContent;
    /// use biscuit_terminal::render_tree::projection::TreeProjectionContext;
    ///
    /// let content = RenderableTerminalContent::String("Hello, world!".into());
    /// let mut ctx = TreeProjectionContext::default();
    /// let result = content.to_tree_nodes(&mut ctx);
    ///
    /// assert_eq!(result.nodes.len(), 1);
    /// ```
    #[must_use]
    pub fn to_tree_nodes(&self, ctx: &mut TreeProjectionContext) -> ProjectionResult {
        // Check recursion depth before processing
        if ctx.enter() {
            ctx.exit();
            let diagnostic = Diagnostic {
                kind: DiagnosticKind::Structural,
                severity: Severity::Error,
                message: "maximum recursion depth exceeded during tree projection".into(),
                span: None,
            };
            return ProjectionResult::with_diagnostic(
                RenderNode::unsupported("recursion depth exceeded"),
                diagnostic,
            );
        }

        let result = match self {
            RenderableTerminalContent::String(s) => ProjectionResult::single(RenderNode::text(s)),

            RenderableTerminalContent::Component(component) => {
                match component.render_tree_node() {
                    Some(node) => ProjectionResult::single(node),
                    None => {
                        // Component does not support tree rendering — apply strictness policy.
                        // Use the stable Rust type name rather than a Debug-derived
                        // heuristic so diagnostic labels are derive-format-independent.
                        let type_label = component.type_name();

                        match ctx.strictness {
                            RenderStrictness::Strict => {
                                let diagnostic = Diagnostic {
                                    kind: DiagnosticKind::Unsupported,
                                    severity: Severity::Error,
                                    message: format!(
                                        "component '{type_label}' does not support tree rendering"
                                    ),
                                    span: None,
                                };
                                ProjectionResult::with_diagnostic(
                                    RenderNode::unsupported(type_label),
                                    diagnostic,
                                )
                            }
                            RenderStrictness::Warn => {
                                // Render to terminal and strip ANSI codes for fallback
                                let term = Terminal::new_optimistic(80);
                                let rendered = component.render(&term);
                                let stripped = strip_ansi_codes(&rendered);

                                emit_fallback_event(type_label);

                                let diagnostic = Diagnostic {
                                    kind: DiagnosticKind::Lossy,
                                    severity: Severity::Warning,
                                    message: format!(
                                        "component '{type_label}' rendered as plain text fallback"
                                    ),
                                    span: None,
                                };
                                ProjectionResult::with_diagnostic(
                                    RenderNode::text(stripped),
                                    diagnostic,
                                )
                            }
                            RenderStrictness::Lossy => {
                                // Render to terminal and strip ANSI codes silently
                                let term = Terminal::new_optimistic(80);
                                let rendered = component.render(&term);
                                let stripped = strip_ansi_codes(&rendered);

                                ProjectionResult::single(RenderNode::text(stripped))
                            }
                        }
                    }
                }
            }
        };

        ctx.exit();
        result
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::rc::Rc;

    use renderable::tree::{NodeKind, RenderStrictness, Severity};

    use super::*;
    use crate::components::renderable::TerminalRenderable;
    use crate::utils::layout::Layout;

    /// A stub component that returns `Some(RenderNode::text("stub"))` from `render_tree_node`.
    #[derive(Debug)]
    struct StubTreeComponent {
        layout: Layout,
    }

    impl StubTreeComponent {
        fn new() -> Self {
            Self {
                layout: Layout::default(),
            }
        }
    }

    impl TerminalRenderable for StubTreeComponent {
        fn render(&self, _term: &Terminal) -> String {
            "stub terminal output".to_string()
        }

        fn layout(&self) -> &Layout {
            &self.layout
        }

        fn layout_mut(&mut self) -> &mut Layout {
            &mut self.layout
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn render_tree_node(&self) -> Option<RenderNode> {
            Some(RenderNode::text("stub"))
        }
    }

    /// A stub component that returns `None` from `render_tree_node` (bespoke-only).
    #[derive(Debug)]
    struct StubBespokeOnly {
        layout: Layout,
    }

    impl StubBespokeOnly {
        fn new() -> Self {
            Self {
                layout: Layout::default(),
            }
        }
    }

    impl TerminalRenderable for StubBespokeOnly {
        fn render(&self, _term: &Terminal) -> String {
            "\x1b[31mbespoke\x1b[0m output".to_string()
        }

        fn layout(&self) -> &Layout {
            &self.layout
        }

        fn layout_mut(&mut self) -> &mut Layout {
            &mut self.layout
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        // render_tree_node defaults to None
    }

    #[test]
    fn string_content_produces_text_node() {
        let content = RenderableTerminalContent::String("hello world".into());
        let mut ctx = TreeProjectionContext::default();
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert!(result.diagnostics.is_empty());
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Text { value } if value == "hello world"
        ));
    }

    #[test]
    fn tree_component_uses_returned_node() {
        let component = StubTreeComponent::new();
        let content = RenderableTerminalContent::Component(Rc::new(component));
        let mut ctx = TreeProjectionContext::default();
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert!(result.diagnostics.is_empty());
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Text { value } if value == "stub"
        ));
    }

    #[test]
    fn bespoke_only_with_strict_produces_unsupported() {
        let component = StubBespokeOnly::new();
        let content = RenderableTerminalContent::Component(Rc::new(component));
        let mut ctx = TreeProjectionContext::with_strictness(RenderStrictness::Strict);
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Unsupported { label } if label.contains("StubBespokeOnly")
        ));
        assert_eq!(result.diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn bespoke_only_with_warn_produces_fallback_and_diagnostic() {
        let component = StubBespokeOnly::new();
        let content = RenderableTerminalContent::Component(Rc::new(component));
        let mut ctx = TreeProjectionContext::with_strictness(RenderStrictness::Warn);
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.diagnostics.len(), 1);
        // Fallback is ANSI-stripped text
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Text { value } if value == "bespoke output"
        ));
        assert_eq!(result.diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn bespoke_only_with_lossy_produces_fallback_silently() {
        let component = StubBespokeOnly::new();
        let content = RenderableTerminalContent::Component(Rc::new(component));
        let mut ctx = TreeProjectionContext::with_strictness(RenderStrictness::Lossy);
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert!(result.diagnostics.is_empty());
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Text { value } if value == "bespoke output"
        ));
    }

    #[test]
    fn recursion_overflow_produces_diagnostic() {
        let content = RenderableTerminalContent::String("test".into());
        let mut ctx = TreeProjectionContext {
            strictness: RenderStrictness::Warn,
            max_depth: 5,
            current_depth: 5, // Already at max
        };
        let result = content.to_tree_nodes(&mut ctx);

        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(matches!(
            &result.nodes[0].kind,
            NodeKind::Unsupported { label } if label.contains("recursion")
        ));
        assert_eq!(result.diagnostics[0].severity, Severity::Error);
    }

    #[test]
    fn context_depth_tracking() {
        let mut ctx = TreeProjectionContext::default();
        assert_eq!(ctx.current_depth, 0);

        assert!(!ctx.enter()); // Returns false (not exceeded)
        assert_eq!(ctx.current_depth, 1);

        ctx.exit();
        assert_eq!(ctx.current_depth, 0);

        // Saturating sub prevents underflow
        ctx.exit();
        assert_eq!(ctx.current_depth, 0);
    }

    /// Deliberately un-overridden `TerminalRenderable` stub used only by
    /// `warn_fallback_emits_warn_once_then_debug`. The unique nested-module
    /// type path guarantees `type_name` collision risk with other tests
    /// or production components is zero.
    mod warn_once_stub {
        use std::any::Any;

        use super::super::*;
        use crate::components::renderable::TerminalRenderable;
        use crate::utils::layout::Layout;

        #[derive(Debug, Default)]
        pub(super) struct StubWarnOnceFallback {
            layout: Layout,
        }

        impl TerminalRenderable for StubWarnOnceFallback {
            fn render(&self, _term: &Terminal) -> String {
                "warn-once stub output".to_string()
            }

            fn layout(&self) -> &Layout {
                &self.layout
            }

            fn layout_mut(&mut self) -> &mut Layout {
                &mut self.layout
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            // render_tree_node intentionally NOT overridden — defaults to None.
            // type_name intentionally NOT overridden — defaults to std::any::type_name::<Self>().
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn warn_fallback_emits_warn_once_then_debug() {
        // Clear the global warn-once state so this test does not depend on
        // execution order relative to other tests in the suite.
        reset_warned_types_for_test();

        let stub_type_name = std::any::type_name::<warn_once_stub::StubWarnOnceFallback>();

        // First projection — should hit the Warn fallback and emit `warn!`.
        let content1 = RenderableTerminalContent::Component(Rc::new(
            warn_once_stub::StubWarnOnceFallback::default(),
        ));
        let mut ctx1 = TreeProjectionContext::with_strictness(RenderStrictness::Warn);
        let result1 = content1.to_tree_nodes(&mut ctx1);

        // Projected node is the text fallback (NodeKind::Text with stub output).
        assert_eq!(result1.nodes.len(), 1);
        assert!(matches!(
            &result1.nodes[0].kind,
            NodeKind::Text { value } if value == "warn-once stub output"
        ));

        // Diagnostic includes the stable Rust type name.
        assert_eq!(result1.diagnostics.len(), 1);
        assert!(
            result1.diagnostics[0].message.contains(stub_type_name),
            "diagnostic message {:?} should contain stable type name {:?}",
            result1.diagnostics[0].message,
            stub_type_name,
        );

        // Second projection — should downgrade to `debug!`.
        let content2 = RenderableTerminalContent::Component(Rc::new(
            warn_once_stub::StubWarnOnceFallback::default(),
        ));
        let mut ctx2 = TreeProjectionContext::with_strictness(RenderStrictness::Warn);
        let _result2 = content2.to_tree_nodes(&mut ctx2);

        // Count WARN vs DEBUG occurrences referencing the stub's type name
        // in the captured log buffer. We expect exactly one WARN and one DEBUG.
        logs_assert(|lines: &[&str]| {
            let warns = lines
                .iter()
                .filter(|l| l.contains("WARN") && l.contains("StubWarnOnceFallback"))
                .count();
            let debugs = lines
                .iter()
                .filter(|l| l.contains("DEBUG") && l.contains("StubWarnOnceFallback"))
                .count();
            if warns != 1 {
                return Err(format!(
                    "expected exactly 1 WARN line referencing StubWarnOnceFallback, got {warns}; lines={lines:?}"
                ));
            }
            if debugs != 1 {
                return Err(format!(
                    "expected exactly 1 DEBUG line referencing StubWarnOnceFallback, got {debugs}; lines={lines:?}"
                ));
            }
            Ok(())
        });
    }

    /// Distinct stub used by the terminal-hint warn-once regression so its
    /// type name does not collide with `warn_once_stub::StubWarnOnceFallback`.
    mod warn_once_terminal_hint_stub {
        use std::any::Any;

        use super::super::*;
        use crate::components::renderable::TerminalRenderable;
        use crate::utils::layout::Layout;

        #[derive(Debug, Default)]
        pub(super) struct StubWarnOnceTerminalHint {
            layout: Layout,
        }

        impl TerminalRenderable for StubWarnOnceTerminalHint {
            fn render(&self, _term: &Terminal) -> String {
                "terminal-hint stub output".to_string()
            }

            fn layout(&self) -> &Layout {
                &self.layout
            }

            fn layout_mut(&mut self) -> &mut Layout {
                &mut self.layout
            }

            fn as_any(&self) -> &dyn Any {
                self
            }

            // render_tree_node intentionally NOT overridden — defaults to None.
        }
    }

    /// Regression: `project_renderable_content` with
    /// `ProjectionMode::Structural { terminal_hint: Some(_) }` must still
    /// emit the same warn-once-then-debug observability event as the
    /// `to_tree_nodes` strictness ladder. Without this, containers that
    /// thread a terminal hint (Compose, OrderedList, UnorderedList) would
    /// silently flatten a component that forgot to override
    /// `render_tree_node`.
    #[test]
    #[tracing_test::traced_test]
    fn structural_terminal_hint_fallback_emits_warn_once_then_debug() {
        reset_warned_types_for_test();

        let term = Terminal::new_optimistic(80);

        // First projection — should hit the terminal-hint fallback and emit `warn!`.
        let content1 = RenderableTerminalContent::Component(Rc::new(
            warn_once_terminal_hint_stub::StubWarnOnceTerminalHint::default(),
        ));
        let nodes1 = project_renderable_content(
            &content1,
            ProjectionMode::Structural {
                terminal_hint: Some(&term),
            },
        );
        assert_eq!(nodes1.len(), 1);
        assert!(matches!(
            &nodes1[0].kind,
            NodeKind::Text { value } if value == "terminal-hint stub output"
        ));

        // Second projection — should downgrade to `debug!`.
        let content2 = RenderableTerminalContent::Component(Rc::new(
            warn_once_terminal_hint_stub::StubWarnOnceTerminalHint::default(),
        ));
        let _nodes2 = project_renderable_content(
            &content2,
            ProjectionMode::Structural {
                terminal_hint: Some(&term),
            },
        );

        logs_assert(|lines: &[&str]| {
            let warns = lines
                .iter()
                .filter(|l| l.contains("WARN") && l.contains("StubWarnOnceTerminalHint"))
                .count();
            let debugs = lines
                .iter()
                .filter(|l| l.contains("DEBUG") && l.contains("StubWarnOnceTerminalHint"))
                .count();
            if warns != 1 {
                return Err(format!(
                    "expected exactly 1 WARN line referencing StubWarnOnceTerminalHint, got {warns}; lines={lines:?}"
                ));
            }
            if debugs != 1 {
                return Err(format!(
                    "expected exactly 1 DEBUG line referencing StubWarnOnceTerminalHint, got {debugs}; lines={lines:?}"
                ));
            }
            Ok(())
        });
    }

    #[test]
    fn projection_result_merge() {
        let mut result1 = ProjectionResult::single(RenderNode::text("first"));
        let result2 = ProjectionResult::with_diagnostic(
            RenderNode::text("second"),
            Diagnostic::lossy("test diagnostic", None),
        );

        result1.merge(result2);

        assert_eq!(result1.nodes.len(), 2);
        assert_eq!(result1.diagnostics.len(), 1);
    }
}
