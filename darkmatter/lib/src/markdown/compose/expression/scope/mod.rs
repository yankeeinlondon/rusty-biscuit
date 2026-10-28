//! One type scope: the type of each binding at one site, as its declaration
//! refined by every guard that encloses the site.
//!
//! A [`TypeScope`] is immutable. Entering a guarded region calls
//! [`TypeScope::assume`], the only producer of guard facts, which returns a
//! new scope sharing its parent's facts. A write calls
//! [`TypeScope::invalidate`]; the end of a conditional calls
//! [`TypeScope::join`]. Consumers ask [`TypeScope::type_of`]; presence is a
//! consequence of that type ([`ScopedType::proves_present`]), never a separate
//! question.
//!
//! Construction, fact lookup, joins, and invalidation are passive: they read
//! the AST and the declared layer only, evaluate no expression, run no
//! function handler, and perform no I/O.
//!
//! There is no `Default`: a site with no enclosing guard says so with
//! [`TypeScope::unguarded`].
//!
//! ```
//! use darkmatter::markdown::compose::expression::{DeclaredTypes, Polarity, TypeScope, parse};
//!
//! let declared = DeclaredTypes::new(None);
//! let site = TypeScope::unguarded(&declared);
//! let guard = parse("ctx.dirty_files && commit_message").unwrap();
//! let branch = site.assume(&guard, Polarity::True);
//! let message = parse("commit_message").unwrap();
//! assert!(branch.type_of_expr(&message).unwrap().guard_presence().is_some());
//! assert!(!site.type_of_expr(&message).unwrap().proves_present());
//! ```

mod facts;
mod iterable;
mod lexical;
mod projection;
mod raw;
mod shape;

use std::collections::HashSet;
use std::sync::Arc;

use crate::markdown::compose::expression::Expr;
use crate::markdown::schemas::DefinitionTable;
use crate::markdown::span::SourceSpan;

pub use iterable::{ItemFrame, PatternMismatch, SourceProjection, project_pattern, project_source};
pub use lexical::{LexicalTypes, TypedLocal};
pub use projection::{BindingId, DeclaredType, DeclaredTypes, Member, Subject};
pub use raw::RawDeclarations;
pub(crate) use projection::{Declaration, VariableClass};
pub use shape::{PresenceEvidence, ScopedType, Shape, SortKeySupport};
pub(crate) use shape::is_exact_fact_type;

/// Which side of a guard a region is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// The guard held: `then`, a `::block`'s body, a ternary's first branch.
    True,
    /// The guard failed: `else`, a ternary's second branch.
    False,
}

impl Polarity {
    /// The other side; `!` swaps sides.
    pub fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
        }
    }
}

/// Where a guard fact came from, for diagnostics and approval provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardOrigin {
    label: String,
    span: Option<SourceSpan>,
}

impl GuardOrigin {
    /// A guard identified by `label`: its authored location
    /// (`success[4].when`) or its expression text.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            span: None,
        }
    }

    /// Attaches the guard's byte span in its source.
    pub fn with_span(mut self, span: SourceSpan) -> Self {
        self.span = Some(span);
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn span(&self) -> Option<&SourceSpan> {
        self.span.as_ref()
    }
}

/// What a mutation may write.
#[derive(Debug, Clone, PartialEq)]
pub enum WriteSet {
    /// These subjects, and every subject above or below them.
    Paths(Vec<Subject>),
    /// Any document property: a write whose destination is not static, or
    /// an effect that can replace document bindings without declaring them.
    AllDocument,
}

impl WriteSet {
    /// What a mutation of the authored destination `path` may write: that
    /// document path (`a.b`, `fm.a.b`) when it is static, otherwise every
    /// document property, since an interpolated or computed destination may
    /// name any of them.
    pub fn destination(path: &str) -> Self {
        let path = crate::markdown::compose::expression::fm_namespace::strip_fm_prefix(path);
        let is_static = !path.is_empty()
            && path
                .split('.')
                .all(|segment| !segment.is_empty() && !segment.contains(['{', '}', '[', ']', '$', '(', ')', '`']));
        match Subject::document_path(path) {
            Some(subject) if is_static => Self::Paths(vec![subject]),
            _ => Self::AllDocument,
        }
    }

    fn touches(&self, subject: &Subject) -> bool {
        match self {
            Self::Paths(paths) => paths.iter().any(|path| path.overlaps(subject)),
            Self::AllDocument => matches!(subject.binding(), BindingId::Document(_)),
        }
    }
}

/// The types at one site. See the module documentation.
#[derive(Debug, Clone)]
pub struct TypeScope<'d> {
    declared: &'d DeclaredTypes<'d>,
    head: Option<Arc<Link>>,
    lexical: Option<Arc<LexicalTypes>>,
}

/// One step of a scope's persistent history, newest first.
#[derive(Debug)]
struct Link {
    step: Step,
    parent: Option<Arc<Link>>,
}

#[derive(Debug)]
enum Step {
    /// The complete type of `subject` from here on.
    Fact { subject: Subject, ty: Box<ScopedType> },
    /// Facts about touched subjects no longer hold.
    Write(WriteSet),
}

impl<'d> TypeScope<'d> {
    /// The scope of a site that no guard encloses.
    pub fn unguarded(declared: &'d DeclaredTypes<'d>) -> Self {
        Self {
            declared,
            head: None,
            lexical: None,
        }
    }

    pub fn declared(&self) -> &'d DeclaredTypes<'d> {
        self.declared
    }

    /// This site inside the lexical `layer`. Facts already assumed stay, and
    /// a fact about a binding the layer aliases (by local id) applies to
    /// it under its new name. The scope this was called on is unchanged, so
    /// dropping the result restores the outer meaning.
    #[must_use]
    pub fn within(&self, layer: Arc<LexicalTypes>) -> Self {
        Self {
            declared: self.declared,
            head: self.head.clone(),
            lexical: Some(layer),
        }
    }

    /// This site with its lexical layer replaced by `lexical`, keeping every
    /// fact. Use it to drop one callback's parameters after typing it, so the
    /// next callback sees the enclosing locals but not its sibling's.
    #[must_use]
    pub fn with_lexical(&self, lexical: Option<Arc<LexicalTypes>>) -> Self {
        Self {
            declared: self.declared,
            head: self.head.clone(),
            lexical,
        }
    }

    /// The innermost lexical layer at this site, if any.
    pub fn lexical(&self) -> Option<&Arc<LexicalTypes>> {
        self.lexical.as_ref()
    }

    /// The type definitions in force: the nearest lexical layer that owns a
    /// table, else the declared layer's.
    pub fn definitions(&self) -> &DefinitionTable {
        self.lexical
            .as_deref()
            .and_then(LexicalTypes::definitions)
            .unwrap_or_else(|| self.declared.definitions())
    }

    /// The stable subject `expr` reads at this site, resolving locals before
    /// document properties; see [`DeclaredTypes::subject`].
    pub fn subject(&self, expr: &Expr) -> Option<Subject> {
        self.declared.subject_in(expr, self.lexical.as_deref())
    }

    /// Whether the variable read `path` resolves to a lexical local here.
    pub fn resolves_to_local(&self, path: &str) -> bool {
        self.declared
            .variable_subject_in(path, self.lexical.as_deref())
            .is_some_and(|subject| matches!(subject.binding(), BindingId::Local(_)))
    }

    /// The scope of the region on `polarity`'s side of `guard`.
    pub fn assume(&self, guard: &Expr, polarity: Polarity) -> Self {
        self.assume_at(guard, polarity, GuardOrigin::new(guard.to_string()))
    }

    /// [`Self::assume`], recording `origin` as the guard's provenance.
    pub fn assume_at(&self, guard: &Expr, polarity: Polarity, origin: GuardOrigin) -> Self {
        facts::assume(self, guard, polarity, &origin)
    }

    /// The type of `subject` at this site.
    pub fn type_of(&self, subject: &Subject) -> ScopedType {
        let mut link = self.head.as_deref();
        let mut written = false;
        while let Some(current) = link {
            match &current.step {
                Step::Fact { subject: fact, ty } if fact == subject => return ty.as_ref().clone(),
                Step::Write(writes) if writes.touches(subject) => {
                    written = true;
                    break;
                }
                _ => {}
            }
            link = current.parent.as_deref();
        }
        let (declared, shape) = self.projected(subject);
        ScopedType::declared_base(declared, shape, self.declared.has_static_scalar(subject), written)
    }

    /// The type of the stable subject `expr` reads, or `None` when it reads
    /// none (a function result, a computed index).
    pub fn type_of_expr(&self, expr: &Expr) -> Option<ScopedType> {
        self.subject(expr).map(|subject| self.type_of(&subject))
    }

    /// The declared type and shape of `subject`: a local's from the lexical
    /// layers, anything else from the declared layer.
    fn projected(&self, subject: &Subject) -> (DeclaredType, Shape) {
        let BindingId::Local(id) = subject.binding() else {
            return self.declared.projected(subject);
        };
        let Some(base) = self.lexical.as_ref().and_then(|layer| layer.shape_of(*id)) else {
            return (DeclaredType::NoProjection, Shape::Unknown { non_null: false });
        };
        let shape = projection::walk(base.clone(), subject.path(), self.definitions());
        match shape.to_definition() {
            Some(def) => (DeclaredType::Property(def), shape),
            None => (DeclaredType::NoProjection, shape),
        }
    }

    /// Every subject a guard proves present at this site, in no particular
    /// order. A subject a later write touched is not listed.
    pub fn guarded_subjects(&self) -> Vec<Subject> {
        let mut subjects: Vec<Subject> = Vec::new();
        let mut link = self.head.as_deref();
        while let Some(current) = link {
            if let Step::Fact { subject, .. } = &current.step
                && !subjects.contains(subject)
            {
                subjects.push(subject.clone());
            }
            link = current.parent.as_deref();
        }
        subjects.retain(|subject| self.type_of(subject).guard_presence().is_some());
        subjects
    }

    /// Whether a guard proves `subject` or one of its ancestors present, so
    /// its root binding exists: `x` proven present covers a read of `x.name`.
    /// A fact about `x.a` covers neither `x` nor its sibling `x.b`.
    pub fn guards_path_or_ancestor(&self, subject: &Subject) -> bool {
        self.head.is_some()
            && subject
                .ancestors_and_self()
                .any(|path| self.type_of(&path).guard_presence().is_some())
    }

    /// This scope after a mutation that may write `writes`. Check the
    /// mutation's own right-hand side under `self` first.
    pub fn invalidate(&self, writes: &WriteSet) -> Self {
        self.push(Step::Write(writes.clone()))
    }

    /// What holds after either of two continuing routes, both derived from
    /// one incoming scope: a subject keeps only what both routes know, and a
    /// possible write on either route invalidates the incoming facts it
    /// touches.
    pub fn join(left: &Self, right: &Self) -> Self {
        debug_assert!(std::ptr::eq(left.declared, right.declared), "joined scopes share one declared layer");
        let left_steps = left.steps_since_shared(right);
        let right_steps = right.steps_since_shared(left);
        let mut joined = Self {
            declared: left.declared,
            head: left.shared_ancestor(right),
            lexical: left.lexical.clone(),
        };
        let mut touched: Vec<&Subject> = Vec::new();
        for step in left_steps.iter().chain(&right_steps) {
            match step {
                Step::Write(writes) => joined = joined.push(Step::Write(writes.clone())),
                Step::Fact { subject, .. } if !touched.contains(&subject) => touched.push(subject),
                Step::Fact { .. } => {}
            }
        }
        for subject in touched {
            let ty = left.type_of(subject).join(&right.type_of(subject));
            joined = joined.push(Step::Fact {
                subject: subject.clone(),
                ty: Box::new(ty),
            });
        }
        joined
    }

    /// Narrows `subject` with `narrow`, unless the subject is an unavailable
    /// global (narrowing never makes one readable) or the guard contradicts
    /// its incoming type.
    fn refine(&self, subject: &Subject, origin: &GuardOrigin, narrow: impl FnOnce(&Shape) -> Shape) -> Self {
        let current = self.type_of(subject);
        if matches!(
            current.declared(),
            DeclaredType::Host(crate::markdown::compose::expression::Availability::Unavailable(_))
        ) {
            return self.clone();
        }
        match current.refined(narrow(current.shape()), origin) {
            Some(ty) if ty != current => self.push(Step::Fact {
                subject: subject.clone(),
                ty: Box::new(ty),
            }),
            _ => self.clone(),
        }
    }

    fn push(&self, step: Step) -> Self {
        Self {
            declared: self.declared,
            head: Some(Arc::new(Link {
                step,
                parent: self.head.clone(),
            })),
            lexical: self.lexical.clone(),
        }
    }

    /// The newest link `self` shares with `other`.
    fn shared_ancestor(&self, other: &Self) -> Option<Arc<Link>> {
        let theirs = other.link_addresses();
        let mut link = self.head.clone();
        while let Some(current) = link {
            crate::note_work!(ScopeJoinAncestorVisit);
            if theirs.contains(&Arc::as_ptr(&current)) {
                return Some(current);
            }
            link = current.parent.clone();
        }
        None
    }

    /// `self`'s steps newer than anything shared with `other`, oldest first.
    fn steps_since_shared(&self, other: &Self) -> Vec<&Step> {
        let theirs = other.link_addresses();
        let mut steps = Vec::new();
        let mut link = self.head.as_deref();
        while let Some(current) = link {
            crate::note_work!(ScopeJoinAncestorVisit);
            if theirs.contains(&std::ptr::from_ref(current)) {
                break;
            }
            steps.push(&current.step);
            link = current.parent.as_deref();
        }
        steps.reverse();
        steps
    }

    fn link_addresses(&self) -> HashSet<*const Link> {
        let mut addresses = HashSet::new();
        let mut link = self.head.as_deref();
        while let Some(current) = link {
            crate::note_work!(ScopeJoinAncestorVisit);
            addresses.insert(std::ptr::from_ref(current));
            link = current.parent.as_deref();
        }
        addresses
    }
}

#[cfg(test)]
mod tests;
