//! Rust re-export extraction: `pub use` in every form, `pub extern crate`,
//! and the macro bodies that may generate either.
//!
//! Only module-level items are read. A `use` inside a function or block body
//! is not part of any module interface, and `pub(self)` is private, so
//! neither is reported.

use tree_sitter::Node;

use crate::analysis::reexports::common::{damaged_nodes, entry, finish, limit};
use crate::file::extract::text_span_for_node;
use crate::file::tree_file::TreeFile;
use crate::shared::{
    ImportRecord, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimit, ReexportLimitReason,
    ReexportReport, TextSpan,
};

pub(super) fn extract(tree_file: &TreeFile, imports: &[ImportRecord]) -> ReexportReport {
    let root = tree_file.root_node();
    let mut walker = Walker {
        source: tree_file.source(),
        imports,
        entries: Vec::new(),
        limits: Vec::new(),
    };
    walker.walk(root, &Context::default());
    walker.limit_damaged_forwarding(root);
    finish(walker.entries, walker.limits)
}

/// What the enclosing module items contribute to an entry.
#[derive(Default, Clone)]
struct Context {
    scope: Vec<String>,
    conditional: bool,
}

struct Walker<'a> {
    source: &'a str,
    imports: &'a [ImportRecord],
    entries: Vec<ReexportEntry>,
    limits: Vec<ReexportLimit>,
}

impl Walker<'_> {
    fn text(&self, node: Node<'_>) -> &str {
        node.utf8_text(self.source.as_bytes()).unwrap_or_default()
    }

    /// Visits module-level items only: the file, `mod` bodies, and the
    /// statement wrappers the grammar puts around item-position macros.
    /// Function bodies, blocks, impls and traits are never entered.
    fn walk(&mut self, container: Node<'_>, context: &Context) {
        // `inner_attribute_item`s (`#![cfg(..)]`) apply to the whole container.
        let mut cursor = container.walk();
        let children: Vec<Node<'_>> = container.children(&mut cursor).collect();
        let mut context = context.clone();
        if children.iter().any(|child| child.kind() == "inner_attribute_item" && self.is_cfg_attribute(*child)) {
            context.conditional = true;
        }
        for child in children {
            match child.kind() {
                "use_declaration" => self.use_declaration(child, &context),
                "extern_crate_declaration" => self.extern_crate(child, &context),
                "mod_item" => self.mod_item(child, &context),
                "macro_definition" | "macro_invocation" => self.macro_item(child),
                "expression_statement" | "ERROR" => self.walk(child, &context),
                _ => {}
            }
        }
    }

    fn mod_item(&mut self, item: Node<'_>, context: &Context) {
        let Some(body) = item.child_by_field_name("body") else { return };
        let mut inner = context.clone();
        if let Some(name) = item.child_by_field_name("name") {
            inner.scope.push(self.text(name).to_owned());
        }
        inner.conditional |= self.has_cfg_attribute(item);
        self.walk(body, &inner);
    }

    /// `unexpanded_macro` for a macro whose tokens contain `pub` and then
    /// `use`: expansion may forward names this file's syntax does not show.
    fn macro_item(&mut self, node: Node<'_>) {
        let mut leaves = Vec::new();
        collect_leaves(node, &mut leaves);
        let mut seen_pub = false;
        let forwards = leaves.iter().any(|leaf| match self.text(*leaf) {
            "pub" => {
                seen_pub = true;
                false
            }
            "use" => seen_pub,
            _ => false,
        });
        if forwards {
            self.limits.push(limit(ReexportLimitReason::UnexpandedMacro, node));
        }
    }

    fn use_declaration(&mut self, declaration: Node<'_>, context: &Context) {
        let Some(restriction) = self.visibility(declaration) else { return };
        let Some(argument) = declaration.child_by_field_name("argument") else { return };
        let site = Site {
            declaration,
            restriction,
            scope: (!context.scope.is_empty()).then(|| context.scope.join("::")),
            conditional: context.conditional || self.has_cfg_attribute(declaration),
        };
        self.use_tree(argument, None, &site);
    }

    /// One entry per leaf of the use tree. `prefix` is the path the enclosing
    /// groups have written so far.
    fn use_tree(&mut self, node: Node<'_>, prefix: Option<&str>, site: &Site<'_>) {
        // A name the parser had to invent is damage, not a forward.
        if node.is_missing() || node.is_error() {
            return;
        }
        match node.kind() {
            "identifier" | "crate" | "super" => {
                let name = self.text(node).to_owned();
                let mut found = self.named(node, &name, &name, prefix.map(str::to_owned), site);
                found.import_span = self.import_span(Some(node));
                self.entries.push(found);
            }
            "scoped_identifier" => {
                let Some(name_node) = node.child_by_field_name("name").filter(|name| !name.is_missing()) else { return };
                let name = self.text(name_node).to_owned();
                let source = join(prefix, self.path_before(node, name_node).as_deref());
                let mut found = self.named(node, &name, &name, source, site);
                found.import_span = self.import_span(Some(name_node));
                self.entries.push(found);
            }
            "use_as_clause" => self.use_as_clause(node, prefix, site),
            "use_wildcard" => {
                let path_node = node.named_child(0);
                let written = path_node.and_then(|path| self.stripped(path));
                let mut found = self.base(ReexportForm::Wildcard, node, site);
                found.source = join(prefix, written.as_deref());
                found.import_span = match path_node {
                    Some(path) => self.import_span(Some(last_name_node(path))),
                    None => self.group_path_import(node),
                };
                self.entries.push(found);
            }
            "scoped_use_list" => {
                let written = node.child_by_field_name("path").and_then(|path| self.stripped(path));
                let joined = join(prefix, written.as_deref());
                if let Some(list) = node.child_by_field_name("list") {
                    self.use_tree(list, joined.as_deref(), site);
                }
            }
            "use_list" => {
                let mut cursor = node.walk();
                let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
                for child in children {
                    if child.kind() == "self" {
                        self.self_leaf(child, None, prefix, site);
                    } else {
                        self.use_tree(child, prefix, site);
                    }
                }
            }
            _ => {}
        }
    }

    fn use_as_clause(&mut self, clause: Node<'_>, prefix: Option<&str>, site: &Site<'_>) {
        let (Some(path), Some(alias)) = (clause.child_by_field_name("path"), clause.child_by_field_name("alias"))
        else {
            return;
        };
        if alias.is_missing() || path.is_missing() {
            return;
        }
        let alias_text = self.text(alias).to_owned();
        if path.kind() == "self" {
            self.self_leaf(clause, Some(alias), prefix, site);
            return;
        }
        let (original, source) = match path.kind() {
            "scoped_identifier" => {
                let Some(name_node) = path.child_by_field_name("name").filter(|name| !name.is_missing()) else { return };
                let before = self.path_before(path, name_node);
                (self.text(name_node).to_owned(), join(prefix, before.as_deref()))
            }
            _ => (self.text(path).to_owned(), prefix.map(str::to_owned)),
        };
        let mut found = self.named(clause, &alias_text, &original, source, site);
        found.import_span = self.import_span(Some(alias));
        self.entries.push(found);
    }

    /// `self` (or `self as alias`) inside a group: the group's own path is
    /// the forwarded module.
    fn self_leaf(&mut self, span_node: Node<'_>, alias: Option<Node<'_>>, prefix: Option<&str>, site: &Site<'_>) {
        let Some(path) = prefix else { return };
        let (parent, last) = match path.rsplit_once("::") {
            Some((parent, last)) => (Some(parent.to_owned()), last),
            None => (None, path),
        };
        let exported = alias.map_or_else(|| last.to_owned(), |alias| self.text(alias).to_owned());
        let mut found = self.named(span_node, &exported, last, parent, site);
        found.import_span = match alias {
            Some(alias) => self.import_span(Some(alias)),
            None => self.group_path_import(span_node),
        };
        self.entries.push(found);
    }

    fn extern_crate(&mut self, declaration: Node<'_>, context: &Context) {
        let Some(restriction) = self.visibility(declaration) else { return };
        let Some(name) = declaration.child_by_field_name("name").filter(|name| !name.is_missing()) else { return };
        let alias = declaration.child_by_field_name("alias").filter(|alias| !alias.is_missing());
        let last = alias.unwrap_or(name);
        let site = Site {
            declaration,
            restriction,
            scope: (!context.scope.is_empty()).then(|| context.scope.join("::")),
            conditional: context.conditional || self.has_cfg_attribute(declaration),
        };
        let mut span = text_span_for_node(name);
        span.end = text_span_for_node(last).end;
        span.end_byte = text_span_for_node(last).end_byte;
        let mut found = entry(
            ReexportForm::Namespace,
            ReexportEvidence::Syntax,
            span,
            text_span_for_node(declaration),
        );
        found.exported_name = Some(self.text(last).to_owned());
        found.source = Some(self.text(name).to_owned());
        found.scope = site.scope;
        found.restriction = site.restriction;
        found.conditional = site.conditional;
        found.import_span = self.import_span(Some(last));
        self.entries.push(found);
    }

    fn named(
        &self,
        span_node: Node<'_>,
        exported: &str,
        original: &str,
        source: Option<String>,
        site: &Site<'_>,
    ) -> ReexportEntry {
        let mut found = self.base(ReexportForm::Named, span_node, site);
        found.exported_name = Some(exported.to_owned());
        found.original_name = Some(original.to_owned());
        found.source = source;
        found
    }

    fn base(&self, form: ReexportForm, span_node: Node<'_>, site: &Site<'_>) -> ReexportEntry {
        let mut found = entry(
            form,
            ReexportEvidence::Syntax,
            text_span_for_node(span_node),
            text_span_for_node(site.declaration),
        );
        found.scope = site.scope.clone();
        found.restriction = site.restriction.clone();
        found.conditional = site.conditional;
        found
    }

    /// The `ImportRecord` span of the binding `node` names, matched by byte
    /// range so an unrecorded binding links nothing.
    fn import_span(&self, node: Option<Node<'_>>) -> Option<TextSpan> {
        let node = node?;
        self.imports
            .iter()
            .find(|import| {
                import.span.start_byte as usize == node.start_byte() && import.span.end_byte as usize == node.end_byte()
            })
            .map(|import| import.span.clone())
    }

    /// The import binding of the enclosing group's path: what a `self` or a
    /// bare `*` inside `a::{..}` binds.
    fn group_path_import(&self, mut node: Node<'_>) -> Option<TextSpan> {
        while let Some(parent) = node.parent() {
            if parent.kind() == "scoped_use_list" {
                return self.import_span(parent.child_by_field_name("path").map(last_name_node));
            }
            node = parent;
        }
        None
    }

    /// The written visibility of `declaration`. `None` means do not report:
    /// private, or `pub(self)`. `Some(None)` is plain `pub`.
    fn visibility(&self, declaration: Node<'_>) -> Option<Option<String>> {
        let mut cursor = declaration.walk();
        let modifier = declaration.children(&mut cursor).find(|child| child.kind() == "visibility_modifier")?;
        let mut restriction = String::new();
        let mut inner = modifier.walk();
        for part in modifier.children(&mut inner) {
            match part.kind() {
                "pub" | "(" | ")" => {}
                "in" => restriction.push_str("in "),
                _ => restriction.push_str(&strip_space(self.text(part))),
            }
        }
        match restriction.as_str() {
            "" => Some(None),
            "self" => None,
            _ => Some(Some(restriction)),
        }
    }

    fn has_cfg_attribute(&self, item: Node<'_>) -> bool {
        let mut previous = item.prev_sibling();
        while let Some(node) = previous {
            match node.kind() {
                "attribute_item" if self.is_cfg_attribute(node) => return true,
                "attribute_item" | "line_comment" | "block_comment" => {}
                _ => return false,
            }
            previous = node.prev_sibling();
        }
        false
    }

    fn is_cfg_attribute(&self, attribute_item: Node<'_>) -> bool {
        let mut cursor = attribute_item.walk();
        attribute_item
            .children(&mut cursor)
            .find(|child| child.kind() == "attribute")
            .and_then(|attribute| attribute.named_child(0))
            .is_some_and(|name| matches!(self.text(name), "cfg" | "cfg_attr"))
    }

    /// The written path of `scoped` before its name, without whitespace or
    /// the trailing `::`; `None` when nothing precedes the name (`::name`).
    fn path_before(&self, scoped: Node<'_>, name: Node<'_>) -> Option<String> {
        let before = self.source.get(scoped.start_byte()..name.start_byte())?;
        let path = strip_space(before);
        let path = path.strip_suffix("::").unwrap_or(&path);
        (!path.is_empty()).then(|| path.to_owned())
    }

    fn stripped(&self, node: Node<'_>) -> Option<String> {
        let text = strip_space(self.text(node));
        (!text.is_empty()).then_some(text)
    }

    /// A damaged region limits coverage only when it holds, or directly
    /// follows, a token that starts a forwarding item.
    fn limit_damaged_forwarding(&mut self, root: Node<'_>) {
        for damaged in damaged_nodes(root) {
            if self.touches_forwarding(damaged) {
                self.limits.push(limit(ReexportLimitReason::ParseError, damaged));
            }
        }
    }

    fn touches_forwarding(&self, damaged: Node<'_>) -> bool {
        let mut ancestor = damaged.parent();
        while let Some(node) = ancestor {
            if matches!(node.kind(), "use_declaration" | "extern_crate_declaration") {
                return true;
            }
            ancestor = node.parent();
        }
        let mut leaves = Vec::new();
        collect_leaves(damaged, &mut leaves);
        let holds_trigger = leaves.iter().any(|leaf| {
            let text = self.text(*leaf);
            text == "use" || (text == "extern" && leaves.iter().any(|other| self.text(*other) == "crate"))
        });
        holds_trigger
            || previous_leaf(damaged).is_some_and(|leaf| {
                let text = self.text(leaf);
                text == "use" || (text == "crate" && self.follows_extern(leaf))
            })
    }

    fn follows_extern(&self, crate_token: Node<'_>) -> bool {
        previous_leaf(crate_token).is_some_and(|leaf| self.text(leaf) == "extern")
    }
}

/// What every entry of one `use` declaration shares.
struct Site<'a> {
    declaration: Node<'a>,
    restriction: Option<String>,
    scope: Option<String>,
    conditional: bool,
}

fn strip_space(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// `prefix::tail`, either part possibly absent.
fn join(prefix: Option<&str>, tail: Option<&str>) -> Option<String> {
    match (prefix, tail) {
        (Some(prefix), Some(tail)) => Some(format!("{prefix}::{tail}")),
        (Some(only), None) | (None, Some(only)) => Some(only.to_owned()),
        (None, None) => None,
    }
}

/// The node of a path whose text is the name it binds: a scoped path's
/// `name`, or the node itself.
fn last_name_node(path: Node<'_>) -> Node<'_> {
    if path.kind() == "scoped_identifier" { path.child_by_field_name("name").unwrap_or(path) } else { path }
}

fn collect_leaves<'t>(node: Node<'t>, out: &mut Vec<Node<'t>>) {
    if node.child_count() == 0 {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_leaves(child, out);
    }
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
