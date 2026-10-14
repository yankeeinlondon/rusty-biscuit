//! API-level pagination: resolution order, the exceptions table, adoption of
//! declared parameters, and the legacy metadata's independence.

mod pagination_support;

use pagination_support::{
    api, endpoint, errors, get, listed, post, resolution, row, with_headers, with_query,
    with_required_query,
};
use schematic_define::pagination::{
    AdoptionOutcome, Collection, CursorSource, Direction, IndexBase, InputParam, Knob, LinkSource,
    ManagedRole, MoreSignal, PageSize, PaginationError, PaginationRuleGroup, ParamLocation,
    Partition, Resolution, ResolutionOrigin, ResolvedList, Strategy, ValueSource, adopted_params,
    caller_knobs, param_adoptions, root_array_item_type, validate_pagination,
};
use schematic_define::{ApiResponse, EndpointParams, PaginationResponse, PaginationStyle, RestMethod};

fn page_default() -> Strategy {
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("per_page").server_default(30).max(100))
        .build()
}

fn root_list(item: &str) -> ResolvedList {
    ResolvedList {
        items: Collection::root().items,
        item_type: Some(item.to_owned()),
    }
}

// ---------------------------------------------------------------------------
// R4: resolution order
// ---------------------------------------------------------------------------

#[test]
fn get_vec_is_a_list_that_inherits_the_default_or_is_unpaginated_without_one() {
    let mut forge = api("Forge");
    forge.endpoints = vec![get("ListReleases", "/releases", "Vec<Release>")];

    assert_eq!(
        resolution(&forge, "ListReleases"),
        Resolution::Unpaginated {
            list: root_list("Release"),
            origin: ResolutionOrigin::Unauthored,
        }
    );
    assert_eq!(caller_knobs(&resolution(&forge, "ListReleases")), vec![]);

    forge.default_pagination = Some(page_default());
    assert_eq!(
        resolution(&forge, "ListReleases"),
        Resolution::Paginated {
            list: root_list("Release"),
            strategy: page_default(),
            alternates: vec![],
            origin: ResolutionOrigin::ApiDefault,
        }
    );
    assert_eq!(
        caller_knobs(&resolution(&forge, "ListReleases")),
        vec![
            Knob::StartPage { base: IndexBase::OneBased },
            Knob::PageSize { param: "per_page".into(), min: 1, max: Some(100), server_default: Some(30) },
            Knob::Next,
        ]
    );
}

#[test]
fn only_get_with_an_explicit_vec_is_recognized() {
    let mut forge = api("Forge");
    forge.default_pagination = Some(page_default());
    forge.endpoints = vec![
        post("UploadResources", "/resources", "Vec<Resource>"),
        endpoint("ReplaceAll", RestMethod::Put, "/things", "Vec<Thing>"),
        get("GetRelease", "/releases/{id}", "Release"),
        get("ListAliased", "/aliased", "ReleaseList"),
        get("ListOpaque", "/opaque", "serde_json::Value"),
        get("ListQualified", "/qualified", "std::vec::Vec<Release>"),
        get("Download", "/download", "Vec<u8>"),
    ];
    forge.endpoints[6].response = ApiResponse::Binary;
    for endpoint in &forge.endpoints {
        assert_eq!(resolution(&forge, &endpoint.id), Resolution::NotAList, "{}", endpoint.id);
    }
    assert_eq!(validate_pagination(&forge), vec![]);
}

#[test]
fn a_row_decides_before_inference() {
    let mut forge = api("Forge");
    forge.default_pagination = Some(page_default());
    let offset = Strategy::offset("skip", MoreSignal::NonEmptyPage).build();
    forge.endpoints = vec![
        get("ListTags", "/tags", "Vec<Tag>"),
        get("GetTagReference", "/refs/{ref}", "Vec<GitRef>"),
        get("ListRuns", "/runs", "RunsResponse"),
        post("SearchThings", "/search", "Vec<Thing>"),
        get("ListOthers", "/others", "Others"),
    ];
    forge.pagination_exceptions = vec![
        row("GetTagReference", Collection::root().unpaginated()),
        row("ListRuns", Collection::at("runs").item_type("Run")),
        row("SearchThings", Collection::root()),
        row("ListOthers", Collection::at("items").item_type("Other").paginate(offset.clone())),
    ];

    assert_eq!(resolution(&forge, "ListTags").origin(), Some(ResolutionOrigin::ApiDefault));
    assert_eq!(
        resolution(&forge, "GetTagReference"),
        Resolution::Unpaginated { list: root_list("GitRef"), origin: ResolutionOrigin::Exception }
    );
    let runs = resolution(&forge, "ListRuns");
    assert_eq!(runs.origin(), Some(ResolutionOrigin::ApiDefault));
    assert_eq!(runs.list().unwrap().items, Collection::at("runs").items);
    assert_eq!(runs.list().unwrap().item_type.as_deref(), Some("Run"));
    // A non-GET endpoint is a list only through a row; its root `Vec<T>` names
    // the item type.
    let search = resolution(&forge, "SearchThings");
    assert_eq!(search.origin(), Some(ResolutionOrigin::ApiDefault));
    assert_eq!(search.list().unwrap().item_type.as_deref(), Some("Thing"));
    assert_eq!(
        resolution(&forge, "ListOthers"),
        Resolution::Paginated {
            list: ResolvedList { items: Collection::at("items").items, item_type: Some("Other".into()) },
            strategy: offset,
            alternates: vec![],
            origin: ResolutionOrigin::Exception,
        }
    );
    assert_eq!(validate_pagination(&forge), vec![]);
}

#[test]
fn a_wrapper_row_without_a_default_is_an_unpaginated_list() {
    let mut plain = api("Plain");
    plain.endpoints = vec![get("ListModels", "/models", "Models")];
    plain.pagination_exceptions = vec![row("ListModels", Collection::at("data").item_type("Model"))];
    assert!(matches!(
        resolution(&plain, "ListModels"),
        Resolution::Unpaginated { origin: ResolutionOrigin::Unauthored, .. }
    ));
    assert_eq!(validate_pagination(&plain), vec![]);
}

#[test]
fn root_array_item_type_reads_only_an_explicit_vec() {
    assert_eq!(root_array_item_type(&ApiResponse::json_type("Vec<Issue>")).as_deref(), Some("Issue"));
    assert_eq!(root_array_item_type(&ApiResponse::json_type(" Vec< serde_json::Value > ")).as_deref(), Some("serde_json::Value"));
    assert_eq!(root_array_item_type(&ApiResponse::json_type("Vec<>")), None);
    assert_eq!(root_array_item_type(&ApiResponse::json_type("ListModelsResponse")), None);
    assert_eq!(root_array_item_type(&ApiResponse::Text), None);
}

// ---------------------------------------------------------------------------
// R5: the exceptions table
// ---------------------------------------------------------------------------

#[test]
fn unknown_row_id_is_rejected_with_the_nearest_endpoint() {
    let mut forge = api("Forge");
    forge.endpoints = vec![get("ListReleases", "/releases", "Vec<Release>"), get("ListTags", "/tags", "Vec<Tag>")];
    forge.pagination_exceptions = vec![
        row("ListReleaze", Collection::root().unpaginated()),
        row("DeleteEverything", Collection::root().unpaginated()),
    ];
    let diagnostics = validate_pagination(&forge);
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].endpoint.as_deref(), Some("ListReleaze"));
    assert_eq!(
        diagnostics[0].error,
        PaginationError::UnknownExceptionId { id: "ListReleaze".into(), nearest: Some("ListReleases".into()) }
    );
    assert_eq!(
        diagnostics[0].to_string(),
        "unknown-exception-id: exception names `ListReleaze`, which is not an endpoint of this API; \
         did you mean `ListReleases`?"
    );
    assert_eq!(
        diagnostics[1].error,
        PaginationError::UnknownExceptionId { id: "DeleteEverything".into(), nearest: None }
    );
    assert_eq!(diagnostics[1].error.group(), PaginationRuleGroup::Exception);

    // The valid sibling.
    forge.pagination_exceptions = vec![row("ListReleases", Collection::root().unpaginated())];
    assert_eq!(validate_pagination(&forge), vec![]);
}

#[test]
fn duplicate_row_id_is_rejected_and_the_first_row_decides() {
    let mut forge = api("Forge");
    forge.default_pagination = Some(page_default());
    forge.endpoints = vec![get("ListReleases", "/releases", "Vec<Release>")];
    forge.pagination_exceptions = vec![
        row("ListReleases", Collection::root().unpaginated()),
        row("ListReleases", Collection::root()),
    ];
    assert_eq!(errors(&forge), vec![PaginationError::DuplicateExceptionId("ListReleases".into())]);
    assert!(matches!(resolution(&forge, "ListReleases"), Resolution::Unpaginated { .. }));
}

#[test]
fn row_list_shape_rules() {
    let shape = |endpoint, collection: Collection| errors(&listed(endpoint, collection.paginate(page_default())));

    assert_eq!(
        shape(get("L", "/l", "Wrapper"), Collection::at("")),
        vec![
            PaginationError::InvalidItemSelector {
                selector: "\"\"[*]".into(),
                reason: "a key is empty or contains `[` or `]`",
            },
            PaginationError::MissingItemType,
        ]
    );
    assert!(matches!(
        shape(get("L", "/l", "Wrapper"), Collection::at("data[0]").item_type("T")).as_slice(),
        [PaginationError::InvalidItemSelector { .. }]
    ));
    assert_eq!(shape(get("L", "/l", "Wrapper"), Collection::at("data")), vec![PaginationError::MissingItemType]);
    assert_eq!(
        shape(get("L", "/l", "Wrapper"), Collection::at("data").item_type(" ")),
        vec![PaginationError::EmptyValue("item type")]
    );
    assert!(matches!(
        shape(get("L", "/l", "Vec<T>"), Collection::at("data").item_type("T")).as_slice(),
        [PaginationError::NotAnArrayLocation(_)]
    ));
    let mut binary = get("L", "/l", "Unused");
    binary.response = ApiResponse::Binary;
    assert_eq!(
        shape(binary, Collection::root().item_type("u8")),
        vec![PaginationError::NotAnArrayLocation("the response is not JSON".into())]
    );
    // A root on a non-`Vec` response needs an authored item type.
    assert_eq!(shape(get("L", "/l", "Wrapper"), Collection::root()), vec![PaginationError::MissingItemType]);

    // Valid siblings: wrapper with a type, explicit root over an opaque body,
    // root `Vec<T>` inferring its type.
    assert_eq!(shape(get("L", "/l", "Wrapper"), Collection::at("data").item_type("T")), vec![]);
    assert_eq!(shape(get("L", "/l", "serde_json::Value"), Collection::at("[*]").item_type("serde_json::Value")), vec![]);
    assert_eq!(shape(get("L", "/l", "Vec<T>"), Collection::root()), vec![]);
}

#[test]
fn an_unpaginated_row_needs_no_item_type() {
    let opt_out = listed(get("ListAuthUsers", "/users", "Paged"), Collection::at("data").unpaginated());
    assert_eq!(validate_pagination(&opt_out), vec![]);
}

#[test]
fn alternates_are_validated_and_resolved_but_add_no_knobs() {
    let keyset = || {
        Strategy::next_link(LinkSource::link_header("next"))
            .page_size(PageSize::new("per_page").server_default(20).max(100))
            .constant("pagination", "keyset")
    };
    let primary = || Collection::root().paginate(page_default());

    let duplicate = listed(get("L", "/l", "Vec<T>"), primary().or_paginate("keyset", keyset()).or_paginate("keyset", keyset()));
    assert_eq!(errors(&duplicate), vec![PaginationError::DuplicateLabel("keyset".into())]);
    let orphan = listed(get("L", "/l", "Vec<T>"), Collection::root().or_paginate("keyset", keyset()));
    assert_eq!(errors(&orphan), vec![PaginationError::AlternatesWithoutPrimary]);
    let opted_out = listed(get("L", "/l", "Vec<T>"), Collection::root().unpaginated().or_paginate("keyset", keyset()));
    assert_eq!(errors(&opted_out), vec![PaginationError::AlternatesOnUnpaginated]);
    let bad_label = listed(get("L", "/l", "Vec<T>"), primary().or_paginate("Key Set", keyset()));
    assert_eq!(errors(&bad_label), vec![PaginationError::InvalidLabel("Key Set".into())]);

    let valid = listed(get("L", "/l", "Vec<T>"), primary().or_paginate("keyset", keyset()));
    assert_eq!(validate_pagination(&valid), vec![]);
    let resolved = resolution(&valid, "L");
    let Resolution::Paginated { alternates, .. } = &resolved else { panic!("paginated expected") };
    assert_eq!(alternates[0].label, "keyset");
    let without = listed(get("L", "/l", "Vec<T>"), primary());
    assert_eq!(caller_knobs(&resolved), caller_knobs(&resolution(&without, "L")));

    // An alternate's own errors name it.
    let broken = listed(get("L", "/l", "Vec<T>"), primary().or_paginate("keyset", keyset().constant("pagination", "x")));
    let diagnostics = validate_pagination(&broken);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].alternate.as_deref(), Some("keyset"));
    assert!(diagnostics[0].to_string().ends_with("(alternate `keyset`)"));
}

#[test]
fn an_alternate_constant_may_shadow_a_declared_parameter() {
    // GitLab `ListGroupProjects` declares `order_by` and `sort`; the keyset
    // alternate fixes both. Alternates are not generated, so no field is
    // hidden and no collision is reported (an open question for when they are).
    let keyset = Strategy::next_link(LinkSource::link_header("next"))
        .constant("pagination", "keyset")
        .constant("order_by", "id")
        .constant("sort", "asc");
    let group_projects = with_query(get("ListGroupProjects", "/groups/{id}/projects", "Vec<Project>"), &["order_by", "sort"]);
    let api = listed(group_projects, Collection::root().paginate(page_default()).or_paginate("keyset", keyset));
    assert_eq!(validate_pagination(&api), vec![]);
}

#[test]
fn api_default_errors_are_reported_once_without_an_endpoint() {
    let mut forge = api("Forge");
    forge.default_pagination = Some(Strategy::cursor("after", CursorSource::last_item("id")).build());
    forge.endpoints = vec![get("ListA", "/a", "Vec<A>"), get("ListB", "/b", "Vec<B>")];
    let diagnostics = validate_pagination(&forge);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].endpoint, None);
    assert_eq!(diagnostics[0].error, PaginationError::CursorWithoutEndSignal);
}

#[test]
fn endpoint_dependent_errors_of_the_default_are_reported_per_endpoint() {
    let mut forge = api("Forge");
    forge.default_pagination = Some(
        Strategy::cursor(InputParam::body("cursor"), CursorSource::body("next")).build(),
    );
    forge.endpoints = vec![get("ListA", "/a", "Vec<A>")];
    let diagnostics = validate_pagination(&forge);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].endpoint.as_deref(), Some("ListA"));
    assert_eq!(diagnostics[0].error, PaginationError::BodyInputOnGet("cursor".into()));
}

// ---------------------------------------------------------------------------
// R7 and R2a: adoption of declared parameters
// ---------------------------------------------------------------------------

fn openai_cursor() -> Strategy {
    Strategy::cursor("after", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .page_size(PageSize::new("limit"))
        .resumable()
        .build()
}

#[test]
fn a_declared_parameter_with_its_knob_is_adopted() {
    let list_files = with_query(get("ListFiles", "/files", "ListFilesResponse"), &["purpose", "limit", "order", "after"]);
    let adopted = adopted_params(&list_files, &openai_cursor());
    let names: Vec<(&str, ManagedRole)> = adopted.iter().map(|a| (a.name.as_str(), a.role)).collect();
    assert_eq!(names, [("after", ManagedRole::Cursor(Direction::Forward)), ("limit", ManagedRole::PageSize)]);
    assert!(adopted.iter().all(|a| a.location == ParamLocation::Query && !a.required));

    let api = listed(list_files, Collection::at("data").item_type("OpenAIFile").paginate(openai_cursor()));
    assert_eq!(validate_pagination(&api), vec![]);
    let knobs = caller_knobs(&resolution(&api, "ListFiles"));
    assert!(knobs.contains(&Knob::ResumeCursor { direction: Direction::Forward, parts: 1 }));
    assert!(knobs.iter().any(|knob| matches!(knob, Knob::PageSize { .. })));
}

#[test]
fn a_declared_parameter_without_its_knob_is_refused_and_kept() {
    // `after` without `.resumable()`: no `.after(..)` knob would replace it.
    let not_resumable = Strategy::cursor("after", CursorSource::body("last_id"))
        .more_when(MoreSignal::has_more("has_more"))
        .build();
    let list_files = with_query(get("ListFiles", "/files", "ListFilesResponse"), &["after"]);
    assert_eq!(adopted_params(&list_files, &not_resumable), vec![]);
    let outcomes: Vec<AdoptionOutcome> = param_adoptions(&list_files, &not_resumable).iter().map(|a| a.outcome).collect();
    assert_eq!(outcomes, [AdoptionOutcome::WithoutKnob]);

    let api = listed(list_files.clone(), Collection::at("data").item_type("F").paginate(not_resumable));
    let diagnostics = validate_pagination(&api);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].error, PaginationError::AdoptionWithoutKnob("after".into()));
    assert_eq!(diagnostics[0].error.group(), PaginationRuleGroup::Parameter);

    // Leaving the parameter out of the strategy keeps the field instead.
    let other_input = Strategy::cursor("cursor", CursorSource::body("last_id")).more_when(MoreSignal::has_more("has_more"));
    assert_eq!(validate_pagination(&listed(list_files, Collection::at("data").item_type("F").paginate(other_input))), vec![]);
}

#[test]
fn a_near_miss_and_an_unadoptable_input_collide() {
    // Same name, different location.
    let header_after = with_headers(get("L", "/l", "Vec<T>"), &["After"]);
    let api = listed(header_after.clone(), Collection::root().paginate(openai_cursor()));
    assert_eq!(errors(&api), vec![PaginationError::CollidesWithEndpointParam("After".into())]);
    assert_eq!(param_adoptions(&header_after, &openai_cursor())[0].outcome, AdoptionOutcome::NearMiss);

    let path_page = get("L", "/pages/{page}", "Vec<T>");
    let api = listed(path_page, Collection::root().paginate(page_default()));
    assert_eq!(errors(&api), vec![PaginationError::CollidesWithEndpointParam("page".into())]);

    // A constant no caller can set.
    let constant = Strategy::page_number("page", MoreSignal::NonEmptyPage).constant("order", "asc");
    let api = listed(with_query(get("L", "/l", "Vec<T>"), &["order"]), Collection::root().paginate(constant));
    assert_eq!(errors(&api), vec![PaginationError::CollidesWithEndpointParam("order".into())]);

    // A header input adopts a declared header without regard to case.
    let header_cursor = Strategy::cursor(InputParam::header("X-Continuation"), CursorSource::header("x-continuation"))
        .resumable()
        .build();
    let declared = with_headers(get("L", "/l", "Vec<T>"), &["x-continuation"]);
    let adopted = adopted_params(&declared, &header_cursor);
    assert_eq!(adopted.len(), 1);
    assert_eq!((adopted[0].location, adopted[0].name.as_str()), (ParamLocation::Header, "x-continuation"));
}

#[test]
fn a_required_managed_parameter_needs_a_value_to_send() {
    let limit_required = with_required_query(get("L", "/l", "Vec<T>"), "limit");
    let without_default = Strategy::cursor("after", CursorSource::body("next"))
        .page_size(PageSize::new("limit").max(100))
        .build();
    let api = listed(limit_required.clone(), Collection::root().paginate(without_default));
    assert_eq!(errors(&api), vec![PaginationError::PagerManagedRequiredWithoutDefault("limit".into())]);

    let with_default = Strategy::cursor("after", CursorSource::body("next"))
        .page_size(PageSize::new("limit").max(100).sent_default(100))
        .build();
    let adopted = adopted_params(&limit_required, &with_default);
    assert!(adopted[0].required && adopted[0].has_sent_default);
    assert_eq!(validate_pagination(&listed(limit_required, Collection::root().paginate(with_default))), vec![]);

    // A page or offset always has its base to send.
    let page_required = with_required_query(get("L", "/l", "Vec<T>"), "page");
    assert_eq!(validate_pagination(&listed(page_required, Collection::root().paginate(page_default()))), vec![]);

    // A required cursor has no first value.
    let cursor_required = with_required_query(get("L", "/l", "Vec<T>"), "after");
    let api = listed(cursor_required, Collection::root().paginate(openai_cursor()));
    assert_eq!(errors(&api), vec![PaginationError::PagerManagedRequiredWithoutDefault("after".into())]);
}

#[test]
fn partition_inputs_are_adopted_with_their_knob() {
    let strategy = Strategy::cursor("after", CursorSource::body("next"))
        .partition(Partition::new("slice", "slices").max(8));
    let declared = with_query(get("L", "/l", "Vec<T>"), &["slice", "slices"]);
    let roles: Vec<ManagedRole> = adopted_params(&declared, &strategy).iter().map(|a| a.role).collect();
    assert_eq!(roles, [ManagedRole::PartitionIndex, ManagedRole::PartitionCount]);
}

// ---------------------------------------------------------------------------
// R8: legacy metadata is inert
// ---------------------------------------------------------------------------

#[test]
fn legacy_pagination_metadata_does_not_change_resolution() {
    let plain_params = EndpointParams::default()
        .with_query_param("page", schematic_define::QueryParamType::Integer, false, None)
        .with_query_param("per_page", schematic_define::QueryParamType::Integer, false, None);
    let mut legacy_params = EndpointParams::default().with_pagination(PaginationStyle::github());
    legacy_params.response_pagination = Some(PaginationResponse::LinkHeader);
    assert_eq!(
        legacy_params.query.iter().map(|p| &p.name).collect::<Vec<_>>(),
        plain_params.query.iter().map(|p| &p.name).collect::<Vec<_>>()
    );

    for default in [None, Some(page_default())] {
        let build = |params: EndpointParams| {
            let mut forge = api("Forge");
            forge.default_pagination = default.clone();
            let mut releases = get("ListReleases", "/releases", "Vec<Release>");
            releases.params = Some(params);
            forge.endpoints = vec![releases];
            forge
        };
        let with_legacy = build(legacy_params.clone());
        let without = build(plain_params.clone());
        let legacy_endpoint = &with_legacy.endpoints[0];
        let plain_endpoint = &without.endpoints[0];
        assert_eq!(resolution(&with_legacy, "ListReleases"), resolution(&without, "ListReleases"));
        assert_eq!(caller_knobs(&resolution(&with_legacy, "ListReleases")), caller_knobs(&resolution(&without, "ListReleases")));
        assert_eq!(validate_pagination(&with_legacy), validate_pagination(&without));
        if let Some(strategy) = &default {
            // The materialized parameters are adopted either way.
            assert_eq!(adopted_params(legacy_endpoint, strategy), adopted_params(plain_endpoint, strategy));
            assert_eq!(adopted_params(legacy_endpoint, strategy).len(), 2);
        }
    }

    // Legacy metadata alone never makes a list.
    let mut forge = api("Forge");
    forge.default_pagination = Some(page_default());
    let mut wrapper = get("ListThings", "/things", "Things");
    wrapper.params = Some(EndpointParams::default().with_pagination(PaginationStyle::bitbucket()));
    forge.endpoints = vec![wrapper];
    assert_eq!(resolution(&forge, "ListThings"), Resolution::NotAList);
}

#[test]
fn total_from_body_is_a_total_knob() {
    let runs = page_default().total(ValueSource::body("total_count"));
    let api = listed(get("ListWorkflowRuns", "/runs", "WorkflowRunsResponse"), Collection::at("workflow_runs").item_type("WorkflowRun").paginate(runs));
    assert_eq!(validate_pagination(&api), vec![]);
    assert!(caller_knobs(&resolution(&api, "ListWorkflowRuns")).contains(&Knob::Total));
}
