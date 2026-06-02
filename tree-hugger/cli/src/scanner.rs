//! File scanning and positional-filter classification.

use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use tree_hugger::{ProgrammingLanguage, SymbolInfo, TreeHuggerError};

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

pub(crate) fn collect_files(
    root: &Path,
    inputs: &[String],
    excluded_files: &[String],
    language: Option<ProgrammingLanguage>,
) -> Result<Vec<PathBuf>, TreeHuggerError> {
    let mut files = Vec::new();
    let mut glob_inputs = Vec::new();

    // A forced `--language` lets us parse an explicitly named file regardless of
    // its extension (extensionless scripts, ambiguous names, or extensions that
    // map to a different language). Such files are honored verbatim and bypass
    // the extension-based scan filter below. Without a forced language we fall
    // back to the directory walk so extension detection still applies.
    for input in inputs {
        if language.is_some()
            && let Some(path) = resolve_explicit_file(input)
        {
            files.push(path);
        } else {
            glob_inputs.push(input.clone());
        }
    }

    // Apply `--exclude-files` to explicitly resolved files so exclusion is
    // consistent with the directory/glob scan below (which feeds the same
    // patterns into the walker overrides).
    if !files.is_empty() && !excluded_files.is_empty() {
        let mut exclude_builder = OverrideBuilder::new(root);
        for excluded_file in excluded_files {
            exclude_builder.add(excluded_file)?;
        }
        let exclude_matcher = exclude_builder.build()?;
        let cwd = std::env::current_dir().ok();
        files.retain(|file| {
            // Match against the same root-relative form the walker sees: the
            // override strips the (absolute) root prefix before glob matching,
            // so anchored patterns only line up once the file is absolute.
            let absolute = match (&cwd, file.is_absolute()) {
                (Some(cwd), false) => cwd.join(file),
                _ => file.clone(),
            };
            !exclude_matcher.matched(&absolute, false).is_whitelist()
        });
    }

    // Walk the tree when scanning the whole package (no inputs) or when some
    // inputs were glob patterns rather than concrete files.
    if inputs.is_empty() || !glob_inputs.is_empty() {
        let mut overrides = OverrideBuilder::new(root);
        for input in &glob_inputs {
            overrides.add(input)?;
        }
        for excluded_file in excluded_files {
            overrides.add(&format!("!{}", excluded_file))?;
        }

        let overrides = overrides.build()?;
        let walker = WalkBuilder::new(root)
            .standard_filters(true)
            .hidden(false)
            .overrides(overrides)
            .build();

        for entry in walker {
            let entry = entry.map_err(TreeHuggerError::Ignore)?;

            let is_file = entry
                .file_type()
                .map(|file| file.is_file())
                .unwrap_or(false);

            if !is_file {
                continue;
            }

            // For a forced language the candidate set is that language's
            // extensions; otherwise any supported file is eligible.
            match language {
                Some(lang) if ProgrammingLanguage::from_path(entry.path()) != Some(lang) => {
                    continue;
                }
                None if ProgrammingLanguage::from_path(entry.path()).is_none() => continue,
                _ => {}
            }

            files.push(entry.into_path());
        }
    }

    files.sort();
    files.dedup();

    if files.is_empty() {
        return Err(TreeHuggerError::NoSourceFiles {
            path: root.to_path_buf(),
        });
    }

    Ok(files)
}

/// Resolves a positional input token to a concrete existing file, relative to
/// the process working directory.
///
/// ## Returns
/// Returns the path when `input` names an existing file and is not a glob;
/// returns `None` for glob patterns or non-files so they flow to the scanner.
pub(crate) fn resolve_explicit_file(input: &str) -> Option<PathBuf> {
    if input.contains(['*', '?', '[']) {
        return None;
    }

    let candidate = PathBuf::from(input);
    candidate.is_file().then_some(candidate)
}
