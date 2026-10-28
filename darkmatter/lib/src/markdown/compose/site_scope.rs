//! The lexical scope each site of an expanded body reads from.
//!
//! A repeated body leaves the structural stage as authored text plus a
//! [`ScopeRange`](super::body_origin::ScopeRange) per copy. Later stages
//! scan that text once and evaluate many sites, so they cannot hold one
//! lookup: [`SiteLookup`] answers every read from the frame of the copy the
//! stage last [`enter`](SiteHook::enter)ed.

use std::cell::{Cell, OnceCell};

use serde_json::Value;

use super::body_origin::ScopeRanges;
use super::expression::iteration::{IterationBudget, IterationError};
use super::expression::{
    BindingView, EvaluationLookup, ExpressionError, LexicalScope, ResolutionContext, ResolvedBinding, ScopedLookup,
};

/// What a stage tells the sites of an expanded body about the position it is
/// about to evaluate.
pub(crate) trait SiteHook {
    /// The stage is about to evaluate the site at `offset` of the text it
    /// scans.
    fn enter(&self, offset: usize);

    /// The stage inserted `bytes` of produced text at `offset`. The insertion
    /// is charged once to every collection expansion that owns the copy.
    fn charge(&self, offset: usize, bytes: usize) -> Result<(), IterationError>;
}

/// An [`EvaluationLookup`] that reads locals from the frame of the repeated
/// body its current site lies in, and otherwise delegates to `inner`.
pub(crate) struct SiteLookup<'a> {
    inner: &'a dyn EvaluationLookup,
    ranges: &'a ScopeRanges,
    scoped: Vec<OnceCell<ScopedLookup<'a>>>,
    current: Cell<Option<usize>>,
    budget: Option<&'a IterationBudget>,
}

impl<'a> SiteLookup<'a> {
    /// `inner` read through the frames of `ranges`. Insertions are charged to
    /// `budget` when there is one.
    pub(crate) fn new(
        inner: &'a dyn EvaluationLookup,
        ranges: &'a ScopeRanges,
        budget: Option<&'a IterationBudget>,
    ) -> Self {
        Self {
            inner,
            ranges,
            scoped: (0..ranges.len()).map(|_| OnceCell::new()).collect(),
            current: Cell::new(None),
            budget,
        }
    }

    fn active(&self) -> &dyn EvaluationLookup {
        match self.current.get() {
            Some(index) => self.scoped[index].get_or_init(|| {
                let range = self.ranges.get(index).expect("an index of these ranges");
                ScopedLookup::new(self.inner, std::sync::Arc::clone(&range.scope))
            }),
            None => self.inner,
        }
    }
}

impl SiteHook for SiteLookup<'_> {
    fn enter(&self, offset: usize) {
        self.current.set(self.ranges.index_at(offset));
    }

    fn charge(&self, offset: usize, bytes: usize) -> Result<(), IterationError> {
        match self.budget {
            Some(budget) => self.ranges.charge(budget, offset, bytes),
            None => Ok(()),
        }
    }
}

impl EvaluationLookup for SiteLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.active().get(path)
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        self.active().resolve(path)
    }

    fn binding_view(&self) -> Option<&BindingView> {
        self.inner.binding_view()
    }

    fn format_resolved(&self, path: &str, value: &Value) -> String {
        self.active().format_resolved(path, value)
    }

    fn get_string(&self, path: &str) -> String {
        self.active().get_string(path)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.inner.resolution_context()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        self.inner.resolution_context_ref()
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.inner.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.inner.context_variable_names()
    }

    fn begin_expression_scope(&self) {
        self.inner.begin_expression_scope();
    }

    fn lexical_scope(&self) -> Option<&std::sync::Arc<LexicalScope>> {
        self.active().lexical_scope()
    }
}
