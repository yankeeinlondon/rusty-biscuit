use std::collections::BTreeSet;

use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, ParamDef,
    ParamStyle, QueryParamType, RestApi, RestMethod,
};

use super::names::{API_CLIENT_METHODS, API_STRUCT_METHODS, SHARED_CALL_METHODS};
use super::*;
use crate::codegen::generate_api_struct;
use crate::codegen::request_structs::format_generated_code;
use crate::diagnostics::{Feature, Severity};
use crate::output::assemble::{assemble_api_module, assemble_split_api_module};
use crate::output::options::OutputOptions;
use crate::validation::validate_rust_client;

fn endpoint(id: &str, method: RestMethod, path: &str, description: &str) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: description.to_string(),
        request: None,
        response: ApiResponse::json_type("Widget"),
        headers: vec![],
        params: None,
        oauth_scopes: None, cross_origin: None, pagination: None,
    }
}

fn param(name: &str, param_type: QueryParamType, description: Option<&str>) -> ParamDef {
    ParamDef { rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required: false,
        description: description.map(str::to_string),
        param_type,
        explode: false,
        style: ParamStyle::Simple,
    }
}

fn api(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{name} API"),
        base_url: "https://api.example.com".to_string(),
        docs_url: None,
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints,
        module_path: None,
        request_suffix: None,
        version: None,
        cross_origin: CrossOrigin::default(),
        request_id_header: None,
        secret_names: vec![],
        default_pagination: None,
        pagination_exceptions: vec![],
    }
}

/// The fixture: required arguments in order, every optional-value location,
/// a body, a method named like an API-struct method, setters named like
/// shared call methods and a keyword, a Markdown description, and an
/// endpoint with no description.
fn widgets() -> RestApi {
    let mut get_widget = endpoint(
        "GetWidget",
        RestMethod::Get,
        "/owners/{owner}/widgets/{widget_id}",
        "Gets *one* widget: `code`, [a link](https://example.com), <b>tags</b> & snake_case_text.\n\nA second paragraph.",
    );
    get_widget.params = Some(EndpointParams {
        query: vec![
            param("limit", QueryParamType::Integer, Some("Results per\n   page.")),
            param("tag", QueryParamType::Array(Box::new(QueryParamType::String)), None),
        ],
        header: vec![param("X-Trace-Id", QueryParamType::String, None)],
        cookie: vec![param("session", QueryParamType::String, None)],
        ..Default::default()
    });

    let mut create_widget = endpoint("CreateWidget", RestMethod::Post, "/owners/{owner}/widgets", "Creates a widget.");
    create_widget.request = Some(ApiRequest::json_type("NewWidget"));

    let mut list_widgets = endpoint("ListWidgets", RestMethod::Get, "/widgets", "");
    list_widgets.params = Some(EndpointParams {
        query: vec![
            param("full", QueryParamType::Boolean, None),
            param("type", QueryParamType::String, None),
            param("build", QueryParamType::String, None),
            param("pages", QueryParamType::Integer, None),
        ],
        ..Default::default()
    });

    let new = endpoint("New", RestMethod::Post, "/new", "Makes something new.");
    let ping = endpoint("Ping", RestMethod::Get, "/ping", "Pings.");

    api("Widgets", vec![get_widget, create_widget, list_widgets, new, ping])
}

fn code(api: &RestApi) -> String {
    format_generated_code(&generate_endpoint_methods(api)).expect("generated endpoint methods parse")
}

/// `text` without whitespace and trailing commas, so an assertion does not
/// depend on where the formatter breaks a line.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<String>().replace(",)", ")").replace(",>", ">")
}

/// The names of the inherent `fn`s of `ty` in `code`.
fn inherent_fns(code: &str, ty: &str) -> BTreeSet<String> {
    inherent_fns_in(&syn::parse_file(code).expect("generated code parses"), ty)
}

fn inherent_fns_in(file: &syn::File, ty: &str) -> BTreeSet<String> {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(block) if block.trait_.is_none() => Some(block),
            _ => None,
        })
        .filter(|block| match &*block.self_ty {
            syn::Type::Path(path) => path.path.segments.last().is_some_and(|segment| segment.ident == ty),
            _ => false,
        })
        .flat_map(|block| &block.items)
        .filter_map(|item| match item {
            syn::ImplItem::Fn(function) => Some(function.sig.ident.to_string()),
            _ => None,
        })
        .collect()
}

/// The doc text of `ty::function` in `code`, one entry per `///` line.
fn fn_docs(code: &str, ty: &str, function: &str) -> Vec<String> {
    let file = syn::parse_file(code).expect("generated code parses");
    for item in &file.items {
        let syn::Item::Impl(block) = item else { continue };
        let syn::Type::Path(path) = &*block.self_ty else { continue };
        if block.trait_.is_some() || !path.path.segments.last().is_some_and(|segment| segment.ident == ty) {
            continue;
        }
        for item in &block.items {
            let syn::ImplItem::Fn(candidate) = item else { continue };
            if candidate.sig.ident != function {
                continue;
            }
            return candidate
                .attrs
                .iter()
                .filter_map(|attr| match &attr.meta {
                    syn::Meta::NameValue(doc) if doc.path.is_ident("doc") => match &doc.value {
                        syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) => Some(text.value()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
        }
    }
    panic!("{ty}::{function} is not generated:\n{code}");
}

fn setters_of(names: &ApiMethodNames, endpoint: &str) -> Vec<(String, String)> {
    names
        .endpoints
        .iter()
        .find(|names| names.endpoint_id == endpoint)
        .unwrap_or_else(|| panic!("no names for {endpoint}"))
        .setters
        .iter()
        .map(|setter| (setter.field.clone(), setter.setter.clone()))
        .collect()
}

fn owned(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

// ---- names and renames ----

#[test]
fn methods_are_snake_case_ids_and_call_types_follow_the_request_name() {
    let names = resolve_names(&widgets());
    let summary: Vec<(&str, &str, &str)> = names
        .endpoints
        .iter()
        .map(|names| (names.method.as_str(), names.call_type.as_str(), names.request_type.as_str()))
        .collect();
    assert_eq!(
        summary,
        [
            ("get_widget", "GetWidgetCall", "GetWidgetRequest"),
            ("create_widget", "CreateWidgetCall", "CreateWidgetRequest"),
            ("list_widgets", "ListWidgetsCall", "ListWidgetsRequest"),
            ("new_endpoint", "NewCall", "NewRequest"),
            ("ping", "PingCall", "PingRequest"),
        ]
    );
    assert!(names.conflicts.is_empty(), "{:?}", names.conflicts);
}

#[test]
fn a_request_suffix_carries_into_the_call_type_name() {
    for (suffix, expected) in [
        ("BasicRequest", "ListClientsBasicCall"),
        ("Params", "ListClientsParamsCall"),
        ("Request", "ListClientsCall"),
    ] {
        let mut api = api("Emqx", vec![endpoint("ListClients", RestMethod::Get, "/clients", "")]);
        api.request_suffix = Some(suffix.to_string());
        assert_eq!(resolve_names(&api).endpoints[0].call_type, expected, "suffix {suffix}");
    }
}

#[test]
fn setters_keep_field_names_and_shared_call_method_names_gain_param() {
    let names = resolve_names(&widgets());
    assert_eq!(
        setters_of(&names, "GetWidget"),
        owned(&[("limit", "limit"), ("tag", "tag"), ("x_trace_id", "x_trace_id"), ("session", "session")])
    );
    assert_eq!(
        setters_of(&names, "ListWidgets"),
        owned(&[
            ("full", "full_param"),
            ("type_", "type_"),
            ("build", "build_param"),
            ("pages", "pages_param"),
        ])
    );
}

#[test]
fn the_rename_list_names_every_rename_with_its_reason() {
    let renamed: Vec<String> = resolve_names(&widgets()).renames.iter().map(ToString::to_string).collect();
    assert_eq!(
        renamed,
        [
            "setter `full` is generated as `full_param`: every call type has a method of that name",
            "setter `build` is generated as `build_param`: every call type has a method of that name",
            "setter `pages` is generated as `pages_param`: every call type has a method of that name",
            "endpoint method `new` is generated as `new_endpoint`: the API struct has a method of that name",
        ]
    );
    let first = &resolve_names(&widgets()).renames[0];
    assert_eq!((first.api.as_str(), first.endpoint.as_str()), ("Widgets", "ListWidgets"));
    assert_eq!((first.target, first.reason), (RenameTarget::Setter, RenameReason::SharedCallMethod));
}

#[test]
fn a_keyword_endpoint_id_takes_the_field_sanitizer_suffix_and_is_listed() {
    let api = api("Widgets", vec![endpoint("Type", RestMethod::Get, "/type", "")]);
    let names = resolve_names(&api);
    assert_eq!(names.endpoints[0].method, "type_");
    assert_eq!(
        names.renames.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["endpoint method `type` is generated as `type_`: it is a Rust keyword or not an identifier"]
    );
}

#[test]
fn every_api_struct_and_trait_method_name_is_renamed() {
    for reserved in API_CLIENT_METHODS.iter().chain(API_STRUCT_METHODS).chain(&["clone", "fmt", "default"]) {
        let id: String = reserved
            .split('_')
            .map(|word| word[..1].to_uppercase() + &word[1..])
            .collect();
        let names = resolve_names(&api("Widgets", vec![endpoint(&id, RestMethod::Get, "/x", "")]));
        assert_eq!(names.endpoints[0].method, format!("{reserved}_endpoint"), "endpoint {id}");
    }
}

#[test]
fn two_endpoints_with_one_method_name_are_a_conflict_naming_both() {
    let api = api(
        "Widgets",
        vec![
            endpoint("GetAPIKey", RestMethod::Get, "/a", ""),
            endpoint("GetApikey", RestMethod::Get, "/b", ""),
            endpoint("New", RestMethod::Get, "/c", ""),
            endpoint("NewEndpoint", RestMethod::Get, "/d", ""),
        ],
    );
    let conflicts: Vec<String> = resolve_names(&api).conflicts.iter().map(ToString::to_string).collect();
    assert_eq!(
        conflicts,
        [
            "endpoint method `get_apikey` is generated for endpoints `GetAPIKey` and `GetApikey`; \
             rename one of the endpoint ids",
            "endpoint method `new_endpoint` is generated for endpoints `New` and `NewEndpoint`; \
             rename one of the endpoint ids",
        ]
    );
}

#[test]
fn a_renamed_setter_that_meets_another_field_is_a_conflict() {
    let mut list = endpoint("ListWidgets", RestMethod::Get, "/widgets", "");
    list.params = Some(EndpointParams::default().with_query_params(vec![
        ("full", QueryParamType::Boolean, false, None),
        ("full_param", QueryParamType::Boolean, false, None),
    ]));
    let conflicts = resolve_names(&api("Widgets", vec![list])).conflicts;
    assert_eq!(conflicts.len(), 1, "{conflicts:?}");
    assert_eq!(
        conflicts[0].to_string(),
        "setter `full_param` is generated for request fields `full` and `full_param`; rename one of \
         the parameters"
    );
}

#[test]
fn endpoint_method_renames_spans_every_api() {
    let other = api("Others", vec![endpoint("Send", RestMethod::Post, "/send", "")]);
    let widgets = widgets();
    let renames = endpoint_method_renames(&[&widgets, &other]);
    let summary: Vec<(&str, &str)> = renames.iter().map(|rename| (rename.api.as_str(), rename.to.as_str())).collect();
    assert_eq!(
        summary,
        [
            ("Widgets", "full_param"),
            ("Widgets", "build_param"),
            ("Widgets", "pages_param"),
            ("Widgets", "new_endpoint"),
            ("Others", "send_endpoint"),
        ]
    );
}

// ---- validation diagnostics ----

#[test]
fn renames_are_warnings_and_conflicts_are_errors_of_the_rust_output() {
    let diagnostics = validate_rust_client(&[&widgets()]);
    let rendered: Vec<String> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.feature == Feature::EndpointMethodRename)
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        rendered,
        [
            "severity=warning api=Widgets endpoint=ListWidgets feature=endpoint-method-rename output=rust: \
             setter `full` is generated as `full_param`: every call type has a method of that name",
            "severity=warning api=Widgets endpoint=ListWidgets feature=endpoint-method-rename output=rust: \
             setter `build` is generated as `build_param`: every call type has a method of that name",
            "severity=warning api=Widgets endpoint=ListWidgets feature=endpoint-method-rename output=rust: \
             setter `pages` is generated as `pages_param`: every call type has a method of that name",
            "severity=warning api=Widgets endpoint=New feature=endpoint-method-rename output=rust: \
             endpoint method `new` is generated as `new_endpoint`: the API struct has a method of that name",
        ]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic.severity == Severity::Warning),
        "renames block nothing: {diagnostics:#?}"
    );

    let clash = api(
        "Widgets",
        vec![endpoint("GetAPIKey", RestMethod::Get, "/a", ""), endpoint("GetApikey", RestMethod::Get, "/b", "")],
    );
    let errors: Vec<String> = validate_rust_client(&[&clash])
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        ["severity=error api=Widgets endpoint=GetApikey feature=generated-name-collision output=rust: \
          endpoint method `get_apikey` is generated for endpoints `GetAPIKey` and `GetApikey`; rename \
          one of the endpoint ids"]
    );
}

#[test]
fn a_call_type_name_shared_with_a_referenced_type_is_a_collision() {
    let mut list = endpoint("ListWidgets", RestMethod::Get, "/widgets", "");
    list.response = ApiResponse::json_type("ListWidgetsCall");
    let errors: Vec<String> = validate_rust_client(&[&api("Widgets", vec![list])])
        .iter()
        .filter(|diagnostic| diagnostic.feature == Feature::GeneratedNameCollision)
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("`ListWidgetsCall`") && errors[0].contains("call type"), "{errors:?}");
}

#[test]
fn call_types_of_two_apis_in_one_module_collide_only_without_distinct_suffixes() {
    let member = |name: &str, suffix: &str| {
        let mut api = api(name, vec![endpoint("ListClients", RestMethod::Get, "/clients", "")]);
        api.module_path = Some("emqx".to_string());
        api.request_suffix = Some(suffix.to_string());
        api
    };
    let (basic, bearer) = (member("EmqxBasic", "BasicRequest"), member("EmqxBearer", "BearerRequest"));
    assert!(validate_rust_client(&[&basic, &bearer]).is_empty());

    let (one, two) = (member("EmqxBasic", "Request"), member("EmqxBearer", "Request"));
    let messages: Vec<String> = validate_rust_client(&[&one, &two])
        .into_iter()
        .filter(|diagnostic| diagnostic.feature == Feature::GeneratedNameCollision)
        .map(|diagnostic| diagnostic.message)
        .collect();
    assert!(
        messages.iter().any(|message| message.starts_with("`ListClientsCall` is generated for the call type")),
        "{messages:?}"
    );
}

// ---- emitted code ----

#[test]
fn an_endpoint_method_takes_the_required_values_in_new_order_and_sends_nothing() {
    let code = squash(&code(&widgets()));
    for expected in [
        "implWidgets{",
        "pubfnget_widget(&self,owner:implInto<String>,widget_id:implInto<String>)->GetWidgetCall<'_>{\
         GetWidgetCall(self.0.send(GetWidgetRequest::new(owner,widget_id)))}",
        "pubfncreate_widget(&self,owner:implInto<String>,body:NewWidget)->CreateWidgetCall<'_>{\
         CreateWidgetCall(self.0.send(CreateWidgetRequest::new(owner,body)))}",
        "pubfnlist_widgets(&self)->ListWidgetsCall<'_>{ListWidgetsCall(self.0.send(ListWidgetsRequest::default()))}",
        "pubfnnew_endpoint(&self)->NewCall<'_>{NewCall(self.0.send(NewRequest::default()))}",
    ] {
        assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
    }
    let without_docs: String = self::code(&widgets())
        .lines()
        .filter(|line| !line.trim_start().starts_with("///"))
        .collect();
    assert!(!without_docs.contains(".await"), "an endpoint method sends nothing:\n{without_docs}");
}

#[test]
fn a_call_type_wraps_the_runtime_call_with_setters_and_shared_forwards() {
    let code = squash(&code(&widgets()));
    for expected in [
        "#[derive(Clone,Debug)]#[must_use=\"acallsendsnothinguntilitisawaited\"]\
         pubstructGetWidgetCall<'a>(schematic_runtime::Call<'a,WidgetsApi,GetWidgetRequest>);",
        "pubfnlimit(self,value:i64)->Self{Self(self.0.map_request(|request|request.with_limit(value)))}",
        "pubfntag(self,value:Vec<String>)->Self{Self(self.0.map_request(|request|request.with_tag(value)))}",
        "pubfnx_trace_id(self,value:String)->Self{Self(self.0.map_request(|request|request.with_x_trace_id(value)))}",
        "pubfnsession(self,value:String)->Self{Self(self.0.map_request(|request|request.with_session(value)))}",
        "pubfnfull_param(self,value:bool)->Self{Self(self.0.map_request(|request|request.with_full(value)))}",
        "pubfntype_(self,value:String)->Self{Self(self.0.map_request(|request|request.with_type_(value)))}",
        "pubfnanonymous(self)->Self{Self(self.0.anonymous())}",
        "pubfncredential(self,credential:schematic_runtime::Credential)->Self{Self(self.0.credential(credential))}",
        "pubfndeadline(self,deadline:&schematic_runtime::Deadline)->Self{Self(self.0.deadline(deadline))}",
        "pubfnfull(self)->schematic_runtime::FullCall<'a,WidgetsApi,GetWidgetRequest>{self.0.full()}",
        "pubfnbuild(self)->Result<Self,schematic_runtime::InvalidArgument>{self.0.build().map(Self)}",
        "impl<'a>::core::future::IntoFutureforGetWidgetCall<'a>{\
         typeOutput=<schematic_runtime::Call<'a,WidgetsApi,GetWidgetRequest>as::core::future::IntoFuture>::Output;\
         typeIntoFuture=schematic_runtime::CallFuture<'a,Self::Output>;\
         #[inline]fninto_future(self)->Self::IntoFuture{::core::future::IntoFuture::into_future(self.0)}}",
    ] {
        assert!(code.contains(expected), "missing `{expected}` in:\n{code}");
    }
}

#[test]
fn each_call_type_has_exactly_its_setters_and_the_shared_methods() {
    let code = code(&widgets());
    let shared: BTreeSet<String> = SHARED_CALL_METHODS.iter().map(ToString::to_string).collect();
    let expect = |setters: &[&str]| -> BTreeSet<String> {
        shared.iter().cloned().chain(setters.iter().map(ToString::to_string)).collect()
    };
    assert_eq!(inherent_fns(&code, "GetWidgetCall"), expect(&["limit", "tag", "x_trace_id", "session"]));
    assert_eq!(inherent_fns(&code, "ListWidgetsCall"), expect(&["full_param", "type_", "build_param", "pages_param"]));
    assert_eq!(inherent_fns(&code, "PingCall"), shared);
    assert_eq!(
        inherent_fns(&code, "Widgets"),
        ["create_widget", "get_widget", "list_widgets", "new_endpoint", "ping"].map(String::from).into()
    );
}

#[test]
fn an_api_with_no_endpoints_gains_an_empty_impl_and_no_call_types() {
    let code = code(&api("Empty", vec![]));
    assert!(!code.contains("struct"), "{code}");
    assert!(inherent_fns(&code, "Empty").is_empty());
}

// ---- rustdoc (R7) ----

#[test]
fn method_docs_carry_description_route_arguments_and_setters() {
    let docs = fn_docs(&code(&widgets()), "Widgets", "get_widget");
    assert_eq!(
        docs,
        [
            " Gets *one* widget: `code`, [a link](https://example.com), <b>tags</b> & snake_case_text.",
            "",
            " A second paragraph.",
            "",
            " `GET /owners/{owner}/widgets/{widget_id}`",
            "",
            " Returns a [`GetWidgetCall`]: chain optional values and per-call settings on it, then `.await` it to send.",
            "",
            " ## Arguments",
            "",
            " - `owner: impl Into<String>`: the `{owner}` path parameter",
            " - `widget_id: impl Into<String>`: the `{widget_id}` path parameter",
            "",
            " ## Optional values",
            "",
            " - [`limit`](GetWidgetCall::limit): the `limit` query parameter (`i64`). Results per page.",
            " - [`tag`](GetWidgetCall::tag): the `tag` query parameter (`Vec<String>`).",
            " - [`x_trace_id`](GetWidgetCall::x_trace_id): the `X-Trace-Id` header parameter (`String`).",
            " - [`session`](GetWidgetCall::session): the `session` cookie parameter (`String`).",
        ]
    );
}

#[test]
fn method_docs_list_the_body_and_omit_empty_sections() {
    let code = code(&widgets());
    assert_eq!(
        fn_docs(&code, "Widgets", "create_widget"),
        [
            " Creates a widget.",
            "",
            " `POST /owners/{owner}/widgets`",
            "",
            " Returns a [`CreateWidgetCall`]: chain optional values and per-call settings on it, then `.await` it to send.",
            "",
            " ## Arguments",
            "",
            " - `owner: impl Into<String>`: the `{owner}` path parameter",
            " - `body: NewWidget`: the request body",
        ]
    );
    assert_eq!(
        fn_docs(&code, "Widgets", "ping"),
        [
            " Pings.",
            "",
            " `GET /ping`",
            "",
            " Returns a [`PingCall`]: chain optional values and per-call settings on it, then `.await` it to send.",
        ]
    );
}

#[test]
fn a_missing_description_leaves_no_leading_blank_line() {
    let docs = fn_docs(&code(&widgets()), "Widgets", "list_widgets");
    assert_eq!(docs[0], " `GET /widgets`", "{docs:#?}");
    assert!(docs.contains(&" - [`full_param`](ListWidgetsCall::full_param): the `full` query parameter (`bool`).".to_string()));
}

#[test]
fn setter_and_call_type_docs_name_the_parameter_and_the_method() {
    let code = code(&widgets());
    assert_eq!(fn_docs(&code, "GetWidgetCall", "limit"), [" Sets the `limit` query parameter (`i64`). Results per page."]);
    assert!(
        squash(&code).contains(&squash(
            "/// A call to the `New` endpoint, returned by [`Widgets::new_endpoint`]."
        )),
        "{code}"
    );
}

#[test]
fn full_docs_say_the_endpoint_setters_come_before_it() {
    let code = code(&widgets());
    assert_eq!(
        fn_docs(&code, "GetWidgetCall", "full"),
        [
            " See [`Call::full`](schematic_runtime::Call::full). Chain it",
            " last: this call's setters are not available on the result.",
        ]
    );
    assert!(
        squash(&code).contains(&squash(
            "[`full`](Self::full), chained after the optional values, returns the whole response instead"
        )),
        "{code}"
    );
}

// ---- assembly ----

#[test]
fn a_module_holds_the_call_types_and_the_endpoint_methods() {
    let module = squash(&format_generated_code(&assemble_api_module(&widgets())).unwrap());
    assert!(module.contains("pubstructWidgets(schematic_runtime::Client<WidgetsApi>);"));
    assert!(module.contains("pubstructPingCall<'a>("));
    assert!(module.contains("pubfnping(&self)->PingCall<'_>"));
}

#[test]
fn a_split_module_puts_call_types_and_methods_beside_the_requests_and_imports_the_api_struct() {
    let parts = assemble_split_api_module(&widgets(), &OutputOptions::default());
    let requests = squash(&format_generated_code(&parts.requests_rs).unwrap());
    let module = squash(&format_generated_code(&parts.mod_rs).unwrap());
    assert!(requests.contains("usesuper::{Widgets,WidgetsApi};"), "{requests}");
    assert!(requests.contains("pubstructPingCall<'a>("), "{requests}");
    assert!(requests.contains("implWidgets{"), "{requests}");
    assert!(!module.contains("Call<'a>"), "{module}");
    assert!(module.contains("pubuserequests::*;"), "{module}");
}

#[test]
fn example_invocation_passes_path_parameters_and_the_body() {
    let api = widgets();
    let invocation = |id: &str| {
        let endpoint = api.endpoints.iter().find(|endpoint| endpoint.id == id).unwrap();
        example_invocation(&api, endpoint, "client")
    };
    assert_eq!(invocation("GetWidget"), r#"client.get_widget("owner", "widget_id")"#);
    assert_eq!(invocation("CreateWidget"), r#"client.create_widget("owner", body)"#);
    assert_eq!(invocation("New"), "client.new_endpoint()");
}

// ---- reserved names stay true to the code they reserve (ruling 6) ----

/// The `pub fn` names in `source`, read as text so a `macro_rules!` body
/// counts.
fn pub_fn_names(source: &str) -> BTreeSet<String> {
    source
        .split("pub fn ")
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect())
        .collect()
}

#[test]
fn api_client_methods_are_exactly_the_methods_api_client_emits() {
    let source = include_str!("../../../../runtime/src/api_client.rs");
    let macro_body = &source[source.find("macro_rules! api_client").expect("the macro is defined")..];
    let reserved: BTreeSet<String> = API_CLIENT_METHODS.iter().map(ToString::to_string).collect();
    assert_eq!(pub_fn_names(macro_body), reserved);
}

#[test]
fn api_struct_methods_are_exactly_the_methods_generate_api_struct_emits() {
    let mut basic = api("Widgets", vec![]);
    basic.auth = AuthStrategy::Basic;
    let code = format_generated_code(&generate_api_struct(&basic)).unwrap();
    let reserved: BTreeSet<String> = API_STRUCT_METHODS.iter().map(ToString::to_string).collect();
    assert_eq!(inherent_fns(&code, "Widgets"), reserved);
}

#[test]
fn shared_call_methods_are_methods_of_the_runtime_call() {
    let file = syn::parse_file(include_str!("../../../../runtime/src/call.rs")).expect("call.rs parses");
    let call = inherent_fns_in(&file, "Call");
    let full_call = inherent_fns_in(&file, "FullCall");
    for method in SHARED_CALL_METHODS {
        assert!(call.contains(*method), "`Call` has no `{method}`: {call:?}");
        assert!(
            *method == "full" || full_call.contains(*method),
            "`FullCall` has no `{method}`: {full_call:?}"
        );
    }
}

// ---- shipped definitions ----

/// Every `pub struct`, `pub enum`, and `pub type` name under `dir`.
fn public_type_names(dir: &std::path::Path, names: &mut BTreeSet<String>) {
    for entry in std::fs::read_dir(dir).expect("read the definitions source") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            public_type_names(&path, names);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = std::fs::read_to_string(&path).expect("read a definitions file");
            for keyword in ["pub struct ", "pub enum ", "pub type "] {
                names.extend(source.split(keyword).skip(1).map(|rest| {
                    rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>()
                }));
            }
        }
    }
}

/// A generated call type would silently shadow a definitions type of the
/// same name, which reaches each module through a glob re-export. Validation
/// only sees the types an endpoint references, so the shipped definitions
/// are checked here.
#[test]
fn no_shipped_call_type_shares_a_name_with_a_definitions_type() {
    let definitions = biscuit_test_harness::manifest_dir!().join("../definitions/src");
    let mut defined = BTreeSet::new();
    public_type_names(&definitions, &mut defined);
    assert!(defined.contains("Release"), "the scan finds definitions types");

    let apis = crate::pipeline::resolve_all_apis();
    let shadowing: Vec<String> = apis
        .iter()
        .flat_map(|api| resolve_names(api).endpoints)
        .map(|names| names.call_type)
        .filter(|call_type| defined.contains(call_type))
        .collect();
    assert!(shadowing.is_empty(), "call types that shadow definitions types: {shadowing:?}");
}
