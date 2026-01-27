use crate::{terminal::Terminal, utils::color::Color};

/// makes the text passed in italic
pub fn italic<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty && term.supports_italic {
        format!("\x1b[3m{}\x1b[23m", content)
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
    /// make text flash (your mileage may vary)
    Flash,
}
