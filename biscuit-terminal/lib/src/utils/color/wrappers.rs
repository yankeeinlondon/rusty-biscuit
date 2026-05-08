use crate::{terminal::Terminal, utils::layout::RenderableWrapper};

use super::{basic::color_code, BasicColor, RgbColor, Tailwind, WebColor, WEB_COLOR_LOOKUP};

/// Wrapper for basic colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct BasicColorWrapper(pub BasicColor);

impl RenderableWrapper for BasicColorWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String {
        let content = content.into();
        format!("\x1b[{}m{}\x1b[0m", color_code(self.0), content)
    }

    fn fallback_render<T: Into<String>>(&self, content: T, _term: &Terminal) -> String {
        let content = content.into();
        format!("\x1b[{}m{}\x1b[0m", color_code(self.0), content)
    }
}

/// Wrapper for RGB colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct RgbColorWrapper(pub RgbColor);

impl RenderableWrapper for RgbColorWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String {
        let content = content.into();
        let rgb = self.0;
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            rgb.red(),
            rgb.green(),
            rgb.blue(),
            content
        )
    }

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String {
        let content = content.into();
        let rgb = self.0;
        // Check terminal color depth and use appropriate encoding
        match term.color_depth {
            crate::discovery::detection::ColorDepth::TrueColor => {
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    rgb.red(),
                    rgb.green(),
                    rgb.blue(),
                    content
                )
            }
            crate::discovery::detection::ColorDepth::Enhanced => {
                // 256-color palette: ESC[38;5;<n>m
                // Convert RGB to nearest 256-color index (simplified approach)
                let r = rgb.red() as f32;
                let g = rgb.green() as f32;
                let b = rgb.blue() as f32;
                // Simple 6x6x6 color cube approximation
                let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                    + ((g / 256.0 * 6.0).floor() as u8)
                    + ((b / 256.0 * 1.0).floor() as u8)
                    + 16;
                format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
            }
            _ => {
                // Fallback to basic color
                format!("\x1b[{}m{}\x1b[0m", color_code(rgb.fallback()), content)
            }
        }
    }
}

/// Wrapper for web colors that implements `RenderableWrapper`.
#[derive(Debug, Clone, Copy)]
pub struct WebColorWrapper(pub WebColor);

impl RenderableWrapper for WebColorWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String {
        let content = content.into();
        let rgb = WEB_COLOR_LOOKUP
            .get(&self.0)
            .copied()
            .unwrap_or(RgbColor::new(128, 128, 128, BasicColor::White));
        format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            rgb.red(),
            rgb.green(),
            rgb.blue(),
            content
        )
    }

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String {
        let content = content.into();
        let rgb = WEB_COLOR_LOOKUP
            .get(&self.0)
            .copied()
            .unwrap_or(RgbColor::new(128, 128, 128, BasicColor::White));
        match term.color_depth {
            crate::discovery::detection::ColorDepth::TrueColor => {
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    rgb.red(),
                    rgb.green(),
                    rgb.blue(),
                    content
                )
            }
            crate::discovery::detection::ColorDepth::Enhanced => {
                let r = rgb.red() as f32;
                let g = rgb.green() as f32;
                let b = rgb.blue() as f32;
                // Simple 6x6x6 color cube approximation
                let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                    + ((g / 256.0 * 6.0).floor() as u8)
                    + ((b / 256.0 * 1.0).floor() as u8)
                    + 16;
                format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
            }
            _ => {
                format!("\x1b[{}m{}\x1b[0m", color_code(rgb.fallback()), content)
            }
        }
    }
}

/// Wrapper for Tailwind colors that implements `RenderableWrapper`.
///
/// Uses `Tailwind::to_hdr_color()` to get the underlying HDR color values
/// for rendering. Special values (Inherit, Current, Transparent) return
/// the content unchanged since they have no meaningful terminal representation.
#[derive(Debug, Clone, Copy)]
pub struct TailwindColorWrapper(pub Tailwind);

impl RenderableWrapper for TailwindColorWrapper {
    fn render<T: Into<String>>(&self, content: T) -> String {
        let content = content.into();

        // Get the HDR color; special values (Inherit/Current/Transparent) return None
        match self.0.to_hdr_color() {
            Some(hdr) => {
                // Render as 24-bit truecolor
                format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    hdr.red(),
                    hdr.green(),
                    hdr.blue(),
                    content
                )
            }
            None => {
                // Special values: return content unchanged
                content
            }
        }
    }

    fn fallback_render<T: Into<String>>(&self, content: T, term: &Terminal) -> String {
        let content = content.into();

        // Get the HDR color; special values (Inherit/Current/Transparent) return None
        match self.0.to_hdr_color() {
            Some(hdr) => {
                match term.color_depth {
                    crate::discovery::detection::ColorDepth::TrueColor => {
                        format!(
                            "\x1b[38;2;{};{};{}m{}\x1b[0m",
                            hdr.red(),
                            hdr.green(),
                            hdr.blue(),
                            content
                        )
                    }
                    crate::discovery::detection::ColorDepth::Enhanced => {
                        // 256-color palette: ESC[38;5;<n>m
                        // Convert RGB to nearest 256-color index using 6x6x6 color cube
                        let r = hdr.red() as f32;
                        let g = hdr.green() as f32;
                        let b = hdr.blue() as f32;
                        let color_idx = ((r / 256.0 * 36.0).floor() as u8)
                            + ((g / 256.0 * 6.0).floor() as u8)
                            + ((b / 256.0 * 1.0).floor() as u8)
                            + 16;
                        format!("\x1b[38;5;{}m{}\x1b[0m", color_idx, content)
                    }
                    _ => {
                        // Fallback to basic ANSI color
                        format!("\x1b[{}m{}\x1b[0m", color_code(hdr.fallback()), content)
                    }
                }
            }
            None => {
                // Special values: return content unchanged
                content
            }
        }
    }
}
