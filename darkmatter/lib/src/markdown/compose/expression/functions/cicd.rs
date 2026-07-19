use serde_json::Value;
use sniff::remote::{CiCdJob, CiCdJobQuery, CiCdJobReference, GitProvider};

use super::escape::collapse_and_escape;
use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::{ExpressionError, ProviderFailureKind};

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
        .ok_or_else(|| not_found("cicd", format!("CI/CD job {id} was not found")))?;
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
        let client = super::provider::build_client(resolved, context.remote_policy(), "cicd")?;
        return Ok((client, reference));
    }

    let resolved = super::provider::resolve(context, None, "cicd")?;
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
    let client = super::provider::build_client(resolved, context.remote_policy(), "cicd")?;
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
        return Ok((None, CiCdJobQuery { limit: Some(limit), ..Default::default() }));
    }
    let mut object = value.as_object().cloned().ok_or_else(|| other("cicd_list", "query must be an object or positive integer"))?;
    let remote = super::provider::authored_remote("cicd_list", object.remove("remote"))?;
    if let Some(direction) = object.remove("direction") {
        let direction = direction.as_str().ok_or_else(|| other("cicd_list", "direction must be a string"))?;
        object.insert("descending".to_string(), Value::Bool(match direction { "ascending" => false, "descending" => true, _ => return Err(other("cicd_list", "direction must be ascending or descending")) }));
    }
    let query: CiCdJobQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("cicd_list", format!("invalid query: {error}")))?;
    query
        .validate_canonical()
        .map_err(|error| other("cicd_list", error.to_string()))?;
    Ok((remote, query))
}

pub(super) fn format_job(job: &CiCdJob) -> String {
    // `display_id` is provider-supplied too, so it goes through the same
    // boundary as the job name rather than being trusted for being short.
    let label = format!("CI job #{} — {}", collapse_and_escape(&job.reference.display_id), collapse_and_escape(&job.name));
    let mut output = match job.web_url.as_deref() { Some(url) => format!("[{label}]({url})"), None => label };
    output.push_str(&format!(" · {}", collapse_and_escape(&job.normalized_status)));
    if let Some(trigger) = &job.trigger { output.push_str(&format!(" · {}", collapse_and_escape(trigger))); }
    if let Some(branch) = &job.branch { output.push_str(&format!(" · {}", collapse_and_escape(branch))); }
    if let Some(commit) = &job.commit { output.push_str(&format!(" @ {}", collapse_and_escape(&commit.chars().take(7).collect::<String>()))); }
    output
}

fn identifier(value: &Value) -> Result<String, ExpressionError> { value.as_u64().map(|id| id.to_string()).or_else(|| value.as_str().map(str::to_string)).filter(|id| !id.is_empty() && id != "0").ok_or_else(|| other("cicd", "identifier must be a positive integer or provider-native string")) }
fn other(function: &str, message: impl Into<String>) -> ExpressionError { ExpressionError::Other { function: function.to_string(), message: message.into() } }

/// A genuine provider 404: the addressed job does not exist.
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
    use sniff::remote::CiCdParentExecution;

    use super::*;
    use crate::markdown::compose::expression::functions::escape::harness;

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
            started_at: None,
            finished_at: None,
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

    /// The projection contract is about what the output *renders as*, not about
    /// which bytes carry backslashes: hostile job metadata must come back through
    /// a CommonMark+GFM parse as its own literal text, with the canonical link
    /// still intact and pointing at the untouched provider URL.
    #[test]
    fn hostile_provider_text_renders_as_literal_text() {
        let mut job = job();
        job.name =
            "**urgent** `code` _name_ ~~gone~~ <script>alert(1)</script> [ ] | $x$ {#id}".to_string();
        job.normalized_status = "fail]ed](https://evil.example)".to_string();
        job.trigger = Some("push&amp;".to_string());
        job.branch = Some("feat/*star*".to_string());

        let (destination, text) = harness::parse_literal(&format_job(&job));
        assert_eq!(destination.as_deref(), Some("https://gitlab.example/acme/widgets/-/jobs/456"));
        assert_eq!(
            text,
            "CI job #456 — **urgent** `code` _name_ ~~gone~~ <script>alert(1)</script> [ ] | $x$ {#id} \
             · fail]ed](https://evil.example) · push&amp; · feat/*star* @ abcdef1"
        );
    }

    /// A job name that already contains backslash escapes must survive as the
    /// literal characters the provider stored, not be re-interpreted.
    #[test]
    fn already_escaped_provider_text_is_not_double_mangled() {
        let mut job = job();
        job.name = r"already \[escaped\] \*text\*".to_string();
        let (_, text) = harness::parse_literal(&format_job(&job));
        assert!(text.contains(r"already \[escaped\] \*text\*"), "{text}");
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
            json!({"remote": 42}),
            json!({"remote": ""}),
            json!({"remote": "   "}),
            json!({"remote": null}),
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
            "statuses": ["failed", "cancelled"],
            "direction": "descending",
            "limit": 20
        }))
        .unwrap();
        assert!(query.descending);
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.statuses.unwrap().as_slice().len(), 2);
    }

    /// The spec spells `parent` as "number(integer) or string" because GitHub,
    /// GitLab, and Gitea number their runs while Bitbucket uses a UUID.
    #[test]
    fn parent_accepts_both_authored_spellings_and_normalizes_them() {
        let (_, numeric) = parse_query(&json!({"parent": 1234})).unwrap();
        let (_, textual) = parse_query(&json!({"parent": "1234"})).unwrap();
        assert_eq!(numeric.parent.as_deref(), Some("1234"));
        assert_eq!(numeric.parent, textual.parent);

        let (_, uuid) = parse_query(&json!({"parent": "{9a3d-0f11}"})).unwrap();
        assert_eq!(uuid.parent.as_deref(), Some("{9a3d-0f11}"));
    }

    /// Byte order rejects this ascending window; instant order accepts it.
    #[test]
    fn datetime_bounds_are_parsed_rather_than_compared_lexically() {
        let (_, query) = parse_query(&json!({
            "created_after": "2026-07-01T23:00:00+14:00",
            "created_before": "2026-07-01T10:00:00Z"
        }))
        .expect("an ascending window written across two offsets is valid");
        assert_eq!(query.created_after.as_deref(), Some("2026-07-01T23:00:00+14:00"));
    }

    /// D24: `cicd_list({})` must not sort oldest-first just because the count
    /// overload is the only form that used to set the flag.
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
