//! TypeScript and JavaScript binding rules (tree-sitter-typescript 0.23.2,
//! tree-sitter-javascript 0.25).
//!
//! `let`, `const`, `class` and block-level `function` are block scoped and
//! hide an outer name for the whole block (the temporal dead zone); `var`
//! belongs to the enclosing function, or to its body when the parameter list
//! holds an expression (a separate var environment the parameters' defaults
//! cannot see, [`var_body`]); `function` and `var` are hoisted. So every
//! binding is visible throughout its scope. TypeScript `interface` and
//! `type` are types only, `class`, `enum` and `namespace` both; JavaScript
//! has values only.
//!
//! A `with` body looks every name up in its object first, and a direct
//! `eval` may add a `var` to its function: neither establishes a binding
//! for a name the file declares further out ([`super::Dynamic`]).

use tree_sitter::Node;

use crate::file::parents::IndexedParent as _;
use crate::shared::ProgrammingLanguage;
use crate::analysis::occurrences::invoked_operand;

use super::{ANY_CATEGORY, BindingRule, BindingSite, Dynamic, Placement, TYPE, VALUE};

/// Scopes the vendored locals query lacks: generators, class static blocks,
/// `switch` bodies, a `with` body, and the declarations whose type or
/// signature parameters would otherwise leak into the enclosing scope.
/// A `with` body is one too, reached from its statement ([`with_body`]).
pub(super) fn extra_scope(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "generator_function_declaration"
            | "generator_function"
            | "function"
            | "class_static_block"
            | "switch_body"
            | "class_declaration"
            | "abstract_class_declaration"
            | "class"
            | "interface_declaration"
            | "type_alias_declaration"
            | "function_signature"
            | "method_signature"
            | "abstract_method_signature"
            | "call_signature"
            | "construct_signature"
            | "function_type"
            | "constructor_type"
        )
}

/// The `body` of `node` when it is a `with` statement.
pub(super) fn with_body(node: Node<'_>) -> Option<Node<'_>> {
    (node.kind() == "with_statement").then(|| node.child_by_field_name("body")).flatten()
}

/// For a `with` statement, its body; for a direct `eval` call
/// ([`is_direct_eval`]), the function around it.
pub(super) fn dynamic<'tree>(node: Node<'tree>, language: ProgrammingLanguage, source: &str) -> Option<Dynamic<'tree>> {
    if let Some(body) = with_body(node) {
        return Some(Dynamic::Scope(body));
    }
    is_direct_eval(node, language, source).then_some(Dynamic::FunctionOf(node))
}

/// Whether `node` is a direct `eval` call, the only call that can add a
/// `var` to the function around it: an ordinary (not optional) call with an
/// argument list whose callee is the identifier `eval`. A callee that only
/// restates `eval` (`(eval)(…)`, TS `(eval as any)(…)`, `eval!`) keeps the
/// call direct: the parentheses pass the reference through, and the
/// type-only forms are erased. An optional call (`eval?.(…)`) is an indirect
/// eval, which runs in the global scope, and a tagged template (``eval`…` ``)
/// passes a strings array, which `eval` returns unevaluated; neither can
/// bind a name here. Binding placement and coverage both read this.
pub(crate) fn is_direct_eval(node: Node<'_>, language: ProgrammingLanguage, source: &str) -> bool {
    if node.kind() != "call_expression"
        || node.child_by_field_name("arguments").is_none_or(|arguments| arguments.kind() != "arguments")
    {
        return false;
    }
    // JavaScript writes `?.` as an `optional_chain` node, TypeScript as an
    // anonymous token.
    let mut cursor = node.walk();
    if node.children(&mut cursor).any(|child| matches!(child.kind(), "optional_chain" | "?.")) {
        return false;
    }
    node.child_by_field_name("function").is_some_and(|function| {
        let function = invoked_operand(function, language);
        function.kind() == "identifier" && function.utf8_text(source.as_bytes()).is_ok_and(|name| crate::analysis::name_identity::equal(language, name, "eval"))
    })
}

/// The scopes a `var` belongs to.
pub(super) fn is_function_scope(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "function"
            | "generator_function"
            | "arrow_function"
            | "method_definition"
            | "class_static_block"
    )
}

/// The body block of the function `node` when its parameter list holds an
/// expression (ECMAScript's `ContainsExpression`: an initializer, or a
/// computed key in a pattern), whose `var`s are then out of the parameters'
/// reach.
pub(super) fn var_body(node: Node<'_>) -> Option<Node<'_>> {
    if !is_function_scope(node.kind()) {
        return None;
    }
    let parameters = node.child_by_field_name("parameters")?;
    let mut stack = vec![parameters];
    let mut contains_expression = false;
    while let Some(current) = stack.pop() {
        match current.kind() {
            "assignment_pattern" | "object_assignment_pattern" | "computed_property_name" => {
                contains_expression = true;
                break;
            }
            "required_parameter" | "optional_parameter" if current.child_by_field_name("value").is_some() => {
                contains_expression = true;
                break;
            }
            // Only patterns are walked: an expression is reached only
            // through an initializer or a computed key, which decide first.
            "formal_parameters" | "required_parameter" | "optional_parameter" | "object_pattern" | "array_pattern"
            | "pair_pattern" | "rest_pattern" => {
                let mut cursor = current.walk();
                stack.extend(current.named_children(&mut cursor));
            }
            _ => {}
        }
    }
    contains_expression
        .then(|| node.child_by_field_name("body"))
        .flatten()
        .filter(|body| body.kind() == "statement_block")
}

const fn declared(categories: u8, callable: bool) -> BindingRule {
    BindingRule {
        categories,
        placement: Placement::Innermost,
        visible_from: 0,
        visible_until: None,
        local: false,
        ordered: false,
        callable,
        pattern_constant: false,
    }
}

pub(super) fn binding_sites<'tree>(node: Node<'tree>, typed: bool, sites: &mut Vec<BindingSite<'tree>>) {
    let both = if typed { TYPE | VALUE } else { VALUE };
    let mut bindings: Vec<(Node<'tree>, BindingRule)> = Vec::new();
    let name = node.child_by_field_name("name");
    let mut named = |rule: BindingRule| {
        if let Some(name) = name.filter(|name| name.kind() != "nested_identifier") {
            bindings.push((name, rule));
        }
    };
    match node.kind() {
        "function_declaration" | "generator_function_declaration" | "function_signature" => {
            named(declared(VALUE, true));
        }
        // A named function or generator expression's name is visible only
        // inside the expression (its parameters and body), never around it.
        "function_expression" | "function" | "generator_function" => named(declared(VALUE, true)),
        // A class expression's name is visible only inside the class.
        "class_declaration" | "abstract_class_declaration" | "class" => named(declared(both, true)),
        "interface_declaration" | "type_alias_declaration" => named(declared(TYPE, false)),
        "enum_declaration" | "internal_module" | "module" => named(declared(both, false)),
        "type_parameter" => named(declared(TYPE, false)),
        "variable_declarator" => {
            let is_var = node.indexed_parent().is_some_and(|parent| parent.kind() == "variable_declaration");
            let is_const = node
                .indexed_parent()
                .and_then(|parent| parent.child_by_field_name("kind"))
                .is_some_and(|kind| kind.kind() == "const");
            // Calling a `const` bound to a function expression calls that
            // function; any other variable may hold anything.
            let callable = is_const
                && node.child_by_field_name("value").is_some_and(|value| {
                    matches!(
                        value.kind(),
                        "arrow_function" | "function_expression" | "function" | "generator_function" | "class"
                    )
                });
            let rule = BindingRule {
                placement: if is_var { Placement::FunctionScope } else { Placement::Innermost },
                ..declared(VALUE, callable)
            };
            pattern_names(node.child_by_field_name("name"), &mut |name| bindings.push((name, rule)));
        }
        "formal_parameters" => {
            let mut cursor = node.walk();
            for parameter in node.named_children(&mut cursor) {
                let pattern = match parameter.kind() {
                    "required_parameter" | "optional_parameter" => parameter.child_by_field_name("pattern"),
                    _ => Some(parameter),
                };
                pattern_names(pattern, &mut |name| bindings.push((name, declared(VALUE, false))));
            }
        }
        "arrow_function" => {
            if let Some(parameter) = node.child_by_field_name("parameter") {
                pattern_names(Some(parameter), &mut |name| bindings.push((name, declared(VALUE, false))));
            }
        }
        "catch_clause" => {
            pattern_names(node.child_by_field_name("parameter"), &mut |name| {
                bindings.push((name, declared(VALUE, false)));
            });
        }
        "for_in_statement" => {
            if let Some(kind) = node.child_by_field_name("kind") {
                let rule = BindingRule {
                    placement: if kind.kind() == "var" { Placement::FunctionScope } else { Placement::Innermost },
                    ..declared(VALUE, false)
                };
                pattern_names(node.child_by_field_name("left"), &mut |name| bindings.push((name, rule)));
            }
        }
        "import_clause" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                import_names(child, typed && node.indexed_parent().is_some_and(type_only_import), &mut |name, type_only| {
                    let rule = if type_only { declared(TYPE, false) } else { declared(ANY_CATEGORY, true) };
                    sites.push(BindingSite { name, rule, is_import: true, bare_pattern: None });
                });
            }
        }
        // TypeScript `import q = require("m");` and `import R = N.S;`.
        "import_require_clause" | "import_alias" => {
            let mut cursor = node.walk();
            let first = node.named_children(&mut cursor).find(|child| !child.is_extra());
            if let Some(name) = first.filter(|name| name.kind() == "identifier") {
                let type_only = typed && (type_only_import(node) || node.indexed_parent().is_some_and(type_only_import));
                let rule = if type_only { declared(TYPE, false) } else { declared(ANY_CATEGORY, true) };
                sites.push(BindingSite { name, rule, is_import: true, bare_pattern: None });
            }
        }
        _ => {}
    }
    sites.extend(
        bindings
            .into_iter()
            .map(|(name, rule)| BindingSite { name, rule, is_import: false, bare_pattern: None }),
    );
}

/// The local names one part of an `import` clause binds: a default import,
/// `* as ns`, and each specifier's alias (else its name). The original name
/// of an aliased specifier is never bound. Each local retains its statement
/// or specifier type-only modifier independently of neighboring imports.
fn import_names<'tree>(node: Node<'tree>, type_only: bool, bind: &mut impl FnMut(Node<'tree>, bool)) {
    match node.kind() {
        "identifier" => bind(node, type_only),
        "namespace_import" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "identifier" {
                    bind(child, type_only);
                }
            }
        }
        "named_imports" => {
            let mut cursor = node.walk();
            for specifier in node.named_children(&mut cursor) {
                if specifier.kind() == "import_specifier"
                    && let Some(local) = specifier
                        .child_by_field_name("alias")
                        .or_else(|| specifier.child_by_field_name("name"))
                    && local.kind() == "identifier"
                {
                    bind(local, type_only || type_only_import(specifier));
                }
            }
        }
        _ => {}
    }
}

/// Only the anonymous modifier token is evidence: a named identifier
/// spelled `type` in `import { type as g }` is an ordinary exported name.
fn type_only_import(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| !child.is_named() && child.kind() == "type")
}

/// The identifiers a binding pattern binds; default values and property
/// keys bind nothing.
fn pattern_names<'tree>(pattern: Option<Node<'tree>>, bind: &mut impl FnMut(Node<'tree>)) {
    let Some(pattern) = pattern else { return };
    let mut stack = vec![pattern];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier" | "shorthand_property_identifier_pattern" => bind(node),
            "pair_pattern" => stack.extend(node.child_by_field_name("value")),
            "assignment_pattern" | "object_assignment_pattern" => stack.extend(node.child_by_field_name("left")),
            "object_pattern" | "array_pattern" | "rest_pattern" => {
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
            _ => {}
        }
    }
}

/// A reference's categories: a TypeScript type name, a namespace qualifier
/// in a type, else a value.
pub(super) fn reference_category(node: Node<'_>, typed: bool) -> u8 {
    if !typed {
        return VALUE;
    }
    if node.kind() == "type_identifier" {
        return TYPE;
    }
    match node.indexed_parent() {
        Some(parent) if parent.kind() == "nested_type_identifier" => TYPE | VALUE,
        _ => VALUE,
    }
}
