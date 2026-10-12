//! `open_pull_requests_with_status` against a loopback GitHub: batched GraphQL
//! enrichment of the REST list, from saved response fixtures.
//!
//! Plain `#[test]`s, as for `open_pull_requests`: the function under test
//! builds its own runtime.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use serial_test::serial;
use sniff::filesystem::git::ApiFlavor;
use sniff::remote::blocking::{
    OpenPullRequests, PrCheckStatus, PrUnavailable, RequestCredentials, open_pull_requests_with,
    open_pull_requests_with_status_with,
};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path, path_regex, query_param};
use wiremock::{Mock, ResponseTemplate};

use super::pr_for_branch::{Lifecycle, Provider, SECRET, SHA, TARGET, pr, without_tokens};

const SUCCESS: &str = include_str!("../fixtures/remote/graphql_check_status/success.json");
const PASSED: &str = include_str!("../fixtures/remote/graphql_check_status/passed.json");
const PARTIAL_ERRORS: &str =
    include_str!("../fixtures/remote/graphql_check_status/partial_errors.json");
const REQUEST_WIDE: &str =
    include_str!("../fixtures/remote/graphql_check_status/request_wide_errors.json");
const PERMISSION: &str =
    include_str!("../fixtures/remote/graphql_check_status/permission_failure.json");
const RATE_LIMIT: &str = include_str!("../fixtures/remote/graphql_check_status/rate_limit.json");
const CHANGED_HEAD: &str =
    include_str!("../fixtures/remote/graphql_check_status/changed_head.json");
const NULL_ROLLUP: &str = include_str!("../fixtures/remote/graphql_check_status/null_rollup.json");

use PrCheckStatus::{Failing, Passed, Running, Unknown};

fn keyed() -> Vec<EnvGuard> {
    let mut guards = without_tokens();
    guards.push(EnvGuard::set_safe("GH_TOKEN", SECRET));
    guards
}

impl Provider {
    fn serve_github_prs(&self, count: u64) {
        let rows = (1..=count)
            .map(|number| pr(ApiFlavor::GitHub, number, Lifecycle::Open, TARGET, Some(SHA)))
            .collect::<Vec<_>>();
        let page = |number: &str, rows: &[Value]| {
            self.mount(
                Mock::given(method("GET"))
                    .and(path("/api/repos/acme/project/pulls"))
                    .and(query_param("page", number))
                    .respond_with(ResponseTemplate::new(200).set_body_json(rows)),
            );
        };
        // A full page promises another, so a final short or empty page ends the walk.
        let mut pages: Vec<&[Value]> = rows.chunks(100).collect();
        if rows.len() % 100 == 0 {
            pages.push(&[]);
        }
        for (index, rows) in pages.into_iter().enumerate() {
            page(&(index + 1).to_string(), rows);
        }
    }

    fn serve_graphql(&self, response: ResponseTemplate) {
        self.mount(
            Mock::given(method("POST"))
                .and(path("/api/graphql"))
                .respond_with(response),
        );
    }

    fn serve_graphql_once(&self, response: ResponseTemplate) {
        self.mount(
            Mock::given(method("POST"))
                .and(path("/api/graphql"))
                .respond_with(response)
                .up_to_n_times(1),
        );
    }

    fn graphql_requests(&self) -> Vec<wiremock::Request> {
        self.received_requests()
            .into_iter()
            .filter(|request| request.method.as_str() == "POST")
            .collect()
    }

    fn with_status_within(&self, deadline: Duration) -> Result<OpenPullRequests, PrUnavailable> {
        open_pull_requests_with_status_with(&self.client(), deadline)
    }

    fn statuses(&self) -> Vec<PrCheckStatus> {
        self.with_status_within(Duration::from_secs(5))
            .expect("the REST list is never lost to enrichment")
            .pull_requests
            .iter()
            .map(|summary| summary.status)
            .collect()
    }
}

fn json_body(text: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).insert_header("content-type", "application/json").set_body_string(text)
}

/// A response that gives every one of `count` lookups an all-passing record.
fn all_passed(count: usize) -> ResponseTemplate {
    let record = serde_json::from_str::<Value>(PASSED).unwrap()["data"]["repository"]["p0"].clone();
    let repository: serde_json::Map<String, Value> =
        (0..count).map(|index| (format!("p{index}"), record.clone())).collect();
    ResponseTemplate::new(200).set_body_json(json!({"data": {"repository": repository}}))
}

#[test]
#[serial]
fn each_pr_gets_the_status_its_rollup_counts_map_to() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(3);
    provider.serve_graphql(json_body(SUCCESS));

    let answer = provider.with_status_within(Duration::from_secs(5)).unwrap();

    let numbered: Vec<_> = answer.pull_requests.iter().map(|s| (s.number, s.status)).collect();
    assert_eq!(numbered, vec![(1, Passed), (2, Failing), (3, Running)]);
    assert_eq!(
        answer.credentials,
        RequestCredentials::Keyed { variables: vec!["GH_TOKEN".to_string()] }
    );
}

#[test]
#[serial]
fn the_query_carries_no_provider_text_and_the_token_is_sent_as_a_bearer() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(2);
    provider.serve_graphql(json_body(SUCCESS));
    provider.statuses();

    let requests = provider.graphql_requests();
    assert_eq!(requests.len(), 1);
    let body: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let query = body["query"].as_str().unwrap();
    assert!(!query.contains("acme") && !query.contains("project"), "{query}");
    assert_eq!(body["variables"], json!({"owner": "acme", "name": "project", "n0": 1, "n1": 2}));
    assert_eq!(
        requests[0].headers.get("authorization").unwrap().to_str().unwrap(),
        format!("Bearer {SECRET}")
    );
}

#[test]
#[serial]
fn the_ordinary_query_sends_no_graphql_and_reports_unknown() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(2);
    provider.serve_graphql(json_body(SUCCESS));

    let answer = open_pull_requests_with(&provider.client(), Duration::from_secs(5)).unwrap();

    assert!(answer.pull_requests.iter().all(|summary| summary.status == Unknown));
    assert!(provider.graphql_requests().is_empty());
}

#[test]
#[serial]
fn requests_are_one_batch_up_to_100_prs_and_two_for_101() {
    let _tokens = keyed();
    for (count, batches) in [(1_u64, 1_usize), (100, 1), (101, 2)] {
        let provider = Provider::start(ApiFlavor::GitHub);
        provider.serve_github_prs(count);
        provider.serve_graphql(all_passed(100));

        let statuses = provider.statuses();

        assert_eq!(statuses.len() as u64, count);
        let sizes: Vec<usize> = provider
            .graphql_requests()
            .iter()
            .map(|request| {
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                body["variables"].as_object().unwrap().len() - 2
            })
            .collect();
        assert_eq!(sizes.len(), batches, "{count} PRs");
        assert_eq!(sizes.iter().sum::<usize>() as u64, count);
        assert!(sizes.iter().all(|size| *size <= 100));
    }
}

#[test]
#[serial]
fn an_empty_list_anonymous_access_and_other_providers_send_no_graphql() {
    let _none = without_tokens();
    let anonymous = Provider::start(ApiFlavor::GitHub);
    anonymous.serve_github_prs(2);
    anonymous.serve_graphql(json_body(SUCCESS));
    assert_eq!(anonymous.statuses(), vec![Unknown, Unknown]);
    assert!(anonymous.graphql_requests().is_empty());
    assert_eq!(
        anonymous.with_status_within(Duration::from_secs(5)).unwrap().credentials,
        RequestCredentials::Anonymous
    );

    let _token = EnvGuard::set_safe("GH_TOKEN", SECRET);
    let empty = Provider::start(ApiFlavor::GitHub);
    empty.mount(
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/repos/acme/project/pulls$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([]))),
    );
    empty.serve_graphql(json_body(SUCCESS));
    assert_eq!(empty.statuses(), Vec::new());
    assert!(empty.graphql_requests().is_empty());

    let _gitlab = EnvGuard::set_safe("GITLAB_TOKEN", SECRET);
    let gitlab = Provider::start(ApiFlavor::GitLab);
    gitlab.serve_list(vec![pr(ApiFlavor::GitLab, 5, Lifecycle::Open, TARGET, Some(SHA))]);
    assert_eq!(gitlab.statuses(), vec![Unknown]);
    assert!(gitlab.graphql_requests().is_empty());
}

#[test]
#[serial]
fn a_request_wide_failure_stops_later_batches_and_keeps_earlier_statuses() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(201);
    provider.serve_graphql_once(all_passed(100));
    provider.serve_graphql(ResponseTemplate::new(500));

    let statuses = provider.statuses();

    assert_eq!(statuses.len(), 201);
    assert!(statuses[..100].iter().all(|status| *status == Passed));
    assert!(statuses[100..].iter().all(|status| *status == Unknown));
    assert_eq!(provider.graphql_requests().len(), 2, "no third batch after the failure");
}

#[test]
#[serial]
fn failures_leave_the_complete_list_with_unknown_statuses() {
    let _tokens = keyed();
    let cases = [
        ("permission", ResponseTemplate::new(403).set_body_string(PERMISSION)),
        ("rate limit status", ResponseTemplate::new(429)),
        ("rate limit body", json_body(RATE_LIMIT)),
        ("no repository", json_body(REQUEST_WIDE)),
        ("server error", ResponseTemplate::new(500)),
        ("not json", ResponseTemplate::new(200).set_body_string("<html>")),
        ("trailing content", json_body(&format!("{SUCCESS} garbage"))),
        (
            "duplicate key",
            json_body(r#"{"data":{"repository":{"p0":null,"p0":null}}}"#),
        ),
    ];
    for (label, response) in cases {
        let provider = Provider::start(ApiFlavor::GitHub);
        provider.serve_github_prs(101);
        provider.serve_graphql(response);

        let statuses = provider.statuses();

        assert_eq!(statuses.len(), 101, "{label}");
        assert!(statuses.iter().all(|status| *status == Unknown), "{label}");
        assert_eq!(provider.graphql_requests().len(), 1, "{label}: stops after the first batch");
    }
}

#[test]
#[serial]
fn per_pr_errors_make_only_the_affected_pr_unknown() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(3);
    provider.serve_graphql(json_body(PARTIAL_ERRORS));

    assert_eq!(provider.statuses(), vec![Passed, Unknown, Unknown]);
}

#[test]
#[serial]
fn a_changed_head_or_null_rollup_is_unknown() {
    let _tokens = keyed();
    for (label, fixture) in [("changed head", CHANGED_HEAD), ("null rollup", NULL_ROLLUP)] {
        let provider = Provider::start(ApiFlavor::GitHub);
        provider.serve_github_prs(1);
        provider.serve_graphql(json_body(fixture));

        assert_eq!(provider.statuses(), vec![Unknown], "{label}");
    }
}

#[test]
#[serial]
fn a_deadline_after_the_complete_list_keeps_the_list() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_github_prs(2);
    provider.serve_graphql(json_body(SUCCESS).set_delay(Duration::from_secs(5)));

    let started = Instant::now();
    let answer = provider.with_status_within(Duration::from_secs(1)).unwrap();

    assert!(started.elapsed() < Duration::from_millis(2500), "{:?}", started.elapsed());
    assert_eq!(answer.pull_requests.len(), 2);
    assert!(answer.pull_requests.iter().all(|summary| summary.status == Unknown));
}

#[test]
#[serial]
fn a_failed_list_is_still_an_error_whatever_enrichment_would_say() {
    let _tokens = keyed();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.mount(
        Mock::given(method("GET"))
            .and(path("/api/repos/acme/project/pulls"))
            .respond_with(ResponseTemplate::new(500)),
    );
    provider.serve_graphql(json_body(SUCCESS));

    assert!(provider.with_status_within(Duration::from_secs(5)).is_err());
    assert!(provider.graphql_requests().is_empty());
}

/// One edit per cell of the input matrix, applied to the real-shaped `passed`
/// fixture's counts, read through the public `PrSummary::status`.
#[test]
#[serial]
fn the_graphql_reader_walks_the_input_robustness_matrix() {
    let _tokens = keyed();
    let run = |edit: &dyn Fn(&mut Value)| -> PrCheckStatus {
        let mut document: Value = serde_json::from_str(PASSED).unwrap();
        edit(&mut document["data"]["repository"]["p0"]);
        let provider = Provider::start(ApiFlavor::GitHub);
        provider.serve_github_prs(1);
        provider.serve_graphql(ResponseTemplate::new(200).set_body_json(document));
        provider.statuses()[0]
    };
    fn contexts(record: &mut Value) -> &mut Value {
        &mut record["commits"]["nodes"][0]["commit"]["statusCheckRollup"]["contexts"]
    }

    // Control row: the unedited fixture is positive, so every edit below is
    // load-bearing.
    assert_eq!(run(&|_| {}), Passed);

    // absent
    assert_eq!(run(&|r| { contexts(r).as_object_mut().unwrap().remove("checkRunCountsByState"); }), Unknown);
    assert_eq!(run(&|r| { contexts(r).as_object_mut().unwrap().remove("checkRunCount"); }), Unknown);
    assert_eq!(run(&|r| { r["commits"]["nodes"][0]["commit"].as_object_mut().unwrap().remove("oid"); }), Unknown);
    assert_eq!(run(&|r| { r["commits"]["nodes"][0]["commit"].as_object_mut().unwrap().remove("statusCheckRollup"); }), Unknown);
    // explicit null
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"] = Value::Null), Unknown);
    assert_eq!(run(&|r| contexts(r)["checkRunCount"] = Value::Null), Unknown);
    assert_eq!(run(&|r| r["commits"]["nodes"][0]["commit"]["statusCheckRollup"] = Value::Null), Unknown);
    // wrong type, whole field
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"] = json!(123)), Unknown);
    assert_eq!(run(&|r| contexts(r)["checkRunCount"] = json!("2")), Unknown);
    // wrong type, one element: cannot be green ...
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"].as_array_mut().unwrap().push(json!(123))), Unknown);
    // ... but a failure elsewhere in the array still counts
    assert_eq!(
        run(&|r| {
            let counts = contexts(r)["checkRunCountsByState"].as_array_mut().unwrap();
            counts.push(json!({"state": "FAILURE", "count": 1}));
            counts.push(json!(123));
        }),
        Failing
    );
    // wrong type, every element
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"] = json!([123])), Unknown);
    assert_eq!(run(&|r| contexts(r)["statusContextCountsByState"] = json!([123])), Unknown);
    // empty: no contexts at all is unknown, not passed
    assert_eq!(
        run(&|r| {
            contexts(r)["checkRunCountsByState"] = json!([]);
            contexts(r)["statusContextCountsByState"] = json!([]);
            contexts(r)["checkRunCount"] = json!(0);
            contexts(r)["statusContextCount"] = json!(0);
        }),
        Unknown
    );
    // counts that disagree with the declared total
    assert_eq!(run(&|r| contexts(r)["checkRunCount"] = json!(5)), Unknown);
    // an unrecognized state, and a completed run without its conclusion
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"].as_array_mut().unwrap()[0]["state"] = json!("SPARKLY")), Unknown);
    assert_eq!(run(&|r| contexts(r)["checkRunCountsByState"].as_array_mut().unwrap()[0]["state"] = json!("COMPLETED")), Unknown);
    // a closed PR
    assert_eq!(run(&|r| r["state"] = json!("CLOSED")), Unknown);
}
