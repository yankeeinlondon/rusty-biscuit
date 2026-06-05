//! Prelude export discovery and resolution.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use tree_hugger::{
    CodeRange, ImportSymbol, ProgrammingLanguage, SymbolInfo, SymbolKind, TreeFile, TreeHuggerError,
    has_package_manifest,
};


/// Resolved prelude export filter data.
#[derive(Debug, Clone)]
pub(crate) struct PreludeFilter {
    /// Symbol names explicitly listed by prelude exports (plus PRELUDE env var).
    pub(crate) names: HashSet<String>,
    /// Resolved prelude export symbols keyed by prelude file path.
    pub(crate) exports_by_file: HashMap<PathBuf, Vec<SymbolInfo>>,
    /// Names contributed only by the `PRELUDE` env var (no resolved metadata).
    ///
    /// These need to be filtered against file scans because they aren't
    /// represented in [`Self::exports_by_file`].
    pub(crate) env_only_names: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedPreludeMetadata {
    kind: SymbolKind,
    doc_comment: Option<String>,
    file: Option<PathBuf>,
    range: Option<CodeRange>,
}

/// Resolves symbol names and direct export entries from a prelude module.
///
/// Resolves prelude exports from `{root}/src/prelude.rs`,
/// `{root}/src/prelude/mod.rs`, and direct child package roots such as
/// `{root}/lib/src/prelude.rs`.
///
/// Parses public `use` exports and merges any additional names from the
/// `PRELUDE` environment variable (comma-separated).
pub(crate) fn resolve_prelude_symbols(root_dir: &Path) -> Result<PreludeFilter, TreeHuggerError> {
    let mut names = HashSet::new();
    let mut exports_by_file: HashMap<PathBuf, Vec<SymbolInfo>> = HashMap::new();
    let mut rust_symbol_cache: HashMap<PathBuf, Vec<SymbolInfo>> = HashMap::new();

    for candidate in discover_prelude_files(root_dir)? {
        let tree_file = TreeFile::with_language(&candidate, None)?;
        let source = std::fs::read_to_string(&candidate).map_err(|source| TreeHuggerError::Io {
            path: candidate.clone(),
            source,
        })?;
        let exports: Vec<ImportSymbol> = tree_file
            .imported_symbols()?
            .into_iter()
            .filter(|import| is_prelude_export(import, &source))
            .collect();
        let mut resolved_exports = Vec::with_capacity(exports.len());
        let package_root =
            package_root_for_prelude_file(&candidate).unwrap_or_else(|| root_dir.to_path_buf());

        for import in &exports {
            names.insert(import.name.clone());
            let metadata =
                resolve_prelude_export_metadata(import, &package_root, &mut rust_symbol_cache)?
                    .unwrap_or(ResolvedPreludeMetadata {
                        kind: SymbolKind::Unknown,
                        doc_comment: None,
                        file: None,
                        range: None,
                    });
            resolved_exports.push(symbol_from_prelude_export(
                import,
                metadata.kind,
                metadata.doc_comment,
                metadata.file,
                metadata.range,
            ));
        }

        exports_by_file.insert(candidate, resolved_exports);
    }

    let resolved_names: HashSet<String> = exports_by_file
        .values()
        .flatten()
        .map(|symbol| symbol.name.clone())
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

pub(crate) fn is_prelude_export(import: &ImportSymbol, file_source: &str) -> bool {
    if import.language != ProgrammingLanguage::Rust {
        return true;
    }

    let Some(statement_range) = import.statement_range.as_ref() else {
        return true;
    };
    let Some(statement) = file_source.get(statement_range.start_byte..statement_range.end_byte)
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

pub(crate) fn resolve_prelude_export_metadata(
    import: &ImportSymbol,
    root_dir: &Path,
    rust_symbol_cache: &mut HashMap<PathBuf, Vec<SymbolInfo>>,
) -> Result<Option<ResolvedPreludeMetadata>, TreeHuggerError> {
    if import.language != ProgrammingLanguage::Rust {
        return Ok(None);
    }

    let Some(source) = import.source.as_deref() else {
        return Ok(None);
    };
    let target_name = import_target_symbol_name(import);

    for candidate in rust_module_source_candidates(root_dir, source) {
        if !candidate.is_file() {
            continue;
        }

        if !rust_symbol_cache.contains_key(&candidate) {
            let tree_file = TreeFile::with_language(&candidate, Some(ProgrammingLanguage::Rust))?;
            let mut symbols = tree_file.exported_symbols()?;
            if symbols.is_empty() {
                symbols = tree_file.symbols()?;
            }
            rust_symbol_cache.insert(candidate.clone(), symbols);
        }

        if let Some(symbol) = rust_symbol_cache
            .get(&candidate)
            .and_then(|symbols| symbols.iter().find(|symbol| symbol.name == target_name))
        {
            return Ok(Some(ResolvedPreludeMetadata {
                kind: symbol.kind,
                doc_comment: symbol.doc_comment.clone(),
                file: Some(symbol.file.clone()),
                range: Some(symbol.range.clone()),
            }));
        }

        // Fallback for symbols not captured by tree-sitter symbol queries (e.g., some type aliases).
        let source_text =
            std::fs::read_to_string(&candidate).map_err(|source| TreeHuggerError::Io {
                path: candidate.clone(),
                source,
            })?;
        if let Some(kind) = infer_rust_decl_kind(&source_text, target_name) {
            return Ok(Some(ResolvedPreludeMetadata {
                kind,
                doc_comment: None,
                file: None,
                range: None,
            }));
        }
    }

    Ok(None)
}

pub(crate) fn import_target_symbol_name(import: &ImportSymbol) -> &str {
    let raw = import
        .original_name
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

pub(crate) fn symbol_from_prelude_export(
    import: &ImportSymbol,
    kind: SymbolKind,
    doc_comment: Option<String>,
    resolved_file: Option<PathBuf>,
    resolved_range: Option<CodeRange>,
) -> SymbolInfo {
    SymbolInfo {
        name: import.name.clone(),
        kind,
        range: resolved_range.unwrap_or_else(|| import.range.clone()),
        language: import.language,
        file: resolved_file.unwrap_or_else(|| import.file.clone()),
        container_name: None,
        container_kind: None,
        doc_comment,
        signature: None,
        type_metadata: None,
    }
}
