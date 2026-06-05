//! Convenience re-exports of the most commonly used `renderable` types.

pub use crate::browser::BrowserRenderable;
pub use crate::color::{BasicColor, Color, HdrColor, RgbColor, Tailwind, WebColor};
pub use crate::layout::{Alignment, Edges, Layout, WordWrap};
pub use crate::markdown::MarkdownRenderable;
pub use crate::style::{
    Background, Border, BorderLineStyle, BorderSides, BorderWeight, EmphasisLayer, PerMode, Style,
    TextEmphasis, UnderlineStyle,
};
pub use crate::stylesheet::{CssTypedProperty, IntoCssValue};
pub use crate::tree::TreeRenderable;
