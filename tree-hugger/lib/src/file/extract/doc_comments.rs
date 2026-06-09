//! Doc-comment extraction and cleaning helpers carved out of `tree_file.rs`.

use super::*;

/// Extracts doc comment from preceding sibling nodes.
///
/// Doc comments can appear:
/// 1. As direct preceding siblings (Rust `///`, Go `//`)
/// 2. As siblings of a parent wrapper (TypeScript exported functions)
pub(crate) fn extract_doc_comment(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    // First try: look at direct preceding siblings
    if let Some(comments) = collect_doc_comments(node, language, source) {
        return Some(comments);
    }

    // Second try: look at parent's preceding siblings (for exported functions, etc.)
    // This handles cases like:
    //   comment  <-- doc comment is here
    //   export_statement
    //     function_declaration  <-- but we're looking from here
    if let Some(parent) = node.parent() {
        let parent_kind = parent.kind();
        // Only traverse up for known wrapper patterns
        if matches!(
            parent_kind,
            "export_statement"
                | "export_declaration"
                | "decorated_definition"  // Python decorators
                | "public_declaration"
        ) {
            return collect_doc_comments(parent, language, source);
        }
    }

    None
}

/// Collects doc comments from preceding siblings of the given node.
pub(crate) fn collect_doc_comments(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling();

    while let Some(sibling) = prev {
        if is_rust_doc_comment_attribute(sibling.kind(), language) {
            // Rust attributes (e.g., #[derive(...)]) can appear between doc comments
            // and declarations, so skip over them while scanning backwards.
            prev = sibling.prev_sibling();
        } else if is_doc_comment_node(sibling.kind(), language) {
            if language == ProgrammingLanguage::Rust
                && let Ok(text) = sibling.utf8_text(source.as_bytes())
                && !(text.trim_start().starts_with("///") || text.trim_start().starts_with("//!"))
            {
                // Ignore non-doc Rust line comments.
                prev = sibling.prev_sibling();
                continue;
            }
            if let Ok(text) = sibling.utf8_text(source.as_bytes()) {
                comments.push(text.to_string());
            }
            prev = sibling.prev_sibling();
        } else if sibling.kind() == "comment" || sibling.kind() == "line_comment" {
            // Only include adjacent comments
            prev = sibling.prev_sibling();
        } else {
            break;
        }
    }

    if comments.is_empty() {
        return None;
    }

    // Reverse to get comments in order
    comments.reverse();

    // Clean and join the comments
    let cleaned: Vec<String> = comments
        .iter()
        .map(|c| clean_doc_comment(c, language))
        .collect();

    Some(cleaned.join("\n"))
}

pub(crate) fn is_rust_doc_comment_attribute(kind: &str, language: ProgrammingLanguage) -> bool {
    language == ProgrammingLanguage::Rust
        && (kind == "attribute_item" || kind == "inner_attribute_item")
}

/// Checks if a node kind is a doc comment for the given language.
pub(crate) fn is_doc_comment_node(kind: &str, language: ProgrammingLanguage) -> bool {
    match language {
        ProgrammingLanguage::Rust => kind == "line_comment",
        ProgrammingLanguage::Python => kind == "expression_statement", // docstrings
        ProgrammingLanguage::Go => kind == "comment",
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::TypeScript => {
            kind == "comment" || kind == "jsx_text"
        }
        ProgrammingLanguage::Java
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Scala => kind == "comment" || kind == "block_comment",
        _ => kind == "comment",
    }
}

/// Cleans doc comment prefixes based on language conventions.
pub(crate) fn clean_doc_comment(comment: &str, language: ProgrammingLanguage) -> String {
    let trimmed = comment.trim();

    match language {
        ProgrammingLanguage::Rust => {
            // Handle /// and //! doc comments
            trimmed
                .strip_prefix("///")
                .or_else(|| trimmed.strip_prefix("//!"))
                .map(|s| s.trim())
                .unwrap_or(trimmed)
                .to_string()
        }
        ProgrammingLanguage::Python => {
            // Handle docstrings (""" ... """ or ''' ... ''')
            let s = trimmed
                .strip_prefix("\"\"\"")
                .and_then(|s| s.strip_suffix("\"\"\""))
                .or_else(|| {
                    trimmed
                        .strip_prefix("'''")
                        .and_then(|s| s.strip_suffix("'''"))
                })
                .unwrap_or(trimmed);
            s.trim().to_string()
        }
        ProgrammingLanguage::Go => {
            // Handle // comments
            trimmed
                .strip_prefix("//")
                .map(|s| s.trim())
                .unwrap_or(trimmed)
                .to_string()
        }
        ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Scala => {
            // Handle JSDoc-style /** ... */ and // comments
            if trimmed.starts_with("/**") && trimmed.ends_with("*/") {
                clean_jsdoc_comment(trimmed)
            } else if trimmed.starts_with("/*") && trimmed.ends_with("*/") {
                trimmed[2..trimmed.len() - 2].trim().to_string()
            } else {
                trimmed
                    .strip_prefix("//")
                    .map(|s| s.trim())
                    .unwrap_or(trimmed)
                    .to_string()
            }
        }
        _ => trimmed.to_string(),
    }
}

/// Cleans JSDoc-style block comments.
pub(crate) fn clean_jsdoc_comment(comment: &str) -> String {
    let content = &comment[3..comment.len() - 2]; // Remove /** and */
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("*")
                .map(|s| s.trim())
                .unwrap_or(trimmed)
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
