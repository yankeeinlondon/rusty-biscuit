//! Targeted golden fixtures for the Postman exporter.
//!
//! Each scenario builds a small synthetic [`RestApi`] (or pair of APIs)
//! that exercises one specific behaviour fixed in earlier review-2 phases:
//!
//! 1. Mixed-auth grouped collection (Phase 3, Finding 1).
//! 2. Auth variables declared in `collection.variable` (Phase 2, Finding 2).
//! 3. Multipart `type: "file"` for file-upload form fields (Phase 1, Finding 3).
//! 4. Path + query parameter shapes for a `GET` endpoint.
//! 5. Grouped-module base-URL aliasing (`baseUrl`, `baseUrl2`).
//!
//! The expected JSON for each scenario lives in
//! `tests/fixtures/postman/golden/`. The fixture files are committed; this
//! test asserts strict equality against them. To regenerate after an
//! intentional generator change, run:
//!
//! ```text
//! BLESS_POSTMAN_GOLDEN=1 cargo test -p schematic-gen --test postman_golden
//! ```
//!
//! Beyond strict equality each scenario also runs the produced JSON
//! through the vendored Postman v2.1.0 schema validator, so a fixture
//! that drifts away from a valid collection cannot quietly stay green.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use jsonschema::Validator;
use schematic_define::auth::ApiKeyLocation;
use schematic_define::params::{EndpointParams, QueryParamType};
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, FormField, RestApi, RestMethod, Schema,
};
use schematic_gen::export::postman::{build_postman_collection, build_postman_collection_grouped};
use serde_json::Value;

// --- Schema validator ------------------------------------------------------

const POSTMAN_SCHEMA_BYTES: &[u8] =
    include_bytes!("fixtures/postman/v2.1.0-collection.schema.json");

fn postman_validator() -> &'static Validator {
    static VALIDATOR: OnceLock<Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_slice(POSTMAN_SCHEMA_BYTES)
            .expect("vendored Postman schema must be valid JSON");
        jsonschema::validator_for(&schema)
            .expect("vendored Postman schema must compile as a JSON Schema")
    })
}

fn validate_postman_json(value: &Value) -> Result<(), Vec<String>> {
    let validator = postman_validator();
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|err| format!("{}: {}", err.instance_path(), err))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// --- Fixture helpers -------------------------------------------------------

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("postman")
        .join("golden")
}

fn pretty_json(value: &Value) -> String {
    let mut out = serde_json::to_string_pretty(value).expect("serialize JSON");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Asserts the produced collection equals the committed fixture and is
/// schema-valid. When `BLESS_POSTMAN_GOLDEN=1`, writes the fixture to disk
/// instead of comparing — used to regenerate after intentional changes.
fn assert_golden(name: &str, produced: &Value) {
    if let Err(errors) = validate_postman_json(produced) {
        panic!(
            "produced fixture {} failed Postman v2.1.0 schema validation:\n  - {}",
            name,
            errors.join("\n  - "),
        );
    }

    let fixture_path = golden_dir().join(format!("{}.json", name));
    let pretty = pretty_json(produced);

    if std::env::var("BLESS_POSTMAN_GOLDEN").is_ok() {
        std::fs::create_dir_all(fixture_path.parent().expect("parent dir"))
            .expect("create golden dir");
        std::fs::write(&fixture_path, &pretty).expect("write golden fixture");
        return;
    }

    let expected = std::fs::read_to_string(&fixture_path).unwrap_or_else(|e| {
        panic!(
            "missing golden fixture {} ({}); regenerate with BLESS_POSTMAN_GOLDEN=1",
            fixture_path.display(),
            e,
        )
    });

    assert_eq!(
        pretty,
        expected,
        "golden fixture {} drifted; regenerate with BLESS_POSTMAN_GOLDEN=1 \
         after confirming the diff is intentional",
        fixture_path.display(),
    );
}

// --- Synthetic APIs --------------------------------------------------------

fn rest_api(
    name: &str,
    base_url: &str,
    auth: AuthStrategy,
    module_path: Option<&str>,
    endpoints: Vec<Endpoint>,
) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} test API", name),
        base_url: base_url.to_string(),
        docs_url: None,
        auth,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints,
        module_path: module_path.map(str::to_string),
        request_suffix: None,
        version: None,
        env_mapping: None,
    }
}

fn endpoint(
    id: &str,
    method: RestMethod,
    path: &str,
    description: &str,
    request: Option<ApiRequest>,
    response: ApiResponse,
    params: Option<EndpointParams>,
) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: description.to_string(),
        request,
        response,
        headers: vec![],
        params,
        oauth_scopes: None,
    }
}

// --- Scenario builders -----------------------------------------------------

/// Two APIs grouped under one module with mixed Basic + Bearer auth.
/// One endpoint id (`ListAlarms`) collides across the two members so the
/// disambiguation suffix kicks in.
fn build_mixed_auth_emqx() -> Value {
    let basic_api = rest_api(
        "EmqxBasic",
        "http://localhost:18083/api/v5",
        AuthStrategy::Basic,
        Some("emqx"),
        vec![
            endpoint(
                "ListAlarms",
                RestMethod::Get,
                "/alarms",
                "List active alarms (basic auth)",
                None,
                ApiResponse::json_type("AlarmList"),
                None,
            ),
            endpoint(
                "GetSettings",
                RestMethod::Get,
                "/settings",
                "Read EMQX settings",
                None,
                ApiResponse::json_type("Settings"),
                None,
            ),
        ],
    );

    let bearer_api = rest_api(
        "EmqxBearer",
        "http://localhost:18083/api/v5",
        AuthStrategy::BearerToken { header: None },
        Some("emqx"),
        vec![
            endpoint(
                "ListAlarms",
                RestMethod::Get,
                "/alarms",
                "List active alarms (bearer auth)",
                None,
                ApiResponse::json_type("AlarmList"),
                None,
            ),
            endpoint(
                "Login",
                RestMethod::Post,
                "/login",
                "Exchange credentials for a bearer token",
                Some(ApiRequest::Json(Schema::new("LoginRequest"))),
                ApiResponse::json_type("LoginResponse"),
                None,
            ),
        ],
    );

    let collection = build_postman_collection_grouped(&[&basic_api, &bearer_api], "emqx");
    serde_json::to_value(&collection).expect("serialize collection")
}

/// Single bearer-token API mirroring OpenAI's shape: confirms
/// `bearerToken` is declared in the collection variable list.
fn build_auth_variables_openai() -> Value {
    let api = rest_api(
        "OpenAI",
        "https://api.openai.com/v1",
        AuthStrategy::BearerToken { header: None },
        None,
        vec![endpoint(
            "ListModels",
            RestMethod::Get,
            "/models",
            "List available models",
            None,
            ApiResponse::json_type("ListModelsResponse"),
            None,
        )],
    );

    let collection = build_postman_collection(&api);
    serde_json::to_value(&collection).expect("serialize collection")
}

/// Single API with a multipart upload endpoint mirroring ElevenLabs'
/// audio sample upload — `audio` field must serialize as `type: "file"`.
fn build_file_upload_elevenlabs() -> Value {
    let api = rest_api(
        "ElevenLabs",
        "https://api.elevenlabs.io/v1",
        AuthStrategy::ApiKey {
            header: "xi-api-key".to_string(),
            value_prefix: None,
        },
        None,
        vec![endpoint(
            "UploadSample",
            RestMethod::Post,
            "/voices/{voice_id}/samples",
            "Upload a voice sample for cloning",
            Some(ApiRequest::form_data(vec![
                FormField::file_accept("audio", vec!["audio/*".into()])
                    .with_description("Audio file (any audio MIME type)"),
                FormField::text("name")
                    .optional()
                    .with_description("Optional sample name"),
            ])),
            ApiResponse::json_type("UploadSampleResponse"),
            None,
        )],
    );

    let collection = build_postman_collection(&api);
    serde_json::to_value(&collection).expect("serialize collection")
}

/// Endpoint with both path variables and query parameters (GitHub
/// `/repos/{owner}/{repo}/issues?state=...`). Verifies path variables get
/// `:owner`/`:repo` segments + a `variable` block, and that the query
/// param is exposed as a Postman query entry.
fn build_path_query_params_github() -> Value {
    let params = EndpointParams::default().with_query_param(
        "state",
        QueryParamType::Enum(vec![
            "open".to_string(),
            "closed".to_string(),
            "all".to_string(),
        ]),
        false,
        Some("Issue state filter"),
    );

    let api = rest_api(
        "GitHub",
        "https://api.github.com",
        AuthStrategy::BearerToken { header: None },
        None,
        vec![endpoint(
            "ListRepoIssues",
            RestMethod::Get,
            "/repos/{owner}/{repo}/issues",
            "List issues for a repository",
            None,
            ApiResponse::json_type("IssueList"),
            Some(params),
        )],
    );

    let collection = build_postman_collection(&api);
    serde_json::to_value(&collection).expect("serialize collection")
}

/// Single API with `ApiKeyParam` auth using query location.
/// Verifies that `"in": "query"` is emitted for query-based API keys.
fn build_api_key_query_auth() -> Value {
    let api = rest_api(
        "ApiKeyQuery",
        "https://api.example.com/v1",
        AuthStrategy::ApiKeyParam {
            name: "api_key".to_string(),
            location: ApiKeyLocation::Query,
        },
        None,
        vec![endpoint(
            "ListItems",
            RestMethod::Get,
            "/items",
            "List all items",
            None,
            ApiResponse::json_type("ItemList"),
            None,
        )],
    );

    let collection = build_postman_collection(&api);
    serde_json::to_value(&collection).expect("serialize collection")
}

/// Two APIs grouped under one Ollama module sharing a base URL prefix
/// but with distinct hosts: native (`/`) and OpenAI-compatible (`/v1`).
/// Verifies grouped base-URL aliasing (`baseUrl`, `baseUrl2`) and merged
/// folders.
fn build_grouped_module_ollama() -> Value {
    let native = rest_api(
        "OllamaNative",
        "http://localhost:11434",
        AuthStrategy::None,
        Some("ollama"),
        vec![endpoint(
            "ListLocalModels",
            RestMethod::Get,
            "/api/tags",
            "List locally installed models",
            None,
            ApiResponse::json_type("TagsResponse"),
            None,
        )],
    );

    let openai_compat = rest_api(
        "OllamaOpenAI",
        "http://localhost:11434/v1",
        AuthStrategy::None,
        Some("ollama"),
        vec![endpoint(
            "ListModels",
            RestMethod::Get,
            "/models",
            "OpenAI-compatible model listing",
            None,
            ApiResponse::json_type("ListModelsResponse"),
            None,
        )],
    );

    let collection = build_postman_collection_grouped(&[&native, &openai_compat], "ollama");
    serde_json::to_value(&collection).expect("serialize collection")
}

// --- Tests -----------------------------------------------------------------

#[test]
fn golden_mixed_auth_emqx() {
    let value = build_mixed_auth_emqx();

    // Behavioural sanity checks (in addition to strict equality below):
    // collection-level auth must be omitted, every request must carry its
    // own auth, and at least one duplicate id must be disambiguated.
    assert!(
        value.get("auth").is_none() || value.get("auth").map(Value::is_null).unwrap_or(false),
        "mixed-auth collection must not declare a collection-level auth"
    );
    let request_names: Vec<String> = walk_request_names(&value);
    let alarms = request_names
        .iter()
        .filter(|n| n.starts_with("ListAlarms"))
        .count();
    assert_eq!(
        alarms, 2,
        "expected two ListAlarms requests after disambiguation, got {} ({:?})",
        alarms, request_names,
    );
    assert!(
        request_names.iter().any(|n| n.contains("(EmqxBasic)")),
        "expected disambiguated name suffix, got {:?}",
        request_names,
    );

    assert_golden("mixed_auth_emqx", &value);
}

#[test]
fn golden_auth_variables_openai() {
    let value = build_auth_variables_openai();

    let var_keys: Vec<String> = value
        .get("variable")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        var_keys.contains(&"bearerToken".to_string()),
        "expected bearerToken in collection.variable, got {:?}",
        var_keys,
    );

    assert_golden("auth_variables_openai", &value);
}

#[test]
fn golden_file_upload_elevenlabs() {
    let value = build_file_upload_elevenlabs();

    // Find the audio field type in the body's formdata array. The endpoint
    // is wrapped in a `Voices` folder, so we search the JSON tree rather
    // than hard-coding a JSON pointer.
    let upload_request = find_request_by_name(&value, "UploadSample")
        .expect("UploadSample request must exist in fixture");
    let audio_type = upload_request
        .pointer("/body/formdata/0/type")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert_eq!(
        audio_type, "file",
        "expected first formdata entry (audio) to have type=file, got {:?}",
        audio_type,
    );

    assert_golden("file_upload_elevenlabs", &value);
}

#[test]
fn golden_path_query_params_github() {
    let value = build_path_query_params_github();

    // Path variables: ListRepoIssues should expose owner and repo.
    let request = find_request_by_name(&value, "ListRepoIssues")
        .expect("ListRepoIssues request must exist in fixture");
    let path_vars: Vec<String> = request
        .pointer("/url/variable")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        path_vars.contains(&"owner".to_string()) && path_vars.contains(&"repo".to_string()),
        "expected path variables owner+repo, got {:?}",
        path_vars,
    );

    let query_keys: Vec<String> = request
        .pointer("/url/query")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        query_keys.contains(&"state".to_string()),
        "expected query param 'state', got {:?}",
        query_keys,
    );

    assert_golden("path_query_params_github", &value);
}

#[test]
fn golden_api_key_query_auth() {
    let value = build_api_key_query_auth();

    // Behavioural sanity checks:
    // auth.apikey must contain "in": "query" and "key": "api_key"
    let auth = value.get("auth").expect("collection must have auth");
    let apikey = auth
        .get("apikey")
        .and_then(Value::as_array)
        .expect("auth must have apikey array");
    let in_entry = apikey
        .iter()
        .find(|v| v.get("key").and_then(Value::as_str) == Some("in"))
        .expect("apikey must have 'in' entry");
    assert_eq!(
        in_entry.get("value").and_then(Value::as_str),
        Some("query"),
        "expected 'in' to be 'query' for ApiKeyParam with Query location"
    );
    let key_entry = apikey
        .iter()
        .find(|v| v.get("key").and_then(Value::as_str) == Some("key"))
        .expect("apikey must have 'key' entry");
    assert_eq!(
        key_entry.get("value").and_then(Value::as_str),
        Some("api_key"),
        "expected 'key' to be 'api_key'"
    );

    assert_golden("api_key_query_auth", &value);
}

#[test]
fn golden_grouped_module_ollama() {
    let value = build_grouped_module_ollama();

    let var_keys: Vec<String> = value
        .get("variable")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("key").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        var_keys.contains(&"baseUrl".to_string()) && var_keys.contains(&"baseUrl2".to_string()),
        "expected baseUrl + baseUrl2 in grouped collection, got {:?}",
        var_keys,
    );

    // Verify OllamaOpenAI's ListModels uses baseUrl2, not baseUrl
    let list_models_req =
        find_request_by_name(&value, "ListModels").expect("ListModels request must exist");
    let raw_url = list_models_req.pointer("/url/raw").and_then(Value::as_str);
    assert_eq!(
        raw_url,
        Some("{{baseUrl2}}/models"),
        "OllamaOpenAI request must use baseUrl2"
    );
    let host = list_models_req
        .pointer("/url/host")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert_eq!(
        host,
        vec!["{{baseUrl2}}"],
        "OllamaOpenAI host must use baseUrl2"
    );

    assert_golden("grouped_module_ollama", &value);
}

// --- Helpers used by behavioural assertions --------------------------------

/// Returns the first `request` object whose sibling `name` matches the
/// requested id. Lets behavioural assertions look up requests without
/// pinning a brittle JSON pointer to the folder structure.
fn find_request_by_name<'a>(value: &'a Value, target: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            if let Some(name) = map.get("name").and_then(Value::as_str)
                && name == target
                && let Some(req) = map.get("request")
            {
                return Some(req);
            }
            for v in map.values() {
                if let Some(found) = find_request_by_name(v, target) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => {
            for v in items {
                if let Some(found) = find_request_by_name(v, target) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

/// Walks the JSON tree and collects every `name` field that sits beside
/// a `request` object. This matches Postman's request-item shape.
fn walk_request_names(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    walk_request_names_inner(value, &mut out);
    out
}

fn walk_request_names_inner(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if map.contains_key("request")
                && let Some(name) = map.get("name").and_then(Value::as_str)
            {
                out.push(name.to_string());
            }
            for v in map.values() {
                walk_request_names_inner(v, out);
            }
        }
        Value::Array(items) => {
            for v in items {
                walk_request_names_inner(v, out);
            }
        }
        _ => {}
    }
}
