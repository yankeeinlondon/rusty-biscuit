use std::io::{self, Write};

use biscuit_terminal::prelude::Prose;
use tree_hugger::SymbolRecord;

use crate::render::{Renderer, literal};
use crate::file_url;

/// Writes the shared Prose markup for one selected file.
pub(crate) fn write_file(out: &mut dyn Write, renderer: &Renderer, records: &[&SymbolRecord], file: &std::path::Path, plain: bool) -> io::Result<()> {
    if records.is_empty() { return Ok(()); }
    for record in records {
        let start = &record.source.declaration_span.start;
        let loc = format!("{}:{}:{}", renderer.display(&record.source.file_path), start.line, start.column);
        let text = literal(&loc);
        let location = if plain { text } else if let Some(url) = file_url(file) {
            format!("<a href={}>{text}</a>", Prose::quoted_attr(&format!("{url}#L{}", start.line)))
        } else { text };
        let declaration = renderer.declaration(record, true);
        let declaration = if plain { strip_generated_tags(&declaration) } else { declaration };
        let docs = renderer.doc_lines(record.docs.raw_doc.as_deref());
        let docs = if plain { strip_generated_tags(&docs) } else { docs };
        if plain {
            writeln!(out, "- {} {}{}", declaration, location, docs)?;
        } else {
            writeln!(out, "- {} <dim>{}</dim>{}", declaration, location, docs)?;
        }
    }
    Ok(())
}

fn strip_generated_tags(markup: &str) -> String {
    let mut result = String::with_capacity(markup.len());
    let mut inside = false;
    for ch in markup.chars() {
        match ch {
            '<' => inside = true,
            '>' if inside => inside = false,
            _ if !inside => result.push(ch),
            _ => {}
        }
    }
    result
}
