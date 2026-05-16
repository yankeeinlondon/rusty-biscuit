use std::borrow::Cow;

/// Terminal-emission trait for color types.
///
/// Implementors wrap string content with the ANSI escape codes required to
/// render that content in a foreground or background color.
pub trait TermColor<'a> {
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the foreground color rendering.
    fn fg(self, content: impl Into<Cow<'a, str>>) -> String;
    /// wraps the content passed in with the escape-codes required
    /// to start and stop the background color rendering.
    fn bg(self, content: impl Into<Cow<'a, str>>) -> String;
}
