pub mod compatibility;
pub mod drift;
pub mod inventory;
pub mod provenance;

pub use provenance::{overlay_query_provenance, query_provenance, vendor_query_provenance};

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use tree_sitter::{Language, Query};

use crate::error::TreeHuggerError;
use crate::shared::ProgrammingLanguage;

/// Identifies the concrete tree-sitter grammar a query must be compiled
/// against.
///
/// A single [`ProgrammingLanguage`] can map to more than one grammar (notably
/// TypeScript, whose `.tsx` files use a distinct TSX grammar). Queries are
/// compiled against the grammar that actually parsed the tree, since node-kind
/// IDs differ between grammars; `id` keys the compiled-query cache so each
/// grammar variant is compiled and cached independently.
#[derive(Debug, Clone, Copy)]
pub struct GrammarRef<'a> {
    /// The language whose query files supply the query text.
    pub language: ProgrammingLanguage,
    /// The grammar the query is compiled against (and that parsed the tree).
    pub grammar: &'a Language,
    /// A stable identifier for the grammar variant, unique across languages.
    pub id: &'static str,
}

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
    /// Captures comment nodes for ignore directive parsing.
    Comments,
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
            Self::Comments => "comments",
        };
        formatter.write_str(label)
    }
}

/// Cache type for compiled tree-sitter queries, keyed by grammar variant.
type QueryCache = Mutex<HashMap<(&'static str, QueryKind), Arc<Query>>>;

static QUERY_CACHE: OnceLock<QueryCache> = OnceLock::new();

/// Loads and caches a query for the requested grammar and kind.
///
/// The query text is resolved from the grammar's [`language`] query files but
/// compiled against [`grammar`], so a TSX-parsed tree is matched with a query
/// compiled for the TSX grammar rather than the plain TypeScript grammar.
///
/// [`language`]: GrammarRef::language
/// [`grammar`]: GrammarRef::grammar
pub fn query_for(grammar: GrammarRef<'_>, kind: QueryKind) -> Result<Arc<Query>, TreeHuggerError> {
    let cache = QUERY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let cache_key = (grammar.id, kind);
    {
        let guard = cache
            .lock()
            .map_err(|_| TreeHuggerError::QueryCachePoisoned)?;
        if let Some(query) = guard.get(&cache_key) {
            return Ok(Arc::clone(query));
        }
    }

    let source = resolve_query_text(grammar.language, kind)?;

    let query = Arc::new(Query::new(grammar.grammar, &source).map_err(|source| {
        TreeHuggerError::QueryError {
            language: grammar.language,
            kind,
            source,
        }
    })?);

    let mut guard = cache
        .lock()
        .map_err(|_| TreeHuggerError::QueryCachePoisoned)?;
    guard.insert(cache_key, Arc::clone(&query));

    Ok(query)
}

/// Checks if query text is effectively empty (all whitespace or only comments).
///
/// Tree-sitter query comments start with `;`, so a file containing only
/// comment lines and whitespace has no actual query patterns to compile.
fn is_query_empty(source: &str) -> bool {
    source
        .lines()
        .all(|line| line.trim().is_empty() || line.trim().starts_with(';'))
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
        let source = lint_query_by_name(language.query_name()).unwrap_or("");
        // Return empty string for comment-only lint files to skip Query::new
        if is_query_empty(source) {
            return Ok(String::new());
        }
        return Ok(source.to_string());
    }

    // References queries come from language-specific directories
    if kind == QueryKind::References {
        let source = references_query_by_name(language.query_name()).unwrap_or("");
        // Return empty string for comment-only references files to skip Query::new
        if is_query_empty(source) {
            return Ok(String::new());
        }
        return Ok(source.to_string());
    }

    // Comments queries come from language-specific directories
    if kind == QueryKind::Comments {
        let source = comments_query_by_name(language.query_name()).unwrap_or("");
        if is_query_empty(source) {
            return Ok(String::new());
        }
        return Ok(source.to_string());
    }

    resolve_locals_query(language.query_name())
}

fn resolve_locals_query(language_name: &str) -> Result<String, TreeHuggerError> {
    let mut visited = HashSet::new();
    let vendor = resolve_vendor_query(language_name, &mut visited)?;

    let Some(overlay_source) = locals_overlay_by_name(language_name) else {
        return Ok(vendor);
    };

    let (inherits, extends, body) = split_query_modelines(overlay_source);
    let mut combined = String::new();

    if extends {
        combined.push_str(&vendor);
        if !vendor.is_empty() && !body.is_empty() {
            combined.push('\n');
        }
    }

    if !inherits.is_empty() {
        let mut overlay_visited = HashSet::new();
        for inherit in inherits {
            let inherited = resolve_vendor_query(&inherit, &mut overlay_visited)?;
            if !inherited.is_empty() {
                combined.push_str(&inherited);
                combined.push('\n');
            }
        }
    }

    combined.push_str(&body);
    Ok(combined)
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

    let (inherits, _, body) = split_query_modelines(source);
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

fn split_query_modelines(source: &str) -> (Vec<String>, bool, String) {
    let mut inherits = Vec::new();
    let mut extends = false;
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

        if trimmed == "extends" || trimmed.starts_with("extends:") {
            extends = true;
            continue;
        }

        body.push(line);
    }

    (inherits, extends, body.join("\n"))
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

fn locals_overlay_by_name(name: &str) -> Option<&'static str> {
    match name {
        "javascript" => Some(include_str!("../../queries/javascript/locals.scm")),
        "typescript" => Some(include_str!("../../queries/typescript/locals.scm")),
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

/// Loads comments query for a language from the language-specific directory.
fn comments_query_by_name(name: &str) -> Option<&'static str> {
    match name {
        "rust" => Some(include_str!("../../queries/rust/comments.scm")),
        "javascript" => Some(include_str!("../../queries/javascript/comments.scm")),
        "typescript" => Some(include_str!("../../queries/typescript/comments.scm")),
        "go" => Some(include_str!("../../queries/go/comments.scm")),
        "python" => Some(include_str!("../../queries/python/comments.scm")),
        "java" => Some(include_str!("../../queries/java/comments.scm")),
        "php" => Some(include_str!("../../queries/php/comments.scm")),
        "perl" => Some(include_str!("../../queries/perl/comments.scm")),
        "bash" => Some(include_str!("../../queries/bash/comments.scm")),
        "zsh" => Some(include_str!("../../queries/zsh/comments.scm")),
        "c" => Some(include_str!("../../queries/c/comments.scm")),
        "cpp" => Some(include_str!("../../queries/cpp/comments.scm")),
        "c_sharp" => Some(include_str!("../../queries/c_sharp/comments.scm")),
        "swift" => Some(include_str!("../../queries/swift/comments.scm")),
        "scala" => Some(include_str!("../../queries/scala/comments.scm")),
        "lua" => Some(include_str!("../../queries/lua/comments.scm")),
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
        | "dead-code" | "undefined-module" => DiagnosticSeverity::Warning,
        // Warning-level rules (pattern)
        "unwrap-call" | "expect-call" | "dbg-macro" | "eval-call" | "exec-call"
        | "debugger-statement" | "breakpoint-call" | "deprecated-syntax" | "console-log"
        | "print-call" | "fmt-println" => DiagnosticSeverity::Warning,
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
        "undefined-module" => "Reference to undefined module or namespace".to_string(),
        // Pattern rules
        "unwrap-call" => "Explicit unwrap() call".to_string(),
        "expect-call" => "Explicit expect() call".to_string(),
        "dbg-macro" => "Debug macro dbg!() call".to_string(),
        "eval-call" => "Use of eval() is discouraged".to_string(),
        "exec-call" => "Use of exec() is discouraged".to_string(),
        "debugger-statement" => "Debugger statement found".to_string(),
        "breakpoint-call" => "Breakpoint call found".to_string(),
        "console-log" => "console.log() call found".to_string(),
        "print-call" => "print() call found".to_string(),
        "fmt-println" => "fmt.Println() call found".to_string(),
        // Legacy rules (kept for compatibility)
        "unused-variable" => "Potentially unused variable".to_string(),
        "shadowed-variable" => "Variable shadows outer binding".to_string(),
        "unreachable-code" => "Unreachable code detected".to_string(),
        "deprecated-syntax" => "Deprecated syntax".to_string(),
        _ => format!("Lint rule: {rule_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_query_empty_detects_empty_string() {
        assert!(is_query_empty(""));
    }

    #[test]
    fn is_query_empty_detects_whitespace_only() {
        assert!(is_query_empty("   "));
        assert!(is_query_empty("\n\n"));
        assert!(is_query_empty("  \n  \n  "));
    }

    #[test]
    fn is_query_empty_detects_comment_only() {
        assert!(is_query_empty("; comment"));
        assert!(is_query_empty("; comment\n; another comment"));
        assert!(is_query_empty(";; double semicolon comment"));
    }

    #[test]
    fn is_query_empty_detects_mixed_comments_and_whitespace() {
        let source = r#"; Bash lint rules
; Capture names follow @diagnostic.{rule-id} convention

; No pattern-based rules for Bash
; Semantic checks (unused-symbol, undefined-symbol) are handled in code
"#;
        assert!(is_query_empty(source));
    }

    #[test]
    fn is_query_empty_rejects_actual_query() {
        let source = r#"; Rust lint rules
(call_expression
  function: (field_expression
    field: (field_identifier) @_method)
  (#eq? @_method "unwrap")) @diagnostic.unwrap-call
"#;
        assert!(!is_query_empty(source));
    }

    #[test]
    fn is_query_empty_rejects_query_with_leading_comments() {
        let source = r#"; Leading comment
; Another comment
(identifier) @diagnostic.test
"#;
        assert!(!is_query_empty(source));
    }

    #[test]
    fn split_query_modelines_supports_inherits_and_extends() {
        let source = r#"; inherits: ecma, javascript
; extends
(class_declaration) @local.definition.class
"#;

        let (inherits, extends, body) = split_query_modelines(source);
        assert_eq!(inherits, vec!["ecma".to_string(), "javascript".to_string()]);
        assert!(extends);
        assert!(body.contains("@local.definition.class"));
    }

    fn default_grammar_ref(language: ProgrammingLanguage) -> (Language, &'static str) {
        (language.tree_sitter_language(), language.query_name())
    }

    #[test]
    fn compiles_typescript_locals_query_with_overlay() {
        let (grammar, id) = default_grammar_ref(ProgrammingLanguage::TypeScript);
        let query = query_for(
            GrammarRef {
                language: ProgrammingLanguage::TypeScript,
                grammar: &grammar,
                id,
            },
            QueryKind::Locals,
        )
        .expect("typescript locals query should compile");
        assert!(query.pattern_count() > 0);
    }

    #[test]
    fn compiles_javascript_locals_query_with_overlay() {
        let (grammar, id) = default_grammar_ref(ProgrammingLanguage::JavaScript);
        let query = query_for(
            GrammarRef {
                language: ProgrammingLanguage::JavaScript,
                grammar: &grammar,
                id,
            },
            QueryKind::Locals,
        )
        .expect("javascript locals query should compile");
        assert!(query.pattern_count() > 0);
    }

    #[test]
    fn compiles_typescript_references_query() {
        let (grammar, id) = default_grammar_ref(ProgrammingLanguage::TypeScript);
        let query = query_for(
            GrammarRef {
                language: ProgrammingLanguage::TypeScript,
                grammar: &grammar,
                id,
            },
            QueryKind::References,
        )
        .expect("typescript references query should compile");
        assert!(query.pattern_count() > 0);
    }
}
