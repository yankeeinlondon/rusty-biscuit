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
            statement_span: import
                .statement_range
                .as_ref()
                .map(|range| crate::shared::TextSpan {
                    start: crate::shared::TextPoint {
                        line: range.start_line as u32,
                        column: range.start_column as u32,
                    },
                    end: crate::shared::TextPoint {
                        line: range.end_line as u32,
                        column: range.end_column as u32,
                    },
                    start_byte: range.start_byte as u32,
                    end_byte: range.end_byte as u32,
                }),
            symbol: SymbolRef {
                id: None,
                name: import.name,
                qualified_name: import.original_name,
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
    let import_refs_by_name: HashMap<_, _> = index
        .imports
        .iter()
        .map(|import| {
            (
                import.name.clone(),
                SymbolRef {
                    id: import.symbol.id.clone(),
                    name: import.name.clone(),
                    qualified_name: import.symbol.qualified_name.clone(),
                },
            )
        })
        .collect();

    let mut parent_links: Vec<Option<SymbolRef>> = vec![None; index.symbols.len()];
    let mut members_by_parent: HashMap<crate::shared::SymbolId, Vec<SymbolRef>> = HashMap::new();

    for (child_idx, symbol) in index.symbols.iter().enumerate() {
        let Some(container_name) = symbol.identity.module_path.as_deref() else {
            continue;
        };

        let Some((parent_id, parent_ref)) =
            find_container_symbol(&index.symbols, child_idx, container_name)
        else {
            continue;
        };

        parent_links[child_idx] = Some(parent_ref.clone());
        members_by_parent
            .entry(parent_id)
            .or_default()
            .push(symbol_ref(symbol));
    }

    let mut references_by_owner: Vec<Vec<SymbolRef>> = vec![Vec::new(); index.symbols.len()];
    let mut dependencies_by_owner: Vec<Vec<SymbolRef>> = vec![Vec::new(); index.symbols.len()];
    let mut referenced_by_target: HashMap<crate::shared::SymbolId, Vec<SymbolRef>> = HashMap::new();

    for reference in tree_file.referenced_symbols()? {
        let Some(owner_idx) = find_owner_symbol(
            &index.symbols,
            reference.range.start_byte as u32,
            reference.range.end_byte as u32,
        ) else {
            continue;
        };

        let owner_ref = symbol_ref(&index.symbols[owner_idx]);
        let target_symbol = find_reference_target(&index.symbols, reference.name.as_str());

        if let Some(target) = target_symbol {
            push_unique_ref(&mut references_by_owner[owner_idx], symbol_ref(target));
            push_unique_ref(
                referenced_by_target.entry(target.id.clone()).or_default(),
                owner_ref,
            );
            continue;
        }

        if let Some(import_ref) = import_refs_by_name.get(&reference.name) {
            push_unique_ref(&mut references_by_owner[owner_idx], import_ref.clone());
            push_unique_ref(&mut dependencies_by_owner[owner_idx], import_ref.clone());
        }
    }

    let mut exported_ids = HashSet::new();
    for export in &mut index.exports {
        if let Some(symbol) = find_export_target(&index.symbols, export) {
            export.symbol.id = Some(symbol.id.clone());
            export.symbol.qualified_name = symbol.identity.qualified_name.clone();
            exported_ids.insert(symbol.id.clone());
        }
    }

    for (idx, symbol) in index.symbols.iter_mut().enumerate() {
        symbol.relations.references = references_by_owner[idx].clone();
        symbol.relations.dependencies = dependencies_by_owner[idx].clone();
        symbol.relations.parent = parent_links[idx].clone();
        symbol.relations.container = parent_links[idx].clone();
        symbol.relations.members = members_by_parent.remove(&symbol.id).unwrap_or_default();
        symbol.relations.referenced_by =
            referenced_by_target.remove(&symbol.id).unwrap_or_default();
        symbol.visibility.is_exported = exported_ids.contains(&symbol.id);
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

fn symbol_ref(symbol: &crate::shared::SymbolRecord) -> SymbolRef {
    SymbolRef {
        id: Some(symbol.id.clone()),
        name: symbol.identity.name.clone(),
        qualified_name: symbol.identity.qualified_name.clone(),
    }
}

fn push_unique_ref(targets: &mut Vec<SymbolRef>, candidate: SymbolRef) {
    let exists = targets.iter().any(|existing| {
        existing.id == candidate.id
            || (existing.name == candidate.name
                && existing.qualified_name == candidate.qualified_name)
    });
    if !exists {
        targets.push(candidate);
    }
}

fn find_owner_symbol(
    symbols: &[crate::shared::SymbolRecord],
    start_byte: u32,
    end_byte: u32,
) -> Option<usize> {
    symbols
        .iter()
        .enumerate()
        .filter(|(_, symbol)| {
            let span = &symbol.source.declaration_span;
            span.start_byte <= start_byte && span.end_byte >= end_byte
        })
        .min_by_key(|(_, symbol)| {
            symbol
                .source
                .declaration_span
                .end_byte
                .saturating_sub(symbol.source.declaration_span.start_byte)
        })
        .map(|(idx, _)| idx)
}

fn find_reference_target<'a>(
    symbols: &'a [crate::shared::SymbolRecord],
    name: &str,
) -> Option<&'a crate::shared::SymbolRecord> {
    symbols.iter().find(|symbol| symbol.identity.name == name)
}

fn find_container_symbol(
    symbols: &[crate::shared::SymbolRecord],
    child_idx: usize,
    container_name: &str,
) -> Option<(crate::shared::SymbolId, SymbolRef)> {
    let child = &symbols[child_idx];
    symbols
        .iter()
        .enumerate()
        .filter(|(idx, candidate)| {
            *idx != child_idx
                && candidate.identity.name == container_name
                && candidate.source.declaration_span.start_byte
                    <= child.source.declaration_span.start_byte
                && candidate.source.declaration_span.end_byte
                    >= child.source.declaration_span.end_byte
        })
        .min_by_key(|(_, candidate)| {
            candidate
                .source
                .declaration_span
                .end_byte
                .saturating_sub(candidate.source.declaration_span.start_byte)
        })
        .map(|(_, candidate)| (candidate.id.clone(), symbol_ref(candidate)))
}

fn find_export_target<'a>(
    symbols: &'a [crate::shared::SymbolRecord],
    export: &ExportRecord,
) -> Option<&'a crate::shared::SymbolRecord> {
    symbols.iter().find(|symbol| {
        symbol.identity.name == export.name
            && symbol.source.name_span.as_ref().is_some_and(|span| {
                span.start_byte == export.span.start_byte && span.end_byte == export.span.end_byte
            })
    })
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
