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

    fn set_char(&self) -> &'static str;
    fn reset_char(&self) -> &'static str;
}

const ESC: &'static str = "\x1b[";
const TERMINAL: &'static str = "m";

const ITALIC: &'static str = "3";
const NOT_ITALIC: &'static str = "23";

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

pub fn curly_underline<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
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

const UNDERLINE: &'static str = "4";
const DOUBLE_UNDERLINE: &'static str = "4:2";
const CURLY_UNDERLINE: &'static str = "4:3";
const DOTTED_UNDERLINE: &'static str = "4:4";
const DASHED_UNDERLINE: &'static str = "4:5";
const NOT_UNDERLINE: &'static str = "24";

const BLINK: &'static str = "5";
const BLINK_FAST: &'static str = "6";
const NOT_BLINK: &'static str = "25";

const STRIKETHROUGH: &'static str = "9";
const NOT_STRIKETHROUGH: &'static str = "29";

const INVERSE: &'static str = "7";
const NOT_INVERSE: &'static str = "27";

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

impl Stylist for Style {
    fn reset_char(&self) -> &'static str {
        match self {
            Style::Underline(_) => NOT_UNDERLINE,
            Style::CurlyUnderline(_) => NOT_UNDERLINE,
            Style::DashedUnderline(_) => NOT_UNDERLINE,
            Style::DottedUnderline(_) => NOT_UNDERLINE,
            Style::DoubleUnderline(_) => NOT_UNDERLINE,
            Style::Italic => NOT_ITALIC,
            Style::Blink => NOT_BLINK,
            Style::BlinkFast => NOT_BLINK,
            Style::Strikethrough => NOT_STRIKETHROUGH,
            Style::Inverse => NOT_INVERSE,

            Style::Bold => BOLD,
            Style::Dim => DIM,
            Style::NormalWeight => NORMAL,
        }
    }

    fn set_char(&self) -> &'static str {
        match self {
            Style::Underline(color) => UNDERLINE,
            Style::CurlyUnderline(color) => CURLY_UNDERLINE,
            Style::DashedUnderline(color) => DASHED_UNDERLINE,
            Style::DottedUnderline(color) => DOUBLE_UNDERLINE,
            Style::DoubleUnderline(color) => DOUBLE_UNDERLINE,
            Style::Italic => ITALIC,
            Style::Blink => BLINK,
            Style::BlinkFast => BLINK_FAST,
            Style::Strikethrough => STRIKETHROUGH,
            Style::Inverse => INVERSE,
            Style::Bold => BOLD,
            Style::Dim => DIM,
            Style::NormalWeight => NORMAL,
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
                match self {
                    &Style::Underline(_) => match term.underline_support.straight {
                        true => self.wrap(content),
                        false => content.into()
                    }
                    &Style::CurlyUnderline(_) => {
                        match term.underline_support.curly {
                            true => self.wrap(content),
                            false => content.into()
                        }
                    },
                    &Style::DoubleUnderline(_) => {
                        match term.underline_support.double {
                            true => self.wrap(content),
                            false => content.into()
                        }
                    },
                    &Style::DashedUnderline(_) => {
                        match term.underline_support.dashed {
                            true => self.wrap(content),
                            false => content.into()
                        }
                    },
                    &Style::DottedUnderline(_) => {
                        match term.underline_support.dotted {
                            true => self.wrap(content),
                            false => content.into()
                        }
                    },

                }
            },
            false => content.into(),
        }
    }
}

const BOLD: &'static str = "\x1b[1m";
const DIM: &'static str = "\x1b[2m";
const NORMAL: &'static str = "\x1b[22m";

/// **FontWeight** struct
///
/// This allows for defining a desired _font weight_ for a renderable component prior
/// to rendering.
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

    fn set_char(&self) -> &'static str {
        match self {
            FontWeight::Bold => "1",
            FontWeight::Dim => "2",
            FontWeight::Normal => "22",
        }
    }

    fn reset_char(&self) -> &'static str {
        "22"
    }
}
