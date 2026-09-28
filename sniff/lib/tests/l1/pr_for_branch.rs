//! `remote::blocking::pull_request_for_branch` against loopback providers.
//!
//! Every test is a plain `#[test]`: the function under test builds its own
//! runtime and refuses to run inside one. wiremock serves from its own thread,
//! so a throwaway runtime only starts the server and mounts mocks.

use std::time::{Duration, Instant};

use biscuit_file::FetchPolicy;
use serde_json::{Value, json};
use serial_test::serial;
use sniff::filesystem::git::{ApiFlavor, ResolvedRemote};
use sniff::remote::FocusedProviderClient;
use sniff::remote::blocking::{PrEvidence, PrState, PrUnavailable, pull_request_for_branch_with};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub(super) const FLAVORS: [ApiFlavor; 4] = [
    ApiFlavor::GitHub,
    ApiFlavor::GitLab,
    ApiFlavor::Gitea,
    ApiFlavor::Bitbucket,
];
pub(super) const TARGET: &str = "acme/project";
pub(super) const BRANCH: &str = "feature";
pub(super) const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
const TOKEN_VARIABLES: [&str; 8] = [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GITLAB_TOKEN",
    "GITLAB_PRIVATE_TOKEN",
    "GITEA_TOKEN",
    "FORGEJO_TOKEN",
    "CODEBERG_TOKEN",
    "BITBUCKET_TOKEN",
];

/// A token value distinctive enough that finding it in any `Display` output
/// can only mean it leaked there.
pub(super) const SECRET: &str = "sniff-test-secret-4f1c9e";

/// Clears every provider token so a developer's real credentials never reach
/// the loopback server and the "no token" paths are deterministic.
pub(super) fn without_tokens() -> Vec<EnvGuard> {
    TOKEN_VARIABLES
        .into_iter()
        .map(EnvGuard::remove_safe)
        .collect()
}

pub(super) struct Provider {
    runtime: tokio::runtime::Runtime,
    server: MockServer,
    flavor: ApiFlavor,
}

impl Provider {
    pub(super) fn start(flavor: ApiFlavor) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("fixture runtime");
        let server = runtime.block_on(MockServer::start());
        Self {
            runtime,
            server,
            flavor,
        }
    }

    pub(super) fn flavor(&self) -> ApiFlavor {
        self.flavor
    }

    pub(super) fn mount(&self, mock: Mock) {
        self.runtime.block_on(mock.mount(&self.server));
    }

    /// Serves `items` (one page) from this provider's PR list endpoint.
    pub(super) fn serve_list(&self, items: Vec<Value>) {
        self.serve_list_response(
            ResponseTemplate::new(200).set_body_json(list_body(self.flavor, items)),
        );
    }

    pub(super) fn serve_list_response(&self, response: ResponseTemplate) {
        self.mount(
            Mock::given(method("GET"))
                .and(path_regex(list_path_pattern(self.flavor)))
                .respond_with(response),
        );
    }

    pub(super) fn client(&self) -> FocusedProviderClient {
        let remote = ResolvedRemote {
            name: "origin".to_string(),
            fetch_url: "git@127.0.0.1:acme/project.git".to_string(),
            push_url: "git@127.0.0.1:acme/project.git".to_string(),
            host: Some("127.0.0.1".to_string()),
            namespace: Some("acme".to_string()),
            repository: Some("project".to_string()),
            api_flavor: self.flavor,
            endpoint: None,
        };
        FocusedProviderClient::with_api_base(
            remote,
            FetchPolicy::deny_all().allow_host("127.0.0.1"),
            &format!("{}/api", self.server.uri()),
        )
        .expect("loopback client")
    }

    fn lookup_within(
        &self,
        source_repo: &str,
        deadline: Duration,
    ) -> Result<Option<PrEvidence>, PrUnavailable> {
        pull_request_for_branch_with(&self.client(), source_repo, BRANCH, deadline)
    }

    fn lookup(&self, source_repo: &str) -> Result<Option<PrEvidence>, PrUnavailable> {
        self.lookup_within(source_repo, Duration::from_secs(5))
    }

    /// Every request the server received, in order.
    pub(super) fn received_requests(&self) -> Vec<wiremock::Request> {
        self.runtime
            .block_on(self.server.received_requests())
            .expect("request recording is on")
    }

    /// Query pairs of every request the server received, in order.
    pub(super) fn received_queries(&self) -> Vec<Vec<(String, String)>> {
        self.runtime
            .block_on(self.server.received_requests())
            .expect("request recording is on")
            .iter()
            .map(|request| {
                request
                    .url
                    .query_pairs()
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect()
            })
            .collect()
    }
}

fn list_path_pattern(flavor: ApiFlavor) -> &'static str {
    match flavor {
        ApiFlavor::GitLab => r"^/api/projects/acme(%2F|/)project/merge_requests$",
        ApiFlavor::Bitbucket => r"^/api/repositories/acme/project/pullrequests$",
        _ => r"^/api/repos/acme/project/pulls$",
    }
}

pub(super) fn list_body(flavor: ApiFlavor, items: Vec<Value>) -> Value {
    match flavor {
        ApiFlavor::Bitbucket => json!({ "values": items }),
        _ => Value::Array(items),
    }
}

/// The variable each flavor's credential cases set. GitHub's and Gitea's are
/// not their first candidate, so a `key` naming them proves the variable
/// that matched is reported.
pub(super) fn token_variable(flavor: ApiFlavor) -> &'static str {
    match flavor {
        ApiFlavor::GitHub => "GITHUB_TOKEN",
        ApiFlavor::GitLab => "GITLAB_TOKEN",
        ApiFlavor::Gitea => "FORGEJO_TOKEN",
        _ => "BITBUCKET_TOKEN",
    }
}

/// The body each provider sends with a 403 that names the token's missing
/// permission or scope.
fn insufficient_body(flavor: ApiFlavor) -> Value {
    match flavor {
        ApiFlavor::GitHub => json!({"message": "Resource not accessible by personal access token"}),
        ApiFlavor::GitLab => json!({
            "error": "insufficient_scope",
            "error_description": "The request requires higher privileges than provided by the access token."
        }),
        ApiFlavor::Gitea => json!({
            "message": "token does not have at least one of required scope(s): [read:repository]"
        }),
        _ => json!({
            "type": "error",
            "error": {"message": "Your credentials lack one or more required privilege scopes."}
        }),
    }
}

/// The `PrUnavailable` condition a credential case must produce; the
/// expected `key` follows from whether the case sets a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Condition {
    Required,
    Rejected,
    Insufficient,
    NotVisible,
    RateLimited,
}

/// One case: its name, whether a token is set, the provider's response, and
/// the condition it must produce.
pub(super) type CredentialCase = (&'static str, bool, ResponseTemplate, Condition);

/// Every status §5 of `2026-09-27-list-freshness-ux` distinguishes, for
/// `flavor`.
pub(super) fn credential_cases(flavor: ApiFlavor) -> Vec<CredentialCase> {
    use Condition::{Insufficient, NotVisible, RateLimited, Rejected, Required};
    let status = ResponseTemplate::new;
    let scoped = || status(403).set_body_json(insufficient_body(flavor));
    let spent = |code| status(code).insert_header("x-ratelimit-remaining", "0");
    let mut cases = vec![
        ("401 anonymous", false, status(401), Required),
        ("401 with token", true, status(401), Rejected),
        ("404 anonymous", false, status(404), NotVisible),
        ("404 with token", true, status(404), NotVisible),
        ("403 scope body", true, scoped(), Insufficient),
        // A scope body without a token says nothing about a token.
        ("403 anonymous scope body", false, scoped(), NotVisible),
        ("403 with token, no body", true, status(403), NotVisible),
        ("429 anonymous", false, status(429), RateLimited),
        ("429 with token", true, status(429), RateLimited),
    ];
    if flavor == ApiFlavor::GitHub {
        let body = json!({"message": "API rate limit exceeded for user ID 1."});
        let limited = status(403).set_body_json(body);
        cases.extend([
            ("403 spent quota", false, spent(403), RateLimited),
            ("429 spent quota", true, spent(429), RateLimited),
            ("403 rate-limit body", true, limited, RateLimited),
        ]);
    }
    cases
}

/// Runs every credential case on every provider: `serve` mounts the case's
/// response on the endpoint `run` queries.
///
/// Asserts the condition, that `key` names `token_variable` exactly when the
/// case set a token, and that no `Display` output holds the token value.
pub(super) fn check_credential_cases<T: std::fmt::Debug>(
    serve: impl Fn(&Provider, ResponseTemplate),
    run: impl Fn(&Provider) -> Result<T, PrUnavailable>,
) {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        for (name, token, response, expected) in credential_cases(flavor) {
            let _token = token.then(|| EnvGuard::set_safe(token_variable(flavor), SECRET));
            let key = token.then(|| token_variable(flavor).to_string());
            let provider = Provider::start(flavor);
            serve(&provider, response);

            let error = run(&provider).expect_err(name);

            let context = format!("{flavor:?} {name}: {error:?}");
            assert!(!error.to_string().contains(SECRET), "{context}");
            let matched = match (expected, &error) {
                (Condition::Required, PrUnavailable::CredentialsRequired { key: found }) => {
                    found.is_none()
                }
                (Condition::Rejected, PrUnavailable::CredentialsRejected { key: found })
                | (
                    Condition::Insufficient,
                    PrUnavailable::CredentialsInsufficient { key: found },
                ) => Some(found) == key.as_ref(),
                (Condition::NotVisible, PrUnavailable::NotFoundOrNotPermitted { .. }) => true,
                (
                    Condition::RateLimited,
                    PrUnavailable::RateLimited {
                        authenticated,
                        key: found,
                    },
                ) => *authenticated == token && *found == key,
                _ => false,
            };
            assert!(matched, "expected {expected:?}; {context}");
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Lifecycle {
    Open,
    Merged,
    ClosedUnmerged,
}

/// One PR row in `flavor`'s list-endpoint shape.
///
/// `source` is the head repository's `owner/repo`. GitLab has no path in the
/// payload, so the fixture maps `TARGET` to project 1 and anything else to 2.
pub(super) fn pr(
    flavor: ApiFlavor,
    number: u64,
    lifecycle: Lifecycle,
    source: &str,
    sha: Option<&str>,
) -> Value {
    let merged_at = (lifecycle == Lifecycle::Merged).then_some("2024-03-01T00:00:00Z");
    let html_url = format!("https://127.0.0.1/acme/project/pull/{number}");
    match flavor {
        ApiFlavor::GitLab => json!({
            "iid": number,
            "title": format!("mr {number}"),
            "state": match lifecycle {
                Lifecycle::Open => "opened",
                Lifecycle::Merged => "merged",
                Lifecycle::ClosedUnmerged => "closed",
            },
            "author": {"username": "dev"},
            "source_branch": BRANCH,
            "target_branch": "main",
            "source_project_id": if source == TARGET { 1 } else { 2 },
            "target_project_id": 1,
            "sha": sha,
            "created_at": "2024-01-01T00:00:00Z",
            "merged_at": merged_at,
            "web_url": html_url,
        }),
        ApiFlavor::Bitbucket => json!({
            "id": number,
            "title": format!("pr {number}"),
            "state": match lifecycle {
                Lifecycle::Open => "OPEN",
                Lifecycle::Merged => "MERGED",
                Lifecycle::ClosedUnmerged => "DECLINED",
            },
            "author": {"display_name": "dev"},
            "source": {
                "branch": {"name": BRANCH},
                "commit": sha.map(|hash| json!({"hash": hash})),
                "repository": {"full_name": source},
            },
            "destination": {
                "branch": {"name": "main"},
                "repository": {"full_name": TARGET},
            },
            "created_on": "2024-01-01T00:00:00Z",
            "updated_on": "2024-03-01T00:00:00Z",
            "links": {"html": {"href": html_url}},
        }),
        // GitHub and Gitea share the shape; Gitea's `head.label` is the bare
        // branch name and it adds the `merged` bool.
        _ => json!({
            "number": number,
            "title": format!("pr {number}"),
            "state": if lifecycle == Lifecycle::Open { "open" } else { "closed" },
            "user": {"login": "dev"},
            "head": {
                "ref": BRANCH,
                "label": if flavor == ApiFlavor::GitHub {
                    format!("{}:{BRANCH}", source.split('/').next().unwrap_or_default())
                } else {
                    BRANCH.to_string()
                },
                "sha": sha,
                "repo": {"full_name": source},
            },
            "base": {"ref": "main", "repo": {"full_name": TARGET}},
            "created_at": "2024-01-01T00:00:00Z",
            "merged": lifecycle == Lifecycle::Merged,
            "merged_at": merged_at,
            "html_url": html_url,
        }),
    }
}

#[test]
#[serial]
fn an_open_pr_is_open_evidence_on_every_provider() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![pr(flavor, 7, Lifecycle::Open, TARGET, Some(SHA))]);

        let evidence = provider.lookup(TARGET).unwrap().expect("open PR found");

        assert_eq!(evidence.number, 7, "{flavor:?}");
        assert_eq!(evidence.state, PrState::Open, "{flavor:?}");
        assert_eq!(evidence.source_repo, TARGET, "{flavor:?}");
        assert_eq!(evidence.source_head_sha.as_deref(), Some(SHA), "{flavor:?}");
        assert_eq!(
            evidence.target_branch.as_deref(),
            Some("main"),
            "{flavor:?}"
        );
        assert_eq!(
            evidence.html_url.as_deref(),
            Some("https://127.0.0.1/acme/project/pull/7"),
            "{flavor:?}"
        );
    }
}

#[test]
#[serial]
fn a_merged_pr_is_merged_evidence_on_every_provider() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![pr(flavor, 8, Lifecycle::Merged, TARGET, Some(SHA))]);

        let evidence = provider.lookup(TARGET).unwrap().expect("merged PR found");

        assert_eq!(
            (evidence.number, evidence.state),
            (8, PrState::Merged),
            "{flavor:?}"
        );
        assert_eq!(evidence.source_head_sha.as_deref(), Some(SHA), "{flavor:?}");
    }
}

#[test]
#[serial]
fn an_open_pr_outranks_a_merged_one_for_the_same_branch() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![
            pr(flavor, 3, Lifecycle::Merged, TARGET, Some(SHA)),
            pr(flavor, 4, Lifecycle::Open, TARGET, Some(SHA)),
        ]);

        let evidence = provider.lookup(TARGET).unwrap().unwrap();

        assert_eq!(
            (evidence.number, evidence.state),
            (4, PrState::Open),
            "{flavor:?}"
        );
    }
}

#[test]
#[serial]
fn closed_unmerged_and_declined_prs_are_never_evidence() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![pr(
            flavor,
            5,
            Lifecycle::ClosedUnmerged,
            TARGET,
            Some(SHA),
        )]);

        assert_eq!(provider.lookup(TARGET), Ok(None), "{flavor:?}");
    }
}

#[test]
#[serial]
fn a_fork_pr_from_a_same_named_branch_does_not_match() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![pr(
            flavor,
            9,
            Lifecycle::Open,
            "forker/project",
            Some(SHA),
        )]);

        assert_eq!(provider.lookup(TARGET), Ok(None), "{flavor:?}");
    }
}

#[test]
#[serial]
fn a_fork_source_matches_only_its_own_repository() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        // The target's PR has the higher number, which would win a tie, so
        // only the source-repository match can select the fork's PR.
        provider.serve_list(vec![
            pr(flavor, 2, Lifecycle::Open, TARGET, Some(SHA)),
            pr(flavor, 1, Lifecycle::Open, "forker/project", Some(SHA)),
        ]);
        if flavor == ApiFlavor::GitLab {
            provider.mount(
                Mock::given(method("GET"))
                    .and(path_regex(r"^/api/projects/forker(%2F|/)project$"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": 2}))),
            );
        }

        let evidence = provider.lookup("forker/project").unwrap().unwrap();

        assert_eq!(evidence.number, 1, "{flavor:?}");
        assert_eq!(evidence.source_repo, "forker/project", "{flavor:?}");
    }
}

#[test]
#[serial]
fn a_missing_head_sha_stays_none_on_the_evidence() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![pr(flavor, 6, Lifecycle::Merged, TARGET, None)]);

        let evidence = provider.lookup(TARGET).unwrap().unwrap();

        assert_eq!(evidence.number, 6, "{flavor:?}");
        assert_eq!(evidence.source_head_sha, None, "{flavor:?}");
    }
}

#[test]
#[serial]
fn credentials_failures_are_unavailable_on_every_provider() {
    check_credential_cases(Provider::serve_list_response, |provider| {
        provider.lookup(TARGET)
    });
}

#[test]
#[serial]
fn a_list_404_is_unavailable_not_an_empty_answer() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list_response(ResponseTemplate::new(404));

        let result = provider.lookup(TARGET);

        assert!(
            matches!(result, Err(PrUnavailable::NotFoundOrNotPermitted { .. })),
            "{flavor:?}: {result:?}"
        );
    }
}

#[test]
#[serial]
fn a_gitlab_fork_project_404_is_unavailable() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::GitLab);
    provider.mount(
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/projects/forker(%2F|/)project$"))
            .respond_with(ResponseTemplate::new(404)),
    );

    let result = provider.lookup("forker/project");

    assert!(
        matches!(result, Err(PrUnavailable::NotFoundOrNotPermitted { .. })),
        "{result:?}"
    );
}

#[test]
#[serial]
fn a_response_slower_than_the_deadline_times_out() {
    let _tokens = without_tokens();
    let deadline = Duration::from_millis(300);
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list_response(
            ResponseTemplate::new(200)
                .set_body_json(list_body(flavor, Vec::new()))
                .set_delay(Duration::from_secs(10)),
        );

        let started = Instant::now();
        let result = provider.lookup_within(TARGET, deadline);

        assert_eq!(
            result,
            Err(PrUnavailable::Timeout { deadline }),
            "{flavor:?}"
        );
        // Well under the focused client's 5 s default, which the deadline
        // must replace rather than add to.
        assert!(started.elapsed() < Duration::from_secs(3), "{flavor:?}");
    }
}

#[test]
#[serial]
fn each_provider_receives_its_server_side_branch_filter() {
    let _tokens = without_tokens();
    let pair = |key: &str, value: &str| (key.to_string(), value.to_string());
    for (flavor, expected) in [
        (
            ApiFlavor::GitHub,
            vec![
                pair("head", "acme:feature"),
                pair("state", "all"),
                pair("per_page", "100"),
            ],
        ),
        (
            ApiFlavor::GitLab,
            vec![
                pair("source_branch", "feature"),
                pair("state", "all"),
                pair("per_page", "100"),
            ],
        ),
        (
            ApiFlavor::Gitea,
            vec![pair("state", "all"), pair("limit", "50")],
        ),
        (
            ApiFlavor::Bitbucket,
            vec![
                pair("q", "source.branch.name=\"feature\""),
                pair("state", "OPEN"),
                pair("state", "MERGED"),
                pair("pagelen", "50"),
            ],
        ),
    ] {
        let provider = Provider::start(flavor);
        provider.serve_list(Vec::new());

        assert_eq!(provider.lookup(TARGET), Ok(None), "{flavor:?}");

        let queries = provider.received_queries();
        assert_eq!(queries.len(), 1, "{flavor:?}");
        for expected_pair in &expected {
            assert!(
                queries[0].contains(expected_pair),
                "{flavor:?} missing {expected_pair:?} in {:?}",
                queries[0]
            );
        }
        assert!(
            flavor != ApiFlavor::Gitea || !queries[0].iter().any(|(key, _)| key == "per_page"),
            "Gitea ignores per_page"
        );
    }
}

#[test]
#[serial]
fn gitea_pages_with_limit_until_the_matching_pr() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::Gitea);
    let unrelated = (100..150)
        .map(|number| {
            let mut row = pr(ApiFlavor::Gitea, number, Lifecycle::Open, TARGET, Some(SHA));
            row["head"]["label"] = json!(format!("other-{number}"));
            row
        })
        .collect::<Vec<_>>();
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .and(wiremock::matchers::query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Value::Array(unrelated))),
    );
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .and(wiremock::matchers::query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([pr(
                ApiFlavor::Gitea,
                7,
                Lifecycle::Merged,
                TARGET,
                Some(SHA)
            )]))),
    );

    let evidence = provider.lookup(TARGET).unwrap().unwrap();

    assert_eq!(evidence.number, 7);
    assert_eq!(provider.received_queries().len(), 2);
}

#[test]
#[serial]
fn a_bitbucket_abbreviated_hash_is_preserved_exactly() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::Bitbucket);
    provider.serve_list(vec![pr(
        ApiFlavor::Bitbucket,
        11,
        Lifecycle::Merged,
        TARGET,
        Some("0123456789ab"),
    )]);

    let evidence = provider.lookup(TARGET).unwrap().unwrap();

    assert_eq!(evidence.source_head_sha.as_deref(), Some("0123456789ab"));
}

#[test]
#[serial]
fn a_gitea_merged_pr_takes_its_sha_and_branch_from_the_list_payload() {
    let _tokens = without_tokens();
    let frozen = "1111111111111111111111111111111111111111";
    let provider = Provider::start(ApiFlavor::Gitea);
    // After the merge the branch was deleted, so `head.ref` names the PR ref
    // and only `head.label` still carries the branch.
    let mut merged = pr(
        ApiFlavor::Gitea,
        12,
        Lifecycle::Merged,
        TARGET,
        Some(frozen),
    );
    merged["head"]["ref"] = json!("refs/pull/12/head");
    provider.serve_list(vec![merged]);
    // The single-PR endpoint reports the moved live tip; it must not be read.
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls/12"))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr(
                ApiFlavor::Gitea,
                12,
                Lifecycle::Merged,
                TARGET,
                Some("2222222222222222222222222222222222222222"),
            )))
            .expect(0),
    );

    let evidence = provider.lookup(TARGET).unwrap().unwrap();

    assert_eq!(evidence.state, PrState::Merged);
    assert_eq!(evidence.source_head_sha.as_deref(), Some(frozen));
}
