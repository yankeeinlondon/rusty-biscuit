//! The lexical scope each site of an expanded body reads from.
//!
//! A repeated body leaves the structural stage as authored text plus a
//! [`ScopeRange`](super::body_origin::ScopeRange) per copy. Later stages
//! scan that text once and evaluate many sites, so they cannot hold one
//! lookup: [`SiteLookup`] answers every read from the frame of the copy the
//! stage last [`enter`](SiteHook::enter)ed.

use std::cell::{Cell, OnceCell};
use std::ops::Range;

use serde_json::Value;

use super::body_origin::{BodyOrigin, ScopeRanges};
use super::collection_blocks::IterationFailure;
use crate::markdown::Markdown;
use crate::markdown::types::{AuthoredSpan, MarkdownError, SourceRef};
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
    entered: Cell<Option<usize>>,
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
            entered: Cell::new(None),
            budget,
        }
    }

    /// The offset the stage last [`enter`](SiteHook::enter)ed: the site a
    /// failure raised since then belongs to.
    pub(crate) fn entered(&self) -> Option<usize> {
        self.entered.get()
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
        self.entered.set(Some(offset));
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

    fn iteration_budget(&self) -> Option<&IterationBudget> {
        self.budget.or_else(|| self.inner.iteration_budget())
    }
}

/// `error`, raised by the site at `span` of a body, with the provenance a
/// report needs: the authored site in `markdown`'s loaded file and the
/// original items of the repeated copy (`ranges`) the site lies in.
///
/// - An exhaustion no stage has located yet is located at the site; it keeps
///   its own variant and the path its counter recorded.
/// - Any other failure inside a repeated copy is wrapped in
///   [`MarkdownError::InIteration`], which changes neither its message, its
///   code, nor whether a caller may recover from it.
/// - A failure outside every copy, and a policy stop, are returned unchanged.
///
/// `origin` maps the text `span` indexes back to the loaded document; the
/// caller proves it describes that text. Without a proven mapping the site is
/// the file alone.
pub(crate) fn scope_failure(
    error: MarkdownError,
    markdown: &Markdown,
    origin: Option<&BodyOrigin>,
    ranges: &ScopeRanges,
    span: Range<usize>,
) -> MarkdownError {
    let iteration = ranges.at(span.start).map(|range| range.iteration.clone()).unwrap_or_default();
    match error {
        MarkdownError::Collection(inner) if inner.is_unlocated_budget() => {
            match authored_location(markdown, origin, &span) {
                Some(location) => (*inner).with_source(location).into(),
                None => MarkdownError::Collection(inner),
            }
        }
        error @ MarkdownError::BlockedDiagnostics { .. } => error,
        error if iteration.is_empty() => error,
        error => MarkdownError::InIteration(Box::new(IterationFailure {
            location: authored_location(markdown, origin, &span).map(Box::new),
            iteration_path: iteration,
            error: Box::new(error),
        })),
    }
}

/// Where `span` of the current body sits in `markdown`'s loaded file: the
/// whole span when every byte of it is authored, otherwise its opener (the
/// directive marker no stage writes), otherwise the file alone.
fn authored_location(markdown: &Markdown, origin: Option<&BodyOrigin>, span: &Range<usize>) -> Option<SourceRef> {
    let context = markdown.loaded_source_context_for_errors()?;
    let located = origin.and_then(|origin| {
        let opener = |len: usize| span.start..(span.start + len).min(span.end.max(span.start + 1));
        [span.clone(), opener(2), opener(1)]
            .into_iter()
            .filter(|range| !range.is_empty())
            .find_map(|range| origin.project(&range))
            .and_then(|range| AuthoredSpan::locate(&context.content, range))
    });
    Some(match located {
        Some(span) => SourceRef::OnDiskSpan { context, span },
        None => SourceRef::OnDisk(context),
    })
}
