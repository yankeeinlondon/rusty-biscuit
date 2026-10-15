//! Pager emission for list endpoints whose definition authors a pagination
//! strategy.
//!
//! An endpoint is paginated when `schematic_define::pagination::resolve`
//! gives [`Resolution::Paginated`]. For each one the generator emits
//! `impl schematic_runtime::pager::Paginated` on its request type, with the
//! strategy and item selector as `&'static` constants, and one capability
//! marker impl per authored knob. Kind-only and session capabilities are the
//! runtime's blanket impls over the `Kind` and `Session` associated types, so
//! nothing is emitted for them. Unpaginated lists, non-lists, and alternates
//! emit nothing.
//!
//! The parameters the strategy adopts (`adopted_params`) leave the request:
//! [`pager_view`] is the one place that removes them, and request structs,
//! call-type setters, their docs, and name validation all read it.

#[cfg(test)]
mod tests;

use std::borrow::Cow;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use schematic_define::pagination::{
    Boundary, Carry, CountUnit, CursorLeg, CursorPart, CursorSource, Direction, FixedParam,
    FollowUp, IndexBase, InputParam, ItemSelector, KeepAlive, Knob, LinkSource, MoreSignal,
    PageSize, PageStrategy, ParamLocation, Partition, Polarity, Resolution, Session, SessionOpen, SessionStep,
    Step, Strategy, ValueSource, adopted_params, caller_knobs, resolve,
};
use schematic_define::{Endpoint, RestApi, RestMethod};

use crate::codegen::request_structs::endpoint_spec::method_tokens;
use crate::codegen::request_structs::shared::type_name_to_tokens;

/// `endpoint` as the Rust client sees it: without the query and header
/// parameters its pagination strategy adopts. Borrowed when nothing is
/// adopted.
pub fn pager_view<'a>(api: &RestApi, endpoint: &'a Endpoint) -> Cow<'a, Endpoint> {
    let Resolution::Paginated { strategy, .. } = resolve(api, endpoint) else {
        return Cow::Borrowed(endpoint);
    };
    let adopted = adopted_params(endpoint, &strategy);
    if adopted.is_empty() {
        return Cow::Borrowed(endpoint);
    }
    let kept = |location: ParamLocation, name: &str| {
        !adopted
            .iter()
            .any(|adoption| adoption.location == location && adoption.name == name)
    };
    let mut endpoint = endpoint.clone();
    if let Some(params) = &mut endpoint.params {
        params.query.retain(|def| kept(ParamLocation::Query, &def.name));
        params.header.retain(|def| kept(ParamLocation::Header, &def.name));
    }
    Cow::Owned(endpoint)
}

/// An adopted parameter that an ordinary send of its endpoint still sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryDefault {
    /// `Query` or `Header`; adoption covers no other location.
    pub location: ParamLocation,
    /// The name as the endpoint declares it.
    pub name: String,
    pub value: String,
}

/// The adopted parameters that `endpoint` declares required, with the value
/// the pager sends for each when the caller sets none.
///
/// An adopted parameter is not a request field, so an ordinary send would
/// otherwise omit a parameter the provider requires; it sends this value
/// instead, and the pager writes its own value over it. Validation rejects a
/// required adopted parameter without such a value, so none is skipped here.
pub fn ordinary_send_defaults(api: &RestApi, endpoint: &Endpoint) -> Vec<OrdinaryDefault> {
    let Resolution::Paginated { strategy, .. } = resolve(api, endpoint) else {
        return Vec::new();
    };
    adopted_params(endpoint, &strategy)
        .into_iter()
        .filter(|adoption| adoption.required)
        .filter_map(|adoption| {
            Some(OrdinaryDefault {
                value: adoption.role.sent_default(&strategy)?,
                location: adoption.location,
                name: adoption.name,
            })
        })
        .collect()
}

/// `api` with [`pager_view`] applied to every endpoint.
pub fn api_pager_view(api: &RestApi) -> Cow<'_, RestApi> {
    let views: Vec<Cow<'_, Endpoint>> =
        api.endpoints.iter().map(|endpoint| pager_view(api, endpoint)).collect();
    if views.iter().all(|view| matches!(view, Cow::Borrowed(_))) {
        return Cow::Borrowed(api);
    }
    let endpoints = views.into_iter().map(Cow::into_owned).collect();
    Cow::Owned(RestApi {
        endpoints,
        ..api.clone()
    })
}

/// A pager control the call type forwards, in emission order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnobForward {
    /// `.page(number)`, for a page-number strategy.
    Page,
    /// `.offset(offset)`, for an offset strategy.
    Offset,
    /// `.after(cursor)`, for a resumable cursor.
    After,
    /// `.before(cursor)`, for a resumable cursor with a backward leg.
    Before,
    /// `.page_size(size)`, for an authored page size.
    PageSize,
    /// `.keep_alive(value)`, for an adjustable session keep-alive.
    KeepAlive,
}

impl KnobForward {
    /// The method name on the call type and on `Pager`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Offset => "offset",
            Self::After => "after",
            Self::Before => "before",
            Self::PageSize => "page_size",
            Self::KeepAlive => "keep_alive",
        }
    }
}

/// The pager methods a paginated endpoint's call type gets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagerForwards {
    /// One per authored setter-like knob.
    pub knobs: Vec<KnobForward>,
    /// Whether `pages`, `fetch_page`, and `collect_all` are forwarded: only
    /// a strategy without a session pages outside a scope. A session
    /// strategy forwards `with_session` and `close_grace` instead.
    pub sessionless: bool,
    /// Whether `partitioned` is forwarded: an authored partition without a
    /// session (a session's partitions are made inside its scope).
    pub partitioned: bool,
}

impl PagerForwards {
    /// Whether the call type gets a pager method named `name` beyond the
    /// entry points every call type reserves.
    pub fn has_control(&self, name: &str) -> bool {
        self.knobs.iter().any(|knob| knob.name() == name)
            || (self.partitioned && name == "partitioned")
            || (!self.sessionless && name == "close_grace")
    }
}

/// The pager forwards of `endpoint`, or `None` when it is not paginated.
pub fn pager_forwards(api: &RestApi, endpoint: &Endpoint) -> Option<PagerForwards> {
    let resolution = resolve(api, endpoint);
    let strategy = resolution.strategy()?;
    let knobs = caller_knobs(&resolution)
        .iter()
        .filter_map(|knob| match knob {
            Knob::StartPage { .. } => Some(KnobForward::Page),
            Knob::StartOffset { .. } => Some(KnobForward::Offset),
            Knob::ResumeCursor {
                direction: Direction::Forward,
                ..
            } => Some(KnobForward::After),
            Knob::ResumeCursor {
                direction: Direction::Backward,
                ..
            } => Some(KnobForward::Before),
            Knob::PageSize { .. } => Some(KnobForward::PageSize),
            Knob::KeepAlive { .. } => Some(KnobForward::KeepAlive),
            _ => None,
        })
        .collect();
    Some(PagerForwards {
        knobs,
        sessionless: strategy.session.is_none(),
        partitioned: strategy.session.is_none() && strategy.partition.is_some(),
    })
}

/// The `Paginated` and capability marker impls of every paginated endpoint
/// of `api`, in endpoint order.
pub fn generate_pagination_impls(api: &RestApi, suffix: &str) -> TokenStream {
    api.endpoints
        .iter()
        .map(|endpoint| paginated_impl(api, endpoint, suffix))
        .collect()
}

fn paginated_impl(api: &RestApi, endpoint: &Endpoint, suffix: &str) -> TokenStream {
    let resolution = resolve(api, endpoint);
    let Resolution::Paginated { list, strategy, .. } = &resolution else {
        return TokenStream::new();
    };
    let request = format_ident!("{}{}", endpoint.id, suffix);
    let item = match &list.item_type {
        Some(item_type) => type_name_to_tokens(item_type),
        // Validation refuses a paginated list without an item type.
        None => unsupported(&format!("the paginated list of `{}` names no item type", endpoint.id)),
    };
    let kind = format_ident!(
        "{}",
        match &strategy.kind {
            PageStrategy::PageNumber(_) => "PageNumberKind",
            PageStrategy::Offset(_) => "OffsetKind",
            PageStrategy::Cursor(_) => "CursorKind",
            PageStrategy::NextLink(_) => "NextLinkKind",
            _ => "UnsupportedKind",
        }
    );
    let session = if strategy.session.is_some() {
        quote! { WithSession }
    } else {
        quote! { NoSession }
    };
    let strategy_value = strategy_tokens(strategy);
    let items = item_selector(&list.items);
    let path = &endpoint.path;
    let markers = caller_knobs(&resolution)
        .into_iter()
        .filter_map(|knob| marker_impl(&knob, &request));

    quote! {
        impl schematic_runtime::pager::Paginated for #request {
            type Item = #item;
            type Kind = schematic_runtime::pager::#kind;
            type Session = schematic_runtime::pager::#session;

            const STRATEGY: &'static schematic_runtime::pager::Strategy = &#strategy_value;
            const ITEMS: &'static schematic_runtime::pager::ItemSelector = &#items;
            const PATH: &'static str = #path;
        }

        #(#markers)*
    }
}

/// The marker impl of one knob; `None` for the knobs a blanket impl or an
/// associated type already covers.
fn marker_impl(knob: &Knob, request: &proc_macro2::Ident) -> Option<TokenStream> {
    let marker = |name: &str| {
        let name = format_ident!("{name}");
        quote! { impl schematic_runtime::pager::#name for #request {} }
    };
    Some(match knob {
        Knob::ResumeCursor { direction, parts } => {
            let name = format_ident!(
                "{}",
                match direction {
                    Direction::Forward => "ResumeAfter",
                    Direction::Backward => "ResumeBefore",
                }
            );
            let cursor = cursor_type(*parts);
            quote! {
                impl schematic_runtime::pager::#name for #request {
                    type Cursor = #cursor;
                }
            }
        }
        Knob::PageSize { .. } => marker("PageSizeKnob"),
        Knob::AdjustPageSize => marker("AdjustablePageSize"),
        Knob::KeepAlive { .. } => marker("KeepAliveKnob"),
        Knob::Partitions { .. } => marker("Partitioned"),
        Knob::Prev => marker("HasPrev"),
        Knob::Total => marker("HasTotal"),
        Knob::Renew => marker("Renewable"),
        // `StartPage`, `StartOffset`, `Sessionless`, and `Sessioned` are
        // blanket impls over `Kind` and `Session`; every pager has a next.
        _ => return None,
    })
}

/// The caller's cursor value: one `String` per part, as a tuple when
/// composite. The runtime implements `CursorValue` for up to four parts.
fn cursor_type(parts: usize) -> TokenStream {
    match parts {
        1 => quote! { String },
        2 => quote! { (String, String) },
        3 => quote! { (String, String, String) },
        4 => quote! { (String, String, String, String) },
        other => unsupported(&format!("a cursor of {other} parts is not supported")),
    }
}

/// A `compile_error!` that fails the generated crate at the unsupported
/// construct rather than aborting generation.
fn unsupported(message: &str) -> TokenStream {
    let message = format!("schematic-gen cannot emit pagination: {message}");
    quote! { ::core::compile_error!(#message) }
}

// ---------------------------------------------------------------------------
// The `&'static` projection (`schematic_runtime::pager::spec`)
// ---------------------------------------------------------------------------

fn strategy_tokens(strategy: &Strategy) -> TokenStream {
    let kind = kind_tokens(&strategy.kind);
    let size = option(strategy.kind.size(), page_size);
    let constants = slice(&strategy.constants, fixed);
    let total = option(strategy.total.as_ref(), source);
    let terminal_values = &strategy.terminal_values;
    let follow_up = option(strategy.follow_up.as_ref(), follow_up);
    let session = option(strategy.session.as_ref(), session);
    let partition = option(strategy.partition.as_ref(), partition);
    quote! {
        schematic_runtime::pager::spec::Strategy {
            kind: #kind,
            size: #size,
            constants: #constants,
            total: #total,
            terminal_values: &[#(#terminal_values),*],
            follow_up: #follow_up,
            session: #session,
            partition: #partition,
        }
    }
}

fn kind_tokens(kind: &PageStrategy) -> TokenStream {
    match kind {
        PageStrategy::PageNumber(kind) => {
            let page = input(&kind.page);
            let base = index_base(kind.base);
            let more = more_signal(&kind.more);
            quote! {
                schematic_runtime::pager::spec::Kind::PageNumber { page: #page, base: #base, more: #more }
            }
        }
        PageStrategy::Offset(kind) => {
            let offset = input(&kind.offset);
            let base = index_base(kind.base);
            let more = more_signal(&kind.more);
            quote! {
                schematic_runtime::pager::spec::Kind::Offset { offset: #offset, base: #base, more: #more }
            }
        }
        PageStrategy::Cursor(kind) => {
            let forward = cursor_leg(&kind.forward);
            let backward = option(kind.backward.as_ref(), cursor_leg);
            let more = option(kind.more.as_ref(), more_signal);
            let resumable = kind.resumable;
            quote! {
                schematic_runtime::pager::spec::Kind::Cursor {
                    forward: #forward,
                    backward: #backward,
                    more: #more,
                    resumable: #resumable,
                }
            }
        }
        PageStrategy::NextLink(kind) => {
            let next = link_source(&kind.next);
            let prev = option(kind.prev.as_ref(), link_source);
            quote! {
                schematic_runtime::pager::spec::Kind::NextLink { next: #next, prev: #prev }
            }
        }
        other => unsupported(&format!("the strategy kind `{}`", other.kind_name())),
    }
}

fn input(param: &InputParam) -> TokenStream {
    match param {
        InputParam::Query(name) => quote! { schematic_runtime::pager::spec::Input::Query(#name) },
        InputParam::Header(name) => quote! { schematic_runtime::pager::spec::Input::Header(#name) },
        InputParam::Body(path) => {
            let path = keys(path.as_segments());
            quote! { schematic_runtime::pager::spec::Input::Body(#path) }
        }
        other => unsupported(&format!("the input `{}`", other.display_name())),
    }
}

fn source(source: &ValueSource) -> TokenStream {
    match source {
        ValueSource::Body(path) => {
            let path = keys(path.as_segments());
            quote! { schematic_runtime::pager::spec::Source::Body(#path) }
        }
        ValueSource::Header(name) => quote! { schematic_runtime::pager::spec::Source::Header(#name) },
        _ => unsupported("a value source of an unknown location"),
    }
}

fn fixed(fixed: &FixedParam) -> TokenStream {
    let param = input(&fixed.param);
    let value = &fixed.value;
    quote! { schematic_runtime::pager::spec::Fixed { param: #param, value: #value } }
}

fn index_base(base: IndexBase) -> TokenStream {
    match base {
        IndexBase::ZeroBased => quote! { schematic_runtime::pager::spec::IndexBase::ZeroBased },
        IndexBase::OneBased => quote! { schematic_runtime::pager::spec::IndexBase::OneBased },
    }
}

fn polarity(polarity: Polarity) -> TokenStream {
    match polarity {
        Polarity::TrueMeansMore => quote! { schematic_runtime::pager::spec::Polarity::TrueMeansMore },
        Polarity::TrueMeansLast => quote! { schematic_runtime::pager::spec::Polarity::TrueMeansLast },
    }
}

fn count_unit(unit: CountUnit) -> TokenStream {
    match unit {
        CountUnit::Items => quote! { schematic_runtime::pager::spec::CountUnit::Items },
        CountUnit::Pages => quote! { schematic_runtime::pager::spec::CountUnit::Pages },
    }
}

fn page_size(size: &PageSize) -> TokenStream {
    let param = input(&size.param);
    let server_default = option(size.server_default, u32_literal);
    let sent_default = option(size.sent_default, u32_literal);
    let min = u32_literal(size.min);
    let max = option(size.max, u32_literal);
    let adjustable = size.adjustable;
    quote! {
        schematic_runtime::pager::spec::PageSize {
            param: #param,
            server_default: #server_default,
            sent_default: #sent_default,
            min: #min,
            max: #max,
            adjustable: #adjustable,
        }
    }
}

fn more_signal(signal: &MoreSignal) -> TokenStream {
    match signal {
        MoreSignal::Flag { source: from, polarity: value } => {
            let from = source(from);
            let value = polarity(*value);
            quote! { schematic_runtime::pager::spec::MoreSignal::Flag { source: #from, polarity: #value } }
        }
        MoreSignal::Present { source: from } => {
            let from = source(from);
            quote! { schematic_runtime::pager::spec::MoreSignal::Present { source: #from } }
        }
        MoreSignal::LinkRel { rel } => {
            quote! { schematic_runtime::pager::spec::MoreSignal::LinkRel { rel: #rel } }
        }
        MoreSignal::Total { source: from, unit } => {
            let from = source(from);
            let unit = count_unit(*unit);
            quote! { schematic_runtime::pager::spec::MoreSignal::Total { source: #from, unit: #unit } }
        }
        MoreSignal::NonEmptyPage => quote! { schematic_runtime::pager::spec::MoreSignal::NonEmptyPage },
        MoreSignal::FullPage => quote! { schematic_runtime::pager::spec::MoreSignal::FullPage },
        _ => unsupported("an end signal of an unknown kind"),
    }
}

fn cursor_source(cursor: &CursorSource) -> TokenStream {
    match cursor {
        CursorSource::Field { source: from } => {
            let from = source(from);
            quote! { schematic_runtime::pager::spec::CursorSource::Field(#from) }
        }
        CursorSource::LastItem { path } => {
            let path = keys(path.as_segments());
            quote! { schematic_runtime::pager::spec::CursorSource::LastItem(#path) }
        }
        CursorSource::FirstItem { path } => {
            let path = keys(path.as_segments());
            quote! { schematic_runtime::pager::spec::CursorSource::FirstItem(#path) }
        }
        _ => unsupported("a cursor source of an unknown kind"),
    }
}

fn cursor_part(part: &CursorPart) -> TokenStream {
    let param = input(&part.param);
    let from = cursor_source(&part.source);
    quote! { schematic_runtime::pager::spec::CursorPart { param: #param, source: #from } }
}

fn cursor_leg(leg: &CursorLeg) -> TokenStream {
    let parts = slice(&leg.parts, cursor_part);
    let boundary = match &leg.boundary {
        Boundary::Exclusive => quote! { schematic_runtime::pager::spec::Boundary::Exclusive },
        Boundary::Inclusive { identity } => {
            let identity = keys(identity.as_segments());
            quote! { schematic_runtime::pager::spec::Boundary::Inclusive { identity: #identity } }
        }
    };
    quote! { schematic_runtime::pager::spec::CursorLeg { parts: #parts, boundary: #boundary } }
}

fn link_source(link: &LinkSource) -> TokenStream {
    match link {
        LinkSource::LinkHeader { rel } => {
            quote! { schematic_runtime::pager::spec::LinkSource::LinkHeader { rel: #rel } }
        }
        LinkSource::Field { source: from } => {
            let from = source(from);
            quote! { schematic_runtime::pager::spec::LinkSource::Field(#from) }
        }
        _ => unsupported("a link source of an unknown kind"),
    }
}

fn carry(carry: &Carry) -> TokenStream {
    match carry {
        Carry::All => quote! { schematic_runtime::pager::spec::Carry::All },
        Carry::Only(names) => quote! { schematic_runtime::pager::spec::Carry::Only(&[#(#names),*]) },
        Carry::AllExcept(names) => {
            quote! { schematic_runtime::pager::spec::Carry::AllExcept(&[#(#names),*]) }
        }
        Carry::Nothing => quote! { schematic_runtime::pager::spec::Carry::Nothing },
    }
}

fn follow_up(follow_up: &FollowUp) -> TokenStream {
    let method = option(follow_up.method.as_ref(), method_tokens);
    let path = option(follow_up.path.as_ref(), |path| quote! { #path });
    let carry = carry(&follow_up.carry);
    quote! {
        schematic_runtime::pager::spec::FollowUp { method: #method, path: #path, carry: #carry }
    }
}

fn session_step(step: &SessionStep) -> TokenStream {
    let method = method_tokens(&step.method);
    let path = &step.path;
    let handle = option(step.handle.as_ref(), input);
    let constants = slice(&step.constants, fixed);
    quote! {
        schematic_runtime::pager::spec::SessionStep {
            method: #method,
            path: #path,
            handle: #handle,
            constants: #constants,
        }
    }
}

fn keep_alive(keep_alive: &KeepAlive) -> TokenStream {
    let default = &keep_alive.default;
    let on_open = option(keep_alive.on_open.as_ref(), input);
    let on_page = option(keep_alive.on_page.as_ref(), input);
    let adjustable = keep_alive.adjustable;
    quote! {
        schematic_runtime::pager::spec::KeepAlive {
            default: #default,
            on_open: #on_open,
            on_page: #on_page,
            adjustable: #adjustable,
        }
    }
}

fn session(session: &Session) -> TokenStream {
    let open = match &session.open {
        SessionOpen::FirstPage => quote! { schematic_runtime::pager::spec::SessionOpen::FirstPage },
        SessionOpen::Request { step, handle } => {
            let step = session_step(step);
            let handle = source(handle);
            quote! { schematic_runtime::pager::spec::SessionOpen::Request { step: #step, handle: #handle } }
        }
    };
    let pages = option(session.pages.as_ref(), |(method, path): &(RestMethod, String)| {
        let method = method_tokens(method);
        quote! { (#method, #path) }
    });
    let handle_from_page = option(session.handle_from_page.as_ref(), source);
    let send_handle_as = option(session.send_handle_as.as_ref(), input);
    let keep_alive = option(session.keep_alive.as_ref(), keep_alive);
    let renew = option(session.renew.as_ref(), session_step);
    let close = option(session.close.as_ref(), session_step);
    quote! {
        schematic_runtime::pager::spec::Session {
            open: #open,
            pages: #pages,
            handle_from_page: #handle_from_page,
            send_handle_as: #send_handle_as,
            keep_alive: #keep_alive,
            renew: #renew,
            close: #close,
        }
    }
}

fn partition(partition: &Partition) -> TokenStream {
    let index = input(&partition.index);
    let count = input(&partition.count);
    let max = option(partition.max, u32_literal);
    quote! {
        schematic_runtime::pager::spec::Partition { index: #index, count: #count, max: #max }
    }
}

fn item_selector(selector: &ItemSelector) -> TokenStream {
    let record = selector.record.iter().map(|step| match step {
        Step::Key(key) => quote! { schematic_runtime::pager::spec::Step::Key(#key) },
        Step::Each => quote! { schematic_runtime::pager::spec::Step::Each },
    });
    let projection = keys(selector.projection.as_segments());
    quote! {
        schematic_runtime::pager::spec::ItemSelector { record: &[#(#record),*], projection: #projection }
    }
}

/// A JSON path as the runtime's `&[&str]` of object keys.
fn keys(segments: &[String]) -> TokenStream {
    quote! { &[#(#segments),*] }
}

fn u32_literal(value: u32) -> TokenStream {
    let literal = Literal::u32_unsuffixed(value);
    quote! { #literal }
}

fn option<T>(value: Option<T>, tokens: impl FnOnce(T) -> TokenStream) -> TokenStream {
    match value {
        Some(value) => {
            let value = tokens(value);
            quote! { Some(#value) }
        }
        None => quote! { None },
    }
}

fn slice<T>(values: &[T], tokens: impl Fn(&T) -> TokenStream) -> TokenStream {
    let values = values.iter().map(tokens);
    quote! { &[#(#values),*] }
}
