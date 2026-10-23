//! Fixed headers (`RestApi::headers`, `Endpoint::headers`) as OpenAPI header
//! parameters (R8, R9).
//!
//! Fixture APIs built with the public builders go through
//! `build_openapi_grouped` and `validate_openapi`; each test asserts the
//! emitted parameters and the `fixed-header` warnings. The committed
//! artifacts are checked for the shipped headers and, over every artifact,
//! for no duplicate, reserved, or credential header parameter.

use schematic_define::builders::Present;
use schematic_define::openapi::{ExportOptions, HeaderGap, HeaderGapReason, header_gaps};
use schematic_define::{
    AuthMethod, AuthPolicy, AuthStrategy, DefinitionContext, Endpoint, EndpointBuilder, ParamDef, QueryParamType, RestApi,
};
use schematic_gen::diagnostics::{Feature, Output, Severity};
use schematic_gen::export::openapi::build_openapi_grouped;
use schematic_gen::validation::validate_openapi;
use serde_json::{Value, json};

fn get(id: &str, path: &str) -> EndpointBuilder<Present, Present> {
    Endpoint::get(id, path).describes(format!("{id} operation")).returns_text()
}

fn bearer_api(headers: &[(&str, &str)], endpoints: Vec<Endpoint>) -> RestApi {
    let ctx = DefinitionContext::types_in("::fixture");
    let mut builder = RestApi::builder(&ctx, "Fixture", "https://fixture.example")
        .describes("Fixture")
        .bearer_env(["FIXTURE_TOKEN"]);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    builder.endpoints(endpoints).build().unwrap()
}

fn openapi(api: &RestApi) -> Value {
    serde_json::to_value(build_openapi_grouped(&[api], "fixture", &ExportOptions::new()).unwrap()).unwrap()
}

fn header_params<'a>(doc: &'a Value, path: &str, method: &str) -> Vec<&'a Value> {
    doc["paths"][path][method]["parameters"]
        .as_array()
        .map(|params| params.iter().filter(|param| param["in"] == "header").collect())
        .unwrap_or_default()
}

fn fixed(name: &str, value: &str) -> Value {
    json!({
        "in": "header", "name": name, "required": true, "style": "simple",
        "schema": {"type": "string", "enum": [value], "default": value},
    })
}

/// `(endpoint, message)` of each `fixed-header` warning.
fn fixed_header_warnings(api: &RestApi) -> Vec<(String, String)> {
    validate_openapi(&[api])
        .into_iter()
        .filter(|diagnostic| diagnostic.feature == Feature::FixedHeader)
        .inspect(|diagnostic| {
            assert_eq!(diagnostic.severity, Severity::Warning);
            assert_eq!(diagnostic.output, Output::OpenApi);
        })
        .map(|diagnostic| (diagnostic.endpoint.unwrap(), diagnostic.message))
        .collect()
}

#[test]
fn api_fixed_header_exports_as_a_required_single_value_enum_with_default() {
    let api = bearer_api(&[("anthropic-version", "2023-06-01")], vec![get("A", "/a").build().unwrap(), get("B", "/b").build().unwrap()]);
    let doc = openapi(&api);

    for path in ["/a", "/b"] {
        assert_eq!(header_params(&doc, path, "get"), [&fixed("anthropic-version", "2023-06-01")]);
    }
    // The extension still carries the declaration.
    assert_eq!(doc["x-schematic"]["headers"], json!([["anthropic-version", "2023-06-01"]]));
    assert!(fixed_header_warnings(&api).is_empty());
}

#[test]
fn endpoint_header_overrides_the_api_header_case_insensitively() {
    let api = bearer_api(
        &[("X-Version", "1"), ("X-Client", "fixture")],
        vec![
            get("Override", "/override").header("x-version", "2").build().unwrap(),
            get("Inherit", "/inherit").build().unwrap(),
            get("Extra", "/extra").header("X-Extra", "yes").build().unwrap(),
        ],
    );
    let doc = openapi(&api);

    assert_eq!(header_params(&doc, "/override", "get"), [&fixed("x-version", "2"), &fixed("X-Client", "fixture")]);
    assert_eq!(header_params(&doc, "/inherit", "get"), [&fixed("X-Version", "1"), &fixed("X-Client", "fixture")]);
    assert_eq!(
        header_params(&doc, "/extra", "get"),
        [&fixed("X-Version", "1"), &fixed("X-Client", "fixture"), &fixed("X-Extra", "yes")],
    );
}

#[test]
fn credential_headers_follow_the_effective_operation_auth() {
    let ctx = DefinitionContext::types_in("::fixture");
    let header_bearer = AuthStrategy::BearerToken { header: Some("X-Token".into()) };
    let other_key = AuthPolicy {
        explicit: vec![AuthMethod::ApiKey { header: "X-Other-Key".into(), value_prefix: None }],
        env_fallback: None,
    };
    let api = RestApi::builder(&ctx, "Fixture", "https://fixture.example")
        .describes("Fixture")
        .authentication(header_bearer, ["FIXTURE_TOKEN"])
        .header("x-token", "fixed-token")
        .header("X-Other-Key", "fixed-key")
        .endpoints(vec![
            get("Inherit", "/inherit").build().unwrap(),
            get("Public", "/public").public().build().unwrap(),
            get("Replace", "/replace").auth(other_key).build().unwrap(),
        ])
        .build()
        .unwrap();
    let doc = openapi(&api);

    // The API scheme sends `X-Token`; `X-Other-Key` is ordinary there.
    assert_eq!(header_params(&doc, "/inherit", "get"), [&fixed("X-Other-Key", "fixed-key")]);
    // Anonymous: neither header is a credential.
    assert_eq!(
        header_params(&doc, "/public", "get"),
        [&fixed("x-token", "fixed-token"), &fixed("X-Other-Key", "fixed-key")],
    );
    // The replacement sends `X-Other-Key` and no longer sends `X-Token`.
    assert_eq!(header_params(&doc, "/replace", "get"), [&fixed("x-token", "fixed-token")]);

    assert_eq!(
        header_gaps(&api).unwrap(),
        [
            HeaderGap {
                endpoint_id: "Inherit".into(),
                name: "x-token".into(),
                value: Some("fixed-token".into()),
                reason: HeaderGapReason::Credential { scheme: "bearerAuth".into() },
            },
            HeaderGap {
                endpoint_id: "Replace".into(),
                name: "X-Other-Key".into(),
                value: Some("fixed-key".into()),
                reason: HeaderGapReason::Credential { scheme: "apiKeyAuth".into() },
            },
        ],
    );
    let warnings = fixed_header_warnings(&api);
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0].0, "Inherit");
    assert!(warnings[0].1.contains("credential header of security scheme `bearerAuth`"), "{}", warnings[0].1);
}

#[test]
fn reserved_headers_are_never_parameters_and_unexpressed_values_warn() {
    let api = bearer_api(
        &[],
        vec![
            get("AcceptMatches", "/accept-matches").header("Accept", "TEXT/PLAIN").build().unwrap(),
            get("AcceptDiffers", "/accept-differs").header("Accept", "application/vnd.fixture+json").build().unwrap(),
            Endpoint::post("TypeMatches", "/type-matches").describes("post").returns_empty()
                .request_text().header("Content-Type", "text/plain").build().unwrap(),
            Endpoint::post("TypeDiffers", "/type-differs").describes("post").returns_empty()
                .request_text().header("content-type", "text/csv").build().unwrap(),
            get("PublicAuthorization", "/public-authorization").public().header("Authorization", "Token abc").build().unwrap(),
            get("BearerAuthorization", "/bearer-authorization").header("Authorization", "Token abc").build().unwrap(),
        ],
    );
    let doc = openapi(&api);

    for (path, method) in [
        ("/accept-matches", "get"),
        ("/accept-differs", "get"),
        ("/type-matches", "post"),
        ("/type-differs", "post"),
        ("/public-authorization", "get"),
        ("/bearer-authorization", "get"),
    ] {
        assert!(header_params(&doc, path, method).is_empty(), "{method} {path}");
    }

    let reasons: Vec<(String, HeaderGapReason)> =
        header_gaps(&api).unwrap().into_iter().map(|gap| (gap.endpoint_id, gap.reason)).collect();
    assert_eq!(
        reasons,
        [
            ("AcceptDiffers".into(), HeaderGapReason::Accept),
            ("TypeDiffers".into(), HeaderGapReason::ContentType),
            ("PublicAuthorization".into(), HeaderGapReason::Authorization),
            ("BearerAuthorization".into(), HeaderGapReason::Credential { scheme: "bearerAuth".into() }),
        ],
    );
    assert_eq!(fixed_header_warnings(&api).len(), 4);
}

#[test]
fn explicit_header_parameters_merge_when_compatible_and_warn_when_not() {
    let api = bearer_api(
        &[("X-Mode", "fast"), ("X-Tier", "gold"), ("X-Count", "3"), ("X-Level", "high")],
        vec![get("Collide", "/collide")
            .header_param(ParamDef::new("x-mode", QueryParamType::String).describes("Processing mode"))
            .header_param(ParamDef::new("X-Tier", QueryParamType::Enum(vec!["gold".into(), "silver".into()])))
            .header_param(ParamDef::new("X-Count", QueryParamType::Integer))
            .header_param(ParamDef::new("X-Level", QueryParamType::Enum(vec!["low".into()])))
            .header_param(ParamDef::new("Accept", QueryParamType::String))
            .build()
            .unwrap()],
    );
    let doc = openapi(&api);

    assert_eq!(
        header_params(&doc, "/collide", "get"),
        [
            &json!({"in": "header", "name": "x-mode", "description": "Processing mode", "required": true,
                "explode": false, "style": "simple", "schema": {"type": "string", "default": "fast"}}),
            &json!({"in": "header", "name": "X-Tier", "required": true, "explode": false, "style": "simple",
                "schema": {"type": "string", "enum": ["gold", "silver"], "default": "gold"}}),
            // Incompatible: exported as declared, still optional.
            &json!({"in": "header", "name": "X-Count", "explode": false, "style": "simple",
                "schema": {"type": "integer"}}),
            &json!({"in": "header", "name": "X-Level", "explode": false, "style": "simple",
                "schema": {"type": "string", "enum": ["low"]}}),
        ],
    );

    let gaps = header_gaps(&api).unwrap();
    let summary: Vec<(&str, Option<&str>, &HeaderGapReason)> =
        gaps.iter().map(|gap| (gap.name.as_str(), gap.value.as_deref(), &gap.reason)).collect();
    assert_eq!(
        summary,
        [
            ("Accept", None, &HeaderGapReason::Accept),
            ("X-Count", Some("3"), &HeaderGapReason::IncompatibleParameter),
            ("X-Level", Some("high"), &HeaderGapReason::IncompatibleParameter),
        ],
    );
    let warnings = fixed_header_warnings(&api);
    assert!(warnings[0].1.starts_with("header parameter `Accept`"), "{}", warnings[0].1);
    assert!(warnings[1].1.starts_with("fixed header `X-Count: 3` conflicts"), "{}", warnings[1].1);
}

#[test]
fn committed_anthropic_artifact_sends_anthropic_version_on_every_operation() {
    let doc: Value = serde_json::from_str(include_str!("../../../openapi/anthropic.json")).unwrap();
    let mut operations = 0;
    for (path, item) in doc["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            if !operation.is_object() || operation.get("operationId").is_none() {
                continue;
            }
            operations += 1;
            assert_eq!(
                header_params(&doc, path, method),
                [&fixed("anthropic-version", "2023-06-01")],
                "{method} {path}",
            );
        }
    }
    assert!(operations > 0);
}

#[test]
fn committed_artifacts_declare_no_duplicate_reserved_or_credential_header_parameter() {
    let artifacts = [
        ("anthropic", include_str!("../../../openapi/anthropic.json")),
        ("artificial_analysis", include_str!("../../../openapi/artificial_analysis.json")),
        ("bitbucket", include_str!("../../../openapi/bitbucket.json")),
        ("elevenlabs", include_str!("../../../openapi/elevenlabs.json")),
        ("emqx", include_str!("../../../openapi/emqx.json")),
        ("eversolo", include_str!("../../../openapi/eversolo.json")),
        ("gitea", include_str!("../../../openapi/gitea.json")),
        ("github", include_str!("../../../openapi/github.json")),
        ("gitlab", include_str!("../../../openapi/gitlab.json")),
        ("huggingface", include_str!("../../../openapi/huggingface.json")),
        ("lmstudio", include_str!("../../../openapi/lmstudio.json")),
        ("ollama", include_str!("../../../openapi/ollama.json")),
        ("openai", include_str!("../../../openapi/openai.json")),
        ("samsung_smart_tv", include_str!("../../../openapi/samsung_smart_tv.json")),
        ("unfolded_circle_core_rest", include_str!("../../../openapi/unfolded_circle_core_rest.json")),
    ];
    let artifact_count = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../openapi"))
        .unwrap()
        .filter(|entry| entry.as_ref().unwrap().path().extension().is_some_and(|ext| ext == "json"))
        .count();
    assert_eq!(artifacts.len(), artifact_count, "every committed OpenAPI artifact is listed here");

    let mut fixed_parameters = 0;
    for (name, text) in artifacts {
        let doc: Value = serde_json::from_str(text).unwrap();
        let schemes = &doc["components"]["securitySchemes"];
        let credential = |scheme: &Value| match scheme["type"].as_str() {
            Some("apiKey") if scheme["in"] == "header" => scheme["name"].as_str().map(str::to_ascii_lowercase),
            Some("apiKey") => None,
            _ => Some("authorization".to_string()),
        };
        for (path, item) in doc["paths"].as_object().unwrap() {
            for (method, operation) in item.as_object().unwrap() {
                if !operation.is_object() || operation.get("operationId").is_none() {
                    continue;
                }
                let security = operation.get("security").unwrap_or(&doc["security"]);
                let credentials: Vec<String> = security
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|requirement| requirement.as_object().unwrap().keys())
                    .filter_map(|scheme| credential(&schemes[scheme]))
                    .collect();
                let mut seen: Vec<String> = Vec::new();
                for param in header_params(&doc, path, method) {
                    let lower = param["name"].as_str().unwrap().to_ascii_lowercase();
                    let at = format!("{name}: {method} {path} header {lower}");
                    assert!(!seen.contains(&lower), "duplicate {at}");
                    assert!(!["accept", "content-type", "authorization"].contains(&lower.as_str()), "reserved {at}");
                    assert!(!credentials.contains(&lower), "credential {at}");
                    if param["schema"]["enum"].as_array().is_some_and(|values| values.len() == 1) {
                        fixed_parameters += 1;
                    }
                    seen.push(lower);
                }
            }
        }
    }
    assert!(fixed_parameters > 0);
}
