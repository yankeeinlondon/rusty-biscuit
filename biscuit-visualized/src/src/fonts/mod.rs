//! Fonts for measuring and drawing diagram text: one font-resolution chain
//! ([`FontChain`]) that both the rasterizer's usvg `FontResolver` and the
//! Mermaid text measurer call, so a label is measured in the face it is drawn
//! in.
//!
//! - [`query`]: what text asks for ([`FontQuery`], built from a `usvg::Font`
//!   or from CSS values).
//! - [`chain`]: the four resolution steps, the fallback order, the bundled
//!   faces' registration, and face identities for cache keys.
//! - [`metadata`]: font-table reads and the installed sans-serif rule.
//! - [`matching`]: CSS face matching within one family (fontdb's rule).
//! - [`bundled`]: the font files compiled into the library.
//! - [`shaper`]: rustybuzz shaping that mirrors usvg's text layout.
//! - [`measure`]: Merman's `TextMeasurer` ([`LabelMeasurer`]), built on the
//!   chain and the shaper.
//!
//! User-facing behavior: `biscuit-visualized/docs/merman.md`.

pub(crate) mod bundled;
pub(crate) mod chain;
pub(crate) mod matching;
pub(crate) mod measure;
pub(crate) mod metadata;
pub(crate) mod query;
pub(crate) mod shaper;

pub(crate) use chain::{FontChain, default_chain};
pub(crate) use measure::LabelMeasurer;
pub(crate) use query::FontQuery;
#[cfg(test)]
pub(crate) use query::GenericFamily;
