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
//! Containment, which record the impl's members belong to; a Swift
//! extension's one owner decision likewise decides both its conformance and
//! its members' container.

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use tree_sitter::Node;

use crate::file::parents::IndexedParent as _;
use crate::analysis::counters::{count_relation_link, count_shared_context_steps};
use crate::analysis::qualifying_prefixes;
use crate::analysis::scope::{TYPE, VALUE};
use crate::file::extract::{
    BasePath, DeclaredBases, ImplHeader, ImportBinding, PathAnchor, RELATION_NAMESPACE, WrittenBase, base_path, declaration_keyword,
    declared_bases, declared_generic_names, file_namespace_index, impl_header, impl_of_member, impl_type_parameters,
    csharp_arity, csharp_partial_shape, import_binding, is_csharp_struct, namespace_name,
};
use crate::shared::{ProgrammingLanguage, SymbolKindV2, SymbolRecord, SymbolRef};

mod lexical;
mod inline;

use inline::InlineNames;

use lexical::{ALL, Lexical, Local, Reach, kind_categories, opens_lexical_region};

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
    /// The Swift extension whose body declares the record: its container is
    /// the extension's owner as [`Declarations::extension_owners`] decides it.
    pub(super) container_extension: Option<Node<'tree>>,
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
    matches!(language, ProgrammingLanguage::Java | ProgrammingLanguage::Cpp | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift | ProgrammingLanguage::Scala)
}

/// Whether a scope's own declarations are found before its wildcard
/// imports: a C#, Swift, Scala, Java or Go declaration of the file shadows
/// an on-demand import, while a Rust glob, C++ using-directive or Python
/// star import makes the scope's lookup unprovable.
fn local_before_wildcard(language: ProgrammingLanguage) -> bool {
    matches!(
        language,
        ProgrammingLanguage::CSharp
            | ProgrammingLanguage::Swift
            | ProgrammingLanguage::Scala
            | ProgrammingLanguage::Java
            | ProgrammingLanguage::Go
    )
}

/// Whether an import written in an owner's body binds a member of that
/// owner, so a qualified path through the owner cannot prove a same-named
/// declaration: a Python class-body import rebinds the class attribute, and
/// elsewhere an import beside a same-named member is a conflict the
/// compiler rejects. A Scala import in an object or a Swift import never
/// makes a member.
fn imports_bind_members(language: ProgrammingLanguage) -> bool {
    !matches!(language, ProgrammingLanguage::Scala | ProgrammingLanguage::Swift)
}

/// Whether a wildcard import in an owner's body may replace any member: a
/// Python star import runs after (or before) the member's statement, while
/// a Rust glob or C++ using-directive yields to the owner's own
/// declarations in qualified lookup.
fn globs_bind_members(language: ProgrammingLanguage) -> bool {
    language == ProgrammingLanguage::Python
}

/// The name groups a base's last segment is looked up in: a type, except
/// a JavaScript or TypeScript class `extends` clause (`value`), which is an
/// expression; Go and Python have one group.
fn final_categories(language: ProgrammingLanguage, value: bool) -> u8 {
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript if value => VALUE,
        ProgrammingLanguage::JavaScript => VALUE,
        ProgrammingLanguage::Rust
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Scala
        | ProgrammingLanguage::Php
        | ProgrammingLanguage::TypeScript => TYPE,
        _ => ALL,
    }
}

/// The name groups a qualifier (every segment but the last) is looked up
/// in: a module, namespace or type; a term (an `object`, a value) in Scala
/// and in a JS/TS value expression; a namespace in a TypeScript type.
fn qualifier_categories(language: ProgrammingLanguage, value: bool) -> u8 {
    match language {
        ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript if value => VALUE,
        ProgrammingLanguage::JavaScript | ProgrammingLanguage::Scala => VALUE,
        ProgrammingLanguage::TypeScript => RELATION_NAMESPACE,
        ProgrammingLanguage::Rust
        | ProgrammingLanguage::Cpp
        | ProgrammingLanguage::Java
        | ProgrammingLanguage::CSharp
        | ProgrammingLanguage::Swift
        | ProgrammingLanguage::Php => TYPE,
        _ => ALL,
    }
}

/// What one named in-file scope binds besides its declarations.
#[derive(Default)]
struct ScopeFacts<'a> {
    /// Names imported there, with the name groups they are bound in.
    imports: HashMap<Cow<'a, str>, u8>,
    /// Names a binding no record reports binds there (a Python module-level
    /// tuple target, a TS module-level destructured value, a Scala pattern
    /// `val`, a Rust item's generic parameter, a write a Python function
    /// makes under `global`), with the name groups of that binding: it hides
    /// only a lookup in those groups, and never resolves.
    unrecorded: HashMap<Cow<'a, str>, u8>,
    /// A wildcard import, C++ using-directive or Swift whole-module import.
    opaque: bool,
    /// An import whose bound names cannot be read.
    unreadable: bool,
}

impl<'a> ScopeFacts<'a> {
    /// Adds `other`'s bindings: the facts of several fragments of one owner.
    fn absorb(&mut self, other: &ScopeFacts<'a>) {
        for (name, &groups) in &other.imports {
            *self.imports.entry(name.clone()).or_default() |= groups;
        }
        for (name, &groups) in &other.unrecorded {
            *self.unrecorded.entry(name.clone()).or_default() |= groups;
        }
        self.opaque |= other.opaque;
        self.unreadable |= other.unreadable;
    }

    fn imports_name(&self, name: &str, categories: u8, language: ProgrammingLanguage) -> bool {
        self.imports.get(crate::analysis::name_identity::key(language, name).as_ref()).is_some_and(|groups| groups & categories != 0)
    }

    fn binds_unrecorded(&self, name: &str, categories: u8, language: ProgrammingLanguage) -> bool {
        self.unrecorded.get(crate::analysis::name_identity::key(language, name).as_ref()).is_some_and(|groups| groups & categories != 0)
    }
}

/// A named scope's facts are read either for every declaration bound
/// under its name (when they are all one owner group) or for one owner
/// group ([`OwnerGroups`]), where same-named owners compete and the site
/// lies in that one ([`Declarations::anchor_around`]).
#[derive(Clone, Copy)]
enum FactsOf<'s> {
    Named(&'s str),
    Owner(usize),
}

/// What makes repeated named-scope definitions one owner: every definition
/// of one namespace, or the C# `partial` fragments of one type, which agree
/// on category, generic arity and enclosing owner group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MergeKey {
    Namespace,
    Partial { category: u8, arity: usize, enclosing: Option<usize> },
}

/// The owners of one file: which named-scope declarations are one owner
/// (reopened namespace definitions, C# `partial` fragments of one type) and
/// which stand alone. Built once per file; every lookup that selects an
/// owner reads its group, never the fragments again.
struct OwnerGroups<'a> {
    /// Each entry's group, named by its first declaration in record order:
    /// itself for a declaration that merges with nothing.
    of: Vec<usize>,
    /// Whether the entry is a definition that may reopen one owner
    /// ([`MergeKey`]); the others compete with every same-named declaration.
    merges: Vec<bool>,
    /// The number of declarations in each group, by group.
    size: HashMap<usize, u32>,
    /// The groups of several declarations one of which writes bases (read
    /// only where [`inherits_member_names`]).
    with_bases: HashSet<usize>,
    /// C#: the owner group a bound name written with this many generic
    /// arguments denotes, `None` when several groups share the arity (a
    /// class and a struct, two non-`partial` types, a namespace and a type).
    by_arity: HashMap<Cow<'a, str>, HashMap<usize, Option<usize>>>,
}

/// The import bindings, lexical scopes and (Rust) impl blocks of one file,
/// from one walk of its tree, made on the first lookup that needs it.
struct FileWalk<'a> {
    /// The facts of each named in-file scope (`""` for the root) by bound
    /// name: every same-named owner's together.
    named: HashMap<String, ScopeFacts<'a>>,
    /// The same facts by the declaration whose scope holds them: one
    /// owner's alone, read where same-named owners compete.
    by_owner: HashMap<usize, ScopeFacts<'a>>,
    /// The same bindings of owners without a record (a Rust impl's generic
    /// parameters), by owning lexical scope and name: the name groups. They
    /// bind only for a lookup made inside that scope.
    owned: HashMap<usize, HashMap<Cow<'a,str>, u8>>,
    /// The names in `owned`, so a lookup of any other name climbs nothing.
    owned_names: HashSet<Cow<'a,str>>,
    /// Python attribute write targets ([`lexical::Collected::qualified_writes`]).
    qualified_writes: Vec<Node<'a>>,
    /// Rust `impl` blocks in source order.
    impls: Vec<Node<'a>>,
    extensions: Vec<Node<'a>>,
    lexical: Rc<Lexical<'a>>,
}

/// A Rust file's [`Lexical`] walk, built once by the definitions pass
/// ([`impl_self_types`]) and handed to [`link_relations`]. The walk reads
/// only the entries' nodes and kinds ([`walk_key`]); a linker whose entries
/// differ in either builds its own.
pub(crate) struct LexicalWalk<'tree> {
    key: Vec<(usize, usize, usize, SymbolKindV2)>,
    lexical: Rc<Lexical<'tree>>,
    collected: lexical::Collected<'tree>,
}

impl<'tree> LexicalWalk<'tree> {
    fn build(entries: &[Declared<'_>], root: Node<'tree>, source: &'tree str) -> Self {
        let (lexical, collected) = Lexical::build(entries, root, ProgrammingLanguage::Rust, source, |_| false);
        Self { key: walk_key(entries), lexical: Rc::new(lexical), collected }
    }

    fn share(&self) -> Self {
        Self { key: self.key.clone(), lexical: Rc::clone(&self.lexical), collected: self.collected.clone() }
    }
}

/// What [`Lexical::build`] reads of each entry.
fn walk_key(entries: &[Declared<'_>]) -> Vec<(usize, usize, usize, SymbolKindV2)> {
    entries.iter().map(|entry| (entry.name.id(), entry.declaration.id(), entry.anchor.id(), entry.kind)).collect()
}

/// What the resolver reads of one declaration: its record or, before
/// records exist, its definition.
pub(super) struct Declared<'a> {
    pub(super) qualified_name: &'a str,
    pub(super) kind: SymbolKindV2,
    pub(super) declaration: Node<'a>,
    pub(super) anchor: Node<'a>,
    pub(super) name: Node<'a>,
}

/// Where one lookup is made from.
struct Site<'s, 'a> {
    /// The declaration, impl or extension whose header writes the path.
    at: Node<'a>,
    /// The path's first byte.
    position: usize,
    /// The named scopes (bound names, [`Declarations::binding_name`]) around
    /// `at`, innermost first, ending at the root.
    scopes: Vec<&'s str>,
    /// The Rust modules around `at` (their declarations), innermost first.
    modules: Vec<usize>,
    /// Type parameters of the named owners around `at` (and its own, when
    /// it is no lexical scope), shared by every base the owner writes.
    type_params: TypeParams,
    /// The path is a JavaScript/TypeScript class `extends` expression.
    value: bool,
}

/// The type parameter names in view at a site: the declaration's own list,
/// then each enclosing owner's, every owner's list shared by all the
/// declarations inside it.
#[derive(Debug, Clone, Default)]
struct TypeParams {
    layers: Vec<Rc<HashSet<String>>>,
    /// An enclosing Swift extension's owner is not proved (external,
    /// aliased, imported or ambiguous), so its generic parameters and member
    /// types are unknown: any name the lexical scopes leave unbound may be
    /// one of them, never an empty context.
    unknown: bool,
}

impl TypeParams {
    fn contains(&self, name: &str) -> bool {
        self.unknown || self.layers.iter().any(|layer| layer.contains(name))
    }

    fn extend(&mut self, other: TypeParams) {
        self.layers.extend(other.layers);
        self.unknown |= other.unknown;
    }
}

/// Where a path's lookup stands after a segment.
enum Owner {
    /// A named scope, by bound name: what follows is found by bound name.
    Named(String),
    /// A lexical owner, by its declaration: what follows is found among its
    /// own declarations.
    Syntax(usize),
    /// A named-scope owner whose bound name competing owners share, by the
    /// owner group ([`OwnerGroups`]) the site's source lies in or a written
    /// C# arity denotes: what follows is found among that group's own
    /// declarations, and is unproved when the group binds it otherwise
    /// ([`Declarations::owner_rebinds`]).
    Anchored(usize),
}

/// Where [`Declarations::binding_scope`] found a first segment bound.
enum Scope<'s> {
    /// The named scope, by bound name.
    Named(&'s str),
    /// The members of this owner group, one of several owners sharing a
    /// bound name, which the site lies in.
    Anchored(usize),
}

/// What a written C# name with its generic argument count denotes
/// ([`Declarations::written_group`]).
enum WrittenGroup {
    /// No path owner of this file is bound under the name.
    Undeclared,
    /// This owner group alone.
    One(usize),
    /// No group of that arity, or several.
    Unproved,
}

/// Up to two distinct declarations per name group: enough to tell "none",
/// "exactly one" and "several" apart for any set of groups, however many
/// declarations share the name.
#[derive(Debug, Default, Clone)]
struct Few {
    slots: [[Option<usize>; 2]; 8],
}

impl Few {
    fn add(&mut self, idx: usize, groups: u8) {
        for (bit, slot) in self.slots.iter_mut().enumerate() {
            if groups & (1 << bit) == 0 || slot.contains(&Some(idx)) {
                continue;
            }
            if let Some(free) = slot.iter_mut().find(|entry| entry.is_none()) {
                *free = Some(idx);
            }
        }
    }

    /// Whether any declaration is held in `groups`.
    fn holds_any(&self, groups: u8) -> bool {
        self.slots.iter().enumerate().any(|(bit, slot)| groups & (1 << bit) != 0 && slot[0].is_some())
    }

    /// Retains at most two distinct declarations across `groups`: enough
    /// to distinguish a unique binding from an ambiguous one.
    fn collect(&self, groups: u8, found: &mut Vec<usize>) {
        for (bit, slot) in self.slots.iter().enumerate() {
            if groups & (1 << bit) == 0 {
                continue;
            }
            for idx in slot.iter().flatten() {
                if found.len() == 2 {
                    return;
                }
                count_shared_context_steps(1);
                if !found.contains(idx) {
                    found.push(*idx);
                }
            }
        }
    }
}

/// Declarations of one name split by the generic arity they declare: a
/// C# type's parameter count (a namespace's is zero) is part of its
/// identity, so `Base` and `Base<T>` never compete for a written
/// `Base<int>`. Everything else (every declaration in other languages, C#
/// members that are no type or namespace) is `free` and matches any
/// written count. Indexed before [`Few`] truncates, so three or more
/// arities of one name each keep their own candidates.
#[derive(Debug, Default, Clone)]
struct ByArity {
    free: Few,
    declared: HashMap<usize, Few>,
}

impl ByArity {
    fn add(&mut self, idx: usize, groups: u8, arity: Option<usize>) {
        match arity {
            Some(arity) => self.declared.entry(arity).or_default().add(idx, groups),
            None => self.free.add(idx, groups),
        }
    }

    /// [`Few::collect`] over the declarations a name written with `arity`
    /// generic arguments may denote.
    fn collect(&self, groups: u8, arity: usize, found: &mut Vec<usize>) {
        self.free.collect(groups, found);
        if let Some(few) = self.declared.get(&arity) {
            few.collect(groups, found);
        }
    }
}

/// The only declaration `found` holds.
fn only(found: &[usize]) -> Option<usize> {
    match found {
        [only] => Some(*only),
        _ => None,
    }
}

/// What every lookup needs of the declarations bound under one name,
/// summarized once per name.
#[derive(Debug, Default, Clone)]
struct NameSummary {
    /// The named-scope ([`Reach::Qualified`]) declarations, by group.
    qualified: Few,
    /// C#: the same declarations by declared arity ([`ByArity`]); empty
    /// in other languages.
    written: ByArity,
    /// Groups in which some named-scope declaration is no path owner
    /// ([`is_path_owner_kind`]); and groups in which the declarations are
    /// not all one owner group ([`OwnerGroups`]): one merges with nothing (a
    /// Rust `mod`, a Scala `object` or any type that is not a C# `partial`
    /// fragment), or two belong to different groups (a namespace beside a
    /// `partial` type, `partial` types of different category, arity or
    /// enclosing owner). Either makes the name competing owners
    /// ([`Declarations::is_ambiguous_owner`]).
    not_owner: u8,
    not_merging: u8,
    /// The group of the first merging declaration, with its name groups.
    merged: Option<(usize, u8)>,
    /// The distinct canonical owners a C++ spelling reaches through inline
    /// namespaces ([`Declarations::denoted`]), by the groups they are
    /// declared (or, as a qualifier only, the qualifier groups) in: two of them make
    /// the spelling ambiguous however their members are spread. Empty
    /// outside the inline summaries of [`Declarations::each_summary`].
    canonical: Few,
    /// Whether any declaration of the name, wherever placed, is type-like;
    /// a type-like one writes bases (read only where
    /// [`inherits_member_names`]); one is a module.
    type_like: bool,
    type_with_bases: bool,
    module: bool,
}

/// What a C++ spelling reaches through inline namespaces
/// ([`Declarations::denoted`]).
#[derive(Debug, Default)]
struct Denoted {
    /// Every canonical name the spelling reaches (reopened definitions
    /// merged); empty when [`Self::beyond_ambiguous`].
    names: Vec<String>,
    /// The positions in `names` that may own a later segment: a pure
    /// qualifier or a declaration in the qualifier groups. At most two are
    /// kept, enough to tell one owner from several.
    owners: Vec<usize>,
    /// A qualifier of the spelling already reaches several owners: the
    /// spelling is ambiguous and nothing below that qualifier is looked
    /// up, so distinct suffixes never revisit its candidates.
    beyond_ambiguous: bool,
}

impl Denoted {
    /// A spelling that names only itself, its own possible owner (outside
    /// C++ files with inline namespaces).
    fn exact(name: &str) -> Self {
        Self { names: vec![name.to_string()], owners: vec![0], beyond_ambiguous: false }
    }
}

/// Every declaration of the file by the name it is bound under, the
/// declarations that name a scope by their nodes, and the scopes that exist
/// only as a qualifier of a declared name (a Java package, a C++ namespace
/// written `a::b`), plus the lexical binding index. Built once per file
/// from all declarations, selected or not, in record order. Every lookup
/// reads per-name summaries and per-owner member indexes, each built once,
/// so its work is bounded by its path, never by how many declarations share
/// a name.
struct Declarations<'a> {
    entries: Vec<Declared<'a>>,
    /// The name each entry is bound under ([`Self::binding_name`]): its
    /// in-file qualified name, except where the language binds the
    /// declaration in a namespace its displayed prefix does not spell.
    bound: Vec<Cow<'a, str>>,
    /// Entries by bound name.
    by_qualified: HashMap<Cow<'a, str>, Vec<usize>>,
    /// Each C++ namespace's inline namespaces, by bound name: a member of
    /// an inline namespace is also a member of the namespace around it
    /// ([`Self::denoted`]). Empty in other languages and in a file without
    /// one.
    inline_children: HashMap<String, Vec<String>>,
    inline_names: OnceCell<InlineNames>,
    denoted_names: RefCell<HashMap<String, Rc<Denoted>>>,
    effective_summaries: RefCell<HashMap<String, Rc<NameSummary>>>,
    denoted_qualifiers: RefCell<HashMap<String, bool>>,
    /// Built on the first lookup that needs it.
    implied: OnceCell<HashSet<Cow<'a, str>>>,
    /// [`NameSummary`] per bound name, built on the first lookup.
    summaries: OnceCell<HashMap<Cow<'a, str>, NameSummary>>,
    /// Each lexical owner's own declarations ([`Self::members`]), by owner
    /// and the rest of their bound name after the owner's.
    direct_members: OnceCell<HashMap<(usize, String), ByArity>>,
    scopes_by_node: HashMap<usize, usize>,
    language: ProgrammingLanguage,
    root: Node<'a>,
    source: &'a str,
    walk: OnceCell<FileWalk<'a>>,
    /// A walk built before these declarations, used by [`Self::walk`] when
    /// its key matches the entries.
    prebuilt: RefCell<Option<LexicalWalk<'a>>>,
    /// PHP `namespace N;` statements, in source order.
    php_namespaces: Vec<Node<'a>>,
    /// The C# or Scala namespace in force at each child of the root.
    namespaces: HashMap<usize, Option<std::rc::Rc<str>>>,
    /// Each Swift extension's owner (by extension node id), resolved once.
    extension_owners: OnceCell<HashMap<usize, Option<usize>>>,
    /// The members (owner entry, member name) a Python attribute write
    /// rebinds, each write's owner path resolved once
    /// ([`Self::written_members`]).
    written_members: OnceCell<HashSet<(usize, &'a str)>>,
    /// The same members by the named-scope owner's bound name.
    written_named: OnceCell<HashMap<String, HashSet<&'a str>>>,
    /// Each enclosing owner's type parameter names, by its node id
    /// ([`Self::type_parameters_around`]).
    generic_layers: RefCell<HashMap<usize, Rc<HashSet<String>>>>,
    /// Swift owner contexts include the actual declaration ancestors.
    generic_contexts: RefCell<HashMap<usize, TypeParams>>,
    /// Which declarations are one owner, built on the first lookup.
    owner_groups: OnceCell<OwnerGroups<'a>>,
    /// The scope facts of each group of several declarations, merged once.
    group_facts: OnceCell<HashMap<usize, ScopeFacts<'a>>>,
}

impl<'a> Declarations<'a> {
    fn new(entries: Vec<Declared<'a>>, root: Node<'a>, language: ProgrammingLanguage, source: &'a str) -> Self {
        // PHP allows `namespace N;` only as a top-level statement.
        let php_namespaces: Vec<Node<'a>> = if language == ProgrammingLanguage::Php {
            let mut cursor = root.walk();
            root.named_children(&mut cursor)
                .filter(|node| node.kind() == "namespace_definition" && node.child_by_field_name("body").is_none())
                .collect()
        } else {
            Vec::new()
        };
        let namespaces = if matches!(language, ProgrammingLanguage::CSharp | ProgrammingLanguage::Scala) {
            file_namespace_index(root, language, source)
        } else {
            HashMap::new()
        };
        let mut scopes_by_node = HashMap::new();
        for (idx, entry) in entries.iter().enumerate() {
            if is_scope_kind(entry.kind) {
                for node in [entry.declaration, entry.anchor] {
                    scopes_by_node.entry(node.id()).or_insert(idx);
                }
            }
        }
        count_shared_context_steps(entries.len());
        let mut declarations = Self {
            entries,
            bound: Vec::new(),
            by_qualified: HashMap::new(),
            inline_children: HashMap::new(),
            inline_names: OnceCell::new(),
            denoted_names: RefCell::new(HashMap::new()),
            effective_summaries: RefCell::new(HashMap::new()),
            denoted_qualifiers: RefCell::new(HashMap::new()),
            implied: OnceCell::new(),
            summaries: OnceCell::new(),
            direct_members: OnceCell::new(),
            scopes_by_node,
            language,
            root,
            source,
            walk: OnceCell::new(),
            prebuilt: RefCell::new(None),
            php_namespaces,
            namespaces,
            extension_owners: OnceCell::new(),
            written_members: OnceCell::new(),
            written_named: OnceCell::new(),
            generic_layers: RefCell::new(HashMap::new()),
            generic_contexts: RefCell::new(HashMap::new()),
            owner_groups: OnceCell::new(),
            group_facts: OnceCell::new(),
        };
        declarations.bound = (0..declarations.entries.len()).map(|idx| declarations.binding_name(idx)).collect();
        for (idx, name) in declarations.bound.iter().enumerate() {
            declarations.by_qualified.entry(name.clone()).or_default().push(idx);
        }
        declarations.inline_children = declarations.inline_children();
        declarations
    }

    /// Each C++ namespace's inline namespaces, by the enclosing name (`""`
    /// for the root). A namespace is inline when any of its definitions
    /// says so. One step per namespace definition.
    fn inline_children(&self) -> HashMap<String, Vec<String>> {
        let mut children: HashMap<String, Vec<String>> = HashMap::new();
        if self.language != ProgrammingLanguage::Cpp {
            return children;
        }
        let mut listed: HashSet<&str> = HashSet::new();
        for (idx, entry) in self.entries.iter().enumerate() {
            if entry.kind != SymbolKindV2::Namespace {
                continue;
            }
            count_shared_context_steps(1);
            let mut cursor = entry.declaration.walk();
            if !entry.declaration.children(&mut cursor).any(|child| child.kind() == "inline") {
                continue;
            }
            let name = self.bound[idx].as_ref();
            let parent = crate::analysis::owner_path(name).unwrap_or("");
            if listed.insert(name) {
                children.entry(parent.to_string()).or_default().push(name.to_string());
            }
        }
        children
    }

    /// Written prefixes are matched through an indexed inline-subtree
    /// member range, never by traversing all descendants for each segment.
    /// Only spellings actually queried are memoized; omitted paths are not
    /// enumerated. Canonical names merge reopened namespace definitions.
    /// A segment is looked up only below the one owner its qualifier
    /// reaches ([`Denoted::owners`]); below a qualifier that reaches
    /// several, the spelling is [`Denoted::beyond_ambiguous`] without a
    /// lookup, so each distinct suffix costs its own segment, never the
    /// qualifier's candidates again.
    fn denoted(&self, name: &str) -> Rc<Denoted> {
        let cached = self.denoted_names.borrow().get(name).cloned();
        if let Some(denoted) = cached { return denoted; }
        if self.inline_children.is_empty() || name.is_empty() {
            let denoted = Rc::new(Denoted::exact(name));
            self.denoted_names.borrow_mut().insert(name.to_string(), Rc::clone(&denoted));
            return denoted;
        }
        let index = self.inline_names.get_or_init(|| {
            // `implied` also splits at a single `:` (a Lua method
            // separator); no C++ spelling ends in one.
            InlineNames::new(self.by_qualified.keys().map(|name| name.as_ref())
                .chain(self.implied().iter().map(|name| name.as_ref()).filter(|name| !name.ends_with(':'))),
                &self.inline_children)
        });
        let mut pending = Vec::new();
        let mut prefix = name;
        let mut parent = loop {
            let cached = self.denoted_names.borrow().get(prefix).cloned();
            if let Some(denoted) = cached { break denoted; }
            if prefix.is_empty() { break Rc::new(Denoted::exact("")); }
            pending.push(prefix);
            prefix = prefix.rsplit_once("::").map_or("", |(parent, _)| parent);
        };
        let owner_groups = qualifier_categories(self.language, false);
        // An iterative prefix walk also bounds stack use for fully written
        // deep paths whose enclosing spellings have not been queried yet.
        for name in pending.into_iter().rev() {
            let segment = name.rsplit_once("::").map_or(name, |(_, segment)| segment);
            let mut denoted = Denoted::default();
            match parent.owners[..] {
                _ if parent.beyond_ambiguous => denoted.beyond_ambiguous = true,
                [] => {}
                [owner] => {
                    // One owner's inline range holds each canonical name
                    // once: no deduplication is needed.
                    for candidate in index.members(&parent.names[owner], segment) {
                        count_shared_context_steps(1);
                        if denoted.owners.len() < 2 && self.may_own(candidate, owner_groups) {
                            denoted.owners.push(denoted.names.len());
                        }
                        denoted.names.push(candidate.clone());
                    }
                }
                _ => denoted.beyond_ambiguous = true,
            }
            parent = Rc::new(denoted);
            self.denoted_names.borrow_mut().insert(name.to_string(), Rc::clone(&parent));
        }
        parent
    }

    /// Whether the canonical `name` may own a later path segment: it
    /// qualifies a declared name or is declared in a named scope in
    /// `groups`.
    fn may_own(&self, name: &str, groups: u8) -> bool {
        self.implied().contains(name)
            || self.summaries().get(name).is_some_and(|summary| summary.qualified.holds_any(groups))
    }

    /// The name entry `idx` is bound under: its in-file qualified name,
    /// except where its displayed prefix is not the namespace the language
    /// binds it in.
    ///
    /// - A PHP class, interface, trait, enum or function always binds in
    ///   its namespace, also when declared in a function, method, closure or
    ///   conditional whose name its displayed name carries
    ///   (`Example\Symbols\reviewProbe::ReviewBase` binds
    ///   `Example\Symbols\ReviewBase`).
    /// - A member of a Scala `package object q` (no record, so not in the
    ///   displayed name) belongs to package `q` (`example.ReviewBase` binds
    ///   `example.q.ReviewBase`).
    fn binding_name(&self, idx: usize) -> Cow<'a, str> {
        let entry = &self.entries[idx];
        let displayed = if self.language == ProgrammingLanguage::Rust {
            // An ASCII name without a raw identifier is its own key.
            if entry.qualified_name.is_ascii() && !entry.qualified_name.contains("r#") {
                Cow::Borrowed(entry.qualified_name)
            } else {
                Cow::Owned(entry.qualified_name.split("::").map(|segment| crate::analysis::name_identity::key(self.language, segment).into_owned()).collect::<Vec<_>>().join("::"))
            }
        } else { crate::analysis::name_identity::key(self.language, entry.qualified_name) };
        let text = |node: Node<'_>| self.source.get(node.start_byte()..node.end_byte());
        match self.language {
            ProgrammingLanguage::Php
                if matches!(
                    entry.kind,
                    SymbolKindV2::Class
                        | SymbolKindV2::Interface
                        | SymbolKindV2::Trait
                        | SymbolKindV2::Enum
                        | SymbolKindV2::Function
                ) =>
            {
                let Some(name) = text(entry.name) else { return displayed };
                let bound = self.join(self.php_statement_namespace(entry.declaration), name);
                if bound == entry.qualified_name { displayed } else { Cow::Owned(bound) }
            }
            ProgrammingLanguage::Scala => {
                let Some(object) = scala_package_object(entry.declaration) else { return displayed };
                let Some(segment) = object.child_by_field_name("name").and_then(text) else { return displayed };
                let package = self.package_scope(object);
                let rest = if package.is_empty() {
                    Some(entry.qualified_name)
                } else {
                    entry.qualified_name.strip_prefix(package.as_str()).and_then(|rest| rest.strip_prefix('.'))
                };
                rest.map_or(displayed, |rest| Cow::Owned(self.join(&self.join(&package, segment), rest)))
            }
            _ => displayed,
        }
    }

    /// The named scope a statement at `node` outside every record is in:
    /// the C#/Scala namespace in force (with the Scala block packages and
    /// `package object` around it) or the PHP namespace.
    fn package_scope(&self, node: Node<'_>) -> String {
        let mut top = node;
        let mut blocks = Vec::new();
        while let Some(parent) = top.indexed_parent() {
            if parent.id() == self.root.id() {
                break;
            }
            if self.language == ProgrammingLanguage::Scala {
                if parent.kind() == "package_clause"
                    && parent.child_by_field_name("body").is_some()
                    && let Some(name) = namespace_name(parent, self.source)
                {
                    blocks.push(name);
                } else if parent.kind() == "package_object"
                    && let Some(name) = parent.child_by_field_name("name")
                    && let Some(name) = self.source.get(name.start_byte()..name.end_byte())
                {
                    blocks.push(name.to_string());
                }
            }
            top = parent;
        }
        let mut scope = self
            .namespaces
            .get(&top.id())
            .and_then(|name| name.as_deref())
            .map_or_else(|| self.php_statement_namespace(node).to_string(), str::to_string);
        for block in blocks.into_iter().rev() {
            scope = self.join(&scope, &block);
        }
        scope
    }

    fn reach(&self, idx: usize) -> Reach {
        self.walk().lexical.placements[idx]
    }

    /// Every proper qualifying prefix of a named-scope declaration's bound
    /// name.
    fn implied(&self) -> &HashSet<Cow<'a, str>> {
        self.implied.get_or_init(|| {
            // One pass over each name, like hashing it: no step beyond the
            // record's own.
            let mut implied = HashSet::new();
            for (idx, bound) in self.bound.iter().enumerate() {
                if self.reach(idx) != Reach::Qualified {
                    continue;
                }
                match bound {
                    Cow::Borrowed(name) => implied.extend(qualifying_prefixes(name).map(Cow::Borrowed)),
                    Cow::Owned(name) => {
                        implied.extend(qualifying_prefixes(name).map(|prefix| Cow::Owned(prefix.to_string())));
                    }
                }
            }
            implied
        })
    }

    fn qualified_name(&self, idx: usize) -> &'a str {
        self.entries[idx].qualified_name
    }

    /// The record whose declaration is the nearest scope around `node`.
    fn enclosing_scope(&self, node: Node<'_>) -> Option<usize> {
        let mut ancestor = node.indexed_parent();
        while let Some(current) = ancestor {
            if let Some(&idx) = self.scopes_by_node.get(&current.id()) {
                return Some(idx);
            }
            ancestor = current.indexed_parent();
        }
        None
    }

    /// The declarations of the Rust modules around `node`, innermost first.
    fn enclosing_modules(&self, node: Node<'_>) -> Vec<usize> {
        let mut modules = Vec::new();
        let mut ancestor = node.indexed_parent();
        while let Some(current) = ancestor {
            if current.kind() == "mod_item"
                && let Some(&idx) = self.scopes_by_node.get(&current.id())
            {
                modules.push(idx);
            }
            ancestor = current.indexed_parent();
        }
        modules
    }

    /// The file's lexical index, import bindings and Rust impls, walked
    /// once. An import of a named scope binds in its nearest enclosing
    /// declaration's scope; a PHP `namespace N;` statement puts the
    /// statements after it in `N`. Imports in lexical scopes are bindings
    /// of those scopes ([`Lexical`]).
    fn walk(&self) -> &FileWalk<'a> {
        self.walk.get_or_init(|| {
            let language = self.language;
            let prebuilt = self.prebuilt.take().filter(|prebuilt| prebuilt.key == walk_key(&self.entries));
            let (lexical, collected) = match prebuilt {
                Some(prebuilt) => (prebuilt.lexical, prebuilt.collected),
                None => {
                    let (lexical, collected) = Lexical::build(&self.entries, self.root, language, self.source, |node| {
                        language == ProgrammingLanguage::Swift
                            && declaration_keyword(node, language).as_deref() == Some("extension")
                    });
                    (Rc::new(lexical), collected)
                }
            };
            let mut walk = FileWalk {
                named: HashMap::new(),
                by_owner: HashMap::new(),
                owned: HashMap::new(),
                owned_names: HashSet::new(),
                qualified_writes: collected.qualified_writes,
                impls: collected.impls,
                extensions: collected.extensions,
                lexical,
            };
            // Each fact goes to its scope's bound name and, inside a record,
            // to that record too.
            let record = |walk: &mut FileWalk<'a>, node: Node<'_>, add: &dyn Fn(&mut ScopeFacts<'a>)| {
                match self.enclosing_scope(node) {
                    Some(idx) => {
                        add(walk.named.entry(self.bound[idx].to_string()).or_default());
                        add(walk.by_owner.entry(idx).or_default());
                    }
                    None => add(walk.named.entry(self.package_scope(node)).or_default()),
                }
            };
            for node in collected.imports {
                match import_binding(node, language, self.source) {
                    Some(ImportBinding::Names(names, categories)) => record(&mut walk, node, &|facts| {
                        for name in &names {
                            *facts.imports.entry(crate::analysis::name_identity::key(language, name)).or_default() |= categories;
                        }
                    }),
                    Some(ImportBinding::Opaque) => record(&mut walk, node, &|facts| {
                        facts.unreadable = true;
                        facts.opaque = true;
                    }),
                    Some(ImportBinding::Module) => record(&mut walk, node, &|facts| facts.opaque = true),
                    None => {}
                }
            }
            for binding in collected.unrecorded {
                let Some(text) = self.source.get(binding.name.start_byte()..binding.name.end_byte()) else { continue };
                if self.names_scope(binding.scope) {
                    let categories = binding.categories;
                    record(&mut walk, binding.name, &|facts| *facts.unrecorded.entry(crate::analysis::name_identity::key(language, text)).or_default() |= categories);
                } else {
                    *walk.owned.entry(binding.scope_index).or_default().entry(crate::analysis::name_identity::key(language, text)).or_default() |= binding.categories;
                    walk.owned_names.insert(crate::analysis::name_identity::key(language, text));
                }
            }
            // A Python module is the root scope, `""`.
            for name in collected.global_writes {
                *walk.named.entry(String::new()).or_default().unrecorded.entry(crate::analysis::name_identity::key(language, name)).or_default() |= ALL;
            }
            walk
        })
    }

    /// Whether the syntax scope `node` is a named scope a qualified name
    /// spells: the file root, a declaration with a record, or the body of
    /// one. Anything else (a Rust impl) is an owner without a record.
    fn names_scope(&self, node: Node<'_>) -> bool {
        node.id() == self.root.id()
            || self.scopes_by_node.contains_key(&node.id())
            || node.indexed_parent().is_some_and(|parent| self.scopes_by_node.contains_key(&parent.id()))
    }

    /// Whether an owner without a record around `at` binds `first` in the
    /// given groups (a Rust impl's generic parameter).
    fn owner_binds(&self, first: &str, at: Node<'_>, categories: u8) -> bool {
        let walk = self.walk();
        let key = crate::analysis::name_identity::key(self.language, first);
        let first = key.as_ref();
        walk.owned_names.contains(first)
            && walk.lexical.scopes_from(at).any(|scope| {
                count_shared_context_steps(1);
                walk.owned.get(&scope).and_then(|names| names.get(first)).is_some_and(|&groups| groups & categories != 0)
            })
    }

    /// The PHP namespace a statement at `node` belongs to
    /// ([`php_namespace_at`]); `""` in other languages.
    fn php_statement_namespace(&self, node: Node<'_>) -> &'a str {
        if self.language != ProgrammingLanguage::Php {
            return "";
        }
        php_namespace_at(node, &self.php_namespaces, self.source)
    }

    /// `prefix` joined with `segment` by the language's path separator.
    fn join(&self, prefix: &str, segment: &str) -> String {
        if prefix.is_empty() {
            segment.to_string()
        } else {
            format!("{prefix}{}{segment}", path_separator(self.language))
        }
    }

    /// The summary of every name, built once (after the lexical
    /// placements): one step per declaration.
    fn summaries(&self) -> &HashMap<Cow<'a, str>, NameSummary> {
        self.summaries.get_or_init(|| {
            let inherits = inherits_member_names(self.language);
            let mut summaries: HashMap<Cow<'a, str>, NameSummary> = HashMap::new();
            for (idx, entry) in self.entries.iter().enumerate() {
                count_shared_context_steps(1);
                let summary = summaries.entry(self.bound[idx].clone()).or_default();
                if is_type_like_kind(entry.kind) {
                    summary.type_like = true;
                    if inherits && !summary.type_with_bases {
                        let bases = declared_bases(entry.name, self.language, self.source);
                        summary.type_with_bases =
                            !(bases.extends.is_empty() && bases.implements.is_empty() && bases.unclassified.is_empty());
                    }
                }
                summary.module |= entry.kind == SymbolKindV2::Module;
                if self.reach(idx) != Reach::Qualified {
                    continue;
                }
                let groups = kind_categories(self.language, entry.kind);
                summary.qualified.add(idx, groups);
                if self.language == ProgrammingLanguage::CSharp {
                    summary.written.add(idx, groups, self.declared_arity(idx));
                }
                if !is_path_owner_kind(entry.kind) {
                    summary.not_owner |= groups;
                }
                let owners = self.owner_groups();
                if !owners.merges[idx] {
                    summary.not_merging |= groups;
                } else {
                    match summary.merged {
                        None => summary.merged = Some((owners.of[idx], groups)),
                        Some((first, first_groups)) if first != owners.of[idx] => {
                            summary.not_merging |= groups | first_groups;
                        }
                        Some(_) => {}
                    }
                }
            }
            summaries
        })
    }

    /// Which declarations are one owner ([`OwnerGroups`]), built once: one
    /// pass over the declarations, like hashing their names (no step beyond
    /// the record's own), plus one counted ancestor walk per C# `partial`
    /// fragment for its enclosing owner, whose group must be the same.
    fn owner_groups(&self) -> &OwnerGroups<'a> {
        self.owner_groups.get_or_init(|| {
            let count = self.entries.len();
            let csharp = self.language == ProgrammingLanguage::CSharp;
            // What a declaration must share to merge, and (a C# `partial`
            // fragment) the owner it is declared in.
            let shapes: Vec<Option<(MergeKey, Option<usize>)>> = (0..count)
                .map(|idx| {
                    let entry = &self.entries[idx];
                    if self.reach(idx) != Reach::Qualified {
                        return None;
                    }
                    if entry.kind == SymbolKindV2::Namespace {
                        return Some((MergeKey::Namespace, None));
                    }
                    if !csharp || !is_type_like_kind(entry.kind) {
                        return None;
                    }
                    let (category, arity) = csharp_partial_shape(entry.declaration, self.source)?;
                    let enclosing = self.enclosing_scope(entry.declaration);
                    count_shared_context_steps(1);
                    Some((MergeKey::Partial { category, arity, enclosing: None }, enclosing))
                })
                .collect();
            let mut of = vec![usize::MAX; count];
            let mut keys: HashMap<(&str, MergeKey), usize> = HashMap::new();
            for start in 0..count {
                if of[start] != usize::MAX {
                    continue;
                }
                // An enclosing owner is a proper ancestor, so the chain of
                // undecided owners ends; the outermost is decided first.
                let mut chain = vec![start];
                while let Some((_, Some(outer))) = shapes[chain[chain.len() - 1]]
                    && of[outer] == usize::MAX
                {
                    chain.push(outer);
                }
                for idx in chain.into_iter().rev() {
                    of[idx] = match shapes[idx] {
                        None => idx,
                        Some((key, outer)) => {
                            let key = match key {
                                MergeKey::Partial { category, arity, .. } => {
                                    MergeKey::Partial { category, arity, enclosing: outer.map(|outer| of[outer]) }
                                }
                                key => key,
                            };
                            *keys.entry((self.bound[idx].as_ref(), key)).or_insert(idx)
                        }
                    };
                }
            }
            let mut size: HashMap<usize, u32> = HashMap::new();
            for &group in &of {
                *size.entry(group).or_default() += 1;
            }
            let mut with_bases = HashSet::new();
            let mut by_arity: HashMap<Cow<'a, str>, HashMap<usize, Option<usize>>> = HashMap::new();
            for (idx, entry) in self.entries.iter().enumerate() {
                let group = of[idx];
                if size[&group] > 1
                    && is_type_like_kind(entry.kind)
                    && inherits_member_names(self.language)
                    && !with_bases.contains(&group)
                {
                    let bases = declared_bases(entry.name, self.language, self.source);
                    if !(bases.extends.is_empty() && bases.implements.is_empty() && bases.unclassified.is_empty()) {
                        with_bases.insert(group);
                    }
                }
                if csharp && self.reach(idx) == Reach::Qualified && is_path_owner_kind(entry.kind) {
                    let arity = if entry.kind == SymbolKindV2::Namespace { 0 } else { csharp_arity(entry.declaration) };
                    by_arity
                        .entry(self.bound[idx].clone())
                        .or_default()
                        .entry(arity)
                        .and_modify(|found| {
                            if *found != Some(group) {
                                *found = None;
                            }
                        })
                        .or_insert(Some(group));
                }
            }
            OwnerGroups { of, merges: shapes.iter().map(Option::is_some).collect(), size, with_bases, by_arity }
        })
    }

    /// The owner group of entry `idx` ([`OwnerGroups`]).
    fn group_of(&self, idx: usize) -> usize {
        self.owner_groups().of[idx]
    }

    /// The declaration of a group of one: the only target a path through it
    /// may name. A group of several fragments has no one ID.
    fn sole(&self, group: usize) -> Option<usize> {
        (self.owner_groups().size.get(&group) == Some(&1)).then_some(group)
    }

    /// Which owner group a C# path segment spelling the bound name `name`
    /// with `arity` written generic arguments denotes: none of this file
    /// when `name` declares no path owner (a qualifier alone, another
    /// language), else the one group of that arity, else unproved.
    fn written_group(&self, name: &str, arity: usize) -> WrittenGroup {
        match self.owner_groups().by_arity.get(name) {
            None => WrittenGroup::Undeclared,
            Some(groups) => match groups.get(&arity) {
                Some(&Some(group)) => WrittenGroup::One(group),
                _ => WrittenGroup::Unproved,
            },
        }
    }

    /// The generic arity that is part of declaration `idx`'s identity: a
    /// C# type's parameter count, zero for a C# namespace; `None` for any
    /// other kind or language, which a name written with any count may
    /// denote.
    fn declared_arity(&self, idx: usize) -> Option<usize> {
        let entry = &self.entries[idx];
        match entry.kind {
            _ if self.language != ProgrammingLanguage::CSharp => None,
            SymbolKindV2::Namespace => Some(0),
            kind if is_type_like_kind(kind) => Some(csharp_arity(entry.declaration)),
            _ => None,
        }
    }

    /// Whether a path segment with `arity` written generic arguments may
    /// name declaration `idx` ([`Self::declared_arity`]).
    fn arity_fits(&self, idx: usize, arity: usize) -> bool {
        self.declared_arity(idx).is_none_or(|declared| declared == arity)
    }

    /// The scope facts of one owner group: its declaration's, or every
    /// fragment's merged ([`Self::group_facts`]).
    fn owner_facts(&self, group: usize) -> Option<&ScopeFacts<'a>> {
        if self.sole(group).is_some() {
            self.walk().by_owner.get(&group)
        } else {
            self.group_facts().get(&group)
        }
    }

    /// The facts of every group of several declarations, merged once from
    /// its fragments' own.
    fn group_facts(&self) -> &HashMap<usize, ScopeFacts<'a>> {
        self.group_facts.get_or_init(|| {
            let mut merged: HashMap<usize, ScopeFacts<'a>> = HashMap::new();
            for (&owner, facts) in &self.walk().by_owner {
                count_shared_context_steps(1);
                let group = self.group_of(owner);
                if self.sole(group).is_none() {
                    merged.entry(group).or_default().absorb(facts);
                }
            }
            merged
        })
    }

    /// Each queried spelling has one bounded candidate/flag summary,
    /// including injected inline members and the distinct canonical owners
    /// they belong to ([`NameSummary::canonical`]); repeated consumers share
    /// it.
    fn each_summary(&self, name: &str, mut read: impl FnMut(&NameSummary)) {
        if self.inline_children.is_empty() {
            if let Some(summary) = self.summaries().get(name) { read(summary); }
            return;
        }
        let cached = self.effective_summaries.borrow().get(name).cloned();
        let summary = cached.unwrap_or_else(|| {
            let mut combined = NameSummary::default();
            let denoted = self.denoted(name);
            let owner_groups = qualifier_categories(self.language, false);
            if denoted.beyond_ambiguous {
                // Two distinct owners: ambiguous wherever it is read, and
                // nothing else is claimed of it.
                combined.canonical.add(0, owner_groups);
                combined.canonical.add(1, owner_groups);
            }
            for (position, bound) in denoted.names.iter().enumerate() {
                count_shared_context_steps(1);
                if self.implied().contains(bound.as_str()) {
                    combined.canonical.add(position, owner_groups);
                }
                if let Some(summary) = self.summaries().get(bound.as_str()) {
                    for (bit, slots) in summary.qualified.slots.iter().enumerate() {
                        for &idx in slots.iter().flatten() { combined.qualified.add(idx, 1 << bit); }
                        if slots.iter().any(Option::is_some) { combined.canonical.add(position, 1 << bit); }
                    }
                    combined.not_owner |= summary.not_owner;
                    combined.not_merging |= summary.not_merging;
                    combined.type_like |= summary.type_like;
                    combined.type_with_bases |= summary.type_with_bases;
                    combined.module |= summary.module;
                }
            }
            let combined = Rc::new(combined);
            self.effective_summaries.borrow_mut().insert(name.to_string(), Rc::clone(&combined));
            combined
        });
        read(&summary);
    }

    /// The named-scope declarations bound as `name` in the given groups:
    /// enough of them (at most two) to tell none, one and several
    /// apart. A lexical declaration is never found by its displayed name:
    /// two same-named owners in sibling blocks display the same member
    /// names, so only [`Self::members`] of the owner a lookup selected reach
    /// one.
    fn named(&self, name: &str, categories: u8) -> Vec<usize> {
        let mut found = Vec::new();
        self.each_summary(name, |summary| summary.qualified.collect(categories, &mut found));
        found
    }

    /// [`Self::named`] for a name written with `arity` generic arguments:
    /// in C# only the declarations of that arity ([`ByArity`]), so a
    /// same-named declaration of another arity neither competes with nor
    /// stands in for the one the count denotes.
    fn named_written(&self, name: &str, categories: u8, arity: usize) -> Vec<usize> {
        if self.language != ProgrammingLanguage::CSharp {
            return self.named(name, categories);
        }
        let mut found = Vec::new();
        if let Some(summary) = self.summaries().get(name) {
            summary.written.collect(categories, arity, &mut found);
        }
        found
    }

    /// Whether any declaration bound as the named scope `prefix` is
    /// type-like, a type-like one writing bases, or a module.
    fn scope_flags(&self, prefix: &str) -> (bool, bool, bool) {
        let mut flags = (false, false, false);
        self.each_summary(prefix, |summary| {
            flags.0 |= summary.type_like;
            flags.1 |= summary.type_with_bases;
            flags.2 |= summary.module;
        });
        flags
    }

    /// The declarations called `segment` in the given groups that `owner`
    /// declares directly in its own body: nearer to it than any other scope
    /// declaration and outside every block, callable and anonymous owner in
    /// between. Identity is the owner group ([`OwnerGroups`]), so a
    /// same-named owner elsewhere contributes nothing, and both owners and
    /// members are groups: a member written in several fragments of one
    /// group is one member, with no one ID ([`Self::sole`]). Only members a
    /// name written with `arity` generic arguments may denote
    /// ([`ByArity`]); at most two per group, as [`Few`].
    fn members(&self, owner: usize, segment: &str, categories: u8, arity: usize) -> Vec<usize> {
        let mut found = Vec::new();
        if let Some(members) = self.direct_members().get(&(owner, segment.to_string())) {
            members.collect(categories, arity, &mut found);
        }
        found
    }

    /// Every declaration under the owner it is declared directly in
    /// ([`Self::direct_owner`]), keyed by the rest of its bound name; one
    /// ancestor walk per declaration, built on the first syntax-owner
    /// lookup.
    fn direct_members(&self) -> &HashMap<(usize, String), ByArity> {
        self.direct_members.get_or_init(|| {
            let separator = path_separator(self.language);
            let mut index: HashMap<(usize, String), ByArity> = HashMap::new();
            for (idx, entry) in self.entries.iter().enumerate() {
                if matches!(self.reach(idx), Reach::GenericParam | Reach::Redirected) {
                    continue;
                }
                let Some(owner) = self.direct_owner(idx) else { continue };
                let owner = self.group_of(owner);
                let (owner_name, name) = (self.bound[owner].as_ref(), self.bound[idx].as_ref());
                let rest = if owner_name.is_empty() {
                    Some(name)
                } else {
                    name.strip_prefix(owner_name).and_then(|rest| rest.strip_prefix(separator))
                };
                if let Some(rest) = rest {
                    index
                        .entry((owner, rest.to_string()))
                        .or_default()
                        .add(self.group_of(idx), kind_categories(self.language, entry.kind), self.declared_arity(idx));
                }
            }
            index
        })
    }

    /// The nearest scope declaration around `member`, when no lexical
    /// region (a block, a callable, an anonymous owner) lies between them.
    fn direct_owner(&self, member: usize) -> Option<usize> {
        let mut ancestor = self.entries[member].declaration.indexed_parent();
        while let Some(current) = ancestor {
            count_shared_context_steps(1);
            match self.scopes_by_node.get(&current.id()) {
                Some(&found) if found != member => return Some(found),
                _ if opens_lexical_region(self.language, current) => return None,
                _ => {}
            }
            ancestor = current.indexed_parent();
        }
        None
    }

    /// Whether the named scope `owner` (a bound name) binds `segment` in
    /// the given groups by anything but its declarations: a binding no
    /// record reports (a pattern target, an augmented assignment, a
    /// deletion), an import where imports bind members
    /// ([`imports_bind_members`]), a wildcard import where it may replace
    /// any member ([`globs_bind_members`]), or a qualified write to the
    /// member through the owner's path ([`Self::written_members`]). Any of
    /// them leaves the member unproved.
    /// `writes` is off only while the writes themselves are resolved.
    /// Only the owners the spelling reaches are read ([`Denoted::owners`]),
    /// never every candidate of the spelling.
    fn named_member_rebound(&self, owner: &str, segment: &str, categories: u8, writes: bool) -> bool {
        let denoted = self.denoted(owner);
        denoted.beyond_ambiguous
            || denoted.owners.iter().any(|&position| {
                count_shared_context_steps(1);
                self.owner_rebinds(FactsOf::Named(&denoted.names[position]), segment, categories, writes)
            })
    }

    /// [`Self::named_member_rebound`] for one canonical owner name, or
    /// (an [`Owner::Anchored`] lookup) for one owner group alone.
    fn owner_rebinds(&self, owner: FactsOf<'_>, segment: &str, categories: u8, writes: bool) -> bool {
        self.scope_facts(owner).is_some_and(|facts| {
            facts.binds_unrecorded(segment, categories, self.language)
                || (imports_bind_members(self.language) && facts.imports_name(segment, categories, self.language))
                || (globs_bind_members(self.language) && facts.opaque)
        }) || (writes
            && match owner {
                FactsOf::Named(owner) => self.written_named().get(owner).is_some_and(|members| members.contains(segment)),
                FactsOf::Owner(owner) => self.is_written(owner, segment),
            })
    }

    /// The imports and unrecorded bindings of a named scope: by bound
    /// name, or of one owner declaration.
    fn scope_facts(&self, of: FactsOf<'_>) -> Option<&ScopeFacts<'a>> {
        let walk = self.walk();
        match of {
            FactsOf::Named(name) => walk.named.get(name),
            FactsOf::Owner(group) => self.owner_facts(group),
        }
    }

    /// [`Self::scope_flags`] of one owner group: whether it is type-like, a
    /// type-like one some fragment of which writes bases (read only where
    /// [`inherits_member_names`]), or a module.
    fn owner_flags(&self, owner: usize) -> (bool, bool, bool) {
        let entry = &self.entries[owner];
        let type_like = is_type_like_kind(entry.kind);
        let with_bases = type_like && inherits_member_names(self.language) && match self.sole(owner) {
            Some(_) => {
                let bases = declared_bases(entry.name, self.language, self.source);
                !(bases.extends.is_empty() && bases.implements.is_empty() && bases.unclassified.is_empty())
            }
            None => self.owner_groups().with_bases.contains(&owner),
        };
        (type_like, with_bases, entry.kind == SymbolKindV2::Module)
    }

    /// The flags and facts a lookup at the named scope `prefix` reads from
    /// `at`: those of the one owner group `at` lies in where same-named
    /// owners compete ([`Self::anchor_around`]), else those of every
    /// declaration bound as `prefix`.
    fn scope_at(&self, prefix: &'_ str, at: Node<'_>) -> (Option<usize>, (bool, bool, bool)) {
        match self.anchor_around(prefix, at) {
            Some(owner) => (Some(owner), self.owner_flags(owner)),
            None => (None, self.scope_flags(prefix)),
        }
    }

    /// Whether the lexical owner's member `member` (named `segment`) shares
    /// its binding in the owner's body with anything else: the syntax-scope
    /// counterpart of [`Self::named_member_rebound`].
    fn syntax_member_rebound(&self, owner: usize, member: usize, segment: &str, categories: u8, writes: bool) -> bool {
        !self.walk().lexical.binds_only(
            self.entries[member].declaration,
            member,
            segment,
            categories,
            imports_bind_members(self.language),
            globs_bind_members(self.language),
        ) || (writes && self.is_written(owner, segment))
    }

    /// Whether a qualified write rebinds `owner`'s member `segment`.
    fn is_written(&self, owner: usize, segment: &str) -> bool {
        count_shared_context_steps(1);
        let written = self.written_members();
        !written.is_empty() && written.contains(&(owner, segment))
    }

    /// [`Self::written_members`] by the bound name of each named-scope
    /// owner.
    fn written_named(&self) -> &HashMap<String, HashSet<&'a str>> {
        self.written_named.get_or_init(|| {
            let mut named: HashMap<String, HashSet<&'a str>> = HashMap::new();
            for &(owner, member) in self.written_members() {
                count_shared_context_steps(1);
                if self.reach(owner) == Reach::Qualified {
                    named.entry(self.bound[owner].to_string()).or_default().insert(member);
                }
            }
            named
        })
    }

    /// The members Python attribute writes rebind: each write's object
    /// path is looked up from the write like a base's qualifier, with no
    /// write consulted, and must name one owner declaration of this file.
    /// A write elsewhere in time (before the owner, in a function) counts
    /// the same: no write order is evaluated.
    fn written_members(&self) -> &HashSet<(usize, &'a str)> {
        self.written_members.get_or_init(|| {
            let mut written = HashSet::new();
            for &write in &self.walk().qualified_writes {
                let (Some(object), Some(attribute)) =
                    (write.child_by_field_name("object"), write.child_by_field_name("attribute"))
                else {
                    continue;
                };
                let Some(member) = self.source.get(attribute.start_byte()..attribute.end_byte()) else { continue };
                let Some(path) = base_path(object, self.language, self.source) else { continue };
                let scope = self.enclosing_scope(write).map_or("", |idx| self.bound[idx].as_ref());
                let type_params = self.type_parameters_around(write, None, Vec::new());
                let site = self.site(write, write.start_byte(), scope, type_params, false);
                if let Some(owner) = self.resolve_path(&path, &site, is_path_owner_kind, false) {
                    written.insert((self.group_of(owner), member));
                }
            }
            written
        })
    }

    fn qualifies(&self, name: &str) -> bool {
        if self.inline_children.is_empty() { return self.implied().contains(name); }
        let cached = self.denoted_qualifiers.borrow().get(name).copied();
        if let Some(found) = cached { return found; }
        let denoted = self.denoted(name);
        let found = denoted.beyond_ambiguous || denoted.names.iter().any(|bound| {
            count_shared_context_steps(1);
            self.implied().contains(bound.as_str())
        });
        self.denoted_qualifiers.borrow_mut().insert(name.to_string(), found);
        found
    }

    /// Whether `name`, written with `arity` generic arguments, is declared
    /// in a named scope in the given groups, or qualifies a declared name.
    /// In C# a declaration of another arity binds nothing, and a name that
    /// only qualifies others binds only unwritten arguments when no type or
    /// namespace declares it: `H` qualifying `H<T>`'s members is no `H`.
    fn is_bound(&self, name: &str, categories: u8, arity: usize) -> bool {
        !self.named_written(name, categories, arity).is_empty()
            || (self.qualifies(name)
                && (self.language != ProgrammingLanguage::CSharp
                    || (arity == 0 && matches!(self.written_group(name, 0), WrittenGroup::Undeclared))))
    }

    /// Whether a path may continue through the scope called `name`: it
    /// names exactly one owner in the qualifier groups, a namespace-like
    /// declaration or one owner group ([`OwnerGroups`]: the definitions of
    /// one namespace the language reopens, or the C# `partial` fragments of
    /// one type identity; repeated Rust modules, Scala objects, other types
    /// and `partial` types of different category, arity or enclosing owner
    /// compete), or it exists only as a qualifier. A C++ spelling that
    /// reaches two distinct namespaces through inline namespaces names no
    /// one owner, whatever their members are. Read from the name's summary,
    /// so a namespace reopened any number of times costs one lookup.
    fn is_traversable(&self, name: &str, categories: u8) -> bool {
        let mut found = Vec::new();
        let mut canonical = Vec::new();
        let (mut not_owner, mut not_merging) = (0, 0);
        self.each_summary(name, |summary| {
            summary.qualified.collect(categories, &mut found);
            summary.canonical.collect(categories, &mut canonical);
            not_owner |= summary.not_owner;
            not_merging |= summary.not_merging;
        });
        if canonical.len() > 1 {
            return false;
        }
        if found.is_empty() {
            return self.qualifies(name);
        }
        not_owner & categories == 0 && (found.len() == 1 || not_merging & categories == 0)
    }

    /// Whether the declarations bound as `name` are several competing
    /// owners rather than one: two declarations in any group that are not
    /// all one owner group ([`OwnerGroups`]), or a C++ spelling that reaches
    /// two distinct inline-namespace members. Such a name never tells its
    /// owners' members, imports, bindings or bases apart by bound name; a
    /// lookup made inside one of them reads that owner group's own
    /// ([`Self::anchors_members`], [`Self::anchor_around`]).
    fn is_ambiguous_owner(&self, name: &str) -> bool {
        let mut found = Vec::new();
        let mut canonical = Vec::new();
        let mut not_merging = 0;
        self.each_summary(name, |summary| {
            summary.qualified.collect(ALL, &mut found);
            summary.canonical.collect(ALL, &mut canonical);
            not_merging |= summary.not_merging;
        });
        canonical.len() > 1 || (found.len() > 1 && not_merging != 0)
    }

    /// Whether lookup through the named-scope declaration `owner` must read
    /// its owner group's members by syntax ([`Owner::Anchored`]): its bound
    /// name is shared by competing owners ([`Self::is_ambiguous_owner`]).
    fn anchors_members(&self, owner: usize) -> bool {
        self.is_ambiguous_owner(&self.bound[owner])
    }

    /// The owner group of the named-scope declaration around `at` bound as
    /// `prefix`, when lookup inside it must read its own members, imports,
    /// bindings and bases ([`Self::anchors_members`]): the source says which
    /// of the competing owners the site is in, and the group adds the other
    /// fragments of that owner alone.
    fn anchor_around(&self, prefix: &str, at: Node<'_>) -> Option<usize> {
        if prefix.is_empty() || !self.is_ambiguous_owner(prefix) {
            return None;
        }
        let mut ancestor = at.indexed_parent();
        while let Some(current) = ancestor {
            count_shared_context_steps(1);
            if let Some(&idx) = self.scopes_by_node.get(&current.id())
                && self.bound[idx] == prefix
            {
                return Some(self.group_of(idx));
            }
            ancestor = current.indexed_parent();
        }
        None
    }

    /// The named-scope prefix at which a relative path's first segment is
    /// bound in the given groups, walking `scopes` innermost first, or
    /// `None` when no binding can be proved to be a declaration of this
    /// file.
    ///
    /// The first scope that binds the segment decides; outer scopes are
    /// never tried after it. A binding is a declaration in the groups looked
    /// up (so a parameter or variable never hides a type a base clause
    /// names, while an alias or a same-group declaration does), a qualifier
    /// of one, or an import or a binding no record reports in those groups
    /// (a destructured value, a pattern `val`, a generic parameter), which
    /// never resolves. A wildcard import or
    /// using-directive without a stronger local binding, a type with
    /// inherited member names (whose member types are found first), and in
    /// Rust a module (an item does not see its parent module's names) end
    /// the search unresolved. Class scopes are skipped where they are not
    /// lexical: always in JS/TS, and in Python every class scope but the
    /// innermost (`scopes[0]`, the body the header is written in).
    /// Declarations of lexical scopes are never seen here; [`Lexical`]
    /// decides them first.
    ///
    /// A prefix several competing owners share (two Rust modules, Scala
    /// objects, C# types that are not fragments of one type) is read as the
    /// owner group `at` lies in: its own members ([`Scope::Anchored`]), imports,
    /// wildcard and unrecorded bindings, kind and bases
    /// ([`Self::scope_at`]), never those of the bound name both share.
    ///
    /// A C# declaration whose generic arity differs from the `arity` the
    /// segment is written with is no binding: `Base` inside `H` beside `H`'s
    /// own `Base<T>` keeps looking outward.
    fn binding_scope<'s>(
        &self,
        first: &str,
        scopes: &[&'s str],
        at: Node<'_>,
        categories: u8,
        arity: usize,
    ) -> Option<Scope<'s>> {
        for (position, &prefix) in scopes.iter().enumerate() {
            count_shared_context_steps(1);
            let (anchor, (is_class, with_bases, is_module)) = self.scope_at(prefix, at);
            if is_class && !class_scope_is_lexical(self.language, position == 0) {
                continue;
            }
            let facts = self.scope_facts(anchor.map_or(FactsOf::Named(prefix), FactsOf::Owner));
            let opaque = facts.is_some_and(|facts| facts.opaque);
            if facts.is_some_and(|facts| facts.imports_name(first, categories, self.language) || facts.binds_unrecorded(first, categories, self.language))
                || (!local_before_wildcard(self.language) && opaque) {
                return None;
            }
            match anchor {
                Some(owner) if !self.members(owner, first, categories, arity).is_empty() => {
                    return Some(Scope::Anchored(owner));
                }
                Some(_) => {}
                None if self.is_bound(&self.join(prefix, first), categories, arity) => {
                    return Some(Scope::Named(prefix));
                }
                None => {}
            }
            if opaque {
                return None;
            }
            if with_bases || (self.language == ProgrammingLanguage::Rust && is_module) {
                return None;
            }
        }
        None
    }

    /// Whether a binding visible from `site` may give `first` a meaning of
    /// this file in the given groups: a lexical binding in view, a
    /// declaration or qualifier of that name in any named scope out to the
    /// root, an import that names it, or an import whose names cannot be
    /// read. Unlike [`Self::binding_scope`] nothing ends the walk early, so
    /// `false` proves that no same-file binding exists; wildcard and
    /// whole-module imports, inherited member types and declarations in
    /// other files are not evidence. A scope competing owners share is read
    /// as the one the site lies in, as in [`Self::binding_scope`], so a
    /// same-named owner elsewhere is no evidence either, nor is a C#
    /// declaration of another generic `arity`.
    fn may_bind(&self, first: &str, site: &Site<'_, 'a>, categories: u8, arity: usize) -> bool {
        let walk = self.walk();
        if walk.lexical.may_bind(first, site.at, site.position, categories)
            || self.owner_binds(first, site.at, categories)
        {
            return true;
        }
        site.scopes.iter().enumerate().any(|(position, &prefix)| {
            count_shared_context_steps(1);
            let (anchor, (is_class, _, _)) = self.scope_at(prefix, site.at);
            if is_class && !class_scope_is_lexical(self.language, position == 0) {
                return false;
            }
            self.scope_facts(anchor.map_or(FactsOf::Named(prefix), FactsOf::Owner)).is_some_and(|facts| {
                facts.imports_name(first, categories, self.language) || facts.binds_unrecorded(first, categories, self.language) || facts.unreadable
            }) || match anchor {
                Some(owner) => !self.members(owner, first, categories, arity).is_empty(),
                None => self.is_bound(&self.join(prefix, first), categories, arity),
            }
        })
    }

    /// The record a written path names, if exactly one eligible declaration
    /// of this file is that name.
    ///
    /// The first segment is decided first: a relative path's by the
    /// lexical scopes around the site ([`Lexical::lookup`]), then (when
    /// none binds it, and it is no visible type parameter of a named owner
    /// nor, in the looked-up groups, a parameter of an owner without a
    /// record around the site, a Rust impl)
    /// by [`Self::binding_scope`] over the site's named scopes; a rooted one
    /// at the file root (`crate::`, PHP `\Name`, C++ `::Name`), Rust
    /// `self::`/`super::` at the module the site's modules give; a Rust
    /// `::name` path is external and never resolves. A Python `global`
    /// directive sends the lookup to the module, where any write the
    /// directive's scope makes of the name is a binding no record reports.
    ///
    /// Each following segment is looked up only inside the owner the
    /// previous one selected. Below a lexical owner (a type or Rust module
    /// declared in a callable or block, found by the lexical lookup or by
    /// `self::`/`super::` from inside it) that is that owner's own
    /// declarations by syntax ([`Self::members`]), so a same-named owner in
    /// a sibling block never lends or hides a member. Below a named scope it
    /// is the bound name, whose qualifier must name one namespace-like owner
    /// ([`Self::is_traversable`]), so a function's local class is not
    /// reachable through the function's name and an ambiguous intermediate
    /// owner stops every later segment. Below a named scope that competing
    /// owners share (two Rust modules or Scala objects of one name, C#
    /// types that are not fragments of one type), entered from a site
    /// inside one of them (`self::`, `super::`, a first segment bound there)
    /// or, in C#, through a name whose written generic argument count
    /// denotes one owner group (`H.Base` the non-generic `H`, `H<int>.Base`
    /// the one-parameter `H`), it is that owner group's own members
    /// ([`Owner::Anchored`]); a C# name no owner group of that arity
    /// declares stops the path. In C# the written generic argument count is
    /// part of every segment's name at every step: the first segment's
    /// binding scope, each member and the final candidates are only the
    /// declarations of that arity ([`Self::declared_arity`]), so `Base<int>`
    /// beside `Base` and `Base<T, U>` names `Base<T>`, and a nearer `Base<T>`
    /// never hides an outer `Base`. Under any owner a segment is unproved when the owner binds its
    /// name by anything but that one declaration (an import, a deletion, a
    /// pattern target, a Python attribute write through the owner's path:
    /// [`Self::named_member_rebound`], [`Self::syntax_member_rebound`]), and
    /// a declaration a Python directive places elsewhere is no member.
    /// The complete name resolves only if it is the only declaration of
    /// that name in the base's name groups and `eligible` accepts its kind.
    fn resolve(&self, path: &BasePath, site: &Site<'_, 'a>, eligible: fn(SymbolKindV2) -> bool) -> Option<usize> {
        // The named and member lookups select by arity already; a lexical
        // binding (never a C# type) is checked here.
        self.resolve_path(path, site, eligible, true)
            .filter(|&found| self.arity_fits(found, path.arities.last().copied().unwrap_or(0)))
    }

    /// [`Self::resolve`]; `writes` consults the qualified writes (off only
    /// while they are themselves being resolved).
    fn resolve_path(
        &self,
        path: &BasePath,
        site: &Site<'_, 'a>,
        eligible: fn(SymbolKindV2) -> bool,
        writes: bool,
    ) -> Option<usize> {
        let canonical;
        let path = if matches!(self.language, ProgrammingLanguage::Rust | ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript) {
            canonical = BasePath { segments: path.segments.iter().map(|name| crate::analysis::name_identity::key(self.language, name).into_owned()).collect(), ..path.clone() };
            &canonical
        } else { path };
        let (first, rest) = path.segments.split_first()?;
        let final_groups = final_categories(self.language, site.value);
        let qualifier_groups = qualifier_categories(self.language, site.value);
        let first_groups = if rest.is_empty() { final_groups } else { qualifier_groups };
        let first_arity = path.arities.first().copied().unwrap_or(0);
        // The module `self::`/`super::` names (`None`: the file root), and
        // where its path continues from.
        let module_at = |out: usize| match out.cmp(&site.modules.len()) {
            std::cmp::Ordering::Less => Some(Some(site.modules[out])),
            std::cmp::Ordering::Equal => Some(None),
            std::cmp::Ordering::Greater => None,
        };
        let in_module = |module: Option<usize>| match module {
            Some(module) if self.reach(module) != Reach::Qualified => (Owner::Syntax(module), &path.segments[..]),
            Some(module) if self.anchors_members(module) => (Owner::Anchored(self.group_of(module)), &path.segments[..]),
            Some(module) => (Owner::Named(self.join(self.qualified_name(module), first)), rest),
            None => (Owner::Named(first.clone()), rest),
        };
        let enter = |scope: Scope<'_>| match scope {
            Scope::Named(prefix) => (Owner::Named(self.join(prefix, first)), rest),
            Scope::Anchored(owner) => (Owner::Anchored(owner), &path.segments[..]),
        };
        let (mut current, rest) = match path.anchor {
            PathAnchor::Relative => match self.walk().lexical.lookup(first, site.at, site.position, first_groups) {
                Local::Found(idx) => {
                    let kind = self.entries[idx].kind;
                    if rest.is_empty() {
                        return eligible(kind).then_some(idx);
                    }
                    if !is_path_owner_kind(kind) {
                        return None;
                    }
                    (Owner::Syntax(idx), rest)
                }
                Local::Unresolved => return None,
                Local::Global => enter(self.binding_scope(first, &[""], site.at, first_groups, first_arity)?),
                Local::Continue => {
                    if site.type_params.contains(first)
                        || self.owner_binds(first, site.at, first_groups)
                    {
                        return None;
                    }
                    enter(self.binding_scope(first, &site.scopes, site.at, first_groups, first_arity)?)
                }
            },
            PathAnchor::Root => (Owner::Named(first.clone()), rest),
            PathAnchor::Extern => return None,
            PathAnchor::SelfModule => in_module(module_at(0)?),
            PathAnchor::Super(out) => in_module(module_at(out)?),
        };
        let offset = path.segments.len() - rest.len();
        for (position, segment) in rest.iter().enumerate() {
            count_shared_context_steps(1);
            let last = position + 1 == rest.len();
            let groups = if last { final_groups } else { qualifier_groups };
            let arity = path.arities.get(offset + position).copied().unwrap_or(0);
            // A C# name that declares owners is entered as the one owner
            // group its written argument count denotes: `H.Base` is read in
            // the non-generic `H` alone, never in `H<T>`.
            if let Owner::Named(name) = &current {
                let written = path.arities.get((offset + position).wrapping_sub(1)).copied().unwrap_or(0);
                let traversable = self.is_traversable(name, qualifier_groups);
                match self.written_group(name, written) {
                    WrittenGroup::One(group) if !traversable => current = Owner::Anchored(group),
                    WrittenGroup::One(_) | WrittenGroup::Undeclared if traversable => {}
                    _ => return None,
                }
            }
            current = match current {
                Owner::Syntax(owner) => {
                    let [only] = self.members(owner, segment, groups, arity)[..] else { return None };
                    if self.syntax_member_rebound(owner, only, segment, groups, writes) {
                        return None;
                    }
                    let kind = self.entries[only].kind;
                    if last {
                        return self.sole(only).filter(|_| eligible(kind));
                    }
                    if !is_path_owner_kind(kind) {
                        return None;
                    }
                    Owner::Syntax(only)
                }
                Owner::Anchored(owner) => {
                    // The members of every fragment of the owner group, by
                    // group: a nested `partial` type is one member.
                    let [only] = self.members(owner, segment, groups, arity)[..] else { return None };
                    // The anchored group's own bindings, not those of
                    // every same-named owner or namespace its spelling
                    // reaches.
                    if self.owner_rebinds(FactsOf::Owner(owner), segment, groups, writes) {
                        return None;
                    }
                    let kind = self.entries[only].kind;
                    if last {
                        return self.sole(only).filter(|_| eligible(kind));
                    }
                    if !is_path_owner_kind(kind) {
                        return None;
                    }
                    if self.anchors_members(only) {
                        Owner::Anchored(only)
                    } else {
                        Owner::Named(self.bound[only].to_string())
                    }
                }
                Owner::Named(name) => {
                    if self.named_member_rebound(&name, segment, groups, writes) {
                        return None;
                    }
                    Owner::Named(self.join(&name, segment))
                }
            };
        }
        let Owner::Named(current) = current else { return None };
        let last_arity = path.arities.last().copied().unwrap_or(0);
        only(&self.named_written(&current, final_groups, last_arity)).filter(|&found| eligible(self.entries[found].kind))
    }

    /// The relation target for `base`; `records` are the declarations'
    /// records.
    fn target(
        &self,
        records: &[SymbolRecord],
        base: &WrittenBase,
        site: &Site<'_, 'a>,
        eligible: fn(SymbolKindV2) -> bool,
    ) -> SymbolRef {
        let resolved = base.path.as_ref().and_then(|path| self.resolve(path, site, eligible)).map(|idx| &records[idx]);
        SymbolRef {
            id: resolved.map(|record| record.id.clone()),
            name: base.text.clone(),
            qualified_name: resolved.map(|record| record.identity.qualified_name.clone()),
            source_span: base.source_span.clone(),
        }
    }

    /// One declaration's unnumbered generic names, shared by owner identity.
    fn generic_layer(&self, owner: usize) -> Rc<HashSet<String>> {
        let entry = &self.entries[owner];
        let cached = self.generic_layers.borrow().get(&entry.declaration.id()).cloned();
        if let Some(layer) = cached { return layer; }
        let names = declared_generic_names(entry.declaration, entry.name, self.language, self.source);
        count_shared_context_steps(names.len());
        let layer = Rc::new(names.into_iter().map(|name| crate::analysis::name_identity::key(self.language, &name).into_owned()).collect());
        self.generic_layers.borrow_mut().insert(entry.declaration.id(), Rc::clone(&layer));
        layer
    }

    /// An extension restores generic bindings from its owner's declaration
    /// ancestors, without restoring ordinary members as lexical header names.
    fn owner_generic_context(&self, owner: usize) -> TypeParams {
        let cached = self.generic_contexts.borrow().get(&owner).cloned();
        if let Some(context) = cached { return context; }
        let mut context = self.type_parameters_around(self.entries[owner].declaration, Some(owner), Vec::new());
        let layer = self.generic_layer(owner);
        if !layer.is_empty() { context.layers.insert(0, layer); }
        self.generic_contexts.borrow_mut().insert(owner, context.clone());
        context
    }

    /// The type parameter names visible at `node` from its named owners:
    /// `own`, then those of every declaration around it (a C++ member class
    /// of a template, a Java inner class of a generic class), in Rust of
    /// every enclosing `impl`, and in Swift of the owner an enclosing
    /// extension extends and that owner's enclosing declarations; an
    /// extension whose owner is not proved leaves the context unknown
    /// ([`TypeParams::unknown`]), never empty. `skip`
    /// is the record `node` declares, whose own
    /// list `own` already holds. Owners that are lexical scopes (callables,
    /// local types, anonymous owners) bind their parameters in [`Lexical`]
    /// instead, where a nearer declaration may come first.
    ///
    /// Names are read without numbering them: an enclosing owner whose list
    /// is too long for `GenericParam::position` still binds every name it
    /// declares. Whether that owner's record can be built is decided only
    /// where the owner is selected, never here.
    ///
    /// Each owner's lookup layer is built once per file and shared by
    /// every declaration inside it ([`TypeParams`]).
    fn type_parameters_around(&self, node: Node<'_>, skip: Option<usize>, own: Vec<String>) -> TypeParams {
        let lexical = &self.walk().lexical;
        count_shared_context_steps(own.len());
        let mut params = TypeParams { layers: vec![Rc::new(own.into_iter().collect::<HashSet<String>>())], unknown: false };
        let mut previous = skip;
        let mut ancestor = node.indexed_parent();
        while let Some(current) = ancestor {
            if lexical.is_lexical_node(current) {
                ancestor = current.indexed_parent();
                continue;
            }
            let read = if let Some(&owner) = self.scopes_by_node.get(&current.id())
                && Some(owner) != previous
            {
                previous = Some(owner);
                Some(owner)
            } else if self.language == ProgrammingLanguage::Rust && current.kind() == "impl_item" {
                None
            } else if self.language == ProgrammingLanguage::Swift
                && let Some(&owner) = self.extension_owners().get(&current.id())
            {
                match owner {
                    Some(owner) => params.extend(self.owner_generic_context(owner)),
                    None => params.unknown = true,
                }
                ancestor = current.indexed_parent();
                continue;
            } else {
                ancestor = current.indexed_parent();
                continue;
            };
            count_shared_context_steps(1);
            let layer = match read {
                Some(owner) => self.generic_layer(owner),
                None => {
                    let cached = self.generic_layers.borrow().get(&current.id()).cloned();
                    cached.unwrap_or_else(|| {
                        let names = impl_type_parameters(current, self.source);
                        count_shared_context_steps(names.len());
                        let layer = Rc::new(names.into_iter().map(|name| crate::analysis::name_identity::key(self.language, &name).into_owned()).collect());
                        self.generic_layers.borrow_mut().insert(current.id(), Rc::clone(&layer));
                        layer
                    })
                }
            };
            if !layer.is_empty() {
                params.layers.push(layer);
            }
            ancestor = current.indexed_parent();
        }
        params
    }

    /// The lookup site of a declaration, impl or extension `at` in the
    /// named scope `scope` (an in-file qualified name), its path starting at
    /// byte `position`.
    fn site<'s>(&'s self, at: Node<'a>, position: usize, scope: &'s str, type_params: TypeParams, value: bool) -> Site<'s, 'a> {
        let scopes = match self.language {
            // A PHP class name is relative to its namespace alone, which a
            // function's or method's display prefix never is.
            ProgrammingLanguage::Php => vec![self.php_statement_namespace(at)],
            // A declaration in a Scala `package object` looks names up from
            // the package the object makes.
            ProgrammingLanguage::Scala => match self.scopes_by_node.get(&at.id()).map(|&idx| &self.bound[idx]) {
                Some(Cow::Owned(bound)) => scope_chain(crate::analysis::owner_path(bound).unwrap_or("")),
                _ => scope_chain(scope),
            },
            _ => scope_chain(scope),
        };
        Site { at, position, scopes, modules: self.enclosing_modules(at), type_params, value }
    }

    /// Each Swift extension's owner: the one same-file type its extended
    /// path names, looked up from the extension like a base.
    fn extension_owners(&self) -> &HashMap<usize, Option<usize>> {
        self.extension_owners.get_or_init(|| {
            self.walk()
                .extensions
                .iter()
                .map(|&extension| {
                    let owner = extension.child_by_field_name("name").and_then(|name| {
                        let path = base_path(name, ProgrammingLanguage::Swift, self.source)?;
                        let scope = self.enclosing_scope(extension).map_or("", |idx| self.qualified_name(idx));
                        let site = self.site(extension, extension.start_byte(), scope, TypeParams::default(), false);
                        self.resolve(&path, &site, is_self_type_kind)
                    });
                    (extension.id(), owner)
                })
                .collect()
        })
    }
}

/// The PHP namespace a statement at `node` belongs to: the nearest
/// enclosing braced `namespace`, else the last of the `namespace N;`
/// `statements` before it, else `""`.
fn php_namespace_at<'a>(node: Node<'_>, statements: &[Node<'_>], source: &'a str) -> &'a str {
    let text = |namespace: Node<'_>| {
        namespace
            .child_by_field_name("name")
            .and_then(|name| source.get(name.start_byte()..name.end_byte()))
            .map_or("", str::trim)
    };
    let mut ancestor = node.indexed_parent();
    while let Some(current) = ancestor {
        if current.kind() == "namespace_definition" && current.child_by_field_name("body").is_some() {
            return text(current);
        }
        ancestor = current.indexed_parent();
    }
    // In source order: the last one before `node` by binary search.
    let before = statements.partition_point(|statement| statement.start_byte() < node.start_byte());
    before.checked_sub(1).map_or("", |last| text(statements[last]))
}

/// The Scala `package object` around `node`, if any: one is only ever a
/// top-level or block-package statement.
fn scala_package_object(node: Node<'_>) -> Option<Node<'_>> {
    let mut ancestor = node.indexed_parent();
    while let Some(current) = ancestor {
        if current.kind() == "package_object" {
            return Some(current);
        }
        ancestor = current.indexed_parent();
    }
    None
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

#[derive(Clone, Copy)]
enum BaseRelation { Extends, Implements, Unclassified }

/// One change to apply to a record.
enum Change {
    Extends(usize, SymbolRef),
    Implements(usize, SymbolRef),
    Unclassified(usize, SymbolRef),
    Container(usize, SymbolRef, bool),
}

/// Fills classified and unclassified bases on the selected records and, when
/// `containment` is asked, links each Rust impl member to its self type and
/// each Swift extension member to its extension's owner
/// (`relations.container`, and `parent` when the self type or owner
/// resolves).
///
/// `prebuilt` is the definitions pass's walk of a Rust file
/// ([`impl_self_types`]), reused when the records are its declarations.
pub(super) fn link_relations<'tree>(
    records: &mut [SymbolRecord],
    inputs: &[RelationInput<'tree>],
    root: Node<'tree>,
    language: ProgrammingLanguage,
    source: &'tree str,
    containment: bool,
    prebuilt: Option<LexicalWalk<'tree>>,
) {
    count_relation_link();
    let bases = written_base_lists(records, inputs, language, source);
    // Only Rust impls and Swift extensions relate anything beyond the bases
    // a record writes.
    if bases.is_empty() && !matches!(language, ProgrammingLanguage::Rust | ProgrammingLanguage::Swift) {
        return;
    }
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
        *declarations.prebuilt.borrow_mut() = prebuilt;
        let mut changes = Vec::new();
        written_bases(&declarations, records, inputs, bases, &mut changes);
        if language == ProgrammingLanguage::Rust {
            impl_relations(&declarations, records, inputs, containment, &mut changes);
        }
        if language == ProgrammingLanguage::Swift {
            swift_extensions(&declarations, records, inputs, containment, &mut changes);
        }
        changes
    };
    let mut written = WrittenTargets::default();
    for change in changes {
        match change {
            Change::Extends(idx, target) => written.push(records, idx, BaseRelation::Extends, target),
            Change::Implements(idx, target) => written.push(records, idx, BaseRelation::Implements, target),
            Change::Unclassified(idx, target) => written.push(records, idx, BaseRelation::Unclassified, target),
            Change::Container(idx, container, resolved) => {
                let relations = &mut records[idx].relations;
                relations.parent = resolved.then(|| container.clone());
                relations.container = Some(container);
            }
        }
    }
}

/// The written expressions already in each record's base lists: equal
/// written expressions appear once per list, in the order first written;
/// distinct instantiations of one target stay distinct. One membership
/// check per target, however long the list grows.
#[derive(Default)]
struct WrittenTargets {
    /// Lists seen so far, each seeded once from what the record already
    /// holds.
    seen: HashMap<(usize, u8), HashSet<String>>,
}

impl WrittenTargets {
    fn push(&mut self, records: &mut [SymbolRecord], idx: usize, relation: BaseRelation, target: SymbolRef) {
        count_shared_context_steps(1);
        let relations = &mut records[idx].relations;
        let list = match relation {
            BaseRelation::Extends => &mut relations.extends,
            BaseRelation::Implements => &mut relations.implements,
            BaseRelation::Unclassified => &mut relations.unclassified_bases,
        };
        let seen = self
            .seen
            .entry((idx, relation as u8))
            .or_insert_with(|| list.iter().map(|existing| existing.name.clone()).collect());
        if seen.insert(target.name.clone()) {
            list.push(target);
        }
    }
}

/// The bases each selected type-like record writes in its own header, by
/// record index; a record that writes none is left out.
fn written_base_lists(
    records: &[SymbolRecord],
    inputs: &[RelationInput<'_>],
    language: ProgrammingLanguage,
    source: &str,
) -> Vec<(usize, DeclaredBases)> {
    records
        .iter()
        .zip(inputs)
        .enumerate()
        .filter(|(_, (record, input))| {
            input.wanted
                && (declares_bases(record.kind)
                    || (language == ProgrammingLanguage::Scala && record.kind == SymbolKindV2::Module))
        })
        .map(|(idx, (_, input))| (idx, declared_bases(input.name, language, source)))
        .filter(|(_, bases)| !(bases.extends.is_empty() && bases.implements.is_empty() && bases.unclassified.is_empty()))
        .collect()
}

/// Resolves the bases [`written_base_lists`] found.
fn written_bases<'a>(
    declarations: &Declarations<'a>,
    records: &[SymbolRecord],
    inputs: &[RelationInput<'a>],
    bases: Vec<(usize, DeclaredBases)>,
    changes: &mut Vec<Change>,
) {
    let language = declarations.language;
    for (idx, bases) in bases {
        let (record, input) = (&records[idx], &inputs[idx]);
        // A lexical declaration's own parameters are bindings of its own
        // scope; a named one's go with its owners'. Only parameters that can
        // name a type count (a C++ `template <int N>` binds a value).
        let own = if declarations.walk().lexical.is_lexical_node(input.declaration) {
            Vec::new()
        } else {
            declared_generic_names(input.declaration, input.name, language, declarations.source)
        };
        let type_params = declarations.type_parameters_around(input.declaration, Some(idx), own);
        let scope = crate::analysis::owner_path(&record.identity.qualified_name).unwrap_or("");
        let mut ordered: Vec<_> = bases.extends.iter().map(|base| (base, BaseRelation::Extends))
            .chain(bases.implements.iter().map(|base| (base, BaseRelation::Implements)))
            .chain(bases.unclassified.iter().map(|base| (base, BaseRelation::Unclassified))).collect();
        ordered.sort_by_key(|(base, _)| base.start_byte);
        for (base, mut category) in ordered {
            // A JS class, and a TS class's `extends`, names a value.
            let value = language == ProgrammingLanguage::JavaScript
                || (language == ProgrammingLanguage::TypeScript
                    && record.kind == SymbolKindV2::Class
                    && matches!(category, BaseRelation::Extends));
            let site = declarations.site(input.declaration, base.start_byte, scope, type_params.clone(), value);
            // A built-in spelling (Go `int`, Swift `Int`, `Swift.Int`,
            // `Unicode.Scalar`) is no relation only when nothing in view
            // rebinds its first segment in the groups it is looked up in; a
            // shadowed one is kept, resolved when the lookup proves a
            // declaration and as written otherwise.
            if base.builtin_spelling
                && let Some(path) = base.path.as_ref()
                && let Some(first) = path.segments.first()
                && !type_params.contains(first)
                && !declarations.may_bind(
                    first,
                    &site,
                    if path.segments.len() == 1 {
                        final_categories(language, value)
                    } else {
                        qualifier_categories(language, value)
                    },
                    path.arities.first().copied().unwrap_or(0),
                )
            {
                continue;
            }
            let resolved = base.path.as_ref().and_then(|path| declarations.resolve(path, &site, is_base_kind));
            let target = SymbolRef {
                id: resolved.map(|owner| records[owner].id.clone()),
                name: base.text.clone(),
                qualified_name: resolved.map(|owner| records[owner].identity.qualified_name.clone()),
                source_span: base.source_span.clone(),
            };
            if matches!(category, BaseRelation::Unclassified) {
                category = match resolved.map(|owner| &declarations.entries[owner]) {
                    Some(owner) if matches!(owner.kind, SymbolKindV2::Interface | SymbolKindV2::Trait) => BaseRelation::Implements,
                    Some(owner) if declares_class(owner, language) => BaseRelation::Extends,
                    _ => BaseRelation::Unclassified,
                };
            }
            changes.push(match category {
                BaseRelation::Extends => Change::Extends(idx, target),
                BaseRelation::Implements => Change::Implements(idx, target),
                BaseRelation::Unclassified => Change::Unclassified(idx, target),
            });
        }
    }
}

/// Whether a resolved target declares a class, the only category a mixed
/// list's first entry inherits from. Languages whose kind is broader than
/// that category decide from the target's own declaration: a Swift `class`
/// is a `Type` beside structs, actors and enums, and a C# `record` or
/// `record class` is a `Type` beside structs and `record struct`s.
fn declares_class(target: &Declared<'_>, language: ProgrammingLanguage) -> bool {
    match (target.kind, language) {
        (SymbolKindV2::Class, _) => true,
        (SymbolKindV2::Type, ProgrammingLanguage::Swift) => {
            declaration_keyword(target.declaration, language).as_deref() == Some("class")
        }
        (SymbolKindV2::Type, ProgrammingLanguage::CSharp) => {
            target.declaration.kind() == "record_declaration" && !is_csharp_struct(target.declaration)
        }
        _ => false,
    }
}

/// Extension owners use the same conservative lookup as base targets
/// ([`Declarations::extension_owners`]). An external or ambiguous owner has
/// no record on which to preserve conformance. The conformances are looked
/// up from the extension with the owner's and its actual enclosing
/// declarations' type parameters in view.
///
/// Under Containment the same decision links every declaration of an
/// extension's body: to the owner's record, or, when the owner is not
/// proved (external, aliased, imported, ambiguous), to a container named by
/// the extended type as written, with no ID and no parent. No later lookup
/// reselects a same-named record by proximity.
fn swift_extensions(
    declarations: &Declarations<'_>,
    records: &[SymbolRecord],
    inputs: &[RelationInput<'_>],
    containment: bool,
    changes: &mut Vec<Change>,
) {
    if containment {
        for (idx, input) in inputs.iter().enumerate() {
            let Some(extension) = input.container_extension else { continue };
            let owner = declarations.extension_owners().get(&extension.id()).copied().flatten();
            let container = match owner {
                Some(owner) => crate::analysis::symbol_ref(&records[owner]),
                None => SymbolRef {
                    id: None,
                    name: extension
                        .child_by_field_name("name")
                        .and_then(|name| declarations.source.get(name.start_byte()..name.end_byte()))
                        .unwrap_or_default()
                        .to_string(),
                    qualified_name: None,
                    source_span: None,
                },
            };
            changes.push(Change::Container(idx, container, owner.is_some()));
        }
    }
    for &extension in &declarations.walk().extensions {
        let Some(name) = extension.child_by_field_name("name") else { continue };
        let Some(&Some(owner)) = declarations.extension_owners().get(&extension.id()) else { continue };
        if !inputs[owner].wanted { continue; }
        let scope = declarations.enclosing_scope(extension).map_or("", |idx| declarations.qualified_name(idx));
        let mut type_params = declarations.type_parameters_around(extension, None, Vec::new());
        type_params.extend(declarations.owner_generic_context(owner));
        for base in declared_bases(name, ProgrammingLanguage::Swift, declarations.source).implements {
            let site = declarations.site(extension, base.start_byte, scope, type_params.clone(), false);
            let target = declarations.target(records, &base, &site, is_base_kind);
            changes.push(Change::Implements(owner, target));
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
    declarations: &Declarations<'_>,
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
                        source_span: None,
                    }
                }
                None => SymbolRef { id: None, name: resolved.header.self_text.clone(), qualified_name: None, source_span: None },
            };
            changes.push(Change::Container(idx, container, resolved.self_type.is_some()));
        }
    }
}

/// The impl's self type and trait, resolved by [`Declarations::resolve`]
/// from the impl's position with every generic parameter visible there (the
/// impl's own and those of the functions, impls and traits around it), so a
/// self type naming an enclosing generic attaches to no declaration.
fn resolve_impl<'a>(
    declarations: &Declarations<'a>,
    records: &[SymbolRecord],
    impl_item: Node<'a>,
) -> Option<ResolvedImpl> {
    let lookup = resolve_self_type(declarations, impl_item)?;
    let implements = lookup.header.trait_base.as_ref().filter(|_| !lookup.header.is_negative).map(|base| {
        declarations.target(records, base, &lookup.site, is_trait_kind)
    });
    Some(ResolvedImpl { header: lookup.header, self_type: lookup.self_type, implements })
}

/// One impl's header, the site its paths resolve from, and the
/// declaration its self type names.
struct ImplLookup<'s, 'a> {
    header: ImplHeader,
    self_type: Option<usize>,
    site: Site<'s, 'a>,
}

/// The impl's header and the declaration its self type names, if one.
fn resolve_self_type<'s, 'a>(declarations: &'s Declarations<'a>, impl_item: Node<'a>) -> Option<ImplLookup<'s, 'a>> {
    let header = impl_header(impl_item, declarations.source)?;
    let scope = declarations.enclosing_scope(impl_item).map_or("", |idx| declarations.qualified_name(idx));
    // A lexical impl's own parameters are bindings of its scope.
    let own = if declarations.walk().lexical.is_lexical_node(impl_item) { Vec::new() } else { header.type_params.clone() };
    let type_params = declarations.type_parameters_around(impl_item, None, own);
    let site = declarations.site(impl_item, impl_item.start_byte(), scope, type_params, false);
    let self_type = header.self_path.as_ref().and_then(|path| declarations.resolve(path, &site, is_self_type_kind));
    Some(ImplLookup { header, self_type, site })
}

/// For each Rust `impl` of the file (by node id), the index in `entries` of
/// the declaration its self type resolves to, by the same decision
/// [`link_relations`] makes; `None` when it does not resolve. The
/// definitions pass reads it for an impl member's container kind, before
/// records exist.
///
/// Also returns the walk it built, for [`link_relations`] to reuse.
pub(super) fn impl_self_types<'tree>(
    entries: Vec<Declared<'_>>,
    root: Node<'tree>,
    source: &'tree str,
) -> (HashMap<usize, Option<usize>>, LexicalWalk<'tree>) {
    let walk = LexicalWalk::build(&entries, root, source);
    let declarations = Declarations::new(entries, root, ProgrammingLanguage::Rust, source);
    *declarations.prebuilt.borrow_mut() = Some(walk.share());
    let owners = declarations
        .walk()
        .impls
        .iter()
        .filter_map(|&node| Some((node.id(), resolve_self_type(&declarations, node)?.self_type)))
        .collect();
    (owners, walk)
}
