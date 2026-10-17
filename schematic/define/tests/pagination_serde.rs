//! Serde on the pagination model: strict round trips and decode failures.
//!
//! Serde here is a model round-trip facility; no definition file is read, so
//! there is no configuration reader to walk a fixture matrix over. These tests
//! pin what a decode accepts: unknown fields, missing lists, wrong types,
//! duplicate keys, and trailing content are errors, and an absent `Option` is
//! the same as `null` because the model defines no difference.

use schematic_define::RestMethod;
use schematic_define::pagination::{
    Collection, CursorSource, FollowUp, InputParam, KeepAlive, LinkSource, MoreSignal, PageSize,
    PaginationException, Partition, Session, SessionStep, Strategy, ValueSource, strategy_knobs,
    validate_strategy,
};
use serde_json::{Value, json};

/// A strategy that sets every dimension, so a round trip covers every type.
fn everything() -> Strategy {
    Strategy::cursor(InputParam::body("search_after"), CursorSource::last_item("sort"))
        .tiebreak(InputParam::body("tiebreak"), CursorSource::last_item("id"))
        .inclusive("id")
        .backward(InputParam::header("X-Before"), CursorSource::first_item("sort"))
        .more_when(MoreSignal::total_items(ValueSource::header("X-Total")))
        .page_size(PageSize::new(InputParam::body("size")).server_default(10).sent_default(50).min(1).max(100).adjustable())
        .resumable()
        .constant("track_total_hits", "true")
        .terminal_value("null")
        .total(ValueSource::body("hits.total.value"))
        .follow_up(FollowUp::to(RestMethod::Post, "/_search").with_carry(schematic_define::pagination::Carry::AllExcept(vec!["from".into()])))
        .session(
            Session::opened_by(SessionStep::new(RestMethod::Post, "/{index}/_pit").constant("keep_alive", "1m"), ValueSource::body("id"))
                .pages_at(RestMethod::Post, "/_search")
                .handle_from_page(ValueSource::body("pit_id"))
                .send_handle_as(InputParam::body("pit.id"))
                .keep_alive(KeepAlive::new("1m").on_open("keep_alive").on_page(InputParam::body("pit.keep_alive")).adjustable())
                .renew(SessionStep::new(RestMethod::Post, "/_pit/renew").handle(InputParam::body("id")))
                .close(SessionStep::new(RestMethod::Delete, "/_pit").handle(InputParam::body("id"))),
        )
        .partition(Partition::new(InputParam::body("slice.id"), InputParam::body("slice.max")).max(8))
}

fn decode(value: Value) -> Result<Strategy, serde_json::Error> {
    serde_json::from_value(value)
}

#[test]
fn every_dimension_survives_a_repeated_round_trip() {
    let original = everything();
    let first = serde_json::to_string(&original).unwrap();
    let decoded: Strategy = serde_json::from_str(&first).unwrap();
    assert_eq!(decoded, original);
    let second = serde_json::to_string(&decoded).unwrap();
    assert_eq!(second, first, "a second write is byte-identical");
    let again: Strategy = serde_json::from_str(&second).unwrap();
    assert_eq!(again, original);
    // The public effects agree too.
    assert_eq!(strategy_knobs(&again), strategy_knobs(&original));
    assert_eq!(validate_strategy(&again), validate_strategy(&original));
}

#[test]
fn every_kind_and_row_round_trips() {
    let kinds = [
        Strategy::page_number("page", MoreSignal::link_rel("next")).zero_based().build(),
        Strategy::offset("offset", MoreSignal::is_last("last")).one_based().build(),
        Strategy::next_link(LinkSource::body_segments(["@odata.nextLink"])).prev(LinkSource::link_header("prev")).build(),
        Strategy::page_number("page", MoreSignal::header_present("X-Next-Page")).build(),
        Strategy::page_number("page", MoreSignal::FullPage).page_size(PageSize::new("n")).build(),
        Strategy::page_number("page", MoreSignal::total_pages(ValueSource::header("X-Pages"))).build(),
    ];
    for strategy in kinds {
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(serde_json::from_str::<Strategy>(&json).unwrap(), strategy, "{json}");
    }
    let rows = vec![
        PaginationException::new("A", Collection::root().unpaginated()),
        PaginationException::new("B", Collection::at("data").item_type("Item")),
        PaginationException::new(
            "C",
            Collection::at("edges[*].node")
                .item_type("serde_json::Value")
                .paginate(everything())
                .or_paginate("keyset", Strategy::next_link(LinkSource::link_header("next"))),
        ),
    ];
    let json = serde_json::to_string(&rows).unwrap();
    assert_eq!(serde_json::from_str::<Vec<PaginationException>>(&json).unwrap(), rows);
}

#[test]
fn serialized_shape_is_tagged_and_readable() {
    let strategy = Strategy::cursor("after_id", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .build();
    let value = serde_json::to_value(&strategy).unwrap();
    assert_eq!(value["kind"]["kind"], "cursor");
    assert_eq!(value["kind"]["forward"]["parts"][0]["param"], json!({ "in": "query", "name": "after_id" }));
    assert_eq!(
        value["kind"]["forward"]["parts"][0]["source"],
        json!({ "from": "field", "source": { "from": "body", "at": ["last_id"] } })
    );
    assert_eq!(value["kind"]["more"]["signal"], "flag");
    assert_eq!(value["kind"]["forward"]["boundary"], json!({ "boundary": "exclusive" }));
    assert_eq!(value["constants"], json!([]));
    assert_eq!(value["session"], Value::Null);

    let row = serde_json::to_value(PaginationException::new("L", Collection::root().unpaginated())).unwrap();
    assert_eq!(row["collection"]["pagination"], json!({ "policy": "unpaginated" }));
    assert_eq!(row["collection"]["items"]["record"], json!(["each"]));
}

/// One edit to a serialized strategy.
type Edit = Box<dyn FnOnce(&mut Value)>;

/// Applies `edit` to a valid serialized strategy and decodes the result.
fn edited(edit: impl FnOnce(&mut Value)) -> Result<Strategy, serde_json::Error> {
    let mut value = serde_json::to_value(everything()).unwrap();
    edit(&mut value);
    decode(value)
}

#[test]
fn the_unedited_control_decodes() {
    assert_eq!(edited(|_| {}).unwrap(), everything());
}

#[test]
fn unknown_fields_are_rejected_at_every_level() {
    let cases: Vec<(&str, Edit)> = vec![
        ("strategy", Box::new(|v| v["trust"] = json!("same_origin"))),
        ("kind", Box::new(|v| v["kind"]["trust"] = json!("same_origin"))),
        ("page size", Box::new(|v| v["kind"]["size"]["default"] = json!(10))),
        ("more signal", Box::new(|v| v["kind"]["more"]["extra"] = json!(true))),
        ("session", Box::new(|v| v["session"]["open_on"] = json!("first"))),
        ("partition", Box::new(|v| v["partition"]["min"] = json!(1))),
        ("cursor part", Box::new(|v| v["kind"]["forward"]["parts"][0]["default"] = json!("x"))),
    ];
    for (place, edit) in cases {
        let error = edited(edit).expect_err(place).to_string();
        assert!(error.contains("unknown field"), "{place}: {error}");
    }
}

#[test]
fn absent_lists_and_flags_are_errors_and_empty_lists_are_not() {
    for field in ["constants", "terminal_values"] {
        let error = edited(|v| {
            v.as_object_mut().unwrap().remove(field);
        })
        .expect_err(field);
        assert!(error.to_string().contains(&format!("missing field `{field}`")), "{error}");
        let empty = edited(|v| v[field] = json!([])).unwrap();
        let emptied = if field == "constants" { empty.constants.len() } else { empty.terminal_values.len() };
        assert_eq!(emptied, 0, "{field}");
    }
    assert_eq!(everything().constants.len(), 1);
    assert_eq!(everything().terminal_values.len(), 1);
    let error = edited(|v| {
        v["kind"]["size"].as_object_mut().unwrap().remove("adjustable");
    })
    .unwrap_err();
    assert!(error.to_string().contains("missing field `adjustable`"), "{error}");
    let error = edited(|v| {
        v["kind"]["forward"].as_object_mut().unwrap().remove("parts");
    })
    .unwrap_err();
    assert!(error.to_string().contains("missing field `parts`"), "{error}");
}

#[test]
fn an_absent_option_is_null_is_no_dimension() {
    let absent = edited(|v| {
        v.as_object_mut().unwrap().remove("partition");
    })
    .unwrap();
    let null = edited(|v| v["partition"] = Value::Null).unwrap();
    assert_eq!(absent.partition, None);
    assert_eq!(null, absent);
    assert!(!strategy_knobs(&absent).iter().any(|k| matches!(k, schematic_define::pagination::Knob::Partitions { .. })));
    assert!(strategy_knobs(&everything()).iter().any(|k| matches!(k, schematic_define::pagination::Knob::Partitions { .. })));
}

#[test]
fn wrong_types_are_rejected_never_dropped() {
    let cases: Vec<(&str, Edit)> = vec![
        ("whole list", Box::new(|v| v["terminal_values"] = json!(123))),
        ("one element", Box::new(|v| v["terminal_values"] = json!(["null", 1]))),
        ("every element", Box::new(|v| v["terminal_values"] = json!([1]))),
        ("composite element", Box::new(|v| v["constants"] = json!([{ "param": { "in": "query", "name": "a" }, "value": "x" }, 7]))),
        ("number", Box::new(|v| v["kind"]["size"]["max"] = json!("100"))),
        ("negative number", Box::new(|v| v["kind"]["size"]["min"] = json!(-1))),
        ("flag", Box::new(|v| v["kind"]["resumable"] = json!("yes"))),
        ("path", Box::new(|v| v["total"]["at"] = json!("hits.total"))),
        ("unknown kind", Box::new(|v| v["kind"]["kind"] = json!("keyset"))),
        ("unknown location", Box::new(|v| v["kind"]["size"]["param"]["in"] = json!("cookie"))),
        ("method", Box::new(|v| v["follow_up"]["method"] = json!("get"))),
    ];
    for (shape, edit) in cases {
        assert!(edited(edit).is_err(), "{shape} decoded");
    }
}

#[test]
fn duplicate_keys_and_trailing_content_are_rejected() {
    let valid = serde_json::to_string(&everything()).unwrap();
    assert!(serde_json::from_str::<Strategy>(&valid).is_ok());

    let duplicated = valid.replacen("\"constants\":", "\"constants\":[],\"constants\":", 1);
    let error = serde_json::from_str::<Strategy>(&duplicated).unwrap_err();
    assert!(error.to_string().contains("duplicate field `constants`"), "{error}");

    let nested = valid.replacen("\"min\":1,", "\"min\":1,\"min\":2,", 1);
    assert_ne!(nested, valid);
    let error = serde_json::from_str::<Strategy>(&nested).unwrap_err();
    assert!(error.to_string().contains("duplicate field `min`"), "{error}");

    let error = serde_json::from_str::<Strategy>(&format!("{valid} {{}}")).unwrap_err();
    assert!(error.to_string().contains("trailing characters"), "{error}");
}
