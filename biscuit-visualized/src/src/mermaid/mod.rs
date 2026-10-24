pub(crate) mod accents;
pub mod color;
pub mod config;
pub(crate) mod contrast;
pub(crate) mod design;
// Wired into the HTML static render path by the embedded-font opt-in; until
// then only its tests use it.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod embed;
pub mod error;
pub(crate) mod gitgraph;
pub(crate) mod gitgraph_layout;
pub(crate) mod newline;
pub mod palette;
pub(crate) mod postprocess;
pub mod render;

pub use color::Rgb;
pub use config::{MermaidConfig, MermaidTheme, QuadrantTheme};
pub use error::MermaidError;
#[doc(hidden)]
pub use gitgraph::{default_gitgraph_commit_step, Bounds, CommitGeometry, GitGraphGeometry, TagGeometry};
pub use palette::{Palette, TerminalColors};
pub use render::{MermaidDiagram, MermaidTarget, NaturalSize};
