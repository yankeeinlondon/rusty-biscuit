//! JavaScript and TypeScript re-exports: ES `export .. from`, import then
//! export, `export default <import>`, TypeScript `export =` and
//! `export import`, and CommonJS `module.exports` forwards.
//!
//! One walk collects the statements that can forward; a module-scope
//! binding table decides which names are imports. Local exports are not
//! forwards and produce nothing.

use std::collections::HashSet;

use tree_sitter::Node;

use super::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ProgrammingLanguage, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit,
    ReexportLimitReason, ReexportReport, TextSpan,
};

mod bindings;
mod cjs;

use bindings::{Binding, Kind, Resolved, Table, has_anonymous, shadowed_below_program};

pub(super) struct Ctx<'a> {
    source: &'a str,
    imports: &'a [ImportRecord],
    typescript: bool,
}

impl<'a> Ctx<'a> {
    pub(super) fn text(&self, node: Node<'_>) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap_or_default()
    }

    /// A string literal's content, or any other node's text.
    pub(super) fn name_text(&self, node: Node<'_>) -> String {
        if node.kind() == "string" { self.string_value(node) } else { self.text(node).to_owned() }
    }

    pub(super) fn string_value(&self, node: Node<'_>) -> String {
        let text = self.text(node);
        let mut chars = text.chars();
        match (chars.next(), chars.next_back()) {
            (Some('"' | '\'' | '`'), Some(_)) => chars.as_str().to_owned(),
            _ => text.to_owned(),
        }
    }

    /// The span of the import record bound at `local`, matched by span.
    pub(super) fn import_span(&self, local: Node<'_>) -> Option<TextSpan> {
        let span = text_span_for_node(local);
        self.imports.iter().any(|import| import.span == span).then_some(span)
    }
}

#[derive(Default)]
pub(super) struct Out {
    pub entries: Vec<ReexportEntry>,
    pub limits: Vec<ReexportLimit>,
}

/// The entry that forwards what `binding` imports as `exported`.
fn binding_entry(
    binding: &Binding,
    local: &str,
    exported: String,
    evidence: ReexportEvidence,
    span: TextSpan,
    statement_span: TextSpan,
) -> ReexportEntry {
    let mut built = match &binding.kind {
        Kind::Named(original) => {
            let mut built = entry(ReexportForm::Named, evidence, span, statement_span);
            built.original_name = Some(original.clone());
            built
        }
        Kind::Whole => entry(ReexportForm::Namespace, evidence, span, statement_span),
    };
    built.exported_name = Some(exported);
    built.local_name = Some(local.to_owned());
    built.source = Some(binding.source.clone());
    built.import_span = binding.import_span.clone();
    built.type_only = binding.type_only;
    built
}

#[derive(Default)]
struct Scan<'t> {
    exports: Vec<Node<'t>>,
    assigns: Vec<Node<'t>>,
    writes: Vec<Node<'t>>,
    calls: Vec<Node<'t>>,
}

fn scan(root: Node<'_>) -> Scan<'_> {
    let mut found = Scan::default();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "export_statement" => found.exports.push(node),
            "assignment_expression" => {
                found.assigns.push(node);
                found.writes.push(node);
            }
            "augmented_assignment_expression"
            | "update_expression"
            | "for_in_statement"
            | "variable_declaration"
            | "function_declaration"
            | "generator_function_declaration" => {
                found.writes.push(node);
            }
            "unary_expression" => found.writes.push(node),
            "call_expression" => found.calls.push(node),
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    found
}

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let ctx = Ctx {
        source: tree_file.source(),
        imports,
        typescript: tree_file.language == ProgrammingLanguage::TypeScript,
    };
    let root = tree_file.root_node();
    let found = scan(root);
    let mut table = Table::build(&ctx, root);
    table.add_writes(&ctx, &found.writes);
    let mut out = Out::default();

    for node in damaged_nodes(root) {
        if touches_forwarding(&ctx, node) {
            let reason = if ctx.typescript && is_typescript_gap(&ctx, node) {
                ReexportLimitReason::GrammarGap
            } else {
                ReexportLimitReason::ParseError
            };
            out.limits.push(limit(reason, node));
        }
    }

    let mut handled = HashSet::new();
    visit_block(&ctx, &table, root, None, &mut handled, &mut out);
    for statement in &found.exports {
        if !handled.contains(&statement.id()) {
            stray_export(&ctx, &table, *statement, &mut out);
        }
    }
    cjs::scan(&ctx, &table, &found.assigns, &found.calls, &mut out);
    let candidates = found.writes.iter().chain(&found.calls).chain(&found.exports).copied();
    cjs::Interface::build(&ctx, &table, candidates).settle(&mut out);
    finish(out.entries, out.limits)
}

/// Whether a damaged node contains or immediately follows syntax that
/// forwards, so that a clean negative would be wrong.
fn touches_forwarding(ctx: &Ctx<'_>, node: Node<'_>) -> bool {
    let mut child = node;
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        match ancestor.kind() {
            "export_statement" => {
                let in_body = [ancestor.child_by_field_name("declaration"), ancestor.child_by_field_name("value")]
                    .into_iter()
                    .flatten()
                    .any(|body| body.id() == child.id() && body.kind() != "import_alias");
                if !in_body {
                    return true;
                }
            }
            "expression_statement" => {
                let text = ctx.text(ancestor);
                if text.contains("require") || text.contains("exports") {
                    return true;
                }
            }
            _ => {}
        }
        child = ancestor;
        parent = ancestor.parent();
    }
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if !current.is_named() && current.kind() == "export" {
            return true;
        }
        if current.kind() == "identifier" && matches!(ctx.text(current), "require" | "exports") {
            return true;
        }
        let mut cursor = current.walk();
        stack.extend(current.children(&mut cursor));
    }
    node.prev_sibling().map(last_leaf).is_some_and(|last| !last.is_named() && matches!(last.kind(), "export" | "from"))
}

fn last_leaf(node: Node<'_>) -> Node<'_> {
    let mut current = node;
    while current.child_count() > 0 {
        match u32::try_from(current.child_count() - 1).ok().and_then(|index| current.child(index)) {
            Some(last) => current = last,
            None => break,
        }
    }
    current
}

/// The TypeScript forms the pinned grammar cannot parse: `export type *`,
/// `export type * as ns from` and `export import D = require("m")`.
fn is_typescript_gap(ctx: &Ctx<'_>, node: Node<'_>) -> bool {
    let mut cursor = node.parent();
    while let Some(ancestor) = cursor {
        if ancestor.kind() == "export_statement" {
            return (node.is_error() && ctx.text(node).trim() == "type")
                || (node.is_missing() && node.parent().is_some_and(|parent| parent.kind() == "import_alias"));
        }
        cursor = ancestor.parent();
    }
    false
}

/// Statements at module level or directly inside a TypeScript namespace or
/// ambient module body.
fn visit_block(
    ctx: &Ctx<'_>,
    table: &Table,
    block: Node<'_>,
    scope: Option<&str>,
    handled: &mut HashSet<usize>,
    out: &mut Out,
) {
    let mut cursor = block.walk();
    for statement in block.children(&mut cursor) {
        visit_statement(ctx, table, statement, scope, handled, out);
    }
}

fn visit_statement(
    ctx: &Ctx<'_>,
    table: &Table,
    statement: Node<'_>,
    scope: Option<&str>,
    handled: &mut HashSet<usize>,
    out: &mut Out,
) {
    match statement.kind() {
        "export_statement" => {
            handled.insert(statement.id());
            if let Some(declaration) = statement.child_by_field_name("declaration")
                && matches!(declaration.kind(), "internal_module" | "module" | "ambient_declaration")
            {
                visit_statement(ctx, table, declaration, scope, handled, out);
                return;
            }
            export_statement(ctx, table, statement, scope, out);
        }
        "ambient_declaration" | "expression_statement" => {
            let mut cursor = statement.walk();
            for child in statement.children(&mut cursor) {
                visit_statement(ctx, table, child, scope, handled, out);
            }
        }
        "internal_module" | "module" => {
            let (Some(name), Some(body)) = (statement.child_by_field_name("name"), statement.child_by_field_name("body"))
            else {
                return;
            };
            let name = ctx.name_text(name).split_whitespace().collect::<String>();
            let qualified = match scope {
                Some(outer) => format!("{outer}.{name}"),
                None => name,
            };
            visit_block(ctx, table, body, Some(&qualified), handled, out);
        }
        _ => {}
    }
}

fn export_statement(
    ctx: &Ctx<'_>,
    table: &Table,
    statement: Node<'_>,
    scope: Option<&str>,
    out: &mut Out,
) {
    if statement.has_error() {
        return;
    }
    let mut cursor = statement.walk();
    let children: Vec<Node<'_>> = statement.children(&mut cursor).collect();
    let anonymous = |kind: &str| children.iter().any(|child| !child.is_named() && child.kind() == kind);
    let statement_span = text_span_for_node(statement);
    let statement_type = anonymous("type");
    let find = |kind: &str| children.iter().copied().find(|child| child.kind() == kind);
    let scope = scope.map(str::to_owned);

    if let Some(source_node) = statement.child_by_field_name("source") {
        let source = ctx.string_value(source_node);
        let mut push = |mut built: ReexportEntry| {
            built.source = Some(source.clone());
            built.scope = scope.clone();
            out.entries.push(built);
        };
        if let Some(clause) = find("export_clause") {
            let mut inner = clause.walk();
            for specifier in clause.children(&mut inner).filter(|c| c.kind() == "export_specifier") {
                let Some(name) = specifier.child_by_field_name("name") else { continue };
                let exported = specifier.child_by_field_name("alias").unwrap_or(name);
                let mut built =
                    entry(ReexportForm::Named, ReexportEvidence::Syntax, text_span_for_node(specifier), statement_span.clone());
                built.exported_name = Some(ctx.name_text(exported));
                built.original_name = Some(ctx.name_text(name));
                built.type_only = statement_type || has_anonymous(specifier, "type");
                push(built);
            }
        } else if let Some(namespace) = find("namespace_export") {
            let mut inner = namespace.walk();
            let Some(name) = namespace.children(&mut inner).last() else { return };
            let mut built =
                entry(ReexportForm::Namespace, ReexportEvidence::Syntax, text_span_for_node(namespace), statement_span);
            built.exported_name = Some(ctx.name_text(name));
            built.type_only = statement_type;
            push(built);
        } else if let Some(star) = children.iter().copied().find(|c| !c.is_named() && c.kind() == "*") {
            let mut built =
                entry(ReexportForm::Wildcard, ReexportEvidence::Syntax, text_span_for_node(star), statement_span);
            built.type_only = statement_type;
            push(built);
        }
        return;
    }

    if let Some(clause) = find("export_clause") {
        let mut inner = clause.walk();
        for specifier in clause.children(&mut inner).filter(|c| c.kind() == "export_specifier") {
            let Some(name) = specifier.child_by_field_name("name") else { continue };
            let exported = specifier.child_by_field_name("alias").unwrap_or(name);
            let local = ctx.text(name);
            if shadowed_below_program(ctx, specifier, local) {
                continue;
            }
            match table.resolve_es(local) {
                Resolved::Binding(binding) => {
                    let mut built = binding_entry(
                        binding,
                        local,
                        ctx.name_text(exported),
                        ReexportEvidence::Syntax,
                        text_span_for_node(specifier),
                        statement_span.clone(),
                    );
                    built.type_only |= statement_type || has_anonymous(specifier, "type");
                    built.scope = scope.clone();
                    out.entries.push(built);
                }
                Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, specifier)),
                Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, specifier)),
                Resolved::Unbound if table.rebound_copy_of_import(local) => {
                    out.limits.push(limit(ReexportLimitReason::ReboundBinding, specifier));
                }
                Resolved::Unbound => {}
            }
        }
    } else if anonymous("default") {
        let Some(value) = statement.child_by_field_name("value").filter(|v| v.kind() == "identifier") else {
            return;
        };
        let local = ctx.text(value);
        if shadowed_below_program(ctx, value, local) {
            return;
        }
        match table.resolve_es(local) {
            Resolved::Binding(binding) => {
                let mut built = binding_entry(
                    binding,
                    local,
                    "default".to_owned(),
                    ReexportEvidence::Syntax,
                    text_span_for_node(value),
                    statement_span,
                );
                built.scope = scope;
                out.entries.push(built);
            }
            Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, value)),
            Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, value)),
            Resolved::Unbound => {}
        }
    } else if ctx.typescript && anonymous("=") {
        let mut inner = statement.walk();
        let Some(expression) = statement.children(&mut inner).skip_while(|c| c.kind() != "=").find(|c| c.is_named())
        else {
            return;
        };
        export_equals(ctx, table, expression, statement_span, out);
    } else if ctx.typescript
        && let Some(alias) = statement.child_by_field_name("declaration").filter(|d| d.kind() == "import_alias")
    {
        export_import(ctx, table, alias, statement_span, scope, out);
    }
}

/// `export = <expression>`.
fn export_equals(
    ctx: &Ctx<'_>,
    table: &Table,
    expression: Node<'_>,
    statement_span: TextSpan,
    out: &mut Out,
) {
    let module_entry = |source: String, local: Option<&str>, import_span: Option<TextSpan>| {
        let mut built = entry(
            ReexportForm::Module,
            ReexportEvidence::Declared,
            text_span_for_node(expression),
            statement_span.clone(),
        );
        built.source = Some(source);
        built.local_name = local.map(str::to_owned);
        built.import_span = import_span;
        built
    };
    match expression.kind() {
        "identifier" => {
            let local = ctx.text(expression);
            if shadowed_below_program(ctx, expression, local) {
                return;
            }
            match table.resolve_es(local) {
                Resolved::Binding(binding) => {
                    let mut built = module_entry(binding.source.clone(), Some(local), binding.import_span.clone());
                    built.type_only = binding.type_only;
                    out.entries.push(built);
                }
                Resolved::Shadowed => out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, expression)),
                Resolved::Rebound => out.limits.push(limit(ReexportLimitReason::ReboundBinding, expression)),
                Resolved::Unbound => {}
            }
        }
        _ => match cjs::require_call(ctx, expression) {
            Some(cjs::Require::Literal(source)) => {
                if cjs::global_shadowed(ctx, table, expression, "require") {
                    out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, expression));
                } else {
                    out.entries.push(module_entry(source, None, None));
                }
            }
            Some(cjs::Require::Dynamic) => out.limits.push(limit(ReexportLimitReason::DynamicForwarding, expression)),
            None if cjs::involves_forwarding(ctx, table, expression) => {
                out.limits.push(limit(ReexportLimitReason::DynamicForwarding, expression));
            }
            None => {}
        },
    }
}

/// `export import A = B.C` reports only when `B` is an import binding.
fn export_import(
    ctx: &Ctx<'_>,
    table: &Table,
    alias: Node<'_>,
    statement_span: TextSpan,
    scope: Option<String>,
    out: &mut Out,
) {
    let mut cursor = alias.walk();
    let children: Vec<Node<'_>> = alias.children(&mut cursor).collect();
    let Some(name) = children.iter().copied().find(|c| c.kind() == "identifier") else { return };
    let Some(target) = children.iter().copied().skip_while(|c| c.kind() != "=").find(|c| c.is_named()) else {
        return;
    };
    let mut leftmost = target;
    while let Some(object) = leftmost.child_by_field_name("object") {
        leftmost = object;
    }
    if leftmost.kind() != "identifier" {
        return;
    }
    let local = ctx.text(leftmost);
    if shadowed_below_program(ctx, alias, local) {
        return;
    }
    let span = text_span_for_node(alias);
    let exported = ctx.text(name).to_owned();
    let binding = match table.resolve_es(local) {
        Resolved::Binding(binding) => binding,
        Resolved::Shadowed => {
            out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, alias));
            return;
        }
        Resolved::Rebound => {
            out.limits.push(limit(ReexportLimitReason::ReboundBinding, alias));
            return;
        }
        Resolved::Unbound => return,
    };
    let mut built = if target.id() == leftmost.id() {
        binding_entry(binding, local, exported, ReexportEvidence::Syntax, span, statement_span)
    } else {
        let path: String = ctx.text(target).split_whitespace().collect();
        let rest = path.get(local.len() + 1..).unwrap_or_default().to_owned();
        let original = match &binding.kind {
            Kind::Named(imported) => format!("{imported}.{rest}"),
            Kind::Whole => rest,
        };
        let mut built = entry(ReexportForm::Named, ReexportEvidence::Syntax, span, statement_span);
        built.exported_name = Some(exported);
        built.original_name = Some(original);
        built.local_name = Some(local.to_owned());
        built.source = Some(binding.source.clone());
        built.import_span = binding.import_span.clone();
        built.type_only = binding.type_only;
        built
    };
    built.scope = scope;
    out.entries.push(built);
}

/// An `export` the grammar accepts where ES modules do not allow one (a
/// block, a function). It forwards nothing, but a name it exports that is
/// really an import hidden by a nearer declaration is worth a limit.
fn stray_export(ctx: &Ctx<'_>, table: &Table, statement: Node<'_>, out: &mut Out) {
    if statement.has_error() || statement.child_by_field_name("source").is_some() {
        return;
    }
    let mut cursor = statement.walk();
    let Some(clause) = statement.children(&mut cursor).find(|c| c.kind() == "export_clause") else { return };
    let mut inner = clause.walk();
    for specifier in clause.children(&mut inner).filter(|c| c.kind() == "export_specifier") {
        let Some(name) = specifier.child_by_field_name("name") else { continue };
        if matches!(table.resolve_es(ctx.text(name)), Resolved::Unbound) {
            continue;
        }
        out.limits.push(limit(ReexportLimitReason::ShadowedGlobal, specifier));
    }
}
