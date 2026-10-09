use std::path::Path;

use super::*;
use crate::file::parents::IndexedParent as _;

/// Extracts the `public` / `protected` / `private` keyword of a TypeScript
/// class member; `None` when it has none.
pub(crate) fn extract_ts_visibility(node: Node<'_>, source: &str) -> Option<Visibility> {
    let modifier = find_child_by_kind(node, "accessibility_modifier")?;
    match modifier.utf8_text(source.as_bytes()).ok()? {
        "public" => Some(Visibility::Public),
        "protected" => Some(Visibility::Protected),
        "private" => Some(Visibility::Private),
        _ => None,
    }
}

/// JavaScript and TypeScript visibility.
///
/// - class members: an access keyword, Private for a `#name`, else Public;
/// - interface members: Public;
/// - enum members: the owning enum's level;
/// - top-level declarations, namespaces included: Public when exported, else
///   ModulePrivate in a module file and Unknown in a classic or global script;
/// - anything else (declarations inside a namespace body, for example): Unknown.
pub(super) fn extract_ecma_visibility(
    declaration: Node<'_>,
    ctx: &DeclarationContext<'_, '_>,
) -> Option<Visibility> {
    let parent = declaration.indexed_parent()?;
    match parent.kind() {
        "class_body" => {
            if let Some(level) = extract_ts_visibility(declaration, ctx.source) {
                return Some(level);
            }
            let is_hash_private = declaration
                .child_by_field_name("name")
                .or_else(|| declaration.child_by_field_name("property"))
                .is_some_and(|name| name.kind() == "private_property_identifier");
            Some(if is_hash_private {
                Visibility::Private
            } else {
                Visibility::Public
            })
        }
        "interface_body" => Some(Visibility::Public),
        "enum_body" => extract_ecma_visibility(parent.indexed_parent()?, ctx),
        _ => top_level_visibility(declaration, ctx.is_ecma_module),
    }
}

fn top_level_visibility(declaration: Node<'_>, is_module: bool) -> Option<Visibility> {
    let mut current = declaration;
    while let Some(parent) = current.indexed_parent() {
        match parent.kind() {
            "program" => return is_module.then_some(Visibility::ModulePrivate),
            "export_statement" => {
                return parent
                    .indexed_parent()
                    .is_some_and(|grandparent| grandparent.kind() == "program")
                    .then_some(Visibility::Public);
            }
            // `const a = 1, b = 2;` declares each name in a declarator, and
            // `const f = () => …` declares a function as a declarator's value.
            "variable_declarator" | "lexical_declaration" | "variable_declaration" => current = parent,
            // `namespace A {}` parses as an expression statement.
            "expression_statement" if current.kind() == "internal_module" => current = parent,
            _ => return None,
        }
    }
    None
}

/// Whether a JavaScript/TypeScript file is a module rather than a classic or
/// global script: by extension (`.mjs`, `.cjs`, `.mts`, `.cts`), by a
/// top-level `import`/`export` statement, or by a `require(...)` call or a
/// `module.exports` / `exports.…` assignment anywhere in the file.
/// `package.json` is not consulted, so the answer depends on this file alone.
pub(crate) fn is_ecma_module_file(path: &Path, root: Node<'_>, source: &str) -> bool {
    let module_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "mjs" | "cjs" | "mts" | "cts"));
    if module_extension {
        return true;
    }

    let mut cursor = root.walk();
    if root
        .children(&mut cursor)
        .any(|child| matches!(child.kind(), "import_statement" | "export_statement"))
    {
        return true;
    }

    has_commonjs_evidence(root, source)
}

fn has_commonjs_evidence(node: Node<'_>, source: &str) -> bool {
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or_default();
    match node.kind() {
        "call_expression"
            if node
                .child_by_field_name("function")
                .map(unparenthesized)
                .is_some_and(|function| function.kind() == "identifier" && text(function) == "require") =>
        {
            return true;
        }
        "assignment_expression"
            if node
                .child_by_field_name("left")
                .is_some_and(|left| is_commonjs_export_target(left, source)) =>
        {
            return true;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| has_commonjs_evidence(child, source))
}

/// `function` inside any parentheses around it: `(require)("fs")` calls
/// `require`; a sequence `(0, require)` is not `require`.
fn unparenthesized(mut function: Node<'_>) -> Node<'_> {
    while function.kind() == "parenthesized_expression"
        && function.named_child_count() == 1
        && let Some(inner) = function.named_child(0)
    {
        function = inner;
    }
    function
}

/// `module.exports`, `exports.x`, `module.exports.x` and the like.
fn is_commonjs_export_target(left: Node<'_>, source: &str) -> bool {
    if left.kind() != "member_expression" {
        return false;
    }
    let text = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or_default();
    let mut object = left;
    loop {
        let Some(inner) = object.child_by_field_name("object") else {
            return false;
        };
        match inner.kind() {
            "identifier" if text(inner) == "exports" => return true,
            "identifier" => {
                return text(inner) == "module"
                    && object
                        .child_by_field_name("property")
                        .is_some_and(|property| text(property) == "exports");
            }
            "member_expression" => object = inner,
            _ => return false,
        }
    }
}
