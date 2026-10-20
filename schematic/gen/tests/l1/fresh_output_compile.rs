//! The fresh-output guard: code the generator emits *now* must compile.
//!
//! Each `slow_` test generates a client crate into a temporary directory
//! outside the checkout and builds it offline through
//! [`crate::common::fresh_output`]. The fixture test also runs the
//! generated clients, through `schematic-runtime`, against a local wiremock
//! server (port 0) and asserts the bytes it receives (bodies, headers, the one
//! Cookie header, query pairs, where each credential goes) and what the
//! envelope reports. The misuse test proves that the generated types reject,
//! at compile time, what the old generic client allowed; the ABI test proves
//! that generated code refuses a runtime of another ABI. They are `slow_`
//! because they invoke Cargo; that keeps them out of `sanity` and off CI
//! archive consumers, which have no Cargo. The CI build job, which does, runs
//! them before it produces the test archive (`just test-fresh-output`).
//!
//! The 18-API test also runs a partial generation of one Ollama member over
//! the fresh output, then compares the result with the committed
//! `schematic/schema/src` and compiles it, WebSocket modules included;
//! generating all 18 APIs takes longer than an ordinary L1 test may. The
//! exported OpenAPI and Postman documents are compared by `artifact_drift`,
//! and the generated manifest by `cargo_gen`'s unit tests.

use std::path::Path;

use schematic_define::{
    ApiKeyLocation, ApiRequest, ApiResponse, AuthMethod, AuthPolicy, AuthStrategy, CrossOrigin,
    Endpoint, EndpointParams, FormField, OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType,
    ParamDef, ParamStyle, PkceRequirement, QueryParamType, RestApi, RestMethod,
};
use schematic_define::pagination::{
    Collection, CursorSource, InputParam, KeepAlive, LinkSource, MoreSignal, PageSize,
    Partition, Session, SessionStep, Strategy, ValueSource,
};
use schematic_gen::output::{
    RustOutputRequest, Selection, generate_and_write_all, get_module_path, prepare_rust_output,
};
use schematic_gen::pipeline::resolve_all_apis;

use crate::common::fresh_output::{
    Definitions, FreshCrate, OFFLINE_CACHE_HINT, TreeSnapshot, diff_summary, doctored_runtime,
    list_dir, offline_cache_hint, schematic_root, snapshot_tree,
};

/// The fixture APIs and the generator features each one covers.
const FIXTURE_COVERAGE: &[(&str, &[&str])] = &[
    (
        "FixtureBearer",
        &[
            "bearer-auth",
            "typed-registration-paths",
            "typed-nullable-containers",
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
    (
        "FixtureSettings",
        &["request-id-header", "secret-names", "deny-cross-origin", "query-and-header-secrets"],
    ),
    ("FixtureNames", &["endpoint-method-renames", "keyword-setter", "query-wire-name", "endpoint-cross-origin"]),
    (
        "FixturePages",
        &[
            "page-number-pager",
            "cursor-pager",
            "next-link-pager",
            "adopted-parameters",
            "required-adopted-defaults",
        ],
    ),
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
    (
        "fixture_settings",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Echo {
    pub ok: bool,
}
"#,
    ),
    ("fixture_names", ""),
    (
        "fixture_pages",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntryCursorPage {
    pub data: Vec<Entry>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EntryLinkPage {
    pub values: Vec<Entry>,
    pub next: Option<String>,
}
"#,
    ),
];

fn endpoint(id: &str, method: RestMethod, path: &str, response: ApiResponse) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("{id} fixture endpoint"); fixture.request = None; fixture.response = response; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

fn fixture_api(name: &str, module: &str, auth: AuthStrategy, endpoints: Vec<Endpoint>) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{name} fixture API"); fixture.base_url = "http://127.0.0.1:1".to_string(); fixture.docs_url = None; fixture.auth = auth; fixture.auth_policy = None; fixture.env_auth = vec![format!("{}_TOKEN", module.to_uppercase())]; fixture.env_username = None; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = endpoints; fixture.module_path = Some(module.to_string()); fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) }
}

fn named_param(name: &str, param_type: QueryParamType) -> ParamDef {
    ParamDef { rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required: false,
        description: None,
        param_type,
        explode: false,
        style: ParamStyle::Simple,
    }
}

fn located_params(query: Vec<ParamDef>, header: Vec<ParamDef>, cookie: Vec<ParamDef>) -> EndpointParams {
    EndpointParams { query, header, cookie }
}

/// A page-number default (adopting `page` and `per_page`, keeping `state`),
/// a resumable cursor on a wrapper list, a body next link, a page-number
/// list whose adopted `page` and `limit` are required (R2a, R11a), and a
/// point-in-time-style session with a keep-alive, renew, close, and
/// partitions (R22, R23).
fn pages_fixture() -> RestApi {
    let int = |name| named_param(name, QueryParamType::Integer);
    let mut list_entries =
        endpoint("ListEntries", RestMethod::Get, "/entries", ApiResponse::json_vec_type("Entry"));
    list_entries.params = Some(located_params(
        vec![int("page"), int("per_page"), named_param("state", QueryParamType::String)],
        vec![],
        vec![],
    ));
    let mut list_cursor =
        endpoint("ListCursor", RestMethod::Get, "/cursor", ApiResponse::json_type("EntryCursorPage"));
    list_cursor.params = Some(located_params(vec![named_param("after", QueryParamType::String)], vec![], vec![]));
    let list_linked =
        endpoint("ListLinked", RestMethod::Get, "/linked", ApiResponse::json_type("EntryLinkPage"));
    let mut allowed_linked = list_linked.clone();
    allowed_linked.id = "ListAllowedLinked".to_string();
    allowed_linked.path = "/allowed-linked".to_string();
    let required = |name| ParamDef { rust_name: None, secret: false, pager_managed: false, required: true, ..int(name) };
    let mut list_required =
        endpoint("ListRequired", RestMethod::Get, "/required", ApiResponse::json_vec_type("Entry"));
    list_required.params = Some(located_params(vec![required("page"), required("limit")], vec![], vec![]));
    let scan_entries = endpoint("ScanEntries", RestMethod::Get, "/scan", ApiResponse::json_vec_type("Entry"));

    let mut api = fixture_api(
        "FixturePages",
        "fixture_pages",
        AuthStrategy::None,
        vec![list_entries, list_cursor, list_linked, allowed_linked, list_required, scan_entries],
    );
    api.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::link_rel("next"))
            .page_size(PageSize::new("per_page").max(100))
            .build(),
    );
    for (id, collection) in [
        (
            "ListCursor",
            Collection::at("data").item_type("Entry").paginate(
                Strategy::cursor("after", CursorSource::last_item("id"))
                    .more_when(MoreSignal::has_more("has_more"))
                    .resumable(),
            ),
        ),
        (
            "ListLinked",
            Collection::at("values")
                .item_type("Entry")
                .paginate(Strategy::next_link(LinkSource::body("next"))),
        ),
        (
            "ListAllowedLinked",
            Collection::at("values")
                .item_type("Entry")
                .paginate(Strategy::next_link(LinkSource::body("next"))),
        ),
        (
            "ListRequired",
            Collection::root().paginate(
                Strategy::page_number("page", MoreSignal::NonEmptyPage)
                    .page_size(PageSize::new("limit").sent_default(25)),
            ),
        ),
        (
            "ScanEntries",
            Collection::root().paginate(
                Strategy::cursor("after", CursorSource::last_item("id"))
                    .more_when(MoreSignal::NonEmptyPage)
                    .session(
                        Session::opened_by(SessionStep::new(RestMethod::Post, "/pit"), ValueSource::body("id"))
                            .send_handle_as(InputParam::query("pit"))
                            .keep_alive(KeepAlive::new("1m").on_open("keep_alive").on_page("keep_alive").adjustable())
                            .renew(SessionStep::new(RestMethod::Post, "/pit/renew").handle("pit"))
                            .close(SessionStep::new(RestMethod::Delete, "/pit").handle("pit")),
                    )
                    .partition(Partition::new(InputParam::query("slice"), InputParam::query("slices")).max(2)),
            ),
        ),
    ] { api.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    api
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

    let mut types = schematic_define::DefinitionContext::types_in("::unused");
    let typed_durations = Endpoint::get("TypedDurations", "/typed-durations")
        .describes("Typed durations")
        .returns_json::<Option<Vec<std::collections::HashMap<String, std::time::Duration>>>>(&mut types)
        .build().unwrap();
    let mut bearer = RestApi::builder(&types, "FixtureBearer", "http://127.0.0.1:1")
        .describes("FixtureBearer fixture API").bearer_env(["FIXTURE_BEARER_TOKEN"])
        .module_path("fixture_bearer")
        .endpoints([typed_durations])
        .build().unwrap();
    bearer.endpoints.splice(0..0, [list_widgets, create_widget, get_widget, search]);
    let mut bearer = crate::common::components::with_referenced_components(bearer);
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
    cookie_key.headers = vec![("Cookie".to_string(), "lang=en".to_string())];
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

    // Secret names, a request-id header, and the tightest cross-origin policy,
    // all from the definition. Neither secret name is a built-in one.
    let mut echo = endpoint("Echo", RestMethod::Get, "/echo", ApiResponse::json_type("Echo"));
    echo.params = Some(located_params(
        vec![ParamDef { secret: true, ..named_param("pin_code", QueryParamType::String) }, named_param("page", QueryParamType::String)],
        vec![ParamDef { secret: true, ..named_param("X-Pin", QueryParamType::String) }],
        vec![],
    ));
    let mut settings = fixture_api(
        "FixtureSettings",
        "fixture_settings",
        AuthStrategy::None,
        vec![echo, endpoint("Moved", RestMethod::Get, "/moved", ApiResponse::Empty)],
    );
    settings.request_id_header = Some("x-request-id".to_string());
    settings.cross_origin = CrossOrigin::deny();

    // Names the generator must rename: a `new` endpoint method (an API-struct
    // method), a `full` setter (a shared call method), and a keyword setter.
    let mut list_items = endpoint("ListItems", RestMethod::Get, "/owners/{owner}/items", ApiResponse::Empty);
    list_items.params = Some(located_params(
        vec![
            named_param("full", QueryParamType::Boolean),
            named_param("type", QueryParamType::String),
            named_param("limit", QueryParamType::Integer),
        ],
        vec![],
        vec![],
    ));
    let mut tree = endpoint("GetTree", RestMethod::Get, "/tree", ApiResponse::Empty);
    tree.params = Some(located_params(
        vec![
            ParamDef { rust_name: Some("git_ref".to_string()), ..named_param("ref", QueryParamType::String) },
            named_param("beta", QueryParamType::Boolean),
            named_param("recursive", QueryParamType::Boolean),
        ],
        vec![],
        vec![],
    ));
    let names = fixture_api(
        "FixtureNames",
        "fixture_names",
        AuthStrategy::None,
        vec![
            endpoint("New", RestMethod::Post, "/new", ApiResponse::Empty), list_items, tree,
            endpoint("AllowedMove", RestMethod::Get, "/allowed-move", ApiResponse::Empty),
            endpoint("SiblingMove", RestMethod::Get, "/sibling-move", ApiResponse::Empty),
        ],
    );

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
        settings,
        names,
        pages_fixture(),
    ]
}

/// Checkout state the guard could plausibly disturb, compared before and
/// after a child build. The committed generated crate is the output a
/// misdirected generator would overwrite; the listings of the area, the
/// generator crate (the tests' working directory), and the three path
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
        let listings = ["", "gen", "define", "definitions", "runtime"]
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
/// port 0 and asserts what the server received and what the envelope
/// reports. The child process gets the `FIXTURE_*` variables from
/// [`FIXTURE_ENV`]; nothing mutates the environment at run time.
const REQUEST_SIDE_TEST: &str = r##"
use schematic_schema::fixture_api_key::{FixtureApiKey, SubmitFormForm, SubmitFormRequest};
use schematic_schema::fixture_basic::{FixtureBasic, GetBlobRequest, GetNoteRequest, SendNoteRequest, UploadBlobRequest};
use schematic_schema::fixture_bearer::{FixtureBearer, GetWidgetRequest, ListWidgetsRequest, SearchWidgetsRequest, Widget};
use schematic_schema::fixture_cookie_key::{FixtureCookieKey, GetProfileRequest};
use schematic_schema::fixture_oauth::{FixtureOAuth, WhoAmIRequest};
use schematic_schema::fixture_policy_key::{FixturePolicyKey, PingRequest};
use schematic_schema::fixture_query_key::{FixtureQueryKey, ListThingsRequest};
use schematic_schema::fixture_names::FixtureNames;
use schematic_schema::fixture_pages::FixturePages;
use schematic_schema::fixture_settings::{Echo, EchoRequest, FixtureSettings, MovedRequest};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{AuthMethod, ErrorKind, Outcome, RedirectRefusal, TransportErrorKind};
use wiremock::matchers::{any, path};
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

// ---- raw request bodies ----

#[tokio::test]
async fn text_body_reaches_the_server_verbatim_under_its_content_type() {
    for text in ["", "line one\nzwei – ünïcödé\r\n"] {
        let server = start_server().await;
        let client = FixtureBasic::new()
            .base_url(server.uri())
            .credentials(Credential::basic("u", "p"))
            .build()
            .unwrap();
        let response = client.send(SendNoteRequest::new(text.to_string())).full().await.unwrap();
        assert_eq!(response.into_result().ok(), Some(()), "an empty endpoint drains the body");

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
        let client = FixtureBasic::new().base_url(server.uri()).basic_auth("u", "p");
        client.send(UploadBlobRequest::new("a b", bytes.clone())).full().await.unwrap();

        let request = only_request(&server).await;
        assert_eq!(request.method.as_str(), "PUT");
        assert_eq!(request.url.path(), "/blobs/a%20b");
        assert_eq!(request.body, bytes);
        assert_eq!(header(&request, "content-type"), ["application/octet-stream"]);
        assert_eq!(header(&request, "authorization"), ["Basic dTpw"]);
    }
}

// ---- header parameters ----

#[tokio::test]
async fn set_header_parameters_override_static_headers_by_case_insensitive_name() {
    let server = start_server().await;
    let client = FixtureBearer::new()
        .base_url(server.uri())
        .credentials(Credential::access_token("t"))
        .build()
        .unwrap();
    let request = SearchWidgetsRequest::default()
        .with_q("a b".to_string())
        .with_x_trace_id("abc 123".to_string())
        .with_x_mode("param".to_string())
        .with_x_ids(vec![1, 22, 333]);
    client.send(request).full().await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "x-trace-id"), ["abc 123"]);
    assert_eq!(header(&request, "x-mode"), ["param"], "the parameter replaces the static header");
    assert_eq!(header(&request, "x-ids"), ["1,22,333"]);
    assert_eq!(header(&request, "x-client"), ["fixture"], "the API's default header is sent");
    assert_eq!(header(&request, "authorization"), ["Bearer t"]);
    assert_eq!(query(&request), "q=a%20b", "header values never join the query");
}

#[tokio::test]
async fn unset_header_parameters_keep_static_headers_and_send_nothing() {
    let server = start_server().await;
    let client = FixtureBearer::new().base_url(server.uri()).build().unwrap();
    client.send(SearchWidgetsRequest::default()).full().await.unwrap();

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
        let client = FixtureBearer::new().base_url(server.uri()).build().unwrap();
        let request = SearchWidgetsRequest::default().with_x_trace_id(value.to_string());
        let error = client.send(request).full().await.unwrap_err();

        let TransportErrorKind::InvalidArgument(argument) = &error.kind else {
            panic!("expected an invalid argument, got {error:?}");
        };
        assert!(argument.reason().contains("X-Trace-Id"), "{argument}");
        for rendered in [error.to_string(), format!("{error:?}")] {
            assert!(!rendered.contains(value), "the value leaked: {rendered}");
        }
        assert_nothing_sent(&server).await;
    }
}

// ---- cookie parameters ----

#[tokio::test]
async fn cookies_from_api_endpoint_and_parameters_share_one_header() {
    let server = start_server().await;
    let client = FixtureBearer::new().base_url(server.uri()).build().unwrap();
    let request = SearchWidgetsRequest::default()
        .with_session("a b;c\r\nX-Evil: 1".to_string())
        .with_prefs(vec!["x,y".to_string(), "é".to_string(), "%".to_string()]);
    client.send(request).full().await.unwrap();

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
    let client = FixtureBearer::new().base_url(server.uri()).build().unwrap();
    client.send(SearchWidgetsRequest::default()).full().await.unwrap();

    let request = only_request(&server).await;
    assert_eq!(header(&request, "cookie"), ["api=1; theme=dark; session=static"]);
}

// ---- API-key parameters and credential sources ----

#[tokio::test]
async fn a_supplied_query_key_is_sent_and_never_duplicates_a_declared_parameter() {
    let server = start_server().await;
    let client = FixtureQueryKey::new().base_url(server.uri()).build().unwrap();
    let keyed = Credential::api_key("SENTINEL key&=");
    let response = client
        .send(ListThingsRequest::default().with_q("x y".to_string())).credential(keyed.clone()).full()
        .await
        .unwrap();

    assert_eq!(response.meta.request.auth, AuthMethod::PerCall);
    assert!(!response.to_string().contains("SENTINEL"), "{response}");
    let request = only_request(&server).await;
    assert_eq!(query(&request), "q=x%20y&api_key=SENTINEL%20key%26%3D");
    assert!(header(&request, "cookie").is_empty());

    // A request that already sets the credential's query name is refused
    // before sending rather than sent with two keys.
    let server = start_server().await;
    let client = FixtureQueryKey::new().base_url(server.uri()).build().unwrap();
    let declared = ListThingsRequest::default().with_api_key("declared".to_string());
    let error = client.send(declared).credential(keyed.clone()).full().await.unwrap_err();
    assert!(matches!(error.kind, TransportErrorKind::InvalidArgument(_)), "{error:?}");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn env_query_key_is_read_from_the_definition_env_names() {
    let server = start_server().await;
    let client = FixtureQueryKey::new().base_url(server.uri()).build().unwrap();
    let response = client.send(ListThingsRequest::default()).full().await.unwrap();

    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar("FIXTURE_QUERY_KEY_TOKEN".to_string()));
    assert_eq!(query(&only_request(&server).await), "api_key=env-query-key");
}

#[tokio::test]
async fn the_first_non_empty_env_variable_supplies_the_key() {
    let server = start_server().await;
    let client = FixtureQueryKey::new()
        .base_url(server.uri())
        .env_order(["FIXTURE_EMPTY_KEY", "FIXTURE_SECOND_KEY"])
        .build()
        .unwrap();
    let response = client.send(ListThingsRequest::default()).full().await.unwrap();

    assert_eq!(response.meta.request.auth, AuthMethod::EnvVar("FIXTURE_SECOND_KEY".to_string()));
    assert_eq!(query(&only_request(&server).await), "api_key=second-key");
}

#[tokio::test]
async fn a_missing_key_sends_the_request_anonymously() {
    let server = start_server().await;
    let client = FixtureQueryKey::new()
        .base_url(server.uri())
        .env_order(["SCHEMATIC_FIXTURE_UNSET_KEY_7F3A"])
        .build()
        .unwrap();
    let response = client.send(ListThingsRequest::default()).full().await.unwrap();

    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(query(&only_request(&server).await), "", "no key is sent and none is invented");
}

#[tokio::test]
async fn a_cookie_key_is_appended_and_never_replaces_a_cookie_of_its_name() {
    let server = start_server().await;
    let client = FixtureCookieKey::new().base_url(server.uri()).build().unwrap();
    client
        .send(GetProfileRequest::default().with_theme("t".to_string())).full()
        .await
        .unwrap();
    assert_eq!(header(&only_request(&server).await, "cookie"), ["lang=en; theme=t; sid=env-cookie-key"]);

    let server = start_server().await;
    let client = FixtureCookieKey::new().base_url(server.uri()).build().unwrap();
    let keyed = Credential::api_key("SENTINEL;v");
    let declared = GetProfileRequest::default().with_sid("param".to_string());
    let error = client.send(declared).credential(keyed.clone()).full().await.unwrap_err();
    assert!(matches!(error.kind, TransportErrorKind::InvalidArgument(_)), "{error:?}");
    assert!(!format!("{error} {error:?}").contains("SENTINEL"), "{error:?}");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn an_anonymous_call_sends_no_credential_from_any_source() {
    let server = start_server().await;
    let client = FixtureCookieKey::new()
        .base_url(server.uri())
        .credentials(Credential::api_key("client-key"))
        .build()
        .unwrap();
    let response = client
        .send(GetProfileRequest::default().with_theme("t".to_string())).anonymous().full()
        .await
        .unwrap();

    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(header(&only_request(&server).await, "cookie"), ["lang=en; theme=t"]);
}

#[tokio::test]
async fn a_policy_without_env_fallback_reads_no_variable() {
    let server = start_server().await;
    let client = FixturePolicyKey::new().base_url(server.uri()).build().unwrap();
    let response = client.send(PingRequest::default()).full().await.unwrap();
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(query(&only_request(&server).await), "", "FIXTURE_POLICY_KEY_TOKEN is not read");

    let server = start_server().await;
    let client = FixturePolicyKey::new().base_url(server.uri()).build().unwrap();
    client
        .send(PingRequest::default()).credential(Credential::api_key("explicit")).full()
        .await
        .unwrap();
    assert_eq!(query(&only_request(&server).await), "token=explicit");
}

// ---- the one-name client: chained settings, api_key, and validation ----

#[tokio::test]
async fn api_key_goes_where_each_generated_definition_takes_a_key() {
    let server = start_server().await;
    let bearer = FixtureBearer::new().base_url(server.uri()).api_key("k-bearer");
    bearer.send(ListWidgetsRequest::default()).full().await.unwrap();
    assert_eq!(header(&only_request(&server).await, "authorization"), ["Bearer k-bearer"]);

    let server = start_server().await;
    let query_key = FixtureQueryKey::new().base_url(server.uri()).api_key("k-query");
    let response = query_key.send(ListThingsRequest::default()).full().await.unwrap();
    assert_eq!(response.meta.request.auth, AuthMethod::ClientLevel, "the set key beats the environment");
    assert_eq!(query(&only_request(&server).await), "api_key=k-query");

    // Basic takes two values, so a single key has nowhere to go.
    let server = start_server().await;
    let basic = FixtureBasic::new().base_url(server.uri()).api_key("k-basic");
    let error = basic.send(GetNoteRequest::default()).full().await.unwrap_err();
    let TransportErrorKind::InvalidClient(build) = &error.kind else {
        panic!("a key on a Basic-only API is refused: {error:?}");
    };
    assert!(build.to_string().contains("API key"), "{build}");
    assert!(!error.to_string().contains("k-basic"), "{error}");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn new_never_fails_and_an_invalid_setting_surfaces_at_build_or_the_first_send() {
    let server = start_server().await;
    let client = FixtureBearer::new().base_url("not a url");
    let error = client.send(ListWidgetsRequest::default()).full().await.unwrap_err();
    assert!(matches!(error.kind, TransportErrorKind::InvalidClient(_)), "{error:?}");
    let built: Result<FixtureBearer, _> = client.build();
    assert!(built.is_err(), "build reports the same setting");

    let built: FixtureBearer = FixtureBearer::new().base_url(server.uri()).build().unwrap();
    built.send(ListWidgetsRequest::default()).full().await.unwrap();
    only_request(&server).await;
}

// ---- no credential in Debug output ----

#[tokio::test]
async fn debug_output_never_contains_a_credential() {
    let header_key = FixtureApiKey::new().credentials(Credential::api_key("SENTINEL-HEADER"));
    let bearer = FixtureBearer::new()
        .credentials(Credential::access_token("SENTINEL-BEARER"))
        .build()
        .unwrap();
    let query_key = FixtureQueryKey::new();
    for (sentinel, rendered) in [
        ("SENTINEL-HEADER", format!("{header_key:?}")),
        ("SENTINEL-BEARER", format!("{bearer:?}")),
        ("env-query-key", format!("{query_key:?}")),
    ] {
        assert!(!rendered.contains(sentinel), "{sentinel} leaked: {rendered}");
    }
    assert!(format!("{bearer:?}").contains("base_url"));
    // A request struct carries parameters, never the client's key.
    let request = ListThingsRequest::default().with_q("x".to_string());
    assert!(!format!("{request:?}").contains("SENTINEL"));
}

// ---- the remaining strategies and bodies reach the server ----

#[tokio::test]
async fn header_key_oauth_and_form_bodies_reach_the_server() {
    let server = start_server().await;
    let client = FixtureApiKey::new()
        .base_url(server.uri())
        .credentials(Credential::api_key("k-1"))
        .build()
        .unwrap();
    let form = SubmitFormForm { name: "a b".to_string(), note: None };
    client.send(SubmitFormRequest::new(form)).full().await.unwrap();
    let request = only_request(&server).await;
    assert_eq!(header(&request, "x-api-key"), ["k-1"]);
    assert_eq!(header(&request, "content-type"), ["application/x-www-form-urlencoded"]);
    assert_eq!(request.body, b"name=a+b");

    let server = start_server().await;
    let client = FixtureOAuth::new().base_url(server.uri()).build().unwrap();
    client
        .send(WhoAmIRequest::default()).credential(Credential::access_token("tok")).full()
        .await
        .unwrap();
    assert_eq!(header(&only_request(&server).await, "authorization"), ["Bearer tok"]);
}

// ---- the declared codec decides; the Content-Type does not ----

#[tokio::test]
async fn each_endpoint_decodes_with_its_declared_codec() {
    let server = MockServer::start().await;
    Mock::given(path("/note"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"a":1}"#, "application/json"))
        .mount(&server)
        .await;
    Mock::given(path("/blob"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("plain", "text/plain"))
        .mount(&server)
        .await;
    Mock::given(path("/widgets/w1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("<html>busy</html>", "text/html"))
        .mount(&server)
        .await;
    let basic = FixtureBasic::new().base_url(server.uri()).build().unwrap();
    let bearer = FixtureBearer::new().base_url(server.uri()).build().unwrap();

    let note: Option<String> = basic.send(GetNoteRequest::default()).full().await.unwrap().into_result().ok();
    assert_eq!(note.as_deref(), Some(r#"{"a":1}"#), "a text endpoint keeps a JSON body as text");
    let blob = basic.send(GetBlobRequest::default()).full().await.unwrap().into_result().ok();
    assert_eq!(blob.as_deref(), Some(&b"plain"[..]));

    let response = bearer.send(GetWidgetRequest::new("w1")).full().await.unwrap();
    assert_eq!(response.meta.status.as_u16(), 200);
    let Outcome::Failure(error) = &response.outcome else {
        panic!("HTML on a JSON endpoint is not a success: {response}");
    };
    assert!(matches!(error.kind, ErrorKind::MalformedSuccess), "{error}");
    assert_eq!(error.body, &b"<html>busy</html>"[..], "the body is kept as evidence");
}

// ---- response hooks through the generated request types ----

#[tokio::test]
async fn a_mutator_registered_for_a_generated_request_runs_only_for_that_endpoint() {
    let server = MockServer::start().await;
    Mock::given(path("/widgets"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"items":[]}"#, "application/json"))
        .mount(&server)
        .await;
    Mock::given(path("/widgets/w1"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"id":"w1","name":"n"}"#, "application/json"))
        .mount(&server)
        .await;
    let client = FixtureBearer::new()
        .base_url(server.uri())
        .mutate_response::<ListWidgetsRequest, _>(|_meta, list| {
            list.items.push(Widget { id: "added".to_string(), name: "by the mutator".to_string() });
            Ok(())
        })
        .build()
        .unwrap();

    let list = client.send(ListWidgetsRequest::default()).full().await.unwrap().into_result().unwrap();
    assert_eq!(list.items.len(), 1, "the mutator ran for its endpoint");
    let widget = client.send(GetWidgetRequest::new("w1")).full().await.unwrap().into_result().unwrap();
    assert_eq!(widget.name, "n", "another endpoint is untouched");
}

// ---- endpoint methods ----

#[tokio::test]
async fn an_endpoint_method_sends_its_values_and_awaits_to_the_data() {
    let server = MockServer::start().await;
    Mock::given(path("/widgets"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"items":[{"id":"a","name":"n"}]}"#, "application/json"))
        .mount(&server)
        .await;
    Mock::given(path("/widgets/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_raw(r#"{"error":"gone"}"#, "application/json"))
        .mount(&server)
        .await;
    let client = FixtureBearer::new().base_url(server.uri()).build().unwrap();

    let list = client
        .list_widgets()
        .limit(5)
        .q("x".to_string())
        .tag(vec!["a".to_string(), "b".to_string()])
        .anonymous()
        .await
        .unwrap();
    assert_eq!(list.items, [Widget { id: "a".to_string(), name: "n".to_string() }]);
    let request = only_request(&server).await;
    assert_eq!(query(&request), "limit=5&q=x&tag=a&tag=b");
    assert!(header(&request, "authorization").is_empty(), "the per-call setting reached the send");

    let full = client.list_widgets().full().await.unwrap();
    assert_eq!(full.meta.status.as_u16(), 200);
    assert_eq!(full.into_result().unwrap().items.len(), 1);

    match client.get_widget("missing").await {
        Err(CallError::Api { error, meta }) => {
            assert_eq!(meta.status.as_u16(), 404);
            assert_eq!(error.body, &br#"{"error":"gone"}"#[..]);
        }
        other => panic!("a 404 is an API error with its response kept: {other:?}"),
    }
}

#[tokio::test]
async fn renamed_endpoint_methods_and_setters_send_under_their_wire_names() {
    let server = start_server().await;
    let client = FixtureNames::new().base_url(server.uri());
    client.list_items("me").full_param(true).type_("t".to_string()).limit(3).await.unwrap();
    let request = only_request(&server).await;
    assert_eq!(request.url.path(), "/owners/me/items");
    assert_eq!(query(&request), "full=true&type=t&limit=3");

    let server = start_server().await;
    let client = FixtureNames::new().base_url(server.uri());
    let built = client.new_endpoint().build().unwrap();
    built.clone().await.unwrap();
    built.await.unwrap();
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.len(), 2, "a built call sent twice makes two requests");
    assert!(received.iter().all(|request| request.method.as_str() == "POST" && request.url.path() == "/new"));
}

#[tokio::test]
async fn optional_query_alias_and_booleans_encode_once_under_wire_names() {
    let server = start_server().await;
    let client = FixtureNames::new().base_url(server.uri());
    client.get_tree().await.unwrap();
    assert_eq!(query(&only_request(&server).await), "", "unset optional inputs stay absent");
    server.reset().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
    client.get_tree().git_ref("feature/a b%&=ü".to_string()).beta(false).recursive(true).await.unwrap();
    let request = only_request(&server).await;
    assert_eq!(query(&request), "ref=feature%2Fa%20b%25%26%3D%C3%BC&beta=false&recursive=true");
    assert_eq!(request.url.query_pairs().collect::<Vec<_>>(), [
        ("ref".into(), "feature/a b%&=ü".into()),
        ("beta".into(), "false".into()),
        ("recursive".into(), "true".into()),
    ]);
    server.reset().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(204)).mount(&server).await;
    client.get_tree().git_ref("feature/x y+z".to_string()).beta(true).recursive(false).await.unwrap();
    assert_eq!(query(&only_request(&server).await), "ref=feature%2Fx%20y%2Bz&beta=true&recursive=false");
}

#[tokio::test]
async fn endpoint_origin_override_allows_redirects_and_caller_can_tighten_it() {
    use schematic_schema::schematic_runtime::OriginPolicy;
    let server = MockServer::start().await;
    let other = std::env::var("FIXTURE_SECOND_ORIGIN").unwrap();
    for (source, target) in [("/allowed-move", "/redirect-allowed"), ("/sibling-move", "/redirect-sibling")] {
        Mock::given(path(source))
            .respond_with(ResponseTemplate::new(302).insert_header("location", format!("{other}{target}")))
            .mount(&server).await;
    }
    let client = FixtureNames::new().base_url(server.uri()).build().unwrap();
    client.allowed_move().await.unwrap();
    let sibling = client.sibling_move().full().await.unwrap();
    assert!(matches!(sibling.outcome, Outcome::Failure(ref error)
        if matches!(error.kind, ErrorKind::RedirectRefused { reason: RedirectRefusal::CrossOrigin })));
    let tightened = FixtureNames::new().base_url(server.uri())
        .cross_origin(OriginPolicy::same_origin()).build().unwrap();
    let response = tightened.allowed_move().full().await.unwrap();
    assert!(matches!(response.outcome, Outcome::Failure(ref error)
        if matches!(error.kind, ErrorKind::RedirectRefused { reason: RedirectRefusal::CrossOrigin })));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

#[tokio::test]
async fn endpoint_origin_override_governs_followed_pagination_links() {
    use schematic_schema::schematic_runtime::{OriginPolicy, pager::PagerError};
    let server = MockServer::start().await;
    let other = std::env::var("FIXTURE_SECOND_ORIGIN").unwrap();
    for (source, target) in [("/allowed-linked", "/page-allowed"), ("/linked", "/page-sibling")] {
        Mock::given(path(source))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "values": [{"id": "first"}], "next": format!("{other}{target}")
            })))
            .mount(&server).await;
    }
    let client = FixturePages::new().base_url(server.uri()).build().unwrap();
    let mut pages = client.list_allowed_linked().pages();
    assert_eq!(pages.next().await.unwrap().unwrap().items[0].id, "first");
    assert_eq!(pages.next().await.unwrap().unwrap().items[0].id, "second");
    assert!(pages.next().await.unwrap().is_none());
    let mut sibling = client.list_linked().pages();
    assert_eq!(sibling.next().await.unwrap().unwrap().items[0].id, "first");
    assert!(matches!(sibling.next().await, Err(PagerError::Transport(ref error))
        if matches!(error.kind, TransportErrorKind::CrossOriginRejected { .. })));
    let tightened = FixturePages::new().base_url(server.uri())
        .cross_origin(OriginPolicy::same_origin()).build().unwrap();
    let mut pages = tightened.list_allowed_linked().pages();
    assert_eq!(pages.next().await.unwrap().unwrap().items[0].id, "first");
    assert!(matches!(pages.next().await, Err(PagerError::Transport(ref error))
        if matches!(error.kind, TransportErrorKind::CrossOriginRejected { .. })));
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
}

// ---- definition settings reach the runtime ----

#[tokio::test]
async fn request_id_secret_names_and_cross_origin_policy_come_from_the_definition() {
    let server = MockServer::start().await;
    Mock::given(path("/echo"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "req-42")
                .set_body_raw(r#"{"ok":true}"#, "application/json"),
        )
        .mount(&server)
        .await;
    Mock::given(path("/moved"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/echo"))
        .mount(&server)
        .await;
    let client = FixtureSettings::new().base_url(server.uri()).build().unwrap();

    let request = EchoRequest::default()
        .with_x_pin("SENTINEL-PIN".to_string())
        .with_pin_code("SENTINEL-CODE".to_string())
        .with_page("2".to_string());
    let response = client.send(request).full().await.unwrap();
    assert_eq!(response.data(), Some(&Echo { ok: true }));
    assert_eq!(response.meta.request_id.as_deref(), Some("req-42"));
    let stored = format!("{response} {:?} {:?}", response.meta.request.query, response.meta.request.headers);
    assert!(!stored.contains("SENTINEL"), "a definition-marked secret was stored: {stored}");
    assert!(response.meta.request.query.contains(&("page".to_string(), "2".to_string())));
    let received = only_request(&server).await;
    assert_eq!(header(&received, "x-pin"), ["SENTINEL-PIN"], "the server still receives the value");

    // The definition's `deny` policy refuses even a same-origin redirect.
    let response = client.send(MovedRequest::default()).full().await.unwrap();
    let Outcome::Failure(error) = &response.outcome else {
        panic!("a redirect under `deny` is not followed: {response}");
    };
    assert!(
        matches!(error.kind, ErrorKind::RedirectRefused { reason: RedirectRefusal::CrossOrigin }),
        "{error}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2, "/echo was not requested again");
}

// ---- pagination ----

#[tokio::test]
async fn generated_paging_runs_only_the_global_hook_before_items_and_continuations() {
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use schematic_schema::fixture_pages::{Entry, ListCursorRequest};
    use wiremock::matchers::query_param;

    let server = MockServer::start().await;
    Mock::given(path("/cursor"))
        .and(query_param("after", "hook-first"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "wire_id": "second", "wire_more": false
        })))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(path("/cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "wire_id": "first", "wire_more": true
        })))
        .mount(&server)
        .await;
    let global_calls = Arc::new(AtomicUsize::new(0));
    let typed_calls = Arc::new(AtomicUsize::new(0));
    let global_count = global_calls.clone();
    let typed_count = typed_calls.clone();
    let client = FixturePages::new()
        .base_url(server.uri())
        .pre_response_json(move |_, json| {
            global_count.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!({
                "data": [{"id": format!("hook-{}", json["wire_id"].as_str().unwrap())}],
                "has_more": json["wire_more"]
            }))
        })
        .mutate_response::<ListCursorRequest, _>(move |_, response| {
            typed_count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(response.data[0].id, "hook-first", "the global hook runs first");
            response.data[0].id = "typed-first".to_string();
            Ok(())
        })
        .build()
        .unwrap();
    let expected = [Entry { id: "hook-first".to_string() }, Entry { id: "hook-second".to_string() }];

    let mut pager = client.list_cursor().pages();
    assert_eq!(pager.next().await.unwrap().unwrap().items, expected[..1]);
    assert_eq!(pager.next().await.unwrap().unwrap().items, expected[1..]);
    assert!(pager.next().await.unwrap().is_none(), "the hook's terminal signal stops requests");
    assert_eq!(global_calls.load(Ordering::SeqCst), 2);
    assert_eq!(typed_calls.load(Ordering::SeqCst), 0);

    let page = client.list_cursor().fetch_page().await.unwrap();
    assert_eq!(page.items, expected[..1]);
    let collected = client.list_cursor().collect_all().await.unwrap();
    assert_eq!(collected, expected);
    assert_eq!(global_calls.load(Ordering::SeqCst), 5);
    assert_eq!(typed_calls.load(Ordering::SeqCst), 0);

    let ordinary = client.list_cursor().await.unwrap();
    assert_eq!(ordinary.data, [Entry { id: "typed-first".to_string() }]);
    assert!(ordinary.has_more);
    assert_eq!(global_calls.load(Ordering::SeqCst), 6);
    assert_eq!(typed_calls.load(Ordering::SeqCst), 1);
    let received = server.received_requests().await.unwrap();
    assert_eq!(received.iter().map(query).collect::<Vec<_>>(),
        ["", "after=hook-first", "", "", "after=hook-first", ""],
        "follow-ups use the transformed item identity; exhaustion sends nothing");
}

#[tokio::test]
async fn generated_pager_retains_api_evidence_and_explicitly_retries() {
    use schematic_runtime::pager::PagerError;
    let server = MockServer::start().await;
    Mock::given(path("/cursor"))
        .respond_with(ResponseTemplate::new(503).set_body_string("first evidence"))
        .mount(&server).await;
    let client = FixturePages::new().base_url(server.uri()).build().unwrap();
    let mut pages = client.list_cursor().pages();
    let error = pages.next().await.unwrap_err();
    assert_eq!(pages.next().await.unwrap_err(), error);
    let PagerError::Api { error: api, meta } = error else { panic!("expected API failure") };
    assert_eq!(meta.status, 503);
    assert_eq!(&api.body[..], b"first evidence");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    server.reset().await;
    Mock::given(path("/cursor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"data":[{"id":"recovered"}],"has_more":false})))
        .mount(&server).await;
    pages.retry().await.unwrap();
    let page = pages.next().await.unwrap().unwrap();
    assert_eq!(page.number, 1);
    assert_eq!(page.items[0].id, "recovered");
    assert!(pages.next().await.unwrap().is_none());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn required_adopted_parameters_are_sent_once_by_an_ordinary_send_and_by_the_pager() {
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .mount(&server)
        .await;
    let client = FixturePages::new().base_url(server.uri()).build().unwrap();

    let entries = client.list_required().await.unwrap();
    assert!(entries.is_empty(), "an ordinary send returns the endpoint's own response");
    let page = client.list_required().page(3).page_size(5).fetch_page().await.unwrap();
    assert_eq!(page.number, 1);

    let received = server.received_requests().await.unwrap();
    let queries: Vec<String> = received.iter().map(query).collect();
    assert_eq!(
        queries,
        ["page=1&limit=25", "page=3&limit=5"],
        "the ordinary send carries the first page and the authored size; the pager replaces both"
    );
}

#[tokio::test]
async fn a_generated_session_opens_pages_and_closes_inside_its_scope() {
    use wiremock::matchers::{method, query_param};
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/pit"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"id":"pit-1"}"#))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/scan"))
        .and(query_param("after", "a"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"[{"id":"a"}]"#))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/pit"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let client = FixturePages::new().base_url(server.uri()).build().unwrap();

    let entries = client
        .scan_entries()
        .keep_alive("5m")
        .with_session(|scope| Box::pin(async move { scope.collect_all().await }))
        .await
        .expect("the close succeeded")
        .expect("the list is complete");
    assert_eq!(entries.len(), 1);

    let received: Vec<String> = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| format!("{} {}?{}", request.method, request.url.path(), query(request)))
        .collect();
    assert_eq!(
        received,
        [
            "POST /pit?keep_alive=5m",
            "GET /scan?pit=pit-1&keep_alive=5m",
            "GET /scan?after=a&pit=pit-1&keep_alive=5m",
            "DELETE /pit?pit=pit-1",
        ],
        "one open, every page with the handle and keep-alive, one close"
    );
}

#[tokio::test]
async fn typed_registration_paths_preserve_nullable_nested_wire_shape() {
    let server = MockServer::start().await;
    Mock::given(path("/typed-durations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([{"elapsed":{"secs":2,"nanos":3}}])))
        .mount(&server).await;
    let client = FixtureBearer::new().base_url(server.uri());
    let result: Option<Vec<std::collections::HashMap<String, std::time::Duration>>> = client.typed_durations().await.unwrap();
    assert_eq!(result.unwrap()[0]["elapsed"], std::time::Duration::new(2, 3));
    assert_eq!(only_request(&server).await.url.path(), "/typed-durations");
    server.reset().await;
    Mock::given(path("/typed-durations"))
        .respond_with(ResponseTemplate::new(200).set_body_string("null"))
        .mount(&server).await;
    assert!(client.typed_durations().await.unwrap().is_none());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
"##;

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
    "a_supplied_query_key_is_sent_and_never_duplicates_a_declared_parameter",
    "env_query_key_is_read_from_the_definition_env_names",
    "the_first_non_empty_env_variable_supplies_the_key",
    "a_missing_key_sends_the_request_anonymously",
    "a_cookie_key_is_appended_and_never_replaces_a_cookie_of_its_name",
    "an_anonymous_call_sends_no_credential_from_any_source",
    "a_policy_without_env_fallback_reads_no_variable",
    "api_key_goes_where_each_generated_definition_takes_a_key",
    "new_never_fails_and_an_invalid_setting_surfaces_at_build_or_the_first_send",
    "debug_output_never_contains_a_credential",
    "header_key_oauth_and_form_bodies_reach_the_server",
    "each_endpoint_decodes_with_its_declared_codec",
    "a_mutator_registered_for_a_generated_request_runs_only_for_that_endpoint",
    "an_endpoint_method_sends_its_values_and_awaits_to_the_data",
    "renamed_endpoint_methods_and_setters_send_under_their_wire_names",
    "optional_query_alias_and_booleans_encode_once_under_wire_names",
    "endpoint_origin_override_allows_redirects_and_caller_can_tighten_it",
    "endpoint_origin_override_governs_followed_pagination_links",
    "request_id_secret_names_and_cross_origin_policy_come_from_the_definition",
    "generated_paging_runs_only_the_global_hook_before_items_and_continuations",
    "generated_pager_retains_api_evidence_and_explicitly_retries",
    "required_adopted_parameters_are_sent_once_by_an_ordinary_send_and_by_the_pager",
    "a_generated_session_opens_pages_and_closes_inside_its_scope",
    "typed_registration_paths_preserve_nullable_nested_wire_shape",
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

struct SecondOrigin {
    url: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    stopping: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SecondOrigin {
    fn start() -> Self {
        use std::io::{Read, Write};
        use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = requests.clone();
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let thread = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stop.load(Ordering::Acquire) { break; }
                stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 1024];
                while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap();
                    assert_ne!(count, 0, "HTTP request ended before its headers");
                    bytes.extend_from_slice(&buffer[..count]);
                }
                let request = String::from_utf8(bytes).unwrap();
                let path = request.split_whitespace().nth(1).unwrap().to_string();
                recorded.lock().unwrap().push(path.clone());
                let (status, body) = if path.starts_with("/page") {
                    ("200 OK", r#"{"values":[{"id":"second"}],"next":null}"#)
                } else { ("204 No Content", "") };
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        Self { url: format!("http://{address}"), requests, stopping, thread: Some(thread) }
    }
}

impl Drop for SecondOrigin {
    fn drop(&mut self) {
        self.stopping.store(true, std::sync::atomic::Ordering::Release);
        let _ = std::net::TcpStream::connect(self.url.trim_start_matches("http://"));
        if let Some(thread) = self.thread.take() { thread.join().unwrap(); }
    }
}

#[test]
fn slow_fixture_apis_compile_and_send_offline() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let area = schematic_root();
    let before = CheckoutWitness::capture(&area);

    let second_origin = SecondOrigin::start();
    let mut apis = fixture_apis();
    for api in &mut apis {
        for endpoint in &mut api.endpoints {
            if matches!(endpoint.id.as_str(), "AllowedMove" | "ListAllowedLinked") {
                endpoint.cross_origin = Some(CrossOrigin::allow_origins([second_origin.url.clone()]));
            }
        }
    }
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
    let mut env = FIXTURE_ENV.to_vec();
    env.push(("FIXTURE_SECOND_ORIGIN", second_origin.url.as_str()));
    let run = fresh.nextest_run(&env);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in REQUEST_SIDE_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }

    let mut downstream = second_origin.requests.lock().unwrap().clone();
    downstream.sort();
    assert_eq!(downstream, ["/page-allowed", "/redirect-allowed"],
        "only endpoint overrides reach the second origin; siblings and caller-tightened calls send nothing there");
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
    let ollama = fresh.src_dir.join("ollama.rs");
    let anthropic = fresh.src_dir.join("anthropic.rs");
    let pristine_anthropic = std::fs::read(&anthropic).unwrap();
    for edited in [&ollama, &anthropic] {
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
    assert!(
        !std::fs::read_to_string(&ollama).unwrap().contains("edited before the partial run"),
        "the selected group is regenerated"
    );
    std::fs::write(&anthropic, pristine_anthropic).unwrap();

    let committed = biscuit_test_harness::manifest_dir!().join("../schema/src");
    let differences = diff_summary(&snapshot_tree(&committed), &snapshot_tree(&fresh.src_dir));
    assert!(
        differences.is_empty(),
        "freshly generated Rust differs from the committed schematic/schema/src.\n\
         Run `just -f schematic/justfile generate` to update it.\n{differences:#?}"
    );

    let websocket: Vec<String> = list_dir(&fresh.src_dir)
        .into_iter()
        .filter(|name| name.ends_with("_ws.rs") || name == "ws_shared.rs")
        .collect();
    assert_eq!(websocket.len(), 6, "the WebSocket modules are part of the compiled crate: {websocket:?}");

    fresh.cargo_check().assert_success();

    before.assert_unchanged(&CheckoutWitness::capture(&area));
}

/// One misuse per entry: a description, the snippet, the compiler error code
/// it must fail with, and text its diagnostic must name. Entry `i` becomes
/// the fixture crate's test target `tests/misuse_<i>.rs`.
const MISUSES: &[(&str, &str, &str, &str)] = &[
    ("external Endpoint literal", "fn misuse(endpoint: schematic_define::Endpoint) { let _ = schematic_define::Endpoint { id: String::new(), ..endpoint }; }", "E0639", "non-exhaustive"),
    ("external RestApi literal", "fn misuse(api: schematic_define::RestApi) { let _ = schematic_define::RestApi { name: String::new(), ..api }; }", "E0639", "non-exhaustive"),
    ("Endpoint has no Default", "fn misuse() { let _ = schematic_define::Endpoint::default(); }", "E0599", "Endpoint"),
    ("RestApi has no Default", "fn misuse() { let _ = schematic_define::RestApi::default(); }", "E0599", "RestApi"),
    ("endpoint description is required", "fn misuse() { let _ = schematic_define::Endpoint::get(\"Get\", \"/\").returns_empty().build(); }", "E0599", "build"),
    ("endpoint success is required", "fn misuse() { let _ = schematic_define::Endpoint::get(\"Get\", \"/\").describes(\"Get\").build(); }", "E0599", "build"),
    ("API description is required", "fn misuse() { let c = schematic_define::DefinitionContext::types_in(\"::models\"); let _ = schematic_define::RestApi::builder(&c, \"Api\", \"https://example.com\").no_auth().build(); }", "E0599", "build"),
    ("API authentication is required", "fn misuse() { let c = schematic_define::DefinitionContext::types_in(\"::models\"); let _ = schematic_define::RestApi::builder(&c, \"Api\", \"https://example.com\").describes(\"Api\").build(); }", "E0599", "build"),
    ("ErrorSet only supports none", "fn misuse() { let _ = schematic_define::ErrorSet::status(400); }", "E0599", "ErrorSet"),
    (
        "a descriptor cannot be constructed from a bare name",
        "fn misuse() { let _ = schematic_define::TypeDesc::new(\"Model\"); }",
        "E0624",
        "private",
    ),
    (
        "a descriptor carries no structure",
        "fn misuse() { let mut c = schematic_define::DefinitionContext::types_in(\"::models\"); let _ = c.register::<String>().unwrap().structure(schematic_define::TypeRef::Named(\"Model\".into())); }",
        "E0599",
        "structure",
    ),
    (
        "rust_path rejects a misspelled type",
        "fn misuse() { let _ = schematic_define::rust_path!(::std::time::Misspelled); }",
        "E0425",
        "Misspelled",
    ),
    (
        "rust_path must name the registration type",
        "fn misuse() { let mut c = schematic_define::DefinitionContext::types_in(\"::models\"); let _ = c.register_as::<std::time::Duration>(schematic_define::rust_path!(::std::ops::Range<u64>)); }",
        "E0308",
        "RustPath",
    ),
    (
        "a success type other than the declared one",
        "async fn misuse(client: FixtureBearer) {
            let _: Response<String, NoDeclaredError> =
                client.send(ListWidgetsRequest::default()).full().await.unwrap();
        }",
        "E0308",
        "WidgetList",
    ),
    (
        "an EndpointSpec impl without its codec",
        "struct Mine;
        impl EndpointSpec for Mine {
            type Api = FixtureBearerApi;
            type Success = ();
            type Failure = NoDeclaredError;
            const ENDPOINT_ID: &'static str = \"Mine\";
            const METHOD: Method = Method::GET;
            fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
                unimplemented!()
            }
        }",
        "E0046",
        "Decoder",
    ),
    (
        "an arbitrary type that is no request",
        "async fn misuse(client: FixtureBearer) {
            let _ = client.send(String::from(\"/widgets\")).full().await;
        }",
        "E0277",
        "EndpointSpec",
    ),
    (
        "another API's generated request through the typed send",
        "async fn misuse(client: FixtureBearer) {
            let _ = client.send(PingRequest::default()).full().await;
        }",
        "E0271",
        "PingRequest",
    ),
    (
        "another API's generated request through the raw send",
        "async fn misuse(client: FixtureBearer) {
            let _ = client.send_raw(PingRequest::default(), &CallOptions::new()).await;
        }",
        "E0271",
        "PingRequest",
    ),
    (
        "a response mutator on a text endpoint",
        "fn misuse() {
            let _ = FixtureBasic::new().mutate_response::<GetNoteRequest, _>(|_, _| Ok(()));
        }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a response mutator for another API's request",
        "fn misuse() {
            let _ = FixtureBearer::new().mutate_response::<PingRequest, _>(|_, _| Ok(()));
        }",
        "E0271",
        "PingRequest",
    ),
    (
        "another API's generated request through the link check",
        "fn misuse(client: FixtureBearer, link: Url) {
            let _ = client.check_link::<PingRequest>(&link);
        }",
        "E0271",
        "PingRequest",
    ),
    (
        "the removed client alias",
        "fn misuse(_: FixtureBearerClient) {}",
        "E0425",
        "FixtureBearerClient",
    ),
    (
        "the removed per-API builder",
        "fn misuse() {
            let _ = FixtureBearer::builder();
        }",
        "E0599",
        "builder",
    ),
    (
        "the removed generic request",
        "async fn misuse(client: FixtureBearer) {
            let _ = client.request::<serde_json::Value>(ListWidgetsRequest::default()).await;
        }",
        "E0599",
        "request",
    ),
    (
        "the removed request enum",
        "fn misuse() {
            let _ = FixtureBearerRequest::from(ListWidgetsRequest::default());
        }",
        "E0433",
        "FixtureBearerRequest",
    ),
    (
        "an endpoint's own setter after full",
        "async fn misuse(client: FixtureBearer) {
            let _ = client.list_widgets().full().limit(5).await;
        }",
        "E0599",
        "limit",
    ),
    (
        "the data, not the envelope, after full",
        "async fn misuse(client: FixtureBearer) {
            let _: WidgetList = client.list_widgets().full().await.unwrap();
        }",
        "E0308",
        "Response",
    ),
    (
        "a pager for an endpoint with no pagination",
        "fn misuse(client: FixtureBearer) {
            let _ = client.send(ListWidgetsRequest::default()).pager();
        }",
        "E0277",
        "is not paginated",
    ),
    (
        "a pager control the definition does not author",
        "fn misuse(client: schematic_schema::fixture_pages::FixturePages) {
            let _ = client.list_linked().pager().page_size(5);
        }",
        "E0277",
        "has no authored page size",
    ),
    (
        "pages of a session endpoint outside its scope",
        "fn misuse(client: schematic_schema::fixture_pages::FixturePages) {
            let _ = client.scan_entries().pages();
        }",
        "E0599",
        "no method named `pages`",
    ),
    (
        "the removed injected-client constructor",
        "fn misuse() {
            let _ = FixtureBearer::with_client_and_base_url(todo!(), \"http://127.0.0.1:1\");
        }",
        "E0599",
        "with_client_and_base_url",
    ),
];

/// What every misuse file imports; the control below proves these imports
/// and a correct use compile, so each failure is the misuse's own.
const MISUSE_PRELUDE: &str = "#![allow(dead_code, unused_imports, unreachable_code)]
use schematic_schema::fixture_basic::{FixtureBasic, GetNoteRequest};
use schematic_schema::fixture_bearer::{FixtureBearer, FixtureBearerApi, ListWidgetsRequest, WidgetList};
use schematic_schema::fixture_policy_key::PingRequest;
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{
    EndpointSpec, InvalidArgument, Method, NoDeclaredError, RequestParts, Url,
};
";

/// A use of endpoint methods from a file that imports only the API struct:
/// no request, call, or trait import is needed to call an endpoint, chain a
/// setter and per-call settings, await, ask for the full response, or build.
const ENDPOINT_METHODS_CONTROL: &str = "#![allow(dead_code)]
use schematic_schema::fixture_bearer::FixtureBearer;

async fn used(client: FixtureBearer, deadline: schematic_schema::schematic_runtime::Deadline) {
    let list = client.list_widgets().limit(10).anonymous().deadline(&deadline).await.unwrap();
    let _: usize = list.items.len();
    let full = client.list_widgets().limit(1).full().anonymous().await.unwrap();
    let _: u16 = full.meta.status.as_u16();
    let built = client.get_widget(\"w1\").build().unwrap();
    let again = built.clone();
    let _ = (built.await, again.await);
    let _ = schematic_schema::fixture_names::FixtureNames::new()
        .list_items(String::from(\"me\"))
        .full_param(true)
        .type_(String::new())
        .await;
}
";

/// The pager surface of freshly generated paginated endpoints, called from a
/// file that imports only the API struct and an item type.
const PAGER_CONTROL: &str = "#![allow(dead_code)]
use schematic_schema::fixture_pages::{Entry, FixturePages};

async fn used(client: FixturePages) {
    let mut pages = client.list_entries().state(String::from(\"open\")).page_size(10).pages();
    if let Ok(Some(page)) = pages.next().await {
        let _: Vec<Entry> = page.items;
    }
    if let Ok(page) = client.list_cursor().after(\"x\").fetch_page().await {
        let _: Vec<Entry> = page.items;
    }
    let _: Result<Vec<Entry>, _> = client.list_linked().collect_all().await;
    let _ = client.list_entries().page(2).page_limit(3).collect_all().await;
}

fn sends<T: Send>(_: T) {}

async fn scoped(client: FixturePages) {
    let session = client.scan_entries().close_grace(std::time::Duration::from_secs(1)).keep_alive(\"5m\").with_session(|scope| Box::pin(async move {
        let _: bool = scope.renew().await.unwrap_or_default();
        if let Ok(slices) = scope.partitioned(2) {
            for mut slice in slices {
                let _ = slice.next().await;
            }
        }
        scope.collect_all().await
    }));
    sends(client.scan_entries().with_session(|scope| Box::pin(async move {
        scope.fetch_page().await.map(|page| page.items.len())
    })));
    let _: Result<Result<Vec<Entry>, _>, _> = session.await;
}
";

#[test]
fn slow_generated_types_reject_misuse_at_compile_time() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let apis = fixture_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate(&api_refs, Definitions::Fixture(FIXTURE_TYPES), &lockfile);

    let control = "async fn used(client: FixtureBearer) {
        let response: Response<WidgetList, NoDeclaredError> =
            client.send(ListWidgetsRequest::default()).full().await.unwrap();
        let _ = response.data();
        let _ = FixtureBearer::new().mutate_response::<ListWidgetsRequest, _>(|_, _| Ok(()));
    }";
    fresh.add_test_file("misuse_control", &format!("{MISUSE_PRELUDE}{control}\n"));
    fresh.add_test_file("endpoint_methods_control", ENDPOINT_METHODS_CONTROL);
    fresh.add_test_file("pager_control", PAGER_CONTROL);
    for (index, (_, snippet, _, _)) in MISUSES.iter().enumerate() {
        fresh.add_test_file(&format!("misuse_{index}"), &format!("{MISUSE_PRELUDE}{snippet}\n"));
    }

    // One Cargo run checks every target; `--keep-going` reports each
    // target's errors instead of stopping at the first.
    let run = fresh.cargo_check_tests();
    assert!(!run.success, "the misuse targets must fail to compile");
    let hint = offline_cache_hint(&run.stderr).map(|hint| format!(" ({hint})")).unwrap_or_default();
    let diagnostics: Vec<&str> = run.stderr.split("\nerror").collect();
    // rustc prints the location with the host's separator.
    let located = |file: &str| -> Vec<&str> {
        let (unix, windows) = (format!("tests/{file}.rs:"), format!("tests\\{file}.rs:"));
        diagnostics
            .iter()
            .copied()
            .filter(|diagnostic| diagnostic.contains(&unix) || diagnostic.contains(&windows))
            .collect()
    };
    assert!(
        located("misuse_control").is_empty(),
        "the control, a correct use with the same imports, must compile{hint}:\n{}",
        run.stderr
    );
    assert!(
        located("endpoint_methods_control").is_empty(),
        "endpoint methods must compile with only the API struct imported{hint}:\n{}",
        run.stderr
    );
    assert!(
        located("pager_control").is_empty(),
        "the generated pager surface must compile{hint}:\n{}",
        run.stderr
    );
    for (index, (misuse, snippet, code, named)) in MISUSES.iter().enumerate() {
        let found = located(&format!("misuse_{index}"));
        assert!(
            found.iter().any(|diagnostic| diagnostic.starts_with(&format!("[{code}]")) && diagnostic.contains(named)),
            "{misuse} did not fail with {code} naming `{named}`{hint}:\n{snippet}\n\n{}",
            run.stderr
        );
    }
}

#[test]
fn slow_a_runtime_of_another_abi_fails_the_build_with_a_readable_message() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let runtime_copy = tempfile::TempDir::new().unwrap();
    let doctored = runtime_copy.path().join("schematic-runtime");
    doctored_runtime(&schematic_root().join("runtime"), &doctored, schematic_gen::RUNTIME_ABI + 1);

    let apis = fixture_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let fresh = FreshCrate::generate_with_runtime(
        &api_refs,
        Definitions::Fixture(FIXTURE_TYPES),
        &lockfile,
        &doctored,
    );

    let run = fresh.cargo_check();
    assert!(!run.success, "a crate generated for another runtime ABI must not build");
    let expected = format!(
        "schematic-schema was generated for schematic-runtime ABI {}",
        schematic_gen::RUNTIME_ABI
    );
    assert!(
        run.stderr.contains(&expected) && run.stderr.contains("just generate"),
        "the build should fail on the ABI assertion with its message{}:\n{}",
        offline_cache_hint(&run.stderr).map(|hint| format!(" ({hint})")).unwrap_or_default(),
        run.stderr
    );
}

/// Names whose definition in generated code would mean the generator still
/// emits part of the runtime, or a removed send path.
const RUNTIME_DEFINITIONS: &[&str] = &[
    "struct Response",
    "enum Outcome",
    "struct ApiError",
    "enum ErrorKind",
    "enum TransportError",
    "struct TransportError",
    "struct ResponseMeta",
    "struct RequestMeta",
    "struct Deadline",
    "trait EndpointSpec",
    "trait Decode",
    "enum ResponseKind",
    "enum SchematicError",
    "struct ResponseContext",
    "struct VariantHooks",
    "type RequestParts",
    "enum RequestBody",
    "enum FormPart",
    "fn redact",
    "fn append_query_pairs",
    "fn resolve_request_auth",
    "fn request<",
    "fn request_text",
    "fn request_bytes",
    "fn request_empty",
    "fn with_client",
    "fn with_base_url",
    "fn variant(&self",
    "fn with_api_key_param",
    "fn api_key_header",
    "fn http_client",
    "reqwest::",
    "std::env::var",
    "redirect::Policy",
];

#[test]
fn generated_rest_code_references_the_runtime_and_defines_none_of_it() {
    let apis = resolve_all_apis();
    let api_refs: Vec<&RestApi> = apis.iter().collect();
    let out = tempfile::TempDir::new().unwrap();
    generate_and_write_all(&api_refs, out.path(), false).expect("generate");

    let mut rest_files = Vec::new();
    for (relative, bytes) in snapshot_tree(out.path()) {
        let Some(bytes) = bytes else { continue };
        let websocket = relative.ends_with("_ws.rs") || relative == "ws_shared.rs";
        if relative.ends_with(".rs") && !websocket {
            rest_files.push((relative, String::from_utf8(bytes).expect("UTF-8 source")));
        }
    }
    assert!(rest_files.len() > 18, "every REST module and the shared files are scanned");
    for (relative, source) in &rest_files {
        for definition in RUNTIME_DEFINITIONS {
            assert!(!source.contains(definition), "{relative} contains `{definition}`");
        }
    }

    let source_of = |name: &str| &rest_files.iter().find(|(path, _)| path == name).unwrap().1;
    assert!(source_of("lib.rs").contains("pub use schematic_runtime;"));
    assert!(source_of("lib.rs").contains("schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI"));
    // GitHub's endpoint methods push its module over the split threshold.
    let github = source_of("github/requests.rs");
    assert!(github.contains("impl schematic_runtime::EndpointSpec for ListPullRequestsRequest"));
    let github_flat: String = github.split_whitespace().collect::<String>().replace(",)", ")");
    assert!(
        github_flat.contains(
            "pubfnlist_releases(&self,owner:implInto<String>,repo:implInto<String>)->ListReleasesCall<'_>"
        ),
        "GitHub has the endpoint method of the documented example"
    );
    // Every REST API has exactly one user-facing client name: its struct,
    // given its settings by the runtime macro. No alias, per-API builder, or
    // fallible constructor remains anywhere.
    let all_rest = rest_files
        .iter()
        .flat_map(|(_, source)| source.split_whitespace())
        .collect::<String>()
        .replace(",)", ")");
    for api in &apis {
        let name = &api.name;
        for expected in [
            format!("pub struct {name}(schematic_runtime::Client<{name}Api>);"),
            format!("pub enum {name}Api {{}}"),
            format!("schematic_runtime::api_client!({name}, {name}Api);"),
        ] {
            let squashed: String = expected.split_whitespace().collect();
            assert_eq!(all_rest.matches(&squashed).count(), 1, "{name}: `{expected}`");
        }
        assert!(!all_rest.contains(&format!("{name}Client")), "{name} still has a client alias");
    }
    for removed in ["ClientBuilder", "fnbuilder()", "BuildError", "pubtype"] {
        assert!(!all_rest.contains(removed), "generated REST code still contains `{removed}`");
    }
    for (relative, source) in &rest_files {
        let module = relative.trim_end_matches(".rs");
        if ["lib", "prelude", "shared"].contains(&module) || relative.ends_with("/requests.rs") || relative.ends_with("/responses.rs") {
            continue;
        }
        assert!(
            source.contains("schematic_runtime::"),
            "{relative} declares an API without referring to the runtime"
        );
    }
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
/// TCP listener so no HTTP library normalizes them. The query split, and the
/// move of request execution into `schematic-runtime`, must leave every line
/// byte-identical, with two recorded exceptions. A base URL that carries its
/// own query (`base-url-with-query`) now gets the declared pairs after `&`,
/// where the generated clients appended a second `?`. Pager-managed inputs
/// (Gitea `page`/`limit`, OpenAI `after`/`limit`) are no longer request
/// fields: they are sent through the pager, which appends them after the
/// request's own pairs, so they moved to the end of their lines with their
/// encoding unchanged.
const WIRE_BASELINE: &str = include_str!("../fixtures/wire_baseline/query_request_lines.txt");

/// Sends representative requests through the generated real clients to a
/// local listener and records `label<TAB>request line` per request. The same
/// file asserts what `into_parts` returns for those requests, so the
/// separated pairs are checked alongside the bytes the runtime produces from
/// them.
const WIRE_CAPTURE_TEST: &str = r#"
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use schematic_schema::gitea::{GetGitTreeRequest, Gitea, ListPullRequestsRequest};
use schematic_schema::gitlab::{GetRepositoryFileRequest, GitLab};
use schematic_schema::openai::{CreateResponseRequest, GetResponseRequest, DeleteResponseRequest, CancelResponseRequest, CompactconversationRequest, GetinputtokencountsRequest, ListInputItemsRequest, ListConversationItemsRequest, OpenAI};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{EndpointSpec, Method, RequestBody};

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

    let key = Credential::api_key("test-key");
    let gitea = Gitea::new().base_url(format!("{base}/api/v1")).build().unwrap();

    // `page` and `limit` are pager-managed, so they travel through the pager.
    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.state = Some("open closed".to_string());
    scalars.sort = Some("ünï+%&=/?#".to_string());
    let _ = gitea.send(scalars).credential(key.clone()).pager().page(2).page_size(1).fetch_page().await;
    record("ordered-scalars-reserved-unicode");

    let mut empty = ListPullRequestsRequest::new("o", "r");
    empty.state = Some(String::new());
    let _ = gitea.send(empty).credential(key.clone()).full().await;
    record("empty-value");

    let _ = gitea.send(ListPullRequestsRequest::new("o", "r")).credential(key.clone()).full().await;
    record("no-query");

    let mut tree = GetGitTreeRequest::new("o", "r", "a b");
    tree.recursive = Some(true);
    let _ = gitea.send(tree).credential(key.clone()).full().await;
    record("legacy-embedded-literal-query");
    for (label, recursive) in [("tree-recursive-omitted", None), ("tree-recursive-false", Some(false))] {
        let mut tree = GetGitTreeRequest::new("o", "r", "a b");
        tree.recursive = recursive;
        let parts = tree.clone().into_parts().unwrap();
        assert_eq!(parts.path, "/repos/o/r/git/trees/a%20b");
        assert_eq!(parts.query, recursive.map(|value| pairs(&[("recursive", if value { "true" } else { "false" })])).unwrap_or_default());
        let _ = gitea.send(tree).credential(key.clone()).full().await;
        record(label);
    }

    let tenant = Gitea::new().base_url(format!("{base}/api/v1?tenant=a b")).build().unwrap();
    let paged = ListPullRequestsRequest::new("o", "r");
    let _ = tenant.send(paged).credential(key.clone()).pager().page(1).fetch_page().await;
    record("base-url-with-query");

    let gitlab = GitLab::new().base_url(format!("{base}/api/v4")).build().unwrap();
    let _ = gitlab
        .send(
            GetRepositoryFileRequest::new("group/project", "dir/file name.rs").with_git_ref("feature/x y+z".to_string()),
        )
        .credential(key.clone())
        .full()
        .await;
    record("legacy-embedded-query-with-placeholder");
    let omitted = GetRepositoryFileRequest::new("group/project", "dir/file name.rs");
    assert!(omitted.clone().into_parts().unwrap().query.is_empty());
    let _ = gitlab.send(omitted).credential(key.clone()).full().await;
    record("file-ref-omitted");
    for (label, git_ref) in [("file-ref-unicode", "ünï+%&=/?#"), ("file-ref-empty", "")] {
        let request = GetRepositoryFileRequest::new("group/project", "dir/file name.rs").with_git_ref(git_ref.to_string());
        let parts = request.clone().into_parts().unwrap();
        assert_eq!(parts.path, "/projects/group%2Fproject/repository/files/dir%2Ffile%20name.rs");
        assert_eq!(parts.query, pairs(&[("ref", git_ref)]));
        let _ = gitlab.send(request).credential(key.clone()).full().await;
        record(label);
    }

    let openai = OpenAI::new().base_url(format!("{base}/v1")).build().unwrap();
    let token = Credential::access_token("test-token");
    let mut items = ListConversationItemsRequest::new("conv 1");
    items.order = Some("a+b".to_string());
    items.include = Some(vec![
        serde_json::json!("a b"),
        serde_json::json!("c+d"),
        serde_json::json!(""),
        serde_json::json!("é"),
    ]);
    let _ = openai.send(items).credential(token.clone()).pager().page_size(5).after("x%y").fetch_page().await;
    record("repeated-array-keys");

    let mut beta = ListInputItemsRequest::new("resp 1");
    beta.beta = Some(true);
    beta.include = Some(vec![serde_json::json!("x y")]);
    let _ = openai.send(beta).credential(token.clone()).pager().page_size(3).fetch_page().await;
    record("legacy-embedded-query-plus-declared");

    macro_rules! beta_pair {
        ($request:expr, $label:literal) => {{
            let mut request = $request;
            let ordinary = request.clone().into_parts().unwrap();
            assert!(!ordinary.path.contains('?'));
            assert!(!ordinary.query.iter().any(|(name, _)| name == "beta"));
            let _ = openai.send(request.clone()).anonymous().full().await.unwrap();
            record(concat!($label, "-ordinary"));
            request.beta = Some(true);
            let beta = request.clone().into_parts().unwrap();
            assert_eq!(beta.path, ordinary.path);
            assert_eq!(beta.method, ordinary.method);
            assert_eq!(beta.query, pairs(&[("beta", "true")]));
            let _ = openai.send(request.clone()).anonymous().full().await.unwrap();
            record(concat!($label, "-beta"));
            request.beta = Some(false);
            assert_eq!(request.clone().into_parts().unwrap().query, pairs(&[("beta", "false")]));
            let _ = openai.send(request).anonymous().full().await.unwrap();
            record(concat!($label, "-false"));
        }};
    }
    beta_pair!(CreateResponseRequest::new(Default::default()), "CreateResponse");
    beta_pair!(GetResponseRequest::new("resp 1"), "GetResponse");
    beta_pair!(DeleteResponseRequest::new("resp 1"), "DeleteResponse");
    beta_pair!(CancelResponseRequest::new("resp 1"), "CancelResponse");
    beta_pair!(CompactconversationRequest::new(Default::default()), "Compactconversation");
    beta_pair!(ListInputItemsRequest::new("resp 1"), "ListInputItems");
    beta_pair!(GetinputtokencountsRequest::new(Default::default()), "Getinputtokencounts");

    let out = std::env::var("WIRE_BASELINE_OUT").expect("WIRE_BASELINE_OUT");
    std::fs::write(out, lines.join("\n") + "\n").expect("write captured lines");
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn into_parts_returns_unencoded_query_pairs_apart_from_the_path() {
    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.state = Some("open closed".to_string());
    scalars.sort = Some("ünï+%&=/?#".to_string());
    let parts = scalars.into_parts().unwrap();
    assert_eq!(parts.method, Method::GET);
    assert_eq!(parts.path, "/repos/own%20er/re%2Bpo%25/pulls");
    assert_eq!(parts.body, RequestBody::Empty);
    assert_eq!(parts.query, pairs(&[("state", "open closed"), ("sort", "ünï+%&=/?#")]));
    assert!(
        !parts.headers.iter().any(|(name, _)| ["state", "sort"].contains(&name.as_str())),
        "query parameters must not travel as headers: {:?}",
        parts.headers
    );

    let mut empty = ListPullRequestsRequest::new("o", "r");
    empty.state = Some(String::new());
    assert_eq!(empty.into_parts().unwrap().query, pairs(&[("state", "")]));

    let parts = ListPullRequestsRequest::new("o", "r").into_parts().unwrap();
    assert_eq!((parts.path.as_str(), parts.query), ("/repos/o/r/pulls", vec![]));

    let mut tree = GetGitTreeRequest::new("o", "r", "a b");
    tree.recursive = Some(true);
    let parts = tree.into_parts().unwrap();
    assert_eq!((parts.path.as_str(), parts.query), ("/repos/o/r/git/trees/a%20b", pairs(&[("recursive", "true")])));
    let parts = GetRepositoryFileRequest::new("group/project", "dir/file name.rs").with_git_ref("feature/x y+z".to_string())
        .into_parts()
        .unwrap();
    assert_eq!(
        (parts.path.as_str(), parts.query),
        ("/projects/group%2Fproject/repository/files/dir%2Ffile%20name.rs", pairs(&[("ref", "feature/x y+z")]))
    );

    let mut items = ListConversationItemsRequest::new("conv 1");
    items.include = Some(vec![serde_json::json!("a b"), serde_json::json!("")]);
    let parts = items.into_parts().unwrap();
    assert_eq!(parts.path, "/conversations/conv%201/items");
    assert_eq!(parts.query, pairs(&[("include", "\"a b\""), ("include", "\"\"")]));

    let mut beta = ListInputItemsRequest::new("resp 1");
    beta.beta = Some(true);
    beta.include = Some(vec![serde_json::json!("x y")]);
    let parts = beta.into_parts().unwrap();
    assert_eq!(parts.path, "/responses/resp%201/input_items");
    assert_eq!(parts.query, pairs(&[("include", "\"x y\""), ("beta", "true")]));
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
    let test = "into_parts_returns_unencoded_query_pairs_apart_from_the_path";
    assert!(report.contains(test), "the child run did not report {test}:\n{report}");

    let captured = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        captured, WIRE_BASELINE,
        "the generated clients put different request lines on the wire"
    );
}
