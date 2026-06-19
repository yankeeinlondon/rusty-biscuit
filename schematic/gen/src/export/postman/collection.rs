//! Collection builders for single and grouped RestApi definitions.

use std::collections::BTreeMap;

use schematic_define::RestApi;

use crate::export::auth::{ExportAuth, map_auth};
use crate::export::path_params::extract_folder_key;

use super::auth::{auth_variables, build_collection_auth};
use super::item::build_request_item;
use super::request::capitalize_folder_name;
use super::types::*;
use super::variables::merge_variables;

/// Builds a Postman collection from a single RestApi.
///
/// ## Examples
///
/// ```
/// use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};
/// use schematic_gen::export::postman::build_postman_collection;
///
/// let api = RestApi {
///     name: "TestApi".to_string(),
///     description: "Test API".to_string(),
///     base_url: "https://api.test.com/v1".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![
///         Endpoint {
///             id: "GetUser".to_string(),
///             method: RestMethod::Get,
///             path: "/users/{id}".to_string(),
///             description: "Get a user by ID".to_string(),
///             request: None,
///             response: ApiResponse::json_type("User"),
///             headers: vec![],
///             params: None,
///             oauth_scopes: None,
///         },
///     ],
///     module_path: None,
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let collection = build_postman_collection(&api);
/// assert_eq!(collection.info.name, "TestApi");
/// assert_eq!(collection.item.len(), 1); // 1 request
/// ```
pub fn build_postman_collection(api: &RestApi) -> PostmanCollection {
    let auth = map_auth(&api.auth);
    let collection_auth = build_collection_auth(&auth);
    let base_url_var = PostmanVariable {
        key: "baseUrl".to_string(),
        value: Some(api.base_url.clone()),
        description: Some("Base URL for all requests".to_string()),
    };

    // Group endpoints by folder
    let mut folders: BTreeMap<Option<String>, Vec<PostmanItem>> = BTreeMap::new();

    for endpoint in &api.endpoints {
        let folder_key = extract_folder_key(&endpoint.path);
        let request_item = build_request_item(endpoint, api, &auth, false, false, "baseUrl");
        folders
            .entry(folder_key.clone())
            .or_default()
            .push(request_item);
    }

    // Build items: folders + root-level requests
    let mut items = Vec::new();

    // Add folders first (sorted by key)
    for (folder_key, folder_items) in folders.iter() {
        if let Some(key) = folder_key {
            items.push(PostmanItem::Folder {
                name: capitalize_folder_name(key),
                item: folder_items.clone(),
            });
        }
    }

    // Add root-level requests
    if let Some(root_items) = folders.get(&None) {
        items.extend(root_items.clone());
    }

    // Build collection variable list: base-URL var first, then any
    // auth-implied variables (e.g. bearerToken, apiKey, username, password).
    // Dedupe by key so a future overlap with the base-URL var name cannot
    // produce duplicate declarations.
    let mut variables = vec![base_url_var];
    merge_variables(&mut variables, auth_variables(&auth));

    PostmanCollection {
        info: PostmanInfo {
            name: api.name.clone(),
            description: if api.description.is_empty() {
                None
            } else {
                Some(api.description.clone())
            },
            schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                .to_string(),
        },
        variable: variables,
        auth: collection_auth,
        item: items,
    }
}

/// Builds a Postman collection from grouped RestApis sharing a module.
///
/// Merges endpoints from all APIs into one collection with:
/// - Title-cased `module_name` as collection name
/// - Collection-level auth when every member API maps to the same
///   [`ExportAuth`]; otherwise per-request auth and `collection.auth = None`
/// - Merged base URLs as variables (one per distinct `base_url`)
/// - Auth-implied variables unioned across every distinct member auth
/// - All endpoints grouped by folder key, with duplicate request IDs
///   in mixed-auth groups disambiguated as `"<Id> (<ApiName>)"`
///
/// ## Examples
///
/// ```
/// use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};
/// use schematic_gen::export::postman::build_postman_collection_grouped;
///
/// let api1 = RestApi {
///     name: "OllamaNative".to_string(),
///     description: "Native API".to_string(),
///     base_url: "http://localhost:11434".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: Some("ollama".to_string()),
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let api2 = RestApi {
///     name: "OllamaOpenAI".to_string(),
///     description: "OpenAI-compatible API".to_string(),
///     base_url: "http://localhost:11434/v1".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: Some("ollama".to_string()),
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let collection = build_postman_collection_grouped(&[&api1, &api2], "ollama");
/// assert_eq!(collection.info.name, "Ollama");
/// assert!(collection.variable.len() >= 2); // At least 2 base URLs
/// ```
pub fn build_postman_collection_grouped(apis: &[&RestApi], module_name: &str) -> PostmanCollection {
    if apis.is_empty() {
        panic!("Cannot build grouped collection from empty API list");
    }

    // Map every member API's auth once. This drives both the
    // uniform-vs-mixed decision and per-request auth emission below.
    let auths: Vec<ExportAuth> = apis.iter().map(|a| map_auth(&a.auth)).collect();
    let uniform = auths.iter().all(|a| a == &auths[0]);

    // Collection-level auth is set only when every member agrees. In a
    // mixed-auth group we omit collection.auth entirely and attach the
    // owning API's auth to each request instead.
    let collection_auth = if uniform {
        build_collection_auth(&auths[0])
    } else {
        None
    };

    // Collect all unique base URLs as variables, and map each base_url to its
    // assigned variable name so requests can reference the correct one.
    let mut base_url_vars = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    let mut base_url_var_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    for (idx, api) in apis.iter().enumerate() {
        if !seen_urls.contains(&api.base_url) {
            seen_urls.insert(&api.base_url);
            let var_name = if idx == 0 {
                "baseUrl".to_string()
            } else {
                format!("baseUrl{}", idx + 1)
            };
            base_url_var_map.insert(api.base_url.clone(), var_name.clone());
            base_url_vars.push(PostmanVariable {
                key: var_name,
                value: Some(api.base_url.clone()),
                description: Some(format!("Base URL for {}", api.name)),
            });
        }
    }

    // For mixed-auth groups, only the request IDs that actually collide
    // across member APIs need disambiguation. Unique IDs stay verbatim
    // so single-API endpoints (e.g. EmqxBearer's Login/Logout) keep
    // their tidy names.
    let duplicate_ids: std::collections::HashSet<String> = if uniform {
        std::collections::HashSet::new()
    } else {
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for api in apis {
            for endpoint in &api.endpoints {
                *counts.entry(endpoint.id.as_str()).or_insert(0) += 1;
            }
        }
        counts
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(id, _)| id.to_string())
            .collect()
    };

    // Merge all endpoints across APIs, grouped by folder
    let mut folders: BTreeMap<Option<String>, Vec<PostmanItem>> = BTreeMap::new();

    for (api, owning_auth) in apis.iter().zip(auths.iter()) {
        let base_url_var = base_url_var_map
            .get(&api.base_url)
            .map(String::as_str)
            .unwrap_or("baseUrl");
        for endpoint in &api.endpoints {
            let folder_key = extract_folder_key(&endpoint.path);
            let disambiguate = !uniform && duplicate_ids.contains(&endpoint.id);
            let request_item = build_request_item(
                endpoint,
                api,
                owning_auth,
                !uniform,
                disambiguate,
                base_url_var,
            );
            folders
                .entry(folder_key.clone())
                .or_default()
                .push(request_item);
        }
    }

    // Build items: folders + root-level requests
    let mut items = Vec::new();

    // Add folders first (sorted by key)
    for (folder_key, folder_items) in folders.iter() {
        if let Some(key) = folder_key {
            items.push(PostmanItem::Folder {
                name: capitalize_folder_name(key),
                item: folder_items.clone(),
            });
        }
    }

    // Add root-level requests
    if let Some(root_items) = folders.get(&None) {
        items.extend(root_items.clone());
    }

    // Capitalize module name for collection title
    let collection_name = capitalize_folder_name(module_name);

    // Merge descriptions from all APIs
    let description = apis
        .iter()
        .map(|api| format!("{}: {}", api.name, api.description))
        .collect::<Vec<_>>()
        .join("\n");

    // Union auth-implied variables across every distinct ExportAuth in
    // the group. For uniform groups this collapses to one auth's
    // variables; for mixed groups every per-request auth still resolves
    // to a declared collection variable.
    let mut variables = base_url_vars;
    let mut seen_auths: Vec<&ExportAuth> = Vec::new();
    for auth in &auths {
        if !seen_auths.contains(&auth) {
            merge_variables(&mut variables, auth_variables(auth));
            seen_auths.push(auth);
        }
    }

    PostmanCollection {
        info: PostmanInfo {
            name: collection_name,
            description: if description.is_empty() {
                None
            } else {
                Some(description)
            },
            schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                .to_string(),
        },
        variable: variables,
        auth: collection_auth,
        item: items,
    }
}
