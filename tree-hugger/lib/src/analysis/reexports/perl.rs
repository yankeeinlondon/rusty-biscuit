//! Perl re-exports: names in a literal `@EXPORT`, `@EXPORT_OK` or
//! `%EXPORT_TAGS` that the same package binds through `use Module qw(..)`,
//! in a package that inherits `Exporter`'s `import`.
//!
//! Perl has no forwarding syntax, so this is a convention read from one file.
//! Anything that can export names by another route (a custom `import`, a
//! typeglob assignment, an explicit `->import` call, a computed list, a
//! qualified `@Pkg::EXPORT_OK`) limits coverage instead of being guessed.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit, ReexportLimitReason,
    ReexportReport,
};

/// A name listed in an export variable.
struct ExportItem<'t> {
    name: String,
    node: Node<'t>,
    statement: Node<'t>,
    conditional: bool,
}

/// A name bound by `use Module qw(..)`.
struct UseBinding<'t> {
    name: String,
    module: String,
    statement: Node<'t>,
}

#[derive(Default)]
struct PackageState<'t> {
    /// `import` is inherited from `Exporter`.
    exporter: bool,
    uses: Vec<UseBinding<'t>>,
    /// `use Module;` and `use Module qw(:tag)`: names come from the other
    /// module's own lists, which this file cannot read.
    implicit_uses: Vec<Node<'t>>,
    items: Vec<ExportItem<'t>>,
    tag_items: Vec<ExportItem<'t>>,
    /// `(name, start_byte)` of each `sub NAME` in the package.
    subs: Vec<(String, usize)>,
    /// Export lists the extractor could not read as literals.
    computed: Vec<Node<'t>>,
}

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let source = tree_file.source();
    let root = tree_file.root_node();
    let mut walker = Walker {
        source,
        packages: HashMap::new(),
        package: None,
        limits: Vec::new(),
    };
    walker.visit(root, false, false);
    walker.scan_dynamic(root);

    let mut limits = std::mem::take(&mut walker.limits);
    for damaged in damaged_nodes(root) {
        if touches_trigger(damaged, source) {
            limits.push(limit(ReexportLimitReason::ParseError, damaged));
        }
    }

    let mut entries = Vec::new();
    for (package, state) in &walker.packages {
        if state.exporter {
            resolve(package.as_deref(), state, imports, &mut entries, &mut limits);
        }
    }
    finish(entries, limits)
}

fn resolve(
    package: Option<&str>,
    state: &PackageState<'_>,
    imports: &[ImportRecord],
    entries: &mut Vec<ReexportEntry>,
    limits: &mut Vec<ReexportLimit>,
) {
    for node in &state.computed {
        limits.push(limit(ReexportLimitReason::ComputedExportList, *node));
    }
    // A tag only names what the two lists already name, so a tagged name is
    // reported on its own only when neither list has it.
    let tag_only = state
        .tag_items
        .iter()
        .filter(|tagged| !state.items.iter().any(|item| item.name == tagged.name));
    for item in state.items.iter().chain(tag_only) {
        let Some(binding) = state.uses.iter().rev().find(|binding| binding.name == item.name) else {
            for implicit in &state.implicit_uses {
                limits.push(limit(ReexportLimitReason::CrossFileExportList, *implicit));
            }
            continue;
        };
        let redefined = state
            .subs
            .iter()
            .any(|(name, at)| *name == item.name && *at > binding.statement.start_byte());
        if redefined {
            limits.push(limit(ReexportLimitReason::ReboundBinding, item.node));
            continue;
        }
        let mut forwarded = entry(
            ReexportForm::Named,
            ReexportEvidence::Declared,
            text_span_for_node(item.node),
            text_span_for_node(item.statement),
        );
        forwarded.exported_name = Some(item.name.clone());
        forwarded.original_name = Some(item.name.clone());
        forwarded.local_name = Some(item.name.clone());
        forwarded.source = Some(binding.module.clone());
        forwarded.scope = package.map(str::to_owned);
        forwarded.conditional = item.conditional;
        let statement_range = binding.statement.byte_range();
        forwarded.import_span = imports
            .iter()
            .find(|record| {
                record.name == item.name
                    && record.span.start_byte as usize >= statement_range.start
                    && record.span.end_byte as usize <= statement_range.end
            })
            .map(|record| record.span.clone());
        entries.push(forwarded);
    }
}

struct Walker<'a, 't> {
    source: &'a str,
    packages: HashMap<Option<String>, PackageState<'t>>,
    package: Option<String>,
    limits: Vec<ReexportLimit>,
}

impl<'t> Walker<'_, 't> {
    fn text(&self, node: Node<'_>) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or_default()
    }

    fn state(&mut self) -> &mut PackageState<'t> {
        self.packages.entry(self.package.clone()).or_default()
    }

    fn visit_children(&mut self, parent: Node<'t>, in_sub: bool, conditional: bool) {
        let mut cursor = parent.walk();
        for child in parent.children(&mut cursor) {
            self.visit(child, in_sub, conditional);
        }
    }

    fn visit(&mut self, node: Node<'t>, in_sub: bool, conditional: bool) {
        match node.kind() {
            "package_statement" => self.package_statement(node, in_sub, conditional),
            "use_no_statement" => self.use_statement(node),
            "use_parent_statement" => {
                if self.lists_exporter(node) {
                    self.state().exporter = true;
                }
            }
            "binary_expression" => self.assignment(node, in_sub, conditional),
            "call_expression_with_spaced_args" | "call_expression_with_args_with_brackets" => {
                self.push_call(node, in_sub, conditional);
            }
            "function_definition" => {
                if let Some(name) = node.children(&mut node.walk()).find(|child| child.kind() == "identifier") {
                    let text = self.text(name).to_owned();
                    self.state().subs.push((text, node.start_byte()));
                }
                self.visit_children(node, true, conditional);
            }
            "anonymous_function" => self.visit_children(node, true, conditional),
            "single_line_statement" => self.visit_children(node, in_sub, true),
            "block" | "standalone_block" | "special_block" => {
                let saved = self.package.clone();
                self.visit_children(node, in_sub, conditional);
                self.package = saved;
            }
            kind if is_conditional_kind(kind) => self.visit_children(node, in_sub, true),
            _ => self.visit_children(node, in_sub, conditional),
        }
    }

    fn package_statement(&mut self, node: Node<'t>, in_sub: bool, conditional: bool) {
        let name = node
            .children(&mut node.walk())
            .find(|child| child.kind() == "package_name")
            .map(|name| self.text(name).to_owned());
        match node.child_by_field_name("body").or_else(|| node.children(&mut node.walk()).find(|c| c.kind() == "block")) {
            Some(block) => {
                let saved = std::mem::replace(&mut self.package, name);
                self.visit_children(block, in_sub, conditional);
                self.package = saved;
            }
            None => self.package = name,
        }
    }

    /// Whether the `use parent`/`use base` list names `Exporter`.
    fn lists_exporter(&self, node: Node<'t>) -> bool {
        let mut cursor = node.walk();
        node.named_children(&mut cursor).any(|child| {
            self.read_items(child)
                .is_some_and(|items| items.iter().any(|item| item.name == "Exporter"))
        })
    }

    fn use_statement(&mut self, node: Node<'t>) {
        let Some(keyword) = node.child(0) else { return };
        if keyword.kind() != "use" {
            return;
        }
        let mut cursor = node.walk();
        let mut named = node.named_children(&mut cursor).filter(|child| child.kind() != "comment");
        let Some(module_node) = named.next().filter(|child| child.kind() == "package_name") else {
            return;
        };
        let module = self.text(module_node).to_owned();
        let arguments: Vec<Node<'t>> = named.filter(|child| child.kind() != "version").collect();
        match module.as_str() {
            "Exporter" => {
                let imports_import = arguments.iter().any(|arg| {
                    self.read_items(*arg).is_some_and(|items| items.iter().any(|item| item.name == "import"))
                });
                if imports_import {
                    self.state().exporter = true;
                }
                return;
            }
            "base" | "parent" => {
                if arguments.iter().any(|arg| {
                    self.read_items(*arg).is_some_and(|items| items.iter().any(|item| item.name == "Exporter"))
                }) {
                    self.state().exporter = true;
                }
                return;
            }
            _ => {}
        }
        // Lowercase modules are pragmas (`strict`, `constant`, `lib`, `vars`).
        if module.chars().next().is_some_and(char::is_lowercase) {
            return;
        }
        let items = match arguments.as_slice() {
            [] => None,
            [only] => self.read_items(*only),
            _ => None,
        };
        let Some(items) = items else {
            self.state().implicit_uses.push(node);
            return;
        };
        let mut implicit = false;
        for item in items {
            if item.name.starts_with([':', '-', '!', '/']) {
                implicit = true;
                continue;
            }
            self.state().uses.push(UseBinding { name: item.name, module: module.clone(), statement: node });
        }
        if implicit {
            self.state().implicit_uses.push(node);
        }
    }

    /// The names of a literal list: `qw(..)`, `("a", 'b')` or one string.
    /// `None` when any element is not a plain string. A leading `&` is
    /// dropped because `&func` and `func` name the same sub.
    fn read_items(&self, node: Node<'t>) -> Option<Vec<Item<'t>>> {
        let mut items = Vec::new();
        self.collect_items(node, &mut items).then_some(items)
    }

    fn collect_items(&self, node: Node<'t>, items: &mut Vec<Item<'t>>) -> bool {
        match node.kind() {
            "word_list_qw" => {
                let mut cursor = node.walk();
                for word in node.named_children(&mut cursor).filter(|child| child.kind() == "list_item") {
                    items.push(Item { name: normalize(self.text(word)), node: word });
                }
                true
            }
            "string_single_quoted" | "string_double_quoted" => {
                if node.named_child_count() > 0 {
                    return false;
                }
                let text = self.text(node);
                let inner = text.get(1..text.len().saturating_sub(1)).unwrap_or_default();
                items.push(Item { name: normalize(inner), node });
                true
            }
            "array" => {
                let mut cursor = node.walk();
                node.named_children(&mut cursor)
                    .filter(|child| !matches!(child.kind(), "normal_comma" | "fat_comma" | "comment"))
                    .all(|child| self.collect_items(child, items))
            }
            _ => false,
        }
    }

    fn assignment(&mut self, node: Node<'t>, in_sub: bool, conditional: bool) {
        let mut cursor = node.walk();
        let children: Vec<Node<'t>> = node.children(&mut cursor).collect();
        let Some(left) = children.first().copied() else { return };
        let Some(value) = children.iter().rev().copied().find(|child| child.is_named()) else {
            return;
        };
        let is_plain_assign = children.iter().any(|child| !child.is_named() && child.kind() == "=");
        let variable = match left.kind() {
            "variable_declaration" => {
                let scope = left
                    .children(&mut left.walk())
                    .find(|child| child.kind() == "scope")
                    .map(|scope| self.text(scope));
                if scope == Some("our") { left.named_children(&mut left.walk()).last() } else { None }
            }
            "array_variable" | "hash_variable" => Some(left),
            _ => None,
        };
        let Some(variable) = variable else {
            // Not an export variable: look for nested subs and conditions.
            for child in children {
                self.visit(child, in_sub, conditional);
            }
            return;
        };
        let name = self.text(variable).trim_start_matches(['@', '%']).to_owned();
        match name.as_str() {
            "EXPORT" | "EXPORT_OK" => {
                self.export_list(node, value, is_plain_assign, in_sub, conditional);
            }
            "EXPORT_TAGS" => self.export_tags(node, value, is_plain_assign, in_sub, conditional),
            "ISA" => {
                let names_exporter = self
                    .read_items(value)
                    .is_some_and(|items| items.iter().any(|item| item.name == "Exporter"));
                if is_plain_assign && names_exporter {
                    self.state().exporter = true;
                }
            }
            _ => {}
        }
    }

    fn export_list(
        &mut self,
        statement: Node<'t>,
        value: Node<'t>,
        readable_operator: bool,
        in_sub: bool,
        conditional: bool,
    ) {
        let items = if readable_operator && !in_sub { self.read_items(value) } else { None };
        let Some(items) = items else {
            self.state().computed.push(statement);
            return;
        };
        for item in items {
            self.state().items.push(ExportItem { name: item.name, node: item.node, statement, conditional });
        }
    }

    /// `%EXPORT_TAGS = (name => [qw(..)], ..)`; the keys parse as barewords.
    fn export_tags(&mut self, statement: Node<'t>, value: Node<'t>, readable_operator: bool, in_sub: bool, conditional: bool) {
        let mut lists = Vec::new();
        let mut readable = readable_operator && !in_sub && value.kind() == "array";
        if readable {
            let mut cursor = value.walk();
            for child in value.named_children(&mut cursor) {
                match child.kind() {
                    "call_expression_with_bareword" | "fat_comma" | "normal_comma" | "comment" => {}
                    "string_single_quoted" | "string_double_quoted" => {}
                    "array_ref" => lists.push(child),
                    _ => readable = false,
                }
            }
        }
        let mut items = Vec::new();
        if readable {
            for list in lists {
                let mut cursor = list.walk();
                for element in list.named_children(&mut cursor) {
                    readable &= self.collect_items(element, &mut items);
                }
            }
        }
        if !readable {
            self.state().computed.push(statement);
            return;
        }
        for item in items {
            self.state().tag_items.push(ExportItem { name: item.name, node: item.node, statement, conditional });
        }
    }

    /// `push @EXPORT_OK, qw(..)` and `push @ISA, 'Exporter'`.
    fn push_call(&mut self, node: Node<'t>, in_sub: bool, conditional: bool) {
        let mut cursor = node.walk();
        let children: Vec<Node<'t>> = node.named_children(&mut cursor).collect();
        let function = children.first().map(|function| self.text(*function)).unwrap_or_default();
        let (Some(arguments), true) = (children.get(1).copied(), matches!(function, "push" | "unshift")) else {
            for child in children {
                self.visit(child, in_sub, conditional);
            }
            return;
        };
        let mut cursor = arguments.walk();
        let mut parts = arguments.named_children(&mut cursor).filter(|part| !matches!(part.kind(), "normal_comma" | "comment"));
        let Some(target) = parts.next().filter(|part| part.kind() == "array_variable") else {
            return;
        };
        let name = self.text(target).trim_start_matches('@').to_owned();
        if !matches!(name.as_str(), "EXPORT" | "EXPORT_OK" | "ISA") {
            return;
        }
        let mut items = Vec::new();
        let readable = parts.all(|part| self.collect_items(part, &mut items));
        if name == "ISA" {
            if readable && items.iter().any(|item| item.name == "Exporter") {
                self.state().exporter = true;
            }
            return;
        }
        if !readable || in_sub {
            self.state().computed.push(node);
            return;
        }
        for item in items {
            self.state().items.push(ExportItem { name: item.name, node: item.node, statement: node, conditional });
        }
    }

    /// Forwarding routes the package analysis cannot follow: a custom
    /// `import`, an explicit `->import` call, a typeglob assignment, and a
    /// qualified export variable (which the grammar misparses without error).
    fn scan_dynamic(&mut self, root: Node<'t>) {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let mut cursor = node.walk();
            stack.extend(node.children(&mut cursor));
            match node.kind() {
                "bareword_import" => self.dynamic(node),
                "method_invocation" => {
                    let method = node
                        .children(&mut node.walk())
                        .skip_while(|child| child.kind() != "arrow_operator")
                        .nth(1)
                        .map(|method| self.text(method).to_owned());
                    if method.as_deref().is_some_and(is_import_method) {
                        self.dynamic(node);
                    }
                }
                "function_definition" => {
                    let name = node.children(&mut node.walk()).find(|child| child.kind() == "identifier");
                    if name.is_some_and(|name| matches!(self.text(name), "import" | "unimport")) {
                        self.dynamic(node);
                    }
                }
                "binary_expression" => {
                    let Some(left) = node.child(0) else { continue };
                    if node.children(&mut node.walk()).any(|child| !child.is_named() && child.kind() == "=")
                        && has_typeglob(left)
                    {
                        self.dynamic(node);
                    }
                    if left.kind() == "package_variable" && self.is_qualified_export(left) {
                        self.limits.push(limit(ReexportLimitReason::GrammarGap, left));
                    }
                }
                "call_expression_with_spaced_args" | "call_expression_with_args_with_brackets" => {
                    let mut cursor = node.walk();
                    let mut children = node.named_children(&mut cursor);
                    let function = children.next().map(|function| self.text(function));
                    let first = children.next().and_then(|arguments| arguments.named_child(0));
                    if matches!(function, Some("push" | "unshift"))
                        && let Some(first) = first.filter(|first| first.kind() == "package_variable")
                        && self.is_qualified_export(first)
                    {
                        self.limits.push(limit(ReexportLimitReason::GrammarGap, first));
                    }
                }
                _ => {}
            }
        }
    }

    fn dynamic(&mut self, node: Node<'_>) {
        self.limits.push(limit(ReexportLimitReason::DynamicForwarding, node));
    }

    /// `@Pkg::EXPORT_OK` and friends.
    fn is_qualified_export(&self, variable: Node<'_>) -> bool {
        let text = self.text(variable);
        text.rsplit("::").next().is_some_and(|last| last.starts_with("EXPORT")) && text.contains("::")
    }
}

struct Item<'t> {
    name: String,
    node: Node<'t>,
}

fn normalize(raw: &str) -> String {
    raw.trim().strip_prefix('&').unwrap_or_else(|| raw.trim()).to_owned()
}

fn is_import_method(name: &str) -> bool {
    matches!(name, "import" | "unimport" | "export_to_level" | "export" | "export_fail")
}

fn has_typeglob(node: Node<'_>) -> bool {
    node.kind().contains("glob")
        || (node.kind() == "variable_declaration"
            && node.children(&mut node.walk()).any(|child| child.kind().contains("glob")))
}

/// Statement kinds whose body runs only under a condition or in a loop.
fn is_conditional_kind(kind: &str) -> bool {
    kind != "function_definition"
        && matches!(kind.split('_').next(), Some("if" | "unless" | "elsif" | "else" | "for" | "foreach" | "while" | "until"))
}

/// Whether the damaged `node` contains, or immediately follows, a token that
/// starts forwarding syntax: `use`, or an `@EXPORT*`, `%EXPORT_TAGS` or
/// `@ISA` variable.
fn touches_trigger(node: Node<'_>, source: &str) -> bool {
    let is_trigger = |leaf: Node<'_>| {
        let text = leaf.utf8_text(source.as_bytes()).unwrap_or_default();
        let bare = text.trim_start_matches(['@', '%', '$']);
        text == "use" || bare.starts_with("EXPORT") || bare == "ISA"
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
