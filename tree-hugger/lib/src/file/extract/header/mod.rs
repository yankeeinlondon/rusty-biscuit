//! What a renderer needs to label and show a declaration, read once from its
//! node at extraction: the declaring keyword and the display header
//! (`source.signature_text`).
//!
//! The header is a contiguous sub-range of the declaration text
//! (`declaration_text`): it starts after the leading in-node annotations and
//! ends before the first of the body, an initializer, accessors, a
//! terminator, or the `=`, `=>` or `:` that introduces a body. Every boundary
//! is a node or token of the syntax tree, never a text search.

use std::ops::Range;

use super::*;

mod keyword;

pub(crate) use keyword::declaration_keyword;

/// Tokens that introduce a body or initializer, dropped with it.
const INTRODUCER_TOKENS: &[&str] = &["=", "=>", ":", ":="];

/// Tokens that bind an initializer to the declared name.
const INITIALIZER_TOKENS: &[&str] = &["=", ":="];

/// Field names whose node is a body, initializer or accessor list rather than
/// part of the header.
const CUT_FIELDS: &[&str] = &["body", "value", "accessors", "computed_value", "default_value"];

/// Child kinds that end the header wherever they appear.
const CUT_KINDS: &[&str] = &["protocol_property_requirements", "arrow_expression_clause"];

/// Child kinds that hold one declared name and its initializer (`int a = 1`
/// in C, `x = 1` in a Java or JavaScript declarator, PHP `$x = 1`, the alias
/// of a Go `type A = B`), searched for the initializer token.
const DECLARATOR_KINDS: &[&str] = &[
    "init_declarator",
    "type_alias",
    "variable_declarator",
    "variable_declaration",
    "equals_value_clause",
    "const_element",
    "property_element",
];

/// The display header of `declaration`, whose captured name spans `name`.
///
/// `None` when the header cannot be isolated: a C/C++ declaration whose type
/// carries a body (`typedef struct { … } Alias;`), or a name the header would
/// not contain because an earlier name's initializer ends it
/// (`int a = 1, b = 2;` for `b`). A type of a grouped Go `type ( … )` block is
/// declared by its own spec ([`context_for_name`]), so its header is the
/// spec's.
pub(crate) fn signature_text(
    declaration: Node<'_>,
    name: Range<usize>,
    language: ProgrammingLanguage,
    source: &str,
) -> Option<String> {
    if has_inline_type_body(declaration, language) {
        return None;
    }

    let start = header_start(declaration, language);
    let end = header_end(declaration, language).max(start);
    let text = source.get(start..end)?.trim_end();
    if text.is_empty() {
        return None;
    }

    let declares_name =
        declaration.start_byte() <= name.start && name.end <= declaration.end_byte();
    let header_holds_name = start <= name.start && name.end <= start + text.len();
    if declares_name && !header_holds_name {
        return None;
    }
    Some(text.to_string())
}

/// Where the header begins: the first child that is neither a comment nor a
/// leading annotation. Inside a modifier list (Java, Swift) only the
/// annotations before the first modifier are skipped, so one written between
/// modifiers stays in the header.
fn header_start(declaration: Node<'_>, language: ProgrammingLanguage) -> usize {
    let mut cursor = declaration.walk();
    for child in declaration.children(&mut cursor) {
        if is_comment(child) || is_annotation(child, language) {
            continue;
        }
        if child.kind() == "modifiers" {
            let mut inner_cursor = child.walk();
            let first_modifier = child
                .children(&mut inner_cursor)
                .find(|inner| !is_comment(*inner) && !is_annotation(*inner, language));
            match first_modifier {
                Some(modifier) => return modifier.start_byte(),
                None => continue,
            }
        }
        return child.start_byte();
    }
    declaration.start_byte()
}

/// Where the header ends: the earliest cut among the body, a cut field or
/// kind, an initializer token and a trailing terminator.
fn header_end(declaration: Node<'_>, language: ProgrammingLanguage) -> usize {
    let parameter_defaults = has_inline_parameter_defaults(declaration, language);
    let mut end = declaration.end_byte();
    let mut previous: Option<Node<'_>> = None;
    let mut cursor = declaration.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let is_cut = cursor.field_name().is_some_and(|field| {
                CUT_FIELDS.contains(&field) && !(parameter_defaults && field == "default_value")
            }) || CUT_KINDS.contains(&child.kind());
            if is_cut {
                let introducer = previous
                    .filter(|token| !token.is_named() && INTRODUCER_TOKENS.contains(&token.kind()));
                end = introducer.unwrap_or(child).start_byte();
                break;
            }
            previous = Some(child);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    if let Some(body) = body_node_for_declaration(declaration) {
        end = end.min(body.start_byte());
    }
    if !parameter_defaults && let Some(token) = initializer_token(declaration) {
        end = end.min(token.start_byte());
    }
    if let Some(nested) = go_type_body(declaration, language) {
        end = end.min(nested);
    }
    if end == declaration.end_byte()
        && let Some(last) = declaration.child(declaration.child_count().saturating_sub(1) as u32)
        && !last.is_named()
        && is_terminator(last.kind(), language)
    {
        end = last.start_byte();
    }
    end
}

/// tree-sitter-swift places a parameter's default (`y: Int = 2`) and its
/// `=` directly in the function or initializer, beside the parameter, so
/// there they belong to the parameter list rather than introduce a body.
fn has_inline_parameter_defaults(declaration: Node<'_>, language: ProgrammingLanguage) -> bool {
    language == ProgrammingLanguage::Swift
        && matches!(
            declaration.kind(),
            "function_declaration" | "init_declaration" | "protocol_function_declaration"
        )
}

/// The first `=` or `:=` binding an initializer, as a direct child of
/// `node` or inside one of its declarator children, in source order.
fn initializer_token(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    children.into_iter().find_map(|child| {
        if !child.is_named() && INITIALIZER_TOKENS.contains(&child.kind()) {
            Some(child)
        } else if DECLARATOR_KINDS.contains(&child.kind()) {
            initializer_token(child)
        } else {
            None
        }
    })
}

/// Go keeps a type's fields in its `type_spec`: the header ends at the
/// struct field list or the interface's opening brace.
fn go_type_body(declaration: Node<'_>, language: ProgrammingLanguage) -> Option<usize> {
    if language != ProgrammingLanguage::Go {
        return None;
    }
    let spec = if declaration.kind() == "type_spec" {
        declaration
    } else {
        find_child_by_kind(declaration, "type_spec")?
    };
    let type_node = spec.child_by_field_name("type")?;
    let mut cursor = type_node.walk();
    let brace = type_node
        .children(&mut cursor)
        .find(|child| child.kind() == "field_declaration_list" || child.kind() == "{");
    brace.map(|brace| brace.start_byte())
}

/// A C/C++ declaration whose type specifier carries its own body
/// (`typedef struct { … } Alias;`, `struct S { … } instance;`): the header
/// would have to include that body.
fn has_inline_type_body(declaration: Node<'_>, language: ProgrammingLanguage) -> bool {
    matches!(language, ProgrammingLanguage::C | ProgrammingLanguage::Cpp)
        && declaration
            .child_by_field_name("type")
            .is_some_and(|type_node| type_node.child_by_field_name("body").is_some())
}

fn is_terminator(token: &str, language: ProgrammingLanguage) -> bool {
    token == ";" || (language == ProgrammingLanguage::Lua && token == "end")
}

fn is_comment(node: Node<'_>) -> bool {
    node.kind().contains("comment")
}

/// Annotation, attribute and decorator nodes, per language. Rust attributes
/// and Python decorators are siblings of the declaration, never inside it.
fn is_annotation(node: Node<'_>, language: ProgrammingLanguage) -> bool {
    use ProgrammingLanguage as L;
    let kind = node.kind();
    match language {
        L::Java => matches!(kind, "annotation" | "marker_annotation"),
        L::CSharp | L::Php => kind == "attribute_list",
        L::Swift => kind == "attribute",
        L::Scala => kind == "annotation",
        L::TypeScript | L::JavaScript => kind == "decorator",
        L::C | L::Cpp => matches!(kind, "attribute_declaration" | "attribute_specifier"),
        L::Rust => kind == "attribute_item",
        L::Go | L::Python | L::Perl | L::Lua | L::Bash | L::Zsh => false,
    }
}
