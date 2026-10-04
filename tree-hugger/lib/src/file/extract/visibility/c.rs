//! C visibility, and the file-scope linkage rule C++ shares.

use super::*;

/// C visibility. Only file-scope functions and objects have an access level,
/// decided by [`linkage_visibility`]; types, tags, members, enumerators and
/// macros have no access modifier and report Unknown.
pub(super) fn extract_c_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
) -> Option<Visibility> {
    (has_linkage(ctx.kind) && is_namespace_scope(declaration))
        .then(|| linkage_visibility(declaration, ctx, false))
}

/// Functions and objects have linkage; nothing else does.
pub(super) fn has_linkage(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function | SymbolKind::Variable | SymbolKind::Constant
    )
}

/// Whether `declaration` sits at file scope, or at namespace scope in C++,
/// looking through `extern "C"` blocks, preprocessor conditionals and
/// template headers.
pub(super) fn is_namespace_scope(declaration: Node<'_>) -> bool {
    let mut current = declaration.parent();
    while let Some(node) = current {
        match node.kind() {
            "translation_unit" => return true,
            "declaration_list" => {
                if !node.parent().is_some_and(|parent| {
                    matches!(parent.kind(), "namespace_definition" | "linkage_specification")
                }) {
                    return false;
                }
            }
            "namespace_definition"
            | "linkage_specification"
            | "template_declaration"
            | "preproc_if"
            | "preproc_ifdef"
            | "preproc_else"
            | "preproc_elif"
            | "preproc_elifdef" => {}
            _ => return false,
        }
        current = node.parent();
    }
    false
}

/// The linkage rule for a file-scope (C) or namespace-scope (C++) function or
/// object: ModulePrivate when it is `static`, sits in an unnamed namespace
/// (C++), is a C++ object that is itself `const` (including `constexpr`)
/// without `extern` or `inline`, or is a function an earlier same-file
/// declaration made `static`; Public otherwise.
pub(super) fn linkage_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
    cpp: bool,
) -> Visibility {
    let source = ctx.source;
    let is_function = ctx.kind.is_function();
    let module_private = has_specifier(declaration, "storage_class_specifier", "static", source)
        || (cpp && in_unnamed_namespace(declaration))
        || (cpp
            && !is_function
            && is_const_object(declaration, ctx.name, source)
            && !has_specifier(declaration, "storage_class_specifier", "extern", source)
            && !has_specifier(declaration, "storage_class_specifier", "inline", source))
        || (is_function && has_earlier_static_declaration(declaration, ctx, cpp));

    if module_private {
        Visibility::ModulePrivate
    } else {
        Visibility::Public
    }
}

fn has_specifier(node: Node<'_>, kind: &str, keyword: &str, source: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        child.kind() == kind && child.utf8_text(source.as_bytes()).ok() == Some(keyword)
    })
}

fn in_unnamed_namespace(declaration: Node<'_>) -> bool {
    let mut current = declaration.parent();
    while let Some(node) = current {
        if node.kind() == "namespace_definition" && node.child_by_field_name("name").is_none() {
            return true;
        }
        current = node.parent();
    }
    false
}

/// Whether the object `name` declares is itself `const`: `constexpr`, or the
/// top-level `const` of the object, not of what a pointer points to
/// (`char* const p` is const, `const char* p` is not). A reference is never a
/// const object.
fn is_const_object(declaration: Node<'_>, name: Node<'_>, source: &str) -> bool {
    if has_specifier(declaration, "type_qualifier", "constexpr", source) {
        return true;
    }

    let Some(mut declarator) = child_on_path(declaration, name) else {
        return false;
    };
    if declarator.kind() == "init_declarator"
        && let Some(inner) = declarator.child_by_field_name("declarator")
    {
        declarator = inner;
    }
    match declarator.kind() {
        "pointer_declarator" => has_specifier(declarator, "type_qualifier", "const", source),
        "reference_declarator" => false,
        _ => has_specifier(declaration, "type_qualifier", "const", source),
    }
}

/// The child of `ancestor` on the path down to `descendant`.
fn child_on_path<'tree>(ancestor: Node<'tree>, descendant: Node<'tree>) -> Option<Node<'tree>> {
    let mut node = descendant;
    while let Some(parent) = node.parent() {
        if parent == ancestor {
            return Some(node);
        }
        node = parent;
    }
    None
}

/// Whether an earlier declaration of the same function in this file is
/// `static`. In C the name alone identifies the function; in C++ the
/// enclosing namespace and the whitespace-free parameter-type text must match
/// as well, so a differently spelled parameter type falls back to the
/// declaration's own rule.
fn has_earlier_static_declaration(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_>,
    cpp: bool,
) -> bool {
    let source = ctx.source;
    let Ok(name) = ctx.name.utf8_text(source.as_bytes()) else {
        return false;
    };
    let identity = |candidate: Node<'_>| {
        cpp.then(|| {
            (
                namespace_path(candidate, source),
                function_declarator(candidate).map(|function| parameter_types(function, source)),
            )
        })
    };
    let own_identity = identity(declaration);

    let mut earlier = Vec::new();
    collect_scope_declarations(ctx.root, declaration.start_byte(), &mut earlier);
    earlier.into_iter().any(|candidate| {
        has_specifier(candidate, "storage_class_specifier", "static", source)
            && function_declarator(candidate)
                .and_then(|function| function.child_by_field_name("declarator"))
                .and_then(|declarator| declarator.utf8_text(source.as_bytes()).ok())
                == Some(name)
            && identity(candidate) == own_identity
    })
}

/// Namespace-scope declarations and definitions that end before `before`.
fn collect_scope_declarations<'tree>(node: Node<'tree>, before: usize, found: &mut Vec<Node<'tree>>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.start_byte() >= before {
            break;
        }
        match child.kind() {
            "declaration" | "function_definition" => found.push(child),
            "namespace_definition" | "linkage_specification" | "declaration_list"
            | "preproc_if" | "preproc_ifdef" | "preproc_else" | "preproc_elif"
            | "preproc_elifdef" => collect_scope_declarations(child, before, found),
            _ => {}
        }
    }
}

/// The `function_declarator` a declaration or definition declares, looking
/// through pointer and reference return-type declarators.
fn function_declarator(declaration: Node<'_>) -> Option<Node<'_>> {
    let mut declarator = declaration.child_by_field_name("declarator")?;
    loop {
        match declarator.kind() {
            "function_declarator" => return Some(declarator),
            kind if kind.ends_with("_declarator") => {
                declarator = declarator.child_by_field_name("declarator")?;
            }
            _ => return None,
        }
    }
}

fn namespace_path(declaration: Node<'_>, source: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = declaration.parent();
    while let Some(node) = current {
        if node.kind() == "namespace_definition" {
            path.push(
                node.child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source.as_bytes()).ok())
                    .unwrap_or_default()
                    .to_string(),
            );
        }
        current = node.parent();
    }
    path
}

/// Each parameter's text without its name and without whitespace, so
/// `(int)` and `(int value)` compare equal.
fn parameter_types(function: Node<'_>, source: &str) -> Vec<String> {
    let Some(parameters) = function.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .filter(|parameter| parameter.kind() != "comment")
        .map(|parameter| {
            let mut text = source
                .get(parameter.byte_range())
                .unwrap_or_default()
                .to_string();
            if let Some(name) = parameter_name(parameter) {
                let start = name.start_byte() - parameter.start_byte();
                let end = name.end_byte() - parameter.start_byte();
                text.replace_range(start..end, "");
            }
            text.chars().filter(|c| !c.is_whitespace()).collect()
        })
        .collect()
}

fn parameter_name(parameter: Node<'_>) -> Option<Node<'_>> {
    let mut declarator = parameter.child_by_field_name("declarator")?;
    loop {
        if declarator.kind() == "identifier" {
            return Some(declarator);
        }
        declarator = declarator
            .child_by_field_name("declarator")
            .or_else(|| {
                let last = u32::try_from(declarator.named_child_count().checked_sub(1)?).ok()?;
                declarator.named_child(last)
            })?;
    }
}
