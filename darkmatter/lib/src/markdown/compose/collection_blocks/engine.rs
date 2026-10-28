//! Structural execution of `::map` / `::filter` regions.
//!
//! One traversal walks the body's page-block and collection regions together,
//! so an inner gate sees the bindings of every enclosing iteration and no
//! stage concatenates bodies to rediscover directives afterward. Both
//! directives are modes of one executor: the positional callback either
//! selects (`::filter`) or transforms (`::map`).
//!
//! Per invocation, in order: the block gate, the source (once), then per
//! source item a budget visit, the selection predicates, an optional stable
//! sort over the selected originals, prefix selection, the map expression,
//! and finally the retained body. The traversal only copies authored text
//! (`ExpansionWriter`), recording the lexical frame each copy was prepared
//! for; `{{ }}` interpolation, shell stages, and transclusion read those
//! frames at each site afterward.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::{Number, Value};

use super::error::{CollectionError, CollectionSite};
use super::parser::site;
use super::types::{CollectionKind, CollectionRegion, QuotedExpression, SortOrder};
use crate::markdown::compose::body_origin::{ExpansionWriter, ScopeRanges, Splice};
use crate::markdown::compose::context::report::CandidateLocus;
use crate::markdown::compose::expression::iteration::{
    IterationBudget, IterationError, IterationFrame, IterationPath,
};
use crate::markdown::compose::expression::{
    BindingPattern, Callback, EvaluationLookup, ExpressionError, Expr, LexicalScope, ScopedLookup, check_local_names,
    evaluate, is_truthy, parse_callback,
};
use crate::markdown::compose::expression::iterable::{IterableSource, SourceEntry};
use crate::markdown::compose::page_blocks::PageBlockRegion;
use crate::markdown::compose::value_origin::OverrideOrigin;
use crate::markdown::compose::{ComposeReport, conditions};
use crate::markdown::types::MarkdownError;
use biscuit_terminal::errors::SourceContext;

/// One region of the unified structural tree.
#[derive(Debug, Clone, Copy)]
enum Region<'a> {
    Page(&'a PageBlockRegion),
    Collection(&'a CollectionRegion),
}

impl Region<'_> {
    fn span(&self) -> &Range<usize> {
        match self {
            Region::Page(region) => &region.span,
            Region::Collection(region) => &region.span,
        }
    }
}

/// A region with the regions nested in its body, page blocks and collections
/// interleaved in document order.
#[derive(Debug)]
pub(crate) struct Node<'a> {
    region: Region<'a>,
    children: Vec<Node<'a>>,
}

/// The unified tree of `pages` and `collections`.
///
/// Both inputs are already nested among their own kind; this interleaves
/// them by span containment.
pub(crate) fn structure<'a>(pages: &'a [PageBlockRegion], collections: &'a [CollectionRegion]) -> Vec<Node<'a>> {
    fn flatten_pages<'a>(regions: &'a [PageBlockRegion], flat: &mut Vec<Region<'a>>) {
        for region in regions {
            flat.push(Region::Page(region));
            flatten_pages(&region.children, flat);
        }
    }
    fn flatten_collections<'a>(regions: &'a [CollectionRegion], flat: &mut Vec<Region<'a>>) {
        for region in regions {
            flat.push(Region::Collection(region));
            flatten_collections(&region.children, flat);
        }
    }
    let mut flat = Vec::new();
    flatten_pages(pages, &mut flat);
    flatten_collections(collections, &mut flat);
    flat.sort_by_key(|region| region.span().start);

    let mut roots: Vec<Node<'a>> = Vec::new();
    let mut open: Vec<Node<'a>> = Vec::new();
    let close = |open: &mut Vec<Node<'a>>, roots: &mut Vec<Node<'a>>| {
        let done = open.pop().expect("a node is open");
        match open.last_mut() {
            Some(parent) => parent.children.push(done),
            None => roots.push(done),
        }
    };
    for region in flat {
        while open.last().is_some_and(|top| top.region.span().end <= region.span().start) {
            close(&mut open, &mut roots);
        }
        open.push(Node { region, children: Vec::new() });
    }
    while !open.is_empty() {
        close(&mut open, &mut roots);
    }
    roots
}

/// Why a traversal ended before it finished.
#[derive(Debug)]
pub(crate) enum Halt {
    /// The `stop` check answered `true`: a blocking warning ends the document
    /// before the next expression runs.
    Blocked,
    /// A failure; no partial result is produced.
    Failed(Box<MarkdownError>),
}

impl<E: Into<MarkdownError>> From<E> for Halt {
    fn from(error: E) -> Self {
        Halt::Failed(Box::new(error.into()))
    }
}

fn failed(error: CollectionError) -> Halt {
    Halt::Failed(Box::new(error.into()))
}

fn budget(error: IterationError) -> Halt {
    failed(CollectionError::Budget(Box::new(error)))
}

/// A sort key: all numbers or all strings.
#[derive(Debug, Clone, PartialEq)]
enum SortKey {
    Number(Number),
    Text(String),
}

impl SortKey {
    fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (SortKey::Number(left), SortKey::Number(right)) => compare_numbers(left, right),
            (SortKey::Text(left), SortKey::Text(right)) => left.cmp(right),
            // Mixed kinds are rejected when the key is computed.
            (SortKey::Number(_), SortKey::Text(_)) => Ordering::Less,
            (SortKey::Text(_), SortKey::Number(_)) => Ordering::Greater,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            SortKey::Number(_) => "number",
            SortKey::Text(_) => "string",
        }
    }
}

fn compare_numbers(left: &Number, right: &Number) -> Ordering {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left.cmp(&right);
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return left.cmp(&right);
    }
    let (left, right) = (left.as_f64().unwrap_or(0.0), right.as_f64().unwrap_or(0.0));
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefix {
    While,
    Until,
}

/// A callback with the authored value it came from, for located failures.
struct Located {
    callback: Callback,
    span: Range<usize>,
}

/// A region's callbacks, parsed once.
struct Plan {
    positional: Option<Located>,
    filter: Option<Located>,
    sort: Option<Located>,
    sort_order: SortOrder,
    prefix: Option<(Prefix, Located)>,
    body_pattern: BindingPattern,
}

/// What the traversal reads and reports to.
pub(crate) struct Execution<'a, L: EvaluationLookup> {
    /// The body being expanded.
    pub content: &'a str,
    /// The document lookup every frame layers over.
    pub lookup: &'a L,
    /// The request's counters.
    pub budget: &'a IterationBudget,
    /// The document's error context for conditions.
    pub context: SourceContext,
    /// The report warnings and counters go to.
    pub report: &'a mut ComposeReport,
    /// Asked before each gate; `true` ends the traversal with
    /// [`Halt::Blocked`].
    pub stop: &'a mut dyn FnMut(&ComposeReport) -> bool,
    /// Plans by region start, so a region invoked once per enclosing
    /// iteration parses its callbacks once.
    plans: HashMap<usize, Rc<Plan>>,
    /// Whether this traversal only discovers the copies a body could have
    /// (see [`Self::new`]).
    discover: bool,
}

/// The expansion of a body.
pub(crate) struct Expansion {
    pub output: String,
    pub splices: Vec<Splice>,
    pub scopes: ScopeRanges,
}

impl<'a, L: EvaluationLookup> Execution<'a, L> {
    /// A traversal of `content`.
    ///
    /// With a [discovery](IterationBudget::discovery) budget the traversal
    /// is condition-blind and selection-blind, for pre-flight: every gate is
    /// taken, every source item gets a copy of the body in registration
    /// order, and no filter, sort, or prefix callback runs. Only the map
    /// expression runs, because a body's targets may read what it produces;
    /// a source, map, or pattern that cannot be read there leaves its names
    /// `null` instead of failing, since execution reports the failure if it
    /// reaches the item.
    pub(crate) fn new(
        content: &'a str,
        lookup: &'a L,
        budget: &'a IterationBudget,
        context: SourceContext,
        report: &'a mut ComposeReport,
        stop: &'a mut dyn FnMut(&ComposeReport) -> bool,
    ) -> Self {
        Self { content, lookup, budget, context, report, stop, plans: HashMap::new(), discover: !budget.is_charging() }
    }

    /// Expands the whole body: `nodes` are its top-level regions.
    ///
    /// ## Errors
    ///
    /// [`Halt::Blocked`] when `stop` ended it; [`Halt::Failed`] for the first
    /// failure.
    pub(crate) fn expand(&mut self, nodes: &[Node<'_>]) -> Result<Expansion, Halt> {
        let mut writer = ExpansionWriter::new(self.content.len(), self.budget);
        let path = IterationPath::empty();
        self.render(&mut writer, 0..self.content.len(), nodes, None, &path)?;
        let (output, splices, scopes) = writer.finish();
        Ok(Expansion { output, splices, scopes })
    }

    fn render(
        &mut self,
        writer: &mut ExpansionWriter<'_>,
        range: Range<usize>,
        nodes: &[Node<'_>],
        scope: Option<&Arc<LexicalScope>>,
        path: &IterationPath,
    ) -> Result<(), Halt> {
        let mut cursor = range.start;
        for node in nodes {
            writer.copy(self.content, cursor..node.region.span().start, path).map_err(budget)?;
            match node.region {
                Region::Page(region) => self.page(writer, region, node, scope, path)?,
                Region::Collection(region) => self.collection(writer, region, node, scope, path)?,
            }
            cursor = node.region.span().end;
        }
        writer.copy(self.content, cursor..range.end, path).map_err(budget)?;
        Ok(())
    }

    fn page(
        &mut self,
        writer: &mut ExpansionWriter<'_>,
        region: &PageBlockRegion,
        node: &Node<'_>,
        scope: Option<&Arc<LexicalScope>>,
        path: &IterationPath,
    ) -> Result<(), Halt> {
        let enabled = match &region.options.when_expr {
            Some(_) if self.discover => true,
            Some(expression) => self.condition(expression, region.start_line, scope)?,
            None => true,
        };
        if enabled {
            self.render(writer, region.body_span.clone(), &node.children, scope, path)?;
            self.report.page_blocks_rendered += 1;
        } else {
            self.report.page_blocks_skipped += 1;
        }
        Ok(())
    }

    /// Evaluates a `when` condition in `scope`: its context warnings are
    /// reported and `stop` is asked before it runs, as page blocks do.
    fn condition(&mut self, expression: &str, line: usize, scope: Option<&Arc<LexicalScope>>) -> Result<bool, Halt> {
        let lookup = self.lookup;
        let warnings = match scope {
            Some(scope) => conditions::collect_condition_context_warnings(
                expression,
                &ScopedLookup::new(lookup, Arc::clone(scope)),
                "condition",
            ),
            None => conditions::collect_condition_context_warnings(expression, lookup, "condition"),
        };
        for warning in warnings {
            self.report.add_warning(warning.at_line(line));
        }
        if (self.stop)(self.report) {
            return Err(Halt::Blocked);
        }
        let mut missing = Vec::new();
        let context = self.context.clone();
        let verdict = match scope {
            Some(scope) => conditions::evaluate_condition_observed(
                expression,
                &ScopedLookup::new(lookup, Arc::clone(scope)),
                line,
                context,
                &mut missing,
            ),
            None => conditions::evaluate_condition_observed(expression, lookup, line, context, &mut missing),
        }
        .map_err(|error| Halt::from(crate::markdown::compose::page_blocks::PageBlockError::from(error)))?;
        self.report.add_unknown_root_candidates("condition", missing, |_| CandidateLocus::BodyLine(line));
        Ok(verdict)
    }

    fn plan(&mut self, region: &CollectionRegion) -> Result<Rc<Plan>, Halt> {
        if let Some(plan) = self.plans.get(&region.span.start) {
            return Ok(Rc::clone(plan));
        }
        let header = &region.header;
        let parse = |quoted: &QuotedExpression| -> Result<Located, Halt> {
            let callback = parse_callback(&quoted.text).map_err(|error| {
                failed(CollectionError::InvalidCallback {
                    site: self.site(quoted.span.clone(), &IterationPath::empty()),
                    message: error.to_string(),
                })
            })?;
            Ok(Located { callback, span: quoted.span.clone() })
        };
        let optional = |quoted: &Option<QuotedExpression>| quoted.as_ref().map(&parse).transpose();
        let positional = optional(&header.callback)?;
        let prefix = match (&header.while_predicate, &header.until_predicate) {
            (Some(quoted), _) => Some((Prefix::While, parse(quoted)?)),
            (None, Some(quoted)) => Some((Prefix::Until, parse(quoted)?)),
            (None, None) => None,
        };
        let body_pattern = match &positional {
            Some(positional) => positional.callback.pattern.clone(),
            None => BindingPattern::Identifier("i".to_string()),
        };
        let plan = Rc::new(Plan {
            positional,
            filter: optional(&header.filter)?,
            sort: optional(&header.sort)?,
            sort_order: header.sort_order.as_ref().map_or(SortOrder::Asc, |order| order.value),
            prefix,
            body_pattern,
        });
        self.plans.insert(region.span.start, Rc::clone(&plan));
        Ok(plan)
    }

    fn site(&self, span: Range<usize>, path: &IterationPath) -> CollectionSite {
        let mut located = site(self.content, Some(std::path::Path::new("document")), span);
        located.iteration_path = path.clone();
        located
    }

    fn collection(
        &mut self,
        writer: &mut ExpansionWriter<'_>,
        region: &CollectionRegion,
        node: &Node<'_>,
        scope: Option<&Arc<LexicalScope>>,
        path: &IterationPath,
    ) -> Result<(), Halt> {
        let header = &region.header;
        let plan = self.plan(region)?;
        self.check_names(&plan, path)?;

        if !self.discover
            && let Some(when) = &header.when
            && !self.condition(&when.text, region.start_line, scope)?
        {
            return Ok(());
        }

        let reference = header.source.value.as_str();
        let value = match self.evaluate(scope, &Expr::Variable(reference.to_string())) {
            Ok(value) => value,
            Err(_) if self.discover => return Ok(()),
            Err(cause) => return Err(self.callback_failure(header.source.span.clone(), path, cause)),
        };
        let source = match IterableSource::capture(value) {
            Ok(source) => source,
            Err(_) if self.discover => return Ok(()),
            Err(error) => {
                return Err(failed(CollectionError::InvalidSource {
                    site: self.site(header.source.span.clone(), path),
                    kind: region.kind,
                    reference: reference.to_string(),
                    message: format!("`{reference}`: {error}"),
                }));
            }
        };

        writer.open_expansion();
        let outcome = self.iterate(writer, region, node, &plan, &source, scope, path);
        writer.close_expansion();
        outcome
    }

    /// Runs every pattern name past the protected-name check, as the passive
    /// analysis did, so a document that skipped analysis cannot shadow a
    /// reserved root.
    fn check_names(&self, plan: &Plan, path: &IterationPath) -> Result<(), Halt> {
        let callbacks = plan
            .positional
            .iter()
            .chain(plan.filter.iter())
            .chain(plan.sort.iter())
            .chain(plan.prefix.iter().map(|(_, located)| located));
        for located in callbacks {
            if let Err(protected) = check_local_names(located.callback.pattern.names(), self.lookup.binding_view()) {
                return Err(failed(CollectionError::InvalidCallback {
                    site: self.site(located.span.clone(), path),
                    message: protected.to_string(),
                }));
            }
        }
        Ok(())
    }

    fn evaluate(&self, scope: Option<&Arc<LexicalScope>>, expression: &Expr) -> Result<Value, ExpressionError> {
        match scope {
            Some(scope) => evaluate(expression, &ScopedLookup::new(self.lookup, Arc::clone(scope))),
            None => evaluate(expression, self.lookup),
        }
    }

    fn callback_failure(&self, span: Range<usize>, path: &IterationPath, cause: ExpressionError) -> Halt {
        match cause {
            ExpressionError::IterationLimit(error) => budget(*error),
            cause => failed(CollectionError::CallbackFailed {
                site: self.site(span, path),
                cause: Box::new(cause),
            }),
        }
    }

    /// Binds `pattern` to the entry's item in a new frame over `parent`.
    fn bind_entry(
        &self,
        parent: Option<&Arc<LexicalScope>>,
        pattern: &BindingPattern,
        entry: &SourceEntry<'_>,
        span: &Range<usize>,
        path: &IterationPath,
    ) -> Result<Arc<LexicalScope>, Halt> {
        let bindings = match pattern {
            BindingPattern::Identifier(name) => vec![(name.clone(), entry.retained(), OverrideOrigin::Data)],
            BindingPattern::Fields(fields) => fields
                .iter()
                .map(|name| {
                    entry
                        .field(name)
                        .map(|value| (name.clone(), value.into_owned(), OverrideOrigin::Data))
                        .ok_or_else(|| self.missing_field(name, span, path))
                })
                .collect::<Result<_, _>>()?,
        };
        Ok(frame(parent, bindings))
    }

    /// Binds `pattern` to a computed value (a map result).
    fn bind_value(
        &self,
        parent: Option<&Arc<LexicalScope>>,
        pattern: &BindingPattern,
        value: Value,
        span: &Range<usize>,
        path: &IterationPath,
    ) -> Result<Arc<LexicalScope>, Halt> {
        let bindings = match pattern {
            BindingPattern::Identifier(name) => vec![(name.clone(), value, OverrideOrigin::Data)],
            BindingPattern::Fields(fields) => {
                let Some(object) = value.as_object() else {
                    return Err(failed(CollectionError::PatternMismatch {
                        site: self.site(span.clone(), path),
                        message: format!(
                            "the mapped result is {}, which cannot satisfy the pattern `{{{}}}`",
                            describe(&value),
                            fields.join(", ")
                        ),
                    }));
                };
                fields
                    .iter()
                    .map(|name| {
                        object
                            .get(name)
                            .map(|found| (name.clone(), found.clone(), OverrideOrigin::Data))
                            .ok_or_else(|| self.missing_field(name, span, path))
                    })
                    .collect::<Result<_, _>>()?
            }
        };
        Ok(frame(parent, bindings))
    }

    fn missing_field(&self, name: &str, span: &Range<usize>, path: &IterationPath) -> Halt {
        failed(CollectionError::PatternMismatch {
            site: self.site(span.clone(), path),
            message: format!("the item has no field `{name}`"),
        })
    }

    /// Evaluates `located` over `entry` in its own parameter frame.
    fn call(
        &self,
        located: &Located,
        parent: Option<&Arc<LexicalScope>>,
        entry: &SourceEntry<'_>,
        path: &IterationPath,
    ) -> Result<Value, Halt> {
        let frame = self.bind_entry(parent, &located.callback.pattern, entry, &located.span, path)?;
        evaluate(&located.callback.body, &ScopedLookup::new(self.lookup, frame))
            .map_err(|cause| self.callback_failure(located.span.clone(), path, cause))
    }

    fn sort_key(
        &self,
        located: &Located,
        parent: Option<&Arc<LexicalScope>>,
        entry: &SourceEntry<'_>,
        first: Option<&SortKey>,
        path: &IterationPath,
    ) -> Result<SortKey, Halt> {
        let value = self.call(located, parent, entry, path)?;
        let key = match &value {
            Value::Number(number) => SortKey::Number(number.clone()),
            Value::String(text) => SortKey::Text(text.clone()),
            other => {
                return Err(failed(CollectionError::InvalidSortKey {
                    site: self.site(located.span.clone(), path),
                    message: format!("the sort key is {}; a key must be a number or a string", describe(other)),
                }));
            }
        };
        if let Some(first) = first
            && first.kind() != key.kind()
        {
            return Err(failed(CollectionError::InvalidSortKey {
                site: self.site(located.span.clone(), path),
                message: format!(
                    "the sort key is a {} but earlier keys are {}s; keys must all be numbers or all be strings",
                    key.kind(),
                    first.kind()
                ),
            }));
        }
        Ok(key)
    }

    #[allow(clippy::too_many_arguments)]
    fn iterate(
        &mut self,
        writer: &mut ExpansionWriter<'_>,
        region: &CollectionRegion,
        node: &Node<'_>,
        plan: &Plan,
        source: &IterableSource,
        scope: Option<&Arc<LexicalScope>>,
        path: &IterationPath,
    ) -> Result<(), Halt> {
        let collection = region.header.source.value.as_str();
        let item_path = |entry: &SourceEntry<'_>| -> IterationPath {
            path.pushed(IterationFrame { collection: collection.to_string(), position: entry.position() })
        };
        // The selecting callbacks, positional first for a filter.
        let selectors = || {
            let positional = (region.kind == CollectionKind::Filter).then_some(plan.positional.as_ref()).flatten();
            positional.into_iter().chain(plan.filter.as_ref())
        };

        let stops = |plan: &Plan, this: &Self, entry: &SourceEntry<'_>, item: &IterationPath| -> Result<bool, Halt> {
            let Some((kind, located)) = &plan.prefix else {
                return Ok(false);
            };
            let truthy = is_truthy(&this.call(located, scope, entry, item)?);
            Ok(match kind {
                Prefix::While => !truthy,
                Prefix::Until => truthy,
            })
        };

        if self.discover {
            for entry in source.entries() {
                let item = item_path(&entry);
                self.emit(writer, region, node, plan, scope, &entry, &item)?;
            }
            return Ok(());
        }

        match &plan.sort {
            None => {
                for entry in source.entries() {
                    let item = item_path(&entry);
                    self.budget.reserve_visit(&item).map_err(budget)?;
                    if !self.selected(selectors(), scope, &entry, &item)? {
                        continue;
                    }
                    if stops(plan, self, &entry, &item)? {
                        break;
                    }
                    self.emit(writer, region, node, plan, scope, &entry, &item)?;
                }
            }
            Some(sort) => {
                let mut kept: Vec<(SourceEntry<'_>, SortKey)> = Vec::new();
                for entry in source.entries() {
                    let item = item_path(&entry);
                    self.budget.reserve_visit(&item).map_err(budget)?;
                    if !self.selected(selectors(), scope, &entry, &item)? {
                        continue;
                    }
                    let key = self.sort_key(sort, scope, &entry, kept.first().map(|(_, key)| key), &item)?;
                    kept.push((entry, key));
                }
                // `sort_by` is stable, so descending order keeps equal keys
                // in their previous order instead of reversing them.
                match plan.sort_order {
                    SortOrder::Asc => kept.sort_by(|left, right| left.1.compare(&right.1)),
                    SortOrder::Desc => kept.sort_by(|left, right| right.1.compare(&left.1)),
                }
                for (entry, _) in &kept {
                    let item = item_path(entry);
                    if stops(plan, self, entry, &item)? {
                        break;
                    }
                    self.emit(writer, region, node, plan, scope, entry, &item)?;
                }
            }
        }
        Ok(())
    }

    /// Whether every selector keeps `entry`, stopping at the first that does
    /// not.
    fn selected<'s>(
        &self,
        selectors: impl Iterator<Item = &'s Located>,
        scope: Option<&Arc<LexicalScope>>,
        entry: &SourceEntry<'_>,
        path: &IterationPath,
    ) -> Result<bool, Halt> {
        for selector in selectors {
            if !is_truthy(&self.call(selector, scope, entry, path)?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Prepares the body for one retained item.
    #[allow(clippy::too_many_arguments)]
    fn emit(
        &mut self,
        writer: &mut ExpansionWriter<'_>,
        region: &CollectionRegion,
        node: &Node<'_>,
        plan: &Plan,
        scope: Option<&Arc<LexicalScope>>,
        entry: &SourceEntry<'_>,
        item: &IterationPath,
    ) -> Result<(), Halt> {
        let (pattern_span, positional) = match &plan.positional {
            Some(located) => (located.span.clone(), Some(located)),
            None => (region.header_span.clone(), None),
        };
        let bound = match (region.kind, positional) {
            (CollectionKind::Map, Some(mapper)) => self
                .call(mapper, scope, entry, item)
                .and_then(|mapped| self.bind_value(scope, &plan.body_pattern, mapped, &pattern_span, item)),
            _ => self.bind_entry(scope, &plan.body_pattern, entry, &pattern_span, item),
        };
        let body = match bound {
            Ok(body) => body,
            Err(Halt::Failed(_)) if self.discover => {
                frame(scope, plan.body_pattern.names().map(|name| (name.to_string(), Value::Null, OverrideOrigin::Data)).collect())
            }
            Err(halt) => return Err(halt),
        };
        writer.open_iteration(Arc::clone(&body), item.clone());
        let rendered = self.render(writer, region.body_span.clone(), &node.children, Some(&body), item);
        writer.close_iteration();
        rendered
    }
}

fn frame(parent: Option<&Arc<LexicalScope>>, bindings: Vec<(String, Value, OverrideOrigin)>) -> Arc<LexicalScope> {
    match parent {
        Some(parent) => parent.child_with_origins(bindings),
        None => LexicalScope::with_origins(bindings),
    }
}

fn describe(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_compare_exactly_for_integers_and_numerically_otherwise() {
        let number = |text: &str| serde_json::from_str::<Number>(text).unwrap();
        assert_eq!(compare_numbers(&number("9007199254740993"), &number("9007199254740992")), Ordering::Greater);
        assert_eq!(compare_numbers(&number("-1"), &number("0.5")), Ordering::Less);
        assert_eq!(compare_numbers(&number("2"), &number("2.0")), Ordering::Equal);
    }

    #[test]
    fn sort_keys_of_different_kinds_name_their_kind() {
        assert_eq!(SortKey::Text("a".into()).kind(), "string");
        assert_eq!(SortKey::Number(Number::from(1)).kind(), "number");
    }
}
