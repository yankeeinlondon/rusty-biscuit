//! Typed path parameters, deprecation, and metadata in generated Rust.
//!
//! One `slow_` test generates a fresh client for a fixture API whose
//! endpoints declare integer, boolean, number, enum, reserved, and repeated
//! typed path placeholders, builds it offline through
//! [`crate::common::fresh_output`], runs child tests that send to a local
//! wiremock server and assert the exact path it received, and then proves
//! at compile time that a typed-path request has no `Default` while a legacy
//! all-optional request keeps it. `slow_` because it invokes Cargo
//! (`just test-fresh-output`).

use schematic_define::{
    AuthStrategy, DefinitionContext, Endpoint, ParamDef, ParamStyle, QueryParamType, RestApi,
};
use schematic_gen::validation::validate_rust_client;

use crate::common::fresh_output::{Definitions, FreshCrate, offline_cache_hint};

const MODULE: &str = "fixture_typed_paths";

/// `GetTyped` declares one path parameter of each scalar kind and an enum,
/// in an order that differs from the template; `GetFile` has a reserved and
/// an ordinary string; `CopyItem` repeats one placeholder; `ListAll` keeps
/// legacy all-optional fields.
fn fixture_api() -> RestApi {
    let ctx = DefinitionContext::types_in(format!("::schematic_definitions::{MODULE}"));
    let typed = Endpoint::get("GetTyped", "/items/{id}/flags/{flag}/ratios/{ratio}/kinds/{kind}")
        .describes("Every typed path kind")
        .returns_empty()
        .path_param(ParamDef::path("kind", QueryParamType::Enum(vec!["first-kind".into(), "second".into()])))
        .path_param(ParamDef::path("ratio", QueryParamType::Number).deprecated())
        .path_param(ParamDef::path("flag", QueryParamType::Boolean))
        .path_param(ParamDef::path("id", QueryParamType::Integer).describes("The item id."))
        .deprecated()
        .build()
        .expect("typed endpoint");
    let file = Endpoint::get("GetFile", "/files/{+file_path}/raw/{name}")
        .describes("A reserved and an ordinary string")
        .returns_empty()
        .path_param(ParamDef::path("file_path", QueryParamType::String))
        .path_param(ParamDef::path("name", QueryParamType::String))
        .build()
        .expect("reserved endpoint");
    let copy = Endpoint::get("CopyItem", "/items/{id}/copies/{id}")
        .describes("A repeated placeholder")
        .returns_empty()
        .path_param(ParamDef::path("id", QueryParamType::Integer))
        .build()
        .expect("repeated endpoint");
    let list = Endpoint::get("ListAll", "/all")
        .describes("Legacy optional query")
        .returns_empty()
        .query(ParamDef {
            deprecated: false,
            rust_name: None,
            secret: false,
            pager_managed: false,
            name: "page".to_string(),
            required: false,
            description: None,
            param_type: QueryParamType::Integer,
            explode: false,
            style: ParamStyle::Form,
        })
        .build()
        .expect("legacy endpoint");
    RestApi::builder(&ctx, "FixtureTypedPaths", "http://127.0.0.1:1")
        .describes("Typed path fixture")
        .authentication(AuthStrategy::None, Vec::<String>::new())
        .module_path(MODULE)
        .endpoints([typed, file, copy, list])
        .build()
        .expect("fixture API")
}

#[test]
fn the_typed_path_fixture_passes_rust_validation() {
    let api = fixture_api();

    let diagnostics = validate_rust_client(&[&api]);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

/// The child tests [`CHILD_TEST`] must report as passed.
const CHILD_TESTS: &[&str] = &[
    "typed_values_reach_the_server_in_their_wire_spelling",
    "reserved_occurrences_stay_raw_and_ordinary_ones_are_encoded",
    "a_repeated_placeholder_sends_its_one_value_at_every_occurrence",
    "a_non_finite_number_fails_before_send_naming_the_parameter",
];

const CHILD_TEST: &str = r##"
use schematic_schema::fixture_typed_paths::{
    CopyItemRequest, FixtureTypedPaths, GetTypedRequest, GetTypedRequestKind,
};
use schematic_schema::schematic_runtime::TransportErrorKind;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn start_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(200)).mount(&server).await;
    server
}

async fn received_paths(server: &MockServer) -> Vec<String> {
    let requests = server.received_requests().await.expect("recording is on");
    requests.iter().map(|request| request.url.path().to_string()).collect()
}

#[tokio::test]
async fn typed_values_reach_the_server_in_their_wire_spelling() {
    let server = start_server().await;
    let client = FixtureTypedPaths::new().base_url(server.uri());

    client.send(GetTypedRequest::new(-42, true, 2.5, GetTypedRequestKind::FirstKind)).full().await.unwrap();
    client.get_typed(7, false, 0.125, GetTypedRequestKind::Second).await.unwrap();

    assert_eq!(
        received_paths(&server).await,
        ["/items/-42/flags/true/ratios/2.5/kinds/first-kind", "/items/7/flags/false/ratios/0.125/kinds/second"]
    );
    assert_eq!(GetTypedRequestKind::FirstKind.to_string(), "first-kind");
}

#[tokio::test]
async fn reserved_occurrences_stay_raw_and_ordinary_ones_are_encoded() {
    let server = start_server().await;
    let client = FixtureTypedPaths::new().base_url(server.uri());

    client.get_file("dir/sub/file.txt", "x/y").await.unwrap();

    assert_eq!(received_paths(&server).await, ["/files/dir/sub/file.txt/raw/x%2Fy"]);
}

#[tokio::test]
async fn a_repeated_placeholder_sends_its_one_value_at_every_occurrence() {
    let server = start_server().await;
    let client = FixtureTypedPaths::new().base_url(server.uri());

    client.send(CopyItemRequest::new(9)).full().await.unwrap();

    assert_eq!(received_paths(&server).await, ["/items/9/copies/9"]);
}

#[tokio::test]
async fn a_non_finite_number_fails_before_send_naming_the_parameter() {
    let server = start_server().await;
    let client = FixtureTypedPaths::new().base_url(server.uri());

    for ratio in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = client
            .send(GetTypedRequest::new(1, true, ratio, GetTypedRequestKind::Second))
            .full()
            .await
            .expect_err("a non-finite path number is refused");
        let TransportErrorKind::InvalidArgument(argument) = &error.kind else {
            panic!("expected an invalid argument, got {:?}", error.kind);
        };
        assert_eq!(argument.reason(), "path parameter `ratio` must be a finite number");
    }

    assert!(received_paths(&server).await.is_empty(), "nothing is sent");
}
"##;

/// What every compile-check file imports; the control proves these imports
/// and correct uses compile, so each failure is the misuse's own.
const COMPILE_PRELUDE: &str = "#![allow(dead_code, unused_imports, unreachable_code)]
use schematic_schema::fixture_typed_paths::{GetTypedRequest, GetTypedRequestKind, ListAllRequest};
";

const CONTROL: &str = "fn used() {
    let _: ListAllRequest = Default::default();
    let _ = ListAllRequest::default().with_page(2);
    let _ = GetTypedRequest::new(1, true, 1.5, GetTypedRequestKind::Second);
}
";

/// One misuse per entry: a description, the snippet, the compiler error code
/// it must fail with, and text its diagnostic must name.
const MISUSES: &[(&str, &str, &str, &str)] = &[
    (
        "a typed-path request has no Default",
        "fn misuse() { let _: GetTypedRequest = Default::default(); }",
        "E0277",
        "Default",
    ),
    (
        "a path enum is non-exhaustive outside its crate",
        "fn misuse(kind: GetTypedRequestKind) -> u8 {
            match kind { GetTypedRequestKind::FirstKind => 1, GetTypedRequestKind::Second => 2 }
        }",
        "E0004",
        "non-exhaustive",
    ),
];

#[test]
fn slow_typed_path_clients_send_wire_values_and_refuse_default() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let api = fixture_api();
    let fresh = FreshCrate::generate(&[&api], Definitions::Fixture(&[(MODULE, "")]), &lockfile);
    let generated = std::fs::read_to_string(fresh.src_dir.join(format!("{MODULE}.rs"))).expect("generated module");
    assert!(!generated.contains("#[deprecated"), "deprecation is documentation only");

    fresh.add_test_file("typed_paths", CHILD_TEST);
    let run = fresh.nextest_run(&[]);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in CHILD_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }

    fresh.add_test_file("compile_control", &format!("{COMPILE_PRELUDE}{CONTROL}"));
    for (index, (_, snippet, _, _)) in MISUSES.iter().enumerate() {
        fresh.add_test_file(&format!("misuse_{index}"), &format!("{COMPILE_PRELUDE}{snippet}\n"));
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
    assert!(
        located("compile_control").is_empty() && located("typed_paths").is_empty(),
        "the control and the child tests must compile{hint}:\n{}",
        check.stderr
    );
    for (index, (misuse, snippet, code, named)) in MISUSES.iter().enumerate() {
        let found = located(&format!("misuse_{index}"));
        assert!(
            found.iter().any(|diagnostic| diagnostic.starts_with(&format!("[{code}]")) && diagnostic.contains(named)),
            "{misuse} did not fail with {code} naming `{named}`{hint}:\n{snippet}\n\n{}",
            check.stderr
        );
    }
}
