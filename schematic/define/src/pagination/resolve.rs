//! Resolution of an endpoint's pagination, its caller knobs, and the plain
//! parameters a strategy adopts.
//!
//! These are the contracts the generator consumes: [`resolve`] decides whether
//! an endpoint is a list and how it pages, [`caller_knobs`] lists the controls
//! the generated client exposes, and [`param_adoptions`] decides which declared
//! parameters become pager-managed (and so leave the request struct).

use super::collection::{Alternate, Pagination};
use super::selector::ItemSelector;
use super::strategy::{IndexBase, InputParam, MoreSignal, PageStrategy, SessionOpen, Strategy};
use crate::params::ParamDef;
use crate::response::ApiResponse;
use crate::types::{Endpoint, RestApi, RestMethod};

/// What decided an endpoint's pagination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResolutionOrigin {
    /// A row of the API's exceptions table (an override or an opt-out).
    Exception,
    /// The API's default strategy, inherited by a list.
    ApiDefault,
    /// Nothing authored applies: a list with no default and no row.
    Unauthored,
}

/// Where a list's items are and what type they decode to.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedList {
    pub items: ItemSelector,
    /// `None` only when nothing names it: a wrapper or non-`Vec` root without
    /// an authored item type, which validation rejects for a paginated list.
    pub item_type: Option<String>,
}

/// The outcome of [`resolve`] for one endpoint.
// Built once per definition at generation time; boxing would only make
// authoring and matching clumsier.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Resolution {
    /// Not a list: no row, and not a GET with an explicit `Vec<T>` response.
    NotAList,
    /// A list without pagination.
    Unpaginated {
        list: ResolvedList,
        origin: ResolutionOrigin,
    },
    /// A paginated list. `alternates` are modeled only; nothing is generated
    /// for them.
    Paginated {
        list: ResolvedList,
        strategy: Strategy,
        alternates: Vec<Alternate>,
        origin: ResolutionOrigin,
    },
}

impl Resolution {
    /// The primary strategy of a paginated list.
    pub fn strategy(&self) -> Option<&Strategy> {
        match self {
            Self::Paginated { strategy, .. } => Some(strategy),
            _ => None,
        }
    }

    /// The origin of a list's resolution; `None` for a non-list.
    pub fn origin(&self) -> Option<ResolutionOrigin> {
        match self {
            Self::NotAList => None,
            Self::Unpaginated { origin, .. } | Self::Paginated { origin, .. } => Some(*origin),
        }
    }

    pub fn list(&self) -> Option<&ResolvedList> {
        match self {
            Self::NotAList => None,
            Self::Unpaginated { list, .. } | Self::Paginated { list, .. } => Some(list),
        }
    }
}

/// Resolves `endpoint`'s pagination within `api`.
///
/// The order is fixed: (1) a row of `api.pagination_exceptions` for the
/// endpoint id decides; (2) otherwise a GET endpoint whose response is an
/// explicit `Vec<T>` is a root-array list that inherits the API default, or is
/// unpaginated without one; (3) otherwise the endpoint is not a list. Type
/// aliases and opaque responses are never inferred to be lists. The first row
/// for an id wins; validation rejects a duplicate.
///
/// ## Examples
///
/// ```
/// use schematic_define::pagination::{resolve, MoreSignal, Resolution, ResolutionOrigin, Strategy};
/// use schematic_define::{ApiResponse, Endpoint, RestMethod};
/// # use schematic_define::{AuthStrategy, CrossOrigin, RestApi};
/// # let mut api = RestApi {
/// #     name: "Forge".into(), description: String::new(), base_url: "https://forge.test".into(),
/// #     docs_url: None, auth: AuthStrategy::None, auth_policy: None, env_auth: vec![],
/// #     env_username: None, headers: vec![], endpoints: vec![], module_path: None,
/// #     request_suffix: None, version: None, env_mapping: None,
/// #     cross_origin: CrossOrigin::default(), request_id_header: None, secret_names: vec![],
/// #     default_pagination: None, pagination_exceptions: vec![],
/// # };
///
/// api.default_pagination = Some(Strategy::page_number("page", MoreSignal::NonEmptyPage).build());
/// let releases = Endpoint {
///     id: "ListReleases".into(),
///     method: RestMethod::Get,
///     path: "/releases".into(),
///     description: String::new(),
///     request: None,
///     response: ApiResponse::json_type("Vec<Release>"),
///     headers: vec![],
///     params: None,
///     oauth_scopes: None, cross_origin: None,
/// };
///
/// let resolution = resolve(&api, &releases);
/// assert_eq!(resolution.origin(), Some(ResolutionOrigin::ApiDefault));
/// assert_eq!(resolution.list().unwrap().item_type.as_deref(), Some("Release"));
/// ```
pub fn resolve(api: &RestApi, endpoint: &Endpoint) -> Resolution {
    let entry = api
        .pagination_exceptions
        .iter()
        .find(|entry| entry.endpoint == endpoint.id);

    let Some(entry) = entry else {
        if endpoint.method != RestMethod::Get {
            return Resolution::NotAList;
        }
        let Some(item_type) = root_array_item_type(&endpoint.response) else {
            return Resolution::NotAList;
        };
        let list = ResolvedList {
            items: ItemSelector::root(),
            item_type: Some(item_type),
        };
        return match &api.default_pagination {
            Some(strategy) => Resolution::Paginated {
                list,
                strategy: strategy.clone(),
                alternates: Vec::new(),
                origin: ResolutionOrigin::ApiDefault,
            },
            None => Resolution::Unpaginated {
                list,
                origin: ResolutionOrigin::Unauthored,
            },
        };
    };

    let collection = &entry.collection;
    let item_type = collection.item_type.clone().or_else(|| {
        collection
            .items
            .is_root_array()
            .then(|| root_array_item_type(&endpoint.response))
            .flatten()
    });
    let list = ResolvedList {
        items: collection.items.clone(),
        item_type,
    };
    match (&collection.pagination, &api.default_pagination) {
        (Pagination::Explicit(strategy), _) => Resolution::Paginated {
            list,
            strategy: strategy.clone(),
            alternates: collection.alternates.clone(),
            origin: ResolutionOrigin::Exception,
        },
        (Pagination::Unpaginated, _) => Resolution::Unpaginated {
            list,
            origin: ResolutionOrigin::Exception,
        },
        (Pagination::Inherit, Some(strategy)) => Resolution::Paginated {
            list,
            strategy: strategy.clone(),
            alternates: collection.alternates.clone(),
            origin: ResolutionOrigin::ApiDefault,
        },
        (Pagination::Inherit, None) => Resolution::Unpaginated {
            list,
            origin: ResolutionOrigin::Unauthored,
        },
    }
}

/// The `T` of an explicit `Vec<T>` JSON response, as written.
///
/// Typed authoring recognizes the descriptor's root Vec shape. Legacy strings
/// recognize only `Vec<..>`; aliases, qualified strings, and free-form JSON
/// still need an authored collection.
pub fn root_array_item_type(response: &ApiResponse) -> Option<String> {
    let ApiResponse::Json(schema) = response else {
        return None;
    };
    if let Some(desc) = &schema.descriptor { return desc.root_array_item(); }
    let inner = schema
        .type_name
        .trim()
        .strip_prefix("Vec<")?
        .strip_suffix('>')?
        .trim();
    (!inner.is_empty()).then(|| inner.to_owned())
}

// ---------------------------------------------------------------------------
// Caller knobs
// ---------------------------------------------------------------------------

/// A cursor direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Direction {
    Forward,
    Backward,
}

/// One control the generated client exposes for a paginated endpoint.
///
/// An endpoint has exactly the knobs its strategy authors; any other control
/// does not compile on its generated call.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Knob {
    /// `.page(n)` before iterating.
    StartPage { base: IndexBase },
    /// `.offset(n)` before iterating.
    StartOffset { base: IndexBase },
    /// `.after(..)` / `.before(..)`; `parts` > 1 means a tuple argument.
    ResumeCursor { direction: Direction, parts: usize },
    /// `.page_size(n)` before iterating.
    PageSize {
        param: String,
        min: u32,
        max: Option<u32>,
        server_default: Option<u32>,
    },
    /// `pages.set_page_size(n)` between pages.
    AdjustPageSize,
    /// `.keep_alive(duration)`.
    KeepAlive { default: String },
    /// `.partitioned(n)`.
    Partitions { max: Option<u32> },
    /// `next()` on the page iterator.
    Next,
    /// `prev()` on the page iterator.
    Prev,
    /// `total()`.
    Total,
    /// `renew()` inside the session scope.
    Renew,
    /// `with_session(..)`, the only way to iterate a session endpoint.
    Session,
}

/// The knobs a strategy authors, in a stable order.
pub fn strategy_knobs(strategy: &Strategy) -> Vec<Knob> {
    let mut knobs = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber(kind) => knobs.push(Knob::StartPage { base: kind.base }),
        PageStrategy::Offset(kind) => knobs.push(Knob::StartOffset { base: kind.base }),
        PageStrategy::Cursor(kind) if kind.resumable => {
            knobs.push(Knob::ResumeCursor {
                direction: Direction::Forward,
                parts: kind.forward.parts.len(),
            });
            if let Some(leg) = &kind.backward {
                knobs.push(Knob::ResumeCursor {
                    direction: Direction::Backward,
                    parts: leg.parts.len(),
                });
            }
        }
        _ => {}
    }
    if let Some(size) = strategy.kind.size() {
        knobs.push(Knob::PageSize {
            param: size.param.display_name(),
            min: size.min,
            max: size.max,
            server_default: size.server_default,
        });
        if size.adjustable {
            knobs.push(Knob::AdjustPageSize);
        }
    }
    if let Some(keep_alive) = strategy
        .session
        .as_ref()
        .and_then(|session| session.keep_alive.as_ref())
        && keep_alive.adjustable
    {
        knobs.push(Knob::KeepAlive {
            default: keep_alive.default.clone(),
        });
    }
    if let Some(partition) = &strategy.partition {
        knobs.push(Knob::Partitions { max: partition.max });
    }
    knobs.push(Knob::Next);
    let has_prev = match &strategy.kind {
        PageStrategy::Cursor(kind) => kind.backward.is_some(),
        PageStrategy::NextLink(kind) => kind.prev.is_some(),
        _ => false,
    };
    if has_prev {
        knobs.push(Knob::Prev);
    }
    if strategy.total.is_some() || matches!(strategy.kind.more(), Some(MoreSignal::Total { .. })) {
        knobs.push(Knob::Total);
    }
    if let Some(session) = &strategy.session {
        if session.renew.is_some() {
            knobs.push(Knob::Renew);
        }
        knobs.push(Knob::Session);
    }
    knobs
}

/// The caller knobs of a resolution: the primary strategy's, and none for an
/// unpaginated list or a non-list. Alternates add nothing.
pub fn caller_knobs(resolution: &Resolution) -> Vec<Knob> {
    resolution.strategy().map(strategy_knobs).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Adoption of plain parameters
// ---------------------------------------------------------------------------

/// The part a strategy input plays on the endpoint's own requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ManagedRole {
    Page,
    Offset,
    Cursor(Direction),
    PageSize,
    PartitionIndex,
    PartitionCount,
    KeepAlive,
    Constant,
    SessionHandle,
}

impl ManagedRole {
    /// Whether the strategy gives the caller a knob that sets this input, so
    /// a declared parameter in its place can be removed from the request.
    fn has_knob(self, strategy: &Strategy) -> bool {
        match self {
            Self::Page | Self::Offset | Self::PageSize | Self::PartitionIndex | Self::PartitionCount => {
                true
            }
            Self::Cursor(_) => matches!(&strategy.kind, PageStrategy::Cursor(kind) if kind.resumable),
            Self::KeepAlive => strategy
                .session
                .as_ref()
                .and_then(|session| session.keep_alive.as_ref())
                .is_some_and(|keep_alive| keep_alive.adjustable),
            Self::Constant | Self::SessionHandle => false,
        }
    }

    /// Whether the pager has a value to send when the caller sets none: the
    /// start of a page or offset, a keep-alive or constant value, or an
    /// authored [`PageSize::sent_default`](super::PageSize::sent_default).
    fn has_sent_default(self, strategy: &Strategy) -> bool {
        match self {
            Self::Constant => true,
            _ => self.sent_default(strategy).is_some(),
        }
    }

    /// The value the pager sends for an adopted input of this role when the
    /// caller sets none: the first page or offset of the strategy's index
    /// base, the authored [`PageSize::sent_default`](super::PageSize::sent_default),
    /// or the keep-alive default. `None` for a role without one, and for a
    /// constant, which is never adopted.
    ///
    /// An ordinary send of the endpoint sends this value for an adopted
    /// parameter the endpoint declares required.
    pub fn sent_default(self, strategy: &Strategy) -> Option<String> {
        let first = |base: IndexBase| match base {
            IndexBase::ZeroBased => "0",
            IndexBase::OneBased => "1",
        };
        match (self, &strategy.kind) {
            (Self::Page, PageStrategy::PageNumber(kind)) => Some(first(kind.base).to_owned()),
            (Self::Offset, PageStrategy::Offset(kind)) => Some(first(kind.base).to_owned()),
            (Self::PageSize, _) => strategy.kind.size()?.sent_default.map(|size| size.to_string()),
            (Self::KeepAlive, _) => Some(strategy.session.as_ref()?.keep_alive.as_ref()?.default.clone()),
            _ => None,
        }
    }
}

/// Where an endpoint declares a plain parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParamLocation {
    Query,
    Header,
    Cookie,
    Path,
}

/// What happens to a declared parameter a strategy input also names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AdoptionOutcome {
    /// Same location and name, with a matching knob: the strategy manages it
    /// and the request struct omits it.
    Adopted,
    /// Same location and name, but no knob would replace the field: rejected.
    WithoutKnob,
    /// Same location and name, for an input no caller can set (a constant or
    /// session handle): rejected as a collision.
    NotAdoptable,
    /// Same name in a different location: rejected as a collision.
    NearMiss,
}

/// One declared parameter that a strategy input names.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ParamAdoption {
    pub role: ManagedRole,
    pub input: InputParam,
    pub location: ParamLocation,
    /// The name as the endpoint declares it.
    pub name: String,
    /// Whether the endpoint declares it required.
    pub required: bool,
    pub outcome: AdoptionOutcome,
    /// Whether the pager sends a value for it when the caller sets none.
    pub has_sent_default: bool,
}

/// Inputs written on the endpoint's own page requests, with their role.
///
/// Session-step inputs go to other requests and are not listed; keep-alive on
/// open is listed only when the first page opens the session.
pub(crate) fn request_inputs(strategy: &Strategy) -> Vec<(ManagedRole, &InputParam)> {
    let mut inputs = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber(kind) => inputs.push((ManagedRole::Page, &kind.page)),
        PageStrategy::Offset(kind) => inputs.push((ManagedRole::Offset, &kind.offset)),
        PageStrategy::Cursor(kind) => {
            for part in &kind.forward.parts {
                inputs.push((ManagedRole::Cursor(Direction::Forward), &part.param));
            }
            for part in kind.backward.iter().flat_map(|leg| &leg.parts) {
                inputs.push((ManagedRole::Cursor(Direction::Backward), &part.param));
            }
        }
        PageStrategy::NextLink(_) => {}
    }
    if let Some(size) = strategy.kind.size() {
        inputs.push((ManagedRole::PageSize, &size.param));
    }
    for fixed in &strategy.constants {
        inputs.push((ManagedRole::Constant, &fixed.param));
    }
    if let Some(partition) = &strategy.partition {
        inputs.push((ManagedRole::PartitionIndex, &partition.index));
        inputs.push((ManagedRole::PartitionCount, &partition.count));
    }
    if let Some(session) = &strategy.session {
        if let Some(handle) = &session.send_handle_as {
            inputs.push((ManagedRole::SessionHandle, handle));
        }
        if let Some(keep_alive) = &session.keep_alive {
            if matches!(session.open, SessionOpen::FirstPage)
                && let Some(input) = &keep_alive.on_open
            {
                inputs.push((ManagedRole::KeepAlive, input));
            }
            if let Some(input) = &keep_alive.on_page {
                inputs.push((ManagedRole::KeepAlive, input));
            }
        }
    }
    inputs
}

/// The endpoint's declared plain parameters, path parameters included.
fn declared_params(endpoint: &Endpoint) -> Vec<(ParamLocation, &str, bool)> {
    let mut declared: Vec<(ParamLocation, &str, bool)> = path_params(&endpoint.path)
        .into_iter()
        .map(|name| (ParamLocation::Path, name, true))
        .collect();
    if let Some(params) = &endpoint.params {
        let located: [(ParamLocation, &Vec<ParamDef>); 3] = [
            (ParamLocation::Query, &params.query),
            (ParamLocation::Header, &params.header),
            (ParamLocation::Cookie, &params.cookie),
        ];
        for (location, defs) in located {
            declared.extend(defs.iter().map(|def| (location, def.name.as_str(), def.required)));
        }
    }
    declared
}

/// Names of the `{placeholders}` in a path template, without a leading `+`.
pub(crate) fn path_params(path: &str) -> Vec<&str> {
    path.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.trim_start_matches('+')))
        .collect()
}

/// Every declared parameter of `endpoint` that an input of `strategy` names,
/// and what happens to it.
///
/// A query input matches a declared query parameter by exact name; a header
/// input matches a declared header without regard to case. The same name,
/// compared without regard to case, in any other location (query, header,
/// cookie, or path) is a near miss. Body
/// inputs are not matched: the request body has no declared parameter list.
///
/// The request-struct emitter removes exactly the [`AdoptionOutcome::Adopted`]
/// parameters; validation rejects the other outcomes.
pub fn param_adoptions(endpoint: &Endpoint, strategy: &Strategy) -> Vec<ParamAdoption> {
    let declared = declared_params(endpoint);
    let mut adoptions = Vec::new();
    for (role, input) in request_inputs(strategy) {
        let (own_location, name) = match input {
            InputParam::Query(name) => (ParamLocation::Query, name.as_str()),
            InputParam::Header(name) => (ParamLocation::Header, name.as_str()),
            InputParam::Body(_) => continue,
        };
        for &(location, declared_name, required) in &declared {
            let same_location = location == own_location
                && match location {
                    ParamLocation::Header => declared_name.eq_ignore_ascii_case(name),
                    _ => declared_name == name,
                };
            let outcome = if same_location {
                if matches!(role, ManagedRole::Constant | ManagedRole::SessionHandle) {
                    AdoptionOutcome::NotAdoptable
                } else if role.has_knob(strategy) {
                    AdoptionOutcome::Adopted
                } else {
                    AdoptionOutcome::WithoutKnob
                }
            } else if location != own_location && declared_name.eq_ignore_ascii_case(name) {
                AdoptionOutcome::NearMiss
            } else {
                continue;
            };
            adoptions.push(ParamAdoption {
                role,
                input: input.clone(),
                location,
                name: declared_name.to_owned(),
                required,
                outcome,
                has_sent_default: role.has_sent_default(strategy),
            });
        }
    }
    adoptions
}

/// The declared parameters the strategy adopts: pager-managed, exempt from
/// required fidelity, and absent from the request struct and its `new()`.
pub fn adopted_params(endpoint: &Endpoint, strategy: &Strategy) -> Vec<ParamAdoption> {
    param_adoptions(endpoint, strategy)
        .into_iter()
        .filter(|adoption| adoption.outcome == AdoptionOutcome::Adopted)
        .collect()
}
