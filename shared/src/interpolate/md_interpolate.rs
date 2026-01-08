pub enum MarkdownScope {
    Frontmatter,
    Prose,
    CodeBlock,
    BlockQuote,
    Heading,
    /// allows search and replace on content within the body
    /// of the HTML which has been italicized(`__` or `**`), bold-faced (`*`),
    /// or had strikethrough applied (`~~`).
    Stylized,
    /// allows search and replace on _only_ italicized content
    /// (the content as well as the surrounding tokens used for
    /// italics will be included)
    Italicized,
    NonItalicized,

}
