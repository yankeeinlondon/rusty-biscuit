//! Cross-provider compatibility layer: classify canonical sources and link
//! targets, and rewrite frontmatter to satisfy every provider's requirements.
//!
//! The logic is split into three clusters — [`classify`] (source/target
//! classification), [`frontmatter_io`] (YAML parsing and document rewriting),
//! and [`properties`] (property satisfaction, alias duplication, name
//! derivation) — leaving this root as a declaration/re-export surface.

pub mod table;

mod classify;
mod frontmatter_io;
mod properties;

pub use classify::{classify_canonical_candidate, classify_target_reference};
pub(crate) use frontmatter_io::{
    fix_frontmatter_indentation_tabs, frontmatter_has_indentation_tabs, parse_markdown_document,
};
pub(crate) use properties::has_claude_specific_properties;

#[cfg(test)]
mod tests;
