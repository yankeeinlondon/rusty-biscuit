#![cfg(feature = "remote")]

use biscuit_file::FetchPolicy;
use serial_test::serial;
use sniff::SniffError;
use sniff::filesystem::git::{ApiFlavor, RemoteEndpoint, ResolvedRemote, resolve_remote_at};
use sniff::remote::{
    CiCdJobQuery, CiCdJobReference, FocusedProviderClient, GitProvider, PullRequestQuery,
    CanonicalPullRequestState, QueryValues,
};
use test_toolkit::EnvGuard;
use wiremock::matchers::{header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Provider page size the client always requests, so a page of exactly this
/// many items is what "the provider may still have more" looks like on the wire.
const PAGE_SIZE: u64 = 100;

/// Renders `n` as an increasing RFC 3339 instant whose lexicographic order
/// matches its numeric order, so fixture IDs double as an ordering oracle.
fn ts(n: u64) -> String {
    format!("2024-01-01T{:02}:{:02}:00Z", n / 60, n % 60)
}

fn pr_item(number: u64, author: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": format!("pr {number}"),
        "state": "open",
        "user": {"login": author},
        "created_at": ts(number),
        "html_url": format!("https://127.0.0.1/pr/{number}"),
    })
}

fn pr_page(numbers: std::ops::RangeInclusive<u64>, author: &str) -> serde_json::Value {
    serde_json::Value::Array(numbers.map(|number| pr_item(number, author)).collect())
}

fn job_item(id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": format!("job {id}"),
        "status": "success",
        "pipeline_id": 1,
        "created_at": ts(id),
    })
}

fn gitlab_job_page(ids: std::ops::RangeInclusive<u64>) -> serde_json::Value {
    serde_json::Value::Array(ids.map(job_item).collect())
}

fn ids(items: impl IntoIterator<Item = String>) -> Vec<String> {
    items.into_iter().collect()
}

/// Reads the `state` query pairs the server actually received.
///
/// Bitbucket encodes multiple states as repeated pairs, so this deliberately
/// preserves both repetition and order rather than collapsing to one value.
async fn recorded_state_params(server: &MockServer) -> Vec<String> {
    let requests = server.received_requests().await.unwrap();
    let request = requests.first().expect("no request reached the provider");
    request
        .url
        .query_pairs()
        .filter(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .collect()
}

/// Host every fixture in this file lives on.
///
/// Two independent controls key off it: the API endpoint a client may contact
/// must belong to the repository's host, and a projected web link must too. A
/// fixture that publishes links elsewhere therefore proves nothing — the link
/// would be correctly dropped and the assertion would pass vacuously — so the
/// loopback provider serves its web UI here as well.
const FIXTURE_HOST: &str = "127.0.0.1";

fn remote(flavor: ApiFlavor) -> ResolvedRemote {
    ResolvedRemote {
        name: "origin".to_string(),
        fetch_url: "git@127.0.0.1:acme/project.git".to_string(),
        push_url: "git@127.0.0.1:acme/project.git".to_string(),
        host: Some(FIXTURE_HOST.to_string()),
        namespace: Some("acme".to_string()),
        repository: Some("project".to_string()),
        api_flavor: flavor,
        endpoint: None,
    }
}

fn provider(flavor: ApiFlavor) -> GitProvider {
    match flavor {
        ApiFlavor::GitHub => GitProvider::GitHub,
        ApiFlavor::GitLab => GitProvider::GitLab,
        ApiFlavor::Gitea | ApiFlavor::Forgejo => GitProvider::Gitea,
        ApiFlavor::Bitbucket => GitProvider::Bitbucket,
        _ => unreachable!(),
    }
}

fn client(server: &MockServer, flavor: ApiFlavor) -> FocusedProviderClient {
    let remote = remote(flavor);
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
    let api_base = format!("{}/api", server.uri());
    if flavor == ApiFlavor::Gitea {
        FocusedProviderClient::with_api_base_and_server_version(
            remote,
            policy,
            &api_base,
            "1.25.0",
        )
        .unwrap()
    } else {
        FocusedProviderClient::with_api_base(remote, policy, &api_base).unwrap()
    }
}

fn reference(flavor: ApiFlavor, id: &str) -> CiCdJobReference {
    CiCdJobReference {
        provider: provider(flavor),
        api_flavor: format!("{flavor:?}"),
        host: "example.com".to_string(),
        namespace: "acme".to_string(),
        repository: "project".to_string(),
        native_id: id.to_string(),
        display_id: id.to_string(),
        original_url: None,
    }
}

/// Repository identity a canonical URL is expected to resolve to.
///
/// Compared as a whole so a route that recovers the right ID but the wrong
/// flavor, namespace, or host fails loudly instead of passing on the ID alone.
#[derive(Debug, PartialEq, Eq)]
struct UrlIdentity {
    flavor: ApiFlavor,
    host: String,
    namespace: String,
    repository: String,
    native_id: String,
}

fn pr_identity(url: &str) -> Result<UrlIdentity, SniffError> {
    let (client, native_id) =
        FocusedProviderClient::from_pull_request_url(url, FetchPolicy::deny_all())?;
    Ok(identity_of(client.remote(), &native_id))
}

fn job_identity(url: &str) -> Result<UrlIdentity, SniffError> {
    let (remote, reference) = FocusedProviderClient::job_reference_from_url(url)?;
    assert_eq!(reference.original_url.as_deref(), Some(url));
    assert_eq!(reference.native_id, reference.display_id);
    Ok(identity_of(&remote, &reference.native_id))
}

fn identity_of(remote: &ResolvedRemote, native_id: &str) -> UrlIdentity {
    UrlIdentity {
        flavor: remote.api_flavor,
        host: remote.host.clone().unwrap_or_default(),
        namespace: remote.namespace.clone().unwrap_or_default(),
        repository: remote.repository.clone().unwrap_or_default(),
        native_id: native_id.to_string(),
    }
}

fn expected(flavor: ApiFlavor, host: &str, namespace: &str, native_id: &str) -> UrlIdentity {
    UrlIdentity {
        flavor,
        host: host.to_string(),
        namespace: namespace.to_string(),
        repository: "project".to_string(),
        native_id: native_id.to_string(),
    }
}

#[test]
fn canonical_web_urls_resolve_every_supported_provider() {
    for (url, wanted) in [
        ("https://github.com/acme/project/pull/7",
         expected(ApiFlavor::GitHub, "github.com", "acme", "7")),
        ("https://gitlab.com/group/sub/project/-/merge_requests/8",
         expected(ApiFlavor::GitLab, "gitlab.com", "group/sub", "8")),
        ("https://gitea.example/acme/project/pulls/9",
         expected(ApiFlavor::Gitea, "gitea.example", "acme", "9")),
        ("https://forgejo.example/acme/project/pulls/9",
         expected(ApiFlavor::Forgejo, "forgejo.example", "acme", "9")),
        ("https://codeberg.org/acme/project/pulls/9",
         expected(ApiFlavor::Forgejo, "codeberg.org", "acme", "9")),
        ("https://bitbucket.org/acme/project/pull-requests/10",
         expected(ApiFlavor::Bitbucket, "bitbucket.org", "acme", "10")),
    ] {
        assert_eq!(pr_identity(url).unwrap(), wanted, "{url}");
    }

    for (url, wanted) in [
        ("https://github.com/acme/project/actions/runs/20/job/21",
         expected(ApiFlavor::GitHub, "github.com", "acme", "21")),
        ("https://gitlab.com/group/sub/project/-/jobs/22",
         expected(ApiFlavor::GitLab, "gitlab.com", "group/sub", "22")),
        ("https://gitea.example/acme/project/actions/runs/20/jobs/23",
         expected(ApiFlavor::Gitea, "gitea.example", "acme", "23")),
        ("https://forgejo.example/acme/project/actions/runs/20/jobs/23",
         expected(ApiFlavor::Forgejo, "forgejo.example", "acme", "23")),
        ("https://bitbucket.org/acme/project/pipelines/results/p1/steps/s1",
         expected(ApiFlavor::Bitbucket, "bitbucket.org", "acme", "p1/s1")),
    ] {
        assert_eq!(job_identity(url).unwrap(), wanted, "{url}");
    }
}

/// The API half of the input contract: every supported provider's own API route
/// for both item kinds, including the official API hostnames, which must map
/// back to the repository's web host.
#[test]
fn canonical_api_urls_resolve_every_supported_provider() {
    for (url, wanted) in [
        ("https://api.github.com/repos/acme/project/pulls/7",
         expected(ApiFlavor::GitHub, "github.com", "acme", "7")),
        ("https://gitlab.com/api/v4/projects/group%2Fsub%2Fproject/merge_requests/8",
         expected(ApiFlavor::GitLab, "gitlab.com", "group/sub", "8")),
        ("https://gitea.example/api/v1/repos/acme/project/pulls/9",
         expected(ApiFlavor::Gitea, "gitea.example", "acme", "9")),
        ("https://forgejo.example/api/v1/repos/acme/project/pulls/9",
         expected(ApiFlavor::Forgejo, "forgejo.example", "acme", "9")),
        ("https://api.bitbucket.org/2.0/repositories/acme/project/pullrequests/10",
         expected(ApiFlavor::Bitbucket, "bitbucket.org", "acme", "10")),
    ] {
        assert_eq!(pr_identity(url).unwrap(), wanted, "{url}");
    }

    for (url, wanted) in [
        ("https://api.github.com/repos/acme/project/actions/jobs/21",
         expected(ApiFlavor::GitHub, "github.com", "acme", "21")),
        ("https://gitlab.com/api/v4/projects/group%2Fsub%2Fproject/jobs/22",
         expected(ApiFlavor::GitLab, "gitlab.com", "group/sub", "22")),
        ("https://gitea.example/api/v1/repos/acme/project/actions/jobs/23",
         expected(ApiFlavor::Gitea, "gitea.example", "acme", "23")),
        ("https://forgejo.example/api/v1/repos/acme/project/actions/jobs/23",
         expected(ApiFlavor::Forgejo, "forgejo.example", "acme", "23")),
        ("https://api.bitbucket.org/2.0/repositories/acme/project/pipelines/p1/steps/s1",
         expected(ApiFlavor::Bitbucket, "bitbucket.org", "acme", "p1/s1")),
    ] {
        assert_eq!(job_identity(url).unwrap(), wanted, "{url}");
    }
}

/// Enterprise and self-managed endpoints keep the origin the caller addressed
/// them by, which is what lets the derived API base reach a non-default port.
#[test]
fn enterprise_and_self_managed_urls_retain_scheme_and_non_default_port() {
    for (url, flavor, host, namespace) in [
        ("https://ghe.example:8443/api/v3/repos/acme/project/pulls/7", ApiFlavor::GitHub, "ghe.example", "acme"),
        ("http://ghe.example:8080/acme/project/pull/7", ApiFlavor::GitHub, "ghe.example", "acme"),
        ("https://git.example:8443/api/v4/projects/group%2Fproject/merge_requests/8", ApiFlavor::GitLab, "git.example", "group"),
        ("http://git.example:8080/group/project/-/merge_requests/8", ApiFlavor::GitLab, "git.example", "group"),
        ("https://gitea.example:3000/api/v1/repos/acme/project/pulls/9", ApiFlavor::Gitea, "gitea.example", "acme"),
        ("https://forgejo.example:3000/acme/project/pulls/9", ApiFlavor::Forgejo, "forgejo.example", "acme"),
    ] {
        let (client, _) =
            FocusedProviderClient::from_pull_request_url(url, FetchPolicy::deny_all()).unwrap();
        let remote = client.remote();
        let parsed = url::Url::parse(url).unwrap();
        assert_eq!(remote.api_flavor, flavor, "{url}");
        assert_eq!(remote.host.as_deref(), Some(host), "{url}");
        assert_eq!(remote.namespace.as_deref(), Some(namespace), "{url}");
        let endpoint = remote.endpoint.as_ref().unwrap();
        assert_eq!(endpoint.scheme, parsed.scheme(), "{url}");
        assert_eq!(endpoint.port, parsed.port(), "{url}");
        assert_eq!(
            remote.http_origin().as_deref(),
            Some(parsed.origin().ascii_serialization().as_str()),
            "{url}"
        );
    }

    let (remote, _) = FocusedProviderClient::job_reference_from_url(
        "https://git.example:8443/api/v4/projects/group%2Fproject/jobs/22",
    )
    .unwrap();
    assert_eq!(remote.http_origin().as_deref(), Some("https://git.example:8443"));
}

/// A host that pins a provider accepts only that provider's routes, so a route
/// shape borrowed from another forge is rejected rather than mis-flavored.
#[test]
fn cross_flavor_route_shapes_are_rejected() {
    for url in [
        // GitLab shapes on GitHub and Bitbucket hosts.
        "https://github.com/group/project/-/merge_requests/8",
        "https://github.com/api/v4/projects/group%2Fproject/merge_requests/8",
        "https://bitbucket.org/group/project/-/merge_requests/8",
        // GitHub shapes on GitLab and Bitbucket hosts.
        "https://gitlab.com/acme/project/pull/7",
        "https://bitbucket.org/acme/project/pull/7",
        // Gitea's `/pulls/` shape on GitHub, GitLab, and Bitbucket hosts.
        "https://github.com/acme/project/pulls/7",
        "https://gitlab.com/api/v1/repos/acme/project/pulls/9",
        "https://bitbucket.org/acme/project/pulls/9",
        // Bitbucket's API shape on the GitHub API host.
        "https://api.github.com/2.0/repositories/acme/project/pullrequests/10",
        // GitHub's shape on Codeberg, which is definitively Forgejo.
        "https://codeberg.org/acme/project/pull/7",
    ] {
        assert!(pr_identity(url).is_err(), "expected rejection: {url}");
    }

    for url in [
        "https://github.com/group/project/-/jobs/22",
        "https://gitlab.com/acme/project/actions/runs/20/job/21",
        "https://bitbucket.org/acme/project/actions/runs/20/jobs/23",
        "https://api.github.com/2.0/repositories/acme/project/pipelines/p1/steps/s1",
    ] {
        assert!(job_identity(url).is_err(), "expected rejection: {url}");
    }
}

/// The two item kinds do not share a route grammar, so asking for a job with a
/// PR URL is a parse failure rather than a lookup that later 404s.
#[test]
fn item_kinds_do_not_accept_each_others_routes() {
    for url in [
        "https://github.com/acme/project/pull/7",
        "https://api.github.com/repos/acme/project/pulls/7",
        "https://gitlab.com/api/v4/projects/group%2Fproject/merge_requests/8",
        "https://bitbucket.org/acme/project/pull-requests/10",
    ] {
        assert!(job_identity(url).is_err(), "expected rejection: {url}");
    }
    for url in [
        "https://github.com/acme/project/actions/runs/20/job/21",
        "https://api.github.com/repos/acme/project/actions/jobs/21",
        "https://gitlab.com/api/v4/projects/group%2Fproject/jobs/22",
        "https://bitbucket.org/acme/project/pipelines/results/p1/steps/s1",
    ] {
        assert!(pr_identity(url).is_err(), "expected rejection: {url}");
    }
}

#[test]
fn malformed_provider_urls_are_rejected() {
    for url in [
        // Not a supported route at all.
        "https://github.com/acme/project/issues/7",
        "not-a-url",
        // Transport and addressing the contract forbids.
        "ftp://github.com/acme/project/pull/7",
        "https://github.com/acme/project/pull/7?state=open",
        "https://github.com/acme/project/pull/7#discussion",
        // Truncated or over-long routes.
        "https://api.github.com/repos/acme/project/pulls",
        "https://api.github.com/repos/acme/project/pulls/7/files",
        "https://github.com/acme/project/pull",
        "https://gitlab.com/project/-/merge_requests/8",
        "https://gitlab.com/group/project/merge_requests/8",
        // A GitLab API project identity that decodes to no namespace at all.
        "https://gitlab.com/api/v4/projects/project/merge_requests/8",
        // An encoded separator smuggled into a flat owner/repository identity.
        "https://api.github.com/repos/acme%2Fevil/project/pulls/7",
        // Zero is not a provider identifier.
        "https://api.github.com/repos/acme/project/pulls/0",
        "https://github.com/acme/project/pull/0",
    ] {
        assert!(pr_identity(url).is_err(), "expected rejection: {url}");
    }

    for url in [
        "https://api.github.com/repos/acme/project/actions/jobs",
        // A web route under an API prefix is not an API route.
        "https://api.github.com/repos/acme/project/actions/runs/20/job/21",
        "https://gitlab.com/api/v4/projects/group%2Fproject/jobs",
        "https://bitbucket.org/acme/project/pipelines/results/p1/steps",
        "https://gitea.example/acme/project/actions/runs/20/jobs/0",
    ] {
        assert!(job_identity(url).is_err(), "expected rejection: {url}");
    }
}

/// Percent-decoding is part of canonical-reference parsing, so validation must
/// run on the decoded value rather than only on the URL parser's segment text.
#[test]
fn encoded_delimiters_controls_and_dot_segments_are_malformed_references() {
    let pull_request_urls = [
        "https://api.github.com/repos/acme/project%3Fadmin/pulls/7",
        "https://api.github.com/repos/acme/project%23fragment/pulls/7",
        "https://api.github.com/repos/acme/project%5Cchild/pulls/7",
        "https://api.github.com/repos/acme/project%01control/pulls/7",
        "https://api.github.com/repos/%2E/project/pulls/7",
        "https://api.github.com/repos/%2E%2E/project/pulls/7",
        "https://gitlab.com/api/v4/projects/group%2Fproject%3Fadmin/merge_requests/8",
        "https://bitbucket.org/acme/project/pull-requests/10%23fragment",
    ];
    for url in pull_request_urls {
        let error = pr_identity(url).unwrap_err();
        assert!(
            matches!(
                error,
                SniffError::InvalidRemoteQuery { field: "id", ref message }
                    if message.contains("canonical")
            ),
            "expected an actionable malformed-reference error for {url}: {error}"
        );
    }

    let job_urls = [
        "https://api.github.com/repos/acme/project/actions/jobs/21%3Fadmin",
        "https://gitlab.com/api/v4/projects/group%2Fproject%23fragment/jobs/22",
        "https://gitea.example/acme%5Cchild/project/actions/runs/20/jobs/23",
        "https://forgejo.example/acme/project%00control/actions/runs/20/jobs/23",
        "https://bitbucket.org/acme/project/pipelines/results/%2E/steps/s1",
        "https://bitbucket.org/acme/project/pipelines/results/p1/steps/%2E%2E",
    ];
    for url in job_urls {
        let error = job_identity(url).unwrap_err();
        assert!(
            matches!(
                error,
                SniffError::InvalidRemoteQuery { field: "id", ref message }
                    if message.contains("canonical")
            ),
            "expected an actionable malformed-reference error for {url}: {error}"
        );
    }
}

#[test]
fn unicode_repository_identities_survive_canonical_reference_parsing() {
    assert_eq!(
        pr_identity("https://gitea.example/%C3%A9quipe/r%C3%A9sum%C3%A9/pulls/7").unwrap(),
        UrlIdentity {
            flavor: ApiFlavor::Gitea,
            host: "gitea.example".to_string(),
            namespace: "équipe".to_string(),
            repository: "résumé".to_string(),
            native_id: "7".to_string(),
        }
    );
    assert_eq!(
        job_identity(
            "https://gitlab.com/api/v4/projects/%E7%A0%94%E7%A9%B6%2F%E6%9E%84%E5%BB%BA/jobs/22"
        )
        .unwrap(),
        UrlIdentity {
            flavor: ApiFlavor::GitLab,
            host: "gitlab.com".to_string(),
            namespace: "研究".to_string(),
            repository: "构建".to_string(),
            native_id: "22".to_string(),
        }
    );
}

/// An official API hostname resolves to the repository's web host, so the
/// derived endpoint reaches the API host through the allowlist rather than by
/// the remote host having been rewritten.
#[test]
fn official_api_hostnames_resolve_to_the_repository_web_host() {
    for (url, api_host, web_host) in [
        ("https://api.github.com/repos/acme/project/pulls/7", "api.github.com", "github.com"),
        ("https://api.bitbucket.org/2.0/repositories/acme/project/pullrequests/10",
         "api.bitbucket.org", "bitbucket.org"),
    ] {
        let (client, _) =
            FocusedProviderClient::from_pull_request_url(url, FetchPolicy::deny_all()).unwrap();
        assert_eq!(client.remote().host.as_deref(), Some(web_host), "{url}");
        assert_ne!(client.remote().host.as_deref(), Some(api_host), "{url}");
    }
}

#[tokio::test]
async fn exact_pull_requests_preserve_identity_and_authoritative_not_found() {
    let cases = [
        (ApiFlavor::GitHub, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://127.0.0.1/pr/7", "url": "https://api.example/pr/7"})),
        (ApiFlavor::GitLab, "/api/projects/acme%2Fproject/merge_requests/7", "/api/projects/acme%2Fproject/merge_requests/8", serde_json::json!({"iid": 7, "title": "Fix", "state": "opened", "author": {"username": "alice"}, "created_at": "2024-01-01", "web_url": "https://127.0.0.1/mr/7"})),
        (ApiFlavor::Gitea, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://127.0.0.1/pr/7"})),
        (ApiFlavor::Forgejo, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://127.0.0.1/pr/7"})),
        (ApiFlavor::Bitbucket, "/api/repositories/acme/project/pullrequests/7", "/api/repositories/acme/project/pullrequests/8", serde_json::json!({"id": 7, "title": "Fix", "state": "OPEN", "author": {"display_name": "alice"}, "created_on": "2024-01-01", "links": {"html": {"href": "https://127.0.0.1/pr/7"}}})),
    ];
    for (flavor, found_path, missing_path, body) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path(found_path)).respond_with(ResponseTemplate::new(200).set_body_json(body)).expect(1).mount(&server).await;
        Mock::given(method("GET")).and(path(missing_path)).respond_with(ResponseTemplate::new(404)).expect(1).mount(&server).await;
        let adapter = client(&server, flavor);
        let record = adapter.get_pull_request("7").await.unwrap().unwrap();
        assert_eq!(record.identity.native_id, "7");
        assert_eq!(record.identity.namespace, "acme");
        assert_eq!(record.identity.repository, "project");
        assert_eq!(record.details.author, "alice");
        assert!(adapter.get_pull_request("8").await.unwrap().is_none());
    }
}

/// Request construction re-encodes every identity segment independently. This
/// is a second boundary behind canonical parsing because repository identities
/// can also come from configured Git remotes or direct library construction.
#[tokio::test]
async fn exact_and_list_paths_encode_unicode_and_structural_identity_bytes() {
    for (namespace, repository, encoded_namespace, encoded_repository) in [
        ("équipe", "résumé", "%C3%A9quipe", "r%C3%A9sum%C3%A9"),
        (
            "acme?owner=#root\\\u{1}",
            "project#fragment?x=1\\\u{2}",
            "acme%3Fowner%3D%23root%5C%01",
            "project%23fragment%3Fx%3D1%5C%02",
        ),
        (".", "..", "%252E", "%252E%252E"),
    ] {
        let server = MockServer::start().await;
        let exact_id = "7?state=closed#fragment\\\u{3}";
        let encoded_id = "7%3Fstate%3Dclosed%23fragment%5C%03";
        let repository_base = format!("repos/{encoded_namespace}/{encoded_repository}");

        Mock::given(method("GET"))
            .and(path(format!("/api/{repository_base}/pulls/{encoded_id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(pr_item(7, "alice")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/api/{repository_base}/pulls")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/api/{repository_base}/actions/jobs/{encoded_id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(job_item(7)))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/api/{repository_base}/actions/jobs")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jobs": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        let mut identity_remote = remote(ApiFlavor::Gitea);
        identity_remote.namespace = Some(namespace.to_string());
        identity_remote.repository = Some(repository.to_string());
        let adapter = FocusedProviderClient::with_api_base_and_server_version(
            identity_remote,
            FetchPolicy::deny_all().allow_host(FIXTURE_HOST),
            &format!("{}/api", server.uri()),
            "1.25.0",
        )
        .unwrap();

        assert!(adapter.get_pull_request(exact_id).await.unwrap().is_some());
        assert!(
            adapter
                .query_pull_requests(PullRequestQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        let exact_reference = CiCdJobReference {
            native_id: exact_id.to_string(),
            display_id: exact_id.to_string(),
            ..reference(ApiFlavor::Gitea, exact_id)
        };
        assert!(adapter.get_cicd_job(&exact_reference).await.unwrap().is_some());
        assert!(
            adapter
                .query_cicd_jobs(CiCdJobQuery::default())
                .await
                .unwrap()
                .items
                .is_empty()
        );

        for request in server.received_requests().await.unwrap() {
            assert!(
                request.url.path().starts_with("/api/repos/"),
                "identity bytes retargeted the path: {}",
                request.url
            );
            assert_ne!(request.url.fragment(), Some("fragment"));
            assert!(
                request.url.query_pairs().all(|(key, value)| {
                    !(key == "state" && value == "closed") && key != "x"
                }),
                "identity bytes retargeted the query: {}",
                request.url
            );
        }
    }
}

#[tokio::test]
async fn exact_jobs_are_normalized_for_every_supported_flavor() {
    let cases = [
        (ApiFlavor::GitHub, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "completed", "conclusion": "success", "run_id": 1})),
        (ApiFlavor::GitLab, "10", "/api/projects/acme%2Fproject/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "pipeline_id": 1})),
        (ApiFlavor::Gitea, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "run_id": 1})),
        (ApiFlavor::Bitbucket, "parent/step", "/api/repositories/acme/project/pipelines/parent/steps/step", serde_json::json!({"uuid": "step", "name": "test", "state": {"name": "COMPLETED", "result": {"name": "SUCCESSFUL"}}})),
    ];
    for (flavor, id, endpoint, body) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let job = client(&server, flavor).get_cicd_job(&reference(flavor, id)).await.unwrap().unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.normalized_status, "success");
        assert!(!job.parent.native_id.is_empty());
    }
}

#[tokio::test]
async fn job_listing_uses_each_supported_flavor_strategy() {
    for flavor in [ApiFlavor::GitHub, ApiFlavor::GitLab, ApiFlavor::Gitea, ApiFlavor::Bitbucket] {
        let server = MockServer::start().await;
        if flavor == ApiFlavor::GitLab {
            Mock::given(method("GET"))
                .and(path("/api/projects/acme%2Fproject/jobs"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"id": 10, "name": "test", "status": "success", "pipeline_id": 1}
                ])))
                .expect(1)
                .mount(&server)
                .await;
        } else if flavor == ApiFlavor::Gitea {
            Mock::given(method("GET"))
                .and(path("/api/repos/acme/project/actions/jobs"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "jobs": [{"id": 10, "name": "test", "status": "success", "run_id": 1}]
                })))
                .expect(1)
                .mount(&server)
                .await;
        } else {
            let (parents_path, jobs_path, parent_body, jobs_body) = if flavor == ApiFlavor::Bitbucket {
                ("/api/repositories/acme/project/pipelines", "/api/repositories/acme/project/pipelines/p1/steps", serde_json::json!({"values": [{"uuid": "p1"}]}), serde_json::json!({"values": [{"uuid": "s1", "name": "test", "state": {"name": "COMPLETED", "result": {"name": "SUCCESSFUL"}}}]}))
            } else {
                ("/api/repos/acme/project/actions/runs", "/api/repos/acme/project/actions/runs/1/jobs", serde_json::json!({"workflow_runs": [{"id": 1, "name": "CI"}]}), serde_json::json!({"jobs": [{"id": 10, "name": "test", "status": "success"}]}))
            };
            Mock::given(method("GET")).and(path(parents_path)).respond_with(ResponseTemplate::new(200).set_body_json(parent_body)).expect(1).mount(&server).await;
            Mock::given(method("GET")).and(path(jobs_path)).respond_with(ResponseTemplate::new(200).set_body_json(jobs_body)).expect(1).mount(&server).await;
        }
        let page = client(&server, flavor).query_cicd_jobs(CiCdJobQuery { limit: Some(2), ..Default::default() }).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "test");
    }
}

/// A GitHub Actions job as the jobs endpoint actually returns it.
///
/// Runner identity is the two flat `runner_*` keys, never a nested `runner`
/// object, and the job carries no `event` and no actor at all — those exist only
/// on the workflow run. Only the link hosts depart from a verbatim capture:
/// they sit on [`FIXTURE_HOST`] because a link off the repository's own host is
/// dropped before any assertion can see it.
fn github_actions_job() -> serde_json::Value {
    serde_json::json!({
        "id": 399444496,
        "run_id": 29679449,
        "run_attempt": 1,
        "url": "https://api.github.com/repos/acme/project/actions/jobs/399444496",
        "html_url": "https://127.0.0.1/acme/project/actions/runs/29679449/job/399444496",
        "head_sha": "f83a4d2b8e5c1907cafe1234567890abcdef0123",
        "status": "completed",
        "conclusion": "failure",
        "created_at": "2024-01-01T00:10:00Z",
        "started_at": "2024-01-01T00:11:00Z",
        "completed_at": "2024-01-01T00:19:00Z",
        "name": "build (ubuntu-latest)",
        "workflow_name": "CI",
        "labels": ["ubuntu-latest"],
        "runner_id": 7,
        "runner_name": "GitHub Actions 7",
        "runner_group_id": 2,
        "runner_group_name": "GitHub Actions",
        "steps": []
    })
}

/// A GitHub workflow run — the only place `head_branch`, `head_sha`, `event`,
/// and the triggering actor exist for the jobs beneath it.
fn github_workflow_run() -> serde_json::Value {
    serde_json::json!({
        "id": 29679449,
        "name": "CI",
        "head_branch": "release/2.1",
        "head_sha": "f83a4d2b8e5c1907cafe1234567890abcdef0123",
        "event": "pull_request",
        "status": "completed",
        "conclusion": "failure",
        "html_url": "https://127.0.0.1/acme/project/actions/runs/29679449",
        "actor": {"login": "alice"},
        "triggering_actor": {"login": "bob"},
        "created_at": "2024-01-01T00:09:00Z"
    })
}

/// A GitLab job with no top-level `sha` and a nameless runner.
///
/// Both omissions are the norm rather than an edge case: GitLab puts the commit
/// under `commit.id`, and self-hosted runners commonly register with an empty
/// `name` and a populated `description`.
fn gitlab_job() -> serde_json::Value {
    serde_json::json!({
        "id": 7742,
        "name": "rspec:linux",
        "stage": "test",
        "status": "failed",
        "ref": "release/2.1",
        "tag": false,
        "allow_failure": false,
        "created_at": "2024-01-01T00:10:00.727Z",
        "started_at": "2024-01-01T00:11:00.722Z",
        "finished_at": "2024-01-01T00:19:00.921Z",
        "duration": 480.4,
        "user": {"id": 3, "name": "Alice Example", "username": "alice"},
        "commit": {
            "id": "0ff3ae198f8601a285adcf5c0fff204ee6fba5fd",
            "short_id": "0ff3ae19",
            "title": "Test the CI integration."
        },
        "pipeline": {
            "id": 6601,
            "project_id": 1,
            "ref": "release/2.1",
            "sha": "0ff3ae198f8601a285adcf5c0fff204ee6fba5fd",
            "source": "merge_request_event",
            "status": "failed"
        },
        "web_url": "https://127.0.0.1/acme/project/-/jobs/7742",
        "runner": {"id": 32, "description": "shared-runner-linux-01", "name": null, "active": true},
        "failure_reason": "script_failure"
    })
}

/// A Bitbucket pipeline step: its lifecycle is an object, and it holds no
/// branch, commit, trigger, or actor of its own.
fn bitbucket_step() -> serde_json::Value {
    serde_json::json!({
        "uuid": "{s1}",
        "name": "Build and test",
        "state": {
            "name": "COMPLETED",
            "type": "pipeline_step_state_completed",
            "result": {"name": "FAILED", "type": "pipeline_step_state_completed_failed"}
        },
        "created_on": "2024-01-01T00:10:00.000Z",
        "started_on": "2024-01-01T00:11:00.000Z",
        "completed_on": "2024-01-01T00:19:00.000Z",
        "duration_in_seconds": 480,
        "pipeline": {"uuid": "{p1}", "type": "pipeline"},
        "runner": {"uuid": "{r1}", "name": "linux-runner-01", "labels": ["self.hosted", "linux"]},
        "links": {"self": {"href": "https://api.bitbucket.org/2.0/steps/s1"}}
    })
}

/// A Bitbucket pipeline — sole carrier of branch, commit, trigger, and actor.
fn bitbucket_pipeline() -> serde_json::Value {
    serde_json::json!({
        "uuid": "{p1}",
        "build_number": 42,
        "state": {"name": "COMPLETED", "result": {"name": "FAILED"}},
        "target": {
            "type": "pipeline_ref_target",
            "ref_type": "branch",
            "ref_name": "release/2.1",
            "commit": {"type": "commit", "hash": "0ff3ae198f8601a285adcf5c0fff204ee6fba5fd"}
        },
        "trigger": {"name": "PUSH", "type": "pipeline_trigger_push"},
        "creator": {"display_name": "Alice Example", "nickname": "alice"},
        "created_on": "2024-01-01T00:09:00.000Z",
        "links": {"html": {"href": "https://127.0.0.1/acme/project/pipelines/results/42"}}
    })
}

/// AC22/spec: the structured record must retain what each provider actually
/// exposes, including the fields no other provider spells the same way.
///
/// Every assertion here reads a field the previous single key-probing normalizer
/// could not reach: `commit.id` (GitLab keeps its top-level `sha` empty),
/// `finished_at`, `runner.description` behind a null `name`, `started_on` /
/// `completed_on`, the flat `runner_name`, and `state.result.name`.
#[tokio::test]
async fn exact_jobs_retain_every_field_the_record_promises() {
    // GitLab: nested commit, stage, full timestamp set, description-only runner.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/acme%2Fproject/jobs/7742"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_job()))
        .expect(1)
        .mount(&server)
        .await;
    let job = client(&server, ApiFlavor::GitLab)
        .get_cicd_job(&reference(ApiFlavor::GitLab, "7742"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.name, "rspec:linux");
    assert_eq!(job.stage.as_deref(), Some("test"));
    assert_eq!(job.normalized_status, "failed");
    assert_eq!(job.native_status, "failed");
    assert_eq!(job.parent.native_id, "6601");
    assert_eq!(job.branch.as_deref(), Some("release/2.1"));
    assert_eq!(
        job.commit.as_deref(),
        Some("0ff3ae198f8601a285adcf5c0fff204ee6fba5fd"),
        "GitLab's commit lives under commit.id, not a top-level sha"
    );
    assert_eq!(job.actor.as_deref(), Some("alice"));
    assert_eq!(job.trigger.as_deref(), Some("merge_request_event"));
    assert_eq!(job.created_at.as_deref(), Some("2024-01-01T00:10:00.727Z"));
    assert_eq!(job.started_at.as_deref(), Some("2024-01-01T00:11:00.722Z"));
    assert_eq!(
        job.finished_at.as_deref(),
        Some("2024-01-01T00:19:00.921Z"),
        "GitLab spells the terminal instant finished_at"
    );
    assert_eq!(
        job.web_url.as_deref(),
        Some("https://127.0.0.1/acme/project/-/jobs/7742")
    );
    assert_eq!(
        job.runner.as_deref(),
        Some("shared-runner-linux-01"),
        "a null runner.name must not shadow the description"
    );

    // Bitbucket: object-shaped state, result-derived verdict, *_on timestamps.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        // Bitbucket UUIDs are brace-wrapped, and braces are not path-safe, so the
        // request that actually reaches the provider is percent-encoded.
        .and(path("/api/repositories/acme/project/pipelines/%7Bp1%7D/steps/%7Bs1%7D"))
        .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_step()))
        .expect(1)
        .mount(&server)
        .await;
    let job = client(&server, ApiFlavor::Bitbucket)
        .get_cicd_job(&reference(ApiFlavor::Bitbucket, "{p1}/{s1}"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.name, "Build and test");
    assert_eq!(
        job.parent.native_id, "{p1}",
        "the parent/job identity must survive the exact lookup"
    );
    assert_eq!(job.native_status, "COMPLETED");
    assert_eq!(
        job.normalized_status, "failed",
        "a COMPLETED step that FAILED is not a success"
    );
    assert_eq!(job.conclusion.as_deref(), Some("FAILED"));
    assert_eq!(job.created_at.as_deref(), Some("2024-01-01T00:10:00.000Z"));
    assert_eq!(job.started_at.as_deref(), Some("2024-01-01T00:11:00.000Z"));
    assert_eq!(job.finished_at.as_deref(), Some("2024-01-01T00:19:00.000Z"));
    assert_eq!(job.runner.as_deref(), Some("linux-runner-01"));

    // GitHub/Gitea: flat runner_name, completed_at, conclusion-derived
    // verdict. Branch/commit/actor/trigger are absent without a parent run.
    for flavor in [ApiFlavor::GitHub, ApiFlavor::Gitea] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/actions/jobs/399444496"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_actions_job()))
            .expect(1)
            .mount(&server)
            .await;
        let job = client(&server, flavor)
            .get_cicd_job(&reference(flavor, "399444496"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(job.name, "build (ubuntu-latest)");
        assert_eq!(job.parent.native_id, "29679449");
        assert_eq!(job.native_status, "completed");
        assert_eq!(
            job.normalized_status, "failed",
            "{flavor:?} status=completed only says it stopped; the conclusion says how"
        );
        assert_eq!(job.conclusion.as_deref(), Some("failure"));
        assert_eq!(
            job.commit.as_deref(),
            Some("f83a4d2b8e5c1907cafe1234567890abcdef0123")
        );
        assert_eq!(job.created_at.as_deref(), Some("2024-01-01T00:10:00Z"));
        assert_eq!(job.started_at.as_deref(), Some("2024-01-01T00:11:00Z"));
        assert_eq!(
            job.finished_at.as_deref(),
            Some("2024-01-01T00:19:00Z"),
            "{flavor:?} spells the terminal instant completed_at"
        );
        assert_eq!(
            job.web_url.as_deref(),
            Some("https://127.0.0.1/acme/project/actions/runs/29679449/job/399444496")
        );
        assert_eq!(
            job.api_url.as_deref(),
            Some("https://api.github.com/repos/acme/project/actions/jobs/399444496")
        );
        assert_eq!(
            job.runner.as_deref(),
            Some("GitHub Actions 7"),
            "{flavor:?} exposes runner_name flat, not under a runner object"
        );
    }
}

/// Branch, commit, trigger, and actor exist only on the parent run/pipeline for
/// the traversal flavors, so they must reach the job the traversal already holds
/// the parent for — and must not be invented when neither object carries them.
#[tokio::test]
async fn parent_run_metadata_reaches_the_jobs_beneath_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [github_workflow_run()]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/29679449/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [github_actions_job()]
        })))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap();
    let job = &page.items[0];
    assert_eq!(job.parent.native_id, "29679449");
    assert_eq!(job.parent.name.as_deref(), Some("CI"));
    assert_eq!(
        job.parent.web_url.as_deref(),
        Some("https://127.0.0.1/acme/project/actions/runs/29679449")
    );
    assert_eq!(
        job.branch.as_deref(),
        Some("release/2.1"),
        "head_branch lives on the run, not the job"
    );
    assert_eq!(
        job.trigger.as_deref(),
        Some("pull_request"),
        "event lives on the run, not the job"
    );
    assert_eq!(
        job.actor.as_deref(),
        Some("bob"),
        "the triggering actor lives on the run, not the job"
    );
    assert_eq!(
        job.commit.as_deref(),
        Some("f83a4d2b8e5c1907cafe1234567890abcdef0123"),
        "the job's own head_sha must win over the run's"
    );

    // Bitbucket steps carry none of these, so the pipeline is the only source.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repositories/acme/project/pipelines"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "values": [bitbucket_pipeline()]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repositories/acme/project/pipelines/%7Bp1%7D/steps"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "values": [bitbucket_step()]
        })))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::Bitbucket)
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap();
    let job = &page.items[0];
    assert_eq!(job.parent.native_id, "{p1}");
    assert_eq!(job.branch.as_deref(), Some("release/2.1"));
    assert_eq!(
        job.commit.as_deref(),
        Some("0ff3ae198f8601a285adcf5c0fff204ee6fba5fd")
    );
    assert_eq!(job.trigger.as_deref(), Some("PUSH"));
    assert_eq!(job.actor.as_deref(), Some("alice"));
    assert_eq!(job.started_at.as_deref(), Some("2024-01-01T00:11:00.000Z"));
    assert_eq!(job.finished_at.as_deref(), Some("2024-01-01T00:19:00.000Z"));
}

/// Inheritance must not become fabrication: a field neither object carries stays
/// `None`, and a job-level value is never replaced by the parent's.
#[tokio::test]
async fn absent_metadata_is_not_invented_from_the_parent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [{"id": 5, "name": "CI"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/5/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [{
                "id": 11,
                "name": "lint",
                "status": "in_progress",
                "head_branch": "feature/local",
                "created_at": "2024-01-01T00:10:00Z"
            }]
        })))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap();
    let job = &page.items[0];
    assert_eq!(job.normalized_status, "running");
    assert_eq!(job.conclusion, None);
    assert_eq!(
        job.branch.as_deref(),
        Some("feature/local"),
        "the job's own branch must survive a parent that has none"
    );
    assert_eq!(job.commit, None);
    assert_eq!(job.actor, None);
    assert_eq!(job.trigger, None);
    assert_eq!(job.finished_at, None);
    assert_eq!(job.runner, None);
}

/// Mounts a two-page GitHub PR domain whose newest items live on the last page.
///
/// Page 1 is exactly `PAGE_SIZE` items, which is the only signal the client has
/// that more pages may exist, so a client that stops early cannot see 101-103.
async fn mount_two_page_pr_domain(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr_page(1..=PAGE_SIZE, "alice")))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr_page(101..=103, "alice")))
        .mount(server)
        .await;
}

/// A `limit` below the domain size must select globally, not per page.
///
/// The newest three PRs are only reachable on page 2, so returning page 1's
/// first rows re-ordered — the pre-fix behavior — fails here.
#[tokio::test]
async fn pull_request_list_orders_the_complete_domain_before_truncating() {
    let server = MockServer::start().await;
    mount_two_page_pr_domain(&server).await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            sort: Some("created".to_string()),
            descending: true,
            limit: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|item| item.identity.native_id.clone())),
        ["103", "102"]
    );
    assert_eq!(page.total, Some(103));
}

#[tokio::test]
async fn pull_request_list_honors_explicit_ascending_direction() {
    let server = MockServer::start().await;
    mount_two_page_pr_domain(&server).await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            sort: Some("created".to_string()),
            descending: false,
            limit: Some(2),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|item| item.identity.native_id.clone())),
        ["1", "2"]
    );
}

/// Exact local filters are only sound over a complete domain, so a match that
/// exists solely on page 3 must still be returned — and D24's newest-first
/// default must apply without the caller naming a sort or a direction.
#[tokio::test]
async fn pull_request_filters_reach_matches_beyond_the_first_page() {
    let server = MockServer::start().await;
    for page in [1, 2] {
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .and(query_param("page", page.to_string()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(pr_page(1..=PAGE_SIZE, "bob")),
            )
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .and(query_param("page", "3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr_page(201..=203, "alice")))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            author: Some("alice".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|item| item.identity.native_id.clone())),
        ["203", "202", "201"]
    );
    assert_eq!(page.total, Some(3));
}

/// A domain that never exhausts within `MAX_PAGES` is not a complete domain, so
/// it must fail loudly rather than return a truncated or empty success.
#[tokio::test]
async fn pull_request_list_reports_an_unexhausted_domain_as_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(pr_page(1..=PAGE_SIZE, "bob")))
        .mount(&server)
        .await;
    let error = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            author: Some("nobody".to_string()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SniffError::IncompleteRemoteDomain { bound: "pull-request pages", .. }
        ),
        "expected an explicit incomplete-domain error, got {error:?}"
    );
}

/// The provider list vocabulary is a widening prefilter, never the canonical
/// token: sending an invented value is a wire-protocol defect the provider
/// would reject or silently misinterpret.
#[tokio::test]
async fn pull_request_state_is_projected_into_each_provider_vocabulary() {
    let cases: [(ApiFlavor, &[CanonicalPullRequestState], &[&str]); 14] = [
        (ApiFlavor::GitHub, &[CanonicalPullRequestState::Open], &["open"]),
        (ApiFlavor::GitHub, &[CanonicalPullRequestState::Closed], &["closed"]),
        (ApiFlavor::GitHub, &[CanonicalPullRequestState::Merged], &["closed"]),
        (
            ApiFlavor::GitHub,
            &[CanonicalPullRequestState::Open, CanonicalPullRequestState::Merged],
            &["all"],
        ),
        (ApiFlavor::Gitea, &[CanonicalPullRequestState::Merged], &["closed"]),
        (ApiFlavor::Forgejo, &[CanonicalPullRequestState::Open], &["open"]),
        (ApiFlavor::GitLab, &[CanonicalPullRequestState::Open], &["opened"]),
        (ApiFlavor::GitLab, &[CanonicalPullRequestState::Closed], &["closed"]),
        (ApiFlavor::GitLab, &[CanonicalPullRequestState::Merged], &["merged"]),
        (
            ApiFlavor::GitLab,
            &[CanonicalPullRequestState::Open, CanonicalPullRequestState::Merged],
            &["all"],
        ),
        (ApiFlavor::Bitbucket, &[CanonicalPullRequestState::Open], &["OPEN"]),
        (ApiFlavor::Bitbucket, &[CanonicalPullRequestState::Merged], &["MERGED"]),
        (
            ApiFlavor::Bitbucket,
            &[CanonicalPullRequestState::Closed],
            &["DECLINED", "SUPERSEDED"],
        ),
        (
            ApiFlavor::Bitbucket,
            &[CanonicalPullRequestState::Open, CanonicalPullRequestState::Merged],
            &["OPEN", "MERGED"],
        ),
    ];
    for (flavor, states, expected) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        client(&server, flavor)
            .query_pull_requests(PullRequestQuery {
                state: Some(QueryValues::Many(states.to_vec())),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            recorded_state_params(&server).await,
            expected,
            "{flavor:?} received the wrong state projection for {states:?}"
        );
    }
}

/// The canonical default is `open`, and it must not reach GitLab verbatim.
#[tokio::test]
async fn default_pull_request_state_uses_each_provider_token() {
    for (flavor, expected) in [
        (ApiFlavor::GitHub, "open"),
        (ApiFlavor::GitLab, "opened"),
        (ApiFlavor::Gitea, "open"),
        (ApiFlavor::Bitbucket, "OPEN"),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;
        client(&server, flavor)
            .query_pull_requests(PullRequestQuery::default())
            .await
            .unwrap();
        assert_eq!(recorded_state_params(&server).await, [expected]);
    }
}

/// Widening still has to answer the canonical question exactly: GitHub cannot
/// filter `merged`, so the local filter must drop the closed-but-unmerged rows
/// the widened `state=closed` request brings back.
#[tokio::test]
async fn widened_provider_state_is_narrowed_by_the_exact_local_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"number": 1, "title": "declined", "state": "closed", "user": {"login": "alice"}, "created_at": ts(1), "html_url": "https://127.0.0.1/pr/1"},
            {"number": 2, "title": "landed", "state": "closed", "merged_at": ts(2), "user": {"login": "alice"}, "created_at": ts(2), "html_url": "https://127.0.0.1/pr/2"}
        ])))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            state: Some(QueryValues::One(CanonicalPullRequestState::Merged)),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(recorded_state_params(&server).await, ["closed"]);
    assert_eq!(
        ids(page.items.iter().map(|item| item.identity.native_id.clone())),
        ["2"]
    );
}

/// The GitLab direct-listing path must order across pages, not within one.
#[tokio::test]
async fn cicd_direct_listing_orders_the_complete_domain_before_truncating() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/acme%2Fproject/jobs"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_job_page(1..=PAGE_SIZE)))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/projects/acme%2Fproject/jobs"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_job_page(101..=103)))
        .mount(&server)
        .await;

    let newest = client(&server, ApiFlavor::GitLab)
        .query_cicd_jobs(CiCdJobQuery { descending: true, limit: Some(2), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(
        ids(newest.items.iter().map(|job| job.reference.native_id.clone())),
        ["103", "102"]
    );
    assert_eq!(newest.total, Some(103));

    let oldest = client(&server, ApiFlavor::GitLab)
        .query_cicd_jobs(CiCdJobQuery { descending: false, limit: Some(2), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(
        ids(oldest.items.iter().map(|job| job.reference.native_id.clone())),
        ["1", "2"]
    );
}

#[tokio::test]
async fn cicd_direct_listing_reports_an_unexhausted_domain_as_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/acme%2Fproject/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_job_page(1..=PAGE_SIZE)))
        .mount(&server)
        .await;
    let error = client(&server, ApiFlavor::GitLab)
        .query_cicd_jobs(CiCdJobQuery {
            name: Some("absent".to_string()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, SniffError::IncompleteRemoteDomain { bound: "job pages", .. }),
        "expected an explicit incomplete-domain error, got {error:?}"
    );
}

/// Parent traversal must order across parents, so the newest job wins even when
/// it belongs to the parent enumerated last.
#[tokio::test]
async fn cicd_parent_traversal_orders_across_every_parent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [{"id": 1, "name": "CI"}, {"id": 2, "name": "CI"}]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/1/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [job_item(10), job_item(11)]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/2/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [job_item(90), job_item(91)]
        })))
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery { descending: true, limit: Some(1), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|job| job.reference.native_id.clone())),
        ["91"]
    );
    assert_eq!(page.total, Some(4));
}

/// More parent executions than the traversal bound means the job domain is not
/// knowable, which must surface as an error rather than a partial list.
#[tokio::test]
async fn cicd_parent_traversal_reports_the_parent_cap_as_an_error() {
    let server = MockServer::start().await;
    let parents = (1..=20)
        .map(|id| serde_json::json!({"id": id, "name": "CI"}))
        .collect::<Vec<_>>();
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": parents
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/api/repos/acme/project/actions/runs/\d+/jobs$"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"jobs": []})),
        )
        .mount(&server)
        .await;
    let error = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SniffError::IncompleteRemoteDomain { bound: "parent executions", .. }
        ),
        "expected an explicit incomplete-domain error, got {error:?}"
    );
}

/// The inspected-job bound is reachable only through parent traversal, where
/// many cleanly exhausted parents can still exceed it in aggregate.
#[tokio::test]
async fn cicd_parent_traversal_reports_the_inspected_job_cap_as_an_error() {
    let server = MockServer::start().await;
    let parents = (1..=19)
        .map(|id| serde_json::json!({"id": id, "name": "CI"}))
        .collect::<Vec<_>>();
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": parents
        })))
        .mount(&server)
        .await;
    let full = (1..=PAGE_SIZE).map(job_item).collect::<Vec<_>>();
    Mock::given(method("GET"))
        .and(path_regex(r"^/api/repos/acme/project/actions/runs/\d+/jobs$"))
        .and(query_param("page", "1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"jobs": full})),
        )
        .mount(&server)
        .await;
    let partial = (1..=50).map(job_item).collect::<Vec<_>>();
    Mock::given(method("GET"))
        .and(path_regex(r"^/api/repos/acme/project/actions/runs/\d+/jobs$"))
        .and(query_param("page", "2"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"jobs": partial})),
        )
        .mount(&server)
        .await;
    let error = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap_err();
    assert!(
        matches!(
            error,
            SniffError::IncompleteRemoteDomain { bound: "inspected jobs", .. }
        ),
        "expected an explicit incomplete-domain error, got {error:?}"
    );
}

/// A three-PR domain in which every declared filter selects a different subset.
///
/// A filter that is accepted but never applied returns the whole domain here,
/// which is what makes the expectations below discriminating; an empty
/// successful response could not tell the two apart.
fn filter_probe_domain() -> serde_json::Value {
    serde_json::json!([
        {
            "number": 1, "title": "alpha", "state": "open", "draft": true,
            "user": {"login": "alice"}, "head": {"ref": "feature/a"}, "base": {"ref": "main"},
            "labels": [{"name": "bug"}],
            "created_at": ts(1), "updated_at": ts(1),
            "html_url": "https://127.0.0.1/pr/1"
        },
        {
            "number": 2, "title": "beta", "state": "open", "draft": false,
            "user": {"login": "bob"}, "head": {"ref": "feature/b"}, "base": {"ref": "develop"},
            "labels": [{"name": "chore"}],
            "created_at": ts(50), "updated_at": ts(50),
            "html_url": "https://127.0.0.1/pr/2"
        },
        {
            "number": 3, "title": "gamma", "state": "closed", "draft": false,
            "user": {"login": "carol"}, "head": {"ref": "feature/c"}, "base": {"ref": "main"},
            "labels": [],
            "created_at": ts(30), "updated_at": ts(30), "merged_at": ts(30),
            "html_url": "https://127.0.0.1/pr/3"
        }
    ])
}

/// Builds a query exercising exactly one canonical field, with the PR numbers
/// that field must select out of [`filter_probe_domain`].
///
/// `direction` and `sort` are ordering controls rather than filters, so their
/// expectation is an order over the default `open` set rather than a subset.
/// Every other case leaves `state` absent, which means the canonical `open`
/// default also applies and PR 3 is out of scope.
fn single_filter_case(field: &str) -> (PullRequestQuery, Vec<&'static str>) {
    let base = PullRequestQuery::default;
    let (query, expected): (PullRequestQuery, Vec<&'static str>) = match field {
        "direction" => (
            PullRequestQuery {
                sort: Some("created".to_string()),
                descending: false,
                ..base()
            },
            vec!["1", "2"],
        ),
        "sort" => (PullRequestQuery { sort: Some("created".to_string()), ..base() }, vec!["2", "1"]),
        "state" => (
            PullRequestQuery {
                state: Some(QueryValues::Many(vec![CanonicalPullRequestState::Merged])),
                ..base()
            },
            vec!["3"],
        ),
        "draft" => (PullRequestQuery { draft: Some(true), ..base() }, vec!["1"]),
        "source_branch" => (
            PullRequestQuery { source_branch: Some("feature/a".to_string()), ..base() },
            vec!["1"],
        ),
        "target_branch" => (
            PullRequestQuery { target_branch: Some("develop".to_string()), ..base() },
            vec!["2"],
        ),
        "author" => (PullRequestQuery { author: Some("bob".to_string()), ..base() }, vec!["2"]),
        "labels" => (
            PullRequestQuery { labels: vec!["chore".to_string()], ..base() },
            vec!["2"],
        ),
        "search" => (PullRequestQuery { search: Some("alpha".to_string()), ..base() }, vec!["1"]),
        "created_after" => (
            PullRequestQuery { created_after: Some(ts(10)), ..base() },
            vec!["2"],
        ),
        "updated_after" => (
            PullRequestQuery { updated_after: Some(ts(10)), ..base() },
            vec!["2"],
        ),
        "created_before" => (
            PullRequestQuery { created_before: Some(ts(10)), ..base() },
            vec!["1"],
        ),
        "updated_before" => (
            PullRequestQuery { updated_before: Some(ts(10)), ..base() },
            vec!["1"],
        ),
        "limit" => (PullRequestQuery { limit: Some(1), ..base() }, vec!["2"]),
        other => panic!("declared filter {other} has no discriminating probe"),
    };
    (query, expected)
}

/// AC22: `capabilities()` is a public promise, not documentation. Every filter
/// it declares must actually change the result set, and every filter it omits
/// from the canonical query vocabulary must be refused rather than silently
/// dropped.
#[tokio::test]
async fn declared_filters_match_the_filters_the_client_actually_honors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(filter_probe_domain()))
        .mount(&server)
        .await;
    let adapter = client(&server, ApiFlavor::GitHub);
    let declared = adapter.capabilities().pull_request_filters;

    for field in &declared {
        let (query, expected) = single_filter_case(field);
        let page = adapter
            .query_pull_requests(query)
            .await
            .unwrap_or_else(|error| panic!("declared filter {field} was not honored: {error}"));
        assert_eq!(
            ids(page.items.iter().map(|item| item.identity.native_id.clone())),
            expected,
            "declared filter {field} did not select the rows it promises"
        );
    }

    for field in ["assignee", "reviewer", "milestone", "commit"] {
        assert!(
            !declared.iter().any(|declared| declared == field),
            "{field} is rejected at runtime but still advertised"
        );
        let query = match field {
            "assignee" => PullRequestQuery { assignee: Some("x".to_string()), ..Default::default() },
            "reviewer" => PullRequestQuery { reviewer: Some("x".to_string()), ..Default::default() },
            "milestone" => PullRequestQuery { milestone: Some("x".to_string()), ..Default::default() },
            _ => PullRequestQuery { commit: Some("x".to_string()), ..Default::default() },
        };
        assert!(
            matches!(
                adapter.query_pull_requests(query).await,
                Err(SniffError::UnsupportedRemoteFilter { .. })
            ),
            "{field} was silently ignored instead of refused"
        );
    }

    let capabilities = adapter.capabilities();
    assert!(!capabilities.logs);
    assert!(!capabilities.artifacts);
    assert!(!capabilities.test_reports);
}

#[tokio::test]
async fn denial_validation_and_provider_failures_remain_distinct() {
    let server = MockServer::start().await;
    let denied = FocusedProviderClient::with_api_base(remote(ApiFlavor::GitHub), FetchPolicy::deny_all(), &format!("{}/api", server.uri())).unwrap();
    assert!(matches!(denied.get_pull_request("1").await, Err(SniffError::RemotePolicyDenied { .. })));
    assert!(server.received_requests().await.unwrap().is_empty());

    let mut mismatched_remote = remote(ApiFlavor::GitHub);
    mismatched_remote.host = Some("provider.example".to_string());
    let mismatched = FocusedProviderClient::with_api_base(
        mismatched_remote,
        FetchPolicy::deny_all().allow_host("provider.example"),
        &format!("{}/api", server.uri()),
    )
    .unwrap();
    assert!(matches!(
        mismatched.get_pull_request("1").await,
        Err(SniffError::RemotePolicyDenied { .. })
    ));
    assert!(server.received_requests().await.unwrap().is_empty());

    let unsupported = client(&server, ApiFlavor::GitHub).query_pull_requests(PullRequestQuery {
        reviewer: Some("alice".to_string()),
        ..Default::default()
    }).await.unwrap_err();
    assert!(matches!(unsupported, SniffError::UnsupportedRemoteFilter { field: "reviewer", .. }));
    assert!(server.received_requests().await.unwrap().is_empty());

    for (status, expected) in [(401, "auth"), (403, "forbidden"), (429, "rate") ] {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/api/repos/acme/project/pulls/1")).respond_with(ResponseTemplate::new(status)).expect(1).mount(&server).await;
        let error = client(&server, ApiFlavor::GitHub).get_pull_request("1").await.unwrap_err();
        match expected {
            "auth" => assert!(matches!(error, SniffError::MissingCredentials { .. } | SniffError::InvalidCredentials { .. })),
            "forbidden" => assert!(matches!(error, SniffError::RemoteForbidden { .. })),
            _ => assert!(matches!(error, SniffError::RateLimited { .. })),
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/repos/acme/project/pulls/1")).respond_with(ResponseTemplate::new(200).set_body_string("not json")).expect(1).mount(&server).await;
    assert!(matches!(client(&server, ApiFlavor::GitHub).get_pull_request("1").await, Err(SniffError::RemoteApi { status: 200, .. })));

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls/1"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://evil.example/leak"),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server, ApiFlavor::GitHub).get_pull_request("1").await,
        Err(SniffError::RemoteUnreachable { .. })
    ));

    let unavailable = FocusedProviderClient::with_api_base(remote(ApiFlavor::GitHub), FetchPolicy::deny_all().allow_host("127.0.0.1"), "http://127.0.0.1:9/api").unwrap();
    assert!(matches!(unavailable.get_pull_request("1").await, Err(SniffError::RemoteUnreachable { .. })));
}

/// D24: newest-first is the default for both queries; Darkmatter's authored
/// DTO fills `descending` from `Default` when `direction` is absent, so the
/// hand-written `Default` is the contract carrier.
#[test]
fn canonical_queries_default_to_newest_first() {
    assert!(PullRequestQuery::default().descending);
    assert!(CiCdJobQuery::default().descending);
}

/// Bounds are validated where Darkmatter validates them: before any client
/// exists, so an unparseable datetime can never reach the network.
#[test]
fn canonical_validation_rejects_unparseable_and_inverted_datetimes() {
    let unparseable = PullRequestQuery {
        created_after: Some("not-a-date".to_string()),
        ..Default::default()
    };
    assert!(matches!(
        unparseable.validate_canonical(),
        Err(SniffError::InvalidRemoteQuery { field: "created_after", .. })
    ));

    // 23:00-05:00 is 04:00Z the next day, so byte order calls this ascending.
    let inverted = CiCdJobQuery {
        created_after: Some("2026-06-30T23:00:00-05:00".to_string()),
        created_before: Some("2026-07-01T00:00:00Z".to_string()),
        ..Default::default()
    };
    assert!(matches!(
        inverted.validate_canonical(),
        Err(SniffError::InvalidRemoteQuery { field: "created_after", .. })
    ));

    // The mirror: byte order rejects this window, instant order accepts it.
    let ascending = PullRequestQuery {
        created_after: Some("2026-07-01T23:00:00+14:00".to_string()),
        created_before: Some("2026-07-01T10:00:00Z".to_string()),
        ..Default::default()
    };
    ascending
        .validate_canonical()
        .expect("offsets order this window ascending");
}

/// The legacy Stage-1 `cursor` is not canonical vocabulary: the focused client
/// paginates internally and must refuse it before any I/O rather than accept
/// and ignore it.
#[tokio::test]
async fn cursor_is_refused_by_the_focused_client_before_io() {
    let server = MockServer::start().await;
    let error = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            cursor: Some("20".to_string()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(error, SniffError::InvalidRemoteQuery { field: "cursor", .. }));

    let error = client(&server, ApiFlavor::GitLab)
        .query_cicd_jobs(CiCdJobQuery {
            cursor: Some("20".to_string()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(error, SniffError::InvalidRemoteQuery { field: "cursor", .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// `stage` is a GitLab-only fact: no other flavor's job objects carry stage
/// data, so matching there would approximate the filter as an empty result.
/// The refusal must land before any request leaves the process, and the
/// capabilities advertisement must agree with it flavor by flavor.
#[tokio::test]
async fn stage_filter_is_refused_before_io_on_flavors_without_stage_data() {
    for flavor in [ApiFlavor::GitHub, ApiFlavor::Gitea, ApiFlavor::Bitbucket] {
        let server = MockServer::start().await;
        let adapter = client(&server, flavor);
        assert!(
            !adapter.capabilities().cicd_job_filters.iter().any(|field| field == "stage"),
            "{flavor:?} advertises stage but cannot honor it"
        );
        let error = adapter
            .query_cicd_jobs(CiCdJobQuery {
                stage: Some("test".to_string()),
                ..Default::default()
            })
            .await
            .unwrap_err();
        assert!(
            matches!(error, SniffError::UnsupportedRemoteFilter { field: "stage", .. }),
            "{flavor:?} did not refuse stage: {error:?}"
        );
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{flavor:?} contacted the provider before refusing stage"
        );
    }

    // GitLab both advertises and honors the filter.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/projects/acme%2Fproject/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"id": 1, "name": "unit", "stage": "test", "status": "success", "pipeline_id": 1, "created_at": ts(1)},
            {"id": 2, "name": "publish", "stage": "deploy", "status": "success", "pipeline_id": 1, "created_at": ts(2)}
        ])))
        .mount(&server)
        .await;
    let adapter = client(&server, ApiFlavor::GitLab);
    assert!(adapter.capabilities().cicd_job_filters.iter().any(|field| field == "stage"));
    let page = adapter
        .query_cicd_jobs(CiCdJobQuery { stage: Some("deploy".to_string()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|job| job.reference.native_id.clone())),
        ["2"]
    );
}

/// The `workflow` filter promises name, definition ID, *and* definition path;
/// the parent run is the only object that knows the latter two.
#[tokio::test]
async fn workflow_filter_matches_definition_id_and_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [
                {"id": 1, "name": "CI", "path": ".github/workflows/ci.yml", "workflow_id": 777},
                {"id": 2, "name": "Deploy", "path": ".github/workflows/deploy.yml", "workflow_id": 888}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/1/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [job_item(10)]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/2/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jobs": [job_item(20)]
        })))
        .mount(&server)
        .await;
    let adapter = client(&server, ApiFlavor::GitHub);
    for (workflow, expected) in [
        (".github/workflows/ci.yml", "10"),
        ("888", "20"),
        ("CI", "10"),
    ] {
        let page = adapter
            .query_cicd_jobs(CiCdJobQuery {
                workflow: Some(workflow.to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            ids(page.items.iter().map(|job| job.reference.native_id.clone())),
            [expected],
            "workflow filter {workflow} selected the wrong jobs"
        );
    }
}

/// `provider-default` means the provider's order verbatim — in *both*
/// directions of the internal flag, because `descending` orders a sort key
/// and provider-default has none. The fixture's provider order (2, 3, 1)
/// deliberately disagrees with every timestamp order so a stray key sort or
/// reversal cannot pass unnoticed.
#[tokio::test]
async fn provider_default_sort_preserves_provider_order_in_both_directions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            pr_item(2, "alice"),
            pr_item(3, "alice"),
            pr_item(1, "alice"),
        ])))
        .mount(&server)
        .await;
    let adapter = client(&server, ApiFlavor::GitHub);
    for descending in [true, false] {
        let page = adapter
            .query_pull_requests(PullRequestQuery {
                sort: Some("provider-default".to_string()),
                descending,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(
            ids(page.items.iter().map(|item| item.identity.native_id.clone())),
            ["2", "3", "1"],
            "provider order was not preserved with descending={descending}"
        );
    }
}

/// A window bound and an item timestamp written in different offsets must be
/// compared as instants, or the exact local filter drops in-range rows.
#[tokio::test]
async fn datetime_filters_compare_instants_not_strings() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"number": 1, "title": "in range", "state": "open", "user": {"login": "alice"},
             "created_at": "2024-01-01T00:30:00Z", "html_url": "https://127.0.0.1/pr/1"}
        ])))
        .mount(&server)
        .await;
    // 14:00+14:00 is 2024-01-01T00:00:00Z, half an hour before the item, but
    // it sorts *after* the item as bytes.
    let page = client(&server, ApiFlavor::GitHub)
        .query_pull_requests(PullRequestQuery {
            created_after: Some("2024-01-01T14:00:00+14:00".to_string()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        ids(page.items.iter().map(|item| item.identity.native_id.clone())),
        ["1"]
    );
}

// ---------------------------------------------------------------------------
// Neutral-host self-managed servers (production discovery path)
// ---------------------------------------------------------------------------

/// Points a throwaway repository's `origin` at a loopback mock provider, so
/// resolution runs against real configured Git state rather than a hand-built
/// `ResolvedRemote`.
fn loopback_repository(remote_url: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(directory.path()).unwrap();
    repository.remote("origin", remote_url).unwrap();
    directory
}

fn gitlab_pr_body() -> serde_json::Value {
    serde_json::json!({
        "iid": 7, "title": "Fix", "state": "opened",
        "author": {"username": "alice"},
        "created_at": "2024-01-01T00:00:00Z",
        "web_url": "https://127.0.0.1/mr/7",
    })
}

fn actions_pr_body() -> serde_json::Value {
    serde_json::json!({
        "number": 7, "title": "Fix", "state": "open",
        "user": {"login": "alice"},
        "created_at": "2024-01-01T00:00:00Z",
        "html_url": "https://127.0.0.1/pr/7",
    })
}

/// An ordinary self-managed GitLab on a neutral host (no vendor token in the
/// hostname, non-default port) must work through the production constructor:
/// configured remote → resolution → bounded discovery → flavored API base.
#[tokio::test]
#[serial]
async fn neutral_host_self_managed_gitlab_resolves_through_the_production_path() {
    let _global_token = EnvGuard::set_safe("GITLAB_TOKEN", "global-gitlab-secret");
    let _host_token = EnvGuard::set_safe(
        "SNIFF_GITLAB_127_2E_0_2E_0_2E_1_TOKEN",
        "host-gitlab-secret",
    );
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "17.0.1",
            "revision": "a1b2c3d4"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/acme%2Fproject/merge_requests/7"))
        .and(header("authorization", "Bearer host-gitlab-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_pr_body()))
        .expect(1)
        .mount(&server)
        .await;

    let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
    let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
    assert_eq!(
        resolved.api_flavor,
        ApiFlavor::Unknown,
        "a loopback host must be ambiguous before discovery"
    );
    let endpoint = resolved.endpoint.clone().expect("endpoint captured");
    assert_eq!(endpoint.scheme, "http");
    assert!(
        endpoint.port.is_some(),
        "the mock server's non-default port must be retained"
    );

    let client = FocusedProviderClient::discover(
        resolved,
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
    )
    .await
    .unwrap();
    assert_eq!(client.remote().api_flavor, ApiFlavor::GitLab);

    let record = client.get_pull_request("7").await.unwrap().expect("PR found");
    assert_eq!(record.identity.provider, GitProvider::GitLab);
    assert_eq!(record.details.title, "Fix");
    assert_eq!(record.details.author, "alice");
    for request in server.received_requests().await.unwrap() {
        assert!(
            !format!("{:?}", request.headers).contains("global-gitlab-secret"),
            "global provider credential reached the discovered host"
        );
    }
}

#[tokio::test]
async fn neutral_host_github_enterprise_resolves_through_the_production_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/meta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "installed_version": "3.16.1"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/acme/project/pulls/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(actions_pr_body()))
        .mount(&server)
        .await;

    let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
    let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
    let client = FocusedProviderClient::discover(
        resolved,
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
    )
    .await
    .unwrap();
    assert_eq!(client.remote().api_flavor, ApiFlavor::GitHub);
    assert_eq!(client.discovery().server_version.as_deref(), Some("3.16.1"));

    let record = client.get_pull_request("7").await.unwrap().expect("PR found");
    assert_eq!(record.identity.provider, GitProvider::GitHub);
}

/// Gitea and Forgejo share one API surface; only the version body tells them
/// apart, and both must query successfully after discovery.
#[tokio::test]
async fn neutral_host_gitea_and_forgejo_are_distinguished_by_the_discovery_probe() {
    for (version_body, expected_flavor) in [
        (serde_json::json!({"version": "1.22.3"}), ApiFlavor::Gitea),
        (serde_json::json!({"version": "9.0.0+forgejo-1.0"}), ApiFlavor::Forgejo),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/version"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(version_body))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/acme/project/pulls/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(actions_pr_body()))
            .mount(&server)
            .await;

        let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
        let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
        let client = FocusedProviderClient::discover(
            resolved,
            FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
        .await
        .unwrap();
        assert_eq!(client.remote().api_flavor, expected_flavor);

        let record = client.get_pull_request("7").await.unwrap().expect("PR found");
        assert_eq!(record.details.author, "alice");
    }
}

/// Gitea 1.25 is the first released API whose repository routes expose both
/// exact job lookup and repository-wide job listing. The latest 1.24 patch
/// must therefore fail both operations before a query request is sent.
#[tokio::test]
async fn gitea_job_capabilities_cross_the_1_25_endpoint_threshold() {
    for (version, supported) in [("1.24.6", false), ("1.25.0", true)] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/version"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/version"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"version": version})),
            )
            .expect(1)
            .mount(&server)
            .await;
        if supported {
            Mock::given(method("GET"))
                .and(path("/api/v1/repos/acme/project/actions/jobs/10"))
                .respond_with(ResponseTemplate::new(200).set_body_json(job_item(10)))
                .expect(1)
                .mount(&server)
                .await;
            Mock::given(method("GET"))
                .and(path("/api/v1/repos/acme/project/actions/jobs"))
                .and(query_param("page", "1"))
                .and(query_param("limit", PAGE_SIZE.to_string()))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "jobs": [job_item(10)],
                    "total_count": 1
                })))
                .expect(1)
                .mount(&server)
                .await;
        }

        let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
        let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
        let adapter = FocusedProviderClient::discover(
            resolved,
            FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
        .await
        .unwrap();
        assert_eq!(adapter.discovery().server_version.as_deref(), Some(version));
        assert_eq!(adapter.capabilities().cicd_jobs, supported);
        assert_eq!(adapter.capabilities().direct_job_listing, supported);
        let request_count_after_discovery = server.received_requests().await.unwrap().len();

        let exact = adapter
            .get_cicd_job(&reference(ApiFlavor::Gitea, "10"))
            .await;
        let list = adapter
            .query_cicd_jobs(CiCdJobQuery::default())
            .await;
        if supported {
            assert_eq!(exact.unwrap().unwrap().reference.native_id, "10");
            assert_eq!(list.unwrap().items.len(), 1);
        } else {
            for error in [exact.unwrap_err(), list.unwrap_err()] {
                assert!(matches!(
                    error,
                    SniffError::UnsupportedServerVersion {
                        ref provider,
                        ref flavor,
                        ref version,
                        ..
                    } if provider == "Gitea" && flavor == "Gitea" && version == "1.24.6"
                ));
            }
            assert_eq!(
                server.received_requests().await.unwrap().len(),
                request_count_after_discovery,
                "unsupported operations reached the provider"
            );
        }
    }
}

/// Forgejo 14 still lacks the exact-job and repository-job endpoint pair used by
/// the normalized contract, so family detection must not inherit Gitea's
/// version threshold merely because both providers share `/api/v1`.
#[tokio::test]
async fn forgejo_14_rejects_exact_and_list_jobs_before_io() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "14.0.0+forgejo-1"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
    let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
    let adapter = FocusedProviderClient::discover(
        resolved,
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
    )
    .await
    .unwrap();
    assert_eq!(adapter.discovery().api_flavor, ApiFlavor::Forgejo);
    assert!(!adapter.capabilities().cicd_jobs);
    let request_count_after_discovery = server.received_requests().await.unwrap().len();

    let exact = adapter
        .get_cicd_job(&reference(ApiFlavor::Forgejo, "10"))
        .await
        .unwrap_err();
    let list = adapter
        .query_cicd_jobs(CiCdJobQuery::default())
        .await
        .unwrap_err();
    for error in [exact, list] {
        assert!(matches!(
            error,
            SniffError::UnsupportedServerVersion {
                ref provider,
                ref flavor,
                ref version,
                ..
            } if provider == "Gitea"
                && flavor == "Forgejo"
                && version == "14.0.0+forgejo-1"
        ));
    }
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        request_count_after_discovery,
        "unsupported Forgejo operations reached the provider"
    );
}

/// Discovery is deny-by-default: an unlisted host fails before any byte
/// leaves the process.
#[tokio::test]
async fn neutral_host_discovery_is_denied_before_any_request() {
    let server = MockServer::start().await;
    let directory = loopback_repository(&format!("{}/acme/project.git", server.uri()));
    let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");

    let error = FocusedProviderClient::discover(resolved, FetchPolicy::deny_all())
        .await
        .unwrap_err();
    assert!(matches!(error, SniffError::RemotePolicyDenied { .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

/// SSH and SCP remotes must reach the production discovery policy boundary
/// through their host-derived HTTPS origin, rather than failing as unsupported
/// Git transports before policy is evaluated.
#[tokio::test]
async fn neutral_host_ssh_and_scp_discovery_checks_the_synthesized_https_host_policy() {
    for remote_url in [
        "ssh://git@git.example:2222/acme/project.git",
        "git@git.example:acme/project.git",
    ] {
        let directory = loopback_repository(remote_url);
        let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");
        assert_eq!(resolved.api_flavor, ApiFlavor::Unknown);

        let error = FocusedProviderClient::discover(resolved, FetchPolicy::deny_all())
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SniffError::RemotePolicyDenied { ref host } if host == "git.example"
        ));
    }
}

#[tokio::test]
async fn ssh_and_scp_gitlab_remotes_construct_through_the_public_discovery_api() {
    for remote_url in [
        "ssh://git@gitlab.com:2222/acme/project.git",
        "git@gitlab.com:acme/project.git",
    ] {
        let directory = loopback_repository(remote_url);
        let resolved = resolve_remote_at(directory.path(), None).unwrap().expect("remote resolved");

        let client = FocusedProviderClient::discover(resolved, FetchPolicy::deny_all())
            .await
            .unwrap();
        assert_eq!(client.remote().api_flavor, ApiFlavor::GitLab);
        assert_eq!(client.discovery().api_flavor, ApiFlavor::GitLab);
    }
}

/// A deterministically classified flavor never probes, and its API base keeps
/// the configured scheme and non-default port instead of assuming
/// `https://{host}`.
#[tokio::test]
async fn known_flavor_clients_derive_the_api_base_from_the_configured_origin() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/projects/acme%2Fproject/merge_requests/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_pr_body()))
        .expect(1)
        .mount(&server)
        .await;

    let mut resolved = remote(ApiFlavor::GitLab);
    resolved.fetch_url = format!("{}/acme/project.git", server.uri());
    resolved.endpoint = Some(RemoteEndpoint {
        scheme: "http".to_string(),
        host: "127.0.0.1".to_string(),
        port: Some(server.address().port()),
    });

    let client = FocusedProviderClient::new(
        resolved,
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
    )
    .unwrap();
    let record = client.get_pull_request("7").await.unwrap().expect("PR found");
    assert_eq!(record.details.title, "Fix");
    // `.expect(1)` above: the exact PR request and nothing else — a known
    // flavor must not spend version probes.
    server.verify().await;
}

// --- Provider-supplied link destinations ---------------------------------

/// Link destinations no provider is entitled to publish for its own repository.
///
/// Each is a distinct escape from the repository's origin: a scheme a renderer
/// would execute or read from disk, an inlined document, a different site, a
/// look-alike host, and a credentialed authority whose rendered host disagrees
/// with the one it resolves to.
const HOSTILE_LINKS: &[&str] = &[
    "javascript:alert(document.domain)",
    "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
    "file:///etc/passwd",
    "ftp://127.0.0.1/acme/project/pull/7",
    "https://evil.example/acme/project/pull/7",
    "https://127.0.0.1.evil.example/acme/project/pull/7",
    "https://127.0.0.1@evil.example/acme/project/pull/7",
    "//evil.example/acme/project/pull/7",
    "not a url",
    "",
];

/// A same-site link whose bytes would still break a Markdown destination if
/// they reached one verbatim: a closing paren, spaces, a tab, a newline, and a
/// control character.
const DELIMITER_BEARING_LINK: &str =
    "https://127.0.0.1/acme/project/pull/7?title=a (b) c\td\ne\u{1}f";

#[tokio::test]
async fn hostile_pull_request_links_are_dropped_on_exact_and_list_surfaces() {
    for hostile in HOSTILE_LINKS {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "number": 7, "title": "Fix", "state": "open",
            "user": {"login": "alice"}, "created_at": "2024-01-01",
            "html_url": hostile,
        });
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::Value::Array(vec![body])),
            )
            .mount(&server)
            .await;

        let adapter = client(&server, ApiFlavor::GitHub);
        let exact = adapter.get_pull_request("7").await.unwrap().unwrap();
        assert_eq!(exact.identity.web_url, None, "exact accepted {hostile:?}");
        assert!(exact.details.html_url.is_empty(), "exact leaked {hostile:?}");

        let listed = adapter
            .query_pull_requests(PullRequestQuery { limit: Some(1), ..Default::default() })
            .await
            .unwrap();
        assert_eq!(listed.items[0].identity.web_url, None, "list accepted {hostile:?}");
        // The record itself still projects: a hostile link costs the link, not
        // the item.
        assert_eq!(listed.items[0].identity.native_id, "7");
    }
}

#[tokio::test]
async fn hostile_cicd_job_links_are_dropped_on_exact_and_list_surfaces() {
    for hostile in HOSTILE_LINKS {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "id": 10, "name": "test", "status": "success",
            "pipeline_id": 1, "created_at": ts(10),
            "web_url": hostile,
        });
        Mock::given(method("GET"))
            .and(path("/api/projects/acme%2Fproject/jobs/10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/projects/acme%2Fproject/jobs"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::Value::Array(vec![body])),
            )
            .mount(&server)
            .await;

        let adapter = client(&server, ApiFlavor::GitLab);
        let exact = adapter
            .get_cicd_job(&reference(ApiFlavor::GitLab, "10"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(exact.web_url, None, "exact accepted {hostile:?}");

        let listed = adapter
            .query_cicd_jobs(CiCdJobQuery { limit: Some(1), ..Default::default() })
            .await
            .unwrap();
        assert_eq!(listed.items[0].web_url, None, "list accepted {hostile:?}");
        // The listed reference is the one the projection built, so it is the
        // surface where a refused link could still leak through `original_url`.
        // (`get_cicd_job` overwrites the reference with the caller's.)
        assert_eq!(
            listed.items[0].reference.original_url, None,
            "the reference retained what the projection refused: {hostile:?}"
        );
        assert_eq!(listed.items[0].reference.native_id, "10");
    }
}

/// A same-site link survives, but only after WHATWG normalization: tabs and
/// newlines are stripped and spaces and control characters percent-encoded, so
/// nothing that reaches a consumer can still carry whitespace.
#[tokio::test]
async fn same_site_links_survive_normalized_on_both_item_kinds() {
    // Tabs and newlines are *removed* by the URL parser rather than encoded;
    // the space and the control character survive as percent-escapes.
    let expected = "https://127.0.0.1/acme/project/pull/7?title=a%20(b)%20cde%01f";

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "number": 7, "title": "Fix", "state": "open",
            "user": {"login": "alice"}, "created_at": "2024-01-01",
            "html_url": DELIMITER_BEARING_LINK,
        })))
        .mount(&server)
        .await;
    let record = client(&server, ApiFlavor::GitHub)
        .get_pull_request("7")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.identity.web_url.as_deref(), Some(expected));
    assert_eq!(record.details.html_url, expected);

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/jobs/10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 10, "name": "test", "status": "success", "run_id": 1,
            "html_url": DELIMITER_BEARING_LINK,
        })))
        .mount(&server)
        .await;
    let job = client(&server, ApiFlavor::GitHub)
        .get_cicd_job(&reference(ApiFlavor::GitHub, "10"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(job.web_url.as_deref(), Some(expected));
}

/// The parent run publishes a link of its own through a separate projection
/// path, so it needs its own proof that the same policy applies there.
#[tokio::test]
async fn parent_run_links_obey_the_same_origin_policy() {
    let server = MockServer::start().await;
    let mut run = github_workflow_run();
    run["html_url"] = serde_json::json!("https://evil.example/acme/project/actions/runs/29679449");
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"workflow_runs": [run]})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/actions/runs/29679449/jobs"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({"jobs": [github_actions_job()]})))
        .mount(&server)
        .await;

    let page = client(&server, ApiFlavor::GitHub)
        .query_cicd_jobs(CiCdJobQuery { limit: Some(1), ..Default::default() })
        .await
        .unwrap();
    let job = &page.items[0];
    assert_eq!(job.parent.web_url, None, "a cross-site run link must not be published");
    assert_eq!(job.parent.name.as_deref(), Some("CI"), "the run itself still projects");
    assert_eq!(
        job.web_url.as_deref(),
        Some("https://127.0.0.1/acme/project/actions/runs/29679449/job/399444496"),
        "the job's own same-site link is unaffected"
    );
}
