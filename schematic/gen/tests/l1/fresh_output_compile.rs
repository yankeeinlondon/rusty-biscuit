//! The fresh-output guard: code the generator emits *now* must compile.
//!
//! Each `slow_` test generates a client crate into a temporary directory
//! outside the checkout and builds it offline through
//! [`crate::common::fresh_output`]. They are `slow_` because they invoke
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
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EndpointParams, FormField, QueryParamType,
    RestApi, RestMethod,
};
use schematic_gen::output::{RustOutputRequest, Selection, get_module_path, prepare_rust_output};
use schematic_gen::pipeline::resolve_all_apis;

use crate::common::fresh_output::{
    Definitions, FreshCrate, TreeSnapshot, diff_summary, list_dir, schematic_root, snapshot_tree,
};

/// The fixture APIs and the generator features each one covers. Text and
/// binary bodies, `ApiKeyParam`, and header and cookie parameters join this
/// list when the generator supports them.
const FIXTURE_COVERAGE: &[(&str, &[&str])] = &[
    ("FixtureBearer", &["bearer-auth", "json-body", "query-parameters", "array-query-parameter"]),
    ("FixtureApiKey", &["api-key-header-auth", "multipart-body", "url-encoded-body"]),
    ("FixtureBasic", &["basic-auth", "empty-response", "text-response", "binary-response"]),
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

    vec![
        fixture_api(
            "FixtureBearer",
            "fixture_bearer",
            AuthStrategy::BearerToken { header: None },
            vec![list_widgets, create_widget, get_widget],
        ),
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
            ],
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

#[test]
fn slow_fixture_apis_compile_offline() {
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
    for module in ["fixture_bearer.rs", "fixture_api_key.rs", "fixture_basic.rs"] {
        assert!(fresh.src_dir.join(module).is_file(), "missing generated {module}");
    }
    fresh.cargo_check().assert_success();

    before.assert_unchanged(&CheckoutWitness::capture(&area));
    eprintln!("compiled fixture APIs: {FIXTURE_COVERAGE:?}");
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
        "the compiler error should be shown:\n{}",
        run.stderr
    );
}

/// Request lines the pre-split generator put on the wire, captured by a raw
/// TCP listener so no HTTP library normalizes them. The query split must
/// leave every line byte-identical.
const WIRE_BASELINE: &str = include_str!("../fixtures/wire_baseline/query_request_lines.txt");

/// Sends representative requests through the generated real clients to a
/// local listener and records `label<TAB>request line` per request.
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
    fresh
        .nextest_run(&[("WIRE_BASELINE_OUT", out_path.to_str().unwrap())])
        .assert_success();

    let captured = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        captured, WIRE_BASELINE,
        "the generated clients put different request lines on the wire"
    );
}
