use serde::Deserialize;
use serde_json::Value;
use sniff::remote::{CiCdJob, CiCdJobQuery, CiCdJobReference, GitProvider, QueryValues};

use super::escape::{collapse_and_escape, markdown_destination};
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
        let policy = context.remote_policy();
        let job = if id.starts_with("http://") || id.starts_with("https://") {
            let (resolved, reference) =
                sniff::remote::FocusedProviderClient::job_reference_from_url(&id)
                    .map_err(|error| other("cicd", error.to_string()))?;
            super::provider::run("cicd", async move {
                let client = super::provider::connect(resolved, policy).await?;
                client.get_cicd_job(&reference).await
            })?
        } else {
            let resolved = super::provider::resolve(context, None, "cicd")?;
            let native_id = id.clone();
            // The reference is built only after the client exists, because a
            // neutral-hostname remote gets its flavor from the discovery probe
            // during construction.
            super::provider::run("cicd", async move {
                let client = super::provider::connect(resolved, policy).await?;
                let reference = repository_reference(client.remote(), &native_id)?;
                client.get_cicd_job(&reference).await
            })?
        }
        .ok_or_else(|| not_found("cicd", format!("CI/CD job {id} was not found")))?;
        Ok(Value::String(format_job(&job)))
    })
}

/// Builds the repository-scoped job reference for an already-connected remote.
fn repository_reference(
    remote: &sniff::filesystem::git::ResolvedRemote,
    native_id: &str,
) -> Result<CiCdJobReference, sniff::SniffError> {
    let provider = match remote.api_flavor {
        sniff::filesystem::git::ApiFlavor::GitHub => GitProvider::GitHub,
        sniff::filesystem::git::ApiFlavor::GitLab => GitProvider::GitLab,
        sniff::filesystem::git::ApiFlavor::Gitea
        | sniff::filesystem::git::ApiFlavor::Forgejo => GitProvider::Gitea,
        sniff::filesystem::git::ApiFlavor::Bitbucket => GitProvider::Bitbucket,
        flavor => {
            return Err(sniff::SniffError::UnsupportedRemoteCapability {
                capability: "exact CI/CD job lookup",
                target: format!("{flavor:?}"),
            });
        }
    };
    Ok(CiCdJobReference {
        provider,
        api_flavor: format!("{:?}", remote.api_flavor),
        host: remote.host.clone().unwrap_or_default(),
        namespace: remote.namespace.clone().unwrap_or_default(),
        repository: remote.repository.clone().unwrap_or_default(),
        native_id: native_id.to_string(),
        display_id: native_id.to_string(),
        original_url: None,
    })
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
        let resolved = super::provider::resolve(context, remote.as_deref(), "cicd_list")?;
        let policy = context.remote_policy();
        let page = super::provider::run("cicd_list", async move {
            let client = super::provider::connect(resolved, policy).await?;
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

/// Authored `cicd_list` query: exactly the documented catalog vocabulary.
///
/// The only path from an authored object to Sniff's internal [`CiCdJobQuery`]
/// (which no longer implements `Deserialize`); internal keys such as
/// `descending` or `cursor` are rejected by name via `deny_unknown_fields`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredCiCdJobQuery {
    statuses: Option<QueryValues<String>>,
    name: Option<String>,
    stage: Option<String>,
    workflow: Option<String>,
    /// The spec spells `parent` as "number(integer) or string": run identity
    /// is an integer on GitHub/GitLab/Gitea and an opaque UUID on Bitbucket,
    /// and both spellings normalize to the string the job matcher compares.
    #[serde(default, deserialize_with = "deserialize_parent_identity")]
    parent: Option<String>,
    branch: Option<String>,
    commit: Option<String>,
    actor: Option<String>,
    trigger: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
    updated_after: Option<String>,
    updated_before: Option<String>,
    direction: Option<String>,
    limit: Option<usize>,
}

fn deserialize_parent_identity<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ParentIdentity {
        Text(String),
        Number(u64),
    }

    Ok(
        Option::<ParentIdentity>::deserialize(deserializer)?.map(|identity| match identity {
            ParentIdentity::Text(text) => text,
            ParentIdentity::Number(number) => number.to_string(),
        }),
    )
}

fn parse_query(value: &Value) -> Result<(Option<String>, CiCdJobQuery), ExpressionError> {
    if let Some(count) = value.as_u64() {
        let limit = usize::try_from(count).ok().filter(|count| (1..=100).contains(count)).ok_or_else(|| other("cicd_list", "count must be between 1 and 100"))?;
        return Ok((None, CiCdJobQuery { limit: Some(limit), ..Default::default() }));
    }
    let mut object = value.as_object().cloned().ok_or_else(|| other("cicd_list", "query must be an object or positive integer"))?;
    let remote = super::provider::authored_remote("cicd_list", object.remove("remote"))?;
    let authored: AuthoredCiCdJobQuery = serde_json::from_value(Value::Object(object))
        .map_err(|error| other("cicd_list", format!("invalid query: {error}")))?;
    let descending =
        super::provider::authored_direction("cicd_list", authored.direction.as_deref(), None)?;
    let query = CiCdJobQuery {
        statuses: authored.statuses,
        name: authored.name,
        stage: authored.stage,
        workflow: authored.workflow,
        parent: authored.parent,
        branch: authored.branch,
        commit: authored.commit,
        actor: authored.actor,
        trigger: authored.trigger,
        created_after: authored.created_after,
        created_before: authored.created_before,
        updated_after: authored.updated_after,
        updated_before: authored.updated_before,
        descending,
        limit: authored.limit,
        cursor: None,
    };
    query
        .validate_canonical()
        .map_err(|error| other("cicd_list", error.to_string()))?;
    Ok((remote, query))
}

pub(super) fn format_job(job: &CiCdJob) -> String {
    // `display_id` is provider-supplied too, so it goes through the same
    // boundary as the job name rather than being trusted for being short.
    let label = format!("CI job #{} — {}", collapse_and_escape(&job.reference.display_id), collapse_and_escape(&job.name));
    let mut output = match job.web_url.as_deref().and_then(markdown_destination) {
        Some(url) => format!("[{label}]({url})"),
        None => label,
    };
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
                definition_id: None,
                definition_path: None,
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

    /// A destination carrying Markdown-active bytes must not be able to close
    /// its own `(...)` and take over the rest of the line.
    #[test]
    fn delimiter_bearing_destinations_cannot_escape_the_link() {
        let mut job = job();
        job.web_url = Some(
            "https://gitlab.example/acme/widgets/-/jobs/456?t=a)+**owned**+[x](https://evil.example)"
                .to_string(),
        );

        let (destination, text) = harness::parse_literal(&format_job(&job));
        assert_eq!(
            destination.as_deref(),
            Some(
                "https://gitlab.example/acme/widgets/-/jobs/456?t=a%29+**owned**+[x]%28https://evil.example%29"
            ),
            "every paren must be encoded, so none can close the destination"
        );
        assert!(text.starts_with("CI job #456 — "), "{text}");
        assert!(!text.contains("owned"), "the injected tail leaked into text: {text}");
    }

    #[test]
    fn non_web_and_unparseable_destinations_drop_the_link() {
        for hostile in [
            "javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "file:///etc/passwd",
            "vbscript:msgbox(1)",
            "/acme/widgets/-/jobs/456",
            "not a url",
            "",
        ] {
            let mut job = job();
            job.web_url = Some(hostile.to_string());
            let (destination, text) = harness::parse_literal(&format_job(&job));
            assert_eq!(destination, None, "{hostile:?} became a destination");
            assert!(
                text.starts_with("CI job #456 — "),
                "{hostile:?} cost more than the link: {text}"
            );
        }
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
            // Internal Sniff query fields are not authored vocabulary; both
            // must be rejected by name, not silently accepted or ignored.
            json!({"descending": true}),
            json!({"cursor": "20"}),
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
