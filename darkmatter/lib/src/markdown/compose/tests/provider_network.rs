//! End-to-end network verification for the `pr*` / `cicd*` expression surface.
//!
//! Every other provider test in the tree stops short of the thing users
//! actually do: the parser tests never build a client, the formatter tests
//! never see a response, and Sniff's Wiremock suite never goes through compose.
//! These fixtures compose real documents against a real HTTP server so the
//! binding, repository resolution, host policy, credential handling, the
//! synchronous bridge onto the run-owned executor, the memoization key, the
//! error mapping, and the Markdown projection are all exercised as they
//! compose.
//!
//! The resolved remote is a loopback Gitea-flavored remote, so the traversal
//! shapes here are the parent-execution ones (`actions/runs` then
//! `actions/runs/{id}/jobs`) rather than GitLab's direct job listing.

use std::collections::HashSet;

use serde_json::{Value, json};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use sniff::filesystem::git::ApiFlavor;

use crate::markdown::Markdown;
use crate::markdown::compose::expression::functions::provider::test_transport;
use crate::markdown::compose::remote::RemoteReadConfig;
use crate::markdown::compose::{ComposeOperation, ComposeOptions};

/// Credential variables Sniff consults for the Gitea/Forgejo flavor.
///
/// Read from the ambient process environment, so a developer with a real token
/// exported would otherwise send it to the mock server and change which
/// authentication branch the client takes. Every fixture clears them, and the
/// one test that needs a token sets it explicitly.
const CREDENTIAL_VARIABLES: [&str; 2] = ["GITEA_TOKEN", "FORGEJO_TOKEN"];

/// One composed provider run: mock server, loopback repository, and the guards
/// that keep the run hermetic.
///
/// Field order is drop order: the API-base and environment guards must outlive
/// nothing in particular, but they are listed after the pieces the test body
/// uses so the guards are the last thing released.
struct Fixture {
    server: MockServer,
    directory: tempfile::TempDir,
    _credentials: Vec<EnvGuard>,
    _transport: test_transport::Guard,
}

impl Fixture {
    /// Starts a mock provider and points a throwaway repository's `origin` at it.
    ///
    /// The remote host must be the literal loopback address the mock server
    /// binds, because the client re-checks the endpoint host against the
    /// remote host before every request; a friendly hostname would be denied
    /// by that check rather than by the policy under test.
    async fn start() -> Self {
        Self::start_with_credential(None).await
    }

    async fn start_with_credential(token: Option<&str>) -> Self {
        let server = MockServer::start().await;
        let directory = tempfile::tempdir().expect("tempdir");
        let repository = git2::Repository::init(directory.path()).expect("git init");
        repository
            .remote("origin", "git@127.0.0.1:acme/widgets.git")
            .expect("configure origin");

        let credentials = CREDENTIAL_VARIABLES
            .iter()
            .map(|name| match token {
                Some(token) => EnvGuard::set_safe(name, token),
                None => EnvGuard::remove_safe(name),
            })
            .collect();

        let transport =
            test_transport::install(format!("{}/api/v1", server.uri()), ApiFlavor::Gitea);
        Self { server, directory, _credentials: credentials, _transport: transport }
    }

    /// Composes `content` as a document sitting in the repository root.
    fn compose(&self, content: &str) -> crate::markdown::MarkdownResult<Markdown> {
        self.compose_with_commands(content, HashSet::new())
    }

    /// Composes and keeps the report, for the surfaces that warn rather than fail.
    fn compose_reported(
        &self,
        content: &str,
    ) -> crate::markdown::MarkdownResult<(Markdown, super::ComposeReport)> {
        self.compose_full(content, vec!["127.0.0.1".into()], HashSet::new())
    }

    fn compose_with_commands(
        &self,
        content: &str,
        approved: HashSet<String>,
    ) -> crate::markdown::MarkdownResult<Markdown> {
        self.compose_with_hosts(content, vec!["127.0.0.1".into()], approved)
    }

    /// Composes with the host policy left at its deny-all default.
    ///
    /// The remote runtime is still installed and the document is otherwise
    /// identical, so the only difference from an allowed run is the one thing
    /// under test: `127.0.0.1` was never authorized.
    fn compose_denied(&self, content: &str) -> crate::markdown::MarkdownResult<Markdown> {
        self.compose_with_hosts(content, Vec::new(), HashSet::new())
    }

    fn compose_with_hosts(
        &self,
        content: &str,
        allowed_hosts: Vec<String>,
        approved: HashSet<String>,
    ) -> crate::markdown::MarkdownResult<Markdown> {
        self.compose_full(content, allowed_hosts, approved).map(|(composed, _)| composed)
    }

    fn compose_full(
        &self,
        content: &str,
        allowed_hosts: Vec<String>,
        approved: HashSet<String>,
    ) -> crate::markdown::MarkdownResult<(Markdown, super::ComposeReport)> {
        let root = self.directory.path().join("root.md");
        std::fs::write(&root, content).expect("write document");
        let markdown: Markdown = content.into();
        let options = ComposeOptions::new()
            .with_source_file(&root)
            .with_allow_remote_transclusion(true)
            .with_remote_read_config(RemoteReadConfig {
                allowed_hosts,
                ..Default::default()
            })
            .with_pre_approved_commands(approved)
            .disable(ComposeOperation::Cleanup)
            .disable(ComposeOperation::Normalization);
        markdown.compose_with(options)
    }

    async fn request_count(&self) -> usize {
        self.server.received_requests().await.expect("recorded requests").len()
    }

    /// Paths the server was asked for, in arrival order.
    async fn request_paths(&self) -> Vec<String> {
        self.server
            .received_requests()
            .await
            .expect("recorded requests")
            .iter()
            .map(|request| request.url.path().to_string())
            .collect()
    }

    async fn mount_json(&self, endpoint: &str, body: Value) {
        Mock::given(method("GET"))
            .and(path(endpoint.to_string()))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&self.server)
            .await;
    }

    async fn mount_status(&self, endpoint: &str, status: u16, body: &str) {
        Mock::given(method("GET"))
            .and(path(endpoint.to_string()))
            .respond_with(ResponseTemplate::new(status).set_body_string(body.to_string()))
            .mount(&self.server)
            .await;
    }
}

const PR_PATH: &str = "/api/v1/repos/acme/widgets/pulls/123";
const PR_LIST_PATH: &str = "/api/v1/repos/acme/widgets/pulls";
const JOB_PATH: &str = "/api/v1/repos/acme/widgets/actions/jobs/456";
const RUNS_PATH: &str = "/api/v1/repos/acme/widgets/actions/runs";
const RUN_JOBS_PATH: &str = "/api/v1/repos/acme/widgets/actions/runs/99/jobs";

fn pr_body(number: u64, title: &str) -> Value {
    json!({
        "number": number,
        "title": title,
        "state": "open",
        "user": {"login": "alice"},
        "draft": false,
        "head": {"ref": "feature/parser"},
        "base": {"ref": "main"},
        "created_at": "2026-07-18T00:00:00Z",
        "html_url": format!("https://127.0.0.1/acme/widgets/pulls/{number}"),
    })
}

fn job_body(id: u64, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "status": "completed",
        "conclusion": "success",
        "run_id": 99,
        "head_branch": "main",
        "head_sha": "abcdef1234567890",
        "created_at": "2026-07-18T00:00:00Z",
        "html_url": format!("https://127.0.0.1/acme/widgets/jobs/{id}"),
    })
}

/// Extracts the composed frontmatter value as a string.
fn frontmatter_string(composed: &Markdown, key: &str) -> String {
    match composed.frontmatter().as_map().get(key) {
        Some(Value::String(text)) => text.clone(),
        other => panic!("frontmatter `{key}` was not a string: {other:?}"),
    }
}

/// Extracts a list-valued frontmatter entry.
///
/// The list functions keep their JSON array shape in frontmatter while body
/// interpolation has to flatten to text, so the two surfaces are compared
/// through this rather than by string equality.
fn frontmatter_list(composed: &Markdown, key: &str) -> Vec<String> {
    match composed.frontmatter().as_map().get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) => text.clone(),
                other => panic!("list entry was not a string: {other:?}"),
            })
            .collect(),
        other => panic!("frontmatter `{key}` was not an array: {other:?}"),
    }
}

/// Asserts the compose failed and returns the message.
fn error_text<T>(result: crate::markdown::MarkdownResult<T>) -> String {
    match result {
        Ok(_) => panic!("a focused provider failure composed successfully instead of erroring"),
        Err(error) => error.to_string(),
    }
}

/// Wraps `expression` so it is evaluated on the frontmatter surface.
///
/// Frontmatter interpolation is the surface that propagates a focused provider
/// failure out of `compose_with`; the body surface downgrades the same failure
/// to a warning (see `body_surface_downgrades_focused_failures_to_warnings`).
fn frontmatter_document(expression: &str) -> String {
    format!("---\nvalue: \"{{{{ {expression} }}}}\"\n---\nunused\n")
}

// ---------------------------------------------------------------------------
// 1. Cross-surface identity (AC23 / AC25 / AC26)
// ---------------------------------------------------------------------------

/// The same normalized call must render the same text wherever it is written.
///
/// Frontmatter interpolation and body interpolation are separate evaluation
/// passes with separate value plumbing, so equality between them is the actual
/// content of "function availability does not vary by document region" — not
/// merely that neither surface reported an unknown function.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn pr_renders_identically_in_frontmatter_and_body() {
    let fixture = Fixture::start().await;
    fixture.mount_json(PR_PATH, pr_body(123, "Fix the parser")).await;

    let composed = fixture
        .compose("---\nstatus: \"{{ pr(123) }}\"\n---\nPR: {{ pr(123) }}\n")
        .expect("compose succeeds");

    let expected = "[PR #123 — Fix the parser](https://127.0.0.1/acme/widgets/pulls/123) \
                    · open · @alice · feature/parser → main";
    assert_eq!(frontmatter_string(&composed, "status"), expected);
    assert_eq!(composed.content(), format!("PR: {expected}"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn pr_list_renders_identically_in_frontmatter_and_body() {
    let fixture = Fixture::start().await;
    fixture
        .mount_json(PR_LIST_PATH, json!([pr_body(123, "Fix the parser")]))
        .await;

    let composed = fixture
        .compose("---\nlist: \"{{ pr_list(5) }}\"\n---\nPRs: {{ pr_list(5) }}\n")
        .expect("compose succeeds");

    let body = composed.content().to_string();
    let front = frontmatter_list(&composed, "list");
    assert_eq!(
        front,
        vec![
            "[PR #123 — Fix the parser](https://127.0.0.1/acme/widgets/pulls/123) \
             · open · @alice · feature/parser → main"
        ]
    );
    for entry in &front {
        assert!(body.contains(entry.as_str()), "body lost a list entry: {body}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn cicd_renders_identically_in_frontmatter_and_body() {
    let fixture = Fixture::start().await;
    fixture.mount_json(JOB_PATH, job_body(456, "build")).await;

    let composed = fixture
        .compose("---\njob: \"{{ cicd(456) }}\"\n---\nJob: {{ cicd(456) }}\n")
        .expect("compose succeeds");

    let front = frontmatter_string(&composed, "job");
    assert!(front.contains("CI job #456 — build"), "frontmatter: {front}");
    assert_eq!(composed.content(), format!("Job: {front}"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn cicd_list_renders_identically_in_frontmatter_and_body() {
    let fixture = Fixture::start().await;
    fixture
        .mount_json(RUNS_PATH, json!({"workflow_runs": [{"id": 99}]}))
        .await;
    fixture
        .mount_json(RUN_JOBS_PATH, json!({"jobs": [job_body(456, "build")]}))
        .await;

    let composed = fixture
        .compose("---\njobs: \"{{ cicd_list(5) }}\"\n---\nJobs: {{ cicd_list(5) }}\n")
        .expect("compose succeeds");

    let body = composed.content().to_string();
    let front = frontmatter_list(&composed, "jobs");
    assert_eq!(
        front,
        vec![
            "[CI job #456 — build](https://127.0.0.1/acme/widgets/jobs/456) \
             · success · main @ abcdef1"
        ]
    );
    for entry in &front {
        assert!(body.contains(entry.as_str()), "body lost a list entry: {body}");
    }
}

/// The `$()` surface evaluates its condition through the same binding table.
///
/// A shell ternary yields the chosen command's output rather than the provider
/// string, so the claim this can carry is availability and shared caching — the
/// value-identity claim lives in the interpolation tests above.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn provider_functions_are_available_in_frontmatter_shell_ternary() {
    let fixture = Fixture::start().await;
    fixture.mount_json(PR_PATH, pr_body(123, "Fix the parser")).await;

    let approved: HashSet<String> =
        ["echo found".to_string(), "echo missing".to_string()].into_iter().collect();
    let composed = fixture
        .compose_with_commands(
            "---\nresolved: $(pr(123) ? echo found : echo missing)\n---\nPR: {{ pr(123) }}\n",
            approved,
        )
        .expect("compose succeeds");

    assert_eq!(frontmatter_string(&composed, "resolved"), "found");
    assert!(composed.content().contains("PR #123"), "{}", composed.content());
    assert_eq!(
        fixture.request_count().await,
        1,
        "the `$()` condition and the body call are the same normalized query and \
         must share one single-flight slot"
    );
}

// ---------------------------------------------------------------------------
// 2. Single-flight and memoization (AC26)
// ---------------------------------------------------------------------------

/// Identical normalized calls collapse to one request across every surface.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn identical_provider_calls_reach_the_server_exactly_once() {
    let fixture = Fixture::start().await;
    Mock::given(method("GET"))
        .and(path(PR_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr_body(123, "Fix the parser")))
        .expect(1)
        .mount(&fixture.server)
        .await;

    fixture
        .compose(
            "---\na: \"{{ pr(123) }}\"\nb: \"{{ pr(123) }}\"\n---\n\
             One: {{ pr(123) }}\n\nTwo: {{ pr(123) }}\n",
        )
        .expect("compose succeeds");

    assert_eq!(fixture.request_count().await, 1);
    fixture.server.verify().await;
}

/// The cache must key on the normalized request, not merely on the function.
///
/// Without this, a key that ignored its arguments would satisfy the
/// exactly-one-request assertion above while silently serving PR 123's record
/// for every other pull request in the document.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn differently_normalized_calls_do_not_share_a_cache_slot() {
    let fixture = Fixture::start().await;
    fixture.mount_json(PR_PATH, pr_body(123, "Fix the parser")).await;
    fixture
        .mount_json("/api/v1/repos/acme/widgets/pulls/124", pr_body(124, "Add a lexer"))
        .await;

    let composed = fixture
        .compose("A: {{ pr(123) }}\n\nB: {{ pr(124) }}\n")
        .expect("compose succeeds");

    let text = composed.content();
    assert!(text.contains("Fix the parser"), "{text}");
    assert!(text.contains("Add a lexer"), "{text}");
    assert_eq!(fixture.request_count().await, 2);
}

/// Distinct list queries are distinct cache entries even though both are
/// `pr_list` against the same endpoint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn list_queries_with_different_shapes_are_not_memoized_together() {
    let fixture = Fixture::start().await;
    fixture
        .mount_json(PR_LIST_PATH, json!([pr_body(123, "Fix the parser")]))
        .await;

    fixture
        .compose(
            "A: {{ pr_list(5) }}\n\nB: {{ pr_list({\"limit\": 5}) }}\n\n\
             C: {{ pr_list({\"limit\": 5, \"direction\": \"ascending\"}) }}\n\n\
             D: {{ pr_list(5) }}\n",
        )
        .expect("compose succeeds");

    // The key is built from the *parsed* query, so A, B and D are one slot even
    // though A and D are authored as a bare count and B as an object. C only
    // changes ordering, which is exactly the kind of difference a key built
    // from the function name and limit alone would lose.
    let paths = fixture.request_paths().await;
    assert_eq!(
        paths.len(),
        2,
        "expected the newest-first slot plus the ascending one: {paths:?}"
    );
}

// ---------------------------------------------------------------------------
// 3. Exact-host policy
// ---------------------------------------------------------------------------

/// A denied host must fail before any byte leaves the process.
///
/// Asserting only on the message would pass an implementation that sent the
/// request and discarded the response, which is precisely the leak the
/// deny-by-default policy exists to prevent.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn denied_host_fails_without_contacting_the_provider() {
    let fixture = Fixture::start().await;
    fixture.mount_json(PR_PATH, pr_body(123, "Fix the parser")).await;

    let message = error_text(fixture.compose_denied(&frontmatter_document("pr(123)")));

    assert!(
        message.contains("127.0.0.1"),
        "the denial must name the host that was refused: {message}"
    );
    assert_eq!(
        fixture.request_count().await,
        0,
        "a denied provider call reached the network anyway"
    );
}

// ---------------------------------------------------------------------------
// 4. Focused error preservation (AC27)
// ---------------------------------------------------------------------------

/// Every focused provider failure must surface as an error, never as an empty
/// value.
///
/// An empty string or `[]` is a legitimate successful answer — a PR with no
/// matches, a job list with nothing queued — so collapsing a failure into one
/// makes an outage indistinguishable from a quiet repository.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn focused_failures_surface_as_errors_rather_than_empty_values() {
    for (status, body, expected) in [
        (404u16, "{}", "not found"),
        // 403 is mapped to a named "denied" state rather than carrying the
        // numeric status through, so the discriminating word is the state.
        (403, "{\"message\":\"forbidden\"}", "denied"),
        (429, "{\"message\":\"slow down\"}", "rate limited"),
        (500, "{\"message\":\"boom\"}", "500"),
        (200, "{ not json", "malformed"),
    ] {
        let fixture = Fixture::start().await;
        fixture.mount_status(PR_PATH, status, body).await;

        let message = error_text(fixture.compose(&frontmatter_document("pr(123)")));
        let lowered = message.to_lowercase();
        assert!(
            lowered.contains(&expected.to_lowercase()),
            "status {status} lost its focused detail: {message}"
        );
        // The memoization layer stores failures as text, so the function prefix
        // is easy to apply twice on the way back out.
        assert!(
            !message.contains("pr(): pr()"),
            "the function prefix was applied twice: {message}"
        );
    }
}

/// A missing credential is its own reportable state, distinct from "no result".
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn missing_credentials_surface_as_a_credential_error() {
    let fixture = Fixture::start().await;
    fixture.mount_status(PR_PATH, 401, "{\"message\":\"auth required\"}").await;

    let message = error_text(fixture.compose(&frontmatter_document("pr(123)")));
    let lowered = message.to_lowercase();
    assert!(
        lowered.contains("credential") || lowered.contains("token") || lowered.contains("401"),
        "a 401 without a configured token must name the credential state: {message}"
    );
}

/// A rejected token is a different state from an absent one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn invalid_credentials_surface_as_an_authorization_error() {
    let fixture = Fixture::start_with_credential(Some("not-a-real-token")).await;
    fixture.mount_status(PR_PATH, 401, "{\"message\":\"bad credentials\"}").await;

    let message = error_text(fixture.compose(&frontmatter_document("pr(123)")));
    assert!(
        message.to_lowercase().contains("401")
            || message.to_lowercase().contains("unauthor")
            || message.to_lowercase().contains("credential"),
        "a rejected token must not read as an empty result: {message}"
    );
}

/// A traversal that hits its safety cap is incomplete, not empty.
///
/// The parent-execution bound is the cheapest of the three caps to reach: one
/// oversized page of workflow runs is enough, and the walk refuses at the run
/// past the bound rather than returning the jobs it managed to collect.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn incomplete_domain_surfaces_rather_than_a_truncated_list() {
    let fixture = Fixture::start().await;
    let runs: Vec<Value> = (1..=21).map(|id| json!({"id": id})).collect();
    fixture.mount_json(RUNS_PATH, json!({"workflow_runs": runs})).await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/api/v1/repos/acme/widgets/actions/runs/\d+/jobs$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"jobs": []})))
        .mount(&fixture.server)
        .await;

    let message = error_text(fixture.compose(&frontmatter_document("cicd_list(5)")));
    let lowered = message.to_lowercase();
    assert!(
        lowered.contains("incomplete") || lowered.contains("parent executions"),
        "a capped traversal must report incompleteness, not a short list: {message}"
    );
}

/// A successful query with no matches is the one case that legitimately renders
/// as an empty list, which is what makes the assertions above discriminating.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn a_successful_empty_query_is_still_an_empty_list() {
    let fixture = Fixture::start().await;
    fixture.mount_json(PR_LIST_PATH, json!([])).await;

    let composed = fixture
        .compose("---\nlist: \"{{ pr_list(5) }}\"\n---\nPRs: {{ pr_list(5) }}\n")
        .expect("compose succeeds");
    assert_eq!(frontmatter_list(&composed, "list"), Vec::<String>::new());
    assert_eq!(composed.content(), "PRs: ");
}

/// The body surface downgrades a focused provider failure to a warning.
///
/// This is the compose-wide policy for non-authoring-fatal expression errors,
/// not something specific to providers: `is_authoring_fatal` covers unknown
/// functions and broken file references, and `ExpressionError::Other` — which
/// is what every focused provider failure becomes — is outside that set. The
/// consequence is a real asymmetry against AC26's frontmatter/body parity
/// claim: the identical call aborts the compose from frontmatter but leaves the
/// unevaluated expression text in the body with only a report warning.
///
/// The value is at least never an empty string or empty array, so the spec's
/// "focused errors are never replaced with empty values" bullet holds on both
/// surfaces. This test pins the behavior that actually ships so a future change
/// to provider-error fatality is a deliberate, visible decision.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn body_surface_downgrades_focused_failures_to_warnings() {
    let fixture = Fixture::start().await;
    fixture.mount_status(PR_PATH, 500, "{\"message\":\"boom\"}").await;

    let (composed, report) =
        fixture.compose_reported("PR: {{ pr(123) }}\n").expect("the body surface does not abort");

    let text = composed.content().to_string();
    assert_ne!(text, "PR: ", "a focused failure must not render as an empty value");
    assert!(
        text.contains("{{ pr(123) }}"),
        "the unevaluated expression is what currently survives: {text}"
    );
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.message.contains("500") || warning.message.contains("pr(123)")),
        "the focused failure left no warning behind: {:?}",
        report.warnings
    );

    // The same call from frontmatter aborts, which is the asymmetry itself.
    let fixture = Fixture::start().await;
    fixture.mount_status(PR_PATH, 500, "{\"message\":\"boom\"}").await;
    let message = error_text(fixture.compose(&frontmatter_document("pr(123)")));
    assert!(message.contains("500"), "{message}");
}

// ---------------------------------------------------------------------------
// 5. Rendered values
// ---------------------------------------------------------------------------

/// Hostile provider text must land in the document as literal text.
///
/// The provider controls the title, so a title containing Markdown must not
/// gain emphasis, a code span, or a competing link destination once it is
/// interpolated into a composed document.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial(provider_transport)]
async fn hostile_provider_titles_stay_literal_in_the_composed_document() {
    let fixture = Fixture::start().await;
    fixture
        .mount_json(PR_PATH, pr_body(123, "**urgent** `code` [click](https://evil.example)"))
        .await;

    let composed = fixture.compose("PR: {{ pr(123) }}\n").expect("compose succeeds");
    let text = composed.content().to_string();

    // The hostile destination survives as *characters* — that is the point of
    // escaping rather than stripping — so the claim has to be made against a
    // CommonMark parse, not a substring search.
    let (destination, literal) =
        crate::markdown::compose::expression::functions::escape::harness::parse_literal(
            text.trim_start_matches("PR: "),
        );
    assert_eq!(destination.as_deref(), Some("https://127.0.0.1/acme/widgets/pulls/123"));
    assert!(
        literal.contains("**urgent** `code` [click](https://evil.example)"),
        "the title did not survive as literal text: {literal}"
    );
}
