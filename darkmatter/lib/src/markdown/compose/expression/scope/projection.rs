//! The shared declared-type layer: what a site knows about a binding before
//! any guard refines it, and the stable subjects guards can refine.

use std::borrow::Cow;
use std::sync::Arc;

use serde_json::{Map, Value};

use super::Shape;
use super::raw::RawDeclarations;
use crate::markdown::compose::context::context_variable_descriptors;
use crate::markdown::compose::expression::binding::{Availability, BindingView, LocalId, NULL_ROOT};
use crate::markdown::compose::expression::fm_namespace::literal_index_key;
use super::LexicalTypes;
use crate::markdown::compose::expression::{
    Expr, FmPath, LOCAL_ROOT, fm_namespace_path, reserved_root_descriptors,
};
use crate::markdown::schemas::catalog::GlobalCatalog;
use crate::markdown::schemas::{
    DefinitionTable, EffectiveSchema, PropertyAtom, PropertyDef, SchemaArm, SchemaShape,
    SimplifiedSchema, TypeExpr,
};

/// The declarations one owner's sites share: its schema projection, its
/// static frontmatter, and the host roots its expressions may read.
///
/// Built once per document or owner and borrowed by every
/// [`TypeScope`](super::TypeScope) of that owner, so no site rebuilds the
/// schema. [`Self::for_schema`] is the complete projection of an effective
/// schema: its authored SimplifiedSchema first, then each matching trigger's
/// shape (a later one wins a shared property), then the conservative
/// projection of the merged JSON Schema ([`RawDeclarations`]), which carries
/// the baseline and every raw JSON Schema layer.
#[derive(Debug, Clone, Copy)]
pub struct DeclaredTypes<'s> {
    schema: Option<&'s SimplifiedSchema>,
    trigger_shapes: &'s [Arc<SchemaShape>],
    raw: Option<&'s RawDeclarations>,
    definitions: Option<&'s DefinitionTable>,
    frontmatter: Option<&'s Map<String, Value>>,
    host_roots: HostRoots<'s>,
    host_catalog: Option<&'s GlobalCatalog>,
}

#[derive(Debug, Clone, Copy)]
enum HostRoots<'s> {
    None,
    Names(&'s [&'s str]),
    Registered(&'s [std::borrow::Cow<'static, str>]),
    View(&'s BindingView),
}

/// A binding a guard can say something about, resolved through the binding
/// model: bare `x` and `fm.x` are the same [`BindingId::Document`] unless a
/// host global shadows bare `x`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subject {
    binding: BindingId,
    path: Vec<Member>,
}

/// The root binding of a [`Subject`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BindingId {
    /// A top-level document property, by key.
    Document(String),
    /// A reserved namespace (`ctx`, `env`).
    Namespace(String),
    /// A host-declared global (`seq`, `loop`, `err`, …).
    Global(String),
    /// A lexical local, by frame and slot rather than spelling, so `x` and
    /// `local.x` are one binding and equal spellings in two frames are not.
    Local(LocalId),
}

/// One constant step below a [`BindingId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Member {
    /// `.name` or `['name']`.
    Field(String),
    /// `[n]`; a negative index counts from the end.
    Index(i64),
}

impl Subject {
    /// A document property path, as a `set` destination names it: `a.b`
    /// is property `a`, member `b`.
    pub fn document_path(path: &str) -> Option<Self> {
        let mut segments = path.split('.');
        let key = segments.next().filter(|key| !key.is_empty())?;
        Some(Self {
            binding: BindingId::Document(key.to_string()),
            path: segments.map(|segment| Member::Field(segment.to_string())).collect(),
        })
    }

    /// The subject below local `id` at `prefix` (the member a destructured
    /// local names, or empty), below constant `rest` (`a.b`, or empty).
    pub(super) fn local(id: LocalId, prefix: &[Member], rest: &str) -> Self {
        Self {
            binding: BindingId::Local(id),
            path: prefix
                .iter()
                .cloned()
                .chain(
                    rest.split('.')
                        .filter(|segment| !segment.is_empty())
                        .map(|segment| Member::Field(segment.to_string())),
                )
                .collect(),
        }
    }

    pub fn binding(&self) -> &BindingId {
        &self.binding
    }

    pub fn path(&self) -> &[Member] {
        &self.path
    }

    /// This subject and each of its ancestors, longest first: `x.a.b`,
    /// `x.a`, `x`.
    pub fn ancestors_and_self(&self) -> impl Iterator<Item = Subject> + '_ {
        (0..=self.path.len()).rev().map(|length| Subject {
            binding: self.binding.clone(),
            path: self.path[..length].to_vec(),
        })
    }

    /// Whether a write to one of these subjects can change the other: the
    /// same binding, and one path a prefix of the other. An index step
    /// matches any member, since `x[-1]`, `x[2]`, and `x['2']` can name one
    /// element.
    pub(super) fn overlaps(&self, other: &Subject) -> bool {
        self.binding == other.binding
            && self.path.iter().zip(&other.path).all(|pair| match pair {
                (Member::Field(left), Member::Field(right)) => left == right,
                _ => true,
            })
    }
}

/// What the declared layer says about a [`Subject`].
#[derive(Debug, Clone, PartialEq)]
pub enum DeclaredType {
    /// A document property, or a constant path below one, with its projected
    /// type: optional where it or an object on its path may be absent.
    Property(PropertyDef),
    /// A top-level document property no layer declares.
    Undeclared,
    /// A document property or path whose type is `unknown`: the owner has no
    /// schema, a root-union arm leaves it undeclared, or the path leaves the
    /// declared shape (a dynamic key, an open object).
    NoProjection,
    /// A `ctx` member with a catalog descriptor.
    Context {
        /// Required, or defaulted to a non-null value.
        present: bool,
    },
    /// Any other reserved-namespace path.
    Namespace,
    /// A host global, with its static availability.
    Host(Availability),
}

/// How a variable path classifies under the owner's binding model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VariableClass<'p> {
    /// A document property: bare `key.rest` or `fm.key.rest`.
    Document { key: &'p str, rest: &'p str },
    /// Bare `fm`, the whole frontmatter.
    WholeFrontmatter,
    /// A reserved namespace root other than `fm`.
    Namespace { root: &'p str, rest: &'p str },
    /// A host global root.
    Host { root: &'p str, rest: &'p str },
    /// The reserved `local` root: `local.<name>.<rest>`. `name` is empty for
    /// bare `local`, which names no binding. Bare locals (`x`) classify as
    /// `Document`; only a [`TypeScope`](super::TypeScope)'s lexical layers
    /// can tell them apart, because `DeclaredTypes` owns no locals.
    Local { name: &'p str, rest: &'p str },
}

/// A top-level declaration lookup.
pub(crate) enum Declaration<'s> {
    /// The projected definition; owned when root-union arms or layers were
    /// folded into it.
    Found(Cow<'s, PropertyDef>),
    Undeclared,
    /// Declared, but its type is `unknown` (or the owner has no schema).
    NoProjection,
}

impl<'s> DeclaredTypes<'s> {
    /// The declared layer of an owner whose only layer is `schema`, with no
    /// static frontmatter and no host roots. [`Self::for_schema`] is the
    /// complete projection of an effective schema.
    pub const fn new(schema: Option<&'s SimplifiedSchema>) -> Self {
        Self {
            schema,
            trigger_shapes: &[],
            raw: None,
            definitions: None,
            frontmatter: None,
            host_roots: HostRoots::None,
            host_catalog: None,
        }
    }

    /// The declared layer of every layer of `schema`, after resolution and
    /// merging: the authored SimplifiedSchema, the matching trigger shapes,
    /// and the merged JSON Schema's [`RawDeclarations`], with recursive
    /// references resolving in [`EffectiveSchema::layered_definitions`].
    /// `None` is an owner with no schema.
    pub fn for_schema(schema: Option<&'s EffectiveSchema>) -> Self {
        let Some(schema) = schema else {
            return Self::new(None);
        };
        Self {
            trigger_shapes: schema.trigger_shapes(),
            raw: Some(schema.raw_declarations()),
            definitions: Some(schema.layered_definitions()),
            ..Self::new(schema.simplified.as_ref())
        }
    }

    /// Adds the table the projection's recursive references point into.
    /// Without it a recursive-typed property keeps its named reference, but
    /// the call check cannot see through it, so every call it reaches is
    /// category 4.
    pub fn with_definitions(mut self, definitions: &'s DefinitionTable) -> Self {
        self.definitions = Some(definitions);
        self
    }

    /// The table recursive references in the projection resolve in; empty
    /// when none was added.
    pub fn definitions(&self) -> &'s DefinitionTable {
        self.definitions.unwrap_or(&NO_DEFINITIONS)
    }

    /// Adds the owner's static frontmatter: a concrete scalar there is
    /// present until a write invalidates it.
    pub fn with_static_frontmatter(mut self, frontmatter: &'s Value) -> Self {
        self.frontmatter = frontmatter.as_object();
        self
    }

    /// Names the host roots the owner's expressions read, without
    /// availability (every one is available).
    pub fn with_host_globals(mut self, roots: &'s [&'s str]) -> Self {
        self.host_roots = HostRoots::Names(roots);
        self
    }

    /// [`Self::with_host_globals`] over an event registry's
    /// [host globals](crate::lifecycle::registry::EventRegistry::host_globals).
    pub(crate) fn with_registered_host_globals(mut self, roots: &'s [std::borrow::Cow<'static, str>]) -> Self {
        self.host_roots = HostRoots::Registered(roots);
        self
    }

    /// Uses `view`'s declared globals and their availability as host roots.
    pub fn with_binding_view(mut self, view: &'s BindingView) -> Self {
        self.host_roots = HostRoots::View(view);
        self
    }

    /// The host binding view, when the declared owner supplied one.
    pub fn binding_view(&self) -> Option<&BindingView> {
        match self.host_roots {
            HostRoots::View(view) => Some(view),
            _ => None,
        }
    }

    /// Types the host's globals by `catalog`, the host catalog its event
    /// registry carries. A host global the catalog does not declare is
    /// typed by Darkmatter's catalog (`err`, `timing`), else `unknown`.
    /// Typing a root does not declare it: only the host roots do.
    pub fn with_global_catalog(mut self, catalog: &'s GlobalCatalog) -> Self {
        self.host_catalog = Some(catalog);
        self
    }

    /// The stable subject `expr` reads: a variable, or constant member and
    /// index steps on one. A function result, a computed index, bare `fm`,
    /// the lazily refreshed `current` root, and the `null` spelling have
    /// none.
    pub fn subject(&self, expr: &Expr) -> Option<Subject> {
        self.subject_in(expr, None)
    }

    /// [`Self::subject`] with lexical locals: a bare name or `local.<name>`
    /// that `lexical` binds is a [`BindingId::Local`] subject, so `x` and
    /// `local.x` are one subject while `fm.x` stays a document subject.
    /// `local.<name>` with no such local has no subject.
    pub(crate) fn subject_in(&self, expr: &Expr, lexical: Option<&LexicalTypes>) -> Option<Subject> {
        match expr {
            Expr::Paren(inner) => self.subject_in(inner, lexical),
            Expr::Variable(path) => self.variable_subject_in(path, lexical),
            Expr::MemberAccess { base, name } => {
                let mut subject = self.subject_in(base, lexical)?;
                subject.path.extend(name.split('.').map(|segment| Member::Field(segment.to_string())));
                Some(subject)
            }
            Expr::Index { base, index } => {
                if let Some(key) = literal_index_key(base, index) {
                    return Some(Subject {
                        binding: BindingId::Document(key.to_string()),
                        path: Vec::new(),
                    });
                }
                let mut subject = self.subject_in(base, lexical)?;
                subject.path.push(constant_member(index)?);
                Some(subject)
            }
            _ => None,
        }
    }

    /// The subject of the variable read `path`; see [`Self::subject`].
    pub(crate) fn variable_subject(&self, path: &str) -> Option<Subject> {
        self.variable_subject_in(path, None)
    }

    /// [`Self::variable_subject`] resolving locals through `lexical`.
    pub(crate) fn variable_subject_in(&self, path: &str, lexical: Option<&LexicalTypes>) -> Option<Subject> {
        if path == NULL_ROOT {
            return None;
        }
        let class = self.classify_variable(path);
        if let Some(lexical) = lexical {
            match class {
                VariableClass::Local { name, rest } => {
                    return lexical.subject(name, rest);
                }
                VariableClass::Document { key, rest }
                    if !crate::markdown::compose::expression::fm_namespace::is_fm_namespace(path) =>
                {
                    if let Some(subject) = lexical.subject(key, rest) {
                        return Some(subject);
                    }
                }
                _ => {}
            }
        }
        let (binding, rest) = match class {
            VariableClass::Document { key, rest } => (BindingId::Document(key.to_string()), rest),
            VariableClass::Namespace { root, rest } if root != CURRENT_ROOT => {
                (BindingId::Namespace(root.to_string()), rest)
            }
            VariableClass::Host { root, rest } => (BindingId::Global(root.to_string()), rest),
            _ => return None,
        };
        let path = rest
            .split('.')
            .filter(|segment| !segment.is_empty())
            .map(|segment| Member::Field(segment.to_string()))
            .collect();
        Some(Subject { binding, path })
    }

    /// Classifies `path` through the reserved namespaces, then the host
    /// roots, then document properties, as the evaluator resolves it.
    pub(crate) fn classify_variable<'p>(&self, path: &'p str) -> VariableClass<'p> {
        match fm_namespace_path(path) {
            Some(FmPath::Whole) => return VariableClass::WholeFrontmatter,
            Some(FmPath::Member(member)) => {
                let (key, rest) = split_root(member);
                return VariableClass::Document { key, rest };
            }
            None => {}
        }
        let (root, rest) = split_root(path);
        if root == LOCAL_ROOT {
            let (name, rest) = split_root(rest);
            VariableClass::Local { name, rest }
        } else if reserved_root_descriptors().iter().any(|descriptor| descriptor.name == root) {
            VariableClass::Namespace { root, rest }
        } else if self.host_availability(root).is_some() {
            VariableClass::Host { root, rest }
        } else {
            VariableClass::Document { key: root, rest }
        }
    }

    /// A top-level property's declaration, from the highest layer that
    /// declares it.
    ///
    /// A root-union document schema folds every arm (`root-level unions` in
    /// the declarations design): an arm that declares the property gives its
    /// type with that arm's optionality, an arm that does not gives the type
    /// a lower layer declares, else `unknown`, and the arms fold with
    /// [`Shape::union`]. An `unknown` result is [`Declaration::NoProjection`].
    pub(crate) fn declaration(&self, key: &str) -> Declaration<'s> {
        match self.schema {
            Some(SimplifiedSchema::Single(shape)) => {
                if let Some(def) = shape.properties.get(key) {
                    return Declaration::Found(Cow::Borrowed(def));
                }
            }
            Some(SimplifiedSchema::Union(arms)) => {
                if let Some(folded) = self.root_union_declaration(arms, key) {
                    return folded;
                }
            }
            None => {}
        }
        match self.lower_declaration(key) {
            Some(Some(def)) => Declaration::Found(Cow::Borrowed(def)),
            Some(None) => Declaration::NoProjection,
            None if self.schema.is_none() && self.trigger_shapes.is_empty() && self.raw.is_none() => {
                Declaration::NoProjection
            }
            None => Declaration::Undeclared,
        }
    }

    /// The fold of `arms` for `key`; `None` when no arm declares it.
    fn root_union_declaration(&self, arms: &'s [SchemaArm], key: &str) -> Option<Declaration<'s>> {
        let declared: Vec<Option<&'s PropertyDef>> = arms
            .iter()
            .map(|arm| match arm {
                SchemaArm::Inline(shape) => shape.properties.get(key),
                SchemaArm::FileRef(_) => None,
            })
            .collect();
        if declared.iter().all(Option::is_none) {
            return None;
        }
        // Arms that agree exactly keep the authored definition, constraints
        // and markers included.
        if let Some(Some(first)) = declared.first()
            && declared.iter().all(|def| *def == Some(*first))
        {
            return Some(Declaration::Found(Cow::Borrowed(*first)));
        }
        let lower = self.lower_declaration(key);
        let mut folded: Option<Shape> = None;
        for def in declared {
            let arm = match def.or(lower.flatten()) {
                Some(def) => Shape::from_definition(def),
                None => Shape::Unknown { non_null: false },
            };
            folded = Some(folded.map_or_else(|| arm.clone(), |folded| folded.union(&arm)));
        }
        Some(match folded.as_ref().and_then(Shape::to_definition) {
            Some(def) => Declaration::Found(Cow::Owned(def)),
            None => Declaration::NoProjection,
        })
    }

    /// The declaration of `key` below the authored schema: the last trigger
    /// shape that declares it, else the merged JSON Schema's projection.
    /// `Some(None)` is a raw declaration whose type is `unknown`.
    fn lower_declaration(&self, key: &str) -> Option<Option<&'s PropertyDef>> {
        if let Some(def) = self.trigger_shapes.iter().rev().find_map(|shape| shape.properties.get(key)) {
            return Some(Some(def));
        }
        self.raw.and_then(|raw| raw.get(key))
    }

    /// The design-time type of every top-level property the projection
    /// declares, keyed by name: the variable-type projection the design-time
    /// call check reads (through [`TypeScope`](super::TypeScope), so guards
    /// refine it).
    ///
    /// A property declared `A(required)` is `A`, constraints kept; an optional
    /// one adds `null` by the shared presence rule. A root union folds its
    /// arms ([`Self::declaration`]). A recursive-typed property is its named
    /// reference, never an expansion, so the projection is finite; a consumer
    /// sees through it with [`Self::definitions`]. A name absent from the map
    /// is `unknown`: undeclared, or declared with a type that folds to
    /// `unknown`. Passive: no file, no value, no expression.
    pub fn top_level_types(&self) -> std::collections::BTreeMap<String, Shape> {
        let mut names: Vec<&str> = Vec::new();
        match self.schema {
            None => {}
            Some(SimplifiedSchema::Single(shape)) => names.extend(shape.properties.keys().map(String::as_str)),
            Some(SimplifiedSchema::Union(arms)) => {
                for arm in arms {
                    if let SchemaArm::Inline(shape) = arm {
                        names.extend(shape.properties.keys().map(String::as_str));
                    }
                }
            }
        }
        for shape in self.trigger_shapes {
            names.extend(shape.properties.keys().map(String::as_str));
        }
        if let Some(raw) = self.raw {
            names.extend(raw.names());
        }
        let mut types = std::collections::BTreeMap::new();
        for name in names {
            if types.contains_key(name) {
                continue;
            }
            if let Declaration::Found(def) = self.declaration(name) {
                types.insert(name.to_string(), Shape::from_definition(&def));
            }
        }
        types
    }

    /// The declared type of `subject`.
    pub fn declared(&self, subject: &Subject) -> DeclaredType {
        self.projected(subject).0
    }

    /// The declared type of `subject` with the shape the declaration
    /// projects to.
    ///
    /// A constant path walks the declared shape one segment at a time
    /// ([`member_shape`]), opening a recursive reference only at the segment
    /// that crosses it. A reserved namespace and a host global are typed by
    /// their catalog declaration: a `ctx` member keeps its descriptor's
    /// presence, an available global's root is present, and an unavailable
    /// global has no type.
    pub(super) fn projected(&self, subject: &Subject) -> (DeclaredType, Shape) {
        let unknown = Shape::Unknown { non_null: false };
        match &subject.binding {
            BindingId::Document(key) => {
                let def = match self.declaration(key) {
                    Declaration::Found(def) => def,
                    Declaration::Undeclared => return (DeclaredType::Undeclared, unknown),
                    Declaration::NoProjection => return (DeclaredType::NoProjection, unknown),
                };
                if subject.path.is_empty() {
                    let shape = Shape::from_definition(&def);
                    return (DeclaredType::Property(def.into_owned()), shape);
                }
                let shape = walk(Shape::from_definition(&def), &subject.path, self.definitions());
                match shape.to_definition() {
                    Some(def) => (DeclaredType::Property(def), shape),
                    None => (DeclaredType::NoProjection, shape),
                }
            }
            BindingId::Namespace(root) => {
                let catalog = GlobalCatalog::darkmatter().global(root);
                let typed = |path: &[Member]| match catalog {
                    Some(global) => walk(
                        Shape::of_type(global.definition()).without_null(),
                        path,
                        global.definitions(),
                    ),
                    None => Shape::Unknown { non_null: false },
                };
                match subject.path.as_slice() {
                    [Member::Field(name)] if root == CONTEXT_ROOT => {
                        match context_variable_descriptors().iter().find(|descriptor| descriptor.name == name) {
                            Some(descriptor) => {
                                let present = descriptor.required
                                    || descriptor.default.as_ref().is_some_and(|value| !value.is_null());
                                let shape = typed(&subject.path);
                                let shape = if present { shape.without_null() } else { shape };
                                (DeclaredType::Context { present }, shape)
                            }
                            None => (DeclaredType::Namespace, typed(&subject.path)),
                        }
                    }
                    path => (DeclaredType::Namespace, typed(path)),
                }
            }
            // A local's type lives in the `TypeScope`'s lexical layers, which
            // `DeclaredTypes` does not own; `TypeScope::type_of` answers it.
            BindingId::Local(_) => (DeclaredType::NoProjection, unknown),
            BindingId::Global(root) => {
                let availability = self.host_availability(root).cloned().unwrap_or(Availability::Available);
                let declaration = self
                    .host_catalog
                    .and_then(|catalog| catalog.global(root))
                    .or_else(|| GlobalCatalog::darkmatter().global(root));
                let shape = match (&availability, declaration) {
                    (Availability::Unavailable(_), _) | (_, None) => unknown,
                    (availability, Some(global)) => {
                        let base = Shape::of_type(global.definition());
                        let base = match availability {
                            Availability::Available => base.without_null(),
                            _ => base.union(&Shape::of_literal(&Value::Null)),
                        };
                        walk(base, &subject.path, global.definitions())
                    }
                };
                (DeclaredType::Host(availability), shape)
            }
        }
    }

    /// Whether the static frontmatter holds a concrete scalar at `subject`.
    pub(super) fn has_static_scalar(&self, subject: &Subject) -> bool {
        let BindingId::Document(key) = &subject.binding else {
            return false;
        };
        subject.path.is_empty()
            && self
                .frontmatter
                .and_then(|frontmatter| frontmatter.get(key))
                .is_some_and(|value| matches!(value, Value::Bool(_) | Value::Number(_) | Value::String(_)))
    }

    fn host_availability(&self, root: &str) -> Option<&'s Availability> {
        match self.host_roots {
            HostRoots::None => None,
            HostRoots::Names(names) => names.contains(&root).then_some(&AVAILABLE),
            HostRoots::Registered(names) => names.iter().any(|name| name == root).then_some(&AVAILABLE),
            HostRoots::View(view) => view.global(root).map(|declaration| declaration.availability()),
        }
    }
}

/// `shape` after each constant step of `path`.
pub(super) fn walk(shape: Shape, path: &[Member], definitions: &DefinitionTable) -> Shape {
    path.iter().fold(shape, |shape, member| member_shape(&shape, member, definitions))
}

/// The type of `member` read from a value of `shape`: the union over its
/// alternatives, with `null` when the value itself may be `null` (a read
/// below `null` is `null`). An alternative the step cannot follow — an
/// undeclared or dynamic member, an open object, a scalar — contributes
/// `unknown`, which absorbs the rest.
fn member_shape(shape: &Shape, member: &Member, definitions: &DefinitionTable) -> Shape {
    let Shape::Arms { arms, nullable } = shape else {
        return Shape::Unknown { non_null: false };
    };
    let mut result = Shape::Arms { arms: Vec::new(), nullable: *nullable };
    for arm in arms {
        result = result.union(&arm_member(arm, member, definitions, definitions.len()));
    }
    result
}

/// [`member_shape`] for one alternative. A recursive reference opens its
/// definition's body for this one step; `aliases` bounds a chain of bodies
/// that are themselves references.
fn arm_member(atom: &PropertyAtom, member: &Member, definitions: &DefinitionTable, aliases: usize) -> Shape {
    let unknown = Shape::Unknown { non_null: false };
    if atom.is_array {
        return match member {
            Member::Index(_) => item_shape(atom).union(&Shape::of_literal(&Value::Null)),
            Member::Field(_) => unknown,
        };
    }
    match (&atom.ty, member) {
        (TypeExpr::InlineObject(shape), Member::Field(name)) => {
            shape.properties.get(name).map_or(unknown, Shape::from_definition)
        }
        (TypeExpr::Tuple(tuple), Member::Index(index)) => {
            let Ok(position) = usize::try_from(*index) else {
                return unknown;
            };
            match (tuple.elements.get(position), &tuple.spread) {
                (Some(element), _) if !element.optional => Shape::of_type(&element.def),
                (Some(element), _) => Shape::of_type(&element.def).union(&Shape::of_literal(&Value::Null)),
                (None, Some(spread)) => item_shape(spread).union(&Shape::of_literal(&Value::Null)),
                (None, None) => Shape::of_literal(&Value::Null),
            }
        }
        (TypeExpr::Ref(key), _) if aliases > 0 => match definitions.get(key) {
            Some(definition) => {
                let Shape::Arms { arms, nullable } = Shape::of_type(&definition.body) else {
                    return unknown;
                };
                let mut result = Shape::Arms { arms: Vec::new(), nullable };
                for arm in &arms {
                    result = result.union(&arm_member(arm, member, definitions, aliases - 1));
                }
                result
            }
            None => unknown,
        },
        _ => unknown,
    }
}

/// The values of one item of the array alternative `atom`.
pub(super) fn item_shape(atom: &PropertyAtom) -> Shape {
    let mut item = atom.clone();
    item.is_array = false;
    item.array_constraints.clear();
    match item.ty {
        TypeExpr::Union(arms) => Shape::of_type(&PropertyDef::Union(arms)),
        _ => Shape::of_type(&PropertyDef::Single(item)),
    }
}

const CONTEXT_ROOT: &str = "ctx";

static NO_DEFINITIONS: std::sync::LazyLock<DefinitionTable> = std::sync::LazyLock::new(DefinitionTable::default);

static AVAILABLE: Availability = Availability::Available;

/// `current` refreshes between expressions, so a fact about it would not
/// survive to the guarded site.
const CURRENT_ROOT: &str = "current";

fn split_root(path: &str) -> (&str, &str) {
    path.split_once('.').unwrap_or((path, ""))
}

/// A constant index step: a string key, or an integral number.
fn constant_member(index: &Expr) -> Option<Member> {
    match index {
        Expr::Paren(inner) => constant_member(inner),
        Expr::StringLiteral(key) => Some(Member::Field(key.clone())),
        Expr::NumberLiteral(number) => index_literal(*number).map(Member::Index),
        Expr::UnaryMinus(inner) => match inner.as_ref() {
            Expr::NumberLiteral(number) => index_literal(-*number).map(Member::Index),
            _ => None,
        },
        _ => None,
    }
}

/// A whole literal within `i64`, never saturated: a larger index is not a
/// constant step the analysis can follow.
fn index_literal(number: f64) -> Option<i64> {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0; // 2^63
    (number.fract() == 0.0 && (-LIMIT..LIMIT).contains(&number)).then_some(number as i64)
}
