//! Python re-exports: module-level imports that a literal `__all__`, a
//! redundant alias or a star import declare, and, when the module has no
//! `__all__`, every public import as runtime exposure.
//!
//! Python has no forwarding syntax, so the extractor reads one file's
//! module-scope statements only. Imports inside functions and classes are not
//! part of the module interface and are never visited. Only the import a
//! name finally holds is forwarded: an import that a later one of the same
//! name always replaces is dropped (see `statement_relation`).

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit, ReexportLimitReason,
    ReexportReport, TextSpan,
};

/// One name an import statement binds at module scope.
struct ImportBinding<'t> {
    /// The name the module exposes (alias if any; the first segment of an
    /// unaliased `import a.b`).
    exported: String,
    /// The imported name, for `from m import a [as b]`.
    original: Option<String>,
    form: ReexportForm,
    source: String,
    conditional: bool,
    /// `import X as X` / `from m import X as X`: the typing spec's declared
    /// re-export spelling.
    redundant_alias: bool,
    /// The module object a namespace import binds to `exported` (`a` for an
    /// unaliased `import a.b`, `p` for `import p as p`); `None` for named and
    /// renamed imports. Two imports binding the same module object do not
    /// replace one another.
    module_object: Option<String>,
    specifier: Node<'t>,
    statement: Node<'t>,
    import_span: Option<TextSpan>,
}

/// One edit to `__all__` at module scope.
enum AllOp {
    Assign(Vec<String>),
    Add(Vec<String>),
    Remove(Vec<String>),
}

#[derive(Default)]
struct Scan<'t> {
    imports: Vec<ImportBinding<'t>>,
    /// `(name, start_byte)` of every module-scope binding that is not an
    /// import.
    rebinds: Vec<(String, usize)>,
    all_ops: Vec<AllOp>,
    has_all: bool,
    limits: Vec<ReexportLimit>,
}

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let source = tree_file.source();
    let root = tree_file.root_node();
    let mut scan = Scan::default();
    let mut walker = Walker { source, records: imports, scan: &mut scan };
    walker.visit_children(root, false);
    let mut limits = std::mem::take(&mut scan.limits);

    for damaged in damaged_nodes(root) {
        if touches_trigger(damaged, source) {
            limits.push(limit(ReexportLimitReason::ParseError, damaged));
        }
    }

    let all_readable = !limits.iter().any(|found| found.reason == ReexportLimitReason::ComputedExportList);
    let declared_names = declared_all_names(&scan, all_readable);
    let mut entries = Vec::new();
    for (index, binding) in scan.imports.iter().enumerate() {
        if binding.form == ReexportForm::Wildcard {
            entries.push(build(binding, ReexportEvidence::Declared));
            continue;
        }
        let replacement = later_import_replacement(binding, &scan.imports[index + 1..]);
        if replacement == Replacement::Replaces {
            continue;
        }
        let listed = declared_names.as_ref().is_some_and(|names| names.contains(&binding.exported));
        let exposed = !scan.has_all && !binding.exported.starts_with('_');
        if !(binding.redundant_alias || listed || exposed) {
            continue;
        }
        let rebound = replacement == Replacement::MayReplace
            || scan
            .rebinds
            .iter()
            .any(|(name, at)| *name == binding.exported && *at > binding.specifier.start_byte());
        if rebound {
            limits.push(limit(ReexportLimitReason::ReboundBinding, binding.specifier));
            continue;
        }
        let evidence = if binding.redundant_alias || listed {
            ReexportEvidence::Declared
        } else {
            ReexportEvidence::Exposed
        };
        entries.push(build(binding, evidence));
    }
    finish(entries, limits)
}

/// The names the module's literal `__all__` ends up listing, or `None` when
/// there is no `__all__` or it is computed.
fn declared_all_names(scan: &Scan<'_>, readable: bool) -> Option<Vec<String>> {
    if !scan.has_all || !readable {
        return None;
    }
    let mut names: Vec<String> = Vec::new();
    for op in &scan.all_ops {
        match op {
            AllOp::Assign(list) => names = list.clone(),
            AllOp::Add(list) => names.extend(list.iter().cloned()),
            AllOp::Remove(list) => names.retain(|name| !list.contains(name)),
        }
    }
    Some(names)
}

/// How a later import of the same local name relates to an earlier one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Replacement {
    /// No later import replaces it, or each one sits in a mutually exclusive
    /// branch (`try`/`except`, `if`/`elif`/`else`, `match` cases).
    None,
    /// A later import may run after it, under a condition it does not share.
    MayReplace,
    /// A later import always runs after it, so it is never the final binding.
    Replaces,
}

/// The strongest replacement of `binding` by any of the `later` imports
/// (in source order) that bind the same local name to a different object.
fn later_import_replacement(binding: &ImportBinding<'_>, later: &[ImportBinding<'_>]) -> Replacement {
    let mut found = Replacement::None;
    for other in later {
        let same_object = binding.module_object.is_some() && binding.module_object == other.module_object;
        if other.form == ReexportForm::Wildcard || other.exported != binding.exported || same_object {
            continue;
        }
        match statement_relation(binding.statement, other.statement) {
            Replacement::Replaces => return Replacement::Replaces,
            Replacement::MayReplace => found = Replacement::MayReplace,
            Replacement::None => {}
        }
    }
    found
}

/// Whether the module-scope statement `later` (after `earlier` in source
/// order) replaces what `earlier` binds. Bounded to the statement structure:
/// sibling arms of one `if`, `match` or `try` (other than `finally`, and a
/// `try` body followed by its `else`) are alternatives; any other condition
/// around `later` that does not also enclose `earlier` makes the replacement
/// possible rather than certain.
fn statement_relation(earlier: Node<'_>, later: Node<'_>) -> Replacement {
    if earlier.id() == later.id() {
        return Replacement::Replaces;
    }
    let mut earlier_chain = vec![earlier];
    while let Some(parent) = earlier_chain.last().and_then(Node::parent) {
        earlier_chain.push(parent);
    }
    let mut conditional = false;
    let mut later_child = later;
    while let Some(ancestor) = later_child.parent() {
        if let Some(at) = earlier_chain.iter().position(|node| node.id() == ancestor.id()) {
            let Some(earlier_child) = at.checked_sub(1).map(|below| earlier_chain[below]) else {
                return Replacement::None;
            };
            let alternatives = match ancestor.kind() {
                "if_statement" => true,
                "try_statement" => !matches!(
                    (earlier_child.kind(), later_child.kind()),
                    (_, "finally_clause") | ("block", "else_clause")
                ),
                _ => earlier_child.kind() == "case_clause" && later_child.kind() == "case_clause",
            };
            return match (alternatives, conditional) {
                (true, _) => Replacement::None,
                (false, true) => Replacement::MayReplace,
                (false, false) => Replacement::Replaces,
            };
        }
        conditional |= match ancestor.kind() {
            "if_statement" | "elif_clause" | "else_clause" | "except_clause" | "except_group_clause"
            | "while_statement" | "for_statement" | "match_statement" | "case_clause" => true,
            "try_statement" => later_child.kind() != "finally_clause",
            _ => false,
        };
        later_child = ancestor;
    }
    Replacement::None
}

fn build(binding: &ImportBinding<'_>, evidence: ReexportEvidence) -> ReexportEntry {
    let mut built = entry(
        binding.form,
        evidence,
        text_span_for_node(binding.specifier),
        text_span_for_node(binding.statement),
    );
    built.exported_name = (binding.form != ReexportForm::Wildcard).then(|| binding.exported.clone());
    built.original_name = binding.original.clone();
    built.source = Some(binding.source.clone());
    built.conditional = binding.conditional;
    built.import_span = binding.import_span.clone();
    built
}

struct Walker<'a, 't> {
    source: &'a str,
    records: &'a [ImportRecord],
    scan: &'a mut Scan<'t>,
}

impl<'t> Walker<'_, 't> {
    fn text(&self, node: Node<'_>) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or_default()
    }

    /// Visits `parent`'s children at module scope. `if`, `try`, loops and
    /// `match` make everything under them conditional; function and class
    /// bodies are not module scope, so only their `__all__` edits are noted.
    fn visit_children(&mut self, parent: Node<'t>, conditional: bool) {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            self.visit(child, conditional);
        }
    }

    fn visit(&mut self, node: Node<'t>, conditional: bool) {
        match node.kind() {
            "import_statement" => self.import_statement(node, conditional),
            "import_from_statement" => self.import_from_statement(node, conditional),
            "future_import_statement" => {}
            "lambda" | "list_comprehension" | "set_comprehension" | "dictionary_comprehension"
            | "generator_expression" => self.note_deferred_all_edits(node),
            "function_definition" | "class_definition" => {
                self.bind_definition(node);
                self.note_deferred_all_edits(node);
            }
            "decorated_definition" => {
                if let Some(definition) = node.child_by_field_name("definition") {
                    self.bind_definition(definition);
                }
                self.note_deferred_all_edits(node);
            }
            "expression_statement" => self.expression_statement(node, conditional),
            "assignment" => self.assignment(node, node, conditional),
            "if_statement" | "try_statement" | "while_statement" | "for_statement" | "match_statement" => {
                if node.kind() == "for_statement"
                    && let Some(target) = node.child_by_field_name("left")
                {
                    self.bind_target(target);
                }
                self.visit_children(node, true);
            }
            "delete_statement" => {
                self.bind_target(node);
                self.note_all_in(node, node);
            }
            "named_expression" => {
                if let Some(name) = node.child_by_field_name("name") {
                    self.bind_target(name);
                }
                self.visit_children(node, conditional);
            }
            "as_pattern_target" => self.bind_target(node),
            "call" => self.nested_call(node, conditional),
            _ => self.visit_children(node, conditional),
        }
    }

    fn bind_definition(&mut self, definition: Node<'t>) {
        if let Some(name) = definition.child_by_field_name("name") {
            let text = self.text(name).to_owned();
            self.scan.rebinds.push((text, name.start_byte()));
        }
    }

    /// Records every plain name `target` binds (not attributes or
    /// subscripts).
    fn bind_target(&mut self, target: Node<'t>) {
        match target.kind() {
            "identifier" => {
                let text = self.text(target).to_owned();
                self.scan.rebinds.push((text, target.start_byte()));
            }
            "attribute" | "subscript" | "call" => {}
            _ => {
                let mut cursor = target.walk();
                for child in target.children(&mut cursor) {
                    self.bind_target(child);
                }
            }
        }
    }

    /// `__all__` bound by anything other than a readable edit is computed.
    fn note_all_in(&mut self, target: Node<'t>, at: Node<'t>) {
        if self.mentions_all_target(target) {
            self.scan.has_all = true;
            self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, at));
        }
    }

    fn mentions_all_target(&self, target: Node<'_>) -> bool {
        match target.kind() {
            "identifier" => self.text(target) == "__all__",
            "attribute" | "call" => false,
            "subscript" => target
                .child_by_field_name("value")
                .is_some_and(|value| value.kind() == "identifier" && self.text(value) == "__all__"),
            _ => {
                let mut cursor = target.walk();
                target.children(&mut cursor).any(|child| self.mentions_all_target(child))
            }
        }
    }

    fn expression_statement(&mut self, statement: Node<'t>, conditional: bool) {
        let mut cursor = statement.walk();
        for expression in statement.named_children(&mut cursor) {
            match expression.kind() {
                "assignment" => self.assignment(statement, expression, conditional),
                "augmented_assignment" => self.augmented_assignment(statement, expression, conditional),
                "call" => self.all_method_call(statement, expression, conditional),
                _ => self.visit_children(expression, conditional),
            }
        }
    }

    fn assignment(&mut self, statement: Node<'t>, assignment: Node<'t>, conditional: bool) {
        let Some(left) = assignment.child_by_field_name("left") else {
            return;
        };
        let right = assignment.child_by_field_name("right");
        if left.kind() == "identifier" && self.text(left) == "__all__" {
            // A bare annotation (`__all__: list[str]`) binds nothing.
            let Some(right) = right else { return };
            self.scan.has_all = true;
            self.apply_all(statement, right, conditional, AllOp::Assign);
            return;
        }
        if right.is_some() || left.kind() != "identifier" {
            self.bind_target(left);
            self.note_all_in(left, statement);
        }
        if let Some(right) = right {
            self.visit(right, conditional);
        }
    }

    fn augmented_assignment(&mut self, statement: Node<'t>, assignment: Node<'t>, conditional: bool) {
        let Some(left) = assignment.child_by_field_name("left") else {
            return;
        };
        let is_plus = assignment.child_by_field_name("operator").is_some_and(|op| self.text(op) == "+=");
        if left.kind() == "identifier" && self.text(left) == "__all__" {
            self.scan.has_all = true;
            match (is_plus, assignment.child_by_field_name("right")) {
                (true, Some(right)) => self.apply_all(statement, right, conditional, AllOp::Add),
                _ => self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, statement)),
            }
            return;
        }
        self.bind_target(left);
    }

    /// A body that is not module scope (function, class, lambda,
    /// comprehension) may run any number of times, so an `__all__` method
    /// call or `global __all__` inside it makes the list computed.
    fn note_deferred_all_edits(&mut self, body: Node<'t>) {
        let mut stack = vec![body];
        while let Some(node) = stack.pop() {
            let edits_all = match node.kind() {
                "call" => self.all_method(node).is_some(),
                "global_statement" => {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor).any(|name| self.text(name) == "__all__")
                }
                _ => false,
            };
            if edits_all {
                self.scan.has_all = true;
                self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, node));
            }
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
        }
    }

    /// The method name of a `__all__.<method>(..)` call.
    fn all_method(&self, call: Node<'_>) -> Option<String> {
        let function = call.child_by_field_name("function").filter(|function| function.kind() == "attribute")?;
        let object = function.child_by_field_name("object")?;
        let attribute = function.child_by_field_name("attribute")?;
        (object.kind() == "identifier" && self.text(object) == "__all__").then(|| self.text(attribute).to_owned())
    }

    /// A `__all__` method call inside a larger expression (`flag and
    /// __all__.append("a")`, a call argument) may or may not run, so the
    /// list is computed.
    fn nested_call(&mut self, call: Node<'t>, conditional: bool) {
        if self.all_method(call).is_some() {
            self.scan.has_all = true;
            self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, call));
        }
        self.visit_children(call, conditional);
    }

    /// `__all__.append("a")`, `.extend([..])`, `.remove("a")`.
    fn all_method_call(&mut self, statement: Node<'t>, call: Node<'t>, conditional: bool) {
        let Some(method) = self.all_method(call) else {
            self.visit_children(call, conditional);
            return;
        };
        self.scan.has_all = true;
        let argument = call.child_by_field_name("arguments").and_then(|args| {
            let mut cursor = args.walk();
            let mut named = args.named_children(&mut cursor);
            let first = named.next();
            if named.next().is_some() { None } else { first }
        });
        match (method.as_str(), argument) {
            ("append", Some(argument)) if argument.kind() == "string" => {
                self.apply_all(statement, argument, conditional, AllOp::Add);
            }
            ("remove", Some(argument)) if argument.kind() == "string" => {
                self.apply_all(statement, argument, conditional, AllOp::Remove);
            }
            ("extend", Some(argument)) => self.apply_all(statement, argument, conditional, AllOp::Add),
            _ => self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, statement)),
        }
    }

    /// Evaluates `value` as a literal name list and records the edit; a
    /// conditional edit cannot be merged, so it is computed.
    fn apply_all(&mut self, statement: Node<'t>, value: Node<'t>, conditional: bool, make: fn(Vec<String>) -> AllOp) {
        let mut read = ReadList::default();
        self.read_list(value, &mut read);
        if conditional || read.computed {
            self.scan.limits.push(limit(ReexportLimitReason::ComputedExportList, statement));
            return;
        }
        if read.cross_file {
            self.scan.limits.push(limit(ReexportLimitReason::CrossFileExportList, statement));
        }
        self.scan.all_ops.push(make(read.names));
    }

    /// Reads a list or tuple of string literals, `+` of those, or
    /// `module.__all__` (cross-file). Anything else is computed.
    fn read_list(&self, node: Node<'_>, read: &mut ReadList) {
        match node.kind() {
            "list" | "tuple" => {
                let mut cursor = node.walk();
                for element in node.named_children(&mut cursor) {
                    match element.kind() {
                        "string" => self.read_string(element, read),
                        "comment" => {}
                        _ => read.computed = true,
                    }
                }
            }
            "string" => self.read_string(node, read),
            "parenthesized_expression" => {
                let mut cursor = node.walk();
                for inner in node.named_children(&mut cursor) {
                    self.read_list(inner, read);
                }
            }
            "binary_operator" => {
                let is_plus = node.child_by_field_name("operator").is_some_and(|op| self.text(op) == "+");
                match (is_plus, node.child_by_field_name("left"), node.child_by_field_name("right")) {
                    (true, Some(left), Some(right)) => {
                        self.read_list(left, read);
                        self.read_list(right, read);
                    }
                    _ => read.computed = true,
                }
            }
            "attribute" => {
                let is_module_all = node
                    .child_by_field_name("attribute")
                    .is_some_and(|attribute| self.text(attribute) == "__all__")
                    && node.child_by_field_name("object").is_some_and(|object| {
                        matches!(object.kind(), "identifier" | "attribute")
                    });
                if is_module_all {
                    read.cross_file = true;
                } else {
                    read.computed = true;
                }
            }
            _ => read.computed = true,
        }
    }

    fn read_string(&self, node: Node<'_>, read: &mut ReadList) {
        let mut cursor = node.walk();
        let mut content = None;
        let mut plain = true;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "string_content" => content = Some(self.text(child).to_owned()),
                "string_start" => {
                    let prefix = self.text(child).to_ascii_lowercase();
                    plain &= !prefix.contains('f') && !prefix.contains('b');
                }
                "interpolation" => plain = false,
                _ => {}
            }
        }
        match content {
            Some(name) if plain => read.names.push(name),
            _ => read.computed = true,
        }
    }

    fn record_for(&self, within: Node<'_>) -> Option<TextSpan> {
        let range = within.byte_range();
        self.records
            .iter()
            .find(|record| {
                record.span.start_byte as usize >= range.start && record.span.end_byte as usize <= range.end
            })
            .map(|record| record.span.clone())
    }

    fn import_statement(&mut self, statement: Node<'t>, conditional: bool) {
        let mut cursor = statement.walk();
        for specifier in statement.children_by_field_name("name", &mut cursor) {
            let (module, alias) = match specifier.kind() {
                "aliased_import" => (
                    specifier.child_by_field_name("name"),
                    specifier.child_by_field_name("alias"),
                ),
                _ => (Some(specifier), None),
            };
            let Some(module) = module else { continue };
            let module_text = self.text(module).to_owned();
            let first_segment = module_text.split('.').next().unwrap_or_default().to_owned();
            let (exported, redundant_alias) = match alias {
                Some(alias) => {
                    let alias_text = self.text(alias).to_owned();
                    let redundant = alias_text == module_text;
                    (alias_text, redundant)
                }
                None => (first_segment, false),
            };
            let module_object = (alias.is_none() || redundant_alias).then(|| exported.clone());
            self.scan.imports.push(ImportBinding {
                exported,
                original: None,
                form: ReexportForm::Namespace,
                source: module_text,
                conditional,
                redundant_alias,
                module_object,
                specifier,
                statement,
                import_span: self.record_for(specifier),
            });
        }
    }

    fn import_from_statement(&mut self, statement: Node<'t>, conditional: bool) {
        let Some(module) = statement.child_by_field_name("module_name") else {
            return;
        };
        let source = self.text(module).to_owned();
        if source == "__future__" {
            return;
        }
        let mut cursor = statement.walk();
        for child in statement.named_children(&mut cursor) {
            if child.kind() == "wildcard_import" {
                self.scan.imports.push(ImportBinding {
                    exported: String::new(),
                    original: None,
                    form: ReexportForm::Wildcard,
                    source: source.clone(),
                    conditional,
                    redundant_alias: false,
                    module_object: None,
                    specifier: child,
                    statement,
                    import_span: self.record_for(module),
                });
            }
        }
        let mut cursor = statement.walk();
        for specifier in statement.children_by_field_name("name", &mut cursor) {
            let (name, alias) = match specifier.kind() {
                "aliased_import" => (
                    specifier.child_by_field_name("name"),
                    specifier.child_by_field_name("alias"),
                ),
                _ => (Some(specifier), None),
            };
            let Some(name) = name else { continue };
            let original = self.text(name).to_owned();
            let exported = alias.map_or_else(|| original.clone(), |alias| self.text(alias).to_owned());
            let redundant_alias = alias.is_some() && exported == original;
            self.scan.imports.push(ImportBinding {
                exported,
                original: Some(original),
                form: ReexportForm::Named,
                source: source.clone(),
                conditional,
                redundant_alias,
                module_object: None,
                specifier,
                statement,
                import_span: self.record_for(specifier),
            });
        }
    }
}

#[derive(Default)]
struct ReadList {
    names: Vec<String>,
    computed: bool,
    cross_file: bool,
}

/// Whether the damaged `node` contains, or immediately follows, a token that
/// starts forwarding syntax: `import`, `from` or `__all__`.
fn touches_trigger(node: Node<'_>, source: &str) -> bool {
    let is_trigger = |leaf: Node<'_>| {
        matches!(leaf.utf8_text(source.as_bytes()).unwrap_or_default(), "import" | "from" | "__all__")
    };
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.child_count() == 0 {
            if is_trigger(current) {
                return true;
            }
        } else {
            let mut cursor = current.walk();
            stack.extend(current.children(&mut cursor));
        }
    }
    previous_leaf(node).is_some_and(is_trigger)
}

/// The last token that ends at or before `node` starts.
fn previous_leaf(node: Node<'_>) -> Option<Node<'_>> {
    let mut current = node;
    loop {
        if let Some(mut previous) = current.prev_sibling() {
            while previous.child_count() > 0 {
                previous = previous.child((previous.child_count() - 1) as u32)?;
            }
            return Some(previous);
        }
        current = current.parent()?;
    }
}
