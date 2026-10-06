//! Lexical scopes of one file and name lookup through them, for the
//! languages whose binding rules are supported (Rust, TypeScript and
//! JavaScript). The declared-relation resolver builds the same tree over
//! its own scope set for every language with declared bases
//! (`file/tree_file/relations/lexical.rs`) and reuses the Rust and ECMA
//! binding sites.
//!
//! A scope is a syntax node: every `@local.scope` capture of the effective
//! locals query, the file root, and the language's extra scope kinds
//! ([`LanguageRules::extra_scope`]). Each binding is placed in the scope where
//! its name is visible, and a name is looked up innermost scope first, then
//! outward; the first scope that binds it decides. Lookup never falls back
//! to "any declaration with this name": that is the caller's `NameOnly`.
//!
//! User contract: `tree-hugger/docs/references.md`.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::shared::ProgrammingLanguage;

mod ecma;
mod rust;

/// A type name: Rust `struct`/`enum`/`trait`/`mod`, TypeScript
/// `interface`/`type`, generic type parameters.
pub(crate) const TYPE: u8 = 1;
/// A value name: functions, variables, constants, parameters.
pub(crate) const VALUE: u8 = 2;
/// A Rust macro name.
pub(crate) const MACRO: u8 = 4;
/// An import, whose category the file alone cannot tell.
pub(crate) const ANY_CATEGORY: u8 = TYPE | VALUE | MACRO;

/// Where a binding is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// The innermost scope around the name, except that a scope node naming
    /// itself (`fn f`, `class C`) puts its name in the enclosing scope.
    Innermost,
    /// The nearest enclosing function scope, else the file (JavaScript
    /// `var`).
    FunctionScope,
}

/// How one binding behaves in lookup.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BindingRule {
    /// The name categories ([`TYPE`], [`VALUE`], [`MACRO`]) it binds.
    pub(crate) categories: u8,
    pub(crate) placement: Placement,
    /// The first byte at which the binding is visible; `0` for the whole
    /// scope (items, hoisted declarations, the TDZ of `let`).
    pub(crate) visible_from: usize,
    /// A Rust `let`: among the visible ones of a scope the latest wins.
    pub(crate) ordered: bool,
    /// Calling the name invokes the declaration itself (a function, a
    /// constructor, an import), not whatever a variable holds.
    pub(crate) callable: bool,
}

/// A binding site found in the syntax tree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BindingSite<'tree> {
    pub(crate) name: Node<'tree>,
    pub(crate) rule: BindingRule,
    pub(crate) is_import: bool,
}

/// What a binding names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindingTarget {
    /// The declaration record at this index.
    Record(usize),
    /// An import: established locally, never an in-file declaration.
    Import,
    /// A local binding no record reports (a destructured or `catch`
    /// binding the locals query does not capture).
    Unreported,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Binding {
    pub(crate) name_start: usize,
    pub(crate) rule: BindingRule,
    pub(crate) target: BindingTarget,
}

/// The syntax-level rules of one supported language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageRules {
    Rust,
    /// TypeScript (`typed`) and JavaScript, which share the ECMA query.
    Ecma { typed: bool },
}

impl LanguageRules {
    /// The rules for `language`, `None` when its references are not
    /// resolved.
    pub(crate) fn for_language(language: ProgrammingLanguage) -> Option<Self> {
        match language {
            ProgrammingLanguage::Rust => Some(Self::Rust),
            ProgrammingLanguage::TypeScript => Some(Self::Ecma { typed: true }),
            ProgrammingLanguage::JavaScript => Some(Self::Ecma { typed: false }),
            _ => None,
        }
    }

    /// A scope the locals query does not capture.
    pub(crate) fn extra_scope(self, node: Node<'_>) -> bool {
        match self {
            Self::Rust => rust::extra_scope(node),
            Self::Ecma { .. } => ecma::extra_scope(node),
        }
    }

    /// The binding sites `node` declares (visited once per node).
    pub(crate) fn binding_sites<'tree>(
        self,
        node: Node<'tree>,
        source: &str,
        sites: &mut Vec<BindingSite<'tree>>,
    ) {
        match self {
            Self::Rust => rust::binding_sites(node, source, sites),
            Self::Ecma { typed } => ecma::binding_sites(node, typed, sites),
        }
    }

    /// Whether `node` (`use a::*`) makes every name of its scope possibly
    /// imported.
    pub(crate) fn is_glob_import(self, node: Node<'_>) -> bool {
        self == Self::Rust && node.kind() == "use_wildcard"
    }

    /// The categories a reference written as `node` looks up.
    pub(crate) fn reference_category(self, node: Node<'_>) -> u8 {
        match self {
            Self::Rust => rust::reference_category(node),
            Self::Ecma { typed } => ecma::reference_category(node, typed),
        }
    }

    /// Whether `kind` is a function scope (the target of a [`Placement::FunctionScope`]).
    fn is_function_scope(self, kind: &str) -> bool {
        match self {
            Self::Rust => matches!(kind, "function_item" | "closure_expression"),
            Self::Ecma { .. } => ecma::is_function_scope(kind),
        }
    }
}

/// A file's scopes, bindings and their per-scope name index.
pub(crate) struct ScopeTree<'tree> {
    /// The language whose function scopes a [`Placement::FunctionScope`]
    /// binding climbs to; without rules every binding is innermost.
    rules: Option<LanguageRules>,
    nodes: Vec<Node<'tree>>,
    parents: Vec<Option<usize>>,
    by_node: HashMap<usize, usize>,
    has_glob: Vec<bool>,
    bindings: Vec<Binding>,
    /// `(scope, name)` → binding indexes, in name position order.
    table: HashMap<(usize, &'tree str), Vec<usize>>,
}

/// The outcome of a lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Lookup {
    /// The bindings of the first scope that binds the name, visible at the
    /// position: one is an established binding, several are ambiguous.
    Found(Vec<usize>),
    /// A glob import in a scope reached before any binding may supply it.
    Glob,
    NotFound,
}

impl<'tree> ScopeTree<'tree> {
    /// Builds the tree from `scope_nodes` (any order, repeats allowed) plus
    /// `root`, which is always a scope.
    pub(crate) fn new(
        rules: LanguageRules,
        root: Node<'tree>,
        scope_nodes: impl IntoIterator<Item = Node<'tree>>,
    ) -> Self {
        Self::with_rules(Some(rules), root, scope_nodes)
    }

    /// [`Self::new`] for a scope set chosen by the caller: the relation
    /// resolver builds one for every language with declared bases, most of
    /// which have no [`LanguageRules`].
    pub(crate) fn with_rules(
        rules: Option<LanguageRules>,
        root: Node<'tree>,
        scope_nodes: impl IntoIterator<Item = Node<'tree>>,
    ) -> Self {
        let mut nodes: Vec<Node<'tree>> = std::iter::once(root).chain(scope_nodes).collect();
        // Outer scopes first, so parents precede children and the order
        // never depends on capture order. Depth breaks a tie between nested
        // nodes of one range (a Python class body and its only statement);
        // a node id is an address, never an order.
        nodes.sort_by_key(|node| (node.start_byte(), std::cmp::Reverse(node.end_byte()), node.id()));
        nodes.dedup_by_key(|node| node.id());
        for run in nodes.chunk_by_mut(|a, b| a.byte_range() == b.byte_range()) {
            if run.len() > 1 {
                run.sort_by_cached_key(|node| ancestors(*node).count());
            }
        }
        let by_node: HashMap<usize, usize> =
            nodes.iter().enumerate().map(|(idx, node)| (node.id(), idx)).collect();
        let parents = nodes
            .iter()
            .map(|node| ancestors(*node).find_map(|ancestor| by_node.get(&ancestor.id()).copied()))
            .collect();
        Self {
            rules,
            has_glob: vec![false; nodes.len()],
            nodes,
            parents,
            by_node,
            bindings: Vec::new(),
            table: HashMap::new(),
        }
    }

    /// The innermost scope strictly around `node`.
    pub(crate) fn scope_around(&self, node: Node<'_>) -> usize {
        ancestors(node)
            .find_map(|ancestor| self.by_node.get(&ancestor.id()).copied())
            .unwrap_or(0)
    }

    /// Marks the scope around a glob import.
    pub(crate) fn add_glob(&mut self, node: Node<'_>) {
        let scope = self.scope_around(node);
        self.has_glob[scope] = true;
    }

    /// Places a binding of `name` (whose text is `text`).
    pub(crate) fn bind(&mut self, name: Node<'tree>, text: &'tree str, rule: BindingRule, target: BindingTarget) {
        let scope = self.placement(name, rule);
        self.bind_in(scope, name.start_byte(), text, rule, target);
    }

    /// The scope a binding of `name` under `rule` is placed in.
    pub(crate) fn placement(&self, name: Node<'_>, rule: BindingRule) -> usize {
        let mut scope = self.scope_around(name);
        if rule.placement == Placement::Innermost && self.names_itself(scope, name) {
            scope = self.parents[scope].unwrap_or(scope);
        }
        if rule.placement == Placement::FunctionScope
            && let Some(rules) = self.rules
        {
            while let Some(parent) = self.parents[scope]
                && !rules.is_function_scope(self.nodes[scope].kind())
            {
                scope = parent;
            }
        }
        scope
    }

    /// Places a binding of `text`, whose name starts at `name_start`, in
    /// `scope`.
    pub(crate) fn bind_in(
        &mut self,
        scope: usize,
        name_start: usize,
        text: &'tree str,
        rule: BindingRule,
        target: BindingTarget,
    ) {
        let idx = self.bindings.len();
        self.bindings.push(Binding { name_start, rule, target });
        self.table.entry((scope, text)).or_default().push(idx);
    }

    /// The scope `node` itself is, if it is one.
    pub(crate) fn scope_of(&self, node: Node<'_>) -> Option<usize> {
        self.by_node.get(&node.id()).copied()
    }

    pub(crate) fn node(&self, scope: usize) -> Node<'tree> {
        self.nodes[scope]
    }

    pub(crate) fn parent(&self, scope: usize) -> Option<usize> {
        self.parents[scope]
    }

    /// The number of scopes, the root included; scope indexes are below it.
    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn has_glob(&self, scope: usize) -> bool {
        self.has_glob[scope]
    }

    /// The scope node `scope` declares `name` as its own name (`fn f`,
    /// `class C`), which is visible around the scope, not inside it.
    fn names_itself(&self, scope: usize, name: Node<'_>) -> bool {
        let node = self.nodes[scope];
        !matches!(node.kind(), "function_expression" | "function" | "class")
            && node.child_by_field_name("name").is_some_and(|own| own.id() == name.id())
    }

    pub(crate) fn binding(&self, idx: usize) -> &Binding {
        &self.bindings[idx]
    }

    /// Looks `name` up in the given categories from the reference `node`,
    /// innermost scope first.
    pub(crate) fn lookup(&self, node: Node<'_>, name: &str, categories: u8) -> Lookup {
        self.lookup_from(self.scope_around(node), node.start_byte(), name, categories)
    }

    /// [`Self::lookup`] from `scope` at byte `position`.
    pub(crate) fn lookup_from(&self, scope: usize, position: usize, name: &str, categories: u8) -> Lookup {
        let mut scope = Some(scope);
        while let Some(current) = scope {
            if let Some(found) = self.visible_in(current, position, name, categories) {
                return Lookup::Found(found);
            }
            if self.has_glob[current] {
                return Lookup::Glob;
            }
            scope = self.parents[current];
        }
        Lookup::NotFound
    }

    /// The bindings of `name` in `scope` alone (no climbing) that are
    /// visible at byte `position` in the given categories, `None` when it
    /// binds no such name: among ordered ones (Rust `let`) the latest wins.
    pub(crate) fn visible_in(&self, scope: usize, position: usize, name: &str, categories: u8) -> Option<Vec<usize>> {
        let bucket = self.table.get(&(scope, name))?;
        let visible = bucket.iter().copied().filter(|&idx| {
            let rule = self.bindings[idx].rule;
            rule.categories & categories != 0 && rule.visible_from <= position
        });
        let (ordered, free): (Vec<usize>, Vec<usize>) = visible.partition(|&idx| self.bindings[idx].rule.ordered);
        if let Some(&latest) = ordered.iter().max_by_key(|&&idx| self.bindings[idx].name_start) {
            return Some(vec![latest]);
        }
        (!free.is_empty()).then_some(free)
    }
}

/// `node`'s ancestors, nearest first.
pub(crate) fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), Node::parent)
}
