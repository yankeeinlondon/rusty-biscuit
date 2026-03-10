/// A small rich-text AST node for provider-agnostic rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RichNode {
    Text(String),
    Bold(Vec<RichNode>),
    Italic(Vec<RichNode>),
    Strikethrough(Vec<RichNode>),
    Code(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Link {
        url: String,
        children: Vec<RichNode>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<RichNode>>,
    },
    Paragraph(Vec<RichNode>),
    Heading {
        level: u8,
        children: Vec<RichNode>,
    },
    SoftBreak,
    HardBreak,
}
