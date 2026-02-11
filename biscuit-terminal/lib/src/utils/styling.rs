use serde::{Deserialize, Serialize};

use crate::{terminal::Terminal, utils::color::Color};

pub trait Stylist {
    /// Wraps the text with escape codes wrapping to provide
    /// the desired style.
    fn wrap<T: Into<String>>(&self, content: T) -> String;

    /// Wraps the text with escape codes wrapping to provide
    /// the desired style. Uses the capabilities of the Terminal
    /// to gracefully fallback from capabilities the terminal
    /// doesn't support.
    fn term_wrap<T: Into<String>>(&self, content: T, term: &Terminal) -> String;

    /// Returns the escape code sequence to enable this style.
    fn set_char(&self) -> String;

    /// Returns the escape code sequence to disable this style.
    fn reset_char(&self) -> String;
}

const ESC: &str = "\x1b[";
const TERMINAL: &str = "m";

const ITALIC: &str = "3";
const NOT_ITALIC: &str = "23";

/// makes the text passed in italic
pub fn italic<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty && term.supports_italic {
        format!(
            "{}{}{}{}{}{}",
            ESC, ITALIC, content, ESC, NOT_ITALIC, TERMINAL
        )
    } else {
        content
    }
}

pub fn underline<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty && term.underline_support.straight {
        format!("\x1b[4m{}\x1b[24m", content)
    } else {
        content
    }
}

pub fn double_underline<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if !term.is_tty {
        return content;
    }

    if term.underline_support.double {
        format!("\x1b[4:2m{}\x1b[24m", content)
    } else if term.underline_support.straight {
        format!("\x1b[4m{}\x1b[24m", content)
    } else {
        content
    }
}

pub fn dotted_underline<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if !term.is_tty {
        return content;
    }

    if term.underline_support.dotted {
        format!("\x1b[4:4m{}\x1b[24m", content)
    } else if term.underline_support.straight {
        format!("\x1b[4m{}\x1b[24m", content)
    } else {
        content
    }
}

pub fn curly_underline<T: Into<String>>(content: T, terminal: Option<&Terminal>) -> String {
    let content = content.into();
    let term: Terminal = match terminal {
        Some(terminal) => Terminal::from(terminal),
        _ => Terminal::new_tty(),
    };
    if !term.is_tty {
        return content;
    }

    if term.underline_support.curly {
        format!("\x1b[4:3m{}\x1b[24m", content)
    } else if term.underline_support.straight {
        format!("\x1b[4m{}\x1b[24m", content)
    } else {
        content
    }
}

pub fn dashed_underline<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if !term.is_tty {
        return content;
    }

    if term.underline_support.dashed {
        format!("\x1b[4:5m{}\x1b[24m", content)
    } else if term.underline_support.straight {
        format!("\x1b[4m{}\x1b[24m", content)
    } else {
        content
    }
}

#[derive(Debug)]
pub enum UnderliningRequest {
    None,
    Straight(Option<Color>),
    Double(Option<Color>),
    Dotted(Option<Color>),
    Curly(Option<Color>),
    Dashed(Option<Color>),
}

const UNDERLINE: &str = "4";
const DOUBLE_UNDERLINE: &str = "4:2";
const CURLY_UNDERLINE: &str = "4:3";
const DOTTED_UNDERLINE: &str = "4:4";
const DASHED_UNDERLINE: &str = "4:5";
const NOT_UNDERLINE: &str = "24";

const BLINK: &str = "5";
const BLINK_FAST: &str = "6";
const NOT_BLINK: &str = "25";

const STRIKETHROUGH: &str = "9";
const NOT_STRIKETHROUGH: &str = "29";

const INVERSE: &str = "7";
const NOT_INVERSE: &str = "27";

/// **Style** struct
///
/// Allows for the description of common Terminal styling
/// options abstracted from a Terminal's underlying capabilities.
///
/// This enum is a `Renderable` component so can later be used
/// to render a block of text with the given style (both with
/// or without the influence of the terminal's capabilities)
///
/// **Note:** this _does not_ include boldfacing or dimming text
/// as this is covered with the `FontWeight` struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Style {
    /// italicize text
    Italic,
    /// underline with the normal straight, single underline element
    Underline(Option<Color>),
    /// add a **double** underline to the wrapped text
    DoubleUnderline(Option<Color>),
    /// add a _dotted_ underline to the wrapped text
    DottedUnderline(Option<Color>),
    /// add "curly" underline to the wrapped text (often used for diagnostic reporting)
    CurlyUnderline(Option<Color>),
    /// add a _dashed_ underline to the wrapped text
    DashedUnderline(Option<Color>),
    Strikethrough,
    /// make text blink (your mileage may vary)
    Blink,
    BlinkFast,
    Inverse,

    Bold,
    Dim,
    NormalWeight,
}

/// Helper to generate underline escape code with optional color.
///
/// Combines the underline style code with SGR 58 (underline color) if a color is provided.
fn underline_with_color(underline_code: &str, color: &Option<Color>) -> String {
    match color {
        Some(c) => {
            if let Some((r, g, b)) = c.to_rgb() {
                format!("{};58;2;{};{};{}", underline_code, r, g, b)
            } else {
                underline_code.to_string()
            }
        }
        None => underline_code.to_string(),
    }
}

impl Stylist for Style {
    fn reset_char(&self) -> String {
        match self {
            // Underlines with color need to reset both underline (24) and underline color (59)
            Style::Underline(Some(_))
            | Style::CurlyUnderline(Some(_))
            | Style::DashedUnderline(Some(_))
            | Style::DottedUnderline(Some(_))
            | Style::DoubleUnderline(Some(_)) => format!("{};59", NOT_UNDERLINE),

            // Underlines without color just reset underline
            Style::Underline(None)
            | Style::CurlyUnderline(None)
            | Style::DashedUnderline(None)
            | Style::DottedUnderline(None)
            | Style::DoubleUnderline(None) => NOT_UNDERLINE.to_string(),

            Style::Italic => NOT_ITALIC.to_string(),
            Style::Blink | Style::BlinkFast => NOT_BLINK.to_string(),
            Style::Strikethrough => NOT_STRIKETHROUGH.to_string(),
            Style::Inverse => NOT_INVERSE.to_string(),

            Style::Bold | Style::Dim | Style::NormalWeight => NORMAL.to_string(),
        }
    }

    fn set_char(&self) -> String {
        match self {
            Style::Underline(color) => underline_with_color(UNDERLINE, color),
            Style::CurlyUnderline(color) => underline_with_color(CURLY_UNDERLINE, color),
            Style::DashedUnderline(color) => underline_with_color(DASHED_UNDERLINE, color),
            Style::DottedUnderline(color) => underline_with_color(DOTTED_UNDERLINE, color),
            Style::DoubleUnderline(color) => underline_with_color(DOUBLE_UNDERLINE, color),
            Style::Italic => ITALIC.to_string(),
            Style::Blink => BLINK.to_string(),
            Style::BlinkFast => BLINK_FAST.to_string(),
            Style::Strikethrough => STRIKETHROUGH.to_string(),
            Style::Inverse => INVERSE.to_string(),
            Style::Bold => BOLD.to_string(),
            Style::Dim => DIM.to_string(),
            Style::NormalWeight => NORMAL.to_string(),
        }
    }

    fn wrap<T: Into<String>>(&self, content: T) -> String {
        format!(
            "{}{}{}{}{}{}",
            ESC,
            self.set_char(),
            content.into(),
            ESC,
            self.reset_char(),
            TERMINAL
        )
    }

    fn term_wrap<T: Into<String>>(&self, content: T, term: &Terminal) -> String {
        match term.is_tty {
            true => {
                match *self {
                    Style::Underline(_) => match term.underline_support.straight {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    Style::CurlyUnderline(_) => match term.underline_support.curly {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    Style::DoubleUnderline(_) => match term.underline_support.double {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    Style::DashedUnderline(_) => match term.underline_support.dashed {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    Style::DottedUnderline(_) => match term.underline_support.dotted {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    Style::Italic => match term.supports_italic {
                        true => self.wrap(content),
                        false => content.into(),
                    },
                    // These styles are widely supported - apply when is_tty
                    Style::Blink => self.wrap(content),
                    Style::BlinkFast => self.wrap(content),
                    Style::Strikethrough => self.wrap(content),
                    Style::Inverse => self.wrap(content),
                    Style::Bold => self.wrap(content),
                    Style::Dim => self.wrap(content),
                    Style::NormalWeight => self.wrap(content),
                }
            }
            false => content.into(),
        }
    }
}

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const NORMAL: &str = "\x1b[22m";

/// **FontWeight** struct
///
/// This allows for defining a desired _font weight_ for a renderable component prior
/// to rendering.
#[derive(Debug, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Dim,
}

impl Stylist for FontWeight {
    fn wrap<T: Into<String>>(&self, content: T) -> String {
        match self {
            FontWeight::Normal => {
                format!("{}{}", NORMAL, content.into())
            }
            FontWeight::Bold => {
                format!("{}{}{}", BOLD, content.into(), NORMAL)
            }
            FontWeight::Dim => {
                format!("{}{}{}", DIM, content.into(), NORMAL)
            }
        }
    }
    fn term_wrap<T: Into<String>>(&self, content: T, term: &Terminal) -> String {
        match term.is_tty {
            true => self.wrap(content),
            false => content.into(),
        }
    }

    fn set_char(&self) -> String {
        match self {
            FontWeight::Bold => "1".to_string(),
            FontWeight::Dim => "2".to_string(),
            FontWeight::Normal => "22".to_string(),
        }
    }

    fn reset_char(&self) -> String {
        "22".to_string()
    }
}
