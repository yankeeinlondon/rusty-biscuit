use serde_json::Value;
use sniff::remote::{CiCdJob, CiCdJobQuery, CiCdJobReference, GitProvider};

use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[
    FunctionBinding { canonical: "cicd", aliases: &[], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(cicd_fn)) },
    FunctionBinding { canonical: "cicd_list", aliases: &["cicdlist"], evaluation: EvaluationMode::Context, handler: Some(FunctionHandler::Context(cicd_list_fn)) },
];

fn cicd_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    if args.len() != 1 { return Err(other("cicd", "requires one job identifier")); }
    let id = identifier(&args[0])?;
    context.cached_provider_query("cicd", format!("cicd:{id}"), || {
        let (client, reference) = client_and_reference(context, &id)?;
        let job = super::provider::run("cicd", async move {
            client.get_cicd_job(&reference).await
        })?
        .ok_or_else(|| other("cicd", format!("CI/CD job {id} was not found")))?;
        Ok(Value::String(format_job(&job)))
    })
}

fn client_and_reference(
    context: &ResolutionContext,
    id: &str,
) -> Result<(sniff::remote::FocusedProviderClient, CiCdJobReference), ExpressionError> {
    if id.starts_with("http://") || id.starts_with("https://") {
        let (resolved, reference) =
            sniff::remote::FocusedProviderClient::job_reference_from_url(id)
                .map_err(|error| other("cicd", error.to_string()))?;
        let client = sniff::remote::FocusedProviderClient::new(resolved, context.remote_policy())
            .map_err(|error| other("cicd", error.to_string()))?;
        return Ok((client, reference));
    }

    let resolved = sniff::filesystem::git::resolve_remote_at(context.caller_dir(), None)
        .map_err(|error| other("cicd", error.to_string()))?
        .ok_or_else(|| other("cicd", "the caller repository has no usable configured remote"))?;
    let provider = match resolved.api_flavor {
        sniff::filesystem::git::ApiFlavor::GitHub => GitProvider::GitHub,
        sniff::filesystem::git::ApiFlavor::GitLab => GitProvider::GitLab,
        sniff::filesystem::git::ApiFlavor::Gitea
        | sniff::filesystem::git::ApiFlavor::Forgejo => GitProvider::Gitea,
        sniff::filesystem::git::ApiFlavor::Bitbucket => GitProvider::Bitbucket,
        flavor => {
            return Err(other(
                "cicd",
                format!("unsupported provider flavor {flavor:?}"),
            ));
        }
    };
    let reference = CiCdJobReference {
        provider,
        api_flavor: format!("{:?}", resolved.api_flavor),
        host: resolved.host.clone().unwrap_or_default(),
        namespace: resolved.namespace.clone().unwrap_or_default(),
        repository: resolved.repository.clone().unwrap_or_default(),
        native_id: id.to_string(),
        display_id: id.to_string(),
        original_url: None,
    };
    let client = sniff::remote::FocusedProviderClient::new(resolved, context.remote_policy())
        .map_err(|error| other("cicd", error.to_string()))?;
    Ok((client, reference))
}

fn cicd_list_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    if args.len() != 1 { return Err(other("cicd_list", "requires one query or count")); }
    let (remote, query) = parse_query(&args[0])?;
    let key = format!(
        "cicd_list:{}:{}",
        remote.as_deref().unwrap_or_default(),
        serde_json::to_string(&query).unwrap_or_default()
    );
    context.cached_provider_query("cicd_list", key, || {
        let client = super::provider::client(context, remote.as_deref(), "cicd_list")?;
        let page = super::provider::run("cicd_list", async move {
            client.query_cicd_jobs(query).await
        })?;
        Ok(Value::Array(
            page.items
                .iter()
                .map(|job| Value::String(format_job(job)))
                .collect(),
        ))
    })
}

fn parse_query(value: &Value) -> Result<(Option<String>, CiCdJobQuery), ExpressionError> {
    if let Some(count) = value.as_u64() {
        let limit = usize::try_from(count).ok().filter(|count| (1..=100).contains(count)).ok_or_else(|| other("cicd_list", "count must be between 1 and 100"))?;
        return Ok((None, CiCdJobQuery { limit: Some(limit), descending: true, ..Default::default() }));
    }
    let mut object = value.as_object().cloned().ok_or_else(|| other("cicd_list", "query must be an object or positive integer"))?;
    let remote = object.remove("remote").and_then(|value| value.as_str().map(str::to_string)).filter(|value| !value.is_empty());
    if let Some(direction) = object.remove("direction") {
        let direction = direction.as_str().ok_or_else(|| other("cicd_list", "direction must be a string"))?;
        object.insert("descending".to_string(), Value::Bool(match direction { "ascending" => false, "descending" => true, _ => return Err(other("cicd_list", "direction must be ascending or descending")) }));
    }
    let query: CiCdJobQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("cicd_list", format!("invalid query: {error}")))?;
    validate_query(&query)?;
    Ok((remote, query))
}

fn validate_query(query: &CiCdJobQuery) -> Result<(), ExpressionError> {
    if matches!(query.limit, Some(0 | 101..)) {
        return Err(other("cicd_list", "limit must be between 1 and 100"));
    }
    for (field, after, before) in [
        ("created", query.created_after.as_ref(), query.created_before.as_ref()),
        ("updated", query.updated_after.as_ref(), query.updated_before.as_ref()),
    ] {
        if after.zip(before).is_some_and(|(after, before)| after > before) {
            return Err(other("cicd_list", format!("{field} time range is inverted")));
        }
    }
    const STATUSES: &[&str] = &[
        "queued", "running", "success", "failed", "cancelled", "skipped", "manual",
    ];
    if query.statuses.as_ref().is_some_and(|statuses| {
        statuses.as_slice().is_empty()
            || statuses
                .as_slice()
                .iter()
                .any(|status| !STATUSES.contains(&status.as_str()))
    }) {
        return Err(other("cicd_list", "statuses contain an unsupported value"));
    }
    Ok(())
}

pub(super) fn format_job(job: &CiCdJob) -> String {
    let label = format!("CI job #{} — {}", job.reference.display_id, clean(&job.name));
    let mut output = match job.web_url.as_deref() { Some(url) => format!("[{label}]({url})"), None => label };
    output.push_str(&format!(" · {}", clean(&job.normalized_status)));
    if let Some(trigger) = &job.trigger { output.push_str(&format!(" · {}", clean(trigger))); }
    if let Some(branch) = &job.branch { output.push_str(&format!(" · {}", clean(branch))); }
    if let Some(commit) = &job.commit { output.push_str(&format!(" @ {}", clean(&commit.chars().take(7).collect::<String>()))); }
    output
}

fn identifier(value: &Value) -> Result<String, ExpressionError> { value.as_u64().map(|id| id.to_string()).or_else(|| value.as_str().map(str::to_string)).filter(|id| !id.is_empty() && id != "0").ok_or_else(|| other("cicd", "identifier must be a positive integer or provider-native string")) }
fn clean(value: &str) -> String { value.split_whitespace().collect::<Vec<_>>().join(" ").replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]") }
fn other(function: &str, message: impl Into<String>) -> ExpressionError { ExpressionError::Other { function: function.to_string(), message: message.into() } }

#[cfg(test)]
mod tests {
    use serde_json::json;
    use sniff::remote::CiCdParentExecution;

    use super::*;

    fn job() -> CiCdJob {
        CiCdJob {
            reference: CiCdJobReference {
                provider: GitProvider::GitLab,
                api_flavor: "GitLab".to_string(),
                host: "gitlab.example".to_string(),
                namespace: "acme".to_string(),
                repository: "widgets".to_string(),
                native_id: "456".to_string(),
                display_id: "456".to_string(),
                original_url: None,
            },
            parent: CiCdParentExecution {
                native_id: "99".to_string(),
                display_id: "99".to_string(),
                name: Some("build".to_string()),
                web_url: None,
            },
            name: " test  [linux] \n suite ".to_string(),
            stage: Some("test".to_string()),
            normalized_status: "failed".to_string(),
            native_status: "failed".to_string(),
            conclusion: Some("failed".to_string()),
            branch: Some("main".to_string()),
            commit: Some("abcdef123456789".to_string()),
            actor: None,
            trigger: Some("push".to_string()),
            created_at: None,
            updated_at: None,
            web_url: Some("https://gitlab.example/acme/widgets/-/jobs/456".to_string()),
            api_url: None,
            runner: None,
        }
    }

    #[test]
    fn formatter_is_deterministic_collapsed_and_markdown_escaped() {
        let expected = "[CI job #456 — test \\[linux\\] suite](https://gitlab.example/acme/widgets/-/jobs/456) · failed · push · main @ abcdef1";
        assert_eq!(format_job(&job()), expected);
        assert_eq!(format_job(&job()), expected);
    }

    #[test]
    fn query_validation_rejects_bad_shapes_before_repository_resolution() {
        for value in [
            json!(0),
            json!(101),
            json!({"unknown": true}),
            json!({"direction": "sideways"}),
            json!({"statuses": 42}),
            json!({"limit": 0}),
        ] {
            assert!(parse_query(&value).is_err(), "accepted {value}");
        }

        let (_, query) = parse_query(&json!({
            "statuses": ["failed", "cancelled"],
            "direction": "descending",
            "limit": 20
        }))
        .unwrap();
        assert!(query.descending);
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.statuses.unwrap().as_slice().len(), 2);
    }
}
