//! Validation of strategies, endpoint collections, and list shapes.

use std::collections::HashSet;
use std::fmt;

use super::collection::{Collection, Pagination};
use super::resolve::{AdoptionOutcome, ParamLocation, Resolution, param_adoptions, path_params, request_inputs, resolve};
use super::selector::{JsonPath, Step};
use super::strategy::{
    Boundary, Carry, CountUnit, CursorSource, InputParam, LinkSource, MoreSignal, PageSize,
    PageStrategy, Session, SessionOpen, SessionStep, Strategy, ValueSource,
};
use crate::params::{ParamDef, QueryParamType};
use crate::response::ApiResponse;
use crate::types::{Endpoint, RestApi, RestMethod};

/// The group a [`PaginationError`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationRuleGroup {
    /// The shape of a strategy, alone or on its endpoint.
    Strategy,
    /// An endpoint collection or the list shape it names.
    Exception,
    /// A strategy input against the endpoint's declared parameters.
    Parameter,
}

/// One broken pagination rule.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaginationError {
    // Strategy shape
    UnmatchedManagedParam(String),
    CursorWithoutEndSignal,
    FullPageOnCursor,
    FullPageWithoutPageSize,
    PageCountOnNonPageNumber,
    InvalidPageSize {
        min: u32,
        server_default: Option<u32>,
        sent_default: Option<u32>,
        max: Option<u32>,
    },
    DuplicateInput(String),
    EmptyValue(&'static str),
    EmptyCursorLeg,
    InclusiveBoundaryNeedsItemKey,
    AdjustablePageSizeOnPageNumber,
    AdjustableLinkSizeNotQuery,
    NextLinkWithFollowUpPath,
    NextLinkMethodUnspecified,
    BodyInputOnGet(String),
    UnknownCarryParam(String),
    UnknownPathParam(String),
    SessionHandleUnreachable,
    SessionHandleNotSent,
    SessionStepNeedsHandle(String),
    KeepAliveWithoutInput,
    InvalidPartitionMax,
    PartitionedFirstPageSession,
    // Endpoint collection and list shape
    InvalidItemSelector { selector: String, reason: &'static str },
    NotAnArrayLocation(String),
    MissingItemType,
    AlternatesWithoutPrimary,
    AlternatesOnUnpaginated,
    DuplicateLabel(String),
    InvalidLabel(String),
    // Declared parameters
    CollidesWithEndpointParam(String),
    AdoptionWithoutKnob(String),
    /// `strategy` is the adopting strategy's [`PageStrategy::kind_name`].
    PagerManagedRequiredWithoutDefault { strategy: &'static str, param: String },
    /// The value the pager sends for a required adopted parameter does not
    /// parse as the parameter's declared type; `expected` names that type.
    PagerManagedDefaultInvalid { strategy: &'static str, param: String, value: String, expected: &'static str },
}

impl PaginationError {
    /// The rule's stable kebab-case id.
    pub fn rule(&self) -> &'static str {
        match self {
            Self::UnmatchedManagedParam(_) => "unmatched-pager-managed-parameter",
            Self::CursorWithoutEndSignal => "cursor-without-end-signal",
            Self::FullPageOnCursor => "full-page-on-cursor",
            Self::FullPageWithoutPageSize => "full-page-without-page-size",
            Self::PageCountOnNonPageNumber => "page-count-on-non-page-number",
            Self::InvalidPageSize { .. } => "invalid-page-size",
            Self::DuplicateInput(_) => "duplicate-input",
            Self::EmptyValue(_) => "empty-value",
            Self::EmptyCursorLeg => "empty-cursor-leg",
            Self::InclusiveBoundaryNeedsItemKey => "inclusive-boundary-needs-item-key",
            Self::AdjustablePageSizeOnPageNumber => "adjustable-page-size-on-page-number",
            Self::AdjustableLinkSizeNotQuery => "adjustable-link-size-not-query",
            Self::NextLinkWithFollowUpPath => "next-link-with-follow-up-path",
            Self::NextLinkMethodUnspecified => "next-link-method-unspecified",
            Self::BodyInputOnGet(_) => "body-input-on-get",
            Self::UnknownCarryParam(_) => "unknown-carry-param",
            Self::UnknownPathParam(_) => "unknown-path-param",
            Self::SessionHandleUnreachable => "session-handle-unreachable",
            Self::SessionHandleNotSent => "session-handle-not-sent",
            Self::SessionStepNeedsHandle(_) => "session-step-needs-handle",
            Self::KeepAliveWithoutInput => "keep-alive-without-input",
            Self::InvalidPartitionMax => "invalid-partition-max",
            Self::PartitionedFirstPageSession => "partitioned-first-page-session",
            Self::InvalidItemSelector { .. } => "invalid-item-selector",
            Self::NotAnArrayLocation(_) => "not-an-array-location",
            Self::MissingItemType => "missing-item-type",
            Self::AlternatesWithoutPrimary => "alternates-without-primary",
            Self::AlternatesOnUnpaginated => "alternates-on-unpaginated",
            Self::DuplicateLabel(_) => "duplicate-label",
            Self::InvalidLabel(_) => "invalid-label",
            Self::CollidesWithEndpointParam(_) => "collides-with-endpoint-param",
            Self::AdoptionWithoutKnob(_) => "adoption-without-knob",
            Self::PagerManagedRequiredWithoutDefault { .. } => "pager-managed-required-without-default",
            Self::PagerManagedDefaultInvalid { .. } => "pager-managed-default-invalid",
        }
    }

    pub fn group(&self) -> PaginationRuleGroup {
        match self {
            Self::InvalidItemSelector { .. }
            | Self::NotAnArrayLocation(_)
            | Self::MissingItemType
            | Self::AlternatesWithoutPrimary
            | Self::AlternatesOnUnpaginated
            | Self::DuplicateLabel(_)
            | Self::InvalidLabel(_) => PaginationRuleGroup::Exception,
            Self::UnmatchedManagedParam(_)
            | Self::CollidesWithEndpointParam(_)
            | Self::AdoptionWithoutKnob(_)
            | Self::PagerManagedRequiredWithoutDefault { .. }
            | Self::PagerManagedDefaultInvalid { .. } => PaginationRuleGroup::Parameter,
            _ => PaginationRuleGroup::Strategy,
        }
    }
}

impl fmt::Display for PaginationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedManagedParam(name) => write!(f, "pager-managed parameter `{name}` has no matching strategy input and caller control"),
            Self::CursorWithoutEndSignal => f.write_str(
                "a cursor read only from items has no end signal; add `more_when(..)`",
            ),
            Self::FullPageOnCursor => {
                f.write_str("a short page cannot end a cursor or next-link strategy")
            }
            Self::FullPageWithoutPageSize => {
                f.write_str("a short-page end signal needs an authored page size")
            }
            Self::PageCountOnNonPageNumber => {
                f.write_str("a total in pages can end only a page-number strategy")
            }
            Self::InvalidPageSize {
                min,
                server_default,
                sent_default,
                max,
            } => write!(
                f,
                "page size bounds are inconsistent (min {min}, server default \
                 {server_default:?}, sent default {sent_default:?}, max {max:?}); min must be \
                 at least 1 and every default within min..=max"
            ),
            Self::DuplicateInput(name) => {
                write!(f, "input `{name}` is written by more than one part of the strategy")
            }
            Self::EmptyValue(what) => write!(f, "{what} is empty"),
            Self::EmptyCursorLeg => f.write_str("a cursor leg has no parts"),
            Self::InclusiveBoundaryNeedsItemKey => f.write_str(
                "an inclusive boundary needs every cursor part read from an item",
            ),
            Self::AdjustablePageSizeOnPageNumber => f.write_str(
                "a page-number strategy cannot change its page size between pages",
            ),
            Self::AdjustableLinkSizeNotQuery => f.write_str(
                "an adjustable next-link page size must be a query parameter",
            ),
            Self::NextLinkWithFollowUpPath => {
                f.write_str("a next-link strategy follows the link, so it has no follow-up path")
            }
            Self::NextLinkMethodUnspecified => f.write_str(
                "a next-link strategy on a non-GET endpoint must state its follow-up method",
            ),
            Self::BodyInputOnGet(name) => {
                write!(f, "input `{name}` is a body field on a request without a body")
            }
            Self::UnknownCarryParam(name) => write!(
                f,
                "carried name `{name}` is neither a declared query parameter nor the page size"
            ),
            Self::UnknownPathParam(name) => {
                write!(f, "path parameter `{{{name}}}` is not a parameter of the endpoint")
            }
            Self::SessionHandleUnreachable => f.write_str(
                "the session handle is read neither from an open step nor from a page",
            ),
            Self::SessionHandleNotSent => f.write_str(
                "page requests never send the session handle, and the continuation does not \
                 carry it",
            ),
            Self::SessionStepNeedsHandle(step) => {
                write!(f, "the session {step} step has no place to send the handle")
            }
            Self::KeepAliveWithoutInput => {
                f.write_str("keep-alive is sent neither on open nor on pages")
            }
            Self::InvalidPartitionMax => f.write_str("the partition maximum must be at least 1"),
            Self::PartitionedFirstPageSession => f.write_str(
                "partitions require a session opened by a separate request; first-page opening \
                 creates independent sessions without shared cleanup",
            ),
            Self::InvalidItemSelector { selector, reason } => {
                write!(f, "item selector `{selector}` is invalid: {reason}")
            }
            Self::NotAnArrayLocation(detail) => {
                write!(f, "the item location is not an array: {detail}")
            }
            Self::MissingItemType => f.write_str(
                "a paginated list must name its item type unless its response is an explicit \
                 `Vec<T>` root array",
            ),
            Self::AlternatesWithoutPrimary => {
                f.write_str("alternates need a primary strategy (an override or an API default)")
            }
            Self::AlternatesOnUnpaginated => f.write_str("an unpaginated list cannot have alternates"),
            Self::DuplicateLabel(label) => write!(f, "alternate label `{label}` is used twice"),
            Self::InvalidLabel(label) => write!(
                f,
                "alternate label `{label}` must be lowercase ASCII letters, digits, and `_`"
            ),
            Self::CollidesWithEndpointParam(name) => write!(
                f,
                "strategy input collides with declared parameter `{name}`, which it cannot adopt"
            ),
            Self::AdoptionWithoutKnob(name) => write!(
                f,
                "strategy would adopt declared parameter `{name}` without authoring the knob \
                 that replaces it; author the knob or leave the parameter out of the strategy"
            ),
            Self::PagerManagedRequiredWithoutDefault { strategy, param } => write!(
                f,
                "required parameter `{param}` is pager-managed by the `{strategy}` strategy, so \
                 the strategy must author a value to send when the caller sets none"
            ),
            Self::PagerManagedDefaultInvalid { strategy, param, value, expected } => write!(
                f,
                "required parameter `{param}` is pager-managed by the `{strategy}` strategy, whose \
                 default `{value}` is not a valid {expected}"
            ),
        }
    }
}

/// A [`PaginationError`] located in an API.
///
/// `endpoint` is `None` for the API default strategy. `alternate` names the
/// labeled alternate the error is in, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationDiagnostic {
    pub endpoint: Option<String>,
    pub alternate: Option<String>,
    pub error: PaginationError,
}

impl fmt::Display for PaginationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error.rule(), self.error)?;
        if let Some(label) = &self.alternate {
            write!(f, " (alternate `{label}`)")?;
        }
        Ok(())
    }
}

/// Validates an API's default strategy, endpoint collections, and every list it
/// resolves.
///
/// Intrinsic strategy errors of the API default are reported once, against no
/// endpoint; each endpoint that inherits it is checked only for what depends
/// on the endpoint (path parameters, methods, declared parameters).
///
pub fn validate_pagination(api: &RestApi) -> Vec<PaginationDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut push = |endpoint: Option<&str>, alternate: Option<&str>, error: PaginationError| {
        diagnostics.push(PaginationDiagnostic {
            endpoint: endpoint.map(str::to_owned),
            alternate: alternate.map(str::to_owned),
            error,
        });
    };

    if let Some(default) = &api.default_pagination {
        for error in validate_strategy(default) {
            push(None, None, error);
        }
    }

    for endpoint in &api.endpoints {
        let resolution = resolve(api, endpoint);
        let adopted = resolution.strategy().map(|strategy| super::adopted_params(endpoint, strategy)).unwrap_or_default();
        if let Some(params) = &endpoint.params {
            for (location, parameters) in [(super::ParamLocation::Query, &params.query),
                (super::ParamLocation::Header, &params.header), (super::ParamLocation::Cookie, &params.cookie)] {
                for param in parameters.iter().filter(|param| param.pager_managed) {
                    if !adopted.iter().any(|entry| entry.location == location && entry.name == param.name) {
                        push(Some(&endpoint.id), None, PaginationError::UnmatchedManagedParam(param.name.clone()));
                    }
                }
            }
        }
        let id = Some(endpoint.id.as_str());
        if let Some(collection) = endpoint.pagination.as_ref()
        {
            for error in collection_errors(api, endpoint, collection) {
                push(id, None, error);
            }
        }
        let Resolution::Paginated {
            list,
            strategy,
            alternates,
            origin,
        } = resolve(api, endpoint)
        else {
            continue;
        };
        if list.item_type.is_none() {
            push(id, None, PaginationError::MissingItemType);
        }
        if origin == super::ResolutionOrigin::Exception {
            for error in validate_strategy(&strategy) {
                push(id, None, error);
            }
        }
        for error in endpoint_errors(endpoint, &strategy, true) {
            push(id, None, error);
        }
        for alternate in &alternates {
            let label = Some(alternate.label.as_str());
            for error in validate_strategy(&alternate.strategy) {
                push(id, label, error);
            }
            for error in endpoint_errors(endpoint, &alternate.strategy, false) {
                push(id, label, error);
            }
        }
    }
    diagnostics
}

/// Errors in an authored collection shape.
fn collection_errors(api: &RestApi, endpoint: &Endpoint, collection: &Collection) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    if let Some(reason) = collection.items.syntax_problem() {
        errors.push(PaginationError::InvalidItemSelector {
            selector: collection.items.to_string(),
            reason,
        });
    }
    if collection.item_type.as_deref().is_some_and(|ty| ty.trim().is_empty()) {
        errors.push(PaginationError::EmptyValue("item type"));
    }
    match &endpoint.response {
        ApiResponse::Json(reference) => {
            let is_vec = super::root_array_item_type(api, &endpoint.response).is_some();
            if is_vec && matches!(collection.items.record.first(), Some(Step::Key(_))) {
                errors.push(PaginationError::NotAnArrayLocation(format!(
                    "the response `{}` is a root array, so it has no key `{}`",
                    api.rust_type(reference).unwrap_or_default(), collection.items
                )));
            }
        }
        _ => errors.push(PaginationError::NotAnArrayLocation(
            "the response is not JSON".to_owned(),
        )),
    }

    if !collection.alternates.is_empty() {
        match (&collection.pagination, &api.default_pagination) {
            (Pagination::Inherit, None) => errors.push(PaginationError::AlternatesWithoutPrimary),
            (Pagination::Unpaginated, _) => errors.push(PaginationError::AlternatesOnUnpaginated),
            _ => {}
        }
    }
    let mut labels = HashSet::new();
    for alternate in &collection.alternates {
        let label = alternate.label.as_str();
        let label_ok = !label.is_empty()
            && label
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_');
        if !label_ok {
            errors.push(PaginationError::InvalidLabel(label.to_owned()));
        }
        if !labels.insert(label) {
            errors.push(PaginationError::DuplicateLabel(label.to_owned()));
        }
    }
    errors
}

/// Validates the rules of a strategy that do not depend on its endpoint.
pub fn validate_strategy(strategy: &Strategy) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let kind = &strategy.kind;
    let cursor_like = matches!(kind, PageStrategy::Cursor(_) | PageStrategy::NextLink(_));

    empty_value_errors(strategy, &mut errors);

    for leg in kind.legs() {
        if leg.parts.is_empty() {
            errors.push(PaginationError::EmptyCursorLeg);
        }
        if matches!(leg.boundary, Boundary::Inclusive { .. })
            && !leg.parts.iter().all(|part| part.source.is_item_derived())
        {
            errors.push(PaginationError::InclusiveBoundaryNeedsItemKey);
        }
    }

    if let PageStrategy::Cursor(cursor) = kind
        && !cursor.forward.parts.is_empty()
        && cursor.forward.parts.iter().all(|part| part.source.is_item_derived())
        && cursor.more.is_none()
    {
        errors.push(PaginationError::CursorWithoutEndSignal);
    }

    match kind.more() {
        Some(MoreSignal::FullPage) if cursor_like => errors.push(PaginationError::FullPageOnCursor),
        Some(MoreSignal::FullPage) if kind.size().is_none() => {
            errors.push(PaginationError::FullPageWithoutPageSize);
        }
        Some(MoreSignal::Total {
            unit: CountUnit::Pages,
            ..
        }) if !matches!(kind, PageStrategy::PageNumber(_)) => {
            errors.push(PaginationError::PageCountOnNonPageNumber);
        }
        _ => {}
    }

    if let Some(size) = kind.size() {
        page_size_errors(size, kind, &mut errors);
    }

    let mut written: Vec<&InputParam> = Vec::new();
    for (_, input) in request_inputs(strategy) {
        if written.iter().any(|seen| seen.same_destination(input)) {
            errors.push(PaginationError::DuplicateInput(input.display_name()));
        }
        written.push(input);
    }

    if matches!(kind, PageStrategy::NextLink(_))
        && strategy.follow_up.as_ref().is_some_and(|f| f.path.is_some())
    {
        errors.push(PaginationError::NextLinkWithFollowUpPath);
    }

    if let Some(session) = &strategy.session {
        errors.extend(session_errors(session, kind));
        if strategy.partition.is_some() && matches!(session.open, SessionOpen::FirstPage) {
            errors.push(PaginationError::PartitionedFirstPageSession);
        }
    }

    if let Some(partition) = &strategy.partition
        && partition.max == Some(0)
    {
        errors.push(PaginationError::InvalidPartitionMax);
    }

    errors
}

fn page_size_errors(size: &PageSize, kind: &PageStrategy, errors: &mut Vec<PaginationError>) {
    let in_bounds = |value: Option<u32>| {
        value.is_none_or(|value| value >= size.min && size.max.is_none_or(|max| value <= max))
    };
    let max_ok = size.max.is_none_or(|max| max >= size.min);
    if size.min == 0 || !max_ok || !in_bounds(size.server_default) || !in_bounds(size.sent_default) {
        errors.push(PaginationError::InvalidPageSize {
            min: size.min,
            server_default: size.server_default,
            sent_default: size.sent_default,
            max: size.max,
        });
    }
    if size.adjustable {
        match kind {
            // Page k at a new size is not "the page after" page k-1.
            PageStrategy::PageNumber(_) => errors.push(PaginationError::AdjustablePageSizeOnPageNumber),
            // The client can only override a query parameter on a URL it does
            // not own.
            PageStrategy::NextLink(_) if !matches!(size.param, InputParam::Query(_)) => {
                errors.push(PaginationError::AdjustableLinkSizeNotQuery);
            }
            _ => {}
        }
    }
}

/// Rejects empty names, paths, and values the pager could not use.
fn empty_value_errors<'a>(strategy: &'a Strategy, errors: &mut Vec<PaginationError>) {
    let mut inputs: Vec<&InputParam> = request_inputs(strategy).into_iter().map(|(_, i)| i).collect();
    let mut sources: Vec<&'a ValueSource> = strategy.total.iter().collect();
    let mut item_paths: Vec<&JsonPath> = Vec::new();
    let mut texts: Vec<(&'static str, &'a str)> = Vec::new();

    let link = |link: &'a LinkSource, sources: &mut Vec<&'a ValueSource>, texts: &mut Vec<(&'static str, &'a str)>| {
        match link {
            LinkSource::LinkHeader { rel } => texts.push(("link relation", rel.as_str())),
            LinkSource::Field { source } => sources.push(source),
        }
    };
    match &strategy.kind {
        PageStrategy::Cursor(_) => {
            for leg in strategy.kind.legs() {
                for part in &leg.parts {
                    match &part.source {
                        CursorSource::Field { source } => sources.push(source),
                        CursorSource::LastItem { path } | CursorSource::FirstItem { path } => {
                            item_paths.push(path);
                        }
                    }
                }
                if let Boundary::Inclusive { identity } = &leg.boundary {
                    item_paths.push(identity);
                }
            }
        }
        PageStrategy::NextLink(next_link) => {
            link(&next_link.next, &mut sources, &mut texts);
            if let Some(prev) = &next_link.prev {
                link(prev, &mut sources, &mut texts);
            }
        }
        _ => {}
    }
    match strategy.kind.more() {
        Some(
            MoreSignal::Flag { source, .. }
            | MoreSignal::Present { source }
            | MoreSignal::Total { source, .. },
        ) => sources.push(source),
        Some(MoreSignal::LinkRel { rel }) => texts.push(("link relation", rel.as_str())),
        _ => {}
    }
    if let Some(follow_up) = &strategy.follow_up {
        if let Some(path) = &follow_up.path {
            texts.push(("follow-up path", path.as_str()));
        }
        if let Carry::Only(names) | Carry::AllExcept(names) = &follow_up.carry {
            texts.extend(names.iter().map(|name| ("carried name", name.as_str())));
        }
    }
    if let Some(session) = &strategy.session {
        let mut steps: Vec<&SessionStep> = Vec::new();
        if let SessionOpen::Request { step, handle } = &session.open {
            steps.push(step);
            sources.push(handle);
        }
        steps.extend(session.renew.iter().chain(session.close.iter()));
        for step in steps {
            texts.push(("session step path", step.path.as_str()));
            inputs.extend(step.handle.iter());
            inputs.extend(step.constants.iter().map(|fixed| &fixed.param));
        }
        if let Some((_, path)) = &session.pages {
            texts.push(("session page path", path.as_str()));
        }
        sources.extend(session.handle_from_page.iter());
        if let Some(keep_alive) = &session.keep_alive {
            texts.push(("keep-alive duration", keep_alive.default.as_str()));
            if let SessionOpen::Request { .. } = session.open {
                inputs.extend(keep_alive.on_open.iter());
            }
        }
    }

    if inputs.iter().any(|input| input.is_empty_destination()) {
        errors.push(PaginationError::EmptyValue("an input name or body path"));
    }
    if sources.iter().any(|source| source.is_empty_location()) {
        errors.push(PaginationError::EmptyValue("a response location"));
    }
    if item_paths.iter().any(|path| path.has_empty_segment()) {
        errors.push(PaginationError::EmptyValue("an item path segment"));
    }
    for (what, text) in texts {
        if text.is_empty() {
            errors.push(PaginationError::EmptyValue(what));
        }
    }
}

fn session_errors(session: &Session, kind: &PageStrategy) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let opened_with_handle = matches!(session.open, SessionOpen::Request { .. });
    if !opened_with_handle && session.handle_from_page.is_none() {
        errors.push(PaginationError::SessionHandleUnreachable);
    }
    if session.send_handle_as.is_none() {
        // Acceptable only if the continuation itself carries the handle.
        let carried_by_cursor = matches!(kind, PageStrategy::Cursor(cursor)
            if cursor.forward.parts.iter().any(|part| matches!(&part.source,
                CursorSource::Field { source } if Some(source) == session.handle_from_page.as_ref())));
        let carried_by_link = matches!(kind, PageStrategy::NextLink(_));
        if !(carried_by_cursor || carried_by_link) {
            errors.push(PaginationError::SessionHandleNotSent);
        }
    }
    for (name, step) in [("renew", &session.renew), ("close", &session.close)] {
        if let Some(step) = step
            && step.handle.is_none()
        {
            errors.push(PaginationError::SessionStepNeedsHandle(name.to_owned()));
        }
    }
    if let Some(keep_alive) = &session.keep_alive
        && keep_alive.on_open.is_none()
        && keep_alive.on_page.is_none()
    {
        errors.push(PaginationError::KeepAliveWithoutInput);
    }
    errors
}

/// Whether a request with `method` carries no body.
fn is_bodyless(method: RestMethod) -> bool {
    matches!(method, RestMethod::Get | RestMethod::Head)
}

/// The rules of a strategy that depend on its endpoint. Declared-parameter
/// rules apply to the primary strategy only: alternates are not generated, so
/// they remove no request field.
fn endpoint_errors(endpoint: &Endpoint, strategy: &Strategy, primary: bool) -> Vec<PaginationError> {
    let mut errors = Vec::new();
    let known_path = path_params(&endpoint.path);
    let check_path = |path: &str, errors: &mut Vec<PaginationError>| {
        for name in path_params(path) {
            if !known_path.contains(&name) {
                errors.push(PaginationError::UnknownPathParam(name.to_owned()));
            }
        }
    };
    let check_body = |input: &InputParam, method: RestMethod, errors: &mut Vec<PaginationError>| {
        if input.is_body() && is_bodyless(method) {
            errors.push(PaginationError::BodyInputOnGet(input.display_name()));
        }
    };

    let is_next_link = matches!(strategy.kind, PageStrategy::NextLink(_));
    let stated_follow_up_method = strategy.follow_up.as_ref().and_then(|f| f.method);
    let follow_up_method = stated_follow_up_method.unwrap_or(if is_next_link {
        RestMethod::Get
    } else {
        endpoint.method
    });
    let session_pages = strategy.session.as_ref().and_then(|s| s.pages.as_ref());
    let first_method = session_pages.map_or(endpoint.method, |(method, _)| *method);

    if is_next_link && endpoint.method != RestMethod::Get && stated_follow_up_method.is_none() {
        errors.push(PaginationError::NextLinkMethodUnspecified);
    }
    if let Some(path) = strategy.follow_up.as_ref().and_then(|f| f.path.as_deref()) {
        check_path(path, &mut errors);
    }

    // First-request inputs.
    let mut first_inputs: Vec<&InputParam> = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber(kind) => first_inputs.push(&kind.page),
        PageStrategy::Offset(kind) => first_inputs.push(&kind.offset),
        _ => {}
    }
    first_inputs.extend(strategy.kind.size().map(|size| &size.param));
    first_inputs.extend(strategy.constants.iter().map(|fixed| &fixed.param));
    if let Some(partition) = &strategy.partition {
        first_inputs.extend([&partition.index, &partition.count]);
    }
    for input in &first_inputs {
        check_body(input, first_method, &mut errors);
    }

    // Cursor parts go only on follow-up requests.
    for leg in strategy.kind.legs() {
        for part in &leg.parts {
            check_body(&part.param, follow_up_method, &mut errors);
        }
    }

    if let Some(session) = &strategy.session {
        let mut steps: Vec<&SessionStep> = Vec::new();
        if let SessionOpen::Request { step, .. } = &session.open {
            steps.push(step);
        }
        steps.extend(session.renew.iter());
        steps.extend(session.close.iter());
        for step in steps {
            check_path(&step.path, &mut errors);
            for input in step.handle.iter().chain(step.constants.iter().map(|fixed| &fixed.param)) {
                check_body(input, step.method, &mut errors);
            }
        }
        if let Some((_, path)) = &session.pages {
            check_path(path, &mut errors);
        }
        if let Some(handle) = &session.send_handle_as {
            check_body(handle, first_method, &mut errors);
        }
        if let Some(keep_alive) = &session.keep_alive {
            let open_method = match &session.open {
                SessionOpen::Request { step, .. } => step.method,
                SessionOpen::FirstPage => endpoint.method,
            };
            if let Some(input) = &keep_alive.on_open {
                check_body(input, open_method, &mut errors);
            }
            if let Some(input) = &keep_alive.on_page {
                check_body(input, follow_up_method, &mut errors);
            }
        }
    }

    // Carried names must be ordinary query parameters or the page size. Body
    // field names cannot be checked (no body schema), so only body-less
    // endpoints are checked.
    if is_bodyless(endpoint.method)
        && let Carry::Only(names) | Carry::AllExcept(names) = strategy.effective_carry()
    {
        let size_name = strategy.kind.size().map(|size| size.param.display_name());
        let declared_query = endpoint
            .params
            .as_ref()
            .map(|params| params.query.as_slice())
            .unwrap_or_default();
        for name in names {
            let known = declared_query.iter().any(|param| param.name == name)
                || size_name.as_deref() == Some(name.as_str());
            if !known && !name.is_empty() {
                errors.push(PaginationError::UnknownCarryParam(name));
            }
        }
    }

    if primary {
        for adoption in param_adoptions(endpoint, strategy) {
            match adoption.outcome {
                AdoptionOutcome::NearMiss | AdoptionOutcome::NotAdoptable => {
                    errors.push(PaginationError::CollidesWithEndpointParam(adoption.name));
                }
                AdoptionOutcome::WithoutKnob => {
                    errors.push(PaginationError::AdoptionWithoutKnob(adoption.name));
                }
                AdoptionOutcome::Adopted if adoption.required && !adoption.has_sent_default => {
                    errors.push(PaginationError::PagerManagedRequiredWithoutDefault {
                        strategy: strategy.kind.kind_name(),
                        param: adoption.name,
                    });
                }
                AdoptionOutcome::Adopted if adoption.required => {
                    let declared = declared_param(endpoint, adoption.location, &adoption.name);
                    if let (Some(value), Some(declared)) = (adoption.role.sent_default(strategy), declared)
                        && let Some(expected) = invalid_sent_default(&declared.param_type, &value)
                    {
                        errors.push(PaginationError::PagerManagedDefaultInvalid {
                            strategy: strategy.kind.kind_name(),
                            param: adoption.name,
                            value,
                            expected,
                        });
                    }
                }
                AdoptionOutcome::Adopted => {}
            }
        }
    }
    errors
}

/// The declared plain parameter an adoption names, matched the way
/// [`param_adoptions`] matched it.
fn declared_param<'a>(endpoint: &'a Endpoint, location: ParamLocation, name: &str) -> Option<&'a ParamDef> {
    let params = endpoint.params.as_ref()?;
    let (defs, any_case) = match location {
        ParamLocation::Query => (&params.query, false),
        ParamLocation::Header => (&params.header, true),
        ParamLocation::Cookie => (&params.cookie, false),
        ParamLocation::Path => return None,
    };
    defs.iter().find(|def| if any_case { def.name.eq_ignore_ascii_case(name) } else { def.name == name })
}

/// The type a sent default fails to be, or `None` when `value` is valid for
/// `param_type`. Types without a parse rule only need a nonempty value.
fn invalid_sent_default(param_type: &QueryParamType, value: &str) -> Option<&'static str> {
    let (valid, expected) = match param_type {
        QueryParamType::Integer => (value.parse::<i64>().is_ok(), "integer"),
        QueryParamType::Number => (value.parse::<f64>().is_ok_and(f64::is_finite), "finite number"),
        QueryParamType::Boolean => (matches!(value, "true" | "false"), "boolean (`true` or `false`)"),
        QueryParamType::Enum(values) => (values.iter().any(|allowed| allowed == value), "value of the declared enum"),
        _ => (!value.is_empty(), "nonempty value"),
    };
    (!valid).then_some(expected)
}
