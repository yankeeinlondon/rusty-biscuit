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
            let param = ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false, name: "filter".into(), required: true,
                description: Some("Filter".into()), param_type: param_type.clone(),
                explode: true, style };
            let params = EndpointParams { path: vec![], query: vec![param.clone()], header: vec![param.clone()],
                cookie: vec![param],  };
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
    let control = json!({"query": [], "header": [], "cookie": []});
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
}

#[test]
fn origin_policy_variants_round_trip() {
    use schematic_define::CrossOrigin;
    for policy in [CrossOrigin::same_origin(), CrossOrigin::allow_origins(Vec::<String>::new()),
        CrossOrigin::allow_origins(["https://uploads.example.com"]),
        CrossOrigin::allow_without_credentials(), CrossOrigin::danger_allow_any(), CrossOrigin::deny()] {
        let text = serde_json::to_string(&policy).unwrap();
        let restored: CrossOrigin = serde_json::from_str(&text).unwrap();
        assert_eq!(restored.name(), policy.name());
        assert_eq!(restored, policy);
    }
}
