use crate::terminal::Terminal;

/// A struct or enum which implements the **Renderable** trait
/// can be reduced down to a string designed to be displayed
/// in a terminal.
pub trait Renderable {
    /// **Opportunistic Render**
    ///
    /// renders without knowledge of the underlying terminal's
    /// capabilities with an "opportunistic" view that the
    /// terminal supports all capabilities.
    fn render() -> String;

    /// **Fallback Render**
    ///
    /// Renders the component based on the capabilities of the
    /// passed in `Terminal`. Will provide graceful fallbacks
    /// when possible.
    fn fallback_render(term: &Terminal) -> String;
}

/// A **RenderableWrapper** is a utility which operates at at
/// a lower level than a **Renderable** component and takes in
/// string content and outputs that string _wrapped_ in some
/// sort of formatting.
pub trait RenderableWrapper {
    fn render<T: Into<String>>(self, content: T) -> String;

    fn fallback_render<T: Into<String>>(self, content: T, term: &Terminal) -> String;
}
