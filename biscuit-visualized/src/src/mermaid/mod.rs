pub mod config;
pub mod error;
pub(crate) mod gitgraph;
// Consumed by the Merman render path's pre-parse rewrites.
#[allow(dead_code)]
pub(crate) mod newline;
pub mod render;

pub use config::{MermaidConfig, MermaidTheme, QuadrantTheme};
pub use error::MermaidError;
#[doc(hidden)]
pub use gitgraph::{default_gitgraph_commit_step, Bounds, CommitGeometry, GitGraphGeometry, TagGeometry};
pub use render::{MermaidDiagram, NaturalSize};
