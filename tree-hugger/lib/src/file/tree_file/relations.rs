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

use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::analysis::counters::{count_relation_link, count_shared_context_steps};
use crate::analysis::qualifying_prefixes;
use crate::file::extract::{
    BasePath, ImplHeader, ImportBinding, PathAnchor, WrittenBase, declared_bases,
    declared_generics, impl_header, impl_of_member, impl_type_parameters, import_binding,
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

/// Kinds a path may pass through to a member: a callable, variable,
/// parameter or field is a runtime or lexical owner, not a namespace its
/// declarations are reachable through.
fn is_path_owner_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Module
            | SymbolKindV2::Namespace
            | SymbolKindV2::Class
            | SymbolKindV2::Interface
            | SymbolKindV2::Type
            | SymbolKindV2::Enum
            | SymbolKindV2::Trait
    )
}

fn is_type_like_kind(kind: SymbolKindV2) -> bool {
    matches!(
        kind,
        SymbolKindV2::Class | SymbolKindV2::Interface | SymbolKindV2::Type | SymbolKindV2::Enum | SymbolKindV2::Trait
    )
}

/// Whether a class scope provides lexical names to a lookup made inside
/// it: never in JS/TS (members are properties); in Python only to code run
/// directly in that class body (`innermost`: the header being resolved sits
/// in it), never to a nested class, method or function, so an enclosing
/// class's classes, variables and imports are all skipped.
fn class_scope_is_lexical(language: ProgrammingLanguage, innermost: bool) -> bool {
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => false,
        ProgrammingLanguage::Python => innermost,
        _ => true,
    }
}

/// Whether a type's inherited members are found by unqualified lookup from
/// inside it, so a type with bases hides what lies outside it.
fn inherits_member_names(language: ProgrammingLanguage) -> bool {
    matches!(language, ProgrammingLanguage::Java | ProgrammingLanguage::Cpp)
}

/// Languages with an [`import_binding`] reader (Rust also needs the walk
/// for its impls).
fn reads_imports(language: ProgrammingLanguage) -> bool {
    matches!(
        language,
        ProgrammingLanguage::Rust
            | ProgrammingLanguage::Cpp
            | ProgrammingLanguage::Php
            | ProgrammingLanguage::Python
            | ProgrammingLanguage::TypeScript
            | ProgrammingLanguage::JavaScript
    )
}

/// The import bindings and (Rust) impl blocks of one file, from one walk of
/// its tree, made on the first lookup that needs it.
#[derive(Default)]
struct FileWalk<'a, 'tree> {
    /// `(scope, name)`: an import in the scope with this in-file qualified
    /// name (`""` for the root) binds `name`.
    imports: HashSet<(&'a str, &'a str)>,
    /// Scopes holding a wildcard import or C++ using-directive.
    opaque: HashSet<&'a str>,
    /// Rust `impl` blocks in source order.
    impls: Vec<Node<'tree>>,
}

/// What the resolver reads of one declaration: its record or, before
/// records exist, its definition.
pub(super) struct Declared<'a, 'tree> {
    pub(super) qualified_name: &'a str,
    pub(super) kind: SymbolKindV2,
    pub(super) declaration: Node<'tree>,
    pub(super) anchor: Node<'tree>,
    pub(super) name: Node<'tree>,
}

/// Every declaration of the file by in-file qualified name, the
/// declarations that name a scope by their nodes, and the scopes that exist
/// only as a qualifier of a declared name (a Java package, a C++ namespace
/// written `a::b`). Built once per file from all declarations, selected or
/// not, in record order.
struct Declarations<'a, 'tree> {
    entries: Vec<Declared<'a, 'tree>>,
    by_qualified: HashMap<&'a str, Vec<usize>>,
    /// Built on the first lookup that needs it.
    implied: OnceCell<HashSet<&'a str>>,
    scopes_by_node: HashMap<usize, usize>,
    language: ProgrammingLanguage,
    root: Node<'tree>,
    source: &'a str,
    walk: OnceCell<FileWalk<'a, 'tree>>,
}

impl<'a, 'tree> Declarations<'a, 'tree> {
    fn new(
        entries: Vec<Declared<'a, 'tree>>,
        root: Node<'tree>,
        language: ProgrammingLanguage,
        source: &'a str,
    ) -> Self {
        let mut by_qualified: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut scopes_by_node = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            by_qualified.entry(entry.qualified_name).or_default().push(idx);
            if is_scope_kind(entry.kind) {
                for node in [entry.declaration, entry.anchor] {
                    scopes_by_node.entry(node.id()).or_insert(idx);
                }
            }
        }
        count_shared_context_steps(entries.len());
        Self {
            entries,
            by_qualified,
            implied: OnceCell::new(),
            scopes_by_node,
            language,
            root,
            source,
            walk: OnceCell::new(),
        }
    }

    /// Every proper qualifying prefix of a record's in-file qualified name.
    fn implied(&self) -> &HashSet<&'a str> {
        self.implied.get_or_init(|| {
            // One pass over each name, like hashing it: no step beyond the
            // record's own.
            self.entries.iter().flat_map(|entry| qualifying_prefixes(entry.qualified_name)).collect()
        })
    }

    fn qualified_name(&self, idx: usize) -> &'a str {
        self.entries[idx].qualified_name
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
                modules.push(self.qualified_name(idx));
            }
            ancestor = current.parent();
        }
        modules
    }

    /// The file's import bindings and Rust impls, walked once. An import
    /// binds in its nearest enclosing scope; a PHP `namespace N;` statement
    /// puts the statements after it in `N`.
    fn walk(&self) -> &FileWalk<'a, 'tree> {
        self.walk.get_or_init(|| {
            let mut walk = FileWalk::default();
            if !reads_imports(self.language) {
                return walk;
            }
            let mut statement_namespace = "";
            let mut stack = vec![self.root];
            while let Some(node) = stack.pop() {
                if self.language == ProgrammingLanguage::Rust && node.kind() == "impl_item" {
                    walk.impls.push(node);
                }
                if self.language == ProgrammingLanguage::Php
                    && node.kind() == "namespace_definition"
                    && node.child_by_field_name("body").is_none()
                {
                    // A namespace record's in-file qualified name is its
                    // name as written.
                    statement_namespace = node
                        .child_by_field_name("name")
                        .and_then(|name| name.utf8_text(self.source.as_bytes()).ok())
                        .map_or("", str::trim);
                }
                if let Some(binding) = import_binding(node, self.language, self.source) {
                    let scope = self.enclosing_scope(node).map_or(statement_namespace, |idx| self.qualified_name(idx));
                    match binding {
                        ImportBinding::Names(names) => walk.imports.extend(names.into_iter().map(|name| (scope, name))),
                        ImportBinding::Opaque => {
                            walk.opaque.insert(scope);
                        }
                    }
                    continue;
                }
                // Children in reverse so nodes are met in source order.
                let mut cursor = node.walk();
                let children: Vec<Node<'tree>> = node.named_children(&mut cursor).collect();
                stack.extend(children.into_iter().rev());
            }
            walk
        })
    }

    /// `prefix` joined with `segment` by the language's path separator.
    fn join(&self, prefix: &str, segment: &str) -> String {
        if prefix.is_empty() {
            segment.to_string()
        } else {
            format!("{prefix}{}{segment}", path_separator(self.language))
        }
    }

    /// Whether `name` is declared or qualifies a declared name.
    fn is_bound(&self, name: &str) -> bool {
        self.by_qualified.contains_key(name) || self.implied().contains(name)
    }

    /// Whether a path may continue through the scope called `name`: every
    /// declaration of that name is a namespace-like owner (and only
    /// namespaces may repeat), or it exists only as a qualifier.
    fn is_traversable(&self, name: &str) -> bool {
        match self.by_qualified.get(name) {
            None => self.implied().contains(name),
            Some(found) => {
                let kinds = || found.iter().map(|&idx| self.entries[idx].kind);
                kinds().all(is_path_owner_kind)
                    && (found.len() == 1
                        || kinds().all(|kind| matches!(kind, SymbolKindV2::Module | SymbolKindV2::Namespace)))
            }
        }
    }

    /// The scope prefix at which a relative path's first segment is bound,
    /// walking `scopes` innermost first, or `None` when no binding can be
    /// proved to be a declaration of this file.
    ///
    /// The first scope that binds the segment decides; outer scopes are
    /// never tried after it. A binding is a declaration (of any kind, so a
    /// parameter, variable or alias shadows), a qualifier of one, or an
    /// import, which never resolves. A scope holding a wildcard import or
    /// using-directive, a Java/C++ type with bases (whose inherited member
    /// types are found first), and in Rust a module (an item does not see
    /// its parent module's names) end the search unresolved. Class scopes
    /// are skipped where they are not lexical: always in JS/TS, and in
    /// Python every class scope but the innermost (`scopes[0]`, the body the
    /// header is written in).
    fn binding_scope<'s>(&self, first: &str, scopes: &[&'s str]) -> Option<&'s str> {
        let walk = self.walk();
        for (position, &prefix) in scopes.iter().enumerate() {
            count_shared_context_steps(1);
            let owners = self.by_qualified.get(prefix).map_or(&[][..], Vec::as_slice);
            let is_class = owners.iter().any(|&idx| is_type_like_kind(self.entries[idx].kind));
            if is_class && !class_scope_is_lexical(self.language, position == 0) {
                continue;
            }
            if walk.opaque.contains(prefix) || walk.imports.contains(&(prefix, first)) {
                return None;
            }
            if self.is_bound(&self.join(prefix, first)) {
                return Some(prefix);
            }
            if inherits_member_names(self.language)
                && owners.iter().any(|&idx| {
                    is_type_like_kind(self.entries[idx].kind) && {
                        let bases = declared_bases(self.entries[idx].name, self.language, self.source);
                        !(bases.extends.is_empty() && bases.implements.is_empty())
                    }
                })
            {
                return None;
            }
            if self.language == ProgrammingLanguage::Rust
                && owners.iter().any(|&idx| self.entries[idx].kind == SymbolKindV2::Module)
            {
                return None;
            }
        }
        None
    }

    /// The record a written path names, if exactly one eligible declaration
    /// of this file is that name.
    ///
    /// The first segment is decided first: a relative path's by
    /// [`Self::binding_scope`] over `scopes` (never when it is a visible
    /// type parameter, `type_params`), a rooted one at the file root
    /// (`crate::`, PHP `\Name`, C++ `::Name`), Rust `self::`/`super::` at
    /// the module `modules` gives; a Rust `::name` path is external and
    /// never resolves. Each following segment is looked up only inside the
    /// previous one, which must be a namespace-like declaration
    /// ([`Self::is_traversable`]), so a function's local class is not
    /// reachable through the function's name. The complete name resolves
    /// only if it is the only declaration of that name and `eligible`
    /// accepts its kind.
    fn resolve(
        &self,
        path: &BasePath,
        scopes: &[&str],
        modules: &[&str],
        type_params: &[String],
        eligible: fn(SymbolKindV2) -> bool,
    ) -> Option<usize> {
        let (first, rest) = path.segments.split_first()?;
        let module_at = |out: usize| match out.cmp(&modules.len()) {
            std::cmp::Ordering::Less => Some(modules[out]),
            std::cmp::Ordering::Equal => Some(""),
            std::cmp::Ordering::Greater => None,
        };
        let base = match path.anchor {
            PathAnchor::Relative => {
                if type_params.contains(first) {
                    return None;
                }
                self.binding_scope(first, scopes)?
            }
            PathAnchor::Root => "",
            PathAnchor::Extern => return None,
            PathAnchor::SelfModule => module_at(0)?,
            PathAnchor::Super(out) => module_at(out)?,
        };
        let mut current = self.join(base, first);
        for segment in rest {
            count_shared_context_steps(1);
            if !self.is_traversable(&current) {
                return None;
            }
            current = self.join(&current, segment);
        }
        match self.by_qualified.get(current.as_str())?.as_slice() {
            [only] if eligible(self.entries[*only].kind) => Some(*only),
            _ => None,
        }
    }

    /// The relation target for `base`; `records` are the declarations'
    /// records.
    fn target(
        &self,
        records: &[SymbolRecord],
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
            .map(|idx| &records[idx]);
        SymbolRef {
            id: resolved.map(|record| record.id.clone()),
            name: base.text.clone(),
            qualified_name: resolved.map(|record| record.identity.qualified_name.clone()),
        }
    }

    /// The type parameter names visible at `node`: `own`, then those of
    /// every declaration around it (a C++ member class of a template, a
    /// Java inner class of a generic class, a Rust item inside a generic
    /// function) and, in Rust, of every enclosing `impl`. `skip` is the
    /// record `node` declares, whose own list `own` already holds.
    fn type_parameters_around(&self, node: Node<'_>, skip: Option<usize>, mut own: Vec<String>) -> Vec<String> {
        let mut previous = skip;
        let mut ancestor = node.parent();
        while let Some(current) = ancestor {
            if let Some(&owner) = self.scopes_by_node.get(&current.id())
                && Some(owner) != previous
            {
                let entry = &self.entries[owner];
                if let Ok(generics) = declared_generics(entry.declaration, entry.name, self.language, self.source) {
                    own.extend(generics.params.into_iter().map(|param| param.name));
                }
                previous = Some(owner);
            } else if self.language == ProgrammingLanguage::Rust && current.kind() == "impl_item" {
                own.extend(impl_type_parameters(current, self.source));
            }
            ancestor = current.parent();
        }
        own
    }
}

/// The scope prefixes a relative path is looked up under from inside the
/// scope named `scope` (an in-file qualified name, `""` for the root),
/// innermost first, ending at the root. A PHP class name is relative to its
/// namespace alone: PHP never falls back to an outer or global namespace
/// for a class.
fn scope_chain(scope: &str, language: ProgrammingLanguage) -> Vec<&str> {
    if language == ProgrammingLanguage::Php {
        return vec![scope];
    }
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
        let entries = records
            .iter()
            .zip(inputs)
            .map(|(record, input)| Declared {
                qualified_name: record.identity.qualified_name.as_str(),
                kind: record.kind,
                declaration: input.declaration,
                anchor: input.anchor,
                name: input.name,
            })
            .collect();
        let declarations = Declarations::new(entries, root, language, source);
        let mut changes = Vec::new();
        written_bases(&declarations, records, inputs, &mut changes);
        if language == ProgrammingLanguage::Rust {
            impl_relations(&declarations, records, inputs, containment, &mut changes);
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
    declarations: &Declarations<'_, '_>,
    records: &[SymbolRecord],
    inputs: &[RelationInput<'_>],
    changes: &mut Vec<Change>,
) {
    let language = declarations.language;
    for (idx, (record, input)) in records.iter().zip(inputs).enumerate() {
        if !input.wanted || !declares_bases(record.kind) {
            continue;
        }
        let bases = declared_bases(input.name, language, declarations.source);
        if bases.extends.is_empty() && bases.implements.is_empty() {
            continue;
        }
        let own = record.type_info.generic_params.iter().map(|param| param.name.clone()).collect();
        let type_params = declarations.type_parameters_around(input.declaration, Some(idx), own);
        let scopes = scope_chain(crate::analysis::owner_path(&record.identity.qualified_name).unwrap_or(""), language);
        let modules = declarations.enclosing_modules(input.declaration);
        for base in &bases.extends {
            let target = declarations.target(records, base, &scopes, &modules, &type_params, is_base_kind);
            changes.push(Change::Extends(idx, target));
        }
        for base in &bases.implements {
            let target = declarations.target(records, base, &scopes, &modules, &type_params, is_base_kind);
            changes.push(Change::Implements(idx, target));
        }
    }
}

/// Rust impls, each resolved once: each positive trait impl's trait goes to
/// the `implements` of its selected members and of its uniquely resolved
/// self type, in file order; inherent and negative impls add no relation.
/// Under Containment every record whose container is an impl's self type
/// (its members, and what the container search places there from inside
/// them) is linked to the resolved self type, or, when it does not resolve,
/// gets a container named by the self type as written and no parent.
fn impl_relations(
    declarations: &Declarations<'_, '_>,
    records: &[SymbolRecord],
    inputs: &[RelationInput<'_>],
    containment: bool,
    changes: &mut Vec<Change>,
) {
    let mut impls: HashMap<usize, ResolvedImpl> = HashMap::new();
    for &node in &declarations.walk().impls {
        if let Some(resolved) = resolve_impl(declarations, records, node) {
            if let (Some(self_type), Some(target)) = (resolved.self_type, &resolved.implements)
                && inputs[self_type].wanted
            {
                changes.push(Change::Implements(self_type, target.clone()));
            }
            impls.insert(node.id(), resolved);
        }
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
                    let owner = &records[owner];
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

/// The impl's self type and trait, resolved by [`Declarations::resolve`]
/// from the impl's scope with every generic parameter visible there (the
/// impl's own and those of the functions, impls and traits around it), so a
/// self type naming an enclosing generic attaches to no declaration.
fn resolve_impl(
    declarations: &Declarations<'_, '_>,
    records: &[SymbolRecord],
    impl_item: Node<'_>,
) -> Option<ResolvedImpl> {
    let lookup = resolve_self_type(declarations, impl_item)?;
    let implements = lookup.header.trait_base.as_ref().filter(|_| !lookup.header.is_negative).map(|base| {
        declarations.target(records, base, &lookup.scopes, &lookup.modules, &lookup.type_params, is_trait_kind)
    });
    Some(ResolvedImpl { header: lookup.header, self_type: lookup.self_type, implements })
}

/// One impl's header, the lookup context its paths resolve in, and the
/// declaration its self type names.
struct ImplLookup<'s> {
    header: ImplHeader,
    self_type: Option<usize>,
    scopes: Vec<&'s str>,
    modules: Vec<&'s str>,
    type_params: Vec<String>,
}

/// The impl's header and the declaration its self type names, if one.
fn resolve_self_type<'a>(declarations: &Declarations<'a, '_>, impl_item: Node<'_>) -> Option<ImplLookup<'a>> {
    let header = impl_header(impl_item, declarations.source)?;
    let scope = declarations.enclosing_scope(impl_item).map_or("", |idx| declarations.qualified_name(idx));
    let scopes = scope_chain(scope, declarations.language);
    let modules = declarations.enclosing_modules(impl_item);
    let type_params = declarations.type_parameters_around(impl_item, None, header.type_params.clone());
    let self_type = header
        .self_path
        .as_ref()
        .and_then(|path| declarations.resolve(path, &scopes, &modules, &type_params, is_self_type_kind));
    Some(ImplLookup { header, self_type, scopes, modules, type_params })
}

/// For each Rust `impl` of the file (by node id), the index in `entries` of
/// the declaration its self type resolves to, by the same decision
/// [`link_relations`] makes; `None` when it does not resolve. The
/// definitions pass reads it for an impl member's container kind, before
/// records exist.
pub(super) fn impl_self_types(
    entries: Vec<Declared<'_, '_>>,
    root: Node<'_>,
    source: &str,
) -> HashMap<usize, Option<usize>> {
    let declarations = Declarations::new(entries, root, ProgrammingLanguage::Rust, source);
    declarations
        .walk()
        .impls
        .iter()
        .filter_map(|&node| Some((node.id(), resolve_self_type(&declarations, node)?.self_type)))
        .collect()
}
