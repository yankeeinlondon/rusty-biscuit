use std::borrow::Cow;

pub mod basic;
pub mod color_enum;
pub mod hdr;
pub mod octet;
pub mod rgb;
pub mod tailwind;
pub mod web;
pub mod wrappers;

pub use basic::{basic_color_to_rgb, BasicColor, FgBg};
pub use color_enum::Color;
pub use hdr::HdrColor;
pub use octet::{Octet, OctetError};
pub use rgb::RgbColor;
pub use tailwind::Tailwind;
pub use web::{WebColor, WEB_COLOR_LOOKUP};
pub use wrappers::{BasicColorWrapper, RgbColorWrapper, TailwindColorWrapper, WebColorWrapper};

pub trait TermColor<'a> {
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String;
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String;
}

#[cfg(test)]
mod tests;
