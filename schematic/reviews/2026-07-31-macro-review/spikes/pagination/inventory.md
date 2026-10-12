# Pagination inventory: REST definitions in `schematic-definitions`

Read-only spike. Snapshot of `schematic/definitions/src` on branch `feat/modern-config`, 2026-10-09.

## Method and confidence

- "Declared" means the endpoint sets `EndpointParams.pagination` (`PaginationStyle`). A repo-wide search finds **no** non-`None` `response_pagination` (`PaginationResponse`) anywhere; every response-side signal below is therefore *undeclared* today.
- OpenAI is generated from OpenAPI: `pagination: None` and `response_pagination: None` on all 265 endpoints, but `after`/`limit`/`page`/`order` are present as ordinary `ParamDef` query params. Pagination is encoded as plain params, not as `PaginationStyle`.
- Confidence tags: **V-def** = confirmed from our own response structs/params in this repo; **V-notes** = confirmed in repo research notes (`schematic/docs/research/elevenlabs-api.md`, `sniff/docs/repos/{github,gitea,gitlab,bitbucket}-api.md`); **U** = recalled from the provider's public docs, not re-fetched in this spike (no web access used). Treat **U** as needing a check before it drives a design.
- 18 REST APIs: anthropic, artificial_analysis (data), artificial_analysis (critpt), bitbucket, elevenlabs (rest), emqx (basic), emqx (bearer), eversolo, gitea, github, gitlab, huggingface, lmstudio, ollama (native), ollama (openai-compat), openai, samsung_smart_tv (rest), unfolded_circle (core_rest). The two EMQX APIs and the two Ollama APIs are separate definitions; EMQX basic/bearer share one endpoint set.

## Cross-cutting definition findings

- `PaginationStyle::gitlab()` is `github()` (page/per_page, default 100, max 100). Real defaults differ: GitHub 30/100, GitLab 20/100. `gitea()` declares 50/100; real Gitea default is 30 and the server cap `MAX_RESPONSE_ITEMS` defaults to 50 (V-notes). `bitbucket()` declares 50/100; real `pagelen` default is 10 (U).
- Declared styles only describe request params. Nothing in a definition says where the continuation signal is, so the generator cannot loop even for the 27 declared endpoints.
- Five definitions model the collection with an envelope whose own fields carry the signal (Anthropic, OpenAI, ElevenLabs, EMQX `meta`, Bitbucket `PaginatedResponse<T>`).

---

## anthropic (1 collection endpoint, 0 declared)

| id | path | shape | declared | real provider | conf |
|---|---|---|---|---|---|
| ListModels | `GET /models` | wrapper `data`; also `first_id`, `last_id`, `has_more` | none (`params: None`) | Request: `after_id`, `before_id`, `limit` (default 20, 1..1000). Reply: `has_more`, `first_id`, `last_id`. Cursor is a model id. | wrapper V-def; params U |

Silent gap: paginated, no params or pagination declared.

## artificial_analysis (data: 6, critpt: 0; 0 declared)

| id | path | shape | declared | real | conf |
|---|---|---|---|---|---|
| ListLlmModels | `GET /data/llms/models` | wrapper `data` (+`status`, `prompt_options`) | none | Unpaginated full list | U |
| ListTextToImageModels | `GET /data/media/text-to-image` | wrapper `data` (+`status`, `include_categories`) | none | Unpaginated | U |
| ListImageEditingModels | `GET /data/media/image-editing` | same | none | Unpaginated | U |
| ListTextToSpeechModels | `GET /data/media/text-to-speech` | same | none | Unpaginated | U |
| ListTextToVideoModels | `GET /data/media/text-to-video` | same | none | Unpaginated | U |
| ListImageToVideoModels | `GET /data/media/image-to-video` | same | none | Unpaginated | U |

CritPt API: `EvaluateCritPt` is a POST scoring call (no collection response).

## bitbucket (9 collection endpoints, 9 declared)

All use `PaginationStyle::bitbucket()` = PageNumber{`page`, `pagelen`, default 50, max 100} via `bitbucket_pagination()`. All responses are `PaginatedResponse<T>` with `values: Vec<T>` and optional `next`, `previous`, `size`, `pagelen`, `page` (V-def).

| id | path | shape | declared | real |
|---|---|---|---|---|
| ListDirectoryContents | `GET /repositories/{workspace}/{repo_slug}/src/{commit}/{path}` | wrapper `values` | PageNumber page/pagelen | same strategy |
| ListPullRequests | `GET .../pullrequests` | wrapper `values` | same | same |
| ListPullRequestComments | `GET .../pullrequests/{id}/comments` | wrapper `values` | same | same |
| ListIssues | `GET .../issues` | wrapper `values` | same | same |
| ListIssueComments | `GET .../issues/{id}/comments` | wrapper `values` | same | same |
| ListIssueChanges | `GET .../issues/{id}/changes` | wrapper `values` | same | same |
| ListTags | `GET .../refs/tags` | wrapper `values` | same | same |
| ListDownloads | `GET .../downloads` | wrapper `values` | same | same |
| ListWorkspaceRepos | `GET /repositories/{workspace}` | wrapper `values` | same | same |

Real behavior (V-notes for next-URL; rest U): request `page` (1-based), `pagelen` (default 10, max 100; some resources allow more). The continuation signal is the body field **`next`**, a complete absolute URL (opaque; follow verbatim, absent on last page). `size` (total) may be omitted for large sets. `page` numbers are not guaranteed stable across mutations, so `next` is the safe loop. The definition declares no `response_pagination` (BodyField{next_field:"next"} would fit). Declared default 50 is wrong (real 10).

## elevenlabs REST (7 collection endpoints, 0 declared)

Definitions declare **no** query params at all for these (V-def); parameters below are from `docs/research/elevenlabs-api.md` (V-notes).

| id | path | shape | real provider |
|---|---|---|---|
| GetHistory | `GET /v1/history` | wrapper `history`; `last_history_item_id`, `has_more`, `scanned_until` | cursor: request `start_after_history_item_id` + `page_size` (default 100, max 1000); reply `last_history_item_id`, `has_more`. V-notes |
| ListVoices | `GET /v2/voices` | wrapper `voices`; `has_more`, `total_count`, `next_page_token` | token: request `next_page_token` + `page_size` (default 10, max 100) + `include_total_count` (default true); reply `next_page_token`, `has_more`, `total_count`. V-notes |
| ListSharedVoices | `GET /v1/shared-voices` | wrapper `voices`; `has_more`, `last_sort_id` | request `page` (0-based integer) + `page_size` (default 30, max 100); reply `has_more`, `last_sort_id` (U: `last_sort_id` is for the cursor-style variant used with `sort`). V-notes for params |
| ListModels | `GET /v1/models` | **root array** (`json_vec_type("ModelInfo")`) | unpaginated |
| ListServiceAccounts | `GET /v1/service-accounts` | wrapper `service_accounts` | unpaginated (U) |
| ListServiceAccountApiKeys | `GET /v1/service-accounts/{id}/api-keys` | wrapper `api_keys` | unpaginated (U) |
| ListWebhooks | `GET /v1/workspace/webhooks` | wrapper `webhooks` | unpaginated (U) |

Silent gaps: GetHistory, ListVoices, ListSharedVoices (three different strategies in one API).

## emqx basic and emqx bearer (14 collection endpoints each, 0 declared; same endpoint set)

All endpoints have `params: None` (V-def), though descriptions say "with pagination" on `/clients` and `/retainer/messages`. Wrapper types that carry `meta: Option<PaginationMeta>{count, limit, page, hasnext}` (V-def): ListBanned, ListClients, ListSubscriptions, ListTopics, ListRetained.

| id | path | shape | real provider |
|---|---|---|---|
| ListClients | `GET /clients` | wrapper `data` + `meta` | request `page` (1-based), `limit` (default 10; U); reply `meta.{page,limit,count,hasnext}`. U |
| ListSubscriptions | `GET /subscriptions` | `data` + `meta` | same. U |
| ListTopics | `GET /topics` | `data` + `meta` | same. U |
| ListRetained | `GET /retainer/messages` | `data` + `meta` | same. U |
| ListBanned | `GET /banned` | `data` + `meta` | same. U |
| ListAuthUsers | `GET /authentication/{id}/users` | **root array** `Vec<AuthUser>` in definition | real reply is `{data:[...], meta:{...}}` with `page`/`limit` (U): **definition shape likely wrong** |
| ListRules | `GET /rules` | `data` only (no `meta` modelled) | real supports `page`/`limit` + `meta` (U); wrapper omits `meta` |
| ListAlarms | `GET /alarms` | `data` only | real supports `page`/`limit` + `meta` (U) |
| ListAuthenticators, ListAuthzSources (`sources`), ListNodes, ListListeners, ListMetrics, ListStats | various | `data` / `sources` | unpaginated config/state lists (U) |

Silent gaps: ListClients, ListSubscriptions, ListTopics, ListRetained, ListBanned, ListAuthUsers, ListRules, ListAlarms. Signal is a **nested** body object (`meta.hasnext`), not a top-level field.

## eversolo (0 true collections, 0 declared)

Device-control API. `getInputAndOutputList`, `getVUModeList`, `getSpPlayModeList` (and the set* calls that echo them) return option lists in wrappers (`input_data`/`output_data`, `data`). Fixed small enumerations, not paginated (V-def). 24 endpoints, many with query params, none pagination.

## gitea (11 collection-bearing endpoints, 8 declared)

Declared: `PaginationStyle::gitea()` = PageNumber{`page`,`limit`, 50, max 100}. Real (V-notes, `sniff/docs/repos/gitea-api.md`): `page` (1-based) + `limit`; default 30, server cap `MAX_RESPONSE_ITEMS` (default 50); reply headers `Link` (rel next/prev/first/last), `X-Total-Count`, also `X-HasMore`, `X-PageCount`, `X-Page`, `X-PerPage` (U for the latter four).

| id | path | shape | declared |
|---|---|---|---|
| ListPullRequests | `GET /repos/{owner}/{repo}/pulls` | root array | yes |
| ListPullRequestFiles | `GET .../pulls/{index}/files` | root array | yes |
| ListIssues | `GET .../issues` | root array | yes |
| ListIssueComments | `GET .../issues/{index}/comments` | root array | yes |
| ListIssueTimeline | `GET .../issues/{index}/timeline` | root array | yes |
| ListTags | `GET .../tags` | root array | yes |
| ListReleases | `GET .../releases` | root array | yes |
| ListOrgRepos | `GET /orgs/{org}/repos` | root array | yes |
| GetGitTree | `GET .../git/trees/{sha}` | wrapper `tree`; has `truncated`, `total_count`, `page` | **no (gap)**: real is paginated with `page` + `per_page` (U); body carries `total_count`/`page`/`truncated` |
| GetGitTreeRecursive | same path + `?recursive=true` | wrapper `tree` | **no (gap)**, same |
| GetTagReference | `GET .../git/refs/{git_ref}` | root array (`Vec<GitRef>`) | no; real unpaginated prefix match (U) |

Note: the tree endpoint uses `per_page`, not `limit`, so one API has two page-size param names.

## github (9 collection endpoints, 9 declared)

Declared: `PaginationStyle::github()` = PageNumber{`page`,`per_page`,100,100}. Real (V-notes): `page` (1-based) + `per_page` (default 30, max 100); continuation = **`Link` header** (rel next/prev/first/last; `last` absent on some endpoints); no total except in specific bodies.

| id | path | shape | declared |
|---|---|---|---|
| ListPullRequests | `GET /repos/{owner}/{repo}/pulls` | root array | yes |
| ListPullRequestFiles | `.../pulls/{pull_number}/files` | root array (provider caps at 3000 files) | yes |
| ListIssues | `.../issues` | root array | yes |
| ListIssueComments | `.../issues/{issue_number}/comments` | root array | yes |
| ListIssueTimeline | `.../issues/{issue_number}/timeline` | root array | yes |
| ListTags | `.../tags` | root array | yes |
| ListReleases | `.../releases` | root array | yes |
| ListWorkflowRuns | `.../actions/runs` | wrapper `workflow_runs` + `total_count` | yes (page/per_page); `total_count` body field carries the total (fits `TotalCount{total_field:"total_count"}`) |
| ListOrgRepos | `GET /orgs/{org}/repos` | root array | yes |

Not collections: GetGitTree/GetGitTreeRecursive (wrapper with `truncated`, provider limit 100,000 entries, not page-paginated; U).

## gitlab (10 collection endpoints, 9 declared)

Declared: `PaginationStyle::gitlab()` = github() (page/per_page 100/100). Real (V-notes, `sniff/docs/repos/gitlab-api.md`): offset pagination `page` + `per_page` (default 20, max 100); headers `X-Page`, `X-Per-Page`, `X-Total`, `X-Total-Pages`, `X-Next-Page`, `X-Prev-Page` and `Link`. `X-Total`/`X-Total-Pages` and `rel="last"` are **omitted above 10,000 records**. Keyset mode (`pagination=keyset`, `order_by`, `sort`, `per_page`; continuation by `Link rel="next"` carrying a `page_token`/`id_after`) exists for projects, group projects and repository tree (U) and is recommended for large sets.

| id | path | shape | declared |
|---|---|---|---|
| ListIssues | `GET /projects/{id}/issues` | root array | yes |
| ListIssueNotes | `.../issues/{issue_iid}/notes` | root array | yes |
| ListIssueParticipants | `.../issues/{issue_iid}/participants` | root array | **no (gap)**: believed paginated like other GitLab lists (U) |
| ListMergeRequests | `GET /projects/{id}/merge_requests` | root array | yes |
| ListMergeRequestCommits | `.../merge_requests/{iid}/commits` | root array | yes |
| ListProjectPipelines | `.../pipelines` | root array | yes |
| ListRepositoryTree | `.../repository/tree` | root array | yes (offset; keyset also available) |
| ListGroupProjects | `GET /groups/{id}/projects` | root array | yes (offset; keyset also available) |
| ListTags | `.../repository/tags` | root array | yes |
| ListReleases | `.../releases` | root array | yes |

Not collections: ListMergeRequestChanges returns the MR object with an embedded `changes` array (deprecated upstream, not paginated).

## huggingface (11 collection endpoints, 0 declared)

All `params: None`; the description for ListModels/ListDatasets mentions `limit` only as free text. All Hub list calls here are **U**.

| id | path | shape | real provider (U) |
|---|---|---|---|
| ListModels | `GET /models` | root array | `limit` request param; continuation = `Link: <...cursor=...>; rel="next"` header |
| ListDatasets | `GET /datasets` | root array | same |
| ListSpaces | `GET /spaces` | root array | same |
| ListModelFiles / ListDatasetFiles / ListSpaceFiles | `GET /{models|datasets|spaces}/{+repo_id}/tree/{revision}` | root array | `limit`, `recursive`, `expand`; continuation = `Link rel="next"` with `cursor` |
| ListModelCommits / ListDatasetCommits | `.../commits/{revision}` | root array | `p` (0-based page index) + `limit`; no Link header (unverified) |
| ListModelDiscussions | `GET /models/{+repo_id}/discussions` | wrapper `discussions` (+`count`) | `p` (0-based page); reply `count`, `start`, `numClosedDiscussions` |
| ListUserRepos | `GET /users/{username}/repos` | root array | unverified endpoint |
| GetUserCollections | `GET /users/{username}/collections` | root array | unverified endpoint |

Silent gaps: all 11. Client never constructs the cursor; it must follow the `Link` URL.

## lmstudio (1 collection endpoint, 0 declared)

ListModels `GET /api/v1/models`: wrapper `{object, data}`; local server, unpaginated (U).

## ollama native and ollama openai-compat (native 2, openai 1; 0 declared)

- native `ListModels` `GET /api/tags`: wrapper `models`; `ListRunningModels` `GET /api/ps`: wrapper `models`. Unpaginated (U).
- openai-compat `ListModels` `GET /v1/models`: wrapper `{object, data}`. Unpaginated (U).

## openai (60 GET collection endpoints + 1 POST search; 0 declared as `PaginationStyle`)

All 60 GET list endpoints return `{object:"list", data:[...], ...}`; 61st row is a POST that pages through the body. Three real strategies (V-def for wrapper fields and param names; defaults from the generated param descriptions: limit default 20 max 100 for id-cursor lists). Every row below has its cursor params declared only as ordinary query params.

- **S6**: request `after` (an object id), `limit`, sometimes `before`/`order` (`asc`/`desc`); reply `has_more`, `first_id`, `last_id` (cursor for the next page is the previous reply's `last_id`; ListFineTuningEvents and ListPaginatedFineTuningJobs reply with `has_more` only, so the cursor is the last item's `id`). 38 endpoints.
- **S7**: request `after` as an opaque cursor, `limit`, `order`; reply `has_more` and `next` (the cursor to pass as `after`). 9 role/group endpoints.
- **S8**: request `page` (cursor string = previous reply's `next_page`), `limit` (bucket count), plus time filters; reply `has_more`, `next_page`. 11 usage endpoints (UsageCosts and UsageImages declare no `limit`). Plus RetrieveVectorStoreFileContent and SearchVectorStore reply with `has_more` + `next_page` but expose **no** request param to continue (documented continuation absent in our definition; SearchVectorStore request body has `max_num_results` only).
- ListModels: unpaginated.

| endpoint id | path | response type (wrapper prop) | declared query params | strategy |
|---|---|---|---|---|
| ListVoiceConsents | `/audio/voice_consents` | VoiceConsentListResource (`data`) | `after`, `limit` | S6 after<-last_id |
| ListBatches | `/batches` | ListBatchesResponse (`data`) | `after`, `limit` | S6 after<-last_id |
| ListChatCompletions | `/chat/completions` | ChatCompletionList (`data`) | `model`, `metadata`, `after`, `limit`, `order` | S6 after<-last_id |
| GetChatCompletionMessages | `/chat/completions/{completion_id}/messages` | ChatCompletionMessageList (`data`) | `after`, `limit`, `order` | S6 after<-last_id |
| ListContainers | `/containers` | ContainerListResource (`data`) | `limit`, `order`, `after`, `name` | S6 after<-last_id |
| ListContainerFiles | `/containers/{container_id}/files` | ContainerFileListResource (`data`) | `limit`, `order`, `after` | S6 after<-last_id |
| ListConversationItems | `/conversations/{conversation_id}/items` | ConversationItemList (`data`) | `limit`, `order`, `after`, `include` | S6 after<-last_id |
| ListEvals | `/evals` | EvalList (`data`) | `after`, `limit`, `order`, `order_by` | S6 after<-last_id |
| GetEvalRuns | `/evals/{eval_id}/runs` | EvalRunList (`data`) | `after`, `limit`, `order`, `status` | S6 after<-last_id |
| GetEvalRunOutputItems | `/evals/{eval_id}/runs/{run_id}/output_items` | EvalRunOutputItemList (`data`) | `after`, `limit`, `status`, `order` | S6 after<-last_id |
| ListFiles | `/files` | ListFilesResponse (`data`) | `purpose`, `limit`, `order`, `after` | S6 after<-last_id |
| ListFineTuningCheckpointPermissions | `/fine_tuning/checkpoints/{fine_tuned_model_checkpoint}/permissions` | ListFineTuningCheckpointPermissionResponse (`data`) | `project_id`, `after`, `limit`, `order` | S6 after<-last_id |
| ListPaginatedFineTuningJobs | `/fine_tuning/jobs` | ListPaginatedFineTuningJobsResponse (`data`) | `after`, `limit`, `metadata` | S6 after<-last_id |
| ListFineTuningJobCheckpoints | `/fine_tuning/jobs/{fine_tuning_job_id}/checkpoints` | ListFineTuningJobCheckpointsResponse (`data`) | `after`, `limit` | S6 after<-last_id |
| ListFineTuningEvents | `/fine_tuning/jobs/{fine_tuning_job_id}/events` | ListFineTuningJobEventsResponse (`data`) | `after`, `limit` | S6 after<-last_id |
| ListModels | `/models` | ListModelsResponse (`data`) | (none) | none (unpaginated) |
| AdminApiKeysList | `/organization/admin_api_keys` | ApiKeyList (`data`) | `after`, `order` | S6 after<-last_id |
| ListAuditLogs | `/organization/audit_logs` | ListAuditLogsResponse (`data`) | `effective_at`, `tenant_only`, `limit`, `after`, `before` | S6 after<-last_id |
| ListOrganizationCertificates | `/organization/certificates` | ListCertificatesResponse (`data`) | `limit`, `after`, `order` | S6 after<-last_id |
| UsageCosts | `/organization/costs` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `api_key_ids`, `group_by`, `page` | S8 page<-next_page |
| ListGroups | `/organization/groups` | GroupListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListGroupRoleAssignments | `/organization/groups/{group_id}/roles` | RoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListGroupUsers | `/organization/groups/{group_id}/users` | UserListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListInvites | `/organization/invites` | InviteListResponse (`data`) | `limit`, `after` | S6 after<-last_id |
| ListProjects | `/organization/projects` | ProjectListResponse (`data`) | `limit`, `after`, `include_archived` | S6 after<-last_id |
| ListProjectApiKeys | `/organization/projects/{project_id}/api_keys` | ProjectApiKeyListResponse (`data`) | `limit`, `after`, `owner_project_access` | S6 after<-last_id |
| ListProjectCertificates | `/organization/projects/{project_id}/certificates` | ListProjectCertificatesResponse (`data`) | `limit`, `after`, `order` | S6 after<-last_id |
| ListProjectGroups | `/organization/projects/{project_id}/groups` | ProjectGroupListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListProjectRateLimits | `/organization/projects/{project_id}/rate_limits` | ProjectRateLimitListResponse (`data`) | `limit`, `after`, `before` | S6 after<-last_id |
| ListProjectServiceAccounts | `/organization/projects/{project_id}/service_accounts` | ProjectServiceAccountListResponse (`data`) | `limit`, `after` | S6 after<-last_id |
| ListProjectSpendAlerts | `/organization/projects/{project_id}/spend_alerts` | ProjectSpendAlertListResource (`data`) | `limit`, `order`, `after`, `before` | S6 after<-last_id |
| ListProjectUsers | `/organization/projects/{project_id}/users` | ProjectUserListResponse (`data`) | `limit`, `after` | S6 after<-last_id |
| ListRoles | `/organization/roles` | PublicRoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListOrganizationSpendAlerts | `/organization/spend_alerts` | OrganizationSpendAlertListResource (`data`) | `limit`, `order`, `after`, `before` | S6 after<-last_id |
| UsageAudioSpeeches | `/organization/usage/audio_speeches` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageAudioTranscriptions | `/organization/usage/audio_transcriptions` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageCodeInterpreterSessions | `/organization/usage/code_interpreter_sessions` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageCompletions | `/organization/usage/completions` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `batch`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageEmbeddings | `/organization/usage/embeddings` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageFileSearchCalls | `/organization/usage/file_search_calls` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `limit`, `page` | S8 page<-next_page |
| UsageImages | `/organization/usage/images` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `sources`, `sizes`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `page` | S8 page<-next_page |
| UsageModerations | `/organization/usage/moderations` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageVectorStores | `/organization/usage/vector_stores` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `group_by`, `limit`, `page` | S8 page<-next_page |
| UsageWebSearchCalls | `/organization/usage/web_search_calls` | UsageResponse (`data`) | `start_time`, `end_time`, `bucket_width`, `project_ids`, `user_ids`, `api_key_ids`, `models`, `group_by`, `limit`, `page` | S8 page<-next_page |
| ListUsers | `/organization/users` | UserListResponse (`data`) | `limit`, `after`, `emails` | S6 after<-last_id |
| ListUserRoleAssignments | `/organization/users/{user_id}/roles` | RoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListProjectGroupRoleAssignments | `/projects/{project_id}/groups/{group_id}/roles` | RoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListProjectRoles | `/projects/{project_id}/roles` | PublicRoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListProjectUserRoleAssignments | `/projects/{project_id}/users/{user_id}/roles` | RoleListResource (`data`) | `limit`, `after`, `order` | S7 after<-next |
| ListInputItems | `/responses/{response_id}/input_items` | ResponseItemList (`data`) | `limit`, `order`, `after`, `include` | S6 after<-last_id |
| ListVectorStores | `/vector_stores` | ListVectorStoresResponse (`data`) | `limit`, `order`, `after`, `before` | S6 after<-last_id |
| ListFilesInVectorStoreBatch | `/vector_stores/{vector_store_id}/file_batches/{batch_id}/files` | ListVectorStoreFilesResponse (`data`) | `limit`, `order`, `after`, `before`, `filter` | S6 after<-last_id |
| ListVectorStoreFiles | `/vector_stores/{vector_store_id}/files` | ListVectorStoreFilesResponse (`data`) | `limit`, `order`, `after`, `before`, `filter` | S6 after<-last_id |
| RetrieveVectorStoreFileContent | `/vector_stores/{vector_store_id}/files/{file_id}/content` | VectorStoreFileContentResponse (`data`) | (none) | next_page+has_more in reply, no request param declared (GAP) |
| SearchVectorStore | `/vector_stores/{vector_store_id}/search` | VectorStoreSearchResultsPage (`data`) | (none) | POST body; next_page+has_more in reply, NO continuation param in request |
| ListVideos | `/videos` | VideoListResource (`data`) | `limit`, `order`, `after` | S6 after<-last_id |
| ListSkills | `/skills` | SkillListResource (`data`) | `limit`, `order`, `after` | S6 after<-last_id |
| ListSkillVersions | `/skills/{skill_id}/versions` | SkillVersionListResource (`data`) | `limit`, `order`, `after` | S6 after<-last_id |
| ListThreadItemsMethod | `/chatkit/threads/{thread_id}/items` | ThreadItemListResource (`data`) | `limit`, `order`, `after`, `before` | S6 after<-last_id |
| ListThreadsMethod | `/chatkit/threads` | ThreadListResource (`data`) | `limit`, `order`, `after`, `before`, `user` | S6 after<-last_id |
| BetaListInputItems | `/responses/{response_id}/input_items?beta=true` | BetaResponseItemList (`data`) | `limit`, `order`, `after`, `include` | S6 after<-last_id |

Related non-paginated list returns: CreateConversationItems (POST, returns `ConversationItemList`) and CreateFineTuningCheckpointPermission (POST, returns a list wrapper) carry list-shaped replies but are create calls; no continuation.

Silent gaps: 60 paginated rows (38 S6 + 9 S7 + 11 usage S8 + RetrieveVectorStoreFileContent + SearchVectorStore); ListModels is the only unpaginated list.

## samsung_smart_tv rest (0) and unfolded_circle core_rest (0)

No collection endpoints. UC `UploadResource` and `RestoreBackup` return root arrays (`ResourceItems`, `BackupRestoreReportItems`) but they are write results, not lists. (UC's real REST list endpoints are not defined here.)

## WebSockets (5: elevenlabs, samsung remote, unfolded_circle core/dock/integration)

None paginate; they are message/stream envelopes.

---

## (1) Summary table

| API | collection endpoints | with declared pagination | distinct real strategies in use |
|---|---|---|---|
| anthropic | 1 | 0 | S6 id-cursor (before_id/after_id) |
| artificial_analysis data | 6 | 0 | none |
| artificial_analysis critpt | 0 | 0 | none |
| bitbucket | 9 | 9 | S5 page+pagelen with body `next` URL |
| elevenlabs rest | 7 (3 paginated) | 0 | S9, S10, S11 (three) |
| emqx basic | 14 (8 paginated) | 0 | S12 page+limit with body `meta` |
| emqx bearer | 14 (8 paginated) | 0 | S12 |
| eversolo | 0 (3 option lists) | 0 | none |
| gitea | 11 (9 paginated incl. tree) | 8 | S4 page+limit (Link, X-Total-Count); S4b tree page+per_page |
| github | 9 | 9 | S1 page+per_page (Link) |
| gitlab | 10 | 9 | S2 offset (X-* headers, Link); S3 keyset available |
| huggingface | 11 | 0 | S13 Link-cursor; S14 `p` page index (U) |
| lmstudio | 1 | 0 | none |
| ollama native | 2 | 0 | none |
| ollama openai | 1 | 0 | none |
| openai | 61 | 0 | S6, S7, S8 (+POST search) |
| samsung rest | 0 | 0 | none |
| unfolded_circle core_rest | 0 | 0 | none |

Totals: about 157 collection endpoints, 35 with declared pagination, all of the declared ones from the four forges (gitea 8, github 9, gitlab 9, bitbucket 9).

## (2) Distinct real strategies (exact parameters and signals)

| id | name | request params | continuation signal in reply | page size |
|---|---|---|---|---|
| S1 | GitHub page+Link | `page` (1-based), `per_page` | `Link` header rel next/prev/first/last; `total_count` body field only on some wrappers | 30 / 100 |
| S2 | GitLab offset | `page`, `per_page` | headers `X-Next-Page`, `X-Prev-Page`, `X-Page`, `X-Per-Page`, `X-Total`, `X-Total-Pages`, `Link`; totals omitted above 10,000 | 20 / 100 |
| S3 | GitLab keyset | `pagination=keyset`, `order_by`, `sort`, `per_page` (+ `page_token`) | `Link rel="next"` only | per_page max 100 |
| S4 | Gitea | `page` (1-based), `limit` | `Link`, `X-Total-Count`, `X-HasMore`, `X-PageCount` | 30 / server cap 50 |
| S4b | Gitea git tree | `page`, `per_page` | body `total_count`, `page`, `truncated` | U |
| S5 | Bitbucket | `page` (1-based), `pagelen` | body `next` (absolute URL), `previous`, `size`, `page`, `pagelen` | 10 / 100 |
| S6 | id cursor | `after` (+ `before`), `limit`, `order`; Anthropic: `after_id`, `before_id`, `limit` | body `has_more`, `first_id`, `last_id` | 20 / 100 (Anthropic 20 / 1000) |
| S7 | opaque cursor | `after`, `limit`, `order` | body `has_more`, `next` | 20 / 100 |
| S8 | page token | `page`, `limit` | body `has_more`, `next_page` | per bucket width |
| S9 | ElevenLabs history | `start_after_history_item_id`, `page_size` | body `last_history_item_id`, `has_more`, `scanned_until` | 100 / 1000 |
| S10 | ElevenLabs voices v2 | `next_page_token`, `page_size`, `include_total_count` | body `next_page_token`, `has_more`, `total_count` | 10 / 100 |
| S11 | ElevenLabs shared | `page` (0-based), `page_size` | body `has_more`, `last_sort_id` | 30 / 100 |
| S12 | EMQX | `page` (1-based), `limit` | body `meta.{page,limit,count,hasnext}` (nested) | 10 (U) |
| S13 | Hugging Face Link cursor | `limit` (+ `cursor` only via the URL) | `Link rel="next"` header | U |
| S14 | Hugging Face page index | `p` (0-based), `limit` | body `count` (+`start`) on discussions | U |

## (3) What does not fit the current enums

Request side (`PageNumber` / `OffsetLimit` / `Cursor`):

- No provider here uses `OffsetLimit`; no definition declares it. All 35 declared endpoints are `PageNumber`.
- `PageNumber` assumes 1-based. ElevenLabs shared voices (`page`, 0-based) and Hugging Face `p` (0-based) need a base/start-index.
- `PageNumber` fixes one `per_page_param`. Gitea uses `limit` on lists but `per_page` on the git tree.
- `Cursor` has one `cursor_param`. Anthropic and OpenAI (`before`) are bidirectional (`after_id`/`before_id`, `after`/`before`).
- HF cursor is never a request param the client builds (it follows `Link`); `Cursor.cursor_param` has nothing to name.
- GitLab keyset needs mode params (`pagination=keyset`, `order_by`, `sort`) with no slot.
- OpenAI `page` and ElevenLabs `next_page_token` are cursors named like or unlike page numbers; naming cannot distinguish `PageNumber` from `Cursor`.

Response side (`BodyField` / `LinkHeader` / `TotalCount`):

- `has_more` plus a derived cursor (id of last item, or `last_id`/`last_history_item_id`): OpenAI S6, Anthropic, ElevenLabs history. `BodyField{next_field}` names a *next* value, not "last id + boolean".
- Nested paths (`meta.hasnext` in EMQX): `next_field` is a flat string.
- Boolean-only signal (`has_more`, `hasnext`): no variant.
- Header counters other than `Link` (GitLab `X-Next-Page`, `X-Total-Pages`; Gitea `X-Total-Count`, `X-HasMore`): `LinkHeader` covers only RFC 5988 `Link`; `TotalCount` is body-only.
- Absolute next URL in the body (Bitbucket `next`): `BodyField` can name it, but nothing says "value is a URL, not a token".
- Item-array location is not recorded anywhere (root array vs `data`/`values`/`voices`/`history`/`workflow_runs`/`discussions`/`tree`).
- POST-with-body pagination: OpenAI `SearchVectorStore` has body-side continuation in the reply and none in the request; no POST search in the other 17 APIs paginates through its body.
- Short-page (return fewer than page size means last page) is not an official signal for any provider here; GitHub/Gitea/GitLab/Bitbucket all give explicit signals.
- Declared defaults conflict with real ones (GitHub 100 vs 30, GitLab 100 vs 20, Bitbucket 50 vs 10, Gitea 50 vs 30); whichever enum is chosen should not hard-code `default_per_page` as a provider fact.
