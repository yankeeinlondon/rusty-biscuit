//! Per-bucket schema structs for the `style:` frontmatter.
//!
//! The root [`StyleFrontmatter`] is defined here in later tasks; per-bucket
//! structs live in sibling files.

pub mod common;
pub mod components;
pub mod page;

pub use common::CommonStyle;
pub use components::{BlockQuoteStyle, TableStyle};
pub use page::{CodeStyle, PageStyle};
