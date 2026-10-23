//! GraphQL operations of a REST API (D17, D29, D34, D35).
//!
//! A [`GraphQlSurface`] on [`crate::RestApi::graphql`] holds the ordered
//! [`AddressRule`]s that derive the GraphQL URL from the client's base URL and
//! the API's [`GraphQlOperation`]s. An operation shares the API's base
//! address, authentication, and naming space with its REST endpoints. Its
//! document is opaque text, sent exactly as written; nothing here parses it.
//! See `schematic/docs/graphql.md`.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::builders::{ErrorSet, Missing, Present};
use crate::error_table::ErrorEntry;
use crate::pagination::{Collection, InputParam, PageStrategy, Pagination};
use crate::{DefinitionContext, DefinitionError, Registration, TypeRef};

/// The GraphQL address rules and operations of one API.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQlSurface {
    /// Tried in order; the first rule matching the base URL's path wins.
    /// Required and nonempty.
    pub address: Vec<AddressRule>,
    /// The operations, in authoring order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<GraphQlOperation>,
}

impl GraphQlSurface {
    /// A surface at `address` with no operations yet.
    pub fn new(address: impl IntoIterator<Item = AddressRule>) -> Self {
        Self { address: address.into_iter().collect(), operations: vec![] }
    }
}

/// How the GraphQL URL derives from the base URL's path (Default D-10).
///
/// Rules compare whole path segments and ignore a trailing slash, so a
/// reverse-proxy prefix survives: `/prefix/api/v3/` becomes
/// `/prefix/api/graphql` under the GitHub rules.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressRule {
    /// A base path ending in `suffix` has that ending replaced by `with`.
    ReplaceSuffix {
        /// Path ending to match, such as `/api/v3`.
        suffix: String,
        /// Replacement, such as `/api/graphql`.
        with: String,
    },
    /// Append this path to any base path.
    Append(String),
}

#[allow(missing_docs)]
impl AddressRule {
    pub fn replace_suffix(suffix: impl Into<String>, with: impl Into<String>) -> Self {
        Self::ReplaceSuffix { suffix: suffix.into(), with: with.into() }
    }
    pub fn append(path: impl Into<String>) -> Self { Self::Append(path.into()) }
    /// GitHub's rules: Enterprise `/api/v3` becomes `/api/graphql`, any other
    /// base gains `/graphql`.
    pub fn github() -> Vec<Self> {
        vec![Self::replace_suffix("/api/v3", "/api/graphql"), Self::append("/graphql")]
    }
}

/// Query or mutation. Documentation and an export label only: both are sent
/// the same way.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationKind {
    #[allow(missing_docs)]
    Query,
    #[allow(missing_docs)]
    Mutation,
}

/// An operation's variables.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphQlVariables {
    /// No variables; the request omits `variables` (Default D-27).
    Unit,
    /// A registered type serializing to a JSON object.
    Typed(TypeRef),
}

/// One GraphQL operation (Default D-9).
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, graphql::GraphQlOperation};
///
/// let mut ctx = DefinitionContext::types_in("::models");
/// let viewer = GraphQlOperation::query("Viewer", "query { viewer { login } }")
///     .describes("The signed-in user")
///     .no_variables()
///     .data::<serde_json::Value>(&mut ctx)
///     .build()?;
/// assert_eq!(viewer.operation_name, None);
/// # Ok::<(), schematic_define::DefinitionError>(())
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphQlOperation {
    /// Shares the generated naming space with the API's endpoint ids.
    pub id: String,
    #[allow(missing_docs)]
    pub description: String,
    #[allow(missing_docs)]
    pub kind: OperationKind,
    /// Opaque GraphQL text, sent unchanged; an empty document is sent too.
    pub document: String,
    #[allow(missing_docs)]
    pub variables: GraphQlVariables,
    /// The type `data` decodes as.
    pub data: TypeRef,
    /// Sent as `operationName` when present, the empty string included;
    /// omitted otherwise (Default D-21).
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    /// Entries overlaying the API's error defaults for non-2xx replies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ErrorEntry>,
    /// Opts out of declared HTTP errors; GraphQL errors stay declared.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub no_declared_errors: bool,
    /// A cursor connection read from `data`, written into `variables`.
    #[serde(default, deserialize_with = "crate::serde_presence::non_null", skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Collection>,
}

impl GraphQlOperation {
    /// Start a query.
    #[allow(clippy::new_ret_no_self)]
    pub fn query(id: impl Into<String>, document: impl Into<String>) -> GraphQlOperationBuilder {
        GraphQlOperationBuilder::new(id.into(), OperationKind::Query, document.into())
    }

    /// Start a mutation.
    pub fn mutation(id: impl Into<String>, document: impl Into<String>) -> GraphQlOperationBuilder {
        GraphQlOperationBuilder::new(id.into(), OperationKind::Mutation, document.into())
    }

    /// The operation's JSON references: typed variables, data, then error
    /// entries.
    pub fn json_references(&self) -> Vec<&TypeRef> {
        operation_json_references!(self)
    }

    /// [`Self::json_references`], mutably.
    // OpenAPI import renames components through it.
    #[cfg_attr(not(feature = "openapi"), allow(dead_code))]
    pub(crate) fn json_references_mut(&mut self) -> Vec<&mut TypeRef> {
        operation_json_references!(self, mut)
    }
}

/// The JSON references of an operation, shared by
/// [`GraphQlOperation::json_references`] and its mutable counterpart so they
/// cannot drift apart. Pass `mut` for mutable references.
macro_rules! operation_json_references {
    ($operation:expr $(, $mutability:tt)?) => {{
        let operation = $operation;
        let mut references = Vec::new();
        if let GraphQlVariables::Typed(reference) = &$($mutability)? operation.variables { references.push(reference); }
        references.push(&$($mutability)? operation.data);
        references.extend(crate::components::error_references!(&$($mutability)? operation.errors $(, $mutability)?));
        references
    }};
}
use operation_json_references;

/// Description, variables, and data are required facts; the type parameters
/// track them so an operation missing one does not build.
///
/// ```compile_fail
/// let mut ctx = schematic_define::DefinitionContext::types_in("::models");
/// schematic_define::graphql::GraphQlOperation::query("Q", "{ a }").describes("Q")
///     .data::<serde_json::Value>(&mut ctx).build();
/// ```
pub struct GraphQlOperationBuilder<D = Missing, V = Missing, T = Missing> {
    operation: GraphQlOperation,
    error: Option<DefinitionError>,
    state: PhantomData<(D, V, T)>,
}

impl GraphQlOperationBuilder {
    fn new(id: String, kind: OperationKind, document: String) -> Self {
        let operation = GraphQlOperation { id, description: String::new(), kind, document, variables: GraphQlVariables::Unit,
            data: TypeRef::Unknown, operation_name: None, errors: vec![], no_declared_errors: false, pagination: None };
        Self { operation, error: None, state: PhantomData }
    }
}

#[allow(missing_docs)]
impl<D, V, T> GraphQlOperationBuilder<D, V, T> {
    fn cast<ND, NV, NT>(self) -> GraphQlOperationBuilder<ND, NV, NT> {
        GraphQlOperationBuilder { operation: self.operation, error: self.error, state: PhantomData }
    }
    fn register<R: Registration>(&mut self, ctx: &mut DefinitionContext) -> Option<TypeRef> {
        match ctx.register_for::<R>("<operation>", &self.operation.id) {
            Ok(desc) => Some(desc.into_type_ref()),
            Err(error) => { if self.error.is_none() { self.error = Some(error); } None }
        }
    }
    pub fn describes(mut self, description: impl Into<String>) -> GraphQlOperationBuilder<Present, V, T> {
        self.operation.description = description.into(); self.cast()
    }
    /// Registers `R` as the variables type.
    pub fn variables<R: Registration>(mut self, ctx: &mut DefinitionContext) -> GraphQlOperationBuilder<D, Present, T> {
        if let Some(reference) = self.register::<R>(ctx) { self.operation.variables = GraphQlVariables::Typed(reference); }
        self.cast()
    }
    pub fn no_variables(mut self) -> GraphQlOperationBuilder<D, Present, T> {
        self.operation.variables = GraphQlVariables::Unit; self.cast()
    }
    /// Registers `R` as the type `data` decodes as.
    pub fn data<R: Registration>(mut self, ctx: &mut DefinitionContext) -> GraphQlOperationBuilder<D, V, Present> {
        if let Some(reference) = self.register::<R>(ctx) { self.operation.data = reference; }
        self.cast()
    }
    pub fn operation_name(mut self, name: impl Into<String>) -> Self { self.operation.operation_name = Some(name.into()); self }
    /// Adds entries, or opts out with [`ErrorSet::none`].
    pub fn errors(mut self, errors: ErrorSet) -> Self {
        let (entries, none) = errors.into_parts();
        self.operation.errors.extend(entries);
        self.operation.no_declared_errors |= none;
        self
    }
    pub fn pagination(mut self, collection: Collection) -> Self { self.operation.pagination = Some(collection); self }
}

impl GraphQlOperationBuilder<Present, Present, Present> {
    /// Check the rules the operation alone can show; API build checks the
    /// rest, including references and the API's error defaults.
    ///
    /// ## Errors
    ///
    /// The first registration failure, or [`DefinitionError::InvalidOperation`].
    pub fn build(self) -> Result<GraphQlOperation, DefinitionError> {
        if let Some(error) = self.error { return Err(error); }
        validate_operation(&self.operation, &[])
            .map_err(|message| invalid("<operation>", &self.operation.id, message))?;
        Ok(self.operation)
    }
}

fn invalid(api: &str, operation: &str, message: impl Into<String>) -> DefinitionError {
    DefinitionError::InvalidOperation { api: api.into(), operation: operation.into(), message: message.into() }
}

/// Check an API's GraphQL surface (R35, R39, R40): address rules, a base URL
/// without query or fragment, unique operation ids distinct from endpoint ids,
/// and each operation's own rules.
pub(crate) fn validate_surface(api: &crate::RestApi, surface: &GraphQlSurface) -> Result<(), DefinitionError> {
    let fail = |message: String| DefinitionError::InvalidDefinition { api: api.name.clone(), endpoint: "<api>".into(), message };
    if surface.address.is_empty() { return Err(fail("graphql: address needs at least one rule".into())); }
    for rule in &surface.address {
        let paths: Vec<&str> = match rule {
            AddressRule::ReplaceSuffix { suffix, with } => vec![suffix, with],
            AddressRule::Append(path) => vec![path],
        };
        for path in paths {
            if !valid_rule_path(path) {
                return Err(fail(format!("graphql: address rule path `{path}` must start with `/` and name nonempty path segments, without a query or fragment")));
            }
        }
    }
    if let Ok(url) = url::Url::parse(&api.base_url)
        && (url.query().is_some() || url.fragment().is_some()) {
        return Err(fail("graphql: a base URL with a query or fragment has no GraphQL address".into()));
    }
    for (index, operation) in surface.operations.iter().enumerate() {
        let fail = |message: String| invalid(&api.name, &operation.id, message);
        if surface.operations[..index].iter().any(|other| other.id == operation.id) {
            return Err(fail("operation id is declared more than once".into()));
        }
        if api.endpoints.iter().any(|endpoint| endpoint.id == operation.id) {
            return Err(fail("operation id equals an endpoint id; they share the generated naming space".into()));
        }
        validate_operation(operation, &api.errors).map_err(fail)?;
        if let Some(variable) = required_pager_variable(api, operation) {
            return Err(fail(format!("pager-managed variable `{variable}` must be optional in the variables type; the pager supplies it")));
        }
    }
    Ok(())
}

/// A top-level variable the pager writes that the registered variables
/// schema lists as `required` (R40: pager-managed inputs are exempt from the
/// caller's required check, so the type must not force the caller to set
/// them). A component that is not bundled yet is checked by reference
/// resolution instead.
fn required_pager_variable(api: &crate::RestApi, operation: &GraphQlOperation) -> Option<String> {
    let (Some(collection), GraphQlVariables::Typed(TypeRef::Named(name))) = (&operation.pagination, &operation.variables) else { return None };
    let Pagination::Explicit(strategy) = &collection.pagination else { return None };
    let required = api.component(name)?.schema.get("required")?.as_array()?;
    crate::pagination::request_inputs(strategy).into_iter().find_map(|(_, input)| match input {
        InputParam::Body(path) if path.as_segments().len() == 2 => {
            let variable = &path.as_segments()[1];
            required.iter().any(|entry| entry.as_str() == Some(variable.as_str())).then(|| variable.clone())
        }
        _ => None,
    })
}

fn valid_rule_path(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/');
    trimmed.starts_with('/') && !trimmed.contains(['?', '#']) && trimmed[1..].split('/').all(|segment| !segment.trim().is_empty())
}

fn validate_operation(operation: &GraphQlOperation, api_errors: &[ErrorEntry]) -> Result<(), String> {
    if operation.id.trim().is_empty() || operation.description.trim().is_empty() {
        return Err("id and description must be nonempty".into());
    }
    if let GraphQlVariables::Typed(reference) = &operation.variables
        && !matches!(reference, TypeRef::Named(_) | TypeRef::Map(_) | TypeRef::KeyedMap { .. }) {
        return Err("variables must be a named type or map serializing to a JSON object".into());
    }
    crate::error_table::validate_error_policy(api_errors, &operation.errors, operation.no_declared_errors)?;
    if let Some(collection) = &operation.pagination { validate_pagination(operation, collection)?; }
    Ok(())
}

/// A paginated operation reads its connection from decoded `data` and writes
/// every pager input into the request body under `variables` (R40).
fn validate_pagination(operation: &GraphQlOperation, collection: &Collection) -> Result<(), String> {
    if let Some(reason) = collection.items.syntax_problem() {
        return Err(format!("pagination items `{}`: {reason}", collection.items));
    }
    if collection.item_type.as_deref().is_none_or(|item| item.trim().is_empty()) {
        return Err("pagination needs an item type".into());
    }
    if !collection.alternates.is_empty() { return Err("pagination alternates are not supported for an operation".into()); }
    let Pagination::Explicit(strategy) = &collection.pagination else {
        return Err("pagination must declare its strategy; an operation inherits no REST default".into());
    };
    if let Some(error) = crate::pagination::validate_strategy(strategy).into_iter().next() {
        return Err(format!("pagination {}: {error}", error.rule()));
    }
    if matches!(strategy.kind, PageStrategy::NextLink(_)) { return Err("a next-link strategy has no link to follow for an operation".into()); }
    if strategy.follow_up.is_some() || strategy.session.is_some() {
        return Err("pagination follow-up and session requests target REST paths, not an operation".into());
    }
    if operation.variables == GraphQlVariables::Unit { return Err("a paginated operation needs typed variables".into()); }
    for (_, input) in crate::pagination::request_inputs(strategy) {
        let under_variables = matches!(input, InputParam::Body(path) if path.as_segments().len() > 1 && path.as_segments()[0] == "variables");
        if !under_variables {
            return Err(format!("pager input `{}` must be a request-body path under `variables`", input.display_name()));
        }
    }
    Ok(())
}
