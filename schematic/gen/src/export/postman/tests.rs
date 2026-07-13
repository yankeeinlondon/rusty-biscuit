//! Unit tests for Postman collection generation.

use std::fs;

use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

use crate::export::auth::{ApiKeyLocation, ExportAuth};
use crate::export::body::{ExportBody, FormField, FormFieldExportKind, map_body};
use crate::parser::extract_path_params;

use super::auth::{auth_variables, build_collection_auth};
use super::collection::{build_postman_collection, build_postman_collection_grouped};
use super::request::{
    build_body, build_form_param, build_url, capitalize_folder_name, rest_method_to_string,
};
use super::types::{PostmanCollection, PostmanItem, PostmanRequest, PostmanVariable};
use super::variables::merge_variables;
use super::write_postman;

fn minimal_api() -> RestApi {
    RestApi {
        name: "TestApi".to_string(),
        description: "A test API".to_string(),
        base_url: "https://api.test.com/v1".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![],
        module_path: None,
        request_suffix: None,
        version: None,
        env_mapping: None,
    }
}

#[test]
fn build_minimal_collection() {
    let api = minimal_api();
    let collection = build_postman_collection(&api);

    assert_eq!(collection.info.name, "TestApi");
    assert_eq!(collection.info.description, Some("A test API".to_string()));
    assert_eq!(
        collection.info.schema,
        "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
    );
    assert_eq!(collection.variable.len(), 1);
    assert_eq!(collection.variable[0].key, "baseUrl");
    assert_eq!(
        collection.variable[0].value,
        Some("https://api.test.com/v1".to_string())
    );
}

#[test]
fn build_collection_with_endpoints() {
    let mut api = minimal_api();
    api.endpoints = vec![
        Endpoint {
            id: "GetUser".to_string(),
            method: RestMethod::Get,
            path: "/users/{id}".to_string(),
            description: "Get a user".to_string(),
            request: None,
            response: ApiResponse::json_type("User"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "ListUsers".to_string(),
            method: RestMethod::Get,
            path: "/users".to_string(),
            description: "List all users".to_string(),
            request: None,
            response: ApiResponse::json_type("UserList"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ];

    let collection = build_postman_collection(&api);
    assert_eq!(collection.item.len(), 1); // 1 folder: "users"

    if let PostmanItem::Folder { name, item } = &collection.item[0] {
        assert_eq!(name, "Users");
        assert_eq!(item.len(), 2);
    } else {
        panic!("Expected folder");
    }
}

#[test]
fn auth_bearer_token() {
    let auth = ExportAuth::Bearer {
        variable: "bearerToken".to_string(),
    };
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "bearer");
    assert!(postman_auth.bearer.is_some());
    let bearer = postman_auth.bearer.unwrap();
    assert_eq!(bearer.len(), 1);
    assert_eq!(bearer[0].key, "token");
    assert_eq!(bearer[0].value, Some("{{bearerToken}}".to_string()));
}

#[test]
fn auth_api_key() {
    let auth = ExportAuth::ApiKey {
        header: "X-API-Key".to_string(),
        variable: "apiKey".to_string(),
        location: ApiKeyLocation::Header,
    };
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "apikey");
    assert!(postman_auth.apikey.is_some());
    let apikey = postman_auth.apikey.unwrap();
    assert_eq!(apikey.len(), 3);
    assert_eq!(apikey[0].key, "key");
    assert_eq!(apikey[0].value, Some("X-API-Key".to_string()));
    assert_eq!(apikey[1].key, "value");
    assert_eq!(apikey[1].value, Some("{{apiKey}}".to_string()));
    assert_eq!(apikey[2].key, "in");
    assert_eq!(apikey[2].value, Some("header".to_string()));
}

#[test]
fn auth_api_key_query_location() {
    let auth = ExportAuth::ApiKey {
        header: "api_key".to_string(),
        variable: "apiKey".to_string(),
        location: ApiKeyLocation::Query,
    };
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "apikey");
    let apikey = postman_auth.apikey.unwrap();
    assert_eq!(apikey[2].key, "in");
    assert_eq!(apikey[2].value, Some("query".to_string()));
}

#[test]
fn auth_api_key_cookie_location() {
    let auth = ExportAuth::ApiKey {
        header: "api_key".to_string(),
        variable: "apiKey".to_string(),
        location: ApiKeyLocation::Cookie,
    };
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "apikey");
    let apikey = postman_auth.apikey.unwrap();
    assert_eq!(apikey[2].key, "in");
    assert_eq!(apikey[2].value, Some("cookie".to_string()));
}

#[test]
fn auth_basic() {
    let auth = ExportAuth::Basic {
        username_var: "username".to_string(),
        password_var: "password".to_string(),
    };
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "basic");
    assert!(postman_auth.basic.is_some());
    let basic = postman_auth.basic.unwrap();
    assert_eq!(basic.len(), 2);
    assert_eq!(basic[0].key, "username");
    assert_eq!(basic[0].value, Some("{{username}}".to_string()));
    assert_eq!(basic[1].key, "password");
    assert_eq!(basic[1].value, Some("{{password}}".to_string()));
}

#[test]
fn auth_none() {
    let auth = ExportAuth::None;
    let postman_auth = build_collection_auth(&auth).unwrap();

    assert_eq!(postman_auth.type_field, "noauth");
    assert!(postman_auth.bearer.is_none());
    assert!(postman_auth.apikey.is_none());
    assert!(postman_auth.basic.is_none());
}

#[test]
fn body_json() {
    let body = ExportBody::Json {
        content_type: "application/json".to_string(),
    };
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "raw");
    assert_eq!(postman_body.raw, Some("{}".to_string()));
    assert!(postman_body.options.is_some());
    assert_eq!(postman_body.options.unwrap().raw.language, "json");
}

#[test]
fn body_form_data() {
    let body = ExportBody::FormData {
        fields: vec![
            FormField {
                name: "file".to_string(),
                required: true,
                description: Some("The file".to_string()),
                kind: FormFieldExportKind::File { accept: vec![] },
            },
            FormField {
                name: "name".to_string(),
                required: false,
                description: None,
                kind: FormFieldExportKind::Text,
            },
        ],
    };
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "formdata");
    assert!(postman_body.formdata.is_some());
    let formdata = postman_body.formdata.unwrap();
    assert_eq!(formdata.len(), 2);
    assert_eq!(formdata[0].key, "file");
    assert_eq!(formdata[0].description, Some("The file".to_string()));
    assert_eq!(formdata[0].type_field, "file");
    assert!(formdata[0].value.is_none());
    assert_eq!(formdata[1].key, "name");
    assert_eq!(formdata[1].type_field, "text");
    assert_eq!(formdata[1].value, Some(String::new()));
}

#[test]
fn body_url_encoded() {
    let body = ExportBody::UrlEncoded {
        fields: vec![FormField {
            name: "grant_type".to_string(),
            required: true,
            description: None,
            kind: FormFieldExportKind::Text,
        }],
    };
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "urlencoded");
    assert!(postman_body.urlencoded.is_some());
    let urlencoded = postman_body.urlencoded.unwrap();
    assert_eq!(urlencoded.len(), 1);
    assert_eq!(urlencoded[0].key, "grant_type");
    assert_eq!(urlencoded[0].type_field, "text");
}

#[test]
fn form_param_file_emits_type_file() {
    let field = FormField {
        name: "audio".to_string(),
        required: true,
        description: None,
        kind: FormFieldExportKind::File {
            accept: vec!["audio/*".to_string()],
        },
    };
    let param = build_form_param(&field);
    assert_eq!(param.type_field, "file");
    assert!(param.value.is_none());
    assert_eq!(param.key, "audio");
}

#[test]
fn form_param_files_emits_type_file() {
    let field = FormField {
        name: "samples".to_string(),
        required: true,
        description: None,
        kind: FormFieldExportKind::Files {
            accept: vec![],
            min: Some(1),
            max: Some(10),
        },
    };
    let param = build_form_param(&field);
    assert_eq!(param.type_field, "file");
    assert!(param.value.is_none());
}

#[test]
fn form_param_text_emits_type_text() {
    let field = FormField {
        name: "name".to_string(),
        required: true,
        description: None,
        kind: FormFieldExportKind::Text,
    };
    let param = build_form_param(&field);
    assert_eq!(param.type_field, "text");
    assert_eq!(param.value, Some(String::new()));
}

#[test]
fn form_param_json_emits_type_text() {
    let field = FormField {
        name: "metadata".to_string(),
        required: true,
        description: None,
        kind: FormFieldExportKind::Json,
    };
    let param = build_form_param(&field);
    assert_eq!(param.type_field, "text");
    assert_eq!(param.value, Some(String::new()));
}

#[test]
fn body_form_data_real_elevenlabs_upload() {
    // Mirrors the AddVoiceSample endpoint in
    // schematic_definitions::elevenlabs: an `audio` file part with
    // an MP3/WAV accept pattern and an optional `name` text part.
    let request = schematic_define::request::ApiRequest::form_data(vec![
        schematic_define::request::FormField::file_accept("audio", vec!["audio/*".into()])
            .with_description("Audio file (mp3, wav, ogg, m4a)"),
        schematic_define::request::FormField::text("name")
            .optional()
            .with_description("Name for the sample"),
    ]);
    let body = map_body(&request);
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "formdata");
    let formdata = postman_body.formdata.expect("formdata array");
    assert_eq!(formdata.len(), 2);
    let audio = formdata
        .iter()
        .find(|p| p.key == "audio")
        .expect("audio part present");
    assert_eq!(audio.type_field, "file");
    assert!(audio.value.is_none());
    let name = formdata
        .iter()
        .find(|p| p.key == "name")
        .expect("name part present");
    assert_eq!(name.type_field, "text");
    assert_eq!(name.value, Some(String::new()));
}

#[test]
fn body_form_data_real_unfolded_circle_file_upload() {
    // Mirrors the multiple unfolded_circle/core_rest endpoints that
    // accept `FormField::file("file")` with no MIME restrictions.
    let request = schematic_define::request::ApiRequest::form_data(vec![
        schematic_define::request::FormField::file("file"),
    ]);
    let body = map_body(&request);
    let postman_body = build_body(&body);

    let formdata = postman_body.formdata.expect("formdata array");
    assert_eq!(formdata.len(), 1);
    assert_eq!(formdata[0].key, "file");
    assert_eq!(formdata[0].type_field, "file");
    assert!(formdata[0].value.is_none());
}

#[test]
fn body_text() {
    let body = ExportBody::Text {
        content_type: "text/plain".to_string(),
    };
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "raw");
    assert_eq!(postman_body.raw, Some(String::new()));
    assert!(postman_body.options.is_some());
    assert_eq!(postman_body.options.unwrap().raw.language, "text");
}

#[test]
fn body_binary() {
    let body = ExportBody::Binary;
    let postman_body = build_body(&body);

    assert_eq!(postman_body.mode, "file");
    assert!(postman_body.raw.is_none());
    assert!(postman_body.formdata.is_none());
    assert!(postman_body.urlencoded.is_none());
}

#[test]
fn url_with_path_params() {
    let endpoint = Endpoint {
        id: "GetModel".to_string(),
        method: RestMethod::Get,
        path: "/models/{model}".to_string(),
        description: "Get a model".to_string(),
        request: None,
        response: ApiResponse::json_type("Model"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    };

    let path_params = extract_path_params(&endpoint.path);
    let url = build_url(
        "https://api.test.com/v1",
        &endpoint.path,
        &path_params,
        &endpoint,
        "baseUrl",
    );

    assert_eq!(url.raw, "{{baseUrl}}/models/:model");
    assert_eq!(url.host, vec!["{{baseUrl}}"]);
    assert_eq!(url.path, vec!["models", ":model"]);
    assert_eq!(url.variable.len(), 1);
    assert_eq!(url.variable[0].key, "model");
    assert_eq!(url.variable[0].value, Some("<model>".to_string()));
}

#[test]
fn reserved_path_param_strips_marker_in_collection() {
    let mut api = minimal_api();
    api.endpoints = vec![Endpoint {
        id: "GetModel".to_string(),
        method: RestMethod::Get,
        path: "/models/{+repo_id}".to_string(),
        description: "Get a model".to_string(),
        request: None,
        response: ApiResponse::json_type("Model"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }];

    let collection = build_postman_collection(&api);
    let json = serde_json::to_string(&collection).unwrap();

    // The `+` is a runtime encoding hint; it must never reach the collection.
    assert!(!json.contains("{+"), "reserved marker leaked: {json}");
    assert!(json.contains(":repo_id"), "expected `:repo_id` segment: {json}");
    // The path variable key is the bare name.
    assert!(json.contains("\"key\":\"repo_id\""));
}

#[test]
fn folder_grouping() {
    let mut api = minimal_api();
    api.endpoints = vec![
        Endpoint {
            id: "ListModels".to_string(),
            method: RestMethod::Get,
            path: "/models".to_string(),
            description: String::new(),
            request: None,
            response: ApiResponse::json_type("ModelList"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetModel".to_string(),
            method: RestMethod::Get,
            path: "/models/{model}".to_string(),
            description: String::new(),
            request: None,
            response: ApiResponse::json_type("Model"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetHealth".to_string(),
            method: RestMethod::Get,
            path: "/health".to_string(),
            description: String::new(),
            request: None,
            response: ApiResponse::json_type("Health"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ];

    let collection = build_postman_collection(&api);
    // Should have 2 folders: "Models" and "Health"
    assert_eq!(collection.item.len(), 2);

    // Verify folder names
    let folder_names: Vec<String> = collection
        .item
        .iter()
        .filter_map(|item| {
            if let PostmanItem::Folder { name, .. } = item {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(folder_names, vec!["Health", "Models"]);
}

#[test]
fn rest_method_to_string_conversion() {
    assert_eq!(rest_method_to_string(&RestMethod::Get), "GET");
    assert_eq!(rest_method_to_string(&RestMethod::Post), "POST");
    assert_eq!(rest_method_to_string(&RestMethod::Put), "PUT");
    assert_eq!(rest_method_to_string(&RestMethod::Patch), "PATCH");
    assert_eq!(rest_method_to_string(&RestMethod::Delete), "DELETE");
    assert_eq!(rest_method_to_string(&RestMethod::Head), "HEAD");
    assert_eq!(rest_method_to_string(&RestMethod::Options), "OPTIONS");
}

#[test]
fn capitalize_folder_name_cases() {
    assert_eq!(capitalize_folder_name("models"), "Models");
    assert_eq!(capitalize_folder_name("users"), "Users");
    assert_eq!(capitalize_folder_name(""), "");
    assert_eq!(capitalize_folder_name("a"), "A");
}

#[test]
fn write_postman_dry_run() {
    let api = minimal_api();
    let temp_dir = std::env::temp_dir();
    let path = write_postman(&api, &temp_dir, true).unwrap();

    assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");
}

#[test]
fn write_postman_creates_file() {
    use tempfile::TempDir;

    let api = minimal_api();
    let temp_dir = TempDir::new().unwrap();
    let path = write_postman(&api, temp_dir.path(), false).unwrap();

    assert!(path.exists());
    assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");

    // Verify it's valid JSON
    let content = fs::read_to_string(&path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).unwrap();
}

#[test]
fn postman_collection_json_structure() {
    let mut api = minimal_api();
    api.auth = AuthStrategy::BearerToken { header: None };
    api.endpoints = vec![
        Endpoint {
            id: "ListModels".to_string(),
            method: RestMethod::Get,
            path: "/models".to_string(),
            description: "List all models".to_string(),
            request: None,
            response: ApiResponse::json_type("ModelList"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CreateCompletion".to_string(),
            method: RestMethod::Post,
            path: "/chat/completions".to_string(),
            description: "Create a chat completion".to_string(),
            request: Some(schematic_define::ApiRequest::json_type(
                "CreateCompletionRequest",
            )),
            response: ApiResponse::json_type("Completion"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ];

    let collection = build_postman_collection(&api);
    let json = serde_json::to_string_pretty(&collection).unwrap();

    // Verify key structure
    assert!(json.contains("\"info\""));
    assert!(json.contains("\"variable\""));
    assert!(json.contains("\"auth\""));
    assert!(json.contains("\"item\""));
    assert!(json.contains("\"bearer\""));
    assert!(json.contains("{{baseUrl}}"));
    // The collection variable list must declare bearerToken so the
    // {{bearerToken}} reference in the auth block resolves on import.
    assert!(json.contains("\"bearerToken\""));

    // Print for manual inspection
    println!("\n{}", json);
}

#[test]
fn auth_variables_for_bearer_returns_one_variable_named_bearer_token() {
    let auth = ExportAuth::Bearer {
        variable: "bearerToken".to_string(),
    };
    let vars = auth_variables(&auth);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].key, "bearerToken");
    assert_eq!(vars[0].value, Some(String::new()));
    assert!(vars[0].description.is_some());
}

#[test]
fn auth_variables_for_api_key_returns_api_key_variable() {
    let auth = ExportAuth::ApiKey {
        header: "X-API-Key".to_string(),
        variable: "apiKey".to_string(),
        location: ApiKeyLocation::Header,
    };
    let vars = auth_variables(&auth);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].key, "apiKey");
}

#[test]
fn auth_variables_for_basic_returns_username_and_password() {
    let auth = ExportAuth::Basic {
        username_var: "username".to_string(),
        password_var: "password".to_string(),
    };
    let vars = auth_variables(&auth);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].key, "username");
    assert_eq!(vars[1].key, "password");
}

#[test]
fn auth_variables_for_none_returns_empty() {
    let auth = ExportAuth::None;
    let vars = auth_variables(&auth);
    assert!(vars.is_empty());
}

#[test]
fn build_postman_collection_bearer_declares_bearer_token() {
    let mut api = minimal_api();
    api.auth = AuthStrategy::BearerToken { header: None };
    let collection = build_postman_collection(&api);
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"baseUrl"));
    assert!(
        keys.contains(&"bearerToken"),
        "expected bearerToken in {:?}",
        keys,
    );
}

#[test]
fn build_postman_collection_basic_declares_username_and_password() {
    let mut api = minimal_api();
    api.auth = AuthStrategy::Basic;
    let collection = build_postman_collection(&api);
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"baseUrl"));
    assert!(keys.contains(&"username"), "missing username in {:?}", keys);
    assert!(keys.contains(&"password"), "missing password in {:?}", keys);
}

#[test]
fn build_postman_collection_api_key_declares_api_key_variable() {
    let mut api = minimal_api();
    api.auth = AuthStrategy::ApiKey {
        header: "X-API-Key".to_string(),
        value_prefix: None,
    };
    let collection = build_postman_collection(&api);
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"baseUrl"));
    assert!(keys.contains(&"apiKey"), "missing apiKey in {:?}", keys);
}

#[test]
fn build_postman_collection_none_declares_only_base_url() {
    let api = minimal_api();
    let collection = build_postman_collection(&api);
    assert_eq!(collection.variable.len(), 1);
    assert_eq!(collection.variable[0].key, "baseUrl");
}

#[test]
fn merge_variables_dedupes_by_key() {
    let mut target = vec![PostmanVariable {
        key: "baseUrl".to_string(),
        value: None,
        description: None,
    }];
    merge_variables(
        &mut target,
        vec![
            PostmanVariable {
                key: "baseUrl".to_string(),
                value: Some("dup".to_string()),
                description: None,
            },
            PostmanVariable {
                key: "bearerToken".to_string(),
                value: None,
                description: None,
            },
        ],
    );
    assert_eq!(target.len(), 2);
    assert_eq!(target[0].key, "baseUrl");
    // First-write-wins: the original `baseUrl` (with value=None) survived.
    assert!(target[0].value.is_none());
    assert_eq!(target[1].key, "bearerToken");
}

#[test]
fn build_postman_collection_grouped_uniform_bearer_declares_bearer_token() {
    let mut api1 = minimal_api();
    api1.name = "OneBearer".to_string();
    api1.auth = AuthStrategy::BearerToken { header: None };
    let mut api2 = minimal_api();
    api2.name = "TwoBearer".to_string();
    api2.base_url = "https://api2.test.com/v1".to_string();
    api2.auth = AuthStrategy::BearerToken { header: None };

    let collection = build_postman_collection_grouped(&[&api1, &api2], "test_module");
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"baseUrl"));
    assert!(keys.contains(&"baseUrl2"));
    assert!(
        keys.contains(&"bearerToken"),
        "missing bearerToken in {:?}",
        keys,
    );
    // Bearer should not be declared twice.
    let bearer_count = collection
        .variable
        .iter()
        .filter(|v| v.key == "bearerToken")
        .count();
    assert_eq!(bearer_count, 1);
}

/// Helper: build a minimal endpoint with a given id and path so the
/// grouped-auth tests can construct realistic API surfaces.
fn endpoint(id: &str, path: &str) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method: RestMethod::Get,
        path: path.to_string(),
        description: String::new(),
        request: None,
        response: ApiResponse::json_type("Resp"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }
}

/// Helper: collect every request item out of a built collection so
/// tests can iterate without recursing through the folder enum.
fn collect_requests(collection: &PostmanCollection) -> Vec<(String, &PostmanRequest)> {
    let mut out = Vec::new();
    for item in &collection.item {
        match item {
            PostmanItem::Request { name, request } => {
                out.push((name.clone(), request.as_ref()));
            }
            PostmanItem::Folder { item, .. } => {
                for inner in item {
                    if let PostmanItem::Request { name, request } = inner {
                        out.push((name.clone(), request.as_ref()));
                    }
                }
            }
        }
    }
    out
}

#[test]
fn grouped_uniform_auth_uses_collection_auth_and_no_request_auth() {
    let mut api1 = minimal_api();
    api1.name = "OneBearer".to_string();
    api1.auth = AuthStrategy::BearerToken { header: None };
    api1.endpoints = vec![endpoint("ListA", "/a"), endpoint("ListB", "/b")];

    let mut api2 = minimal_api();
    api2.name = "TwoBearer".to_string();
    api2.base_url = "https://api2.test.com/v1".to_string();
    api2.auth = AuthStrategy::BearerToken { header: None };
    api2.endpoints = vec![endpoint("ListC", "/c")];

    let collection = build_postman_collection_grouped(&[&api1, &api2], "uniform");
    let coll_auth = collection.auth.as_ref().expect("collection auth set");
    assert_eq!(coll_auth.type_field, "bearer");

    for (name, request) in collect_requests(&collection) {
        assert!(
            request.auth.is_none(),
            "request {} should inherit collection auth",
            name,
        );
        // Names are not disambiguated when auth is uniform.
        assert!(
            !name.contains('('),
            "uniform auth should not disambiguate names: {}",
            name,
        );
    }
}

#[test]
fn grouped_mixed_auth_omits_collection_auth_and_emits_per_request() {
    let mut basic = minimal_api();
    basic.name = "BasicApi".to_string();
    basic.auth = AuthStrategy::Basic;
    basic.endpoints = vec![
        endpoint("ListItems", "/items"),
        endpoint("BasicOnly", "/bo"),
    ];

    let mut bearer = minimal_api();
    bearer.name = "BearerApi".to_string();
    bearer.base_url = "https://api2.test.com/v1".to_string();
    bearer.auth = AuthStrategy::BearerToken { header: None };
    bearer.endpoints = vec![
        endpoint("ListItems", "/items"),
        endpoint("BearerOnly", "/be"),
    ];

    let collection = build_postman_collection_grouped(&[&basic, &bearer], "mixed");
    assert!(
        collection.auth.is_none(),
        "mixed-auth collection must omit collection.auth",
    );

    let requests = collect_requests(&collection);
    // ListItems is duplicated; BasicOnly + BearerOnly are unique.
    assert_eq!(requests.len(), 4);

    for (name, request) in &requests {
        let auth = request
            .auth
            .as_ref()
            .unwrap_or_else(|| panic!("request {} missing auth", name));
        // Auth type is whichever the owning API uses.
        if name.contains("BasicApi") || name == "BasicOnly" {
            assert_eq!(auth.type_field, "basic", "{}", name);
        } else if name.contains("BearerApi") || name == "BearerOnly" {
            assert_eq!(auth.type_field, "bearer", "{}", name);
        } else {
            panic!("unexpected request name in mixed group: {}", name);
        }
    }
}

#[test]
fn grouped_mixed_auth_declares_both_auth_variable_sets() {
    let mut basic = minimal_api();
    basic.name = "BasicApi".to_string();
    basic.auth = AuthStrategy::Basic;
    basic.endpoints = vec![endpoint("X", "/x")];

    let mut bearer = minimal_api();
    bearer.name = "BearerApi".to_string();
    bearer.base_url = "https://api2.test.com/v1".to_string();
    bearer.auth = AuthStrategy::BearerToken { header: None };
    bearer.endpoints = vec![endpoint("Y", "/y")];

    let collection = build_postman_collection_grouped(&[&basic, &bearer], "mixed");
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"username"), "missing username in {:?}", keys);
    assert!(keys.contains(&"password"), "missing password in {:?}", keys);
    assert!(
        keys.contains(&"bearerToken"),
        "missing bearerToken in {:?}",
        keys,
    );
}

#[test]
fn grouped_mixed_auth_disambiguates_duplicate_request_names() {
    let mut basic = minimal_api();
    basic.name = "EmqxBasic".to_string();
    basic.auth = AuthStrategy::Basic;
    basic.endpoints = vec![endpoint("ListAlarms", "/alarms")];

    let mut bearer = minimal_api();
    bearer.name = "EmqxBearer".to_string();
    bearer.base_url = "http://localhost:18083/api/v5".to_string();
    bearer.auth = AuthStrategy::BearerToken { header: None };
    bearer.endpoints = vec![endpoint("ListAlarms", "/alarms")];

    let collection = build_postman_collection_grouped(&[&basic, &bearer], "emqx");
    let names: Vec<String> = collect_requests(&collection)
        .into_iter()
        .map(|(n, _)| n)
        .collect();

    assert!(
        names.contains(&"ListAlarms (EmqxBasic)".to_string()),
        "expected disambiguated Basic name in {:?}",
        names,
    );
    assert!(
        names.contains(&"ListAlarms (EmqxBearer)".to_string()),
        "expected disambiguated Bearer name in {:?}",
        names,
    );
    // Verify the bare name no longer appears.
    assert!(
        !names.iter().any(|n| n == "ListAlarms"),
        "bare ListAlarms must be replaced when duplicated: {:?}",
        names,
    );
}

#[test]
fn grouped_mixed_auth_preserves_unique_request_names() {
    let mut basic = minimal_api();
    basic.name = "EmqxBasic".to_string();
    basic.auth = AuthStrategy::Basic;
    basic.endpoints = vec![
        endpoint("ListAlarms", "/alarms"), // duplicated
        endpoint("BasicOnly", "/bo"),      // unique to Basic
    ];

    let mut bearer = minimal_api();
    bearer.name = "EmqxBearer".to_string();
    bearer.base_url = "http://localhost:18083/api/v5".to_string();
    bearer.auth = AuthStrategy::BearerToken { header: None };
    bearer.endpoints = vec![
        endpoint("ListAlarms", "/alarms"), // duplicated
        endpoint("Login", "/login"),       // unique to Bearer
    ];

    let collection = build_postman_collection_grouped(&[&basic, &bearer], "emqx");
    let names: Vec<String> = collect_requests(&collection)
        .into_iter()
        .map(|(n, _)| n)
        .collect();

    // Unique IDs stay verbatim; duplicates are renamed.
    assert!(names.contains(&"BasicOnly".to_string()), "{:?}", names);
    assert!(names.contains(&"Login".to_string()), "{:?}", names);
    assert!(
        names.contains(&"ListAlarms (EmqxBasic)".to_string()),
        "{:?}",
        names,
    );
    assert!(
        names.contains(&"ListAlarms (EmqxBearer)".to_string()),
        "{:?}",
        names,
    );
}

#[test]
fn build_postman_collection_grouped_mixed_auth_unions_variables() {
    let mut api1 = minimal_api();
    api1.name = "BasicApi".to_string();
    api1.auth = AuthStrategy::Basic;
    let mut api2 = minimal_api();
    api2.name = "BearerApi".to_string();
    api2.base_url = "https://api2.test.com/v1".to_string();
    api2.auth = AuthStrategy::BearerToken { header: None };

    let collection = build_postman_collection_grouped(&[&api1, &api2], "mixed_module");
    let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
    assert!(keys.contains(&"username"), "missing username in {:?}", keys);
    assert!(keys.contains(&"password"), "missing password in {:?}", keys);
    assert!(
        keys.contains(&"bearerToken"),
        "missing bearerToken in {:?}",
        keys,
    );
}
