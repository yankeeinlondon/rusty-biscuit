//! Prelude export discovery and resolution (`--prelude`).
//!
//! Resolution is the first of two scan phases: the prelude files are
//! analyzed, their public re-exports are resolved against the modules they
//! name, and every file analyzed on the way is kept in a request-local map
//! the scan reuses, so no file is parsed twice. A target module outside the
//! scan is analyzed on demand and never listed.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_hugger::{
    CodeRange, ImportRecord, ProgrammingLanguage, SymbolInfo, SymbolKind, SymbolKindData, SymbolScope,
    SymbolQuery, SymbolRecord, TextSpan, TreeHuggerError, file_identity, has_package_manifest,
    stable_symbol_key, symbol_id_from_stable_key,
};

use crate::scan::{Analyzed, Analyzer};

/// Resolved prelude export filter data.
#[derive(Debug, Clone)]
pub(crate) struct PreludeFilter {
    /// Symbol names explicitly listed by prelude exports (plus PRELUDE env var).
    pub(crate) names: HashSet<String>,
    /// Synthetic export records keyed by prelude file path: one per public
    /// re-export, under its exported (alias) name, at its target's
    /// declaration when resolved.
    pub(crate) exports_by_file: HashMap<PathBuf, Vec<SymbolRecord>>,
    /// Names contributed only by the `PRELUDE` env var (no resolved metadata).
    ///
    /// These need to be filtered against file scans because they aren't
    /// represented in [`Self::exports_by_file`].
    pub(crate) env_only_names: HashSet<String>,
}

/// Resolves symbol names and direct export entries from a prelude module.
///
/// Resolves prelude exports from `{root}/src/prelude.rs`,
/// `{root}/src/prelude/mod.rs`, and direct child package roots such as
/// `{root}/lib/src/prelude.rs`.
///
/// Every file is analyzed with `query`, which must select every kind (a
/// target is looked up among all of a module's declarations), and kept in
/// `analyzed` for the scan to reuse. Merges any additional names from the
/// `PRELUDE` environment variable (comma-separated).
pub(crate) fn resolve_prelude_symbols(
    root_dir: &Path,
    analyzer: &Analyzer,
    query: &SymbolQuery,
    analyzed: &mut HashMap<PathBuf, Analyzed>,
) -> Result<PreludeFilter, TreeHuggerError> {
    let mut names = HashSet::new();
    let mut exports_by_file: HashMap<PathBuf, Vec<SymbolRecord>> = HashMap::new();

    for candidate in discover_prelude_files(root_dir)? {
        // Keyed by identity so the scan finds the analysis under whatever
        // spelling it lists the file.
        let candidate = file_identity(&candidate);
        // The export lines are read from the bytes the analysis read.
        let source = match analyzed.get(&candidate) {
            Some(_) => std::fs::read_to_string(&candidate).map_err(|source| TreeHuggerError::Io {
                path: candidate.clone(),
                source,
            })?,
            None => {
                let file = analyzer.read(&candidate)?;
                let source = file.source().to_string();
                analyzed.insert(candidate.clone(), analyzer.analyze_source(file, query)?);
                source
            }
        };
        let prelude = &analyzed[&candidate];
        let language = prelude.result.index.language;
        let exports: Vec<ImportRecord> = prelude
            .result
            .index
            .imports
            .iter()
            .filter(|import| is_prelude_export(import, language, &source))
            .cloned()
            .collect();
        let package_root =
            package_root_for_prelude_file(&candidate).unwrap_or_else(|| root_dir.to_path_buf());

        let mut resolved_exports = Vec::with_capacity(exports.len());
        for (ordinal, import) in exports.iter().enumerate() {
            names.insert(import.name.clone());
            let target = resolve_prelude_export(import, language, &package_root, analyzer, query, analyzed)?;
            resolved_exports.push(synthetic_export(import, ordinal, &candidate, language, target));
        }

        exports_by_file.insert(candidate, resolved_exports);
    }

    let resolved_names: HashSet<String> = exports_by_file
        .values()
        .flatten()
        .map(|symbol| symbol.identity.name.clone())
        .collect();

    let mut env_only_names = HashSet::new();
    if let Ok(env_val) = std::env::var("PRELUDE") {
        for name in env_val.split(',') {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }
            names.insert(trimmed.to_string());
            if !resolved_names.contains(trimmed) {
                env_only_names.insert(trimmed.to_string());
            }
        }
    }

    Ok(PreludeFilter {
        names,
        exports_by_file,
        env_only_names,
    })
}

/// `file`'s analysis from `analyzed`, analyzing it first if this run has
/// not yet.
fn analyzed_file<'a>(
    file: &Path,
    analyzer: &Analyzer,
    query: &SymbolQuery,
    analyzed: &'a mut HashMap<PathBuf, Analyzed>,
) -> Result<&'a Analyzed, TreeHuggerError> {
    if !analyzed.contains_key(file) {
        let analysis = analyzer.analyze(file, query)?;
        analyzed.insert(file.to_path_buf(), analysis);
    }
    Ok(&analyzed[file])
}

pub(crate) fn discover_prelude_files(root_dir: &Path) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut package_roots = BTreeSet::new();
    package_roots.insert(root_dir.to_path_buf());

    if let Ok(entries) = std::fs::read_dir(root_dir) {
        for entry in entries {
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if has_package_manifest(&path) {
                package_roots.insert(path);
            }
        }
    }

    let mut files = Vec::new();
    for package_root in package_roots {
        let prelude_rs = package_root.join("src/prelude.rs");
        if prelude_rs.is_file() {
            files.push(prelude_rs);
        }

        let prelude_mod_rs = package_root.join("src/prelude/mod.rs");
        if prelude_mod_rs.is_file() {
            files.push(prelude_mod_rs);
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

pub(crate) fn package_root_for_prelude_file(path: &Path) -> Option<PathBuf> {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("prelude.rs") => path.parent().and_then(Path::parent).map(Path::to_path_buf),
        Some("mod.rs") => path
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf),
        _ => None,
    }
}

/// Whether `import` re-exports: every import of a non-Rust prelude, and a
/// Rust `use` only when the statement is `pub`.
pub(crate) fn is_prelude_export(import: &ImportRecord, language: ProgrammingLanguage, file_source: &str) -> bool {
    if language != ProgrammingLanguage::Rust {
        return true;
    }

    let Some(statement_span) = import.statement_span.as_ref() else {
        return true;
    };
    let Some(statement) =
        file_source.get(statement_span.start_byte as usize..statement_span.end_byte as usize)
    else {
        return false;
    };

    is_public_rust_use_statement(statement)
}

pub(crate) fn is_public_rust_use_statement(statement: &str) -> bool {
    let trimmed = statement.trim_start();
    let Some(after_pub) = trimmed.strip_prefix("pub") else {
        return false;
    };
    let after_pub = after_pub.trim_start();
    if after_pub.starts_with("use ") {
        return true;
    }

    if let Some(after_scope) = after_pub.strip_prefix('(')
        && let Some(close_idx) = after_scope.find(')')
    {
        return after_scope[close_idx + 1..]
            .trim_start()
            .starts_with("use ");
    }

    false
}

/// What a prelude re-export resolved to.
enum ExportTarget {
    /// The target's declaration record.
    Declaration(Box<SymbolRecord>),
    /// A kind read from the module text, for a declaration the extractor
    /// does not capture; its location stays unknown.
    InferredKind(SymbolKind),
    Unresolved,
}

fn resolve_prelude_export(
    import: &ImportRecord,
    language: ProgrammingLanguage,
    root_dir: &Path,
    analyzer: &Analyzer,
    query: &SymbolQuery,
    analyzed: &mut HashMap<PathBuf, Analyzed>,
) -> Result<ExportTarget, TreeHuggerError> {
    if language != ProgrammingLanguage::Rust {
        return Ok(ExportTarget::Unresolved);
    }

    let Some(source) = import.source.as_deref() else {
        return Ok(ExportTarget::Unresolved);
    };
    let target_name = import_target_symbol_name(import);

    for candidate in rust_module_source_candidates(root_dir, source) {
        if !candidate.is_file() {
            continue;
        }
        let candidate = file_identity(&candidate);

        // Exported declarations first, as the module's public surface; a
        // module with none falls back to every declaration.
        let records = &analyzed_file(&candidate, analyzer, query, analyzed)?.result.index.symbols;
        let has_exports = records.iter().any(|record| record.visibility.is_exported);
        if let Some(record) = records.iter().find(|record| {
            (record.visibility.is_exported || !has_exports) && record.identity.name == target_name
        }) {
            return Ok(ExportTarget::Declaration(Box::new(record.clone())));
        }

        // Fallback for symbols not captured by tree-sitter symbol queries (e.g., some type aliases).
        let source_text =
            std::fs::read_to_string(&candidate).map_err(|source| TreeHuggerError::Io {
                path: candidate.clone(),
                source,
            })?;
        if let Some(kind) = infer_rust_decl_kind(&source_text, target_name) {
            return Ok(ExportTarget::InferredKind(kind));
        }
    }

    Ok(ExportTarget::Unresolved)
}

pub(crate) fn import_target_symbol_name(import: &ImportRecord) -> &str {
    let raw = import
        .symbol
        .qualified_name
        .as_deref()
        .unwrap_or(import.name.as_str());
    raw.rsplit("::").next().unwrap_or(raw)
}

pub(crate) fn infer_rust_decl_kind(source: &str, target_name: &str) -> Option<SymbolKind> {
    let patterns = [
        (SymbolKind::Trait, format!("pub trait {target_name}")),
        (SymbolKind::Enum, format!("pub enum {target_name}")),
        (SymbolKind::Type, format!("pub struct {target_name}")),
        (SymbolKind::Type, format!("pub type {target_name}")),
        (SymbolKind::Function, format!("pub fn {target_name}")),
        (SymbolKind::Constant, format!("pub const {target_name}")),
        (SymbolKind::Constant, format!("pub static {target_name}")),
    ];

    patterns
        .iter()
        .find_map(|(kind, marker)| source.contains(marker).then_some(*kind))
}

pub(crate) fn rust_module_source_candidates(root_dir: &Path, source: &str) -> Vec<PathBuf> {
    let normalized = source
        .trim()
        .trim_start_matches("crate::")
        .trim_start_matches("self::")
        .trim_start_matches("::")
        .trim_end_matches("::");

    if normalized.is_empty() || normalized.starts_with("super::") {
        return Vec::new();
    }

    let module = normalized.replace("::", "/");
    vec![
        root_dir.join("src").join(format!("{module}.rs")),
        root_dir.join("src").join(module).join("mod.rs"),
    ]
}

/// The record a prelude lists for one re-export: the exported (alias) name,
/// the target's kind, keyword, doc comment and declaration location when
/// resolved (else the `use` itself), and nothing that belongs to the target
/// alone (signature, members, container, access).
///
/// It is marked synthetic and re-exported, and its ID is derived from the
/// prelude file and its position there, so it never borrows the target's ID
/// while naming a different symbol.
fn synthetic_export(
    import: &ImportRecord,
    ordinal: usize,
    prelude_file: &Path,
    language: ProgrammingLanguage,
    target: ExportTarget,
) -> SymbolRecord {
    let (kind, file, span, doc_comment, keyword) = match &target {
        ExportTarget::Declaration(record) => (
            SymbolKind::from(record.kind),
            record.source.file_path.clone(),
            &record.source.declaration_span,
            record.docs.raw_doc.clone(),
            record.identity.keyword.clone(),
        ),
        ExportTarget::InferredKind(kind) => (*kind, prelude_file.to_path_buf(), &import.span, None, None),
        ExportTarget::Unresolved => (
            SymbolKind::Unknown,
            prelude_file.to_path_buf(),
            &import.span,
            None,
            None,
        ),
    };
    let mut record = SymbolRecord::from(SymbolInfo {
        name: import.name.clone(),
        kind,
        // A re-export is a file-level name of the prelude.
        scope: SymbolScope::Item,
        qualified_name: import.name.clone(),
        is_namespace_rooted: false,
        range: code_range(span),
        language,
        file,
        container_name: None,
        container_kind: None,
        doc_comment,
        visibility: None,
        is_static: false,
        is_static_storage: false,
        is_async: false,
        signature: None,
        type_metadata: None,
    });
    record.kind_data = SymbolKindData::Unknown;
    record.identity.keyword = keyword;
    record.identity.stable_key = stable_symbol_key(
        language,
        prelude_file,
        record.kind,
        &format!("prelude-export::{}", import.name),
        ordinal as u32,
    );
    record.id = symbol_id_from_stable_key(&record.identity.stable_key);
    record.visibility.is_exported = true;
    record.visibility.is_reexported = true;
    record.visibility.is_synthetic = true;
    record
}

fn code_range(span: &TextSpan) -> CodeRange {
    CodeRange {
        start_line: span.start.line as usize,
        start_column: span.start.column as usize,
        end_line: span.end.line as usize,
        end_column: span.end.column as usize,
        start_byte: span.start_byte as usize,
        end_byte: span.end_byte as usize,
    }
}
