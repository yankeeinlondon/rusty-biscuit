//! Pagination coverage of every shipped REST API (R27, R28, R33).
//!
//! The expectation table is exhaustive: every endpoint that resolves as a list
//! has a row asserting its resolution origin, item location and type, strategy,
//! exact caller knobs, and adopted request fields; every endpoint without a row
//! must resolve as "not a list". A new list endpoint, a changed default, or an
//! import that renames an endpoint fails here until its row is updated.
//!
//! Rows are grouped by provider family: `forges` (GitHub, GitLab, Gitea,
//! Bitbucket), `openai`, and `others` (every remaining API).

mod forges;
mod openai;
mod others;

use std::collections::{BTreeMap, BTreeSet};

use schematic_define::RestApi;
use schematic_define::pagination::{
    CursorLeg, CursorSource, Direction, IndexBase, Knob, LinkSource, MoreSignal, PageStrategy,
    Polarity, Resolution, ResolutionOrigin, Strategy, ValueSource, adopted_params, caller_knobs,
    resolve, validate_pagination,
};
use schematic_definitions::apis_by_module;

pub use schematic_define::pagination::Knob::{Next, Prev, Total};

/// One API's expected list endpoints.
pub struct Api {
    /// `RestApi::name`.
    pub name: &'static str,
    pub rows: Vec<Row>,
}

/// One list endpoint and how it must resolve.
pub struct Row {
    pub endpoint: &'static str,
    pub expect: Expect,
}

pub enum Expect {
    Paged {
        origin: ResolutionOrigin,
        /// The item selector as displayed: `[*]` for a root array.
        items: &'static str,
        item_type: &'static str,
        /// [`describe`] of the resolved strategy.
        strategy: &'static str,
        knobs: Vec<Knob>,
        /// Declared request parameters the strategy adopts, in any order.
        adopted: &'static [&'static str],
    },
    Unpaged {
        origin: ResolutionOrigin,
        items: &'static str,
        why: Unpaged,
    },
}

/// Why a list is left unpaginated.
pub enum Unpaged {
    /// R28: the provider row was recalled, not verified. With the definition
    /// gaps below, this is the R33 verification task.
    Unverified {
        recalled: &'static str,
        source_needed: &'static str,
    },
    /// The definition disagrees with the provider evidence; resolving it needs
    /// verification and an endpoint-literal change.
    DefinitionGap {
        gap: &'static str,
        recalled: &'static str,
        source_needed: &'static str,
    },
    /// The provider replies with a continuation but documents no request input
    /// to send it back.
    ProviderGap(&'static str),
    /// A root array that is a lookup, not a list the provider pages.
    NotPaged(&'static str),
    /// A root array in an API with no default and no evidence of paging.
    NoPagination,
}

impl Row {
    pub fn paged(
        endpoint: &'static str,
        origin: ResolutionOrigin,
        items: &'static str,
        item_type: &'static str,
        strategy: &'static str,
        knobs: Vec<Knob>,
        adopted: &'static [&'static str],
    ) -> Self {
        Self {
            endpoint,
            expect: Expect::Paged {
                origin,
                items,
                item_type,
                strategy,
                knobs,
                adopted,
            },
        }
    }

    pub fn unpaged(endpoint: &'static str, origin: ResolutionOrigin, items: &'static str, why: Unpaged) -> Self {
        Self {
            endpoint,
            expect: Expect::Unpaged { origin, items, why },
        }
    }
}

pub fn start_page(base: IndexBase) -> Knob {
    Knob::StartPage { base }
}

/// A one-part resumable cursor.
pub fn resume(direction: Direction) -> Knob {
    Knob::ResumeCursor { direction, parts: 1 }
}

pub fn page_size(param: &str, min: u32, max: Option<u32>, server_default: Option<u32>) -> Knob {
    Knob::PageSize {
        param: param.to_owned(),
        min,
        max,
        server_default,
    }
}

/// A one-line description of a strategy, compared verbatim by the table.
///
/// The shape is `<kind>: <inputs>; more <signal>; total <source>; const k=v`,
/// with absent dimensions omitted. Examples:
/// `page_number: page page from 1; more Link rel=next` and
/// `cursor: after <- body last_id; before <- body first_id; more flag body has_more`.
pub fn describe(strategy: &Strategy) -> String {
    let mut parts = Vec::new();
    match &strategy.kind {
        PageStrategy::PageNumber(kind) => {
            parts.push(format!("page {} from {}", kind.page.display_name(), base(kind.base)));
        }
        PageStrategy::Offset(kind) => {
            parts.push(format!("offset {} from {}", kind.offset.display_name(), base(kind.base)));
        }
        PageStrategy::Cursor(kind) => {
            parts.push(leg(&kind.forward));
            if let Some(backward) = &kind.backward {
                parts.push(leg(backward));
            }
        }
        PageStrategy::NextLink(kind) => {
            parts.push(format!("next {}", link(&kind.next)));
            if let Some(prev) = &kind.prev {
                parts.push(format!("prev {}", link(prev)));
            }
        }
        other => parts.push(format!("{other:?}")),
    }
    if let Some(more) = strategy.kind.more() {
        parts.push(format!("more {}", signal(more)));
    }
    if let Some(total) = &strategy.total {
        parts.push(format!("total {}", value(total)));
    }
    for constant in &strategy.constants {
        parts.push(format!("const {}={}", constant.param.display_name(), constant.value));
    }
    format!("{}: {}", strategy.kind.kind_name(), parts.join("; "))
}

fn base(base: IndexBase) -> u8 {
    match base {
        IndexBase::ZeroBased => 0,
        IndexBase::OneBased => 1,
    }
}

fn value(source: &ValueSource) -> String {
    match source {
        ValueSource::Body(path) => format!("body {path}"),
        ValueSource::Header(name) => format!("header {name}"),
        other => format!("{other:?}"),
    }
}

fn leg(leg: &CursorLeg) -> String {
    leg.parts
        .iter()
        .map(|part| {
            let source = match &part.source {
                CursorSource::Field { source } => value(source),
                CursorSource::LastItem { path } => format!("last item {path}"),
                CursorSource::FirstItem { path } => format!("first item {path}"),
                other => format!("{other:?}"),
            };
            format!("{} <- {source}", part.param.display_name())
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn link(source: &LinkSource) -> String {
    match source {
        LinkSource::LinkHeader { rel } => format!("Link rel={rel}"),
        LinkSource::Field { source } => value(source),
        other => format!("{other:?}"),
    }
}

fn signal(signal: &MoreSignal) -> String {
    match signal {
        MoreSignal::Flag {
            source,
            polarity: Polarity::TrueMeansMore,
        } => format!("flag {}", value(source)),
        MoreSignal::Flag {
            source,
            polarity: Polarity::TrueMeansLast,
        } => format!("last-flag {}", value(source)),
        MoreSignal::Present { source } => format!("present {}", value(source)),
        MoreSignal::LinkRel { rel } => format!("Link rel={rel}"),
        MoreSignal::Total { source, unit } => format!("total {unit:?} {}", value(source)),
        MoreSignal::NonEmptyPage => "non-empty page".to_owned(),
        MoreSignal::FullPage => "full page".to_owned(),
        other => format!("{other:?}"),
    }
}

fn expectations() -> Vec<Api> {
    let mut apis = forges::apis();
    apis.extend(openai::apis());
    apis.extend(others::apis());
    apis
}

fn shipped_apis() -> Vec<RestApi> {
    apis_by_module().into_values().flatten().collect()
}

/// Compares one endpoint's resolution with its row; `None` when they agree.
fn mismatch(api: &RestApi, endpoint: &schematic_define::Endpoint, row: Option<&Row>) -> Option<String> {
    let at = format!("{}.{}", api.name, endpoint.id);
    let resolution = resolve(api, endpoint);
    let Some(row) = row else {
        return (resolution != Resolution::NotAList)
            .then(|| format!("{at}: resolves as a list ({:?}) but has no row", resolution.origin()));
    };
    let list = resolution.list();
    let items = list.map(|list| list.items.to_string());
    match (&row.expect, &resolution) {
        (
            Expect::Paged {
                origin,
                items: want_items,
                item_type,
                strategy,
                knobs,
                adopted,
            },
            Resolution::Paginated { strategy: got, .. },
        ) => {
            let mut problems = Vec::new();
            if resolution.origin() != Some(*origin) {
                problems.push(format!("origin {:?}, want {origin:?}", resolution.origin()));
            }
            if items.as_deref() != Some(*want_items) {
                problems.push(format!("items {items:?}, want {want_items:?}"));
            }
            let got_type = list.and_then(|list| list.item_type.as_deref());
            if got_type != Some(*item_type) {
                problems.push(format!("item type {got_type:?}, want {item_type:?}"));
            }
            let described = describe(got);
            if described != *strategy {
                problems.push(format!("strategy `{described}`, want `{strategy}`"));
            }
            let got_knobs = caller_knobs(&resolution);
            if &got_knobs != knobs {
                problems.push(format!("knobs {got_knobs:?}, want {knobs:?}"));
            }
            let got_adopted: BTreeSet<String> =
                adopted_params(endpoint, got).into_iter().map(|adoption| adoption.name).collect();
            let want_adopted: BTreeSet<String> = adopted.iter().map(|name| (*name).to_owned()).collect();
            if got_adopted != want_adopted {
                problems.push(format!("adopted {got_adopted:?}, want {want_adopted:?}"));
            }
            (!problems.is_empty()).then(|| format!("{at}: {}", problems.join("; ")))
        }
        (
            Expect::Unpaged {
                origin,
                items: want_items,
                ..
            },
            Resolution::Unpaginated { .. },
        ) => {
            let mut problems = Vec::new();
            if resolution.origin() != Some(*origin) {
                problems.push(format!("origin {:?}, want {origin:?}", resolution.origin()));
            }
            if items.as_deref() != Some(*want_items) {
                problems.push(format!("items {items:?}, want {want_items:?}"));
            }
            (!problems.is_empty()).then(|| format!("{at}: {}", problems.join("; ")))
        }
        (Expect::Paged { .. }, other) => Some(format!("{at}: want paginated, resolves {other:?}")),
        (Expect::Unpaged { .. }, other) => Some(format!("{at}: want unpaginated, resolves {other:?}")),
    }
}

#[test]
fn every_list_endpoint_of_every_api_resolves_as_authored() {
    let expected = expectations();
    let by_api: BTreeMap<&str, &Api> = expected.iter().map(|api| (api.name, api)).collect();
    let mut problems = Vec::new();

    for api in shipped_apis() {
        let Some(expected_api) = by_api.get(api.name.as_str()) else {
            problems.push(format!("{}: no expectation table", api.name));
            continue;
        };
        for endpoint in &api.endpoints {
            let row = expected_api.rows.iter().find(|row| row.endpoint == endpoint.id);
            problems.extend(mismatch(&api, endpoint, row));
        }
    }

    assert!(problems.is_empty(), "{} mismatches:\n{}", problems.len(), problems.join("\n"));
}

#[test]
fn every_row_names_one_endpoint_of_a_shipped_api() {
    let shipped = shipped_apis();
    let mut problems = Vec::new();
    let mut names = BTreeSet::new();
    for expected in expectations() {
        if !names.insert(expected.name) {
            problems.push(format!("{}: two expectation tables", expected.name));
        }
        let Some(api) = shipped.iter().find(|api| api.name == expected.name) else {
            problems.push(format!("{}: not a shipped API", expected.name));
            continue;
        };
        let mut seen = BTreeSet::new();
        for row in &expected.rows {
            if !seen.insert(row.endpoint) {
                problems.push(format!("{}.{}: two rows", api.name, row.endpoint));
            }
            if !api.endpoints.iter().any(|endpoint| endpoint.id == row.endpoint) {
                problems.push(format!("{}.{}: no such endpoint", api.name, row.endpoint));
            }
        }
    }
    assert_eq!(names.len(), 18, "one table per shipped REST API");
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_shipped_definition_validates() {
    let diagnostics: Vec<String> = shipped_apis()
        .iter()
        .flat_map(|api| {
            validate_pagination(api)
                .into_iter()
                .map(move |diagnostic| format!("{}: {diagnostic:?}", api.name))
        })
        .collect();
    assert!(diagnostics.is_empty(), "{}", diagnostics.join("\n"));
}

#[test]
fn paginated_counts_per_api() {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for api in expectations() {
        let paged = api
            .rows
            .iter()
            .filter(|row| matches!(row.expect, Expect::Paged { .. }))
            .count();
        if paged > 0 {
            counts.insert(api.name, paged);
        }
    }
    let expected: BTreeMap<&str, usize> = [
        ("Bitbucket", 8),
        ("ElevenLabs", 2),
        ("GitHub", 9),
        ("GitLab", 9),
        ("Gitea", 8),
        ("OpenAI", 57),
    ]
    .into_iter()
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(counts.values().sum::<usize>(), 93);
}

/// Every list left unpaginated for want of evidence carries its recalled
/// strategy and the source needed to verify it (R33).
#[test]
fn every_unverified_list_names_its_recalled_strategy_and_needed_source() {
    let mut task = Vec::new();
    for api in expectations() {
        for row in &api.rows {
            let Expect::Unpaged { why, .. } = &row.expect else { continue };
            let (recalled, source_needed) = match why {
                Unpaged::Unverified {
                    recalled,
                    source_needed,
                }
                | Unpaged::DefinitionGap {
                    recalled,
                    source_needed,
                    ..
                } => (recalled, source_needed),
                _ => continue,
            };
            assert!(
                !recalled.trim().is_empty() && !source_needed.trim().is_empty(),
                "{}.{}: an unverified row names its recalled strategy and needed source",
                api.name,
                row.endpoint
            );
            task.push(format!("{}.{}", api.name, row.endpoint));
        }
    }
    let emqx_lists = [
        "ListAlarms",
        "ListAuthUsers",
        "ListBanned",
        "ListClients",
        "ListRetained",
        "ListRules",
        "ListSubscriptions",
        "ListTopics",
    ];
    let hub_lists = [
        "GetUserCollections",
        "ListDatasetCommits",
        "ListDatasetFiles",
        "ListDatasets",
        "ListModelCommits",
        "ListModelDiscussions",
        "ListModelFiles",
        "ListModels",
        "ListSpaceFiles",
        "ListSpaces",
        "ListUserRepos",
    ];
    let mut expected: Vec<String> = [
        "Anthropic.ListModels",
        "ElevenLabs.ListVoices",
        "GitLab.ListIssueParticipants",
        "Gitea.GetGitTree",
        "Gitea.GetGitTreeRecursive",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(["EmqxBasic", "EmqxBearer"].into_iter().flat_map(|api| emqx_lists.map(|id| format!("{api}.{id}"))))
    .chain(hub_lists.map(|id| format!("HuggingFaceHub.{id}")))
    .collect();
    expected.sort();
    task.sort();
    assert_eq!(task, expected, "the R33 verification task");
}
