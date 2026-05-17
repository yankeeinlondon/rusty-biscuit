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

use renderable::tree::{Diagnostic, DiagnosticKind, RenderNode, RenderStrictness, Severity};

use crate::components::renderable::RenderableTerminalContent;
use crate::discovery::eval::strip_ansi_codes;
use crate::terminal::Terminal;

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
                        // Component does not support tree rendering — apply strictness policy
                        let type_name = format!("{:?}", component);
                        let type_label = type_name
                            .split_whitespace()
                            .next()
                            .unwrap_or("Component")
                            .to_string();

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
                                    RenderNode::unsupported(&type_label),
                                    diagnostic,
                                )
                            }
                            RenderStrictness::Warn => {
                                // Render to terminal and strip ANSI codes for fallback
                                let term = Terminal::new_optimistic(80);
                                let rendered = component.render(&term);
                                let stripped = strip_ansi_codes(&rendered);

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
