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
    AddressRule, ApiKeyLocation, ApiRequest, AuthMethod, AuthPolicy, AuthStrategy,
    AuthorizationCodeGrant, CrossOrigin, Endpoint, EndpointParams, EnvList, ErrorEntry, FormField,
    GraphQlOperation, GraphQlSurface, GraphQlVariables, OAuth2ClientAuthMethod, OAuth2Config, ParamDef,
    ParamStyle, PkceRequirement, QueryParamType, ResponseShape, RestApi, RestMethod, StatusMatch, StreamEnd,
    StreamFraming, SuccessShape, TypeRef,
};
use schematic_define::pagination::{
    Collection, CursorSource, InputParam, KeepAlive, LinkSource, MoreSignal, PageSize,
    Partition, Relay, Session, SessionStep, Strategy, ValueSource,
};
use schematic_gen::output::{
    RustOutputRequest, Selection, generate_and_write_all, prepare_rust_output,
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
            "required-query-beside-adopted",
        ],
    ),
    (
        "FixtureQuery",
        &[
            "required-query-parameters",
            "query-enums",
            "exploded-and-delimited-arrays",
            "all-optional-default",
            "unserializable-query-left-out",
        ],
    ),
    (
        "FixtureShapes",
        &[
            "json-and-sse-success-enum",
            "ndjson-stream-kept-terminator",
            "byte-stream",
            "named-variants-and-fallback",
            "status-selected-json-shapes",
            "api-error-defaults",
            "endpoint-error-overlay",
            "error-opt-out",
            "single-and-enum-failures",
        ],
    ),
    ("FixtureGraphQl", &["graphql-typed-variables", "graphql-unit-variables", "graphql-address-rules", "graphql-http-errors",
        "graphql-relay-pager", "graphql-check-status-parity", "graphql-unparsed-document", "graphql-anonymous-bearer"]),
    (
        "FixtureRules",
        &[
            "value-rules-parameters",
            "value-rules-json-body",
            "value-rules-custom-serialization",
            "value-rules-recursive-component",
            "value-rules-text-body",
            "value-rules-form-fields",
            "value-rules-declared-beside-extra-values",
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
    ("fixture_query", ""),
    (
        "fixture_shapes",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Delta {
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    pub n: u32,
    pub done: bool,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Listing {
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Record {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Job {
    pub job: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Problem {
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Conflict {
    pub reason: String,
}
"#,
    ),
    (
        "fixture_graphql",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Login {
    pub login: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ViewerData {
    pub viewer: Login,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CheckVars {
    pub owner: String,
    pub name: String,
    pub n0: i64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CheckData {
    pub repository: Option<serde_json::Value>,
}

/// Relay variables: the pager writes `first` and `after`, so both are
/// optional; `after` is sent `null` until the pager has a cursor.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssuesVars {
    pub owner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<u32>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IssuesData {
    pub repository: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Issue {
    pub number: u64,
}

/// Variables of sniff's check-status query for three pull requests.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityVars {
    pub owner: String,
    pub name: String,
    pub n0: u64,
    pub n1: u64,
    pub n2: u64,
}

/// `data` of sniff's check-status query; every field the query can null is
/// an `Option`.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityData {
    pub repository: Option<ParityRepository>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityRepository {
    pub p0: Option<ParityPullRequest>,
    pub p1: Option<ParityPullRequest>,
    pub p2: Option<ParityPullRequest>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityPullRequest {
    pub state: String,
    pub commits: ParityCommits,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityCommits {
    pub nodes: Vec<ParityCommitNode>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityCommitNode {
    pub commit: ParityCommit,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityCommit {
    pub oid: String,
    pub status_check_rollup: Option<ParityRollup>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityRollup {
    pub contexts: ParityContexts,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParityContexts {
    pub check_run_count: u64,
    pub status_context_count: u64,
    pub check_run_counts_by_state: Vec<ParityStateCount>,
    pub status_context_counts_by_state: Vec<ParityStateCount>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParityStateCount {
    pub state: String,
    pub count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Problem {
    pub message: String,
}
"#,
    ),
    (
        "fixture_rules",
        r#"
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub qty: i64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Extra {
    pub note: String,
}

/// A renamed, a skipped, and a flattened field: its rules are checked on
/// the JSON it serializes to.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NewItem {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(skip)]
    pub internal: String,
    #[serde(flatten)]
    pub extra: Extra,
    pub price: f64,
    pub lines: Vec<Line>,
    pub labels: std::collections::BTreeMap<String, String>,
    pub parent: Option<Box<NewItem>>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Meta {
    pub title: String,
}

/// A declared property beside extra entries whose values have rules.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mixed {
    pub label: String,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Holder {
    pub inner: Mixed,
}
"#,
    ),
];

fn endpoint(id: &str, method: RestMethod, path: &str, response: Vec<SuccessShape>) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("{id} fixture endpoint"); fixture.request = None; fixture.response = response; fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

fn fixture_api(name: &str, module: &str, auth: AuthStrategy, endpoints: Vec<Endpoint>) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{name} fixture API"); fixture.base_url = "http://127.0.0.1:1".to_string(); fixture.docs_url = None; fixture.auth = auth; fixture.auth_policy = None; fixture.env_auth = vec![format!("{}_TOKEN", module.to_uppercase())]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = endpoints; fixture.module_path = Some(module.to_string()); fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::common::components::with_referenced_components(fixture) }
}

fn named_param(name: &str, param_type: QueryParamType) -> ParamDef {
    ParamDef { constraints: None, deprecated: false, rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required: false,
        description: None,
        param_type,
        explode: false,
        style: ParamStyle::Form,
    }
}

fn located_params(query: Vec<ParamDef>, header: Vec<ParamDef>, cookie: Vec<ParamDef>) -> EndpointParams {
    EndpointParams { path: vec![], query, header, cookie }
}

/// A page-number default (adopting `page` and `per_page`, keeping `state`),
/// a resumable cursor on a wrapper list, a body next link, a page-number
/// list whose adopted `page` and `limit` are required (R2a, R11a), and a
/// point-in-time-style session with a keep-alive, renew, close, and
/// partitions (R22, R23).
fn pages_fixture() -> RestApi {
    let int = |name| named_param(name, QueryParamType::Integer);
    let mut list_entries =
        endpoint("ListEntries", RestMethod::Get, "/entries", vec![SuccessShape::json_vec_type("Entry")]);
    list_entries.params = Some(located_params(
        vec![int("page"), int("per_page"), named_param("state", QueryParamType::String)],
        vec![],
        vec![],
    ));
    let mut list_cursor =
        endpoint("ListCursor", RestMethod::Get, "/cursor", vec![SuccessShape::json_type("EntryCursorPage")]);
    list_cursor.params = Some(located_params(vec![named_param("after", QueryParamType::String)], vec![], vec![]));
    let list_linked =
        endpoint("ListLinked", RestMethod::Get, "/linked", vec![SuccessShape::json_type("EntryLinkPage")]);
    let mut allowed_linked = list_linked.clone();
    allowed_linked.id = "ListAllowedLinked".to_string();
    allowed_linked.path = "/allowed-linked".to_string();
    let required = |name| ParamDef { rust_name: None, secret: false, pager_managed: false, required: true, ..int(name) };
    let mut list_required =
        endpoint("ListRequired", RestMethod::Get, "/required", vec![SuccessShape::json_vec_type("Entry")]);
    // `scope` is required and no strategy adopts it: the one `new()` argument.
    let scope = ParamDef { required: true, ..named_param("scope", QueryParamType::String) };
    list_required.params = Some(located_params(vec![required("page"), scope, required("limit")], vec![], vec![]));
    let scan_entries = endpoint("ScanEntries", RestMethod::Get, "/scan", vec![SuccessShape::json_vec_type("Entry")]);

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
        vec![SuccessShape::json_type("WidgetList")],
    );
    list_widgets.params = Some(EndpointParams::default().with_query_params(vec![
        ("limit", QueryParamType::Integer, false, None),
        ("q", QueryParamType::String, false, None),
        ("tag", QueryParamType::Array(Box::new(QueryParamType::String)), false, None),
    ]));
    let mut create_widget =
        endpoint("CreateWidget", RestMethod::Post, "/widgets", vec![SuccessShape::json_type("Widget")]);
    create_widget.request = Some(ApiRequest::json_type("CreateWidget"));
    let get_widget = endpoint(
        "GetWidget",
        RestMethod::Get,
        "/widgets/{widget_id}",
        vec![SuccessShape::json_type("Widget")],
    );
    // Static endpoint headers, including a Cookie, that parameters must
    // override (header) or merge with (cookie).
    let mut search = endpoint("SearchWidgets", RestMethod::Get, "/widgets/search", vec![SuccessShape::empty()]);
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

    let mut upload = endpoint("UploadFile", RestMethod::Post, "/files", vec![SuccessShape::json_type("Accepted")]);
    upload.request = Some(ApiRequest::form_data(vec![
        FormField::text("purpose"),
        FormField::file("file"),
    ]));
    let mut submit = endpoint("SubmitForm", RestMethod::Post, "/forms", vec![SuccessShape::json_type("Accepted")]);
    submit.request = Some(ApiRequest::url_encoded(vec![
        FormField::text("name"),
        FormField::text("note").optional(),
    ]));

    let mut send_note = endpoint("SendNote", RestMethod::Post, "/notes", vec![SuccessShape::empty()]);
    send_note.request = Some(ApiRequest::Text {
        content_type: "text/plain; charset=utf-8".to_string(),
    });
    let mut upload_blob = endpoint("UploadBlob", RestMethod::Put, "/blobs/{name}", vec![SuccessShape::empty()]);
    upload_blob.request = Some(ApiRequest::Binary {
        content_type: "application/octet-stream".to_string(),
    });

    // The declared `api_key` query parameter shares the credential's name.
    let mut list_things = endpoint("ListThings", RestMethod::Get, "/things", vec![SuccessShape::empty()]);
    list_things.params = Some(located_params(
        vec![named_param("q", QueryParamType::String), named_param("api_key", QueryParamType::String)],
        vec![],
        vec![],
    ));
    // The declared `sid` cookie parameter shares the credential's name.
    let mut get_profile = endpoint("GetProfile", RestMethod::Get, "/profile", vec![SuccessShape::empty()]);
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
        vec![endpoint("Ping", RestMethod::Get, "/ping", vec![SuccessShape::empty()])],
    );
    policy_key.auth_policy = Some(AuthPolicy {
        explicit: vec![AuthMethod::ApiKeyParam {
            name: "token".to_string(),
            location: ApiKeyLocation::Query,
        }],
        env_fallback: None,
    });
    let oauth = OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
        authorization_url: "http://127.0.0.1:1/authorize".to_string(),
        token_url: "http://127.0.0.1:1/token".to_string(),
        revocation_url: None,
        default_scopes: vec!["read".to_string()],
        pkce: PkceRequirement::Required,
        client_auth: OAuth2ClientAuthMethod::None,
    });

    // Secret names, a request-id header, and the tightest cross-origin policy,
    // all from the definition. Neither secret name is a built-in one.
    let mut echo = endpoint("Echo", RestMethod::Get, "/echo", vec![SuccessShape::json_type("Echo")]);
    echo.params = Some(located_params(
        vec![ParamDef { secret: true, ..named_param("pin_code", QueryParamType::String) }, named_param("page", QueryParamType::String)],
        vec![ParamDef { secret: true, ..named_param("X-Pin", QueryParamType::String) }],
        vec![],
    ));
    let mut settings = fixture_api(
        "FixtureSettings",
        "fixture_settings",
        AuthStrategy::None,
        vec![echo, endpoint("Moved", RestMethod::Get, "/moved", vec![SuccessShape::empty()])],
    );
    settings.request_id_header = Some("x-request-id".to_string());
    settings.cross_origin = CrossOrigin::deny();

    // Names the generator must rename: a `new` endpoint method (an API-struct
    // method), a `full` setter (a shared call method), and a keyword setter.
    let mut list_items = endpoint("ListItems", RestMethod::Get, "/owners/{owner}/items", vec![SuccessShape::empty()]);
    list_items.params = Some(located_params(
        vec![
            named_param("full", QueryParamType::Boolean),
            named_param("type", QueryParamType::String),
            named_param("limit", QueryParamType::Integer),
        ],
        vec![],
        vec![],
    ));
    let mut tree = endpoint("GetTree", RestMethod::Get, "/tree", vec![SuccessShape::empty()]);
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
            endpoint("New", RestMethod::Post, "/new", vec![SuccessShape::empty()]), list_items, tree,
            endpoint("AllowedMove", RestMethod::Get, "/allowed-move", vec![SuccessShape::empty()]),
            endpoint("SiblingMove", RestMethod::Get, "/sibling-move", vec![SuccessShape::empty()]),
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
            AuthStrategy::Basic { username_env: EnvList::new(vec![]), password_env: EnvList::single("FIXTURE_BASIC_TOKEN") },
            vec![
                endpoint("DeleteThing", RestMethod::Delete, "/things/{id}", vec![SuccessShape::empty()]),
                endpoint("GetNote", RestMethod::Get, "/note", vec![SuccessShape::text()]),
                endpoint("GetBlob", RestMethod::Get, "/blob", vec![SuccessShape::binary()]),
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
            vec![endpoint("WhoAmI", RestMethod::Get, "/me", vec![SuccessShape::empty()])],
        ),
        settings,
        names,
        pages_fixture(),
        query_fixture(),
        shapes_fixture(),
        graphql_fixture(),
        rules_fixture(),
    ]
}

fn named(name: &str) -> TypeRef { TypeRef::Named(name.to_string()) }


fn shape(kind: ResponseShape, edit: impl FnOnce(&mut SuccessShape)) -> SuccessShape {
    let mut shape = SuccessShape::new(kind);
    edit(&mut shape);
    shape
}

/// Several success shapes, streams, and declared errors.
fn shapes_fixture() -> RestApi {
    let mut create = endpoint("CreateMessage", RestMethod::Post, "/messages", vec![
        shape(ResponseShape::Json(named("Message")), |s| s.note = Some("returned when `stream` is false or omitted".into())),
        shape(
            ResponseShape::Stream { framing: StreamFraming::Sse(named("Delta")), end: StreamEnd::DataLineEquals("[DONE]".into()), keep: false },
            |s| s.note = Some("returned when `stream` is true".into()),
        ),
    ]);
    create.errors = vec![ErrorEntry::new(StatusMatch::exact(404), ResponseShape::Text)];
    let chunks = endpoint("StreamChunks", RestMethod::Get, "/chunks", vec![SuccessShape::new(ResponseShape::Stream {
        framing: StreamFraming::Ndjson(named("Chunk")),
        end: StreamEnd::FieldEquals { field: "done".into(), value: serde_json::json!(true) },
        keep: true,
    })]);
    let mut download = endpoint("Download", RestMethod::Get, "/download", vec![shape(
        ResponseShape::Stream { framing: StreamFraming::Bytes, end: StreamEnd::ConnectionClose, keep: false },
        |s| s.content_type = Some("audio/mpeg".into()),
    )]);
    download.no_declared_errors = true;
    let source = endpoint("GetSource", RestMethod::Get, "/source/{path}", vec![
        shape(ResponseShape::Json(named("Listing")), |s| s.variant = Some("Directory".into())),
        shape(ResponseShape::Binary, |s| { s.fallback = true; s.variant = Some("File".into()); }),
    ]);
    let mut submit = endpoint("Submit", RestMethod::Post, "/jobs", vec![
        shape(ResponseShape::Json(named("Record")), |s| { s.statuses = vec![200]; s.variant = Some("Record".into()); }),
        shape(ResponseShape::Json(named("Job")), |s| { s.statuses = vec![202]; s.variant = Some("Job".into()); }),
    ]);
    submit.errors = vec![ErrorEntry::new(StatusMatch::exact(409), ResponseShape::Json(named("Conflict")))];
    let mut api = fixture_api("FixtureShapes", "fixture_shapes", AuthStrategy::None, vec![create, chunks, download, source, submit]);
    api.errors = vec![ErrorEntry::new(StatusMatch::class(5), ResponseShape::Json(named("Problem")))];
    crate::common::components::with_referenced_components(api)
}

/// Value rules on every kind of request input: query and header
/// parameters (checked as typed values), a JSON body whose stored
/// components carry rules (a renamed, a skipped, and a flattened field, a
/// map, array items, a decimal `multipleOf`, and a recursive reference), a
/// text body, and multipart text and JSON fields.
fn rules_fixture() -> RestApi {
    use schematic_define::constraints::Constraints;
    use schematic_define::SchemaComponent;

    let rules = |param: ParamDef, constraints: Constraints| ParamDef { constraints: Some(Box::new(constraints)), ..param };
    let mut list = endpoint("ListItems", RestMethod::Get, "/items", vec![SuccessShape::empty()]);
    list.params = Some(located_params(
        vec![
            rules(named_param("limit", QueryParamType::Integer), Constraints::new().minimum(1).maximum(100)),
            rules(named_param("name", QueryParamType::String), Constraints::new().min_length(1).max_length(5)),
            rules(
                named_param("ids", QueryParamType::Array(Box::new(QueryParamType::Integer))),
                Constraints::new().min_items(1).max_items(3).unique_items().items(Constraints::new().minimum(1)),
            ),
        ],
        vec![rules(named_param("X-Tag", QueryParamType::String), Constraints::new().pattern("^[a-z]+$"))],
        vec![],
    ));
    let mut create = endpoint("CreateItem", RestMethod::Post, "/items", vec![SuccessShape::empty()]);
    create.request = Some(ApiRequest::json_type("NewItem"));
    let mut text = endpoint("SendText", RestMethod::Post, "/text", vec![SuccessShape::empty()]);
    text.request = Some(ApiRequest::Text { content_type: "text/plain".to_string() });
    text.request_constraints = Some(Box::new(Constraints::new().max_length(3)));
    let mut upload = endpoint("Upload", RestMethod::Post, "/upload", vec![SuccessShape::empty()]);
    upload.request = Some(ApiRequest::form_data(vec![
        FormField::text("purpose").constraints(Constraints::new().allowed_values(["a", "b"])),
        FormField::json("meta", named("Meta")),
    ]));

    // `label` checks nothing itself; it must still never take the
    // extra-value rule, at the root, through a named reference, or in a
    // multipart JSON part.
    let mut mixed = endpoint("CreateMixed", RestMethod::Post, "/mixed", vec![SuccessShape::empty()]);
    mixed.request = Some(ApiRequest::json_type("Mixed"));
    let mut holder = endpoint("CreateHolder", RestMethod::Post, "/holder", vec![SuccessShape::empty()]);
    holder.request = Some(ApiRequest::json_type("Holder"));
    let mut upload_mixed = endpoint("UploadMixed", RestMethod::Post, "/upload-mixed", vec![SuccessShape::empty()]);
    upload_mixed.request = Some(ApiRequest::form_data(vec![FormField::json("mixed", named("Mixed"))]));

    let mut api = fixture_api(
        "FixtureRules",
        "fixture_rules",
        AuthStrategy::None,
        vec![list, create, text, upload, mixed, holder, upload_mixed],
    );
    let component = |name: &str, schema: serde_json::Value| SchemaComponent { name: name.to_string(), rust_path: Some(name.to_string()), schema };
    api.schemas = vec![
        component("NewItem", serde_json::json!({
            "type": "object",
            "properties": {
                "displayName": {"type": "string", "maxLength": 10},
                "internal": {"type": "string", "maxLength": 1},
                "note": {"type": "string", "maxLength": 4},
                "price": {"type": "number", "format": "double", "multipleOf": 0.5},
                "lines": {"type": "array", "maxItems": 3, "items": {"$ref": "#/components/schemas/Line"}},
                "labels": {"type": "object", "additionalProperties": {"type": "string", "maxLength": 8}},
                "parent": {"anyOf": [{"$ref": "#/components/schemas/NewItem"}, {"type": "null"}]},
            },
        })),
        component("Line", serde_json::json!({
            "type": "object",
            "properties": {"qty": {"type": "integer", "format": "int64", "minimum": 1}},
        })),
        component("Meta", serde_json::json!({
            "type": "object",
            "properties": {"title": {"type": "string", "minLength": 1}},
        })),
        component("Mixed", serde_json::json!({
            "type": "object",
            "properties": {"label": {"type": "string"}},
            "additionalProperties": {"type": "string", "maxLength": 3},
        })),
        component("Holder", serde_json::json!({
            "type": "object",
            "properties": {"inner": {"$ref": "#/components/schemas/Mixed"}},
        })),
    ];
    api
}

/// The document sniff's `batch_query(3)` builds
/// (`sniff/lib/src/remote/check_status.rs`); the guard test
/// `the_parity_document_is_sniffs_check_status_query` re-derives it from
/// that source.
const CHECK_STATUS_DOCUMENT: &str = "query($owner:String!,$name:String!,$n0:Int!,$n1:Int!,$n2:Int!){repository(owner:$owner,name:$name){p0:pullRequest(number:$n0){...F}p1:pullRequest(number:$n1){...F}p2:pullRequest(number:$n2){...F}}}fragment F on PullRequest{state commits(last:1){nodes{commit{oid statusCheckRollup{contexts(first:1){checkRunCount statusContextCount checkRunCountsByState{state count} statusContextCountsByState{state count}}}}}}}";

/// GraphQL operations beside one REST endpoint, under the GitHub address
/// rules. The definition's base ends in `/api/v3`; tests point it at a mock.
fn graphql_fixture() -> RestApi {
    let mut ctx = schematic_define::DefinitionContext::types_in("::fixture");
    let mut viewer = GraphQlOperation::query("Viewer", "query { viewer { login } }")
        .describes("The signed-in user").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
    viewer.data = named("ViewerData");
    let mut check = GraphQlOperation::query("CheckStatus", "query CheckStatus($owner: String!, $name: String!, $n0: Int!) { repository(owner: $owner, name: $name) { p0: pullRequest(number: $n0) { number } } }")
        .describes("Check status of pull requests").no_variables().data::<serde_json::Value>(&mut ctx)
        .operation_name("CheckStatus").build().unwrap();
    check.variables = GraphQlVariables::Typed(named("CheckVars"));
    check.data = named("CheckData");
    check.errors = vec![ErrorEntry::new(StatusMatch::exact(403), ResponseShape::Json(named("Problem")))];
    let mut rename = GraphQlOperation::mutation("Rename", "mutation { rename { login } }")
        .describes("Rename the signed-in user").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
    rename.data = named("Login");
    rename.no_declared_errors = true;
    let mut issues = GraphQlOperation::query("Issues", "query Issues($owner: String!, $first: Int, $after: String) { repository(owner: $owner) { issues(first: $first, after: $after) { nodes { number } pageInfo { endCursor hasNextPage } } } }")
        .describes("Issues of a repository, page by page").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
    issues.variables = GraphQlVariables::Typed(named("IssuesVars"));
    issues.pagination = Some(Relay::forward("repository.issues").max_page_size(50).collection("Issue"));
    issues.data = named("IssuesData");
    // R43: sniff's check-status query for three pull requests, unnamed, with
    // no declared HTTP errors so a 403 keeps its raw body.
    let mut parity = GraphQlOperation::query("CheckStatusParity", CHECK_STATUS_DOCUMENT)
        .describes("Sniff's check-status query, for parity").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
    parity.variables = GraphQlVariables::Typed(named("ParityVars"));
    parity.data = named("ParityData");
    // Not valid GraphQL: the document is sent as written, never parsed.
    let mut draft = GraphQlOperation::mutation("Draft", "mutation { draft(title: \"unterminated")
        .describes("A document the server rejects").no_variables().data::<serde_json::Value>(&mut ctx).build().unwrap();
    draft.data = named("Login");
    let mut api = fixture_api("FixtureGraphQl", "fixture_graphql", AuthStrategy::BearerToken { header: None }, vec![
        endpoint("GetUser", RestMethod::Get, "/user", vec![SuccessShape::json_type("Login")]),
    ]);
    api.base_url = "http://127.0.0.1:1/api/v3".to_string();
    let mut surface = GraphQlSurface::new(AddressRule::github());
    surface.operations = vec![viewer, check, rename, issues, parity, draft];
    api.graphql = Some(surface);
    crate::common::components::with_referenced_components(api)
}

/// A query parameter with every field the generator reads.
fn query_param(name: &str, required: bool, param_type: QueryParamType, style: ParamStyle, explode: bool) -> ParamDef {
    ParamDef { required, style, explode, ..named_param(name, param_type) }
}

/// Every supported query form (PD8): required scalars and enums, optional
/// numbers and booleans, exploded and delimited arrays of strings and enums
/// whose values hold the delimiters, and a JSON value the client leaves out;
/// beside it an all-optional request that keeps `Default`.
fn query_fixture() -> RestApi {
    let strings = || QueryParamType::Array(Box::new(QueryParamType::String));
    let enums = |values: &[&str]| QueryParamType::Enum(values.iter().map(|value| value.to_string()).collect());
    let mut find = endpoint("Find", RestMethod::Get, "/find", vec![SuccessShape::empty()]);
    find.params = Some(located_params(
        vec![
            query_param("q", true, QueryParamType::String, ParamStyle::Form, false),
            query_param("kind", true, enums(&["plain", "ext-ra"]), ParamStyle::Form, false),
            query_param("n", false, QueryParamType::Integer, ParamStyle::Form, true),
            query_param("ratio", false, QueryParamType::Number, ParamStyle::Form, true),
            query_param("flag", false, QueryParamType::Boolean, ParamStyle::Form, true),
            query_param("tags", false, strings(), ParamStyle::Form, true),
            query_param("csv", false, strings(), ParamStyle::Form, false),
            query_param("spaced", false, strings(), ParamStyle::SpaceDelimited, false),
            query_param("piped", false, strings(), ParamStyle::PipeDelimited, false),
            query_param("kinds", false, QueryParamType::Array(Box::new(enums(&["a,b", "c|d", "e f"]))), ParamStyle::Form, false),
            query_param("marks", false, QueryParamType::Array(Box::new(enums(&["x", "y"]))), ParamStyle::Form, true),
            query_param("meta", false, QueryParamType::Json, ParamStyle::Form, true),
        ],
        vec![],
        vec![],
    ));
    let mut browse = endpoint("Browse", RestMethod::Get, "/browse", vec![SuccessShape::empty()]);
    browse.params = Some(located_params(
        vec![
            query_param("page", false, QueryParamType::Integer, ParamStyle::Form, true),
            query_param("sort", false, enums(&["asc", "desc"]), ParamStyle::Form, true),
        ],
        vec![],
        vec![],
    ));
    fixture_api("FixtureQuery", "fixture_query", AuthStrategy::None, vec![find, browse])
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
use schematic_schema::fixture_query::{
    BrowseRequest, BrowseRequestSort, FindRequest, FindRequestKind, FindRequestKinds, FindRequestMarks, FixtureQuery,
};
use schematic_schema::fixture_settings::{Echo, EchoRequest, FixtureSettings, MovedRequest};
use schematic_schema::fixture_rules::{
    CreateHolderRequest, CreateItemRequest, CreateMixedRequest, Extra, FixtureRules, Holder, Line, ListItemsRequest, Meta, Mixed,
    NewItem, SendTextRequest, UploadForm, UploadMixedForm, UploadMixedRequest, UploadRequest,
};
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
    assert_eq!(query(&request), "limit=5&q=x&tag=a,b", "a form array without explode is comma-delimited");
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

    // `new()` takes only `scope`: `page` and `limit` are required too, but
    // the strategy adopts them.
    let entries = client.list_required("team a").await.unwrap();
    assert!(entries.is_empty(), "an ordinary send returns the endpoint's own response");
    let page = client.list_required("team a").page(3).page_size(5).fetch_page().await.unwrap();
    assert_eq!(page.number, 1);

    let received = server.received_requests().await.unwrap();
    let queries: Vec<String> = received.iter().map(query).collect();
    assert_eq!(
        queries,
        ["scope=team%20a&page=1&limit=25", "scope=team%20a&page=3&limit=5"],
        "the ordinary send carries the first page and the authored size; the pager replaces both"
    );
}

#[tokio::test]
async fn every_page_carries_the_required_value_beside_the_changing_adopted_ones() {
    use wiremock::matchers::query_param;
    let server = MockServer::start().await;
    for (page, body) in [("1", r#"[{"id":"one"}]"#), ("2", r#"[{"id":"two"}]"#)] {
        Mock::given(query_param("page", page))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
    }
    Mock::given(any()).respond_with(ResponseTemplate::new(200).set_body_string("[]")).mount(&server).await;
    let client = FixturePages::new().base_url(server.uri()).build().unwrap();

    let items = client.list_required("x,y").collect_all().await.unwrap();

    let ids: Vec<&str> = items.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, ["one", "two"]);
    let queries: Vec<String> = server.received_requests().await.unwrap().iter().map(query).collect();
    assert_eq!(
        queries,
        ["scope=x%2Cy&page=1&limit=25", "scope=x%2Cy&page=2&limit=25", "scope=x%2Cy&page=3&limit=25"]
    );
}

#[tokio::test]
async fn query_values_reach_the_wire_in_their_declared_form() {
    let server = start_server().await;
    let client = FixtureQuery::new().base_url(server.uri()).build().unwrap();
    let strings = |values: &[&str]| values.iter().map(|value| value.to_string()).collect::<Vec<_>>();

    client
        .find("a b&c=d/é", FindRequestKind::ExtRa)
        .n(-3)
        .ratio(0.5)
        .flag(false)
        .tags(strings(&["a,b", "c d", ""]))
        .csv(strings(&["a,b", "c%2C", "ü"]))
        .spaced(strings(&["x y", "z"]))
        .piped(strings(&["p|q", "r"]))
        .kinds(vec![FindRequestKinds::Ab, FindRequestKinds::Cd, FindRequestKinds::EF])
        .marks(vec![FindRequestMarks::X, FindRequestMarks::Y])
        .await
        .unwrap();

    let request = only_request(&server).await;
    assert_eq!(
        query(&request),
        "q=a%20b%26c%3Dd%2F%C3%A9&kind=ext-ra&n=-3&ratio=0.5&flag=false\
         &tags=a%2Cb&tags=c%20d&tags=\
         &csv=a%2Cb,c%252C,%C3%BC\
         &spaced=x%20y%20z\
         &piped=p%7Cq%7Cr\
         &kinds=a%2Cb,c%7Cd,e%20f\
         &marks=x&marks=y",
        "each element is encoded once; a delimiter inside an element stays escaped"
    );
}

#[tokio::test]
async fn unset_values_are_omitted_and_empty_arrays_are_sent_empty() {
    let server = start_server().await;
    let client = FixtureQuery::new().base_url(server.uri()).build().unwrap();

    client.find("x", FindRequestKind::Plain).await.unwrap();
    let empty = FindRequest::new("x", FindRequestKind::Plain)
        .with_tags(vec![])
        .with_csv(vec![])
        .with_spaced(vec![])
        .with_piped(vec![])
        .with_kinds(vec![])
        .with_marks(vec![]);
    client.send(empty).await.unwrap();
    // A setter replaces a required value.
    client.send(FindRequest::new("x", FindRequestKind::Plain).with_q("y".to_string())).await.unwrap();
    // An all-optional request keeps `Default`.
    client.send(BrowseRequest::default()).await.unwrap();
    client.send(BrowseRequest::default().with_sort(BrowseRequestSort::Desc).with_page(2)).await.unwrap();

    let queries: Vec<String> = server.received_requests().await.unwrap().iter().map(query).collect();
    assert_eq!(
        queries,
        [
            "q=x&kind=plain",
            "q=x&kind=plain&tags=&csv=&spaced=&piped=&kinds=&marks=",
            "q=y&kind=plain",
            "",
            "page=2&sort=desc",
        ]
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

// ---- response model v2: several shapes, streams, declared errors ----

async fn reply(server: &MockServer, at: &str, template: ResponseTemplate) {
    server.reset().await;
    Mock::given(path(at.to_string())).respond_with(template).mount(server).await;
}

#[tokio::test]
async fn a_multi_shape_endpoint_returns_the_variant_its_content_type_selects() {
    use schematic_schema::fixture_shapes::{CreateMessageRequest, CreateMessageResponse, Delta, FixtureShapes, Message};
    let server = MockServer::start().await;
    let client = FixtureShapes::new().base_url(server.uri()).build().unwrap();

    reply(&server, "/messages", ResponseTemplate::new(200).set_body_raw(r#"{"id":"m1"}"#, "Application/JSON; charset=utf-8")).await;
    match client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap() {
        CreateMessageResponse::Json(message) => assert_eq!(message, Message { id: "m1".into() }),
        other => panic!("expected the JSON variant: {other:?}"),
    }

    let events = "data: {\"text\":\"a\"}\n\ndata: {\"text\":\"b\"}\n\ndata: [DONE]\n\n";
    reply(&server, "/messages", ResponseTemplate::new(200).set_body_raw(events, "text/event-stream")).await;
    let CreateMessageResponse::Stream(mut stream) = client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap() else {
        panic!("expected the stream variant");
    };
    let mut items = Vec::new();
    while let Some(item) = stream.next().await {
        items.push(item.unwrap());
    }
    assert_eq!(items, [Delta { text: "a".into() }, Delta { text: "b".into() }], "`[DONE]` is consumed");

    reply(&server, "/messages", ResponseTemplate::new(200).set_body_raw("<html/>", "text/html")).await;
    let response = client.send(CreateMessageRequest::default()).full().await.unwrap();
    let Outcome::Failure(error) = &response.outcome else { panic!("HTML selects no shape: {response}") };
    assert!(matches!(error.kind, ErrorKind::UnexpectedContentType), "{error}");
    assert_eq!(error.body, &b"<html/>"[..]);
}

#[tokio::test]
async fn single_stream_endpoints_deliver_items_and_bytes_as_they_are_framed() {
    use schematic_schema::fixture_shapes::{Chunk, DownloadRequest, FixtureShapes, StreamChunksRequest};
    let server = MockServer::start().await;
    let client = FixtureShapes::new().base_url(server.uri()).build().unwrap();

    reply(&server, "/chunks", ResponseTemplate::new(200).set_body_raw("{\"n\":1,\"done\":false}\n\n{\"n\":2,\"done\":true}\n", "application/x-ndjson")).await;
    let mut chunks = client.send(StreamChunksRequest::default()).full().await.unwrap().into_result().unwrap();
    let mut items = Vec::new();
    while let Some(item) = chunks.next().await {
        items.push(item.unwrap());
    }
    assert_eq!(items, [Chunk { n: 1, done: false }, Chunk { n: 2, done: true }], "the kept `done` line is the last item");

    reply(&server, "/chunks", ResponseTemplate::new(200).set_body_raw("{\"n\":1,\"done\":false}\n", "application/x-ndjson")).await;
    let mut early = client.send(StreamChunksRequest::default()).full().await.unwrap().into_result().unwrap();
    assert!(early.next().await.unwrap().is_ok());
    let ended = early.next().await.unwrap().unwrap_err();
    assert!(matches!(ended, schematic_schema::schematic_runtime::StreamError::EndedEarly), "{ended:?}");
    assert!(early.next().await.is_none());

    reply(&server, "/download", ResponseTemplate::new(200).set_body_raw(vec![1u8, 2, 3, 4], "audio/mpeg")).await;
    let mut bytes = client.send(DownloadRequest::default()).full().await.unwrap().into_result().unwrap();
    let mut received = Vec::new();
    while let Some(chunk) = bytes.next().await {
        received.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(received, [1, 2, 3, 4]);
}

#[tokio::test]
async fn named_fallback_and_status_shapes_select_their_variants() {
    use schematic_schema::fixture_shapes::{FixtureShapes, GetSourceRequest, GetSourceResponse, Job, Listing, Record, SubmitRequest, SubmitResponse};
    let server = MockServer::start().await;
    let client = FixtureShapes::new().base_url(server.uri()).build().unwrap();

    reply(&server, "/source/docs", ResponseTemplate::new(200).set_body_raw(r#"{"entries":["a.md"]}"#, "application/json")).await;
    match client.send(GetSourceRequest::new("docs")).full().await.unwrap().into_result().unwrap() {
        GetSourceResponse::Directory(listing) => assert_eq!(listing, Listing { entries: vec!["a.md".into()] }),
        other => panic!("expected Directory: {other:?}"),
    }
    reply(&server, "/source/logo.png", ResponseTemplate::new(200).set_body_raw(vec![0x89u8, b'P'], "image/png")).await;
    match client.send(GetSourceRequest::new("logo.png")).full().await.unwrap().into_result().unwrap() {
        GetSourceResponse::File(bytes) => assert_eq!(&bytes[..], [0x89, b'P']),
        other => panic!("an unmatched media type takes the fallback: {other:?}"),
    }

    reply(&server, "/jobs", ResponseTemplate::new(200).set_body_raw(r#"{"id":"r1"}"#, "application/json")).await;
    assert!(matches!(client.send(SubmitRequest::default()).full().await.unwrap().into_result().unwrap(), SubmitResponse::Record(Record { id }) if id == "r1"));
    reply(&server, "/jobs", ResponseTemplate::new(202).set_body_raw(r#"{"job":"j1"}"#, "application/json")).await;
    assert!(matches!(client.send(SubmitRequest::default()).full().await.unwrap().into_result().unwrap(), SubmitResponse::Job(Job { job }) if job == "j1"));

    let mutated = FixtureShapes::new()
        .base_url(server.uri())
        .mutate_response::<SubmitRequest, _>(|_meta, response| {
            if let SubmitResponse::Job(job) = response {
                job.job.push_str(" (seen)");
            }
            Ok(())
        })
        .build()
        .unwrap();
    assert!(matches!(mutated.send(SubmitRequest::default()).full().await.unwrap().into_result().unwrap(), SubmitResponse::Job(Job { job }) if job == "j1 (seen)"),
        "an all-JSON enum accepts a typed mutator");
}

#[tokio::test]
async fn declared_errors_decode_by_the_most_specific_status_and_opt_outs_stay_undeclared() {
    use schematic_schema::fixture_shapes::{
        Conflict, CreateMessageError, CreateMessageRequest, DownloadRequest, FixtureShapes, Problem, StreamChunksRequest, SubmitError, SubmitRequest,
    };
    let server = MockServer::start().await;
    let client = FixtureShapes::new().base_url(server.uri()).build().unwrap();

    reply(&server, "/messages", ResponseTemplate::new(404).set_body_raw("no such thread", "text/plain")).await;
    let error = client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(CreateMessageError::Text(text)) if text == "no such thread"), "{error}");
    reply(&server, "/messages", ResponseTemplate::new(503).set_body_raw(r#"{"message":"busy"}"#, "application/json")).await;
    let error = client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(CreateMessageError::Problem(Problem { message })) if message == "busy"), "API defaults are inherited: {error}");
    reply(&server, "/messages", ResponseTemplate::new(503).set_body_raw("<html/>", "text/html")).await;
    let error = client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::MalformedError), "{error}");
    assert_eq!(error.body, &b"<html/>"[..]);
    reply(&server, "/messages", ResponseTemplate::new(400).set_body_raw("bad", "text/plain")).await;
    let error = client.send(CreateMessageRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Undeclared), "no entry matches 400: {error}");

    reply(&server, "/jobs", ResponseTemplate::new(409).set_body_raw(r#"{"reason":"taken"}"#, "application/json")).await;
    let error = client.send(SubmitRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(SubmitError::Conflict(Conflict { reason })) if reason == "taken"), "{error}");

    reply(&server, "/chunks", ResponseTemplate::new(500).set_body_raw(r#"{"message":"down"}"#, "application/json")).await;
    let error = client.send(StreamChunksRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(Problem { message }) if message == "down"), "one distinct shape is the failure type itself: {error}");

    reply(&server, "/download", ResponseTemplate::new(500).set_body_raw(r#"{"message":"down"}"#, "application/json")).await;
    let error = client.send(DownloadRequest::default()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Undeclared), "an opted-out endpoint declares nothing: {error}");
}

// ---- GraphQL operations ----

#[tokio::test]
async fn graphql_operations_post_their_document_to_the_address_the_rules_derive() {
    use schematic_schema::fixture_graphql::{CheckData, CheckStatusRequest, CheckVars, FixtureGraphQl, GetUserRequest, Login, ViewerData, ViewerRequest};
    let server = MockServer::start().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"data":{"repository":null,"viewer":{"login":"me"}},"login":"me"}"#, "application/json"))
        .mount(&server)
        .await;
    let enterprise = FixtureGraphQl::new().base_url(format!("{}/api/v3/", server.uri())).build().unwrap();
    let variables = CheckVars { owner: "o".into(), name: "r".into(), n0: 7 };
    let data = enterprise.send(CheckStatusRequest::new(variables)).full().await.unwrap().into_result().unwrap();
    assert_eq!(data, CheckData { repository: None });
    let request = only_request(&server).await;
    assert_eq!(request.method.as_str(), "POST");
    assert_eq!(request.url.path(), "/api/graphql", "`/api/v3` is replaced");
    assert_eq!(header(&request, "accept"), ["application/json"]);
    assert_eq!(header(&request, "content-type"), ["application/json"]);
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body, serde_json::json!({
        "query": CheckStatusRequest::DOCUMENT,
        "variables": {"owner": "o", "name": "r", "n0": 7},
        "operationName": "CheckStatus",
    }));

    server.reset().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"data":{"viewer":{"login":"me"}},"login":"me"}"#, "application/json"))
        .mount(&server)
        .await;
    let dotcom = FixtureGraphQl::new().base_url(server.uri()).build().unwrap();
    let data = dotcom.send(ViewerRequest::new()).full().await.unwrap().into_result().unwrap();
    assert_eq!(data, ViewerData { viewer: Login { login: "me".into() } });
    let request = only_request(&server).await;
    assert_eq!(request.url.path(), "/graphql", "any other base gains `/graphql`");
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body, serde_json::json!({"query": "query { viewer { login } }"}), "no variables or operationName member");

    server.reset().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"login":"me"}"#, "application/json"))
        .mount(&server)
        .await;
    let user = enterprise.send(GetUserRequest::default()).full().await.unwrap().into_result().unwrap();
    assert_eq!(user, Login { login: "me".into() });
    assert_eq!(only_request(&server).await.url.path(), "/api/v3/user", "REST paths stay under the base");

    server.reset().await;
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"data":{"viewer":{"login":"me"}}}"#, "application/json"))
        .mount(&server)
        .await;
    let overridden = FixtureGraphQl::new().base_url(server.uri()).graphql_url(format!("{}/custom/gql", server.uri())).build().unwrap();
    overridden.send(ViewerRequest::new()).full().await.unwrap().into_result().unwrap();
    assert_eq!(only_request(&server).await.url.path(), "/custom/gql", "an override beats the rules");
}

#[tokio::test]
async fn graphql_errors_and_http_errors_reach_the_composite_failure() {
    use schematic_schema::fixture_graphql::{CheckStatusRequest, CheckVars, FixtureGraphQl, Problem, RenameRequest};
    use schematic_schema::schematic_runtime::GraphQlOperationError;
    let server = MockServer::start().await;
    let client = FixtureGraphQl::new().base_url(server.uri()).build().unwrap();
    let variables = || CheckVars { owner: "o".into(), name: "r".into(), n0: 1 };

    reply(&server, "/graphql", ResponseTemplate::new(200).set_body_raw(
        r#"{"data":null,"errors":[{"message":"limited","type":"RATE_LIMITED"}]}"#, "application/json")).await;
    let error = client.send(CheckStatusRequest::new(variables())).full().await.unwrap().into_result().unwrap_err();
    let ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) = &error.kind else { panic!("{error}") };
    assert_eq!(failure.errors[0].message, "limited");
    assert_eq!(failure.errors[0].other["type"], "RATE_LIMITED");
    assert!(failure.partial_data.is_none());

    reply(&server, "/graphql", ResponseTemplate::new(403).set_body_raw(r#"{"message":"forbidden"}"#, "application/json")).await;
    let error = client.send(CheckStatusRequest::new(variables())).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(GraphQlOperationError::Http(Problem { message })) if message == "forbidden"), "{error}");

    reply(&server, "/graphql", ResponseTemplate::new(403).set_body_raw(r#"{"message":"forbidden"}"#, "application/json")).await;
    let error = client.send(RenameRequest::new()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Undeclared), "an opted-out operation declares no HTTP error: {error}");
    reply(&server, "/graphql", ResponseTemplate::new(200).set_body_raw(r#"{"errors":[{"message":"denied"}]}"#, "application/json")).await;
    let error = client.send(RenameRequest::new()).full().await.unwrap().into_result().unwrap_err();
    assert!(matches!(&error.kind, ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) if failure.errors[0].message == "denied"),
        "GraphQL errors stay declared after an HTTP opt-out: {error}");
}

fn relay_page(numbers: &[u64], end: serde_json::Value, more: bool) -> serde_json::Value {
    let nodes: Vec<serde_json::Value> = numbers.iter().map(|number| serde_json::json!({"number": number})).collect();
    serde_json::json!({"data": {"repository": {"issues": {"nodes": nodes, "pageInfo": {"endCursor": end, "hasNextPage": more}}}}})
}

async fn relay_server(pages: impl Fn(Option<&str>) -> serde_json::Value + Send + Sync + 'static) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/graphql"))
        .respond_with(move |request: &Request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            ResponseTemplate::new(200).set_body_json(pages(body["variables"]["after"].as_str()))
        })
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn a_generated_relay_pager_pages_data_and_stops_at_graphql_errors() {
    use schematic_schema::fixture_graphql::{FixtureGraphQl, Issue, IssuesRequest, IssuesVars};
    use schematic_schema::schematic_runtime::GraphQlOperationError;
    use schematic_schema::schematic_runtime::pager::{IncompleteReason, PagerError};
    let variables = || IssuesVars { owner: "acme".into(), first: None, after: None };

    let server = relay_server(|after| match after {
        None => relay_page(&[1, 2], serde_json::json!("c1"), true),
        Some("c1") => relay_page(&[3], serde_json::Value::Null, false),
        other => panic!("unexpected cursor {other:?}"),
    }).await;
    let client = FixtureGraphQl::new().base_url(server.uri()).build().unwrap();
    let items = client.send(IssuesRequest::new(variables())).pager().page_size(2).collect_all().await.unwrap();
    assert_eq!(items, [Issue { number: 1 }, Issue { number: 2 }, Issue { number: 3 }]);
    let sent: Vec<serde_json::Value> = server.received_requests().await.unwrap().iter()
        .map(|request| serde_json::from_slice::<serde_json::Value>(&request.body).unwrap()).collect();
    assert_eq!(sent.iter().map(|body| body["variables"].clone()).collect::<Vec<_>>(), [
        serde_json::json!({"owner": "acme", "first": 2, "after": null}),
        serde_json::json!({"owner": "acme", "first": 2, "after": "c1"}),
    ], "a null initial cursor is valid; the pager writes its inputs over the serialized variables");
    assert!(sent.iter().all(|body| body["query"] == IssuesRequest::DOCUMENT));
    assert!(sent.iter().all(|body| body.get("operationName").is_none()));

    let error = client.send(IssuesRequest::new(variables())).pager().page_size(51).fetch_page().await.unwrap_err();
    assert!(matches!(error, PagerError::Transport(_)), "the authored maximum refuses 51 before sending: {error:?}");

    let failing = relay_server(|after| match after {
        None => relay_page(&[1], serde_json::json!("c1"), true),
        _ => {
            let mut page = relay_page(&[2], serde_json::json!("c2"), true);
            page["errors"] = serde_json::json!([{"message": "rate limited", "type": "RATE_LIMITED"}]);
            page
        }
    }).await;
    let client = FixtureGraphQl::new().base_url(failing.uri()).build().unwrap();
    let incomplete = client.send(IssuesRequest::new(variables())).pager().collect_all().await.unwrap_err();
    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(incomplete.items, [Issue { number: 1 }], "partial data never joins the collected items");
    let Some(PagerError::Api { error, .. }) = &incomplete.error else { panic!("{incomplete:?}") };
    let ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) = &error.kind else { panic!("{error:?}") };
    assert_eq!(failure.errors[0].other["type"], "RATE_LIMITED");
}

/// R43: every sniff check-status fixture through the generated operation,
/// with the HTTP status and JSON content type the fixture table assigns.
#[tokio::test]
async fn check_status_parity_preserves_every_sniff_fixture() {
    use schematic_schema::fixture_graphql::{CheckStatusParityRequest, FixtureGraphQl, ParityData, ParityVars};
    use schematic_schema::schematic_runtime::{GraphQlFailure, GraphQlOperationError};
    let fixture = |name: &str| std::env::var(format!("CHECK_STATUS_{}", name.to_uppercase())).expect("the parent passes every fixture");
    let server = MockServer::start().await;
    let client = FixtureGraphQl::new().base_url(server.uri()).build().unwrap();
    let request = || CheckStatusParityRequest::new(ParityVars { owner: "acme".into(), name: "project".into(), n0: 7, n1: 8, n2: 9 });
    let succeed = |name: &'static str| {
        let (client, server) = (&client, &server);
        async move {
            reply(server, "/graphql", ResponseTemplate::new(200).set_body_raw(fixture(name), "application/json")).await;
            let response = client.send(request()).full().await.expect("a response");
            response.into_result().unwrap_or_else(|error| panic!("{name}: {error}"))
        }
    };
    let fail = |name: &'static str, status: u16| {
        let (client, server) = (&client, &server);
        async move {
            reply(server, "/graphql", ResponseTemplate::new(status).set_body_raw(fixture(name), "application/json")).await;
            client.send(request()).full().await.expect("a response").into_result().expect_err(name)
        }
    };
    let graphql = |error: &schematic_schema::schematic_runtime::ApiError<GraphQlOperationError<ParityData, schematic_schema::schematic_runtime::NoDeclaredError>>| -> GraphQlFailure<ParityData> {
        match &error.kind {
            ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) => failure.clone(),
            other => panic!("expected a GraphQL failure, got {other:?}"),
        }
    };

    let success = succeed("success").await;
    let body: serde_json::Value = serde_json::from_slice(&only_request(&server).await.body).unwrap();
    assert_eq!(body, serde_json::json!({"query": CheckStatusParityRequest::DOCUMENT,
        "variables": {"owner": "acme", "name": "project", "n0": 7, "n1": 8, "n2": 9}}), "sniff's document and variables, unnamed");
    let repository = success.repository.expect("success has a repository");
    let failing = repository.p1.expect("p1").commits.nodes[0].commit.status_check_rollup.clone().expect("rollup");
    assert!(failing.contexts.check_run_counts_by_state.iter().any(|count| count.state == "FAILURE"), "{failing:?}");
    assert!(repository.p2.is_some());

    let passed = succeed("passed").await.repository.expect("passed has a repository");
    assert!(passed.p1.is_none() && passed.p2.is_none(), "absent lookups decode as None");
    let changed = succeed("changed_head").await.repository.unwrap().p0.unwrap();
    assert_eq!(changed.commits.nodes[0].commit.oid, "fedcba9876543210fedcba9876543210fedcba98", "a changed head is domain data");
    let null_rollup = succeed("null_rollup").await.repository.unwrap().p0.unwrap();
    assert_eq!(null_rollup.commits.nodes[0].commit.status_check_rollup, None, "a null rollup is domain data");

    let partial = fail("partial_errors", 200).await;
    let failure = graphql(&partial);
    assert_eq!(failure.errors[0].other["type"], "NOT_FOUND");
    let partial_data = failure.partial_data.expect("usable partial data").repository.expect("repository");
    assert!(partial_data.p0.is_some() && partial_data.p1.is_none());
    assert_eq!(partial.body.as_ref(), fixture("partial_errors").as_bytes(), "the raw body is kept");

    for name in ["request_wide_errors", "rate_limit"] {
        let error = fail(name, 200).await;
        let failure = graphql(&error);
        let expected: serde_json::Value = serde_json::from_str(&fixture(name)).unwrap();
        assert_eq!(failure.errors[0].message, expected["errors"][0]["message"], "{name}");
        assert_eq!(failure.errors[0].other["type"], expected["errors"][0]["type"], "{name}: GitHub's type is kept");
        assert_eq!(error.body.as_ref(), fixture(name).as_bytes(), "{name}");
    }

    let permission = fail("permission_failure", 403).await;
    assert!(matches!(permission.kind, ErrorKind::Undeclared), "a 403 takes the HTTP path, not the wrapper: {permission}");
    assert_eq!(permission.body.as_ref(), fixture("permission_failure").as_bytes(), "the raw permission body is kept");
}

#[tokio::test]
async fn graphql_sends_report_their_credential_and_keep_unparsed_documents_and_transport_failures_apart() {
    use schematic_schema::fixture_graphql::{DraftRequest, FixtureGraphQl, ViewerRequest};
    use schematic_schema::schematic_runtime::Credential;
    let server = MockServer::start().await;
    reply(&server, "/graphql", ResponseTemplate::new(200).set_body_raw(r#"{"errors":[{"message":"Syntax Error","locations":[{"line":1,"column":30}]}]}"#, "application/json")).await;
    let client = FixtureGraphQl::new().base_url(server.uri()).env_order(["SCHEMATIC_FIXTURE_UNSET_KEY_7F3A"]).build().unwrap();

    let response = client.send(DraftRequest::new()).full().await.expect("a GraphQL failure is a received response");
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous, "no credential resolves, so the operation is sent anonymously");
    let request = only_request(&server).await;
    assert!(header(&request, "authorization").is_empty());
    let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
    assert_eq!(body, serde_json::json!({"query": "mutation { draft(title: \"unterminated"}), "an invalid document is sent unchanged");
    let error = response.into_result().expect_err("into_result surfaces the GraphQL failure");
    assert!(error.declared().is_some(), "{error}");

    reply(&server, "/graphql", ResponseTemplate::new(200).set_body_raw(r#"{"data":{"viewer":{"login":"me"}}}"#, "application/json")).await;
    let response = client.send(ViewerRequest::new()).credential(Credential::access_token("tok")).full().await.unwrap();
    assert_eq!(response.meta.request.auth, AuthMethod::PerCall);
    assert_eq!(header(&only_request(&server).await, "authorization"), ["Bearer tok"]);

    let unreachable = FixtureGraphQl::new().base_url("http://127.0.0.1:1").build().unwrap();
    let outer = unreachable.send(ViewerRequest::new()).full().await;
    assert!(outer.is_err(), "the outer result carries transport failures only");
}

// ---- value rules ----

fn assert_refused(error: &TransportError, expected: &str) {
    let TransportErrorKind::InvalidArgument(argument) = &error.kind else {
        panic!("expected an invalid argument, got {error:?}");
    };
    assert_eq!(argument.reason(), expected);
}

fn valid_item() -> NewItem {
    NewItem {
        display_name: "Lamp".to_string(),
        internal: "not serialized, so not checked".to_string(),
        extra: Extra { note: "ok".to_string() },
        price: 2.5,
        lines: vec![Line { qty: 1 }],
        labels: [("room".to_string(), "den".to_string())].into(),
        parent: None,
    }
}

#[tokio::test]
async fn a_violating_request_is_refused_on_every_send_path_and_nothing_is_sent() {
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let expected = "2 constraint violations: \
        query.limit violates maximum: must be at most 100; \
        query.name violates maxLength: must be at most 5 characters long";
    let request = || ListItemsRequest::default().with_limit(500).with_name("abcdef".to_string());

    let violations = request().validate().unwrap_err();
    let found: Vec<(String, &str)> = violations.iter().map(|v| (v.path_text(), v.keyword().as_str())).collect();
    assert_eq!(found, [("query.limit".to_string(), "maximum"), ("query.name".to_string(), "maxLength")]);

    let built = client.list_items().limit(500).name("abcdef".to_string()).build().unwrap_err();
    assert_eq!(built.reason(), expected);
    match client.list_items().limit(500).name("abcdef".to_string()).await {
        Err(CallError::Transport(error)) => assert_refused(&error, expected),
        other => panic!("an awaited call wraps the refusal as a transport error: {other:?}"),
    }
    assert_refused(&client.list_items().limit(500).name("abcdef".to_string()).full().await.unwrap_err(), expected);
    match client.send(request()).await {
        Err(CallError::Transport(error)) => assert_refused(&error, expected),
        other => panic!("an awaited send wraps the refusal as a transport error: {other:?}"),
    }
    assert_refused(&client.send(request()).full().await.unwrap_err(), expected);
    assert_refused(&client.send_raw(request(), &CallOptions::new()).await.unwrap_err(), expected);

    // Each item of an array parameter, its count, and its uniqueness.
    let ids = ListItemsRequest::default().with_ids(vec![0, 2, 2, 4]);
    let found: Vec<(String, &str)> =
        ids.validate().unwrap_err().iter().map(|v| (v.path_text(), v.keyword().as_str())).collect();
    assert_eq!(found, [
        ("query.ids".to_string(), "maxItems"),
        ("query.ids".to_string(), "uniqueItems"),
        ("query.ids[0]".to_string(), "minimum"),
    ]);
    assert!(client.send(ids).full().await.is_err());
    let tag = ListItemsRequest::default().with_x_tag("Not lower".to_string());
    assert_eq!(tag.validate().unwrap_err().to_string(), "header.x_tag violates pattern: must match the declared pattern");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn valid_values_pass_their_checks_and_are_sent_unchanged() {
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    // Five code points, eleven UTF-8 bytes: length counts code points.
    let request = ListItemsRequest::default()
        .with_limit(100)
        .with_name("日本語ab".to_string())
        .with_ids(vec![1, 2, 3])
        .with_x_tag("abc".to_string());
    request.validate().unwrap();
    client.send(request).full().await.unwrap();
    let received = only_request(&server).await;
    assert_eq!(query(&received), "limit=100&name=%E6%97%A5%E6%9C%AC%E8%AA%9Eab&ids=1,2,3");
    assert_eq!(header(&received, "x-tag"), ["abc"]);

    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let body = valid_item();
    CreateItemRequest::new(body.clone()).validate().unwrap();
    client.send(CreateItemRequest::new(body.clone())).full().await.unwrap();
    let received = only_request(&server).await;
    let sent: serde_json::Value = serde_json::from_slice(&received.body).unwrap();
    assert_eq!(sent, serde_json::to_value(&body).unwrap(), "the checked value is the one sent");
    assert_eq!(sent["displayName"], "Lamp");
    assert_eq!(sent["note"], "ok", "a flattened field is sent at the top level");
    assert!(sent.get("internal").is_none(), "a skipped field is neither sent nor checked");
}

#[tokio::test]
async fn a_body_is_checked_as_it_serializes_including_renamed_flattened_and_recursive_fields() {
    let mut body = valid_item();
    body.display_name = "x".repeat(11);
    body.internal = "y".repeat(50);
    body.extra.note = "too long".to_string();
    body.price = 2.25;
    body.lines = vec![Line { qty: 3 }, Line { qty: 0 }];
    body.parent = Some(Box::new(NewItem { display_name: "z".repeat(11), ..NewItem::default() }));
    let violations = CreateItemRequest::new(body.clone()).validate().unwrap_err();
    let found: Vec<(String, &str)> = violations.iter().map(|v| (v.path_text(), v.keyword().as_str())).collect();
    assert_eq!(found, [
        ("body.displayName".to_string(), "maxLength"),
        ("body.lines[1].qty".to_string(), "minimum"),
        ("body.note".to_string(), "maxLength"),
        ("body.parent.displayName".to_string(), "maxLength"),
        ("body.price".to_string(), "multipleOf"),
    ]);

    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let error = client.send(CreateItemRequest::new(body)).full().await.unwrap_err();
    assert_refused(&error, &violations.to_string());
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn a_field_changed_after_validate_is_caught_by_the_send() {
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let mut request = ListItemsRequest::default().with_limit(10);
    request.validate().unwrap();
    request.limit = Some(0);
    let error = client.send(request).full().await.unwrap_err();
    assert_refused(&error, "query.limit violates minimum: must be at least 1");

    let mut create = CreateItemRequest::new(valid_item());
    create.validate().unwrap();
    create.body.lines.push(Line { qty: -1 });
    let error = client.send(create).full().await.unwrap_err();
    assert_refused(&error, "body.lines[1].qty violates minimum: must be at least 1");
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn text_bodies_and_form_fields_are_checked_before_they_are_encoded() {
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let error = client.send(SendTextRequest::new("abcd".to_string())).full().await.unwrap_err();
    assert_refused(&error, "body violates maxLength: must be at most 3 characters long");
    let form = UploadForm { purpose: "c".to_string(), meta: Meta { title: String::new() } };
    let error = client.send(UploadRequest::new(form)).full().await.unwrap_err();
    assert_refused(&error, "2 constraint violations: \
        body.purpose violates enum: must be one of the 2 allowed values; \
        body.meta.title violates minLength: must be at least 1 character long");
    assert_nothing_sent(&server).await;

    client.send(SendTextRequest::new("abc".to_string())).full().await.unwrap();
    assert_eq!(only_request(&server).await.body, b"abc");

    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let form = UploadForm { purpose: "a".to_string(), meta: Meta { title: "t".to_string() } };
    UploadRequest::new(form.clone()).validate().unwrap();
    client.send(UploadRequest::new(form)).full().await.unwrap();
    let body = String::from_utf8(only_request(&server).await.body).unwrap();
    assert!(body.contains("name=\"purpose\""), "{body}");
    assert!(body.contains("name=\"meta\"") && body.contains(r#"{"title":"t"}"#), "the checked JSON part is sent: {body}");
}

#[tokio::test]
async fn reports_of_secret_values_and_map_keys_never_contain_them() {
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let mut body = valid_item();
    body.labels = [("api-token-SECRETKEY".to_string(), "s3cr3t-value".to_string())].into();
    let request = CreateItemRequest::new(body);
    let violations = request.clone().validate().unwrap_err();
    assert_eq!(violations.to_string(), "body.labels[[redacted]] violates maxLength: must be at most 8 characters long");
    let error = client.send(request).full().await.unwrap_err();
    let named = ListItemsRequest::default().with_name("SECRETNAME".to_string());
    let named_violations = named.validate().unwrap_err();
    let built = client.list_items().name("SECRETNAME".to_string()).build().unwrap_err();
    for rendered in [
        violations.to_string(),
        format!("{violations:?}"),
        error.to_string(),
        format!("{error:?}"),
        named_violations.to_string(),
        format!("{named_violations:?}"),
        built.to_string(),
        format!("{built:?}"),
    ] {
        for secret in ["SECRET", "s3cr3t"] {
            assert!(!rendered.contains(secret), "{secret} leaked: {rendered}");
        }
    }
    assert_nothing_sent(&server).await;
}

#[tokio::test]
async fn a_declared_property_is_never_checked_by_the_extra_value_plan() {
    let mixed = |extra: &[(&str, &str)]| Mixed {
        label: "long declared label".to_string(),
        extra: extra.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
    };
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    CreateMixedRequest::new(mixed(&[("ok", "abc")])).validate().unwrap();
    CreateHolderRequest::new(Holder { inner: mixed(&[]) }).validate().unwrap();
    UploadMixedRequest::new(UploadMixedForm { mixed: mixed(&[]) }).validate().unwrap();
    client.send(CreateMixedRequest::new(mixed(&[]))).full().await.unwrap();
    let sent: serde_json::Value = serde_json::from_slice(&only_request(&server).await.body).unwrap();
    assert_eq!(sent, serde_json::json!({"label": "long declared label"}));

    // A genuinely extra entry still takes the rule, under a redacted key.
    let server = start_server().await;
    let client = FixtureRules::new().base_url(server.uri()).build().unwrap();
    let extra = [("SECRETKEY", "too long")];
    for (request, expected) in [
        (CreateMixedRequest::new(mixed(&extra)).validate(), "body[[redacted]]"),
        (CreateHolderRequest::new(Holder { inner: mixed(&extra) }).validate(), "body.inner[[redacted]]"),
        (UploadMixedRequest::new(UploadMixedForm { mixed: mixed(&extra) }).validate(), "body.mixed[[redacted]]"),
    ] {
        let found: Vec<(String, &str)> =
            request.unwrap_err().iter().map(|v| (v.path_text(), v.keyword().as_str())).collect();
        assert_eq!(found, [(expected.to_string(), "maxLength")]);
    }
    assert!(client.send(CreateMixedRequest::new(mixed(&extra))).full().await.is_err());
    assert_nothing_sent(&server).await;
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
    "every_page_carries_the_required_value_beside_the_changing_adopted_ones",
    "query_values_reach_the_wire_in_their_declared_form",
    "unset_values_are_omitted_and_empty_arrays_are_sent_empty",
    "a_generated_session_opens_pages_and_closes_inside_its_scope",
    "typed_registration_paths_preserve_nullable_nested_wire_shape",
    "a_multi_shape_endpoint_returns_the_variant_its_content_type_selects",
    "single_stream_endpoints_deliver_items_and_bytes_as_they_are_framed",
    "named_fallback_and_status_shapes_select_their_variants",
    "declared_errors_decode_by_the_most_specific_status_and_opt_outs_stay_undeclared",
    "graphql_operations_post_their_document_to_the_address_the_rules_derive",
    "graphql_errors_and_http_errors_reach_the_composite_failure",
    "a_generated_relay_pager_pages_data_and_stops_at_graphql_errors",
    "check_status_parity_preserves_every_sniff_fixture",
    "graphql_sends_report_their_credential_and_keep_unparsed_documents_and_transport_failures_apart",
    "a_violating_request_is_refused_on_every_send_path_and_nothing_is_sent",
    "valid_values_pass_their_checks_and_are_sent_unchanged",
    "a_body_is_checked_as_it_serializes_including_renamed_flattened_and_recursive_fields",
    "a_field_changed_after_validate_is_caught_by_the_send",
    "text_bodies_and_form_fields_are_checked_before_they_are_encoded",
    "reports_of_secret_values_and_map_keys_never_contain_them",
    "a_declared_property_is_never_checked_by_the_extra_value_plan",
];

/// Renders a `format!` template whose arguments are all named in `args`:
/// `{{` and `}}` are literal braces, `{name}` is replaced.
fn render_format(template: &str, args: &[(&str, String)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(index) = rest.find(['{', '}']) {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        if rest.starts_with("{{") || rest.starts_with("}}") {
            out.push_str(&rest[..1]);
            rest = &rest[2..];
        } else {
            let end = rest.find('}').expect("a closed argument");
            let name = &rest[1..end];
            out.push_str(&args.iter().find(|(arg, _)| *arg == name).unwrap_or_else(|| panic!("no argument {name}")).1);
            rest = &rest[end + 1..];
        }
    }
    out + rest
}

/// R43 uses sniff's own query: re-derive `batch_query(3)` from the source
/// of `sniff/lib/src/remote/check_status.rs` and compare it with the
/// fixture's document, so a change to sniff's query fails here.
#[test]
fn the_parity_document_is_sniffs_check_status_query() {
    let source = include_str!("../../../../sniff/lib/src/remote/check_status.rs").replace("\r\n", "\n");
    let function = source.split("pub(crate) fn batch_query(count: usize) -> String {").nth(1).expect("sniff declares batch_query")
        .split("\n}\n").next().unwrap();
    let variable = r#"format!(",$n{index}:Int!")"#;
    let lookup = r#"format!("p{index}:pullRequest(number:$n{index}){{...F}}")"#;
    assert!(function.contains(variable) && function.contains(lookup), "sniff's per-lookup formats changed:\n{function}");
    // The query template: a string literal whose `\` line continuations
    // drop the newline and the next line's indentation.
    let literal = function.split("format!(\n        \"").nth(1).expect("the query format").split("\"\n    )").next().unwrap();
    let template: String = literal.split("\\\n").map(str::trim_start).collect();
    let count = 3;
    let args = |index: usize| [("index", index.to_string())];
    let variables: String = (0..count).map(|index| render_format(&variable[9..variable.len() - 2], &args(index))).collect();
    let lookups: String = (0..count).map(|index| render_format(&lookup[9..lookup.len() - 2], &args(index))).collect();
    let document = render_format(&template, &[("variables", variables), ("lookups", lookups)]);
    assert_eq!(document, CHECK_STATUS_DOCUMENT);
}

#[test]
fn request_side_test_list_names_every_child_test() {
    let declared: Vec<&str> = REQUEST_SIDE_TEST
        .split("async fn ")
        .skip(1)
        .filter_map(|rest| rest.split('(').next())
        .filter(|name| !["start_server", "only_request", "assert_nothing_sent", "reply", "relay_server"].contains(name))
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
    // R43: sniff's frozen check-status fixtures, read here so the CI input
    // index sees each file, and handed to the child test unchanged.
    env.extend([
        ("CHECK_STATUS_SUCCESS", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/success.json")),
        ("CHECK_STATUS_PASSED", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/passed.json")),
        ("CHECK_STATUS_CHANGED_HEAD", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/changed_head.json")),
        ("CHECK_STATUS_NULL_ROLLUP", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/null_rollup.json")),
        ("CHECK_STATUS_PARTIAL_ERRORS", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/partial_errors.json")),
        ("CHECK_STATUS_REQUEST_WIDE_ERRORS", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/request_wide_errors.json")),
        ("CHECK_STATUS_RATE_LIMIT", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/rate_limit.json")),
        ("CHECK_STATUS_PERMISSION_FAILURE", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/permission_failure.json")),
    ]);
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
    // Ollama's module group is large enough to be written as a directory.
    let ollama = fresh.src_dir.join("ollama/mod.rs");
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
        registrations: schematic_definitions::registry::registrations(),
        selection: Selection::Apis(&ollama_native),
        output_dir: &fresh.src_dir,
        manifest_dir: None,
    })
    .expect("prepare the partial run");
    assert_eq!(prepared.regenerated, ["ollama"]);
    let groups: std::collections::BTreeSet<String> =
        api_refs.iter().map(|api| schematic_definitions::naming::resolve_module_name(api)).collect();
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
    ("ErrorSet has no status shorthand", "fn misuse() { let _ = schematic_define::ErrorSet::status(400); }", "E0599", "ErrorSet"),
    (
        "GraphQL variables of another type",
        "fn misuse() { let _ = schematic_schema::fixture_graphql::CheckStatusRequest::new(schematic_schema::fixture_graphql::Login::default()); }",
        "E0308",
        "CheckVars",
    ),
    (
        "an unauthored pager knob on a GraphQL operation",
        "fn misuse() { let c = schematic_schema::fixture_graphql::FixtureGraphQl::new(); let _ = c.send(schematic_schema::fixture_graphql::IssuesRequest::new(Default::default())).pager().after(String::new()); }",
        "E0277",
        "IssuesRequest",
    ),
    (
        "a mutator on an SSE/NDJSON stream endpoint",
        "fn misuse() { let _ = schematic_schema::fixture_shapes::FixtureShapes::new().mutate_response::<schematic_schema::fixture_shapes::StreamChunksRequest, _>(|_, _| Ok(())); }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a mutator on a byte stream endpoint",
        "fn misuse() { let _ = schematic_schema::fixture_shapes::FixtureShapes::new().mutate_response::<schematic_schema::fixture_shapes::DownloadRequest, _>(|_, _| Ok(())); }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a mutator on a text endpoint",
        "fn misuse() { let _ = FixtureBasic::new().mutate_response::<GetNoteRequest, _>(|_, _| Ok(())); }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a mutator on a binary endpoint",
        "fn misuse() { let _ = FixtureBasic::new().mutate_response::<schematic_schema::fixture_basic::GetBlobRequest, _>(|_, _| Ok(())); }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a mutator on JSON mixed with a stream",
        "fn misuse() { let _ = schematic_schema::fixture_shapes::FixtureShapes::new().mutate_response::<schematic_schema::fixture_shapes::CreateMessageRequest, _>(|_, _| Ok(())); }",
        "E0277",
        "SupportsHooks",
    ),
    (
        "a descriptor cannot be constructed from a bare name",
        "fn misuse() { let _ = schematic_define::TypeDesc::new(\"Model\"); }",
        "E0624",
        "private",
    ),
    (
        "a descriptor's structure is set only by context composition",
        "fn misuse() { let mut c = schematic_define::DefinitionContext::types_in(\"::models\"); let mut d = c.register::<String>().unwrap(); d.structure = Some(schematic_define::TypeRef::Named(\"Model\".into())); }",
        "E0616",
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
    (
        "a request with a required query value has no Default",
        "fn misuse() {
            let _ = schematic_schema::fixture_query::FindRequest::default();
        }",
        "E0599",
        "default",
    ),
    (
        "a request with a required query value used where Default is required",
        "fn needs_default<T: Default>() {}
        fn misuse() {
            needs_default::<schematic_schema::fixture_query::FindRequest>();
        }",
        "E0277",
        "Default",
    ),
    (
        "new() without every required query value",
        "fn misuse() {
            let _ = schematic_schema::fixture_query::FindRequest::new(\"q\");
        }",
        "E0061",
        "2 arguments",
    ),
    (
        "an endpoint method without every required query value",
        "fn misuse(client: schematic_schema::fixture_query::FixtureQuery) {
            let _ = client.find(\"q\");
        }",
        "E0061",
        "2 arguments",
    ),
    (
        "a string where a query enum is required",
        "fn misuse() {
            let _ = schematic_schema::fixture_query::FindRequest::new(\"q\", \"plain\");
        }",
        "E0308",
        "FindRequestKind",
    ),
    (
        "a required query value has no call-type setter",
        "fn misuse(client: schematic_schema::fixture_query::FixtureQuery) {
            let _ = client.find(\"q\", schematic_schema::fixture_query::FindRequestKind::Plain).kind(
                schematic_schema::fixture_query::FindRequestKind::ExtRa,
            );
        }",
        "E0599",
        "kind",
    ),
    (
        "a query value the client cannot serialize has no setter",
        "fn misuse() {
            let _ = schematic_schema::fixture_query::FindRequest::new(\"q\", schematic_schema::fixture_query::FindRequestKind::Plain)
                .with_meta(Default::default());
        }",
        "E0599",
        "with_meta",
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
        // An all-JSON success enum and a GraphQL operation's data accept a
        // typed mutator.
        let _ = schematic_schema::fixture_shapes::FixtureShapes::new()
            .mutate_response::<schematic_schema::fixture_shapes::SubmitRequest, _>(|_, _: &mut schematic_schema::fixture_shapes::SubmitResponse| Ok(()));
        let _ = schematic_schema::fixture_graphql::FixtureGraphQl::new()
            .mutate_response::<schematic_schema::fixture_graphql::ViewerRequest, _>(|_, _: &mut schematic_schema::fixture_graphql::ViewerData| Ok(()));
        // A Relay operation's authored knob compiles.
        let graphql = schematic_schema::fixture_graphql::FixtureGraphQl::new();
        let _ = graphql.send(schematic_schema::fixture_graphql::IssuesRequest::new(Default::default())).pager().page_size(10);
        // An all-optional request keeps `Default`; a required query value
        // goes through `new()`.
        fn needs_default<T: Default>() {}
        needs_default::<schematic_schema::fixture_query::BrowseRequest>();
        let _ = schematic_schema::fixture_query::FindRequest::new(
            \"q\",
            schematic_schema::fixture_query::FindRequestKind::Plain,
        );
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

use schematic_schema::gitea::{GetGitTreeRequest, Gitea, ListPullRequestsRequest, ListPullRequestsRequestSort, ListPullRequestsRequestState};
use schematic_schema::gitlab::{GetRepositoryFileRequest, GitLab};
use schematic_schema::openai::{CreateResponseRequest, GetResponseRequest, DeleteResponseRequest, CancelResponseRequest, CompactconversationRequest, GetinputtokencountsRequest, ListInputItemsRequest, ListConversationItemsRequest, ListConversationItemsRequestOrder, OpenAI, UsageCostsRequest, UsageCostsRequestBucketWidth, UsageCostsRequestGroupBy};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{EndpointSpec, Method, QueryValue, RequestBody};

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
    // Enum values are sent by their wire spelling, in declaration order.
    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.state = Some(ListPullRequestsRequestState::Closed);
    scalars.sort = Some(ListPullRequestsRequestSort::Recentupdate);
    let _ = gitea.send(scalars).credential(key.clone()).pager().page(2).page_size(1).fetch_page().await;
    record("ordered-enum-scalars");

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
            GetRepositoryFileRequest::new("group/project", "dir/file name.rs", "feature/x y+z"),
        )
        .credential(key.clone())
        .full()
        .await;
    record("legacy-embedded-query-with-placeholder");
    for (label, git_ref) in [("file-ref-unicode", "ünï+%&=/?#"), ("file-ref-empty", "")] {
        let request = GetRepositoryFileRequest::new("group/project", "dir/file name.rs", git_ref);
        let parts = request.clone().into_parts().unwrap();
        assert_eq!(parts.path, "/projects/group%2Fproject/repository/files/dir%2Ffile%20name.rs");
        assert_eq!(parts.query, pairs(&[("ref", git_ref)]));
        let _ = gitlab.send(request).credential(key.clone()).full().await;
        record(label);
    }

    let openai = OpenAI::new().base_url(format!("{base}/v1")).build().unwrap();
    let token = Credential::access_token("test-token");
    let mut items = ListConversationItemsRequest::new("conv 1");
    items.order = Some(ListConversationItemsRequestOrder::Desc);
    let _ = openai.send(items).credential(token.clone()).pager().page_size(5).after("x%y").fetch_page().await;
    record("declared-enum-then-pager-cursor");

    // A required value, exploded string and enum arrays, and an enum scalar.
    let usage = UsageCostsRequest::new(1_700_000_000)
        .with_bucket_width(UsageCostsRequestBucketWidth::V1d)
        .with_project_ids(vec!["a b".to_string(), "c+d".to_string(), String::new(), "é".to_string()])
        .with_group_by(vec![UsageCostsRequestGroupBy::ProjectId, UsageCostsRequestGroupBy::LineItem]);
    let _ = openai.send(usage).credential(token.clone()).full().await;
    record("repeated-array-keys");

    let empty = UsageCostsRequest::new(0).with_project_ids(Vec::new());
    let _ = openai.send(empty).credential(token.clone()).full().await;
    record("empty-array");

    let mut beta = ListInputItemsRequest::new("resp 1");
    beta.beta = Some(true);
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

fn pairs(list: &[(&str, &str)]) -> Vec<(String, QueryValue)> {
    list.iter().map(|(k, v)| (k.to_string(), QueryValue::from(*v))).collect()
}

#[test]
fn into_parts_returns_unencoded_query_pairs_apart_from_the_path() {
    let mut scalars = ListPullRequestsRequest::new("own er", "re+po%");
    scalars.state = Some(ListPullRequestsRequestState::Closed);
    scalars.sort = Some(ListPullRequestsRequestSort::Recentupdate);
    let parts = scalars.into_parts().unwrap();
    assert_eq!(parts.method, Method::GET);
    assert_eq!(parts.path, "/repos/own%20er/re%2Bpo%25/pulls");
    assert_eq!(parts.body, RequestBody::Empty);
    assert_eq!(parts.query, pairs(&[("state", "closed"), ("sort", "recentupdate")]));
    assert!(
        !parts.headers.iter().any(|(name, _)| ["state", "sort"].contains(&name.as_str())),
        "query parameters must not travel as headers: {:?}",
        parts.headers
    );

    let empty = UsageCostsRequest::new(0).with_project_ids(Vec::new());
    assert_eq!(empty.into_parts().unwrap().query, pairs(&[("start_time", "0"), ("project_ids", "")]));

    let parts = ListPullRequestsRequest::new("o", "r").into_parts().unwrap();
    assert_eq!((parts.path.as_str(), parts.query), ("/repos/o/r/pulls", vec![]));

    let mut tree = GetGitTreeRequest::new("o", "r", "a b");
    tree.recursive = Some(true);
    let parts = tree.into_parts().unwrap();
    assert_eq!((parts.path.as_str(), parts.query), ("/repos/o/r/git/trees/a%20b", pairs(&[("recursive", "true")])));
    let parts = GetRepositoryFileRequest::new("group/project", "dir/file name.rs", "feature/x y+z")
        .into_parts()
        .unwrap();
    assert_eq!(
        (parts.path.as_str(), parts.query),
        ("/projects/group%2Fproject/repository/files/dir%2Ffile%20name.rs", pairs(&[("ref", "feature/x y+z")]))
    );

    let usage = UsageCostsRequest::new(5)
        .with_project_ids(vec!["a b".to_string(), String::new()])
        .with_group_by(vec![UsageCostsRequestGroupBy::ApiKeyId]);
    let parts = usage.into_parts().unwrap();
    assert_eq!(parts.path, "/organization/costs");
    assert_eq!(
        parts.query,
        pairs(&[("start_time", "5"), ("project_ids", "a b"), ("project_ids", ""), ("group_by", "api_key_id")])
    );

    let mut beta = ListInputItemsRequest::new("resp 1");
    beta.beta = Some(true);
    let parts = beta.into_parts().unwrap();
    assert_eq!(parts.path, "/responses/resp%201/input_items");
    assert_eq!(parts.query, pairs(&[("beta", "true")]));
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

/// A Bearer-authenticated WebSocket API whose one endpoint is host-capable
/// (every message bidirectional), so the generator emits its host module.
/// No shipped WebSocket definition uses `BearerToken`.
fn bearer_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{
        ConnectionLifecycle, EndpointRole, MessageDirection, MessageSchema, WebSocketApi,
        WebSocketEndpoint,
    };

    WebSocketApi {
        name: "BearerFixtureWs".to_string(),
        description: "Bearer-authenticated WebSocket fixture".to_string(),
        base_url: "ws://127.0.0.1".to_string(),
        docs_url: None,
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec!["BEARER_FIXTURE_WS_TOKEN".to_string()],
        env_mapping: None,
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Stream".to_string(),
            path: "/stream".to_string(),
            description: "Echo stream".to_string(),
            role: EndpointRole::Host,
            lifecycle: ConnectionLifecycle::default(),
            messages: vec![MessageSchema::new(
                "Frame",
                MessageDirection::Bidirectional,
                Schema::new("serde_json::Value"),
            )],
            ..WebSocketEndpoint::default()
        }],
        runtime: None,
    }
}

/// Child tests run against the generated Bearer host. The child process gets
/// `BEARER_FIXTURE_WS_TOKEN=fixture-token`; the host reads it per upgrade.
const BEARER_WS_HOST_TEST: &str = r##"
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use schematic_schema::ws_bearer_fixture::{
    BearerFixtureWsHost, StreamClientMessage, StreamServerMessage, WsConnectionContext, WsHandler,
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error, Message};

struct Echo;

impl WsHandler for Echo {
    fn handle_message(
        &self,
        message: StreamClientMessage,
        _context: WsConnectionContext,
    ) -> impl std::future::Future<Output = Option<StreamServerMessage>> + Send {
        let StreamClientMessage::Frame(value) = message;
        async move { Some(StreamServerMessage::Frame(value)) }
    }
}

async fn start_host() -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(BearerFixtureWsHost::serve(listener, Arc::new(Echo)));
    addr
}

/// The upgrade's HTTP status, and on success one echoed frame.
async fn upgrade(authorization: Option<&str>) -> (u16, Option<serde_json::Value>) {
    let addr = start_host().await;
    let mut request = format!("ws://{addr}/stream").into_client_request().unwrap();
    if let Some(value) = authorization {
        request.headers_mut().insert("Authorization", value.parse().unwrap());
    }
    match tokio_tungstenite::connect_async(request).await {
        Ok((mut ws, response)) => {
            ws.send(Message::Text(r#"{"ping":1}"#.into())).await.unwrap();
            let echoed = match ws.next().await {
                Some(Ok(Message::Text(text))) => serde_json::from_str(text.as_ref()).unwrap(),
                other => panic!("expected an echoed text frame, got {other:?}"),
            };
            (response.status().as_u16(), Some(echoed))
        }
        Err(Error::Http(response)) => (response.status().as_u16(), None),
        Err(other) => panic!("unexpected handshake failure: {other}"),
    }
}

#[tokio::test]
async fn bearer_host_rejects_a_missing_token_with_401() {
    assert_eq!(upgrade(None).await, (401, None));
}

#[tokio::test]
async fn bearer_host_rejects_a_wrong_token_with_401() {
    assert_eq!(upgrade(Some("Bearer wrong-token")).await, (401, None));
}

#[tokio::test]
async fn bearer_host_upgrades_a_matching_token() {
    assert_eq!(
        upgrade(Some("Bearer fixture-token")).await,
        (101, Some(serde_json::json!({"ping": 1})))
    );
}
"##;

/// A client fixture whose handshake uses every supported parameter form:
/// a path value, a required and an optional scalar query, an exploded and a
/// comma/pipe-delimited array.
fn query_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{MessageDirection, MessageSchema, WebSocketApi, WebSocketEndpoint};
    use schematic_define::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
    let array = |element: QueryParamType| QueryParamType::Array(Box::new(element));

    WebSocketApi {
        name: "QueryFixtureWs".to_string(),
        description: "Handshake parameter fixture".to_string(),
        base_url: "ws://127.0.0.1".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_mapping: None,
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Feed".to_string(),
            path: "/rooms/{room}/feed".to_string(),
            description: "Feed".to_string(),
            params: EndpointParams {
                path: vec![ParamDef::path("room", QueryParamType::String)],
                query: vec![
                    ParamDef::new("mode", QueryParamType::String).required(),
                    ParamDef::new("limit", QueryParamType::Integer),
                    ParamDef { explode: true, ..ParamDef::new("tag", array(QueryParamType::String)) },
                    ParamDef::new("ids", array(QueryParamType::Integer)),
                    ParamDef {
                        style: ParamStyle::PipeDelimited,
                        ..ParamDef::new("flags", array(QueryParamType::Boolean))
                    },
                ],
                ..EndpointParams::default()
            },
            messages: vec![MessageSchema::new(
                "Frame",
                MessageDirection::Server,
                Schema::new("serde_json::Value"),
            )],
            ..WebSocketEndpoint::default()
        }],
        runtime: None,
    }
}

/// Child tests: the generated client's handshake target, as the server sees it.
const QUERY_WS_CLIENT_TEST: &str = r##"
use schematic_schema::ws_query_fixture::{FeedConnectionParams, QueryFixtureWs};
use schematic_schema::ws_shared::WsClientOptions;

/// The request target of one handshake to a base URL with a `/base` path.
async fn target(room: &str, params: FeedConnectionParams) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut seen = None;
        let _ws = tokio_tungstenite::accept_hdr_async(stream, |request: &tokio_tungstenite::tungstenite::handshake::server::Request, response| {
            seen = Some(request.uri().to_string());
            Ok(response)
        })
        .await
        .unwrap();
        seen.unwrap()
    });
    let client = QueryFixtureWs::with_base_url(format!("ws://{addr}/base"));
    let ws = client.connect_feed(room, params, WsClientOptions::default()).await.unwrap();
    let _ = ws.close().await;
    server.await.unwrap()
}

#[tokio::test]
async fn path_value_is_one_encoded_segment_and_unset_options_are_omitted() {
    let params = FeedConnectionParams { mode: "live".to_string(), limit: None, tag: None, ids: None, flags: None };
    assert_eq!(target("a/b c?d", params).await, "/base/rooms/a%2Fb%20c%3Fd/feed?mode=live");
}

#[tokio::test]
async fn array_values_follow_their_declared_style() {
    let params = FeedConnectionParams {
        mode: "a b".to_string(),
        limit: Some(5),
        tag: Some(vec!["x,y".to_string(), "z".to_string()]),
        ids: Some(vec![1, 2]),
        flags: Some(vec![true, false]),
    };
    assert_eq!(
        target("r", params).await,
        "/base/rooms/r/feed?flags=true%7Cfalse&ids=1,2&limit=5&mode=a%20b&tag=x%2Cy&tag=z"
    );
}
"##;

#[test]
fn slow_ws_handshake_parameters_reach_the_server_in_their_declared_form() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let module = schematic_gen::output::ws_modules::render_ws_api_module(&query_ws_fixture())
        .expect("the query fixture lowers");

    let fresh = FreshCrate::generate(&[], Definitions::Checkout, &lockfile);
    fresh.add_module("ws_query_fixture", &module);
    fresh.add_test_file("query_ws_client", QUERY_WS_CLIENT_TEST);
    let run = fresh.nextest_run(&[]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in [
        "path_value_is_one_encoded_segment_and_unset_options_are_omitted",
        "array_values_follow_their_declared_style",
    ] {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}

#[test]
fn slow_bearer_ws_host_compiles_and_checks_tokens() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let module = schematic_gen::output::ws_modules::render_ws_api_module(&bearer_ws_fixture())
        .expect("the Bearer fixture lowers");
    assert!(module.contains("pub struct BearerFixtureWsHost"), "{module}");

    let fresh = FreshCrate::generate(&[], Definitions::Checkout, &lockfile);
    fresh.add_module("ws_bearer_fixture", &module);
    fresh.cargo_check().assert_success();

    fresh.add_test_file("bearer_ws_host", BEARER_WS_HOST_TEST);
    let run = fresh.nextest_run(&[("BEARER_FIXTURE_WS_TOKEN", "fixture-token")]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in [
        "bearer_host_rejects_a_missing_token_with_401",
        "bearer_host_rejects_a_wrong_token_with_401",
        "bearer_host_upgrades_a_matching_token",
    ] {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}

/// Payload types the typed WebSocket fixtures name, added to the fresh crate
/// as `src/ws_payloads.rs` and named from the generated modules as
/// `crate::ws_payloads::…`. Each derives only the serde trait its direction
/// uses and none derives `Clone`, so the generated enums must ask for no
/// more than that.
const WS_PAYLOADS: &str = r#"
#[derive(Debug, serde::Deserialize)]
pub struct Envelope {
    pub kind: String,
    #[serde(default)]
    pub msg: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Pong {
    pub msg: String,
    pub seq: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct Special {
    pub detail: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Ping {
    pub seq: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct Say {
    pub text: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Join {
    pub name: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Chat {
    pub text: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Welcome {
    pub greeting: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Said {
    pub from: String,
    pub text: String,
}

/// Never serializes, so a host write of it fails.
#[derive(Debug)]
pub struct Unwritable;

impl serde::Serialize for Unwritable {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("unwritable fixture payload"))
    }
}
"#;

fn ws_fixture_endpoint(
    id: &str,
    path: &str,
    role: schematic_define::websocket::EndpointRole,
    messages: Vec<schematic_define::websocket::MessageSchema>,
) -> schematic_define::websocket::WebSocketEndpoint {
    schematic_define::websocket::WebSocketEndpoint {
        id: id.to_string(),
        path: path.to_string(),
        description: format!("{id} fixture endpoint"),
        role,
        messages,
        ..schematic_define::websocket::WebSocketEndpoint::default()
    }
}

fn ws_fixture_api(
    name: &str,
    auth: AuthStrategy,
    env_auth: &[&str],
    endpoints: Vec<schematic_define::websocket::WebSocketEndpoint>,
) -> schematic_define::WebSocketApi {
    schematic_define::WebSocketApi {
        name: name.to_string(),
        description: format!("{name} fixture"),
        base_url: "ws://127.0.0.1".to_string(),
        docs_url: None,
        auth,
        auth_policy: None,
        env_auth: env_auth.iter().map(|name| name.to_string()).collect(),
        env_mapping: None,
        headers: vec![],
        version: None,
        endpoints,
        runtime: None,
    }
}

/// A client API whose `Feed` server direction overlaps by strict subsets
/// (`kind=resp` fallback, `kind=event`, `kind=event,msg=special`,
/// `kind=resp,msg=pong`) and whose `Strict` server direction has no
/// fallback. `Listen` sends nothing and `Shout` receives nothing, so each
/// has one uninhabited enum.
fn typed_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{EndpointRole, MessageDirection, MessageSchema};
    let payload = |name: &str| Schema::with_path(name, "crate::ws_payloads");
    let value = || Schema::new("serde_json::Value");
    let server = |name: &str, schema| MessageSchema::new(name, MessageDirection::Server, schema);
    let client = |name: &str, schema| MessageSchema::new(name, MessageDirection::Client, schema);

    ws_fixture_api(
        "TypedFixtureWs",
        AuthStrategy::None,
        &[],
        vec![
            ws_fixture_endpoint("Feed", "/feed", EndpointRole::Client, vec![
                server("Response", payload("Envelope")).fallback(),
                server("Event", payload("Envelope")).matches("kind", "event"),
                server("Pong", payload("Pong")).matches("kind", "resp").matches("msg", "pong"),
                server("Special", payload("Special")).matches("kind", "event").matches("msg", "special"),
                client("Ping", payload("Ping")),
                client("Say", payload("Say")),
            ]),
            ws_fixture_endpoint("Strict", "/strict", EndpointRole::Client, vec![
                server("Hello", value()).matches("type", "hello"),
                server("Bye", value()).matches("type", "bye"),
                client("Note", value()),
            ]),
            ws_fixture_endpoint("Listen", "/listen", EndpointRole::Client, vec![server("Tick", value())]),
            ws_fixture_endpoint("Shout", "/shout", EndpointRole::Client, vec![client("Line", value())]),
        ],
    )
}

/// A client API framed as JSON in binary frames.
fn binary_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{
        EndpointRole, FrameFormat, MessageDirection, MessageSchema, WebSocketRuntimeHints,
    };
    let mut api = ws_fixture_api("BinaryFixtureWs", AuthStrategy::None, &[], vec![ws_fixture_endpoint(
        "Blob",
        "/blob",
        EndpointRole::Client,
        vec![
            MessageSchema::new("Got", MessageDirection::Server, Schema::new("serde_json::Value")),
            MessageSchema::new("Put", MessageDirection::Client, Schema::new("serde_json::Value")),
        ],
    )]);
    api.runtime = Some(WebSocketRuntimeHints {
        frame_format: FrameFormat::JsonBinary,
        ..WebSocketRuntimeHints::default()
    });
    api
}

/// A multi-endpoint host that is not Unfolded Circle's: `Room` (host only,
/// a path value) and `Lobby` (both roles), behind an API key header.
fn chat_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{EndpointRole, MessageDirection, MessageSchema};
    let payload = |name: &str| Schema::with_path(name, "crate::ws_payloads");
    let value = || Schema::new("serde_json::Value");
    let server = |name: &str, schema| MessageSchema::new(name, MessageDirection::Server, schema);
    let client = |name: &str, schema| MessageSchema::new(name, MessageDirection::Client, schema);

    let mut room = ws_fixture_endpoint("Room", "/rooms/{room}", EndpointRole::Host, vec![
        client("Join", payload("Join")).matches("op", "join"),
        client("Chat", payload("Chat")).matches("op", "chat"),
        server("Welcome", payload("Welcome")),
        server("Said", payload("Said")),
        server("Unwritable", payload("Unwritable")),
    ]);
    room.params = EndpointParams {
        path: vec![ParamDef::path("room", QueryParamType::String)],
        ..EndpointParams::default()
    };
    ws_fixture_api(
        "ChatFixtureWs",
        AuthStrategy::ApiKey { header: "x-fixture-key".to_string(), value_prefix: None },
        &["CHAT_FIXTURE_WS_KEY"],
        vec![
            room,
            ws_fixture_endpoint("Lobby", "/lobby", EndpointRole::Both, vec![
                client("Hello", value()),
                server("Notice", value()),
            ]),
        ],
    )
}

/// Imports of every typed-send misuse file and its control.
const WS_MISUSE_PRELUDE: &str = "#![allow(dead_code, unused_imports, unreachable_code)]
use schematic_schema::ws_payloads::{Ping, Say};
use schematic_schema::ws_typed_fixture::{
    FeedClient, FeedClientMessage, FeedServerMessage, ListenClientMessage, ShoutServerMessage,
    StrictClient, StrictClientMessage,
};
";

/// Correct typed sends with the misuse imports, and a match over each
/// uninhabited enum: it must compile, so each misuse fails for its own reason.
const WS_SEND_CONTROL: &str = "async fn used(feed: FeedClient, strict: StrictClient) {
    let _ = feed.send(FeedClientMessage::Ping(Ping { seq: 1 })).await;
    let _ = feed.send(FeedClientMessage::Say(Say { text: String::new() })).await;
    let _ = strict.send(StrictClientMessage::Note(serde_json::Value::Null)).await;
    let _ = feed.send_value(serde_json::json!({})).await;
}

fn nothing_to_send(message: ListenClientMessage) -> ! {
    match message {}
}

fn nothing_to_receive(message: ShoutServerMessage) -> ! {
    match message {}
}
";

/// R10: sending a value of the wrong type does not compile. Same shape as
/// [`MISUSES`]; entry `i` becomes `tests/ws_misuse_<i>.rs`.
const WS_MISUSES: &[(&str, &str, &str, &str)] = &[
    (
        "a server message is not a client message",
        "async fn misuse(client: FeedClient, message: FeedServerMessage) { let _ = client.send(message).await; }",
        "E0308",
        "FeedServerMessage",
    ),
    (
        "a bare payload is not a message",
        "async fn misuse(client: FeedClient) { let _ = client.send(Ping { seq: 1 }).await; }",
        "E0308",
        "found `Ping`",
    ),
    (
        "another endpoint's message is not this endpoint's",
        "async fn misuse(client: StrictClient) { let _ = client.send(FeedClientMessage::Say(Say { text: String::new() })).await; }",
        "E0308",
        "StrictClientMessage",
    ),
];

/// Child tests of the generated typed clients against a loopback
/// tokio-tungstenite server on port 0.
const TYPED_WS_CLIENT_TEST: &str = r##"
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use schematic_schema::ws_binary_fixture::{BinaryFixtureWs, BlobClientMessage, BlobServerMessage};
use schematic_schema::ws_payloads::{Ping, Say};
use schematic_schema::ws_shared::{WsClientOptions, WsError, WsRawFrame};
use schematic_schema::ws_typed_fixture::{
    FeedClient, FeedClientMessage, FeedServerMessage, StrictServerMessage, TypedFixtureWs,
};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

type Peer = WebSocketStream<TcpStream>;

/// A bound on a hang, never a synchronization.
const WAIT: Duration = Duration::from_secs(5);

async fn within<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(WAIT, future).await.expect("timed out")
}

/// A loopback base URL and the server side of the one connection it accepts.
async fn loopback() -> (String, tokio::task::JoinHandle<Peer>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        tokio_tungstenite::accept_async(stream).await.unwrap()
    });
    (format!("ws://{addr}"), server)
}

async fn connect_feed() -> (FeedClient, Peer) {
    let (url, server) = loopback().await;
    let client = within(TypedFixtureWs::with_base_url(url).connect_feed(WsClientOptions::default()))
        .await
        .unwrap();
    (client, within(server).await.unwrap())
}

async fn send_text(peer: &mut Peer, frame: &str) {
    peer.send(Message::Text(frame.to_string().into())).await.unwrap();
}

async fn next_feed(client: &mut FeedClient) -> FeedServerMessage {
    within(client.next_event()).await.expect("the connection is open").expect("the frame decodes")
}

/// The next data frame the peer receives.
async fn received(peer: &mut Peer) -> Message {
    loop {
        match within(peer.next()).await.expect("the connection is open").unwrap() {
            frame @ (Message::Text(_) | Message::Binary(_)) => return frame,
            _ => continue,
        }
    }
}

async fn received_text(peer: &mut Peer) -> String {
    match received(peer).await {
        Message::Text(text) => text.as_str().to_string(),
        other => panic!("expected a text frame, got {other:?}"),
    }
}

#[tokio::test]
async fn every_server_message_is_selected_from_its_frame() {
    let (mut client, mut peer) = connect_feed().await;
    for frame in [
        r#"{"kind":"resp","msg":"pong","seq":7}"#,
        r#"{"kind":"resp","msg":"other"}"#,
        r#"{"kind":"event","msg":"special","detail":"d1"}"#,
        r#"{"kind":"event","msg":"tick"}"#,
        r#"{"kind":"resp"}"#,
    ] {
        send_text(&mut peer, frame).await;
    }

    // Also matches the generic `kind=resp`: the strict superset wins.
    match next_feed(&mut client).await {
        FeedServerMessage::Pong(pong) => assert_eq!((pong.msg.as_str(), pong.seq), ("pong", 7)),
        other => panic!("expected Pong, got {other:?}"),
    }
    match next_feed(&mut client).await {
        FeedServerMessage::Response(envelope) => assert_eq!(envelope.msg.as_deref(), Some("other")),
        other => panic!("expected the Response fallback, got {other:?}"),
    }
    // Also matches the generic `kind=event`.
    match next_feed(&mut client).await {
        FeedServerMessage::Special(special) => assert_eq!(special.detail, "d1"),
        other => panic!("expected Special, got {other:?}"),
    }
    match next_feed(&mut client).await {
        FeedServerMessage::Event(envelope) => {
            assert_eq!((envelope.kind.as_str(), envelope.msg.as_deref()), ("event", Some("tick")));
        }
        other => panic!("expected Event, got {other:?}"),
    }
    match next_feed(&mut client).await {
        FeedServerMessage::Response(envelope) => {
            assert_eq!((envelope.kind.as_str(), envelope.msg.as_deref()), ("resp", None));
        }
        other => panic!("expected the Response fallback, got {other:?}"),
    }
}

#[tokio::test]
async fn an_unmatched_frame_without_a_fallback_is_a_decode_error_and_the_connection_stays_usable() {
    let (url, server) = loopback().await;
    let mut client = within(TypedFixtureWs::with_base_url(url).connect_strict(WsClientOptions::default()))
        .await
        .unwrap();
    let mut peer = within(server).await.unwrap();
    let unmatched = r#"{ "type": "unknown",  "n": 1 }"#;
    send_text(&mut peer, unmatched).await;
    send_text(&mut peer, r#"{"type":"bye","n":2}"#).await;

    match within(client.next_event()).await.expect("the connection is open") {
        Err(WsError::Decode { frame: WsRawFrame::Text(text), reason }) => {
            assert_eq!(text, unmatched, "the frame is kept exactly as received");
            assert!(reason.contains("matches no declared message"), "{reason}");
        }
        other => panic!("expected a text Decode error, got {other:?}"),
    }
    match within(client.next_event()).await.expect("the connection is open") {
        Ok(StrictServerMessage::Bye(value)) => assert_eq!(value, json!({"type": "bye", "n": 2})),
        other => panic!("expected Bye, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_text_and_binary_frames_are_decode_errors_that_keep_the_frame() {
    let (mut client, mut peer) = connect_feed().await;
    let broken_text = r#"{"kind": "event", "#;
    let broken_bytes = vec![0xff, 0x00, b'{'];
    send_text(&mut peer, broken_text).await;
    peer.send(Message::Binary(broken_bytes.clone().into())).await.unwrap();
    send_text(&mut peer, r#"{"kind":"event","msg":"after"}"#).await;

    match within(client.next_event()).await.expect("the connection is open") {
        Err(WsError::Decode { frame: WsRawFrame::Text(text), reason }) => {
            assert_eq!(text, broken_text);
            assert!(reason.starts_with("not JSON"), "{reason}");
        }
        other => panic!("expected a text Decode error, got {other:?}"),
    }
    match within(client.next_event()).await.expect("the connection is open") {
        Err(WsError::Decode { frame: WsRawFrame::Binary(bytes), .. }) => assert_eq!(bytes, broken_bytes),
        other => panic!("expected a binary Decode error, got {other:?}"),
    }
    match next_feed(&mut client).await {
        FeedServerMessage::Event(envelope) => assert_eq!(envelope.msg.as_deref(), Some("after")),
        other => panic!("expected Event, got {other:?}"),
    }
}

#[tokio::test]
async fn send_writes_the_payload_alone_and_send_value_writes_the_value_verbatim() {
    let (client, mut peer) = connect_feed().await;
    client.send(FeedClientMessage::Ping(Ping { seq: 3 })).await.unwrap();
    client.send(FeedClientMessage::Say(Say { text: "hi".to_string() })).await.unwrap();
    let value = json!({"kind": "free", "raw": [1, 2]});
    client.send_value(value.clone()).await.unwrap();

    assert_eq!(received_text(&mut peer).await, r#"{"seq":3}"#);
    assert_eq!(received_text(&mut peer).await, r#"{"text":"hi"}"#);
    let sent: serde_json::Value = serde_json::from_str(&received_text(&mut peer).await).unwrap();
    assert_eq!(sent, value);
}

#[tokio::test]
async fn typed_and_value_receives_share_one_queue_and_the_event_stream_keeps_the_connection_open() {
    let (mut client, mut peer) = connect_feed().await;
    send_text(&mut peer, r#"{"kind":"event","msg":"first"}"#).await;
    send_text(&mut peer, r#"{"kind":"resp","msg":"pong","seq":2}"#).await;
    send_text(&mut peer, r#"{"kind":"event","msg":"third"}"#).await;

    let first = within(client.next_value_event()).await.expect("the connection is open").unwrap();
    assert_eq!(first, json!({"kind": "event", "msg": "first"}));
    match next_feed(&mut client).await {
        FeedServerMessage::Pong(pong) => assert_eq!(pong.seq, 2),
        other => panic!("expected the second frame as Pong, got {other:?}"),
    }

    let mut events = client.events();
    match within(events.next()).await.expect("the stream is open").unwrap() {
        FeedServerMessage::Event(envelope) => assert_eq!(envelope.msg.as_deref(), Some("third")),
        other => panic!("expected the third frame as Event, got {other:?}"),
    }
    // Sent after the client was consumed: the stream holds the connection.
    send_text(&mut peer, r#"{"kind":"event","msg":"special","detail":"late"}"#).await;
    match within(events.next()).await.expect("the stream is open").unwrap() {
        FeedServerMessage::Special(special) => assert_eq!(special.detail, "late"),
        other => panic!("expected Special, got {other:?}"),
    }

    drop(events);
    loop {
        match within(peer.next()).await {
            None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
            Some(Ok(Message::Text(_) | Message::Binary(_))) => panic!("no data after the stream is dropped"),
            Some(Ok(_)) => continue,
        }
    }
}

#[tokio::test]
async fn a_json_binary_api_sends_and_receives_binary_frames() {
    let (url, server) = loopback().await;
    let mut client = within(BinaryFixtureWs::with_base_url(url).connect_blob(WsClientOptions::default()))
        .await
        .unwrap();
    let mut peer = within(server).await.unwrap();
    client.send(BlobClientMessage::Put(json!({"id": 1}))).await.unwrap();
    client.send_value(json!({"id": 2})).await.unwrap();

    for expected in [json!({"id": 1}), json!({"id": 2})] {
        match received(&mut peer).await {
            Message::Binary(bytes) => {
                assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(), expected);
            }
            other => panic!("expected a binary frame, got {other:?}"),
        }
    }
    peer.send(Message::Binary(serde_json::to_vec(&json!({"id": 3})).unwrap().into())).await.unwrap();
    match within(client.next_event()).await.expect("the connection is open") {
        Ok(BlobServerMessage::Got(value)) => assert_eq!(value, json!({"id": 3})),
        other => panic!("expected Got, got {other:?}"),
    }
}

#[tokio::test]
async fn an_endpoint_without_server_messages_reports_every_frame_as_undecodable() {
    let (url, server) = loopback().await;
    let mut client = within(TypedFixtureWs::with_base_url(url).connect_shout(WsClientOptions::default()))
        .await
        .unwrap();
    let mut peer = within(server).await.unwrap();
    send_text(&mut peer, r#"{"any":1}"#).await;
    match within(client.next_event()).await.expect("the connection is open") {
        Err(WsError::Decode { frame: WsRawFrame::Text(text), .. }) => assert_eq!(text, r#"{"any":1}"#),
        other => panic!("expected a Decode error, got {other:?}"),
    }
}
"##;

/// The child tests [`TYPED_WS_CLIENT_TEST`] must report.
const TYPED_WS_CLIENT_TESTS: &[&str] = &[
    "every_server_message_is_selected_from_its_frame",
    "an_unmatched_frame_without_a_fallback_is_a_decode_error_and_the_connection_stays_usable",
    "malformed_text_and_binary_frames_are_decode_errors_that_keep_the_frame",
    "send_writes_the_payload_alone_and_send_value_writes_the_value_verbatim",
    "typed_and_value_receives_share_one_queue_and_the_event_stream_keeps_the_connection_open",
    "a_json_binary_api_sends_and_receives_binary_frames",
    "an_endpoint_without_server_messages_reports_every_frame_as_undecodable",
];

/// Child tests of the generated multi-endpoint host serving on port 0. The
/// child process gets `CHAT_FIXTURE_WS_KEY=chat-key`.
const CHAT_WS_HOST_TEST: &str = r##"
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use schematic_schema::ws_chat_fixture::{
    ChatFixtureEventHub, ChatFixtureWs, ChatFixtureWsHost, LobbyClientMessage, LobbyServerMessage,
    RoomClientMessage, RoomServerMessage, WsConnectionContext, WsHandler,
};
use schematic_schema::ws_payloads::{Said, Unwritable, Welcome};
use schematic_schema::ws_shared::{WsClientOptions, WsError, WsRawFrame};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::{Error, Message};

type Peer = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// The key the child environment configures for the host.
const KEY: &str = "chat-key";

/// A bound on a hang, never a synchronization.
const WAIT: Duration = Duration::from_secs(5);

async fn within<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(WAIT, future).await.expect("timed out")
}

#[derive(Clone, Copy)]
enum Greeting {
    Welcome,
    Nothing,
    Unwritable,
}

struct Chat {
    greeting: Greeting,
    room_hooks: AtomicUsize,
    handled: Mutex<Vec<String>>,
    errors: mpsc::UnboundedSender<String>,
}

impl Chat {
    fn new(greeting: Greeting) -> (Arc<Self>, mpsc::UnboundedReceiver<String>) {
        let (errors, reported) = mpsc::unbounded_channel();
        let chat = Self { greeting, room_hooks: AtomicUsize::new(0), handled: Mutex::new(Vec::new()), errors };
        (Arc::new(chat), reported)
    }

    fn handled(&self) -> Vec<String> {
        self.handled.lock().unwrap().clone()
    }

    fn hooks(&self) -> usize {
        self.room_hooks.load(Ordering::SeqCst)
    }
}

impl WsHandler for Chat {
    fn handle_room_message(
        &self,
        message: RoomClientMessage,
        _context: WsConnectionContext,
    ) -> impl std::future::Future<Output = Option<RoomServerMessage>> + Send {
        let (entry, reply) = match message {
            RoomClientMessage::Join(join) => (
                format!("room join {}", join.name),
                Said { from: "host".to_string(), text: format!("welcome {}", join.name) },
            ),
            RoomClientMessage::Chat(chat) => (
                format!("room chat {}", chat.text),
                Said { from: "room".to_string(), text: chat.text },
            ),
        };
        self.handled.lock().unwrap().push(entry);
        async move { Some(RoomServerMessage::Said(reply)) }
    }

    fn on_room_authenticated(
        &self,
        context: WsConnectionContext,
    ) -> impl std::future::Future<Output = Option<RoomServerMessage>> + Send {
        self.room_hooks.fetch_add(1, Ordering::SeqCst);
        let greeting = match self.greeting {
            Greeting::Welcome => Some(RoomServerMessage::Welcome(Welcome {
                greeting: format!("verified={}", context.credential_verified()),
            })),
            Greeting::Nothing => None,
            Greeting::Unwritable => Some(RoomServerMessage::Unwritable(Unwritable)),
        };
        async move { greeting }
    }

    fn handle_lobby_message(
        &self,
        message: LobbyClientMessage,
        _context: WsConnectionContext,
    ) -> impl std::future::Future<Output = Option<LobbyServerMessage>> + Send {
        let LobbyClientMessage::Hello(value) = message;
        self.handled.lock().unwrap().push(format!("lobby hello {value}"));
        async move { Some(LobbyServerMessage::Notice(json!({"lobby": value}))) }
    }

    fn on_error(
        &self,
        error: WsError,
        _context: WsConnectionContext,
    ) -> impl std::future::Future<Output = ()> + Send {
        let report = match &error {
            WsError::Decode { frame: WsRawFrame::Text(text), .. } => format!("decode {text}"),
            WsError::Serde(serde) => format!("serde {serde}"),
            other => format!("other {other:?}"),
        };
        let _ = self.errors.send(report);
        async {}
    }
}

async fn start(handler: Arc<Chat>) -> (std::net::SocketAddr, ChatFixtureEventHub) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hub = ChatFixtureWsHost::new_event_hub();
    tokio::spawn(ChatFixtureWsHost::serve_with_hub(listener, handler, hub.clone()));
    (addr, hub)
}

/// An upgraded connection, or the HTTP status that refused it.
async fn dial(addr: std::net::SocketAddr, path: &str, key: Option<&str>) -> Result<Peer, u16> {
    let mut request = format!("ws://{addr}{path}").into_client_request().unwrap();
    if let Some(key) = key {
        request.headers_mut().insert("x-fixture-key", key.parse().unwrap());
    }
    match within(tokio_tungstenite::connect_async(request)).await {
        Ok((peer, _)) => Ok(peer),
        Err(Error::Http(response)) => Err(response.status().as_u16()),
        Err(other) => panic!("unexpected handshake failure: {other}"),
    }
}

async fn send_json(peer: &mut Peer, value: Value) {
    peer.send(Message::Text(value.to_string().into())).await.unwrap();
}

async fn send_text(peer: &mut Peer, text: &str) {
    peer.send(Message::Text(text.to_string().into())).await.unwrap();
}

/// The next data frame, as JSON.
async fn next_json(peer: &mut Peer) -> Value {
    loop {
        match within(peer.next()).await.expect("the connection is open").unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            Message::Binary(bytes) => panic!("expected a text frame, got {bytes:?}"),
            Message::Close(close) => panic!("the connection closed: {close:?}"),
            _ => continue,
        }
    }
}

#[tokio::test]
async fn the_room_hook_message_is_the_first_frame_and_runs_once_per_connection() {
    let (chat, _errors) = Chat::new(Greeting::Welcome);
    let (addr, _hub) = start(chat.clone()).await;
    let mut peer = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();
    // Sent before anything is read: the hook's message still comes first.
    send_json(&mut peer, json!({"op": "join", "name": "alice"})).await;

    assert_eq!(next_json(&mut peer).await, json!({"greeting": "verified=true"}));
    assert_eq!(next_json(&mut peer).await, json!({"from": "host", "text": "welcome alice"}));
    assert_eq!(chat.hooks(), 1);
}

#[tokio::test]
async fn a_hook_returning_none_sends_nothing_before_the_reply() {
    let (chat, _errors) = Chat::new(Greeting::Nothing);
    let (addr, _hub) = start(chat.clone()).await;
    let mut peer = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();
    send_json(&mut peer, json!({"op": "chat", "text": "hi"})).await;

    assert_eq!(next_json(&mut peer).await, json!({"from": "room", "text": "hi"}));
    assert_eq!(chat.hooks(), 1, "the hook ran and sent nothing");
}

#[tokio::test]
async fn a_rejected_key_never_runs_the_hook() {
    let (chat, _errors) = Chat::new(Greeting::Welcome);
    let (addr, _hub) = start(chat.clone()).await;
    assert_eq!(dial(addr, "/rooms/abc", Some("wrong-key")).await.err(), Some(401));
    assert_eq!(dial(addr, "/rooms/abc", None).await.err(), Some(401));

    let mut peer = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();
    assert_eq!(next_json(&mut peer).await, json!({"greeting": "verified=true"}));
    assert_eq!(chat.hooks(), 1, "only the accepted upgrade ran the hook");
}

#[tokio::test]
async fn a_hook_message_that_fails_to_write_closes_the_connection_and_reaches_on_error() {
    let (chat, mut errors) = Chat::new(Greeting::Unwritable);
    let (addr, _hub) = start(chat.clone()).await;
    let mut peer = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();

    loop {
        match within(peer.next()).await {
            None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
            Some(Ok(Message::Text(_) | Message::Binary(_))) => panic!("no data frame after a failed hook write"),
            Some(Ok(_)) => continue,
        }
    }
    let report = within(errors.recv()).await.expect("on_error ran");
    assert!(report.starts_with("serde ") && report.contains("unwritable fixture payload"), "{report}");
    assert_eq!(chat.hooks(), 1);
    assert!(chat.handled().is_empty());
}

#[tokio::test]
async fn room_messages_arrive_typed_and_an_undecodable_frame_reaches_on_error_without_closing() {
    let (chat, mut errors) = Chat::new(Greeting::Nothing);
    let (addr, _hub) = start(chat.clone()).await;
    let mut peer = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();

    send_json(&mut peer, json!({"op": "join", "name": "bob"})).await;
    assert_eq!(next_json(&mut peer).await, json!({"from": "host", "text": "welcome bob"}));
    send_text(&mut peer, r#"{"op":"dance"}"#).await;
    send_text(&mut peer, "not json").await;
    send_json(&mut peer, json!({"op": "chat", "text": "still here"})).await;
    // The undecodable frames got no reply: the next frame answers the chat.
    assert_eq!(next_json(&mut peer).await, json!({"from": "room", "text": "still here"}));

    assert_eq!(within(errors.recv()).await.unwrap(), r#"decode {"op":"dance"}"#);
    assert_eq!(within(errors.recv()).await.unwrap(), "decode not json");
    assert_eq!(chat.handled(), ["room join bob", "room chat still here"]);
}

#[tokio::test]
async fn connections_route_by_path_and_unknown_paths_are_rejected_with_404() {
    let (chat, _errors) = Chat::new(Greeting::Nothing);
    let (addr, _hub) = start(chat.clone()).await;

    let mut room = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();
    send_json(&mut room, json!({"op": "chat", "text": "r"})).await;
    assert_eq!(next_json(&mut room).await, json!({"from": "room", "text": "r"}));

    // The generated Lobby client reads its key from the definition's env
    // mapping (`CHAT_FIXTURE_WS_KEY`, set for the child run) on connect.
    let api = ChatFixtureWs::with_base_url(format!("ws://{addr}"));
    let mut lobby = within(api.connect_lobby(WsClientOptions::default())).await.unwrap();
    lobby.send(LobbyClientMessage::Hello(json!("l"))).await.unwrap();
    match within(lobby.next_event()).await.expect("the connection is open") {
        Ok(LobbyServerMessage::Notice(value)) => assert_eq!(value, json!({"lobby": "l"})),
        other => panic!("expected Notice, got {other:?}"),
    }
    assert_eq!(chat.handled(), ["room chat r", r#"lobby hello "l""#]);

    for path in ["/nowhere", "/rooms", "/rooms/", "/rooms/abc/extra"] {
        assert_eq!(dial(addr, path, Some(KEY)).await.err(), Some(404), "{path}");
    }
}

#[tokio::test]
async fn typed_broadcasts_reach_their_endpoint_only_and_value_broadcasts_reach_every_connection() {
    let (chat, _errors) = Chat::new(Greeting::Nothing);
    let (addr, hub) = start(chat.clone()).await;
    // A round trip on each proves both connections are registered.
    let mut room = dial(addr, "/rooms/abc", Some(KEY)).await.unwrap();
    send_json(&mut room, json!({"op": "chat", "text": "ready"})).await;
    assert_eq!(next_json(&mut room).await, json!({"from": "room", "text": "ready"}));
    let mut lobby = dial(addr, "/lobby", Some(KEY)).await.unwrap();
    send_json(&mut lobby, json!("ready")).await;
    assert_eq!(next_json(&mut lobby).await, json!({"lobby": "ready"}));

    let rooms_only = RoomServerMessage::Said(Said { from: "hub".to_string(), text: "rooms only".to_string() });
    assert_eq!(hub.broadcast(rooms_only).await.unwrap(), 1);
    assert_eq!(hub.broadcast_value(json!({"all": 1})).await, 2);
    assert_eq!(hub.broadcast(LobbyServerMessage::Notice(json!("lobby only"))).await.unwrap(), 1);
    assert_eq!(hub.broadcast_value(json!({"all": 2})).await, 2);

    // Each connection's queue is ordered, so a typed broadcast it missed
    // would have arrived before the value that follows it.
    assert_eq!(next_json(&mut room).await, json!({"from": "hub", "text": "rooms only"}));
    assert_eq!(next_json(&mut room).await, json!({"all": 1}));
    assert_eq!(next_json(&mut room).await, json!({"all": 2}));
    assert_eq!(next_json(&mut lobby).await, json!({"all": 1}));
    assert_eq!(next_json(&mut lobby).await, json!("lobby only"));
    assert_eq!(next_json(&mut lobby).await, json!({"all": 2}));
}
"##;

/// The child tests [`CHAT_WS_HOST_TEST`] must report.
const CHAT_WS_HOST_TESTS: &[&str] = &[
    "the_room_hook_message_is_the_first_frame_and_runs_once_per_connection",
    "a_hook_returning_none_sends_nothing_before_the_reply",
    "a_rejected_key_never_runs_the_hook",
    "a_hook_message_that_fails_to_write_closes_the_connection_and_reaches_on_error",
    "room_messages_arrive_typed_and_an_undecodable_frame_reaches_on_error_without_closing",
    "connections_route_by_path_and_unknown_paths_are_rejected_with_404",
    "typed_broadcasts_reach_their_endpoint_only_and_value_broadcasts_reach_every_connection",
];

/// The `#[tokio::test]` functions a child test source declares, in order.
fn child_tests(source: &str) -> Vec<&str> {
    source
        .split("#[tokio::test]")
        .skip(1)
        .filter_map(|rest| rest.trim_start().strip_prefix("async fn "))
        .filter_map(|rest| rest.split('(').next())
        .collect()
}

#[test]
fn ws_child_test_lists_name_every_child_test() {
    assert_eq!(child_tests(TYPED_WS_CLIENT_TEST), TYPED_WS_CLIENT_TESTS);
    assert_eq!(child_tests(CHAT_WS_HOST_TEST), CHAT_WS_HOST_TESTS);
    assert_eq!(child_tests(CREDENTIAL_WS_CLIENT_TEST), CREDENTIAL_WS_CLIENT_TESTS);
    assert_eq!(child_tests(SESSION_WS_CLIENT_TEST), SESSION_WS_CLIENT_TESTS);
}

/// R10 and R14 in one fresh crate: typed clients (selection, decode errors,
/// typed and value sends, one receive queue, binary framing, uninhabited
/// directions), sends of the wrong type failing to compile, and a
/// multi-endpoint host that is not Unfolded Circle's.
#[test]
fn slow_typed_ws_clients_and_a_multi_endpoint_host_compile_and_run() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let render = |api: schematic_define::WebSocketApi| {
        schematic_gen::output::ws_modules::render_ws_api_module(&api)
            .unwrap_or_else(|error| panic!("{} lowers: {error}", api.name))
    };
    let typed = render(typed_ws_fixture());
    let binary = render(binary_ws_fixture());
    let chat = render(chat_ws_fixture());

    // R14: the host is protocol-neutral and named for its API.
    assert!(!chat.contains("UnfoldedCircle"), "{chat}");
    assert!(chat.contains("pub struct ChatFixtureEventHub"), "{chat}");
    assert!(chat.contains("fn on_room_authenticated"), "{chat}");
    assert!(chat.contains("fn on_lobby_authenticated"), "{chat}");
    assert!(!chat.contains("authentication_success_response"), "{chat}");

    let fresh = FreshCrate::generate(&[], Definitions::Checkout, &lockfile);
    fresh.add_module("ws_payloads", WS_PAYLOADS);
    fresh.add_module("ws_typed_fixture", &typed);
    fresh.add_module("ws_binary_fixture", &binary);
    fresh.add_module("ws_chat_fixture", &chat);
    fresh.cargo_check().assert_success();

    fresh.add_test_file("ws_send_control", &format!("{WS_MISUSE_PRELUDE}{WS_SEND_CONTROL}"));
    fresh.add_test_file("typed_ws_client", TYPED_WS_CLIENT_TEST);
    fresh.add_test_file("chat_ws_host", CHAT_WS_HOST_TEST);
    for (index, (_, snippet, _, _)) in WS_MISUSES.iter().enumerate() {
        fresh.add_test_file(&format!("ws_misuse_{index}"), &format!("{WS_MISUSE_PRELUDE}{snippet}\n"));
    }
    let check = fresh.cargo_check_tests();
    assert!(!check.success, "the misuse targets must fail to compile");
    let hint = offline_cache_hint(&check.stderr).map(|hint| format!(" ({hint})")).unwrap_or_default();
    let diagnostics: Vec<&str> = check.stderr.split("\nerror").collect();
    // rustc prints the location with the host's separator.
    let located = |file: &str| -> Vec<&str> {
        let (unix, windows) = (format!("tests/{file}.rs:"), format!("tests\\{file}.rs:"));
        diagnostics
            .iter()
            .copied()
            .filter(|diagnostic| diagnostic.contains(&unix) || diagnostic.contains(&windows))
            .collect()
    };
    for control in ["ws_send_control", "typed_ws_client", "chat_ws_host"] {
        assert!(located(control).is_empty(), "{control} must compile{hint}:\n{}", check.stderr);
    }
    for (index, (misuse, snippet, code, named)) in WS_MISUSES.iter().enumerate() {
        let found = located(&format!("ws_misuse_{index}"));
        assert!(
            found.iter().any(|diagnostic| diagnostic.starts_with(&format!("[{code}]")) && diagnostic.contains(named)),
            "{misuse} did not fail with {code} naming `{named}`{hint}:\n{snippet}\n\n{}",
            check.stderr
        );
        std::fs::remove_file(fresh.crate_dir.join(format!("tests/ws_misuse_{index}.rs"))).unwrap();
    }

    let run = fresh.nextest_run(&[("CHAT_FIXTURE_WS_KEY", "chat-key")]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in TYPED_WS_CLIENT_TESTS.iter().chain(CHAT_WS_HOST_TESTS) {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}

/// Payloads of the correlated `HeaderKeyFixtureWs` endpoint, added to the
/// fresh crate as `src/ws_credential_payloads.rs`.
const WS_CREDENTIAL_PAYLOADS: &str = r#"
#[derive(Debug, serde::Serialize)]
pub struct Ask {
    pub question: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Count {
    pub n: u64,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Answer {
    pub text: String,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Total {
    pub sum: u64,
}

#[derive(Debug, PartialEq, serde::Deserialize)]
pub struct Notice {
    pub text: String,
}
"#;

/// A client API with one `Feed` endpoint receiving JSON values.
fn feed_ws_fixture(name: &str, auth: AuthStrategy, env_auth: &[&str]) -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{EndpointRole, MessageDirection, MessageSchema};
    ws_fixture_api(name, auth, env_auth, vec![ws_fixture_endpoint("Feed", "/feed", EndpointRole::Client, vec![
        MessageSchema::new("Frame", MessageDirection::Server, Schema::new("serde_json::Value")),
    ])])
}

/// An API key header behind two env variables (the child sets the first
/// to empty), a constant header, and a correlated `Rpc` endpoint with
/// string ids: `Ask` answered by `Answer`, `Count` by `Total`, and a
/// `Notice` that answers nothing.
fn header_key_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{
        CorrelationHints, EndpointRole, MessageDirection, MessageSchema, RequestIdType,
        WebSocketEndpointHints, WebSocketRuntimeHints,
    };
    let payload = |name: &str| Schema::with_path(name, "crate::ws_credential_payloads");
    let server = |name: &str| MessageSchema::new(name, MessageDirection::Server, payload(name));
    let client = |name: &str| MessageSchema::new(name, MessageDirection::Client, payload(name));

    let mut rpc = ws_fixture_endpoint("Rpc", "/rpc", EndpointRole::Client, vec![
        client("Ask").responds_with("Answer"),
        client("Count").responds_with("Total"),
        server("Answer").matches("kind", "answer"),
        server("Total").matches("kind", "total"),
        server("Notice").matches("kind", "notice"),
    ]);
    rpc.runtime = Some(WebSocketEndpointHints {
        correlation: Some(CorrelationHints {
            request_id_field: "id".to_string(),
            response_id_field: "re".to_string(),
            default_timeout_secs: 5,
        }),
        ..WebSocketEndpointHints::default()
    });
    let mut api = ws_fixture_api(
        "HeaderKeyFixtureWs",
        AuthStrategy::ApiKey { header: "X-Fixture-Key".to_string(), value_prefix: None },
        &["HEADER_KEY_FIXTURE_WS_EMPTY", "HEADER_KEY_FIXTURE_WS_KEY"],
        vec![rpc],
    );
    api.headers = vec![("X-Client".to_string(), "fixture".to_string())];
    api.runtime = Some(WebSocketRuntimeHints {
        request_id_type: RequestIdType::String,
        ..WebSocketRuntimeHints::default()
    });
    api
}

/// The child environment of [`CREDENTIAL_WS_CLIENT_TEST`]. Only
/// fixture-specific names; `BEARER_FIXTURE_CLIENT_WS_TOKEN` and
/// `COOKIE_KEY_FIXTURE_WS_SESSION` stay unset.
const CREDENTIAL_WS_ENV: &[(&str, &str)] = &[
    ("HEADER_KEY_FIXTURE_WS_EMPTY", ""),
    ("HEADER_KEY_FIXTURE_WS_KEY", "env-key"),
    ("QUERY_KEY_FIXTURE_WS_KEY", "query-env-key"),
    ("BASIC_FIXTURE_WS_USER", "env-user"),
    ("BASIC_FIXTURE_WS_PASS", "env-pass"),
];

/// Child tests of generated client handshakes and correlated requests
/// against loopback tokio-tungstenite servers on port 0.
const CREDENTIAL_WS_CLIENT_TEST: &str = r##"
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use schematic_schema::schematic_runtime::{AuthMethod, Credential};
use schematic_schema::ws_basic_fixture::BasicFixtureWs;
use schematic_schema::ws_bearer_client_fixture::BearerFixtureClientWs;
use schematic_schema::ws_cookie_key_fixture::CookieKeyFixtureWs;
use schematic_schema::ws_credential_payloads::{Answer, Ask, Count, Notice, Total};
use schematic_schema::ws_header_key_fixture::{HeaderKeyFixtureWs, RpcClient, RpcServerMessage};
use schematic_schema::ws_query_key_fixture::QueryKeyFixtureWs;
use schematic_schema::ws_shared::{ReconnectPolicy, WsClientOptions, WsError};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;

type Peer = WebSocketStream<TcpStream>;

/// A bound on a hang, never a synchronization.
const WAIT: Duration = Duration::from_secs(2);

/// How long a listener stays unconnected to show that nothing dialed it.
const QUIET: Duration = Duration::from_millis(300);

async fn within<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(WAIT, future).await.expect("timed out")
}

/// One upgrade request as the server saw it.
struct Seen {
    target: String,
    headers: http::HeaderMap,
}

impl Seen {
    fn values(&self, name: &str) -> Vec<&str> {
        self.headers.get_all(name).iter().map(|value| value.to_str().unwrap()).collect()
    }

    /// Every place a credential could travel: each Authorization,
    /// X-Fixture-Key, and Cookie header, and the query string.
    fn carriers(&self) -> Vec<String> {
        let mut found: Vec<String> = ["authorization", "x-fixture-key", "cookie"]
            .iter()
            .flat_map(|name| self.values(name).into_iter().map(move |value| format!("{name}: {value}")))
            .collect();
        if let Some((_, query)) = self.target.split_once('?') {
            found.push(format!("query: {query}"));
        }
        found
    }
}

async fn accept_one(listener: TcpListener) -> (Seen, Peer) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut seen = None;
    let peer = tokio_tungstenite::accept_hdr_async(stream, |request: &Request, response: Response| {
        seen = Some(Seen { target: request.uri().to_string(), headers: request.headers().clone() });
        Ok(response)
    })
    .await
    .unwrap();
    (seen.unwrap(), peer)
}

/// A loopback base URL and the server side of the one upgrade it accepts.
async fn listen() -> (String, tokio::task::JoinHandle<(Seen, Peer)>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (url, tokio::spawn(accept_one(listener)))
}

/// Answers every upgrade with `status` and `headers`; resolves to how many
/// upgrades arrived once none has for [`QUIET`].
async fn reject_all(listener: TcpListener, status: u16, headers: Vec<(&'static str, String)>) -> usize {
    let mut upgrades = 0;
    let mut wait = WAIT;
    while let Ok(accepted) = tokio::time::timeout(wait, listener.accept()).await {
        wait = QUIET;
        upgrades += 1;
        let (stream, _) = accepted.unwrap();
        let headers = headers.clone();
        let _ = tokio_tungstenite::accept_hdr_async(stream, move |_: &Request, _: Response| {
            let mut answer = http::Response::builder().status(status);
            for (name, value) in &headers {
                answer = answer.header(*name, value.as_str());
            }
            Err::<Response, ErrorResponse>(answer.body(None).unwrap())
        })
        .await;
    }
    upgrades
}

#[tokio::test]
async fn a_per_call_credential_wins_over_the_client_and_the_environment() {
    let (url, server) = listen().await;
    let options = WsClientOptions::builder().credential(Credential::api_key("call-key")).build();
    let api = HeaderKeyFixtureWs::with_base_url(url).api_key("client-key");
    let client = within(api.connect_rpc(options)).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.carriers(), ["x-fixture-key: call-key"]);
    assert_eq!(seen.values("x-client"), ["fixture"]);
    assert_eq!(client.auth_method(), &AuthMethod::PerCall);
}

#[tokio::test]
async fn a_client_credential_wins_over_the_environment() {
    let (url, server) = listen().await;
    let api = HeaderKeyFixtureWs::with_base_url(url).api_key("client-key");
    let client = within(api.connect_rpc(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.carriers(), ["x-fixture-key: client-key"]);
    assert_eq!(client.auth_method(), &AuthMethod::ClientLevel);
}

#[tokio::test]
async fn the_first_non_empty_env_variable_supplies_the_header_key() {
    let (url, server) = listen().await;
    let client = within(HeaderKeyFixtureWs::with_base_url(url).connect_rpc(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    // HEADER_KEY_FIXTURE_WS_EMPTY comes first and is set to "".
    assert_eq!(seen.carriers(), ["x-fixture-key: env-key"]);
    assert_eq!(seen.values("x-client"), ["fixture"]);
    assert_eq!(client.auth_method(), &AuthMethod::EnvVar("HEADER_KEY_FIXTURE_WS_KEY".to_string()));
}

#[tokio::test]
async fn an_env_query_key_is_one_query_pair_and_nothing_else() {
    let (url, server) = listen().await;
    let client = within(QueryKeyFixtureWs::with_base_url(url).connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.target, "/feed?access_key=query-env-key");
    assert_eq!(seen.carriers(), ["query: access_key=query-env-key"]);
    assert_eq!(client.auth_method(), &AuthMethod::EnvVar("QUERY_KEY_FIXTURE_WS_KEY".to_string()));
}

#[tokio::test]
async fn a_cookie_key_merges_into_an_existing_cookie_header_and_never_duplicates_its_name() {
    let (url, server) = listen().await;
    let api = CookieKeyFixtureWs::with_base_url(url.clone()).api_key("cookie-key").header("Cookie", "theme=dark");
    let client = within(api.connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.carriers(), ["cookie: theme=dark; session=cookie-key"]);
    assert_eq!(client.auth_method(), &AuthMethod::ClientLevel);

    let taken = CookieKeyFixtureWs::with_base_url(url).api_key("cookie-key").header("Cookie", "session=mine");
    match within(taken.connect_feed(WsClientOptions::default())).await.err() {
        Some(WsError::InvalidCredential(reason)) => assert!(!reason.contains("cookie-key"), "{reason}"),
        other => panic!("expected InvalidCredential, got {other:?}"),
    }
}

#[tokio::test]
async fn basic_credentials_from_the_env_lists_are_one_authorization_header() {
    let (url, server) = listen().await;
    let client = within(BasicFixtureWs::with_base_url(url).connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    // base64("env-user:env-pass")
    assert_eq!(seen.carriers(), ["authorization: Basic ZW52LXVzZXI6ZW52LXBhc3M="]);
    assert!(matches!(client.auth_method(), AuthMethod::EnvVar(_)), "{:?}", client.auth_method());
}

#[tokio::test]
async fn a_client_basic_credential_wins_over_the_env_lists() {
    let (url, server) = listen().await;
    let api = BasicFixtureWs::with_base_url(url).credentials(Credential::basic("client-user", "client-pass"));
    let client = within(api.connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    // base64("client-user:client-pass")
    assert_eq!(seen.carriers(), ["authorization: Basic Y2xpZW50LXVzZXI6Y2xpZW50LXBhc3M="]);
    assert_eq!(client.auth_method(), &AuthMethod::ClientLevel);
}

#[tokio::test]
async fn a_bearer_key_is_sent_as_an_access_token() {
    let (url, server) = listen().await;
    let api = BearerFixtureClientWs::with_base_url(url).api_key("bearer-token");
    let client = within(api.connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.carriers(), ["authorization: Bearer bearer-token"]);
    assert_eq!(client.auth_method(), &AuthMethod::ClientLevel);
}

#[tokio::test]
async fn nothing_resolved_connects_anonymously_and_reports_it() {
    let (url, server) = listen().await;
    let client = within(BearerFixtureClientWs::with_base_url(url).connect_feed(WsClientOptions::default())).await.unwrap();
    let (seen, _peer) = within(server).await.unwrap();
    assert_eq!(seen.target, "/feed");
    assert_eq!(seen.carriers(), Vec::<String>::new());
    assert_eq!(client.auth_method(), &AuthMethod::Anonymous);
}

#[tokio::test]
async fn a_401_upgrade_is_rejected_once_and_never_retried() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(reject_all(listener, 401, vec![("WWW-Authenticate", "Bearer".to_string())]));
    let options = WsClientOptions::builder()
        .credential(Credential::access_token("rejected-token"))
        .reconnect(Some(ReconnectPolicy {
            initial_backoff: Duration::from_millis(10),
            max_backoff: Duration::from_millis(20),
            ..ReconnectPolicy::default()
        }))
        .build();
    let error = within(BearerFixtureClientWs::with_base_url(url).connect_feed(options)).await.err().expect("rejected");
    assert!(matches!(error, WsError::HandshakeRejected { status: 401, .. }), "{error:?}");
    assert!(error.is_terminal_handshake_rejection());
    let rendered = format!("{error} {error:?}");
    assert!(!rendered.contains("rejected-token"), "{rendered}");
    assert_eq!(within(server).await.unwrap(), 1, "the server received a second upgrade");
}

#[tokio::test]
async fn a_302_upgrade_is_not_followed_and_its_location_is_redacted() {
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = target.local_addr().unwrap().to_string();
    let redirecting = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", redirecting.local_addr().unwrap());
    let location = format!("ws://{target_addr}/feed?access_key=leaked-key");
    let server = tokio::spawn(reject_all(redirecting, 302, vec![("Location", location)]));
    let options = WsClientOptions::builder().credential(Credential::api_key("call-key")).build();

    let error = within(QueryKeyFixtureWs::with_base_url(url).connect_feed(options)).await.err().expect("redirected");
    assert!(matches!(error, WsError::HandshakeRejected { status: 302, .. }), "{error:?}");
    let shown = error.location().expect("the redacted Location is kept");
    assert!(shown.contains(&target_addr), "{shown}");
    assert!(!shown.contains("leaked-key"), "{shown}");
    let rendered = format!("{error} {error:?}");
    assert!(!rendered.contains("leaked-key") && !rendered.contains("call-key"), "{rendered}");
    assert_eq!(within(server).await.unwrap(), 1, "the redirecting server received a second upgrade");
    assert!(tokio::time::timeout(QUIET, target.accept()).await.is_err(), "the redirect target was dialed");
}

async fn connect_rpc(options: WsClientOptions) -> (RpcClient, Peer) {
    let (url, server) = listen().await;
    let client = within(HeaderKeyFixtureWs::with_base_url(url).connect_rpc(options)).await.unwrap();
    let (_, peer) = within(server).await.unwrap();
    (client, peer)
}

/// The next text frame the peer receives, as JSON.
async fn received(peer: &mut Peer) -> Value {
    loop {
        match within(peer.next()).await.expect("the connection is open").unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            _ => continue,
        }
    }
}

async fn reply(peer: &mut Peer, frame: Value) {
    peer.send(Message::Text(frame.to_string().into())).await.unwrap();
}

async fn next_rpc(client: &mut RpcClient) -> RpcServerMessage {
    within(client.next_event()).await.expect("the connection is open").expect("the frame decodes")
}

#[tokio::test]
async fn string_request_ids_go_out_as_json_strings_and_return_the_declared_response() {
    let (mut client, mut peer) = connect_rpc(WsClientOptions::default()).await;
    let server = async {
        assert_eq!(received(&mut peer).await, json!({"question": "life", "id": "1"}));
        // The number 1 is not the string id "1".
        reply(&mut peer, json!({"kind": "answer", "re": 1, "text": "number id"})).await;
        reply(&mut peer, json!({"kind": "answer", "re": "1", "text": "forty-two"})).await;
        assert_eq!(received(&mut peer).await, json!({"n": 3, "id": "2"}));
        reply(&mut peer, json!({"kind": "total", "re": "2", "sum": 6})).await;
    };
    let requests = async {
        let answer = client.request_ask(Ask { question: "life".to_string() }).await.unwrap();
        let total = client.request_count(Count { n: 3 }).await.unwrap();
        (answer, total)
    };
    let ((answer, total), ()) = within(async { tokio::join!(requests, server) }).await;
    assert_eq!(answer, Answer { text: "forty-two".to_string() });
    assert_eq!(total, Total { sum: 6 });
    match next_rpc(&mut client).await {
        RpcServerMessage::Answer(answer) => assert_eq!(answer, Answer { text: "number id".to_string() }),
        other => panic!("expected the number-id Answer as an event, got {other:?}"),
    }
}

#[tokio::test]
async fn a_same_id_frame_of_another_message_is_an_event_not_the_answer() {
    let (mut client, mut peer) = connect_rpc(WsClientOptions::default()).await;
    let server = async {
        assert_eq!(received(&mut peer).await["id"], json!("1"));
        reply(&mut peer, json!({"kind": "notice", "re": "1", "text": "not the answer"})).await;
        reply(&mut peer, json!({"kind": "total", "re": "1", "sum": 7})).await;
        reply(&mut peer, json!({"kind": "answer", "re": "1", "text": "the answer"})).await;
    };
    let (answer, ()) = within(async { tokio::join!(client.request_ask(Ask { question: "q".to_string() }), server) }).await;
    assert_eq!(answer.unwrap(), Answer { text: "the answer".to_string() });
    match next_rpc(&mut client).await {
        RpcServerMessage::Notice(notice) => assert_eq!(notice, Notice { text: "not the answer".to_string() }),
        other => panic!("expected Notice, got {other:?}"),
    }
    match next_rpc(&mut client).await {
        RpcServerMessage::Total(total) => assert_eq!(total, Total { sum: 7 }),
        other => panic!("expected Total, got {other:?}"),
    }
}

/// Answers the next request the peer receives, echoing its id.
async fn answer_next(peer: &mut Peer, text: &str) {
    let request = received(peer).await;
    reply(peer, json!({"kind": "answer", "re": request["id"], "text": text})).await;
}

#[tokio::test]
async fn a_timed_out_request_frees_its_pending_slot() {
    let options = WsClientOptions::builder()
        .max_pending_requests(1)
        .request_timeout(Duration::from_millis(200))
        .build();
    let (client, mut peer) = connect_rpc(options).await;
    let mut first = Box::pin(client.request_ask(Ask { question: "unanswered".to_string() }));
    tokio::select! {
        result = &mut first => panic!("answered without a reply: {result:?}"),
        request = received(&mut peer) => assert_eq!(request["question"], json!("unanswered")),
    }
    match within(client.request_count(Count { n: 1 })).await {
        Err(WsError::BackpressureFull(1)) => {}
        other => panic!("expected BackpressureFull(1), got {other:?}"),
    }
    assert!(matches!(within(first).await, Err(WsError::RequestTimeout(_))));
    // A request that is not a JSON object is refused before it is sent.
    assert!(matches!(within(client.request_value(json!(5))).await, Err(WsError::Protocol(_))));

    let (answer, ()) = within(async {
        tokio::join!(client.request_ask(Ask { question: "again".to_string() }), answer_next(&mut peer, "freed"))
    })
    .await;
    assert_eq!(answer.unwrap(), Answer { text: "freed".to_string() });
}

#[tokio::test]
async fn a_cancelled_request_frees_its_pending_slot() {
    let options = WsClientOptions::builder()
        .max_pending_requests(1)
        .request_timeout(Duration::from_secs(30))
        .build();
    let (client, mut peer) = connect_rpc(options).await;
    let mut first = Box::pin(client.request_ask(Ask { question: "cancelled".to_string() }));
    tokio::select! {
        result = &mut first => panic!("answered without a reply: {result:?}"),
        request = received(&mut peer) => assert_eq!(request["question"], json!("cancelled")),
    }
    match within(client.request_count(Count { n: 1 })).await {
        Err(WsError::BackpressureFull(1)) => {}
        other => panic!("expected BackpressureFull(1), got {other:?}"),
    }
    drop(first);

    let (answer, ()) = within(async {
        tokio::join!(client.request_ask(Ask { question: "again".to_string() }), answer_next(&mut peer, "freed"))
    })
    .await;
    assert_eq!(answer.unwrap(), Answer { text: "freed".to_string() });
}
"##;

/// The child tests [`CREDENTIAL_WS_CLIENT_TEST`] must report.
const CREDENTIAL_WS_CLIENT_TESTS: &[&str] = &[
    "a_per_call_credential_wins_over_the_client_and_the_environment",
    "a_client_credential_wins_over_the_environment",
    "the_first_non_empty_env_variable_supplies_the_header_key",
    "an_env_query_key_is_one_query_pair_and_nothing_else",
    "a_cookie_key_merges_into_an_existing_cookie_header_and_never_duplicates_its_name",
    "basic_credentials_from_the_env_lists_are_one_authorization_header",
    "a_client_basic_credential_wins_over_the_env_lists",
    "a_bearer_key_is_sent_as_an_access_token",
    "nothing_resolved_connects_anonymously_and_reports_it",
    "a_401_upgrade_is_rejected_once_and_never_retried",
    "a_302_upgrade_is_not_followed_and_its_location_is_redacted",
    "string_request_ids_go_out_as_json_strings_and_return_the_declared_response",
    "a_same_id_frame_of_another_message_is_an_event_not_the_answer",
    "a_timed_out_request_frees_its_pending_slot",
    "a_cancelled_request_frees_its_pending_slot",
];

/// R5, R6, R7, and R11 in one fresh crate: handshake credential placement
/// for each strategy and its precedence, anonymous connects, rejected and
/// redirected upgrades, and typed correlated requests.
#[test]
fn slow_ws_handshake_credentials_and_correlated_requests_run() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let render = |api: schematic_define::WebSocketApi| {
        schematic_gen::output::ws_modules::render_ws_api_module(&api)
            .unwrap_or_else(|error| panic!("{} lowers: {error}", api.name))
    };
    let header_key = render(header_key_ws_fixture());
    let query_key = render(feed_ws_fixture(
        "QueryKeyFixtureWs",
        AuthStrategy::ApiKeyParam { name: "access_key".to_string(), location: ApiKeyLocation::Query },
        &["QUERY_KEY_FIXTURE_WS_KEY"],
    ));
    let cookie_key = render(feed_ws_fixture(
        "CookieKeyFixtureWs",
        AuthStrategy::ApiKeyParam { name: "session".to_string(), location: ApiKeyLocation::Cookie },
        &["COOKIE_KEY_FIXTURE_WS_SESSION"],
    ));
    let basic = render(feed_ws_fixture(
        "BasicFixtureWs",
        AuthStrategy::Basic {
            username_env: EnvList::single("BASIC_FIXTURE_WS_USER"),
            password_env: EnvList::single("BASIC_FIXTURE_WS_PASS"),
        },
        &[],
    ));
    let bearer = render(feed_ws_fixture(
        "BearerFixtureClientWs",
        AuthStrategy::BearerToken { header: None },
        &["BEARER_FIXTURE_CLIENT_WS_TOKEN"],
    ));
    for (name, module) in [("header key", &header_key), ("query key", &query_key), ("cookie key", &cookie_key), ("bearer", &bearer)] {
        assert!(module.contains("pub fn api_key("), "the {name} client takes a single-secret key:\n{module}");
    }
    assert!(!basic.contains("pub fn api_key("), "a Basic-only client has no single-secret key:\n{basic}");
    assert!(header_key.contains("pub async fn request_ask("), "{header_key}");
    assert!(header_key.contains("pub async fn request_count("), "{header_key}");

    let fresh = FreshCrate::generate(&[], Definitions::Checkout, &lockfile);
    fresh.add_module("ws_credential_payloads", WS_CREDENTIAL_PAYLOADS);
    fresh.add_module("ws_header_key_fixture", &header_key);
    fresh.add_module("ws_query_key_fixture", &query_key);
    fresh.add_module("ws_cookie_key_fixture", &cookie_key);
    fresh.add_module("ws_basic_fixture", &basic);
    fresh.add_module("ws_bearer_client_fixture", &bearer);
    fresh.add_test_file("credential_ws_client", CREDENTIAL_WS_CLIENT_TEST);

    let run = fresh.nextest_run(CREDENTIAL_WS_ENV);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in CREDENTIAL_WS_CLIENT_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}

/// The opening payload of `SessionFixtureWs`, added to the fresh crate as
/// `src/ws_session_payloads.rs`. Its key field is optional and renamed, so
/// the bound wire field is `api-key`.
const WS_SESSION_PAYLOADS: &str = r#"
#[derive(Debug, serde::Serialize)]
pub struct Hello {
    pub mode: String,
    #[serde(rename = "api-key", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}
"#;

/// An API key header and one `Feed` endpoint whose opening `Hello` binds
/// its `api-key` field to the handshake credential and whose auth flow
/// hints generate a setup connect.
fn session_ws_fixture() -> schematic_define::WebSocketApi {
    use schematic_define::Schema;
    use schematic_define::websocket::{
        AuthFlowHints, ConnectionLifecycle, EndpointRole, MessageDirection, MessageSchema,
        WebSocketEndpointHints,
    };
    let mut feed = ws_fixture_endpoint("Feed", "/feed", EndpointRole::Client, vec![MessageSchema::new(
        "Frame",
        MessageDirection::Server,
        Schema::new("serde_json::Value"),
    )]);
    feed.lifecycle = ConnectionLifecycle {
        open: Some(MessageSchema::new(
            "Hello",
            MessageDirection::Client,
            Schema::with_path("Hello", "crate::ws_session_payloads"),
        )),
        close: None,
        keepalive: None,
        open_credential_field: Some("api-key".into()),
    };
    feed.runtime = Some(WebSocketEndpointHints {
        auth_flow: Some(AuthFlowHints {
            challenge_message: "Frame".to_string(),
            auth_request_schema: "serde_json::Value".to_string(),
            success_indicator: None,
            failure_indicator: None,
        }),
        ..WebSocketEndpointHints::default()
    });
    ws_fixture_api(
        "SessionFixtureWs",
        AuthStrategy::ApiKey { header: "X-Session-Key".to_string(), value_prefix: None },
        &[],
        vec![feed],
    )
}

/// Child tests of a generated client's session setup and recovery: the
/// opening message, the setup callback, the reconnect budget, and the
/// receive timeout, against loopback tokio-tungstenite servers on port 0.
const SESSION_WS_CLIENT_TEST: &str = r##"
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use schematic_schema::ws_session_fixture::{FeedClient, FeedSetup, SessionFixtureWs};
use schematic_schema::ws_session_payloads::Hello;
use schematic_schema::ws_shared::{
    ReconnectPolicy, WsClientOptions, WsConnectionState, WsError, WsSetupError,
};
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http;

type Peer = WebSocketStream<TcpStream>;

/// A bound on a hang, never a synchronization.
const WAIT: Duration = Duration::from_secs(5);

/// How long a listener stays unconnected to show that nothing dialed it.
const QUIET: Duration = Duration::from_millis(300);

const KEY: &str = "session-key";

async fn within<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(WAIT, future).await.expect("timed out")
}

fn hello() -> Hello {
    Hello { mode: "fast".to_string(), api_key: None }
}

/// A loopback listener and a client of it holding [`KEY`].
async fn bind() -> (TcpListener, SessionFixtureWs) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    (listener, SessionFixtureWs::with_base_url(url).api_key(KEY))
}

fn options(max_attempts: Option<u32>) -> WsClientOptions {
    WsClientOptions::builder()
        .reconnect(Some(ReconnectPolicy {
            initial_backoff: Duration::from_millis(20),
            max_backoff: Duration::from_millis(20),
            multiplier: 1.0,
            jitter: 0.0,
            max_attempts,
        }))
        .build()
}

/// A setup callback that only counts its runs.
fn counting(runs: &Arc<AtomicUsize>) -> impl Fn(FeedSetup) -> std::future::Ready<Result<(), WsSetupError>> + Send + Sync + 'static {
    let runs = Arc::clone(runs);
    move |_session| {
        runs.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(()))
    }
}

async fn received(peer: &mut Peer) -> Value {
    loop {
        match within(peer.next()).await.expect("the connection is open").unwrap() {
            Message::Text(text) => return serde_json::from_str(text.as_str()).unwrap(),
            _ => continue,
        }
    }
}

async fn send(peer: &mut Peer, frame: Value) {
    peer.send(Message::Text(frame.to_string().into())).await.unwrap();
}

/// Accepts the next upgrade and checks its key header and that its first
/// frame is the opening message with the key filled in.
async fn accept_opened(listener: &TcpListener) -> Peer {
    let (stream, _) = within(listener.accept()).await.unwrap();
    let mut key = None;
    let mut peer = within(tokio_tungstenite::accept_hdr_async(stream, |request: &Request, response: Response| {
        key = request.headers().get("x-session-key").map(|value| value.to_str().unwrap().to_string());
        Ok(response)
    }))
    .await
    .unwrap();
    assert_eq!(key.as_deref(), Some(KEY));
    assert_eq!(received(&mut peer).await, json!({"mode": "fast", "api-key": KEY}));
    peer
}

/// Answers the next upgrade with 503.
async fn refuse(listener: &TcpListener) {
    let (stream, _) = within(listener.accept()).await.unwrap();
    let _ = tokio_tungstenite::accept_hdr_async(stream, |_: &Request, _: Response| {
        Err::<Response, ErrorResponse>(http::Response::builder().status(503).body(None).unwrap())
    })
    .await;
}

/// Closes the connection from the server, which the client redials. A
/// Close frame loses the connection without an error event.
async fn drop_connection(mut peer: Peer) {
    let _ = peer.close(None).await;
}

async fn next_value(client: &mut FeedClient) -> Value {
    within(client.next_value_event()).await.expect("the connection is open").expect("a data frame")
}

async fn assert_not_dialed(listener: &TcpListener) {
    assert!(tokio::time::timeout(QUIET, listener.accept()).await.is_err(), "the listener was dialed");
}

#[tokio::test]
async fn the_opening_frame_comes_first_on_the_initial_connection_and_on_a_redial() {
    let (listener, api) = bind().await;
    let (client, peer) = tokio::join!(within(api.connect_feed_with_setup(hello(), options(None), |_| async { Ok(()) })), accept_opened(&listener));
    let mut client = client.unwrap();
    drop_connection(peer).await;
    let mut peer = accept_opened(&listener).await;
    send(&mut peer, json!({"after": "redial"})).await;
    assert_eq!(next_value(&mut client).await, json!({"after": "redial"}));
    within(client.close()).await.unwrap();
    assert_eq!(client.state(), WsConnectionState::Closed);
}

#[tokio::test]
async fn an_opening_key_that_differs_from_the_credential_fails_the_connect_before_dialing() {
    let (listener, api) = bind().await;
    let opening = Hello { mode: "fast".to_string(), api_key: Some("other-key".to_string()) };
    match within(api.connect_feed_with_setup(opening, options(None), |_| async { Ok(()) })).await.err() {
        Some(WsError::InvalidCredential(reason)) => {
            assert!(!reason.contains("other-key") && !reason.contains(KEY), "{reason}");
        }
        other => panic!("expected InvalidCredential, got {other:?}"),
    }
    assert_not_dialed(&listener).await;
}

#[tokio::test]
async fn setup_runs_with_the_handshake_key_before_ready_on_the_initial_connection_and_on_a_redial() {
    let (listener, api) = bind().await;
    let runs = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&runs);
    let setup = move |mut session: FeedSetup| {
        let runs = Arc::clone(&counted);
        async move {
            let key = session.credential().expose_secret().map(str::to_string);
            session.send_value(json!({"setup": key})).await?;
            if session.next_value().await? != json!({"ack": true}) {
                return Err(WsSetupError::Transient("no ack".to_string()));
            }
            runs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    };
    let server = async {
        let mut peer = accept_opened(&listener).await;
        assert_eq!(received(&mut peer).await, json!({"setup": KEY}));
        send(&mut peer, json!({"ack": true})).await;
        peer
    };
    let (client, peer) = tokio::join!(within(api.connect_feed_with_setup(hello(), options(None), setup)), server);
    let mut client = client.unwrap();
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(client.state(), WsConnectionState::Ready);

    drop_connection(peer).await;
    let mut peer = accept_opened(&listener).await;
    assert_eq!(received(&mut peer).await, json!({"setup": KEY}));
    // The redial's setup is waiting for its ack.
    assert_eq!(client.state(), WsConnectionState::Authenticating);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    send(&mut peer, json!({"ack": true})).await;
    send(&mut peer, json!({"after": "setup"})).await;
    assert_eq!(next_value(&mut client).await, json!({"after": "setup"}));
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert_eq!(client.state(), WsConnectionState::Ready);
    within(client.close()).await.unwrap();
}

#[tokio::test]
async fn a_rejected_setup_fails_the_connect_with_setup_rejected_after_one_connection() {
    let (listener, api) = bind().await;
    let setup = |_session: FeedSetup| async { Err::<(), _>(WsSetupError::Rejected("denied".to_string())) };
    let (result, _peer) =
        tokio::join!(within(api.connect_feed_with_setup(hello(), options(None), setup)), accept_opened(&listener));
    match result.err() {
        Some(WsError::SetupRejected(reason)) => assert_eq!(reason, "denied"),
        other => panic!("expected SetupRejected, got {other:?}"),
    }
    assert_not_dialed(&listener).await;
}

#[tokio::test]
async fn the_reconnect_budget_counts_consecutive_failed_redials_and_resets_after_each_setup() {
    let (listener, api) = bind().await;
    let runs = Arc::new(AtomicUsize::new(0));
    let (client, mut peer) = tokio::join!(
        within(api.connect_feed_with_setup(hello(), options(Some(1)), counting(&runs))),
        accept_opened(&listener)
    );
    let mut client = client.unwrap();
    // Three redials in a row, each completing setup, within a budget of one.
    for redial in 1..=3 {
        drop_connection(peer).await;
        peer = accept_opened(&listener).await;
        send(&mut peer, json!({"redial": redial})).await;
        assert_eq!(next_value(&mut client).await, json!({"redial": redial}));
        assert_eq!(client.state(), WsConnectionState::Ready);
        assert_eq!(runs.load(Ordering::SeqCst), redial + 1);
    }

    drop_connection(peer).await;
    refuse(&listener).await;
    match within(client.next_value_event()).await {
        Some(Err(WsError::HandshakeRejected { status: 503, .. })) => {}
        other => panic!("expected the refused redial, got {other:?}"),
    }
    assert!(within(client.next_value_event()).await.is_none(), "the event stream ends");
    assert_eq!(client.state(), WsConnectionState::Closed);
    assert_eq!(runs.load(Ordering::SeqCst), 4);
    assert_not_dialed(&listener).await;
}

#[tokio::test]
async fn a_silent_server_times_out_the_receive_and_the_client_redials() {
    let (listener, api) = bind().await;
    let runs = Arc::new(AtomicUsize::new(0));
    let limit = Duration::from_millis(300);
    let options = WsClientOptions { receive_timeout: Some(limit), ..options(None) };
    let (client, _silent) = tokio::join!(
        within(api.connect_feed_with_setup(hello(), options, counting(&runs))),
        accept_opened(&listener)
    );
    let mut client = client.unwrap();
    match within(client.next_value_event()).await {
        Some(Err(WsError::ReceiveTimeout(after))) => assert_eq!(after, limit),
        other => panic!("expected ReceiveTimeout, got {other:?}"),
    }
    let mut peer = accept_opened(&listener).await;
    send(&mut peer, json!({"after": "timeout"})).await;
    assert_eq!(next_value(&mut client).await, json!({"after": "timeout"}));
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    assert_eq!(client.state(), WsConnectionState::Ready);
    within(client.close()).await.unwrap();
}
"##;

/// The child tests [`SESSION_WS_CLIENT_TEST`] must report.
const SESSION_WS_CLIENT_TESTS: &[&str] = &[
    "the_opening_frame_comes_first_on_the_initial_connection_and_on_a_redial",
    "an_opening_key_that_differs_from_the_credential_fails_the_connect_before_dialing",
    "setup_runs_with_the_handshake_key_before_ready_on_the_initial_connection_and_on_a_redial",
    "a_rejected_setup_fails_the_connect_with_setup_rejected_after_one_connection",
    "the_reconnect_budget_counts_consecutive_failed_redials_and_resets_after_each_setup",
    "a_silent_server_times_out_the_receive_and_the_client_redials",
];

/// Session setup and recovery through a generated client: the opening
/// message bound to the handshake key, the setup callback on every
/// connection, the consecutive-failure reconnect budget, and the receive
/// timeout.
#[test]
fn slow_ws_session_setup_and_recovery_run() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let session = schematic_gen::output::ws_modules::render_ws_api_module(&session_ws_fixture())
        .unwrap_or_else(|error| panic!("SessionFixtureWs lowers: {error}"));
    assert!(session.contains("pub async fn connect_feed_with_setup<"), "{session}");
    assert!(!session.contains("pub async fn connect_feed("), "a challenge endpoint has no setup-less connect: {session}");
    assert!(session.contains("pub type FeedSetup"), "{session}");

    let fresh = FreshCrate::generate(&[], Definitions::Checkout, &lockfile);
    fresh.add_module("ws_session_payloads", WS_SESSION_PAYLOADS);
    fresh.add_module("ws_session_fixture", &session);
    fresh.add_test_file("session_ws_client", SESSION_WS_CLIENT_TEST);

    let run = fresh.nextest_run(&[]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in SESSION_WS_CLIENT_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}
