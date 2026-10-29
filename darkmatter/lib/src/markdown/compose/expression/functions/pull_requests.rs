use serde::Deserialize;
use serde_json::Value;
use sniff::remote::{
    CanonicalPullRequestState, PullRequestQuery, PullRequestRecord, QueryValues,
};

use super::escape::{collapse_and_escape, markdown_destination};
use super::{BoundCall, FunctionBinding, ResolutionContext};
use crate::markdown::compose::expression::{ExpressionError, ProviderFailureKind};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding::context("pr", &[], pr_fn),
    FunctionBinding::context("pr_list", &["prlist"], pr_list_fn),
];

fn pr_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let id = positive_identifier("pr", call.args().get(0))?;
    context.cached_provider_query("pr", format!("pr:{id}"), || {
        let record = if id.starts_with("http://") || id.starts_with("https://") {
            let (client, query_id) = sniff::remote::FocusedProviderClient::from_pull_request_url(
                &id,
                context.remote_policy(),
            )
            .map_err(|error| other("pr", error.to_string()))?;
            super::provider::run("pr", async move { client.get_pull_request(&query_id).await })?
        } else {
            let resolved = super::provider::resolve(context, None, "pr")?;
            let policy = context.remote_policy();
            let query_id = id.clone();
            super::provider::run("pr", async move {
                let client = super::provider::connect(resolved, policy).await?;
                client.get_pull_request(&query_id).await
            })?
        }
        .ok_or_else(|| not_found("pr", format!("pull request {id} was not found")))?;
        Ok(Value::String(format_pr(&record)))
    })
}

/// Overloads in catalog order: `(query: object)`, then `(count)`; a count
/// arrives already bound to a whole number from 1 to 100.
fn pr_list_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let (remote, query) = match call.args().object(0) {
        Some(object) => parse_query(object)?,
        None => (None, count_query(call.args().number(0))),
    };
    let key = format!(
        "pr_list:{}:{}",
        remote.as_deref().unwrap_or_default(),
        serde_json::to_string(&query).unwrap_or_default()
    );
    context.cached_provider_query("pr_list", key, || {
        let resolved = super::provider::resolve(context, remote.as_deref(), "pr_list")?;
        let policy = context.remote_policy();
        let page = super::provider::run("pr_list", async move {
            let client = super::provider::connect(resolved, policy).await?;
            client.query_pull_requests(query).await
        })?;
        Ok(Value::Array(
            page.items
                .iter()
                .map(|record| Value::String(format_pr(record)))
                .collect(),
        ))
    })
}

/// Authored `pr_list` query: exactly the documented catalog vocabulary.
///
/// This DTO is the only path from an authored object to Sniff's internal
/// [`PullRequestQuery`], which no longer implements `Deserialize`. Internal
/// keys such as `descending` or `cursor` are unknown here and rejected by
/// name via `deny_unknown_fields`; ordering is authored as `direction`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredPullRequestQuery {
    state: Option<QueryValues<CanonicalPullRequestState>>,
    draft: Option<bool>,
    source_branch: Option<String>,
    target_branch: Option<String>,
    author: Option<String>,
    assignee: Option<String>,
    reviewer: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    milestone: Option<String>,
    search: Option<String>,
    commit: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
    updated_after: Option<String>,
    updated_before: Option<String>,
    sort: Option<String>,
    direction: Option<String>,
    limit: Option<usize>,
}

/// The newest-first query a bare `count` asks for.
fn count_query(count: Option<f64>) -> PullRequestQuery {
    PullRequestQuery { limit: count.map(|count| count as usize), ..Default::default() }
}

fn parse_query(
    object: &serde_json::Map<String, Value>,
) -> Result<(Option<String>, PullRequestQuery), ExpressionError> {
    let mut object = object.clone();
    let remote = super::provider::authored_remote("pr_list", object.remove("remote"))?;
    let authored: AuthoredPullRequestQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("pr_list", format!("invalid query: {error}")))?;
    let descending = super::provider::authored_direction(
        "pr_list",
        authored.direction.as_deref(),
        authored.sort.as_deref(),
    )?;
    let query = PullRequestQuery {
        state: authored.state,
        source_branch: authored.source_branch,
        target_branch: authored.target_branch,
        author: authored.author,
        assignee: authored.assignee,
        reviewer: authored.reviewer,
        labels: authored.labels,
        milestone: authored.milestone,
        search: authored.search,
        commit: authored.commit,
        created_after: authored.created_after,
        created_before: authored.created_before,
        updated_after: authored.updated_after,
        updated_before: authored.updated_before,
        draft: authored.draft,
        sort: authored.sort,
        descending,
        limit: authored.limit,
        cursor: None,
    };
    query
        .validate_canonical()
        .map_err(|error| other("pr_list", error.to_string()))?;
    Ok((remote, query))
}

pub(super) fn format_pr(record: &PullRequestRecord) -> String {
    let title = collapse_and_escape(&record.details.title);
    // `display_id` is provider-supplied too, so it goes through the same
    // boundary as the title rather than being trusted for being short.
    let label = format!("PR {} — {title}", collapse_and_escape(&record.identity.display_id));
    let mut output = match record.identity.web_url.as_deref().and_then(markdown_destination) {
        Some(url) => format!("[{label}]({url})"),
        None => label,
    };
    output.push_str(&format!(" · {} · @{}", collapse_and_escape(&record.details.state), collapse_and_escape(&record.details.author)));
    if let (Some(source), Some(target)) = (&record.details.source_branch, &record.details.target_branch) { output.push_str(&format!(" · {} → {}", collapse_and_escape(source), collapse_and_escape(target))); }
    output
}

/// The bound identifier as text: a whole number from the `number` arm, or the
/// `string` arm, which must still spell a positive number or a canonical URL.
fn positive_identifier(function: &str, value: Option<&Value>) -> Result<String, ExpressionError> {
    let id = match value {
        // A whole float such as `3.0` binds `integer` too.
        Some(Value::Number(number)) => number.as_u64().or_else(|| number.as_f64().map(|id| id as u64)).map(|id| id.to_string()),
        Some(Value::String(text)) => Some(text.clone()),
        _ => None,
    }
    .filter(|id| !id.is_empty() && id != "0")
    .ok_or_else(|| other(function, "identifier must be a positive integer or canonical URL"))?;
    if id.chars().all(|character| character.is_ascii_digit())
        || id.starts_with("http://")
        || id.starts_with("https://")
    {
        Ok(id)
    } else {
        Err(other(function, "identifier must be a positive integer or canonical URL"))
    }
}
fn other(function: &str, message: impl Into<String>) -> ExpressionError { ExpressionError::Other { function: function.to_string(), message: message.into() } }

/// A genuine provider 404: the addressed item does not exist. Classified
/// distinctly from a transport failure so the surface rule can treat a
/// confirmed absence as the focused not-found kind.
fn not_found(function: &str, message: impl Into<String>) -> ExpressionError {
    ExpressionError::Provider {
        function: function.to_string(),
        kind: ProviderFailureKind::NotFound,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sniff::remote::{GitProvider, PullRequestInfo, PullRequestReference};

    use super::*;
    use crate::markdown::compose::expression::functions::escape::harness;

    /// [`super::parse_query`] for an authored object literal.
    fn parse_query(value: &Value) -> Result<(Option<String>, PullRequestQuery), ExpressionError> {
        super::parse_query(value.as_object().expect("an object query"))
    }

    /// Criterion 23: a count that converts selects the count overload with
    /// the converted number; one that cannot bind either overload is a type
    /// error naming both signatures, raised before the handler (so before any
    /// provider request).
    #[test]
    fn counts_select_the_count_overload_or_fail_before_the_handler() {
        use crate::markdown::compose::expression::functions::test_hooks::{handler_entries, reset_handler_entries};

        let registration = super::super::function_registration("pr_list").expect("registered");
        for count in [json!("3"), json!(3)] {
            let call = registration.bind_call(std::slice::from_ref(&count)).expect("binds");
            assert_eq!(call.overload(), 1, "{count} selects `count`");
            assert_eq!(call.args().get(0), Some(&json!(3)), "{count}");
        }

        reset_handler_entries();
        let context = crate::markdown::compose::expression::ResolutionContext::default();
        for count in [json!("0"), json!("2.5"), json!(0), json!(101)] {
            let error = super::super::call_function("pr_list", std::slice::from_ref(&count), Some(&context))
                .expect("registered")
                .unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{count}: {error:?}");
            let message = error.to_string();
            assert!(message.contains("no signature accepts these arguments"), "{count}: {message}");
            assert!(message.contains("pr_list(query: object)"), "{count}: {message}");
            assert!(message.contains("pr_list(count: number(integer;min(1);max(100)))"), "{count}: {message}");
        }
        assert_eq!(handler_entries("pr_list"), 0);
    }

    fn record() -> PullRequestRecord {
        PullRequestRecord {
            identity: PullRequestReference {
                provider: GitProvider::GitHub,
                api_flavor: "GitHub".to_string(),
                host: "github.example".to_string(),
                namespace: "acme".to_string(),
                repository: "widgets".to_string(),
                native_id: "123".to_string(),
                display_id: "#123".to_string(),
                number: Some(123),
                web_url: Some("https://github.example/acme/widgets/pull/123".to_string()),
                api_url: None,
                original_url: None,
            },
            details: PullRequestInfo {
                number: 123,
                title: " Fix  [parser] \n now ".to_string(),
                state: "open".to_string(),
                author: "ali[ce]".to_string(),
                draft: false,
                source_branch: Some("feature/[parser]".to_string()),
                target_branch: Some("main".to_string()),
                labels: Vec::new(),
                body: None,
                created_at: "2026-07-18T00:00:00Z".to_string(),
                updated_at: None,
                merged_at: None,
                html_url: "https://github.example/acme/widgets/pull/123".to_string(),
                source_repo: None,
                source_repo_is_target: None,
                source_head_sha: None,
            },
        }
    }

    #[test]
    fn formatter_is_deterministic_collapsed_and_markdown_escaped() {
        let expected = "[PR #123 — Fix \\[parser\\] now](https://github.example/acme/widgets/pull/123) · open · @ali\\[ce\\] · feature/\\[parser\\] → main";
        assert_eq!(format_pr(&record()), expected);
        assert_eq!(format_pr(&record()), expected);
    }

    /// The projection contract is about what the output *renders as*, not about
    /// which bytes carry backslashes: a hostile title must come back through a
    /// CommonMark+GFM parse as its own literal text, with the canonical link
    /// still intact and pointing at the untouched provider URL.
    #[test]
    fn hostile_provider_text_renders_as_literal_text() {
        let mut record = record();
        record.details.title =
            "**urgent** `code` _name_ ~~gone~~ <img src=x onerror=alert(1)> [ ] | $x$ {#id}"
                .to_string();
        record.details.author = "a]lice](https://evil.example)".to_string();
        record.details.state = "<script>".to_string();
        record.details.source_branch = Some("feat/*star*".to_string());
        record.details.target_branch = Some("main&amp;".to_string());

        let (destination, text) = harness::parse_literal(&format_pr(&record));
        assert_eq!(destination.as_deref(), Some("https://github.example/acme/widgets/pull/123"));
        assert_eq!(
            text,
            "PR #123 — **urgent** `code` _name_ ~~gone~~ <img src=x onerror=alert(1)> [ ] | $x$ {#id} \
             · <script> · @a]lice](https://evil.example) · feat/*star* → main&amp;"
        );
    }

    /// A destination carrying Markdown-active bytes must not be able to close
    /// its own `(...)` and take over the rest of the line.
    #[test]
    fn delimiter_bearing_destinations_cannot_escape_the_link() {
        let mut record = record();
        record.identity.web_url = Some(
            "https://github.example/acme/widgets/pull/123?t=a)+**owned**+[x](https://evil.example)"
                .to_string(),
        );

        let rendered = format_pr(&record);
        let (destination, text) = harness::parse_literal(&rendered);
        assert_eq!(
            destination.as_deref(),
            Some(
                "https://github.example/acme/widgets/pull/123?t=a%29+**owned**+[x]%28https://evil.example%29"
            ),
            "every paren must be encoded, so none can close the destination"
        );
        assert!(text.starts_with("PR #123 — "), "{text}");
        assert!(!text.contains("owned"), "the injected tail leaked into text: {text}");
    }

    #[test]
    fn non_web_and_unparseable_destinations_drop_the_link() {
        for hostile in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "vbscript:msgbox(1)",
            "/acme/widgets/pull/123",
            "not a url",
            "",
        ] {
            let mut record = record();
            record.identity.web_url = Some(hostile.to_string());
            let rendered = format_pr(&record);
            let (destination, text) = harness::parse_literal(&rendered);
            assert_eq!(destination, None, "{hostile:?} became a destination");
            assert!(
                text.starts_with("PR #123 — "),
                "{hostile:?} cost more than the link: {text}"
            );
        }
    }

    /// A title that already contains backslash escapes must survive as the
    /// literal characters the provider stored, not be re-interpreted.
    #[test]
    fn already_escaped_provider_text_is_not_double_mangled() {
        let mut record = record();
        record.details.title = r"already \[escaped\] \*text\*".to_string();
        let (_, text) = harness::parse_literal(&format_pr(&record));
        assert!(text.contains(r"already \[escaped\] \*text\*"), "{text}");
    }

    #[test]
    fn query_validation_rejects_bad_shapes_before_repository_resolution() {
        for value in [
            json!({"unknown": true}),
            json!({"direction": "sideways"}),
            // Internal Sniff query fields are not authored vocabulary; both
            // must be rejected by name, not silently accepted or ignored.
            json!({"descending": true}),
            json!({"cursor": "20"}),
            // `direction` orders a sort key; `provider-default` asks for the
            // provider's own order verbatim, so the pair is contradictory.
            json!({"sort": "provider-default", "direction": "ascending"}),
            json!({"sort": "provider-default", "direction": "descending"}),
            json!({"state": "invalid"}),
            json!({"limit": 0}),
            // `remote` is authored identity, not a hint: a wrong type or a
            // blank name must not degrade into "the preferred remote".
            json!({"remote": 42}),
            json!({"remote": ""}),
            json!({"remote": "   "}),
            json!({"remote": null}),
            // `draft` is an independent boolean and there is no `all`, so
            // neither is a legal `state` token.
            json!({"state": "draft"}),
            json!({"state": "all"}),
            json!({"state": ["open", "draft"]}),
            json!({"created_after": "not-a-date"}),
            json!({"updated_before": "2026-13-45"}),
            // Inverted only once the offsets are resolved: 23:00-05:00 is
            // 04:00Z the next day, so byte order calls this window ascending.
            json!({
                "created_after": "2026-06-30T23:00:00-05:00",
                "created_before": "2026-07-01T00:00:00Z"
            }),
        ] {
            assert!(parse_query(&value).is_err(), "accepted {value}");
        }

        let (_, query) = parse_query(&json!({
            "state": ["open", "merged"],
            "direction": "ascending",
            "limit": 20
        }))
        .unwrap();
        assert!(!query.descending);
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.state.unwrap().as_slice().len(), 2);
    }

    #[test]
    fn canonical_state_accepts_exactly_the_three_authored_tokens() {
        for state in ["open", "closed", "merged"] {
            let (_, query) = parse_query(&json!({"state": state}))
                .unwrap_or_else(|error| panic!("rejected {state}: {error:?}"));
            assert_eq!(query.state.unwrap().as_slice()[0].as_str(), state);
        }
    }

    /// The mirror of the inverted-range case: 23:00+14:00 sorts *after*
    /// 10:00Z as bytes but lands an hour *before* it as an instant, so byte
    /// order rejects a window that is in fact ascending.
    #[test]
    fn datetime_bounds_are_parsed_rather_than_compared_lexically() {
        let (_, query) = parse_query(&json!({
            "created_after": "2026-07-01T23:00:00+14:00",
            "created_before": "2026-07-01T10:00:00Z"
        }))
        .expect("an ascending window written across two offsets is valid");
        assert_eq!(query.created_after.as_deref(), Some("2026-07-01T23:00:00+14:00"));
    }

    /// D24: the object, empty-object, and count forms must all agree.
    #[test]
    fn every_call_form_defaults_to_newest_first() {
        for value in [json!({}), json!({"limit": 5})] {
            let (_, query) = parse_query(&value).unwrap();
            assert!(query.descending, "{value} did not default to newest-first");
        }
        assert!(count_query(Some(5.0)).descending, "a bare count did not default to newest-first");
        let (_, explicit) = parse_query(&json!({"direction": "ascending"})).unwrap();
        assert!(!explicit.descending);
    }

    #[test]
    fn authored_remote_survives_parsing_when_it_names_a_remote() {
        let (remote, _) = parse_query(&json!({"remote": "upstream"})).unwrap();
        assert_eq!(remote.as_deref(), Some("upstream"));
        let (absent, _) = parse_query(&json!({})).unwrap();
        assert_eq!(absent, None);
    }

    /// `provider-default` without a `direction` is the one valid spelling; it
    /// passes through so Sniff can preserve the provider's order verbatim.
    #[test]
    fn provider_default_sort_is_valid_only_without_a_direction() {
        let (_, query) = parse_query(&json!({"sort": "provider-default"})).unwrap();
        assert_eq!(query.sort.as_deref(), Some("provider-default"));
    }
}
