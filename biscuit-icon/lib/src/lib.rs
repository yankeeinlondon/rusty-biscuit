//! Curated, offline-embedded domain icons plus on-demand Iconify lookup.
//!
//! Domain icons are accessed enum-first (e.g. [`domain::Os::Finder`]) with a
//! fallible string convenience layer ([`Icon::os`]). Any of the 200,000+
//! Iconify icons can be fetched at runtime via [`Icon::iconify`] and cached to
//! a local SQLite database.

pub mod body;
pub mod cache;
pub mod catalog;
pub mod domain;
pub mod error;
pub mod glyph;
pub mod icon;
pub mod iconify;
pub mod render;
pub mod style;

pub use body::IconBody;
pub use cache::SetInfo;
pub use error::IconError;
pub use glyph::Glyph;
pub use icon::{Icon, Source};
pub use style::{Flip, Rotate, Style};
