//! GraphQL operations of a REST API through the public builders (R35, R39,
//! R40, R44): registration, naming, address rules, error policy, Relay
//! pagination into `variables`, and serialized forms.

use schematic_define::pagination::{Collection, CursorSource, InputParam, LinkSource, MoreSignal, PageSize, Strategy};
use schematic_define::{
    AddressRule, DefinitionContext, Endpoint, ErrorSet, ErrorShape, GraphQlOperation, GraphQlVariables, OperationKind, RestApi,
    StatusMatch, TypeRef, definition_type,
};
use serde_json::json;

definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct IssuesVars {
    pub owner: String, pub first: Option<u32>, pub after: Option<String> });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct IssuesData { pub repository: Option<serde_json::Value> });
definition_type!(#[derive(schemars::JsonSchema)] #[allow(dead_code)] pub struct Elsewhere { pub id: u64 });

const DOCUMENT: &str = "query Issues($owner: String!, $first: Int, $after: String) { repository(owner: $owner) { issues { nodes { id } } } }";

fn relay() -> Collection {
    Collection::at("repository.issues.nodes").item_type("Issue").paginate(
        Strategy::cursor(InputParam::body("variables.after"), CursorSource::body("repository.issues.pageInfo.endCursor"))
            .more_when(MoreSignal::has_more("repository.issues.pageInfo.hasNextPage"))
            .page_size(PageSize::new(InputParam::body("variables.first")).max(100)))
}

fn issues(ctx: &mut DefinitionContext) -> GraphQlOperation {
    GraphQlOperation::query("Issues", DOCUMENT).describes("Repository issues")
        .variables::<IssuesVars>(ctx).data::<IssuesData>(ctx).pagination(relay()).build().unwrap()
}

fn api(ctx: &DefinitionContext, address: Vec<AddressRule>, operations: Vec<GraphQlOperation>) -> Result<RestApi, String> {
    let endpoint = Endpoint::get("GetRepo", "/repos/{owner}").describes("Get a repository").returns_empty().build().unwrap();
    RestApi::builder(ctx, "GitHub", "https://ghe.example.com/prefix/api/v3").describes("GitHub").bearer_env(["GITHUB_TOKEN"])
        .endpoints([endpoint]).graphql(address).operations(operations).build().map_err(|error| error.to_string())
}

fn fails(result: Result<RestApi, String>, needle: &str) {
    let error = result.expect_err(needle);
    assert!(error.contains(needle), "{error}\n  expected {needle:?}");
}

#[test]
fn an_operation_registers_its_types_and_belongs_to_the_api() {
    let mut ctx = DefinitionContext::types_in("::models");
    let operation = issues(&mut ctx);
    assert_eq!(operation.kind, OperationKind::Query);
    assert_eq!(operation.document, DOCUMENT);
    assert_eq!(operation.variables, GraphQlVariables::Typed(TypeRef::Named("IssuesVars".into())));
    assert_eq!(operation.data, TypeRef::Named("IssuesData".into()));
    let viewer = GraphQlOperation::mutation("Touch", "mutation { touch }").describes("Touch").no_variables()
        .data::<serde_json::Value>(&mut ctx).operation_name("").build().unwrap();
    assert_eq!((viewer.kind, viewer.variables.clone(), viewer.operation_name.as_deref()), (OperationKind::Mutation, GraphQlVariables::Unit, Some("")));

    let built = api(&ctx, AddressRule::github(), vec![operation.clone(), viewer]).unwrap();
    let surface = built.graphql.as_ref().unwrap();
    assert_eq!(surface.address, AddressRule::github());
    assert_eq!(surface.operations[0], operation);
    assert!(built.component("IssuesVars").is_some() && built.component("IssuesData").is_some());
    // An unchanged REST API carries no GraphQL surface.
    let plain = RestApi::builder(&ctx, "Plain", "https://api.example.com").describes("Plain").no_auth().build().unwrap();
    assert!(plain.graphql.is_none());
}

#[test]
fn an_unregistered_data_or_variables_type_is_a_validation_error_naming_the_operation() {
    let mut ctx = DefinitionContext::types_in("::models");
    let mut other = DefinitionContext::types_in("::other");
    let data = GraphQlOperation::query("Foreign", "{ a }").describes("Foreign data").no_variables()
        .data::<Elsewhere>(&mut other).build().unwrap();
    fails(api(&ctx, AddressRule::github(), vec![data]), "GitHub operation Foreign: unresolved schema reference \"Elsewhere\"");
    let variables = GraphQlOperation::query("ForeignVars", "{ a }").describes("Foreign variables")
        .variables::<Elsewhere>(&mut other).data::<IssuesData>(&mut ctx).build().unwrap();
    fails(api(&ctx, AddressRule::github(), vec![variables]), "GitHub operation ForeignVars: unresolved schema reference \"Elsewhere\"");
    let error = GraphQlOperation::query("Q", "{ a }").describes("Q").no_variables()
        .data::<std::collections::HashMap<bool, IssuesData>>(&mut ctx).build().unwrap_err().to_string();
    assert!(error.contains("unsupported map key bool"), "{error}");
}

#[test]
fn operation_facts_ids_and_variables_shape_are_checked() {
    let mut ctx = DefinitionContext::types_in("::models");
    let error = GraphQlOperation::query(" ", "{ a }").describes("Blank").no_variables().data::<IssuesData>(&mut ctx).build().unwrap_err();
    assert_eq!(error.to_string(), "<operation> operation  : id and description must be nonempty");
    let error = GraphQlOperation::query("List", "{ a }").describes("List").variables::<Vec<IssuesVars>>(&mut ctx)
        .data::<IssuesData>(&mut ctx).build().unwrap_err();
    assert!(error.to_string().contains("variables must be a named type or map serializing to a JSON object"), "{error}");
    // An empty or unparsable document is sent unchanged, so it builds.
    assert!(GraphQlOperation::query("Raw", "").describes("Empty").no_variables().data::<IssuesData>(&mut ctx).build().is_ok());
    assert!(GraphQlOperation::query("Bad", "query {").describes("Bad").no_variables().data::<IssuesData>(&mut ctx).build().is_ok());

    let one = GraphQlOperation::query("Issues", "{ a }").describes("One").no_variables().data::<IssuesData>(&mut ctx).build().unwrap();
    fails(api(&ctx, AddressRule::github(), vec![one.clone(), one.clone()]), "GitHub operation Issues: operation id is declared more than once");
    let clash = GraphQlOperation::query("GetRepo", "{ a }").describes("Clash").no_variables().data::<IssuesData>(&mut ctx).build().unwrap();
    fails(api(&ctx, AddressRule::github(), vec![clash]), "GitHub operation GetRepo: operation id equals an endpoint id");
}

#[test]
fn address_rules_are_ordered_segment_paths_on_a_query_free_base() {
    let ctx = DefinitionContext::types_in("::models");
    assert!(api(&ctx, AddressRule::github(), vec![]).is_ok());
    assert!(api(&ctx, vec![AddressRule::append("/graphql/")], vec![]).is_ok());
    fails(api(&ctx, vec![], vec![]), "GitHub endpoint <api>: graphql: address needs at least one rule");
    for path in ["graphql", "/", "", "/a//b", "/graphql?x=1", "/graphql#top"] {
        fails(api(&ctx, vec![AddressRule::append(path)], vec![]), &format!("address rule path `{path}` must start with `/`"));
        fails(api(&ctx, vec![AddressRule::replace_suffix(path, "/api/graphql")], vec![]), &format!("address rule path `{path}`"));
    }
    let mut ctx_ops = DefinitionContext::types_in("::models");
    let operation = issues(&mut ctx_ops);
    let error = RestApi::builder(&ctx_ops, "Api", "https://api.example.com/").describes("Api").no_auth()
        .operations([operation.clone()]).build().unwrap_err().to_string();
    assert!(error.contains("graphql: address needs at least one rule"), "{error}");
    let error = RestApi::builder(&ctx_ops, "Api", "https://api.example.com/v3?tenant=a").describes("Api").no_auth()
        .graphql(AddressRule::github()).operations([operation]).build().unwrap_err().to_string();
    assert!(error.contains("a base URL with a query or fragment has no GraphQL address"), "{error}");
}

#[test]
fn an_operation_error_policy_overlays_api_defaults_or_opts_out() {
    let mut ctx = DefinitionContext::types_in("::models");
    let overlay = GraphQlOperation::query("Overlay", "{ a }").describes("Overlay").no_variables().data::<IssuesData>(&mut ctx)
        .errors(ErrorSet::new().on(StatusMatch::exact(403), ErrorShape::empty())).build().unwrap();
    let opted_out = GraphQlOperation::query("OptOut", "{ a }").describes("Opt out").no_variables().data::<IssuesData>(&mut ctx)
        .errors(ErrorSet::none()).build().unwrap();
    let endpoint = Endpoint::get("GetRepo", "/repos").describes("Get").returns_empty().build().unwrap();
    let built = RestApi::builder(&ctx, "GitHub", "https://api.github.com").describes("GitHub").no_auth()
        .default_errors(ErrorSet::new().on(StatusMatch::class(4), ErrorShape::text())).endpoints([endpoint])
        .graphql(AddressRule::github()).operations([overlay, opted_out]).build().unwrap();
    let operations = &built.graphql.as_ref().unwrap().operations;
    let overlay = built.effective_operation_errors(&operations[0]);
    assert_eq!(overlay.lookup(403).unwrap().shape, schematic_define::ResponseShape::Empty);
    assert_eq!(overlay.lookup(401).unwrap().shape, schematic_define::ResponseShape::Text);
    assert!(built.effective_operation_errors(&operations[1]).entries().is_empty());
    let error = GraphQlOperation::query("Both", "{ a }").describes("Both").no_variables().data::<IssuesData>(&mut ctx)
        .errors(ErrorSet::none()).errors(ErrorSet::new().on(StatusMatch::exact(403), ErrorShape::text())).build().unwrap_err();
    assert!(error.to_string().contains("operation Both: opts out of declared errors and also declares error entries"), "{error}");
}

#[test]
fn relay_pagination_writes_variables_and_reads_from_data() {
    let mut ctx = DefinitionContext::types_in("::models");
    let paged = |collection: Collection, ctx: &mut DefinitionContext| GraphQlOperation::query("Issues", DOCUMENT).describes("Issues")
        .variables::<IssuesVars>(ctx).data::<IssuesData>(ctx).pagination(collection).build();
    assert!(paged(relay(), &mut ctx).is_ok());

    let cursor = |param: InputParam| Strategy::cursor(param, CursorSource::body("repository.issues.pageInfo.endCursor"))
        .more_when(MoreSignal::has_more("repository.issues.pageInfo.hasNextPage"));
    let cases: Vec<(Collection, &str)> = vec![
        (Collection::at("nodes").item_type("Issue").paginate(cursor(InputParam::query("after"))), "pager input `after` must be a request-body path under `variables`"),
        (Collection::at("nodes").item_type("Issue").paginate(cursor(InputParam::body("after"))), "pager input `after` must be a request-body path under `variables`"),
        (Collection::at("nodes").item_type("Issue").paginate(cursor(InputParam::body("variables"))), "pager input `variables` must be a request-body path"),
        (Collection::at("nodes").item_type("Issue").paginate(cursor(InputParam::body("variables.after")).page_size(PageSize::new("first"))),
            "pager input `first` must be a request-body path"),
        (Collection::at("nodes").item_type("Issue"), "pagination must declare its strategy"),
        (Collection::at("nodes").item_type("Issue").unpaginated(), "pagination must declare its strategy"),
        (Collection::at("nodes").paginate(cursor(InputParam::body("variables.after"))), "pagination needs an item type"),
        (Collection::at("nodes").item_type("Issue").paginate(Strategy::next_link(LinkSource::body("next"))), "a next-link strategy has no link to follow"),
        (Collection::at("nodes[").item_type("Issue").paginate(cursor(InputParam::body("variables.after"))), "a key is empty or contains `[` or `]`"),
    ];
    for (collection, needle) in cases {
        let error = paged(collection, &mut ctx).unwrap_err().to_string();
        assert!(error.contains(needle), "{error}\n  expected {needle:?}");
    }
    let error = GraphQlOperation::query("Unit", "{ a }").describes("Unit").no_variables().data::<IssuesData>(&mut ctx)
        .pagination(relay()).build().unwrap_err();
    assert!(error.to_string().contains("a paginated operation needs typed variables"), "{error}");
}

#[test]
fn graphql_members_keep_their_frozen_serialized_forms() {
    let mut ctx = DefinitionContext::types_in("::models");
    let operation = GraphQlOperation::query("Viewer", "{ viewer { login } }").describes("Viewer").no_variables()
        .data::<IssuesData>(&mut ctx).build().unwrap();
    assert_eq!(serde_json::to_value(&operation).unwrap(), json!({"id": "Viewer", "description": "Viewer", "kind": "Query",
        "document": "{ viewer { login } }", "variables": "Unit", "data": {"Named": "IssuesData"}}));
    assert_eq!(serde_json::to_value(AddressRule::github()).unwrap(),
        json!([{"ReplaceSuffix": {"suffix": "/api/v3", "with": "/api/graphql"}}, {"Append": "/graphql"}]));
    let named = issues(&mut ctx);
    let value = serde_json::to_value(&named).unwrap();
    assert_eq!(value["variables"], json!({"Typed": {"Named": "IssuesVars"}}));
    assert_eq!(serde_json::from_value::<GraphQlOperation>(value).unwrap(), named);
    let mut with_name = operation.clone();
    with_name.operation_name = Some(String::new());
    assert_eq!(serde_json::to_value(&with_name).unwrap()["operation_name"], json!(""));
    let mut null_name = serde_json::to_value(&operation).unwrap();
    null_name["operation_name"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<GraphQlOperation>(null_name).is_err());
}
