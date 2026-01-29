use std::{fmt::Alignment, path::{Path, absolute}};

use crate::{components::renderable::Renderable};

pub enum ImageWidth {
    /// fill available space (using margins as offsets)
    Fill,
    /// use a percentage of the available space where
    /// available space is the number of characters - (margin_left + margin_right)
    Percent(f32),
    /// a fixed width based on character width
    Characters(u32)
}

pub struct TerminalImage {
    /// fully qualified filename
    pub filename: String,
    /// relative file path from repo root (if in a repo) or CWD if not in a repo
    pub relative: String,

    /// alternative text for terminals which do not support
    pub alt_text: Option<String>,

    /// when a caller specifies a filename and size, they use the `{filename}|{size}` convention
    /// and the `width_raw` captures the `|` onward.
    ///
    /// - `filename.jpg|25` - will capture a string of `|25` and be translated into
    ///    a fixed width of 25 characters.
    /// - whitespace doesn't matter so `filename.jpg|25` and `filename.jpg | 25` are captured
    ///   literally in both cases for `width_raw` but translated exactly the same for the
    ///   `width` enumeration.
    pub width_raw: Option<String>,

    /// Image widths default to 50% of the available width (in characters) but
    /// if you add a `|` character and then a desired width then this will be
    /// used instead
    pub width: ImageWidth,

    pub alignment: Alignment,

    pub margin_left: u32,
    pub margin_right: u32,
}

impl Default for TerminalImage {
    /// a convenience function but not an image filepath
    fn default() -> Self {
        Self {
            filename: "".to_string(),
            relative: ".".to_string(),
            alt_text: None,

            width_raw: None,
            width: ImageWidth::Percent(0.5),
            margin_left: 0,
            margin_right: 0
        }
    }
}


impl Renderable for TerminalImage {
    fn fallback_render(_term: &crate::terminal::Terminal) -> String {
        todo!()
    }

    fn render() -> String {
        todo!()
    }
}

impl TerminalImage {
    pub fn new(filepath: Path) -> Self {
        Self {
            filename: absolute(filepath).to_string(),
            relative: relative(path).to_string(),
            ..default()
        }
    }
}
