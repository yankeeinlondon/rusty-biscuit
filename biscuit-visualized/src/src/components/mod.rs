//! Diagram components for terminal, browser, and render-tree output.
//!
//! [`MermaidDiagram`], [`GraphExpression`], and [`GitGraph`] decide what a
//! diagram shows and render it for every target; `biscuit-terminal` only
//! puts the resulting pixels on screen. They need the `terminal` feature.
//!
//! These are distinct from the artifact-level renderers they wrap
//! ([`crate::mermaid::MermaidDiagram`], [`crate::graph::GraphDiagram`]), which
//! produce SVG/PNG files and know nothing about terminals.
//!
//! [`TerminalDiagramRenderer`] is the render-tree hook that lets
//! `biscuit-terminal`'s tree renderer draw Mermaid code fences.

mod display;
mod fallback_policy;
pub mod git_graph;
pub mod graph_expression;
pub mod mermaid;
mod render_hook;

pub use git_graph::{
    GitGraph, GitGraphPlan, GraphLine, GraphPullRequest, GraphViewport, LaneEntry, LaneMerge,
};
pub use graph_expression::{GraphExpression, GraphInputSyntax, GraphOrientation, GraphRenderError};
#[cfg(feature = "clap")]
pub use fallback_policy::MermaidFallbackFeature;
pub use fallback_policy::{
    MERMAID_FALLBACK_ENV, MermaidFallbackPolicy, ResolvedFallbackPolicy, TextFallback,
};
pub use mermaid::{
    MermaidConfig, MermaidDiagram, MermaidPresentation, MermaidRenderError, MermaidRenderResult,
    MermaidTheme, QuadrantTheme, PARTIAL_NOTE_ID, TEXT_BLOCK_CHROME_COLUMNS, partial_note,
};
pub use render_hook::TerminalDiagramRenderer;
