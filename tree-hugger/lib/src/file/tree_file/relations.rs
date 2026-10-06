//! Declared type relations (`relations.extends` / `relations.implements`)
//! and Rust impl ownership, resolved against one per-file index of every
//! declaration.
//!
//! A written base gets a target `id` (and its in-file `qualified_name`)
//! only when its path names exactly one eligible declaration of this file
//! under the language's lexical rule; see [`Declarations::resolve`].
//! Everything else keeps its written text alone. Rust `impl` blocks are
//! traversed once per file, empty ones included, and the same self-type
//! resolution decides both the type-level `implements` summary and, under
//! Containment, which record the impl's members belong to.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::analysis::counters::{count_relation_link, count_shared_context_steps};
use crate::analysis::qualifying_prefixes;
use crate::file::extract::{
    BasePath, ImplHeader, PathAnchor, WrittenBase, declared_bases, declared_generics, impl_header,
    impl_of_member,
};
use crate::shared::{ProgrammingLanguage, SymbolKindV2, SymbolRecord, SymbolRef};

/// The nodes one record was built from, in record order.
pub(super) struct RelationInput<'tree> {
    pub(super) declaration: Node<'tree>,
    pub(super) anchor: Node<'tree>,
    pub(super) name: Node<'tree>,
    /// The record's kind is selected: only selected records get relations.
    pub(super) wanted: bool,
    /// The Rust impl whose self type names the record's container: its
    /// members and everything declared inside them.
    pub(super) container_impl: Option<Node<'tree>>,
}

/// Kinds a base list may name: a class, interface, trait or (Go, Rust,
/// Swift) a type. A type alias would need expansion and an enum is no base.
fn is_base_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Class | SymbolKindV2::Interface | SymbolKindV2::Trait | SymbolKindV2::Type
    )
}

/// Kinds a Rust impl's self type may name.
fn is_self_type_kind(kind: SymbolKindV2) -> bool {
    matches!(kind, SymbolKindV2::Type | SymbolKindV2::Class | SymbolKindV2::Enum)
}

fn is_trait_kind(kind: SymbolKindV2) -> bool {
    kind == SymbolKindV2::Trait
}

/// Kinds that may write a base list.
fn declares_bases(kind: SymbolKindV2) -> bool {
    is_base_kind(kind) || kind == SymbolKindV2::Enum
}

/// Kinds whose declaration names the scope of what it encloses.
fn is_scope_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Function
            | SymbolKindV2::Method
            | SymbolKindV2::Constructor
            | SymbolKindV2::Type
            | SymbolKindV2::Class
            | SymbolKindV2::Interface
            | SymbolKindV2::Enum
            | SymbolKindV2::Trait
            | SymbolKindV2::Module
            | SymbolKindV2::Namespace
    )
}

/// The separator a language writes between the segments of a type path.
fn path_separator(language: ProgrammingLanguage) -> &'static str {
    match language {
        ProgrammingLanguage::Rust | ProgrammingLanguage::Cpp | ProgrammingLanguage::Perl => "::",
        ProgrammingLanguage::Php => "\\",
        _ => ".",
    }
}

/// Every declaration of the file by in-file qualified name, and the
/// records that name a scope by their nodes. Built once per file from all
/// records, selected or not.
struct Declarations<'a> {
    records: &'a [SymbolRecord],
    by_qualified: HashMap<&'a str, Vec<usize>>,
    scopes_by_node: HashMap<usize, usize>,
    language: ProgrammingLanguage,
}

impl<'a> Declarations<'a> {
    fn new(records: &'a [SymbolRecord], inputs: &[RelationInput<'_>], language: ProgrammingLanguage) -> Self {
        let mut by_qualified: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut scopes_by_node = HashMap::new();
        for (idx, (record, input)) in records.iter().zip(inputs).enumerate() {
            by_qualified
                .entry(record.identity.qualified_name.as_str())
                .or_default()
                .push(idx);
            if is_scope_kind(record.kind) {
                for node in [input.declaration, input.anchor] {
                    scopes_by_node.entry(node.id()).or_insert(idx);
                }
            }
        }
        count_shared_context_steps(records.len());
        Self { records, by_qualified, scopes_by_node, language }
    }

    /// The record whose declaration is the nearest scope around `node`.
    fn enclosing_scope(&self, node: Node<'_>) -> Option<usize> {
        let mut ancestor = node.parent();
        while let Some(current) = ancestor {
            if let Some(&idx) = self.scopes_by_node.get(&current.id()) {
                return Some(idx);
            }
            ancestor = current.parent();
        }
        None
    }

    /// The in-file qualified names of the Rust modules around `node`,
    /// innermost first.
    fn enclosing_modules(&self, node: Node<'_>) -> Vec<&'a str> {
        let mut modules = Vec::new();
        let mut ancestor = node.parent();
        while let Some(current) = ancestor {
            if current.kind() == "mod_item"
                && let Some(&idx) = self.scopes_by_node.get(&current.id())
            {
                modules.push(self.records[idx].identity.qualified_name.as_str());
            }
            ancestor = current.parent();
        }
        modules
    }

    /// The record a written path names, if exactly one eligible declaration
    /// of this file is that name.
    ///
    /// The path is joined onto each scope prefix `scopes` lists, innermost
    /// first, for a relative path, onto the root for `crate::` or a PHP
    /// `\Name`, and onto the module `modules` gives for Rust `self::` and
    /// `super::`. The first prefix at which *any* declaration has the
    /// joined name decides: it resolves only if that declaration is the
    /// only one and `eligible` accepts its kind, so a nearer type alias,
    /// variable or duplicate shadows an outer type rather than falling
    /// through to it. A relative path whose first segment is a visible type
    /// parameter never resolves. In Rust a relative path is looked up only
    /// up to the nearest module, since an item does not see its parent
    /// module's names.
    fn resolve(
        &self,
        path: &BasePath,
        scopes: &[&str],
        modules: &[&str],
        type_params: &[String],
        eligible: fn(SymbolKindV2) -> bool,
    ) -> Option<usize> {
        let separator = path_separator(self.language);
        let joined = path.segments.join(separator);
        let module_at = |out: usize| match out.cmp(&modules.len()) {
            std::cmp::Ordering::Less => Some(modules[out]),
            std::cmp::Ordering::Equal => Some(""),
            std::cmp::Ordering::Greater => None,
        };
        let prefixes: Vec<&str> = match path.anchor {
            PathAnchor::Relative => {
                if path.segments.first().is_some_and(|first| type_params.contains(first)) {
                    return None;
                }
                scopes.to_vec()
            }
            PathAnchor::Root => vec![""],
            PathAnchor::SelfModule => vec![module_at(0)?],
            PathAnchor::Super(out) => vec![module_at(out)?],
        };
        for prefix in prefixes {
            count_shared_context_steps(1);
            let key = if prefix.is_empty() { joined.clone() } else { format!("{prefix}{separator}{joined}") };
            if let Some(found) = self.by_qualified.get(key.as_str()) {
                return match found.as_slice() {
                    [only] if eligible(self.records[*only].kind) => Some(*only),
                    _ => None,
                };
            }
            if self.language == ProgrammingLanguage::Rust
                && self.by_qualified.get(prefix).is_some_and(|scope| {
                    scope.iter().any(|&idx| self.records[idx].kind == SymbolKindV2::Module)
                })
            {
                return None;
            }
        }
        None
    }

    /// The relation target for `base`.
    fn target(
        &self,
        base: &WrittenBase,
        scopes: &[&str],
        modules: &[&str],
        type_params: &[String],
        eligible: fn(SymbolKindV2) -> bool,
    ) -> SymbolRef {
        let resolved = base
            .path
            .as_ref()
            .and_then(|path| self.resolve(path, scopes, modules, type_params, eligible))
            .map(|idx| &self.records[idx]);
        SymbolRef {
            id: resolved.map(|record| record.id.clone()),
            name: base.text.clone(),
            qualified_name: resolved.map(|record| record.identity.qualified_name.clone()),
        }
    }
}

/// The scope prefixes a relative path is looked up under from inside the
/// scope named `scope` (an in-file qualified name, `""` for the root),
/// innermost first, ending at the root.
fn scope_chain(scope: &str) -> Vec<&str> {
    let mut chain = Vec::new();
    if !scope.is_empty() {
        chain.push(scope);
        chain.extend(qualifying_prefixes(scope));
    }
    chain.push("");
    chain
}

/// One Rust impl, as resolved.
struct ResolvedImpl {
    header: ImplHeader,
    /// The self type's record, when its path names exactly one.
    self_type: Option<usize>,
    /// The implemented trait (none for an inherent or negative impl).
    implements: Option<SymbolRef>,
}

/// One change to apply to a record.
enum Change {
    Extends(usize, SymbolRef),
    Implements(usize, SymbolRef),
    Container(usize, SymbolRef, bool),
}

/// Fills `extends`/`implements` on the selected records and, when
/// `containment` is asked, links each Rust impl member to its self type
/// (`relations.container`, and `parent` when the self type resolves).
pub(super) fn link_relations(
    records: &mut [SymbolRecord],
    inputs: &[RelationInput<'_>],
    root: Node<'_>,
    language: ProgrammingLanguage,
    source: &str,
    containment: bool,
) {
    count_relation_link();
    let changes = {
        let declarations = Declarations::new(records, inputs, language);
        let mut changes = Vec::new();
        written_bases(&declarations, inputs, source, &mut changes);
        if language == ProgrammingLanguage::Rust {
            impl_relations(&declarations, inputs, root, source, containment, &mut changes);
        }
        changes
    };
    for change in changes {
        match change {
            Change::Extends(idx, target) => push_unique(&mut records[idx].relations.extends, target),
            Change::Implements(idx, target) => push_unique(&mut records[idx].relations.implements, target),
            Change::Container(idx, container, resolved) => {
                let relations = &mut records[idx].relations;
                relations.parent = resolved.then(|| container.clone());
                relations.container = Some(container);
            }
        }
    }
}

/// Equal written expressions appear once per list; distinct
/// instantiations of one target stay distinct.
fn push_unique(list: &mut Vec<SymbolRef>, target: SymbolRef) {
    if !list.iter().any(|existing| existing.name == target.name) {
        list.push(target);
    }
}

/// The bases each selected type-like record writes in its own header.
fn written_bases(
    declarations: &Declarations<'_>,
    inputs: &[RelationInput<'_>],
    source: &str,
    changes: &mut Vec<Change>,
) {
    let language = declarations.language;
    for (idx, (record, input)) in declarations.records.iter().zip(inputs).enumerate() {
        if !input.wanted || !declares_bases(record.kind) {
            continue;
        }
        let bases = declared_bases(input.name, language, source);
        if bases.extends.is_empty() && bases.implements.is_empty() {
            continue;
        }
        let type_params = visible_type_parameters(declarations, inputs, idx, source);
        let scopes = match crate::analysis::owner_path(&record.identity.qualified_name) {
            Some(owner) => scope_chain(owner),
            None => vec![""],
        };
        let modules = declarations.enclosing_modules(input.declaration);
        for base in &bases.extends {
            let target = declarations.target(base, &scopes, &modules, &type_params, is_base_kind);
            changes.push(Change::Extends(idx, target));
        }
        for base in &bases.implements {
            let target = declarations.target(base, &scopes, &modules, &type_params, is_base_kind);
            changes.push(Change::Implements(idx, target));
        }
    }
}

/// The type parameters a base written by record `idx` can name: its own,
/// then those of every declaration around it (a C++ member class of a
/// template, a Java inner class of a generic class).
fn visible_type_parameters(
    declarations: &Declarations<'_>,
    inputs: &[RelationInput<'_>],
    idx: usize,
    source: &str,
) -> Vec<String> {
    let language = declarations.language;
    let mut names: Vec<String> = declarations.records[idx]
        .type_info
        .generic_params
        .iter()
        .map(|param| param.name.clone())
        .collect();
    let mut previous = idx;
    let mut ancestor = inputs[idx].declaration.parent();
    while let Some(current) = ancestor {
        if let Some(&owner) = declarations.scopes_by_node.get(&current.id())
            && owner != idx
            && owner != previous
        {
            let input = &inputs[owner];
            if let Ok(generics) = declared_generics(input.declaration, input.name, language, source) {
                names.extend(generics.params.into_iter().map(|param| param.name));
            }
            previous = owner;
        }
        ancestor = current.parent();
    }
    names
}

/// Rust impls, traversed once: each positive trait impl's trait goes to the
/// `implements` of its selected members and of its uniquely resolved self
/// type, in file order; inherent and negative impls add no relation. Under
/// Containment every record whose container is an impl's self type (its
/// members, and what the container search places there from inside them) is
/// linked to the resolved self type, or, when it does not resolve, gets a
/// container named by the self type as written and no parent.
fn impl_relations(
    declarations: &Declarations<'_>,
    inputs: &[RelationInput<'_>],
    root: Node<'_>,
    source: &str,
    containment: bool,
    changes: &mut Vec<Change>,
) {
    let mut impls: HashMap<usize, ResolvedImpl> = HashMap::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "impl_item"
            && let Some(resolved) = resolve_impl(declarations, node, source)
        {
            if let (Some(self_type), Some(target)) = (resolved.self_type, &resolved.implements)
                && inputs[self_type].wanted
            {
                changes.push(Change::Implements(self_type, target.clone()));
            }
            impls.insert(node.id(), resolved);
        }
        // Children in reverse so impls are met in source order.
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
        stack.extend(children.into_iter().rev());
    }
    if impls.is_empty() {
        return;
    }
    for (idx, input) in inputs.iter().enumerate() {
        let Some(impl_item) = input.container_impl else {
            continue;
        };
        let Some(resolved) = impls.get(&impl_item.id()) else {
            continue;
        };
        // Only the impl's own members state which trait supplied them.
        if input.wanted
            && impl_of_member(input.declaration).is_some_and(|item| item.id() == impl_item.id())
            && let Some(target) = &resolved.implements
        {
            changes.push(Change::Implements(idx, target.clone()));
        }
        if containment {
            let container = match resolved.self_type {
                Some(owner) => {
                    let owner = &declarations.records[owner];
                    SymbolRef {
                        id: Some(owner.id.clone()),
                        name: owner.identity.name.clone(),
                        qualified_name: Some(owner.identity.qualified_name.clone()),
                    }
                }
                None => SymbolRef { id: None, name: resolved.header.self_text.clone(), qualified_name: None },
            };
            changes.push(Change::Container(idx, container, resolved.self_type.is_some()));
        }
    }
}

fn resolve_impl(declarations: &Declarations<'_>, impl_item: Node<'_>, source: &str) -> Option<ResolvedImpl> {
    let header = impl_header(impl_item, source)?;
    let scope = declarations
        .enclosing_scope(impl_item)
        .map_or("", |idx| declarations.records[idx].identity.qualified_name.as_str());
    let scopes = scope_chain(scope);
    let modules = declarations.enclosing_modules(impl_item);
    let self_type = header.self_path.as_ref().and_then(|path| {
        declarations.resolve(path, &scopes, &modules, &header.type_params, is_self_type_kind)
    });
    let implements = header
        .trait_base
        .as_ref()
        .filter(|_| !header.is_negative)
        .map(|base| declarations.target(base, &scopes, &modules, &header.type_params, is_trait_kind));
    Some(ResolvedImpl { header, self_type, implements })
}
