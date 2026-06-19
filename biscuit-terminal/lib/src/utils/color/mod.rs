//! Terminal color rendering.
//!
//! The color *data* types live in [`renderable::color`] and are re-exported
//! here for backwards compatibility. Terminal-specific ANSI emission (the
//! [`TermColor`] trait, its impls, and the `RenderableWrapper` color wrappers)
//! lives in this crate.

pub mod color_terminal;
pub mod wrappers;

pub use renderable::color::{
    BasicColor, Color, FgBg, HdrColor, Octet, OctetError, RgbColor, Tailwind, WEB_COLOR_LOOKUP,
    WebColor, basic_color_to_rgb,
};

pub use crate::utils::term_color::TermColor;
pub use wrappers::{BasicColorWrapper, RgbColorWrapper, TailwindColorWrapper, WebColorWrapper};

#[cfg(test)]
mod tests;
