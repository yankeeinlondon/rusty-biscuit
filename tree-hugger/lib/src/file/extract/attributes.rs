//! A declaration's attributes, annotations and decorators, parsed once per
//! declaration: each one's full syntactic name, its top-level arguments (read
//! by the deprecation and test flags) and its raw argument text (the record's
//! [`Attribute`] value). Name and arguments are read from the syntax tree, so
//! comments anywhere in an attribute are skipped and a comma inside a string
//! or a nested list never splits an argument. Arguments are never evaluated,
//! imports never resolved and macros never expanded.

use super::*;
use crate::file::parents::IndexedParent as _;
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
/// and `b` none), Java and Scala annotations, C# attribute lists, Python,
/// TypeScript and JavaScript decorators and Swift attributes; every other
/// language has none. File-level forms (Rust inner attributes, C#
/// `[assembly: …]`) are never a declaration's.
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
            for modifiers in children_of_kind(modifier_host(declaration, language), "modifiers") {
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
            for list in children_of_kind(modifier_host(declaration, language), "attribute_list") {
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
                .indexed_parent()
                .filter(|parent| parent.kind() == "decorated_definition")
            {
                for decorator in children_of_kind(decorated, "decorator") {
                    if let Some(expression) = first_named_non_comment(decorator) {
                        attributes.push(decorator_attribute(expression, language, source));
                    }
                }
            }
        }
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
            for decorator in ecma_decorators(declaration) {
                if let Some(expression) = first_named_non_comment(decorator) {
                    attributes.push(decorator_attribute(expression, language, source));
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
        ProgrammingLanguage::Go
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

/// The modifier and attribute host of a Java or C# declaration: a field's
/// or local's whole declaration (which holds the modifier list), not the
/// declarator naming one of its variables; any other declaration itself.
pub(crate) fn modifier_host(declaration: Node<'_>, language: ProgrammingLanguage) -> Node<'_> {
    let mut host = declaration;
    loop {
        let climbs = match language {
            ProgrammingLanguage::Java => host.kind() == "variable_declarator",
            ProgrammingLanguage::CSharp => matches!(host.kind(), "variable_declarator" | "variable_declaration"),
            _ => false,
        };
        match host.indexed_parent() {
            Some(parent) if climbs => host = parent,
            _ => return host,
        }
    }
}

/// The decorators of a TypeScript or JavaScript declaration, in source
/// order: an exported class's on its `export` statement (`@d export class`),
/// a TypeScript method's as the `class_body` children directly before it,
/// then the declaration's own. A parameter's decorators are the
/// parameter's, never its function's.
fn ecma_decorators(declaration: Node<'_>) -> Vec<Node<'_>> {
    let mut decorators = Vec::new();
    if let Some(parent) = declaration.indexed_parent() {
        match parent.kind() {
            "export_statement" => decorators.extend(children_of_kind(parent, "decorator")),
            "class_body" => {
                let mut preceding = Vec::new();
                let mut sibling = declaration.prev_sibling();
                while let Some(node) = sibling {
                    match node.kind() {
                        "decorator" => preceding.push(node),
                        kind if is_comment_kind(kind) => {}
                        _ => break,
                    }
                    sibling = node.prev_sibling();
                }
                decorators.extend(preceding.into_iter().rev());
            }
            _ => {}
        }
    }
    decorators.extend(children_of_kind(declaration, "decorator"));
    decorators
}

/// A Python, TypeScript or JavaScript decorator expression, read through any
/// parentheses around it (`@(deprecated)`). A path (`name`, `a.b`) is the
/// name with no arguments; a call of a path is that path with the call's
/// arguments. Any other expression is kept whole as the name, as written
/// and trimmed, with no arguments: `@(lambda f: f)` is `lambda f: f`.
fn decorator_attribute(expression: Node<'_>, language: ProgrammingLanguage, source: &str) -> SyntacticAttribute {
    if matches!(expression.kind(), "parenthesized_expression")
        && let Some(inner) = first_named_non_comment(expression)
    {
        return decorator_attribute(inner, language, source);
    }
    let call = match language {
        ProgrammingLanguage::Python => "call",
        _ => "call_expression",
    };
    if is_decorator_path(expression, language) {
        return SyntacticAttribute {
            name: token_text(expression, source),
            arguments: None,
            value: None,
        };
    }
    if expression.kind() == call
        && let Some(function) = expression.child_by_field_name("function")
        && is_decorator_path(function, language)
        && let Some(arguments) = expression
            .child_by_field_name("arguments")
            .filter(|arguments| matches!(arguments.kind(), "argument_list" | "arguments"))
    {
        return SyntacticAttribute {
            name: token_text(function, source),
            arguments: Some(delimited_arguments(arguments, source)),
            value: Some(delimited_text(arguments, source)),
        };
    }
    SyntacticAttribute {
        name: node_source(expression, source).trim().to_string(),
        arguments: None,
        value: None,
    }
}

/// An identifier, or a member access of a path by a plain name (`a.b.c`).
fn is_decorator_path(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    let (member, property_field) = match language {
        ProgrammingLanguage::Python => ("attribute", "attribute"),
        _ => ("member_expression", "property"),
    };
    match node.kind() {
        "identifier" => true,
        kind if kind == member => {
            node.child_by_field_name("object")
                .is_some_and(|object| is_decorator_path(object, language))
                && node
                    .child_by_field_name(property_field)
                    .is_some_and(|property| matches!(property.kind(), "identifier" | "property_identifier"))
        }
        _ => false,
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
    is_comment_kind(node.kind())
}

fn is_comment_kind(kind: &str) -> bool {
    matches!(kind, "comment" | "line_comment" | "block_comment" | "multiline_comment")
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
