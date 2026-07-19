//! Focused provider queries that preserve errors and item identity.

use biscuit_file::FetchPolicy;
use serde_json::Value;

use crate::filesystem::git::{ApiFlavor, ResolvedRemote};
use crate::SniffError;

use super::{
    CiCdJob, CiCdJobPage, CiCdJobQuery, CiCdJobReference, CiCdParentExecution, GitProvider,
    ProviderCapabilities, PullRequestInfo, PullRequestPage, PullRequestQuery, PullRequestRecord,
    PullRequestReference,
};

const MAX_PAGES: usize = 20;
const MAX_PARENT_EXECUTIONS: usize = 20;
const MAX_JOBS_INSPECTED: usize = 2_000;

/// Provider client bound to one already-resolved configured remote.
#[derive(Debug, Clone)]
pub struct FocusedProviderClient {
    remote: ResolvedRemote,
    policy: FetchPolicy,
    api_base: url::Url,
    original_reference: Option<String>,
}

impl FocusedProviderClient {
    /// Creates a focused client using the provider's canonical API base URL.
    pub fn new(remote: ResolvedRemote, policy: FetchPolicy) -> Result<Self, SniffError> {
        let base = canonical_api_base(&remote)?;
        Self::with_api_base(remote, policy, &base)
    }

    /// Creates a focused client with an explicit enterprise/test API base.
    ///
    /// The base host is still checked against `policy` before credentials are
    /// read or an HTTP client is constructed.
    pub fn with_api_base(
        remote: ResolvedRemote,
        policy: FetchPolicy,
        api_base: &str,
    ) -> Result<Self, SniffError> {
        let mut api_base = url::Url::parse(api_base).map_err(|error| SniffError::RemoteUnreachable {
            url: api_base.to_string(),
            message: error.to_string(),
        })?;
        if !api_base.path().ends_with('/') {
            api_base.set_path(&format!("{}/", api_base.path().trim_end_matches('/')));
        }
        Ok(Self {
            remote,
            policy,
            api_base,
            original_reference: None,
        })
    }

    /// Creates a client and repository-scoped identity from a canonical PR/MR URL.
    pub fn from_pull_request_url(
        raw: &str,
        policy: FetchPolicy,
    ) -> Result<(Self, String), SniffError> {
        let (remote, id) = parse_provider_url(raw, ReferenceKind::PullRequest)?;
        let mut client = Self::new(remote, policy)?;
        client.original_reference = Some(raw.to_string());
        Ok((client, id))
    }

    /// Resolves a canonical provider job URL without performing network I/O.
    pub fn job_reference_from_url(raw: &str) -> Result<(ResolvedRemote, CiCdJobReference), SniffError> {
        let (remote, id) = parse_provider_url(raw, ReferenceKind::CiCdJob)?;
        let reference = CiCdJobReference {
            provider: git_provider(remote.api_flavor),
            api_flavor: format!("{:?}", remote.api_flavor),
            host: remote.host.clone().unwrap_or_default(),
            namespace: remote.namespace.clone().unwrap_or_default(),
            repository: remote.repository.clone().unwrap_or_default(),
            native_id: id.clone(),
            display_id: id,
            original_url: Some(raw.to_string()),
        };
        Ok((remote, reference))
    }

    /// Explicit capabilities for the selected initial provider flavor.
    pub fn capabilities(&self) -> ProviderCapabilities {
        let jobs = matches!(
            self.remote.api_flavor,
            ApiFlavor::GitHub
                | ApiFlavor::GitLab
                | ApiFlavor::Gitea
                | ApiFlavor::Forgejo
                | ApiFlavor::Bitbucket
        );
        ProviderCapabilities {
            pull_requests: jobs,
            cicd_jobs: jobs,
            pagination: jobs,
            direct_job_listing: matches!(self.remote.api_flavor, ApiFlavor::GitLab),
            bounded_parent_traversal: matches!(
                self.remote.api_flavor,
                ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo | ApiFlavor::Bitbucket
            ),
            logs: false,
            artifacts: false,
            test_reports: false,
            pull_request_filters: [
                "state", "draft", "source_branch", "target_branch", "author", "labels",
                "search", "created_after", "created_before", "updated_after", "updated_before",
                "sort", "direction", "limit",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            cicd_job_filters: [
                "statuses", "name", "stage", "workflow", "parent", "branch", "commit",
                "actor", "trigger", "created_after", "created_before", "updated_after",
                "updated_before", "direction", "limit",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        }
    }

    /// Gets one exact pull/merge request. Authoritative 404 is `Ok(None)`.
    pub async fn get_pull_request(
        &self,
        native_id: &str,
    ) -> Result<Option<PullRequestRecord>, SniffError> {
        positive_id(native_id, "id")?;
        let path = self.pr_exact_path(native_id)?;
        let Some(value) = self.get_json(&path, &[]).await? else {
            return Ok(None);
        };
        Ok(Some(self.normalize_pr(value)?))
    }

    /// Queries PRs with bounded provider pagination and exact local filters.
    pub async fn query_pull_requests(
        &self,
        mut query: PullRequestQuery,
    ) -> Result<PullRequestPage, SniffError> {
        validate_pr_query(&query, self.remote.api_flavor)?;
        if query.state.is_none() {
            query.state = Some(super::QueryValues::One(super::PullRequestState::Open));
        }
        let limit = query.limit.unwrap_or(20);
        let page_size = limit.clamp(1, 100);
        let mut normalized = Vec::new();
        let mut exhausted = false;
        for page in 1..=MAX_PAGES {
            let path = self.pr_list_path()?;
            let params = self.pr_page_params(&query, page, page_size);
            let Some(value) = self.get_json(&path, &params).await? else {
                exhausted = true;
                break;
            };
            let (items, next) = page_items(value);
            let count = items.len();
            for item in items {
                let record = self.normalize_pr(item)?;
                if pr_matches(&record.details, &query) {
                    normalized.push(record);
                    if normalized.len() == limit {
                        break;
                    }
                }
            }
            if normalized.len() == limit {
                break;
            }
            if !next && count < page_size {
                exhausted = true;
                break;
            }
        }
        sort_prs(&mut normalized, query.sort.as_deref(), query.descending);
        Ok(PullRequestPage {
            total: exhausted.then_some(normalized.len()),
            next: (!exhausted && normalized.len() == limit).then(|| "provider-next".to_string()),
            items: normalized,
            warnings: Vec::new(),
        })
    }

    /// Gets one exact CI/CD job. Authoritative 404 is `Ok(None)`.
    pub async fn get_cicd_job(
        &self,
        reference: &CiCdJobReference,
    ) -> Result<Option<CiCdJob>, SniffError> {
        let path = self.job_exact_path(&reference.native_id)?;
        let Some(value) = self.get_json(&path, &[]).await? else {
            return Ok(None);
        };
        let parent = if self.remote.api_flavor == ApiFlavor::Bitbucket {
            reference.native_id.split_once('/').map(|(parent, _)| CiCdParentExecution {
                native_id: parent.to_string(),
                display_id: parent.to_string(),
                name: None,
                web_url: None,
            })
        } else {
            None
        };
        let mut job = self.normalize_job(value, parent)?;
        job.reference = reference.clone();
        Ok(Some(job))
    }

    /// Queries jobs through a direct endpoint or bounded parent traversal.
    pub async fn query_cicd_jobs(
        &self,
        query: CiCdJobQuery,
    ) -> Result<CiCdJobPage, SniffError> {
        validate_job_query(&query)?;
        let limit = query.limit.unwrap_or(20);
        let mut jobs = if self.remote.api_flavor == ApiFlavor::GitLab {
            self.direct_jobs(&query, limit).await?
        } else {
            self.jobs_via_parents(&query, limit).await?
        };
        jobs.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        if query.descending {
            jobs.reverse();
        }
        jobs.truncate(limit);
        Ok(CiCdJobPage {
            total: None,
            next: (jobs.len() == limit).then(|| "provider-next".to_string()),
            items: jobs,
            warnings: vec!["provider did not expose an authoritative total".to_string()],
        })
    }

    async fn direct_jobs(
        &self,
        query: &CiCdJobQuery,
        limit: usize,
    ) -> Result<Vec<CiCdJob>, SniffError> {
        let mut jobs = Vec::new();
        let page_size = 100;
        let mut inspected = 0;
        for page in 1..=MAX_PAGES {
            let path = format!("projects/{}/jobs", encoded_project(&self.remote));
            let params = vec![("page".to_string(), page.to_string()), ("per_page".to_string(), page_size.to_string())];
            let Some(value) = self.get_json(&path, &params).await? else { break };
            let (items, next) = page_items(value);
            let count = items.len();
            for item in items {
                inspected += 1;
                let job = self.normalize_job(item, None)?;
                if job_matches(&job, query) {
                    jobs.push(job);
                    if jobs.len() == limit {
                        return Ok(jobs);
                    }
                }
                if inspected == MAX_JOBS_INSPECTED {
                    return Ok(jobs);
                }
            }
            if !next && count < page_size { break; }
        }
        Ok(jobs)
    }

    async fn jobs_via_parents(
        &self,
        query: &CiCdJobQuery,
        limit: usize,
    ) -> Result<Vec<CiCdJob>, SniffError> {
        let parent_path = self.parent_list_path()?;
        let mut jobs = Vec::new();
        let mut parents_inspected = 0;
        let mut jobs_inspected = 0;
        for parent_page in 1..=MAX_PAGES {
            let params = pagination_params(
                self.remote.api_flavor,
                parent_page,
                MAX_PARENT_EXECUTIONS,
            );
            let Some(value) = self.get_json(&parent_path, &params).await? else {
                break;
            };
            let (parents, next_parent_page) =
                page_items_named(value, &["workflow_runs", "values"]);
            let parent_count = parents.len();
            for parent in parents {
                if parents_inspected == MAX_PARENT_EXECUTIONS {
                    return Ok(jobs);
                }
                parents_inspected += 1;
                let parent_id = string_id(&parent, &["id", "uuid"])?;
                let parent_identity = parent_execution(&parent, &parent_id);
                let path = self.parent_jobs_path(&parent_id)?;
                for job_page in 1..=MAX_PAGES {
                    let params = pagination_params(self.remote.api_flavor, job_page, 100);
                    let Some(value) = self.get_json(&path, &params).await? else {
                        break;
                    };
                    let (items, next_job_page) =
                        page_items_named(value, &["jobs", "values"]);
                    let job_count = items.len();
                    for item in items {
                        jobs_inspected += 1;
                        let job = self.normalize_job(item, Some(parent_identity.clone()))?;
                        if job_matches(&job, query) {
                            jobs.push(job);
                            if jobs.len() == limit {
                                return Ok(jobs);
                            }
                        }
                        if jobs_inspected == MAX_JOBS_INSPECTED {
                            return Ok(jobs);
                        }
                    }
                    if !next_job_page && job_count < 100 {
                        break;
                    }
                }
            }
            if !next_parent_page && parent_count < MAX_PARENT_EXECUTIONS {
                break;
            }
        }
        Ok(jobs)
    }

    async fn get_json(
        &self,
        path: &str,
        params: &[(String, String)],
    ) -> Result<Option<Value>, SniffError> {
        let mut endpoint = self.api_base.join(path).map_err(|error| SniffError::RemoteUnreachable {
            url: self.api_base.to_string(), message: error.to_string(),
        })?;
        endpoint.query_pairs_mut().extend_pairs(params.iter().map(|(k, v)| (k, v)));
        let remote_host = self.remote.host.as_deref().unwrap_or_default();
        if !self.policy.is_allowed(remote_host) {
            return Err(SniffError::RemotePolicyDenied {
                host: remote_host.to_string(),
            });
        }
        let endpoint_host = endpoint.host_str().unwrap_or_default();
        if !provider_endpoint_allowed(remote_host, endpoint_host, self.remote.api_flavor) {
            return Err(SniffError::RemotePolicyDenied {
                host: endpoint_host.to_string(),
            });
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| transport(&endpoint, error))?;
        let (token, variable) = credential(self.remote.api_flavor);
        let mut request = client.get(endpoint.clone()).header(reqwest::header::USER_AGENT, "sniff/focused-provider");
        if let Some(token) = token.as_ref() {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.map_err(|error| transport(&endpoint, error))?;
        let status = response.status().as_u16();
        match status {
            200..=299 => response.json().await.map(Some).map_err(|error| SniffError::RemoteApi {
                provider: provider_name(self.remote.api_flavor), status,
                message: format!("malformed JSON response: {error}"),
            }),
            404 => Ok(None),
            401 if token.is_none() => Err(SniffError::MissingCredentials {
                provider: provider_name(self.remote.api_flavor), env_var: variable.to_string(),
            }),
            401 => Err(SniffError::InvalidCredentials {
                provider: provider_name(self.remote.api_flavor), message: "provider rejected credentials".to_string(),
            }),
            403 => Err(SniffError::RemoteForbidden {
                provider: provider_name(self.remote.api_flavor), message: "provider denied the query".to_string(),
            }),
            429 => Err(SniffError::RateLimited { provider: provider_name(self.remote.api_flavor), retry_after: None }),
            300..=399 => Err(SniffError::RemoteUnreachable {
                url: endpoint.to_string(), message: "redirect blocked".to_string(),
            }),
            status => Err(SniffError::RemoteApi {
                provider: provider_name(self.remote.api_flavor), status, message: "provider query failed".to_string(),
            }),
        }
    }

    fn pr_exact_path(&self, id: &str) -> Result<String, SniffError> {
        let base = repo_path(&self.remote);
        Ok(match self.remote.api_flavor {
            ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("repos/{base}/pulls/{id}"),
            ApiFlavor::GitLab => format!("projects/{}/merge_requests/{id}", encoded_project(&self.remote)),
            ApiFlavor::Bitbucket => format!("repositories/{base}/pullrequests/{id}"),
            _ => return Err(unsupported("pull-request lookup", self.remote.api_flavor)),
        })
    }

    fn pr_list_path(&self) -> Result<String, SniffError> {
        let base = repo_path(&self.remote);
        Ok(match self.remote.api_flavor {
            ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("repos/{base}/pulls"),
            ApiFlavor::GitLab => format!("projects/{}/merge_requests", encoded_project(&self.remote)),
            ApiFlavor::Bitbucket => format!("repositories/{base}/pullrequests"),
            _ => return Err(unsupported("pull-request query", self.remote.api_flavor)),
        })
    }

    fn pr_page_params(&self, query: &PullRequestQuery, page: usize, size: usize) -> Vec<(String, String)> {
        let state = query.state.as_ref()
            .and_then(|states| (states.as_slice().len() == 1).then(|| states.as_slice()[0].as_str().to_string()))
            .unwrap_or_else(|| "all".to_string());
        if self.remote.api_flavor == ApiFlavor::Bitbucket {
            vec![("page".to_string(), page.to_string()), ("pagelen".to_string(), size.to_string()), ("state".to_string(), state.to_ascii_uppercase())]
        } else {
            vec![("page".to_string(), page.to_string()), ("per_page".to_string(), size.to_string()), ("state".to_string(), state)]
        }
    }

    fn job_exact_path(&self, native_id: &str) -> Result<String, SniffError> {
        let base = repo_path(&self.remote);
        Ok(match self.remote.api_flavor {
            ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("repos/{base}/actions/jobs/{native_id}"),
            ApiFlavor::GitLab => format!("projects/{}/jobs/{native_id}", encoded_project(&self.remote)),
            ApiFlavor::Bitbucket => {
                let (parent, job) = native_id.split_once('/').ok_or_else(|| SniffError::InvalidRemoteQuery {
                    field: "id", message: "Bitbucket job identity must be parent/job".to_string(),
                })?;
                format!("repositories/{base}/pipelines/{parent}/steps/{job}")
            }
            _ => return Err(unsupported("exact CI/CD job lookup", self.remote.api_flavor)),
        })
    }

    fn parent_list_path(&self) -> Result<String, SniffError> {
        let base = repo_path(&self.remote);
        Ok(match self.remote.api_flavor {
            ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("repos/{base}/actions/runs"),
            ApiFlavor::Bitbucket => format!("repositories/{base}/pipelines"),
            _ => return Err(unsupported("CI/CD parent traversal", self.remote.api_flavor)),
        })
    }

    fn parent_jobs_path(&self, parent: &str) -> Result<String, SniffError> {
        let base = repo_path(&self.remote);
        Ok(match self.remote.api_flavor {
            ApiFlavor::GitHub | ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("repos/{base}/actions/runs/{parent}/jobs"),
            ApiFlavor::Bitbucket => format!("repositories/{base}/pipelines/{parent}/steps"),
            _ => return Err(unsupported("CI/CD parent jobs", self.remote.api_flavor)),
        })
    }

    fn normalize_pr(&self, value: Value) -> Result<PullRequestRecord, SniffError> {
        let number = value_u64(&value, &["number", "iid", "id"])?;
        let details = PullRequestInfo {
            number,
            title: value_string(&value, &["title"]).unwrap_or_default(),
            state: value_string(&value, &["state"]).unwrap_or_else(|| "unknown".to_string()).to_ascii_lowercase(),
            author: nested_string(&value, &[&["user", "login"], &["author", "username"], &["author", "display_name"]]).unwrap_or_else(|| "unknown".to_string()),
            draft: value_bool(&value, &["draft", "work_in_progress"]).unwrap_or(false),
            source_branch: nested_string(&value, &[&["head", "ref"], &["source", "branch", "name"]]).or_else(|| value_string(&value, &["source_branch"])),
            target_branch: nested_string(&value, &[&["base", "ref"], &["destination", "branch", "name"]]).or_else(|| value_string(&value, &["target_branch"])),
            labels: value.get("labels").and_then(Value::as_array).map(|labels| labels.iter().filter_map(|label| label.as_str().map(str::to_string).or_else(|| value_string(label, &["name"]))).collect()).unwrap_or_default(),
            body: value_string(&value, &["body", "description"]),
            created_at: value_string(&value, &["created_at", "created_on"]).unwrap_or_default(),
            updated_at: value_string(&value, &["updated_at", "updated_on"]),
            merged_at: value_string(&value, &["merged_at"]),
            html_url: nested_string(&value, &[&["links", "html", "href"]]).or_else(|| value_string(&value, &["html_url", "web_url"])).unwrap_or_default(),
        };
        Ok(PullRequestRecord {
            identity: PullRequestReference {
                provider: git_provider(self.remote.api_flavor),
                api_flavor: format!("{:?}", self.remote.api_flavor),
                host: self.remote.host.clone().unwrap_or_default(),
                namespace: self.remote.namespace.clone().unwrap_or_default(),
                repository: self.remote.repository.clone().unwrap_or_default(),
                native_id: number.to_string(), display_id: format!("#{number}"), number: Some(number),
                web_url: (!details.html_url.is_empty()).then(|| details.html_url.clone()),
                api_url: value_string(&value, &["url"]),
                original_url: self.original_reference.clone(),
            },
            details,
        })
    }

    fn normalize_job(&self, value: Value, parent: Option<CiCdParentExecution>) -> Result<CiCdJob, SniffError> {
        let id = string_id(&value, &["id", "uuid"])?;
        let raw_status = value_string(&value, &["status", "state"]).unwrap_or_else(|| "unknown".to_string());
        let web_url = nested_string(&value, &[&["links", "html", "href"]]).or_else(|| value_string(&value, &["html_url", "web_url"]));
        let parent = parent.unwrap_or_else(|| {
            let parent_id = value_id(&value, &["run_id", "pipeline_id"]).unwrap_or_default();
            CiCdParentExecution { native_id: parent_id.clone(), display_id: parent_id, name: None, web_url: None }
        });
        Ok(CiCdJob {
            reference: CiCdJobReference {
                provider: git_provider(self.remote.api_flavor), api_flavor: format!("{:?}", self.remote.api_flavor),
                host: self.remote.host.clone().unwrap_or_default(), namespace: self.remote.namespace.clone().unwrap_or_default(),
                repository: self.remote.repository.clone().unwrap_or_default(), native_id: id.clone(), display_id: id,
                original_url: web_url.clone(),
            },
            parent,
            name: value_string(&value, &["name"]).unwrap_or_else(|| "unnamed job".to_string()),
            stage: value_string(&value, &["stage"]), normalized_status: normalize_status(&raw_status), native_status: raw_status,
            conclusion: value_string(&value, &["conclusion"]), branch: value_string(&value, &["head_branch", "ref"]),
            commit: value_string(&value, &["head_sha", "sha"]), actor: nested_string(&value, &[&["actor", "login"], &["user", "username"]]),
            trigger: value_string(&value, &["event", "source"]), created_at: value_string(&value, &["created_at", "created_on", "started_at"]),
            updated_at: value_string(&value, &["updated_at", "updated_on", "completed_at"]), web_url,
            api_url: value_string(&value, &["url"]), runner: nested_string(&value, &[&["runner", "name"]]),
        })
    }
}

#[derive(Clone, Copy)]
enum ReferenceKind {
    PullRequest,
    CiCdJob,
}

fn parse_provider_url(
    raw: &str,
    kind: ReferenceKind,
) -> Result<(ResolvedRemote, String), SniffError> {
    let url = url::Url::parse(raw).map_err(|error| SniffError::InvalidRemoteQuery {
        field: "id",
        message: format!("expected a positive native ID or canonical provider URL: {error}"),
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.query().is_some() || url.fragment().is_some() {
        return Err(SniffError::InvalidRemoteQuery {
            field: "id",
            message: "canonical provider URLs must be HTTP(S) URLs without query or fragment"
                .to_string(),
        });
    }
    let host = url.host_str().ok_or_else(|| SniffError::InvalidRemoteQuery {
        field: "id",
        message: "canonical provider URL is missing a host".to_string(),
    })?;
    let segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let parsed = match kind {
        ReferenceKind::PullRequest => parse_pr_segments(host, &segments),
        ReferenceKind::CiCdJob => parse_job_segments(host, &segments),
    }
    .ok_or_else(|| SniffError::InvalidRemoteQuery {
        field: "id",
        message: "URL is not a canonical supported-provider reference".to_string(),
    })?;
    let (api_flavor, namespace, repository, native_id) = parsed;
    positive_id(&native_id, "id")?;
    let repository_url = format!(
        "{}://{}/{}/{}.git",
        url.scheme(),
        host,
        namespace,
        repository
    );
    Ok((
        ResolvedRemote {
            name: raw.to_string(),
            fetch_url: repository_url.clone(),
            push_url: repository_url,
            host: Some(host.to_string()),
            namespace: Some(namespace),
            repository: Some(repository),
            api_flavor,
        },
        native_id,
    ))
}

fn parse_pr_segments(
    host: &str,
    segments: &[&str],
) -> Option<(ApiFlavor, String, String, String)> {
    let (marker, flavor, repository_offset) = if host.eq_ignore_ascii_case("bitbucket.org") {
        ("pull-requests", ApiFlavor::Bitbucket, 1)
    } else if segments.windows(2).any(|window| window == ["-", "merge_requests"]) {
        ("merge_requests", ApiFlavor::GitLab, 2)
    } else if segments.contains(&"pull") {
        ("pull", ApiFlavor::GitHub, 1)
    } else if segments.contains(&"pulls") {
        let flavor = if host.to_ascii_lowercase().contains("forgejo") {
            ApiFlavor::Forgejo
        } else {
            ApiFlavor::Gitea
        };
        ("pulls", flavor, 1)
    } else {
        return None;
    };
    let marker_index = segments.iter().position(|segment| *segment == marker)?;
    let repository_index = marker_index.checked_sub(repository_offset)?;
    let repository = segments.get(repository_index)?.to_string();
    let namespace = segments.get(..repository_index)?.join("/");
    let native_id = segments.get(marker_index + 1)?.to_string();
    (!namespace.is_empty()).then_some((flavor, namespace, repository, native_id))
}

fn parse_job_segments(
    host: &str,
    segments: &[&str],
) -> Option<(ApiFlavor, String, String, String)> {
    if let Some(marker) = segments.iter().position(|segment| *segment == "jobs")
        && marker >= 2 && segments[marker - 1] == "-"
    {
        return Some((
            ApiFlavor::GitLab,
            segments[..marker - 2].join("/"),
            segments[marker - 2].to_string(),
            segments.get(marker + 1)?.to_string(),
        ));
    }
    if let Some(actions) = segments.iter().position(|segment| *segment == "actions") {
        let flavor = if host.eq_ignore_ascii_case("github.com")
            || segments.contains(&"job")
        {
            ApiFlavor::GitHub
        } else if host.to_ascii_lowercase().contains("forgejo") {
            ApiFlavor::Forgejo
        } else {
            ApiFlavor::Gitea
        };
        let native_id = segments.last()?.to_string();
        return (actions >= 2).then(|| {
            (
                flavor,
                segments[..actions - 1].join("/"),
                segments[actions - 1].to_string(),
                native_id,
            )
        });
    }
    if host.eq_ignore_ascii_case("bitbucket.org") {
        let pipelines = segments.iter().position(|segment| *segment == "pipelines")?;
        let steps = segments.iter().position(|segment| *segment == "steps")?;
        let parent = if segments.get(pipelines + 1) == Some(&"results") {
            segments.get(pipelines + 2)?
        } else {
            segments.get(pipelines + 1)?
        };
        let job = segments.get(steps + 1)?;
        return (pipelines >= 2).then(|| {
            (
                ApiFlavor::Bitbucket,
                segments[..pipelines - 1].join("/"),
                segments[pipelines - 1].to_string(),
                format!("{parent}/{job}"),
            )
        });
    }
    None
}

fn canonical_api_base(remote: &ResolvedRemote) -> Result<String, SniffError> {
    let host = remote.host.as_deref().unwrap_or_default();
    Ok(match remote.api_flavor {
        ApiFlavor::GitHub if host == "github.com" => "https://api.github.com/".to_string(),
        ApiFlavor::GitHub => format!("https://{host}/api/v3/"),
        ApiFlavor::GitLab => format!("https://{host}/api/v4/"),
        ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("https://{host}/api/v1/"),
        ApiFlavor::Bitbucket => "https://api.bitbucket.org/2.0/".to_string(),
        _ => return Err(unsupported("provider queries", remote.api_flavor)),
    })
}

fn provider_endpoint_allowed(remote_host: &str, endpoint_host: &str, flavor: ApiFlavor) -> bool {
    remote_host.eq_ignore_ascii_case(endpoint_host)
        || (flavor == ApiFlavor::GitHub
            && remote_host.eq_ignore_ascii_case("github.com")
            && endpoint_host.eq_ignore_ascii_case("api.github.com"))
        || (flavor == ApiFlavor::Bitbucket
            && remote_host.eq_ignore_ascii_case("bitbucket.org")
            && endpoint_host.eq_ignore_ascii_case("api.bitbucket.org"))
}

fn validate_pr_query(query: &PullRequestQuery, flavor: ApiFlavor) -> Result<(), SniffError> {
    validate_limit(query.limit)?;
    if query.state.as_ref().is_some_and(|states| states.as_slice().is_empty()) {
        return Err(SniffError::InvalidRemoteQuery { field: "state", message: "must not be empty".to_string() });
    }
    if query.sort.as_deref().is_some_and(|sort| !matches!(sort, "created" | "updated" | "provider-default")) {
        return Err(SniffError::InvalidRemoteQuery { field: "sort", message: "expected created, updated, or provider-default".to_string() });
    }
    for (field, after, before) in [
        ("created_after", query.created_after.as_ref(), query.created_before.as_ref()),
        ("updated_after", query.updated_after.as_ref(), query.updated_before.as_ref()),
    ] {
        if after.zip(before).is_some_and(|(after, before)| after > before) {
            return Err(SniffError::InvalidRemoteQuery { field, message: "range is inverted".to_string() });
        }
    }
    for (field, present) in [("assignee", query.assignee.is_some()), ("reviewer", query.reviewer.is_some()), ("milestone", query.milestone.is_some()), ("commit", query.commit.is_some())] {
        if present { return Err(SniffError::UnsupportedRemoteFilter { field, provider: format!("{flavor:?}") }); }
    }
    Ok(())
}

fn validate_job_query(query: &CiCdJobQuery) -> Result<(), SniffError> {
    validate_limit(query.limit)?;
    if query.created_after.as_ref().zip(query.created_before.as_ref()).is_some_and(|(a, b)| a > b) {
        return Err(SniffError::InvalidRemoteQuery { field: "created_after", message: "must not be later than created_before".to_string() });
    }
    if query.updated_after.as_ref().zip(query.updated_before.as_ref()).is_some_and(|(a, b)| a > b) {
        return Err(SniffError::InvalidRemoteQuery { field: "updated_after", message: "must not be later than updated_before".to_string() });
    }
    const STATUSES: &[&str] = &["queued", "running", "success", "failed", "cancelled", "skipped", "manual"];
    if let Some(statuses) = &query.statuses
        && (statuses.as_slice().is_empty()
            || statuses
                .as_slice()
                .iter()
                .any(|status| !STATUSES.contains(&status.as_str())))
    {
        return Err(SniffError::InvalidRemoteQuery {
            field: "statuses",
            message: format!("expected one or more of: {}", STATUSES.join(", ")),
        });
    }
    Ok(())
}

fn validate_limit(limit: Option<usize>) -> Result<(), SniffError> {
    if matches!(limit, Some(0 | 101..)) {
        Err(SniffError::InvalidRemoteQuery { field: "limit", message: "must be between 1 and 100".to_string() })
    } else { Ok(()) }
}

fn positive_id(id: &str, field: &'static str) -> Result<(), SniffError> {
    if id.is_empty() || id == "0" { Err(SniffError::InvalidRemoteQuery { field, message: "must be a positive provider identifier".to_string() }) } else { Ok(()) }
}

fn page_items(value: Value) -> (Vec<Value>, bool) { page_items_named(value, &["values"]) }

fn pagination_params(flavor: ApiFlavor, page: usize, size: usize) -> Vec<(String, String)> {
    let size_key = if flavor == ApiFlavor::Bitbucket {
        "pagelen"
    } else {
        "per_page"
    };
    vec![
        ("page".to_string(), page.to_string()),
        (size_key.to_string(), size.to_string()),
    ]
}

fn page_items_named(value: Value, names: &[&str]) -> (Vec<Value>, bool) {
    if let Value::Array(items) = value { return (items, false); }
    let next = value.get("next").is_some_and(|next| !next.is_null());
    for name in names { if let Some(items) = value.get(*name).and_then(Value::as_array) { return (items.clone(), next); } }
    (Vec::new(), next)
}

fn pr_matches(item: &PullRequestInfo, query: &PullRequestQuery) -> bool {
    query.state.as_ref().is_none_or(|states| states.as_slice().iter().any(|state| match state {
        super::PullRequestState::Open => item.state == "open" || item.state == "opened",
        super::PullRequestState::Closed => matches!(item.state.as_str(), "closed" | "declined" | "superseded"),
        super::PullRequestState::Merged => item.state == "merged" || item.merged_at.is_some(),
        super::PullRequestState::Draft => item.draft,
        super::PullRequestState::All => true,
    }))
        && query.source_branch.as_ref().is_none_or(|v| item.source_branch.as_ref() == Some(v))
        && query.target_branch.as_ref().is_none_or(|v| item.target_branch.as_ref() == Some(v))
        && query.author.as_ref().is_none_or(|v| item.author.eq_ignore_ascii_case(v))
        && query.draft.is_none_or(|v| item.draft == v)
        && query.labels.iter().all(|v| item.labels.iter().any(|a| a.eq_ignore_ascii_case(v)))
        && query.search.as_ref().is_none_or(|v| { let v = v.to_ascii_lowercase(); item.title.to_ascii_lowercase().contains(&v) || item.body.as_ref().is_some_and(|b| b.to_ascii_lowercase().contains(&v)) })
        && query.created_after.as_ref().is_none_or(|v| &item.created_at >= v)
        && query.created_before.as_ref().is_none_or(|v| &item.created_at <= v)
        && query.updated_after.as_ref().is_none_or(|v| item.updated_at.as_ref().is_some_and(|a| a >= v))
        && query.updated_before.as_ref().is_none_or(|v| item.updated_at.as_ref().is_some_and(|a| a <= v))
}

fn sort_prs(items: &mut [PullRequestRecord], sort: Option<&str>, descending: bool) {
    match sort.unwrap_or("provider-default") {
        "created" => items.sort_by(|a, b| a.details.created_at.cmp(&b.details.created_at)),
        "updated" => items.sort_by(|a, b| a.details.updated_at.cmp(&b.details.updated_at)),
        _ => {}
    }
    if descending { items.reverse(); }
}

fn job_matches(job: &CiCdJob, query: &CiCdJobQuery) -> bool {
    query.statuses.as_ref().is_none_or(|statuses| statuses.as_slice().iter().any(|s| s.eq_ignore_ascii_case(&job.normalized_status)))
        && query.name.as_ref().is_none_or(|v| &job.name == v)
        && query.stage.as_ref().is_none_or(|v| job.stage.as_ref() == Some(v))
        && query.workflow.as_ref().is_none_or(|v| job.parent.name.as_ref() == Some(v) || &job.parent.native_id == v)
        && query.parent.as_ref().is_none_or(|v| &job.parent.native_id == v)
        && query.branch.as_ref().is_none_or(|v| job.branch.as_ref() == Some(v))
        && query.commit.as_ref().is_none_or(|v| job.commit.as_ref() == Some(v))
        && query.actor.as_ref().is_none_or(|v| job.actor.as_ref() == Some(v))
        && query.trigger.as_ref().is_none_or(|v| job.trigger.as_ref() == Some(v))
        && query.created_after.as_ref().is_none_or(|v| job.created_at.as_ref().is_some_and(|a| a >= v))
        && query.created_before.as_ref().is_none_or(|v| job.created_at.as_ref().is_some_and(|a| a <= v))
        && query.updated_after.as_ref().is_none_or(|v| job.updated_at.as_ref().is_some_and(|a| a >= v))
        && query.updated_before.as_ref().is_none_or(|v| job.updated_at.as_ref().is_some_and(|a| a <= v))
}

fn parent_execution(value: &Value, id: &str) -> CiCdParentExecution {
    CiCdParentExecution { native_id: id.to_string(), display_id: id.to_string(), name: value_string(value, &["name"]), web_url: value_string(value, &["html_url", "web_url"]) }
}

fn repo_path(remote: &ResolvedRemote) -> String { format!("{}/{}", remote.namespace.as_deref().unwrap_or_default(), remote.repository.as_deref().unwrap_or_default()) }
fn encoded_project(remote: &ResolvedRemote) -> String { urlencoding::encode(&repo_path(remote)).into_owned() }
fn value_string(value: &Value, names: &[&str]) -> Option<String> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str()).map(str::to_string)) }
fn value_bool(value: &Value, names: &[&str]) -> Option<bool> { names.iter().find_map(|name| value.get(*name).and_then(Value::as_bool)) }
fn value_u64(value: &Value, names: &[&str]) -> Result<u64, SniffError> { names.iter().find_map(|name| value.get(*name).and_then(Value::as_u64)).ok_or_else(|| malformed("missing numeric identity")) }
fn value_id(value: &Value, names: &[&str]) -> Option<String> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|id| id.to_string())))) }
fn string_id(value: &Value, names: &[&str]) -> Result<String, SniffError> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|id| id.to_string())))).ok_or_else(|| malformed("missing job identity")) }
fn nested_string(value: &Value, paths: &[&[&str]]) -> Option<String> { paths.iter().find_map(|path| path.iter().try_fold(value, |v, key| v.get(*key)).and_then(Value::as_str).map(str::to_string)) }
fn normalize_status(status: &str) -> String { match status.to_ascii_lowercase().as_str() { "success" | "completed" => "success", "failure" | "failed" => "failed", "cancelled" | "canceled" => "cancelled", "queued" | "pending" | "created" => "queued", "running" | "in_progress" => "running", other => other }.to_string() }
fn provider_name(flavor: ApiFlavor) -> String { format!("{flavor:?}") }
fn git_provider(flavor: ApiFlavor) -> GitProvider { match flavor { ApiFlavor::GitHub => GitProvider::GitHub, ApiFlavor::GitLab => GitProvider::GitLab, ApiFlavor::Gitea | ApiFlavor::Forgejo => GitProvider::Gitea, ApiFlavor::Bitbucket => GitProvider::Bitbucket, _ => unreachable!("unsupported focused provider flavor") } }
fn credential(flavor: ApiFlavor) -> (Option<String>, &'static str) { let names: &[&str] = match flavor { ApiFlavor::GitHub => &["GH_TOKEN", "GITHUB_TOKEN"], ApiFlavor::GitLab => &["GITLAB_TOKEN"], ApiFlavor::Gitea | ApiFlavor::Forgejo => &["GITEA_TOKEN", "FORGEJO_TOKEN"], ApiFlavor::Bitbucket => &["BITBUCKET_TOKEN"], _ => &[] }; (names.iter().find_map(|name| std::env::var(name).ok()), names.first().copied().unwrap_or("PROVIDER_TOKEN")) }
fn unsupported(capability: &'static str, flavor: ApiFlavor) -> SniffError { SniffError::UnsupportedRemoteCapability { capability, target: format!("{flavor:?}") } }
fn malformed(message: &str) -> SniffError { SniffError::RemoteApi { provider: "provider".to_string(), status: 200, message: format!("malformed response: {message}") } }
fn transport(url: &url::Url, error: impl std::fmt::Display) -> SniffError { SniffError::RemoteUnreachable { url: url.to_string(), message: error.to_string() } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_policy_allows_only_same_host_or_official_api_mapping() {
        assert!(provider_endpoint_allowed(
            "git.example",
            "git.example",
            ApiFlavor::GitLab
        ));
        assert!(provider_endpoint_allowed(
            "github.com",
            "api.github.com",
            ApiFlavor::GitHub
        ));
        assert!(provider_endpoint_allowed(
            "bitbucket.org",
            "api.bitbucket.org",
            ApiFlavor::Bitbucket
        ));
        assert!(!provider_endpoint_allowed(
            "github.com",
            "evil.example",
            ApiFlavor::GitHub
        ));
        assert!(!provider_endpoint_allowed(
            "git.example",
            "api.github.com",
            ApiFlavor::GitHub
        ));
    }
}
