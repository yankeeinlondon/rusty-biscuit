//! Collection builders for single and grouped RestApi definitions.

use std::collections::{BTreeMap, HashMap, HashSet};

use schematic_define::RestApi;

use crate::export::auth::{ExportAuth, map_auth, with_variable_suffix};
use crate::export::examples::ExampleSource;
use crate::export::path_params::extract_folder_key;

use super::auth::build_collection_auth;
use super::graphql::graphql_items;
use super::item::{Member, build_request_item};
use super::request::capitalize_folder_name;
use super::types::*;
use super::variables::{Collected, merge_variables};

const SCHEMA_URL: &str = "https://schema.getpostman.com/json/collection/v2.1.0/collection.json";

/// A collection with what building it collected: secret variables for an
/// environment, and diagnostics for examples or operations left out.
pub(crate) struct Built {
    pub collection: PostmanCollection,
    pub collected: Collected,
}

/// Builds a Postman collection from a single RestApi.
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, RestApi, Endpoint};
/// use schematic_gen::export::postman::build_postman_collection;
///
/// let mut ctx = DefinitionContext::types_in("crate::types");
/// let endpoint = Endpoint::get("GetUser", "/users/{id}")
///     .describes("Get a user by ID")
///     .returns_json::<serde_json::Value>(&mut ctx)
///     .build().unwrap();
/// let api = RestApi::builder(&ctx, "TestApi", "https://api.test.com/v1")
///     .describes("Test API")
///     .no_auth()
///     .endpoints([endpoint])
///     .build().unwrap();
///
/// let collection = build_postman_collection(&api);
/// assert_eq!(collection.info.name, "TestApi");
/// assert_eq!(collection.item.len(), 1); // 1 request
/// ```
pub fn build_postman_collection(api: &RestApi) -> PostmanCollection {
    build_single(api).collection
}

/// [`build_postman_collection`] with what the build collected.
pub(crate) fn build_single(api: &RestApi) -> Built {
    let auth = map_auth(&api.auth);
    let collection_auth = build_collection_auth(&auth);
    let member = Member {
        api,
        auth,
        base_url_var: "baseUrl".to_string(),
        request_level_auth: false,
        variable_suffix: None,
        examples: ExampleSource::new(api),
    };
    let description = (!api.description.is_empty()).then(|| api.description.clone());
    assemble(vec![member], api.name.clone(), description, collection_auth, &HashSet::new())
}

/// Builds a Postman collection from grouped RestApis sharing a module.
///
/// Merges endpoints from all APIs into one collection with:
/// - Title-cased `module_name` as collection name
/// - One base-URL variable per member (`baseUrl`, `baseUrl2`, …), even
///   when two members share a URL
/// - In a module of several APIs, credential variables suffixed with the
///   member's name (`bearerToken_EmqxBearer`), so members never share a
///   credential
/// - Collection-level auth when every member maps to the same
///   [`ExportAuth`] (after suffixing, only when none authenticates);
///   otherwise per-request auth and `collection.auth = None`
/// - All endpoints grouped by folder key, with duplicate request IDs
///   in mixed-auth groups disambiguated as `"<Id> (<ApiName>)"`
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, RestApi};
/// use schematic_gen::export::postman::build_postman_collection_grouped;
///
/// let ctx = DefinitionContext::types_in("crate::types");
/// let api1 = RestApi::builder(&ctx, "OllamaNative", "http://localhost:11434")
///     .describes("Native API")
///     .no_auth()
///     .module_path("ollama")
///     .build().unwrap();
///
/// let api2 = RestApi::builder(&ctx, "OllamaOpenAI", "http://localhost:11434/v1")
///     .describes("OpenAI-compatible API")
///     .no_auth()
///     .module_path("ollama")
///     .build().unwrap();
///
/// let collection = build_postman_collection_grouped(&[&api1, &api2], "ollama");
/// assert_eq!(collection.info.name, "Ollama");
/// assert!(collection.variable.len() >= 2); // At least 2 base URLs
/// ```
pub fn build_postman_collection_grouped(apis: &[&RestApi], module_name: &str) -> PostmanCollection {
    build_grouped(apis, module_name).collection
}

/// [`build_postman_collection_grouped`] with what the build collected.
pub(crate) fn build_grouped(apis: &[&RestApi], module_name: &str) -> Built {
    if apis.is_empty() {
        panic!("Cannot build grouped collection from empty API list");
    }
    let shared = apis.len() > 1;

    let auths: Vec<ExportAuth> = apis
        .iter()
        .map(|api| {
            let auth = map_auth(&api.auth);
            if shared { with_variable_suffix(auth, &api.name) } else { auth }
        })
        .collect();
    let uniform = auths.iter().all(|a| a == &auths[0]);
    let collection_auth = if uniform { build_collection_auth(&auths[0]) } else { None };

    // Only the request IDs that collide across members of a mixed-auth group
    // need disambiguation; unique IDs stay verbatim.
    let duplicate_ids: HashSet<String> = if uniform {
        HashSet::new()
    } else {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for api in apis {
            for endpoint in &api.endpoints {
                *counts.entry(endpoint.id.as_str()).or_insert(0) += 1;
            }
        }
        counts.into_iter().filter(|(_, n)| *n > 1).map(|(id, _)| id.to_string()).collect()
    };

    let members = apis
        .iter()
        .zip(auths)
        .enumerate()
        .map(|(index, (api, auth))| Member {
            api,
            auth,
            base_url_var: numbered("baseUrl", index),
            request_level_auth: !uniform,
            variable_suffix: shared.then(|| api.name.clone()),
            examples: ExampleSource::new(api),
        })
        .collect();

    let description = apis
        .iter()
        .map(|api| format!("{}: {}", api.name, api.description))
        .collect::<Vec<_>>()
        .join("\n");
    let description = (!description.is_empty()).then_some(description);
    assemble(members, capitalize_folder_name(module_name), description, collection_auth, &duplicate_ids)
}

/// `base` for the first member, then `base2`, `base3`, ….
fn numbered(base: &str, index: usize) -> String {
    if index == 0 { base.to_string() } else { format!("{base}{}", index + 1) }
}

/// Builds every member's requests and the collection's variables: base
/// URLs, GraphQL addresses, credentials, then parameters.
fn assemble(
    members: Vec<Member<'_>>,
    name: String,
    description: Option<String>,
    auth: Option<PostmanAuth>,
    duplicate_ids: &HashSet<String>,
) -> Built {
    let mut collected = Collected::default();
    let mut variables = Vec::new();
    let mut folders: BTreeMap<Option<String>, Vec<PostmanItem>> = BTreeMap::new();
    let mut graphql = Vec::new();

    for (index, member) in members.iter().enumerate() {
        let label = if members.len() > 1 { format!("Base URL for {}", member.api.name) } else { "Base URL for all requests".to_string() };
        variables.push(PostmanVariable {
            key: member.base_url_var.clone(),
            value: Some(member.api.base_url.clone()),
            description: Some(label),
        });
        collected.auth(&member.auth);
        for endpoint in &member.api.endpoints {
            let disambiguate = duplicate_ids.contains(&endpoint.id);
            let item = build_request_item(endpoint, member, disambiguate, &mut collected);
            folders.entry(extract_folder_key(&endpoint.path)).or_default().push(item);
        }
        if let Some((address, items)) = graphql_items(member, &numbered("graphqlUrl", index), &mut collected) {
            variables.push(address);
            graphql.extend(items);
        }
    }

    // Folders first (sorted by key), then GraphQL operations, then
    // root-level requests.
    let mut items = Vec::new();
    for (folder_key, folder_items) in &folders {
        if let Some(key) = folder_key {
            items.push(PostmanItem::Folder { name: capitalize_folder_name(key), item: folder_items.clone() });
        }
    }
    if !graphql.is_empty() {
        items.push(PostmanItem::Folder { name: "GraphQL".to_string(), item: graphql });
    }
    if let Some(root_items) = folders.remove(&None) {
        items.extend(root_items);
    }

    merge_variables(&mut variables, collected.credentials.clone());
    merge_variables(&mut variables, collected.parameters.clone());

    Built {
        collection: PostmanCollection {
            info: PostmanInfo { name, description, schema: SCHEMA_URL.to_string() },
            variable: variables,
            auth,
            item: items,
        },
        collected,
    }
}
