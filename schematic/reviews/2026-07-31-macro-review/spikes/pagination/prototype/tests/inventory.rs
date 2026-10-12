//! Authors every API family in `inventory.md` and asserts the resolved
//! pagination and the caller-visible surface.

use pagination_prototype::*;

// ---------------------------------------------------------------------------
// Definitions, written the way schematic-definitions would write them
// ---------------------------------------------------------------------------

fn github() -> ApiDef {
    // S1: page + per_page, continuation = Link rel="next".
    let default = Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("per_page").server_default(30).max(100));
    let repo = "/repos/{owner}/{repo}";
    ApiDef::new("GitHub", "https://api.github.com")
        .default_pagination(default)
        .endpoint(EndpointDef::get("ListPullRequests", &format!("{repo}/pulls"), "Vec<PullRequest>").query(&["state"]).collection(Collection::root()))
        .endpoint(EndpointDef::get("ListPullRequestFiles", &format!("{repo}/pulls/{{n}}/files"), "Vec<PrFile>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssues", &format!("{repo}/issues"), "Vec<Issue>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssueComments", &format!("{repo}/issues/{{n}}/comments"), "Vec<Comment>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssueTimeline", &format!("{repo}/issues/{{n}}/timeline"), "Vec<TimelineEvent>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListTags", &format!("{repo}/tags"), "Vec<Tag>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListReleases", &format!("{repo}/releases"), "Vec<Release>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListOrgRepos", "/orgs/{org}/repos", "Vec<Repo>").collection(Collection::root()))
        // Wrapper with a body total: inherit the default, add total().
        .endpoint(
            EndpointDef::get("ListWorkflowRuns", &format!("{repo}/actions/runs"), "WorkflowRuns").collection(
                Collection::at("workflow_runs").paginate(
                    Strategy::page_number("page", MoreSignal::link_rel("next"))
                        .page_size(PageSize::new("per_page").server_default(30).max(100))
                        .total(ValueSource::body("total_count")),
                ),
            ),
        )
        // Not a collection: no pagination possible.
        .endpoint(EndpointDef::get("GetGitTree", &format!("{repo}/git/trees/{{sha}}"), "GitTree"))
}

fn gitlab_keyset() -> Strategy {
    Strategy::next_link(LinkSource::link_header("next"))
        .page_size(PageSize::new("per_page").server_default(20).max(100))
        .constant("pagination", "keyset")
        .constant("order_by", "id")
        .constant("sort", "asc")
}

fn gitlab() -> ApiDef {
    // S2: page + per_page; X-Next-Page is empty on the last page; X-Total is
    // omitted above 10,000 rows, so it is exposed but not the end signal.
    let default = Strategy::page_number("page", MoreSignal::header_present("X-Next-Page"))
        .page_size(PageSize::new("per_page").server_default(20).max(100))
        .total(ValueSource::header("X-Total"));
    let project = "/projects/{id}";
    ApiDef::new("GitLab", "https://gitlab.com/api/v4")
        .default_pagination(default)
        .endpoint(EndpointDef::get("ListIssues", &format!("{project}/issues"), "Vec<Issue>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssueNotes", &format!("{project}/issues/{{iid}}/notes"), "Vec<Note>").collection(Collection::root()))
        // Inventory gap: inherits the default by being declared a collection.
        .endpoint(EndpointDef::get("ListIssueParticipants", &format!("{project}/issues/{{iid}}/participants"), "Vec<User>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListMergeRequests", &format!("{project}/merge_requests"), "Vec<MergeRequest>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListMergeRequestCommits", &format!("{project}/merge_requests/{{iid}}/commits"), "Vec<Commit>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListProjectPipelines", &format!("{project}/pipelines"), "Vec<Pipeline>").collection(Collection::root()))
        // S3: offset by default, keyset selectable.
        .endpoint(EndpointDef::get("ListRepositoryTree", &format!("{project}/repository/tree"), "Vec<TreeEntry>").collection(Collection::root().or_paginate("keyset", gitlab_keyset())))
        .endpoint(EndpointDef::get("ListGroupProjects", "/groups/{id}/projects", "Vec<Project>").collection(Collection::root().or_paginate("keyset", gitlab_keyset())))
        .endpoint(EndpointDef::get("ListTags", &format!("{project}/repository/tags"), "Vec<Tag>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListReleases", &format!("{project}/releases"), "Vec<Release>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListMergeRequestChanges", &format!("{project}/merge_requests/{{iid}}/changes"), "MergeRequestChanges"))
}

fn gitea_default() -> Strategy {
    // S4: page + limit; Link rel next; X-Total-Count. Server cap is 50.
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("limit").server_default(30).max(50))
        .total(ValueSource::header("X-Total-Count"))
}

fn gitea() -> ApiDef {
    let repo = "/repos/{owner}/{repo}";
    let tree = || {
        // S4b: same API, different page-size name and a body signal.
        Collection::at("tree").paginate(
            Strategy::page_number("page", MoreSignal::has_more("truncated"))
                .page_size(PageSize::new("per_page"))
                .total(ValueSource::body("total_count")),
        )
    };
    ApiDef::new("Gitea", "https://gitea.com/api/v1")
        .default_pagination(gitea_default())
        .endpoint(EndpointDef::get("ListPullRequests", &format!("{repo}/pulls"), "Vec<PullRequest>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListPullRequestFiles", &format!("{repo}/pulls/{{index}}/files"), "Vec<PrFile>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssues", &format!("{repo}/issues"), "Vec<Issue>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssueComments", &format!("{repo}/issues/{{index}}/comments"), "Vec<Comment>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListIssueTimeline", &format!("{repo}/issues/{{index}}/timeline"), "Vec<TimelineEvent>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListTags", &format!("{repo}/tags"), "Vec<Tag>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListReleases", &format!("{repo}/releases"), "Vec<Release>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListOrgRepos", "/orgs/{org}/repos", "Vec<Repo>").collection(Collection::root()))
        .endpoint(EndpointDef::get("GetGitTree", &format!("{repo}/git/trees/{{sha}}"), "GitTree").collection(tree()))
        .endpoint(EndpointDef::get("GetGitTreeRecursive", &format!("{repo}/git/trees/{{sha}}?recursive=true"), "GitTree").collection(tree()))
        // Root array, but a prefix match the server never pages: opt out.
        .endpoint(EndpointDef::get("GetTagReference", &format!("{repo}/git/refs/{{git_ref}}"), "Vec<GitRef>").collection(Collection::root().unpaginated()))
}

fn bitbucket() -> ApiDef {
    // S5: follow body `next`; `pagelen` is the only knob; `size` is a total.
    let default = Strategy::next_link(LinkSource::body("next"))
        .prev(LinkSource::body("previous"))
        .page_size(PageSize::new("pagelen").server_default(10).max(100))
        .total(ValueSource::body("size"));
    let repo = "/repositories/{workspace}/{repo_slug}";
    let mut api = ApiDef::new("Bitbucket", "https://api.bitbucket.org/2.0").default_pagination(default);
    for (id, path) in [
        ("ListDirectoryContents", format!("{repo}/src/{{commit}}/{{path}}")),
        ("ListPullRequests", format!("{repo}/pullrequests")),
        ("ListPullRequestComments", format!("{repo}/pullrequests/{{id}}/comments")),
        ("ListIssues", format!("{repo}/issues")),
        ("ListIssueComments", format!("{repo}/issues/{{id}}/comments")),
        ("ListIssueChanges", format!("{repo}/issues/{{id}}/changes")),
        ("ListTags", format!("{repo}/refs/tags")),
        ("ListDownloads", format!("{repo}/downloads")),
        ("ListWorkspaceRepos", "/repositories/{workspace}".to_owned()),
    ] {
        api = api.endpoint(EndpointDef::get(id, &path, "PaginatedResponse<T>").collection(Collection::at("values")));
    }
    api
}

fn anthropic() -> ApiDef {
    // S6 (Anthropic spelling): after_id <- last_id, before_id <- first_id.
    let models = Strategy::cursor("after_id", CursorSource::body("last_id"))
        .backward("before_id", CursorSource::body("first_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit").server_default(20).max(1000))
        .resumable();
    ApiDef::new("Anthropic", "https://api.anthropic.com/v1")
        .endpoint(EndpointDef::get("ListModels", "/models", "ListModelsResponse").collection(Collection::at("data").paginate(models)))
}

fn openai_id_cursor() -> CursorBuilder {
    Strategy::cursor("after", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit").server_default(20).max(100))
        .resumable()
}

fn openai() -> ApiDef {
    // API default = S6, the 38-endpoint majority.
    let s7 = Strategy::cursor("after", CursorSource::body("next"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit").server_default(20).max(100));
    let s8 = Strategy::cursor("page", CursorSource::body("next_page"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"));
    let s8_no_limit = Strategy::cursor("page", CursorSource::body("next_page")).more_when(MoreSignal::has_more("has_more"));
    let events = Strategy::cursor("after", CursorSource::last_item("id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit").server_default(20).max(100));
    let bidirectional = openai_id_cursor().backward("before", CursorSource::body("first_id"));

    ApiDef::new("OpenAI", "https://api.openai.com/v1")
        .default_pagination(openai_id_cursor())
        .endpoint(EndpointDef::get("ListBatches", "/batches", "ListBatchesResponse").collection(Collection::at("data")))
        .endpoint(EndpointDef::get("ListChatCompletions", "/chat/completions", "ChatCompletionList").query(&["model", "metadata", "order"]).collection(Collection::at("data")))
        .endpoint(EndpointDef::get("ListVectorStores", "/vector_stores", "ListVectorStoresResponse").query(&["order"]).collection(Collection::at("data").paginate(bidirectional)))
        .endpoint(EndpointDef::get("ListFineTuningEvents", "/fine_tuning/jobs/{id}/events", "ListFineTuningJobEventsResponse").collection(Collection::at("data").paginate(events)))
        .endpoint(EndpointDef::get("ListRoles", "/organization/roles", "PublicRoleListResource").query(&["order"]).collection(Collection::at("data").paginate(s7)))
        .endpoint(EndpointDef::get("UsageCompletions", "/organization/usage/completions", "UsageResponse").query(&["start_time", "end_time", "bucket_width"]).collection(Collection::at("data").paginate(s8)))
        .endpoint(EndpointDef::get("UsageCosts", "/organization/costs", "UsageResponse").query(&["start_time", "end_time"]).collection(Collection::at("data").paginate(s8_no_limit)))
        .endpoint(EndpointDef::get("ListModels", "/models", "ListModelsResponse").collection(Collection::at("data").unpaginated()))
        // Reply carries has_more/next_page, but no documented request input
        // continues it. Not expressible; recorded as unpaginated.
        .endpoint(EndpointDef::post("SearchVectorStore", "/vector_stores/{id}/search", "VectorStoreSearchResultsPage").collection(Collection::at("data").unpaginated()))
        .endpoint(EndpointDef::get("RetrieveVectorStoreFileContent", "/vector_stores/{id}/files/{file_id}/content", "VectorStoreFileContentResponse").collection(Collection::at("data").unpaginated()))
}

fn elevenlabs() -> ApiDef {
    // Three strategies in one API, no API default.
    let history = Strategy::cursor("start_after_history_item_id", CursorSource::body("last_history_item_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("page_size").server_default(100).max(1000));
    let voices = Strategy::cursor("next_page_token", CursorSource::body("next_page_token"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("page_size").server_default(10).max(100))
        .total(ValueSource::body("total_count"));
    let shared = Strategy::page_number("page", MoreSignal::has_more("has_more"))
        .zero_based()
        .page_size(PageSize::new("page_size").server_default(30).max(100));
    ApiDef::new("ElevenLabs", "https://api.elevenlabs.io")
        .endpoint(EndpointDef::get("GetHistory", "/v1/history", "HistoryResponse").collection(Collection::at("history").paginate(history)))
        .endpoint(EndpointDef::get("ListVoices", "/v2/voices", "VoicesResponse").query(&["include_total_count"]).collection(Collection::at("voices").paginate(voices)))
        .endpoint(EndpointDef::get("ListSharedVoices", "/v1/shared-voices", "SharedVoicesResponse").collection(Collection::at("voices").paginate(shared)))
        .endpoint(EndpointDef::get("ListModels", "/v1/models", "Vec<ModelInfo>").collection(Collection::root()))
        .endpoint(EndpointDef::get("ListServiceAccounts", "/v1/service-accounts", "ServiceAccounts").collection(Collection::at("service_accounts")))
        .endpoint(EndpointDef::get("ListWebhooks", "/v1/workspace/webhooks", "Webhooks").collection(Collection::at("webhooks")))
}

/// EMQX basic and bearer share one endpoint set; only auth differs.
fn emqx(name: &str) -> ApiDef {
    let default = Strategy::page_number("page", MoreSignal::has_more("meta.hasnext"))
        .page_size(PageSize::new("limit").server_default(10))
        .total(ValueSource::body("meta.count"));
    let mut api = ApiDef::new(name, "http://localhost:18083/api/v5").default_pagination(default);
    for (id, path) in [
        ("ListClients", "/clients"),
        ("ListSubscriptions", "/subscriptions"),
        ("ListTopics", "/topics"),
        ("ListRetained", "/retainer/messages"),
        ("ListBanned", "/banned"),
        ("ListAuthUsers", "/authentication/{id}/users"),
        ("ListRules", "/rules"),
        ("ListAlarms", "/alarms"),
    ] {
        api = api.endpoint(EndpointDef::get(id, path, "Paged<T>").collection(Collection::at("data")));
    }
    for (id, path) in [("ListNodes", "/nodes"), ("ListListeners", "/listeners"), ("ListAuthenticators", "/authentication")] {
        api = api.endpoint(EndpointDef::get(id, path, "Vec<T>").collection(Collection::root().unpaginated()));
    }
    api.endpoint(EndpointDef::get("ListAuthzSources", "/authorization/sources", "Sources").collection(Collection::at("sources").unpaginated()))
}

fn huggingface() -> ApiDef {
    // S13: follow Link rel="next"; only `limit` is caller-visible.
    let link_cursor = || Strategy::next_link(LinkSource::link_header("next")).page_size(PageSize::new("limit"));
    // S14: 0-based `p`; commits have no documented end signal.
    let commits = || Strategy::page_number("p", MoreSignal::NonEmptyPage).zero_based().page_size(PageSize::new("limit"));
    let discussions = Strategy::page_number("p", MoreSignal::total_items(ValueSource::body("count"))).zero_based();
    let mut api = ApiDef::new("HuggingFace", "https://huggingface.co/api");
    for (id, path) in [("ListModels", "/models"), ("ListDatasets", "/datasets"), ("ListSpaces", "/spaces"), ("ListModelFiles", "/models/{+repo_id}/tree/{revision}")] {
        api = api.endpoint(EndpointDef::get(id, path, "Vec<T>").query(&["search", "author"]).collection(Collection::root().paginate(link_cursor())));
    }
    api.endpoint(EndpointDef::get("ListModelCommits", "/models/{+repo_id}/commits/{revision}", "Vec<Commit>").collection(Collection::root().paginate(commits())))
        .endpoint(EndpointDef::get("ListModelDiscussions", "/models/{+repo_id}/discussions", "Discussions").collection(Collection::at("discussions").paginate(discussions)))
}

fn unpaginated_apis() -> Vec<ApiDef> {
    vec![
        ApiDef::new("ArtificialAnalysis", "https://artificialanalysis.ai/api/v2")
            .endpoint(EndpointDef::get("ListLlmModels", "/data/llms/models", "LlmModels").collection(Collection::at("data")))
            .endpoint(EndpointDef::get("ListTextToImageModels", "/data/media/text-to-image", "Media").collection(Collection::at("data"))),
        ApiDef::new("CritPt", "https://artificialanalysis.ai/api/v2").endpoint(EndpointDef::post("EvaluateCritPt", "/critpt/evaluate", "CritPtScore")),
        ApiDef::new("LmStudio", "http://localhost:1234").endpoint(EndpointDef::get("ListModels", "/api/v1/models", "Models").collection(Collection::at("data"))),
        ApiDef::new("Ollama", "http://localhost:11434")
            .endpoint(EndpointDef::get("ListModels", "/api/tags", "Tags").collection(Collection::at("models")))
            .endpoint(EndpointDef::get("ListRunningModels", "/api/ps", "Running").collection(Collection::at("models"))),
        ApiDef::new("OllamaOpenAi", "http://localhost:11434/v1").endpoint(EndpointDef::get("ListModels", "/models", "Models").collection(Collection::at("data"))),
        // Device APIs: option lists and write results are not collections.
        ApiDef::new("Eversolo", "http://eversolo.local:9529").endpoint(EndpointDef::get("GetInputAndOutputList", "/ZidooMusicControl/v2/getInputAndOutputList", "InputOutput")),
        ApiDef::new("SamsungSmartTv", "https://tv.local:8002").endpoint(EndpointDef::get("GetDeviceInfo", "/api/v2/", "DeviceInfo")),
        ApiDef::new("UnfoldedCircle", "http://remote.local/api").endpoint(EndpointDef::post("UploadResource", "/resources/{type}", "Vec<ResourceItem>")),
    ]
}

fn all_apis() -> Vec<ApiDef> {
    let mut apis = vec![
        github(),
        gitlab(),
        gitea(),
        bitbucket(),
        anthropic(),
        openai(),
        elevenlabs(),
        emqx("EmqxBasic"),
        emqx("EmqxBearer"),
        huggingface(),
    ];
    apis.extend(unpaginated_apis());
    apis
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn paginated(api: &ApiDef, id: &str) -> (Strategy, Origin, Vec<Alternate>, ItemSelector) {
    match api.resolve(id).unwrap_or_else(|| panic!("{id} not defined")) {
        Resolved::Paginated {
            primary,
            origin,
            alternates,
            items,
        } => (primary, origin, alternates, items),
        other => panic!("{id}: expected paginated, got {other:?}"),
    }
}

fn knobs(api: &ApiDef, id: &str) -> Vec<Knob> {
    caller_knobs(&api.resolve(id).unwrap())
}

fn size_knob(param: &str, server_default: Option<u32>, max: Option<u32>) -> Knob {
    Knob::PageSize {
        param: param.to_owned(),
        min: 1,
        max,
        server_default,
    }
}

// ---------------------------------------------------------------------------
// Whole-inventory properties
// ---------------------------------------------------------------------------

#[test]
fn every_inventory_api_is_valid() {
    let apis = all_apis();
    assert_eq!(apis.len(), 18, "18 REST APIs in inventory.md");
    for api in &apis {
        assert_eq!(validate_api(api), vec![], "{} has invalid pagination", api.name);
    }
}

#[test]
fn every_definition_round_trips_through_serde() {
    for api in all_apis() {
        let json = serde_json::to_string(&api).unwrap();
        let back: ApiDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, api, "{} did not round-trip", api.name);
    }
}

#[test]
fn unpaginated_endpoints_expose_no_knobs() {
    for api in all_apis() {
        for endpoint in &api.endpoints {
            let resolved = api.resolve(&endpoint.id).unwrap();
            if !matches!(resolved, Resolved::Paginated { .. }) {
                assert_eq!(caller_knobs(&resolved), vec![], "{}::{}", api.name, endpoint.id);
            }
        }
    }
}

#[test]
fn every_paginated_endpoint_exposes_next() {
    for api in all_apis() {
        for endpoint in &api.endpoints {
            let resolved = api.resolve(&endpoint.id).unwrap();
            if matches!(resolved, Resolved::Paginated { .. }) {
                assert!(caller_knobs(&resolved).contains(&Knob::Next), "{}::{}", api.name, endpoint.id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-API
// ---------------------------------------------------------------------------

#[test]
fn github_inherits_page_number_with_link_signal() {
    let api = github();
    let (primary, origin, alternates, items) = paginated(&api, "ListIssues");
    assert_eq!(origin, Origin::ApiDefault);
    assert!(alternates.is_empty());
    assert!(items.is_root_array());
    assert!(matches!(
        primary.kind,
        PageStrategy::PageNumber { base: IndexBase::OneBased, more: MoreSignal::LinkRel { ref rel }, .. } if rel == "next"
    ));
    assert_eq!(
        knobs(&api, "ListIssues"),
        vec![Knob::StartPage { base: IndexBase::OneBased }, size_knob("per_page", Some(30), Some(100)), Knob::Next]
    );
}

#[test]
fn github_workflow_runs_overrides_to_add_total_at_wrapper() {
    let api = github();
    let (primary, origin, _, items) = paginated(&api, "ListWorkflowRuns");
    assert_eq!(origin, Origin::Endpoint);
    assert_eq!(items, ItemSelector::parse("workflow_runs"));
    assert_eq!(primary.total, Some(ValueSource::body("total_count")));
    assert!(knobs(&api, "ListWorkflowRuns").contains(&Knob::Total));
    assert_eq!(api.resolve("GetGitTree"), Some(Resolved::NotACollection));
}

#[test]
fn gitlab_offset_default_with_keyset_alternate() {
    let api = gitlab();
    let (primary, _, _, _) = paginated(&api, "ListIssueParticipants");
    assert!(matches!(
        primary.kind,
        PageStrategy::PageNumber { more: MoreSignal::Present { source: ValueSource::Header(ref h) }, .. } if h == "X-Next-Page"
    ));

    let (_, origin, alternates, _) = paginated(&api, "ListGroupProjects");
    assert_eq!(origin, Origin::ApiDefault, "primary stays the API default");
    assert_eq!(alternates.len(), 1);
    let keyset = &alternates[0];
    assert_eq!(keyset.label, "keyset");
    assert!(matches!(keyset.strategy.kind, PageStrategy::NextLink { .. }));
    let constants: Vec<_> = keyset.strategy.constants.iter().map(|c| (c.param.display_name(), c.value.as_str())).collect();
    assert_eq!(
        constants,
        vec![("pagination".to_owned(), "keyset"), ("order_by".to_owned(), "id"), ("sort".to_owned(), "asc")]
    );
    assert_eq!(
        knobs(&api, "ListGroupProjects"),
        vec![
            Knob::StartPage { base: IndexBase::OneBased },
            size_knob("per_page", Some(20), Some(100)),
            Knob::Next,
            Knob::Total,
            Knob::Alternate { label: "keyset".into(), knobs: vec![size_knob("per_page", Some(20), Some(100)), Knob::Next] },
        ]
    );
    assert_eq!(api.resolve("ListMergeRequestChanges"), Some(Resolved::NotACollection));
}

#[test]
fn gitea_tree_overrides_page_size_name_and_signal() {
    let api = gitea();
    let list = paginated(&api, "ListIssues").0;
    assert_eq!(list.kind.size().unwrap().param, InputParam::query("limit"));
    assert_eq!(list.kind.size().unwrap().max, Some(50));
    let tree = paginated(&api, "GetGitTree").0;
    assert_eq!(tree.kind.size().unwrap().param, InputParam::query("per_page"));
    assert!(matches!(tree.kind, PageStrategy::PageNumber { more: MoreSignal::Flag { polarity: Polarity::TrueMeansMore, .. }, .. }));
    assert_eq!(api.resolve("GetTagReference"), Some(Resolved::Unpaginated { items: ItemSelector::root() }));
    // The override helper reaches the same answer from the default.
    let derived = gitea_default().with_page_size(PageSize::new("per_page"));
    assert_eq!(derived.kind.size().unwrap().param, InputParam::query("per_page"));
}

#[test]
fn bitbucket_follows_body_next_and_hides_page_number() {
    let api = bitbucket();
    assert_eq!(api.endpoints.len(), 9);
    for endpoint in &api.endpoints {
        let (primary, origin, _, items) = paginated(&api, &endpoint.id);
        assert_eq!(origin, Origin::ApiDefault);
        assert_eq!(items, ItemSelector::parse("values"));
        assert!(matches!(primary.kind, PageStrategy::NextLink { trust: LinkTrust::SameOrigin, .. }));
    }
    let surface = knobs(&api, "ListPullRequests");
    assert_eq!(surface, vec![size_knob("pagelen", Some(10), Some(100)), Knob::Next, Knob::Prev, Knob::Total]);
    assert!(!surface.iter().any(|knob| matches!(knob, Knob::StartPage { .. })));
}

#[test]
fn anthropic_bidirectional_id_cursor() {
    let api = anthropic();
    let (primary, origin, _, items) = paginated(&api, "ListModels");
    assert_eq!(origin, Origin::Endpoint);
    assert_eq!(items, ItemSelector::parse("data"));
    let PageStrategy::Cursor { forward, backward, more, .. } = &primary.kind else { panic!("cursor expected") };
    assert_eq!(forward.parts[0].param, InputParam::query("after_id"));
    assert_eq!(forward.parts[0].source, CursorSource::body("last_id"));
    assert_eq!(backward.as_ref().unwrap().parts[0].source, CursorSource::body("first_id"));
    assert_eq!(more, &Some(MoreSignal::has_more("has_more")));
    assert_eq!(
        knobs(&api, "ListModels"),
        vec![
            Knob::ResumeCursor { direction: Direction::Forward, parts: 1 },
            Knob::ResumeCursor { direction: Direction::Backward, parts: 1 },
            Knob::PageSize { param: "limit".into(), min: 1, max: Some(1000), server_default: Some(20) },
            Knob::Next,
            Knob::Prev,
        ]
    );
}

#[test]
fn openai_three_strategies_by_override() {
    let api = openai();
    let (default, origin, _, _) = paginated(&api, "ListBatches");
    assert_eq!(origin, Origin::ApiDefault);
    assert!(matches!(&default.kind, PageStrategy::Cursor { forward, .. } if forward.parts[0].source == CursorSource::body("last_id")));

    let events = paginated(&api, "ListFineTuningEvents").0;
    assert!(matches!(&events.kind, PageStrategy::Cursor { forward, .. } if forward.parts[0].source == CursorSource::last_item("id")));

    let roles = paginated(&api, "ListRoles").0;
    assert!(matches!(&roles.kind, PageStrategy::Cursor { forward, resumable: false, .. } if forward.parts[0].source == CursorSource::body("next")));
    assert_eq!(knobs(&api, "ListRoles"), vec![size_knob("limit", Some(20), Some(100)), Knob::Next]);

    let usage = paginated(&api, "UsageCompletions").0;
    assert!(matches!(&usage.kind, PageStrategy::Cursor { forward, .. } if forward.parts[0].param == InputParam::query("page")));
    assert_eq!(knobs(&api, "UsageCosts"), vec![Knob::Next], "no limit declared, so no page-size knob");

    assert!(knobs(&api, "ListVectorStores").contains(&Knob::Prev));
    assert!(!knobs(&api, "ListBatches").contains(&Knob::Prev));
    for id in ["ListModels", "SearchVectorStore", "RetrieveVectorStoreFileContent"] {
        assert!(matches!(api.resolve(id), Some(Resolved::Unpaginated { .. })), "{id}");
    }
}

#[test]
fn openai_plain_after_param_collides_with_strategy() {
    // Today's OpenAI definitions declare `after`/`limit` as ordinary params.
    let api = ApiDef::new("OpenAI", "https://api.openai.com/v1")
        .default_pagination(openai_id_cursor())
        .endpoint(EndpointDef::get("ListFiles", "/files", "ListFilesResponse").query(&["purpose", "limit", "order", "after"]).collection(Collection::at("data")));
    assert_eq!(
        validate_api(&api),
        vec![
            ("ListFiles".to_owned(), PaginationError::CollidesWithEndpointParam("after".into())),
            ("ListFiles".to_owned(), PaginationError::CollidesWithEndpointParam("limit".into())),
        ]
    );
}

#[test]
fn elevenlabs_three_strategies_and_default_unpaginated() {
    let api = elevenlabs();
    assert!(api.default_pagination.is_none());
    assert!(matches!(paginated(&api, "GetHistory").0.kind, PageStrategy::Cursor { .. }));
    assert!(knobs(&api, "ListVoices").contains(&Knob::Total));
    let shared = paginated(&api, "ListSharedVoices").0;
    assert!(matches!(shared.kind, PageStrategy::PageNumber { base: IndexBase::ZeroBased, .. }));
    assert_eq!(knobs(&api, "ListSharedVoices")[0], Knob::StartPage { base: IndexBase::ZeroBased });
    for id in ["ListModels", "ListServiceAccounts", "ListWebhooks"] {
        assert!(matches!(api.resolve(id), Some(Resolved::Unpaginated { .. })), "{id}");
    }
}

#[test]
fn emqx_nested_meta_signal_shared_by_both_auth_variants() {
    let basic = emqx("EmqxBasic");
    let bearer = emqx("EmqxBearer");
    assert_eq!(basic.endpoints, bearer.endpoints);
    assert_eq!(basic.default_pagination, bearer.default_pagination);
    let primary = paginated(&basic, "ListClients").0;
    let PageStrategy::PageNumber { more: MoreSignal::Flag { source: ValueSource::Body(path), polarity }, .. } = &primary.kind else {
        panic!("nested flag expected")
    };
    assert_eq!(path.as_segments(), ["meta", "hasnext"]);
    assert_eq!(*polarity, Polarity::TrueMeansMore);
    assert!(matches!(basic.resolve("ListNodes"), Some(Resolved::Unpaginated { .. })));
    assert_eq!(basic.endpoints.iter().filter(|e| matches!(basic.resolve(&e.id), Some(Resolved::Paginated { .. }))).count(), 8);
}

#[test]
fn huggingface_link_cursor_and_zero_based_p() {
    let api = huggingface();
    assert_eq!(knobs(&api, "ListModels"), vec![size_knob("limit", None, None), Knob::Next]);
    let commits = paginated(&api, "ListModelCommits").0;
    assert!(matches!(commits.kind, PageStrategy::PageNumber { base: IndexBase::ZeroBased, more: MoreSignal::NonEmptyPage, .. }));
    assert_eq!(knobs(&api, "ListModelDiscussions"), vec![Knob::StartPage { base: IndexBase::ZeroBased }, Knob::Next, Knob::Total]);
}

#[test]
fn local_and_device_apis_are_unpaginated_or_not_collections() {
    for api in unpaginated_apis() {
        assert!(api.default_pagination.is_none(), "{}", api.name);
        for endpoint in &api.endpoints {
            let resolved = api.resolve(&endpoint.id).unwrap();
            let expected_collection = endpoint.collection.is_some();
            match resolved {
                Resolved::Unpaginated { .. } => assert!(expected_collection),
                Resolved::NotACollection => assert!(!expected_collection),
                Resolved::Paginated { .. } => panic!("{}::{} paginated", api.name, endpoint.id),
            }
        }
    }
}
