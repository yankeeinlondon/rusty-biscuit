//! Frontmatter `style:` parser for darkmatter documents.
//!
//! See `renderable/features/_unscheduled/style-property/spec.md` for the
//! design. Sub-spec #1: schema + parser only; no rendering changes.

pub mod alignment;
pub mod color;
pub mod error;
pub mod length;
pub mod schema;
pub mod warning;

pub use color::StyleColor;
pub use error::StyleParseError;
pub use warning::{StyleSpan, StyleWarning, StyleWarningKind};

#[cfg(test)]
mod tests {
    /// Smoke test: the module is reachable and compiles.
    #[test]
    fn module_compiles() {
        // Intentionally empty. Existence is the assertion.
    }
}
