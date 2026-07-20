//! Focused provider queries that preserve errors and item identity.

use biscuit_file::FetchPolicy;
use serde_json::Value;

use crate::filesystem::git::{ApiFlavor, ResolvedRemote};
use crate::SniffError;

use super::provider_url::{parse_provider_url, ReferenceKind};
use super::types::{at_or_after, at_or_before, optional_timestamp_order, timestamp_order};
use super::web_link::trusted_web_link;
use super::{
    CanonicalPullRequestState, CiCdJob, CiCdJobPage, CiCdJobQuery, CiCdJobReference,
    CiCdParentExecution, GitProvider, ProviderCapabilities, PullRequestInfo, PullRequestPage,
    PullRequestQuery, PullRequestRecord, PullRequestReference,
};

const MAX_PAGES: usize = 20;
const MAX_PARENT_EXECUTIONS: usize = 20;
const MAX_JOBS_INSPECTED: usize = 2_000;

/// Provider page size used for every list traversal.
///
/// Decoupled from the caller's `limit`: exact filters and ordering are emulated
/// over the complete domain, so the walk always wants the provider's maximum
/// page rather than just enough rows to fill one answer.
const PAGE_SIZE: usize = 100;

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
    ///
    /// The base is derived from the remote's configured endpoint origin, so a
    /// self-managed server's scheme and non-default port are preserved. An
    /// `ApiFlavor::Unknown` remote errors here; use [`Self::discover`] to let
    /// the bounded discovery probe identify a neutral-hostname server first.
    pub fn new(remote: ResolvedRemote, policy: FetchPolicy) -> Result<Self, SniffError> {
        let base = canonical_api_base(&remote)?;
        Self::with_api_base(remote, policy, &base)
    }

    /// Creates a focused client, probing an ambiguous self-hosted endpoint.
    ///
    /// Known flavors behave exactly like [`Self::new`] with no network I/O. An
    /// `ApiFlavor::Unknown` remote runs the same allowlisted, bounded discovery
    /// probe `remote_vendor` uses (exact-host consent before any request,
    /// redirect-disabled transport, GitLab/Gitea/Forgejo version endpoints
    /// only) and constructs the client for the detected flavor. This is the
    /// production path for ordinary self-managed servers on neutral hostnames.
    ///
    /// ## Errors
    ///
    /// Propagates the probe's policy, transport, and
    /// [`SniffError::UnsupportedProvider`] failures for hosts that cannot be
    /// identified, and [`Self::new`]'s errors for the constructed client.
    pub async fn discover(remote: ResolvedRemote, policy: FetchPolicy) -> Result<Self, SniffError> {
        if remote.api_flavor != ApiFlavor::Unknown {
            return Self::new(remote, policy);
        }
        let mut remote = remote;
        let fetch_url = remote.fetch_url.clone();
        let probe_policy = policy.clone();
        // The probe is blocking (it drives its own current-thread runtime), so
        // it must leave the async worker before it can block.
        let flavor = tokio::task::spawn_blocking(move || {
            crate::filesystem::git::remote_observation::probe_self_hosted_flavor(
                &fetch_url,
                &probe_policy,
            )
        })
        .await
        .map_err(|error| SniffError::RemoteInit {
            provider: "discovery".to_string(),
            message: error.to_string(),
        })??;
        remote.api_flavor = flavor;
        Self::new(remote, policy)
    }

    /// The resolved remote this client is bound to, including any flavor
    /// detected by [`Self::discover`].
    pub fn remote(&self) -> &ResolvedRemote {
        &self.remote
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
            // `stage` is advertised only where validation accepts it: GitLab is
            // the sole flavor whose job objects carry stage data, so every
            // other flavor rejects the filter instead of matching nothing.
            cicd_job_filters: [
                "statuses", "name", "workflow", "parent", "branch", "commit",
                "actor", "trigger", "created_after", "created_before", "updated_after",
                "updated_before", "direction", "limit",
            ]
            .into_iter()
            .map(str::to_string)
            .chain(matches!(self.remote.api_flavor, ApiFlavor::GitLab).then(|| "stage".to_string()))
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

    /// Queries PRs over a complete bounded domain with exact local filters.
    ///
    /// The provider's own state vocabulary is only ever a widening prefilter, so
    /// the walk must reach provider exhaustion before `pr_matches` and the
    /// requested order can be trusted. `total` is the match count across the
    /// whole domain, which can exceed the returned item count.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::IncompleteRemoteDomain`] when [`MAX_PAGES`] is
    /// reached first, because a partial domain cannot answer the query exactly.
    pub async fn query_pull_requests(
        &self,
        mut query: PullRequestQuery,
    ) -> Result<PullRequestPage, SniffError> {
        validate_pr_query(&query, self.remote.api_flavor)?;
        if query.state.is_none() {
            query.state = Some(super::QueryValues::One(CanonicalPullRequestState::Open));
        }
        let limit = query.limit.unwrap_or(20);
        let mut normalized = Vec::new();
        let mut exhausted = false;
        for page in 1..=MAX_PAGES {
            let path = self.pr_list_path()?;
            let params = self.pr_page_params(&query, page, PAGE_SIZE);
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
                }
            }
            if !next && count < PAGE_SIZE {
                exhausted = true;
                break;
            }
        }
        if !exhausted {
            return Err(incomplete_domain(
                self.remote.api_flavor,
                "pull-request pages",
                MAX_PAGES,
            ));
        }
        sort_prs(&mut normalized, query.sort.as_deref(), query.descending);
        let total = normalized.len();
        normalized.truncate(limit);
        Ok(PullRequestPage {
            total: Some(total),
            next: None,
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
        // Bitbucket's job identity is `parent/step`, so the caller's reference is
        // a more reliable parent than the step body's own `pipeline.uuid`, which
        // the steps endpoint may omit.
        let parent = if self.remote.api_flavor == ApiFlavor::Bitbucket {
            reference
                .native_id
                .split_once('/')
                .map(|(parent, _)| ParentContext::identity_only(parent.to_string()))
        } else {
            None
        };
        let mut job = self.normalize_job(value, parent)?;
        job.reference = reference.clone();
        Ok(Some(job))
    }

    /// Queries jobs through a direct endpoint or bounded parent traversal.
    ///
    /// Both strategies collect the complete bounded domain before ordering and
    /// truncating, so `total` is the domain-wide match count rather than the
    /// number of returned items.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::IncompleteRemoteDomain`] when a page, parent, or
    /// inspection bound stops the walk before provider exhaustion.
    pub async fn query_cicd_jobs(
        &self,
        query: CiCdJobQuery,
    ) -> Result<CiCdJobPage, SniffError> {
        validate_job_query(&query, self.remote.api_flavor)?;
        let limit = query.limit.unwrap_or(20);
        let mut jobs = if self.remote.api_flavor == ApiFlavor::GitLab {
            self.direct_jobs(&query).await?
        } else {
            self.jobs_via_parents(&query).await?
        };
        jobs.sort_by(|left, right| {
            optional_timestamp_order(left.created_at.as_deref(), right.created_at.as_deref())
        });
        if query.descending {
            jobs.reverse();
        }
        let total = jobs.len();
        jobs.truncate(limit);
        Ok(CiCdJobPage {
            total: Some(total),
            next: None,
            items: jobs,
            warnings: Vec::new(),
        })
    }

    async fn direct_jobs(&self, query: &CiCdJobQuery) -> Result<Vec<CiCdJob>, SniffError> {
        let mut jobs = Vec::new();
        let mut inspected = 0;
        let mut exhausted = false;
        for page in 1..=MAX_PAGES {
            let path = format!("projects/{}/jobs", encoded_project(&self.remote));
            let params = pagination_params(self.remote.api_flavor, page, PAGE_SIZE);
            let Some(value) = self.get_json(&path, &params).await? else {
                exhausted = true;
                break;
            };
            let (items, next) = page_items(value);
            let count = items.len();
            for item in items {
                inspected += 1;
                if inspected > MAX_JOBS_INSPECTED {
                    return Err(incomplete_domain(
                        self.remote.api_flavor,
                        "inspected jobs",
                        MAX_JOBS_INSPECTED,
                    ));
                }
                let job = self.normalize_job(item, None)?;
                if job_matches(&job, query) {
                    jobs.push(job);
                }
            }
            if !next && count < PAGE_SIZE {
                exhausted = true;
                break;
            }
        }
        if !exhausted {
            return Err(incomplete_domain(self.remote.api_flavor, "job pages", MAX_PAGES));
        }
        Ok(jobs)
    }

    async fn jobs_via_parents(&self, query: &CiCdJobQuery) -> Result<Vec<CiCdJob>, SniffError> {
        let parent_path = self.parent_list_path()?;
        let mut jobs = Vec::new();
        let mut parents_inspected = 0;
        let mut jobs_inspected = 0;
        let mut parents_exhausted = false;
        for parent_page in 1..=MAX_PAGES {
            let params = pagination_params(
                self.remote.api_flavor,
                parent_page,
                MAX_PARENT_EXECUTIONS,
            );
            let Some(value) = self.get_json(&parent_path, &params).await? else {
                parents_exhausted = true;
                break;
            };
            let (parents, next_parent_page) =
                page_items_named(value, &["workflow_runs", "values"]);
            let parent_count = parents.len();
            for parent in parents {
                parents_inspected += 1;
                if parents_inspected > MAX_PARENT_EXECUTIONS {
                    return Err(incomplete_domain(
                        self.remote.api_flavor,
                        "parent executions",
                        MAX_PARENT_EXECUTIONS,
                    ));
                }
                let parent_id = string_id(&parent, &["id", "uuid"])?;
                let parent_identity = parent_context(
                    &parent,
                    &parent_id,
                    self.remote.api_flavor,
                    self.remote.host.as_deref().unwrap_or_default(),
                );
                let path = self.parent_jobs_path(&parent_id)?;
                let mut jobs_exhausted = false;
                for job_page in 1..=MAX_PAGES {
                    let params = pagination_params(self.remote.api_flavor, job_page, PAGE_SIZE);
                    let Some(value) = self.get_json(&path, &params).await? else {
                        jobs_exhausted = true;
                        break;
                    };
                    let (items, next_job_page) =
                        page_items_named(value, &["jobs", "values"]);
                    let job_count = items.len();
                    for item in items {
                        jobs_inspected += 1;
                        if jobs_inspected > MAX_JOBS_INSPECTED {
                            return Err(incomplete_domain(
                                self.remote.api_flavor,
                                "inspected jobs",
                                MAX_JOBS_INSPECTED,
                            ));
                        }
                        let job = self.normalize_job(item, Some(parent_identity.clone()))?;
                        if job_matches(&job, query) {
                            jobs.push(job);
                        }
                    }
                    if !next_job_page && job_count < PAGE_SIZE {
                        jobs_exhausted = true;
                        break;
                    }
                }
                if !jobs_exhausted {
                    return Err(incomplete_domain(
                        self.remote.api_flavor,
                        "job pages",
                        MAX_PAGES,
                    ));
                }
            }
            if !next_parent_page && parent_count < MAX_PARENT_EXECUTIONS {
                parents_exhausted = true;
                break;
            }
        }
        if !parents_exhausted {
            return Err(incomplete_domain(
                self.remote.api_flavor,
                "parent execution pages",
                MAX_PAGES,
            ));
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
        let mut params = pagination_params(self.remote.api_flavor, page, size);
        let states = query.state.as_ref().map(super::QueryValues::as_slice).unwrap_or_default();
        params.extend(pr_state_params(self.remote.api_flavor, states));
        params
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
        let host = self.remote.host.clone().unwrap_or_default();
        let web_url = trusted_web_link(
            nested_string(&value, &[&["links", "html", "href"]])
                .or_else(|| value_string(&value, &["html_url", "web_url"])),
            &host,
        );
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
            html_url: web_url.clone().unwrap_or_default(),
        };
        Ok(PullRequestRecord {
            identity: PullRequestReference {
                provider: git_provider(self.remote.api_flavor),
                api_flavor: format!("{:?}", self.remote.api_flavor),
                host,
                namespace: self.remote.namespace.clone().unwrap_or_default(),
                repository: self.remote.repository.clone().unwrap_or_default(),
                native_id: number.to_string(), display_id: format!("#{number}"), number: Some(number),
                web_url,
                api_url: value_string(&value, &["url"]),
                original_url: self.original_reference.clone(),
            },
            details,
        })
    }

    /// Projects one provider's job object onto the structured record.
    ///
    /// Dispatch is on the resolved flavor rather than one union of key probes
    /// because the same key means different things per provider: Bitbucket's
    /// `state` is an object where GitLab's is absent, GitLab's `ref` is the
    /// branch where GitHub has none, and GitLab's top-level `sha` is usually
    /// absent while its `commit.id` is not. A shared probe order silently
    /// resolves each field to whichever provider spells it first, which is how
    /// the fields this record promises went missing.
    ///
    /// `parent` carries the workflow-run/pipeline metadata that only the parent
    /// object holds. It fills a field solely when the job itself has none, so an
    /// inherited value is never invented and never overwrites a job-level one.
    fn normalize_job(&self, value: Value, parent: Option<ParentContext>) -> Result<CiCdJob, SniffError> {
        let projected = match self.remote.api_flavor {
            ApiFlavor::GitLab => project_gitlab_job(&value)?,
            ApiFlavor::Bitbucket => project_bitbucket_job(&value)?,
            _ => project_actions_job(&value)?,
        };
        let parent = parent.unwrap_or_else(|| {
            ParentContext::identity_only(projected.parent_id.clone().unwrap_or_default())
        });
        let host = self.remote.host.clone().unwrap_or_default();
        let web_url = trusted_web_link(projected.web_url, &host);
        Ok(CiCdJob {
            reference: CiCdJobReference {
                provider: git_provider(self.remote.api_flavor), api_flavor: format!("{:?}", self.remote.api_flavor),
                host, namespace: self.remote.namespace.clone().unwrap_or_default(),
                repository: self.remote.repository.clone().unwrap_or_default(),
                native_id: projected.id.clone(), display_id: projected.id,
                original_url: web_url.clone(),
            },
            parent: parent.identity,
            name: projected.name.unwrap_or_else(|| "unnamed job".to_string()),
            stage: projected.stage,
            normalized_status: normalize_status(&projected.normalized_source),
            native_status: projected.native_status,
            conclusion: projected.conclusion,
            branch: projected.branch.or(parent.branch),
            commit: projected.commit.or(parent.commit),
            actor: projected.actor.or(parent.actor),
            trigger: projected.trigger.or(parent.trigger),
            created_at: projected.created_at,
            started_at: projected.started_at,
            finished_at: projected.finished_at.clone(),
            updated_at: projected.updated_at.or(projected.finished_at),
            web_url,
            api_url: projected.api_url,
            runner: projected.runner,
        })
    }
}

/// One provider's job object flattened onto the fields the record promises.
///
/// `normalized_source` is separate from `native_status` because the token that
/// answers "did this job succeed" is not always the one the provider calls its
/// status: a GitHub job's `status` is `completed` whether it passed or failed,
/// and Bitbucket buries the verdict in `state.result.name`.
#[derive(Debug, Default)]
struct JobProjection {
    id: String,
    name: Option<String>,
    stage: Option<String>,
    native_status: String,
    normalized_source: String,
    conclusion: Option<String>,
    branch: Option<String>,
    commit: Option<String>,
    actor: Option<String>,
    trigger: Option<String>,
    created_at: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    updated_at: Option<String>,
    web_url: Option<String>,
    api_url: Option<String>,
    runner: Option<String>,
    parent_id: Option<String>,
}

/// Parent-run metadata plus the identity the record exposes.
///
/// The extra fields are deliberately not on the public [`CiCdParentExecution`]:
/// a job that inherits its run's branch reports it as *the job's* branch, so
/// widening the identity type would publish a second, redundant copy of the
/// same facts and force every consumer to decide which one wins.
#[derive(Debug, Clone)]
struct ParentContext {
    identity: CiCdParentExecution,
    branch: Option<String>,
    commit: Option<String>,
    actor: Option<String>,
    trigger: Option<String>,
}

impl ParentContext {
    fn identity_only(id: String) -> Self {
        Self {
            identity: CiCdParentExecution {
                native_id: id.clone(),
                display_id: id,
                name: None,
                web_url: None,
                definition_id: None,
                definition_path: None,
            },
            branch: None,
            commit: None,
            actor: None,
            trigger: None,
        }
    }
}

/// GitHub Actions job, shared verbatim by Gitea and Forgejo.
///
/// Runner identity is two flat keys (`runner_name`, `runner_group_name`), never
/// a nested `runner` object, and the run-level `event`/actor are absent here —
/// they arrive through [`ParentContext`].
fn project_actions_job(value: &Value) -> Result<JobProjection, SniffError> {
    let native_status = value_string(value, &["status"]).unwrap_or_else(|| "unknown".to_string());
    let conclusion = value_string(value, &["conclusion"]);
    let completed_at = value_string(value, &["completed_at"]);
    Ok(JobProjection {
        id: string_id(value, &["id"])?,
        name: value_string(value, &["name"]),
        normalized_source: match (native_status.as_str(), conclusion.as_deref()) {
            ("completed", Some(conclusion)) => conclusion.to_string(),
            _ => native_status.clone(),
        },
        native_status,
        conclusion,
        branch: value_string(value, &["head_branch"]),
        commit: value_string(value, &["head_sha"]),
        created_at: value_string(value, &["created_at"]),
        started_at: value_string(value, &["started_at"]),
        finished_at: completed_at,
        updated_at: value_string(value, &["updated_at"]),
        web_url: value_string(value, &["html_url"]),
        api_url: value_string(value, &["url"]),
        runner: nonempty(value_string(value, &["runner_name"]))
            .or_else(|| nonempty(value_string(value, &["runner_group_name"]))),
        parent_id: value_id(value, &["run_id"]),
        ..JobProjection::default()
    })
}

/// GitLab job.
///
/// The commit is nested under `commit.id`; a top-level `sha` is usually absent,
/// which is why the branch and commit fall back to the embedded `pipeline`
/// summary rather than to a flat key. Self-hosted runners routinely register
/// with an empty `name` and a meaningful `description`, so an empty name must
/// not shadow the description.
fn project_gitlab_job(value: &Value) -> Result<JobProjection, SniffError> {
    let status = value_string(value, &["status"]).unwrap_or_else(|| "unknown".to_string());
    Ok(JobProjection {
        id: string_id(value, &["id"])?,
        name: value_string(value, &["name"]),
        stage: value_string(value, &["stage"]),
        normalized_source: status.clone(),
        native_status: status,
        branch: value_string(value, &["ref"])
            .or_else(|| nested_string(value, &[&["pipeline", "ref"]])),
        commit: nested_string(value, &[&["commit", "id"], &["pipeline", "sha"]])
            .or_else(|| value_string(value, &["sha"])),
        actor: nested_string(value, &[&["user", "username"], &["user", "name"]]),
        trigger: nested_string(value, &[&["pipeline", "source"]]),
        created_at: value_string(value, &["created_at"]),
        started_at: value_string(value, &["started_at"]),
        finished_at: value_string(value, &["finished_at"]),
        web_url: value_string(value, &["web_url"]),
        runner: nonempty(nested_string(value, &[&["runner", "name"]]))
            .or_else(|| nonempty(nested_string(value, &[&["runner", "description"]]))),
        parent_id: nested_id(value, &[&["pipeline", "id"]])
            .or_else(|| value_id(value, &["pipeline_id"])),
        ..JobProjection::default()
    })
}

/// Bitbucket Pipelines step.
///
/// A step's lifecycle is an object, not a string: `state.name` says how far it
/// got and `state.result.name` says how it ended, so a step that is `COMPLETED`
/// and `FAILED` must normalize from the result. Branch, commit, trigger, and
/// actor live only on the parent pipeline.
fn project_bitbucket_job(value: &Value) -> Result<JobProjection, SniffError> {
    let native_status = nested_string(value, &[&["state", "name"]])
        .unwrap_or_else(|| "unknown".to_string());
    let result = nested_string(value, &[&["state", "result", "name"]]);
    let completed_on = value_string(value, &["completed_on"]);
    Ok(JobProjection {
        id: string_id(value, &["uuid"])?,
        name: value_string(value, &["name"]),
        normalized_source: result.clone().unwrap_or_else(|| native_status.clone()),
        native_status,
        conclusion: result,
        created_at: value_string(value, &["created_on"]),
        started_at: value_string(value, &["started_on"]),
        finished_at: completed_on,
        web_url: nested_string(value, &[&["links", "html", "href"]]),
        api_url: nested_string(value, &[&["links", "self", "href"]]),
        runner: nonempty(nested_string(value, &[&["runner", "name"]])),
        parent_id: nested_string(value, &[&["pipeline", "uuid"]]),
        ..JobProjection::default()
    })
}

/// Derives the API base from the remote's configured endpoint origin.
///
/// Self-managed servers keep the scheme and non-default port their remote URL
/// was configured with; SSH-configured remotes fall back to the provider's
/// canonical HTTPS assumption because an SSH port is not an API port.
fn canonical_api_base(remote: &ResolvedRemote) -> Result<String, SniffError> {
    let host = remote.host.as_deref().unwrap_or_default();
    let origin = remote
        .http_origin()
        .unwrap_or_else(|| format!("https://{host}"));
    Ok(match remote.api_flavor {
        ApiFlavor::GitHub if host == "github.com" => "https://api.github.com/".to_string(),
        ApiFlavor::GitHub => format!("{origin}/api/v3/"),
        ApiFlavor::GitLab => format!("{origin}/api/v4/"),
        ApiFlavor::Gitea | ApiFlavor::Forgejo => format!("{origin}/api/v1/"),
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
    query.validate_canonical()?;
    if query.cursor.is_some() {
        return Err(SniffError::InvalidRemoteQuery {
            field: "cursor",
            message: "not part of the canonical query vocabulary; focused queries paginate internally".to_string(),
        });
    }
    for (field, present) in [("assignee", query.assignee.is_some()), ("reviewer", query.reviewer.is_some()), ("milestone", query.milestone.is_some()), ("commit", query.commit.is_some())] {
        if present { return Err(SniffError::UnsupportedRemoteFilter { field, provider: format!("{flavor:?}") }); }
    }
    Ok(())
}

fn validate_job_query(query: &CiCdJobQuery, flavor: ApiFlavor) -> Result<(), SniffError> {
    query.validate_canonical()?;
    if query.cursor.is_some() {
        return Err(SniffError::InvalidRemoteQuery {
            field: "cursor",
            message: "not part of the canonical query vocabulary; focused queries paginate internally".to_string(),
        });
    }
    // Only GitLab job objects carry stage data; matching `stage` anywhere else
    // would approximate the filter as "no matches" instead of refusing it.
    if query.stage.is_some() && flavor != ApiFlavor::GitLab {
        return Err(SniffError::UnsupportedRemoteFilter {
            field: "stage",
            provider: format!("{flavor:?}"),
        });
    }
    Ok(())
}

fn positive_id(id: &str, field: &'static str) -> Result<(), SniffError> {
    if id.is_empty() || id == "0" { Err(SniffError::InvalidRemoteQuery { field, message: "must be a positive provider identifier".to_string() }) } else { Ok(()) }
}

/// Projects the canonical state set onto one provider's list-endpoint vocabulary.
///
/// Every provider rejects tokens outside its own vocabulary, and no provider
/// exposes all three canonical states as distinct list filters, so a canonical
/// state without a wire equivalent widens to the broadest token the provider
/// does accept. `pr_matches` then narrows the widened response back to the exact
/// canonical request, which is why widening never loses precision.
///
/// ## Notes
///
/// Canonical `merged` has no GitHub/Gitea/Forgejo list token; those services
/// surface merged PRs under `closed`. Bitbucket has no "any state" token at
/// all, so a mixed request repeats `state` instead of inventing one.
fn pr_state_params(
    flavor: ApiFlavor,
    states: &[CanonicalPullRequestState],
) -> Vec<(String, String)> {
    if states.is_empty() {
        return Vec::new();
    }
    if flavor == ApiFlavor::Bitbucket {
        let mut tokens: Vec<&'static str> = Vec::new();
        for state in states {
            let expansion: &[&'static str] = match state {
                CanonicalPullRequestState::Open => &["OPEN"],
                CanonicalPullRequestState::Merged => &["MERGED"],
                CanonicalPullRequestState::Closed => &["DECLINED", "SUPERSEDED"],
            };
            for token in expansion {
                if !tokens.contains(token) {
                    tokens.push(token);
                }
            }
        }
        return tokens
            .into_iter()
            .map(|token| ("state".to_string(), token.to_string()))
            .collect();
    }
    let mut tokens: Vec<&'static str> = Vec::new();
    for state in states {
        let token = match (flavor, state) {
            (ApiFlavor::GitLab, CanonicalPullRequestState::Open) => "opened",
            (ApiFlavor::GitLab, CanonicalPullRequestState::Closed) => "closed",
            (ApiFlavor::GitLab, CanonicalPullRequestState::Merged) => "merged",
            (_, CanonicalPullRequestState::Open) => "open",
            (_, CanonicalPullRequestState::Closed | CanonicalPullRequestState::Merged) => "closed",
        };
        if !tokens.contains(&token) {
            tokens.push(token);
        }
    }
    let token = if tokens.len() == 1 { tokens[0] } else { "all" };
    vec![("state".to_string(), token.to_string())]
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
        CanonicalPullRequestState::Open => item.state == "open" || item.state == "opened",
        CanonicalPullRequestState::Closed => matches!(item.state.as_str(), "closed" | "declined" | "superseded") && item.merged_at.is_none(),
        CanonicalPullRequestState::Merged => item.state == "merged" || item.merged_at.is_some(),
    }))
        && query.source_branch.as_ref().is_none_or(|v| item.source_branch.as_ref() == Some(v))
        && query.target_branch.as_ref().is_none_or(|v| item.target_branch.as_ref() == Some(v))
        && query.author.as_ref().is_none_or(|v| item.author.eq_ignore_ascii_case(v))
        && query.draft.is_none_or(|v| item.draft == v)
        && query.labels.iter().all(|v| item.labels.iter().any(|a| a.eq_ignore_ascii_case(v)))
        && query.search.as_ref().is_none_or(|v| { let v = v.to_ascii_lowercase(); item.title.to_ascii_lowercase().contains(&v) || item.body.as_ref().is_some_and(|b| b.to_ascii_lowercase().contains(&v)) })
        && query.created_after.as_deref().is_none_or(|v| at_or_after(&item.created_at, v))
        && query.created_before.as_deref().is_none_or(|v| at_or_before(&item.created_at, v))
        && query.updated_after.as_deref().is_none_or(|v| item.updated_at.as_deref().is_some_and(|a| at_or_after(a, v)))
        && query.updated_before.as_deref().is_none_or(|v| item.updated_at.as_deref().is_some_and(|a| at_or_before(a, v)))
}


/// Orders the complete domain, newest-first unless the caller says otherwise.
///
/// An absent `sort` is not the same as an explicit `provider-default`: the
/// ratified default is newest-first, which needs a real ordering key, whereas
/// `provider-default` is an author asking to keep whatever order the provider
/// returned. Collapsing the two would leave `pr_list({})` at the mercy of each
/// provider's own default ordering. A provider-default result is never
/// reversed — `descending` only has meaning relative to a sort key, so
/// provider order is preserved verbatim whichever way the flag points.
fn sort_prs(items: &mut [PullRequestRecord], sort: Option<&str>, descending: bool) {
    match sort {
        None | Some("created") => items.sort_by(|a, b| timestamp_order(&a.details.created_at, &b.details.created_at)),
        Some("updated") => items.sort_by(|a, b| optional_timestamp_order(a.details.updated_at.as_deref(), b.details.updated_at.as_deref())),
        _ => return,
    }
    if descending { items.reverse(); }
}

fn job_matches(job: &CiCdJob, query: &CiCdJobQuery) -> bool {
    query.statuses.as_ref().is_none_or(|statuses| statuses.as_slice().iter().any(|s| s.eq_ignore_ascii_case(&job.normalized_status)))
        && query.name.as_ref().is_none_or(|v| &job.name == v)
        && query.stage.as_ref().is_none_or(|v| job.stage.as_ref() == Some(v))
        && query.workflow.as_ref().is_none_or(|v| {
            job.parent.name.as_ref() == Some(v)
                || &job.parent.native_id == v
                || job.parent.definition_id.as_ref() == Some(v)
                || job.parent.definition_path.as_ref() == Some(v)
        })
        && query.parent.as_ref().is_none_or(|v| &job.parent.native_id == v)
        && query.branch.as_ref().is_none_or(|v| job.branch.as_ref() == Some(v))
        && query.commit.as_ref().is_none_or(|v| job.commit.as_ref() == Some(v))
        && query.actor.as_ref().is_none_or(|v| job.actor.as_ref() == Some(v))
        && query.trigger.as_ref().is_none_or(|v| job.trigger.as_ref() == Some(v))
        && query.created_after.as_deref().is_none_or(|v| job.created_at.as_deref().is_some_and(|a| at_or_after(a, v)))
        && query.created_before.as_deref().is_none_or(|v| job.created_at.as_deref().is_some_and(|a| at_or_before(a, v)))
        && query.updated_after.as_deref().is_none_or(|v| job.updated_at.as_deref().is_some_and(|a| at_or_after(a, v)))
        && query.updated_before.as_deref().is_none_or(|v| job.updated_at.as_deref().is_some_and(|a| at_or_before(a, v)))
}

/// Captures the run/pipeline metadata its jobs cannot see for themselves.
///
/// A GitHub job object carries no `event` and no actor, and a Bitbucket step
/// carries no branch or commit at all; both live on the parent the traversal
/// already holds. Reading them here is the only point at which they are still
/// in scope.
fn parent_context(value: &Value, id: &str, flavor: ApiFlavor, host: &str) -> ParentContext {
    let identity = CiCdParentExecution {
        native_id: id.to_string(),
        display_id: id.to_string(),
        name: value_string(value, &["name"]),
        web_url: trusted_web_link(
            nested_string(value, &[&["links", "html", "href"]])
                .or_else(|| value_string(value, &["html_url", "web_url"])),
            host,
        ),
        // Actions-family runs carry their workflow definition (`workflow_id`,
        // `path`); Bitbucket pipelines have neither, so the probes miss there.
        definition_id: value_id(value, &["workflow_id"]),
        definition_path: value_string(value, &["path"]),
    };
    if flavor == ApiFlavor::Bitbucket {
        return ParentContext {
            identity,
            branch: nested_string(value, &[&["target", "ref_name"]]),
            commit: nested_string(value, &[&["target", "commit", "hash"]]),
            actor: nested_string(value, &[&["creator", "nickname"], &["creator", "display_name"]]),
            trigger: nested_string(value, &[&["trigger", "name"]]),
        };
    }
    ParentContext {
        identity,
        branch: value_string(value, &["head_branch"]),
        commit: value_string(value, &["head_sha"]),
        actor: nested_string(
            value,
            &[&["triggering_actor", "login"], &["actor", "login"]],
        ),
        trigger: value_string(value, &["event"]),
    }
}

fn repo_path(remote: &ResolvedRemote) -> String { format!("{}/{}", remote.namespace.as_deref().unwrap_or_default(), remote.repository.as_deref().unwrap_or_default()) }
fn encoded_project(remote: &ResolvedRemote) -> String { urlencoding::encode(&repo_path(remote)).into_owned() }
fn value_string(value: &Value, names: &[&str]) -> Option<String> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str()).map(str::to_string)) }
fn value_bool(value: &Value, names: &[&str]) -> Option<bool> { names.iter().find_map(|name| value.get(*name).and_then(Value::as_bool)) }
fn value_u64(value: &Value, names: &[&str]) -> Result<u64, SniffError> { names.iter().find_map(|name| value.get(*name).and_then(Value::as_u64)).ok_or_else(|| malformed("missing numeric identity")) }
fn value_id(value: &Value, names: &[&str]) -> Option<String> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|id| id.to_string())))) }
fn string_id(value: &Value, names: &[&str]) -> Result<String, SniffError> { names.iter().find_map(|name| value.get(*name).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|id| id.to_string())))).ok_or_else(|| malformed("missing job identity")) }
fn nested_string(value: &Value, paths: &[&[&str]]) -> Option<String> { paths.iter().find_map(|path| path.iter().try_fold(value, |v, key| v.get(*key)).and_then(Value::as_str).map(str::to_string)) }
/// Nested identity that may arrive as a JSON number (GitLab) or string (Bitbucket).
fn nested_id(value: &Value, paths: &[&[&str]]) -> Option<String> { paths.iter().find_map(|path| path.iter().try_fold(value, |v, key| v.get(*key)).and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_u64().map(|id| id.to_string())))) }
fn nonempty(value: Option<String>) -> Option<String> { value.filter(|value| !value.trim().is_empty()) }
/// Folds one provider's lifecycle token onto [`super::CICD_JOB_STATUSES`].
///
/// `completed` still maps to `success` for providers whose terminal state has no
/// separate verdict; GitHub and Bitbucket resolve their verdict before calling
/// this, so a completed-but-failed job never reaches that arm.
fn normalize_status(status: &str) -> String { match status.to_ascii_lowercase().as_str() { "success" | "successful" | "completed" => "success", "failure" | "failed" | "error" => "failed", "cancelled" | "canceled" | "stopped" | "expired" => "cancelled", "queued" | "pending" | "created" | "ready" => "queued", "running" | "in_progress" => "running", "paused" | "halted" => "manual", "not_run" => "skipped", other => other }.to_string() }
fn provider_name(flavor: ApiFlavor) -> String { format!("{flavor:?}") }
fn git_provider(flavor: ApiFlavor) -> GitProvider { match flavor { ApiFlavor::GitHub => GitProvider::GitHub, ApiFlavor::GitLab => GitProvider::GitLab, ApiFlavor::Gitea | ApiFlavor::Forgejo => GitProvider::Gitea, ApiFlavor::Bitbucket => GitProvider::Bitbucket, _ => unreachable!("unsupported focused provider flavor") } }
fn credential(flavor: ApiFlavor) -> (Option<String>, &'static str) { let names: &[&str] = match flavor { ApiFlavor::GitHub => &["GH_TOKEN", "GITHUB_TOKEN"], ApiFlavor::GitLab => &["GITLAB_TOKEN"], ApiFlavor::Gitea | ApiFlavor::Forgejo => &["GITEA_TOKEN", "FORGEJO_TOKEN"], ApiFlavor::Bitbucket => &["BITBUCKET_TOKEN"], _ => &[] }; (names.iter().find_map(|name| std::env::var(name).ok()), names.first().copied().unwrap_or("PROVIDER_TOKEN")) }
fn unsupported(capability: &'static str, flavor: ApiFlavor) -> SniffError { SniffError::UnsupportedRemoteCapability { capability, target: format!("{flavor:?}") } }
fn incomplete_domain(flavor: ApiFlavor, bound: &'static str, limit: usize) -> SniffError { SniffError::IncompleteRemoteDomain { provider: provider_name(flavor), bound, limit } }
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
