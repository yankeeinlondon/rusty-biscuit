//! The scope inventory of one body.
//!
//! [`BodyScopes`] answers "what do I know at this byte offset?" for every
//! consumer: the composition call check, directive-target analysis, and
//! editors. It holds the page-block guards ([`BodyGuards`], its conditional
//! view) and one plan per `::map` / `::filter` region, and derives each
//! scope on demand from the enclosing site scope, so a caller keeps owning
//! the declarations the scope borrows.
//!
//! A region contributes, in order and outermost first:
//!
//! 1. its `when` gate, assumed true (the gate resolves in the enclosing scope);
//! 2. the source's entry type, projected on the true side of that gate;
//! 3. one independent parameter frame per option callback, each typed over the
//!    selection facts the callbacks before it establish, never over another
//!    callback's parameter names;
//! 4. the body frame: the retained original item (`::filter`) or the mapped
//!    result as a new subject (`::map`), bound through the positional pattern
//!    or the default `i`.
//!
//! Nothing here evaluates an expression or reads a file. A defect that makes
//! a scope unreliable (an untyped source, a mismatched pattern) is reported by
//! [`analyze_collection_blocks`](super::analyze_collection_blocks); the scope
//! itself degrades to unknown local types so every later check stays
//! conservative.

use std::ops::Range;
use std::sync::Arc;

use super::parser::scan_collection_regions_lenient;
use super::types::{CollectionHeader, CollectionKind, CollectionRegion, QuotedExpression};
use crate::markdown::compose::expression::{
    BindingPattern, Callback, Expr, GuardOrigin, ItemFrame, LexicalTypes, PatternMismatch, Polarity, Shape,
    SortKeySupport, SourceProjection, SpannedCallback, SpannedExpr, TypeScope, TypedLocal, expression_type,
    parse_callback_spanned, parse_condition_spanned,
};
use crate::markdown::compose::page_blocks::{BodyGuard, BodyGuards};
use crate::markdown::compose::pipeline::operations::{ComposeOperation, ComposeOperationSet};

/// Which authored expression of a region a site is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionRole {
    /// `when="..."`, resolved in the enclosing scope.
    Gate,
    /// The positional callback: the `::map` mapper or the `::filter` predicate.
    Positional,
    /// `filter="..."`.
    Filter,
    /// `sort="..."`.
    Sort,
    /// `while="..."`.
    While,
    /// `until="..."`.
    Until,
}

impl ExpressionRole {
    /// How the author names it: the option, or `callback` for the positional.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Gate => "when",
            Self::Positional => "callback",
            Self::Filter => "filter",
            Self::Sort => "sort",
            Self::While => "while",
            Self::Until => "until",
        }
    }
}

/// One authored expression of a region with the scope it is evaluated in.
#[derive(Debug, Clone)]
pub struct ExpressionSite<'d> {
    /// Which expression.
    pub role: ExpressionRole,
    /// The span of the region it belongs to, opener through closer.
    pub region: Range<usize>,
    /// The decoded text the expression was parsed from.
    pub text: String,
    /// Authored byte offset of `text[0]` in the scanned text. Adding a decoded
    /// offset to it is exact only when the value contains no escape; use
    /// [`ExpressionSite::authored_range`] to project a span.
    pub origin: usize,
    /// The quoted value `text` was decoded from, which maps decoded offsets
    /// back across its escapes.
    pub(crate) quoted: QuotedExpression,
    /// The condition, or the callback's body. Its spans index `text`.
    pub expression: SpannedExpr,
    /// The scope the expression is evaluated in. For a callback it holds the
    /// callback's own parameters.
    pub scope: TypeScope<'d>,
}

impl ExpressionSite<'_> {
    /// The authored span of a decoded span of [`text`](Self::text), accurate
    /// across escaped quotes and multibyte text.
    pub fn authored_range(&self, decoded: Range<usize>) -> Range<usize> {
        self.quoted
            .authored_range(decoded.clone())
            .unwrap_or(self.origin + decoded.start..self.origin + decoded.end)
    }
}

#[derive(Debug, Clone)]
pub(super) struct ParsedCallback {
    pub quoted: QuotedExpression,
    pub spanned: SpannedCallback,
    pub erased: Callback,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedGate {
    pub quoted: QuotedExpression,
    pub spanned: SpannedExpr,
    pub expr: Expr,
}

/// A region parsed once for scope derivation.
#[derive(Debug, Clone)]
pub(super) struct RegionPlan {
    pub kind: CollectionKind,
    pub span: Range<usize>,
    pub body_span: Range<usize>,
    pub start_line: usize,
    pub header: CollectionHeader,
    pub source: Expr,
    pub gate: Option<ParsedGate>,
    pub positional: Option<ParsedCallback>,
    pub filter: Option<ParsedCallback>,
    pub sort: Option<ParsedCallback>,
    pub prefix: Option<(ExpressionRole, ParsedCallback)>,
}

/// A defect found while typing a region.
#[derive(Debug, Clone)]
pub(super) enum Problem {
    /// The source has no iterable declaration.
    UntypedSource,
    /// The source has a non-null alternative that is never iterable.
    InvalidSource(Vec<String>),
    /// A callback's pattern cannot bind the entry.
    Pattern(ExpressionRole, PatternMismatch),
    /// The mapped result cannot satisfy the body pattern.
    Result(PatternMismatch),
    /// The sort key is provably not a number or a string.
    SortKey(String),
}

pub(super) struct Evaluation<'d> {
    pub callbacks: Vec<(ExpressionRole, TypeScope<'d>)>,
    pub body: TypeScope<'d>,
    pub problems: Vec<Problem>,
}

fn parse_callback(quoted: &QuotedExpression) -> Option<ParsedCallback> {
    let spanned = parse_callback_spanned(&quoted.text).ok()?;
    Some(ParsedCallback { quoted: quoted.clone(), erased: spanned.erase(), spanned })
}

fn pattern_names(pattern: &BindingPattern) -> Vec<&str> {
    match pattern {
        BindingPattern::Identifier(name) => vec![name.as_str()],
        BindingPattern::Fields(fields) => fields.iter().map(String::as_str).collect(),
    }
}

/// `pattern` bound to unknown values: the conservative scope of a callback
/// whose pattern could not be checked.
fn unknown_binding<'d>(scope: &TypeScope<'d>, parent: Option<&Arc<LexicalTypes>>, pattern: &BindingPattern) -> TypeScope<'d> {
    let locals = pattern_names(pattern)
        .into_iter()
        .map(|name| TypedLocal::new(name, Shape::Unknown { non_null: false }))
        .collect();
    let layer = match parent {
        Some(parent) => parent.child(locals),
        None => LexicalTypes::new(locals),
    };
    scope.within(layer)
}

fn bind<'d>(
    frame: &ItemFrame,
    scope: &TypeScope<'d>,
    parent: Option<&Arc<LexicalTypes>>,
    pattern: &BindingPattern,
) -> (TypeScope<'d>, Option<PatternMismatch>) {
    match frame.enter(scope, parent, pattern) {
        Ok(entered) => (entered, None),
        Err(mismatch) => (unknown_binding(scope, parent, pattern), Some(mismatch)),
    }
}

impl RegionPlan {
    fn new(region: &CollectionRegion) -> Option<Self> {
        let header = &region.header;
        let gate = match &header.when {
            Some(quoted) => {
                let spanned = parse_condition_spanned(&quoted.text).ok()?;
                Some(ParsedGate { quoted: quoted.clone(), expr: spanned.erase(), spanned })
            }
            None => None,
        };
        let optional = |quoted: &Option<QuotedExpression>| -> Option<Option<ParsedCallback>> {
            match quoted {
                Some(quoted) => parse_callback(quoted).map(Some),
                None => Some(None),
            }
        };
        let prefix = match (&header.while_predicate, &header.until_predicate) {
            (Some(quoted), _) => Some((ExpressionRole::While, parse_callback(quoted)?)),
            (None, Some(quoted)) => Some((ExpressionRole::Until, parse_callback(quoted)?)),
            (None, None) => None,
        };
        Some(Self {
            kind: region.kind,
            span: region.span.clone(),
            body_span: region.body_span.clone(),
            start_line: region.start_line,
            header: header.clone(),
            source: Expr::Variable(header.source.value.clone()),
            gate,
            positional: optional(&header.callback)?,
            filter: optional(&header.filter)?,
            sort: optional(&header.sort)?,
            prefix,
        })
    }

    fn body_pattern(&self) -> BindingPattern {
        match &self.positional {
            Some(positional) => positional.erased.pattern.clone(),
            None => BindingPattern::Identifier("i".to_string()),
        }
    }

    /// Every authored callback in evaluation-independent header order.
    pub(super) fn callbacks(&self) -> impl Iterator<Item = (ExpressionRole, &ParsedCallback)> {
        self.positional
            .iter()
            .map(|callback| (ExpressionRole::Positional, callback))
            .chain(self.filter.iter().map(|callback| (ExpressionRole::Filter, callback)))
            .chain(self.sort.iter().map(|callback| (ExpressionRole::Sort, callback)))
            .chain(self.prefix.iter().map(|(role, callback)| (*role, callback)))
    }

    /// Types this region in the enclosing scope `outer`.
    pub(super) fn evaluate<'d>(&self, outer: &TypeScope<'d>) -> Evaluation<'d> {
        let mut problems = Vec::new();
        let gate = self.gate.as_ref().map(|gate| &gate.expr);
        let gated = match &self.gate {
            Some(gate) => outer.assume_at(
                &gate.expr,
                Polarity::True,
                GuardOrigin::new(format!(
                    "::{} when=\"{}\" (line {})",
                    self.kind.opener(),
                    gate.quoted.text,
                    self.start_line
                )),
            ),
            None => outer.clone(),
        };

        let entry = match outer.project_source(gate, &self.source) {
            SourceProjection::Entries(shape) => shape,
            SourceProjection::UnknownShape => {
                problems.push(Problem::UntypedSource);
                SourceProjection::recovered_entries()
            }
            SourceProjection::Unsuitable(arms) => {
                problems.push(Problem::InvalidSource(arms));
                SourceProjection::recovered_entries()
            }
        };

        let body_pattern = self.body_pattern();
        let mut patterns: Vec<&BindingPattern> = self.callbacks().map(|(_, callback)| &callback.erased.pattern).collect();
        patterns.push(&body_pattern);
        let frame = ItemFrame::new(entry, &patterns);

        let parent = gated.lexical().cloned();
        let mut accumulated = gated.clone();
        let mut callbacks: Vec<(ExpressionRole, TypeScope<'d>)> = Vec::new();

        // Types one selecting predicate over the facts so far, then keeps the
        // facts it proves on the retained side and drops its parameters.
        let select = |role: ExpressionRole,
                          callback: &ParsedCallback,
                          polarity: Polarity,
                          accumulated: &mut TypeScope<'d>,
                          callbacks: &mut Vec<(ExpressionRole, TypeScope<'d>)>,
                          problems: &mut Vec<Problem>| {
            let (evaluating, mismatch) = bind(&frame, accumulated, parent.as_ref(), &callback.erased.pattern);
            if let Some(mismatch) = mismatch {
                problems.push(Problem::Pattern(role, mismatch));
            }
            callbacks.push((role, evaluating.clone()));
            *accumulated = evaluating.assume(&callback.erased.body, polarity).with_lexical(parent.clone());
        };

        if self.kind == CollectionKind::Filter
            && let Some(positional) = &self.positional
        {
            select(ExpressionRole::Positional, positional, Polarity::True, &mut accumulated, &mut callbacks, &mut problems);
        }
        if let Some(filter) = &self.filter {
            select(ExpressionRole::Filter, filter, Polarity::True, &mut accumulated, &mut callbacks, &mut problems);
        }

        // The sort key reads the selected originals; it retains nothing.
        if let Some(sort) = &self.sort {
            let (evaluating, mismatch) = bind(&frame, &accumulated, parent.as_ref(), &sort.erased.pattern);
            match mismatch {
                Some(mismatch) => problems.push(Problem::Pattern(ExpressionRole::Sort, mismatch)),
                None => {
                    let key = expression_type(&sort.spanned.body, &evaluating);
                    if let SortKeySupport::Unsupported(ty) = key.sort_key_support(evaluating.definitions()) {
                        problems.push(Problem::SortKey(ty));
                    }
                }
            }
            callbacks.push((ExpressionRole::Sort, evaluating));
        }

        if let Some((role, prefix)) = &self.prefix {
            let polarity = if *role == ExpressionRole::While { Polarity::True } else { Polarity::False };
            select(*role, prefix, polarity, &mut accumulated, &mut callbacks, &mut problems);
        }

        let body = match self.kind {
            CollectionKind::Filter => {
                let (body, mismatch) = bind(&frame, &accumulated, parent.as_ref(), &body_pattern);
                // A positional mismatch is already reported by its predicate.
                if let Some(mismatch) = mismatch
                    && self.positional.is_none()
                {
                    problems.push(Problem::Pattern(ExpressionRole::Positional, mismatch));
                }
                body
            }
            CollectionKind::Map => self.map_body(&frame, &accumulated, parent.as_ref(), &body_pattern, &mut callbacks, &mut problems),
        };

        Evaluation { callbacks, body, problems }
    }

    fn map_body<'d>(
        &self,
        frame: &ItemFrame,
        accumulated: &TypeScope<'d>,
        parent: Option<&Arc<LexicalTypes>>,
        body_pattern: &BindingPattern,
        callbacks: &mut Vec<(ExpressionRole, TypeScope<'d>)>,
        problems: &mut Vec<Problem>,
    ) -> TypeScope<'d> {
        let default;
        let mapper = match &self.positional {
            Some(positional) => &positional.spanned,
            None => match parse_callback_spanned("i => i") {
                Ok(parsed) => {
                    default = parsed;
                    &default
                }
                Err(_) => return unknown_binding(accumulated, parent, body_pattern),
            },
        };
        let (evaluating, mismatch) = bind(frame, accumulated, parent, &mapper.pattern.erase());
        if self.positional.is_some() {
            callbacks.push((ExpressionRole::Positional, evaluating));
        }
        if let Some(mismatch) = mismatch {
            problems.push(Problem::Pattern(ExpressionRole::Positional, mismatch));
            return unknown_binding(accumulated, parent, body_pattern);
        }
        match frame.mapped(accumulated, mapper, body_pattern) {
            Ok((_, scope)) => scope,
            Err(mismatch) => {
                problems.push(Problem::Result(mismatch));
                unknown_binding(accumulated, parent, body_pattern)
            }
        }
    }
}

fn flatten(regions: &[CollectionRegion], plans: &mut Vec<RegionPlan>) {
    for region in regions {
        plans.extend(RegionPlan::new(region));
        flatten(&region.children, plans);
    }
}

enum Step<'a> {
    Guard(&'a BodyGuard),
    Region(&'a RegionPlan),
}

/// The scope inventory of one body: which locals and guard facts hold at each
/// cursor or site.
///
/// This is the one authority for in-body scopes. [`BodyGuards`] is its
/// conditional-only projection ([`Self::conditional`]), and a document with
/// no `::map` / `::filter` region answers exactly as that projection does.
/// Build one with [`Self::scan`], or take it from
/// [`CollectionAnalysis::scopes`](super::CollectionAnalysis::scopes).
///
/// Scopes are derived, not stored: [`Self::scope_at`] takes the site scope
/// the caller owns and layers each enclosing guard and region over it, so the
/// result borrows the same declarations.
#[derive(Debug, Clone, Default)]
pub struct BodyScopes {
    guards: BodyGuards,
    plans: Vec<RegionPlan>,
    /// Byte offset of the start of each line of the scanned body; index 0 is
    /// line 1. Empty for an inventory built from an analysis.
    line_starts: Vec<usize>,
}

impl BodyScopes {
    /// An inventory with no scoped regions: every offset sees the site scope.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Scans `body` for the guards and regions `operations` enables.
    ///
    /// This is the editor-safe entry: a body whose regions do not pair, or a
    /// region whose header does not validate, contributes no scope (the
    /// strict parse reports it), which only makes analysis more conservative.
    /// A disabled wrapper is inert text and contributes nothing.
    pub fn scan(body: &str, operations: &ComposeOperationSet) -> Self {
        Self::scan_body(body, 0, operations)
    }

    /// [`Self::scan`] for a document body that starts after `frontmatter_lines`
    /// lines of its file, so each guard's label names the marker's file line.
    pub fn scan_body(body: &str, frontmatter_lines: usize, operations: &ComposeOperationSet) -> Self {
        let guards = if operations.contains(ComposeOperation::PageBlocks) {
            BodyGuards::scan_body(body, frontmatter_lines)
        } else {
            BodyGuards::default()
        };
        let regions = if operations.contains(ComposeOperation::CollectionBlocks) && body.contains("::") {
            scan_collection_regions_lenient(body).regions
        } else {
            Vec::new()
        };
        let mut scopes = Self::from_parts(guards, &regions);
        scopes.line_starts = std::iter::once(0)
            .chain(body.match_indices('\n').map(|(index, _)| index + 1))
            .collect();
        scopes
    }

    pub(super) fn from_parts(guards: BodyGuards, regions: &[CollectionRegion]) -> Self {
        let mut plans = Vec::new();
        flatten(regions, &mut plans);
        Self { guards, plans, line_starts: Vec::new() }
    }

    /// Whether the body has neither a guarded nor a collection region.
    pub fn is_empty(&self) -> bool {
        self.guards.is_empty() && self.plans.is_empty()
    }

    /// The conditional projection: the `::block when` guards alone.
    pub fn conditional(&self) -> &BodyGuards {
        &self.guards
    }

    pub(super) fn plans(&self) -> &[RegionPlan] {
        &self.plans
    }

    /// `site` narrowed by every guard and region enclosing byte `offset`,
    /// outermost first.
    ///
    /// Inside a region's body the result binds the region's body frame and
    /// carries its selection facts; the region's own header is outside its
    /// body. An untypable region contributes unknown local types.
    pub fn scope_at<'d>(&self, site: TypeScope<'d>, offset: usize) -> TypeScope<'d> {
        let mut steps: Vec<(usize, Step<'_>)> = self
            .guards
            .enclosing(offset)
            .into_iter()
            .map(|guard| (guard.body_span.start, Step::Guard(guard)))
            .chain(
                self.plans
                    .iter()
                    .filter(|plan| plan.body_span.start <= offset && offset < plan.body_span.end)
                    .map(|plan| (plan.body_span.start, Step::Region(plan))),
            )
            .collect();
        steps.sort_by_key(|(start, _)| *start);
        steps.into_iter().fold(site, |scope, (_, step)| match step {
            Step::Guard(guard) => guard.assume(&scope),
            Step::Region(plan) => plan.evaluate(&scope).body,
        })
    }

    /// [`Self::scope_at`] for the start of the 1-based `line` of the scanned
    /// body, or `site` unchanged when the inventory has no such line.
    pub fn scope_at_line<'d>(&self, site: TypeScope<'d>, line: usize) -> TypeScope<'d> {
        match line.checked_sub(1).and_then(|index| self.line_starts.get(index)) {
            Some(&offset) => self.scope_at(site, offset),
            None => site,
        }
    }

    /// Every authored expression of every region in header order, outermost
    /// region first, each with the scope it is evaluated in under `site`.
    ///
    /// A gate is evaluated in the enclosing scope; a callback in its own
    /// parameter frame. Source paths are not expressions and are not listed.
    pub fn expression_sites<'d>(&self, site: &TypeScope<'d>) -> Vec<ExpressionSite<'d>> {
        let mut sites = Vec::new();
        for plan in &self.plans {
            let outer = self.scope_at(site.clone(), plan.span.start);
            if let Some(gate) = &plan.gate {
                sites.push(ExpressionSite {
                    role: ExpressionRole::Gate,
                    region: plan.span.clone(),
                    text: gate.quoted.text.clone(),
                    origin: authored_origin(&gate.quoted),
                    quoted: gate.quoted.clone(),
                    expression: gate.spanned.clone(),
                    scope: outer.clone(),
                });
            }
            let evaluation = plan.evaluate(&outer);
            let mut ordered: Vec<(ExpressionRole, &ParsedCallback)> = plan.callbacks().collect();
            ordered.sort_by_key(|(_, callback)| callback.quoted.span.start);
            for (role, callback) in ordered {
                let Some((_, scope)) = evaluation.callbacks.iter().find(|(found, _)| *found == role) else {
                    continue;
                };
                sites.push(ExpressionSite {
                    role,
                    region: plan.span.clone(),
                    text: callback.quoted.text.clone(),
                    origin: authored_origin(&callback.quoted),
                    quoted: callback.quoted.clone(),
                    expression: callback.spanned.body.clone(),
                    scope: scope.clone(),
                });
            }
        }
        sites
    }
}

/// The authored offset of the first decoded byte.
pub(super) fn authored_origin(quoted: &QuotedExpression) -> usize {
    quoted.authored_range(0..1.min(quoted.text.len())).map_or(quoted.span.start + 1, |range| range.start)
}
