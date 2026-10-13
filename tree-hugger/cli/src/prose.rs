//! `--format prose`: the declaration listing as unrendered biscuit-terminal
//! Prose markup, laid out exactly as the plain listing (indentation, bullets,
//! doc continuation lines), so rendering it without styles or wrapping gives
//! the `--format plain` text.

use std::io::{self, Write};

use biscuit_terminal::prelude::InlineProse;
use renderable::tree::{NodeKind, RenderNode};

use crate::render::{Layout, Row, literal, neutralized};

/// The bullet a biscuit-terminal `UnorderedList` puts before each item; its
/// continuation lines are indented by the bullet's width.
const BULLET: &str = "- ";

/// [`Layout`] writing Prose markup. With `plain` (global `--plain`), the
/// generated style and link nodes are dropped while the text keeps its
/// literal escaping, so the output stays valid Prose.
pub(crate) struct ProseLayout<'w> {
    out: &'w mut dyn Write,
    plain: bool,
}

impl<'w> ProseLayout<'w> {
    pub(crate) fn new(out: &'w mut dyn Write, plain: bool) -> Self {
        Self { out, plain }
    }

    fn line(&mut self, indent: usize, markup: &str) -> io::Result<()> {
        let markup = if self.plain { unstyled(markup) } else { markup.to_string() };
        writeln!(self.out, "{:indent$}{markup}", "")
    }
}

impl Layout for ProseLayout<'_> {
    fn heading(&mut self, _level: u8, indent: usize, markup: &str) -> io::Result<()> {
        self.line(indent, markup)
    }

    fn row(&mut self, row: Row<'_>) -> io::Result<()> {
        let mut lines = row.markup.split('\n');
        self.line(row.indent, lines.next().unwrap_or_default())?;
        for doc in lines {
            self.line(row.doc_indent.unwrap_or(row.indent), doc)?;
        }
        Ok(())
    }

    fn list(&mut self, indent: usize, _depth: usize, items: &[String]) -> io::Result<()> {
        for item in items {
            let mut lines = item.split('\n');
            let first = lines.next().unwrap_or_default();
            let first = if self.plain { unstyled(first) } else { first.to_string() };
            writeln!(self.out, "{:indent$}{BULLET}{first}", "")?;
            for doc in lines {
                self.line(indent + BULLET.len(), doc)?;
            }
        }
        Ok(())
    }

    fn note(&mut self, indent: usize, markup: &str) -> io::Result<()> {
        self.line(indent, markup)
    }

    fn blank(&mut self) -> io::Result<()> {
        writeln!(self.out)
    }

    fn writer(&mut self) -> &mut dyn Write {
        self.out
    }
}

/// One line of generated markup without its style and link tags: the parsed
/// text, escaped again as literal Prose. The text is never scanned for tags,
/// so escaped source text such as `Vec\<i32\>` survives.
fn unstyled(markup: &str) -> String {
    fn text(nodes: &[RenderNode], into: &mut String) {
        for node in nodes {
            match &node.kind {
                NodeKind::Text { value } | NodeKind::InlineCode { value } | NodeKind::Code { value, .. } => {
                    into.push_str(value);
                }
                _ => text(node.children(), into),
            }
        }
    }
    let mut collected = String::new();
    // Inline parsing normalizes carriage returns; expose controls before parsing.
    text(&InlineProse::new(neutralized(markup)).to_render_nodes(), &mut collected);
    literal(&collected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unstyled_markup_keeps_escaped_source_text() {
        let markup = format!(
            "<i>public</i> <green>function</green> <b>{}</b> <dim><a href=\"file:///a.rs#L1\">a.rs:1:1</a></dim>",
            literal("sample(x: Vec<i32>) <img> **b** `c` [x](u) \\n")
        );
        assert_eq!(
            unstyled(&markup),
            literal("public function sample(x: Vec<i32>) <img> **b** `c` [x](u) \\n a.rs:1:1")
        );
    }
}
