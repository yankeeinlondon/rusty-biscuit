use schematic_define::{CrossOrigin, DefinitionContext, Endpoint, ParamDef, QueryParamType, RestApi};
use schematic_define::pagination::{MoreSignal, Strategy};

fn assemble(endpoint: Endpoint) -> Result<RestApi, schematic_define::DefinitionError> {
    let ctx = DefinitionContext::types_in("::models");
    RestApi::builder(&ctx, "Forms", "https://a.test").describes("Forms")
        .no_auth().endpoints([endpoint]).build()
}

#[test]
fn locations_markers_and_policy_survive_repeated_transfer() {
    let ep = Endpoint::get("Read", "/files/{file}").describes("Read").returns_empty()
        .cross_origin(CrossOrigin::allow_origins(["https://b.test:443"]))
        .query(ParamDef::new("ref", QueryParamType::String).rust_name("git_ref").secret())
        .header_param(ParamDef::new("x-pin", QueryParamType::String).secret())
        .cookie(ParamDef::new("session", QueryParamType::String).secret()).build().unwrap();
    let mut api = assemble(ep).unwrap();
    for _ in 0..2 {
        api = serde_json::from_str(&serde_json::to_string(&api).unwrap()).unwrap();
        let ep = api.endpoints[0].clone();
        let built = assemble(ep).unwrap();
        let params = built.endpoints[0].params.as_ref().unwrap();
        assert_eq!(params.query[0].name, "ref");
        assert_eq!(params.query[0].rust_name.as_deref(), Some("git_ref"));
        assert!(params.query[0].secret && params.header[0].secret && params.cookie[0].secret);
        assert_eq!(built.endpoints[0].cross_origin, Some(CrossOrigin::allow_origins(["https://b.test:443"])));
    }
}

#[test]
fn every_get_body_family_is_rejected_and_post_is_accepted() {
    for method in [schematic_define::RestMethod::Get, schematic_define::RestMethod::Post] {
        for body in 0..5 {
            let mut ctx = DefinitionContext::types_in("::models");
            let ep = Endpoint::new("Read", method, "/files").describes("Read").returns_empty();
            let result = match body {
                0 => ep.request_json::<String>(&mut ctx),
                1 => ep.request_text(), 2 => ep.request_binary(),
                3 => ep.request_form(vec![]), _ => ep.request_urlencoded(vec![]),
            }.build();
            if method == schematic_define::RestMethod::Get {
                let error = result.unwrap_err().to_string();
                assert!(error.contains("Read") && error.contains("GET"), "{error}");
            } else { assert!(result.is_ok()); }
        }
    }
    // HEAD has no new restriction in this phase.
    assert!(Endpoint::head("Head", "/").describes("Head").returns_empty().request_text().build().is_ok());
}

#[test]
fn metadata_normalizes_header_names_and_rejects_invalid_origins_and_names() {
    let ctx = DefinitionContext::types_in("::models");
    let api = RestApi::builder(&ctx, "Forms", "https://a.test").describes("Forms").no_auth()
        .request_id_header("X-Request-ID").secret_header("X-Pin").build().unwrap();
    assert_eq!(api.request_id_header.as_deref(), Some("x-request-id"));
    assert_eq!(api.secret_names, ["x-pin"]);
    for name in ["", "bad name", "x\nattack", "é"] {
        assert!(RestApi::builder(&ctx, "Forms", "https://a.test").describes("Forms").no_auth().request_id_header(name).build().is_err());
    }
    for origin in ["https://b.test/path", "https://u@b.test", "https://b.test?x=1", "ftp://b.test", "*", "https://b.test/#x"] {
        let ep = Endpoint::get("Read", "/").describes("Read").returns_empty().cross_origin(CrossOrigin::allow_origins([origin])).build().unwrap();
        let error = assemble(ep).unwrap_err().to_string();
        assert!(error.contains("Forms") && error.contains("Read"), "{error}");
    }
}

#[test]
fn pager_marker_requires_matching_strategy_and_authored_required_default() {
    let mut ctx = DefinitionContext::types_in("::models");
    for required in [false, true] {
        for matched in [false, true] {
            for default in [false, true] {
                let mut param = ParamDef::new(if matched { "limit" } else { "other" }, QueryParamType::Integer).pager_managed();
                if required { param = param.required(); }
                let ep = Endpoint::get("List", "/").describes("List").returns_json::<Vec<String>>(&mut ctx).query(param).build().unwrap();
                let mut api = serde_json::to_value(&ep).unwrap();
                for _ in 0..2 { api = serde_json::to_value(serde_json::from_value::<Endpoint>(api).unwrap()).unwrap(); }
                let ep: Endpoint = serde_json::from_value(api).unwrap();
                let size = schematic_define::pagination::PageSize::new("limit");
                let size = if default { size.sent_default(25) } else { size };
                let strategy = Strategy::page_number("page", MoreSignal::NonEmptyPage).page_size(size).build();
                let result = RestApi::builder(&ctx, "Forms", "https://a.test").describes("Forms").no_auth()
                    .default_pagination(strategy).endpoints([ep]).build();
                if !matched { assert!(result.is_err()); }
                else if !required || default {
                    let api = result.as_ref().unwrap();
                    let resolution = schematic_define::pagination::resolve(api, &api.endpoints[0]);
                    let adopted = schematic_define::pagination::adopted_params(&api.endpoints[0], resolution.strategy().unwrap());
                    assert!(adopted.iter().any(|entry| entry.name == "limit" && entry.location == schematic_define::pagination::ParamLocation::Query));
                    assert!(schematic_define::pagination::caller_knobs(&resolution).iter().any(|knob| matches!(knob, schematic_define::pagination::Knob::PageSize { param, .. } if param == "limit")));
                }
                if matched && required && !default {
                    let error = result.unwrap_err().to_string();
                    assert!(error.contains("pager-managed-required-without-default"), "{error}");
                }
            }
        }
    }
}

#[test]
fn transferred_get_body_reports_destination_api_endpoint_and_method() {
    let mut ep = Endpoint::get("Read", "/").describes("Read").returns_empty().build().unwrap();
    ep.request = Some(schematic_define::ApiRequest::Text { content_type: "text/plain".into() });
    let error = assemble(ep).unwrap_err().to_string();
    assert!(error.contains("Forms") && error.contains("Read") && error.contains("GET"), "{error}");
}

#[test]
fn unmatched_markers_and_invalid_aliases_cannot_hide_inputs() {
    for location in 0..3 {
        let param = ParamDef::new("page", QueryParamType::Integer).pager_managed();
        let ep = Endpoint::get("Read", "/").describes("Read").returns_empty();
        let ep = match location { 0 => ep.query(param), 1 => ep.header_param(param), _ => ep.cookie(param) }.build().unwrap();
        let error = assemble(ep).unwrap_err().to_string();
        assert!(error.contains("unmatched-pager-managed-parameter"), "{error}");
    }
    for alias in ["", "_", "ref", "let", "3value", "bad-name"] {
        let ep = Endpoint::get("Read", "/").describes("Read").returns_empty()
            .query(ParamDef::new("ref", QueryParamType::String).rust_name(alias)).build().unwrap();
        assert!(assemble(ep).is_err(), "{alias}");
    }
}

#[test]
fn json_definition_forms_matrix_reaches_public_assembly() {
    let mut ctx = DefinitionContext::types_in("::models");
    let ep = Endpoint::get("List", "/").describes("List").returns_json::<Vec<String>>(&mut ctx)
        .cross_origin(CrossOrigin::allow_origins(["https://b.test"]))
        .query(ParamDef::new("limit", QueryParamType::Integer).required().rust_name("page_limit").secret().pager_managed())
        .build().unwrap();
    let strategy = Strategy::page_number("page", MoreSignal::NonEmptyPage)
        .page_size(schematic_define::pagination::PageSize::new("limit").sent_default(25)).build();
    let check = |text: &str| -> Result<RestApi, String> {
        let ep: Endpoint = serde_json::from_str(text).map_err(|error| error.to_string())?;
        RestApi::builder(&ctx, "Forms", "https://a.test").describes("Forms").no_auth()
            .default_pagination(strategy.clone()).endpoints([ep]).build().map_err(|error| error.to_string())
    };
    let original = serde_json::to_string(&ep).unwrap();
    let control = check(&original).unwrap();
    assert_eq!(control.endpoints[0].cross_origin, ep.cross_origin);
    let param = &control.endpoints[0].params.as_ref().unwrap().query[0];
    assert!(param.secret && param.pager_managed);
    assert_eq!(param.rust_name.as_deref(), Some("page_limit"));
    let fields = [
        ("cross_origin", r#"{"AllowOrigins":["https://b.test"]}"#, true, r#"{"AllowOrigins":[]}"#, true),
        ("rust_name", r#""page_limit""#, true, r#""""#, false),
        ("secret", "true", false, "[]", false),
        ("pager_managed", "true", false, "[]", false),
    ];
    for (field, value, null_ok, empty, empty_ok) in fields {
        let needle = format!("\"{field}\":{value}");
        assert!(original.contains(&needle), "{field} is load-bearing in the control");
        let absent = if original.contains(&format!("{needle},")) { original.replace(&format!("{needle},"), "") }
            else { original.replace(&format!(",{needle}"), "") };
        assert_ne!(absent, original);
        let api = check(&absent).unwrap();
        let p = &api.endpoints[0].params.as_ref().unwrap().query[0];
        match field {
            "cross_origin" => assert!(api.endpoints[0].cross_origin.is_none()),
            "rust_name" => assert!(p.rust_name.is_none()),
            "secret" => assert!(!p.secret),
            _ => assert!(!p.pager_managed),
        }
        for (shape, replacement, accepted) in [
            ("null", "null", null_ok), ("whole wrong type", "123", false),
            ("one wrong element", r#"["a",123]"#, false),
            ("every wrong element", "[123]", false), ("empty", empty, empty_ok),
        ] {
            let text = original.replace(&needle, &format!("\"{field}\":{replacement}"));
            assert_eq!(check(&text).is_ok(), accepted, "{field}: {shape}");
        }
        for empty_wrong_shape in ["[]", "{}"] {
            let text = original.replace(&needle, &format!("\"{field}\":{empty_wrong_shape}"));
            assert!(check(&text).is_err(), "{field}: empty wrong shape {empty_wrong_shape}");
        }
        if field == "cross_origin" {
            for replacement in [r#"{"AllowOrigins":["https://b.test",123]}"#, r#"{"AllowOrigins":[123]}"#] {
                assert!(check(&original.replace(&needle, &format!("\"{field}\":{replacement}"))).is_err());
            }
        }
        assert!(check(&original.replace(&needle, &format!("{needle},{needle}"))).is_err(), "{field}: duplicate");
        assert!(check(&format!("{original} garbage")).is_err(), "{field}: trailing garbage");
    }
}
