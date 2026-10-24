pub(crate) mod accents;
pub mod color;
pub mod config;
pub(crate) mod contrast;
pub(crate) mod design;
pub(crate) mod embed;
pub mod error;
pub(crate) mod gitgraph;
pub(crate) mod gitgraph_layout;
pub(crate) mod newline;
pub mod palette;
pub(crate) mod postprocess;
pub mod render;
pub mod text;

pub use color::Rgb;
pub use config::{MermaidConfig, MermaidTheme, QuadrantTheme};
pub use embed::{DiagramFont, FontEmbeddingError, PageFont, SubsetError, page_font};
pub use error::MermaidError;
#[doc(hidden)]
pub use gitgraph::{default_gitgraph_commit_step, Bounds, CommitGeometry, GitGraphGeometry, TagGeometry};
pub use palette::{Palette, TerminalColors};
pub use render::{MermaidDiagram, MermaidTarget, NaturalSize};
pub use text::{
    TextCharset, TextCoverage, TextDrawing, TextEligibility, TextLayout, TextOptions, TextProjection, TextRender,
    TextReport, TextSource, text_eligibility,
};
