//! `load_data` options on HTTP(S) sources: an explicit `format` for an
//! extensionless URL, host policy before any warmed memo entry, the reference
//! check's deferral, and redirect policy.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure};
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{CacheAccessMode, ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use tempfile::TempDir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// A document whose only content is one `load_data` call on `url`.
fn doc_loading(dir: &Path, url: &str, options: &str) -> PathBuf {
    write(dir, "doc.md", &format!("{{{{ as_csv(load_data(\"{url}\", \"string[]\"{options})) }}}}\n"))
}

const JSON_FORMAT: &str = ", {format: \"json\"}";

/// Options anchored on `doc` (and so on its temp dir), with the transport
/// cache off so every request a test counts reaches the mock.
fn anchored(doc: &Path, allowed_host: &str) -> ComposeOptions {
    ComposeOptions::new()
        .with_source_file(doc)
        .with_cache_access_mode(CacheAccessMode::Off)
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec![allowed_host.into()], ..Default::default() })
}

fn allowing_loopback(doc: &Path) -> ComposeOptions {
    anchored(doc, "127.0.0.1")
}

/// A run that allows only `example.com`, so `127.0.0.1` is refused.
fn denying_loopback(doc: &Path) -> ComposeOptions {
    anchored(doc, "example.com")
}

async fn compose_blocking(doc: PathBuf, options: ComposeOptions) -> Composed {
    tokio::task::spawn_blocking(move || {
        let request = crate::request_support::request(options);
        Markdown::try_from(doc.as_path()).expect("document loads").compose_with_warnings(&request)
    })
    .await
    .unwrap()
}

fn text_of(composed: Composed) -> (String, ComposeReport) {
    match composed {
        Ok((markdown, report)) => (markdown.content().trim().to_string(), report),
        Err(failure) => panic!("composition failed: {}", failure.error),
    }
}

async fn requests(server: &MockServer) -> usize {
    server.received_requests().await.unwrap_or_default().len()
}

/// Serves a JSON array at the extensionless `GET /export?id=7`.
async fn export_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/export"))
        .and(query_param("id", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[\"macOS\", \"Linux\"]"))
        .mount(&server)
        .await;
    server
}

/// The kind of the file-reference failure in `failure`'s cause chain.
fn file_reference_kind(failure: &ComposeFailure) -> Option<FileRefFailure> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        let expression = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
        match expression.map(ExpressionError::without_callbacks) {
            Some(ExpressionError::FileReference(diagnostic)) => return Some(diagnostic.kind),
            Some(ExpressionError::Data(data)) => {
                if let DataError::FileReference(diagnostic) = &**data {
                    return Some(diagnostic.kind);
                }
            }
            _ => {}
        }
        current = error.source();
    }
    None
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extensionless_url_loads_with_json_format() {
    let server = export_server().await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", server.uri());
    let doc = doc_loading(dir.path(), &url, JSON_FORMAT);

    let (text, report) = text_of(compose_blocking(doc.clone(), allowing_loopback(&doc)).await);
    assert_eq!(text, "macOS, Linux");
    assert_eq!(report.read_set, vec![url], "the read-set entry keeps the query");
    assert_eq!(requests(&server).await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn extensionless_url_without_format_is_unsupported() {
    let server = export_server().await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", server.uri());
    let doc = doc_loading(dir.path(), &url, "");

    let failure = compose_blocking(doc.clone(), allowing_loopback(&doc)).await.unwrap_err();
    let codes: Vec<_> = error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect();
    assert_eq!(codes, vec!["dm.data.unsupported_format"], "{}", failure.error);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn denied_host_is_refused_even_with_a_warmed_memo() {
    let server = export_server().await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", server.uri());
    let doc = doc_loading(dir.path(), &url, JSON_FORMAT);
    let runtime = DataRuntime::new();

    let (warmed, _) = text_of(compose_blocking(doc.clone(), allowing_loopback(&doc).with_data_runtime(runtime.clone())).await);
    assert_eq!(warmed, "macOS, Linux");
    let warm_stats = runtime.stats();
    assert_eq!(warm_stats.reads, 1, "{warm_stats:?}");

    let failure =
        compose_blocking(doc.clone(), denying_loopback(&doc).with_data_runtime(runtime.clone())).await.unwrap_err();
    assert_eq!(file_reference_kind(&failure), Some(FileRefFailure::RemoteDenied), "{}", failure.error);
    assert!(!failure.error.to_string().contains("macOS"), "no body was returned: {}", failure.error);
    assert_eq!(requests(&server).await, 1, "the denied run made no request");
    let stats = runtime.stats();
    assert_eq!(
        (stats.reads, stats.source_hits, stats.bind_hits),
        (warm_stats.reads, warm_stats.source_hits, warm_stats.bind_hits),
        "the denied call reached no memo layer: {stats:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn allowed_host_with_a_warmed_memo_makes_no_second_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/export"))
        .and(query_param("id", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[\"macOS\", \"Linux\"]"))
        .expect(1)
        .mount(&server)
        .await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", server.uri());
    let doc = doc_loading(dir.path(), &url, JSON_FORMAT);
    let runtime = DataRuntime::new();
    let options = allowing_loopback(&doc).with_data_runtime(runtime.clone()).with_shared_remote_fetch();

    for run in ["warming", "warmed"] {
        let (text, _) = text_of(compose_blocking(doc.clone(), options.clone()).await);
        assert_eq!(text, "macOS, Linux", "{run}");
    }
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.binds), (1, 1, 1), "{stats:?}");
    server.verify().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reference_check_defers_and_composition_reads_later() {
    let server = export_server().await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", server.uri());
    let doc = doc_loading(dir.path(), &url, JSON_FORMAT);
    let runtime = DataRuntime::new();
    let base = ComposeOptions::new()
        .with_source_file(&doc)
        .with_cache_access_mode(CacheAccessMode::Off)
        .with_data_runtime(runtime.clone());

    let check_runtime = runtime.clone();
    let (text, check_stats, check_read_set) = tokio::task::spawn_blocking(move || {
        let md = Markdown::try_from(doc.as_path()).unwrap();
        // As in `md compose`, the check runs before host permission is wired.
        let check = crate::request_support::request(base.clone().for_reference_check());
        md.validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&check)))
            .unwrap_or_else(|error| panic!("{error}"));
        let check_stats = check_runtime.stats();
        let check_read_set = check_runtime.read_set();
        let allowed = base
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
            .with_shared_remote_fetch();
        let (composed, _) = md
            .compose_with_warnings(&crate::request_support::request(allowed))
            .unwrap_or_else(|failure| panic!("{}", failure.error));
        (composed.content().trim().to_string(), check_stats, check_read_set)
    })
    .await
    .unwrap();

    assert_eq!(check_read_set, vec![url], "the check saw the call");
    assert_eq!(check_stats.reads, 0, "the check deferred the read and memoized nothing: {check_stats:?}");
    assert_eq!(text, "macOS, Linux");
    assert_eq!(requests(&server).await, 1);
    assert_eq!(runtime.stats().reads, 1, "{:?}", runtime.stats());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_format_does_not_bypass_redirect_policy() {
    // The target is a loopback server reached as `localhost`, a host the run
    // does not allow.
    let target = export_server().await;
    let target_url = format!("http://localhost:{}/export?id=7", target.address().port());
    let origin = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/export"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", target_url.as_str()))
        .mount(&origin)
        .await;
    let dir = TempDir::new().unwrap();
    let url = format!("{}/export?id=7", origin.uri());
    let doc = doc_loading(dir.path(), &url, JSON_FORMAT);

    let failure = compose_blocking(doc.clone(), allowing_loopback(&doc)).await.unwrap_err();
    let message = failure.error.to_string();
    assert!(message.contains("redirect (HTTP 302)") && message.contains("blocked by fetch policy"), "{message}");
    assert!(!message.contains("macOS"), "no body was returned: {message}");
    assert_eq!(requests(&origin).await, 1);
    assert_eq!(requests(&target).await, 0, "the redirect was not followed");
    assert_eq!(failure.read_set, vec![url]);
}
