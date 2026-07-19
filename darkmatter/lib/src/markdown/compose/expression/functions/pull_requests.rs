use serde_json::Value;
use sniff::remote::{PullRequestQuery, PullRequestRecord};

use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;

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
        .ok_or_else(|| other("pr", format!("pull request {id} was not found")))?;
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
    let remote = object.remove("remote").and_then(|value| value.as_str().map(str::to_string)).filter(|value| !value.is_empty());
    if let Some(direction) = object.remove("direction") {
        let direction = direction.as_str().ok_or_else(|| other("pr_list", "direction must be a string"))?;
        object.insert("descending".to_string(), Value::Bool(match direction { "ascending" => false, "descending" => true, _ => return Err(other("pr_list", "direction must be ascending or descending")) }));
    }
    let query: PullRequestQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("pr_list", format!("invalid query: {error}")))?;
    validate_query(&query)?;
    Ok((remote, query))
}

fn validate_query(query: &PullRequestQuery) -> Result<(), ExpressionError> {
    if matches!(query.limit, Some(0 | 101..)) {
        return Err(other("pr_list", "limit must be between 1 and 100"));
    }
    for (field, after, before) in [
        ("created", query.created_after.as_ref(), query.created_before.as_ref()),
        ("updated", query.updated_after.as_ref(), query.updated_before.as_ref()),
    ] {
        if after.zip(before).is_some_and(|(after, before)| after > before) {
            return Err(other("pr_list", format!("{field} time range is inverted")));
        }
    }
    Ok(())
}

pub(super) fn format_pr(record: &PullRequestRecord) -> String {
    let title = clean(&record.details.title);
    let label = format!("PR {} — {title}", record.identity.display_id);
    let mut output = match record.identity.web_url.as_deref() { Some(url) => format!("[{label}]({url})"), None => label };
    output.push_str(&format!(" · {} · @{}", clean(&record.details.state), clean(&record.details.author)));
    if let (Some(source), Some(target)) = (&record.details.source_branch, &record.details.target_branch) { output.push_str(&format!(" · {} → {}", clean(source), clean(target))); }
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
fn clean(value: &str) -> String { value.split_whitespace().collect::<Vec<_>>().join(" ").replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]") }
fn other(function: &str, message: impl Into<String>) -> ExpressionError { ExpressionError::Other { function: function.to_string(), message: message.into() } }

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sniff::remote::{GitProvider, PullRequestInfo, PullRequestReference};

    use super::*;

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

    #[test]
    fn query_validation_rejects_bad_shapes_before_repository_resolution() {
        for value in [
            json!(0),
            json!(101),
            json!({"unknown": true}),
            json!({"direction": "sideways"}),
            json!({"state": "invalid"}),
            json!({"limit": 0}),
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
}
