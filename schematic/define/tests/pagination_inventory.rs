//! Every API family of the pagination inventory, authored as a fixture with an
//! API default and endpoint collections, with its resolved strategies and
//! caller knobs asserted.
//!
//! Ported from the pagination prototype's `inventory.rs`. These fixtures prove
//! the model expresses each inventoried strategy; they are not the real
//! definitions. Rows the inventory marks unverified (Anthropic, EMQX, Hugging
//! Face) are authored here as mechanisms only: the real definitions leave them
//! unpaginated until verified.

mod pagination_support;

use pagination_support::{api, get, post, registered, resolution, with_query};
use schematic_define::RestApi;
use schematic_define::pagination::{
    Collection, CursorSource, Direction, IndexBase, InputParam, ItemSelector, Knob, LinkSource,
    MoreSignal, PageSize, PageStrategy, PaginationError, Polarity,
    Resolution, ResolutionOrigin, Strategy, ValueSource, caller_knobs, validate_pagination,
};

fn github() -> RestApi {
    let default = Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("per_page").server_default(30).max(100))
        .build();
    let repo = "/repos/{owner}/{repo}";
    let mut github = api("GitHub");
    github.endpoints = vec![
        with_query(get("ListPullRequests", &format!("{repo}/pulls"), "Vec<PullRequest>"), &["state", "page", "per_page"]),
        get("ListPullRequestFiles", &format!("{repo}/pulls/{{n}}/files"), "Vec<PrFile>"),
        get("ListIssues", &format!("{repo}/issues"), "Vec<Issue>"),
        get("ListIssueComments", &format!("{repo}/issues/{{n}}/comments"), "Vec<Comment>"),
        get("ListIssueTimeline", &format!("{repo}/issues/{{n}}/timeline"), "Vec<TimelineEvent>"),
        get("ListTags", &format!("{repo}/tags"), "Vec<Tag>"),
        get("ListReleases", &format!("{repo}/releases"), "Vec<Release>"),
        get("ListOrgRepos", "/orgs/{org}/repos", "Vec<Repo>"),
        get("ListWorkflowRuns", &format!("{repo}/actions/runs"), "WorkflowRunsResponse"),
        get("GetGitTree", &format!("{repo}/git/trees/{{sha}}"), "GitTree"),
    ];
    { let (id, collection) = (
        "ListWorkflowRuns",
        Collection::at("workflow_runs")
            .item_type("WorkflowRun")
            .paginate(default.clone().total(ValueSource::body("total_count"))),
    );  github.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };
    github.default_pagination = Some(default);
    github
}

fn gitlab_keyset() -> Strategy {
    Strategy::next_link(LinkSource::link_header("next"))
        .page_size(PageSize::new("per_page").server_default(20).max(100))
        .constant("pagination", "keyset")
        .constant("order_by", "id")
        .constant("sort", "asc")
}

fn gitlab() -> RestApi {
    let project = "/projects/{id}";
    let mut gitlab = api("GitLab");
    gitlab.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::header_present("X-Next-Page"))
            .page_size(PageSize::new("per_page").server_default(20).max(100))
            .total(ValueSource::header("X-Total")),
    );
    gitlab.endpoints = vec![
        get("ListIssues", &format!("{project}/issues"), "Vec<Issue>"),
        get("ListIssueNotes", &format!("{project}/issues/{{iid}}/notes"), "Vec<Note>"),
        get("ListIssueParticipants", &format!("{project}/issues/{{iid}}/participants"), "Vec<User>"),
        get("ListMergeRequests", &format!("{project}/merge_requests"), "Vec<MergeRequest>"),
        get("ListRepositoryTree", &format!("{project}/repository/tree"), "Vec<TreeEntry>"),
        get("ListGroupProjects", "/groups/{id}/projects", "Vec<Project>"),
        get("ListMergeRequestChanges", &format!("{project}/merge_requests/{{iid}}/changes"), "MergeRequestChanges"),
    ];
    for (id, collection) in [
        // Unverified: left unpaginated.
        ("ListIssueParticipants", Collection::root().unpaginated()),
        ("ListRepositoryTree", Collection::root().or_paginate("keyset", gitlab_keyset())),
        ("ListGroupProjects", Collection::root().or_paginate("keyset", gitlab_keyset())),
    ] { gitlab.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    gitlab
}

fn gitea_default() -> Strategy {
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("limit").server_default(30).max(50))
        .total(ValueSource::header("X-Total-Count"))
}

fn gitea() -> RestApi {
    let repo = "/repos/{owner}/{repo}";
    let tree = || {
        Collection::at("tree").item_type("GitEntry").paginate(
            Strategy::page_number("page", MoreSignal::has_more("truncated"))
                .page_size(PageSize::new("per_page"))
                .total(ValueSource::body("total_count")),
        )
    };
    let mut gitea = api("Gitea");
    gitea.default_pagination = Some(gitea_default());
    gitea.endpoints = vec![
        get("ListPullRequests", &format!("{repo}/pulls"), "Vec<PullRequest>"),
        get("ListIssues", &format!("{repo}/issues"), "Vec<Issue>"),
        get("ListReleases", &format!("{repo}/releases"), "Vec<Release>"),
        get("GetGitTree", &format!("{repo}/git/trees/{{sha}}"), "GitTree"),
        get("GetTagReference", &format!("{repo}/git/refs/{{git_ref}}"), "Vec<GitRef>"),
    ];
    for (id, collection) in [
        ("GetGitTree", tree()),
        // Root array, but a prefix match the server never pages.
        ("GetTagReference", Collection::root().unpaginated()),
    ] { gitea.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    gitea
}

fn bitbucket() -> RestApi {
    let repo = "/repositories/{workspace}/{repo_slug}";
    let mut bitbucket = api("Bitbucket");
    bitbucket.default_pagination = Some(
        Strategy::next_link(LinkSource::body("next"))
            .prev(LinkSource::body("previous"))
            .page_size(PageSize::new("pagelen").server_default(10).max(100))
            .total(ValueSource::body("size")),
    );
    let lists = [
        ("ListPullRequests", format!("{repo}/pullrequests"), "PullRequest"),
        ("ListPullRequestComments", format!("{repo}/pullrequests/{{id}}/comments"), "Comment"),
        ("ListIssues", format!("{repo}/issues"), "Issue"),
        ("ListIssueComments", format!("{repo}/issues/{{id}}/comments"), "Comment"),
        ("ListIssueChanges", format!("{repo}/issues/{{id}}/changes"), "IssueChange"),
        ("ListTags", format!("{repo}/refs/tags"), "Tag"),
        ("ListDownloads", format!("{repo}/downloads"), "Download"),
        ("ListWorkspaceRepos", "/repositories/{workspace}".to_owned(), "Repository"),
    ];
    for (id, path, item) in lists {
        // `page` stays a request field: a next link has no page input.
        bitbucket.endpoints.push(with_query(get(id, &path, "Paginated"), &["page", "pagelen"]));
        { let (id, collection) = (id, Collection::at("values").item_type(item)); bitbucket.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    }
    bitbucket
}

fn anthropic() -> RestApi {
    let mut anthropic = api("Anthropic");
    anthropic.endpoints = vec![get("ListModels", "/models", "ListModelsResponse")];
    { let (id, collection) = (
        "ListModels",
        Collection::at("data").item_type("ModelInfo").paginate(
            Strategy::cursor("after_id", CursorSource::body("last_id"))
                .backward("before_id", CursorSource::body("first_id"))
                .more_when(MoreSignal::has_more("has_more"))
                .page_size(PageSize::new("limit").server_default(20).max(1000))
                .resumable(),
        ),
    );  anthropic.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };
    anthropic
}

fn openai_id_cursor() -> Strategy {
    Strategy::cursor("after", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable()
        .build()
}

fn openai() -> RestApi {
    let s7 = Strategy::cursor("after", CursorSource::body("next"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable();
    let s8 = Strategy::cursor("page", CursorSource::body("next_page"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable();
    let s8_no_limit = Strategy::cursor("page", CursorSource::body("next_page"))
        .more_when(MoreSignal::has_more("has_more"))
        .resumable();
    let events = Strategy::cursor("after", CursorSource::last_item("id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable();
    let bidirectional = Strategy::cursor("after", CursorSource::body("last_id"))
        .backward("before", CursorSource::body("first_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable();

    let mut openai = api("OpenAI");
    openai.default_pagination = Some(openai_id_cursor());
    openai.endpoints = vec![
        with_query(get("ListBatches", "/batches", "ListBatchesResponse"), &["after", "limit"]),
        with_query(get("ListVectorStores", "/vector_stores", "ListVectorStoresResponse"), &["order", "after", "before", "limit"]),
        with_query(get("ListFineTuningEvents", "/fine_tuning/jobs/{id}/events", "ListFineTuningJobEventsResponse"), &["after", "limit"]),
        with_query(get("ListRoles", "/organization/roles", "PublicRoleListResource"), &["order", "after", "limit"]),
        with_query(get("UsageCompletions", "/organization/usage/completions", "UsageResponse"), &["start_time", "page", "limit"]),
        with_query(get("UsageCosts", "/organization/costs", "UsageResponse"), &["start_time", "page"]),
        get("ListModels", "/models", "ListModelsResponse"),
        post("SearchVectorStore", "/vector_stores/{id}/search", "VectorStoreSearchResultsPage"),
    ];
    let data = |item: &str| Collection::at("data").item_type(item);
    for (id, collection) in [
        ("ListBatches", data("Batch")),
        ("ListVectorStores", data("VectorStore").paginate(bidirectional)),
        ("ListFineTuningEvents", data("FineTuningJobEvent").paginate(events)),
        ("ListRoles", data("Role").paginate(s7)),
        ("UsageCompletions", data("UsageBucket").paginate(s8)),
        ("UsageCosts", data("UsageBucket").paginate(s8_no_limit)),
        ("ListModels", data("Model").unpaginated()),
        // Provider gap: no documented request input continues the reply.
        ("SearchVectorStore", data("SearchResult").unpaginated()),
    ] { openai.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    openai
}

fn elevenlabs() -> RestApi {
    let mut elevenlabs = api("ElevenLabs");
    elevenlabs.endpoints = vec![
        get("GetHistory", "/v1/history", "HistoryResponse"),
        get("ListSharedVoices", "/v1/shared-voices", "SharedVoicesResponse"),
        get("ListModels", "/v1/models", "Vec<ModelInfo>"),
        get("ListWebhooks", "/v1/workspace/webhooks", "Webhooks"),
    ];
    for (id, collection) in [
        (
            "GetHistory",
            Collection::at("history").item_type("HistoryItem").paginate(
                Strategy::cursor("start_after_history_item_id", CursorSource::body("last_history_item_id"))
                    .more_when(MoreSignal::has_more("has_more"))
                    .page_size(PageSize::new("page_size").server_default(100).max(1000)),
            ),
        ),
        (
            "ListSharedVoices",
            Collection::at("voices").item_type("SharedVoice").paginate(
                Strategy::page_number("page", MoreSignal::has_more("has_more"))
                    .zero_based()
                    .page_size(PageSize::new("page_size").server_default(30).max(100)),
            ),
        ),
    ] { elevenlabs.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    elevenlabs
}

/// EMQX basic and bearer share one endpoint set; only auth differs.
fn emqx(name: &str) -> RestApi {
    let mut emqx = api(name);
    emqx.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::has_more("meta.hasnext"))
            .page_size(PageSize::new("limit").server_default(10))
            .total(ValueSource::body("meta.count")),
    );
    for (id, path) in [("ListClients", "/clients"), ("ListSubscriptions", "/subscriptions"), ("ListBanned", "/banned")] {
        emqx.endpoints.push(get(id, path, "Paged"));
        { let (id, collection) = (id, Collection::at("data").item_type("serde_json::Value")); emqx.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    }
    emqx.endpoints.push(get("ListNodes", "/nodes", "Vec<Node>"));
    { let (id, collection) = ("ListNodes", Collection::root().unpaginated()); emqx.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    emqx
}

fn huggingface() -> RestApi {
    let mut hf = api("HuggingFaceHub");
    for (id, path) in [("ListModels", "/models"), ("ListDatasets", "/datasets")] {
        hf.endpoints.push(with_query(get(id, path, "Vec<Repo>"), &["search", "limit"]));
        { let (id, collection) = (
            id,
            Collection::root().paginate(Strategy::next_link(LinkSource::link_header("next")).page_size(PageSize::new("limit"))),
        ); hf.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    }
    hf.endpoints.push(get("ListModelCommits", "/models/{+repo_id}/commits/{revision}", "Vec<Commit>"));
    { let (id, collection) = (
        "ListModelCommits",
        Collection::root().paginate(Strategy::page_number("p", MoreSignal::NonEmptyPage).zero_based().page_size(PageSize::new("limit"))),
    ); hf.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    hf.endpoints.push(get("ListModelDiscussions", "/models/{+repo_id}/discussions", "Discussions"));
    { let (id, collection) = (
        "ListModelDiscussions",
        Collection::at("discussions")
            .item_type("Discussion")
            .paginate(Strategy::page_number("p", MoreSignal::total_items(ValueSource::body("count"))).zero_based()),
    ); hf.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection); };
    hf
}

fn unpaginated_apis() -> Vec<RestApi> {
    let single = |name: &str, endpoints| {
        let mut fixture = api(name);
        fixture.endpoints = endpoints;
        fixture
    };
    vec![
        single("ArtificialAnalysis", vec![get("ListLlmModels", "/data/llms/models", "LlmModels")]),
        single("LmStudio", vec![get("ListModels", "/api/v1/models", "Models")]),
        single("Ollama", vec![get("ListModels", "/api/tags", "Tags")]),
        single("Eversolo", vec![get("GetInputAndOutputList", "/ZidooMusicControl/v2/getInputAndOutputList", "InputOutput")]),
        // A write that returns an array is not a list without an explicit collection.
        single("UnfoldedCircle", vec![post("UploadResource", "/resources/{type}", "Vec<ResourceItem>")]),
    ]
}

fn all_apis() -> Vec<RestApi> {
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

fn paginated(api: &RestApi, id: &str) -> (Strategy, ResolutionOrigin, ItemSelector) {
    match resolution(api, id) {
        Resolution::Paginated { strategy, origin, list, .. } => (strategy, origin, list.items),
        other => panic!("{id}: expected paginated, got {other:?}"),
    }
}

fn knobs(api: &RestApi, id: &str) -> Vec<Knob> {
    caller_knobs(&resolution(api, id))
}

fn size_knob(param: &str, server_default: Option<u32>, max: Option<u32>) -> Knob {
    Knob::PageSize { param: param.to_owned(), min: 1, max, server_default }
}

// ---------------------------------------------------------------------------
// Whole-inventory properties
// ---------------------------------------------------------------------------

#[test]
fn every_inventory_fixture_is_valid() {
    for api in all_apis() {
        assert_eq!(validate_pagination(&registered(api.clone())), vec![], "{}", api.name);
    }
}

#[test]
fn every_authored_strategy_and_collection_round_trips_through_serde() {
    for api in all_apis() {
        if let Some(default) = &api.default_pagination {
            let json = serde_json::to_string(default).unwrap();
            assert_eq!(&serde_json::from_str::<Strategy>(&json).unwrap(), default, "{}", api.name);
        }
        let collections: Vec<_> = api.endpoints.iter().map(|endpoint| endpoint.pagination.clone()).collect();
        let json = serde_json::to_string(&collections).unwrap();
        let back: Vec<Option<Collection>> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, collections, "{}", api.name);
    }
}

#[test]
fn unpaginated_endpoints_expose_no_knobs_and_paginated_ones_expose_next() {
    for api in all_apis() {
        for endpoint in &api.endpoints {
            let resolved = resolution(&api, &endpoint.id);
            let knobs = caller_knobs(&resolved);
            if matches!(resolved, Resolution::Paginated { .. }) {
                assert!(knobs.contains(&Knob::Next), "{}::{}", api.name, endpoint.id);
            } else {
                assert_eq!(knobs, vec![], "{}::{}", api.name, endpoint.id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per API
// ---------------------------------------------------------------------------

#[test]
fn github_inherits_page_number_with_link_signal() {
    let api = github();
    let (strategy, origin, items) = paginated(&api, "ListIssues");
    assert_eq!(origin, ResolutionOrigin::ApiDefault);
    assert!(items.is_root_array());
    assert!(matches!(
        &strategy.kind,
        PageStrategy::PageNumber(kind) if kind.base == IndexBase::OneBased
            && kind.more == MoreSignal::link_rel("next")
    ));
    assert_eq!(
        knobs(&api, "ListIssues"),
        vec![Knob::StartPage { base: IndexBase::OneBased }, size_knob("per_page", Some(30), Some(100)), Knob::Next]
    );
}

#[test]
fn github_workflow_runs_overrides_to_add_total_at_wrapper() {
    let api = github();
    let (strategy, origin, items) = paginated(&api, "ListWorkflowRuns");
    assert_eq!(origin, ResolutionOrigin::Exception);
    assert_eq!(items, ItemSelector::parse("workflow_runs"));
    assert_eq!(strategy.total, Some(ValueSource::body("total_count")));
    assert!(knobs(&api, "ListWorkflowRuns").contains(&Knob::Total));
    assert_eq!(resolution(&api, "GetGitTree"), Resolution::NotAList);
}

#[test]
fn gitlab_page_number_default_with_keyset_alternate_modeled_only() {
    let api = gitlab();
    let (strategy, _, _) = paginated(&api, "ListIssues");
    assert!(matches!(
        &strategy.kind,
        PageStrategy::PageNumber(kind) if kind.more == MoreSignal::header_present("X-Next-Page")
    ));
    assert!(matches!(resolution(&api, "ListIssueParticipants"), Resolution::Unpaginated { origin: ResolutionOrigin::Exception, .. }));

    let Resolution::Paginated { alternates, origin, .. } = resolution(&api, "ListGroupProjects") else {
        panic!("paginated expected")
    };
    assert_eq!(origin, ResolutionOrigin::ApiDefault, "the primary stays the API default");
    assert_eq!(alternates.len(), 1);
    assert_eq!(alternates[0].label, "keyset");
    assert!(matches!(alternates[0].strategy.kind, PageStrategy::NextLink(_)));
    let constants: Vec<_> = alternates[0].strategy.constants.iter().map(|c| (c.param.display_name(), c.value.as_str())).collect();
    assert_eq!(constants, vec![("pagination".to_owned(), "keyset"), ("order_by".to_owned(), "id"), ("sort".to_owned(), "asc")]);
    assert_eq!(
        knobs(&api, "ListGroupProjects"),
        vec![Knob::StartPage { base: IndexBase::OneBased }, size_knob("per_page", Some(20), Some(100)), Knob::Next, Knob::Total],
        "an alternate adds no knob"
    );
    assert_eq!(resolution(&api, "ListMergeRequestChanges"), Resolution::NotAList);
}

#[test]
fn gitea_tree_overrides_page_size_name_and_signal() {
    let api = gitea();
    let list = paginated(&api, "ListIssues").0;
    assert_eq!(list.kind.size().unwrap().param, InputParam::query("limit"));
    assert_eq!(list.kind.size().unwrap().max, Some(50));
    let tree = paginated(&api, "GetGitTree").0;
    assert_eq!(tree.kind.size().unwrap().param, InputParam::query("per_page"));
    assert!(matches!(
        &tree.kind,
        PageStrategy::PageNumber(kind) if matches!(kind.more, MoreSignal::Flag { polarity: Polarity::TrueMeansMore, .. })
    ));
    assert!(matches!(resolution(&api, "GetTagReference"), Resolution::Unpaginated { origin: ResolutionOrigin::Exception, .. }));
    let derived = gitea_default().with_page_size(PageSize::new("per_page"));
    assert_eq!(derived.kind.size().unwrap().param, InputParam::query("per_page"));
}

#[test]
fn bitbucket_follows_body_next_and_hides_no_page_number() {
    let api = bitbucket();
    assert_eq!(api.endpoints.len(), 8);
    for endpoint in &api.endpoints {
        let (strategy, origin, items) = paginated(&api, &endpoint.id);
        assert_eq!(origin, ResolutionOrigin::ApiDefault);
        assert_eq!(items, ItemSelector::parse("values"));
        assert!(matches!(strategy.kind, PageStrategy::NextLink(_)));
    }
    let surface = knobs(&api, "ListPullRequests");
    assert_eq!(surface, vec![size_knob("pagelen", Some(10), Some(100)), Knob::Next, Knob::Prev, Knob::Total]);
    let adopted = schematic_define::pagination::adopted_params(&api.endpoints[0], &paginated(&api, "ListPullRequests").0);
    assert_eq!(adopted.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["pagelen"], "page is kept");
}

#[test]
fn anthropic_bidirectional_id_cursor_mechanism() {
    let api = anthropic();
    let (strategy, origin, items) = paginated(&api, "ListModels");
    assert_eq!(origin, ResolutionOrigin::Exception);
    assert_eq!(items, ItemSelector::parse("data"));
    let PageStrategy::Cursor(cursor) = &strategy.kind else { panic!("cursor expected") };
    assert_eq!(cursor.forward.parts[0].param, InputParam::query("after_id"));
    assert_eq!(cursor.backward.as_ref().unwrap().parts[0].source, CursorSource::body("first_id"));
    assert_eq!(
        knobs(&api, "ListModels"),
        vec![
            Knob::ResumeCursor { direction: Direction::Forward, parts: 1 },
            Knob::ResumeCursor { direction: Direction::Backward, parts: 1 },
            size_knob("limit", Some(20), Some(1000)),
            Knob::Next,
            Knob::Prev,
        ]
    );
}

#[test]
fn openai_strategies_by_override_and_adopted_parameters() {
    let api = openai();
    let (default, origin, _) = paginated(&api, "ListBatches");
    assert_eq!(origin, ResolutionOrigin::ApiDefault);
    assert!(matches!(&default.kind, PageStrategy::Cursor(c) if c.forward.parts[0].source == CursorSource::body("last_id")));

    let events = paginated(&api, "ListFineTuningEvents").0;
    assert!(matches!(&events.kind, PageStrategy::Cursor(c) if c.forward.parts[0].source == CursorSource::last_item("id")));
    let roles = paginated(&api, "ListRoles").0;
    assert!(matches!(&roles.kind, PageStrategy::Cursor(c) if c.forward.parts[0].source == CursorSource::body("next")));
    let usage = paginated(&api, "UsageCompletions").0;
    assert!(matches!(&usage.kind, PageStrategy::Cursor(c) if c.forward.parts[0].param == InputParam::query("page")));
    assert_eq!(
        knobs(&api, "UsageCosts"),
        vec![Knob::ResumeCursor { direction: Direction::Forward, parts: 1 }, Knob::Next],
        "no limit declared, so no page-size knob"
    );

    assert!(knobs(&api, "ListVectorStores").contains(&Knob::Prev));
    assert!(!knobs(&api, "ListBatches").contains(&Knob::Prev));
    for id in ["ListModels", "SearchVectorStore"] {
        assert!(matches!(resolution(&api, id), Resolution::Unpaginated { .. }), "{id}");
    }
}

#[test]
fn openai_cursor_without_resume_cannot_adopt_after() {
    let mut fixture = openai();
    let s7_not_resumable = Strategy::cursor("after", CursorSource::body("next")).more_when(MoreSignal::has_more("has_more"));
    fixture.endpoints.iter_mut().find(|endpoint| endpoint.id == "ListRoles").unwrap().pagination = Some(Collection::at("data").item_type("Role").paginate(s7_not_resumable));
    let errors: Vec<PaginationError> = validate_pagination(&registered(fixture.clone())).into_iter().map(|d| d.error).collect();
    assert_eq!(errors, vec![PaginationError::AdoptionWithoutKnob("after".into())]);
}

#[test]
fn elevenlabs_two_strategies_and_no_default() {
    let api = elevenlabs();
    assert!(api.default_pagination.is_none());
    assert!(matches!(paginated(&api, "GetHistory").0.kind, PageStrategy::Cursor(_)));
    let shared = paginated(&api, "ListSharedVoices").0;
    assert!(matches!(&shared.kind, PageStrategy::PageNumber(kind) if kind.base == IndexBase::ZeroBased));
    assert_eq!(knobs(&api, "ListSharedVoices")[0], Knob::StartPage { base: IndexBase::ZeroBased });
    assert!(matches!(resolution(&api, "ListModels"), Resolution::Unpaginated { .. }));
    assert_eq!(resolution(&api, "ListWebhooks"), Resolution::NotAList);
}

#[test]
fn emqx_nested_meta_signal_mechanism_shared_by_both_auth_variants() {
    let basic = emqx("EmqxBasic");
    let bearer = emqx("EmqxBearer");
    assert_eq!(basic.endpoints, bearer.endpoints);
    assert_eq!(basic.default_pagination, bearer.default_pagination);
    let strategy = paginated(&basic, "ListClients").0;
    let PageStrategy::PageNumber(kind) = &strategy.kind else { panic!("page number expected") };
    let MoreSignal::Flag { source: ValueSource::Body(path), polarity } = &kind.more else {
        panic!("nested flag expected")
    };
    assert_eq!(path.as_segments(), ["meta", "hasnext"]);
    assert_eq!(*polarity, Polarity::TrueMeansMore);
    assert!(matches!(resolution(&basic, "ListNodes"), Resolution::Unpaginated { .. }));
}

#[test]
fn huggingface_link_cursor_and_zero_based_p_mechanisms() {
    let api = huggingface();
    assert_eq!(knobs(&api, "ListModels"), vec![size_knob("limit", None, None), Knob::Next]);
    let commits = paginated(&api, "ListModelCommits").0;
    assert!(matches!(
        &commits.kind,
        PageStrategy::PageNumber(kind) if kind.base == IndexBase::ZeroBased && kind.more == MoreSignal::NonEmptyPage
    ));
    assert_eq!(
        knobs(&api, "ListModelDiscussions"),
        vec![Knob::StartPage { base: IndexBase::ZeroBased }, Knob::Next, Knob::Total]
    );
}

#[test]
fn local_and_device_apis_are_unpaginated_or_not_lists() {
    for api in unpaginated_apis() {
        assert!(api.default_pagination.is_none(), "{}", api.name);
        for endpoint in &api.endpoints {
            assert_eq!(resolution(&api, &endpoint.id), Resolution::NotAList, "{}::{}", api.name, endpoint.id);
        }
    }
}
