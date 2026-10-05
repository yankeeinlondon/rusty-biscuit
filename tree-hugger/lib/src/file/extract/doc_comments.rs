//! Attached documentation: the doc comments before a declaration, or a
//! Python docstring. Ordinary comments are never documentation.

use super::*;

/// The cleaned documentation attached to the declaration `node`, or `None`
/// when it has none (or only blank doc comments).
///
/// Which comments document a declaration is per language (see
/// [`is_doc_comment_text`]); ordinary comments between them and the
/// declaration are skipped. Python reads the docstring of a `def` or
/// `class`. Lua, Perl, Bash and Zsh have no doc-comment detection and always
/// return `None`.
pub(crate) fn extract_doc_comment(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    match language {
        ProgrammingLanguage::Python => python_docstring(node, source),
        ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => None,
        _ => {
            // A declaration wrapped in `export`, a JS/TS variable statement or
            // a C++ template header is documented before the wrapper.
            let mut current = node;
            loop {
                if let Some(doc) = collect_doc_comments(current, language, source) {
                    return Some(doc);
                }
                match current.parent() {
                    Some(parent) if is_doc_wrapper_kind(parent.kind()) => current = parent,
                    _ => return None,
                }
            }
        }
    }
}

fn is_doc_wrapper_kind(kind: &str) -> bool {
    matches!(
        kind,
        "export_statement"
            | "export_declaration"
            | "ambient_declaration"
            | "lexical_declaration"
            | "variable_declarator"
            | "variable_declaration"
            | "template_declaration"
    )
}

/// Collects the doc comments in the run of comments (and, in Rust,
/// attributes) directly before `node`.
fn collect_doc_comments(
    node: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    let mut comments = Vec::new();
    let mut prev = node.prev_sibling();
    // Go documents a declaration only with the comment group that ends on
    // the line before it.
    let mut next_start_row = node.start_position().row;

    while let Some(sibling) = prev {
        let kind = sibling.kind();
        if language == ProgrammingLanguage::Rust
            && matches!(kind, "attribute_item" | "inner_attribute_item")
        {
            // Attributes (`#[derive(...)]`) may sit between docs and the item.
        } else if is_comment_node_kind(kind) {
            if language == ProgrammingLanguage::Go
                && next_start_row.saturating_sub(sibling.end_position().row) > 1
            {
                break;
            }
            if let Ok(text) = sibling.utf8_text(source.as_bytes())
                && is_doc_comment_text(text, language)
            {
                comments.push(text);
            }
        } else {
            break;
        }
        next_start_row = sibling.start_position().row;
        prev = sibling.prev_sibling();
    }

    comments.reverse();
    let cleaned: Vec<String> = comments
        .iter()
        .map(|comment| clean_doc_comment(comment, language))
        .collect();
    let joined = cleaned.join("\n");
    (!joined.trim().is_empty()).then_some(joined)
}

fn is_comment_node_kind(kind: &str) -> bool {
    matches!(
        kind,
        "comment" | "line_comment" | "block_comment" | "multiline_comment"
    )
}

/// Whether a comment's text is a doc comment in `language`:
///
/// - Rust: `///` and `/** */` (outer docs; `//!` documents the enclosing
///   module, not the next item)
/// - Go: every comment (Go's doc comments are the plain comments directly
///   before a declaration)
/// - Java, JavaScript, TypeScript, PHP, Scala: `/** */`
/// - C#, Swift: `///` and `/** */`
/// - C, C++: Doxygen's `///`, `//!`, `/** */` and `/*! */`
pub(crate) fn is_doc_comment_text(text: &str, language: ProgrammingLanguage) -> bool {
    let text = text.trim_start();
    let triple_slash = text.starts_with("///") && !text.starts_with("////");
    let javadoc = text.starts_with("/**") && !text.starts_with("/***") && !text.starts_with("/**/");
    match language {
        ProgrammingLanguage::Rust => triple_slash || javadoc,
        ProgrammingLanguage::Go => true,
        ProgrammingLanguage::Java
        | ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::Scala => javadoc,
        ProgrammingLanguage::CSharp | ProgrammingLanguage::Swift => triple_slash || javadoc,
        ProgrammingLanguage::C | ProgrammingLanguage::Cpp => {
            triple_slash || javadoc || text.starts_with("//!") || text.starts_with("/*!")
        }
        ProgrammingLanguage::Python
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => false,
    }
}

/// The docstring of a Python `def` or `class` (decorated or not): the
/// string that is the first statement of its body.
fn python_docstring(node: Node<'_>, source: &str) -> Option<String> {
    let definition = if node.kind() == "decorated_definition" {
        node.child_by_field_name("definition")?
    } else {
        node
    };
    if !matches!(definition.kind(), "function_definition" | "class_definition") {
        return None;
    }
    let first = definition.child_by_field_name("body")?.named_child(0)?;
    if first.kind() != "expression_statement" || first.named_child_count() != 1 {
        return None;
    }
    let string = first.named_child(0).filter(|child| child.kind() == "string")?;
    let mut cursor = string.walk();
    let content: String = string
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "string_content")
        .filter_map(|child| child.utf8_text(source.as_bytes()).ok())
        .collect();
    let content = content.trim();
    (!content.is_empty()).then(|| content.to_string())
}

/// Removes a doc comment's delimiters.
fn clean_doc_comment(comment: &str, language: ProgrammingLanguage) -> String {
    let trimmed = comment.trim();
    if trimmed.starts_with("/*") && trimmed.ends_with("*/") && trimmed.len() >= 4 {
        let opener = if trimmed.starts_with("/**") || trimmed.starts_with("/*!") { 3 } else { 2 };
        return clean_block_comment(&trimmed[opener.min(trimmed.len() - 2)..trimmed.len() - 2]);
    }
    let line = ["///", "//!", "//"]
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);
    // A Go doc paragraph break is an empty `//` line; keep it empty.
    if language == ProgrammingLanguage::Go {
        return line.strip_prefix(' ').unwrap_or(line).trim_end().to_string();
    }
    line.trim().to_string()
}

/// The lines of a block comment's body without their leading `*`, blank
/// lines dropped.
fn clean_block_comment(body: &str) -> String {
    body.lines()
        .map(|line| {
            let line = line.trim();
            line.strip_prefix('*').map_or(line, str::trim)
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
