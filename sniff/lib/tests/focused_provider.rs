#![cfg(feature = "remote")]

use biscuit_file::FetchPolicy;
use sniff::SniffError;
use sniff::filesystem::git::{ApiFlavor, ResolvedRemote};
use sniff::remote::{
    CiCdJobQuery, CiCdJobReference, FocusedProviderClient, GitProvider, PullRequestQuery,
    CanonicalPullRequestState, QueryValues,
};
use wiremock::matchers::{method, path, path_regex, query_param};
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
        "html_url": format!("https://example/pr/{number}"),
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

fn remote(flavor: ApiFlavor) -> ResolvedRemote {
    ResolvedRemote {
        name: "origin".to_string(),
        fetch_url: "git@127.0.0.1:acme/project.git".to_string(),
        push_url: "git@127.0.0.1:acme/project.git".to_string(),
        host: Some("127.0.0.1".to_string()),
        namespace: Some("acme".to_string()),
        repository: Some("project".to_string()),
        api_flavor: flavor,
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
    FocusedProviderClient::with_api_base(
        remote(flavor),
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
        &format!("{}/api", server.uri()),
    ).unwrap()
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

#[test]
fn canonical_pr_and_job_urls_preserve_repository_scoped_identity() {
    for (url, id) in [
        ("https://github.com/acme/project/pull/7", "7"),
        ("https://gitlab.com/group/sub/project/-/merge_requests/8", "8"),
        ("https://forgejo.example/acme/project/pulls/9", "9"),
        ("https://bitbucket.org/acme/project/pull-requests/10", "10"),
    ] {
        let (_, parsed) = FocusedProviderClient::from_pull_request_url(
            url,
            FetchPolicy::deny_all(),
        )
        .unwrap();
        assert_eq!(parsed, id);
    }

    for (url, native_id, namespace) in [
        ("https://github.com/acme/project/actions/runs/20/job/21", "21", "acme"),
        ("https://gitlab.com/group/sub/project/-/jobs/22", "22", "group/sub"),
        (
            "https://bitbucket.org/acme/project/pipelines/results/p1/steps/s1",
            "p1/s1",
            "acme",
        ),
    ] {
        let (remote, reference) = FocusedProviderClient::job_reference_from_url(url).unwrap();
        assert_eq!(reference.native_id, native_id);
        assert_eq!(reference.original_url.as_deref(), Some(url));
        assert_eq!(remote.namespace.as_deref(), Some(namespace));
        assert_eq!(remote.repository.as_deref(), Some("project"));
    }

    assert!(FocusedProviderClient::from_pull_request_url(
        "https://github.com/acme/project/issues/7",
        FetchPolicy::deny_all(),
    )
    .is_err());
}

#[tokio::test]
async fn exact_pull_requests_preserve_identity_and_authoritative_not_found() {
    let cases = [
        (ApiFlavor::GitHub, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://github.example/pr/7", "url": "https://api.example/pr/7"})),
        (ApiFlavor::GitLab, "/api/projects/acme%2Fproject/merge_requests/7", "/api/projects/acme%2Fproject/merge_requests/8", serde_json::json!({"iid": 7, "title": "Fix", "state": "opened", "author": {"username": "alice"}, "created_at": "2024-01-01", "web_url": "https://gitlab.example/mr/7"})),
        (ApiFlavor::Gitea, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://gitea.example/pr/7"})),
        (ApiFlavor::Forgejo, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://forgejo.example/pr/7"})),
        (ApiFlavor::Bitbucket, "/api/repositories/acme/project/pullrequests/7", "/api/repositories/acme/project/pullrequests/8", serde_json::json!({"id": 7, "title": "Fix", "state": "OPEN", "author": {"display_name": "alice"}, "created_on": "2024-01-01", "links": {"html": {"href": "https://bitbucket.example/pr/7"}}})),
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

#[tokio::test]
async fn exact_jobs_are_normalized_for_every_initial_flavor() {
    let cases = [
        (ApiFlavor::GitHub, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "completed", "conclusion": "success", "run_id": 1})),
        (ApiFlavor::GitLab, "10", "/api/projects/acme%2Fproject/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "pipeline_id": 1})),
        (ApiFlavor::Gitea, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "run_id": 1})),
        (ApiFlavor::Forgejo, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "run_id": 1})),
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
async fn job_listing_uses_direct_or_bounded_parent_strategy_for_every_flavor() {
    for flavor in [ApiFlavor::GitHub, ApiFlavor::GitLab, ApiFlavor::Gitea, ApiFlavor::Forgejo, ApiFlavor::Bitbucket] {
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
/// on the workflow run.
fn github_actions_job() -> serde_json::Value {
    serde_json::json!({
        "id": 399444496,
        "run_id": 29679449,
        "run_attempt": 1,
        "url": "https://api.github.com/repos/acme/project/actions/jobs/399444496",
        "html_url": "https://github.com/acme/project/actions/runs/29679449/job/399444496",
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
        "html_url": "https://github.com/acme/project/actions/runs/29679449",
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
        "web_url": "https://gitlab.example/acme/project/-/jobs/7742",
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
        "links": {"html": {"href": "https://bitbucket.org/acme/project/pipelines/results/42"}}
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
        Some("https://gitlab.example/acme/project/-/jobs/7742")
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

    // GitHub/Gitea/Forgejo: flat runner_name, completed_at, conclusion-derived
    // verdict. Branch/commit/actor/trigger are absent without a parent run.
    for flavor in [ApiFlavor::GitHub, ApiFlavor::Gitea, ApiFlavor::Forgejo] {
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
            Some("https://github.com/acme/project/actions/runs/29679449/job/399444496")
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
        Some("https://github.com/acme/project/actions/runs/29679449")
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
            {"number": 1, "title": "declined", "state": "closed", "user": {"login": "alice"}, "created_at": ts(1), "html_url": "https://example/pr/1"},
            {"number": 2, "title": "landed", "state": "closed", "merged_at": ts(2), "user": {"login": "alice"}, "created_at": ts(2), "html_url": "https://example/pr/2"}
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
            "html_url": "https://example/pr/1"
        },
        {
            "number": 2, "title": "beta", "state": "open", "draft": false,
            "user": {"login": "bob"}, "head": {"ref": "feature/b"}, "base": {"ref": "develop"},
            "labels": [{"name": "chore"}],
            "created_at": ts(50), "updated_at": ts(50),
            "html_url": "https://example/pr/2"
        },
        {
            "number": 3, "title": "gamma", "state": "closed", "draft": false,
            "user": {"login": "carol"}, "head": {"ref": "feature/c"}, "base": {"ref": "main"},
            "labels": [],
            "created_at": ts(30), "updated_at": ts(30), "merged_at": ts(30),
            "html_url": "https://example/pr/3"
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
    let (value, expected): (serde_json::Value, Vec<&'static str>) = match field {
        "direction" => {
            return (
                serde_json::from_value(
                    serde_json::json!({"sort": "created", "descending": false}),
                )
                .unwrap(),
                vec!["1", "2"],
            );
        }
        "sort" => (serde_json::json!("created"), vec!["2", "1"]),
        "state" => (serde_json::json!(["merged"]), vec!["3"]),
        "draft" => (serde_json::json!(true), vec!["1"]),
        "source_branch" => (serde_json::json!("feature/a"), vec!["1"]),
        "target_branch" => (serde_json::json!("develop"), vec!["2"]),
        "author" => (serde_json::json!("bob"), vec!["2"]),
        "labels" => (serde_json::json!(["chore"]), vec!["2"]),
        "search" => (serde_json::json!("alpha"), vec!["1"]),
        "created_after" | "updated_after" => (serde_json::json!(ts(10)), vec!["2"]),
        "created_before" | "updated_before" => (serde_json::json!(ts(10)), vec!["1"]),
        "limit" => (serde_json::json!(1), vec!["2"]),
        other => panic!("declared filter {other} has no discriminating probe"),
    };
    let query = serde_json::json!({ field: value });
    (serde_json::from_value(query).unwrap(), expected)
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
        assert!(
            matches!(
                adapter
                    .query_pull_requests(
                        serde_json::from_value(serde_json::json!({ field: "x" })).unwrap(),
                    )
                    .await,
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

/// The canonical vocabulary is closed, and the closure has to survive the
/// deserialization Darkmatter performs before it ever resolves a repository.
#[test]
fn canonical_query_deserialization_rejects_out_of_vocabulary_input() {
    for value in [
        serde_json::json!({"state": "draft"}),
        serde_json::json!({"state": "all"}),
        serde_json::json!({"state": ["open", "draft"]}),
        serde_json::json!({"unknown": true}),
        serde_json::json!({"remote": "origin"}),
    ] {
        assert!(
            serde_json::from_value::<PullRequestQuery>(value.clone()).is_err(),
            "PullRequestQuery accepted {value}"
        );
    }
    for state in ["open", "closed", "merged"] {
        serde_json::from_value::<PullRequestQuery>(serde_json::json!({ "state": state }))
            .unwrap_or_else(|error| panic!("rejected canonical state {state}: {error}"));
    }
    assert!(serde_json::from_value::<CiCdJobQuery>(serde_json::json!({"unknown": true})).is_err());
}

/// The spec spells `parent` as "number(integer) or string"; both must land on
/// the string form the job matcher compares against.
#[test]
fn cicd_parent_accepts_integer_and_string_identities() {
    let numeric: CiCdJobQuery =
        serde_json::from_value(serde_json::json!({"parent": 1234})).unwrap();
    let textual: CiCdJobQuery =
        serde_json::from_value(serde_json::json!({"parent": "1234"})).unwrap();
    assert_eq!(numeric.parent.as_deref(), Some("1234"));
    assert_eq!(numeric.parent, textual.parent);
}

/// D24: newest-first is the default for both queries, including the
/// `#[serde(default)]` fill-in an absent `direction` relies on.
#[test]
fn canonical_queries_default_to_newest_first() {
    assert!(PullRequestQuery::default().descending);
    assert!(CiCdJobQuery::default().descending);
    assert!(
        serde_json::from_value::<PullRequestQuery>(serde_json::json!({}))
            .unwrap()
            .descending
    );
    assert!(
        serde_json::from_value::<CiCdJobQuery>(serde_json::json!({"limit": 5}))
            .unwrap()
            .descending
    );
}

/// Bounds are validated where Darkmatter validates them: before any client
/// exists, so an unparseable datetime can never reach the network.
#[test]
fn canonical_validation_rejects_unparseable_and_inverted_datetimes() {
    let unparseable: PullRequestQuery =
        serde_json::from_value(serde_json::json!({"created_after": "not-a-date"})).unwrap();
    assert!(matches!(
        unparseable.validate_canonical(),
        Err(SniffError::InvalidRemoteQuery { field: "created_after", .. })
    ));

    // 23:00-05:00 is 04:00Z the next day, so byte order calls this ascending.
    let inverted: CiCdJobQuery = serde_json::from_value(serde_json::json!({
        "created_after": "2026-06-30T23:00:00-05:00",
        "created_before": "2026-07-01T00:00:00Z"
    }))
    .unwrap();
    assert!(matches!(
        inverted.validate_canonical(),
        Err(SniffError::InvalidRemoteQuery { field: "created_after", .. })
    ));

    // The mirror: byte order rejects this window, instant order accepts it.
    let ascending: PullRequestQuery = serde_json::from_value(serde_json::json!({
        "created_after": "2026-07-01T23:00:00+14:00",
        "created_before": "2026-07-01T10:00:00Z"
    }))
    .unwrap();
    ascending
        .validate_canonical()
        .expect("offsets order this window ascending");
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
             "created_at": "2024-01-01T00:30:00Z", "html_url": "https://example/pr/1"}
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
