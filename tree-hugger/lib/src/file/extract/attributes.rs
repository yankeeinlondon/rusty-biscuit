//! A declaration's attributes, annotations and decorators, read only as far
//! as the deprecation and test flags need: each one's full syntactic name and
//! raw argument text. Arguments are never evaluated, imports never resolved
//! and macros never expanded, and nothing here fills the record's
//! `attributes` facet.

use super::*;

/// One attribute as written: `#[tokio::test]` is `tokio::test` with no
/// arguments, `@Deprecated(since = "9")` is `Deprecated` with
/// `since = "9"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntacticAttribute {
    /// The path as written, whitespace removed.
    pub(crate) name: String,
    /// The text between the argument delimiters (or after `=` in Rust's
    /// `#[name = value]`), trimmed; `None` without arguments.
    pub(crate) arguments: Option<String>,
}

/// The attributes written on `declaration`, in source order.
///
/// Read from Rust outer attributes, C++ `[[…]]` attribute declarations, Java
/// and Scala annotations, C# attribute lists, Python decorators and Swift
/// attributes; every other language has none.
pub(crate) fn declaration_attributes(
    declaration: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<SyntacticAttribute> {
    let mut attributes = Vec::new();
    match language {
        ProgrammingLanguage::Rust => rust_outer_attributes(declaration, source, &mut attributes),
        ProgrammingLanguage::Cpp => {
            for list in children_of_kind(declaration, "attribute_declaration") {
                for attribute in children_of_kind(list, "attribute") {
                    let name = match attribute.child_by_field_name("prefix") {
                        Some(prefix) => format!(
                            "{}::{}",
                            compact_text(prefix, source),
                            field_text(attribute, "name", source)
                        ),
                        None => field_text(attribute, "name", source),
                    };
                    let arguments = find_child_by_kind(attribute, "argument_list")
                        .map(|arguments| delimited_text(arguments, source));
                    attributes.push(SyntacticAttribute { name, arguments });
                }
            }
        }
        ProgrammingLanguage::Java => {
            let host = if declaration.kind() == "variable_declarator" {
                declaration.parent().unwrap_or(declaration)
            } else {
                declaration
            };
            for modifiers in children_of_kind(host, "modifiers") {
                let mut cursor = modifiers.walk();
                for annotation in modifiers.named_children(&mut cursor) {
                    if matches!(annotation.kind(), "marker_annotation" | "annotation") {
                        attributes.push(annotation_attribute(annotation, source));
                    }
                }
            }
        }
        ProgrammingLanguage::Scala => {
            for annotation in children_of_kind(declaration, "annotation") {
                attributes.push(annotation_attribute(annotation, source));
            }
        }
        ProgrammingLanguage::CSharp => {
            let mut host = declaration;
            while matches!(host.kind(), "variable_declarator" | "variable_declaration")
                && let Some(parent) = host.parent()
            {
                host = parent;
            }
            for list in children_of_kind(host, "attribute_list") {
                for attribute in children_of_kind(list, "attribute") {
                    attributes.push(SyntacticAttribute {
                        name: field_text(attribute, "name", source),
                        arguments: find_child_by_kind(attribute, "attribute_argument_list")
                            .map(|arguments| delimited_text(arguments, source)),
                    });
                }
            }
        }
        ProgrammingLanguage::Python => {
            if let Some(decorated) = declaration
                .parent()
                .filter(|parent| parent.kind() == "decorated_definition")
            {
                for decorator in children_of_kind(decorated, "decorator") {
                    if let Some(expression) = decorator.named_child(0) {
                        attributes.push(python_decorator(expression, source));
                    }
                }
            }
        }
        ProgrammingLanguage::Swift => {
            for modifiers in children_of_kind(declaration, "modifiers") {
                for attribute in children_of_kind(modifiers, "attribute") {
                    attributes.push(swift_attribute(attribute, source));
                }
            }
        }
        ProgrammingLanguage::TypeScript
        | ProgrammingLanguage::JavaScript
        | ProgrammingLanguage::Go
        | ProgrammingLanguage::C
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::Perl
        | ProgrammingLanguage::Lua
        | ProgrammingLanguage::Bash
        | ProgrammingLanguage::Zsh => {}
    }
    attributes
}

/// The `#[…]` attributes directly before a Rust item, field or variant;
/// comments between them are skipped.
pub(crate) fn rust_outer_attributes(
    item: Node<'_>,
    source: &str,
    attributes: &mut Vec<SyntacticAttribute>,
) {
    let start = attributes.len();
    let mut prev = item.prev_sibling();
    while let Some(sibling) = prev {
        match sibling.kind() {
            "attribute_item" => {
                if let Some(attribute) = find_child_by_kind(sibling, "attribute") {
                    attributes.push(rust_attribute(attribute, source));
                }
            }
            "line_comment" | "block_comment" => {}
            _ => break,
        }
        prev = sibling.prev_sibling();
    }
    attributes[start..].reverse();
}

/// A Rust `attribute` node: its path, then a token tree or `= value`.
pub(crate) fn rust_attribute(attribute: Node<'_>, source: &str) -> SyntacticAttribute {
    let name = attribute
        .named_child(0)
        .map(|path| compact_text(path, source))
        .unwrap_or_default();
    let arguments = attribute
        .child_by_field_name("arguments")
        .map(|arguments| delimited_text(arguments, source))
        .or_else(|| {
            attribute
                .child_by_field_name("value")
                .and_then(|value| value.utf8_text(source.as_bytes()).ok())
                .map(|value| value.trim().to_string())
        });
    SyntacticAttribute { name, arguments }
}

/// A Java or Scala annotation: `@name` with an optional argument list.
fn annotation_attribute(annotation: Node<'_>, source: &str) -> SyntacticAttribute {
    SyntacticAttribute {
        name: field_text(annotation, "name", source),
        arguments: annotation
            .child_by_field_name("arguments")
            .map(|arguments| delimited_text(arguments, source)),
    }
}

/// A Python decorator expression: a name, a dotted name or a call of one.
fn python_decorator(expression: Node<'_>, source: &str) -> SyntacticAttribute {
    match expression.kind() {
        "call" => SyntacticAttribute {
            name: expression
                .child_by_field_name("function")
                .map(|function| compact_text(function, source))
                .unwrap_or_default(),
            arguments: expression
                .child_by_field_name("arguments")
                .map(|arguments| delimited_text(arguments, source)),
        },
        _ => SyntacticAttribute {
            name: compact_text(expression, source),
            arguments: None,
        },
    }
}

/// A Swift `@name(…)` attribute, whose arguments are the attribute's own
/// children between its parentheses.
fn swift_attribute(attribute: Node<'_>, source: &str) -> SyntacticAttribute {
    let name = find_child_by_kind(attribute, "user_type")
        .map(|name| compact_text(name, source))
        .unwrap_or_default();
    let mut cursor = attribute.walk();
    let children: Vec<Node<'_>> = attribute.children(&mut cursor).collect();
    let open = children.iter().position(|child| child.kind() == "(");
    let close = children.iter().rposition(|child| child.kind() == ")");
    let arguments = match (open, close) {
        (Some(open), Some(close)) if open < close => source
            .get(children[open].end_byte()..children[close].start_byte())
            .map(|text| text.trim().to_string()),
        _ => None,
    };
    SyntacticAttribute { name, arguments }
}

fn children_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .collect()
}

fn field_text(node: Node<'_>, field: &str, source: &str) -> String {
    node.child_by_field_name(field)
        .map(|child| compact_text(child, source))
        .unwrap_or_default()
}

/// A node's text with all whitespace removed (`org . junit . Test` reads as
/// `org.junit.Test`).
fn compact_text(node: Node<'_>, source: &str) -> String {
    node.utf8_text(source.as_bytes())
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

/// The text inside a delimited node (`(…)`, `[…]`, `{…}`), trimmed.
fn delimited_text(node: Node<'_>, source: &str) -> String {
    let text = node.utf8_text(source.as_bytes()).unwrap_or_default().trim();
    let mut characters = text.chars();
    match (characters.next(), characters.next_back()) {
        (Some('(' | '[' | '{'), Some(')' | ']' | '}')) => text[1..text.len() - 1].trim().to_string(),
        _ => text.to_string(),
    }
}
