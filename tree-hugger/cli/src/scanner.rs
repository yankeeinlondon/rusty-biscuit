//! File scanning and positional-filter classification.

use std::path::Path;

use tree_hugger::scanner::resolve_explicit_file;
use tree_hugger::ProgrammingLanguage;

use crate::CommandKind;

/// Classification of positional filter tokens.
#[derive(Debug, Clone, Default)]
pub(crate) struct ScanFilters {
    pub(crate) file_filters: Vec<String>,
    /// Name filter tokens as given (see `selection::Selection`).
    pub(crate) symbol_names: Vec<String>,
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
            let mut symbol_names = Vec::new();
            for filter in filters {
                // With a forced `--language`, a positional token that names an
                // existing file is a file filter even when it lacks a recognized
                // extension (e.g. an extensionless script). Without this it would
                // be misclassified as a symbol name and never reach the
                // explicit-file resolver in `collect_selected_files`.
                let forced_explicit_file =
                    language.is_some() && resolve_explicit_file(filter).is_some();
                if is_file_filter_token(filter) || forced_explicit_file {
                    file_filters.push(filter.clone());
                } else {
                    symbol_names.push(filter.clone());
                }
            }
            ScanFilters {
                file_filters,
                symbol_names,
            }
        }
        CommandKind::Imports | CommandKind::Lint { .. } => ScanFilters {
            file_filters: filters.to_vec(),
            symbol_names: Vec::new(),
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

// Re-export the library's discovery so the CLI and lib share one
// implementation.
pub(crate) use tree_hugger::scanner::collect_selected_files;
