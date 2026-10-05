//! A declaration's attributes, annotations and decorators, read only as far
//! as the deprecation and test flags need: each one's full syntactic name and
//! its top-level arguments. Both are read from the syntax tree, so comments
//! anywhere in an attribute are skipped and a comma inside a string or a
//! nested list never splits an argument. Arguments are never evaluated,
//! imports never resolved and macros never expanded, and nothing here fills
//! the record's `attributes` facet.

use super::*;

/// One attribute as written: `#[tokio::test]` is `tokio::test` with no
/// arguments, `@Deprecated(since = "9")` is `Deprecated` with the one
/// argument `since="9"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntacticAttribute {
    /// The path as written, whitespace and comments removed.
    pub(crate) name: String,
    /// The top-level arguments between the delimiters (Rust's
    /// `#[name = value]` has the value as its one argument), each as
    /// [`token_text`]; `None` without an argument list. `cfg(any(test, x))`
    /// has the one argument `any(test,x)`.
    pub(crate) arguments: Option<Vec<String>>,
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
                            token_text(prefix, source),
                            field_text(attribute, "name", source)
                        ),
                        None => field_text(attribute, "name", source),
                    };
                    let arguments = find_child_by_kind(attribute, "argument_list")
                        .map(|arguments| delimited_arguments(arguments, source));
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
                            .map(|arguments| delimited_arguments(arguments, source)),
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
                    if let Some(expression) = first_named_non_comment(decorator) {
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
    let name = first_named_non_comment(attribute)
        .map(|path| token_text(path, source))
        .unwrap_or_default();
    let arguments = attribute
        .child_by_field_name("arguments")
        .map(|arguments| delimited_arguments(arguments, source))
        .or_else(|| {
            attribute
                .child_by_field_name("value")
                .map(|value| vec![token_text(value, source)])
        });
    SyntacticAttribute { name, arguments }
}

/// A Java or Scala annotation: `@name` with an optional argument list.
fn annotation_attribute(annotation: Node<'_>, source: &str) -> SyntacticAttribute {
    SyntacticAttribute {
        name: field_text(annotation, "name", source),
        arguments: annotation
            .child_by_field_name("arguments")
            .map(|arguments| delimited_arguments(arguments, source)),
    }
}

/// A Python decorator expression: a name, a dotted name or a call of one,
/// read through any parentheses around it (`@(deprecated)`).
fn python_decorator(expression: Node<'_>, source: &str) -> SyntacticAttribute {
    match expression.kind() {
        "parenthesized_expression" => match first_named_non_comment(expression) {
            Some(inner) => python_decorator(inner, source),
            None => SyntacticAttribute { name: String::new(), arguments: None },
        },
        "call" => SyntacticAttribute {
            name: expression
                .child_by_field_name("function")
                .map(|function| token_text(function, source))
                .unwrap_or_default(),
            arguments: expression
                .child_by_field_name("arguments")
                .map(|arguments| delimited_arguments(arguments, source)),
        },
        _ => SyntacticAttribute {
            name: token_text(expression, source),
            arguments: None,
        },
    }
}

/// A Swift `@name(…)` attribute, whose arguments are the attribute's own
/// children between its parentheses.
fn swift_attribute(attribute: Node<'_>, source: &str) -> SyntacticAttribute {
    let name = find_child_by_kind(attribute, "user_type")
        .map(|name| token_text(name, source))
        .unwrap_or_default();
    let mut cursor = attribute.walk();
    let children: Vec<Node<'_>> = attribute.children(&mut cursor).collect();
    let open = children.iter().position(|child| child.kind() == "(");
    let close = children.iter().rposition(|child| child.kind() == ")");
    let arguments = match (open, close) {
        (Some(open), Some(close)) if open < close => {
            Some(split_arguments(&children[open + 1..close], source))
        }
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
        .map(|child| token_text(child, source))
        .unwrap_or_default()
}

fn is_comment(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "line_comment" | "block_comment" | "multiline_comment"
    )
}

fn first_named_non_comment(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| !is_comment(*child))
}

/// A node's tokens joined with no separator, comments and the whitespace
/// between tokens dropped (`org . /* c */ junit . Test` reads as
/// `org.junit.Test`). A token's own text, such as a string literal's
/// content, is kept as written.
fn token_text(node: Node<'_>, source: &str) -> String {
    let mut text = String::new();
    push_tokens(node, source, &mut text);
    text
}

fn push_tokens(node: Node<'_>, source: &str, text: &mut String) {
    if is_comment(node) {
        return;
    }
    if node.child_count() == 0 {
        text.push_str(node.utf8_text(source.as_bytes()).unwrap_or_default().trim());
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        push_tokens(child, source, text);
    }
}

/// The top-level arguments of a delimited node (`(…)`, `[…]`, `{…}`, a Rust
/// token tree): its children between the first and last, split at `,`.
fn delimited_arguments(node: Node<'_>, source: &str) -> Vec<String> {
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
    match children.len() {
        0..=2 => Vec::new(),
        count => split_arguments(&children[1..count - 1], source),
    }
}

/// Sibling nodes split at their own `,` tokens, each argument as
/// [`token_text`]; comments are skipped and an empty argument (a trailing
/// comma) is dropped.
fn split_arguments(children: &[Node<'_>], source: &str) -> Vec<String> {
    let mut arguments = vec![String::new()];
    for child in children {
        if child.kind() == "," {
            arguments.push(String::new());
        } else if let Some(current) = arguments.last_mut() {
            push_tokens(*child, source, current);
        }
    }
    arguments.retain(|argument| !argument.is_empty());
    arguments
}
