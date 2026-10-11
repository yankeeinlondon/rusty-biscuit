//! CommonJS forwarding: `module.exports = require(..)`, property
//! assignments and object literals built from `require` bindings.

use tree_sitter::Node;

use super::bindings::{Binding, Kind, Resolved, Table, is_function, shadowed_below_program};
use super::{Ctx, Out, binding_entry};
use crate::analysis::reexports::common::{entry, limit};
use crate::file::extract::text_span_for_node;
use crate::shared::{ReexportEvidence, ReexportForm, ReexportLimitReason};

pub(super) enum Require {
    Literal(String),
    Dynamic,
}

/// `require("m")` as a call; `None` when `node` is not a `require` call.
pub(super) fn require_call(ctx: &Ctx<'_>, node: Node<'_>) -> Option<Require> {
    if node.kind() != "call_expression" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    if function.kind() != "identifier" || ctx.text(function) != "require" {
        return None;
    }
    let arguments = node.child_by_field_name("arguments")?;
    let mut cursor = arguments.walk();
    let args: Vec<_> = arguments.named_children(&mut cursor).filter(|c| c.kind() != "comment").collect();
    match args.as_slice() {
        [only] if only.kind() == "string" => Some(Require::Literal(ctx.string_value(*only))),
        _ => Some(Require::Dynamic),
    }
}

enum Root {
    Module,
    Exports,
}

enum Target {
    Whole,
    Prop(String),
    Computed,
}

enum Value<'t> {
    Require(String),
    RequireMember(String, String),
    Binding(String),
    BindingMember(String, String),
    Object(Node<'t>),
    Other,
    Dynamic,
}

fn is_module_exports(ctx: &Ctx<'_>, node: Node<'_>) -> bool {
    node.kind() == "member_expression"
        && node.child_by_field_name("object").is_some_and(|o| o.kind() == "identifier" && ctx.text(o) == "module")
        && node.child_by_field_name("property").is_some_and(|p| ctx.text(p) == "exports")
}

fn target(ctx: &Ctx<'_>, left: Node<'_>) -> Option<(Root, Target)> {
    if is_module_exports(ctx, left) {
        return Some((Root::Module, Target::Whole));
    }
    let (object, key) = match left.kind() {
        "member_expression" => {
            let property = left.child_by_field_name("property")?;
            (left.child_by_field_name("object")?, Some(ctx.text(property).to_owned()))
        }
        "subscript_expression" => {
            let index = left.child_by_field_name("index")?;
            let key = (index.kind() == "string").then(|| ctx.string_value(index));
            (left.child_by_field_name("object")?, key)
        }
        _ => return None,
    };
    let root = if is_module_exports(ctx, object) {
        Root::Module
    } else if object.kind() == "identifier" && ctx.text(object) == "exports" {
        Root::Exports
    } else {
        return None;
    };
    Some((root, key.map_or(Target::Computed, Target::Prop)))
}

/// Whether `node` mentions a `require` call or a module-scope binding.
fn involves(ctx: &Ctx<'_>, table: &Table, node: Node<'_>) -> bool {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        match current.kind() {
            "identifier" | "shorthand_property_identifier" if table.has_binding(ctx.text(current)) => return true,
            "call_expression" if require_call(ctx, current).is_some() => return true,
            _ => {}
        }
        let mut cursor = current.walk();
        stack.extend(current.children(&mut cursor));
    }
    false
}

fn contains_require(ctx: &Ctx<'_>, node: Node<'_>) -> bool {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if require_call(ctx, current).is_some() {
            return true;
        }
        let mut cursor = current.walk();
        stack.extend(current.children(&mut cursor));
    }
    false
}

fn classify<'t>(ctx: &Ctx<'_>, table: &Table, node: Node<'t>) -> Value<'t> {
    match node.kind() {
        "parenthesized_expression" => node
            .named_child(0)
            .map_or(Value::Other, |inner| classify(ctx, table, inner)),
        "call_expression" => match require_call(ctx, node) {
            Some(Require::Literal(source)) => Value::Require(source),
            Some(Require::Dynamic) => Value::Dynamic,
            None if involves(ctx, table, node) => Value::Dynamic,
            None => Value::Other,
        },
        "member_expression" => {
            let object = node.child_by_field_name("object");
            let property = node.child_by_field_name("property").filter(|p| p.kind() == "property_identifier");
            match (object, property) {
                (Some(object), Some(property)) => {
                    if let Some(Require::Literal(source)) = require_call(ctx, object) {
                        Value::RequireMember(source, ctx.text(property).to_owned())
                    } else if object.kind() == "identifier" && table.has_binding(ctx.text(object)) {
                        Value::BindingMember(ctx.text(object).to_owned(), ctx.text(property).to_owned())
                    } else if involves(ctx, table, node) {
                        Value::Dynamic
                    } else {
                        Value::Other
                    }
                }
                _ if involves(ctx, table, node) => Value::Dynamic,
                _ => Value::Other,
            }
        }
        "identifier" if table.has_binding(ctx.text(node)) => Value::Binding(ctx.text(node).to_owned()),
        "object" => Value::Object(node),
        _ if involves(ctx, table, node) => Value::Dynamic,
        _ => Value::Other,
    }
}

struct Site<'a> {
    span: crate::shared::TextSpan,
    statement_span: crate::shared::TextSpan,
    conditional: bool,
    node: Node<'a>,
}

fn is_shadowed(ctx: &Ctx<'_>, table: &Table, node: Node<'_>, name: &str) -> bool {
    table.is_declared(name) || shadowed_below_program(ctx, node, name)
}

/// Whether `node` sits inside a function, and whether a branch decides if it
/// runs.
fn placement(node: Node<'_>) -> (bool, bool) {
    let (mut in_function, mut conditional) = (false, false);
    let mut current = node.parent();
    while let Some(ancestor) = current {
        if is_function(ancestor) {
            in_function = true;
        }
        if matches!(
            ancestor.kind(),
            "if_statement"
                | "else_clause"
                | "switch_case"
                | "switch_default"
                | "try_statement"
                | "catch_clause"
                | "finally_clause"
                | "for_statement"
                | "for_in_statement"
                | "while_statement"
                | "do_statement"
                | "ternary_expression"
                | "binary_expression"
        ) {
            conditional = true;
        }
        current = ancestor.parent();
    }
    (in_function, conditional)
}

fn statement_of(node: Node<'_>) -> Node<'_> {
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "expression_statement" {
            return parent;
        }
        if matches!(parent.kind(), "program" | "statement_block") {
            break;
        }
        current = parent;
    }
    node
}

/// Reports the forwards and limits of every `module.exports` / `exports`
/// write and merge call in the file.
pub(super) fn scan(ctx: &Ctx<'_>, table: &Table, assigns: &[Node<'_>], calls: &[Node<'_>], out: &mut Out) {
    for assign in assigns {
        if assign.has_error() {
            continue;
        }
        let (Some(left), Some(right)) = (assign.child_by_field_name("left"), assign.child_by_field_name("right"))
        else {
            continue;
        };
        let Some((root, target)) = target(ctx, left) else { continue };
        if !involves(ctx, table, right) {
            continue;
        }
        let root_name = match root {
            Root::Module => "module",
            Root::Exports => "exports",
        };
        let shadowed = is_shadowed(ctx, table, *assign, root_name)
            || (contains_require(ctx, right) && is_shadowed(ctx, table, *assign, "require"));
        if shadowed {
            out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, *assign));
            continue;
        }
        let (in_function, conditional) = placement(*assign);
        if in_function {
            out.limits.push(limit(ReexportLimitReason::DynamicForwarding, *assign));
            continue;
        }
        let statement = statement_of(*assign);
        let site = Site {
            span: text_span_for_node(*assign),
            statement_span: text_span_for_node(statement),
            conditional,
            node: *assign,
        };
        let value = classify(ctx, table, right);
        match target {
            Target::Whole => whole(ctx, table, value, &site, out),
            Target::Prop(name) => property(ctx, table, name, value, &site, out),
            Target::Computed => out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node)),
        }
    }
    for call in calls {
        merge_call(ctx, table, *call, out);
    }
}

/// `Object.assign(module.exports, ..)` with an import-derived argument.
fn merge_call(ctx: &Ctx<'_>, table: &Table, call: Node<'_>, out: &mut Out) {
    if call.has_error() {
        return;
    }
    let (Some(function), Some(arguments)) = (call.child_by_field_name("function"), call.child_by_field_name("arguments"))
    else {
        return;
    };
    let is_assign = function.kind() == "member_expression"
        && function.child_by_field_name("object").is_some_and(|o| ctx.text(o) == "Object")
        && function.child_by_field_name("property").is_some_and(|p| ctx.text(p) == "assign");
    if !is_assign {
        return;
    }
    let mut cursor = arguments.walk();
    let args: Vec<_> = arguments.named_children(&mut cursor).collect();
    let Some((first, rest)) = args.split_first() else { return };
    let merges_exports = is_module_exports(ctx, *first) || (first.kind() == "identifier" && ctx.text(*first) == "exports");
    if !merges_exports || !rest.iter().any(|arg| involves(ctx, table, *arg)) {
        return;
    }
    let root_name = if first.kind() == "identifier" { "exports" } else { "module" };
    let shadowed = is_shadowed(ctx, table, call, root_name)
        || (rest.iter().any(|arg| contains_require(ctx, *arg)) && is_shadowed(ctx, table, call, "require"));
    let reason = if shadowed { ReexportLimitReason::ShadowedGlobal } else { ReexportLimitReason::DynamicForwarding };
    out.limits.push(limit(reason, call));
}

fn declared_entry(form: ReexportForm, site: &Site<'_>) -> crate::shared::ReexportEntry {
    let mut built = entry(form, ReexportEvidence::Declared, site.span.clone(), site.statement_span.clone());
    built.conditional = site.conditional;
    built
}

/// `module.exports = <value>`.
fn whole(ctx: &Ctx<'_>, table: &Table, value: Value<'_>, site: &Site<'_>, out: &mut Out) {
    match value {
        Value::Require(source) => {
            let mut built = declared_entry(ReexportForm::Module, site);
            built.source = Some(source);
            out.entries.push(built);
        }
        Value::Binding(name) => match table.resolve_any(&name) {
            Resolved::Binding(binding) if matches!(binding.kind, Kind::Whole) => {
                let mut built = declared_entry(ReexportForm::Module, site);
                built.source = Some(binding.source.clone());
                built.local_name = Some(name);
                built.import_span = binding.import_span.clone();
                built.type_only = binding.type_only;
                out.entries.push(built);
            }
            Resolved::Binding(_) | Resolved::Unbound => {
                out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node));
            }
            Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, site.node)),
            Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, site.node)),
        },
        Value::Object(object) => object_members(ctx, table, object, site, out),
        Value::RequireMember(..) | Value::BindingMember(..) | Value::Dynamic => {
            out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node));
        }
        Value::Other => {}
    }
}

fn object_members(ctx: &Ctx<'_>, table: &Table, object: Node<'_>, site: &Site<'_>, out: &mut Out) {
    let mut cursor = object.walk();
    for member in object.named_children(&mut cursor) {
        let member_site = Site {
            span: text_span_for_node(member),
            statement_span: site.statement_span.clone(),
            conditional: site.conditional,
            node: member,
        };
        match member.kind() {
            "shorthand_property_identifier" => {
                let name = ctx.text(member).to_owned();
                let value = Value::Binding(name.clone());
                property(ctx, table, name, value, &member_site, out);
            }
            "pair" => {
                let (Some(key), Some(value)) = (member.child_by_field_name("key"), member.child_by_field_name("value"))
                else {
                    continue;
                };
                if key.kind() == "computed_property_name" {
                    if involves(ctx, table, value) {
                        out.limits.push(limit(ReexportLimitReason::DynamicForwarding, member));
                    }
                    continue;
                }
                let name = ctx.name_text(key);
                property(ctx, table, name, classify(ctx, table, value), &member_site, out);
            }
            "spread_element" if involves(ctx, table, member) => {
                out.limits.push(limit(ReexportLimitReason::DynamicForwarding, member));
            }
            _ => {}
        }
    }
}

/// One exported property `name` initialized from `value`.
fn property(ctx: &Ctx<'_>, table: &Table, name: String, value: Value<'_>, site: &Site<'_>, out: &mut Out) {
    match value {
        Value::Require(source) => {
            let mut built = declared_entry(ReexportForm::Namespace, site);
            built.exported_name = Some(name);
            built.source = Some(source);
            out.entries.push(built);
        }
        Value::RequireMember(source, original) => {
            let mut built = declared_entry(ReexportForm::Named, site);
            built.exported_name = Some(name);
            built.original_name = Some(original);
            built.source = Some(source);
            out.entries.push(built);
        }
        Value::Binding(local) => match table.resolve_any(&local) {
            Resolved::Binding(binding) => {
                let mut built = binding_entry(binding, &local, name, ReexportEvidence::Declared, site.span.clone(), site.statement_span.clone());
                built.conditional = site.conditional;
                out.entries.push(built);
            }
            Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, site.node)),
            Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, site.node)),
            Resolved::Unbound => {}
        },
        Value::BindingMember(local, member) => match table.resolve_any(&local) {
            Resolved::Binding(Binding { kind: Kind::Whole, source, import_span, type_only }) => {
                let mut built = declared_entry(ReexportForm::Named, site);
                built.exported_name = Some(name);
                built.original_name = Some(member);
                built.local_name = Some(local);
                built.source = Some(source.clone());
                built.import_span = import_span.clone();
                built.type_only = *type_only;
                out.entries.push(built);
            }
            Resolved::Binding(_) => out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node)),
            Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, site.node)),
            Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, site.node)),
            Resolved::Unbound => {}
        },
        Value::Object(object) => {
            if involves(ctx, table, object) {
                out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node));
            }
        }
        Value::Dynamic => out.limits.push(limit(ReexportLimitReason::DynamicForwarding, site.node)),
        Value::Other => {}
    }
}

/// Whether `node` is a call `require(..)` or mentions a binding; shared
/// with the TypeScript `export =` form.
pub(super) fn involves_forwarding(ctx: &Ctx<'_>, table: &Table, node: Node<'_>) -> bool {
    involves(ctx, table, node)
}

/// Whether `name` is hidden at `node` (module-scope declaration of a global
/// or a shadowing scope).
pub(super) fn global_shadowed(ctx: &Ctx<'_>, table: &Table, node: Node<'_>, name: &str) -> bool {
    is_shadowed(ctx, table, node, name)
}
