use serde_json::Value;
use sniff::remote::{PullRequestQuery, PullRequestRecord};

use super::escape::collapse_and_escape;
use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::{ExpressionError, ProviderFailureKind};

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "pr", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(pr_fn)) },
    FunctionBinding { canonical: "pr_list", aliases: &["prlist"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(pr_list_fn)) },
];

fn pr_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    if args.len() != 1 { return Err(other("pr", format!("requires one identifier, got {}", args.len()))); }
    let id = positive_identifier("pr", &args[0])?;
    context.cached_provider_query("pr", format!("pr:{id}"), || {
        let (client, query_id) = if id.starts_with("http://") || id.starts_with("https://") {
            sniff::remote::FocusedProviderClient::from_pull_request_url(
                &id,
                context.remote_policy(),
            )
            .map_err(|error| other("pr", error.to_string()))?
        } else {
            (super::provider::client(context, None, "pr")?, id.clone())
        };
        let record = super::provider::run("pr", async move {
            client.get_pull_request(&query_id).await
        })?
        .ok_or_else(|| not_found("pr", format!("pull request {id} was not found")))?;
        Ok(Value::String(format_pr(&record)))
    })
}

fn pr_list_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    if args.len() != 1 { return Err(other("pr_list", format!("requires one query or count, got {}", args.len()))); }
    let (remote, query) = parse_query(&args[0])?;
    let key = format!(
        "pr_list:{}:{}",
        remote.as_deref().unwrap_or_default(),
        serde_json::to_string(&query).unwrap_or_default()
    );
    context.cached_provider_query("pr_list", key, || {
        let client = super::provider::client(context, remote.as_deref(), "pr_list")?;
        let page = super::provider::run("pr_list", async move {
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

fn parse_query(value: &Value) -> Result<(Option<String>, PullRequestQuery), ExpressionError> {
    if let Some(count) = value.as_u64() {
        let limit = usize::try_from(count).ok().filter(|count| (1..=100).contains(count)).ok_or_else(|| other("pr_list", "count must be between 1 and 100"))?;
        return Ok((None, PullRequestQuery { limit: Some(limit), ..Default::default() }));
    }
    let mut object = value.as_object().cloned().ok_or_else(|| other("pr_list", "query must be an object or positive integer"))?;
    let remote = super::provider::authored_remote("pr_list", object.remove("remote"))?;
    if let Some(direction) = object.remove("direction") {
        let direction = direction.as_str().ok_or_else(|| other("pr_list", "direction must be a string"))?;
        object.insert("descending".to_string(), Value::Bool(match direction { "ascending" => false, "descending" => true, _ => return Err(other("pr_list", "direction must be ascending or descending")) }));
    }
    let query: PullRequestQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("pr_list", format!("invalid query: {error}")))?;
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
    let mut output = match record.identity.web_url.as_deref() { Some(url) => format!("[{label}]({url})"), None => label };
    output.push_str(&format!(" · {} · @{}", collapse_and_escape(&record.details.state), collapse_and_escape(&record.details.author)));
    if let (Some(source), Some(target)) = (&record.details.source_branch, &record.details.target_branch) { output.push_str(&format!(" · {} → {}", collapse_and_escape(source), collapse_and_escape(target))); }
    output
}

fn positive_identifier(function: &str, value: &Value) -> Result<String, ExpressionError> {
    let id = value
        .as_u64()
        .map(|id| id.to_string())
        .or_else(|| value.as_str().map(str::to_string))
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
            json!(0),
            json!(101),
            json!({"unknown": true}),
            json!({"direction": "sideways"}),
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
        for value in [json!({}), json!({"limit": 5}), json!(5)] {
            let (_, query) = parse_query(&value).unwrap();
            assert!(query.descending, "{value} did not default to newest-first");
        }
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
}
