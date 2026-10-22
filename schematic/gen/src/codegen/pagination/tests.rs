use schematic_define::pagination::{
    Collection, CursorLeg, CursorSource, InputParam, KeepAlive, LinkSource, MoreSignal, PageSize,
    Partition, Session, SessionStep, Strategy, ValueSource,
};
use schematic_define::{
    ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, ParamDef, ParamStyle,
    QueryParamType, RestApi, RestMethod,
};

use super::*;
use crate::codegen::endpoint_methods::{RenameReason, resolve_names};
use crate::codegen::request_structs::format_generated_code;
use crate::diagnostics::Feature;
use crate::output::assemble::assemble_api_module;
use crate::validation::validate_rust_client;

fn param(name: &str, required: bool) -> ParamDef {
    ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required,
        description: None,
        param_type: QueryParamType::Integer,
        explode: false,
        style: ParamStyle::Form,
    }
}

fn get(id: &str, path: &str, response: ApiResponse, query: Vec<ParamDef>) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = RestMethod::Get; fixture.path = path.to_string(); fixture.description = format!("{id} endpoint"); fixture.request = None; fixture.response = response; fixture.headers = vec![]; fixture.params = (!query.is_empty()).then(|| EndpointParams {
            query,
            ..Default::default()
        }); fixture.cross_origin = None; fixture.pagination = None; fixture }
}

fn api(endpoints: Vec<Endpoint>) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "Shelf".to_string(); fixture.description = "Shelf API".to_string(); fixture.base_url = "https://api.example.com".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = endpoints; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; crate::test_utils::with_referenced_components(fixture) }
}

/// A GitHub-like API: page numbers ended by a `Link` relation, a bounded
/// page size, a root-array list, and one non-list endpoint.
fn github_like() -> RestApi {
    let mut api = api(vec![
        get(
            "ListReleases",
            "/repos/{owner}/releases",
            ApiResponse::json_vec_type("Release"),
            vec![param("page", false), param("per_page", false), param("draft", false)],
        ),
        get("GetRelease", "/releases/{id}", ApiResponse::json_type("Release"), vec![]),
    ]);
    api.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::link_rel("next"))
            .page_size(PageSize::new("per_page").server_default(30).max(100))
            .build(),
    );
    api
}

fn impls(api: &RestApi) -> String {
    format_generated_code(&generate_pagination_impls(api, "Request")).expect("pager impls parse")
}

fn module(api: &RestApi) -> String {
    format_generated_code(&assemble_api_module(api)).expect("API module parses")
}

/// The text of the item in `code` that starts with `start`, up to its
/// closing brace at column zero.
fn item<'a>(code: &'a str, start: &str) -> &'a str {
    let from = code.find(start).unwrap_or_else(|| panic!("`{start}` is not in:\n{code}"));
    let rest = &code[from..];
    let to = rest.find("\n}\n").map_or(rest.len(), |end| end + 2);
    &rest[..to]
}

/// The method `name` of the impl block `block`, with its doc and attribute
/// lines.
fn method(block: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = block.lines().collect();
    let signature = format!("    pub fn {name}(");
    let at = lines.iter().position(|line| line.starts_with(&signature))?;
    let start = (0..at)
        .rev()
        .take_while(|&index| lines[index].starts_with("    ///") || lines[index].starts_with("    #["))
        .last()
        .unwrap_or(at);
    let end = (at..lines.len()).find(|&index| lines[index] == "    }")?;
    Some(lines[start..=end].join("\n"))
}

/// `text` without whitespace and trailing commas, so an assertion does not
/// depend on where the formatter breaks a line.
fn squash(text: &str) -> String {
    let mut squashed: String = text.split_whitespace().collect();
    while let Some(at) = [",)", ",]", ",}", ",>"].iter().find_map(|trailing| squashed.find(trailing)) {
        squashed.remove(at);
    }
    squashed
}

#[test]
fn page_number_strategy_emits_its_projection_markers_and_forwards() {
    let api = github_like();

    let code = impls(&api);
    assert_eq!(
        code,
        r#"impl schematic_runtime::pager::Paginated for ListReleasesRequest {
    type Item = Release;
    type Kind = schematic_runtime::pager::PageNumberKind;
    type Session = schematic_runtime::pager::NoSession;
    const STRATEGY: &'static schematic_runtime::pager::Strategy = &schematic_runtime::pager::spec::Strategy {
        kind: schematic_runtime::pager::spec::Kind::PageNumber {
            page: schematic_runtime::pager::spec::Input::Query("page"),
            base: schematic_runtime::pager::spec::IndexBase::OneBased,
            more: schematic_runtime::pager::spec::MoreSignal::LinkRel {
                rel: "next",
            },
        },
        size: Some(schematic_runtime::pager::spec::PageSize {
            param: schematic_runtime::pager::spec::Input::Query("per_page"),
            server_default: Some(30),
            sent_default: None,
            min: 1,
            max: Some(100),
            adjustable: false,
        }),
        constants: &[],
        total: None,
        terminal_values: &[],
        follow_up: None,
        session: None,
        partition: None,
    };
    const ITEMS: &'static schematic_runtime::pager::ItemSelector = &schematic_runtime::pager::spec::ItemSelector {
        record: &[schematic_runtime::pager::spec::Step::Each],
        projection: &[],
    };
    const PATH: &'static str = "/repos/{owner}/releases";
}
impl schematic_runtime::pager::PageSizeKnob for ListReleasesRequest {}
"#
    );

    let module = module(&api);
    let call = item(&module, "impl<'a> ListReleasesCall<'a> {");
    let pager_methods: Vec<String> = ["pager", "page", "page_size", "pages", "fetch_page", "collect_all"]
        .into_iter()
        .map(|name| method(call, name).unwrap_or_else(|| panic!("no `{name}` in:\n{call}")))
        .collect();
    assert_eq!(
        pager_methods.join("\n"),
        r#"    /// See [`Call::pager`](schematic_runtime::Call::pager).
    #[inline]
    pub fn pager(
        self,
    ) -> schematic_runtime::pager::Pager<'a, ShelfApi, ListReleasesRequest> {
        self.0.pager()
    }
    /// See [`Pager::page`](schematic_runtime::pager::Pager::page).
    #[inline]
    pub fn page(
        self,
        number: u64,
    ) -> schematic_runtime::pager::Pager<'a, ShelfApi, ListReleasesRequest> {
        self.0.pager().page(number)
    }
    /// See [`Pager::page_size`](schematic_runtime::pager::Pager::page_size).
    #[inline]
    pub fn page_size(
        self,
        size: u32,
    ) -> schematic_runtime::pager::Pager<'a, ShelfApi, ListReleasesRequest> {
        self.0.pager().page_size(size)
    }
    /// See [`Pager::pages`](schematic_runtime::pager::Pager::pages).
    #[inline]
    pub fn pages(
        self,
    ) -> schematic_runtime::pager::Pages<'a, ShelfApi, ListReleasesRequest> {
        self.0.pager().pages()
    }
    /// See [`Pager::fetch_page`](schematic_runtime::pager::Pager::fetch_page).
    #[inline]
    pub fn fetch_page(
        self,
    ) -> schematic_runtime::CallFuture<
        'a,
        schematic_runtime::pager::PageResult<ListReleasesRequest>,
    > {
        self.0.pager().fetch_page()
    }
    /// See [`Pager::collect_all`](schematic_runtime::pager::Pager::collect_all).
    #[inline]
    pub fn collect_all(
        self,
    ) -> schematic_runtime::CallFuture<
        'a,
        schematic_runtime::pager::CollectResult<ListReleasesRequest>,
    > {
        self.0.pager().collect_all()
    }"#
    );
    assert!(module.contains("The list is paginated: [`pager`](Self::pager)"));

    let unpaged = item(&module, "impl<'a> GetReleaseCall<'a> {");
    assert!(!unpaged.contains("pager") && !unpaged.contains("pages"), "{unpaged}");
}

#[test]
fn adopted_parameters_leave_the_request_and_the_call_type() {
    let api = github_like();

    let module = module(&api);
    let request = item(&module, "pub struct ListReleasesRequest {");
    assert!(request.contains("pub draft: Option<i64>"), "{request}");
    assert!(!request.contains("pub page:") && !request.contains("pub per_page:"), "{request}");
    for adopted in ["with_page(", "with_per_page(", "pub fn per_page("] {
        assert!(!module.contains(adopted), "`{adopted}` is still generated:\n{module}");
    }
    assert!(module.contains("pub fn with_draft(") && module.contains("pub fn draft("));
    assert!(
        module.contains("ListReleasesRequest::new(owner)"),
        "the request still takes only its path parameter"
    );

    let names = resolve_names(&api);
    let setters: Vec<&str> = names.endpoints[0].setters.iter().map(|s| s.setter.as_str()).collect();
    assert_eq!(setters, ["draft"]);
    assert!(names.renames.is_empty(), "{:?}", names.renames);
}

#[test]
fn only_adopted_required_parameters_keep_default() {
    let mut api = api(vec![get(
        "ListShelves",
        "/shelves",
        ApiResponse::json_vec_type("Shelf"),
        vec![param("page", true), param("per_page", true), param("draft", false)],
    )]);
    api.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::NonEmptyPage)
            .page_size(PageSize::new("per_page").sent_default(50))
            .build(),
    );

    let module = module(&api);
    let request = item(&module, "pub struct ListShelvesRequest {");
    assert!(!request.contains("pub page:") && !request.contains("pub per_page:"), "{request}");
    let derives = &module[..module.find("pub struct ListShelvesRequest").unwrap()];
    let derive_line = derives.lines().rev().find(|line| line.starts_with("#[derive(")).unwrap();
    assert!(derive_line.contains("Default"), "{derive_line}");
    assert!(module.contains("ListShelvesRequest::default()"), "{module}");
}

#[test]
fn an_ordinary_send_carries_required_adopted_defaults_and_omits_optional_ones() {
    let shelves = get(
        "ListShelves",
        "/shelves",
        ApiResponse::json_vec_type("Shelf"),
        vec![param("page", true), param("per_page", true), param("draft", false)],
    );
    let mut books = get("ListBooks", "/books", ApiResponse::json_vec_type("Book"), vec![param("offset", true), param("limit", false)]);
    books.response = ApiResponse::json_type("BookList");
    let mut api = api(vec![shelves, books]);
    api.default_pagination = Some(
        Strategy::page_number("page", MoreSignal::NonEmptyPage)
            .page_size(PageSize::new("per_page").sent_default(50))
            .build(),
    );
    { let (id, collection) = (
        "ListBooks",
        Collection::at("books").item_type("Book").paginate(
            Strategy::offset("offset", MoreSignal::NonEmptyPage)
                .page_size(PageSize::new("limit").sent_default(20)),
        ),
    );  api.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };

    let module = module(&api);
    let shelves = squash(item(&module, "impl schematic_runtime::EndpointSpec for ListShelvesRequest"));
    assert!(
        shelves.contains(r#"letmutquery=query;query.extend([("page".to_owned(),schematic_runtime::QueryValue::from("1")),("per_page".to_owned(),schematic_runtime::QueryValue::from("50"))]);"#),
        "the first page and the authored size are sent:\n{shelves}"
    );
    let books = squash(item(&module, "impl schematic_runtime::EndpointSpec for ListBooksRequest"));
    assert!(
        books.contains(r#"query.extend([("offset".to_owned(),schematic_runtime::QueryValue::from("0"))]);"#),
        "a zero-based offset starts at zero:\n{books}"
    );
    assert!(!books.contains(r#""limit".to_owned()"#), "an optional adopted size is omitted:\n{books}");
}

#[test]
fn composite_inclusive_cursor_with_backward_leg_and_total() {
    let mut api = api(vec![get(
        "ListEvents",
        "/events",
        ApiResponse::json_vec_type("Event"),
        vec![param("after", false), param("after_id", false), param("before", false)],
    )]);
    api.default_pagination = Some(
        Strategy::cursor_leg(
            CursorLeg::single("after", CursorSource::last_item("created_at"))
                .tiebreak("after_id", CursorSource::last_item("id"))
                .inclusive("id"),
        )
        .backward("before", CursorSource::first_item("id"))
        .more_when(MoreSignal::has_more("has_more"))
        .resumable()
        .page_size(PageSize::new("limit").adjustable())
        .total(ValueSource::body("meta.total")),
    );

    let code = impls(&api);
    let squashed = squash(&code);
    for expected in [
        "typeKind=schematic_runtime::pager::CursorKind;",
        "forward:schematic_runtime::pager::spec::CursorLeg{parts:&[schematic_runtime::pager::spec::CursorPart{param:schematic_runtime::pager::spec::Input::Query(\"after\"),source:schematic_runtime::pager::spec::CursorSource::LastItem(&[\"created_at\"])},schematic_runtime::pager::spec::CursorPart{param:schematic_runtime::pager::spec::Input::Query(\"after_id\"),source:schematic_runtime::pager::spec::CursorSource::LastItem(&[\"id\"])}],boundary:schematic_runtime::pager::spec::Boundary::Inclusive{identity:&[\"id\"],},}",
        "backward:Some(schematic_runtime::pager::spec::CursorLeg{parts:&[schematic_runtime::pager::spec::CursorPart{param:schematic_runtime::pager::spec::Input::Query(\"before\"),source:schematic_runtime::pager::spec::CursorSource::FirstItem(&[\"id\"])}],boundary:schematic_runtime::pager::spec::Boundary::Exclusive})",
        "more:Some(schematic_runtime::pager::spec::MoreSignal::Flag{source:schematic_runtime::pager::spec::Source::Body(&[\"has_more\"]),polarity:schematic_runtime::pager::spec::Polarity::TrueMeansMore})",
        "resumable:true",
        "total:Some(schematic_runtime::pager::spec::Source::Body(&[\"meta\",\"total\"]))",
    ] {
        assert!(squashed.contains(&squash(expected)), "missing {expected} in:\n{code}");
    }
    let markers: Vec<&str> = code.lines().filter(|line| line.starts_with("impl ")).collect();
    assert_eq!(
        markers,
        [
            "impl schematic_runtime::pager::Paginated for ListEventsRequest {",
            "impl schematic_runtime::pager::ResumeAfter for ListEventsRequest {",
            "impl schematic_runtime::pager::ResumeBefore for ListEventsRequest {",
            "impl schematic_runtime::pager::PageSizeKnob for ListEventsRequest {}",
            "impl schematic_runtime::pager::AdjustablePageSize for ListEventsRequest {}",
            "impl schematic_runtime::pager::HasPrev for ListEventsRequest {}",
            "impl schematic_runtime::pager::HasTotal for ListEventsRequest {}",
        ]
    );
    assert!(squashed.contains("ResumeAfterforListEventsRequest{typeCursor=(String,String);}"));
    assert!(squashed.contains("ResumeBeforeforListEventsRequest{typeCursor=String;}"));

    let module = squash(&module(&api));
    assert!(module.contains(&squash(
        "pubfnafter(self,cursor:implInto<<ListEventsRequestasschematic_runtime::pager::ResumeAfter>::Cursor>)->schematic_runtime::pager::Pager<'a,ShelfApi,ListEventsRequest>{self.0.pager().after(cursor)}"
    )));
    assert!(module.contains("self.0.pager().before(cursor)"));
    assert!(!module.contains("pubafter:") && !module.contains("pubafter_id:") && !module.contains("pubbefore:"));
}

#[test]
fn next_link_with_body_prev_on_a_wrapper_list() {
    let mut api = api(vec![get("ListRepos", "/repositories/{workspace}", ApiResponse::json_type("RepoPage"), vec![])]);
    { let (id, collection) = (
        "ListRepos",
        Collection::at("values").item_type("Repo").paginate(
            Strategy::next_link(LinkSource::body("next")).prev(LinkSource::body("previous")),
        ),
    );  api.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };

    let code = squash(&impls(&api));
    for expected in [
        "typeItem=Repo;",
        "typeKind=schematic_runtime::pager::NextLinkKind;",
        "kind:schematic_runtime::pager::spec::Kind::NextLink{next:schematic_runtime::pager::spec::LinkSource::Field(schematic_runtime::pager::spec::Source::Body(&[\"next\"])),prev:Some(schematic_runtime::pager::spec::LinkSource::Field(schematic_runtime::pager::spec::Source::Body(&[\"previous\"]),),),}",
        "size:None",
        "record:&[schematic_runtime::pager::spec::Step::Key(\"values\"),schematic_runtime::pager::spec::Step::Each],projection:&[]}",
        "constPATH:&'staticstr=\"/repositories/{workspace}\";",
        "implschematic_runtime::pager::HasPrevforListReposRequest{}",
    ] {
        assert!(code.contains(&squash(expected)), "missing {expected} in:\n{code}");
    }
    assert!(!code.contains("PageSizeKnob"));

    let module = module(&api);
    let call = item(&module, "impl<'a> ListReposCall<'a> {");
    for absent in ["page_size", "pub fn page(", "pub fn after("] {
        assert!(!call.contains(absent), "`{absent}` is forwarded:\n{call}");
    }
    assert!(call.contains("pub fn pages(") && call.contains("pub fn collect_all("));
}

#[test]
fn one_based_offset_strategy() {
    let mut api = api(vec![get("ListLogs", "/logs", ApiResponse::json_vec_type("Log"), vec![param("start", false)])]);
    api.default_pagination =
        Some(Strategy::offset("start", MoreSignal::NonEmptyPage).one_based().build());

    let code = squash(&impls(&api));
    assert!(code.contains(&squash(
        "kind:schematic_runtime::pager::spec::Kind::Offset{offset:schematic_runtime::pager::spec::Input::Query(\"start\"),base:schematic_runtime::pager::spec::IndexBase::OneBased,more:schematic_runtime::pager::spec::MoreSignal::NonEmptyPage}"
    )));
    assert!(code.contains("typeKind=schematic_runtime::pager::OffsetKind;"));

    let module = squash(&module(&api));
    assert!(module.contains("pubfnoffset(self,offset:u64)->schematic_runtime::pager::Pager<'a,ShelfApi,ListLogsRequest>{self.0.pager().offset(offset)}"));
    assert!(!module.contains("pubstart:"));
}

#[test]
fn session_strategy_forwards_with_session_and_its_knobs_but_no_outside_scope_iteration() {
    let mut api = api(vec![get("Scroll", "/scroll", ApiResponse::json_vec_type("Hit"), vec![param("keep", false)])]);
    api.default_pagination = Some(
        Strategy::cursor("cursor", CursorSource::body("next"))
            .more_when(MoreSignal::has_more("more"))
            .page_size(PageSize::new("size"))
            .session(
                Session::opened_by_first_page()
                    .handle_from_page(ValueSource::body("scroll_id"))
                    .send_handle_as(InputParam::query("scroll_id"))
                    .keep_alive(KeepAlive::new("1m").on_page("keep").adjustable())
                    .renew(SessionStep::new(RestMethod::Post, "/scroll/renew").handle("scroll_id"))
                    .close(SessionStep::new(RestMethod::Delete, "/scroll").handle("scroll_id")),
            ),
    );

    let code = impls(&api);
    let squashed = squash(&code);
    assert!(squashed.contains("typeSession=schematic_runtime::pager::WithSession;"));
    assert!(squashed.contains(&squash(
        "renew:Some(schematic_runtime::pager::spec::SessionStep{method:schematic_runtime::Method::POST,path:\"/scroll/renew\",handle:Some(schematic_runtime::pager::spec::Input::Query(\"scroll_id\")),constants:&[],})"
    )));
    assert!(squashed.contains(&squash(
        "keep_alive:Some(schematic_runtime::pager::spec::KeepAlive{default:\"1m\",on_open:None,on_page:Some(schematic_runtime::pager::spec::Input::Query(\"keep\")),adjustable:true,})"
    )));
    let markers: Vec<&str> = code.lines().filter(|line| line.starts_with("impl ")).collect();
    assert_eq!(
        markers,
        [
            "impl schematic_runtime::pager::Paginated for ScrollRequest {",
            "impl schematic_runtime::pager::PageSizeKnob for ScrollRequest {}",
            "impl schematic_runtime::pager::KeepAliveKnob for ScrollRequest {}",
            "impl schematic_runtime::pager::Renewable for ScrollRequest {}",
        ]
    );

    let module = module(&api);
    let call = item(&module, "impl<'a> ScrollCall<'a> {");
    for present in ["pub fn pager(", "pub fn page_size(", "pub fn keep_alive(", "pub fn close_grace(", "pub fn with_session<"] {
        assert!(call.contains(present), "`{present}` is missing from a session call:\n{call}");
    }
    let squashed_call = squash(call);
    assert!(
        squashed_call.contains(&squash(
            "F: for<'s> FnOnce(&'s mut schematic_runtime::pager::SessionScope<'a, ShelfApi, ScrollRequest>) -> schematic_runtime::CallFuture<'s, O> + Send + 'a"
        )),
        "{call}"
    );
    // A session pages, renews, and partitions only inside its scope.
    for absent in ["pub fn pages(", "pub fn fetch_page(", "pub fn collect_all(", "renew", "partitioned"] {
        assert!(!call.contains(absent), "`{absent}` is forwarded on a session call:\n{call}");
    }
    assert!(!module.contains("pub keep:"), "the adjustable keep-alive adopts `keep`");
}

#[test]
fn a_partition_without_a_session_forwards_partitioned_and_with_one_does_not() {
    let partitioned = |session: Option<Session>| {
        let mut api = api(vec![get(
            "Scan",
            "/scan",
            ApiResponse::json_vec_type("Hit"),
            vec![param("slice", false), param("slices", false), param("partitioned", false)],
        )]);
        let strategy = Strategy::cursor("cursor", CursorSource::body("next"))
            .more_when(MoreSignal::has_more("more"))
            .partition(Partition::new(InputParam::query("slice"), InputParam::query("slices")).max(4));
        api.default_pagination = Some(match session {
            Some(session) => strategy.session(session),
            None => strategy,
        });
        module(&api)
    };

    let module = partitioned(None);
    let call = item(&module, "impl<'a> ScanCall<'a> {");
    assert!(call.contains("pub fn partitioned("), "{call}");
    assert!(call.contains("self.0.pager().partitioned(count)"), "{call}");
    // The setter of a plain parameter named like the forward is renamed.
    assert!(call.contains("pub fn partitioned_param("), "{call}");
    assert!(!call.contains("pub fn with_session<"), "{call}");
    assert!(
        module.contains("impl schematic_runtime::pager::Partitioned for ScanRequest {}"),
        "{module}"
    );

    let module = partitioned(Some(
        Session::opened_by(
            SessionStep::new(RestMethod::Post, "/pit"),
            ValueSource::body("id"),
        )
        .send_handle_as(InputParam::query("pit"))
        .close(SessionStep::new(RestMethod::Delete, "/pit").handle("pit")),
    ));
    let call = item(&module, "impl<'a> ScanCall<'a> {");
    assert!(call.contains("pub fn with_session<"), "{call}");
    // Partitions of a session are made inside its scope, so the call type
    // forwards none and its `partitioned` setter keeps its name.
    assert!(!call.contains("self.0.pager().partitioned"), "{call}");
    assert!(!call.contains("partitioned_param"), "{call}");
    assert!(
        module.contains("impl schematic_runtime::pager::Partitioned for ScanRequest {}"),
        "{module}"
    );
}

#[test]
fn unpaginated_lists_and_non_lists_emit_nothing_pagination_related() {
    let mut post = get("CreateShelf", "/shelves", ApiResponse::json_type("Shelf"), vec![]);
    post.method = RestMethod::Post;
    let plain = api(vec![
        get("ListShelves", "/shelves", ApiResponse::json_vec_type("Shelf"), vec![param("page", false)]),
        get("GetShelf", "/shelves/{id}", ApiResponse::json_type("Shelf"), vec![]),
        post,
    ]);
    let mut opted_out = plain.clone();
    opted_out.default_pagination =
        Some(Strategy::page_number("page", MoreSignal::NonEmptyPage).build());
    { let (id, collection) = ("ListShelves", Collection::root().unpaginated());  opted_out.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };

    for api in [&plain, &opted_out] {
        assert!(generate_pagination_impls(api, "Request").is_empty());
        let module = module(api);
        assert!(!module.contains("pager") && !module.contains("Paginated"), "{module}");
        assert!(module.contains("pub page: Option<i64>"), "{module}");
    }
    assert_eq!(module(&plain), module(&opted_out));
}

#[test]
fn alternates_do_not_change_the_output() {
    let strategy = || Strategy::page_number("page", MoreSignal::NonEmptyPage).build();
    let row = |collection: Collection| {
        let mut api = api(vec![get(
            "ListProjects",
            "/projects",
            ApiResponse::json_vec_type("Project"),
            vec![param("page", false), param("order_by", false)],
        )]);
        { let (id, collection) = ("ListProjects", collection);  api.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };
        api
    };
    let primary = row(Collection::root().paginate(strategy()));
    let with_alternate = row(Collection::root().paginate(strategy()).or_paginate(
        "keyset",
        Strategy::cursor("id_after", CursorSource::last_item("id"))
            .more_when(MoreSignal::NonEmptyPage)
            .constant("order_by", "id"),
    ));

    assert_eq!(module(&primary), module(&with_alternate));
    assert!(!module(&with_alternate).contains("keyset"));
}



#[test]
fn repeated_generation_is_identical() {
    let api = github_like();
    assert_eq!(module(&api), module(&api));
    assert_eq!(impls(&api), impls(&api));
}

#[test]
fn a_setter_named_like_a_pager_control_is_renamed() {
    let mut api = api(vec![get(
        "ListFiles",
        "/files",
        ApiResponse::json_vec_type("File"),
        vec![param("starting_after", false), param("after", false), param("page_size", false)],
    )]);
    api.default_pagination = Some(
        Strategy::cursor("starting_after", CursorSource::last_item("id"))
            .more_when(MoreSignal::has_more("has_more"))
            .resumable()
            .page_size(PageSize::new("limit"))
            .build(),
    );

    let names = resolve_names(&api);
    let setters: Vec<(&str, &str)> = names.endpoints[0]
        .setters
        .iter()
        .map(|setter| (setter.field.as_str(), setter.setter.as_str()))
        .collect();
    assert_eq!(setters, [("after", "after_param"), ("page_size", "page_size_param")]);
    assert!(names.renames.iter().all(|rename| rename.reason == RenameReason::PagerMethod));

    let warnings: Vec<String> = validate_rust_client(&[&api])
        .into_iter()
        .filter(|diagnostic| diagnostic.feature == Feature::EndpointMethodRename)
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert_eq!(
        warnings,
        [
            "setter `after` is generated as `after_param`: the endpoint's pager has a method of that name",
            "setter `page_size` is generated as `page_size_param`: the endpoint's pager has a method of that name",
        ]
    );

    let module = module(&api);
    assert!(module.contains("pub fn after_param(self, value: i64) -> Self"), "{module}");
    assert!(module.contains("pub fn page_size_param(self, value: i64) -> Self"), "{module}");
}

#[test]
fn an_unpaginated_endpoint_reserves_only_the_pager_entry_points() {
    let api = api(vec![get(
        "ListAll",
        "/all",
        ApiResponse::json_vec_type("Thing"),
        vec![param("pager", false), param("after", false), param("page_size", false)],
    )]);

    let names = resolve_names(&api);
    let setters: Vec<&str> = names.endpoints[0].setters.iter().map(|s| s.setter.as_str()).collect();
    assert_eq!(setters, ["pager_param", "after", "page_size"]);
    assert_eq!(names.renames[0].reason, RenameReason::SharedCallMethod);
}

#[test]
fn wrapper_item_types_are_emitted_as_authored() {
    for item_type in ["serde_json::Value", "crate::types::Thing", "Vec<String>"] {
        let mut api = api(vec![get("ListThings", "/things", ApiResponse::json_type("ThingPage"), vec![])]);
        { let (id, collection) = (
            "ListThings",
            Collection::at("data")
                .item_type(item_type)
                .paginate(Strategy::next_link(LinkSource::link_header("next"))),
        );  api.endpoints.iter_mut().find(|endpoint| endpoint.id == id).expect("known endpoint").pagination = Some(collection);  };

        let code = impls(&api);
        assert!(code.contains(&format!("    type Item = {item_type};\n")), "{code}");
    }
}
