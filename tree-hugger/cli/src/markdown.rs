//! `--format markdown`: the declaration listing as CommonMark. Package,
//! module and file groups become headings; listing bullets, outline rows and
//! class sections become nested list items; doc lines continue their item
//! after a hard line break. The listing's Prose markup is parsed into inline
//! nodes: bold and italic become `**`/`*`, presentation-only colors, dimming
//! and underlining are dropped, and links keep only their label. Every text
//! node is CommonMark-escaped here, at the output boundary, so source text
//! cannot open a link, HTML, an entity, emphasis or a block. Control
//! characters were already made visible by `render::literal`, and a text
//! node is shown with the same escapes if one still holds a control, so no
//! source newline or terminal control reaches the document.

use std::io::{self, Write};

use biscuit_terminal::prelude::InlineProse;
use renderable::tree::{NodeKind, RenderNode};

use crate::render::{Layout, Row, neutralized};

/// Spaces per nesting level: the content column of a `- ` bullet.
const NEST: usize = 2;

/// [`Layout`] writing CommonMark.
pub(crate) struct MarkdownLayout<'w> {
    out: &'w mut dyn Write,
    /// Nothing written yet: a document never starts with a blank line.
    at_start: bool,
    /// A blank line is due before the next block.
    blank_due: bool,
}

impl<'w> MarkdownLayout<'w> {
    pub(crate) fn new(out: &'w mut dyn Write) -> Self {
        Self { out, at_start: true, blank_due: false }
    }

    fn begin_block(&mut self) -> io::Result<()> {
        if self.blank_due && !self.at_start {
            writeln!(self.out)?;
        }
        self.blank_due = false;
        self.at_start = false;
        Ok(())
    }

    /// One list item at `depth`, its doc lines continuing the item.
    fn item(&mut self, depth: usize, markup: &str) -> io::Result<()> {
        self.begin_block()?;
        let mut lines = markup.split('\n');
        let mut text = inline(lines.next().unwrap_or_default());
        write!(self.out, "{:width$}- ", "", width = depth * NEST)?;
        for doc in lines {
            // A trailing backslash is a hard line break; the continuation
            // line is indented to the item's content column.
            writeln!(self.out, "{text}\\")?;
            text = format!("{:width$}{}", "", inline(doc), width = (depth + 1) * NEST);
        }
        writeln!(self.out, "{text}")
    }
}

impl Layout for MarkdownLayout<'_> {
    fn heading(&mut self, level: u8, _indent: usize, markup: &str) -> io::Result<()> {
        self.blank_due = true;
        self.begin_block()?;
        let marks = "#".repeat(usize::from(level) + 1);
        // The scan root's own package section has an empty heading, which
        // the terminal shows as a blank line; a Markdown heading names it.
        let text = inline(markup);
        let text = if text.is_empty() { "." } else { text.as_str() };
        writeln!(self.out, "{marks} {text}")?;
        self.blank_due = true;
        Ok(())
    }

    fn row(&mut self, row: Row<'_>) -> io::Result<()> {
        self.item(row.depth, row.markup)
    }

    fn list(&mut self, _indent: usize, depth: usize, items: &[String]) -> io::Result<()> {
        for item in items {
            self.item(depth, item)?;
        }
        Ok(())
    }

    fn note(&mut self, _indent: usize, markup: &str) -> io::Result<()> {
        self.blank_due = true;
        self.begin_block()?;
        writeln!(self.out, "{}", inline(markup))?;
        self.blank_due = true;
        Ok(())
    }

    fn blank(&mut self) -> io::Result<()> {
        self.blank_due = true;
        Ok(())
    }

    fn writer(&mut self) -> &mut dyn Write {
        self.out
    }
}

/// One line of generated Prose markup as CommonMark inline content.
fn inline(markup: &str) -> String {
    let mut line = Escaper::default();
    // Inline parsing normalizes carriage returns; expose controls before parsing.
    line.nodes(&InlineProse::new(neutralized(markup)).to_render_nodes());
    let mut text = line.text;
    // Trailing spaces would form a hard break or be trimmed from a heading.
    let kept = text.trim_end_matches(' ').len();
    let trailing = text.len() - kept;
    text.truncate(kept);
    text.push_str(&"&#32;".repeat(trailing));
    text
}

/// Builds one line, tracking what the line start makes special.
#[derive(Default)]
struct Escaper {
    text: String,
    /// Something was written, so the next character is not at the line start.
    started: bool,
    /// Everything written so far is ASCII digits (an ordered-list marker
    /// would follow).
    only_digits: bool,
}

impl Escaper {
    fn nodes(&mut self, nodes: &[RenderNode]) {
        for node in nodes {
            match &node.kind {
                NodeKind::Text { value } | NodeKind::InlineCode { value } | NodeKind::Code { value, .. } => {
                    self.source_text(value);
                }
                NodeKind::Strong { children } => self.emphasis("**", children),
                NodeKind::Emphasis { children } => self.emphasis("*", children),
                // Spans (color, dim, underline), deletions and links keep
                // only their content: Markdown carries no presentation and
                // generates no links.
                _ => self.nodes(node.children()),
            }
        }
    }

    /// `children` between `delimiter`s; an empty run writes nothing, and a
    /// boundary space becomes `&#32;` so the delimiters stay flanking.
    fn emphasis(&mut self, delimiter: &str, children: &[RenderNode]) {
        let mut inner = Escaper { started: true, ..Escaper::default() };
        inner.nodes(children);
        let inner = inner.text;
        if inner.is_empty() {
            return;
        }
        let trimmed = inner.trim_matches(' ');
        let leading = inner.len() - inner.trim_start_matches(' ').len();
        let trailing = inner.len() - inner.trim_end_matches(' ').len();
        let (leading, trailing) = if trimmed.is_empty() { (inner.len(), 0) } else { (leading, trailing) };
        self.text.push_str(delimiter);
        self.text.push_str(&"&#32;".repeat(leading));
        self.text.push_str(trimmed);
        self.text.push_str(&"&#32;".repeat(trailing));
        self.text.push_str(delimiter);
        self.started = true;
        self.only_digits = false;
    }

    /// CommonMark-escapes one text node: inline syntax everywhere, and
    /// block syntax where it would start the line.
    fn source_text(&mut self, value: &str) {
        for character in value.chars() {
            let at_start = !self.started;
            match character {
                ' ' if at_start => {
                    self.text.push_str("&#32;");
                    self.started = true;
                    self.only_digits = false;
                    continue;
                }
                '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '!' | '#' | '~' | '|' => {
                    self.text.push('\\');
                }
                '-' | '+' | '=' if at_start => self.text.push('\\'),
                '.' | ')' if self.only_digits => self.text.push('\\'),
                // `inline` gets one line, so a control here is source text
                // that skipped `render::literal`; it must still not become a
                // line break or a terminal control.
                control if control.is_control() => {
                    self.source_text(&neutralized(&control.to_string()));
                    continue;
                }
                _ => {}
            }
            self.text.push(character);
            self.only_digits = character.is_ascii_digit() && (at_start || self.only_digits);
            self.started = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::literal;

    #[test]
    fn source_syntax_stays_literal_and_generated_emphasis_survives() {
        let markup = format!(
            "<i>public</i> <green>fn</green> <b>{}</b> <dim><a href=\"file:///a\">{}</a></dim>",
            literal("a [x](u) **b** `c` <img> &copy; | ~d~ \\"),
            literal("a.rs:1:2"),
        );
        assert_eq!(
            inline(&markup),
            "*public* fn **a \\[x\\](u) \\*\\*b\\*\\* \\`c\\` \\<img\\> \\&copy; \\| \\~d\\~ \\\\** a.rs:1:2"
        );
    }

    #[test]
    fn line_start_block_syntax_is_escaped() {
        for (source, expected) in [
            ("# h", "\\# h"),
            ("- x", "\\- x"),
            ("+ x", "\\+ x"),
            ("===", "\\==="),
            ("12. x", "12\\. x"),
            ("3) x", "3\\) x"),
            ("> q", "\\> q"),
            ("    code", "&#32;   code"),
            ("x - 1. y", "x - 1. y"),
            ("end ", "end&#32;"),
        ] {
            assert_eq!(inline(&literal(source)), expected, "{source:?}");
        }
    }

    #[test]
    fn controls_in_a_text_node_are_shown_as_their_literal_escapes() {
        let source = "a\u{1b}[31m\u{9b}31m\u{85}\u{7}\tb\r";
        assert_eq!(inline(&format!("<b>{source}</b>")), inline(&format!("<b>{}</b>", literal(source))));
        assert_eq!(inline(source), r"a\\u{1b}\[31m\\u{9b}31m\\u{85}\\u{7}\\u{9}b\\u{d}");
    }

    #[test]
    fn empty_and_space_bounded_emphasis_stays_balanced() {
        assert_eq!(inline("<b></b>"), "");
        assert_eq!(inline(&format!("<b>{}</b>", literal(" x "))), "**&#32;x&#32;**");
    }
}
