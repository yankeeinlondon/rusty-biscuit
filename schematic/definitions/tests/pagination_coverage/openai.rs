//! OpenAI, paginated through the hand-written overlay.
//!
//! Every list is a wrapper at `data`. Page-size bounds are the ones the vendored
//! `specs/openai/openapi.yaml` states for every request of the endpoint; a
//! provider minimum of 0 is held at the model's minimum of 1.

use schematic_define::pagination::Direction::{Backward, Forward};
use schematic_define::pagination::ResolutionOrigin::{ApiDefault, Exception};
use schematic_define::pagination::{Knob, ResolutionOrigin};

use crate::{Api, Next, Prev, Row, Unpaged, page_size, resume};

const LAST_ID: &str = "cursor: after <- body last_id; more flag body has_more";
const BOTH_IDS: &str =
    "cursor: after <- body last_id; before <- body first_id; more flag body has_more";
const LAST_ITEM: &str = "cursor: after <- last item id; more flag body has_more";
const NEXT: &str = "cursor: after <- body next; more flag body has_more";
const NEXT_PAGE: &str = "cursor: page <- body next_page; more flag body has_more";

fn limit(max: Option<u32>, server_default: Option<u32>) -> Knob {
    page_size("limit", 1, max, server_default)
}

fn forward(
    endpoint: &'static str,
    origin: ResolutionOrigin,
    item_type: &'static str,
    strategy: &'static str,
    size: Knob,
) -> Row {
    Row::paged(
        endpoint,
        origin,
        "data[*]",
        item_type,
        strategy,
        vec![resume(Forward), size, Next],
        &["after", "limit"],
    )
}

/// The API default, inherited: `after <- last_id`, unbounded `limit`.
fn inherited(endpoint: &'static str, item_type: &'static str) -> Row {
    forward(endpoint, ApiDefault, item_type, LAST_ID, limit(None, None))
}

fn last_id(endpoint: &'static str, item_type: &'static str, size: Knob) -> Row {
    forward(endpoint, Exception, item_type, LAST_ID, size)
}

fn last_item(endpoint: &'static str, item_type: &'static str, size: Knob) -> Row {
    forward(endpoint, Exception, item_type, LAST_ITEM, size)
}

fn next(endpoint: &'static str, item_type: &'static str, size: Knob) -> Row {
    forward(endpoint, Exception, item_type, NEXT, size)
}

fn both_ids(endpoint: &'static str, item_type: &'static str, size: Knob) -> Row {
    Row::paged(
        endpoint,
        Exception,
        "data[*]",
        item_type,
        BOTH_IDS,
        vec![resume(Forward), resume(Backward), size, Next, Prev],
        &["after", "before", "limit"],
    )
}

fn usage(endpoint: &'static str, size: Knob) -> Row {
    Row::paged(
        endpoint,
        Exception,
        "data[*]",
        "UsageTimeBucket",
        NEXT_PAGE,
        vec![resume(Forward), size, Next],
        &["page", "limit"],
    )
}

fn opted_out(endpoint: &'static str, why: Unpaged) -> Row {
    Row::unpaged(endpoint, Exception, "data[*]", why)
}

pub fn apis() -> Vec<Api> {
    let one_to_hundred = || limit(Some(100), Some(20));
    let unbounded = || limit(None, None);
    let provider_gap = "the reply carries has_more and next_page, but the request has no \
                        documented input to send next_page back";

    vec![Api {
        name: "OpenAI",
        rows: vec![
            inherited("ListChatCompletions", "CreateChatCompletionResponse"),
            inherited("GetChatCompletionMessages", "serde_json::Value"),
            inherited("ListEvals", "Eval"),
            inherited("GetEvalRuns", "EvalRun"),
            inherited("GetEvalRunOutputItems", "EvalRunOutputItem"),
            inherited("ListFineTuningCheckpointPermissions", "FineTuningCheckpointPermission"),
            inherited("ListFineTuningJobCheckpoints", "FineTuningJobCheckpoint"),
            inherited("AdminApiKeysList", "AdminApiKey"),
            last_id("ListVoiceConsents", "VoiceConsentResource", one_to_hundred()),
            last_id("ListBatches", "Batch", one_to_hundred()),
            last_id("ListContainers", "ContainerResource", one_to_hundred()),
            last_id("ListContainerFiles", "ContainerFileResource", one_to_hundred()),
            last_id("ListConversationItems", "ConversationItem", one_to_hundred()),
            last_id("ListOrganizationCertificates", "OrganizationCertificate", one_to_hundred()),
            last_id("ListInvites", "Invite", one_to_hundred()),
            last_id("ListProjects", "Project", one_to_hundred()),
            last_id("ListProjectApiKeys", "ProjectApiKey", one_to_hundred()),
            last_id("ListProjectCertificates", "OrganizationProjectCertificate", one_to_hundred()),
            last_id("ListProjectServiceAccounts", "ProjectServiceAccount", one_to_hundred()),
            last_id("ListProjectUsers", "ProjectUser", one_to_hundred()),
            last_id("ListUsers", "User", one_to_hundred()),
            last_id("ListInputItems", "ItemResource", one_to_hundred()),
            last_id("BetaListInputItems", "BetaItemResource", one_to_hundred()),
            last_id("ListFiles", "OpenAIFile", limit(Some(10_000), Some(10_000))),
            last_id("ListVideos", "VideoResource", limit(Some(100), None)),
            last_id("ListSkills", "SkillResource", limit(Some(100), None)),
            last_id("ListSkillVersions", "SkillVersionResource", limit(Some(100), None)),
            both_ids("ListAuditLogs", "AuditLog", one_to_hundred()),
            both_ids("ListProjectRateLimits", "ProjectRateLimit", limit(None, Some(100))),
            both_ids("ListProjectSpendAlerts", "ProjectSpendAlert", one_to_hundred()),
            both_ids("ListOrganizationSpendAlerts", "OrganizationSpendAlert", one_to_hundred()),
            both_ids("ListVectorStores", "VectorStoreObject", one_to_hundred()),
            both_ids("ListFilesInVectorStoreBatch", "VectorStoreFileObject", one_to_hundred()),
            both_ids("ListVectorStoreFiles", "VectorStoreFileObject", one_to_hundred()),
            both_ids("ListThreadItemsMethod", "ThreadItem", one_to_hundred()),
            both_ids("ListThreadsMethod", "ThreadResource", one_to_hundred()),
            last_item("ListFineTuningEvents", "FineTuningJobEvent", limit(None, Some(20))),
            last_item("ListPaginatedFineTuningJobs", "FineTuningJob", limit(None, Some(20))),
            next("ListGroups", "GroupResponse", limit(Some(1000), Some(100))),
            next("ListGroupUsers", "GroupUser", limit(Some(1000), Some(100))),
            next("ListGroupRoleAssignments", "AssignedRoleDetails", limit(Some(1000), None)),
            next("ListUserRoleAssignments", "AssignedRoleDetails", limit(Some(1000), None)),
            next("ListProjectGroupRoleAssignments", "AssignedRoleDetails", limit(Some(1000), None)),
            next("ListProjectUserRoleAssignments", "AssignedRoleDetails", limit(Some(1000), None)),
            next("ListRoles", "Role", limit(Some(1000), Some(1000))),
            next("ListProjectRoles", "Role", limit(Some(1000), Some(1000))),
            next("ListProjectGroups", "ProjectGroup", one_to_hundred()),
            usage("UsageCosts", limit(Some(180), Some(7))),
            usage("UsageAudioSpeeches", unbounded()),
            usage("UsageAudioTranscriptions", unbounded()),
            usage("UsageCodeInterpreterSessions", unbounded()),
            usage("UsageCompletions", unbounded()),
            usage("UsageEmbeddings", unbounded()),
            usage("UsageFileSearchCalls", unbounded()),
            usage("UsageImages", unbounded()),
            usage("UsageModerations", unbounded()),
            usage("UsageVectorStores", unbounded()),
            usage("UsageWebSearchCalls", unbounded()),
            opted_out("ListModels", Unpaged::NotPaged("the reply has no continuation")),
            opted_out("RetrieveVectorStoreFileContent", Unpaged::ProviderGap(provider_gap)),
            opted_out("SearchVectorStore", Unpaged::ProviderGap(provider_gap)),
        ],
    }]
}
