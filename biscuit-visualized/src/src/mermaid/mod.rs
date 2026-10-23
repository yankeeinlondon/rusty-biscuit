pub mod config;
pub mod error;
pub(crate) mod gitgraph;
pub(crate) mod gitgraph_layout;
pub(crate) mod newline;
pub(crate) mod postprocess;
pub mod render;

pub use config::{MermaidConfig, MermaidTheme, QuadrantTheme};
pub use error::MermaidError;
#[doc(hidden)]
pub use gitgraph::{default_gitgraph_commit_step, Bounds, CommitGeometry, GitGraphGeometry, TagGeometry};
pub use render::{MermaidDiagram, MermaidTarget, NaturalSize};
