//! The lexical part of a written base's lookup: the scopes inside callables,
//! blocks and anonymous owners, where a declaration's displayed qualified
//! name does not say where it is visible.
//!
//! The resolver decides a relative path's first segment here first, then
//! (when no lexical scope binds it) by in-file qualified name in the named
//! scopes around ([`super::Declarations`]). Lexical scopes are syntax nodes,
//! so a declaration in a finished or sibling block is never in view; each
//! binding carries its name groups (type, value, namespace), so a value binding
//! never hides a type a base clause names in a language that separates them;
//! and bindings no record reports (destructuring and match patterns, Scala
//! pattern `val`s, anonymous owners' type parameters, Python
//! `global`/`nonlocal`) are bindings all the same. Such a binding outside
//! every lexical scope keeps its name groups and its syntax scope when it is
//! handed to the named-scope lookup ([`Unrecorded`]).
//!
//! A Python `global`/`nonlocal` directive redirects reads and writes alike:
//! a write of the name in the directive's scope (an assignment or pattern
//! target, an import, a `class` or `def`) binds at the destination, the
//! module ([`Collected::global_writes`]) or the enclosing function that
//! binds the name, never in the directive's scope. The destination's own
//! declaration is then no longer proved for any reader that sees it, and a
//! redirected declaration (with everything declared inside it) is no
//! member of the class whose body writes it ([`Reach::Redirected`]).
//!
//! The tree is [`ScopeTree`], shared with reference resolution; Rust and
//! TypeScript/JavaScript binding sites come from the same
//! [`LanguageRules::binding_sites`] readers. Built once per file from the one
//! relation walk, without the References pass.

use std::collections::{HashMap, HashSet};

use tree_sitter::Node;

use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::count_shared_context_steps;
use crate::analysis::declaration_names::{python_captures, python_targets, scala_pattern_names};
use crate::analysis::scope::{
    ANY_CATEGORY, BindingRule, BindingSite, BindingTarget, LanguageRules, MACRO, Placement, ScopeTree, TYPE, VALUE,
    ancestors,
};
use crate::file::extract::{
    ImportBinding, RELATION_NAMESPACE, cpp_param_may_name_type, declared_bases, declared_generic_names,
};
use crate::shared::{ProgrammingLanguage, SymbolKindV2};

use super::Declared;

/// Every name group.
pub(super) const ALL: u8 = ANY_CATEGORY | RELATION_NAMESPACE;

/// Where a declaration's name is bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Reach {
    /// In a named scope (file, namespace, module, type outside any
    /// callable): found by in-file qualified name.
    Qualified,
    /// In a lexical scope that is not a type body (a block, a callable, a
    /// Rust module declared in one): found by lexical lookup and, in such a
    /// module, through the module's path as a member of its declaration.
    Local,
    /// In the body of a type declared in a lexical scope: found by lexical
    /// lookup from inside the type and, through its owner's path, only as a
    /// member of that owner's declaration.
    Member,
    /// A generic parameter captured as a declaration (a C++ template
    /// parameter): a binding of its list's owner, never a target.
    GenericParam,
    /// A Python declaration a `global`/`nonlocal` directive of its scope
    /// binds at the directive's destination, or one declared inside such a
    /// declaration: no member of the owner its displayed name spells, and
    /// never found by that name.
    Redirected,
}

/// What a lexical scope is for the lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Plain,
    /// A type's member list; `barrier` when the type inherits member names
    /// that lookup from inside it would find first.
    Body { barrier: bool },
    /// A Rust `mod`: an item inside does not see the names around it.
    Module,
}

/// The outcome of the lexical part of a lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Local {
    /// The one declaration a lexical scope binds the name to.
    Found(usize),
    /// A lexical scope binds the name to no single declaration (an import,
    /// a pattern, a type parameter, several declarations) or may bind it (a
    /// wildcard import, an inherited member type).
    Unresolved,
    /// A Python `global` directive: look the name up at the module, where
    /// the directive's scope also places its writes of the name.
    Global,
    /// No lexical scope binds the name.
    Continue,
}

/// One file's lexical scopes and bindings.
pub(super) struct Lexical<'a> {
    language: ProgrammingLanguage,
    tree: ScopeTree<'a>,
    lexical: Vec<bool>,
    roles: Vec<Role>,
    /// Type parameter names each lexical scope binds (a callable's or a
    /// local type's own list), whatever their position.
    generic_names: HashMap<usize, HashSet<String>>,
    globals: HashSet<(usize, &'a str)>,
    nonlocals: HashSet<(usize, &'a str)>,
    pub(super) placements: Vec<Reach>,
}

/// What the relation walk collected outside lexical scopes.
#[derive(Clone)]
pub(super) struct Collected<'a> {
    /// Import statements in named scopes.
    pub(super) imports: Vec<Node<'a>>,
    /// Names bound outside lexical scopes by a binding no record reports (a
    /// Python module-level tuple assignment, a TS module-level destructured
    /// value, a Scala pattern `val` in a package or object, a Rust generic
    /// parameter of a named item or impl).
    pub(super) unrecorded: Vec<Unrecorded<'a>>,
    pub(super) impls: Vec<Node<'a>>,
    pub(super) extensions: Vec<Node<'a>>,
    /// Names a Python scope writes under a `global` directive: each rebinds
    /// the module's name, so no module declaration of it is proved, before
    /// or after the write.
    pub(super) global_writes: Vec<&'a str>,
    /// Python attribute write targets (`Holder.Base = x`, `Holder.Base +=
    /// x`, `del Holder.Base`, a `for`/`with` attribute target) whose object
    /// is a plain dotted name: each rebinds the member its owner path
    /// names, wherever the write sits.
    pub(super) qualified_writes: Vec<Node<'a>>,
}

/// A binding no record reports, placed in a scope that is not lexical.
#[derive(Clone)]
pub(super) struct Unrecorded<'a> {
    pub(super) name: Node<'a>,
    /// The syntax scope the binding belongs to: the file root, a named
    /// owner (type, module, object), or an owner without a record (a Rust
    /// impl), whose parameters bind only inside it.
    pub(super) scope: Node<'a>,
    /// The same scope's index in the lexical tree ([`Lexical::scopes_from`]).
    pub(super) scope_index: usize,
    /// The name groups it binds: a destructured value or a Rust `const`
    /// parameter is a value and never hides a type.
    pub(super) categories: u8,
}

/// A binding site found by the walk, with its own placement when the
/// language has no shared [`LanguageRules`].
struct Site<'a> {
    site: BindingSite<'a>,
    scope_hint: Option<Node<'a>>,
    directive: Option<Directive>,
}

#[derive(Clone, Copy)]
enum Directive {
    Global,
    Nonlocal,
}

const fn rule(categories: u8, visible_from: usize) -> BindingRule {
    BindingRule {
        categories,
        placement: Placement::Innermost,
        visible_from,
        visible_until: None,
        local: false,
        ordered: false,
        callable: false,
        pattern_constant: false,
    }
}

impl<'a> Lexical<'a> {
    /// Walks the file once: scopes, binding sites, imports, Rust impls and
    /// Swift extensions; then places every binding of a lexical scope.
    pub(super) fn build(
        entries: &[Declared<'_>],
        root: Node<'a>,
        language: ProgrammingLanguage,
        source: &'a str,
        swift_extension: impl Fn(Node<'_>) -> bool,
    ) -> (Self, Collected<'a>) {
        let rules = LanguageRules::for_language(language);
        let mut scope_nodes = Vec::new();
        let mut sites: Vec<Site<'a>> = Vec::new();
        let mut imports = Vec::new();
        let mut collected = Collected {
            imports: Vec::new(),
            unrecorded: Vec::new(),
            impls: Vec::new(),
            extensions: Vec::new(),
            global_writes: Vec::new(),
            qualified_writes: Vec::new(),
        };
        let mut stack = vec![root];
        let mut shared = Vec::new();
        let mut dynamics = Vec::new();
        while let Some(node) = stack.pop() {
            if language == ProgrammingLanguage::Rust && node.kind() == "impl_item" {
                collected.impls.push(node);
            }
            if language == ProgrammingLanguage::Swift && swift_extension(node) {
                collected.extensions.push(node);
            }
            if scope_role(language, node).is_some() {
                scope_nodes.push(node);
            }
            if let Some(binding) = crate::file::extract::import_binding(node, language, source) {
                imports.push((node, binding));
                continue;
            }
            if let Some(rules) = rules {
                shared.clear();
                rules.binding_sites(node, &mut shared);
                dynamics.extend(rules.dynamic(node, source));
                // A bare Rust pattern name (`BindingSite::bare_pattern`) is
                // placed as a binding without deciding whether it reads a
                // constant: it is a value, and no Rust relation lookup reads
                // the value group.
                sites.extend(shared.drain(..).filter(|site| !site.is_import).map(|mut site| {
                    // A type parameter never resolves, in a type or a value
                    // position alike.
                    let parent = site.name.indexed_parent().map(|parent| parent.kind());
                    if parent == Some("type_parameter") && language != ProgrammingLanguage::Rust {
                        site.rule.categories = ALL;
                    }
                    if matches!(parent, Some("internal_module" | "module")) {
                        site.rule.categories |= RELATION_NAMESPACE;
                    }
                    Site { site, scope_hint: None, directive: None }
                }));
            }
            match language {
                ProgrammingLanguage::Python => python_sites(node, source, &mut sites, &mut collected.qualified_writes),
                ProgrammingLanguage::Scala => scala_sites(node, source, &mut sites),
                _ => {}
            }
            let mut cursor = node.walk();
            let children: Vec<Node<'a>> = node.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }

        let mut tree = ScopeTree::with_rules(rules, root, scope_nodes);
        // A `with` body or a function holding a direct `eval` may bind any
        // name: a glob for every lookup that reaches it.
        for dynamic in dynamics {
            tree.add_dynamic(dynamic);
        }
        // One step per scope and per binding site: the walk itself visits
        // each node once, like the parse.
        count_shared_context_steps(tree.len() + sites.len());
        let mut lexical = vec![false; tree.len()];
        let mut roles = vec![Role::Plain; tree.len()];
        for scope in 0..tree.len() {
            let node = tree.node(scope);
            let (role, opens) =
                if scope == 0 { (Role::Plain, false) } else { scope_role(language, node).unwrap_or((Role::Plain, false)) };
            roles[scope] = match role {
                Role::Body { .. } => Role::Body { barrier: body_barrier(language, node, source) },
                other => other,
            };
            lexical[scope] = opens || tree.parent(scope).is_some_and(|parent| lexical[parent]);
        }
        let mut index = Self {
            language,
            tree,
            lexical,
            roles,
            generic_names: HashMap::new(),
            globals: HashSet::new(),
            nonlocals: HashSet::new(),
            placements: vec![Reach::Qualified; entries.len()],
        };
        index.place(entries, source, imports, sites, &mut collected);
        (index, collected)
    }

    fn is_lexical(&self, scope: usize) -> bool {
        self.lexical[scope]
    }

    /// The innermost scope strictly around `node`.
    fn scope_around(&self, node: Node<'_>) -> usize {
        self.tree.scope_around(node)
    }

    /// Whether `node` is (or lies in) a lexical scope.
    pub(super) fn is_lexical_node(&self, node: Node<'_>) -> bool {
        let scope = self.tree.scope_of(node).unwrap_or_else(|| self.scope_around(node));
        self.is_lexical(scope)
    }

    /// The scopes a lookup made from `at` passes, innermost first: `at`
    /// itself when it is a scope, then every scope around it up to the root.
    pub(super) fn scopes_from(&self, at: Node<'_>) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(self.start(at)), |&scope| self.tree.parent(scope))
    }

    fn place(
        &mut self,
        entries: &[Declared<'_>],
        source: &'a str,
        imports: Vec<(Node<'a>, ImportBinding<'a>)>,
        sites: Vec<Site<'a>>,
        collected: &mut Collected<'a>,
    ) {
        let text = |node: Node<'_>| source.get(node.start_byte()..node.end_byte()).unwrap_or_default();
        // Directives first: Python requires one before any use of its name
        // in the scope, but the walk meets a nested scope's writes earlier.
        // A module-level directive changes nothing.
        for Site { site, directive, .. } in &sites {
            let Some(directive) = directive else { continue };
            let scope = self.tree.placement(site.name, site.rule);
            if scope == 0 {
                continue;
            }
            match directive {
                Directive::Global => self.globals.insert((scope, text(site.name))),
                Directive::Nonlocal => self.nonlocals.insert((scope, text(site.name))),
            };
        }
        let mut nonlocal_writes: Vec<(usize, &'a str)> = Vec::new();
        for (node, binding) in imports {
            let scope = self.scope_around(node);
            let binding = match binding {
                ImportBinding::Names(names, categories) => ImportBinding::Names(
                    names
                        .into_iter()
                        .filter(|name| !self.redirect(scope, name, collected, &mut nonlocal_writes))
                        .collect(),
                    categories,
                ),
                other => other,
            };
            if !self.is_lexical(scope) {
                // The named-scope walk reads the import again, redirected
                // names included: a further binding of a name that is
                // already unproved.
                collected.imports.push(node);
                continue;
            }
            match binding {
                ImportBinding::Names(names, categories) => {
                    for name in names {
                        self.tree.bind_in(scope, 0, name, rule(categories, 0), BindingTarget::Import);
                    }
                }
                ImportBinding::Opaque | ImportBinding::Module => self.tree.add_glob(node),
            }
        }

        let by_range: HashMap<(usize, usize), usize> = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| ((entry.name.start_byte(), entry.name.end_byte()), idx))
            .collect();
        let mut placed: HashSet<(usize, usize)> = HashSet::new();
        for Site { site, scope_hint, directive } in sites {
            if directive.is_some() {
                continue;
            }
            let range = (site.name.start_byte(), site.name.end_byte());
            if !placed.insert(range) {
                continue;
            }
            let scope = match scope_hint {
                Some(node) => self.tree.scope_of(node).unwrap_or_else(|| self.scope_around(node)),
                None => self.tree.placement(site.name, site.rule),
            };
            let record = by_range.get(&range).copied();
            if self.redirect(scope, text(site.name), collected, &mut nonlocal_writes) {
                if let Some(idx) = record {
                    self.placements[idx] = Reach::Redirected;
                }
                continue;
            }
            if !self.is_lexical(scope) {
                if record.is_none() {
                    collected.unrecorded.push(Unrecorded {
                        name: site.name,
                        scope: self.tree.node(scope),
                        scope_index: scope,
                        categories: site.rule.categories,
                    });
                }
                continue;
            }
            let target = record.map_or(BindingTarget::Unreported, BindingTarget::Record);
            if let Some(idx) = record {
                self.placements[idx] = self.local_placement(scope);
            }
            self.tree.bind_in(scope, site.name.start_byte(), text(site.name), site.rule, target);
        }

        for (idx, entry) in entries.iter().enumerate() {
            let range = (entry.name.start_byte(), entry.name.end_byte());
            if is_generic_parameter(entry) {
                self.placements[idx] = Reach::GenericParam;
                let scope = self.scope_around(entry.declaration);
                if self.is_lexical(scope) && !placed.contains(&range) {
                    let categories = match entry.kind {
                        SymbolKindV2::Type | SymbolKindV2::Class | SymbolKindV2::TypeAlias => ALL,
                        _ if cpp_param_may_name_type(entry.declaration) => ALL,
                        _ => VALUE,
                    };
                    self.tree.bind_in(scope, 0, text(entry.name), rule(categories, 0), BindingTarget::Unreported);
                }
                continue;
            }
            let own = self.tree.scope_of(entry.declaration).or_else(|| self.tree.scope_of(entry.anchor));
            if let Some(own) = own
                && self.is_lexical(own)
            {
                let names = declared_generic_names(entry.declaration, entry.name, self.language, source);
                if !names.is_empty() {
                    self.generic_names.entry(own).or_default().extend(names.into_iter().map(|name| crate::analysis::name_identity::key(self.language, &name).into_owned()));
                }
            }
            if placed.contains(&range) {
                continue;
            }
            let scope = self.scope_around(entry.declaration);
            if self.redirect(scope, text(entry.name), collected, &mut nonlocal_writes) {
                self.placements[idx] = Reach::Redirected;
                continue;
            }
            if !self.is_lexical(scope) {
                continue;
            }
            let owner_kind = self.tree.node(scope).kind();
            // Associated items, fields and variants of a Rust type are
            // never lexical names.
            if self.language == ProgrammingLanguage::Rust
                && matches!(owner_kind, "impl_item" | "trait_item" | "struct_item" | "enum_item" | "union_item")
            {
                self.placements[idx] = Reach::Member;
                continue;
            }
            self.placements[idx] = self.local_placement(scope);
            let visible_from = self.visible_from(entry, scope);
            self.tree.bind_in(
                scope,
                entry.name.start_byte(),
                text(entry.name),
                rule(kind_categories(self.language, entry.kind), visible_from),
                BindingTarget::Record(idx),
            );
        }

        for (scope, name) in nonlocal_writes {
            if let Some(destination) = self.nonlocal_destination(scope, name) {
                self.tree.bind_in(destination, 0, name, rule(ALL, 0), BindingTarget::Unreported);
            }
        }
        self.redirect_descendants(entries);
    }

    /// Marks every declaration inside a redirected one as redirected too:
    /// its displayed name runs through an owner that does not have it as a
    /// member. One sort of the redirected ranges, then one lookup per entry.
    fn redirect_descendants(&mut self, entries: &[Declared<'_>]) {
        let mut ranges: Vec<(usize, usize)> = self
            .placements
            .iter()
            .zip(entries)
            .filter(|(reach, _)| **reach == Reach::Redirected)
            .map(|(_, entry)| (entry.declaration.start_byte(), entry.declaration.end_byte()))
            .collect();
        if ranges.is_empty() {
            return;
        }
        ranges.sort_unstable();
        // Syntax ranges nest or are disjoint: keep the outermost ones.
        let mut outermost: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
        for range in ranges {
            if outermost.last().is_none_or(|&(_, end)| range.0 >= end) {
                outermost.push(range);
            }
        }
        count_shared_context_steps(entries.len() + outermost.len());
        for (idx, entry) in entries.iter().enumerate() {
            let (start, end) = (entry.declaration.start_byte(), entry.declaration.end_byte());
            let before = outermost.partition_point(|&(range_start, _)| range_start < start);
            let inside = before > 0 && outermost[before - 1].1 >= end;
            if inside && self.placements[idx] != Reach::GenericParam {
                self.placements[idx] = Reach::Redirected;
            }
        }
    }

    /// Whether the scope `member` (a declaration this file records as
    /// `idx`) is placed in binds `name` in the given groups only to that
    /// declaration: no other declaration, no binding no record reports
    /// (a pattern target, an augmented assignment, a deletion), and, where
    /// an import in a body binds a member of its owner (`imports`), no
    /// import of the name nor (`globs`) a wildcard import. Read for a
    /// lexical owner's member, whose body is a scope of the tree.
    pub(super) fn binds_only(
        &self,
        member: Node<'_>,
        idx: usize,
        name: &str,
        categories: u8,
        imports: bool,
        globs: bool,
    ) -> bool {
        let scope = self.scope_around(member);
        if globs && self.tree.has_glob(scope) {
            return false;
        }
        let Some(found) = self.tree.visible_in(scope, usize::MAX, name, categories) else { return false };
        found.iter().all(|&binding| match self.tree.binding(binding).target {
            BindingTarget::Record(record) => record == idx,
            BindingTarget::Import => !imports,
            BindingTarget::Unreported | BindingTarget::Unknown => false,
        })
    }

    /// Whether a write of `name` in `scope` binds elsewhere, under a Python
    /// directive of that scope: a `global` write is kept for the module, a
    /// `nonlocal` one for [`Self::nonlocal_destination`] once every binding
    /// is placed.
    fn redirect(
        &self,
        scope: usize,
        name: &'a str,
        collected: &mut Collected<'a>,
        nonlocal_writes: &mut Vec<(usize, &'a str)>,
    ) -> bool {
        if self.globals.contains(&(scope, name)) {
            collected.global_writes.push(name);
            true
        } else if self.nonlocals.contains(&(scope, name)) {
            nonlocal_writes.push((scope, name));
            true
        } else {
            false
        }
    }

    /// The scope a Python `nonlocal` directive of `scope` names: the nearest
    /// enclosing function that binds `name` itself (class scopes and
    /// functions with their own `nonlocal` for it are passed). `None` when
    /// there is none, which Python rejects.
    fn nonlocal_destination(&self, scope: usize, name: &str) -> Option<usize> {
        let mut current = self.tree.parent(scope);
        while let Some(candidate) = current {
            count_shared_context_steps(1);
            if self.tree.node(candidate).kind() == "function_definition"
                && !self.nonlocals.contains(&(candidate, name))
            {
                if self.globals.contains(&(candidate, name)) {
                    return None;
                }
                if self.tree.visible_in(candidate, usize::MAX, name, ALL).is_some() {
                    return Some(candidate);
                }
            }
            current = self.tree.parent(candidate);
        }
        None
    }

    fn local_placement(&self, scope: usize) -> Reach {
        match self.roles[scope] {
            Role::Body { .. } => Reach::Member,
            _ => Reach::Local,
        }
    }

    /// Where a lexical declaration becomes visible: a Go local type at its
    /// name and a Go variable or constant after its declaration, a C++
    /// declaration at its name, a Java local class at its name. Everything
    /// else is visible throughout its scope (Rust items, Swift and Scala
    /// local types, hoisted or temporal-dead-zone ECMA declarations).
    fn visible_from(&self, entry: &Declared<'_>, scope: usize) -> usize {
        match self.language {
            ProgrammingLanguage::Go
                if matches!(
                    entry.kind,
                    SymbolKindV2::Type | SymbolKindV2::TypeAlias | SymbolKindV2::Interface | SymbolKindV2::Class
                ) =>
            {
                entry.name.start_byte()
            }
            ProgrammingLanguage::Go => entry.declaration.end_byte(),
            ProgrammingLanguage::Cpp => entry.name.start_byte(),
            ProgrammingLanguage::Java if !matches!(self.roles[scope], Role::Body { .. }) => entry.name.start_byte(),
            _ => 0,
        }
    }

    /// The lexical scope a lookup made from `at` starts in: `at` itself when
    /// it is a scope (a declaration's own type parameters are in view from
    /// its header), else the innermost scope around it.
    fn start(&self, at: Node<'_>) -> usize {
        self.tree.scope_of(at).unwrap_or_else(|| self.scope_around(at))
    }

    /// Looks `first` up in the given name groups through the lexical scopes
    /// around `at` (a declaration, impl or extension; `own` when its own
    /// scope is where the lookup starts), at byte `position`.
    pub(super) fn lookup(&self, first: &str, at: Node<'_>, position: usize, categories: u8) -> Local {
        let mut scope = self.start(at);
        let own = self.tree.scope_of(at);
        let mut passed_other = false;
        while self.is_lexical(scope) {
            count_shared_context_steps(1);
            let skip = match self.roles[scope] {
                // Members are properties in JS/TS; a Python class body is
                // seen only by code written directly in it.
                Role::Body { .. } => {
                    matches!(self.language, ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript)
                        || (self.language == ProgrammingLanguage::Python && passed_other)
                }
                _ => false,
            };
            if !skip {
                if self.globals.contains(&(scope, first)) {
                    return Local::Global;
                }
                if !self.nonlocals.contains(&(scope, first)) {
                    if self.generic_names.get(&scope).is_some_and(|names| names.contains(crate::analysis::name_identity::key(self.language, first).as_ref())) {
                        return Local::Unresolved;
                    }
                    if let Some(found) = self.tree.visible_in(scope, position, first, categories) {
                        return match found.as_slice() {
                            [only] => match self.tree.binding(*only).target {
                                BindingTarget::Record(idx) => Local::Found(idx),
                                _ => Local::Unresolved,
                            },
                            _ => Local::Unresolved,
                        };
                    }
                }
                if self.tree.has_glob(scope) {
                    return Local::Unresolved;
                }
                match self.roles[scope] {
                    Role::Body { barrier: true } | Role::Module => return Local::Unresolved,
                    _ => {}
                }
            }
            if Some(scope) != own {
                passed_other = true;
            }
            let Some(parent) = self.tree.parent(scope) else { break };
            scope = parent;
        }
        Local::Continue
    }

    /// Whether any lexical scope around `at` binds `first` in the given
    /// groups at `position`, or may (a wildcard import); nothing ends the
    /// walk early, so `false` proves no lexical binding exists.
    pub(super) fn may_bind(&self, first: &str, at: Node<'_>, position: usize, categories: u8) -> bool {
        let mut scope = self.start(at);
        while self.is_lexical(scope) {
            count_shared_context_steps(1);
            if self.generic_names.get(&scope).is_some_and(|names| names.contains(crate::analysis::name_identity::key(self.language, first).as_ref()))
                || self.globals.contains(&(scope, first))
                || self.tree.visible_in(scope, position, first, categories).is_some()
                || self.tree.has_glob(scope)
            {
                return true;
            }
            let Some(parent) = self.tree.parent(scope) else { break };
            scope = parent;
        }
        false
    }
}

/// Whether a declaration is a generic parameter: its declaration is an
/// entry of a type or template parameter list.
fn is_generic_parameter(entry: &Declared<'_>) -> bool {
    const LISTS: &[&str] = &["template_parameter_list", "type_parameters", "type_parameter_list"];
    LISTS.contains(&entry.declaration.kind())
        || entry.declaration.indexed_parent().is_some_and(|parent| LISTS.contains(&parent.kind()))
}

/// The name groups a declaration of `kind` binds in `language`: a type
/// lookup (a base clause) never sees a value, and a value lookup (a JS/TS
/// class `extends`) never sees a type. Languages with one name group (Go,
/// Python) bind every group.
pub(super) fn kind_categories(language: ProgrammingLanguage, kind: SymbolKindV2) -> u8 {
    use SymbolKindV2 as K;
    let type_like = matches!(
        kind,
        K::Class | K::Interface | K::Trait | K::Type | K::TypeAlias | K::Enum | K::Module | K::Namespace
    );
    match language {
        ProgrammingLanguage::Rust => match kind {
            K::Macro => MACRO,
            _ if type_like => TYPE,
            _ => VALUE,
        },
        ProgrammingLanguage::Cpp
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Php => {
            if type_like {
                TYPE
            } else {
                VALUE
            }
        }
        // An `object` and a package are terms: a stable path's qualifier.
        ProgrammingLanguage::Scala => match kind {
            K::Enum => TYPE | VALUE,
            K::Module | K::Namespace => VALUE,
            _ if type_like => TYPE,
            _ => VALUE,
        },
        ProgrammingLanguage::TypeScript => match kind {
            K::Class | K::Enum => TYPE | VALUE,
            K::Interface | K::TypeAlias | K::Type | K::Trait => TYPE,
            K::Module | K::Namespace => RELATION_NAMESPACE | VALUE,
            _ => VALUE,
        },
        ProgrammingLanguage::JavaScript => VALUE,
        _ => ALL,
    }
}

/// Whether `node` opens a lexical region of `language` (a callable, a
/// block, an anonymous owner): what it declares is no member of the type or
/// module around it.
pub(super) fn opens_lexical_region(language: ProgrammingLanguage, node: Node<'_>) -> bool {
    scope_role(language, node).is_some_and(|(_, opens)| opens)
}

/// The role of `node` when it is a relation scope of `language`, and
/// whether it opens a lexical region (a callable, a block, an anonymous
/// owner). C#, PHP and the languages without declared bases have no lexical
/// scope a base can be written in.
fn scope_role(language: ProgrammingLanguage, node: Node<'_>) -> Option<(Role, bool)> {
    let kind = node.kind();
    let parent = || node.indexed_parent().map(|parent| parent.kind()).unwrap_or_default();
    let body = Role::Body { barrier: false };
    let opens = |kinds: &[&str]| kinds.contains(&kind);
    let found = match language {
        ProgrammingLanguage::Rust => match kind {
            "function_item" | "closure_expression" | "block" => (Role::Plain, true),
            "impl_item" | "trait_item" | "struct_item" | "enum_item" | "union_item" | "type_item"
            | "function_signature_item" => (Role::Plain, false),
            "mod_item" => (Role::Module, false),
            _ => return None,
        },
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => match kind {
            "class_body" => (body, false),
            // A namespace body is a named scope.
            "statement_block" if matches!(parent(), "internal_module" | "module") => return None,
            _ if node.indexed_parent().is_some_and(|parent| {
                parent.kind() == "with_statement"
                    && parent.child_by_field_name("body").is_some_and(|body| body.id() == node.id())
            }) =>
            {
                (Role::Plain, true)
            }
            _ if opens(&[
                "function_declaration", "function_expression", "function", "generator_function",
                "generator_function_declaration", "arrow_function", "method_definition", "class_static_block",
                "statement_block", "for_statement", "for_in_statement", "catch_clause", "switch_body", "class",
            ]) => (Role::Plain, true),
            "class_declaration" | "abstract_class_declaration" | "interface_declaration" | "type_alias_declaration"
            | "function_signature" | "method_signature" => (Role::Plain, false),
            _ => return None,
        },
        ProgrammingLanguage::Python => match kind {
            "block" if parent() == "class_definition" => (body, false),
            "class_definition" => (Role::Plain, false),
            _ if opens(&[
                "function_definition", "lambda", "list_comprehension", "set_comprehension",
                "dictionary_comprehension", "generator_expression",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        ProgrammingLanguage::Go => match kind {
            _ if opens(&[
                "function_declaration", "method_declaration", "func_literal", "block", "for_statement",
                "if_statement", "expression_switch_statement", "type_switch_statement", "select_statement",
                "expression_case", "type_case", "default_case", "communication_case",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        ProgrammingLanguage::Cpp => match kind {
            "field_declaration_list" => (body, false),
            "struct_specifier" | "class_specifier" | "union_specifier" => {
                // An unnamed class is an anonymous owner.
                (Role::Plain, node.child_by_field_name("name").is_none())
            }
            _ if opens(&[
                "function_definition", "lambda_expression", "compound_statement", "for_statement",
                "for_range_loop", "while_statement", "if_statement", "switch_statement", "catch_clause",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        ProgrammingLanguage::Java => match kind {
            "class_body" | "interface_body" | "enum_body" | "annotation_type_body" => (body, false),
            "class_declaration" | "interface_declaration" | "enum_declaration" | "record_declaration"
            | "annotation_type_declaration" => (Role::Plain, false),
            "object_creation_expression" => {
                (Role::Plain, node.named_children(&mut node.walk()).any(|child| child.kind() == "class_body"))
            }
            _ if opens(&[
                "method_declaration", "constructor_declaration", "compact_constructor_declaration",
                "lambda_expression", "block", "constructor_body", "static_initializer", "for_statement",
                "enhanced_for_statement", "catch_clause", "switch_block_statement_group",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        ProgrammingLanguage::Swift => match kind {
            "class_body" | "protocol_body" | "enum_class_body" => (body, false),
            "class_declaration" | "protocol_declaration" => (Role::Plain, false),
            _ if opens(&[
                "function_declaration", "init_declaration", "deinit_declaration", "lambda_literal", "statements",
                "computed_property", "subscript_declaration",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        ProgrammingLanguage::Scala => match kind {
            "template_body" | "enum_body" => (body, false),
            "class_definition" | "object_definition" | "trait_definition" | "enum_definition" => {
                (Role::Plain, false)
            }
            _ if opens(&[
                "function_definition", "lambda_expression", "block", "indented_block", "case_clause",
                "for_expression", "instance_expression",
            ]) => (Role::Plain, true),
            _ => return None,
        },
        _ => return None,
    };
    Some(found)
}

/// Whether the type owning `body` inherits member names that unqualified
/// lookup from inside it finds before anything outside (Java, C++, C#,
/// Swift, Scala): it writes a base, or it is an anonymous class created from
/// one.
fn body_barrier(language: ProgrammingLanguage, body: Node<'_>, source: &str) -> bool {
    if !matches!(
        language,
        ProgrammingLanguage::Java
            | ProgrammingLanguage::Cpp
            | ProgrammingLanguage::CSharp
            | ProgrammingLanguage::Swift
            | ProgrammingLanguage::Scala
    ) {
        return false;
    }
    let Some(owner) = body.indexed_parent() else { return false };
    if let Some(name) = owner.child_by_field_name("name") {
        let bases = declared_bases(name, language, source);
        return !(bases.extends.is_empty() && bases.implements.is_empty() && bases.unclassified.is_empty());
    }
    match owner.kind() {
        "object_creation_expression" | "instance_expression" => true,
        _ => owner.named_children(&mut owner.walk()).any(|child| child.kind() == "base_class_clause"),
    }
}

/// Python binding sites no record reports: assignment (augmented
/// included), `del`, `for`, `with`, `except` and comprehension targets
/// (every name of a nested or starred pattern), walrus names, `match`
/// captures, and `global`/`nonlocal` directives. A name binds in the innermost function, lambda,
/// comprehension or class body around it (a walrus skips comprehensions).
/// An attribute target of the same statements is a qualified write
/// (`writes`, [`Collected::qualified_writes`]).
fn python_sites<'a>(node: Node<'a>, source: &str, sites: &mut Vec<Site<'a>>, writes: &mut Vec<Node<'a>>) {
    let mut bind = |name: Node<'a>, hint: Option<Node<'a>>, directive: Option<Directive>| {
        sites.push(Site { site: BindingSite { name, rule: rule(ALL, 0), is_import: false, bare_pattern: None }, scope_hint: hint, directive });
    };
    match node.kind() {
        "assignment" | "augmented_assignment" | "for_statement" | "for_in_clause" => {
            python_targets(node.child_by_field_name("left"), &mut |name| bind(name, None, None), writes);
        }
        "delete_statement" => {
            let mut cursor = node.walk();
            let targets: Vec<Node<'a>> = node.named_children(&mut cursor).collect();
            for target in targets {
                python_targets(Some(target), &mut |name| bind(name, None, None), writes);
            }
        }
        "as_pattern" => {
            // `with … as target`, `except E as name`; a `case … as name`
            // alias is read with its pattern.
            if !ancestors(node).any(|ancestor| ancestor.kind() == "case_clause") {
                python_targets(node.child_by_field_name("alias"), &mut |name| bind(name, None, None), writes);
            }
        }
        "named_expression" => {
            if let Some(name) = node.child_by_field_name("name") {
                let hint = ancestors(node).find(|ancestor| {
                    matches!(ancestor.kind(), "function_definition" | "lambda" | "module")
                        || (ancestor.kind() == "block"
                            && ancestor.indexed_parent().is_some_and(|parent| parent.kind() == "class_definition"))
                });
                bind(name, hint, None);
            }
        }
        "case_clause" => {
            let mut cursor = node.walk();
            let patterns: Vec<Node<'a>> =
                node.named_children(&mut cursor).filter(|child| child.kind() == "case_pattern").collect();
            for pattern in patterns {
                python_captures(pattern, source, &mut |name| bind(name, None, None));
            }
        }
        "global_statement" | "nonlocal_statement" => {
            let directive = if node.kind() == "global_statement" { Directive::Global } else { Directive::Nonlocal };
            let mut cursor = node.walk();
            let names: Vec<Node<'a>> = node.named_children(&mut cursor).filter(|child| child.kind() == "identifier").collect();
            for name in names {
                bind(name, None, Some(directive));
            }
        }
        _ => {}
    }
}

/// Scala binding sites no record reports: the names a pattern `val`/`var`,
/// a `for` generator or a `case` binds ([`scala_pattern_names`]), and a
/// polymorphic lambda's type parameters.
fn scala_sites<'a>(node: Node<'a>, source: &str, sites: &mut Vec<Site<'a>>) {
    let mut bind = |name: Node<'a>, categories: u8| {
        sites.push(Site { site: BindingSite { name, rule: rule(categories, 0), is_import: false, bare_pattern: None }, scope_hint: None, directive: None });
    };
    match node.kind() {
        "val_definition" | "var_definition" => {
            scala_pattern_names(node.child_by_field_name("pattern"), source, &mut |name| bind(name, VALUE));
        }
        "case_clause" => scala_pattern_names(node.child_by_field_name("pattern"), source, &mut |name| bind(name, VALUE)),
        "enumerator" => {
            let mut cursor = node.walk();
            let binds = node.children(&mut cursor).any(|child| matches!(child.kind(), "<-" | "="));
            if binds {
                scala_pattern_names(node.named_child(0), source, &mut |name| bind(name, VALUE));
            }
        }
        "lambda_expression" => {
            if let Some(list) = node.child_by_field_name("type_parameters") {
                let mut cursor = list.walk();
                let names: Vec<Node<'a>> = list.children_by_field_name("name", &mut cursor).collect();
                for name in names {
                    bind(name, ALL);
                }
                let mut cursor = list.walk();
                let variant: Vec<Node<'a>> = list
                    .named_children(&mut cursor)
                    .filter(|child| matches!(child.kind(), "covariant_type_parameter" | "contravariant_type_parameter"))
                    .filter_map(|child| child.child_by_field_name("name"))
                    .collect();
                for name in variant {
                    bind(name, ALL);
                }
            }
        }
        _ => {}
    }
}
