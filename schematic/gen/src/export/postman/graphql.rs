//! GraphQL operations as runnable Postman requests.
//!
//! Each operation of an API's GraphQL surface becomes one `POST` to the
//! API's GraphQL address, held in a collection variable derived from the
//! base URL by the surface's address rules. The JSON body is `query` (the
//! authored document), `operationName` when the operation declares one, and
//! `variables`: an example of the registered variables type, or `{}` for an
//! operation without variables.

use schematic_define::graphql::{GraphQlOperation, GraphQlVariables};
use schematic_define::AddressRule;
use serde_json::{Map, Value, json};

use crate::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use crate::export::body::ExportBody;

use super::auth::build_collection_auth;
use super::item::Member;
use super::request::build_body;
use super::responses::{json_response, saved_responses};
use super::types::*;
use super::variables::{Collected, reference};

/// The GraphQL address `rules` derive from `base_url`: the first matching
/// rule rewrites the URL's path, comparing whole segments and ignoring a
/// trailing slash, the runtime's derivation. `None` when no rule matches or
/// the URL has no `scheme://host` prefix.
pub(crate) fn graphql_address(base_url: &str, rules: &[AddressRule]) -> Option<String> {
    let scheme_end = base_url.find("://")? + 3;
    let (origin, path) = match base_url[scheme_end..].find('/') {
        Some(slash) => base_url.split_at(scheme_end + slash),
        None => (base_url, ""),
    };
    let split = |path: &str| path.split('/').filter(|segment| !segment.is_empty()).map(str::to_owned).collect::<Vec<_>>();
    let mut segments = split(path);
    for rule in rules {
        let tail = match rule {
            AddressRule::ReplaceSuffix { suffix, with } => {
                let suffix = split(suffix);
                if !segments.ends_with(&suffix) {
                    continue;
                }
                segments.truncate(segments.len() - suffix.len());
                with
            }
            AddressRule::Append(tail) => tail,
            // `AddressRule` is non-exhaustive; a later rule is not applied.
            _ => continue,
        };
        segments.extend(split(tail));
        return Some(format!("{origin}/{}", segments.join("/")));
    }
    None
}

/// The member's GraphQL requests, or none when it has no surface. An
/// address no rule derives is reported, and its operations are left out.
pub(crate) fn graphql_items(member: &Member<'_>, address_var: &str, collected: &mut Collected) -> Option<(PostmanVariable, Vec<PostmanItem>)> {
    let api = member.api;
    let surface = api.graphql.as_ref()?;
    if surface.operations.is_empty() {
        return None;
    }
    let Some(address) = graphql_address(&api.base_url, &surface.address) else {
        collected.diagnostics.push(ValidationDiagnostic::new(
            Severity::Warning,
            &api.name,
            None,
            Feature::GraphQlOperation,
            Output::Postman,
            format!(
                "no GraphQL address rule matches the base URL {}, so its {} operation(s) are not exported",
                api.base_url,
                surface.operations.len()
            ),
        ));
        return None;
    };
    let variable = PostmanVariable {
        key: address_var.to_string(),
        value: Some(address),
        description: Some(format!("GraphQL endpoint of {}", api.name)),
    };
    let items = surface.operations.iter().map(|operation| operation_item(operation, member, address_var, collected)).collect();
    Some((variable, items))
}

fn operation_item(operation: &GraphQlOperation, member: &Member<'_>, address_var: &str, collected: &mut Collected) -> PostmanItem {
    let api = member.api;
    let variables = match &operation.variables {
        GraphQlVariables::Typed(reference) => match member.examples.example(reference) {
            Ok(value) => value,
            Err(error) => {
                collected.example_failure(api, &operation.id, "GraphQL variables", &error);
                Value::Object(Map::new())
            }
        },
        _ => Value::Object(Map::new()),
    };
    let mut body = Map::new();
    body.insert("query".to_string(), Value::String(operation.document.clone()));
    if let Some(name) = &operation.operation_name {
        body.insert("operationName".to_string(), Value::String(name.clone()));
    }
    body.insert("variables".to_string(), variables);
    let json_body = ExportBody::Json { content_type: "application/json".to_string() };

    // The success reply wraps the operation's data; HTTP errors follow.
    let data = match member.examples.example(&operation.data) {
        Ok(data) => Some(serde_json::to_string_pretty(&json!({"data": data})).unwrap_or_default()),
        Err(error) => {
            collected.example_failure(api, &operation.id, "200 OK (application/json) response", &error);
            None
        }
    };
    let mut response = vec![json_response(200, data)];
    let (errors, failures) = saved_responses(&member.examples, &[], &api.effective_operation_errors(operation));
    response.extend(errors);
    for (role, error) in failures {
        collected.example_failure(api, &operation.id, &role, &error);
    }

    let raw = reference(address_var);
    PostmanItem::Request {
        name: operation.id.clone(),
        request: Box::new(PostmanRequest {
            method: "POST".to_string(),
            url: PostmanUrl { raw: raw.clone(), host: vec![raw], path: Vec::new(), query: Vec::new(), variable: Vec::new() },
            auth: if member.request_level_auth { build_collection_auth(&member.auth) } else { None },
            header: Vec::new(),
            body: Some(build_body(&json_body, Some(&Value::Object(body)))),
            description: (!operation.description.is_empty()).then(|| operation.description.clone()),
        }),
        response,
    }
}
