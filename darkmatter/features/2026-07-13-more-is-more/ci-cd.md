---
prompt: |-
  - use the 'sniff' skill

  In the sniff library we need to be able get a CICD execution or a set of CICD execution's based a query syntax provided by the various git providers:

  - Github
  - GitLab
  - Gittea
  - bitbucket
  - etc.

  We have two general query types we'd like to be able expose:

  1. pass in a uniquely identifying representation for a CICD execution and get back a single CICD execution (if there's a match)
  2. pass in a "query" and get back a list of CICD execution's

  Your task is to start by looking at what the sniff library currently supports and documenting how well you think the current
  sniff implementation would do in meeting our goals.

  After that, research all the git providers that sniff supports and determine if there are potentially useful features or query capabilities that are available but not currently available via Sniff.

  - write down the list of features that are currently unavailable or less ergonomically available then they could be by sniff
  - for each feature, describe how this feature is reached via API
  - try to group all similar features across providers into the same section
last_updated: 2026-07-14
hash: b7ab11d96c318581-0b8dd322cb51d6de
---
# CI/CD Execution Query Research

## Scope

This assessment covers the remote providers represented by Sniff’s `GitProvider` enum:

- GitHub
- GitLab, including self-hosted GitLab
- Gitea, with Forgejo treated as a related but potentially divergent implementation
- Bitbucket Cloud

Sniff’s Bitbucket implementation uses Bitbucket Cloud API shapes and URLs. Bitbucket Server/Data Center should not be assumed to have CI/CD parity with it.

The research distinguishes:

- **CI/CD configuration**: evidence that a repository is configured for a CI/CD system.
- **CI/CD execution**: a concrete workflow run or pipeline with an identity, state, timestamps, trigger, commit, and related jobs.

Those concepts are currently combined in Sniff’s `CiCdInfo`, but they need different models for the proposed lookup and query APIs.

## Executive Assessment

The current implementation does not meet the single-execution lookup goal and meets only a narrow, GitHub-only portion of the list-query goal.

| Capability                                             | GitHub                 | GitLab | Gitea/Forgejo | Bitbucket |
|--------------------------------------------------------|-----------------------:|-------:|--------------:|----------:|
| Detect configuration                                   | Yes                    | Yes    | Partial       | Yes       |
| List actual executions                                 | Latest first page only | No     | No            | No        |
| Filter executions                                      | No                     | No     | No            | No        |
| Retrieve one execution by ID                           | No                     | No     | No            | No        |
| Preserve provider execution ID                         | No                     | No     | No            | No        |
| Paginate through executions                            | No                     | No     | No            | No        |
| Retrieve jobs, steps, logs, artifacts, or test results | No                     | No     | No            | No        |
| Dedicated library query model                          | No                     | No     | No            | No        |
| Dedicated CLI command                                  | No                     | No     | No            | No        |

Sniff is therefore not yet able to represent either requested operation reliably:

1. A single execution cannot be requested because `CiCdInfo` has no provider-native ID and `RemoteRepoProvider` has no single-execution method.
2. A query cannot be expressed because `list_workflow_runs(owner, repo, limit)` accepts only a limit. GitLab, Gitea, and Bitbucket use the trait’s default empty implementation.

## Current Sniff Implementation

### Public surface

`RemoteRepoProvider` currently exposes two CI/CD methods:

- `detect_cicd(owner, repo) -> Option<CiCdInfo>`
- `list_workflow_runs(owner, repo, limit) -> Vec<CiCdInfo>`

`GitRemote` delegates both methods to the selected provider.

`fetch_report()` requests five executions. If listing fails or returns an empty list, it falls back to configuration detection. Errors from these optional requests are deliberately swallowed.

The CLI exposes this information only as part of the remote repository report. It has no `sniff repo ci`, `ci-cd`, or equivalent focused command. The renderer decides whether a `CiCdInfo` is an execution by checking whether `started_at` is present.

### Provider behavior

#### GitHub

GitHub is the only provider that implements `list_workflow_runs`.

It calls:

```text
GET /repos/{owner}/{repo}/actions/runs?per_page={limit}
```

It then maps the first response page into `CiCdInfo`.

Although the generated Schematic request already supports `page`, `status`, `branch`, and `event`, Sniff exposes none of them. The current implementation sets only `per_page`.

The mapper also discards important fields already present in Schematic’s `WorkflowRun`:

- execution ID
- workflow ID
- run number
- attempt number
- head commit SHA
- actor and triggering actor
- API URL
- updated timestamp

It also places GitHub’s `created_at` value into `CiCdInfo.started_at`, even though GitHub exposes a distinct `run_started_at` value.

#### GitLab

GitLab only checks the default-branch repository tree for `.gitlab-ci.yml`. It does not retrieve pipelines.

This is especially notable because Schematic already defines `ListProjectPipelinesRequest` and a `Pipeline` response type with initial support for:

- page and page size
- status
- source
- ref
- commit SHA

The missing functionality is therefore primarily in Sniff’s provider integration, although Schematic’s endpoint definition is less complete than GitLab’s current API.

#### Gitea and Forgejo

Sniff detects:

- `.gitea/workflows/`
- `.drone.yml`
- `.woodpecker.yml`
- `.woodpecker/`

It returns only the first detected system and does not retrieve executions from any of them.

Gitea’s default `WORKFLOW_DIRS` includes both `.gitea/workflows` and `.github/workflows`, so Sniff can miss valid Gitea Actions configuration. The directory list is also instance-configurable. [Gitea configuration documents the default workflow directories](https://docs.gitea.com/administration/config-cheat-sheet).

Forgejo normally uses `.forgejo/workflows`, with `.github/workflows` as a fallback. Sniff checks neither path and can therefore identify a Forgejo repository correctly while missing its native CI configuration. [Forgejo documents this workflow-directory behavior](https://forgejo.org/docs/latest/user/actions/overview/).

Gitea and Forgejo API capabilities are version-sensitive and should not be treated as interchangeable merely because they share a provider variant.

#### Bitbucket

Bitbucket only checks for `bitbucket-pipelines.yml` in the default branch. It does not query Bitbucket Pipelines.

The implementation does not verify whether Pipelines is enabled, nor does it distinguish file presence from a usable Pipelines installation.

### Data-model limitations

`CiCdInfo` contains:

- provider
- configuration path
- name
- status
- conclusion
- HTML URL
- start timestamp
- branch
- event

It does not contain enough information to identify or fully describe an execution:

- provider-native execution ID
- repository or provider host
- attempt number
- workflow or pipeline definition ID
- commit SHA
- actor
- creation, start, completion, and update timestamps
- duration and queue duration
- jobs or steps
- pagination metadata
- raw provider status
- capability or provenance information

More importantly, the same type represents both:

- a configuration observation such as `status = "detected"`, and
- an actual execution such as `status = "completed"`.

That makes an empty execution list, an unsupported provider, an authorization failure, and a configuration-only result difficult to distinguish.

## Exact Execution Lookup

A provider-neutral reference must include the provider host and repository identity. A bare numeric ID is not globally meaningful.

A robust reference should accept either:

- a structured reference containing provider, host, repository, native ID, and optional attempt; or
- a provider web/API URL that can be parsed into that structure.

### Provider APIs

| Provider        | Canonical identity                                              | API access                                                                                                                                         |
|-----------------|-----------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub          | host + owner/repository + `run_id`; optionally `attempt_number` | `GET /repos/{owner}/{repo}/actions/runs/{run_id}`; historical attempt: `GET /repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt_number}` |
| GitLab          | host + project + pipeline `id`                                  | `GET /projects/{id-or-encoded-path}/pipelines/{pipeline_id}`                                                                                       |
| Gitea 1.25+     | host + owner/repository + run ID                                | `GET /repos/{owner}/{repo}/actions/runs/{run}`                                                                                                     |
| Bitbucket Cloud | workspace/repository + pipeline UUID                            | `GET /repositories/{workspace}/{repo_slug}/pipelines/{pipeline_uuid}`                                                                              |

GitHub’s exact-run endpoint explicitly identifies `run_id` as the workflow run’s unique identifier. [GitHub workflow-run API](https://docs.github.com/en/rest/actions/workflow-runs).

GitLab exposes both `id` and `iid`, but the retrieval route accepts `pipeline_id`; Sniff should preserve the canonical `id` used by the API rather than assuming the display-oriented `iid` is accepted. [GitLab Pipelines API](https://docs.gitlab.com/api/pipelines/).

Bitbucket exposes both a pipeline UUID and a human-facing build number. The UUID is the exact lookup key. [Bitbucket Pipelines API](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pipelines/).

Gitea 1.25 added repository-scoped exact lookup, jobs, and artifacts around a run ID. Its implementation remains version-dependent. [Gitea 1.25 repository Actions API source](https://github.com/go-gitea/gitea/blob/release/v1.25/routers/api/v1/repo/action.go).

### Currently unavailable through Sniff

- Exact lookup for every provider
- Parsing execution URLs into references
- GitHub attempt-specific lookup
- Preservation of GitLab pipeline IDs and IIDs
- Preservation of Bitbucket pipeline UUIDs and build numbers
- Provider-version checks before using Gitea endpoints
- Correct `Option` semantics where only a genuine not-found response becomes `None`; authentication and transport failures should remain errors

## Execution List Queries

A common query can cover many useful dimensions:

- status or conclusion
- workflow or pipeline definition
- branch or tag
- commit SHA
- trigger or event
- actor
- creation or update time range
- latest-only behavior
- limit and pagination
- sorting

Provider-specific extensions are still necessary because the native query models are not equivalent.

### Status, ref, commit, trigger, and actor filters

| Provider  | API access and supported filters                                                                                                                                     |
|-----------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | `GET /repos/{owner}/{repo}/actions/runs` with `actor`, `branch`, `event`, `status`, `created`, `check_suite_id`, and `head_sha`                                      |
| GitLab    | `GET /projects/{id}/pipelines` with `status`, `source`, `ref`, `sha`, `username`, creation/update ranges, and other filters                                          |
| Gitea     | Current admin-wide listing supports `event`, `branch`, repeatable `status`, `actor`, `head_sha`, `page`, and `limit`; normal repository listing is version-dependent |
| Bitbucket | `GET /repositories/{workspace}/{repo_slug}/pipelines` with creator, ref, commit, selector, creation date, trigger type, status, sorting, and pagination parameters   |

GitHub’s status parameter accepts both active statuses and completed conclusions, including `queued`, `in_progress`, `success`, `failure`, `cancelled`, `skipped`, and `timed_out`. Its `created` parameter uses GitHub’s date-range search syntax. Filtered searches are capped at 1,000 results. [GitHub workflow-run filters](https://docs.github.com/en/rest/actions/workflow-runs).

GitLab supports:

- `name`
- `order_by`
- `ref`
- `scope`
- `sha`
- `sort`
- `source`
- `status`
- `updated_after` and `updated_before`
- `created_after` and `created_before`
- `username`
- `yaml_errors`

Setting `source=parent_pipeline` includes child pipelines, which are excluded by default. [GitLab pipeline filters](https://docs.gitlab.com/api/pipelines/).

Gitea 1.25’s administrator endpoint is:

```text
GET /admin/actions/runs
```

It supports `event`, `branch`, `status`, `actor`, `head_sha`, `page`, and `limit`, but it is instance-wide and requires administrator authority. It is not an appropriate default mechanism for querying one ordinary repository. [Gitea administrator Actions API source](https://github.com/go-gitea/gitea/blob/release/v1.25/routers/api/v1/admin/action.go).

Bitbucket’s pipeline endpoint uses explicit query parameters:

- `creator.uuid`
- `target.ref_type`
- `target.ref_name`
- `target.branch`
- `target.commit.hash`
- `target.selector.pattern`
- `target.selector.type`
- `created_on`
- `trigger_type`
- `status`

Trigger types include `PUSH`, `MANUAL`, `SCHEDULED`, and `PARENT_STEP`. Selector types include branch, tag, custom, pull-request, and default pipelines. [Bitbucket pipeline query parameters](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pipelines/).

Bitbucket also has a generic query language for many paginated resources, with comparison, containment, list, logical, nested-field, and sorting operations. The Pipelines endpoint currently documents explicit pipeline-specific parameters, so Sniff should model those rather than assume arbitrary BBQL expressions are accepted there. An opaque provider-query escape hatch could still be retained for future compatibility. [Bitbucket filtering and sorting syntax](https://developer.atlassian.com/cloud/bitbucket/rest/intro/).

### Workflow or definition scoping

| Provider  | API access                                                                                                                                                                          |
|-----------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub    | `GET /repos/{owner}/{repo}/actions/workflows/{workflow_id-or-file}/runs`                                                                                                            |
| GitLab    | Use the pipeline `name`, `source`, ref, and hierarchy filters; GitLab does not use a directly equivalent workflow-file route                                                        |
| Gitea     | Repository workflow-definition endpoints exist; repository run listing by workflow remains version-sensitive. The web UI has a workflow filter, but it is not a stable API contract |
| Bitbucket | Use `target.selector.type` and `target.selector.pattern`                                                                                                                            |

GitHub accepts either a workflow ID or workflow filename, making queries such as “latest failed run of `ci.yml` on `main`” directly expressible. [GitHub workflow-scoped run listing](https://docs.github.com/en/rest/actions/workflow-runs).

### Currently unavailable through Sniff

- Every filter listed above
- Workflow-specific listing
- Commit-SHA lookup
- Actor filtering
- Time-window queries
- GitLab child-pipeline inclusion
- Bitbucket custom-pipeline selector queries
- Provider-specific raw query parameters
- Query validation against provider capabilities or server versions

## Pagination, Sorting, and Latest-Execution Queries

### Provider APIs

| Provider  | Pagination and ordering                                                                             |
|-----------|-----------------------------------------------------------------------------------------------------|
| GitHub    | `page` and `per_page`, maximum 100 per page; results are newest-first with no general sort selector |
| GitLab    | `page` and `per_page`, plus `order_by` and `sort`                                                   |
| Gitea     | `page` and `limit`; maximums are instance-configurable                                              |
| Bitbucket | `page`, `pagelen`, and response `next` links; `sort` supports documented pipeline fields            |

GitLab also provides a direct latest-pipeline operation:

```text
GET /projects/{id}/pipelines/latest
GET /projects/{id}/pipelines/latest?ref={branch-or-tag}
```

This is more precise and efficient than listing one item because it means “the latest pipeline for the most recent commit on the selected ref,” not merely the newest pipeline returned by a broad query. [GitLab latest-pipeline endpoint](https://docs.gitlab.com/api/pipelines/).

Gitea exposes latest workflow status through workflow badge URLs filtered by branch and event, but a badge is not a structured execution record. [Gitea workflow badges](https://docs.gitea.com/usage/actions/badge).

### Current Sniff limitations

- GitHub fetches only the first page.
- A requested limit over GitHub’s maximum cannot be fulfilled by following pages.
- The total matching count is discarded.
- There is no continuation token or page abstraction.
- “Latest” is implemented implicitly by requesting a small first page.
- GitLab’s dedicated latest endpoint is unavailable.
- Bitbucket’s `next` URL cannot be represented.
- Provider ordering guarantees are not exposed.

A list API should return a page model containing results, total count when available, and an opaque continuation value. It should not return only `Vec<CiCdInfo>`.

## Rich Execution Details

Exact lookup responses contain substantially more information than Sniff currently preserves.

### Commonly useful fields

- Native execution ID and human-facing sequence number
- Attempt number
- Workflow or pipeline definition ID and path
- Display title
- Commit SHA and previous SHA
- Branch or tag
- Trigger type
- Triggering user
- Created, queued, started, completed, and updated timestamps
- Duration and queue duration
- Provider-native status and normalized status
- Web and API URLs
- Pull or merge request association
- Parent or child pipeline relationship
- Configuration errors
- Coverage
- Archived or expired state

GitHub returns run ID, run number, attempt, workflow ID, path, head SHA, actors, timestamps, and related resource URLs. [GitHub workflow-run response](https://docs.github.com/en/rest/actions/workflow-runs).

GitLab’s single-pipeline response adds `iid`, user, `started_at`, `finished_at`, duration, queued duration, coverage, YAML errors, and detailed status. [GitLab single-pipeline response](https://docs.gitlab.com/api/pipelines/).

Bitbucket returns UUID, build number, creator, target, trigger, state, variables, timestamps, build seconds, and configuration sources. [Bitbucket pipeline response](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pipelines/).

All of these are unavailable or lossy through Sniff.

## Jobs, Steps, and Pipeline Hierarchy

A top-level success or failure is often insufficient for diagnosis. All major providers expose lower-level execution units.

| Provider    | API access                                                                                           |
|-------------|------------------------------------------------------------------------------------------------------|
| GitHub      | `GET /repos/{owner}/{repo}/actions/runs/{run_id}/jobs`; attempt-specific jobs can also be requested  |
| GitLab      | `GET /projects/{id}/pipelines/{pipeline_id}/jobs`; bridges are available at `/bridges`               |
| Gitea 1.25+ | `GET /repos/{owner}/{repo}/actions/runs/{run}/jobs`; individual job lookup is also available         |
| Bitbucket   | `GET /repositories/{workspace}/{repo_slug}/pipelines/{pipeline_uuid}/steps` and `/steps/{step_uuid}` |

GitHub jobs contain their own steps, statuses, conclusions, timestamps, runner labels, and runner identity.

GitLab jobs expose stage, failure reason, runner, duration, artifacts, coverage, retry history, and downstream bridges. Retried jobs can be included or excluded. [GitLab Jobs API](https://docs.gitlab.com/api/jobs/).

Bitbucket exposes pipeline steps and service-container logs separately. [Bitbucket pipeline steps](https://developer.atlassian.com/cloud/bitbucket/rest/api-group-pipelines/).

Sniff currently exposes none of this hierarchy.

## Logs, Artifacts, and Test Results

### Logs

| Provider  | API access                                                                                                              |
|-----------|-------------------------------------------------------------------------------------------------------------------------|
| GitHub    | `GET /repos/{owner}/{repo}/actions/runs/{run_id}/logs`; job log: `GET /repos/{owner}/{repo}/actions/jobs/{job_id}/logs` |
| GitLab    | `GET /projects/{id}/jobs/{job_id}/trace`                                                                                |
| Gitea     | `GET /repos/{owner}/{repo}/actions/jobs/{job_id}/logs`                                                                  |
| Bitbucket | `GET /repositories/{workspace}/{repo_slug}/pipelines/{pipeline_uuid}/steps/{step_uuid}/log` and `/logs/{log_uuid}`      |

### Artifacts

| Provider    | API access                                                                                                                                       |
|-------------|--------------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub      | `GET /repos/{owner}/{repo}/actions/runs/{run_id}/artifacts`; artifacts can be filtered by name                                                   |
| GitLab      | Artifacts are job-scoped under `/projects/{id}/jobs/{job_id}/artifacts`; individual files and latest-successful-ref artifacts are also available |
| Gitea 1.25+ | `GET /repos/{owner}/{repo}/actions/runs/{run}/artifacts`, plus repository and individual artifact endpoints                                      |
| Bitbucket   | Pipeline and step resources expose artifact-related links; support is less uniform than GitHub or GitLab                                         |

[GitHub’s artifact API](https://docs.github.com/en/rest/actions/artifacts) exposes artifact identity, size, expiry, digest, download URL, and associated workflow run.

[GitLab’s artifact API](https://docs.gitlab.com/api/job_artifacts/) supports complete archives, individual files, archive browsing, and lookup by ref and job name.

### Test results

| Provider      | API access                                                                                                                                |
|---------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| GitHub        | Test details normally come from job steps, logs, artifacts, checks, or third-party annotations rather than one workflow-run test endpoint |
| GitLab        | `GET /projects/{id}/pipelines/{pipeline_id}/test_report` and `/test_report_summary`                                                       |
| Gitea/Forgejo | No stable cross-version test-report equivalent                                                                                            |
| Bitbucket     | Step-level `/test_reports`, `/test_reports/test_cases`, and test-case reason endpoints                                                    |

GitLab and Bitbucket can therefore answer higher-level questions such as “which test suites failed?” without downloading and interpreting raw logs. Sniff cannot currently do this.

## Operational Capabilities

These go beyond the two required read operations but are useful adjacent capabilities.

| Capability               | GitHub                                         | GitLab                                                   | Gitea/Forgejo              | Bitbucket                                                    |
|--------------------------|------------------------------------------------|----------------------------------------------------------|----------------------------|--------------------------------------------------------------|
| Start or dispatch        | Workflow dispatch endpoint                     | Create pipeline endpoint                                 | Workflow dispatch endpoint | `POST /pipelines`                                            |
| Cancel                   | Cancel and force-cancel run endpoints          | `POST /pipelines/{pipeline_id}/cancel`                   | Version-dependent          | `POST /pipelines/{pipeline_uuid}/stopPipeline`               |
| Retry                    | Retry entire, failed, or selected jobs         | `POST /pipelines/{pipeline_id}/retry` and job retry APIs | Version-dependent          | Start a new pipeline; no directly equivalent retry operation |
| Approvals or deployments | Run approvals and pending deployment endpoints | Manual jobs and environment/deployment APIs              | Version-dependent          | Deployment and environment APIs                              |

These should remain separate from the initial read-only query interface so that adding observation does not implicitly authorize mutation.

## Configuration and Capability Discovery

File detection alone is not a reliable indication of execution availability.

### GitHub

- Workflows can exist outside the default branch.
- Disabled workflows and historical runs can exist even if a workflow file has been removed.
- The workflows API can report active and disabled definitions more accurately than a tree scan.

### GitLab

- Projects can use a custom CI configuration path rather than `.gitlab-ci.yml`.
- Pipelines can be child, multi-project, scheduled, externally triggered, or created from merge requests.
- The project’s CI configuration and pipeline APIs are more authoritative than file presence.

### Gitea and Forgejo

- Gitea workflow directories are instance-configurable.
- Gitea API support differs materially by server version.
- Forgejo’s workflow directories and Actions API are not guaranteed to match Gitea’s.
- External systems such as Drone and Woodpecker require their own base URL and credentials; detecting their configuration file does not provide an execution API.

### Bitbucket

- `bitbucket-pipelines.yml` can exist while Pipelines is disabled.
- `GET /repositories/{workspace}/{repo_slug}/pipelines_config` is a better capability signal.
- Pipelines queries require an appropriate pipeline-read scope.

Sniff should expose explicit capability information such as:

- configuration detected
- execution listing supported
- exact lookup supported
- jobs supported
- logs supported
- artifacts supported
- test reports supported
- mutation supported
- required authentication
- provider/server version restrictions

## Authentication Gaps

Authentication failures must not be collapsed into “no executions.”

The current `fetch_report()` behavior hides optional CI/CD errors and may fall back to configuration detection, making permission failures indistinguishable from empty history.

Bitbucket deserves additional attention: Sniff currently documents username plus App Password authentication, while Bitbucket now recommends API tokens or OAuth access tokens and marks App Passwords as deprecated. [Bitbucket authentication documentation](https://developer.atlassian.com/cloud/bitbucket/rest/intro/).

A query result should distinguish at least:

- supported and successful with no matches
- unsupported by the provider or server version
- unauthenticated
- authenticated but unauthorized
- rate limited
- provider unavailable
- malformed query

## Recommended Model

### Separate configuration from execution

Use distinct types, conceptually:

```text
CiCdConfiguration
CiCdExecution
CiCdExecutionRef
CiCdExecutionQuery
CiCdExecutionPage
CiCdCapabilities
```

`CiCdConfiguration` should retain configuration-file and provider-detection behavior. `CiCdExecution` should always represent a real provider execution and always carry a native identity.

### Exact reference

A reference should contain:

```text
provider
host
repository
native execution ID
optional attempt
```

Provider URLs should be accepted as ergonomic input and normalized into the same structure.

Bare numeric IDs should only be accepted when the provider and repository are already supplied by context.

### Common query

The portable query should include:

```text
statuses
workflow or pipeline name
workflow definition ID or path
ref
commit SHA
actor
trigger or event
created range
updated range
limit
continuation
sort direction
latest
```

Provider-specific filters should be available through typed extensions or a clearly labeled native-query escape hatch.

### Status preservation

Each execution should retain:

- the raw provider status
- the raw provider conclusion or result
- a normalized lifecycle status

Normalization must not discard meaningful states such as manual, waiting for approval, blocked, skipped, neutral, timed out, or configuration error.

### Error and pagination contracts

- Exact lookup should return `None` only for an authoritative not-found response.
- List queries should return a page with continuation and total count when available.
- Unsupported capabilities should be explicit, not represented by an empty vector.
- Aggregate reports may degrade gracefully, but focused query APIs should preserve errors.

## Conclusion

Sniff has a useful provider abstraction and the beginning of a normalized execution record, but its CI/CD support is presently reporting-oriented rather than query-oriented.

The strongest existing foundation is:

- provider detection and authentication setup
- remote URL normalization
- a provider-neutral trait
- GitHub’s initial workflow-run listing
- Schematic definitions for GitHub runs and GitLab pipelines
- terminal rendering for recent executions

The principal gaps are:

- no execution identity
- no exact lookup
- no query model
- GitHub-only execution listing
- no pagination
- lossy execution mapping
- configuration and execution conflated in one type
- no jobs, logs, artifacts, or test results
- hidden errors and unsupported capabilities
- incomplete Gitea/Forgejo workflow detection
- outdated Bitbucket authentication ergonomics

Meeting the two requested goals requires a new execution-specific API rather than extending `list_workflow_runs(owner, repo, limit)` with more positional parameters.
