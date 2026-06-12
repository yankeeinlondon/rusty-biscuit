//! File scanning and positional-filter classification.

use std::path::Path;

use tree_hugger::scanner::resolve_explicit_file;
use tree_hugger::{ProgrammingLanguage, SymbolInfo};

use crate::CommandKind;

/// Classification of positional filter tokens.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScanFilters {
    pub(crate) file_filters: Vec<String>,
    pub(crate) symbol_globs: Vec<String>,
}

pub(crate) fn classify_filters(
    filters: &[String],
    command: &CommandKind,
    language: Option<ProgrammingLanguage>,
) -> ScanFilters {
    match command {
        CommandKind::Functions
        | CommandKind::Types
        | CommandKind::Symbols
        | CommandKind::Classes { .. } => {
            let mut file_filters = Vec::new();
            let mut symbol_globs = Vec::new();
            for filter in filters {
                // With a forced `--language`, a positional token that names an
                // existing file is a file filter even when it lacks a recognized
                // extension (e.g. an extensionless script). Without this it would
                // be misclassified as a symbol glob and never reach the
                // explicit-file resolver in `collect_files`.
                let forced_explicit_file =
                    language.is_some() && resolve_explicit_file(filter).is_some();
                if is_file_filter_token(filter) || forced_explicit_file {
                    file_filters.push(filter.clone());
                } else {
                    symbol_globs.push(normalize_symbol_glob(filter));
                }
            }
            ScanFilters {
                file_filters,
                symbol_globs,
            }
        }
        CommandKind::Imports | CommandKind::Lint { .. } => ScanFilters {
            file_filters: filters.to_vec(),
            symbol_globs: Vec::new(),
        },
    }
}

pub(crate) fn is_file_filter_token(token: &str) -> bool {
    if token.contains('/') || token.contains('\\') {
        return true;
    }

    let extension = Path::new(token).extension().and_then(|ext| ext.to_str());
    extension
        .and_then(ProgrammingLanguage::from_extension)
        .is_some()
}

pub(crate) fn normalize_symbol_glob(token: &str) -> String {
    if let Some(strict_name) = token.strip_suffix('!')
        && !strict_name.is_empty()
    {
        // Trailing `!` switches from fuzzy auto-wrapped matching to strict exact matching.
        // Example: `parse_width_spec!` matches only `parse_width_spec`.
        return strict_name.to_string();
    }

    if token.contains('*') {
        token.to_string()
    } else {
        format!("*{token}*")
    }
}

pub(crate) fn normalize_excluded_symbol_glob(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }

    if let Some(strict_name) = token.strip_suffix('!')
        && !strict_name.is_empty()
    {
        // Keep parity with positional filters: trailing `!` means strict name.
        return Some(strict_name.to_string());
    }

    Some(token.to_string())
}

pub(crate) fn matches_symbol_filters(name: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return true;
    }

    patterns
        .iter()
        .any(|pattern| wildcard_match(pattern.as_str(), name))
}

pub(crate) fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');
    let segments: Vec<&str> = pattern
        .split('*')
        .filter(|segment| !segment.is_empty())
        .collect();
    if segments.is_empty() {
        return true;
    }

    let mut index = 0usize;
    for (position, segment) in segments.iter().enumerate() {
        if position == 0 && !starts_with_wildcard {
            if !value[index..].starts_with(segment) {
                return false;
            }
            index += segment.len();
            continue;
        }

        match value[index..].find(segment) {
            Some(found) => {
                index += found + segment.len();
            }
            None => return false,
        }
    }

    if ends_with_wildcard {
        true
    } else {
        value.ends_with(segments.last().unwrap_or(&""))
    }
}

pub(crate) fn apply_symbol_filters(
    symbols: Vec<SymbolInfo>,
    include_symbol_globs: &[String],
    exclude_symbol_globs: &[String],
) -> Vec<SymbolInfo> {
    if include_symbol_globs.is_empty() && exclude_symbol_globs.is_empty() {
        return symbols;
    }

    symbols
        .into_iter()
        .filter(|symbol| {
            let included = include_symbol_globs.is_empty()
                || matches_symbol_filters(&symbol.name, include_symbol_globs);
            let excluded = !exclude_symbol_globs.is_empty()
                && matches_symbol_filters(&symbol.name, exclude_symbol_globs);
            included && !excluded
        })
        .collect()
}

// Re-export `collect_files` from the library so the CLI and lib share one
// implementation.
pub(crate) use tree_hugger::scanner::collect_files;
