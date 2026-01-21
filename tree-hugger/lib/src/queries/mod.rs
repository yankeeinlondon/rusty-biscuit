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
    /// Captures all identifier references (usages) in a file.
    References,
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
            Self::References => "references",
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
    // Syntax and DeadCode queries are not yet implemented
    if matches!(kind, QueryKind::Syntax | QueryKind::DeadCode) {
        return Ok(String::new());
    }

    // Lint queries come from language-specific directories
    if kind == QueryKind::Lint {
        return Ok(lint_query_by_name(language.query_name())
            .unwrap_or("")
            .to_string());
    }

    // References queries come from language-specific directories
    if kind == QueryKind::References {
        return Ok(references_query_by_name(language.query_name())
            .unwrap_or("")
            .to_string());
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

/// Loads lint query for a language from the language-specific directory.
fn lint_query_by_name(name: &str) -> Option<&'static str> {
    match name {
        "rust" => Some(include_str!("../../queries/rust/lint.scm")),
        "javascript" => Some(include_str!("../../queries/javascript/lint.scm")),
        "typescript" => Some(include_str!("../../queries/typescript/lint.scm")),
        "go" => Some(include_str!("../../queries/go/lint.scm")),
        "python" => Some(include_str!("../../queries/python/lint.scm")),
        "java" => Some(include_str!("../../queries/java/lint.scm")),
        "php" => Some(include_str!("../../queries/php/lint.scm")),
        "perl" => Some(include_str!("../../queries/perl/lint.scm")),
        "bash" => Some(include_str!("../../queries/bash/lint.scm")),
        "zsh" => Some(include_str!("../../queries/zsh/lint.scm")),
        "c" => Some(include_str!("../../queries/c/lint.scm")),
        "cpp" => Some(include_str!("../../queries/cpp/lint.scm")),
        "c_sharp" => Some(include_str!("../../queries/c_sharp/lint.scm")),
        "swift" => Some(include_str!("../../queries/swift/lint.scm")),
        "scala" => Some(include_str!("../../queries/scala/lint.scm")),
        "lua" => Some(include_str!("../../queries/lua/lint.scm")),
        _ => None,
    }
}

/// Loads references query for a language from the language-specific directory.
fn references_query_by_name(name: &str) -> Option<&'static str> {
    match name {
        "rust" => Some(include_str!("../../queries/rust/references.scm")),
        "javascript" => Some(include_str!("../../queries/javascript/references.scm")),
        "typescript" => Some(include_str!("../../queries/typescript/references.scm")),
        "go" => Some(include_str!("../../queries/go/references.scm")),
        "python" => Some(include_str!("../../queries/python/references.scm")),
        "java" => Some(include_str!("../../queries/java/references.scm")),
        "php" => Some(include_str!("../../queries/php/references.scm")),
        "perl" => Some(include_str!("../../queries/perl/references.scm")),
        "bash" => Some(include_str!("../../queries/bash/references.scm")),
        "zsh" => Some(include_str!("../../queries/zsh/references.scm")),
        "c" => Some(include_str!("../../queries/c/references.scm")),
        "cpp" => Some(include_str!("../../queries/cpp/references.scm")),
        "c_sharp" => Some(include_str!("../../queries/c_sharp/references.scm")),
        "swift" => Some(include_str!("../../queries/swift/references.scm")),
        "scala" => Some(include_str!("../../queries/scala/references.scm")),
        "lua" => Some(include_str!("../../queries/lua/references.scm")),
        _ => None,
    }
}

use crate::shared::DiagnosticSeverity;

/// Maps rule IDs to their severity level.
pub fn severity_for_rule(rule_id: &str) -> DiagnosticSeverity {
    match rule_id {
        // Error-level rules (semantic)
        "unreachable-code" | "invalid-syntax" | "undefined-variable" | "undefined-symbol" => {
            DiagnosticSeverity::Error
        }
        // Warning-level rules (semantic)
        "unused-variable" | "shadowed-variable" | "unused-symbol" | "unused-import"
        | "dead-code" => DiagnosticSeverity::Warning,
        // Warning-level rules (pattern)
        "unwrap-call" | "expect-call" | "dbg-macro" | "eval-call" | "exec-call"
        | "debugger-statement" | "breakpoint-call" | "deprecated-syntax" => {
            DiagnosticSeverity::Warning
        }
        // Default to info
        _ => DiagnosticSeverity::Info,
    }
}

/// Generates a human-readable message for a lint rule.
pub fn format_rule_message(rule_id: &str) -> String {
    match rule_id {
        // Semantic rules
        "undefined-symbol" => "Reference to undefined symbol".to_string(),
        "unused-symbol" => "Symbol defined but never used".to_string(),
        "unused-import" => "Imported symbol is never used".to_string(),
        "dead-code" => "Unreachable code after unconditional exit".to_string(),
        // Pattern rules
        "unwrap-call" => "Explicit unwrap() call".to_string(),
        "expect-call" => "Explicit expect() call".to_string(),
        "dbg-macro" => "Debug macro dbg!() call".to_string(),
        "eval-call" => "Use of eval() is discouraged".to_string(),
        "exec-call" => "Use of exec() is discouraged".to_string(),
        "debugger-statement" => "Debugger statement found".to_string(),
        "breakpoint-call" => "Breakpoint call found".to_string(),
        // Legacy rules (kept for compatibility)
        "unused-variable" => "Potentially unused variable".to_string(),
        "shadowed-variable" => "Variable shadows outer binding".to_string(),
        "unreachable-code" => "Unreachable code detected".to_string(),
        "deprecated-syntax" => "Deprecated syntax".to_string(),
        _ => format!("Lint rule: {rule_id}"),
    }
}
