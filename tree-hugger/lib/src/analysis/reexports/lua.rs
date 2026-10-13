//! Lua re-exports: what the chunk returns.
//!
//! Lua has no forwarding syntax, so only a bounded convention is read from the
//! top level of the chunk: a final `return require("m")` (whole module), a
//! returned table constructor, or a local table `M` whose fields are assigned
//! import-derived values before a final `return M`. Lua has no import
//! records, so entries never carry an `import_span`.
//!
//! Anything dynamic (`module()`, metatables, copy loops, `pcall(require, ..)`
//! results that are returned) limits coverage instead of guessing. So does a
//! use of the returned `M` that its chunk-level field writes do not describe:
//! a nested write, a write through `M or t`, a write or constructor field
//! under a key whose string value is unknown ([`Key`]), or handing `M` to
//! other code ([`Scan::table_uses`]). Parentheses are transparent everywhere
//! ([`unparenthesized`]).
//!
//! Every name is tied to the declaration it resolves to at its own site
//! ([`resolve`]), never to its spelling: a write to, or a read of, a nested
//! local, parameter or loop variable of the same name says nothing about the
//! chunk-level binding.

use std::collections::{HashMap, HashSet};

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
    /// Which of two writes in one statement wins is undefined, and one of
    /// them is import-derived: the field is withheld and limits a report
    /// that returns its table, unless a later write replaces it.
    undecided: bool,
}

/// The named fields of a table, or `Err` with the first constructor field
/// whose [`Key::Unknown`] key may equal any of them, so none can be stated.
type TableFields<'tree> = Result<Vec<TableField<'tree>>, Node<'tree>>;

enum Binding<'tree> {
    Value(Value),
    Table(TableFields<'tree>),
    /// Bound from `pcall(require, ..)`: whether the module loaded is dynamic.
    Protected(Node<'tree>),
    Other,
}

/// Where a name is bound at one site.
#[derive(Clone, Copy)]
enum Resolved<'tree> {
    /// The chunk-level declaration statement (`local`, `local function`).
    Chunk(Node<'tree>),
    /// A local of a nested block or function, a parameter or a loop variable.
    Nested,
    /// No declaration is in scope: the global of that name.
    Global,
}

/// A chunk-level binding: its name, the statement declaring it, and what it
/// holds.
struct Bound<'tree> {
    name: String,
    declaration: Node<'tree>,
    binding: Binding<'tree>,
}

struct Scan<'tree, 'src> {
    source: &'src str,
    require_available: bool,
    env: Vec<Bound<'tree>>,
    /// The first write anywhere in the file to each chunk-level binding,
    /// keyed by declaration and name.
    reassigned: HashMap<(usize, String), Node<'tree>>,
    /// The first write anywhere in the file to each global.
    global_writes: HashMap<String, Node<'tree>>,
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
        global_writes: HashMap::new(),
        limits: Vec::new(),
    };
    scan.collect_writes(root);

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

/// `node` without the parentheses around it: `((M))` is `M`. Every reader of
/// a table, callee, argument or key goes through this one normalization, so a
/// spelling the use classifier treats as the table is one the field
/// collector folds too.
fn unparenthesized(node: Node<'_>) -> Node<'_> {
    let mut node = node;
    while node.kind() == "parenthesized_expression" {
        let inner = named_children(node);
        let [inner] = inner.as_slice() else { break };
        node = *inner;
    }
    node
}

/// The identifier a call is made through (`require` in `require("m")` and
/// `(require)("m")`).
fn callee(call: Node<'_>) -> Option<Node<'_>> {
    let name = unparenthesized(call.child_by_field_name("name")?);
    (name.kind() == "identifier").then_some(name)
}

/// The name of [`callee`].
fn call_name<'a>(call: Node<'_>, source: &'a str) -> Option<&'a str> {
    text(callee(call)?, source)
}

fn named_children<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).filter(|child| child.kind() != "comment").collect()
}

/// The single string literal argument of a call.
fn string_argument<'a>(call: Node<'_>, source: &'a str) -> Option<&'a str> {
    let arguments = named_children(call.child_by_field_name("arguments")?);
    let [argument] = arguments.as_slice() else { return None };
    string_value(unparenthesized(*argument), source)
}

fn string_value<'a>(node: Node<'_>, source: &'a str) -> Option<&'a str> {
    if node.kind() != "string" {
        return None;
    }
    let content = text(node.child_by_field_name("content")?, source)?;
    (!content.is_empty()).then_some(content)
}

/// What a table key names. One interpretation serves constructor fields
/// (`{x = v}`, `{["x"] = v}`), assignment targets (`M.x`, `M["x"]`,
/// `function M:x()`) and member reads (`m["x"]`), so equal keys share one
/// field identity whichever way they are written.
enum Key {
    /// A string key: an identifier, or a string literal read verbatim.
    Name(String),
    /// A number or boolean key. It never equals a string key, so it cannot
    /// replace a named field.
    NonString,
    /// A key whose string value this scan does not know: a variable, any
    /// other expression, or a literal it does not decode (see
    /// [`literal_key`]). It may equal any named field.
    Unknown,
}

/// What `key` names. `bracketed` is whether it is written `[key]`; otherwise
/// it is a bare name (`x` in `{x = v}`, `M.x`, `M:x`).
fn table_key(key: Node<'_>, bracketed: bool, source: &str) -> Key {
    if !bracketed {
        return text(key, source).map_or(Key::Unknown, |name| Key::Name(name.to_owned()));
    }
    match key.kind() {
        "string" => literal_key(key, source).map_or(Key::Unknown, |name| Key::Name(name.to_owned())),
        "number" | "true" | "false" => Key::NonString,
        _ => Key::Unknown,
    }
}

/// The value of a string literal whose source spelling is its value. An
/// escape sequence (`"a\"b"`) or the newline a long bracket drops after its
/// opening (`[[⏎x]]`) makes the spelling differ, and the empty string is not
/// a usable name, so those are declined rather than decoded.
fn literal_key<'a>(string: Node<'_>, source: &'a str) -> Option<&'a str> {
    let content = string.child_by_field_name("content")?;
    if content.named_child_count() > 0 {
        return None;
    }
    let value = text(content, source)?;
    let long_bracket = string.child_by_field_name("start").is_some_and(|start| start.kind().starts_with('['));
    let drops_newline = long_bracket && value.starts_with(['\n', '\r']);
    (!value.is_empty() && !drops_newline).then_some(value)
}

/// The key of a constructor `field`; `None` for a positional field.
fn field_key(field: Node<'_>, source: &str) -> Option<Key> {
    let name = field.child_by_field_name("name")?;
    Some(table_key(name, has_child(field, "["), source))
}

/// The key an index expression (`M.x`, `M:x`, `M[k]`) reads or writes.
fn index_key(index: Node<'_>, source: &str) -> Option<Key> {
    let (field, bracketed) = match index.kind() {
        "dot_index_expression" => (index.child_by_field_name("field")?, false),
        "method_index_expression" => (index.child_by_field_name("method")?, false),
        "bracket_index_expression" => (index.child_by_field_name("field")?, true),
        _ => return None,
    };
    Some(table_key(field, bracketed, source))
}

/// The identifier naming the table an index expression reads or writes:
/// `M` in `M.x`, `(M)["x"]` and `((M)):x`. `None` for any other table
/// expression, such as `(M or t).x`.
fn table_identifier(index: Node<'_>) -> Option<Node<'_>> {
    let table = unparenthesized(index.child_by_field_name("table")?);
    (table.kind() == "identifier").then_some(table)
}

/// The targets of a `variable_list`, without `<const>`/`<close>` attributes.
fn list_names(list: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = list.walk();
    list.children_by_field_name("name", &mut cursor).collect()
}

/// The names and initializers of a `variable_declaration` or plain
/// `assignment_statement`.
fn targets_and_values<'tree>(assignment: Node<'tree>) -> (Vec<Node<'tree>>, Vec<Node<'tree>>) {
    let mut targets = Vec::new();
    let mut values = Vec::new();
    for child in named_children(assignment) {
        match child.kind() {
            "variable_list" => targets = list_names(child),
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

/// The leading `local`/`global`/`function` keyword of a declaration.
fn keyword(statement: Node<'_>) -> Option<&'static str> {
    statement.child(0).map(|first| first.kind())
}

/// Whether `statement` declares `name`, and if so whether as a `global`.
fn declares(statement: Node<'_>, name: &str, source: &str) -> Option<bool> {
    let names = match (statement.kind(), keyword(statement)) {
        ("variable_declaration", _) => named_children(statement)
            .into_iter()
            .flat_map(|child| match child.kind() {
                "variable_list" => list_names(child),
                "assignment_statement" => targets_and_values(child).0,
                _ => Vec::new(),
            })
            .collect(),
        ("function_declaration", Some("local" | "global")) => statement.child_by_field_name("name").into_iter().collect(),
        _ => Vec::new(),
    };
    names
        .iter()
        .any(|declared| declared.kind() == "identifier" && text(*declared, source) == Some(name))
        .then(|| keyword(statement) == Some("global"))
}

/// The declaration `identifier` refers to at its own site, following Lua's
/// lexical scoping: a `local` is visible after its statement to the end of
/// its block, a `local function` also in its own body, parameters and loop
/// variables in their body, and a `repeat` body's locals in its `until`.
/// A nested `global x` declaration makes `x` the global inside its block.
fn resolve<'tree>(identifier: Node<'tree>, source: &str) -> Resolved<'tree> {
    let Some(name) = text(identifier, source) else { return Resolved::Nested };
    let mut child = identifier;
    while let Some(parent) = child.parent() {
        let found = match parent.kind() {
            "chunk" | "block" => named_children(parent)
                .into_iter()
                .rev()
                .filter(|statement| statement.end_byte() <= child.start_byte())
                .find_map(|statement| declares(statement, name, source).map(|global| (statement, global))),
            "repeat_statement" if parent.child_by_field_name("condition") == Some(child) => parent
                .child_by_field_name("body")
                .into_iter()
                .flat_map(named_children)
                .rev()
                .find_map(|statement| declares(statement, name, source).map(|global| (statement, global))),
            "function_declaration" | "function_definition" if parent.child_by_field_name("body") == Some(child) => {
                let parameters = parent.child_by_field_name("parameters");
                let mut cursor = parent.walk();
                let is_parameter = parameters.is_some_and(|parameters| {
                    parameters
                        .children_by_field_name("name", &mut cursor)
                        .any(|parameter| text(parameter, source) == Some(name))
                });
                if is_parameter {
                    return Resolved::Nested;
                }
                declares(parent, name, source).map(|global| (parent, global))
            }
            "for_statement" if parent.child_by_field_name("body") == Some(child) => {
                let clause = parent.child_by_field_name("clause");
                let variables = match clause {
                    Some(clause) if clause.kind() == "for_numeric_clause" => {
                        clause.child_by_field_name("name").into_iter().collect()
                    }
                    Some(clause) => named_children(clause)
                        .into_iter()
                        .filter(|list| list.kind() == "variable_list")
                        .flat_map(list_names)
                        .collect(),
                    None => Vec::new(),
                };
                if variables.iter().any(|variable| text(*variable, source) == Some(name)) {
                    return Resolved::Nested;
                }
                None
            }
            _ => None,
        };
        if let Some((declaration, global)) = found {
            let at_chunk = declaration.parent().is_some_and(|p| p.kind() == "chunk");
            return match (at_chunk, global) {
                (true, _) => Resolved::Chunk(declaration),
                (false, true) => Resolved::Global,
                (false, false) => Resolved::Nested,
            };
        }
        child = parent;
    }
    Resolved::Global
}

impl<'tree> Scan<'tree, '_> {
    /// Plain assignments and `function x() end` anywhere in the file, each
    /// recorded against the binding its target resolves to.
    fn collect_writes(&mut self, node: Node<'tree>) {
        let targets = match node.kind() {
            "assignment_statement" if node.parent().is_some_and(|p| p.kind() != "variable_declaration") => {
                targets_and_values(node).0
            }
            "function_declaration" if keyword(node) == Some("function") => {
                node.child_by_field_name("name").into_iter().collect()
            }
            _ => Vec::new(),
        };
        for target in targets.into_iter().filter(|target| target.kind() == "identifier") {
            let Some(name) = text(target, self.source) else { continue };
            match resolve(target, self.source) {
                Resolved::Chunk(declaration) => {
                    self.reassigned.entry((declaration.id(), name.to_owned())).or_insert(node);
                }
                Resolved::Global => {
                    self.global_writes.entry(name.to_owned()).or_insert(node);
                }
                Resolved::Nested => {}
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_writes(child);
        }
    }

    /// The chunk-level binding `identifier` resolves to at its site.
    fn bound(&self, identifier: Node<'tree>) -> Option<&Bound<'tree>> {
        let Resolved::Chunk(declaration) = resolve(identifier, self.source) else { return None };
        let name = text(identifier, self.source)?;
        self.env.iter().rev().find(|bound| bound.declaration == declaration && bound.name == name)
    }

    /// The first write to the chunk-level binding `identifier` resolves to.
    fn rebinding(&self, identifier: Node<'tree>) -> Option<Node<'tree>> {
        let bound = self.bound(identifier)?;
        self.reassigned.get(&(bound.declaration.id(), bound.name.clone())).copied()
    }

    /// `local require` at chunk level, or a write to the global `require`
    /// anywhere it is not shadowed, means `require` is not the global module
    /// loader, so nothing is derived from it.
    fn check_require_shadow(&mut self, root: Node<'tree>, statements: &[Node<'tree>]) {
        let mut shadow = self.global_writes.get("require").copied();
        for statement in statements {
            if declares(*statement, "require", self.source).is_some() {
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
                let inner = unparenthesized(node);
                if inner == node { None } else { self.eval(inner) }
            }
            "function_call" => {
                if !self.require_available || call_name(node, self.source) != Some("require") {
                    return None;
                }
                let loader = callee(node)?;
                if !matches!(resolve(loader, self.source), Resolved::Global) {
                    return None;
                }
                let source = string_argument(node, self.source)?;
                Some(Value::Module { source: source.to_owned(), local: None })
            }
            "identifier" => self.lookup(node),
            "dot_index_expression" | "bracket_index_expression" => {
                let Key::Name(field) = index_key(node, self.source)? else { return None };
                let table = node.child_by_field_name("table")?;
                match self.eval(table)? {
                    Value::Module { source, local } => {
                        Some(Value::Member { source, field, local })
                    }
                    Value::Member { .. } => None,
                }
            }
            _ => None,
        }
    }

    fn lookup(&mut self, identifier: Node<'tree>) -> Option<Value> {
        let name = text(identifier, self.source)?;
        let assigned = self.rebinding(identifier);
        match &self.bound(identifier)?.binding {
            Binding::Value(value) => {
                let mut value = value.clone();
                if let Some(assigned) = assigned {
                    self.limits.push(limit(ReexportLimitReason::ReboundBinding, assigned));
                    return None;
                }
                match &mut value {
                    Value::Module { local, .. } | Value::Member { local, .. } => *local = Some(name.to_owned()),
                }
                Some(value)
            }
            Binding::Protected(call) => {
                let call = *call;
                self.limits.push(limit(ReexportLimitReason::DynamicForwarding, call));
                None
            }
            Binding::Table(_) | Binding::Other => None,
        }
    }

    fn declare(&mut self, statement: Node<'tree>) {
        let Some(assignment) = assignment_of(statement) else {
            for list in named_children(statement) {
                for name in list_names(list) {
                    if let Some(name) = text(name, self.source) {
                        self.env.push(Bound { name: name.to_owned(), declaration: statement, binding: Binding::Other });
                    }
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
                Some(value) if unparenthesized(value).kind() == "table_constructor" => {
                    Binding::Table(self.constructor_fields(unparenthesized(value), statement))
                }
                Some(value) if self.is_protected_require(value) => Binding::Protected(value),
                Some(value) => self.eval(value).map_or(Binding::Other, Binding::Value),
                None if values.len() == 1 && self.is_protected_require(values[0]) && index > 0 => {
                    Binding::Protected(values[0])
                }
                None => Binding::Other,
            };
            bindings.push(Bound { name: name.to_owned(), declaration: statement, binding });
        }
        self.env.extend(bindings);
    }

    fn is_protected_require(&self, value: Node<'tree>) -> bool {
        if value.kind() != "function_call" || call_name(value, self.source) != Some("pcall") {
            return false;
        }
        let Some(arguments) = value.child_by_field_name("arguments") else { return false };
        let arguments = named_children(arguments);
        arguments.first().map(|first| unparenthesized(*first)).is_some_and(|first| {
            first.kind() == "identifier" && text(first, self.source) == Some("require")
        })
    }

    /// The named fields of a table constructor. Positional, number and
    /// boolean keys never name a string field and are skipped.
    ///
    /// Lua leaves the order of a constructor's assignments undefined when a
    /// key repeats, so a repeated key where either value is import-derived
    /// fails the whole constructor, as an unknown key does.
    fn constructor_fields(&mut self, constructor: Node<'tree>, statement: Node<'tree>) -> TableFields<'tree> {
        let mut fields: Vec<TableField<'tree>> = Vec::new();
        for field in named_children(constructor).into_iter().filter(|child| child.kind() == "field") {
            let (Some(key), Some(value)) = (field_key(field, self.source), field.child_by_field_name("value")) else {
                continue;
            };
            let name = match key {
                Key::Name(name) => name,
                Key::NonString => continue,
                Key::Unknown => return Err(field),
            };
            let value = self.eval(value);
            let earlier = fields.iter().find(|existing| existing.name == name);
            if earlier.is_some_and(|earlier| earlier.value.is_some() || value.is_some()) {
                return Err(field);
            }
            upsert(&mut fields, TableField { name, value, span: field, statement, undecided: false });
        }
        Ok(fields)
    }

    /// The fields of the chunk-level table `identifier` resolves to.
    fn table_mut(&mut self, identifier: Node<'tree>) -> Option<&mut Vec<TableField<'tree>>> {
        let Resolved::Chunk(declaration) = resolve(identifier, self.source) else { return None };
        let name = text(identifier, self.source)?;
        self.env.iter_mut().rev().find(|bound| bound.declaration == declaration && bound.name == name).and_then(
            |bound| match &mut bound.binding {
                Binding::Table(Ok(fields)) => Some(fields),
                _ => None,
            },
        )
    }

    /// `M.x = v` and `M["x"] = v` at chunk level; the last write to a field
    /// decides what it holds.
    ///
    /// Lua leaves the order of one statement's assignments undefined, so a
    /// field written twice in it (`M.x, M.x = m.x, 1`) where either value is
    /// import-derived is [`TableField::undecided`], as a repeated constructor
    /// key fails the constructor.
    fn assign(&mut self, statement: Node<'tree>) {
        let (targets, values) = targets_and_values(statement);
        let mut written: Vec<(Node<'tree>, String, Option<Value>, bool)> = Vec::new();
        for (index, target) in targets.into_iter().enumerate() {
            let Some((table, field)) = self.field_target(target) else { continue };
            let value = match values.get(index) {
                Some(value) => self.eval(*value),
                None => None,
            };
            let same = |(other, name, _, _): &(Node<'tree>, String, Option<Value>, bool)| {
                *name == field && text(*other, self.source) == text(table, self.source)
            };
            if let Some(earlier) = written.iter_mut().find(|earlier| same(earlier)) {
                earlier.3 |= earlier.2.is_some() || value.is_some();
                earlier.2 = None;
                continue;
            }
            written.push((table, field, value, false));
        }
        for (table, field, value, undecided) in written {
            if let Some(fields) = self.table_mut(table) {
                upsert(fields, TableField { name: field, value, span: statement, statement, undecided });
            }
        }
    }

    /// `function M.f() end` overwrites field `f` with a new function.
    fn define_function(&mut self, statement: Node<'tree>) {
        let Some(name) = statement.child_by_field_name("name") else { return };
        let Some((table, field)) = self.field_target(name) else { return };
        if let Some(fields) = self.table_mut(table) {
            upsert(fields, TableField { name: field, value: None, span: statement, statement, undecided: false });
        }
    }

    /// The table identifier and named field an assignment target writes.
    /// Other keys and tables are left to [`Scan::table_uses`].
    fn field_target(&self, target: Node<'tree>) -> Option<(Node<'tree>, String)> {
        let table = table_identifier(target)?;
        let Key::Name(field) = index_key(target, self.source)? else { return None };
        Some((table, field))
    }

    /// Metatable and copy-loop forwarding in a chunk-level statement.
    fn scan_dynamic(&mut self, node: Node<'tree>) {
        match node.kind() {
            "function_definition" | "function_declaration" => return,
            "function_call" if call_name(node, self.source) == Some("setmetatable") => {
                if let Some(arguments) = node.child_by_field_name("arguments") {
                    let constructors = named_children(arguments).into_iter().map(unparenthesized);
                    for constructor in constructors.filter(|a| a.kind() == "table_constructor") {
                        for field in named_children(constructor) {
                            let (Some(key), Some(value)) =
                                (field_key(field, self.source), field.child_by_field_name("value"))
                            else {
                                continue;
                            };
                            let may_be_index = match key {
                                Key::Name(name) => name == "__index",
                                Key::Unknown => true,
                                Key::NonString => false,
                            };
                            if may_be_index && self.eval(value).is_some() {
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
        let value = unparenthesized(value);
        if value.kind() == "table_constructor" {
            match self.constructor_fields(value, statement) {
                Ok(fields) => push_fields(&fields, statement, entries),
                Err(field) => self.limits.push(limit(ReexportLimitReason::ReboundBinding, field)),
            }
            return;
        }
        if value.kind() == "identifier" {
            let is_table = self.bound(value).is_some_and(|bound| matches!(bound.binding, Binding::Table(_)));
            if is_table {
                if let Some(assigned) = self.rebinding(value) {
                    self.limits.push(limit(ReexportLimitReason::ReboundBinding, assigned));
                    return;
                }
                let uses = self.table_uses(value, statement);
                self.limits.extend(uses.limits);
                if let Some(Binding::Table(Err(field))) = self.bound(value).map(|bound| &bound.binding) {
                    let field = *field;
                    self.limits.push(limit(ReexportLimitReason::ReboundBinding, field));
                    return;
                }
                if uses.all {
                    return;
                }
                if let Some(fields) = self.table_mut(value) {
                    let fields: Vec<(Node<'tree>, Node<'tree>, String, Option<Value>, bool)> = fields
                        .iter()
                        .filter(|f| !uses.fields.contains(&f.name))
                        .map(|f| (f.span, f.statement, f.name.clone(), f.value.clone(), f.undecided))
                        .collect();
                    for (span, field_statement, name, field_value, undecided) in fields {
                        if undecided {
                            self.limits.push(limit(ReexportLimitReason::ReboundBinding, field_statement));
                            continue;
                        }
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

impl<'tree> Scan<'tree, '_> {
    /// The uses of the returned table `identifier` that the chunk-level field
    /// writes do not account for. Every reference resolving to the same
    /// declaration is classified; `final_return` is the chunk's last
    /// statement, so no use follows it.
    fn table_uses(&self, identifier: Node<'tree>, final_return: Node<'tree>) -> TableUses {
        let mut uses = TableUses::default();
        let (Resolved::Chunk(declaration), Some(name), Some(root)) =
            (resolve(identifier, self.source), text(identifier, self.source), final_return.parent())
        else {
            return uses;
        };
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
            if node.kind() != "identifier"
                || text(node, self.source) != Some(name)
                || !matches!(resolve(node, self.source), Resolved::Chunk(found) if found == declaration)
            {
                continue;
            }
            match self.table_use(node, final_return) {
                Some(TableUse::Write { field: Some(field), statement }) => {
                    uses.limits.push(limit(ReexportLimitReason::ReboundBinding, statement));
                    uses.fields.insert(field);
                }
                Some(TableUse::Write { field: None, statement }) => {
                    uses.limits.push(limit(ReexportLimitReason::ReboundBinding, statement));
                    uses.all = true;
                }
                Some(TableUse::Escape(site)) => {
                    uses.limits.push(limit(ReexportLimitReason::DynamicForwarding, site));
                    uses.all = true;
                }
                None => {}
            }
        }
        uses
    }

    /// What one reference to the returned table does with it. `None` for a
    /// read, a chunk-level write of a named key through the table itself or
    /// its parenthesized spelling (already folded into the fields), a write
    /// under a number or boolean key, the final `return`,
    /// and the table's own declaration or plain reassignment (which
    /// `rebinding` reports).
    fn table_use(&self, identifier: Node<'tree>, final_return: Node<'tree>) -> Option<TableUse<'tree>> {
        let mut node = identifier;
        let mut parent = node.parent()?;
        // `(M)` is `M`, and `M or x` may evaluate to `M`.
        while parent.kind() == "parenthesized_expression"
            || (parent.kind() == "binary_expression" && (has_child(parent, "and") || has_child(parent, "or")))
        {
            node = parent;
            parent = node.parent()?;
        }
        let is_field = |name: &str| parent.child_by_field_name(name) == Some(node);
        match parent.kind() {
            "variable_list" | "parameters" | "attribute" | "goto_statement" | "label_statement"
            | "binary_expression" | "unary_expression" => None,
            "for_numeric_clause" | "function_declaration" if is_field("name") => None,
            "if_statement" | "elseif_statement" | "while_statement" | "repeat_statement" if is_field("condition") => {
                None
            }
            "function_call" if is_field("name") => None,
            "dot_index_expression" | "bracket_index_expression" | "method_index_expression" if !is_field("table") => {
                None
            }
            "dot_index_expression" | "bracket_index_expression" | "method_index_expression" => {
                self.indexed_use(parent)
            }
            "field" if is_field("name") && !has_child(parent, "[") => None,
            "expression_list" if parent.parent() == Some(final_return) => None,
            "arguments" => Some(TableUse::Escape(parent.parent().unwrap_or(parent))),
            _ => Some(TableUse::Escape(node)),
        }
    }

    /// `M.x`, `M[k]` or `M:f`: a read, a field write, or a method call that
    /// hands the table over as `self`.
    fn indexed_use(&self, index: Node<'tree>) -> Option<TableUse<'tree>> {
        let holder = index.parent()?;
        let statement = match holder.kind() {
            "variable_list" => holder.parent().filter(|s| s.kind() == "assignment_statement")?,
            "function_declaration" if holder.child_by_field_name("name") == Some(index) => holder,
            "function_call" if index.kind() == "method_index_expression" => {
                return Some(TableUse::Escape(holder));
            }
            _ => return None,
        };
        // `assign`/`define_function` fold exactly the chunk-level writes whose
        // table is `table_identifier`; `(M or t).x = v` is not one of them.
        let folded = statement.parent().is_some_and(|p| p.kind() == "chunk") && table_identifier(index).is_some();
        match index_key(index, self.source)? {
            Key::Name(_) if folded => None,
            Key::Name(field) => Some(TableUse::Write { field: Some(field), statement }),
            Key::NonString => None,
            Key::Unknown => Some(TableUse::Write { field: None, statement }),
        }
    }
}

/// One reference to the returned table that its chunk-level fields do not
/// describe.
enum TableUse<'tree> {
    /// A field write inside a nested block or function, through an `and`/`or`
    /// expression that may evaluate to the table, or under a [`Key::Unknown`]
    /// key (`field` is `None`).
    Write { field: Option<String>, statement: Node<'tree> },
    /// The table handed to other code: a call argument, a method receiver, or
    /// a value stored elsewhere. That code can change any field.
    Escape(Node<'tree>),
}

/// The limits the returned table's uses cause and the fields they withhold.
#[derive(Default)]
struct TableUses {
    limits: Vec<ReexportLimit>,
    /// No field of the table can be stated.
    all: bool,
    /// Fields a write that is not folded may replace.
    fields: HashSet<String>,
}

fn has_child(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
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
