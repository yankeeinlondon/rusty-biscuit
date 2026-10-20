//! The shared declared-type layer: what a site knows about a binding before
//! any guard refines it, and the stable subjects guards can refine.

use serde_json::{Map, Value};

use crate::markdown::compose::context::context_variable_descriptors;
use crate::markdown::compose::expression::binding::{Availability, BindingView, NULL_ROOT};
use crate::markdown::compose::expression::fm_namespace::literal_index_key;
use crate::markdown::compose::expression::{Expr, FmPath, fm_namespace_path, reserved_root_descriptors};
use crate::markdown::schemas::{PropertyDef, SchemaArm, SimplifiedSchema};

/// The declarations one owner's sites share: its schema projection, its
/// static frontmatter, and the host roots its expressions may read.
///
/// Built once per document or owner and borrowed by every
/// [`TypeScope`](super::TypeScope) of that owner, so no site rebuilds the
/// schema.
#[derive(Debug, Clone, Copy)]
pub struct DeclaredTypes<'s> {
    schema: Option<&'s SimplifiedSchema>,
    frontmatter: Option<&'s Map<String, Value>>,
    host_roots: HostRoots<'s>,
}

#[derive(Debug, Clone, Copy)]
enum HostRoots<'s> {
    None,
    Names(&'s [&'s str]),
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
    /// A top-level document property the schema declares.
    Property(PropertyDef),
    /// A top-level document property the schema does not declare.
    Undeclared,
    /// A document property, but the owner has no typed projection: no
    /// SimplifiedSchema, or a root union whose arms disagree on it.
    NoProjection,
    /// A path below a document property. Nested projection is planned, so
    /// the type is unknown.
    NestedProperty,
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
}

/// A top-level declaration lookup.
pub(crate) enum Declaration<'s> {
    Found(&'s PropertyDef),
    Undeclared,
    /// No schema, or a root union whose arms disagree.
    NoProjection,
}

impl<'s> DeclaredTypes<'s> {
    /// The declared layer of an owner whose projection is `schema`
    /// (`EffectiveSchema.simplified`), with no static frontmatter and no
    /// host roots.
    pub const fn new(schema: Option<&'s SimplifiedSchema>) -> Self {
        Self {
            schema,
            frontmatter: None,
            host_roots: HostRoots::None,
        }
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

    /// Uses `view`'s declared globals and their availability as host roots.
    pub fn with_binding_view(mut self, view: &'s BindingView) -> Self {
        self.host_roots = HostRoots::View(view);
        self
    }

    /// The stable subject `expr` reads: a variable, or constant member and
    /// index steps on one. A function result, a computed index, bare `fm`,
    /// the lazily refreshed `current` root, and the `null` spelling have
    /// none.
    pub fn subject(&self, expr: &Expr) -> Option<Subject> {
        match expr {
            Expr::Paren(inner) => self.subject(inner),
            Expr::Variable(path) => self.variable_subject(path),
            Expr::MemberAccess { base, name } => {
                let mut subject = self.subject(base)?;
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
                let mut subject = self.subject(base)?;
                subject.path.push(constant_member(index)?);
                Some(subject)
            }
            _ => None,
        }
    }

    /// The subject of the variable read `path`; see [`Self::subject`].
    pub(crate) fn variable_subject(&self, path: &str) -> Option<Subject> {
        if path == NULL_ROOT {
            return None;
        }
        let (binding, rest) = match self.classify_variable(path) {
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
        if reserved_root_descriptors().iter().any(|descriptor| descriptor.name == root) {
            VariableClass::Namespace { root, rest }
        } else if self.host_availability(root).is_some() {
            VariableClass::Host { root, rest }
        } else {
            VariableClass::Document { key: root, rest }
        }
    }

    /// A top-level property's declaration. A root union contributes one only
    /// when every arm that declares the property declares it identically.
    pub(crate) fn declaration(&self, key: &str) -> Declaration<'s> {
        let Some(schema) = self.schema else {
            return Declaration::NoProjection;
        };
        match schema {
            SimplifiedSchema::Single(shape) => shape.properties.get(key).map_or(Declaration::Undeclared, Declaration::Found),
            SimplifiedSchema::Union(arms) => {
                let mut found: Option<&'s PropertyDef> = None;
                for arm in arms {
                    let SchemaArm::Inline(shape) = arm else {
                        return Declaration::NoProjection;
                    };
                    match (shape.properties.get(key), found) {
                        (Some(def), Some(previous)) if def != previous => return Declaration::NoProjection,
                        (Some(def), _) => found = Some(def),
                        (None, _) => {}
                    }
                }
                found.map_or(Declaration::Undeclared, Declaration::Found)
            }
        }
    }

    /// The declared type of `subject`.
    pub fn declared(&self, subject: &Subject) -> DeclaredType {
        match &subject.binding {
            BindingId::Document(_) if !subject.path.is_empty() => DeclaredType::NestedProperty,
            BindingId::Document(key) => match self.declaration(key) {
                Declaration::Found(def) => DeclaredType::Property(def.clone()),
                Declaration::Undeclared => DeclaredType::Undeclared,
                Declaration::NoProjection => DeclaredType::NoProjection,
            },
            BindingId::Namespace(root) => match subject.path.as_slice() {
                [Member::Field(name)] if root == CONTEXT_ROOT => context_variable_descriptors()
                    .iter()
                    .find(|descriptor| descriptor.name == name)
                    .map_or(DeclaredType::Namespace, |descriptor| DeclaredType::Context {
                        present: descriptor.required || descriptor.default.as_ref().is_some_and(|value| !value.is_null()),
                    }),
                _ => DeclaredType::Namespace,
            },
            BindingId::Global(root) => {
                DeclaredType::Host(self.host_availability(root).cloned().unwrap_or(Availability::Available))
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
            HostRoots::View(view) => view.global(root).map(|declaration| declaration.availability()),
        }
    }
}

const CONTEXT_ROOT: &str = "ctx";

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
        Expr::NumberLiteral(number) if number.fract() == 0.0 => Some(Member::Index(*number as i64)),
        Expr::UnaryMinus(inner) => match inner.as_ref() {
            Expr::NumberLiteral(number) if number.fract() == 0.0 => Some(Member::Index(-(*number as i64))),
            _ => None,
        },
        _ => None,
    }
}
