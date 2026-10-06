//! A declaration's attributes, annotations and decorators, parsed once per
//! declaration: each one's full syntactic name, its top-level arguments (read
//! by the deprecation and test flags) and its raw argument text (the record's
//! [`Attribute`] value). Name and arguments are read from the syntax tree, so
//! comments anywhere in an attribute are skipped and a comma inside a string
//! or a nested list never splits an argument. Arguments are never evaluated,
//! imports never resolved and macros never expanded.

use super::*;
use crate::shared::Attribute;

/// One attribute as written: `#[tokio::test]` is `tokio::test` with no
/// arguments, `@Deprecated(since = "9")` is `Deprecated` with the one
/// argument `since="9"` and the value `since = "9"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntacticAttribute {
    /// The path as written, whitespace and comments removed.
    pub(crate) name: String,
    /// The top-level arguments between the delimiters (Rust's
    /// `#[name = value]` has the value as its one argument), each as
    /// [`token_text`]; `None` without an argument list. `cfg(any(test, x))`
    /// has the one argument `any(test,x)`.
    pub(crate) arguments: Option<Vec<String>>,
    /// The source text between the outer delimiters (Rust's `= value`: the
    /// value), trimmed and otherwise as written, comments included; `None`
    /// exactly when `arguments` is, `Some("")` for explicit empty `()`.
    pub(crate) value: Option<String>,
}

impl From<&SyntacticAttribute> for Attribute {
    fn from(attribute: &SyntacticAttribute) -> Self {
        Self {
            name: attribute.name.clone(),
            value: attribute.value.clone(),
        }
    }
}

/// The attributes written on `declaration` for the entity `name` (the
/// captured name node), in source order.
///
/// Read from Rust outer attributes, C++ `[[…]]` attribute declarations (the
/// declaration's own, then those after `name` in its declarator, see
/// [`declarator_id_attributes`]: `int a [[deprecated]], b;` gives `a` one
/// and `b` none), Java and Scala annotations, C# attribute lists, Python
/// decorators and Swift attributes; every other language has none.
pub(crate) fn declaration_attributes(
    declaration: Node<'_>,
    name: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<SyntacticAttribute> {
    let mut attributes = Vec::new();
    match language {
        ProgrammingLanguage::Rust => rust_outer_attributes(declaration, source, &mut attributes),
        ProgrammingLanguage::Cpp => {
            let lists = children_of_kind(declaration, "attribute_declaration")
                .into_iter()
                .chain(declarator_id_attributes(declaration, name));
            for list in lists {
                for attribute in children_of_kind(list, "attribute") {
                    let name = match attribute.child_by_field_name("prefix") {
                        Some(prefix) => format!(
                            "{}::{}",
                            token_text(prefix, source),
                            field_text(attribute, "name", source)
                        ),
                        None => field_text(attribute, "name", source),
                    };
                    let list = find_child_by_kind(attribute, "argument_list");
                    attributes.push(SyntacticAttribute {
                        name,
                        arguments: list.map(|arguments| delimited_arguments(arguments, source)),
                        value: list.map(|arguments| delimited_text(arguments, source)),
                    });
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
                    let arguments = find_child_by_kind(attribute, "attribute_argument_list");
                    attributes.push(SyntacticAttribute {
                        name: field_text(attribute, "name", source),
                        arguments: arguments.map(|arguments| delimited_arguments(arguments, source)),
                        value: arguments.map(|arguments| delimited_text(arguments, source)),
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
    if let Some(arguments) = attribute.child_by_field_name("arguments") {
        return SyntacticAttribute {
            name,
            arguments: Some(delimited_arguments(arguments, source)),
            value: Some(delimited_text(arguments, source)),
        };
    }
    let value = attribute.child_by_field_name("value");
    SyntacticAttribute {
        name,
        arguments: value.map(|value| vec![token_text(value, source)]),
        value: value.map(|value| node_source(value, source).trim().to_string()),
    }
}

/// A Java or Scala annotation: `@name` with an optional argument list.
fn annotation_attribute(annotation: Node<'_>, source: &str) -> SyntacticAttribute {
    let arguments = annotation.child_by_field_name("arguments");
    SyntacticAttribute {
        name: field_text(annotation, "name", source),
        arguments: arguments.map(|arguments| delimited_arguments(arguments, source)),
        value: arguments.map(|arguments| delimited_text(arguments, source)),
    }
}

/// A Python decorator expression: a name, a dotted name or a call of one,
/// read through any parentheses around it (`@(deprecated)`).
fn python_decorator(expression: Node<'_>, source: &str) -> SyntacticAttribute {
    match expression.kind() {
        "parenthesized_expression" => match first_named_non_comment(expression) {
            Some(inner) => python_decorator(inner, source),
            None => SyntacticAttribute {
                name: String::new(),
                arguments: None,
                value: None,
            },
        },
        "call" => {
            let arguments = expression.child_by_field_name("arguments");
            SyntacticAttribute {
                name: expression
                    .child_by_field_name("function")
                    .map(|function| token_text(function, source))
                    .unwrap_or_default(),
                arguments: arguments.map(|arguments| delimited_arguments(arguments, source)),
                value: arguments.map(|arguments| delimited_text(arguments, source)),
            }
        }
        _ => SyntacticAttribute {
            name: token_text(expression, source),
            arguments: None,
            value: None,
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
    let (arguments, value) = match (open, close) {
        (Some(open), Some(close)) if open < close => (
            Some(split_arguments(&children[open + 1..close], source)),
            Some(text_between(children[open], children[close], source)),
        ),
        _ => (None, None),
    };
    SyntacticAttribute { name, arguments, value }
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

/// The raw text of a delimited node between its first and last child.
fn delimited_text(node: Node<'_>, source: &str) -> String {
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();
    match children.as_slice() {
        [open, .., close] => text_between(*open, *close, source),
        _ => String::new(),
    }
}

/// The source text strictly between two nodes, trimmed.
fn text_between(open: Node<'_>, close: Node<'_>, source: &str) -> String {
    source
        .get(open.end_byte()..close.start_byte())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn node_source<'source>(node: Node<'_>, source: &'source str) -> &'source str {
    source.get(node.byte_range()).unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// One declaration's attributes as `(name, arguments, value)`.
    type Expected = Vec<(&'static str, Option<Vec<&'static str>>, Option<&'static str>)>;

    fn parse(language: ProgrammingLanguage, file_name: &str, source: &str) -> tree_sitter::Tree {
        let (grammar, _) = language.grammar_for_path(Path::new(file_name));
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&grammar).expect("grammar");
        parser.parse(source, None).expect("parse")
    }

    fn first<'tree>(node: Node<'tree>, matches: &dyn Fn(Node<'tree>) -> bool) -> Option<Node<'tree>> {
        if matches(node) {
            return Some(node);
        }
        let mut cursor = node.walk();
        let children: Vec<Node<'tree>> = node.children(&mut cursor).collect();
        children.into_iter().find_map(|child| first(child, matches))
    }

    /// The raw value is the text between the outer delimiters, trimmed and
    /// otherwise as written; it is `None` with no argument list, `Some("")`
    /// for explicit empty delimiters, and the conversion to the record's
    /// [`Attribute`] keeps the name and that value. The arguments the flag
    /// readers use are unchanged.
    #[test]
    fn attributes_keep_their_raw_argument_text() {
        let cases: Vec<(ProgrammingLanguage, &str, &str, &str, &str, Expected)> = vec![
            (
                ProgrammingLanguage::Rust,
                "a.rs",
                "#[tokio::test]\nasync fn t() {}\n",
                "function_item",
                "t",
                vec![("tokio::test", None, None)],
            ),
            (
                ProgrammingLanguage::Rust,
                "a.rs",
                "#[doc = \"x  y\"]\n#[cfg_attr(test, derive(Debug))]\n#[allow()]\n#[cfg(/*c*/ test)]\nstruct S;\n",
                "struct_item",
                "S",
                vec![
                    ("doc", Some(vec!["\"x  y\""]), Some("\"x  y\"")),
                    ("cfg_attr", Some(vec!["test", "derive(Debug)"]), Some("test, derive(Debug)")),
                    ("allow", Some(vec![]), Some("")),
                    ("cfg", Some(vec!["test"]), Some("/*c*/ test")),
                ],
            ),
            (
                ProgrammingLanguage::Java,
                "A.java",
                "class A { @Deprecated(since = \"9\") @Test @B() void m() {} }\n",
                "method_declaration",
                "m",
                vec![
                    ("Deprecated", Some(vec!["since=\"9\""]), Some("since = \"9\"")),
                    ("Test", None, None),
                    ("B", Some(vec![]), Some("")),
                ],
            ),
            (
                ProgrammingLanguage::CSharp,
                "a.cs",
                "class A { [Obsolete( \"a  b\" )] [Fact] [B()] void M() {} }\n",
                "method_declaration",
                "M",
                vec![
                    ("Obsolete", Some(vec!["\"a  b\""]), Some("\"a  b\"")),
                    ("Fact", None, None),
                    ("B", Some(vec![]), Some("")),
                ],
            ),
            (
                ProgrammingLanguage::Python,
                "a.py",
                "@app.route( \"/x\" ,  methods=[\"GET\"] )\n@staticmethod\ndef f():\n    pass\n",
                "function_definition",
                "f",
                vec![
                    ("app.route", Some(vec!["\"/x\"", "methods=[\"GET\"]"]), Some("\"/x\" ,  methods=[\"GET\"]")),
                    ("staticmethod", None, None),
                ],
            ),
            (
                ProgrammingLanguage::Cpp,
                "a.cpp",
                "[[deprecated(\"why  not\")]] void f();\n",
                "declaration",
                "f",
                vec![("deprecated", Some(vec!["\"why  not\""]), Some("\"why  not\""))],
            ),
            (
                ProgrammingLanguage::Scala,
                "a.scala",
                "object O {\n  @deprecated(\"m\", \"1.0\") def f = 1\n}\n",
                "function_definition",
                "f",
                vec![("deprecated", Some(vec!["\"m\"", "\"1.0\""]), Some("\"m\", \"1.0\""))],
            ),
            (
                ProgrammingLanguage::Swift,
                "a.swift",
                "@available(*, deprecated)\n@objc\nfunc f() {}\n",
                "function_declaration",
                "f",
                vec![
                    ("available", Some(vec!["*", "deprecated"]), Some("*, deprecated")),
                    ("objc", None, None),
                ],
            ),
        ];
        for (language, file_name, source, declaration_kind, name, expected) in cases {
            let label = format!("{language:?} `{source}`");
            let tree = parse(language, file_name, source);
            assert!(!tree.root_node().has_error(), "{label}: parses cleanly");
            let declaration =
                first(tree.root_node(), &|node| node.kind() == declaration_kind).expect("declaration");
            let name_node = first(declaration, &|node| {
                node.child_count() == 0 && node.is_named() && &source[node.byte_range()] == name
            })
            .expect("name");
            let attributes = declaration_attributes(declaration, name_node, language, source);
            let owned = |name: &str, arguments: Option<Vec<&str>>, value: Option<&str>| SyntacticAttribute {
                name: name.to_string(),
                arguments: arguments.map(|arguments| arguments.into_iter().map(str::to_string).collect()),
                value: value.map(str::to_string),
            };
            let expected: Vec<SyntacticAttribute> = expected
                .into_iter()
                .map(|(name, arguments, value)| owned(name, arguments, value))
                .collect();
            assert_eq!(attributes, expected, "{label}");
            let records: Vec<(String, Option<String>)> = attributes
                .iter()
                .map(Attribute::from)
                .map(|attribute| (attribute.name, attribute.value))
                .collect();
            let expected_records: Vec<(String, Option<String>)> = expected
                .iter()
                .map(|attribute| (attribute.name.clone(), attribute.value.clone()))
                .collect();
            assert_eq!(records, expected_records, "{label}: record attributes");
        }
    }
}
