//! Names of the endpoint methods, call types, and setters of one API, and
//! the renames that keep them from colliding.
//!
//! The rules, in the order they apply:
//!
//! - An endpoint method is its endpoint id in snake case, through the same
//!   sanitizer as request fields, so a keyword gains a trailing `_`.
//! - An endpoint method named like a method of the API struct gains an
//!   `_endpoint` suffix ([`RenameReason::ApiStructMethod`]).
//! - A setter is its request field's name, so it matches the request's
//!   `with_*` method. A setter named like a method every call type has, or
//!   reserves for its pager, gains a `_param` suffix
//!   ([`RenameReason::SharedCallMethod`]).
//! - A setter named like a pager control its own call type forwards
//!   (`page`, `offset`, `after`, `before`, `page_size`) gains a `_param`
//!   suffix ([`RenameReason::PagerMethod`]). Only paginated endpoints
//!   forward controls, and only the ones their strategy authors.
//! - A parameter the endpoint's pagination strategy adopts has no setter: the
//!   names are resolved on the endpoint's [`pager_view`], as the request
//!   struct is generated.
//! - Two endpoint methods, or two setters of one call type, that still share
//!   a name are a [`NameConflict`]; generation refuses them rather than
//!   inventing a numbered name.
//!
//! A call type is named after its request type, with the trailing `Request`
//! replaced by `Call`, so the request suffix that keeps two APIs of one
//! module apart keeps their call types apart too.

use std::collections::BTreeMap;
use std::fmt;

use schematic_define::openapi::import::naming::{sanitize_rust_field_ident, to_snake_case};
use schematic_define::{Endpoint, RestApi};

use crate::codegen::pagination::{PagerForwards, pager_forwards, pager_view};
use crate::codegen::request_structs::shared::{
    QueryParamInfo, extract_cookie_params, extract_header_params, extract_query_params,
    param_field_name,
};
use crate::output::get_request_suffix;

/// The per-call methods every generated call type forwards to the runtime
/// call. A setter with one of these names gains a `_param` suffix.
pub const SHARED_CALL_METHODS: &[&str] = &["anonymous", "credential", "deadline", "full", "build"];

/// The pager entry points of a paginated endpoint's call type, reserved on
/// every call type so that authoring pagination for an endpoint later renames
/// no setter a caller already uses.
pub const PLANNED_CALL_METHODS: &[&str] = &["pager", "pages", "fetch_page", "collect_all"];

/// Methods a call type has through the traits it implements. A setter with
/// one of these names would shadow the trait method in method-call syntax.
pub const CALL_TYPE_TRAIT_METHODS: &[&str] = &["clone", "clone_from", "fmt", "into_future"];

/// The inherent methods `schematic_runtime::api_client!` gives every API
/// struct.
pub const API_CLIENT_METHODS: &[&str] = &[
    "base_url",
    "timeout",
    "connect_timeout",
    "proxy",
    "add_root_certificate",
    "danger_accept_invalid_certificates",
    "user_agent",
    "default_header",
    "transport",
    "credentials",
    "api_key",
    "env_order",
    "cross_origin",
    "pre_response_json",
    "mutate_response",
    "build",
    "send",
    "send_raw",
    "check_link",
];

/// The inherent methods `generate_api_struct` gives an API struct. Only
/// APIs that accept HTTP Basic credentials get `basic_auth`; it is reserved
/// for every API so a method name does not depend on the auth strategy.
pub const API_STRUCT_METHODS: &[&str] = &["new", "definition", "basic_auth"];

/// Methods an API struct has through the traits it implements (`Clone`,
/// `Debug`, `Default`).
pub const API_STRUCT_TRAIT_METHODS: &[&str] = &["clone", "clone_from", "fmt", "default"];

/// Why a generated name differs from the name its rule would give.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenameReason {
    /// A setter took a `_param` suffix: every call type has a method of that
    /// name.
    SharedCallMethod,
    /// An endpoint method took an `_endpoint` suffix: the API struct has a
    /// method of that name.
    ApiStructMethod,
    /// An endpoint method's snake-case id is a Rust keyword or not an
    /// identifier, so the request-field sanitizer changed it.
    NotAnIdentifier,
    /// A setter took a `_param` suffix: the call type forwards the
    /// endpoint's pager control of that name.
    PagerMethod,
}

impl fmt::Display for RenameReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SharedCallMethod => "every call type has a method of that name",
            Self::ApiStructMethod => "the API struct has a method of that name",
            Self::NotAnIdentifier => "it is a Rust keyword or not an identifier",
            Self::PagerMethod => "the endpoint's pager has a method of that name",
        })
    }
}

/// What a rename applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RenameTarget {
    /// The endpoint method on the API struct.
    EndpointMethod,
    /// An optional-value setter on the call type.
    Setter,
}

impl fmt::Display for RenameTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EndpointMethod => "endpoint method",
            Self::Setter => "setter",
        })
    }
}

/// One generated name that differs from the name its rule gives.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rename {
    /// The API whose struct or call type carries the name.
    pub api: String,
    /// The endpoint the name belongs to.
    pub endpoint: String,
    /// What was renamed.
    pub target: RenameTarget,
    /// The name the rule gives.
    pub from: String,
    /// The generated name.
    pub to: String,
    /// Why `from` could not be used.
    pub reason: RenameReason,
}

impl fmt::Display for Rename {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} `{}` is generated as `{}`: {}",
            self.target, self.from, self.to, self.reason
        )
    }
}

/// A generated name claimed twice after every rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameConflict {
    /// The API the conflict is in.
    pub api: String,
    /// What the name is.
    pub target: RenameTarget,
    /// The shared name.
    pub name: String,
    /// The endpoints claiming it: two or more for an endpoint method, the one
    /// owning endpoint for setters.
    pub endpoints: Vec<String>,
    /// The request fields claiming it, for setters; empty otherwise.
    pub fields: Vec<String>,
}

impl fmt::Display for NameConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.target {
            RenameTarget::EndpointMethod => write!(
                f,
                "endpoint method `{}` is generated for endpoints {}; rename one of the endpoint ids",
                self.name,
                quoted(&self.endpoints)
            ),
            RenameTarget::Setter => write!(
                f,
                "setter `{}` is generated for request fields {}; rename one of the parameters",
                self.name,
                quoted(&self.fields)
            ),
        }
    }
}

fn quoted(names: &[String]) -> String {
    names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(" and ")
}

/// Where an optional value is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamLocation {
    /// A query parameter.
    Query,
    /// A header parameter.
    Header,
    /// A cookie parameter.
    Cookie,
}

impl fmt::Display for ParamLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Query => "query",
            Self::Header => "header",
            Self::Cookie => "cookie",
        })
    }
}

/// One optional value of an endpoint and the call-type setter that sets it.
#[derive(Debug, Clone)]
pub struct SetterName {
    /// The request field, and the suffix of its `with_*` method.
    pub field: String,
    /// The setter on the call type.
    pub setter: String,
    /// Where the value is sent.
    pub location: ParamLocation,
    /// The parameter as declared.
    pub param: QueryParamInfo,
}

/// The generated names of one endpoint.
#[derive(Debug, Clone)]
pub struct EndpointNames {
    /// The endpoint id.
    pub endpoint_id: String,
    /// The request type, `{id}{suffix}`.
    pub request_type: String,
    /// The call type.
    pub call_type: String,
    /// The endpoint method on the API struct.
    pub method: String,
    /// The setters, in query, header, cookie declaration order.
    pub setters: Vec<SetterName>,
    /// The pager methods of a paginated endpoint's call type.
    pub pager: Option<PagerForwards>,
}

/// The generated names of every endpoint of one API.
#[derive(Debug, Clone)]
pub struct ApiMethodNames {
    /// One entry per endpoint, in declaration order.
    pub endpoints: Vec<EndpointNames>,
    /// Every name that differs from its rule's name.
    pub renames: Vec<Rename>,
    /// Names still claimed twice; generation refuses the API when any exist.
    pub conflicts: Vec<NameConflict>,
}

/// The call type name for a request type name.
pub fn call_type_name(request_type: &str) -> String {
    let stem = request_type.strip_suffix("Request").unwrap_or(request_type);
    format!("{stem}Call")
}

/// Resolves the endpoint method, call type, and setter names of `api`.
pub fn resolve_names(api: &RestApi) -> ApiMethodNames {
    let suffix = get_request_suffix(api);
    let mut renames = Vec::new();
    let mut conflicts = Vec::new();
    let mut endpoints = Vec::new();

    for endpoint in &api.endpoints {
        let names = endpoint_names(api, endpoint, &suffix, &mut renames);
        conflicts.extend(setter_conflicts(api, &names));
        endpoints.push(names);
    }

    let mut by_method: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for names in &endpoints {
        by_method.entry(&names.method).or_default().push(names.endpoint_id.clone());
    }
    for (method, ids) in by_method {
        if ids.len() > 1 {
            conflicts.push(NameConflict {
                api: api.name.clone(),
                target: RenameTarget::EndpointMethod,
                name: method.to_string(),
                endpoints: ids,
                fields: Vec::new(),
            });
        }
    }

    ApiMethodNames {
        endpoints,
        renames,
        conflicts,
    }
}

/// Every rename across `apis`, in API then endpoint order.
pub fn endpoint_method_renames(apis: &[&RestApi]) -> Vec<Rename> {
    apis.iter().flat_map(|api| resolve_names(api).renames).collect()
}

fn endpoint_names(
    api: &RestApi,
    endpoint: &Endpoint,
    suffix: &str,
    renames: &mut Vec<Rename>,
) -> EndpointNames {
    let mut rename = |target, from: &str, to: &str, reason| {
        renames.push(Rename {
            api: api.name.clone(),
            endpoint: endpoint.id.clone(),
            target,
            from: from.to_string(),
            to: to.to_string(),
            reason,
        });
    };

    let snake = to_snake_case(&endpoint.id);
    let sanitized = sanitize_rust_field_ident(&endpoint.id);
    if sanitized != snake {
        rename(RenameTarget::EndpointMethod, &snake, &sanitized, RenameReason::NotAnIdentifier);
    }
    let method = if is_api_struct_method(&sanitized) {
        let renamed = format!("{sanitized}_endpoint");
        rename(RenameTarget::EndpointMethod, &sanitized, &renamed, RenameReason::ApiStructMethod);
        renamed
    } else {
        sanitized
    };

    let pager = pager_forwards(api, endpoint);
    let endpoint = pager_view(api, endpoint);
    let is_pager_control = |name: &str| {
        pager
            .as_ref()
            .is_some_and(|pager| pager.knobs.iter().any(|knob| knob.name() == name))
    };
    let located = [
        (ParamLocation::Query, extract_query_params(&endpoint.params)),
        (ParamLocation::Header, extract_header_params(&endpoint.params)),
        (ParamLocation::Cookie, extract_cookie_params(&endpoint.params)),
    ];
    let mut setters = Vec::new();
    for (location, params) in located {
        for param in params {
            let field = param_field_name(&param.name);
            let setter = if is_call_type_method(&field) {
                let renamed = format!("{field}_param");
                rename(RenameTarget::Setter, &field, &renamed, RenameReason::SharedCallMethod);
                renamed
            } else if is_pager_control(&field) {
                let renamed = format!("{field}_param");
                rename(RenameTarget::Setter, &field, &renamed, RenameReason::PagerMethod);
                renamed
            } else {
                field.clone()
            };
            setters.push(SetterName {
                field,
                setter,
                location,
                param,
            });
        }
    }

    let request_type = format!("{}{suffix}", endpoint.id);
    EndpointNames {
        endpoint_id: endpoint.id.clone(),
        call_type: call_type_name(&request_type),
        request_type,
        method,
        setters,
        pager,
    }
}

/// Setter names claimed twice once `_param` suffixes are applied, for
/// example a `full` parameter beside a `full_param` parameter. Two request
/// fields with one name are reported by request-field validation instead.
fn setter_conflicts(api: &RestApi, names: &EndpointNames) -> Vec<NameConflict> {
    let mut by_setter: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for setter in &names.setters {
        by_setter.entry(&setter.setter).or_default().push(&setter.field);
    }
    by_setter
        .into_iter()
        .filter(|(_, fields)| fields.len() > 1 && fields.iter().any(|field| *field != fields[0]))
        .map(|(setter, fields)| NameConflict {
            api: api.name.clone(),
            target: RenameTarget::Setter,
            name: setter.to_string(),
            endpoints: vec![names.endpoint_id.clone()],
            fields: fields.into_iter().map(str::to_string).collect(),
        })
        .collect()
}

fn is_api_struct_method(name: &str) -> bool {
    [API_CLIENT_METHODS, API_STRUCT_METHODS, API_STRUCT_TRAIT_METHODS]
        .iter()
        .any(|reserved| reserved.contains(&name))
}

fn is_call_type_method(name: &str) -> bool {
    [SHARED_CALL_METHODS, PLANNED_CALL_METHODS, CALL_TYPE_TRAIT_METHODS]
        .iter()
        .any(|reserved| reserved.contains(&name))
}
