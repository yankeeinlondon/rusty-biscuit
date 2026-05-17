/// **RenderTarget**
///
/// An enumerated list of targets that a _renderable_ component might target
pub enum RenderTarget {
    /// Markdown is a much more ergonomic way to write content then HTML but it looses a lot of features
    /// when it keeps it's ergonomic shape. Many people don't realize, however, that Markdown is a
    /// **superset** of HTML, not a _subset_.
    ///
    /// That means that while the way Markdown is _typically written_ does produce a a functional subset
    /// it doesn't have to be. However, as soon as you start writing lots of "inline HTML" in your Markdown
    /// you should really question why you're doing this:
    ///
    /// - you have lost the ergnomoics that Markdown authors typically desire
    /// - Markdown readers may support _some_ inline HTML features but they rarely support that well
    ///
    /// However, there **is** a use case where this makes perfect sense:
    ///
    /// - when an author uses a DSL like **Darkmatter** to write both Markdown _plus_ additional features
    ///   that ergonomic Markdown can't support, a library like Darkmatter can inject this inline HTML
    Markdown,
    /// Terminal library's support styling (colors, bold fonts, etc.) through the use of escape codes
    /// and when rendering content to
    Terminal,
    /// The browser is the most capable target to render to and also probably the most popular.
    ///
    /// A component which can render to a browser will need to render in two different ways:
    ///
    /// 1. `render_html()`
    ///     -
    ///
    /// 2. `render_html_components()`
    Browser,
    Ast,
}
