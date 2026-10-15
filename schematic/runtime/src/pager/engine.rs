//! The traversal state machine, free of I/O and of the endpoint's types.
//!
//! [`Engine::plan`] says what the next request of a direction carries, or
//! that there is none; [`Engine::accept`] reads a decoded page, returns its
//! items, and decides the next position of both directions. The pager in
//! `pages.rs` sends the planned request and decodes the items.
//!
//! Decisions about a page that do not invalidate the page itself (a missing,
//! unusable, or repeated continuation) are stored as the direction's next
//! position, so the page's items are still delivered and the error is
//! returned by the next request of that direction, before anything is sent.

use http::HeaderMap;
use serde_json::Value;
use url::Url;

use super::error::ContinuationProblem;
use super::json::{self, Continuation, Read};
use super::link;
use super::progress::{BoundaryError, BoundaryTracker, History};
use super::spec::{CountUnit, CursorLeg, CursorSource, IndexBase, Input, ItemSelector, Kind, LinkSource, MoreSignal, Polarity, Strategy};
use crate::meta::ResponseMeta;

/// A direction of traversal. A page-number or offset strategy has only
/// [`Forward`](Self::Forward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Forward,
    Backward,
}

/// A position the caller chose before the first request.
#[derive(Debug, Clone, Default)]
enum Start {
    #[default]
    First,
    Page(u64),
    Offset(u64),
    After(Vec<Value>),
    Before(Vec<Value>),
}

/// The next request of one direction.
#[derive(Debug, Clone)]
enum Position {
    /// Nothing has been sent in this traversal.
    Start,
    Page(u64),
    Offset(u64),
    /// Cursor values, and the response they came from.
    Cursor(Vec<Value>, Box<ResponseMeta>),
    /// A resolved link, and the response it came from.
    Link(Url, Box<ResponseMeta>),
    /// The next index cannot be represented.
    Overflow,
    /// The provider signaled the end.
    Done,
    /// No continuation in this direction is known.
    Unavailable,
    /// The last page shows that the next request cannot be made.
    Failed(Deferred),
}

/// A failure the last page proved, reported by the next request of its
/// direction.
#[derive(Debug, Clone)]
pub(crate) struct Deferred {
    pub(crate) kind: DeferredKind,
    pub(crate) meta: Box<ResponseMeta>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeferredKind {
    Missing,
    Invalid(ContinuationProblem),
    Repeated,
}

/// What the next request of a direction is.
#[derive(Debug)]
pub(crate) enum Planned {
    Send(Plan),
    /// The traversal in this direction is over.
    Done,
    /// The next request cannot be made; nothing is sent.
    Failed(Deferred),
}

/// One page request to send.
#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub(crate) direction: Direction,
    /// Whether this request follows an earlier page, so the follow-up shape
    /// and carry rule apply.
    pub(crate) follow_up: bool,
    /// The followed link, for a next-link strategy.
    pub(crate) link: Option<Url>,
    /// Managed inputs, written over whatever the request already holds.
    pub(crate) inputs: Vec<(Input, Value)>,
    /// Why an argument is refused; the request is reported as an invalid
    /// argument without being sent.
    pub(crate) problem: Option<String>,
}

/// A decoded page as the engine reads it.
pub(crate) struct PageView<'p> {
    pub(crate) body: &'p Value,
    /// The response headers as received, not redacted.
    pub(crate) headers: &'p HeaderMap,
    /// The URL of the request that received the response, after redirects,
    /// before any credential: the base of a relative link.
    pub(crate) url: Option<&'p Url>,
    pub(crate) meta: &'p ResponseMeta,
}

/// A page whose items can be delivered.
#[derive(Debug)]
pub(crate) struct Accepted {
    /// The projected items to decode, in document order.
    pub(crate) items: Vec<Value>,
    pub(crate) total: Option<u64>,
}

/// Why a page delivers nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rejected {
    /// The authored collection is missing, not an array, or not projectable.
    Malformed,
    /// Every record was already delivered at an inclusive boundary.
    BoundaryNoProgress,
    /// An empty page-number or offset page whose signal says more follows.
    EmptyPageClaimsMore,
}

/// What the request in flight was sent with.
#[derive(Debug, Clone, Copy)]
struct Sent {
    direction: Direction,
    page: Option<u64>,
    offset: Option<u64>,
    size: Option<u32>,
}

/// Configuration and iteration state of one traversal. A clone starts an
/// independent traversal from the same configuration (a partition).
#[derive(Debug, Clone)]
pub(crate) struct Engine {
    strategy: &'static Strategy,
    selector: &'static ItemSelector,
    size: Option<u32>,
    size_adjusted: bool,
    start: Start,
    forward: Position,
    backward: Position,
    forward_history: History,
    backward_history: History,
    boundary: Option<BoundaryTracker>,
    sent: Option<Sent>,
    pages: u32,
    records: u64,
    items: u64,
    total: Option<u64>,
}

impl Engine {
    pub(crate) fn new(strategy: &'static Strategy, selector: &'static ItemSelector) -> Self {
        let boundary = match strategy.kind {
            Kind::Cursor { forward, .. } => BoundaryTracker::new(&forward),
            _ => None,
        };
        Self {
            strategy,
            selector,
            size: None,
            size_adjusted: false,
            start: Start::First,
            forward: Position::Start,
            backward: Position::Unavailable,
            forward_history: History::default(),
            backward_history: History::default(),
            boundary,
            sent: None,
            pages: 0,
            records: 0,
            items: 0,
            total: None,
        }
    }

    pub(crate) fn set_page_size(&mut self, size: u32) {
        self.size = Some(size);
    }

    /// Changes the size from the next request on.
    pub(crate) fn adjust_page_size(&mut self, size: u32) {
        self.size = Some(size);
        self.size_adjusted = true;
    }

    pub(crate) fn start_at_page(&mut self, page: u64) {
        self.start = Start::Page(page);
    }

    pub(crate) fn start_at_offset(&mut self, offset: u64) {
        self.start = Start::Offset(offset);
    }

    pub(crate) fn start_after(&mut self, values: Vec<Value>) {
        self.start = Start::After(values);
    }

    pub(crate) fn start_before(&mut self, values: Vec<Value>) {
        self.start = Start::Before(values);
    }

    /// Page requests planned so far.
    pub(crate) fn pages(&self) -> u32 {
        self.pages
    }

    /// Items delivered so far.
    pub(crate) fn items(&self) -> u64 {
        self.items
    }

    /// The latest total observed.
    pub(crate) fn total(&self) -> Option<u64> {
        self.total
    }

    /// Whether the forward direction has observed the provider's end.
    pub(crate) fn is_finished(&self) -> bool {
        matches!(self.forward, Position::Done)
    }

    /// Whether the last page proved the next forward request cannot be
    /// made; the next request reports it without sending.
    pub(crate) fn has_deferred_failure(&self) -> bool {
        matches!(self.forward, Position::Failed(_))
    }

    pub(crate) fn adjust_retry_size(&mut self, plan: &mut Plan, size: u32) {
        self.adjust_page_size(size);
        if let Some(spec) = self.strategy.size {
            plan.inputs.retain(|(input, _)| *input != spec.param);
        }
        if plan.problem.as_deref().is_some_and(|problem| problem.starts_with("the page size ")) {
            plan.problem = None;
        }
        if let Some(mut sent) = self.sent {
            self.plan_managed(plan, &mut sent);
            self.sent = Some(sent);
        }
    }

    /// Keep delivery facts when re-reading an already delivered page solely
    /// to repair its continuation. The refreshed positions and total survive.
    pub(crate) fn restore_delivery_from(&mut self, delivered: &Self) {
        self.records = delivered.records;
        self.items = delivered.items;
        self.pages = delivered.pages;
        self.boundary = delivered.boundary.clone();
        self.forward_history = delivered.forward_history.clone();
        self.backward_history = delivered.backward_history.clone();
    }

    /// The page size the next request sends: the caller's, else the
    /// authored sendable default.
    fn effective_size(&self) -> Option<u32> {
        self.size.or(self.strategy.size.and_then(|size| size.sent_default))
    }

    /// Plans the next request of `direction`.
    pub(crate) fn plan(&mut self, direction: Direction) -> Planned {
        let position = match direction {
            Direction::Forward => &self.forward,
            Direction::Backward => &self.backward,
        };
        let mut plan = Plan {
            direction,
            follow_up: true,
            link: None,
            inputs: Vec::new(),
            problem: None,
        };
        let mut sent = Sent {
            direction,
            page: None,
            offset: None,
            size: None,
        };
        match position.clone() {
            Position::Done | Position::Unavailable => return Planned::Done,
            Position::Failed(deferred) => return Planned::Failed(deferred),
            Position::Overflow => plan.problem = Some("the next page position overflows".to_owned()),
            Position::Start => {
                plan.follow_up = false;
                self.plan_start(&mut plan, &mut sent);
            }
            Position::Page(page) => {
                if let Kind::PageNumber { page: input, .. } = self.strategy.kind {
                    plan.inputs.push((input, Value::from(page)));
                    sent.page = Some(page);
                }
            }
            Position::Offset(offset) => {
                if let Kind::Offset { offset: input, .. } = self.strategy.kind {
                    plan.inputs.push((input, Value::from(offset)));
                    sent.offset = Some(offset);
                }
            }
            Position::Cursor(values, from) => {
                let Some(leg) = self.leg(direction) else {
                    return Planned::Done;
                };
                if self.tracks_history(direction) && !self.history(direction).insert(&history_key(&values)) {
                    return Planned::Failed(Deferred {
                        kind: DeferredKind::Repeated,
                        meta: from,
                    });
                }
                push_parts(&mut plan, &leg, values);
            }
            Position::Link(url, from) => {
                if !self.history(direction).insert(&[url.as_str().to_owned()]) {
                    return Planned::Failed(Deferred {
                        kind: DeferredKind::Repeated,
                        meta: from,
                    });
                }
                plan.link = Some(url);
            }
        }
        if plan.direction != direction {
            sent.direction = plan.direction;
        }
        self.plan_managed(&mut plan, &mut sent);
        self.sent = Some(sent);
        self.pages = self.pages.saturating_add(1);
        Planned::Send(plan)
    }

    /// The first request: the caller's start position, or the strategy's.
    fn plan_start(&mut self, plan: &mut Plan, sent: &mut Sent) {
        let start = std::mem::take(&mut self.start);
        match self.strategy.kind {
            Kind::PageNumber { page, base, .. } => {
                let number = match start {
                    Start::Page(number) => number,
                    _ => base.first(),
                };
                if number < base.first() {
                    plan.problem = Some(format!("the start page is below the first page, {}", base.first()));
                }
                plan.inputs.push((page, Value::from(number)));
                sent.page = Some(number);
            }
            Kind::Offset { offset, base, .. } => {
                let index = match start {
                    Start::Offset(index) => index,
                    _ => base.first(),
                };
                if index < base.first() {
                    plan.problem = Some(format!("the start offset is below the first offset, {}", base.first()));
                }
                plan.inputs.push((offset, Value::from(index)));
                sent.offset = Some(index);
            }
            Kind::Cursor { forward, backward, .. } => match start {
                Start::After(values) => {
                    if self.tracks_history(Direction::Forward) {
                        self.forward_history.insert(&history_key(&values));
                    }
                    if values.len() == forward.parts.len() {
                        push_parts(plan, &forward, values);
                    } else {
                        plan.problem = Some("the start cursor has the wrong number of parts".to_owned());
                    }
                }
                Start::Before(values) => match backward {
                    Some(leg) if values.len() == leg.parts.len() => {
                        self.backward_history.insert(&history_key(&values));
                        plan.direction = Direction::Backward;
                        push_parts(plan, &leg, values);
                    }
                    _ => plan.problem = Some("the start cursor has the wrong number of parts".to_owned()),
                },
                _ => {}
            },
            Kind::NextLink { .. } => {}
        }
    }

    /// The size and constants every request of the traversal manages.
    fn plan_managed(&self, plan: &mut Plan, sent: &mut Sent) {
        if let Some(size_spec) = self.strategy.size {
            // A followed link carries the provider's own size unless the
            // caller changed it during the traversal.
            let send_size = plan.link.is_none() || self.size_adjusted;
            if let Some(size) = self.effective_size()
                && send_size
            {
                if size < size_spec.min {
                    plan.problem.get_or_insert(format!("the page size is below the declared minimum, {}", size_spec.min));
                }
                if let Some(max) = size_spec.max
                    && size > max
                {
                    plan.problem.get_or_insert(format!("the page size is above the declared maximum, {max}"));
                }
                plan.inputs.push((size_spec.param, Value::from(size)));
                sent.size = Some(size);
            } else if let Some(size) = self.effective_size() {
                sent.size = Some(size);
            }
        }
        if plan.link.is_none() {
            for fixed in self.strategy.constants {
                plan.inputs.push((fixed.param, Value::from(fixed.value)));
            }
        }
    }

    fn leg(&self, direction: Direction) -> Option<CursorLeg> {
        match (self.strategy.kind, direction) {
            (Kind::Cursor { forward, .. }, Direction::Forward) => Some(forward),
            (Kind::Cursor { backward, .. }, Direction::Backward) => backward,
            _ => None,
        }
    }

    fn history(&mut self, direction: Direction) -> &mut History {
        match direction {
            Direction::Forward => &mut self.forward_history,
            Direction::Backward => &mut self.backward_history,
        }
    }

    /// An inclusive forward boundary repeats its key on purpose; its
    /// progress is the tracker's, not the history's.
    fn tracks_history(&self, direction: Direction) -> bool {
        !(direction == Direction::Forward && self.boundary.is_some())
    }

    /// Reads a page received for the last planned request.
    ///
    /// ## Errors
    ///
    /// Returns [`Rejected`] when the page delivers no item: its collection
    /// is malformed, it makes no progress at an inclusive boundary, or it is
    /// an empty page that claims more follow.
    pub(crate) fn accept(&mut self, view: &PageView<'_>) -> Result<Accepted, Rejected> {
        let sent = self.sent.take().unwrap_or(Sent {
            direction: Direction::Forward,
            page: None,
            offset: None,
            size: None,
        });
        let records = json::records(view.body, self.selector).map_err(|_| Rejected::Malformed)?;
        let projected = records
            .iter()
            .map(|record| json::project(record, self.selector))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Rejected::Malformed)?;
        let kept: Vec<usize> = match (&mut self.boundary, sent.direction) {
            (Some(tracker), Direction::Forward) => tracker.accept(&records).map_err(|error| match error {
                BoundaryError::NoProgress => Rejected::BoundaryNoProgress,
                BoundaryError::MissingKey => Rejected::Malformed,
            })?,
            _ => (0..records.len()).collect(),
        };
        let items: Vec<Value> = kept.iter().map(|&index| projected[index].clone()).collect();
        self.records = self.records.saturating_add(records.len() as u64);

        let meta = Box::new(view.meta.clone());
        let fail = |kind| Position::Failed(Deferred { kind, meta: meta.clone() });

        let informational = match self.strategy.total {
            Some(source) => json::total(&json::read(source, view.body, view.headers)),
            None => Ok(None),
        };
        let signal = self.more_signal(view, records.len(), sent);
        let (next, other) = match (informational, signal) {
            (Err(()), _) => (fail(DeferredKind::Invalid(ContinuationProblem::Total)), Position::Unavailable),
            (_, Err(problem)) => (fail(DeferredKind::Invalid(problem)), Position::Unavailable),
            (Ok(total), Ok((more, signal_total))) => {
                if let Some(total) = total.or(signal_total) {
                    self.total = Some(total);
                }
                self.next_positions(view, &records, sent, more, &meta)?
            }
        };
        // A deliberate step in one direction is not a cycle of the other:
        // each run in a direction starts its own history.
        match sent.direction {
            Direction::Forward => {
                self.forward = next;
                self.backward = other;
                self.backward_history.clear();
            }
            Direction::Backward => {
                self.backward = next;
                self.forward = other;
                self.forward_history.clear();
            }
        }
        self.items = self.items.saturating_add(items.len() as u64);
        Ok(Accepted {
            items,
            total: self.total,
        })
    }

    /// The next position of the requested direction and of the other one.
    fn next_positions(
        &self,
        view: &PageView<'_>,
        records: &[&Value],
        sent: Sent,
        more: Option<bool>,
        meta: &ResponseMeta,
    ) -> Result<(Position, Position), Rejected> {
        let fail = |kind| {
            Position::Failed(Deferred {
                kind,
                meta: Box::new(meta.clone()),
            })
        };
        let count = records.len() as u64;
        match self.strategy.kind {
            Kind::PageNumber { .. } | Kind::Offset { .. } => {
                let more = more.unwrap_or(count > 0);
                if count == 0 && more {
                    return Err(Rejected::EmptyPageClaimsMore);
                }
                let next = if !more {
                    Position::Done
                } else if let Some(page) = sent.page {
                    page.checked_add(1).map_or(Position::Overflow, Position::Page)
                } else if let Some(offset) = sent.offset {
                    offset.checked_add(count).map_or(Position::Overflow, Position::Offset)
                } else {
                    Position::Done
                };
                Ok((next, Position::Unavailable))
            }
            Kind::Cursor { forward, backward, .. } => {
                let (leg, other_leg) = match sent.direction {
                    Direction::Forward => (Some(forward), backward),
                    Direction::Backward => (backward, Some(forward)),
                };
                let next = match leg.map(|leg| self.leg_continuation(&leg, view, records)) {
                    None => Position::Done,
                    Some(continuation) => match (more, continuation) {
                        (Some(false), _) => Position::Done,
                        (_, LegContinuation::Invalid) => fail(DeferredKind::Invalid(ContinuationProblem::Cursor)),
                        (_, LegContinuation::Values(values)) => Position::Cursor(values, Box::new(meta.clone())),
                        (Some(true), LegContinuation::End) | (_, LegContinuation::Partial) => {
                            fail(DeferredKind::Missing)
                        }
                        (None, LegContinuation::End) => Position::Done,
                    },
                };
                let other = match other_leg.map(|leg| self.leg_continuation(&leg, view, records)) {
                    Some(LegContinuation::Values(values)) => Position::Cursor(values, Box::new(meta.clone())),
                    _ => Position::Unavailable,
                };
                Ok((next, other))
            }
            Kind::NextLink { next, prev } => {
                let (source, other_source) = match sent.direction {
                    Direction::Forward => (Some(next), prev),
                    Direction::Backward => (prev, Some(next)),
                };
                let next = match source.map(|source| self.link_continuation(source, view)) {
                    None | Some(Ok(None)) => Position::Done,
                    Some(Ok(Some(url))) => Position::Link(url, Box::new(meta.clone())),
                    Some(Err(problem)) => fail(DeferredKind::Invalid(problem)),
                };
                let other = match other_source.map(|source| self.link_continuation(source, view)) {
                    Some(Ok(Some(url))) => Position::Link(url, Box::new(meta.clone())),
                    _ => Position::Unavailable,
                };
                Ok((next, other))
            }
        }
    }

    /// The authored end signal of a page: `Some(more)` when it decides,
    /// `None` when it is absent or cannot decide, and the total it carries.
    fn more_signal(&self, view: &PageView<'_>, records: usize, sent: Sent) -> Result<(Option<bool>, Option<u64>), ContinuationProblem> {
        let Some(signal) = self.strategy.more() else {
            return Ok((None, None));
        };
        let read = |source| json::read(source, view.body, view.headers);
        match signal {
            MoreSignal::Flag { source, polarity } => {
                let flag = json::flag(&read(source)).map_err(|()| ContinuationProblem::MoreSignal)?;
                Ok((flag.map(|value| value == (polarity == Polarity::TrueMeansMore)), None))
            }
            MoreSignal::Present { source } => Ok((Some(json::present(&read(source))), None)),
            MoreSignal::LinkRel { rel } => {
                let targets = link::targets_with_rel(view.headers, rel).map_err(|_| ContinuationProblem::Link)?;
                Ok((Some(!targets.is_empty()), None))
            }
            MoreSignal::Total { source, unit } => {
                let Some(total) = json::total(&read(source)).map_err(|()| ContinuationProblem::Total)? else {
                    return Ok((None, None));
                };
                Ok((self.covered(sent, records, unit).map(|covered| covered < total), Some(total)))
            }
            MoreSignal::NonEmptyPage => Ok((Some(records > 0), None)),
            MoreSignal::FullPage => match sent.size {
                Some(size) => Ok((Some(records >= size as usize), None)),
                None => Ok((Some(records > 0), None)),
            },
        }
    }

    /// How much of a total, in `unit`, the traversal has covered once this
    /// page is counted; `None` when it cannot be known.
    fn covered(&self, sent: Sent, records: usize, unit: CountUnit) -> Option<u64> {
        let pages_through = |base: IndexBase, page: u64| page.checked_sub(base.first())?.checked_add(1);
        match (self.strategy.kind, unit) {
            (Kind::PageNumber { base, .. }, CountUnit::Pages) => pages_through(base, sent.page?),
            (Kind::PageNumber { base, .. }, CountUnit::Items) => {
                pages_through(base, sent.page?)?.checked_mul(u64::from(sent.size?))
            }
            (Kind::Offset { base, .. }, CountUnit::Items) => {
                sent.offset?.checked_add(records as u64)?.checked_sub(base.first())
            }
            (Kind::Offset { base, .. }, CountUnit::Pages) => {
                let size = u64::from(sent.size?);
                let through = sent.offset?.checked_add(records as u64)?.checked_sub(base.first())?;
                Some(through.div_ceil(size.max(1)))
            }
            (_, CountUnit::Items) => Some(self.records),
            (_, CountUnit::Pages) => Some(u64::from(self.pages)),
        }
    }

    /// The values of `leg` a page carries.
    fn leg_continuation(&self, leg: &CursorLeg, view: &PageView<'_>, records: &[&Value]) -> LegContinuation {
        let mut values = Vec::new();
        let mut ended = 0;
        for part in leg.parts {
            let structured = matches!(part.param, Input::Body(_));
            let read = match part.source {
                CursorSource::Field(source) => json::read(source, view.body, view.headers),
                CursorSource::LastItem(path) => records
                    .last()
                    .and_then(|record| json::lookup(record, path))
                    .map_or(Read::Absent, Read::Json),
                CursorSource::FirstItem(path) => records
                    .first()
                    .and_then(|record| json::lookup(record, path))
                    .map_or(Read::Absent, Read::Json),
            };
            match json::continuation(&read, self.strategy.terminal_values, structured) {
                Continuation::Value(value) => values.push(value),
                Continuation::End => ended += 1,
                Continuation::Invalid => return LegContinuation::Invalid,
            }
        }
        match (values.is_empty(), ended) {
            (_, 0) => LegContinuation::Values(values),
            (true, _) => LegContinuation::End,
            (false, _) => LegContinuation::Partial,
        }
    }

    /// The URL a link source names, resolved against the response's URL.
    fn link_continuation(&self, source: LinkSource, view: &PageView<'_>) -> Result<Option<Url>, ContinuationProblem> {
        let targets = match source {
            LinkSource::LinkHeader { rel } => {
                link::targets_with_rel(view.headers, rel).map_err(|_| ContinuationProblem::Link)?
            }
            LinkSource::Field(field) => {
                match json::continuation(&json::read(field, view.body, view.headers), self.strategy.terminal_values, false) {
                    Continuation::End => Vec::new(),
                    Continuation::Value(Value::String(text)) => vec![text],
                    Continuation::Value(_) | Continuation::Invalid => return Err(ContinuationProblem::Link),
                }
            }
        };
        let mut resolved: Vec<Url> = Vec::new();
        for target in targets {
            let mut url = match view.url {
                Some(base) => base.join(&target),
                None => Url::parse(&target),
            }
            .map_err(|_| ContinuationProblem::Link)?;
            url.set_fragment(None);
            if !resolved.contains(&url) {
                resolved.push(url);
            }
        }
        match resolved.len() {
            0 => Ok(None),
            1 => Ok(resolved.pop()),
            _ => Err(ContinuationProblem::AmbiguousLink),
        }
    }
}

/// The values of one cursor leg found on a page.
enum LegContinuation {
    Values(Vec<Value>),
    /// Every part ended.
    End,
    /// Some parts are present and some ended.
    Partial,
    Invalid,
}

fn push_parts(plan: &mut Plan, leg: &CursorLeg, values: Vec<Value>) {
    for (part, value) in leg.parts.iter().zip(values) {
        plan.inputs.push((part.param, value));
    }
}

/// Cursor values compared exactly, by their JSON text.
fn history_key(values: &[Value]) -> Vec<String> {
    values.iter().map(Value::to_string).collect()
}
