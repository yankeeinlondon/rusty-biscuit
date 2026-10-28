//! The passive declared-type view of lexical locals.
//!
//! A [`LexicalTypes`] layer is the type-side twin of a runtime
//! [`LexicalScope`](crate::markdown::compose::expression::LexicalScope) frame:
//! it names the locals a callback or directive body introduces and the
//! [`Shape`] each is declared to have, without any value. A
//! [`TypeScope`](super::TypeScope) borrows its owner's declarations and owns
//! these layers, so a layer can also own the type definitions that its
//! local types refer to.
//!
//! Layers are immutable and share their parents through [`Arc`]. A local's
//! identity is a [`LocalId`], never its spelling. A binding built with
//! [`TypedLocal::aliasing`] reuses an existing id, so two callbacks that name
//! the same original item differently share that item's facts, while
//! [`TypedLocal::new`] mints a fresh subject, as a map result is.

use std::sync::Arc;

use super::Shape;
use super::projection::Subject;
use crate::markdown::compose::expression::binding::{LocalId, next_frame_id};
use crate::markdown::schemas::DefinitionTable;

/// One local a layer declares.
#[derive(Debug, Clone)]
pub struct TypedLocal {
    name: String,
    id: Option<LocalId>,
    shape: Shape,
}

impl TypedLocal {
    /// A local that is a new subject, distinct from every other binding.
    pub fn new(name: impl Into<String>, shape: Shape) -> Self {
        Self {
            name: name.into(),
            id: None,
            shape,
        }
    }

    /// A new-subject local declared by `definition`.
    pub fn declared(name: impl Into<String>, definition: &crate::markdown::schemas::PropertyDef) -> Self {
        Self::new(name, Shape::of_type(definition))
    }

    /// A local naming the subject `id` already identifies, so facts about
    /// either name are facts about both.
    pub fn aliasing(name: impl Into<String>, id: LocalId, shape: Shape) -> Self {
        Self {
            name: name.into(),
            id: Some(id),
            shape,
        }
    }
}

#[derive(Debug)]
struct DeclaredLocal {
    name: String,
    id: LocalId,
    shape: Shape,
}

/// One immutable layer of declared lexical locals. See the module
/// documentation.
#[derive(Debug)]
pub struct LexicalTypes {
    locals: Vec<DeclaredLocal>,
    definitions: Option<DefinitionTable>,
    parent: Option<Arc<LexicalTypes>>,
}

impl LexicalTypes {
    /// An outermost layer declaring `locals` in order.
    pub fn new(locals: Vec<TypedLocal>) -> Arc<Self> {
        Self::build(None, locals, None)
    }

    /// A nested layer declaring `locals`, shadowing this layer's.
    pub fn child(self: &Arc<Self>, locals: Vec<TypedLocal>) -> Arc<Self> {
        Self::build(Some(Arc::clone(self)), locals, None)
    }

    /// [`Self::new`] for locals whose types refer to `definitions`.
    pub fn new_defining(locals: Vec<TypedLocal>, definitions: DefinitionTable) -> Arc<Self> {
        Self::build(None, locals, Some(definitions))
    }

    /// [`Self::child`] for locals whose types refer to `definitions`. The
    /// table replaces, not extends, the enclosing one for this layer and
    /// below, so it must hold every definition those locals reach.
    pub fn child_defining(self: &Arc<Self>, locals: Vec<TypedLocal>, definitions: DefinitionTable) -> Arc<Self> {
        Self::build(Some(Arc::clone(self)), locals, Some(definitions))
    }

    fn build(parent: Option<Arc<Self>>, locals: Vec<TypedLocal>, definitions: Option<DefinitionTable>) -> Arc<Self> {
        let frame = next_frame_id();
        Arc::new(Self {
            locals: locals
                .into_iter()
                .enumerate()
                .map(|(slot, local)| DeclaredLocal {
                    id: local.id.unwrap_or_else(|| LocalId::new(frame, slot)),
                    name: local.name,
                    shape: local.shape,
                })
                .collect(),
            definitions,
            parent,
        })
    }

    /// The id of the nearest local named `name`.
    pub fn id_of(&self, name: &str) -> Option<LocalId> {
        self.nearest(name).map(|local| local.id)
    }

    /// The declared shape of the nearest local named `name`.
    pub fn shape_of_name(&self, name: &str) -> Option<&Shape> {
        self.nearest(name).map(|local| &local.shape)
    }

    /// The declared shape of the nearest binding with identity `id`.
    pub fn shape_of(&self, id: LocalId) -> Option<&Shape> {
        self.layers()
            .find_map(|layer| layer.locals.iter().find(|local| local.id == id))
            .map(|local| &local.shape)
    }

    /// Every visible name, nearest first, shadowed names once.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for local in self.layers().flat_map(|layer| layer.locals.iter()) {
            if !names.contains(&local.name.as_str()) {
                names.push(&local.name);
            }
        }
        names
    }

    /// The definitions of the nearest layer that owns a table.
    pub fn definitions(&self) -> Option<&DefinitionTable> {
        self.layers().find_map(|layer| layer.definitions.as_ref())
    }

    /// The subject of local `name` below the constant path `rest`.
    pub(super) fn subject(&self, name: &str, rest: &str) -> Option<Subject> {
        self.id_of(name).map(|id| Subject::local(id, rest))
    }

    fn nearest(&self, name: &str) -> Option<&DeclaredLocal> {
        self.layers()
            .find_map(|layer| layer.locals.iter().find(|local| local.name == name))
    }

    fn layers(&self) -> impl Iterator<Item = &LexicalTypes> {
        std::iter::successors(Some(self), |layer| layer.parent.as_deref())
    }
}
