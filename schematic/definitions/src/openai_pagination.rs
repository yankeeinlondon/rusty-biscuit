//! Hand-written pagination overlay for the importer-generated OpenAI definition.
//!
//! The OpenAPI importer emits no pagination, so `define_openai_api()` passes its
//! generated [`RestApi`] through [`apply`] via the importer's generic overlay hook
//! (`schematic-gen import --overlay crate::openai_pagination::apply`). `just
//! import-openai` regenerates `openai/` but never writes this file.
//!
//! Every OpenAI list replies with a wrapper whose items are at `data`, so each
//! list, paginated or not, has an exception row; none inherits by being a root
//! array. A row without its own strategy inherits the API default.
//!
//! The API-default page size has no maximum: OpenAI's `limit` bounds differ per
//! endpoint, and some depend on `bucket_width`. A row authors a maximum only
//! where the vendored `specs/openai/openapi.yaml` states one that holds for every
//! request. Where the provider documents a minimum of 0, the minimum stays at 1,
//! the lowest the pagination model accepts.

use schematic_define::RestApi;
use schematic_define::pagination::{
    Collection, CursorSource, MoreSignal, PageSize, PaginationException, Strategy,
};

/// Sets the OpenAI default strategy and one exception row per list endpoint.
#[must_use]
pub fn apply(mut api: RestApi) -> RestApi {
    api.default_pagination = Some(last_id_cursor(limit()));
    api.pagination_exceptions = exceptions();
    api
}

fn limit() -> PageSize {
    PageSize::new("limit")
}

/// `after` takes the reply's `last_id`; the reply's `has_more` ends the walk.
fn last_id_cursor(size: PageSize) -> Strategy {
    Strategy::cursor("after", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(size)
        .resumable()
        .build()
}

/// [`last_id_cursor`] plus a backward leg: `before` takes the reply's `first_id`.
fn bidirectional_cursor(size: PageSize) -> Strategy {
    Strategy::cursor("after", CursorSource::body("last_id"))
        .backward("before", CursorSource::body("first_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(size)
        .resumable()
        .build()
}

/// The reply carries no `last_id`; `after` is the last item's `id`.
fn last_item_cursor(size: PageSize) -> Strategy {
    Strategy::cursor("after", CursorSource::last_item("id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(size)
        .resumable()
        .build()
}

/// `after` takes the reply's opaque `next` cursor.
fn next_cursor(size: PageSize) -> Strategy {
    Strategy::cursor("after", CursorSource::body("next"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(size)
        .resumable()
        .build()
}

/// The usage endpoints: a cursor sent as `page` from the reply's `next_page`.
fn next_page_cursor(size: PageSize) -> Strategy {
    Strategy::cursor("page", CursorSource::body("next_page"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(size)
        .resumable()
        .build()
}

fn inherit(endpoint: &str, item_type: &str) -> PaginationException {
    PaginationException::new(endpoint, Collection::at("data").item_type(item_type))
}

fn paged(endpoint: &str, item_type: &str, strategy: Strategy) -> PaginationException {
    PaginationException::new(
        endpoint,
        Collection::at("data").item_type(item_type).paginate(strategy),
    )
}

fn unpaged(endpoint: &str, item_type: &str) -> PaginationException {
    PaginationException::new(endpoint, Collection::at("data").item_type(item_type).unpaginated())
}

fn exceptions() -> Vec<PaginationException> {
    // "Limit can range between 1 and 100, and the default is 20."
    let one_to_hundred = || limit().max(100).server_default(20);
    // Schema `minimum: 0, maximum: 100`; the default is stated where known.
    let up_to_hundred = || limit().max(100);
    let up_to_thousand = || limit().max(1000);

    vec![
        // The API default, with a page size the document leaves unbounded.
        inherit("ListChatCompletions", "CreateChatCompletionResponse"),
        inherit("GetChatCompletionMessages", "serde_json::Value"),
        inherit("ListEvals", "Eval"),
        inherit("GetEvalRuns", "EvalRun"),
        inherit("GetEvalRunOutputItems", "EvalRunOutputItem"),
        inherit("ListFineTuningCheckpointPermissions", "FineTuningCheckpointPermission"),
        inherit("ListFineTuningJobCheckpoints", "FineTuningJobCheckpoint"),
        inherit("AdminApiKeysList", "AdminApiKey"),
        // The API default's mechanism with the documented page-size bounds.
        paged("ListVoiceConsents", "VoiceConsentResource", last_id_cursor(one_to_hundred())),
        paged("ListBatches", "Batch", last_id_cursor(one_to_hundred())),
        paged("ListContainers", "ContainerResource", last_id_cursor(one_to_hundred())),
        paged("ListContainerFiles", "ContainerFileResource", last_id_cursor(one_to_hundred())),
        paged("ListConversationItems", "ConversationItem", last_id_cursor(one_to_hundred())),
        paged(
            "ListOrganizationCertificates",
            "OrganizationCertificate",
            last_id_cursor(one_to_hundred()),
        ),
        paged("ListInvites", "Invite", last_id_cursor(one_to_hundred())),
        paged("ListProjects", "Project", last_id_cursor(one_to_hundred())),
        paged("ListProjectApiKeys", "ProjectApiKey", last_id_cursor(one_to_hundred())),
        paged(
            "ListProjectCertificates",
            "OrganizationProjectCertificate",
            last_id_cursor(one_to_hundred()),
        ),
        paged(
            "ListProjectServiceAccounts",
            "ProjectServiceAccount",
            last_id_cursor(one_to_hundred()),
        ),
        paged("ListProjectUsers", "ProjectUser", last_id_cursor(one_to_hundred())),
        paged("ListUsers", "User", last_id_cursor(one_to_hundred())),
        paged("ListInputItems", "ItemResource", last_id_cursor(one_to_hundred())),
        paged("BetaListInputItems", "BetaItemResource", last_id_cursor(one_to_hundred())),
        paged(
            "ListFiles",
            "OpenAIFile",
            last_id_cursor(limit().max(10_000).server_default(10_000)),
        ),
        paged("ListVideos", "VideoResource", last_id_cursor(up_to_hundred())),
        paged("ListSkills", "SkillResource", last_id_cursor(up_to_hundred())),
        paged("ListSkillVersions", "SkillVersionResource", last_id_cursor(up_to_hundred())),
        // These declare `before`, so they page backward from `first_id` too.
        paged("ListAuditLogs", "AuditLog", bidirectional_cursor(one_to_hundred())),
        paged(
            "ListProjectRateLimits",
            "ProjectRateLimit",
            bidirectional_cursor(limit().server_default(100)),
        ),
        paged(
            "ListProjectSpendAlerts",
            "ProjectSpendAlert",
            bidirectional_cursor(up_to_hundred().server_default(20)),
        ),
        paged(
            "ListOrganizationSpendAlerts",
            "OrganizationSpendAlert",
            bidirectional_cursor(up_to_hundred().server_default(20)),
        ),
        paged("ListVectorStores", "VectorStoreObject", bidirectional_cursor(one_to_hundred())),
        paged(
            "ListFilesInVectorStoreBatch",
            "VectorStoreFileObject",
            bidirectional_cursor(one_to_hundred()),
        ),
        paged(
            "ListVectorStoreFiles",
            "VectorStoreFileObject",
            bidirectional_cursor(one_to_hundred()),
        ),
        paged(
            "ListThreadItemsMethod",
            "ThreadItem",
            bidirectional_cursor(up_to_hundred().server_default(20)),
        ),
        paged(
            "ListThreadsMethod",
            "ThreadResource",
            bidirectional_cursor(up_to_hundred().server_default(20)),
        ),
        // `after` is documented as the last item's identifier.
        paged(
            "ListFineTuningEvents",
            "FineTuningJobEvent",
            last_item_cursor(limit().server_default(20)),
        ),
        paged(
            "ListPaginatedFineTuningJobs",
            "FineTuningJob",
            last_item_cursor(limit().server_default(20)),
        ),
        // Role and group lists reply with `next` instead of `last_id`.
        paged("ListGroups", "GroupResponse", next_cursor(up_to_thousand().server_default(100))),
        paged("ListGroupUsers", "GroupUser", next_cursor(up_to_thousand().server_default(100))),
        paged("ListGroupRoleAssignments", "AssignedRoleDetails", next_cursor(up_to_thousand())),
        paged("ListUserRoleAssignments", "AssignedRoleDetails", next_cursor(up_to_thousand())),
        paged(
            "ListProjectGroupRoleAssignments",
            "AssignedRoleDetails",
            next_cursor(up_to_thousand()),
        ),
        paged(
            "ListProjectUserRoleAssignments",
            "AssignedRoleDetails",
            next_cursor(up_to_thousand()),
        ),
        paged("ListRoles", "Role", next_cursor(up_to_thousand().server_default(1000))),
        paged("ListProjectRoles", "Role", next_cursor(up_to_thousand().server_default(1000))),
        paged("ListProjectGroups", "ProjectGroup", next_cursor(up_to_hundred().server_default(20))),
        // The size counts time buckets. Only costs states one range; the other
        // usage endpoints' maximum and default depend on `bucket_width`.
        paged(
            "UsageCosts",
            "UsageTimeBucket",
            next_page_cursor(limit().max(180).server_default(7)),
        ),
        paged("UsageAudioSpeeches", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageAudioTranscriptions", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageCodeInterpreterSessions", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageCompletions", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageEmbeddings", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageFileSearchCalls", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageImages", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageModerations", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageVectorStores", "UsageTimeBucket", next_page_cursor(limit())),
        paged("UsageWebSearchCalls", "UsageTimeBucket", next_page_cursor(limit())),
        // The reply has no continuation: only `object` and `data`.
        unpaged("ListModels", "Model"),
        // Provider gap: the reply carries `has_more` and `next_page`, but the
        // document gives the request no input to send `next_page` back.
        unpaged("RetrieveVectorStoreFileContent", "serde_json::Value"),
        unpaged("SearchVectorStore", "VectorStoreSearchResultItem"),
    ]
}
