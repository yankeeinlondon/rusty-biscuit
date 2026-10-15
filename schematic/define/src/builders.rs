//! Required-fact REST authoring. Descriptors survive lowering so API assembly
//! can validate their entire shape against its destination context.
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
use crate::{ApiRequest, ApiResponse, AuthPolicy, AuthStrategy, CrossOrigin, DefinitionContext,
    DefinitionError, Endpoint, EndpointParams, EnvMapping, FormField, FormFieldKind,
    ParamDef, Registration, RestApi, RestMethod, Schema, TypeDesc};

/// A required fact has not been declared.
pub struct Missing;
/// A required fact has been declared.
pub struct Present;

/// One success shape. Several successes arrive with response model v2.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Success(ApiResponse);
#[allow(missing_docs)]
impl Success {
    pub fn json(desc: TypeDesc) -> Self { Self(ApiResponse::Json(typed_schema(desc))) }
    pub fn text() -> Self { Self(ApiResponse::Text) }
    pub fn binary() -> Self { Self(ApiResponse::Binary) }
    pub fn empty() -> Self { Self(ApiResponse::Empty) }
    fn lower(self) -> ApiResponse { self.0 }
}

/// Declared errors are reserved for response model v2; currently only none.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct ErrorSet { _private: () }
impl ErrorSet {
    /// No declared errors; provider responses still use runtime error handling.
    pub fn none() -> Self { Self { _private: () } }
}

impl FormField {
    /// Typed JSON form metadata, validated with other endpoint references at API build.
    pub fn json_desc(name: impl Into<String>, desc: TypeDesc) -> Self {
        Self::json(name, typed_schema(desc))
    }
}

fn typed_schema(desc: TypeDesc) -> Schema {
    Schema { type_name: desc.rust_path(), module_path: None, descriptor: Some(desc) }
}
fn invalid(api: &str, endpoint: &str, message: impl Into<String>) -> DefinitionError {
    DefinitionError::InvalidDefinition { api: api.into(), endpoint: endpoint.into(), message: message.into() }
}

/// Description and success are independent required facts. Build records
/// registration errors instead of allowing later setters to conceal them.
pub struct EndpointBuilder<D = Missing, S = Missing> {
    endpoint: Endpoint,
    successes: usize,
    error: Option<DefinitionError>,
    state: PhantomData<(D, S)>,
}
#[allow(missing_docs)]
impl Endpoint {
    // Construction returns typestate so required facts cannot be omitted.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(id: impl Into<String>, method: RestMethod, path: impl Into<String>) -> EndpointBuilder {
        EndpointBuilder { endpoint: Endpoint { id: id.into(), method, path: path.into(),
            description: String::new(), request: None, response: ApiResponse::Empty,
            headers: vec![], params: None, oauth_scopes: None }, successes: 0, error: None, state: PhantomData }
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
        EndpointBuilder { endpoint: self.endpoint, successes: self.successes, error: self.error, state: PhantomData }
    }
    pub fn describes(mut self, description: impl Into<String>) -> EndpointBuilder<Present, S> {
        self.endpoint.description = description.into(); self.cast()
    }
    pub fn returns(mut self, success: Success) -> EndpointBuilder<D, Present> {
        self.successes += 1; self.endpoint.response = success.lower(); self.cast()
    }
    pub fn returns_json<T: Registration>(mut self, ctx: &mut DefinitionContext) -> EndpointBuilder<D, Present> {
        match ctx.register_for::<T>("<endpoint>", &self.endpoint.id) {
            Ok(desc) => self.returns(Success::json(desc)),
            Err(error) => { if self.error.is_none() { self.error = Some(error); } self.cast() }
        }
    }
    pub fn returns_text(self) -> EndpointBuilder<D, Present> { self.returns(Success::text()) }
    pub fn returns_binary(self) -> EndpointBuilder<D, Present> { self.returns(Success::binary()) }
    pub fn returns_empty(self) -> EndpointBuilder<D, Present> { self.returns(Success::empty()) }
    pub fn errors(self, _: ErrorSet) -> Self { self }
    pub fn request_json<T: Registration>(mut self, ctx: &mut DefinitionContext) -> Self {
        match ctx.register_for::<T>("<endpoint>", &self.endpoint.id) {
            Ok(desc) => self.endpoint.request = Some(ApiRequest::Json(typed_schema(desc))),
            Err(error) => if self.error.is_none() { self.error = Some(error); },
        }
        self
    }
    pub fn request_form(mut self, fields: Vec<FormField>) -> Self { self.endpoint.request = Some(ApiRequest::FormData { fields }); self }
    pub fn request_urlencoded(mut self, fields: Vec<FormField>) -> Self { self.endpoint.request = Some(ApiRequest::UrlEncoded { fields }); self }
    pub fn request_text(mut self) -> Self { self.endpoint.request = Some(ApiRequest::Text { content_type: "text/plain".into() }); self }
    pub fn request_text_with_content_type(mut self, content_type: impl Into<String>) -> Self { self.endpoint.request = Some(ApiRequest::Text { content_type: content_type.into() }); self }
    pub fn request_binary_with_content_type(mut self, content_type: impl Into<String>) -> Self { self.endpoint.request = Some(ApiRequest::Binary { content_type: content_type.into() }); self }
    pub fn request_binary(mut self) -> Self { self.endpoint.request = Some(ApiRequest::Binary { content_type: "application/octet-stream".into() }); self }
    pub fn params(mut self, params: EndpointParams) -> Self { self.endpoint.params = Some(params); self }
    pub fn query(mut self, param: ParamDef) -> Self { self.endpoint.params.get_or_insert_with(EndpointParams::new).query.push(param); self }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self { self.endpoint.headers.push((name.into(), value.into())); self }
}
impl EndpointBuilder<Present, Present> {
    /// Lower to the existing endpoint model. API build checks destination references.
    pub fn build(self) -> Result<Endpoint, DefinitionError> {
        if let Some(error) = self.error { return Err(error); }
        let ep = self.endpoint;
        if self.successes > 1 { return Err(invalid("<endpoint>", &ep.id, "several success shapes arrive with response model v2")); }
        if ep.id.trim().is_empty() || ep.description.trim().is_empty() || !ep.path.starts_with('/') {
            return Err(invalid("<endpoint>", &ep.id, "id and description must be nonempty and path must start with /"));
        }
        Ok(ep)
    }
}

/// API description and authentication must be stated before build. The context
/// is borrowed after endpoints are constructed, leaving registration ergonomic.
pub struct RestApiBuilder<'a, D = Missing, A = Missing> {
    ctx: &'a DefinitionContext,
    api: RestApi,
    auth_choices: usize,
    error: Option<DefinitionError>,
    state: PhantomData<(D, A)>,
}
impl RestApi {
    /// Start a definition in its destination module-group context.
    pub fn builder(ctx: &DefinitionContext, name: impl Into<String>, base_url: impl Into<String>) -> RestApiBuilder<'_> {
        RestApiBuilder { ctx, api: RestApi { name: name.into(), base_url: base_url.into(), description: String::new(),
            docs_url: None, auth: AuthStrategy::None, auth_policy: None, env_auth: vec![], env_username: None,
            headers: vec![], endpoints: vec![], module_path: None, request_suffix: None, version: None,
            env_mapping: None, cross_origin: CrossOrigin::same_origin(), request_id_header: None, secret_names: vec![],
            default_pagination: None, pagination_exceptions: vec![] }, auth_choices: 0, error: None, state: PhantomData }
    }
}
#[allow(missing_docs)]
impl<'a, D, A> RestApiBuilder<'a, D, A> {
    fn cast<ND, NA>(self) -> RestApiBuilder<'a, ND, NA> {
        RestApiBuilder { ctx: self.ctx, api: self.api, auth_choices: self.auth_choices, error: self.error, state: PhantomData }
    }
    pub fn describes(mut self, description: impl Into<String>) -> RestApiBuilder<'a, Present, A> { self.api.description = description.into(); self.cast() }
    pub fn authentication<I, N>(mut self, auth: AuthStrategy, env: I) -> RestApiBuilder<'a, D, Present>
    where I: IntoIterator<Item = N>, N: Into<String> {
        self.auth_choices += 1; self.api.auth = auth; self.api.env_auth = env.into_iter().map(Into::into).collect(); self.cast()
    }
    pub fn no_auth(self) -> RestApiBuilder<'a, D, Present> { self.authentication(AuthStrategy::None, std::iter::empty::<String>()) }
    pub fn bearer_env<I, N>(self, names: I) -> RestApiBuilder<'a, D, Present> where I: IntoIterator<Item = N>, N: Into<String> {
        self.authentication(AuthStrategy::BearerToken { header: None }, names)
    }
    pub fn basic_env<I, N>(self, username: impl Into<String>, passwords: I) -> RestApiBuilder<'a, D, Present> where I: IntoIterator<Item = N>, N: Into<String> {
        self.env_username(username).authentication(AuthStrategy::Basic, passwords)
    }
    pub fn env_username(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        if self.api.env_username.as_ref().is_some_and(|old| old != &name) {
            self.error = Some(invalid(&self.api.name, "<api>", "username environment conflicts with authentication declaration"));
        }
        self.api.env_username = Some(name); self
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
    pub fn default_errors(self, _: ErrorSet) -> Self { self }
}
impl RestApiBuilder<'_, Present, Present> {
    /// Validate coherent authentication and every typed reference, without
    /// weakening output-specific generator validation for legacy fixtures.
    pub fn build(self) -> Result<RestApi, DefinitionError> {
        self.ctx.finish()?;
        if let Some(error) = self.error { return Err(error); }
        let api = self.api;
        let fail = |message| invalid(&api.name, "<api>", message);
        if self.auth_choices != 1 { return Err(fail("authentication must be declared exactly once")); }
        let url = url::Url::parse(&api.base_url).map_err(|_| fail("invalid base URL"))?;
        if api.name.trim().is_empty() || api.description.trim().is_empty() || !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(fail("name and description must be nonempty and base URL must be HTTP(S)"));
        }
        if let Some(mapping) = &api.env_mapping {
            let expected = RestApi::legacy_env_mapping_for(&api.auth, &api.env_auth, api.env_username.as_deref());
            if (expected.bearer_token.is_some() && mapping.bearer_token != expected.bearer_token)
                || (expected.basic_user.is_some() && mapping.basic_user != expected.basic_user)
                || (expected.basic_pass.is_some() && mapping.basic_pass != expected.basic_pass)
                || (expected.api_key.is_some() && mapping.api_key != expected.api_key) {
                return Err(fail("explicit environment mapping conflicts with authentication declaration"));
            }
        }
        for ep in &api.endpoints {
            let mut schemas = vec![];
            if let ApiResponse::Json(schema) = &ep.response { schemas.push(schema); }
            if let Some(request) = &ep.request {
                match request {
                    ApiRequest::Json(schema) => schemas.push(schema),
                    ApiRequest::FormData { fields } | ApiRequest::UrlEncoded { fields } => for field in fields {
                        if let FormFieldKind::Json(schema) = &field.kind { schemas.push(schema); }
                    },
                    _ => {},
                }
            }
            for schema in schemas {
                if let Some(desc) = &schema.descriptor {
                    self.ctx.validate_reference(&api.name, &ep.id, desc)?;
                    if schema.type_name != desc.rust_path() || schema.module_path.is_some() {
                        return Err(invalid(&api.name, &ep.id, "lowered schema disagrees with its typed descriptor"));
                    }
                } else {
                    return Err(DefinitionError::InvalidDescriptor { api: api.name.clone(),
                        endpoint: ep.id.clone(), rust_type: schema.type_name.clone() });
                }
            }
        }
        Ok(api)
    }
}
