/// A component capable of rendering itself as Markdown output.
///
/// Markdown is a _superset_ of HTML which means, strictly speaking, it can
/// contain as much inline HTML as you like but there are good reasons to
/// keep this to a minimum:
///
/// 1. Every HTML tag makes the authoring experience less ergonomic
/// 2. While many renders might render a few inline elements effectively you should expect inconsistent
pub trait MarkdownRenderable {
    /// Renders the component as a Markdown string (including Markdown body and optionally
    /// YAML Frontmatter).
    ///
    /// - any valid Markdown can be passed through "as is"
    /// - some renderers might offer (on their input) an option to "clean" the markdown to make it more idiomatic
    /// - some renderers might decide to provide an option (on their input) to
    fn render_markdown(&self) -> String;

    /// Renders the component as a Markdown string (including Markdown body and optionally
    /// YAML Frontmatter).
    ///
    /// - Features often supported by markdown renders like:
    ///     - iframes (darkmatter will convert )
    ///     - disclosures (darkmatter will convert ergonomic markdown directives into HTML variant)
    ///     - nicer table rendering (by converting Markdown tables to HTML tables)
    ///
    /// > **Note:** the features supported should _never_ rely on Javascript!
    fn render_markdown_plus(&self) -> String;
}
