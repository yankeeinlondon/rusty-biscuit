use std::collections::{HashMap, HashSet};

use crate::error::TreeHuggerError;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    AnalysisPass, CommentAttachment, Diagnostic, DocsFacet, ExportRecord, FileSymbolIndex,
    ImportRecord, ParsedDocs, SemanticFacet, SymbolKindV2, SymbolRef, TriState, now_epoch_ms,
};

/// Runs the staged analysis pipeline for a file.
pub fn analyze_file(
    tree_file: &TreeFile,
    requested_passes: &[AnalysisPass],
) -> Result<FileSymbolIndex, TreeHuggerError> {
    let mut index = parse_pass(tree_file)?;

    for pass in requested_passes {
        match pass {
            AnalysisPass::Parse => {
                complete_pass(&mut index, AnalysisPass::Parse);
            }
            AnalysisPass::Bind => bind_pass(tree_file, &mut index)?,
            AnalysisPass::Semantic => semantic_pass(&mut index),
            AnalysisPass::Docs => docs_pass(&mut index),
        }
    }

    Ok(index)
}

/// Parse pass: builds the baseline `FileSymbolIndex` with v2 symbol records.
fn parse_pass(tree_file: &TreeFile) -> Result<FileSymbolIndex, TreeHuggerError> {
    let symbols = tree_file.symbol_records()?;
    let imports = tree_file
        .imported_symbols()?
        .into_iter()
        .map(|import| ImportRecord {
            name: import.name.clone(),
            alias: import.alias,
            source: import.source,
            span: crate::shared::TextSpan {
                start: crate::shared::TextPoint {
                    line: import.range.start_line as u32,
                    column: import.range.start_column as u32,
                },
                end: crate::shared::TextPoint {
                    line: import.range.end_line as u32,
                    column: import.range.end_column as u32,
                },
                start_byte: import.range.start_byte as u32,
                end_byte: import.range.end_byte as u32,
            },
            symbol: SymbolRef {
                id: None,
                name: import.name,
                qualified_name: None,
            },
        })
        .collect::<Vec<_>>();

    let exports = tree_file
        .exported_symbols()?
        .into_iter()
        .map(|export| ExportRecord {
            name: export.name.clone(),
            span: crate::shared::TextSpan {
                start: crate::shared::TextPoint {
                    line: export.range.start_line as u32,
                    column: export.range.start_column as u32,
                },
                end: crate::shared::TextPoint {
                    line: export.range.end_line as u32,
                    column: export.range.end_column as u32,
                },
                start_byte: export.range.start_byte as u32,
                end_byte: export.range.end_byte as u32,
            },
            symbol: SymbolRef {
                id: None,
                name: export.name,
                qualified_name: None,
            },
        })
        .collect::<Vec<_>>();

    let diagnostics: Vec<Diagnostic> = tree_file.diagnostics();

    Ok(FileSymbolIndex {
        schema_version: crate::shared::SchemaVersion::V2_0,
        file: tree_file.file.clone(),
        language: tree_file.language,
        file_hash: tree_file.hash.clone(),
        completed_passes: vec![AnalysisPass::Parse],
        symbols,
        imports,
        exports,
        diagnostics,
    })
}

/// Bind pass: fills basic relation edges and resolves IDs by name.
fn bind_pass(tree_file: &TreeFile, index: &mut FileSymbolIndex) -> Result<(), TreeHuggerError> {
    let mut symbols_by_name: HashMap<String, crate::shared::SymbolId> = HashMap::new();
    for symbol in &index.symbols {
        symbols_by_name.insert(symbol.identity.name.clone(), symbol.id.clone());
    }

    let references = tree_file.referenced_symbols()?;
    let mut references_by_name: HashMap<String, usize> = HashMap::new();
    for reference in references {
        *references_by_name.entry(reference.name).or_insert(0) += 1;
    }

    for symbol in &mut index.symbols {
        let mut seen = HashSet::new();

        symbol.relations.references.clear();
        for import in &index.imports {
            if seen.insert(import.name.clone()) {
                symbol.relations.references.push(SymbolRef {
                    id: symbols_by_name.get(&import.name).cloned(),
                    name: import.name.clone(),
                    qualified_name: None,
                });
            }
        }

        symbol.relations.dependencies = symbol.relations.references.clone();

        if references_by_name.contains_key(&symbol.identity.name) {
            symbol.relations.referenced_by.push(SymbolRef {
                id: symbols_by_name.get(&symbol.identity.name).cloned(),
                name: symbol.identity.name.clone(),
                qualified_name: symbol.identity.qualified_name.clone(),
            });
        }

        // Export state is finalized in bind pass based on export edges.
        symbol.visibility.is_exported = index.exports.iter().any(|export| {
            export.name == symbol.identity.name
                || export.symbol.id.as_ref().is_some_and(|id| id == &symbol.id)
        });

        symbol.provenance.parse_pass = AnalysisPass::Bind.as_str().to_string();
        symbol.provenance.updated_at_epoch_ms = now_epoch_ms();
    }

    complete_pass(index, AnalysisPass::Bind);
    Ok(())
}

/// Semantic pass: fills initial low-risk semantic flags.
fn semantic_pass(index: &mut FileSymbolIndex) {
    for symbol in &mut index.symbols {
        if !matches!(symbol.kind, SymbolKindV2::Function | SymbolKindV2::Method) {
            continue;
        }

        let mut semantics = SemanticFacet::default();
        let is_recursive = symbol
            .relations
            .references
            .iter()
            .any(|reference| reference.name == symbol.identity.name);

        semantics.is_recursive = Some(if is_recursive {
            TriState::Yes
        } else {
            TriState::No
        });

        let panics = symbol
            .relations
            .references
            .iter()
            .any(|reference| reference.name == "panic" || reference.name == "panic!");

        semantics.may_panic = Some(if panics {
            TriState::Yes
        } else {
            TriState::Unknown
        });
        semantics.may_throw = Some(TriState::Unknown);
        semantics.mutates_self = Some(TriState::Unknown);
        semantics.mutates_arguments = Some(TriState::Unknown);

        symbol.semantics = semantics;
        symbol.provenance.parse_pass = AnalysisPass::Semantic.as_str().to_string();
        symbol.provenance.updated_at_epoch_ms = now_epoch_ms();
    }

    complete_pass(index, AnalysisPass::Semantic);
}

/// Docs pass: parses raw docs into structured `ParsedDocs`.
fn docs_pass(index: &mut FileSymbolIndex) {
    for symbol in &mut index.symbols {
        symbol.docs = parse_docs_facet(symbol.docs.clone());
        symbol.provenance.parse_pass = AnalysisPass::Docs.as_str().to_string();
        symbol.provenance.updated_at_epoch_ms = now_epoch_ms();
    }

    complete_pass(index, AnalysisPass::Docs);
}

fn parse_docs_facet(mut docs: DocsFacet) -> DocsFacet {
    let Some(raw) = docs.raw_doc.clone() else {
        return docs;
    };

    let mut parsed = ParsedDocs::default();
    let mut summary_lines = Vec::new();

    for line in raw.lines() {
        let trimmed = line
            .trim()
            .trim_start_matches("///")
            .trim_start_matches("//")
            .trim_start_matches('*')
            .trim();

        if let Some(rest) = trimmed.strip_prefix("@param") {
            let value = rest.trim();
            if let Some((name, description)) = value.split_once(' ') {
                parsed.params.push(crate::shared::DocParam {
                    name: name.trim().to_string(),
                    description: description.trim().to_string(),
                });
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("@returns") {
            parsed.returns = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('@')
            && let Some((name, value)) = rest.split_once(' ')
        {
            parsed.tags.push(crate::shared::DocTag {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            });
            continue;
        }

        if !trimmed.is_empty() {
            summary_lines.push(trimmed.to_string());
        }
    }

    if !summary_lines.is_empty() {
        parsed.summary = Some(summary_lines.join(" "));
    }

    if !parsed.params.is_empty() || parsed.returns.is_some() || parsed.summary.is_some() {
        docs.parsed = Some(parsed);
    }

    if docs.comments.is_empty() {
        // Fallback comment attachment when parse pass only populated raw_doc.
        if let Some(raw_doc) = docs.raw_doc.clone() {
            docs.comments.push(crate::shared::AttachedComment {
                kind: crate::shared::CommentKind::Doc,
                attachment: CommentAttachment::Leading,
                span: crate::shared::TextSpan {
                    start: crate::shared::TextPoint { line: 0, column: 0 },
                    end: crate::shared::TextPoint { line: 0, column: 0 },
                    start_byte: 0,
                    end_byte: 0,
                },
                raw_text: raw_doc.clone(),
                cleaned_text: raw_doc.trim().to_string(),
                line_distance: None,
            });
        }
    }

    docs
}

fn complete_pass(index: &mut FileSymbolIndex, pass: AnalysisPass) {
    if !index.completed_passes.contains(&pass) {
        index.completed_passes.push(pass);
    }
}

#[cfg(test)]
mod tests;
