//! What the generator would emit, written by hand. Per endpoint: a unit
//! struct, `Endpoint`, `Paginated` with one static strategy, one empty or
//! one-const impl per authored knob, and a one-line client method. No
//! per-endpoint pager type and no per-endpoint method bodies.

use serde::Deserialize;

use crate::client::{Client, Method};
use crate::pager::Pager;
use crate::spec::*;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Item {
    pub id: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Hit {
    #[serde(rename = "_source")]
    pub source: Item,
}

// --- GitHub ListIssues: page number + Link rel=next signal + page size ------
pub struct ListIssues;
impl Endpoint for ListIssues {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/repos/o/r/issues", items: &[], total: None };
}
impl Paginated for ListIssues {
    type Kind = PageNumber;
    const STRATEGY: &'static PageNumberSpec = &PageNumberSpec { page: Input::Query("page"), base: 1, more: More::LinkRel("next") };
}
impl PageSizeKnob for ListIssues {
    const PAGE_SIZE: SizeSpec = SizeSpec { param: Input::Query("per_page"), min: 1, max: Some(100) };
}

// --- OpenAI ListRoles: opaque forward cursor, not resumable, no size --------
pub struct ListRoles;
impl Endpoint for ListRoles {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/v1/roles", items: &["data"], total: None };
}
impl Paginated for ListRoles {
    type Kind = Cursor;
    const STRATEGY: &'static CursorSpec = &CursorSpec {
        forward: CursorLeg { param: Input::Query("after"), from: ValueAt::Body(&["last_id"]) },
        more: Some(More::Flag(ValueAt::Body(&["has_more"]))),
    };
}

// --- Anthropic ListMessages: resumable, bidirectional id cursor + size ------
pub struct ListBatches;
impl Endpoint for ListBatches {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/v1/messages/batches", items: &["data"], total: None };
}
impl Paginated for ListBatches {
    type Kind = Cursor;
    const STRATEGY: &'static CursorSpec = &CursorSpec {
        forward: CursorLeg { param: Input::Query("after_id"), from: ValueAt::Body(&["last_id"]) },
        more: Some(More::Flag(ValueAt::Body(&["has_more"]))),
    };
}
impl ResumeAfter for ListBatches {}
impl HasPrev for ListBatches {
    const PREV: &'static CursorLeg = &CursorLeg { param: Input::Query("before_id"), from: ValueAt::Body(&["first_id"]) };
}
impl PageSizeKnob for ListBatches {
    const PAGE_SIZE: SizeSpec = SizeSpec { param: Input::Query("limit"), min: 1, max: Some(1000) };
}

// --- ListEvents: resumable forward-only cursor -----------------------------
pub struct ListEvents;
impl Endpoint for ListEvents {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/events", items: &["data"], total: None };
}
impl Paginated for ListEvents {
    type Kind = Cursor;
    const STRATEGY: &'static CursorSpec =
        &CursorSpec { forward: CursorLeg { param: Input::Query("starting_after"), from: ValueAt::Body(&["next_cursor"]) }, more: None };
}
impl ResumeAfter for ListEvents {}

// --- Bitbucket ListRepos: next link in the body, size, total ----------------
pub struct ListRepos;
impl Endpoint for ListRepos {
    type Item = Item;
    const SPEC: &'static EndpointSpec =
        &EndpointSpec { method: Method::Get, path: "/2.0/repositories/w", items: &["values"], total: Some(ValueAt::Body(&["size"])) };
}
impl Paginated for ListRepos {
    type Kind = NextLink;
    const STRATEGY: &'static LinkSpec = &LinkSpec { next: LinkFrom::Body(&["next"]) };
}
impl PageSizeKnob for ListRepos {
    const PAGE_SIZE: SizeSpec = SizeSpec { param: Input::Query("pagelen"), min: 1, max: Some(100) };
}

// --- Link-header next + prev, no size ---------------------------------------
pub struct ListCommits;
impl Endpoint for ListCommits {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/commits", items: &[], total: None };
}
impl Paginated for ListCommits {
    type Kind = NextLink;
    const STRATEGY: &'static LinkSpec = &LinkSpec { next: LinkFrom::Header("next") };
}
impl HasPrev for ListCommits {
    const PREV: &'static LinkFrom = &LinkFrom::Header("prev");
}

// --- Elasticsearch scroll: session with keep-alive and close ----------------
pub struct SearchScroll;
impl Endpoint for SearchScroll {
    type Item = Hit;
    const SPEC: &'static EndpointSpec = &EndpointSpec {
        method: Method::Post,
        path: "/idx/_search",
        items: &["hits", "hits"],
        total: Some(ValueAt::Body(&["hits", "total", "value"])),
    };
}
impl Paginated for SearchScroll {
    type Kind = Scroll;
    const STRATEGY: &'static ScrollSpec = &ScrollSpec {
        open_keep_alive: Input::Query("scroll"),
        default_keep_alive: "1m",
        follow_path: "/_search/scroll",
        handle_from: ValueAt::Body(&["_scroll_id"]),
        handle_param: Input::Body("scroll_id"),
        keep_alive_param: Input::Body("scroll"),
        close_path: "/_search/scroll",
    };
}
impl PageSizeKnob for SearchScroll {
    const PAGE_SIZE: SizeSpec = SizeSpec { param: Input::Body("size"), min: 1, max: Some(10_000) };
}
impl KeepAliveKnob for SearchScroll {}

// --- Ollama ListModels: a list, unpaginated: no Paginated impl at all -------
pub struct ListModels;
impl Endpoint for ListModels {
    type Item = Item;
    const SPEC: &'static EndpointSpec = &EndpointSpec { method: Method::Get, path: "/api/tags", items: &["models"], total: None };
}

// --- the generated client methods: one line each -----------------------------
impl Client {
    pub fn list_issues(&self) -> Pager<ListIssues> {
        Pager::new(self)
    }
    pub fn list_roles(&self) -> Pager<ListRoles> {
        Pager::new(self)
    }
    pub fn list_batches(&self) -> Pager<ListBatches> {
        Pager::new(self)
    }
    pub fn list_events(&self) -> Pager<ListEvents> {
        Pager::new(self)
    }
    pub fn list_repos(&self) -> Pager<ListRepos> {
        Pager::new(self)
    }
    pub fn list_commits(&self) -> Pager<ListCommits> {
        Pager::new(self)
    }
    pub fn search_scroll(&self) -> Pager<SearchScroll> {
        Pager::new(self)
    }
}
