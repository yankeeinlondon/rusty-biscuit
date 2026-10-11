//! Lua re-exports: what the chunk returns.
//!
//! Lua has no forwarding syntax, so only a bounded convention is read from the
//! top level of the chunk: a final `return require("m")` (whole module), a
//! returned table constructor, or a local table `M` whose fields are assigned
//! import-derived values before a final `return M`. Lua has no import
//! records, so entries never carry an `import_span`.
//!
//! Anything dynamic (`module()`, metatables, copy loops, `pcall(require, ..)`
//! results that are returned) limits coverage instead of guessing.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit, ReexportLimitReason,
    ReexportReport,
};

/// A value traced back to a literal `require`.
#[derive(Clone)]
enum Value {
    /// The module itself.
    Module { source: String, local: Option<String> },
    /// One member of the module.
    Member { source: String, field: String, local: Option<String> },
}

/// A table field whose last write decides what the table exposes.
struct TableField<'tree> {
    name: String,
    value: Option<Value>,
    span: Node<'tree>,
    statement: Node<'tree>,
}

enum Binding<'tree> {
    Value(Value),
    Table(Vec<TableField<'tree>>),
    /// Bound from `pcall(require, ..)`: whether the module loaded is dynamic.
    Protected(Node<'tree>),
    Other,
}

struct Scan<'tree, 'src> {
    source: &'src str,
    require_available: bool,
    env: Vec<(String, Binding<'tree>)>,
    /// Identifier targets of plain assignments anywhere in the file, first
    /// occurrence per name.
    reassigned: HashMap<String, Node<'tree>>,
    limits: Vec<ReexportLimit>,
}

pub(super) fn extract(tree_file: &TreeFile, _imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let source = tree_file.source();
    let mut scan = Scan {
        source,
        require_available: true,
        env: Vec::new(),
        reassigned: HashMap::new(),
        limits: Vec::new(),
    };
    scan.collect_reassignments(root);

    let statements: Vec<Node<'_>> = {
        let mut cursor = root.walk();
        root.named_children(&mut cursor).filter(|node| node.kind() != "comment").collect()
    };
    scan.check_require_shadow(root, &statements);

    let mut entries = Vec::new();
    for statement in &statements {
        if statement.kind() == "function_call" && call_name(*statement, source) == Some("module") {
            scan.limits.push(limit(ReexportLimitReason::DynamicForwarding, *statement));
        }
        scan.scan_dynamic(*statement);
        match statement.kind() {
            "variable_declaration" => scan.declare(*statement),
            "assignment_statement" => scan.assign(*statement),
            "function_declaration" => scan.define_function(*statement),
            "return_statement" if statements.last() == Some(statement) => {
                scan.returned(*statement, &mut entries);
            }
            _ => {}
        }
    }

    let mut limits = scan.limits;
    limits.extend(forwarding_damage(root, source));
    finish(entries, limits)
}

fn text<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    node.utf8_text(source.as_bytes()).ok()
}

/// The identifier a call is made through (`require` in `require("m")`).
fn call_name<'a>(call: Node<'_>, source: &'a str) -> Option<&'a str> {
    let name = call.child_by_field_name("name")?;
    if name.kind() == "identifier" { text(name, source) } else { None }
}

fn named_children<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).filter(|child| child.kind() != "comment").collect()
}

/// The single string literal argument of a call.
fn string_argument<'a>(call: Node<'_>, source: &'a str) -> Option<&'a str> {
    let arguments = named_children(call.child_by_field_name("arguments")?);
    let [argument] = arguments.as_slice() else { return None };
    string_value(*argument, source)
}

fn string_value<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    if node.kind() != "string" {
        return None;
    }
    let content = text(node.child_by_field_name("content")?, source)?;
    (!content.is_empty()).then_some(content)
}

/// The names and initializers of a `variable_declaration` or plain
/// `assignment_statement`.
fn targets_and_values<'tree>(assignment: Node<'tree>) -> (Vec<Node<'tree>>, Vec<Node<'tree>>) {
    let mut targets = Vec::new();
    let mut values = Vec::new();
    for child in named_children(assignment) {
        match child.kind() {
            "variable_list" => targets = named_children(child),
            "expression_list" => values = named_children(child),
            _ => {}
        }
    }
    (targets, values)
}

/// The `assignment_statement` of a statement: the statement itself or the
/// one a `local` declaration wraps.
fn assignment_of(statement: Node<'_>) -> Option<Node<'_>> {
    if statement.kind() == "assignment_statement" {
        return Some(statement);
    }
    named_children(statement).into_iter().find(|child| child.kind() == "assignment_statement")
}

impl<'tree> Scan<'tree, '_> {
    fn collect_reassignments(&mut self, node: Node<'tree>) {
        if node.kind() == "assignment_statement" && node.parent().is_some_and(|p| p.kind() != "variable_declaration") {
            let (targets, _) = targets_and_values(node);
            for target in targets.into_iter().filter(|target| target.kind() == "identifier") {
                if let Some(name) = text(target, self.source) {
                    self.reassigned.entry(name.to_owned()).or_insert(node);
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_reassignments(child);
        }
    }

    /// `local require` or `require = ..` at chunk level means `require` is
    /// not the global module loader, so nothing is derived from it.
    fn check_require_shadow(&mut self, root: Node<'tree>, statements: &[Node<'tree>]) {
        let mut shadow = self.reassigned.get("require").copied();
        for statement in statements {
            let shadows = match statement.kind() {
                "variable_declaration" => assignment_of(*statement)
                    .map(|assignment| targets_and_values(assignment).0)
                    .unwrap_or_else(|| named_children(*statement))
                    .iter()
                    .any(|target| target.kind() == "identifier" && text(*target, self.source) == Some("require")),
                "function_declaration" => statement
                    .child_by_field_name("name")
                    .is_some_and(|name| name.kind() == "identifier" && text(name, self.source) == Some("require")),
                _ => false,
            };
            if shadows {
                shadow = shadow.or(Some(*statement));
            }
        }
        if let Some(node) = shadow {
            self.require_available = false;
            if self.calls_require(root) {
                self.limits.push(limit(ReexportLimitReason::ShadowedGlobal, node));
            }
        }
    }

    fn calls_require(&self, node: Node<'tree>) -> bool {
        if node.kind() == "function_call" && call_name(node, self.source) == Some("require") {
            return true;
        }
        let mut cursor = node.walk();
        node.children(&mut cursor).any(|child| self.calls_require(child))
    }

    /// The value an expression carries when it comes from a literal `require`.
    fn eval(&mut self, node: Node<'tree>) -> Option<Value> {
        match node.kind() {
            "parenthesized_expression" => {
                let inner = named_children(node);
                let [inner] = inner.as_slice() else { return None };
                self.eval(*inner)
            }
            "function_call" => {
                if !self.require_available || call_name(node, self.source) != Some("require") {
                    return None;
                }
                let source = string_argument(node, self.source)?;
                Some(Value::Module { source: source.to_owned(), local: None })
            }
            "identifier" => self.lookup(node),
            "dot_index_expression" | "bracket_index_expression" => {
                let field = node.child_by_field_name("field")?;
                let field = if node.kind() == "dot_index_expression" {
                    text(field, self.source)?
                } else {
                    string_value(field, self.source)?
                };
                let table = node.child_by_field_name("table")?;
                match self.eval(table)? {
                    Value::Module { source, local } => {
                        Some(Value::Member { source, field: field.to_owned(), local })
                    }
                    Value::Member { .. } => None,
                }
            }
            _ => None,
        }
    }

    fn lookup(&mut self, identifier: Node<'tree>) -> Option<Value> {
        let name = text(identifier, self.source)?;
        let binding = self.env.iter().rev().find(|(bound, _)| bound == name).map(|(_, binding)| binding)?;
        match binding {
            Binding::Value(value) => {
                let mut value = value.clone();
                if let Some(assigned) = self.reassigned.get(name) {
                    self.limits.push(limit(ReexportLimitReason::ReboundBinding, *assigned));
                    return None;
                }
                match &mut value {
                    Value::Module { local, .. } | Value::Member { local, .. } => *local = Some(name.to_owned()),
                }
                Some(value)
            }
            Binding::Protected(call) => {
                self.limits.push(limit(ReexportLimitReason::DynamicForwarding, *call));
                None
            }
            Binding::Table(_) | Binding::Other => None,
        }
    }

    fn declare(&mut self, statement: Node<'tree>) {
        let Some(assignment) = assignment_of(statement) else {
            for name in named_children(statement) {
                if let Some(name) = text(name, self.source) {
                    self.env.push((name.to_owned(), Binding::Other));
                }
            }
            return;
        };
        let (targets, values) = targets_and_values(assignment);
        let mut bindings = Vec::new();
        for (index, target) in targets.iter().enumerate() {
            let Some(name) = text(*target, self.source) else { continue };
            let value = values.get(index).copied();
            let binding = match value {
                Some(value) if value.kind() == "table_constructor" => {
                    Binding::Table(self.constructor_fields(value, statement))
                }
                Some(value) if self.is_protected_require(value) => Binding::Protected(value),
                Some(value) => self.eval(value).map_or(Binding::Other, Binding::Value),
                None if values.len() == 1 && self.is_protected_require(values[0]) && index > 0 => {
                    Binding::Protected(values[0])
                }
                None => Binding::Other,
            };
            bindings.push((name.to_owned(), binding));
        }
        self.env.extend(bindings);
    }

    fn is_protected_require(&self, value: Node<'tree>) -> bool {
        if value.kind() != "function_call" || call_name(value, self.source) != Some("pcall") {
            return false;
        }
        let Some(arguments) = value.child_by_field_name("arguments") else { return false };
        let arguments = named_children(arguments);
        arguments.first().is_some_and(|first| first.kind() == "identifier" && text(*first, self.source) == Some("require"))
    }

    /// The named fields of a table constructor, evaluated in order.
    fn constructor_fields(&mut self, constructor: Node<'tree>, statement: Node<'tree>) -> Vec<TableField<'tree>> {
        let mut fields: Vec<TableField<'tree>> = Vec::new();
        for field in named_children(constructor).into_iter().filter(|child| child.kind() == "field") {
            let (Some(name), Some(value)) = (field.child_by_field_name("name"), field.child_by_field_name("value"))
            else {
                continue;
            };
            let Some(name) = text(name, self.source) else { continue };
            let value = self.eval(value);
            upsert(&mut fields, TableField { name: name.to_owned(), value, span: field, statement });
        }
        fields
    }

    fn table_mut(&mut self, name: &str) -> Option<&mut Vec<TableField<'tree>>> {
        self.env.iter_mut().rev().find(|(bound, _)| bound == name).and_then(|(_, binding)| match binding {
            Binding::Table(fields) => Some(fields),
            _ => None,
        })
    }

    /// `M.x = v` and `M["x"] = v` at chunk level; the last write to a field
    /// decides what it holds.
    fn assign(&mut self, statement: Node<'tree>) {
        let (targets, values) = targets_and_values(statement);
        for (index, target) in targets.into_iter().enumerate() {
            let Some((table, field)) = self.field_target(target) else { continue };
            let value = match values.get(index) {
                Some(value) => self.eval(*value),
                None => None,
            };
            if let Some(fields) = self.table_mut(&table) {
                upsert(fields, TableField { name: field, value, span: statement, statement });
            }
        }
    }

    /// `function M.f() end` overwrites field `f` with a new function.
    fn define_function(&mut self, statement: Node<'tree>) {
        let Some(name) = statement.child_by_field_name("name") else { return };
        let Some((table, field)) = self.field_target(name) else { return };
        if let Some(fields) = self.table_mut(&table) {
            upsert(fields, TableField { name: field, value: None, span: statement, statement });
        }
    }

    /// The table identifier and literal field name an assignment target writes.
    fn field_target(&self, target: Node<'tree>) -> Option<(String, String)> {
        let table = target.child_by_field_name("table")?;
        if table.kind() != "identifier" {
            return None;
        }
        let field = match target.kind() {
            "dot_index_expression" => text(target.child_by_field_name("field")?, self.source)?,
            "method_index_expression" => text(target.child_by_field_name("method")?, self.source)?,
            "bracket_index_expression" => string_value(target.child_by_field_name("field")?, self.source)?,
            _ => return None,
        };
        Some((text(table, self.source)?.to_owned(), field.to_owned()))
    }

    /// Metatable and copy-loop forwarding in a chunk-level statement.
    fn scan_dynamic(&mut self, node: Node<'tree>) {
        match node.kind() {
            "function_definition" | "function_declaration" => return,
            "function_call" if call_name(node, self.source) == Some("setmetatable") => {
                if let Some(arguments) = node.child_by_field_name("arguments") {
                    for constructor in named_children(arguments).into_iter().filter(|a| a.kind() == "table_constructor") {
                        for field in named_children(constructor) {
                            let (Some(name), Some(value)) =
                                (field.child_by_field_name("name"), field.child_by_field_name("value"))
                            else {
                                continue;
                            };
                            if text(name, self.source) == Some("__index") && self.eval(value).is_some() {
                                self.limits.push(limit(ReexportLimitReason::DynamicForwarding, node));
                            }
                        }
                    }
                }
            }
            "for_statement" if self.copies_import(node) => {
                self.limits.push(limit(ReexportLimitReason::DynamicForwarding, node));
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.scan_dynamic(child);
        }
    }

    /// `for k, v in pairs(<import>)`.
    fn copies_import(&mut self, statement: Node<'tree>) -> bool {
        let Some(clause) = statement.child_by_field_name("clause") else { return false };
        if clause.kind() != "for_generic_clause" {
            return false;
        }
        for list in named_children(clause).into_iter().filter(|child| child.kind() == "expression_list") {
            for call in named_children(list) {
                let iterator = call_name(call, self.source);
                if call.kind() != "function_call" || !matches!(iterator, Some("pairs" | "ipairs" | "next")) {
                    continue;
                }
                let Some(arguments) = call.child_by_field_name("arguments") else { continue };
                if let Some(first) = named_children(arguments).first()
                    && self.eval(*first).is_some()
                {
                    return true;
                }
            }
        }
        false
    }

    /// The chunk's final `return`.
    fn returned(&mut self, statement: Node<'tree>, entries: &mut Vec<ReexportEntry>) {
        let values: Vec<Node<'tree>> =
            named_children(statement).into_iter().flat_map(|list| named_children(list)).collect();
        match values.as_slice() {
            [] => {}
            [single] => self.returned_single(*single, statement, entries),
            many => {
                // `require` hands the caller only the first value; the rest
                // are not an interface this scan can state.
                let derived = many.iter().filter(|value| self.eval(**value).is_some()).count();
                if derived > 0 {
                    self.limits.push(limit(ReexportLimitReason::DynamicForwarding, statement));
                }
            }
        }
    }

    fn returned_single(&mut self, value: Node<'tree>, statement: Node<'tree>, entries: &mut Vec<ReexportEntry>) {
        let mut value = value;
        while value.kind() == "parenthesized_expression" {
            let inner = named_children(value);
            let [inner] = inner.as_slice() else { return };
            value = *inner;
        }
        if value.kind() == "table_constructor" {
            let fields = self.constructor_fields(value, statement);
            push_fields(&fields, statement, entries);
            return;
        }
        if value.kind() == "identifier" {
            let name = text(value, self.source).unwrap_or_default();
            let is_table = self.env.iter().rev().find(|(bound, _)| bound == name).is_some_and(|(_, b)| matches!(b, Binding::Table(_)));
            if is_table {
                if let Some(assigned) = self.reassigned.get(name) {
                    self.limits.push(limit(ReexportLimitReason::ReboundBinding, *assigned));
                    return;
                }
                if let Some(fields) = self.table_mut(name) {
                    let fields: Vec<(Node<'tree>, Node<'tree>, String, Option<Value>)> =
                        fields.iter().map(|f| (f.span, f.statement, f.name.clone(), f.value.clone())).collect();
                    for (span, field_statement, name, field_value) in fields {
                        push_field(&name, field_value.as_ref(), span, field_statement, entries);
                    }
                }
                return;
            }
        }
        if let Some(Value::Module { source, local }) = self.eval(value) {
            let mut found = entry(
                ReexportForm::Module,
                ReexportEvidence::Declared,
                text_span_for_node(value),
                text_span_for_node(statement),
            );
            found.source = Some(source);
            found.local_name = local;
            entries.push(found);
        }
    }
}

/// Replaces an earlier write to the same field: the last write wins.
fn upsert<'tree>(fields: &mut Vec<TableField<'tree>>, field: TableField<'tree>) {
    fields.retain(|existing| existing.name != field.name);
    fields.push(field);
}

fn push_fields(fields: &[TableField<'_>], _statement: Node<'_>, entries: &mut Vec<ReexportEntry>) {
    for field in fields {
        push_field(&field.name, field.value.as_ref(), field.span, field.statement, entries);
    }
}

fn push_field(
    name: &str,
    value: Option<&Value>,
    span: Node<'_>,
    statement: Node<'_>,
    entries: &mut Vec<ReexportEntry>,
) {
    let Some(value) = value else { return };
    let (form, source, original, local) = match value {
        Value::Member { source, field, local } => (ReexportForm::Named, source, Some(field.clone()), local),
        Value::Module { source, local } => (ReexportForm::Namespace, source, None, local),
    };
    let mut found = entry(form, ReexportEvidence::Declared, text_span_for_node(span), text_span_for_node(statement));
    found.exported_name = Some(name.to_owned());
    found.original_name = original;
    found.local_name = local.clone();
    found.source = Some(source.clone());
    entries.push(found);
}

/// `parse_error` limits for damage that contains or directly follows a
/// chunk-final `return` or a `require`.
fn forwarding_damage(root: Node<'_>, source: &str) -> Vec<ReexportLimit> {
    damaged_nodes(root)
        .into_iter()
        .filter(|node| !inside_function(*node))
        .filter(|node| contains_trigger(*node, source) || in_chunk_return(*node) || previous_leaf(*node).is_some_and(|leaf| is_trigger(leaf, source)))
        .map(|node| limit(ReexportLimitReason::ParseError, node))
        .collect()
}

fn is_trigger(leaf: Node<'_>, source: &str) -> bool {
    leaf.kind() == "return" || (leaf.kind() == "identifier" && text(leaf, source) == Some("require"))
}

/// Damage nested in the chunk's own `return` statement.
fn in_chunk_return(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind() == "return_statement" {
            return ancestor.parent().is_some_and(|p| p.kind() == "chunk");
        }
        parent = ancestor.parent();
    }
    false
}

fn inside_function(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(ancestor.kind(), "function_definition" | "function_declaration") {
            return true;
        }
        parent = ancestor.parent();
    }
    false
}

fn contains_trigger(node: Node<'_>, source: &str) -> bool {
    if node.child_count() == 0 {
        return is_trigger(node, source);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| contains_trigger(child, source))
}

/// The last leaf token that ends before `node` starts.
fn previous_leaf(node: Node<'_>) -> Option<Node<'_>> {
    let mut current = node;
    loop {
        if let Some(mut sibling) = current.prev_sibling() {
            while sibling.child_count() > 0 {
                sibling = sibling.child(u32::try_from(sibling.child_count() - 1).ok()?)?;
            }
            return Some(sibling);
        }
        current = current.parent()?;
    }
}
