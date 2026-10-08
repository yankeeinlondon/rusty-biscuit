use std::io::{self, Write};

use tree_hugger::SymbolRecord;

use crate::output::SignatureMode;
use crate::render::{Renderer, signature_cell};

/// CommonMark-escapes source text at the point it enters a Markdown text node.
pub(crate) fn escape_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            escaped.push('\n');
            continue;
        }
        if ch.is_control() {
            escaped.push(' ');
            continue;
        }
        if "\\`*_{}[]()#+-.!>|&<>".contains(ch) || (ch == ' ' && index == 0) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

/// Writes selected records as a Markdown list without creating links from paths.
pub(crate) fn write_file(
    out: &mut dyn Write,
    renderer: &Renderer,
    records: &[&SymbolRecord],
    signatures: SignatureMode,
    comments: bool,
) -> io::Result<()> {
    for record in records {
        let start = &record.source.declaration_span.start;
        let path = renderer.display(&record.source.file_path);
        let declaration = format!("{} {}", record.identity.name, signature_cell(record, signatures));
        writeln!(out, "- {} — {}:{}", escape_text(&declaration), escape_text(&path), start.line)?;
        if comments && let Some(doc) = record.docs.raw_doc.as_deref() {
            for line in doc.trim().lines().map(str::trim).filter(|line| !line.is_empty()) {
                writeln!(out, "  {}", escape_text(line))?;
            }
        }
    }
    Ok(())
}
