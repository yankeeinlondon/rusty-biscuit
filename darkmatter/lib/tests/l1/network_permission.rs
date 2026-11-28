//! The configured `network` permission decides which URLs a composition and
//! a schema slot may fetch: `allow` in the user file and, inside a
//! repository, in both its committed and working-copy files permits every
//! HTTP(S) host; a valid `deny` permits only the run's `--allow host=HOST`
//! hosts; a valid `ask` permits the same hosts and leaves every other URL
//! needing approval; an invalid `permissions` block permits nothing. Consent never relaxes the transport rules: redirects stay
//! refused, and a denied host's cached body is never served.
//!
//! Every case works in its own temp folder with a temp home, prepares its
//! requests there, and fetches only from a loopback mock server.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeRequest, RequestSnapshot};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CHILD_TEXT: &str = "Remote child.";
const ROWS_JSON: &str = "[\"a\", \"b\"]";
const SLOT_APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";
const SLOT_NETWORK_DENIED: &str = "dm.schema.slot_network_denied";

/// Runs git with a fixture-local identity, so the caller's hooks and signing
/// configuration never reach the fixture.
fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed");
}

fn network_yaml(network: &str) -> String {
    format!("permissions:\n  network: {network}\n")
}

/// A temp work folder and a temp home beside it.
struct Case {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Case {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let base = biscuit_file::canonicalize_simplified(temp.path()).expect("canonical temp dir");
        let root = base.join("work");
        let home = base.join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        Self { _temp: temp, root, home }
    }

    /// Writes the user file with `network: <network>`.
    fn user(&self, network: &str) -> &Self {
        std::fs::write(self.home.join(".darkmatter.yaml"), network_yaml(network)).unwrap();
        self
    }

    /// Makes the work folder a repository whose committed file says
    /// `network: <head>` and whose working copy says `network: <worktree>`.
    fn repository(&self, head: &str, worktree: &str) -> &Self {
        git(&self.root, &["init", "-q"]);
        std::fs::write(self.root.join(".darkmatter.yaml"), network_yaml(head)).unwrap();
        git(&self.root, &["add", ".darkmatter.yaml"]);
        git(&self.root, &["commit", "-q", "-m", "policy"]);
        std::fs::write(self.root.join(".darkmatter.yaml"), network_yaml(worktree)).unwrap();
        self
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

}

/// The request `md compose` prepares: anchored in the work folder `root`,
/// reading the configuration of `home`, with remote transclusion on.
fn request(root: &Path, home: &Path, document: &Path, options: ComposeOptions) -> ComposeRequest {
    let snapshot = RequestSnapshot::new(root).with_home(Some(home.to_path_buf())).with_env(HashMap::new());
    let options = options.with_source_file(document).with_allow_remote_transclusion(true);
    ComposeRequest::prepare(options, &snapshot).expect("request")
}

type Composed = Result<String, Box<ComposeFailure>>;

/// Composes `body` as `name` in `case`, off the async test's runtime (the
/// remote runtime blocks on its own executor).
async fn compose(case: &Case, name: &str, body: &str, options: ComposeOptions) -> Composed {
    let document = case.write(name, body);
    let (root, home) = (case.root.clone(), case.home.clone());
    tokio::task::spawn_blocking(move || {
        let request = request(&root, &home, &document, options);
        let markdown = Markdown::try_from(document.as_path()).expect("document loads");
        markdown.compose_with_warnings(&request).map(|(composed, _)| composed.content().trim().to_string())
    })
    .await
    .unwrap()
}

/// The failure's whole cause chain, for asserting on what it says.
fn chain(failure: &ComposeFailure) -> String {
    let mut text = String::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        text.push_str(&error.to_string());
        text.push('\n');
        current = error.source();
    }
    text
}

fn composed(result: Composed) -> String {
    result.unwrap_or_else(|failure| panic!("composition failed: {}", chain(&failure)))
}

fn refused(result: Composed) -> String {
    match result {
        Ok(text) => panic!("composition fetched what it must not: {text}"),
        Err(failure) => chain(&failure),
    }
}

async fn requests_to(server: &MockServer, route: &str) -> usize {
    server.received_requests().await.unwrap_or_default().iter().filter(|request| request.url.path() == route).count()
}

/// Serves `/child.md` and `/rows.json`, both cacheable for an hour.
async fn server() -> MockServer {
    let server = MockServer::start().await;
    for (route, body) in [("/child.md", CHILD_TEXT), ("/rows.json", ROWS_JSON)] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_string(body).insert_header("cache-control", "max-age=3600"))
            .mount(&server)
            .await;
    }
    server
}

fn transclusion(server: &MockServer) -> String {
    format!("::file {}/child.md\n", server.uri())
}

fn exact_loopback() -> ComposeOptions {
    ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_allow_composes_a_remote_file() {
    let server = server().await;
    let case = Case::new();
    case.user("allow");

    let text = composed(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);

    assert!(text.contains(CHILD_TEXT), "{text}");
    assert_eq!(requests_to(&server, "/child.md").await, 1);
}

/// A user block that lives in another file (`permissions: ./host.yaml`) is
/// followed: its `network: allow` grants exactly as an inline block does.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_referenced_user_block_that_allows_composes_a_remote_file() {
    let server = server().await;
    let case = Case::new();
    std::fs::write(case.home.join(".darkmatter.yaml"), "permissions: ./host.yaml\n").unwrap();
    std::fs::write(case.home.join("host.yaml"), format!("other: kept\n{}", network_yaml("allow"))).unwrap();

    let text = composed(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);

    assert!(text.contains(CHILD_TEXT), "{text}");
    assert_eq!(requests_to(&server, "/child.md").await, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_default_refuses_a_remote_file() {
    let server = server().await;
    let case = Case::new();

    let failure = refused(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);

    assert!(failure.contains("denied for host '127.0.0.1'"), "{failure}");
    assert_eq!(requests_to(&server, "/child.md").await, 0, "a denied host is never contacted");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repository_allow_under_user_deny_is_denied() {
    let server = server().await;
    let case = Case::new();
    case.user("deny").repository("allow", "allow");

    refused(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);
    assert_eq!(requests_to(&server, "/child.md").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_and_repository_allow_is_allowed() {
    let server = server().await;
    let case = Case::new();
    case.user("allow").repository("allow", "allow");

    let text = composed(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);
    assert!(text.contains(CHILD_TEXT), "{text}");
}

/// An uncommitted edit to the working copy cannot widen what `HEAD` says.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn worktree_allow_over_a_committed_deny_is_denied() {
    let server = server().await;
    let case = Case::new();
    case.user("allow").repository("deny", "allow");

    refused(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);
    assert_eq!(requests_to(&server, "/child.md").await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_exact_host_still_allows_under_deny() {
    let server = server().await;
    let case = Case::new();
    case.user("deny");

    let text = composed(compose(&case, "doc.md", &transclusion(&server), exact_loopback()).await);
    assert!(text.contains(CHILD_TEXT), "{text}");
}

/// A runtime shared before the snapshot was attached (the request loads it)
/// is replaced with one under the configured decision.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runtime_shared_before_the_configuration_follows_it() {
    let server = server().await;
    let case = Case::new();
    case.user("allow");

    let options = ComposeOptions::new().with_shared_remote_fetch();
    let text = composed(compose(&case, "doc.md", &transclusion(&server), options).await);
    assert!(text.contains(CHILD_TEXT), "{text}");
}

/// An invalid `permissions` block never authorizes, and the refusal names
/// the configuration error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_invalid_permissions_block_denies_and_reports_why() {
    let server = server().await;
    let case = Case::new();
    case.user("sometimes");

    let failure = refused(compose(&case, "doc.md", &transclusion(&server), ComposeOptions::new()).await);

    assert!(failure.contains("the configuration is invalid"), "{failure}");
    assert_eq!(requests_to(&server, "/child.md").await, 0);
}

/// Blanket consent still refuses every redirect hop, so the `Location` host
/// is never contacted unless a new fetch asks the policy again; a redirect to
/// a non-HTTP scheme is refused the same way.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn allow_still_refuses_redirects() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/hop.md"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", format!("{}/final.md", server.uri())))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/ftp.md"))
        .respond_with(ResponseTemplate::new(301).insert_header("location", "ftp://127.0.0.1/final.md"))
        .mount(&server)
        .await;
    let case = Case::new();
    case.user("allow");

    for route in ["/hop.md", "/ftp.md"] {
        let body = format!("::file {}{route}\n", server.uri());
        let failure = refused(compose(&case, "doc.md", &body, ComposeOptions::new()).await);
        assert!(failure.to_ascii_lowercase().contains("redirect"), "{route}: {failure}");
    }
    assert_eq!(requests_to(&server, "/final.md").await, 0, "no redirect was followed");
}

/// A body cached while the network was allowed is not served once the
/// configuration denies its host.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cached_body_is_not_served_under_deny() {
    let server = server().await;
    let case = Case::new();
    let cache = tempfile::tempdir().unwrap();
    let cached = || ComposeOptions::new().with_cache_root(cache.path());

    case.user("allow");
    composed(compose(&case, "doc.md", &transclusion(&server), cached()).await);
    case.user("allow");
    composed(compose(&case, "again.md", &transclusion(&server), cached()).await);
    assert_eq!(requests_to(&server, "/child.md").await, 1, "the second run is served from the cache");

    case.user("deny");
    refused(compose(&case, "denied.md", &transclusion(&server), cached()).await);
    assert_eq!(requests_to(&server, "/child.md").await, 1);
}

/// A document whose `$schema` slot reads `url`, and whose value the slot's
/// bound admits.
fn slot_document(url: &str) -> String {
    format!(
        "---\n$schema:\n    count: \"string(min({{{{ length(load_data('{url}', 'string[]')) }}}}))\"\ncount: abc\n---\n\nBody.\n"
    )
}

/// The body reads `url` with the same call.
fn body_document(url: &str) -> String {
    format!("{{{{ length(load_data('{url}', 'string[]')) }}}}\n")
}

/// Whether the failure is the slot gate's refusal.
fn slot_refused(result: &Composed) -> bool {
    match result {
        Ok(_) => false,
        Err(failure) => {
            error_diagnostics(&failure.error)
                .iter()
                .any(|record| record.code == SLOT_APPROVAL_REQUIRED || record.code == SLOT_NETWORK_DENIED)
                || chain(failure).contains("denied")
        }
    }
}

/// A schema-slot URL read and a document URL read take the same decision
/// under the same configuration and per-run hosts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slot_and_composition_reads_decide_alike() {
    let server = server().await;
    let url = format!("{}/rows.json", server.uri());
    type Expectation = (&'static str, fn() -> ComposeOptions, bool);
    let cases: [Expectation; 5] = [
        ("allow", ComposeOptions::new, true),
        ("deny", ComposeOptions::new, false),
        ("deny", exact_loopback, true),
        ("ask", ComposeOptions::new, false),
        ("ask", exact_loopback, true),
    ];
    for (network, options, allowed) in cases {
        let case = Case::new();
        case.user(network);

        let slot = compose(&case, "slot.md", &slot_document(&url), options()).await;
        let body = compose(&case, "body.md", &body_document(&url), options()).await;

        assert_eq!(slot.is_ok(), allowed, "slot under {network}: {:?}", slot.as_ref().err().map(|f| chain(f)));
        assert_eq!(body.is_ok(), allowed, "body under {network}: {:?}", body.as_ref().err().map(|f| chain(f)));
        if allowed {
            assert_eq!(body.unwrap(), "2");
        } else {
            assert!(slot_refused(&slot), "{:?}", slot.as_ref().err().map(|f| chain(f)));
        }
    }
}

// ── Invalid configuration outranks a per-run host ───────────────────────
//
// Every URL surface (`::file`, `::code`, a read-side expression, a schema
// slot) under one configuration and the exact per-run host `127.0.0.1`.
// An invalid `permissions` block (any non-null shape of `network` that is
// not `allow`/`ask`/`deny`, or any other invalid part of the block) is a located
// configuration error and no request; a valid `allow`/`deny`, or an omitted
// block, fetches. A `null` is the same as leaving the key out.

/// The four URL surfaces, each reading `server`'s `/rows.json`.
fn url_surfaces(server: &MockServer) -> [(&'static str, String); 4] {
    let url = format!("{}/rows.json", server.uri());
    [
        ("::file", format!("::file {url}\n")),
        ("::code", format!("::code {url}\n")),
        ("expression", body_document(&url)),
        ("schema slot", slot_document(&url)),
    ]
}

/// Composes every URL surface under the user file `config` with the exact
/// per-run host, in its own case, returning each surface's result and the
/// requests the run made.
async fn under_exact_host(config: &str) -> (Vec<(&'static str, Composed)>, usize, PathBuf) {
    let server = server().await;
    let case = Case::new();
    let user_file = case.home.join(".darkmatter.yaml");
    std::fs::write(&user_file, config).unwrap();
    let mut results = Vec::new();
    for (surface, body) in url_surfaces(&server) {
        results.push((surface, compose(&case, "doc.md", &body, exact_loopback()).await));
    }
    (results, requests_to(&server, "/rows.json").await, user_file)
}

/// Asserts every surface refused with the configuration error naming
/// `user_file` (and each of `also_named`) and that nothing was requested.
async fn assert_config_error(config: &str, also_named: &[&str]) {
    let (results, requests, user_file) = under_exact_host(config).await;
    let location = user_file.display().to_string();
    for (surface, result) in results {
        let failure = match result {
            Ok(text) => panic!("{surface} under {config:?} fetched with an invalid configuration: {text}"),
            Err(failure) => chain(&failure),
        };
        assert!(failure.contains(&location), "{surface} under {config:?} does not name {location}: {failure}");
        for name in also_named {
            assert!(failure.contains(name), "{surface} under {config:?} does not name {name}: {failure}");
        }
    }
    assert_eq!(requests, 0, "{config:?}: an invalid configuration requests nothing");
}

/// Asserts every surface fetched the allowed host.
async fn assert_fetches(config: &str) {
    let (results, requests, _) = under_exact_host(config).await;
    for (surface, result) in results {
        if let Err(failure) = result {
            panic!("{surface} under {config:?} refused the allowed host: {}", chain(&failure));
        }
    }
    assert!(requests > 0, "{config:?}: the allowed host was fetched");
}

macro_rules! invalid_configuration_rows {
    ($($name:ident: $config:expr, $named:expr;)*) => {$(
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $name() {
            assert_config_error($config, $named).await;
        }
    )*};
}

invalid_configuration_rows! {
    network_number_outranks_an_exact_host: "permissions:\n  network: 123\n", &["/permissions/network"];
    network_list_outranks_an_exact_host: "permissions:\n  network: []\n", &["/permissions/network"];
    network_empty_string_outranks_an_exact_host: "permissions:\n  network: \"\"\n", &["/permissions/network"];
    network_unknown_word_outranks_an_exact_host: "permissions:\n  network: sometimes\n", &["/permissions/network"];
    an_invalid_file_read_entry_outranks_an_exact_host: "permissions:\n  file-read: [123]\n", &["/permissions/file-read"];
    an_invalid_shell_rule_outranks_an_exact_host: "permissions:\n  shell-commands: [frobnicate echo]\n", &["/permissions/shell-commands"];
    a_missing_permissions_reference_outranks_an_exact_host: "permissions: ./missing.yaml\n", &["missing.yaml"];
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn valid_allow_fetches_every_surface_for_an_exact_host() {
    assert_fetches(&network_yaml("allow")).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn valid_deny_fetches_every_surface_for_an_exact_host() {
    assert_fetches(&network_yaml("deny")).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn omitted_permissions_fetch_every_surface_for_an_exact_host() {
    assert_fetches("{}\n").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_null_network_fetches_every_surface_for_an_exact_host_as_if_omitted() {
    assert_fetches("permissions:\n  network: null\n").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn null_permissions_fetch_every_surface_for_an_exact_host_as_if_omitted() {
    assert_fetches("permissions: null\n").await;
}

/// Under fail-fast, a fetched child whose frontmatter does not parse fails
/// under the frontmatter parse code, naming the URL, not as a generic composition
/// failure; the same child with valid frontmatter composes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_remote_child_with_malformed_frontmatter_keeps_its_parse_code() {
    let server = MockServer::start().await;
    for (route, body) in [("/bad.md", "---\nkey: [unclosed\n---\nBad child.\n"), ("/good.md", "---\nkey: ok\n---\nGood child.\n")] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .mount(&server)
            .await;
    }
    let case = Case::new();
    case.user("allow");

    let good = composed(compose(&case, "good.md", &format!("::file {}/good.md\n", server.uri()), ComposeOptions::new()).await);
    assert!(good.contains("Good child."), "{good}");

    // Without fail-fast, composition tolerates a failed child as a placeholder.
    let failure = compose(&case, "bad.md", &format!("::file {}/bad.md\n", server.uri()), ComposeOptions::new().with_fail_fast(true))
        .await
        .expect_err("malformed remote frontmatter fails");
    let records = error_diagnostics(&failure.error);
    let codes: Vec<&str> = records.iter().map(|record| record.code.as_str()).collect();
    assert_eq!(codes, ["dm.frontmatter.yaml_parse"], "{}", chain(&failure));
    assert!(chain(&failure).contains("/bad.md"), "the error names the URL: {}", chain(&failure));
}
