//! `remote::blocking::open_pull_requests` against loopback providers.
//!
//! Shares `pr_for_branch`'s fixture: plain `#[test]`s, because the function
//! under test builds its own runtime and refuses to run inside one.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use serial_test::serial;
use sniff::filesystem::git::ApiFlavor;
use sniff::remote::blocking::{PrSummary, PrUnavailable, open_pull_requests_with};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

use super::pr_for_branch::{
    FLAVORS, Lifecycle, Provider, SHA, TARGET, list_body, pr, without_tokens,
};

const FORK: &str = "forker/project";

impl Provider {
    fn open_within(&self, deadline: Duration) -> Result<Vec<PrSummary>, PrUnavailable> {
        open_pull_requests_with(&self.client(), deadline)
    }

    fn open(&self) -> Result<Vec<PrSummary>, PrUnavailable> {
        self.open_within(Duration::from_secs(5))
    }
}

/// An open-PR row from `source` whose head branch is `branch`.
fn open_pr(flavor: ApiFlavor, number: u64, source: &str, branch: &str) -> Value {
    let mut row = pr(flavor, number, Lifecycle::Open, source, Some(SHA));
    match flavor {
        ApiFlavor::GitLab => row["source_branch"] = json!(branch),
        ApiFlavor::Bitbucket => row["source"]["branch"]["name"] = json!(branch),
        _ => {
            row["head"]["ref"] = json!(branch);
            row["head"]["label"] = json!(format!(
                "{}:{branch}",
                source.split('/').next().unwrap_or_default()
            ));
        }
    }
    row
}

type Fields = (
    u64,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Every field of `summary`, since `PrSummary` is `#[non_exhaustive]`.
fn fields(summary: &PrSummary) -> Fields {
    (
        summary.number,
        summary.html_url.clone(),
        summary.source_repo.clone(),
        summary.source_branch.clone(),
        summary.target_branch.clone(),
    )
}

fn expected(number: u64, source: &str, branch: &str) -> Fields {
    (
        number,
        Some(format!("https://127.0.0.1/acme/project/pull/{number}")),
        Some(source.to_string()),
        Some(branch.to_string()),
        Some("main".to_string()),
    )
}

fn open_fields(provider: &Provider) -> Vec<Fields> {
    provider.open().unwrap().iter().map(fields).collect()
}

/// Serves GitLab's `projects/{id}` for the fork's project ID (2 in the
/// shared fixture).
fn serve_gitlab_fork_project(provider: &Provider, response: ResponseTemplate) {
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/projects/2"))
            .respond_with(response),
    );
}

#[test]
#[serial]
fn open_prs_map_number_url_source_and_branches_on_every_provider() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![
            open_pr(flavor, 7, TARGET, "feature"),
            open_pr(flavor, 9, FORK, "feature"),
        ]);
        if flavor == ApiFlavor::GitLab {
            serve_gitlab_fork_project(
                &provider,
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": 2, "path_with_namespace": FORK})),
            );
        }

        assert_eq!(
            open_fields(&provider),
            vec![expected(7, TARGET, "feature"), expected(9, FORK, "feature")],
            "{flavor:?}"
        );
    }
}

#[test]
#[serial]
fn rows_that_are_not_open_are_dropped() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(vec![
            pr(flavor, 3, Lifecycle::Merged, TARGET, Some(SHA)),
            pr(flavor, 4, Lifecycle::ClosedUnmerged, TARGET, Some(SHA)),
            open_pr(flavor, 5, TARGET, "feature"),
        ]);

        let numbers: Vec<u64> = provider
            .open()
            .unwrap()
            .iter()
            .map(|summary| summary.number)
            .collect();

        assert_eq!(numbers, vec![5], "{flavor:?}");
    }
}

#[test]
#[serial]
fn an_empty_list_is_an_empty_answer_on_every_provider() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list(Vec::new());

        assert_eq!(provider.open(), Ok(Vec::new()), "{flavor:?}");
    }
}

#[test]
#[serial]
fn each_provider_receives_its_open_state_filter() {
    let _tokens = without_tokens();
    let pair = |key: &str, value: &str| (key.to_string(), value.to_string());
    for (flavor, expected) in [
        (
            ApiFlavor::GitHub,
            vec![pair("state", "open"), pair("per_page", "100")],
        ),
        (
            ApiFlavor::GitLab,
            vec![pair("state", "opened"), pair("per_page", "100")],
        ),
        (
            ApiFlavor::Gitea,
            vec![pair("state", "open"), pair("limit", "50")],
        ),
        (
            ApiFlavor::Bitbucket,
            vec![pair("state", "OPEN"), pair("pagelen", "50")],
        ),
    ] {
        let provider = Provider::start(flavor);
        provider.serve_list(Vec::new());

        assert_eq!(provider.open(), Ok(Vec::new()), "{flavor:?}");

        let queries = provider.received_queries();
        assert_eq!(queries.len(), 1, "{flavor:?}");
        for expected_pair in &expected {
            assert!(
                queries[0].contains(expected_pair),
                "{flavor:?} missing {expected_pair:?} in {:?}",
                queries[0]
            );
        }
    }
}

#[test]
#[serial]
fn a_full_gitea_page_is_followed_by_the_next() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::Gitea);
    let first_page = (100..150)
        .map(|number| open_pr(ApiFlavor::Gitea, number, TARGET, &format!("b{number}")))
        .collect::<Vec<_>>();
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(Value::Array(first_page))),
    );
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([open_pr(
                ApiFlavor::Gitea,
                7,
                TARGET,
                "last"
            )]))),
    );

    let summaries = provider.open().unwrap();

    assert_eq!(summaries.len(), 51);
    assert_eq!(summaries[50].number, 7);
    assert_eq!(provider.received_queries().len(), 2);
}

#[test]
#[serial]
fn a_bitbucket_next_link_is_followed() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::Bitbucket);
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repositories/acme/project/pullrequests"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [open_pr(ApiFlavor::Bitbucket, 1, TARGET, "one")],
                "next": "https://127.0.0.1/unused?page=2",
            }))),
    );
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repositories/acme/project/pullrequests"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [open_pr(ApiFlavor::Bitbucket, 2, FORK, "two")],
            }))),
    );

    let numbers: Vec<u64> = provider
        .open()
        .unwrap()
        .iter()
        .map(|summary| summary.number)
        .collect();

    assert_eq!(numbers, vec![1, 2]);
}

#[test]
#[serial]
fn a_gitlab_fork_project_lookup_is_made_once_per_fork() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::GitLab);
    provider.serve_list(vec![
        open_pr(ApiFlavor::GitLab, 1, FORK, "one"),
        open_pr(ApiFlavor::GitLab, 2, FORK, "two"),
    ]);
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/projects/2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": 2, "path_with_namespace": FORK})),
            )
            .expect(1),
    );

    let sources: Vec<Option<String>> = provider
        .open()
        .unwrap()
        .into_iter()
        .map(|summary| summary.source_repo)
        .collect();

    assert_eq!(sources, vec![Some(FORK.to_string()); 2]);
}

#[test]
#[serial]
fn an_unseen_gitlab_fork_has_no_source_repository_but_the_list_answers() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::GitLab);
    provider.serve_list(vec![
        open_pr(ApiFlavor::GitLab, 1, TARGET, "mine"),
        open_pr(ApiFlavor::GitLab, 2, FORK, "theirs"),
    ]);
    serve_gitlab_fork_project(&provider, ResponseTemplate::new(404));

    let summaries = provider.open().unwrap();

    assert_eq!(summaries[0].source_repo.as_deref(), Some(TARGET));
    assert_eq!(summaries[1].number, 2);
    assert_eq!(summaries[1].source_repo, None);
}

#[test]
#[serial]
fn auth_failures_are_unavailable_not_empty_on_every_provider() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        for (status, token) in [(401, None), (401, Some("rejected")), (403, Some("scoped"))] {
            let _token = token.map(|value| {
                let variable = match flavor {
                    ApiFlavor::GitHub => "GITHUB_TOKEN",
                    ApiFlavor::GitLab => "GITLAB_TOKEN",
                    ApiFlavor::Gitea => "GITEA_TOKEN",
                    _ => "BITBUCKET_TOKEN",
                };
                EnvGuard::set_safe(variable, value)
            });
            let provider = Provider::start(flavor);
            provider.serve_list_response(ResponseTemplate::new(status));

            let result = provider.open();

            assert!(
                matches!(result, Err(PrUnavailable::Auth { .. })),
                "{flavor:?} {status} token={token:?}: {result:?}"
            );
        }
    }
}

#[test]
#[serial]
fn a_private_repository_list_404_is_unavailable_not_empty() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_list_response(ResponseTemplate::new(404));

        let result = provider.open();

        assert!(
            matches!(result, Err(PrUnavailable::NotFoundOrNotPermitted { .. })),
            "{flavor:?}: {result:?}"
        );
    }
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
        let result = provider.open_within(deadline);

        assert_eq!(
            result,
            Err(PrUnavailable::Timeout { deadline }),
            "{flavor:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(3), "{flavor:?}");
    }
}
