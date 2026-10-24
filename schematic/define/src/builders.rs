//! Required-fact REST authoring. Typed registrations lower to component
//! references; API build stores the context's components and checks that
//! every reference resolves to one.
//!
//! ```compile_fail
//! use schematic_define::Endpoint;
//! Endpoint::default();
//! ```
//! ```compile_fail
//! use schematic_define::RestApi;
//! RestApi::default();
//! ```
//! ```compile_fail
//! use schematic_define::Endpoint;
//! Endpoint::get("Get", "/").returns_empty().build();
//! ```
//! ```compile_fail
//! use schematic_define::Endpoint;
//! Endpoint::get("Get", "/").describes("Get").build();
//! ```
//! ```compile_fail
//! use schematic_define::{RestApi, DefinitionContext};
//! let ctx = DefinitionContext::types_in("::models");
//! RestApi::builder(&ctx, "Api", "https://example.com").no_auth().build();
//! ```
//! ```compile_fail
//! use schematic_define::{RestApi, DefinitionContext};
//! let ctx = DefinitionContext::types_in("::models");
//! RestApi::builder(&ctx, "Api", "https://example.com").describes("Api").build();
//! ```
//! ```compile_fail
//! schematic_define::ErrorSet::status(400);
//! ```

use std::marker::PhantomData;
use serde_json::Value;
use crate::error_table::{validate_error_entries, validate_error_policy};
use crate::response::validate_success_shapes;
use crate::{AddressRule, ErrorEntry, GraphQlOperation, GraphQlSurface, JsonTag, ResponseShape, StatusMatch, StreamEnd, StreamFraming, SuccessShape, TypeRef};
use crate::{ApiRequest, AuthMethod, AuthPolicy, AuthStrategy, CrossOrigin, DefinitionContext,
    DefinitionError, Endpoint, EndpointAuth, EndpointParams, EnvAuthStrategy, EnvList, EnvMapping, FormField,
    ParamDef, ParamStyle, QueryParamType, RateLimit, Registration, RestApi, RestMethod, RetryGuidance, TypeDesc};

/// A required fact has not been declared.
pub struct Missing;
/// A required fact has been declared.
pub struct Present;

/// One success shape and its selection facts; `.returns` may repeat.
///
/// A stream ends at connection close unless an `ends_*` modifier names a
/// terminator. A JSON shape whose registered type is a sequence or a map is
/// tagged as an array or object when another JSON shape shares its status and
/// media type, unless it declares a tag.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Success { shape: SuccessShape, inferred: Option<JsonTag>, misuse: Option<&'static str> }
#[allow(missing_docs)]
impl Success {
    fn of(shape: ResponseShape) -> Self { Self { shape: SuccessShape::new(shape), inferred: None, misuse: None } }
    fn stream(framing: StreamFraming) -> Self {
        Self::of(ResponseShape::Stream { framing, end: StreamEnd::ConnectionClose, keep: false })
    }
    pub fn json(desc: TypeDesc) -> Self {
        let inferred = match desc.type_ref() {
            TypeRef::Array(_) => Some(JsonTag::Array),
            TypeRef::Map(_) | TypeRef::KeyedMap { .. } => Some(JsonTag::Object),
            _ => None,
        };
        Self { inferred, ..Self::of(ResponseShape::Json(desc.into_type_ref())) }
    }
    pub fn text() -> Self { Self::of(ResponseShape::Text) }
    pub fn binary() -> Self { Self::of(ResponseShape::Binary) }
    pub fn empty() -> Self { Self::of(ResponseShape::Empty) }
    /// Server-sent events whose data decodes as `item`.
    pub fn sse(item: TypeDesc) -> Self { Self::stream(StreamFraming::Sse(item.into_type_ref())) }
    /// One JSON `item` per line.
    pub fn ndjson(item: TypeDesc) -> Self { Self::stream(StreamFraming::Ndjson(item.into_type_ref())) }
    /// Raw byte chunks.
    pub fn bytes() -> Self { Self::stream(StreamFraming::Bytes) }
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self { self.shape.content_type = Some(content_type.into()); self }
    /// Selects this shape by an exact 2xx status (R3a); may repeat.
    pub fn status(mut self, status: u16) -> Self { self.shape.statuses.push(status); self }
    /// Selects this JSON shape by a top-level field's value (R3b).
    pub fn tagged(mut self, name: impl Into<String>, value: impl Into<Value>) -> Self {
        self.shape.json_tag = Some(JsonTag::Field { name: name.into(), value: value.into() }); self
    }
    pub fn json_array(mut self) -> Self { self.shape.json_tag = Some(JsonTag::Array); self }
    pub fn json_object(mut self) -> Self { self.shape.json_tag = Some(JsonTag::Object); self }
    /// Selected for any unmatched media type (R3c).
    pub fn fallback(mut self) -> Self { self.shape.fallback = true; self }
    /// When this shape is returned; variant rustdoc and export only.
    pub fn note(mut self, note: impl Into<String>) -> Self { self.shape.note = Some(note.into()); self }
    /// Names this shape's variant of the generated success enum (D-15).
    pub fn variant(mut self, name: impl Into<String>) -> Self { self.shape.variant = Some(name.into()); self }
    pub fn ends_at_data_line(self, data: impl Into<String>) -> Self { self.ends(StreamEnd::DataLineEquals(data.into())) }
    pub fn ends_at_event(self, name: impl Into<String>) -> Self { self.ends(StreamEnd::EventNamed(name.into())) }
    /// Ends at the item whose field at dotted `path` equals `value`.
    pub fn ends_when_field(self, path: impl Into<String>, value: impl Into<Value>) -> Self {
        self.ends(StreamEnd::FieldEquals { field: path.into(), value: value.into() })
    }
    /// Delivers the terminator as the last item instead of consuming it.
    pub fn keep_end(mut self) -> Self {
        match &mut self.shape.shape {
            ResponseShape::Stream { keep, .. } => *keep = true,
            _ => self.misuse = Some("`keep_end` applies only to a stream shape"),
        }
        self
    }
    fn ends(mut self, terminator: StreamEnd) -> Self {
        match &mut self.shape.shape {
            ResponseShape::Stream { end, .. } => *end = terminator,
            _ => self.misuse = Some("an `ends_*` terminator applies only to a stream shape"),
        }
        self
    }
}

/// An error body's shape and optional declared media type.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ErrorShape { shape: ResponseShape, content_type: Option<String> }
#[allow(missing_docs)]
impl ErrorShape {
    fn of(shape: ResponseShape) -> Self { Self { shape, content_type: None } }
    pub fn json(desc: TypeDesc) -> Self { Self::of(ResponseShape::Json(desc.into_type_ref())) }
    pub fn text() -> Self { Self::of(ResponseShape::Text) }
    pub fn binary() -> Self { Self::of(ResponseShape::Binary) }
    pub fn empty() -> Self { Self::of(ResponseShape::Empty) }
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self { self.content_type = Some(content_type.into()); self }
}

/// Declared error entries, or an opt-out from every one.
///
/// On an endpoint or operation, [`ErrorSet::none`] opts out of the API
/// defaults as well; on an API it declares no defaults.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ErrorSet { entries: Vec<ErrorEntry>, none: bool }
impl ErrorSet {
    /// No declared errors; provider responses still use runtime error handling.
    pub fn none() -> Self { Self { entries: vec![], none: true } }
    /// An empty set to add entries to.
    #[allow(clippy::new_without_default)] // public `Default` impls are an allow-listed set
    pub fn new() -> Self { Self { entries: vec![], none: false } }
    /// Decode statuses `status` matches as `shape`.
    pub fn on(mut self, status: StatusMatch, shape: ErrorShape) -> Self {
        self.entries.push(ErrorEntry { content_type: shape.content_type, ..ErrorEntry::new(status, shape.shape) }); self
    }
    pub(crate) fn into_parts(self) -> (Vec<ErrorEntry>, bool) { (self.entries, self.none) }
}

impl FormField {
    /// Typed JSON form metadata, validated with other endpoint references at API build.
    pub fn json_desc(name: impl Into<String>, desc: TypeDesc) -> Self {
        Self::json(name, desc.into_type_ref())
    }
}

fn invalid(api: &str, endpoint: &str, message: impl Into<String>) -> DefinitionError {
    DefinitionError::InvalidDefinition { api: api.into(), endpoint: endpoint.into(), message: message.into() }
}

/// Description and success are independent required facts. Build records
/// registration errors instead of allowing later setters to conceal them.
pub struct EndpointBuilder<D = Missing, S = Missing> {
    endpoint: Endpoint,
    // Lowered to `endpoint.response` at build.
    successes: Vec<Success>,
    auth_choices: usize,
    error: Option<DefinitionError>,
    state: PhantomData<(D, S)>,
}
#[allow(missing_docs)]
impl Endpoint {
    // Construction returns typestate so required facts cannot be omitted.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(id: impl Into<String>, method: RestMethod, path: impl Into<String>) -> EndpointBuilder {
        EndpointBuilder { endpoint: Endpoint { id: id.into(), method, path: path.into(),
            description: String::new(), request: None, request_constraints: None, response: vec![],
            headers: vec![], params: None, cross_origin: None, pagination: None, rate_limit: None, retry_guidance: None,
            deprecated: false, auth: None, errors: vec![], no_declared_errors: false }, successes: vec![], auth_choices: 0, error: None, state: PhantomData }
    }
    pub fn get(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Get, path) }
    pub fn post(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Post, path) }
    pub fn put(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Put, path) }
    pub fn patch(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Patch, path) }
    pub fn delete(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Delete, path) }
    pub fn head(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Head, path) }
    pub fn options(id: impl Into<String>, path: impl Into<String>) -> EndpointBuilder { Self::new(id, RestMethod::Options, path) }
}
#[allow(missing_docs)]
impl<D, S> EndpointBuilder<D, S> {
    fn cast<ND, NS>(self) -> EndpointBuilder<ND, NS> {
        EndpointBuilder { endpoint: self.endpoint, successes: self.successes, auth_choices: self.auth_choices, error: self.error, state: PhantomData }
    }
    pub fn describes(mut self, description: impl Into<String>) -> EndpointBuilder<Present, S> {
        self.endpoint.description = description.into(); self.cast()
    }
    pub fn returns(mut self, success: Success) -> EndpointBuilder<D, Present> {
        self.successes.push(success); self.cast()
    }
    pub fn returns_json<T: Registration>(mut self, ctx: &mut DefinitionContext) -> EndpointBuilder<D, Present> {
        match ctx.register_for::<T>("<endpoint>", &self.endpoint.id) {
            Ok(desc) => {
                let mut success = Success::json(desc);
                if success.inferred.is_none() && schemars::schema_for!(T).get("type") == Some(&Value::from("object")) {
                    success.inferred = Some(JsonTag::Object);
                }
                self.returns(success)
            }
            Err(error) => { if self.error.is_none() { self.error = Some(error); } self.cast() }
        }
    }
    pub fn returns_text(self) -> EndpointBuilder<D, Present> { self.returns(Success::text()) }
    pub fn returns_binary(self) -> EndpointBuilder<D, Present> { self.returns(Success::binary()) }
    pub fn returns_empty(self) -> EndpointBuilder<D, Present> { self.returns(Success::empty()) }
    /// Adds entries overlaying the API defaults, or opts out with
    /// [`ErrorSet::none`]; opting out and adding entries fails at build.
    pub fn errors(mut self, errors: ErrorSet) -> Self {
        let (entries, none) = errors.into_parts();
        self.endpoint.errors.extend(entries);
        self.endpoint.no_declared_errors |= none;
        self
    }
    pub fn request_json<T: Registration>(mut self, ctx: &mut DefinitionContext) -> Self {
        match ctx.register_for::<T>("<endpoint>", &self.endpoint.id) {
            Ok(desc) => self.endpoint.request = Some(ApiRequest::Json(desc.into_type_ref())),
            Err(error) => if self.error.is_none() { self.error = Some(error); },
        }
        self
    }
    pub fn request_form(mut self, fields: Vec<FormField>) -> Self { self.endpoint.request = Some(ApiRequest::FormData { fields }); self }
    pub fn request_urlencoded(mut self, fields: Vec<FormField>) -> Self { self.endpoint.request = Some(ApiRequest::UrlEncoded { fields }); self }
    pub fn request_text(mut self) -> Self { self.endpoint.request = Some(ApiRequest::Text { content_type: "text/plain".into() }); self }
    pub fn request_text_with_content_type(mut self, content_type: impl Into<String>) -> Self { self.endpoint.request = Some(ApiRequest::Text { content_type: content_type.into() }); self }
    pub fn request_binary_with_content_type(mut self, content_type: impl Into<String>) -> Self { self.endpoint.request = Some(ApiRequest::Binary { content_type: content_type.into() }); self }
    /// Rules on the request body's root value: a text body, or a JSON body
    /// whose root is not a schema component (a component's rules are
    /// Schemars attributes on its model). Checked at build.
    pub fn request_constraints(mut self, rules: crate::constraints::Constraints) -> Self { self.endpoint.request_constraints = Some(Box::new(rules)); self }
    pub fn request_binary(mut self) -> Self { self.endpoint.request = Some(ApiRequest::Binary { content_type: "application/octet-stream".into() }); self }
    pub fn pagination(mut self, collection: crate::pagination::Collection) -> Self { self.endpoint.pagination = Some(collection); self }
    pub fn cross_origin(mut self, policy: CrossOrigin) -> Self { self.endpoint.cross_origin = Some(policy); self }
    pub fn header_param(mut self, param: ParamDef) -> Self { self.endpoint.params.get_or_insert_with(EndpointParams::new).header.push(param); self }
    pub fn cookie(mut self, param: ParamDef) -> Self { self.endpoint.params.get_or_insert_with(EndpointParams::new).cookie.push(param); self }
    pub fn params(mut self, params: EndpointParams) -> Self { self.endpoint.params = Some(params); self }
    pub fn query(mut self, param: ParamDef) -> Self { self.endpoint.params.get_or_insert_with(EndpointParams::new).query.push(param); self }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self { self.endpoint.headers.push((name.into(), value.into())); self }
    /// Declares a typed path parameter, forced required, `Simple` and not
    /// exploded. Once any is declared, build requires one per placeholder.
    pub fn path_param(mut self, param: ParamDef) -> Self {
        let param = ParamDef { required: true, style: ParamStyle::Simple, explode: false, ..param };
        self.endpoint.params.get_or_insert_with(EndpointParams::new).path.push(param); self
    }
    /// Replaces the API rate-limit declaration whole for this endpoint.
    pub fn rate_limit(mut self, rate_limit: RateLimit) -> Self { self.endpoint.rate_limit = Some(rate_limit); self }
    /// Replaces the API retry guidance whole for this endpoint.
    pub fn retry_guidance(mut self, guidance: RetryGuidance) -> Self { self.endpoint.retry_guidance = Some(guidance); self }
    pub fn deprecated(mut self) -> Self { self.endpoint.deprecated = true; self }
    /// No authentication for this endpoint. One of `public`, `auth`, and
    /// `auth_with_env_mapping` at most; a second fails at build.
    pub fn public(self) -> Self { self.override_auth(EndpointAuth::Public) }
    /// Replaces the API policy, reading credentials through the API's
    /// effective environment mapping, which must suit the policy's fallback.
    pub fn auth(self, policy: AuthPolicy) -> Self { self.override_auth(EndpointAuth::Policy { policy, env_mapping: None }) }
    /// Replaces the API policy and the API environment mapping whole.
    pub fn auth_with_env_mapping(self, policy: AuthPolicy, env_mapping: EnvMapping) -> Self {
        self.override_auth(EndpointAuth::Policy { policy, env_mapping: Some(env_mapping) })
    }
    fn override_auth(mut self, auth: EndpointAuth) -> Self { self.auth_choices += 1; self.endpoint.auth = Some(auth); self }
}
impl EndpointBuilder<Present, Present> {
    /// Lower to the existing endpoint model. Endpoint-only vocabulary rules
    /// (typed paths, metadata, the auth override) are checked here and again
    /// at API build; API build alone checks references and credential sources.
    pub fn build(self) -> Result<Endpoint, DefinitionError> {
        if let Some(error) = self.error { return Err(error); }
        let mut ep = self.endpoint;
        if let Some(misuse) = self.successes.iter().find_map(|success| success.misuse) { return Err(invalid("<endpoint>", &ep.id, misuse)); }
        let shapes = infer_json_tags(self.successes);
        validate_success_shapes(&shapes).map_err(|message| invalid("<endpoint>", &ep.id, message))?;
        ep.response = shapes;
        if ep.id.trim().is_empty() || ep.description.trim().is_empty() || !ep.path.starts_with('/') {
            return Err(invalid("<endpoint>", &ep.id, "id and description must be nonempty and path must start with /"));
        }
        if ep.path.contains('?') {
            return Err(invalid("<endpoint>", &ep.id, format!("path `{}` contains a query; declare query parameters separately", ep.path)));
        }
        if ep.method == RestMethod::Get && ep.request.is_some() {
            return Err(invalid("<endpoint>", &ep.id, "GET endpoint cannot declare a request body"));
        }
        if self.auth_choices > 1 {
            return Err(invalid("<endpoint>", &ep.id, "endpoint authentication must be declared at most once (public, auth, or auth_with_env_mapping)"));
        }
        validate_endpoint_vocabulary("<endpoint>", &ep)?;
        Ok(ep)
    }
}

/// Applies each undeclared shape's inferred array or object tag when another
/// JSON shape shares its media type and an applicable status.
fn infer_json_tags(successes: Vec<Success>) -> Vec<SuccessShape> {
    let shares = |a: &SuccessShape, b: &SuccessShape| matches!((&a.shape, &b.shape), (ResponseShape::Json(_), ResponseShape::Json(_)))
        && a.media_type() == b.media_type()
        && (a.statuses.is_empty() && b.statuses.is_empty() || a.statuses.iter().any(|status| b.statuses.contains(status)));
    let shapes: Vec<SuccessShape> = successes.iter().map(|success| success.shape.clone()).collect();
    successes.into_iter().enumerate().map(|(index, success)| {
        let mut shape = success.shape;
        if shape.json_tag.is_none() && shapes.iter().enumerate().any(|(other, peer)| other != index && shares(&shape, peer)) {
            shape.json_tag = success.inferred;
        }
        shape
    }).collect()
}

/// API description and authentication must be stated before build. The context
/// is borrowed after endpoints are constructed, leaving registration ergonomic.
pub struct RestApiBuilder<'a, D = Missing, A = Missing> {
    ctx: &'a DefinitionContext,
    api: RestApi,
    auth_choices: usize,
    // Resolved against the declared authentication at build, so
    // `env_username` may be called before or after it.
    username: Option<String>,
    error: Option<DefinitionError>,
    state: PhantomData<(D, A)>,
}
impl RestApi {
    /// Start a definition in its destination module-group context.
    pub fn builder(ctx: &DefinitionContext, name: impl Into<String>, base_url: impl Into<String>) -> RestApiBuilder<'_> {
        RestApiBuilder { ctx, api: RestApi { name: name.into(), base_url: base_url.into(), description: String::new(),
            docs_url: None, auth: AuthStrategy::None, auth_policy: None, env_auth: vec![],
            headers: vec![], endpoints: vec![], module_path: None, request_suffix: None, version: None,
            env_mapping: None, cross_origin: CrossOrigin::same_origin(), request_id_header: None, secret_names: vec![],
            default_pagination: None, rate_limit: None, retry_guidance: None, errors: vec![], graphql: None, schemas: vec![] }, auth_choices: 0, username: None, error: None, state: PhantomData }
    }
}
#[allow(missing_docs)]
impl<'a, D, A> RestApiBuilder<'a, D, A> {
    fn cast<ND, NA>(self) -> RestApiBuilder<'a, ND, NA> {
        RestApiBuilder { ctx: self.ctx, api: self.api, auth_choices: self.auth_choices, username: self.username, error: self.error, state: PhantomData }
    }
    pub fn describes(mut self, description: impl Into<String>) -> RestApiBuilder<'a, Present, A> { self.api.description = description.into(); self.cast() }
    /// `env` is the `env_auth` chain for token and key strategies. Basic
    /// carries its own lists, so a nonempty `env` with Basic fails at build.
    pub fn authentication<I, N>(mut self, auth: AuthStrategy, env: I) -> RestApiBuilder<'a, D, Present>
    where I: IntoIterator<Item = N>, N: Into<String> {
        self.auth_choices += 1; self.api.auth = auth; self.api.env_auth = env.into_iter().map(Into::into).collect(); self.cast()
    }
    pub fn no_auth(self) -> RestApiBuilder<'a, D, Present> { self.authentication(AuthStrategy::None, std::iter::empty::<String>()) }
    pub fn bearer_env<I, N>(self, names: I) -> RestApiBuilder<'a, D, Present> where I: IntoIterator<Item = N>, N: Into<String> {
        self.authentication(AuthStrategy::BearerToken { header: None }, names)
    }
    pub fn basic_env<I, N>(self, username: impl Into<String>, passwords: I) -> RestApiBuilder<'a, D, Present> where I: IntoIterator<Item = N>, N: Into<String> {
        let auth = AuthStrategy::Basic { username_env: EnvList::single(username),
            password_env: EnvList::new(passwords.into_iter().map(Into::into).collect()) };
        self.authentication(auth, std::iter::empty::<String>())
    }
    /// Names the Basic username variable. At build, with Basic authentication
    /// it fills an empty `username_env` and must otherwise equal it; with any
    /// other authentication it is accepted only when the explicit
    /// `env_mapping` names exactly this `basic_user`, since it has nowhere
    /// else to be stored.
    pub fn env_username(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if self.username.as_ref().is_some_and(|old| old != &name) {
            self.error = Some(invalid(&self.api.name, "<api>", "username environment conflicts with authentication declaration"));
        }
        self.username = Some(name); self
    }
    pub fn env_mapping(mut self, mapping: EnvMapping) -> Self { self.api.env_mapping = Some(mapping); self }
    pub fn auth_policy(mut self, policy: AuthPolicy) -> Self { self.api.auth_policy = Some(policy); self }
    pub fn endpoints(mut self, endpoints: impl IntoIterator<Item = Endpoint>) -> Self { self.api.endpoints.extend(endpoints); self }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self { self.api.headers.push((name.into(), value.into())); self }
    pub fn docs_url(mut self, url: impl Into<String>) -> Self { self.api.docs_url = Some(url.into()); self }
    pub fn version(mut self, version: impl Into<String>) -> Self { self.api.version = Some(version.into()); self }
    pub fn module_path(mut self, path: impl Into<String>) -> Self { self.api.module_path = Some(path.into()); self }
    pub fn request_suffix(mut self, suffix: impl Into<String>) -> Self { self.api.request_suffix = Some(suffix.into()); self }
    pub fn cross_origin(mut self, policy: CrossOrigin) -> Self { self.api.cross_origin = policy; self }
    pub fn request_id_header(mut self, name: impl Into<String>) -> Self { self.api.request_id_header = Some(name.into().to_ascii_lowercase()); self }
    pub fn secret_header(mut self, name: impl Into<String>) -> Self { self.api.secret_names.push(name.into().to_ascii_lowercase()); self }
    pub fn default_pagination(mut self, strategy: crate::pagination::Strategy) -> Self { self.api.default_pagination = Some(strategy); self }
    /// Rate-limit headers inherited by every endpoint without its own.
    pub fn rate_limit(mut self, rate_limit: RateLimit) -> Self { self.api.rate_limit = Some(rate_limit); self }
    /// Retry guidance inherited by every endpoint without its own.
    pub fn retry_guidance(mut self, guidance: RetryGuidance) -> Self { self.api.retry_guidance = Some(guidance); self }
    /// Error entries every endpoint and operation inherits.
    /// [`ErrorSet::none`] here adds nothing: an empty API table is not an
    /// endpoint opt-out.
    pub fn default_errors(mut self, errors: ErrorSet) -> Self { self.api.errors.extend(errors.into_parts().0); self }
    /// Declares a GraphQL surface at `address`, replacing earlier rules.
    pub fn graphql(mut self, address: impl IntoIterator<Item = AddressRule>) -> Self {
        let address: Vec<AddressRule> = address.into_iter().collect();
        match &mut self.api.graphql {
            Some(surface) => surface.address = address,
            None => self.api.graphql = Some(GraphQlSurface::new(address)),
        }
        self
    }
    /// Adds GraphQL operations; build fails unless [`Self::graphql`] gave
    /// the surface an address.
    pub fn operations(mut self, operations: impl IntoIterator<Item = GraphQlOperation>) -> Self {
        self.api.graphql.get_or_insert_with(|| GraphQlSurface::new([])).operations.extend(operations); self
    }
}
impl RestApiBuilder<'_, Present, Present> {
    /// Validate coherent authentication, every typed reference, and the
    /// vocabulary of the API and each endpoint, without weakening
    /// output-specific generator validation for legacy fixtures.
    pub fn build(self) -> Result<RestApi, DefinitionError> {
        let schemas = self.ctx.components()?;
        if let Some(error) = self.error { return Err(error); }
        let mut api = self.api;
        api.schemas = schemas;
        let fail = |message: &str| invalid(&api.name, "<api>", message);
        if self.auth_choices != 1 { return Err(fail("authentication must be declared exactly once")); }
        if let AuthStrategy::Basic { username_env, .. } = &mut api.auth {
            if let Some(name) = self.username {
                let declared = EnvList::single(name);
                if username_env.names().is_empty() { *username_env = declared; }
                else if *username_env != declared { return Err(fail("username environment conflicts with authentication declaration")); }
            }
        } else if let Some(name) = self.username
            && api.env_mapping.as_ref().and_then(|mapping| mapping.basic_user.as_ref()) != Some(&EnvList::single(name)) {
            return Err(fail("username environment requires Basic authentication or a matching explicit basic_user mapping"));
        }
        validate_api(&api)?;
        Ok(api)
    }
}

/// Every rule a built API satisfies that its fields alone can show; API
/// build and [`crate::DefinitionBundle::from_json`] run it.
pub(crate) fn validate_api(api: &RestApi) -> Result<(), DefinitionError> {
    let fail = |message: &str| invalid(&api.name, "<api>", message);
    if matches!(api.auth, AuthStrategy::Basic { .. }) && !api.env_auth.is_empty() {
        return Err(fail("Basic authentication carries its own environment lists; env_auth must be empty"));
    }
    let url = url::Url::parse(&api.base_url).map_err(|_| fail("invalid base URL"))?;
    if api.name.trim().is_empty() || api.description.trim().is_empty() || !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(fail("name and description must be nonempty and base URL must be HTTP(S)"));
    }
    if let Some(mapping) = &api.env_mapping
        && env_mapping_conflicts(&api.auth, &api.env_auth, mapping) {
        return Err(fail("explicit environment mapping conflicts with authentication declaration"));
    }
    validate_origin(&api.cross_origin).map_err(|message| fail(&message))?;
    // Fields are public and deserializable, so values built through the
    // validating constructors may since have been mutated.
    if let Some(rate_limit) = &api.rate_limit { rate_limit.validate().map_err(|error| fail(&format!("rate_limit: {error}")))?; }
    if let Some(guidance) = &api.retry_guidance { guidance.validate().map_err(|error| fail(&format!("retry_guidance: {error}")))?; }
    // The OpenAPI importer builds grants without validation; this is the
    // definition's gate.
    if let AuthStrategy::OAuth2(config) = &api.auth { config.validate().map_err(|error| fail(&format!("auth: {error}")))?; }
    if let Some(policy) = &api.auth_policy { validate_oauth2(policy).map_err(|error| fail(&format!("auth_policy: {error}")))?; }
    if let Some(name) = &api.request_id_header
        && !valid_header_name(name) { return Err(fail("invalid request-id header name")); }
    for name in &api.secret_names {
        if !valid_header_name(name) { return Err(fail("invalid secret header name")); }
    }
    validate_error_entries(&api.errors).map_err(|message| fail(&message))?;
    if let Some(surface) = &api.graphql { crate::graphql::validate_surface(api, surface)?; }
    if let Some(diagnostic) = crate::pagination::validate_pagination(api).into_iter().next() {
        return Err(invalid(&api.name, diagnostic.endpoint.as_deref().unwrap_or("<api>"),
            format!("{}: {}", diagnostic.error.rule(), diagnostic.error)));
    }
    for ep in &api.endpoints {
        if ep.path.contains('?') {
            return Err(invalid(&api.name, &ep.id, format!("path `{}` contains a query; declare query parameters separately", ep.path)));
        }
        if ep.method == RestMethod::Get && ep.request.is_some() {
            return Err(invalid(&api.name, &ep.id, "GET endpoint cannot declare a request body"));
        }
        if let Some(policy) = &ep.cross_origin {
            validate_origin(policy).map_err(|message| invalid(&api.name, &ep.id, message))?;
        }
        validate_endpoint_vocabulary(&api.name, ep)?;
        validate_error_policy(&api.errors, &ep.errors, ep.no_declared_errors).map_err(|message| invalid(&api.name, &ep.id, message))?;
        if let Some(message) = credential_source_problem(api, ep) { return Err(invalid(&api.name, &ep.id, message)); }
        if let Some(params) = &ep.params {
            for param in params.query.iter().chain(&params.header).chain(&params.cookie) {
                if let Some(name) = &param.rust_name
                    && !valid_rust_name(name) { return Err(invalid(&api.name, &ep.id, "invalid Rust parameter name")); }
            }
        }
    }
    if let Some(problem) = crate::constraints::component_problems(api).into_iter().next() {
        return Err(DefinitionError::InvalidConstraint { api: api.name.clone(), problem: Box::new(problem) });
    }
    api.validate_references()?;
    api.validate_schema_closure()
}

/// Rules needing only the endpoint. Endpoint build runs them, and API build
/// again, since every field stays public and deserializable.
fn validate_endpoint_vocabulary(api: &str, ep: &Endpoint) -> Result<(), DefinitionError> {
    let fail = |message: String| invalid(api, &ep.id, message);
    if let Some(rate_limit) = &ep.rate_limit { rate_limit.validate().map_err(|error| fail(format!("rate_limit: {error}")))?; }
    if let Some(guidance) = &ep.retry_guidance { guidance.validate().map_err(|error| fail(format!("retry_guidance: {error}")))?; }
    if let Some(EndpointAuth::Policy { policy, .. }) = &ep.auth {
        if policy.explicit.is_empty() && policy.env_fallback.is_none() {
            return Err(fail("endpoint auth policy accepts no authentication method; use public() for an unauthenticated endpoint".into()));
        }
        validate_oauth2(policy).map_err(|error| fail(format!("endpoint auth policy: {error}")))?;
    }
    validate_success_shapes(&ep.response).map_err(&fail)?;
    validate_error_policy(&[], &ep.errors, ep.no_declared_errors).map_err(&fail)?;
    validate_typed_paths(ep).map_err(fail)?;
    match crate::constraints::endpoint_problems(ep).into_iter().next() {
        Some(problem) => Err(DefinitionError::InvalidConstraint { api: api.into(), problem: Box::new(problem) }),
        None => Ok(()),
    }
}

fn validate_oauth2(policy: &AuthPolicy) -> Result<(), crate::OAuth2ConfigError> {
    for method in &policy.explicit {
        if let AuthMethod::OAuth2(config) = method { config.validate()?; }
    }
    Ok(())
}

/// Typed path declarations (an empty list keeps legacy untyped placeholders).
///
/// Field collisions compare the explicit `rust_name`, else the wire name with
/// each character outside `[A-Za-z0-9_]` replaced by `_`. That is coarser
/// than the generator's identifier sanitizer (no snake-casing), so the
/// generator's `generated-name-collision` diagnostic remains the full check.
fn validate_typed_paths(ep: &Endpoint) -> Result<(), String> {
    let Some(params) = ep.params.as_ref().filter(|params| !params.path.is_empty()) else { return Ok(()) };
    let placeholders = ep.path_placeholders();
    let mut declared: Vec<&str> = Vec::new();
    for param in &params.path {
        let name = param.name.as_str();
        if declared.contains(&name) { return Err(format!("path parameter `{name}` is declared more than once")); }
        declared.push(name);
        if !placeholders.contains(&name) { return Err(format!("path parameter `{name}` has no `{{{name}}}` placeholder in `{}`", ep.path)); }
        if !param.required { return Err(format!("path parameter `{name}` must be required")); }
        if param.style != ParamStyle::Simple { return Err(format!("path parameter `{name}` must use the Simple style, not {:?}", param.style)); }
        match &param.param_type {
            QueryParamType::Array(_) | QueryParamType::Json => {
                return Err(format!("path parameter `{name}` must be a scalar or enum, not an array or JSON value"));
            }
            QueryParamType::Enum(values) if values.is_empty() => return Err(format!("path parameter `{name}` declares an enum with no values")),
            QueryParamType::Enum(values) => {
                if let Some(value) = values.iter().enumerate().find_map(|(index, value)| values[..index].contains(value).then_some(value)) {
                    return Err(format!("path parameter `{name}` declares enum value `{value}` more than once"));
                }
            }
            _ => {}
        }
        if let Some(rust_name) = &param.rust_name
            && !valid_rust_name(rust_name) { return Err(format!("path parameter `{name}` has invalid Rust name `{rust_name}`")); }
    }
    if let Some(missing) = placeholders.iter().find(|placeholder| !declared.contains(placeholder)) {
        return Err(format!("path placeholder `{{{missing}}}` has no declared path parameter"));
    }
    let field = |param: &ParamDef| param.rust_name.clone().unwrap_or_else(|| {
        param.name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
    });
    let others = [("path", &params.path), ("query", &params.query), ("header", &params.header), ("cookie", &params.cookie)];
    for (index, path_param) in params.path.iter().enumerate() {
        let path_field = field(path_param);
        for (location, defs) in others {
            let skip = if location == "path" { index + 1 } else { 0 };
            if let Some(other) = defs.iter().skip(skip).find(|other| field(other) == path_field) {
                return Err(format!("path parameter `{}` and {location} parameter `{}` both generate the Rust field `{path_field}`",
                    path_param.name, other.name));
            }
        }
    }
    Ok(())
}

/// Why a replacement policy's environment fallback has no usable source, if
/// it has none. The source is the endpoint mapping if declared, else the
/// API's effective mapping.
pub(crate) fn credential_source_problem(api: &RestApi, ep: &Endpoint) -> Option<String> {
    let Some(EndpointAuth::Policy { policy, env_mapping }) = &ep.auth else { return None };
    let fallback = policy.env_fallback.as_ref()?;
    let (source, mapping) = match env_mapping {
        Some(mapping) => ("the endpoint env_mapping", mapping.clone()),
        None => ("the API's effective env mapping", api.default_env_mapping()),
    };
    let listed = |list: &Option<EnvList>| list.as_ref().is_some_and(|list| !list.names().is_empty());
    let (method, problem) = match fallback {
        EnvAuthStrategy::BearerToken { .. } => ("bearer token", (!listed(&mapping.bearer_token)).then(|| "no `bearer_token` variables".to_owned())),
        EnvAuthStrategy::ApiKey { header, .. } => ("API key", match &mapping.api_key {
            None => Some("no `api_key` variables".to_owned()),
            Some(key) if key.names.names().is_empty() => Some("no `api_key` variables".to_owned()),
            Some(key) if !key.header.eq_ignore_ascii_case(header) => Some(format!("`api_key` for header `{}`, not `{header}`", key.header)),
            Some(_) => None,
        }),
        EnvAuthStrategy::Basic => ("Basic", match (listed(&mapping.basic_user), listed(&mapping.basic_pass)) {
            (true, true) => None,
            (false, true) => Some("no `basic_user` variables".to_owned()),
            (true, false) => Some("no `basic_pass` variables".to_owned()),
            (false, false) => Some("no `basic_user` or `basic_pass` variables".to_owned()),
        }),
        EnvAuthStrategy::ApiKeyParam { .. } => {
            let reusable = env_mapping.is_none() && !api.env_auth.is_empty()
                && api.effective_auth_policy().env_fallback.as_ref() == Some(fallback);
            return (!reusable).then(|| "endpoint auth policy falls back to a parameter API key from the environment, which only the \
                API's own identical parameter API key fallback and its env_auth variables can supply; an env_mapping cannot".to_owned());
        }
    };
    problem.map(|problem| format!("endpoint auth policy falls back to {method} environment credentials, but {source} has {problem}"))
}

/// Whether an explicit `mapping` names different variables for a credential
/// than `auth` and `env_auth` already declare.
pub(crate) fn env_mapping_conflicts(auth: &AuthStrategy, env_auth: &[String], mapping: &EnvMapping) -> bool {
    let expected = RestApi::legacy_env_mapping_for(auth, env_auth);
    (expected.bearer_token.is_some() && mapping.bearer_token.is_some() && mapping.bearer_token != expected.bearer_token)
        || (expected.basic_user.is_some() && mapping.basic_user.is_some() && mapping.basic_user != expected.basic_user)
        || (expected.basic_pass.is_some() && mapping.basic_pass.is_some() && mapping.basic_pass != expected.basic_pass)
        || (expected.api_key.is_some() && mapping.api_key.is_some() && mapping.api_key != expected.api_key)
}

pub(crate) fn valid_header_name(name: &str) -> bool {
    !name.is_empty() && name.bytes().all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
}
pub(crate) fn valid_rust_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        && name != "_" && !["as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen"].contains(&name)
}
pub(crate) fn validate_origin(policy: &CrossOrigin) -> Result<(), String> {
    if let CrossOrigin::AllowOrigins(origins) = policy {
        for text in origins {
            let url = url::Url::parse(text).map_err(|_| format!("invalid exact origin `{text}`"))?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
                || !url.username().is_empty() || url.password().is_some()
                || !matches!(url.path(), "" | "/") || url.query().is_some() || url.fragment().is_some() {
                return Err(format!("invalid exact origin `{text}`"));
            }
        }
    }
    Ok(())
}
