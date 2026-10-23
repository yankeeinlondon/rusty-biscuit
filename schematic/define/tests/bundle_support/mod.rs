//! A typed fixture API whose bundle exercises every schema form a context
//! registers (recursive roots, a nested type recursing into its root,
//! containers, an alias, arbitrary JSON, a composition) and the additive
//! vocabulary (typed paths, deprecation, metadata, endpoint authentication,
//! ordered Basic lists, all three OAuth2 grants, declared errors at both
//! levels with an opt-out, a paginated GraphQL operation, and success shapes:
//! a JSON + SSE + fallback set with notes and variant names, tagged JSON
//! shapes beside a byte stream and a 204 empty shape, and an NDJSON stream).

use std::collections::HashMap;

use schematic_define::pagination::{Collection, CursorSource, InputParam, MoreSignal, PageSize, Strategy};
use schematic_define::{
    AddressRule, AuthMethod, AuthPolicy, AuthorizationCodeGrant, Backoff, ClientCredentialsGrant, DefinitionBundle,
    DefinitionContext, DefinitionError, DeviceCodeGrant, Endpoint, EnvAuthStrategy, ErrorSet, ErrorShape, GraphQlOperation,
    OAuth2ClientAuthMethod, OAuth2Config, ParamDef, PkceRequirement, QueryParamType, RateLimit, ResetUnit, RestApi,
    RetryGuidance, StatusMatch, StatusSelector, Success, definition_type, rust_path,
};
use schemars::JsonSchema;

pub mod models {
    use super::*;

    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Node { pub value: i64, pub children: Vec<Node> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Parent { pub name: String, pub child: Option<Box<Child>> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Child { pub parent: Option<Box<Parent>> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Item {
        /// Item identifier.
        #[schemars(example = &"item-1")]
        pub id: String,
        pub note: Option<String>,
    });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Cat { pub meows: bool });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Dog { pub barks: u8 });
    definition_type!(#[derive(JsonSchema)] #[serde(untagged)] #[allow(dead_code)] pub enum Pet { Cat(Cat), Dog(Dog) });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Problem { pub message: String });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct IssuesVars { pub owner: String, pub first: Option<u32>, pub after: Option<String> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct IssuesData { pub issues: Vec<Item> });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Reply { pub kind: String, pub text: String });
    definition_type!(#[derive(JsonSchema)] #[allow(dead_code)] pub struct Event { pub kind: String });
    #[derive(JsonSchema)]
    #[allow(dead_code)]
    pub struct Widget { pub size: u32 }
    impl schematic_define::Registration for Widget {
        fn register(ctx: &mut DefinitionContext) -> Result<schematic_define::TypeDesc, DefinitionError> {
            ctx.register_as::<Self>(rust_path!(crate::bundle_support::models::Widget).schema_name("Thing"))
        }
    }
}
use models::*;

fn rate_limit(prefix: &str) -> RateLimit {
    RateLimit::new(format!("{prefix}-limit"), format!("{prefix}-remaining"), format!("{prefix}-reset"), ResetUnit::EpochSeconds)
        .unwrap()
        .with_retry_after_header("retry-after")
        .unwrap()
}

fn grants() -> [OAuth2Config; 3] {
    [
        OAuth2Config::AuthorizationCodePkce(AuthorizationCodeGrant {
            authorization_url: "https://auth.example.com/authorize".into(), token_url: "https://auth.example.com/token".into(),
            revocation_url: Some("https://auth.example.com/revoke".into()), default_scopes: vec!["read".into(), "write".into()],
            pkce: PkceRequirement::Supported, client_auth: OAuth2ClientAuthMethod::None }),
        OAuth2Config::ClientCredentials(ClientCredentialsGrant {
            token_url: "https://auth.example.com/token".into(), revocation_url: None, default_scopes: vec![],
            client_auth: OAuth2ClientAuthMethod::ClientSecretPost }),
        OAuth2Config::DeviceCode(DeviceCodeGrant {
            device_authorization_url: "https://auth.example.com/device".into(), token_url: "https://auth.example.com/token".into(),
            revocation_url: None, default_scopes: vec!["read".into()], client_auth: OAuth2ClientAuthMethod::ClientSecretBasic }),
    ]
}

/// The fixture API and its context's structural catalog.
pub fn fixture() -> (RestApi, schematic_define::ModelCatalog) {
    let mut ctx = DefinitionContext::types_in("::fixture::models");
    let node = Endpoint::get("GetNode", "/nodes/{id}").describes("Get a node tree")
        .path_param(ParamDef::path("id", QueryParamType::Integer).describes("Node id"))
        .rate_limit(rate_limit("node"))
        .errors(ErrorSet::new().on(StatusMatch::exact(404), ErrorShape::empty()).on(StatusMatch::range(420, 429), ErrorShape::text().content_type("text/html")))
        .returns_json::<Node>(&mut ctx).build().unwrap();
    let parents = Endpoint::get("ListParents", "/parents").describes("List parents").deprecated()
        .query(ParamDef::new("order", QueryParamType::Enum(vec!["asc".into(), "desc".into()])).deprecated())
        .returns_json::<Vec<Parent>>(&mut ctx).build().unwrap();
    let lookup = Endpoint::get("Lookup", "/lookup").describes("Items by key")
        .retry_guidance(RetryGuidance::new([StatusSelector::Code(503)], false, 2, Backoff::Fixed { delay_ms: 100 }).unwrap())
        .auth(AuthPolicy { explicit: vec![AuthMethod::Basic], env_fallback: None })
        .returns_json::<HashMap<String, Item>>(&mut ctx).build().unwrap();
    let raw = Endpoint::get("Raw", "/raw").describes("Arbitrary JSON").public().errors(ErrorSet::none())
        .returns_json::<serde_json::Value>(&mut ctx).build().unwrap();
    let (cat, dog) = (ctx.register::<Cat>().unwrap(), ctx.register::<Dog>().unwrap());
    ctx.one_of::<Pet>([cat, dog]).unwrap();
    let pets = Endpoint::post("CreatePet", "/pets").describes("Create a pet")
        .request_json::<Pet>(&mut ctx)
        .returns_json::<Option<Widget>>(&mut ctx).build().unwrap();
    let problem = ctx.register::<Problem>().unwrap();
    let issues = GraphQlOperation::query("Issues", "query Issues($owner: String!, $first: Int, $after: String) { issues { id } }")
        .describes("Issues of an owner").variables::<IssuesVars>(&mut ctx).data::<IssuesData>(&mut ctx).operation_name("Issues")
        .errors(ErrorSet::new().on(StatusMatch::exact(403), ErrorShape::json(problem.clone())))
        .pagination(Collection::at("issues").item_type("Item").paginate(
            Strategy::cursor(InputParam::body("variables.after"), CursorSource::body("pageInfo.endCursor"))
                .more_when(MoreSignal::has_more("pageInfo.hasNextPage")).page_size(PageSize::new(InputParam::body("variables.first")).max(100))))
        .build().unwrap();
    let viewer = GraphQlOperation::mutation("Touch", "").describes("Touch nothing").no_variables()
        .data::<serde_json::Value>(&mut ctx).errors(ErrorSet::none()).build().unwrap();
    let (reply, event) = (ctx.register::<Reply>().unwrap(), ctx.register::<Event>().unwrap());
    let converse = Endpoint::post("Converse", "/converse").describes("Reply whole or as events")
        .returns(Success::json(reply.clone()).note("when stream is false").variant("Whole"))
        .returns(Success::sse(event.clone()).ends_at_event("done").keep_end().note("when stream is true"))
        .returns(Success::binary().fallback().variant("Attachment")).build().unwrap();
    let submit = Endpoint::post("Submit", "/submit").describes("Tagged replies, audio, or nothing")
        .returns(Success::json(reply).tagged("kind", "reply")).returns(Success::json(event.clone()).tagged("kind", "event"))
        .returns(Success::bytes().content_type("audio/mpeg")).returns(Success::empty().status(204)).build().unwrap();
    let follow = Endpoint::get("Follow", "/follow").describes("Events per line")
        .returns(Success::ndjson(event).ends_when_field("kind", "end").status(200)).build().unwrap();
    let api = RestApi::builder(&ctx, "Fixture", "https://api.example.com/v1")
        .describes("Bundle fixture API")
        .basic_env("FIXTURE_USER", ["FIXTURE_PASSWORD", "FIXTURE_PASSWORD_OLD"])
        .auth_policy(AuthPolicy {
            explicit: std::iter::once(AuthMethod::Basic).chain(grants().map(AuthMethod::OAuth2)).collect(),
            env_fallback: Some(EnvAuthStrategy::Basic),
        })
        .header("X-Fixture-Version", "2026-10-21")
        .rate_limit(rate_limit("api"))
        .retry_guidance(RetryGuidance::new([StatusSelector::Code(429), StatusSelector::Range { start: 500, end: 599 }], true, 3,
            Backoff::Exponential { base_ms: 250, cap_ms: 8000 }).unwrap())
        .default_errors(ErrorSet::new().on(StatusMatch::class(5), ErrorShape::json(problem)).on(StatusMatch::exact(404), ErrorShape::text()))
        .endpoints([node, parents, lookup, raw, pets, converse, submit, follow])
        .graphql(AddressRule::github()).operations([issues, viewer])
        .build().unwrap();
    (api, ctx.models().clone())
}

/// The fixture as the bundle tool builds it.
pub fn fixture_bundle() -> DefinitionBundle {
    let (api, models) = fixture();
    DefinitionBundle::new(api).unwrap().with_models(models).unwrap()
}
