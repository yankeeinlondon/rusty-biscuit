//! Endpoint collection contracts across builder, transfer, and API validation boundaries.
mod pagination_support;
use pagination_support::api;
use schematic_define::{DefinitionContext, Endpoint, RestApi};
use schematic_define::pagination::{Collection, MoreSignal, PaginationException, Resolution, ResolutionOrigin, PaginationError, Strategy, resolve, validate_pagination};

fn page() -> Strategy { Strategy::page_number("page", MoreSignal::NonEmptyPage).build() }

#[test]
fn endpoint_collection_precedes_legacy_row_and_preserves_inheritance() {
    let endpoint = Endpoint::get("List", "/items").describes("Items").returns_empty()
        .pagination(Collection::at("data").item_type("Item").unpaginated()).build().unwrap();
    let mut api = api("Test");
    api.default_pagination = Some(page());
    api.pagination_exceptions.push(PaginationException::new("List", Collection::root().paginate(page())));
    assert!(matches!(resolve(&api, &endpoint), Resolution::Unpaginated { origin: ResolutionOrigin::Exception, .. }));
    let mut inherited = endpoint.clone();
    inherited.pagination = Some(Collection::at("data").item_type("Item"));
    let resolved = resolve(&api, &inherited);
    assert_eq!(resolved.origin(), Some(ResolutionOrigin::ApiDefault));
    assert_eq!(resolved.list().unwrap().items.to_string(), "data[*]");
    assert_eq!(resolved.list().unwrap().item_type.as_deref(), Some("Item"));
    api.default_pagination = None;
    assert_eq!(resolve(&api, &inherited).origin(), Some(ResolutionOrigin::Unauthored));
}

#[test]
fn endpoint_collection_and_strategy_errors_reach_api_build() {
    let mut ctx = DefinitionContext::types_in("::models");
    for (collection, expected_code) in [(Collection::at("").item_type("Item").paginate(page()), "invalid-item-selector"),
        (Collection::root().item_type("Item").paginate(Strategy::page_number("", MoreSignal::NonEmptyPage).build()), "empty-value")] {
        let endpoint = Endpoint::get("List", "/items").describes("Items").returns_json::<serde_json::Value>(&mut ctx).pagination(collection).build().unwrap();
        let mut raw = api("Test");
        raw.endpoints.push(endpoint.clone());
        let diagnostics = validate_pagination(&raw);
        assert!(diagnostics.iter().any(|diagnostic| match &diagnostic.error {
            PaginationError::InvalidItemSelector { .. } => expected_code == "invalid-item-selector",
            PaginationError::EmptyValue(_) => expected_code == "empty-value",
            _ => false,
        }), "{diagnostics:?}");
        assert!(RestApi::builder(&ctx, "Test", "https://example.test").describes("Test").no_auth().endpoints([endpoint]).build().is_err());
    }
}

#[test]
fn endpoint_collection_serde_optional_shape_and_repeated_round_trip() {
    let endpoint = Endpoint::get("List", "/items").describes("Items").returns_empty().build().unwrap();
    let baseline = serde_json::to_value(&endpoint).unwrap();
    assert!(baseline.get("pagination").is_none());
    for input in [serde_json::Value::Null, serde_json::json!(123), serde_json::json!([]), serde_json::json!([{}, 123]), serde_json::json!([123]), serde_json::json!({})] {
        let mut value = baseline.clone(); value["pagination"] = input.clone();
        let result = serde_json::from_value::<schematic_define::Endpoint>(value);
        if input.is_null() { assert_eq!(result.unwrap().pagination, None); } else { assert!(result.is_err(), "{input}"); }
    }
    let json = serde_json::to_string(&endpoint).unwrap();
    assert!(serde_json::from_str::<schematic_define::Endpoint>(&format!("{json} trailing")).is_err());
    let duplicate = format!("{},\"pagination\":null,\"pagination\":null}}", &json[..json.len() - 1]);
    assert!(serde_json::from_str::<schematic_define::Endpoint>(&duplicate).is_err());
    let mut ctx = DefinitionContext::types_in("::models");
    let mut authored = Endpoint::get("List", "/items").describes("Items")
        .returns_json::<serde_json::Value>(&mut ctx)
        .pagination(Collection::at("data").item_type("Item").unpaginated()).build().unwrap();
    for _ in 0..2 {
        let json = serde_json::to_string(&authored).unwrap();
        let reread: schematic_define::Endpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(reread, authored);
        let api = RestApi::builder(&ctx, "Test", "https://example.test").describes("Test")
            .no_auth().endpoints([reread.clone()]).build().unwrap();
        assert!(matches!(resolve(&api, &api.endpoints[0]), Resolution::Unpaginated { .. }));
        assert_eq!(resolve(&api, &api.endpoints[0]).list().unwrap().item_type.as_deref(), Some("Item"));
        authored = reread;
    }
}

#[test]
fn endpoint_explicit_collection_drives_public_pager_knobs() {
    let mut ctx = DefinitionContext::types_in("::models");
    let endpoint = Endpoint::post("Search", "/search").describes("Search")
        .returns_json::<serde_json::Value>(&mut ctx)
        .pagination(Collection::at("data").item_type("serde_json::Value").paginate(page()))
        .build().unwrap();
    let api = RestApi::builder(&ctx, "Test", "https://example.test")
        .describes("Test").no_auth().endpoints([endpoint.clone()]).build().unwrap();
    let resolution = resolve(&api, &endpoint);
    assert_eq!(resolution.origin(), Some(ResolutionOrigin::Exception));
    assert_eq!(resolution.strategy(), Some(&page()));
    assert_eq!(resolution.list().unwrap().item_type.as_deref(), Some("serde_json::Value"));
    assert!(schematic_define::pagination::caller_knobs(&resolution).contains(&schematic_define::pagination::Knob::Next));
}
