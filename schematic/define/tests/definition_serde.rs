//! The JSON interchange contract is lossless and does not invent required data.
use schematic_define::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
use serde_json::{Value, json};

#[test]
fn parameter_variants_round_trip_twice() {
    for param_type in [
        QueryParamType::String, QueryParamType::Integer, QueryParamType::Number,
        QueryParamType::Boolean, QueryParamType::Json,
        QueryParamType::Array(Box::new(QueryParamType::String)),
        QueryParamType::Enum(vec![]), QueryParamType::Enum(vec!["a".into(), "b".into()]),
    ] {
        for style in [ParamStyle::Form, ParamStyle::Simple, ParamStyle::SpaceDelimited,
            ParamStyle::PipeDelimited, ParamStyle::DeepObject] {
            let param = ParamDef { rust_name: None, secret: false, pager_managed: false, name: "filter".into(), required: true,
                description: Some("Filter".into()), param_type: param_type.clone(),
                explode: true, style };
            let params = EndpointParams { query: vec![param.clone()], header: vec![param.clone()],
                cookie: vec![param], pagination: None, response_pagination: None };
            let first = serde_json::to_string(&params).unwrap();
            let restored: EndpointParams = serde_json::from_str(&first).unwrap();
            let second = serde_json::to_string(&restored).unwrap();
            let reread: EndpointParams = serde_json::from_str(&second).unwrap();
            assert_eq!(params, reread);
            assert_eq!(first, second);
            assert_eq!(reread.query, reread.header);
            assert_eq!(reread.header, reread.cookie);
        }
    }
}

#[test]
fn parameter_collections_reject_malformed_json_matrix() {
    let control = json!({"query": [], "header": [], "cookie": [],
        "pagination": null, "response_pagination": null});
    assert_eq!(serde_json::from_value::<EndpointParams>(control.clone()).unwrap(), EndpointParams::default());
    for field in ["query", "header", "cookie"] {
        for replacement in [None, Some(Value::Null), Some(json!(123)),
            Some(json!([123])), Some(json!([{}, 123])), Some(json!({}))] {
            let mut input = control.clone();
            if let Some(value) = replacement { input[field] = value; }
            else { input.as_object_mut().unwrap().remove(field); }
            assert!(serde_json::from_value::<EndpointParams>(input).is_err(), "{field}");
        }
        let duplicate = format!("{{\"query\":[],\"header\":[],\"cookie\":[],\"{field}\":[]}}");
        assert!(serde_json::from_str::<EndpointParams>(&duplicate).is_err());
    }
    for input in ["{\"query\":[],\"header\":[],\"cookie\":[]} garbage", "{"] {
        assert!(serde_json::from_str::<EndpointParams>(input).is_err());
    }
    for field in ["pagination", "response_pagination"] {
        let mut absent = control.clone();
        absent.as_object_mut().unwrap().remove(field);
        assert_eq!(serde_json::from_value::<EndpointParams>(absent).unwrap(), EndpointParams::default());
        let mut wrong = control.clone();
        wrong[field] = json!(123);
        assert!(serde_json::from_value::<EndpointParams>(wrong).is_err());
    }
}

#[test]
fn legacy_pagination_and_origin_policy_variants_round_trip() {
    use schematic_define::{CrossOrigin, PaginationStyle};
    for policy in [CrossOrigin::same_origin(), CrossOrigin::allow_origins(Vec::<String>::new()),
        CrossOrigin::allow_origins(["https://uploads.example.com"]),
        CrossOrigin::allow_without_credentials(), CrossOrigin::danger_allow_any(), CrossOrigin::deny()] {
        let text = serde_json::to_string(&policy).unwrap();
        let restored: CrossOrigin = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.name(), policy.name());
        assert_eq!(restored, policy);
    }
    for style in [PaginationStyle::github(), PaginationStyle::offset_limit("skip", "take", 0, u32::MAX),
        PaginationStyle::cursor("after", None, 0), PaginationStyle::cursor("after", Some("limit"), u32::MAX)] {
        let text = serde_json::to_string(&style).unwrap();
        let restored: PaginationStyle = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.to_query_params(), style.to_query_params());
        assert_eq!(restored, style);
    }
    for invalid in [
        r#"{"Cursor":{"cursor_param":"after","limit_param":null,"default_limit":"1"}}"#,
        r#"{"Cursor":{"cursor_param":"after","limit_param":null,"default_limit":-1}}"#,
        r#"{"Cursor":{"cursor_param":"after","limit_param":null,"default_limit":4294967296}}"#,
        r#"{"Cursor":{"cursor_param":"after","limit_param":null}}"#,
    ] {
        assert!(serde_json::from_str::<PaginationStyle>(invalid).is_err(), "{invalid}");
    }
}
