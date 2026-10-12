//! The fresh-output guard: code the generator emits *now* must compile.
//!
//! Each `slow_` test generates a client crate into a temporary directory
//! outside the checkout and builds it offline through
//! [`crate::common::fresh_output`]. The fixture test also runs the
//! generated clients against a local wiremock server (port 0) and asserts
//! the bytes it receives: bodies, headers, the one Cookie header, query
//! pairs, and where each credential goes. They are `slow_` because they invoke
//! Cargo; that keeps them out of `sanity` and off CI archive consumers, which
//! have no Cargo. The CI build job, which does, runs them before it produces
//! the test archive (`just test-fresh-output`).
//!
//! The 18-API test also runs a partial generation of one Ollama member over
//! the fresh output, then compares the result with the committed
//! `schematic/schema/src` and compiles it; generating all 18 APIs takes longer
//! than an ordinary L1 test may. The exported OpenAPI and Postman
//! documents are compared by `artifact_drift`, and the generated manifest by
//! `cargo_gen`'s unit tests.

use std::path::Path;

use schematic_define::{
    ApiKeyLocation, ApiRequest, ApiResponse, AuthMethod, AuthPolicy, AuthStrategy, Endpoint,
    EndpointParams, FormField, OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, ParamDef,
    ParamStyle, PkceRequirement, QueryParamType, RestApi, RestMethod,
};
use schematic_gen::output::{RustOutputRequest, Selection, get_module_path, prepare_rust_output};
use schematic_gen::pipeline::resolve_all_apis;

use crate::common::fresh_output::{
    Definitions, FreshCrate, OFFLINE_CACHE_HINT, TreeSnapshot, diff_summary, list_dir,
    offline_cache_hint, schematic_root, snapshot_tree,
};

/// The fixture APIs and the generator features each one covers.
const FIXTURE_COVERAGE: &[(&str, &[&str])] = &[
    (
        "FixtureBearer",
        &[
            "bearer-auth",
            "json-body",
            "query-parameters",
            "array-query-parameter",
            "header-parameters",
            "cookie-parameters",
        ],
    ),
    ("FixtureApiKey", &["api-key-header-auth", "multipart-body", "url-encoded-body"]),
    (
        "FixtureBasic",
        &[
            "basic-auth",
            "empty-response",
            "text-response",
            "binary-response",
            "text-body",
            "binary-body",
        ],
    ),
    ("FixtureQueryKey", &["api-key-param-query-auth"]),
    ("FixtureCookieKey", &["api-key-param-cookie-auth"]),
    ("FixturePolicyKey", &["api-key-param-explicit-only-policy"]),
    ("FixtureOAuth", &["oauth-auth"]),
];

/// Request and response types for the fixture APIs, one module per API.
const FIXTURE_TYPES: &[(&str, &str)] = &[
    (
        "fixture_bearer",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Widget {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CreateWidget {
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WidgetList {
    pub items: Vec<Widget>,
}
"#,
    ),
    (
        "fixture_api_key",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Accepted {
    pub ok: bool,
}
"#,
    ),
    ("fixture_basic", ""),
    ("fixture_query_key", ""),
    ("fixture_cookie_key", ""),
    ("fixture_policy_key", ""),
    ("fixture_oauth", ""),
];

fn endpoint(id: &str, method: RestMethod, path: &str, response: ApiResponse) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: format!("{id} fixture endpoint"),
        request: None,
        response,
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }
}

fn fixture_api(name: &str, module: &str, auth: AuthStrategy, endpoints: Vec<Endpoint>) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{name} fixture API"),
        base_url: "http://127.0.0.1:1".to_string(),
        docs_url: None,
        auth,
        auth_policy: None,
        env_auth: vec![format!("{}_TOKEN", module.to_uppercase())],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints,
        module_path: Some(module.to_string()),
        request_suffix: None,
        version: None,
    }
}

fn named_param(name: &str, param_type: QueryParamType) -> ParamDef {
    ParamDef {
        name: name.to_string(),
        required: false,
        description: None,
        param_type,
        explode: false,
        style: ParamStyle::Simple,
    }
}

fn located_params(query: Vec<ParamDef>, header: Vec<ParamDef>, cookie: Vec<ParamDef>) -> EndpointParams {
    EndpointParams { query, header, cookie, ..Default::default() }
}

fn fixture_apis() -> Vec<RestApi> {
    let mut list_widgets = endpoint(
        "ListWidgets",
        RestMethod::Get,
        "/widgets",
        ApiResponse::json_type("WidgetList"),
    );
    list_widgets.params = Some(EndpointParams::default().with_query_params(vec![
        ("limit", QueryParamType::Integer, false, None),
        ("q", QueryParamType::String, false, None),
        ("tag", QueryParamType::Array(Box::new(QueryParamType::String)), false, None),
    ]));
    let mut create_widget =
        endpoint("CreateWidget", RestMethod::Post, "/widgets", ApiResponse::json_type("Widget"));
    create_widget.request = Some(ApiRequest::json_type("CreateWidget"));
    let get_widget = endpoint(
        "GetWidget",
        RestMethod::Get,
        "/widgets/{widget_id}",
        ApiResponse::json_type("Widget"),
    );
    // Static endpoint headers, including a Cookie, that parameters must
    // override (header) or merge with (cookie).
    let mut search = endpoint("SearchWidgets", RestMethod::Get, "/widgets/search", ApiResponse::Empty);
    search.headers = vec![
        ("X-Mode".to_string(), "static".to_string()),
        ("Cookie".to_string(), "theme=dark; session=static".to_string()),
    ];
    search.params = Some(located_params(
        vec![named_param("q", QueryParamType::String)],
        vec![
            named_param("X-Trace-Id", QueryParamType::String),
            named_param("x-mode", QueryParamType::String),
            named_param("X-Ids", QueryParamType::Array(Box::new(QueryParamType::Integer))),
        ],
        vec![
            named_param("session", QueryParamType::String),
            named_param("prefs", QueryParamType::Array(Box::new(QueryParamType::String))),
            named_param("note", QueryParamType::String),
        ],
    ));

    let mut upload = endpoint("UploadFile", RestMethod::Post, "/files", ApiResponse::json_type("Accepted"));
    upload.request = Some(ApiRequest::form_data(vec![
        FormField::text("purpose"),
        FormField::file("file"),
    ]));
    let mut submit = endpoint("SubmitForm", RestMethod::Post, "/forms", ApiResponse::json_type("Accepted"));
    submit.request = Some(ApiRequest::url_encoded(vec![
        FormField::text("name"),
        FormField::text("note").optional(),
    ]));

    let mut send_note = endpoint("SendNote", RestMethod::Post, "/notes", ApiResponse::Empty);
    send_note.request = Some(ApiRequest::Text {
        content_type: "text/plain; charset=utf-8".to_string(),
    });
    let mut upload_blob = endpoint("UploadBlob", RestMethod::Put, "/blobs/{name}", ApiResponse::Empty);
    upload_blob.request = Some(ApiRequest::Binary {
        content_type: "application/octet-stream".to_string(),
    });

    // The declared `api_key` query parameter shares the credential's name.
    let mut list_things = endpoint("ListThings", RestMethod::Get, "/things", ApiResponse::Empty);
    list_things.params = Some(located_params(
        vec![named_param("q", QueryParamType::String), named_param("api_key", QueryParamType::String)],
        vec![],
        vec![],
    ));
    // The declared `sid` cookie parameter shares the credential's name.
    let mut get_profile = endpoint("GetProfile", RestMethod::Get, "/profile", ApiResponse::Empty);
    get_profile.params = Some(located_params(
        vec![],
        vec![],
        vec![named_param("sid", QueryParamType::String), named_param("theme", QueryParamType::String)],
    ));

    let mut bearer = fixture_api(
        "FixtureBearer",
        "fixture_bearer",
        AuthStrategy::BearerToken { header: None },
        vec![list_widgets, create_widget, get_widget, search],
    );
    bearer.headers = vec![
        ("X-Client".to_string(), "fixture".to_string()),
        ("Cookie".to_string(), "api=1; theme=api".to_string()),
    ];
    let mut cookie_key = fixture_api(
        "FixtureCookieKey",
        "fixture_cookie_key",
        AuthStrategy::ApiKeyParam { name: "sid".to_string(), location: ApiKeyLocation::Cookie },
        vec![get_profile],
    );
    cookie_key.headers = vec![("Cookie".to_string(), "lang=en; sid=static".to_string())];
    // The legacy strategy says "no auth"; the effective policy accepts only
    // an explicit query key, so its env fallback is disabled.
    let mut policy_key = fixture_api(
        "FixturePolicyKey",
        "fixture_policy_key",
        AuthStrategy::None,
        vec![endpoint("Ping", RestMethod::Get, "/ping", ApiResponse::Empty)],
    );
    policy_key.auth_policy = Some(AuthPolicy {
        explicit: vec![AuthMethod::ApiKeyParam {
            name: "token".to_string(),
            location: ApiKeyLocation::Query,
        }],
        env_fallback: None,
    });
    let oauth = OAuth2Config {
        grant_type: OAuth2GrantType::AuthorizationCodePkce,
        authorization_url: Some("http://127.0.0.1:1/authorize".to_string()),
        token_url: "http://127.0.0.1:1/token".to_string(),
        revocation_url: None,
        device_authorization_url: None,
        default_scopes: vec!["read".to_string()],
        pkce: PkceRequirement::Required,
        client_auth: OAuth2ClientAuthMethod::None,
    };

    vec![
        bearer,
        fixture_api(
            "FixtureApiKey",
            "fixture_api_key",
            AuthStrategy::ApiKey { header: "X-Api-Key".to_string(), value_prefix: None },
            vec![upload, submit],
        ),
        fixture_api(
            "FixtureBasic",
            "fixture_basic",
            AuthStrategy::Basic,
            vec![
                endpoint("DeleteThing", RestMethod::Delete, "/things/{id}", ApiResponse::Empty),
                endpoint("GetNote", RestMethod::Get, "/note", ApiResponse::Text),
                endpoint("GetBlob", RestMethod::Get, "/blob", ApiResponse::Binary),
                send_note,
                upload_blob,
            ],
        ),
        fixture_api(
            "FixtureQueryKey",
            "fixture_query_key",
            AuthStrategy::ApiKeyParam { name: "api_key".to_string(), location: ApiKeyLocation::Query },
            vec![list_things],
        ),
        cookie_key,
        policy_key,
        fixture_api(
            "FixtureOAuth",
            "fixture_oauth",
            AuthStrategy::OAuth2(oauth),
            vec![endpoint("WhoAmI", RestMethod::Get, "/me", ApiResponse::Empty)],
        ),
    ]
}

/// Checkout state the guard could plausibly disturb, compared before and
/// after a child build. The committed generated crate is the output a
/// misdirected generator would overwrite; the listings of the area, the
/// generator crate (the tests' working directory), and both path
/// dependencies catch a stray `Cargo.lock`, `target/`, or wrong-directory
/// output tree. Nothing else in the monorepo is scanned.
#[derive(Debug, PartialEq)]
struct CheckoutWitness {
    schema_crate: TreeSnapshot,
    listings: Vec<(String, Vec<String>)>,
}

impl CheckoutWitness {
    fn capture(area: &Path) -> Self {
        let mut schema_crate = TreeSnapshot::new();
        for entry in list_dir(&area.join("schema")) {
            if entry == "target" {
                continue;
            }
            let path = area.join("schema").join(&entry);
            if path.is_dir() {
                schema_crate.insert(entry.clone(), None);
                for (relative, bytes) in snapshot_tree(&path) {
                    schema_crate.insert(format!("{entry}/{relative}"), bytes);
                }
            } else {
                schema_crate.insert(entry, Some(std::fs::read(&path).expect("read schema file")));
            }
        }
        let listings = ["", "gen", "define", "definitions"]
            .iter()
            .map(|dir| (dir.to_string(), list_dir(&area.join(dir))))
            .collect();
        Self { schema_crate, listings }
    }

    fn assert_unchanged(&self, after: &CheckoutWitness) {
        let changed = diff_summary(&self.schema_crate, &after.schema_crate);
        assert!(changed.is_empty(), "the child build changed schematic/schema: {changed:#?}");
        assert_eq!(self.listings, after.listings, "the child build changed a checkout listing");
    }
}

#[test]
fn fixture_coverage_names_every_fixture_api() {
    let apis = fixture_apis();
    let names: Vec<&str> = apis.iter().map(|api| api.name.as_str()).collect();
    let covered: Vec<&str> = FIXTURE_COVERAGE.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, covered);
    let modules: Vec<String> = apis.iter().filter_map(|api| api.module_path.clone()).collect();
    let typed: Vec<&str> = FIXTURE_TYPES.iter().map(|(module, _)| *module).collect();
    assert_eq!(modules, typed);
}

/// Runtime tests compiled into the fixture crate. Each one sends requests
/// through a freshly generated client to a local wiremock server bound to
/// port 0 and asserts what the server received. The child process gets the
/// `FIXTURE_*` variables from [`FIXTURE_ENV`]; nothing mutates the
/// environment at run time.
const REQUEST_SIDE_TEST: &str = r#"
use schematic_schema::fixture_api_key::{FixtureApiKey, SubmitFormForm, SubmitFormRequest};
use schematic_schema::fixture_basic::{FixtureBasic, SendNoteRequest, UploadBlobRequest};
use schematic_schema::fixture_bearer::{FixtureBearer, SearchWidgetsRequest};
use schematic_schema::fixture_cookie_key::{FixtureCookieKey, GetProfileRequest};
use schematic_schema::fixture_oauth::{FixtureOAuth, WhoAmIRequest};
use schematic_schema::fixture_policy_key::{FixturePolicyKey, PingRequest};
use schematic_schema::fixture_query_key::{FixtureQueryKey, ListThingsRequest};
use schematic_schema::shared::SchematicError;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

async fn start_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;
    server
}

async fn only_request(server: &MockServer) -> Request {
    let mut received = server.received_requests().await.expect("recording is on");
    assert_eq!(received.len(), 1, "exactly one request reaches the server");
    received.remove(0)
}

async fn assert_nothing_sent(server: &MockServer) {
    assert!(server.received_requests().await.unwrap().is_empty(), "nothing may be sent");
}

fn header(request: &Request, name: &str) -> Vec<String> {
    request
        .headers
        .get_all(name)
        .iter()
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .collect()
}

fn query(request: &Request) -> String {
    request.url.query().unwrap_or_default().to_string()
}

// ---- R21, R22: raw bodies ----

#[tokio::test]
async fn text_body_reaches_the_server_verbatim_under_its_content_type() {
    for text in ["", "line one\nzwei – ünïcödé\r\n"] {
        let server = start_server().await;
        let client = FixtureBasic::with_base_url(server.uri()).basic_auth("u", "p");
        client.send_note(SendNoteRequest::new(text.to_string())).await.unwrap();

        let request = only_request(&server).await;
        assert_eq!(request.method.as_str(), "POST");
        assert_eq!(request.url.path(), "/notes");
        assert_eq!(request.body, text.as_bytes());
        assert_eq!(header(&request, "content-type"), ["text/plain; charset=utf-8"]);
    }
}

#[tokio::test]
async fn binary_body_reaches_the_server_byte_for_byte() {
    let not_utf8 = vec![0u8, 0x9F, 0x92, 0x96, 0xFF, b'\n', b'\r', 0xC3];
    assert!(String::from_utf8(not_utf8.clone()).is_err());
    for bytes in [not_utf8, Vec::new()] {
        let server = start_server().await;
        let client = FixtureBasic::with_base_url(server.uri()).basic_auth("u", "p");
        client.upload_blob(UploadBlobRequest::new("a b", bytes.clone())).await.unwrap();

        let request = only_request(&server).await;
        assert_eq!(request.method.as_str(), "PUT");
        assert_eq!(request.url.path(), "/blobs/a%20b");
        assert_eq!(request.body, bytes);
        assert_eq!(header(&request, "content-type"), ["application/octet-stream"]);
        assert_eq!(header(&request, "authorization"), ["Basic dTpw"]);
    }
}

// ---- R25: header parameters ----

#[tokio::test]
async fn set_header_parameters_override_static_headers_by_case_insensitive_name() {
    let server = start_server().await;
    let client = FixtureBearer::with_base_url(server.uri()).bearer_token("t");
    let request = SearchWidgetsRequest::default()
        .with_q("a b".to_string())
        .with_x_trace_id("abc 123".to_string())
        .with_x_mode("param".to_string())
        .with_x_ids(vec![1, 22, 333]);
    client.search_widgets(request).await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "x-trace-id"), ["abc 123"]);
    assert_eq!(header(&request, "x-mode"), ["param"], "the parameter replaces the static header");
    assert_eq!(header(&request, "x-ids"), ["1,22,333"]);
    assert_eq!(header(&request, "x-client"), ["fixture"]);
    assert_eq!(header(&request, "authorization"), ["Bearer t"]);
    assert_eq!(query(&request), "q=a%20b", "header values never join the query");
}

#[tokio::test]
async fn unset_header_parameters_keep_static_headers_and_send_nothing() {
    let server = start_server().await;
    let client = FixtureBearer::with_base_url(server.uri()).bearer_token("t");
    client.search_widgets(SearchWidgetsRequest::default()).await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "x-mode"), ["static"]);
    assert!(header(&request, "x-trace-id").is_empty());
    assert!(header(&request, "x-ids").is_empty());
    assert_eq!(query(&request), "");
}

#[tokio::test]
async fn invalid_header_parameter_values_fail_before_send_without_echoing_them() {
    // Control characters are invalid; non-ASCII bytes are RFC 9110 `obs-text`,
    // which the HTTP types accept and send as raw UTF-8.
    for value in ["bad\r\nX-Injected: 1", "nul\u{0}byte", "del\u{7f}"] {
        let server = start_server().await;
        let client = FixtureBearer::with_base_url(server.uri()).bearer_token("t");
        let request = SearchWidgetsRequest::default().with_x_trace_id(value.to_string());
        let error = client.search_widgets(request).await.unwrap_err();

        assert!(
            matches!(
                &error,
                SchematicError::Header(schematic_define::HeaderError::InvalidHeaderValue(name))
                    if name == "X-Trace-Id"
            ),
            "{error:?}"
        );
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(value), "the value leaked: {rendered}");
        }
        assert_nothing_sent(&server).await;
    }
}

// ---- R26: cookie parameters ----

#[tokio::test]
async fn cookies_from_api_endpoint_and_parameters_share_one_header() {
    let server = start_server().await;
    let client = FixtureBearer::with_base_url(server.uri()).bearer_token("t");
    let request = SearchWidgetsRequest::default()
        .with_session("a b;c\r\nX-Evil: 1".to_string())
        .with_prefs(vec!["x,y".to_string(), "é".to_string(), "%".to_string()]);
    client.search_widgets(request).await.unwrap();

    let request = only_request(&server).await;
    // API cookies first, then the endpoint's (`theme` replaced in place),
    // then set parameters (`session` replaced in place, `prefs` appended).
    assert_eq!(
        header(&request, "cookie"),
        ["api=1; theme=dark; session=a%20b%3Bc%0D%0AX-Evil:%201; prefs=x%2Cy,%C3%A9,%"]
    );
    assert!(header(&request, "x-evil").is_empty());
}

#[tokio::test]
async fn unset_cookie_parameters_still_merge_api_and_endpoint_cookies() {
    let server = start_server().await;
    let client = FixtureBearer::with_base_url(server.uri()).bearer_token("t");
    client.search_widgets(SearchWidgetsRequest::default()).await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "cookie"), ["api=1; theme=dark; session=static"]);
}

// ---- R23, R24, R26: API-key parameters ----

#[tokio::test]
async fn explicit_query_key_wins_over_env_and_owns_its_query_name() {
    let server = start_server().await;
    let client = FixtureQueryKey::with_base_url(server.uri()).with_api_key_param("SENTINEL key&=");
    let request = ListThingsRequest::default()
        .with_q("x y".to_string())
        .with_api_key("declared".to_string());
    client.list_things(request).await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(query(&request), "q=x%20y&api_key=SENTINEL%20key%26%3D");
    assert!(header(&request, "cookie").is_empty());
}

#[tokio::test]
async fn env_query_key_is_read_from_the_definition_env_names() {
    let server = start_server().await;
    FixtureQueryKey::with_base_url(server.uri())
        .list_things(ListThingsRequest::default())
        .await
        .unwrap();

    assert_eq!(query(&only_request(&server).await), "api_key=env-query-key");
}

#[tokio::test]
async fn the_first_non_empty_env_variable_supplies_the_key() {
    let server = start_server().await;
    let client = FixtureQueryKey::with_base_url(server.uri())
        .variant()
        .env_auth(vec!["FIXTURE_EMPTY_KEY".to_string(), "FIXTURE_SECOND_KEY".to_string()])
        .build();
    client.list_things(ListThingsRequest::default()).await.unwrap();

    assert_eq!(query(&only_request(&server).await), "api_key=second-key");
}

#[tokio::test]
async fn a_missing_key_is_authentication_required_and_nothing_is_sent() {
    let server = start_server().await;
    let client = FixtureQueryKey::with_base_url(server.uri())
        .variant()
        .env_auth(vec!["SCHEMATIC_FIXTURE_UNSET_KEY_7F3A".to_string()])
        .build();
    let error = client.list_things(ListThingsRequest::default()).await.unwrap_err();

    let SchematicError::AuthenticationRequired { explicit_methods, env_fallback_vars, .. } = &error
    else {
        panic!("expected AuthenticationRequired, got {error:?}");
    };
    assert_eq!(env_fallback_vars, &["SCHEMATIC_FIXTURE_UNSET_KEY_7F3A"]);
    assert!(explicit_methods[0].contains("api_key"), "{explicit_methods:?}");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn cookie_key_replaces_the_same_named_cookie_in_place() {
    let server = start_server().await;
    let client = FixtureCookieKey::with_base_url(server.uri()).with_api_key_param("SENTINEL;v");
    let request = GetProfileRequest::default()
        .with_sid("param".to_string())
        .with_theme("dark".to_string());
    client.get_profile(request).await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "cookie"), ["lang=en; sid=SENTINEL%3Bv; theme=dark"]);
    assert_eq!(query(&request), "");
}

#[tokio::test]
async fn env_cookie_key_is_appended_when_no_cookie_has_its_name() {
    let server = start_server().await;
    let client = FixtureCookieKey::with_base_url(server.uri())
        .variant_with_headers(schematic_define::Headers::default())
        .variant()
        .env_auth(vec!["FIXTURE_COOKIE_KEY_TOKEN".to_string()])
        .build();
    client.get_profile(GetProfileRequest::default().with_theme("t".to_string())).await.unwrap();

    assert_eq!(header(&only_request(&server).await, "cookie"), ["theme=t; sid=env-cookie-key"]);
}

#[tokio::test]
async fn a_policy_without_env_fallback_reads_no_variable() {
    let server = start_server().await;
    let error = FixturePolicyKey::with_base_url(server.uri())
        .ping(PingRequest::default())
        .await
        .unwrap_err();
    assert!(matches!(error, SchematicError::AuthenticationRequired { .. }), "{error:?}");
    assert_nothing_sent(&server).await;

    let client = FixturePolicyKey::with_base_url(server.uri()).with_api_key_param("explicit");
    client.ping(PingRequest::default()).await.unwrap();
    assert_eq!(query(&only_request(&server).await), "token=explicit");
}

// ---- R27: no credential in Debug output ----

#[tokio::test]
async fn debug_output_never_contains_a_credential() {
    let header_key = FixtureApiKey::new().api_key("SENTINEL-HEADER");
    let query_key = FixtureQueryKey::new().with_api_key_param("SENTINEL-QUERY");
    let cookie_key = FixtureCookieKey::new().with_api_key_param("SENTINEL-COOKIE");
    let bearer = FixtureBearer::new().bearer_token("SENTINEL-BEARER");
    for (sentinel, rendered) in [
        ("SENTINEL-HEADER", format!("{header_key:?}")),
        ("SENTINEL-QUERY", format!("{query_key:?}")),
        ("SENTINEL-COOKIE", format!("{cookie_key:?}")),
        ("SENTINEL-BEARER", format!("{bearer:?}")),
        ("env-query-key", format!("{:?}", FixtureQueryKey::new())),
    ] {
        assert!(!rendered.contains(sentinel), "{sentinel} leaked: {rendered}");
        assert!(rendered.contains("base_url"), "{rendered}");
    }
    // A request struct carries parameters, never the client's key.
    let request = ListThingsRequest::default().with_q("x".to_string());
    assert!(!format!("{request:?}").contains("SENTINEL"));
}

// ---- R19: the supported strategies and bodies still reach the server ----

#[tokio::test]
async fn header_key_oauth_and_form_bodies_reach_the_server() {
    let server = start_server().await;
    let client = FixtureApiKey::with_base_url(server.uri()).api_key("k-1");
    let form = SubmitFormForm { name: "a b".to_string(), note: None };
    client.request::<serde_json::Value>(SubmitFormRequest::new(form)).await.unwrap();
    let request = only_request(&server).await;
    assert_eq!(header(&request, "x-api-key"), ["k-1"]);
    assert_eq!(header(&request, "content-type"), ["application/x-www-form-urlencoded"]);
    assert_eq!(request.body, b"name=a+b");

    let server = start_server().await;
    FixtureOAuth::with_base_url(server.uri())
        .oauth_token("tok")
        .who_am_i(WhoAmIRequest::default())
        .await
        .unwrap();
    assert_eq!(header(&only_request(&server).await, "authorization"), ["Bearer tok"]);
}
"#;

/// Environment for the child test process; see [`REQUEST_SIDE_TEST`].
const FIXTURE_ENV: &[(&str, &str)] = &[
    ("FIXTURE_QUERY_KEY_TOKEN", "env-query-key"),
    ("FIXTURE_COOKIE_KEY_TOKEN", "env-cookie-key"),
    ("FIXTURE_POLICY_KEY_TOKEN", "policy-env-key"),
    ("FIXTURE_EMPTY_KEY", ""),
    ("FIXTURE_SECOND_KEY", "second-key"),
];

/// The runtime tests [`REQUEST_SIDE_TEST`] must report as passed.
const REQUEST_SIDE_TESTS: &[&str] = &[
    "text_body_reaches_the_server_verbatim_under_its_content_type",
    "binary_body_reaches_the_server_byte_for_byte",
    "set_header_parameters_override_static_headers_by_case_insensitive_name",
    "unset_header_parameters_keep_static_headers_and_send_nothing",
    "invalid_header_parameter_values_fail_before_send_without_echoing_them",
    "cookies_from_api_endpoint_and_parameters_share_one_header",
    "unset_cookie_parameters_still_merge_api_and_endpoint_cookies",
    "explicit_query_key_wins_over_env_and_owns_its_query_name",
    "env_query_key_is_read_from_the_definition_env_names",
    "the_first_non_empty_env_variable_supplies_the_key",
    "a_missing_key_is_authentication_required_and_nothing_is_sent",
    "cookie_key_replaces_the_same_named_cookie_in_place",
    "env_cookie_key_is_appended_when_no_cookie_has_its_name",
    "a_policy_without_env_fallback_reads_no_variable",
    "debug_output_never_contains_a_credential",
    "header_key_oauth_and_form_bodies_reach_the_server",
];

#[test]
fn request_side_test_list_names_every_child_test() {
    let declared: Vec<&str> = REQUEST_SIDE_TEST
        .split("async fn ")
        .skip(1)
        .filter_map(|rest| rest.split('(').next())
        .filter(|name| !["start_server", "only_request", "assert_nothing_sent"].contains(name))
        .collect();
    assert_eq!(declared, REQUEST_SIDE_TESTS);
}

#[test]
fn slow_fixture_apis_compile_and_send_offline() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let area = schematic_root();
    let before = CheckoutWitness::capture(&area);

    let apis = fixture_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Fixture(FIXTURE_TYPES), &lockfile);
    assert!(
        !fresh.crate_dir.starts_with(&area),
        "the fresh crate must live outside the checkout"
    );
    for (module, _) in FIXTURE_TYPES {
        assert!(fresh.src_dir.join(format!("{module}.rs")).is_file(), "missing generated {module}");
    }
    fresh.cargo_check().assert_success();

    fresh.add_test_file("request_side", REQUEST_SIDE_TEST);
    let run = fresh.nextest_run(FIXTURE_ENV);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in REQUEST_SIDE_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }

    before.assert_unchanged(&CheckoutWitness::capture(&area));
    eprintln!("compiled and exercised fixture APIs: {FIXTURE_COVERAGE:?}");
}

#[test]
fn slow_real_apis_compile_offline() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let area = schematic_root();
    let before = CheckoutWitness::capture(&area);

    let apis = resolve_all_apis();
    assert_eq!(apis.len(), 18, "the guard covers every real REST API");
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Checkout, &lockfile);

    // Partial generation of one Ollama member over the full output: both
    // Ollama members are regenerated and every other module is kept as it is.
    // The compile below then builds the retained-plus-regenerated crate.
    let ollama_requests = fresh.src_dir.join("ollama/requests.rs");
    let anthropic = fresh.src_dir.join("anthropic.rs");
    let pristine_anthropic = std::fs::read(&anthropic).unwrap();
    for edited in [&ollama_requests, &anthropic] {
        let mut bytes = std::fs::read(edited).unwrap();
        bytes.extend_from_slice(b"// edited before the partial run\n");
        std::fs::write(edited, bytes).unwrap();
    }
    let edited_anthropic = std::fs::read(&anthropic).unwrap();
    let ollama_native: Vec<&RestApi> =
        api_refs.iter().copied().filter(|api| api.name == "OllamaNative").collect();
    let prepared = prepare_rust_output(&RustOutputRequest {
        registry: &api_refs,
        selection: Selection::Apis(&ollama_native),
        output_dir: &fresh.src_dir,
        manifest_dir: None,
    })
    .expect("prepare the partial run");
    assert_eq!(prepared.regenerated, ["ollama"]);
    let groups: std::collections::BTreeSet<String> =
        api_refs.iter().map(|api| get_module_path(api)).collect();
    assert_eq!(prepared.retained.len(), groups.len() - 1, "every other module group is retained");
    prepared.plan.publish().expect("publish the partial run");
    assert_eq!(std::fs::read(&anthropic).unwrap(), edited_anthropic, "retained module untouched");
    std::fs::write(&anthropic, pristine_anthropic).unwrap();

    let committed = biscuit_test_harness::manifest_dir!().join("../schema/src");
    let differences = diff_summary(&snapshot_tree(&committed), &snapshot_tree(&fresh.src_dir));
    assert!(
        differences.is_empty(),
        "freshly generated Rust differs from the committed schematic/schema/src.\n\
         Run `just -f schematic/justfile generate` to update it.\n{differences:#?}"
    );

    fresh.cargo_check().assert_success();

    before.assert_unchanged(&CheckoutWitness::capture(&area));
}

#[test]
fn slow_negative_control_reports_compiler_error() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let apis = fixture_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Fixture(FIXTURE_TYPES), &lockfile);

    let module = fresh.src_dir.join("fixture_basic.rs");
    let mut source = std::fs::read_to_string(&module).unwrap();
    source.push_str("\npub fn injected_type_error() -> u32 {\n    \"not a number\"\n}\n");
    std::fs::write(&module, source).unwrap();

    let run = fresh.cargo_check();
    assert!(!run.success, "a type error in emitted code must fail the check:\n{}", run.stderr);
    assert!(
        run.stderr.contains("error[E0308]") && run.stderr.contains("injected_type_error"),
        "the compiler error should be shown{}:\n{}",
        offline_cache_hint(&run.stderr).map(|hint| format!(" ({hint})")).unwrap_or_default(),
        run.stderr
    );
}

/// The exact stderr `build-linux` produced when its Cargo cache lacked a crate
/// pinned by `schema/Cargo.lock` gets the fetch hint; a compiler error does not.
#[test]
fn offline_cache_misses_name_the_fetch_that_fills_the_cache() {
    let offline_miss = "error: failed to download `cc v1.2.52`\n\n\
        Caused by:\n  attempting to make an HTTP request, but --offline was specified\n";
    assert_eq!(offline_cache_hint(offline_miss), Some(OFFLINE_CACHE_HINT));
    assert!(
        OFFLINE_CACHE_HINT
            .contains("cargo fetch --manifest-path schematic/schema/Cargo.toml"),
        "{OFFLINE_CACHE_HINT}"
    );

    let compiler_error = "error[E0308]: mismatched types\n --> src/fixture_basic.rs:9:5\n";
    assert_eq!(offline_cache_hint(compiler_error), None);
    assert_eq!(offline_cache_hint(""), None);
}

/// Request lines the pre-split generator put on the wire, captured by a raw
/// TCP listener so no HTTP library normalizes them. The query split must
/// leave every line byte-identical.
const WIRE_BASELINE: &str = include_str!("../fixtures/wire_baseline/query_request_lines.txt");

/// Sends representative requests through the generated real clients to a
/// local listener and records `label<TAB>request line` per request. The same
/// file asserts what `into_parts` returns for those requests and how
/// `append_query_pairs` assembles a URL, so the separated pairs are checked
/// alongside the bytes they produce.
const WIRE_CAPTURE_TEST: &str = r#"
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use schematic_schema::gitea::{GetGitTreeRecursiveRequest, Gitea, ListPullRequestsRequest};
use schematic_schema::gitlab::{GetRepositoryFileRequest, GitLab};
use schematic_schema::openai::{BetaListInputItemsRequest, ListConversationItemsRequest, OpenAI};

fn start_recorder() -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap() == 0 || header == "\r\n" {
                    break;
                }
            }
            tx.send(request_line.trim_end().to_string()).unwrap();
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
            );
        }
    });
    (base, rx)
}

#[tokio::test]
async fn capture_query_request_lines() {
    let (base, rx) = start_recorder();
    let mut lines = Vec::new();
    let mut record = |label: &str| {
        let line = rx.recv_timeout(Duration::from_secs(20)).expect("request reached the listener");
        lines.push(format!("{label}\t{line}"));
    };

    let gitea = Gitea::with_base_url(format!("{base}/api/v1")).api_key("test-key");

    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.page = Some(2);
    scalars.limit = Some(0);
    scalars.state = Some("open closed".to_string());
    scalars.sort = Some("ünï+%&=/?#".to_string());
    let _ = gitea.request::<serde_json::Value>(scalars).await;
    record("ordered-scalars-reserved-unicode");

    let mut empty = ListPullRequestsRequest::new("o", "r");
    empty.state = Some(String::new());
    let _ = gitea.request::<serde_json::Value>(empty).await;
    record("empty-value");

    let _ = gitea.request::<serde_json::Value>(ListPullRequestsRequest::new("o", "r")).await;
    record("no-query");

    let _ = gitea
        .request::<serde_json::Value>(GetGitTreeRecursiveRequest::new("o", "r", "a b"))
        .await;
    record("legacy-embedded-literal-query");

    let tenant = Gitea::with_base_url(format!("{base}/api/v1?tenant=a b")).api_key("test-key");
    let mut paged = ListPullRequestsRequest::new("o", "r");
    paged.page = Some(1);
    let _ = tenant.request::<serde_json::Value>(paged).await;
    record("base-url-with-query");

    let gitlab = GitLab::with_base_url(format!("{base}/api/v4")).api_key("test-key");
    let _ = gitlab
        .request::<serde_json::Value>(GetRepositoryFileRequest::new(
            "group/project",
            "dir/file name.rs",
            "feature/x y+z",
        ))
        .await;
    record("legacy-embedded-query-with-placeholder");

    let openai = OpenAI::with_base_url(format!("{base}/v1")).bearer_token("test-token");
    let mut items = ListConversationItemsRequest::new("conv 1");
    items.limit = Some(5);
    items.order = Some("a+b".to_string());
    items.after = Some("x%y".to_string());
    items.include = Some(vec![
        serde_json::json!("a b"),
        serde_json::json!("c+d"),
        serde_json::json!(""),
        serde_json::json!("é"),
    ]);
    let _ = openai.request::<serde_json::Value>(items).await;
    record("repeated-array-keys");

    let mut beta = BetaListInputItemsRequest::new("resp 1");
    beta.limit = Some(3);
    beta.include = Some(vec![serde_json::json!("x y")]);
    let _ = openai.request::<serde_json::Value>(beta).await;
    record("legacy-embedded-query-plus-declared");

    let out = std::env::var("WIRE_BASELINE_OUT").expect("WIRE_BASELINE_OUT");
    std::fs::write(out, lines.join("\n") + "\n").expect("write captured lines");
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn into_parts_returns_unencoded_query_pairs_apart_from_the_path() {
    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.page = Some(2);
    scalars.limit = Some(0);
    scalars.state = Some("open closed".to_string());
    scalars.sort = Some("ünï+%&=/?#".to_string());
    let (method, path, body, headers, query) = scalars.into_parts().unwrap();
    assert_eq!(method, "GET");
    assert_eq!(path, "/repos/own%20er/re%2Bpo%25/pulls");
    assert_eq!(body, schematic_schema::shared::RequestBody::Empty);
    assert_eq!(
        query,
        pairs(&[("page", "2"), ("limit", "0"), ("state", "open closed"), ("sort", "ünï+%&=/?#")])
    );
    assert!(
        !headers.iter().any(|(name, _)| ["page", "limit", "state", "sort"].contains(&name.as_str())),
        "query parameters must not travel as headers: {headers:?}"
    );

    let mut empty = ListPullRequestsRequest::new("o", "r");
    empty.state = Some(String::new());
    assert_eq!(empty.into_parts().unwrap().4, pairs(&[("state", "")]));

    let (_, path, _, _, query) = ListPullRequestsRequest::new("o", "r").into_parts().unwrap();
    assert_eq!((path.as_str(), query), ("/repos/o/r/pulls", vec![]));

    // Legacy templates keep their embedded query, with its escaping, in the path.
    let (_, path, _, _, query) =
        GetGitTreeRecursiveRequest::new("o", "r", "a b").into_parts().unwrap();
    assert_eq!((path.as_str(), query), ("/repos/o/r/git/trees/a%20b?recursive=true", vec![]));
    let (_, path, _, _, query) =
        GetRepositoryFileRequest::new("group/project", "dir/file name.rs", "feature/x y+z")
            .into_parts()
            .unwrap();
    assert_eq!(
        (path.as_str(), query),
        ("/projects/group%2Fproject/repository/files/dir%2Ffile%20name.rs?ref=feature%2Fx%20y%2Bz", vec![])
    );

    let mut items = ListConversationItemsRequest::new("conv 1");
    items.limit = Some(5);
    items.include = Some(vec![serde_json::json!("a b"), serde_json::json!("")]);
    let (_, path, _, _, query) = items.into_parts().unwrap();
    assert_eq!(path, "/conversations/conv%201/items");
    assert_eq!(query, pairs(&[("limit", "5"), ("include", "\"a b\""), ("include", "\"\"")]));

    let mut beta = BetaListInputItemsRequest::new("resp 1");
    beta.limit = Some(3);
    let (_, path, _, _, query) = beta.into_parts().unwrap();
    assert_eq!(path, "/responses/resp%201/input_items?beta=true");
    assert_eq!(query, pairs(&[("limit", "3")]));
}

#[test]
fn append_query_pairs_encodes_once_and_keeps_existing_queries() {
    use schematic_schema::shared::append_query_pairs;

    let mut url = "http://h/api/v1?tenant=a b/repos/o/r/pulls".to_string();
    append_query_pairs(&mut url, "/repos/o/r/pulls", &pairs(&[("page", "1")]));
    assert_eq!(url, "http://h/api/v1?tenant=a b/repos/o/r/pulls?page=1");

    let mut url = "http://h/trees/x?recursive=true".to_string();
    append_query_pairs(&mut url, "/trees/x?recursive=true", &pairs(&[("a", "b c"), ("a", "")]));
    assert_eq!(url, "http://h/trees/x?recursive=true&a=b%20c&a=");

    let mut url = "http://h/items".to_string();
    append_query_pairs(&mut url, "/items", &[]);
    assert_eq!(url, "http://h/items");

    let mut url = "http://h/items".to_string();
    append_query_pairs(&mut url, "/items", &pairs(&[("$top", "a%20b+c")]));
    assert_eq!(url, "http://h/items?%24top=a%2520b%2Bc", "encoded exactly once, never decoded");
}
"#;

#[test]
fn slow_query_wire_matches_pre_split_baseline() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let apis = resolve_all_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Checkout, &lockfile);
    fresh.add_test_file("wire_capture", WIRE_CAPTURE_TEST);

    // A path the child creates, not a file this process holds open: Windows
    // may refuse a second writer on an open handle.
    let out_dir = tempfile::TempDir::new().unwrap();
    let out_path = out_dir.path().join("request_lines.txt");
    let run = fresh.nextest_run(&[("WIRE_BASELINE_OUT", out_path.to_str().unwrap())]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in [
        "into_parts_returns_unencoded_query_pairs_apart_from_the_path",
        "append_query_pairs_encodes_once_and_keeps_existing_queries",
    ] {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }

    let captured = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        captured, WIRE_BASELINE,
        "the generated clients put different request lines on the wire"
    );
}
