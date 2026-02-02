use crate::terminal::Terminal;

/// make the text passed in BOLD
pub fn bold<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty {
        format!("\x1b[1m{}\x1b[22m", content)
    } else {
        content
    }
}

/// makes the text passed in DIM
pub fn dim<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty {
        format!("\x1b[2m{}\x1b[22m", content)
    } else {
        content
    }
}

pub fn normal<T: Into<String>>(content: T, terminal: Option<Terminal>) -> String {
    let content = content.into();
    let term = terminal.unwrap_or_default();
    if term.is_tty {
        format!("\x1b[22m{}\x1b[22m", content) // Resets both bold and dim
    } else {
        content
    }
}

/// **FontWeight** struct
///
/// This allows for defining a desired _font weight_ for a renderable component prior
/// to rendering.
pub enum FontWeight {
    Normal,
    Bold,
    Dim,
}

impl FontWeight {
    /// Wraps the provided content with the appropriate ANSI escape codes for this
    /// font weight, based on the terminal capabilities.
    ///
    /// ## Arguments
    ///
    /// * `content` - The text to wrap with font weight styling
    /// * `terminal` - Optional terminal to check TTY capability
    ///
    /// ## Returns
    ///
    /// The styled string with ANSI codes if terminal is TTY, otherwise plain text
    pub fn wrap<T: Into<String>>(&self, content: T, terminal: Option<Terminal>) -> String {
        match self {
            FontWeight::Normal => normal(content, terminal),
            FontWeight::Bold => bold(content, terminal),
            FontWeight::Dim => dim(content, terminal),
        }
    }
}
