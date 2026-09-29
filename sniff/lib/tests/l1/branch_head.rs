//! `remote::blocking::branch_head` and `credential_env` against loopback
//! providers.
//!
//! Shares `pr_for_branch`'s fixture: plain `#[test]`s, because the function
//! under test builds its own runtime and refuses to run inside one.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use serial_test::serial;
use sniff::filesystem::git::ApiFlavor;
use sniff::remote::blocking::{
    BranchHead, CredentialEnv, PrUnavailable, branch_head, branch_head_with, credential_env,
};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, ResponseTemplate};

use super::pr_for_branch::{
    FLAVORS, Provider, SECRET, SHA, check_credential_cases, without_tokens,
};

const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

impl Provider {
    fn head_within(&self, branch: &str, deadline: Duration) -> Result<BranchHead, PrUnavailable> {
        branch_head_with(&self.client(), branch, deadline)
    }

    fn head(&self, branch: &str) -> Result<BranchHead, PrUnavailable> {
        self.head_within(branch, Duration::from_secs(5))
    }

    /// Serves `response` for every branch-head request under this repository.
    fn serve_branch_response(&self, response: ResponseTemplate) {
        self.mount(
            Mock::given(method("GET"))
                .and(path_regex(format!("^{}", branch_prefix(self.flavor()))))
                .respond_with(response),
        );
    }

    /// Serves `sha` in the provider's own field, only at `feature`'s endpoint.
    fn serve_head(&self, sha: &str) {
        let flavor = self.flavor();
        self.mount(
            Mock::given(method("GET"))
                .and(path_regex(format!("^{}feature$", branch_prefix(flavor))))
                .respond_with(ResponseTemplate::new(200).set_body_json(head_body(flavor, sha))),
        );
    }
}

/// The request path up to the branch segment, as sent on the wire.
fn branch_prefix(flavor: ApiFlavor) -> &'static str {
    match flavor {
        ApiFlavor::GitHub => "/api/repos/acme/project/git/ref/heads/",
        ApiFlavor::GitLab => "/api/projects/acme%2Fproject/repository/branches/",
        ApiFlavor::Gitea => "/api/repos/acme/project/branches/",
        _ => "/api/repositories/acme/project/refs/branches/",
    }
}

fn head_body(flavor: ApiFlavor, sha: &str) -> Value {
    match flavor {
        ApiFlavor::GitHub => json!({
            "ref": "refs/heads/feature",
            "object": {"sha": sha, "type": "commit"}
        }),
        ApiFlavor::Bitbucket => json!({"name": "feature", "target": {"hash": sha}}),
        _ => json!({"name": "feature", "commit": {"id": sha}}),
    }
}

#[test]
#[serial]
fn each_provider_reports_its_own_sha_field() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        for sha in [SHA, SHA256] {
            let provider = Provider::start(flavor);
            provider.serve_head(sha);

            let head = provider.head("feature");

            assert_eq!(
                head,
                Ok(BranchHead {
                    sha: sha.to_string()
                }),
                "{flavor:?}"
            );
        }
    }
}

#[test]
#[serial]
fn a_head_that_is_not_a_full_lowercase_object_id_is_rejected() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        for sha in [
            SHA.to_ascii_uppercase(),
            // Bitbucket sends 12-character prefixes elsewhere.
            SHA[..12].to_string(),
            SHA[..39].to_string(),
            format!("{}g", &SHA[..39]),
            String::new(),
        ] {
            let provider = Provider::start(flavor);
            provider.serve_head(&sha);

            let result = provider.head("feature");

            assert!(
                matches!(result, Err(PrUnavailable::Other { .. })),
                "{flavor:?} {sha:?}: {result:?}"
            );
        }
    }
}

#[test]
#[serial]
fn a_body_without_the_sha_field_is_rejected() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_branch_response(
            ResponseTemplate::new(200).set_body_json(json!({"name": "feature"})),
        );

        let result = provider.head("feature");

        assert!(
            matches!(result, Err(PrUnavailable::Other { .. })),
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
        provider.serve_branch_response(
            ResponseTemplate::new(200)
                .set_body_json(head_body(flavor, SHA))
                .set_delay(Duration::from_secs(10)),
        );

        let started = Instant::now();
        let result = provider.head_within("feature", deadline);

        assert_eq!(
            result,
            Err(PrUnavailable::Timeout { deadline }),
            "{flavor:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(3), "{flavor:?}");
    }
}

#[test]
#[serial]
fn a_branch_name_is_one_encoded_path_segment() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_branch_response(
            ResponseTemplate::new(200).set_body_json(head_body(flavor, SHA)),
        );

        let head = provider.head("feature/a b/ü?x=1#y");

        assert_eq!(head.map(|head| head.sha), Ok(SHA.to_string()), "{flavor:?}");
        let requests = provider.received_requests();
        assert_eq!(requests.len(), 1, "{flavor:?}");
        assert_eq!(
            requests[0].url.path(),
            format!(
                "{}feature%2Fa%20b%2F%C3%BC%3Fx%3D1%23y",
                branch_prefix(flavor)
            ),
            "{flavor:?}"
        );
        // `fetch_json` always opens the query, so an unparameterized request
        // ends in a bare `?`.
        assert_eq!(
            requests[0].url.query().unwrap_or_default(),
            "",
            "{flavor:?}"
        );
    }
}

#[test]
#[serial]
fn without_a_token_the_request_is_anonymous_and_answers() {
    let _tokens = without_tokens();
    for flavor in FLAVORS {
        let provider = Provider::start(flavor);
        provider.serve_head(SHA);

        let head = provider.head("feature");

        assert_eq!(head.map(|head| head.sha), Ok(SHA.to_string()), "{flavor:?}");
        let requests = provider.received_requests();
        assert_eq!(requests.len(), 1, "{flavor:?}");
        for header in ["authorization", "private-token"] {
            assert!(
                !requests[0].headers.contains_key(header),
                "{flavor:?} sent {header}"
            );
        }
    }
}

#[test]
#[serial]
fn every_credentials_condition_is_classified_on_every_provider() {
    check_credential_cases(Provider::serve_branch_response, |provider| {
        provider.head("feature")
    });
}

#[test]
#[serial]
fn the_key_names_the_first_set_candidate_variable() {
    let _tokens = without_tokens();
    let _first = EnvGuard::set_safe("GH_TOKEN", SECRET);
    let _second = EnvGuard::set_safe("GITHUB_TOKEN", "sniff-test-other-token");
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_branch_response(ResponseTemplate::new(401));

    let result = provider.head("feature");

    assert_eq!(
        result,
        Err(PrUnavailable::CredentialsRejected {
            key: "GH_TOKEN".to_string()
        })
    );
    let authorization = provider.received_requests()[0]
        .headers
        .get("authorization")
        .map(|value| value.to_str().unwrap().to_string());
    assert_eq!(authorization, Some(format!("Bearer {SECRET}")));
}

#[test]
#[serial]
fn an_empty_branch_is_refused_without_a_request() {
    let _tokens = without_tokens();
    let provider = Provider::start(ApiFlavor::GitHub);
    provider.serve_head(SHA);

    let result = provider.head("");

    assert!(
        matches!(result, Err(PrUnavailable::Other { .. })),
        "{result:?}"
    );
    assert!(provider.received_requests().is_empty());
}

#[test]
fn an_unsupported_remote_is_refused_without_echoing_its_url() {
    for url in [
        format!("https://oauth2:{SECRET}@git.internal.example/acme/project.git"),
        format!("https://{SECRET}@github.com/"),
        format!("/srv/{SECRET}/project.git"),
    ] {
        let result = branch_head(&url, "main", Duration::from_secs(1));

        let error = result.expect_err(&url);
        assert!(
            matches!(error, PrUnavailable::Unsupported { .. }),
            "{url}: {error:?}"
        );
        assert!(!error.to_string().contains(SECRET), "{error}");
    }
}

#[test]
fn credential_env_lists_each_providers_variables_in_lookup_order() {
    let env = |provider: &str, variables: &[&str]| {
        Some(CredentialEnv {
            provider: provider.to_string(),
            variables: variables.iter().map(|name| (*name).to_string()).collect(),
        })
    };
    let gitea_variables = ["GITEA_TOKEN", "FORGEJO_TOKEN", "CODEBERG_TOKEN"];
    for (url, expected) in [
        (
            "https://github.com/acme/project.git",
            env("GitHub", &["GH_TOKEN", "GITHUB_TOKEN"]),
        ),
        (
            "git@github.com:acme/project.git",
            env("GitHub", &["GH_TOKEN", "GITHUB_TOKEN"]),
        ),
        (
            "https://gitlab.com/group/sub/project.git",
            env("GitLab", &["GITLAB_TOKEN", "GITLAB_PRIVATE_TOKEN"]),
        ),
        (
            "https://gitea.example.com/acme/project.git",
            env("Gitea", &gitea_variables),
        ),
        (
            "https://codeberg.org/acme/project.git",
            env("Forgejo", &gitea_variables),
        ),
        (
            "git@bitbucket.org:acme/project.git",
            env("Bitbucket", &["BITBUCKET_TOKEN"]),
        ),
        ("https://git.internal.example/acme/project.git", None),
        ("https://dev.azure.com/acme/project/_git/project", None),
        ("https://github.com/", None),
        ("/srv/git/project.git", None),
    ] {
        assert_eq!(credential_env(url), expected, "{url}");
    }
}
