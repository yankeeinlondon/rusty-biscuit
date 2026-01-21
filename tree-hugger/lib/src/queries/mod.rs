use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use tree_sitter::Query;

use crate::error::TreeHuggerError;
use crate::shared::ProgrammingLanguage;

/// Represents the type of tree-sitter query being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryKind {
    Locals,
    Imports,
    Exports,
    Lint,
    Syntax,
    DeadCode,
}

impl fmt::Display for QueryKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Locals => "locals",
            Self::Imports => "imports",
            Self::Exports => "exports",
            Self::Lint => "lint",
            Self::Syntax => "syntax",
            Self::DeadCode => "dead_code",
        };
        formatter.write_str(label)
    }
}

/// Cache type for compiled tree-sitter queries.
type QueryCache = Mutex<HashMap<(ProgrammingLanguage, QueryKind), Arc<Query>>>;

static QUERY_CACHE: OnceLock<QueryCache> = OnceLock::new();

/// Loads and caches a query for the requested language and kind.
pub fn query_for(
    language: ProgrammingLanguage,
    kind: QueryKind,
) -> Result<Arc<Query>, TreeHuggerError> {
    let cache = QUERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = cache
            .lock()
            .map_err(|_| TreeHuggerError::QueryCachePoisoned)?;
        if let Some(query) = guard.get(&(language, kind)) {
            return Ok(Arc::clone(query));
        }
    }

    let source = resolve_query_text(language, kind)?;

    let query = Arc::new(
        Query::new(&language.tree_sitter_language(), &source).map_err(|source| {
            TreeHuggerError::QueryError {
                language,
                kind,
                source,
            }
        })?,
    );

    let mut guard = cache
        .lock()
        .map_err(|_| TreeHuggerError::QueryCachePoisoned)?;
    guard.insert((language, kind), Arc::clone(&query));

    Ok(query)
}

fn resolve_query_text(
    language: ProgrammingLanguage,
    kind: QueryKind,
) -> Result<String, TreeHuggerError> {
    if matches!(
        kind,
        QueryKind::Lint | QueryKind::Syntax | QueryKind::DeadCode
    ) {
        return Ok(String::new());
    }

    let mut visited = HashSet::new();
    resolve_vendor_query(language.query_name(), &mut visited)
}

fn resolve_vendor_query(
    language_name: &str,
    visited: &mut HashSet<String>,
) -> Result<String, TreeHuggerError> {
    if !visited.insert(language_name.to_string()) {
        return Ok(String::new());
    }

    let source = vendor_locals_by_name(language_name).ok_or_else(|| {
        TreeHuggerError::MissingVendorQuery {
            name: language_name.to_string(),
        }
    })?;

    let (inherits, body) = split_inherits(source);
    let mut combined = String::new();

    for inherit in inherits {
        let inherited = resolve_vendor_query(&inherit, visited)?;
        if !inherited.is_empty() {
            combined.push_str(&inherited);
            combined.push('\n');
        }
    }

    combined.push_str(&body);

    Ok(combined)
}

fn split_inherits(source: &str) -> (Vec<String>, String) {
    let mut inherits = Vec::new();
    let mut body = Vec::new();

    for line in source.lines() {
        let mut trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(';') {
            trimmed = rest.trim_start();
            if let Some(rest) = trimmed.strip_prefix(';') {
                trimmed = rest.trim_start();
            }
        }

        if let Some(rest) = trimmed.strip_prefix("inherits:") {
            for entry in rest.split(',') {
                let name = entry.trim();
                if !name.is_empty() {
                    inherits.push(name.to_string());
                }
            }
            continue;
        }

        body.push(line);
    }

    (inherits, body.join("\n"))
}

fn vendor_locals_by_name(name: &str) -> Option<&'static str> {
    match name {
        "rust" => Some(include_str!("../../queries/vendor/rust/locals.scm")),
        "javascript" => Some(include_str!("../../queries/vendor/javascript/locals.scm")),
        "typescript" => Some(include_str!("../../queries/vendor/typescript/locals.scm")),
        "go" => Some(include_str!("../../queries/vendor/go/locals.scm")),
        "python" => Some(include_str!("../../queries/vendor/python/locals.scm")),
        "java" => Some(include_str!("../../queries/vendor/java/locals.scm")),
        "php" => Some(include_str!("../../queries/vendor/php/locals.scm")),
        "perl" => Some(include_str!("../../queries/vendor/perl/locals.scm")),
        "bash" => Some(include_str!("../../queries/vendor/bash/locals.scm")),
        "zsh" => Some(include_str!("../../queries/vendor/zsh/locals.scm")),
        "c" => Some(include_str!("../../queries/vendor/c/locals.scm")),
        "cpp" => Some(include_str!("../../queries/vendor/cpp/locals.scm")),
        "c_sharp" => Some(include_str!("../../queries/vendor/c_sharp/locals.scm")),
        "swift" => Some(include_str!("../../queries/vendor/swift/locals.scm")),
        "scala" => Some(include_str!("../../queries/vendor/scala/locals.scm")),
        "lua" => Some(include_str!("../../queries/vendor/lua/locals.scm")),
        "ecma" => Some(include_str!("../../queries/vendor/ecma/locals.scm")),
        "php_only" => Some(include_str!("../../queries/vendor/php_only/locals.scm")),
        _ => None,
    }
}
