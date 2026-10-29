//! The named-type reference graph and the cycles it admits.
//!
//! Before expansion, the resolver walks every named type a schema reaches,
//! once each, and records each reference between two definitions as an edge.
//! An edge **descends** when its reference sits where a child value is
//! checked: an object property, a dictionary value, an array item, or a tuple
//! slot. An alias, a union arm, and anything inside a `function(...)`
//! signature do not descend.
//!
//! ```mermaid
//! flowchart LR
//!     discover["discover reachable definitions"] --> scc["group mutually reachable definitions"]
//!     scc --> drop["drop every descending edge"]
//!     drop --> check{"cycle left?"}
//!     check -- yes --> reject["NonDescendingCycle naming only those references"]
//!     check -- no --> admit["members of every cyclic group become references"]
//! ```
//!
//! A cycle survives only if it can revisit the same value without moving to
//! a child (`a: b` with `b: a`, or `a: [a, string]`), which no finite
//! document can satisfy and a naive walker would loop on. Every other cycle
//! describes a nesting grammar with a finite leaf.
//!
//! The same walk counts **use sites**: each distinct authored value
//! reference, identified by the namespace it is written in, its full source
//! path, and its target. The root's declarations and every definition a value
//! reaches are walked once each, so two uses of a parent never multiply the
//! single reference written in the parent's body, and a reference reached
//! only through a `function(...)` signature counts for nothing. An acyclic
//! definition with two or more sites whose value is **structured** (an object
//! or dictionary, a tuple, or a list or union of something structured) is
//! selected for storage as a shared definition.

use std::{
    collections::{HashMap, HashSet},
    fs,
    ops::Range,
    path::PathBuf,
};

use indexmap::IndexMap;

use super::{ImportEngine, Namespace, pattern_key_source_name};
use crate::markdown::schemas::{
    SchemaArm, SchemaShape, SimplifiedSchema,
    errors::{CycleReference, SchemaError},
    simplified::{
        DefKey, SchemaReferenceSite, SchemaSourceId, SchemaSourcePath, SchemaSpanKind,
        parse_standalone_schema_document,
        types::{PropertyAtom, PropertyDef, TypeExpr},
    },
};

/// One reference from a definition's body to a definition.
struct Edge {
    from: usize,
    to: usize,
    descends: bool,
    site: SchemaReferenceSite,
}

/// Where a reference sits, which decides whether its edge descends.
#[derive(Clone, Copy)]
struct Position {
    /// Some enclosing step entered a child value.
    descends: bool,
    /// The reference is inside a `function(...)` signature, which describes
    /// a callable rather than a value: its references never descend, and one
    /// that does not resolve is left alone, as expansion never reads it.
    in_function: bool,
}

impl Position {
    const TOP: Self = Self {
        descends: false,
        in_function: false,
    };

    fn child(self) -> Self {
        Self {
            descends: !self.in_function,
            ..self
        }
    }
}

#[derive(Default)]
struct Graph {
    /// Each reached definition with the namespace that declares it, in
    /// first-reached order.
    nodes: IndexMap<DefKey, Namespace>,
    edges: Vec<Edge>,
    /// The definition each authored reference resolved to, by the body it is
    /// written in (`None` for the root) and its path there, so a body's
    /// top-level alias can be followed without resolving it again.
    targets: HashMap<(Option<usize>, SchemaSourcePath), usize>,
    /// Every value reference: the body it is written in and its target.
    value_sites: Vec<(Option<usize>, usize)>,
    /// The distinct value sites already recorded, by target, namespace, and
    /// path, so the root walk and a walk of the root's own named types count
    /// one authored reference once.
    seen_sites: HashSet<(usize, SchemaSourceId, SchemaSourcePath)>,
    /// One entry per distinct value site, in discovery order.
    counted: Vec<(Option<usize>, usize)>,
}

/// The definitions expansion treats as references rather than inlining.
pub(super) struct Discovery {
    /// The members of every reference cycle.
    pub(super) cyclic: HashSet<DefKey>,
    /// The acyclic definitions selected for storage: structured values with
    /// at least two distinct authored use sites.
    pub(super) shared: HashSet<DefKey>,
}

impl ImportEngine<'_> {
    /// Discovers the definitions `schema` reaches and returns those that take
    /// part in a reference cycle and those selected for sharing.
    ///
    /// A reference that does not resolve adds no edge; expansion reports it
    /// in its usual order.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::NonDescendingCycle`] naming every reference of each
    /// cycle that never descends into a child value.
    pub(super) fn discover(
        &mut self,
        schema: &SimplifiedSchema,
        current: &Namespace,
        root: &SchemaSourcePath,
    ) -> Result<Discovery, SchemaError> {
        let mut graph = Graph::default();
        match schema {
            SimplifiedSchema::Single(shape) => {
                self.discover_shape(shape, current, root, Position::TOP, None, &mut graph);
            }
            SimplifiedSchema::Union(arms) => {
                for (index, arm) in arms.iter().enumerate() {
                    if let SchemaArm::Inline(shape) = arm {
                        self.discover_shape(shape, current, &root.union_arm(index), Position::TOP, None, &mut graph);
                    }
                }
            }
        }
        let mut next = 0;
        while let Some((key, namespace)) = graph.nodes.get_index(next).map(|(key, ns)| (key.clone(), ns.clone())) {
            let def = namespace
                .named_types
                .get(&key.name)
                .expect("a discovered definition exists in its namespace");
            let path = SchemaSourcePath::root().property(key.name.as_str());
            self.discover_def(def, &namespace, &path, Position::TOP, Some(next), &mut graph);
            next += 1;
        }
        let cyclic = graph.admit()?;
        let shared = graph.shared(&cyclic);
        Ok(Discovery { cyclic, shared })
    }

    fn discover_shape(
        &mut self,
        shape: &SchemaShape,
        namespace: &Namespace,
        path: &SchemaSourcePath,
        position: Position,
        from: Option<usize>,
        graph: &mut Graph,
    ) {
        for (name, def) in &shape.properties {
            self.discover_def(def, namespace, &path.property(name.as_str()), position, from, graph);
        }
        for pattern in &shape.pattern_keys {
            let path = path.property(pattern_key_source_name(&pattern.key));
            self.discover_def(&pattern.def, namespace, &path, position, from, graph);
        }
    }

    fn discover_def(
        &mut self,
        def: &PropertyDef,
        namespace: &Namespace,
        path: &SchemaSourcePath,
        position: Position,
        from: Option<usize>,
        graph: &mut Graph,
    ) {
        match def {
            PropertyDef::Single(atom) => self.discover_atom(atom, namespace, path, position, from, graph),
            PropertyDef::Union(arms) => {
                for (index, arm) in arms.iter().enumerate() {
                    self.discover_atom(arm, namespace, &path.union_arm(index), position, from, graph);
                }
            }
        }
    }

    /// Paths mirror the ones expansion reports, so a cycle's sites address
    /// the same places in the source map.
    fn discover_atom(
        &mut self,
        atom: &PropertyAtom,
        namespace: &Namespace,
        path: &SchemaSourcePath,
        position: Position,
        from: Option<usize>,
        graph: &mut Graph,
    ) {
        let position = if atom.is_array { position.child() } else { position };
        match &atom.ty {
            TypeExpr::Imported { name, reference } => {
                self.discover_reference(name, reference, namespace, path, position, from, graph);
            }
            TypeExpr::InlineObject(shape) => {
                self.discover_shape(shape, namespace, path, position.child(), from, graph);
            }
            TypeExpr::Union(arms) => {
                for (index, arm) in arms.iter().enumerate() {
                    self.discover_atom(arm, namespace, &path.union_arm(index), position, from, graph);
                }
            }
            TypeExpr::Tuple(tuple) => {
                for (index, element) in tuple.elements.iter().enumerate() {
                    self.discover_def(&element.def, namespace, &path.union_arm(index), position.child(), from, graph);
                }
                if let Some(spread) = &tuple.spread {
                    let path = path.union_arm(tuple.elements.len());
                    self.discover_atom(spread, namespace, &path, position.child(), from, graph);
                }
            }
            TypeExpr::Function(function) => {
                let signature = Position {
                    descends: false,
                    in_function: true,
                };
                for parameter in function.parameters.as_slice() {
                    let path = path.property(parameter.name.as_str());
                    self.discover_def(&parameter.def, namespace, &path, signature, from, graph);
                }
                self.discover_def(&function.returns, namespace, path, signature, from, graph);
            }
            TypeExpr::Primitive(_) | TypeExpr::Ref(_) => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn discover_reference(
        &mut self,
        name: &str,
        reference: &str,
        namespace: &Namespace,
        path: &SchemaSourcePath,
        position: Position,
        from: Option<usize>,
        graph: &mut Graph,
    ) {
        let target = if position.in_function {
            // Expansion never reads a signature, so resolving one must not
            // change what the schema depends on or probed.
            let dependencies = self.dependencies.clone();
            let searched = self.searched.clone();
            let target = self.resolve_namespace(reference, namespace);
            self.dependencies = dependencies;
            self.searched = searched;
            target
        } else {
            self.resolve_namespace(reference, namespace)
        };
        let Ok(target) = target else {
            return;
        };
        if !target.named_types.contains_key(name) {
            return;
        }
        let key = DefKey::new(target.key.clone(), name);
        let to = match graph.nodes.get_index_of(&key) {
            Some(index) => index,
            None => graph.nodes.insert_full(key, target).0,
        };
        graph.targets.insert((from, path.clone()), to);
        if !position.in_function {
            graph.value_sites.push((from, to));
            let source = namespace.key.source_id();
            if graph.seen_sites.insert((to, source, path.clone())) {
                graph.counted.push((from, to));
            }
        }
        if let Some(from) = from {
            graph.edges.push(Edge {
                from,
                to,
                descends: position.descends,
                site: SchemaReferenceSite {
                    source: namespace.key.source_id(),
                    path: path.clone(),
                },
            });
        }
    }
}

impl Graph {
    /// The acyclic definitions with two or more distinct use sites written at
    /// the root or in a body a value reaches, whose value is structured.
    fn shared(&self, cyclic: &HashSet<DefKey>) -> HashSet<DefKey> {
        // A body reached only through a `function(...)` signature describes
        // a callable: its references are not value uses.
        let mut reachable = vec![false; self.nodes.len()];
        let mut queue: Vec<usize> = Vec::new();
        for &(from, to) in &self.value_sites {
            if from.is_none() && !reachable[to] {
                reachable[to] = true;
                queue.push(to);
            }
        }
        while let Some(node) = queue.pop() {
            for &(from, to) in &self.value_sites {
                if from == Some(node) && !reachable[to] {
                    reachable[to] = true;
                    queue.push(to);
                }
            }
        }
        let mut sites = vec![0usize; self.nodes.len()];
        for &(from, to) in &self.counted {
            if from.is_none_or(|from| reachable[from]) {
                sites[to] += 1;
            }
        }
        let mut structured = HashMap::new();
        let mut shared = HashSet::new();
        for (node, (key, _)) in self.nodes.iter().enumerate() {
            if sites[node] >= 2
                && !cyclic.contains(key)
                && self.is_structured(node, cyclic, &mut structured, &mut HashSet::new())
            {
                shared.insert(key.clone());
            }
        }
        shared
    }

    /// Whether `node`'s value is structured, following top-level aliases
    /// through the targets discovery already resolved. Memoized in `memo`.
    fn is_structured(
        &self,
        node: usize,
        cyclic: &HashSet<DefKey>,
        memo: &mut HashMap<usize, bool>,
        visiting: &mut HashSet<usize>,
    ) -> bool {
        if let Some(&known) = memo.get(&node) {
            return known;
        }
        let (key, namespace) = self.nodes.get_index(node).expect("a node index is in range");
        if cyclic.contains(key) {
            return true;
        }
        if !visiting.insert(node) {
            return false;
        }
        let path = SchemaSourcePath::root().property(key.name.as_str());
        let def = namespace
            .named_types
            .get(&key.name)
            .expect("a discovered definition exists in its namespace");
        let structured = match def {
            PropertyDef::Single(atom) => self.atom_is_structured(atom, node, &path, cyclic, memo, visiting),
            PropertyDef::Union(arms) => arms.iter().enumerate().any(|(index, arm)| {
                self.atom_is_structured(arm, node, &path.union_arm(index), cyclic, memo, visiting)
            }),
        };
        visiting.remove(&node);
        memo.insert(node, structured);
        structured
    }

    /// A list is structured through its items, so `[]` changes nothing here.
    fn atom_is_structured(
        &self,
        atom: &PropertyAtom,
        node: usize,
        path: &SchemaSourcePath,
        cyclic: &HashSet<DefKey>,
        memo: &mut HashMap<usize, bool>,
        visiting: &mut HashSet<usize>,
    ) -> bool {
        match &atom.ty {
            TypeExpr::InlineObject(_) | TypeExpr::Tuple(_) | TypeExpr::Ref(_) => true,
            TypeExpr::Union(arms) => arms.iter().enumerate().any(|(index, arm)| {
                self.atom_is_structured(arm, node, &path.union_arm(index), cyclic, memo, visiting)
            }),
            TypeExpr::Imported { .. } => self
                .targets
                .get(&(Some(node), path.clone()))
                .is_some_and(|&target| self.is_structured(target, cyclic, memo, visiting)),
            TypeExpr::Primitive(_) | TypeExpr::Function(_) => false,
        }
    }

    /// The members of every cyclic group, or the cycles that never descend.
    fn admit(&self) -> Result<HashSet<DefKey>, SchemaError> {
        if self.edges.is_empty() {
            return Ok(HashSet::new());
        }
        let all: Vec<&Edge> = self.edges.iter().collect();
        let mut cyclic = HashSet::new();
        for group in cyclic_groups(self.nodes.len(), &all) {
            cyclic.extend(group.into_iter().map(|node| self.key(node).clone()));
        }

        let still: Vec<&Edge> = self.edges.iter().filter(|edge| !edge.descends).collect();
        let mut cycles = Vec::new();
        for group in cyclic_groups(self.nodes.len(), &still) {
            let members: HashSet<usize> = group.into_iter().collect();
            let references = still
                .iter()
                .filter(|edge| members.contains(&edge.from) && members.contains(&edge.to))
                .map(|edge| self.cycle_reference(edge))
                .collect();
            cycles.push(references);
        }
        if cycles.is_empty() {
            return Ok(cyclic);
        }
        locate_reference_spans(&mut cycles);
        Err(SchemaError::NonDescendingCycle { cycles })
    }

    fn key(&self, node: usize) -> &DefKey {
        self.nodes.get_index(node).expect("an edge names a discovered node").0
    }

    fn cycle_reference(&self, edge: &Edge) -> CycleReference {
        let target = self.key(edge.to);
        CycleReference {
            site: edge.site.clone(),
            target: target.name.clone(),
            target_source: target.namespace.source_id(),
            span: None,
        }
    }
}

/// The strongly connected components of `edges` over `count` nodes that hold
/// a cycle (more than one member, or a self-edge), each in discovery order,
/// and the components in order of their first member.
fn cyclic_groups(count: usize, edges: &[&Edge]) -> Vec<Vec<usize>> {
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); count];
    let mut self_edge = vec![false; count];
    for edge in edges {
        successors[edge.from].push(edge.to);
        if edge.from == edge.to {
            self_edge[edge.from] = true;
        }
    }
    let mut groups: Vec<Vec<usize>> = tarjan(&successors)
        .into_iter()
        .filter(|group| group.len() > 1 || self_edge[group[0]])
        .map(|mut group| {
            group.sort_unstable();
            group
        })
        .collect();
    groups.sort_unstable_by_key(|group| group[0]);
    groups
}

/// Tarjan's strongly connected components, iteratively, so a long chain of
/// definitions cannot exhaust the stack.
fn tarjan(successors: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = successors.len();
    let mut index = vec![usize::MAX; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut groups = Vec::new();
    let mut next_index = 0;
    for start in 0..count {
        if index[start] != usize::MAX {
            continue;
        }
        // Each frame is a node and the position of its next successor.
        let mut frames = vec![(start, 0)];
        index[start] = next_index;
        low[start] = next_index;
        next_index += 1;
        stack.push(start);
        on_stack[start] = true;
        while let Some(&mut (node, ref mut cursor)) = frames.last_mut() {
            if let Some(&successor) = successors[node].get(*cursor) {
                *cursor += 1;
                if index[successor] == usize::MAX {
                    index[successor] = next_index;
                    low[successor] = next_index;
                    next_index += 1;
                    stack.push(successor);
                    on_stack[successor] = true;
                    frames.push((successor, 0));
                } else if on_stack[successor] {
                    low[node] = low[node].min(index[successor]);
                }
                continue;
            }
            frames.pop();
            if let Some(&(parent, _)) = frames.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == index[node] {
                let mut group = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    group.push(member);
                    if member == node {
                        break;
                    }
                }
                groups.push(group);
            }
        }
    }
    groups
}

/// Fills the span of each reference authored in a schema file from that
/// file's source map. Runs only on the error path; a reference in the inline
/// `$schema` keeps `None` for the consumer that holds the document text to
/// project, as for [`SchemaError::UnknownNamedType`].
fn locate_reference_spans(cycles: &mut [Vec<CycleReference>]) {
    let mut maps: HashMap<PathBuf, Option<crate::markdown::schemas::SchemaSourceMap>> = HashMap::new();
    for reference in cycles.iter_mut().flatten() {
        let SchemaSourceId::File(path) = &reference.site.source else {
            continue;
        };
        let map = maps.entry(path.clone()).or_insert_with(|| {
            let text = fs::read_to_string(path).ok()?;
            Some(parse_standalone_schema_document(&text, path).ok()??.source_map)
        });
        reference.span = map.as_ref().and_then(|map| first_span(map, &reference.site.path));
    }
}

fn first_span(map: &crate::markdown::schemas::SchemaSourceMap, path: &SchemaSourcePath) -> Option<Range<usize>> {
    map.spans(path, SchemaSpanKind::ImportName).first().cloned()
}
