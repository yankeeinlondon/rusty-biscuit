//! Partitioned pagination at the definition level: a rotating-handle point
//! in time sliced into partitions, inherited from the API's default, is a
//! valid definition whose callers get the session scope and a bounded
//! partition knob.
//!
//! The runtime half of the same definition (one open, slice inputs on every
//! request, the latest observed handle, one close) is the `PitSearch`
//! fixture of `schematic-runtime`'s `pager_partition` suite.

mod pagination_support;

use pagination_support::{api, post, resolution};
use schematic_define::RestMethod;
use schematic_define::pagination::{
    Collection, CursorSource, InputParam, KeepAlive, Knob, MoreSignal, PageSize, PaginationError, Partition, ResolutionOrigin, SessionOpen,
    Session, SessionStep, Strategy, ValueSource, caller_knobs, validate_pagination,
};

/// An Elasticsearch-style point in time: a separate request opens it and
/// returns the handle, every page returns the latest handle and sends it as
/// `pit.id` with a keep-alive, the close sends the handle, and the search is
/// sliced into at most four partitions.
fn sliced_point_in_time() -> Strategy {
    Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .more_when(MoreSignal::NonEmptyPage)
        .page_size(PageSize::new(InputParam::body("size")).max(10_000))
        .session(
            Session::opened_by(SessionStep::new(RestMethod::Post, "/{index}/_pit"), ValueSource::body("id"))
                .pages_at(RestMethod::Post, "/_search")
                .handle_from_page(ValueSource::body("pit_id"))
                .send_handle_as(InputParam::body("pit.id"))
                .keep_alive(
                    KeepAlive::new("1m")
                        .on_open("keep_alive")
                        .on_page(InputParam::body("pit.keep_alive"))
                        .adjustable(),
                )
                .close(SessionStep::new(RestMethod::Delete, "/_pit").handle(InputParam::body("id"))),
        )
        .partition(Partition::new(InputParam::body("slice.id"), InputParam::body("slice.max")).max(4))
}

#[test]
fn a_rotating_handle_point_in_time_with_four_partitions_inherited_from_the_api_default_validates() {
    let mut search = api("Search");
    search.default_pagination = Some(sliced_point_in_time());
    search.endpoints = vec![post("SearchLogs", "/{index}/_search", "SearchResponse")];
    { let (id, collection) = ("SearchLogs", Collection::at("hits.hits[*]._source").item_type("Hit"));  search.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };

    assert_eq!(validate_pagination(&search), vec![]);

    let resolved = resolution(&search, "SearchLogs");
    assert_eq!(resolved.origin(), Some(ResolutionOrigin::ApiDefault));
    let knobs = caller_knobs(&resolved);
    assert!(knobs.contains(&Knob::Session), "{knobs:?}");
    assert!(knobs.contains(&Knob::Partitions { max: Some(4) }), "{knobs:?}");
    assert!(knobs.contains(&Knob::KeepAlive { default: "1m".into() }), "{knobs:?}");
}

#[test]
fn first_page_sessions_require_no_partitions_in_defaults_overrides_and_alternates() {
    for site in ["default", "override", "alternate"] {
        for partitioned in [false, true] {
            let mut strategy = sliced_point_in_time();
            strategy.session.as_mut().unwrap().open = SessionOpen::FirstPage;
            if !partitioned {
                strategy.partition = None;
            }
            let mut search = api("Search");
            search.endpoints = vec![post("SearchLogs", "/{index}/_search", "SearchResponse")];
            let collection = Collection::at("hits.hits[*]._source").item_type("Hit");
            { let (id, collection) = ("SearchLogs", match site {
                "default" => {
                    search.default_pagination = Some(strategy);
                    collection
                }
                "override" => collection.paginate(strategy),
                "alternate" => collection.paginate(sliced_point_in_time()).or_paginate("scroll", strategy),
                _ => unreachable!(),
            });  search.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };
            let diagnostics = validate_pagination(&search);
            if partitioned {
                assert_eq!(diagnostics.len(), 1, "{site}: {diagnostics:?}");
                let diagnostic = &diagnostics[0];
                assert_eq!(diagnostic.error, PaginationError::PartitionedFirstPageSession, "{site}");
                assert_eq!(diagnostic.error.rule(), "partitioned-first-page-session");
                assert_eq!(diagnostic.endpoint.as_deref(), (site != "default").then_some("SearchLogs"));
                assert_eq!(diagnostic.alternate.as_deref(), (site == "alternate").then_some("scroll"));
            } else {
                assert_eq!(diagnostics, vec![], "first-page session without partitions at {site}");
            }
        }
    }
}
