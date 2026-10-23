//! GraphQL operations over the generic send: the [`GraphQl`] codec, the
//! [`GraphQlFailure`] a 2xx reply can carry, and the composite
//! [`GraphQlOperationError`] failure type.
//!
//! A GraphQL operation is a POST whose body [`request_body`] builds. Its 2xx
//! reply is classified from the JSON wrapper before typed data is decoded,
//! so a data mismatch never hides the server's errors:
//!
//! | `errors` | `data` | Outcome |
//! | --- | --- | --- |
//! | a valid non-empty array | anything | declared [`GraphQlFailure`]; `partial_data` only for non-null data that decodes |
//! | absent or `[]` | non-null, decodes | success |
//! | absent or `[]` | absent, null, or does not decode | malformed success |
//! | `null`, not an array, or an invalid entry | anything | malformed success |
//!
//! Every status from 200 to 299 is classified this way. A non-2xx reply takes
//! the endpoint's HTTP error table and is never read as a wrapper. The
//! envelope keeps the raw body in every failure; response-level extensions
//! stay readable there.
//!
//! The client-global JSON hook sees the wire wrapper, once, before
//! classification. A typed mutator runs on decoded data only after a
//! failure-free reply, never on partial data.
//!
//! A paginated operation's pages are classified the same way by
//! [`page_data`]: the pager reads items and continuations from `data`, and a
//! page with GraphQL errors ends paging as a declared failure whose partial
//! data never joins the collected items.
//!
//! ## The operation address
//!
//! An endpoint whose [`EndpointSpec::ADDRESS`](crate::EndpointSpec::ADDRESS)
//! is [`Address::GraphQl`](crate::Address::GraphQl) is sent to the client's
//! GraphQL address rather than to a path under its base URL. The address is
//! the caller's [`Client::graphql_url`](crate::Client::graphql_url) when set,
//! and otherwise is derived from the client's effective base URL by the
//! [`AddressRule`]s the API declares through
//! [`ApiDefinition::graphql`](crate::ApiDefinition::graphql):
//!
//! ```
//! use schematic_runtime::graphql::derive_address;
//! use schematic_runtime::{AddressRule, Url};
//!
//! const GITHUB: &[AddressRule] = &[
//!     AddressRule::ReplaceSuffix { suffix: "/api/v3", with: "/api/graphql" },
//!     AddressRule::Append("/graphql"),
//! ];
//! let derive = |base: &str| derive_address(&Url::parse(base).unwrap(), GITHUB).unwrap().to_string();
//! assert_eq!(derive("https://api.github.com"), "https://api.github.com/graphql");
//! assert_eq!(derive("https://ghe.example/prefix/api/v3/"), "https://ghe.example/prefix/api/graphql");
//! ```

use std::any::Any;
use std::fmt;
use std::marker::PhantomData;

use bytes::Bytes;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::Url;

use crate::codec::{Decode, DecodeFailure, DecodeFailureKind, LiveBody};
use crate::endpoint::{ErrorBody, RequestBody};
use crate::error::InvalidArgument;
use crate::hooks::{HookStage, SupportsHooks};
use crate::pager::PageRootError;

/// The JSON body of a GraphQL request: `{"query", "variables",
/// "operationName"}`.
///
/// `document` is sent exactly as written. `variables` of `None` (an
/// operation without variables) omits the member; otherwise it must
/// serialize to a JSON object. `operationName` is sent only when given.
///
/// ## Errors
///
/// Returns [`InvalidArgument`] when the variables do not serialize to a
/// JSON object; nothing is sent.
pub fn request_body<V: Serialize>(document: &str, variables: Option<&V>, operation_name: Option<&str>) -> Result<RequestBody, InvalidArgument> {
    let mut body = Map::new();
    body.insert("query".to_owned(), Value::String(document.to_owned()));
    if let Some(variables) = variables {
        let variables = serde_json::to_value(variables).map_err(|_| InvalidArgument::new("GraphQL variables could not be serialized"))?;
        if !variables.is_object() {
            return Err(InvalidArgument::new("GraphQL variables must serialize to a JSON object"));
        }
        body.insert("variables".to_owned(), variables);
    }
    if let Some(name) = operation_name {
        body.insert("operationName".to_owned(), Value::String(name.to_owned()));
    }
    Ok(RequestBody::Json(Value::Object(body).to_string()))
}

/// One way to derive an API's GraphQL address from a base URL; see
/// [`derive_address`].
///
/// Paths are compared and rewritten as `/`-separated segments, so a rule's
/// leading or trailing slash is not significant.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressRule {
    /// Matches a base whose path ends in the segments of `suffix`, and
    /// replaces those segments with the segments of `with`.
    ReplaceSuffix {
        /// The trailing path segments to match.
        suffix: &'static str,
        /// The segments that replace them.
        with: &'static str,
    },
    /// Matches every base, and appends these path segments.
    Append(&'static str),
}

/// Why [`derive_address`] could not derive an address.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressError {
    /// The base carries a query or fragment, which a path rule cannot place.
    QueryOrFragment,
    /// The base has no hierarchical path, such as a `data:` URL.
    NotHierarchical,
    /// No rule matched the base.
    NoRuleMatched,
}

impl AddressError {
    /// A short description, free of the URL.
    pub fn reason(self) -> &'static str {
        match self {
            Self::QueryOrFragment => "the base URL carries a query or fragment",
            Self::NotHierarchical => "the base URL has no hierarchical path",
            Self::NoRuleMatched => "no GraphQL address rule matches the base URL",
        }
    }
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for AddressError {}

/// The GraphQL address `rules` derive from `base`: the first rule that
/// matches, in order, rewrites the base's path. Scheme, host, port, and any
/// path prefix before a replaced suffix are kept; a trailing slash on the
/// base is ignored.
///
/// ## Errors
///
/// Returns [`AddressError`] when the base carries a query or fragment, has no
/// hierarchical path, or no rule matches.
pub fn derive_address(base: &Url, rules: &[AddressRule]) -> Result<Url, AddressError> {
    if base.query().is_some() || base.fragment().is_some() {
        return Err(AddressError::QueryOrFragment);
    }
    let mut segments: Vec<&str> = base.path_segments().ok_or(AddressError::NotHierarchical)?.collect();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    let split = |path: &'static str| path.split('/').filter(|segment| !segment.is_empty());
    for rule in rules {
        let tail = match *rule {
            AddressRule::ReplaceSuffix { suffix, with } => {
                let suffix: Vec<&str> = split(suffix).collect();
                if !segments.ends_with(&suffix) {
                    continue;
                }
                segments.truncate(segments.len() - suffix.len());
                with
            }
            AddressRule::Append(tail) => tail,
        };
        segments.extend(split(tail));
        let mut address = base.clone();
        address.set_path(&format!("/{}", segments.join("/")));
        return Ok(address);
    }
    Err(AddressError::NoRuleMatched)
}

/// One entry of a GraphQL `errors` array.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "Map<String, Value>")]
#[non_exhaustive]
pub struct GraphQlError {
    /// The error's description; required.
    pub message: String,
    /// The response field the error belongs to.
    pub path: Option<Vec<PathSegment>>,
    /// Where in the document the error arose.
    pub locations: Option<Vec<Location>>,
    /// The error's `extensions` member, as sent.
    pub extensions: Option<Value>,
    /// Every other member, such as GitHub's top-level `type`.
    pub other: Map<String, Value>,
}

/// One segment of a [`GraphQlError::path`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(untagged)]
pub enum PathSegment {
    /// A field name or alias.
    Field(String),
    /// An index into a list.
    Index(u64),
}

/// A position in a GraphQL document; both numbers start at 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "RawLocation")]
pub struct Location {
    /// The line.
    pub line: u64,
    /// The column.
    pub column: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLocation {
    line: u64,
    column: u64,
}

impl TryFrom<RawLocation> for Location {
    type Error = &'static str;

    fn try_from(raw: RawLocation) -> Result<Self, Self::Error> {
        if raw.line == 0 || raw.column == 0 {
            return Err("GraphQL locations start at line 1, column 1");
        }
        Ok(Self {
            line: raw.line,
            column: raw.column,
        })
    }
}

impl TryFrom<Map<String, Value>> for GraphQlError {
    type Error = String;

    fn try_from(mut map: Map<String, Value>) -> Result<Self, Self::Error> {
        let message = match map.remove("message") {
            Some(Value::String(message)) => message,
            _ => return Err("a GraphQL error needs a string message".to_owned()),
        };
        let path = member(&mut map, "path")?;
        let locations = member(&mut map, "locations")?;
        let extensions = map.remove("extensions");
        Ok(Self {
            message,
            path,
            locations,
            extensions,
            other: map,
        })
    }
}

/// Removes and decodes the optional member `name`; `null` reads as absent.
fn member<T: DeserializeOwned>(map: &mut Map<String, Value>, name: &str) -> Result<Option<T>, String> {
    map.remove(name).map(serde_json::from_value).transpose().map(Option::flatten).map_err(|error| error.to_string())
}

impl fmt::Display for GraphQlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// A 2xx GraphQL reply whose `errors` array is non-empty.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct GraphQlFailure<D> {
    /// The errors, in the order sent.
    pub errors: Vec<GraphQlError>,
    /// The reply's `data` when it is non-null and decodes as `D`; `None`
    /// when it is absent, null, or does not decode.
    pub partial_data: Option<D>,
}

/// The failure type of a GraphQL operation: a [`GraphQlFailure`] from a 2xx
/// reply, or a declared HTTP error body `H`.
///
/// With HTTP errors undeclared, `H` is
/// [`NoDeclaredError`](crate::NoDeclaredError) and every non-2xx reply is
/// undeclared; GraphQL failures stay declared.
#[derive(Debug, Clone, PartialEq)]
pub enum GraphQlOperationError<D, H> {
    /// The server's GraphQL errors.
    GraphQl(GraphQlFailure<D>),
    /// A declared HTTP error body.
    Http(H),
}

impl<A, D, H> ErrorBody<A> for GraphQlOperationError<D, H>
where
    D: Send + 'static,
    H: ErrorBody<A>,
{
    const DECLARED: bool = H::DECLARED;

    fn decode_body(alternative: usize, body: &[u8]) -> Option<Self> {
        H::decode_body(alternative, body).map(Self::Http)
    }

    fn from_declared(failure: Box<dyn Any + Send>) -> Option<Self> {
        failure.downcast::<GraphQlFailure<D>>().ok().map(|failure| Self::GraphQl(*failure))
    }
}

/// Decodes a GraphQL reply's `data` into `D`, after classifying its wrapper
/// (see the [module documentation](self)).
///
/// It supports response hooks: the global JSON hook on the wrapper and a
/// typed mutator on `D`.
pub struct GraphQl<D>(PhantomData<fn() -> D>);

impl<D: DeserializeOwned + Send + 'static> SupportsHooks for GraphQl<D> {}

impl<D: DeserializeOwned + Send + 'static> Decode for GraphQl<D> {
    type Output = D;

    async fn decode(body: LiveBody) -> Result<D, DecodeFailure> {
        let bytes = body.collect().await?;
        let wrapper = serde_json::from_slice(&bytes).map_err(|_| DecodeFailure::new(DecodeFailureKind::Malformed, bytes.clone()))?;
        classify(wrapper, bytes)
    }

    async fn decode_hooked(body: LiveBody, hooks: HookStage<'_>) -> Result<D, DecodeFailure> {
        let bytes = body.collect().await?;
        let wrapper = serde_json::from_slice(&bytes).map_err(|_| DecodeFailure::new(DecodeFailureKind::Malformed, bytes.clone()))?;
        let wrapper = hooks.pre_response_json(wrapper).map_err(|_| DecodeFailure::new(DecodeFailureKind::Hook, bytes.clone()))?;
        let mut data = classify(wrapper, bytes.clone())?;
        hooks.mutate(&mut data).map_err(|_| DecodeFailure::new(DecodeFailureKind::Hook, bytes))?;
        Ok(data)
    }
}

fn classify<D: DeserializeOwned + Send + 'static>(wrapper: Value, bytes: Bytes) -> Result<D, DecodeFailure> {
    let malformed = || DecodeFailure::new(DecodeFailureKind::Malformed, bytes.clone());
    match classify_wrapper::<D>(wrapper) {
        Classified::Data(data) => serde_json::from_value(data).map_err(|_| malformed()),
        Classified::Failed(failure) => Err(DecodeFailure::declared(failure, bytes)),
        Classified::Malformed => Err(malformed()),
    }
}

/// A wrapper's classification before its data is decoded.
enum Classified<D> {
    /// No errors and non-null data, not yet decoded.
    Data(Value),
    /// A valid non-empty `errors` array.
    Failed(GraphQlFailure<D>),
    /// Not an object, an invalid `errors` member, or no data without errors.
    Malformed,
}

fn classify_wrapper<D: DeserializeOwned>(wrapper: Value) -> Classified<D> {
    let Value::Object(mut wrapper) = wrapper else {
        return Classified::Malformed;
    };
    let data = wrapper.remove("data").filter(|data| !data.is_null());
    let errors: Vec<GraphQlError> = match wrapper.remove("errors") {
        None => Vec::new(),
        Some(errors @ Value::Array(_)) => match serde_json::from_value(errors) {
            Ok(errors) => errors,
            Err(_) => return Classified::Malformed,
        },
        Some(_) => return Classified::Malformed,
    };
    if !errors.is_empty() {
        let partial_data = data.and_then(|data| serde_json::from_value(data).ok());
        return Classified::Failed(GraphQlFailure { errors, partial_data });
    }
    data.map_or(Classified::Malformed, Classified::Data)
}

/// The page root of a paginated GraphQL operation: the reply's `data`,
/// classified as an ordinary reply is (see the [module documentation](self)).
///
/// A paginated operation's item, cursor, and end-signal paths are relative
/// to `data`. A reply with GraphQL errors ends paging as a declared
/// [`GraphQlFailure`], whose `partial_data` is kept in the error and never
/// joins the collected items. Data that does not decode as `D` is a
/// malformed page, as it is a malformed success for one send.
///
/// ## Errors
///
/// [`PageRootError::Declared`] for GraphQL errors, and
/// [`PageRootError::Malformed`] for a wrapper the classification rejects.
pub fn page_data<D, H>(wrapper: Value) -> Result<Value, PageRootError<GraphQlOperationError<D, H>>>
where
    D: DeserializeOwned,
{
    match classify_wrapper::<D>(wrapper) {
        Classified::Data(data) => match D::deserialize(&data) {
            Ok(_) => Ok(data),
            Err(_) => Err(PageRootError::Malformed),
        },
        Classified::Failed(failure) => Err(PageRootError::Declared(GraphQlOperationError::GraphQl(failure))),
        Classified::Malformed => Err(PageRootError::Malformed),
    }
}
