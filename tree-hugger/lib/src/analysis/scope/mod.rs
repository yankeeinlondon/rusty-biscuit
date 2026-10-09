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
//! Not every enclosing scope is reachable. Leaving a Rust item (a nested
//! `fn`, `const`, `static`, type, trait or `impl`) leaves its enclosing
//! locals and generic parameters behind ([`Boundary::Item`]); leaving an
//! inline `mod` leaves every name but a `macro_rules!` macro behind
//! ([`Boundary::Module`]). A scope whose names may come from somewhere the
//! file does not say (a Rust glob import, an ECMA `with` body, the function
//! around a direct `eval`) ends the search as [`Lookup::Glob`].
//!
//! User contract: `tree-hugger/docs/references.md`.

use std::collections::HashMap;

use tree_sitter::Node;

use crate::file::parents::parent;
use crate::shared::ProgrammingLanguage;

mod ecma;
mod rust;

pub(crate) use ecma::is_direct_eval;

/// A type name: Rust `struct`/`enum`/`trait`/`mod`, TypeScript
/// `interface`/`type`, generic type parameters.
pub(crate) const TYPE: u8 = 1;
/// A value name: functions, variables, constants, parameters.
pub(crate) const VALUE: u8 = 2;
/// A Rust macro name.
pub(crate) const MACRO: u8 = 4;
/// An import without a source modifier that restricts its name category.
pub(crate) const ANY_CATEGORY: u8 = TYPE | VALUE | MACRO;

/// Where a binding is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// The innermost scope around the name, except that a scope node naming
    /// itself (`fn f`, `class C`) puts its name in the enclosing scope.
    Innermost,
    /// The nearest enclosing function scope, else the file (JavaScript
    /// `var`). A function whose parameters hold an expression keeps its
    /// body's `var`s in the body ([`LanguageRules::var_body`]), out of reach
    /// of those expressions.
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
    /// The first byte at which the binding is no longer visible; `None`
    /// for the rest of the scope. A Rust `if let` binding ends with its
    /// consequence, so the `else` branch does not see it.
    pub(crate) visible_until: Option<usize>,
    /// A binding of one item's body or header (a Rust `let`, parameter,
    /// pattern or generic parameter): a nested item cannot name it.
    pub(crate) local: bool,
    /// A Rust `let`: among the visible ones of a scope the latest wins.
    pub(crate) ordered: bool,
    /// Calling the name invokes the declaration itself (a function, a
    /// constructor, an import), not whatever a variable holds.
    pub(crate) callable: bool,
    /// A bare identifier pattern that reaches this binding names it instead
    /// of binding a new name: a Rust `const`, unit struct or `const`
    /// generic parameter.
    pub(crate) pattern_constant: bool,
}

/// A binding site found in the syntax tree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BindingSite<'tree> {
    pub(crate) name: Node<'tree>,
    pub(crate) rule: BindingRule,
    pub(crate) is_import: bool,
    /// A Rust identifier pattern written without `mut`, `ref` or `@`: it
    /// binds only when the name, looked up where it is written, does not
    /// reach a [`BindingRule::pattern_constant`] binding (rustc's rule; the
    /// spelling decides nothing). `None` for every other site.
    pub(crate) bare_pattern: Option<Refutability>,
}

/// Where a bare pattern name is written, which decides how an identity the
/// file cannot establish is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refutability {
    /// A `let` without `else`, a parameter or a `for` pattern: a constant
    /// there compiles only when its type has a single value (a unit struct,
    /// `const C: () = ()`), so an unestablished name is read as a binding.
    Irrefutable,
    /// A `match` arm, `if let`, `while let`, `let … else`: an unestablished
    /// name may as well be a constant, and stays unknown.
    Refutable,
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
    /// A bare Rust pattern name whose identity the file cannot establish:
    /// an import or a glob in view may supply a constant of that name, so
    /// it is a new binding or a reference. Whatever reaches it is uncertain.
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Binding {
    pub(crate) name_start: usize,
    pub(crate) rule: BindingRule,
    pub(crate) target: BindingTarget,
}

/// What leaving a scope for its parent leaves behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Boundary {
    None,
    /// A Rust item: the enclosing [`BindingRule::local`] bindings.
    Item,
    /// A Rust inline `mod`: everything but macros.
    Module,
}

/// A construct that may bind any name in a scope the file cannot tell.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Dynamic<'tree> {
    /// This node is a scope, every name of which may come from elsewhere
    /// (the body of an ECMA `with`, whose object's properties are names).
    Scope(Node<'tree>),
    /// The function around this node may gain any `var` (a direct `eval`
    /// call in sloppy code). Strict code cannot, but the file does not
    /// always say which it is, so every direct `eval` counts.
    FunctionOf(Node<'tree>),
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

    /// The extra scope `node` opens below itself: a TS/JS `with` statement's
    /// body. Read from the statement, so a walk never asks a node for its
    /// parent.
    pub(crate) fn extra_child_scope<'tree>(self, node: Node<'tree>) -> Option<Node<'tree>> {
        match self {
            Self::Rust => None,
            Self::Ecma { .. } => ecma::with_body(node),
        }
    }

    /// The binding sites `node` declares (visited once per node).
    pub(crate) fn binding_sites<'tree>(self, node: Node<'tree>, sites: &mut Vec<BindingSite<'tree>>) {
        match self {
            Self::Rust => rust::binding_sites(node, sites),
            Self::Ecma { typed } => ecma::binding_sites(node, typed, sites),
        }
    }

    /// Whether `node` (`use a::*`) makes every name of its scope possibly
    /// imported.
    pub(crate) fn is_glob_import(self, node: Node<'_>) -> bool {
        self == Self::Rust && node.kind() == "use_wildcard"
    }

    /// The language these rules read.
    pub(crate) fn language(self) -> ProgrammingLanguage {
        match self {
            Self::Rust => ProgrammingLanguage::Rust,
            Self::Ecma { typed: true } => ProgrammingLanguage::TypeScript,
            Self::Ecma { typed: false } => ProgrammingLanguage::JavaScript,
        }
    }

    /// The dynamic name source `node` introduces, if any.
    pub(crate) fn dynamic<'tree>(self, node: Node<'tree>, source: &str) -> Option<Dynamic<'tree>> {
        match self {
            Self::Rust => None,
            Self::Ecma { .. } => ecma::dynamic(node, self.language(), source),
        }
    }

    /// What leaving the scope node `node` (or any node between it and its
    /// parent scope) leaves behind.
    fn boundary(self, node: Node<'_>) -> Boundary {
        match self {
            Self::Rust => rust::boundary(node),
            Self::Ecma { .. } => Boundary::None,
        }
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

    /// The body of the function scope `node` when its `var`s live apart
    /// from its parameters: ECMAScript gives a function whose parameter
    /// list holds an expression (a default, a computed key) a separate var
    /// environment, so `function g(x = f()) { var f; }` calls the outer `f`.
    fn var_body(self, node: Node<'_>) -> Option<Node<'_>> {
        match self {
            Self::Rust => None,
            Self::Ecma { .. } => ecma::var_body(node),
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
    /// What leaving each scope for its parent leaves behind.
    boundaries: Vec<Boundary>,
    /// Per function scope, the body scope its [`Placement::FunctionScope`]
    /// bindings go to instead ([`LanguageRules::var_body`]).
    var_bodies: Vec<Option<usize>>,
    has_glob: Vec<bool>,
    bindings: Vec<Binding>,
    /// `(scope, name)` → binding indexes, in binding order, not position
    /// order: `visible_in` picks the latest `ordered` binding by name position.
    table: HashMap<usize, HashMap<String, Vec<usize>>>,
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
        let parents: Vec<Option<usize>> = nodes
            .iter()
            .map(|node| ancestors(*node).find_map(|ancestor| by_node.get(&ancestor.id()).copied()))
            .collect();
        let boundaries = nodes
            .iter()
            .zip(&parents)
            .map(|(node, parent)| {
                let Some(rules) = rules else { return Boundary::None };
                let parent = parent.map(|parent| nodes[parent].id());
                // The strongest boundary between the scope node (included)
                // and its parent scope's node (excluded).
                std::iter::once(*node)
                    .chain(ancestors(*node))
                    .take_while(|ancestor| Some(ancestor.id()) != parent)
                    .map(|ancestor| rules.boundary(ancestor))
                    .fold(Boundary::None, |strongest, next| match (strongest, next) {
                        (Boundary::Module, _) | (_, Boundary::Module) => Boundary::Module,
                        (Boundary::Item, _) | (_, Boundary::Item) => Boundary::Item,
                        _ => Boundary::None,
                    })
            })
            .collect();
        let var_bodies = nodes
            .iter()
            .map(|node| {
                let body = rules?.var_body(*node)?;
                by_node.get(&body.id()).copied()
            })
            .collect();
        Self {
            rules,
            boundaries,
            var_bodies,
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

    /// Marks the scope a [`Dynamic`] name source reaches. A direct `eval`
    /// outside every function is not marked: a `var` it adds there is the
    /// same global binding an assignment could change, never a nearer one.
    pub(crate) fn add_dynamic(&mut self, dynamic: Dynamic<'_>) {
        let scope = match dynamic {
            Dynamic::Scope(node) => self.scope_of(node),
            Dynamic::FunctionOf(node) => {
                let rule = BindingRule {
                    categories: ANY_CATEGORY,
                    placement: Placement::FunctionScope,
                    visible_from: 0,
                    visible_until: None,
                    local: false,
                    ordered: false,
                    callable: false,
                    pattern_constant: false,
                };
                Some(self.placement(node, rule)).filter(|&scope| scope != 0)
            }
        };
        if let Some(scope) = scope {
            self.has_glob[scope] = true;
        }
    }

    fn name_key<'name>(&self, name: &'name str) -> std::borrow::Cow<'name, str> {
        self.rules.map_or_else(|| std::borrow::Cow::Borrowed(name), |rules| super::name_identity::key(rules.language(), name))
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
            let mut below = None;
            while let Some(parent) = self.parents[scope]
                && !rules.is_function_scope(self.nodes[scope].kind())
            {
                below = Some(scope);
                scope = parent;
            }
            if let Some(body) = below
                && self.var_bodies[scope] == Some(body)
            {
                return body;
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
        let key = self.name_key(text).into_owned();
        self.table.entry(scope).or_default().entry(key).or_default().push(idx);
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
    /// `class C`), which is visible around the scope, not inside it. An ECMA
    /// function, generator or class expression's name is the opposite:
    /// visible only inside the expression.
    fn names_itself(&self, scope: usize, name: Node<'_>) -> bool {
        let node = self.nodes[scope];
        !matches!(node.kind(), "function_expression" | "function" | "generator_function" | "class")
            && node.child_by_field_name("name").is_some_and(|own| own.id() == name.id())
    }

    pub(crate) fn binding(&self, idx: usize) -> &Binding {
        &self.bindings[idx]
    }

    /// The number of bindings placed: the index the next one gets.
    pub(crate) fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Replaces what binding `idx` binds to; where it is placed is unchanged.
    pub(crate) fn set_target(&mut self, idx: usize, target: BindingTarget) {
        self.bindings[idx].target = target;
    }

    /// Looks `name` up in the given categories from the reference `node`,
    /// innermost scope first.
    pub(crate) fn lookup(&self, node: Node<'_>, name: &str, categories: u8) -> Lookup {
        self.lookup_from(self.scope_around(node), node.start_byte(), name, categories)
    }

    /// [`Self::lookup`] from `scope` at byte `position`, through the
    /// [`Boundary`] of every scope it leaves.
    pub(crate) fn lookup_from(&self, scope: usize, position: usize, name: &str, mut categories: u8) -> Lookup {
        let key = self.name_key(name);
        let name = key.as_ref();
        let mut scope = Some(scope);
        let mut items_only = false;
        while let Some(current) = scope {
            if let Some(found) = self.visible(current, position, name, categories, items_only) {
                return Lookup::Found(found);
            }
            if self.has_glob[current] {
                return Lookup::Glob;
            }
            match self.boundaries[current] {
                Boundary::None => {}
                Boundary::Item => items_only = true,
                Boundary::Module => {
                    items_only = true;
                    categories &= MACRO;
                    if categories == 0 {
                        return Lookup::NotFound;
                    }
                }
            }
            scope = self.parents[current];
        }
        Lookup::NotFound
    }

    /// The bindings of `name` in `scope` alone (no climbing) that are
    /// visible at byte `position` in the given categories, `None` when it
    /// binds no such name: among ordered ones (Rust `let`) the latest wins.
    pub(crate) fn visible_in(&self, scope: usize, position: usize, name: &str, categories: u8) -> Option<Vec<usize>> {
        self.visible(scope, position, &self.name_key(name), categories, false)
    }

    /// [`Self::visible_in`], without [`BindingRule::local`] bindings when
    /// `items_only`.
    fn visible(&self, scope: usize, position: usize, name: &str, categories: u8, items_only: bool) -> Option<Vec<usize>> {
        let bucket = self.table.get(&scope)?.get(name)?;
        let visible = bucket.iter().copied().filter(|&idx| {
            let rule = self.bindings[idx].rule;
            rule.categories & categories != 0
                && rule.visible_from <= position
                && rule.visible_until.is_none_or(|until| position < until)
                && !(items_only && rule.local)
        });
        // The latest ordered binding by name position (the last of equals),
        // else every unordered one.
        let mut latest: Option<usize> = None;
        let mut free = Vec::new();
        for idx in visible {
            if !self.bindings[idx].rule.ordered {
                free.push(idx);
            } else if latest.is_none_or(|best| self.bindings[idx].name_start >= self.bindings[best].name_start) {
                latest = Some(idx);
            }
        }
        if let Some(latest) = latest {
            return Some(vec![latest]);
        }
        (!free.is_empty()).then_some(free)
    }
}

/// `node`'s ancestors, nearest first.
pub(crate) fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(parent(node), |node| parent(*node))
}
