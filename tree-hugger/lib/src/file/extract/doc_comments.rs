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
    let joined = if language == ProgrammingLanguage::Go {
        go_comment_group_text(&comments)
    } else {
        comments
            .iter()
            .map(|comment| clean_doc_comment(comment))
            .collect::<Vec<_>>()
            .join("\n")
    };
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
/// first statement of its body when that statement is a nonempty string
/// constant, as Python itself decides.
///
/// A docstring is one ordinary string literal (no prefix, `r`, `u`; any
/// quoting) or an adjacent concatenation of them, optionally parenthesized.
/// A bytes (`b`) or formatted (`f`, `t`) literal is an expression, not a
/// docstring, and so is a concatenation containing one. The text is the
/// literal parts' source content joined and trimmed, never evaluated: `""`
/// and whitespace are undocumented, an escape such as `"\u0041"` is not.
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
    let mut expression = first.named_child(0)?;
    while expression.kind() == "parenthesized_expression" {
        expression = single_non_comment_child(expression)?;
    }
    let parts: Vec<Node<'_>> = match expression.kind() {
        "string" => vec![expression],
        "concatenated_string" => {
            let mut cursor = expression.walk();
            expression
                .named_children(&mut cursor)
                .filter(|child| child.kind() != "comment")
                .collect()
        }
        _ => return None,
    };
    let mut content = String::new();
    for part in parts {
        if part.kind() != "string" || !is_unicode_python_literal(part, source) {
            return None;
        }
        let mut cursor = part.walk();
        for child in part.named_children(&mut cursor) {
            if child.kind() == "string_content" {
                content.push_str(child.utf8_text(source.as_bytes()).ok()?);
            }
        }
    }
    let content = content.trim();
    (!content.is_empty()).then(|| content.to_string())
}

/// The only named child of `node` other than comments.
fn single_non_comment_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut children = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment");
    let only = children.next()?;
    children.next().is_none().then_some(only)
}

/// Whether a Python `string` node is an ordinary (`str`) literal: its
/// prefix, read from the `string_start` token before the quote, has neither
/// a bytes (`b`) nor a formatted (`f`, `t`) letter.
fn is_unicode_python_literal(string: Node<'_>, source: &str) -> bool {
    let Some(start) = string
        .child(0)
        .filter(|child| child.kind() == "string_start")
        .and_then(|child| child.utf8_text(source.as_bytes()).ok())
    else {
        return false;
    };
    start
        .chars()
        .take_while(|character| !matches!(character, '"' | '\''))
        .all(|character| matches!(character.to_ascii_lowercase(), 'r' | 'u'))
}

/// Removes a doc comment's delimiters and its Javadoc/Doxygen decoration
/// (a third opening `*` or `!`, a leading `*` on each line). Go comments
/// never come here: see [`go_comment_group_text`].
fn clean_doc_comment(comment: &str) -> String {
    let trimmed = comment.trim();
    if trimmed.starts_with("/*") && trimmed.ends_with("*/") && trimmed.len() >= 4 {
        let opener = if trimmed.starts_with("/**") || trimmed.starts_with("/*!") { 3 } else { 2 };
        return clean_block_comment(&trimmed[opener.min(trimmed.len() - 2)..trimmed.len() - 2]);
    }
    let line = ["///", "//!", "//"]
        .iter()
        .find_map(|prefix| trimmed.strip_prefix(prefix))
        .unwrap_or(trimmed);
    line.trim().to_string()
}

/// The nonblank lines of a block comment's body, trimmed and without their
/// leading `*`.
fn clean_block_comment(body: &str) -> String {
    body.lines()
        .map(|line| {
            let trimmed = line.trim();
            trimmed.strip_prefix('*').map_or(trimmed, str::trim)
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The text of a Go comment group as `go/ast`'s `CommentGroup.Text` and
/// `go/doc/comment` read it. Go defines no decoration: a `//` comment loses
/// its marker and one following space, a block loses exactly `/*` and `*/`
/// (plus the whitespace that follows `/*` on its line), and every other
/// character, a third opening `*` or a leading `*` on a line included, is
/// text. Trailing whitespace and the group's leading and trailing blank
/// lines are dropped, then the whitespace prefix common to every nonblank
/// line is removed; any indentation left marks a code block, which the
/// `Deprecated:` paragraph rule depends on.
fn go_comment_group_text(comments: &[&str]) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for comment in comments {
        let comment = comment.trim();
        if let Some(body) = comment.strip_prefix("/*").and_then(|rest| rest.strip_suffix("*/")) {
            // The space after `/*` separates the text from the delimiter, so
            // the first line's indentation is not compared with the others'
            // (`/* Intro\nDeprecated: …` is one paragraph, not a code line
            // and a marker). Only whitespace goes: a `*` there is text.
            let mut body_lines = body.lines().map(str::trim_end);
            if let Some(first) = body_lines.next() {
                lines.push(first.trim_start_matches([' ', '\t']));
            }
            lines.extend(body_lines);
        } else {
            let line = comment.strip_prefix("//").unwrap_or(comment);
            lines.push(line.strip_prefix(' ').unwrap_or(line).trim_end());
        }
    }
    let Some(first) = lines.iter().position(|line| !line.is_empty()) else {
        return String::new();
    };
    let last = lines.iter().rposition(|line| !line.is_empty()).unwrap_or(first);
    let lines = &lines[first..=last];
    let indentation = |line: &str| line.len() - line.trim_start_matches([' ', '\t']).len();
    let common = lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| &line[..indentation(line)])
        .reduce(|common, prefix| {
            let shared = common
                .bytes()
                .zip(prefix.bytes())
                .take_while(|(left, right)| left == right)
                .count();
            &common[..shared]
        })
        .unwrap_or("");
    lines
        .iter()
        .map(|line| line.get(common.len()..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}
