//! Hand-written pagination overlay for the importer-generated OpenAI definition.
//!
//! The OpenAPI importer emits no pagination, so `define_openai_api()` passes its
//! generated [`RestApi`] through [`apply`] via the importer's generic overlay hook
//! (`schematic-gen import --overlay crate::openai_pagination::apply`). `just
//! import-openai` regenerates `openai/` but never writes this file.
//!
//! The overlay also scopes the importer's response model. The import declares
//! every success shape and error status the document lists; the overlay keeps
//! one success shape per endpoint, the one the importer chose before it read
//! several (the first JSON shape, else the first shape), and no errors, so an
//! endpoint's generated success and failure types change only when it is
//! migrated deliberately. Where the importer separated a referenced union into
//! one shape per member, its earlier choice was the union itself, so the
//! overlay restores it (`GetConversationItem` returns `ConversationItem`). Several streaming endpoints, such as
//! `CreateChatCompletion`, would also need author-chosen names for their
//! success enums. The one migrated endpoint, `GetResponse`, gets its shapes
//! from [`stream_get_response`].
//!
//! Every OpenAI list replies with a wrapper whose items are at `data`, so each
//! list, paginated or not, has an endpoint collection; none inherits by being a root
//! array. A collection without its own strategy inherits the API default.
//!
//! The API-default page size has no maximum: OpenAI's `limit` bounds differ per
//! endpoint, and some depend on `bucket_width`. A collection authors a maximum only
//! where the vendored `specs/openai/openapi.yaml` states one that holds for every
//! request. Where the provider documents a minimum of 0, the minimum stays at 1,
//! the lowest the pagination model accepts. An adopted `limit` parameter
//! carries the bounds the import read from the document, and a page-size
//! parameter must agree with the strategy that adopts it, so [`apply`]
//! rewrites each adopted parameter's `minimum`/`maximum` to its strategy's
//! ([`align_page_size_bounds`]).

use schematic_define::{ResponseShape, RestApi, StreamEnd, StreamFraming, SuccessShape, TypeRef};
use schematic_define::pagination::{
    Collection, CursorSource, MoreSignal, PageSize, ParamLocation, Strategy, adopted_page_size,
};

/// Sets the OpenAI default strategy and a collection on each list endpoint,
/// and scopes the imported response model (see the module docs).
#[must_use]
pub fn apply(mut api: RestApi) -> RestApi {
    api.default_pagination = Some(last_id_cursor(limit()));
    for (id, collection) in collections() {
        let endpoint = api.endpoints.iter_mut().find(|endpoint| endpoint.id == id)
            .unwrap_or_else(|| panic!("OpenAI pagination overlay names unknown endpoint `{id}`"));
        endpoint.pagination = Some(collection);
    }
    for endpoint in &mut api.endpoints {
        let kept = endpoint.response.iter().position(|shape| matches!(shape.shape, ResponseShape::Json(_))).unwrap_or(0);
        let shapes = std::mem::take(&mut endpoint.response);
        endpoint.response = shapes.into_iter().nth(kept).into_iter().collect();
        endpoint.errors.clear();
    }
    keep_conversation_item_union(&mut api);
    stream_get_response(&mut api);
    align_page_size_bounds(&mut api);
    api
}

/// Gives each adopted page-size parameter its strategy's bounds, replacing
/// the document's (`minimum: 0` where the strategy's minimum is 1, and no
/// `maximum` where the strategy leaves the size unbounded). Its other rules,
/// such as `format`, are kept.
fn align_page_size_bounds(api: &mut RestApi) {
    let adopted: Vec<_> = api.endpoints.iter().map(|endpoint| adopted_page_size(api, endpoint)).collect();
    for (endpoint, adopted) in api.endpoints.iter_mut().zip(adopted) {
        let (Some(adopted), Some(params)) = (adopted, endpoint.params.as_mut()) else { continue };
        let declared = match adopted.location {
            ParamLocation::Query => &mut params.query,
            ParamLocation::Header => &mut params.header,
            ParamLocation::Cookie => &mut params.cookie,
            ParamLocation::Path => &mut params.path,
        };
        if let Some(param) = declared.iter_mut().find(|param| param.name == adopted.name && param.constraints.is_some()) {
            param.constraints = Some(Box::new(adopted.constraints));
        }
    }
}

/// The import separates `ConversationItem`, a `type`-discriminated `oneOf`
/// behind a `$ref`, into one tagged shape per member; the first is only
/// `Message`. The union already decodes every member by `type`.
fn keep_conversation_item_union(api: &mut RestApi) {
    let endpoint = api.endpoints.iter_mut().find(|endpoint| endpoint.id == "GetConversationItem")
        .expect("OpenAI response overlay names unknown endpoint `GetConversationItem`");
    endpoint.response = vec![SuccessShape::json(TypeRef::Named("ConversationItem".into()))];
}

/// `GetResponse` replays a stored response as server-sent events when its
/// `stream` query parameter is true. The vendored document lists only the JSON
/// reply (OpenAI's current document lists both), so the stream is authored
/// here: `BetaResponseStreamEvent` items until the connection closes, since no
/// terminator is documented for the Responses stream (`response.completed`
/// and the other terminal events are items).
fn stream_get_response(api: &mut RestApi) {
    let endpoint = api.endpoints.iter_mut().find(|endpoint| endpoint.id == "GetResponse")
        .expect("OpenAI response overlay names unknown endpoint `GetResponse`");
    let mut events = SuccessShape::new(ResponseShape::Stream {
        framing: StreamFraming::Sse(TypeRef::Named("BetaResponseStreamEvent".into())),
        end: StreamEnd::ConnectionClose,
        keep: false,
    });
    events.note = Some("returned when the `stream` query parameter is true".into());
    for shape in &mut endpoint.response {
        shape.note = Some("returned when the `stream` query parameter is false or omitted".into());
    }
    endpoint.response.push(events);
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

fn inherit(endpoint: &str, item_type: &str) -> (String, Collection) {
    (endpoint.to_owned(), Collection::at("data").item_type(item_type))
}

fn paged(endpoint: &str, item_type: &str, strategy: Strategy) -> (String, Collection) {
    (
        endpoint.to_owned(),
        Collection::at("data").item_type(item_type).paginate(strategy),
    )
}

fn unpaged(endpoint: &str, item_type: &str) -> (String, Collection) {
    (endpoint.to_owned(), Collection::at("data").item_type(item_type).unpaginated())
}

fn collections() -> Vec<(String, Collection)> {
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
        paged("ListInputItems", "BetaItemResource", last_id_cursor(one_to_hundred())),
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

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic(expected = "OpenAI pagination overlay names unknown endpoint `ListChatCompletions`")]
    fn overlay_rejects_unknown_endpoint_id() {
        let ctx = schematic_define::DefinitionContext::types_in("::models");
        let api = schematic_define::RestApi::builder(&ctx, "OverlayFixture", "https://example.test")
            .describes("An imported API missing the overlay endpoint").no_auth().build().unwrap();
        let _ = super::apply(api);
    }

    #[test]
    fn get_response_returns_json_or_server_sent_events_and_every_other_endpoint_keeps_one_shape() {
        use schematic_define::{ResponseShape, StreamEnd, StreamFraming, TypeRef};
        let api = crate::openai::define_openai_api();
        let endpoint = |id: &str| api.endpoints.iter().find(|endpoint| endpoint.id == id).unwrap();

        let get = endpoint("GetResponse");
        let shapes: Vec<_> = get.response.iter().map(|shape| (&shape.shape, shape.note.as_deref())).collect();
        assert_eq!(shapes, [
            (&ResponseShape::Json(TypeRef::Named("BetaResponse".into())), Some("returned when the `stream` query parameter is false or omitted")),
            (&ResponseShape::Stream { framing: StreamFraming::Sse(TypeRef::Named("BetaResponseStreamEvent".into())),
                end: StreamEnd::ConnectionClose, keep: false }, Some("returned when the `stream` query parameter is true")),
        ]);
        assert!(get.errors.is_empty());

        // The import lists JSON or binary plus SSE, and error statuses, for
        // these; the overlay keeps the shape the importer chose before.
        for (id, kept) in [
            ("CreateChatCompletion", ResponseShape::Json(TypeRef::Named("CreateChatCompletionResponse".into()))),
            ("CreateResponse", ResponseShape::Json(TypeRef::Named("BetaResponse".into()))),
            ("CreateSpeech", ResponseShape::Binary),
            ("RetrieveVideoContent", ResponseShape::Json(TypeRef::Primitive(schematic_define::PrimitiveType::Json))),
        ] {
            assert_eq!(endpoint(id).response.iter().map(|shape| &shape.shape).collect::<Vec<_>>(), [&kept], "{id}");
        }
        for other in api.endpoints.iter().filter(|endpoint| endpoint.id != "GetResponse") {
            assert_eq!(other.response.len(), 1, "{}", other.id);
            assert!(other.errors.is_empty(), "{}", other.id);
        }
        api.validate_references().unwrap();
    }
}
