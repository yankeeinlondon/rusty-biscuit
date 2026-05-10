//! Postman collection export.
//!
//! Generates Postman Collection v2.1.0 JSON files from schematic RestApi definitions.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use schematic_define::{RestApi, RestMethod};
use serde::Serialize;

use crate::errors::GeneratorError;
use crate::export::auth::{ApiKeyLocation, ExportAuth, map_auth};
use crate::export::body::{ExportBody, FormField, FormFieldExportKind, map_body};
use crate::export::naming::resolve_module_name;
use crate::export::path_params::extract_folder_key;
use crate::parser::extract_path_params;

/// Postman Collection v2.1.0 format.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanCollection {
    /// Collection metadata.
    pub info: PostmanInfo,
    /// Collection-level variables.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
    /// Collection-level authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    /// Collection items (folders and requests).
    pub item: Vec<PostmanItem>,
}

/// Collection information metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanInfo {
    /// Collection name.
    pub name: String,
    /// Collection description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Postman schema version URL.
    pub schema: String,
}

/// Collection item (folder or request).
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum PostmanItem {
    /// A folder containing nested items.
    Folder {
        /// Folder name.
        name: String,
        /// Nested items.
        item: Vec<PostmanItem>,
    },
    /// A single request.
    Request {
        /// Request name.
        name: String,
        /// Request details.
        request: Box<PostmanRequest>,
    },
}

/// HTTP request details.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanRequest {
    /// HTTP method (GET, POST, etc.).
    pub method: String,
    /// Request URL.
    pub url: PostmanUrl,
    /// Request-level authentication override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<PostmanAuth>,
    /// Request headers.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub header: Vec<PostmanHeader>,
    /// Request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<PostmanBody>,
    /// Request description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// URL components.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanUrl {
    /// Full URL string.
    pub raw: String,
    /// Host parts (including {{baseUrl}} variable).
    pub host: Vec<String>,
    /// Path segments.
    pub path: Vec<String>,
    /// Query parameters.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub query: Vec<PostmanQuery>,
    /// Path variables.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variable: Vec<PostmanVariable>,
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanAuth {
    /// Authentication type.
    #[serde(rename = "type")]
    pub type_field: String,
    /// Bearer token configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer: Option<Vec<PostmanVariable>>,
    /// API key configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apikey: Option<Vec<PostmanVariable>>,
    /// Basic auth configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic: Option<Vec<PostmanVariable>>,
}

/// Variable definition.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanVariable {
    /// Variable key.
    pub key: String,
    /// Variable value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Variable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// HTTP header.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanHeader {
    /// Header key.
    pub key: String,
    /// Header value.
    pub value: String,
}

/// Request body.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanBody {
    /// Body mode (raw, formdata, urlencoded, file).
    pub mode: String,
    /// Raw body content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
    /// Form data fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formdata: Option<Vec<PostmanFormParam>>,
    /// URL-encoded fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urlencoded: Option<Vec<PostmanFormParam>>,
    /// Body options (for raw mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<PostmanBodyOptions>,
}

/// Form parameter (for formdata or urlencoded).
#[derive(Debug, Clone, Serialize)]
pub struct PostmanFormParam {
    /// Field key.
    pub key: String,
    /// Field value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Field description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Field type (text or file).
    #[serde(rename = "type")]
    pub type_field: String,
}

/// Body options.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanBodyOptions {
    /// Raw body options.
    pub raw: PostmanRawOptions,
}

/// Raw body language options.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanRawOptions {
    /// Language for syntax highlighting.
    pub language: String,
}

/// Query parameter.
#[derive(Debug, Clone, Serialize)]
pub struct PostmanQuery {
    /// Query parameter key.
    pub key: String,
    /// Query parameter value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Query parameter description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Builds a Postman collection from a single RestApi.
///
/// ## Examples
///
/// ```
/// use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};
/// use schematic_gen::postman_output::build_postman_collection;
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

/// Builds collection-level authentication.
fn build_collection_auth(auth: &ExportAuth) -> Option<PostmanAuth> {
    match auth {
        ExportAuth::Bearer { variable } => Some(PostmanAuth {
            type_field: "bearer".to_string(),
            bearer: Some(vec![PostmanVariable {
                key: "token".to_string(),
                value: Some(format!("{{{{{}}}}}", variable)),
                description: None,
            }]),
            apikey: None,
            basic: None,
        }),
        ExportAuth::ApiKey {
            header,
            variable,
            location,
        } => Some(PostmanAuth {
            type_field: "apikey".to_string(),
            bearer: None,
            apikey: Some(vec![
                PostmanVariable {
                    key: "key".to_string(),
                    value: Some(header.clone()),
                    description: None,
                },
                PostmanVariable {
                    key: "value".to_string(),
                    value: Some(format!("{{{{{}}}}}", variable)),
                    description: None,
                },
                PostmanVariable {
                    key: "in".to_string(),
                    value: Some(match location {
                        ApiKeyLocation::Header => "header".to_string(),
                        ApiKeyLocation::Query => "query".to_string(),
                        ApiKeyLocation::Cookie => "cookie".to_string(),
                    }),
                    description: None,
                },
            ]),
            basic: None,
        }),
        ExportAuth::Basic {
            username_var,
            password_var,
        } => Some(PostmanAuth {
            type_field: "basic".to_string(),
            bearer: None,
            apikey: None,
            basic: Some(vec![
                PostmanVariable {
                    key: "username".to_string(),
                    value: Some(format!("{{{{{}}}}}", username_var)),
                    description: None,
                },
                PostmanVariable {
                    key: "password".to_string(),
                    value: Some(format!("{{{{{}}}}}", password_var)),
                    description: None,
                },
            ]),
        }),
        ExportAuth::None => Some(PostmanAuth {
            type_field: "noauth".to_string(),
            bearer: None,
            apikey: None,
            basic: None,
        }),
    }
}

/// Returns the collection-level Postman variables implied by the given
/// [`ExportAuth`].
///
/// The auth blocks generated by [`build_collection_auth`] reference
/// `{{bearerToken}}`, `{{apiKey}}`, `{{username}}`, and `{{password}}`
/// without declaring them. This helper produces matching
/// [`PostmanVariable`] entries (with empty `value` so importers prompt the
/// user) so collections can declare every variable they reference.
///
/// ## Returns
///
/// - [`ExportAuth::Bearer`] → one variable keyed `"bearerToken"`.
/// - [`ExportAuth::ApiKey`] → one variable keyed `"apiKey"`.
/// - [`ExportAuth::Basic`] → two variables keyed `"username"` and
///   `"password"`.
/// - [`ExportAuth::None`] → empty vector.
///
/// ## Notes
///
/// The variable keys mirror the names used in [`map_auth`] and
/// [`build_collection_auth`]. If those mappings change (e.g. a future
/// caller passes a non-default variable name through `ExportAuth::Bearer`),
/// update this helper in lockstep so the declared key matches the
/// reference inside the auth block.
fn auth_variables(auth: &ExportAuth) -> Vec<PostmanVariable> {
    match auth {
        ExportAuth::Bearer { variable } => vec![PostmanVariable {
            key: variable.clone(),
            value: Some(String::new()),
            description: Some("Bearer token for Authorization header".to_string()),
        }],
        ExportAuth::ApiKey { variable, .. } => vec![PostmanVariable {
            key: variable.clone(),
            value: Some(String::new()),
            description: Some("API key value".to_string()),
        }],
        ExportAuth::Basic {
            username_var,
            password_var,
        } => vec![
            PostmanVariable {
                key: username_var.clone(),
                value: Some(String::new()),
                description: Some("Username for Basic authentication".to_string()),
            },
            PostmanVariable {
                key: password_var.clone(),
                value: Some(String::new()),
                description: Some("Password for Basic authentication".to_string()),
            },
        ],
        ExportAuth::None => Vec::new(),
    }
}

/// Appends `additions` to `target`, dropping any whose `key` is already
/// declared. Preserves order: existing entries stay first, new entries
/// follow in argument order.
fn merge_variables(target: &mut Vec<PostmanVariable>, additions: Vec<PostmanVariable>) {
    for var in additions {
        if !target.iter().any(|existing| existing.key == var.key) {
            target.push(var);
        }
    }
}

/// Builds a request item from an endpoint.
///
/// `owning_auth` is the auth that should govern this specific request.
/// When `emit_request_level_auth` is `true`, that auth is serialized
/// directly on the request (used in mixed-auth grouped collections so
/// each request advertises its own auth even though the collection has
/// none). When `false`, the request inherits collection-level auth and
/// emits no `auth` block.
///
/// When `disambiguate_with_api_name` is `true`, the request name is
/// suffixed with the owning API name (e.g. `"ListAlarms (EmqxBasic)"`)
/// so duplicate IDs across grouped APIs become distinguishable in
/// Postman's UI and exports.
fn build_request_item(
    endpoint: &schematic_define::Endpoint,
    api: &RestApi,
    owning_auth: &ExportAuth,
    emit_request_level_auth: bool,
    disambiguate_with_api_name: bool,
    base_url_var: &str,
) -> PostmanItem {
    let path_params = extract_path_params(&endpoint.path);
    let url = build_url(
        &api.base_url,
        &endpoint.path,
        &path_params,
        endpoint,
        base_url_var,
    );
    let headers = build_headers(endpoint, api);
    let body = endpoint
        .request
        .as_ref()
        .map(|req| build_body(&map_body(req)));

    let name = if disambiguate_with_api_name {
        format!("{} ({})", endpoint.id, api.name)
    } else {
        endpoint.id.clone()
    };

    let auth = if emit_request_level_auth {
        build_collection_auth(owning_auth)
    } else {
        None
    };

    PostmanItem::Request {
        name,
        request: Box::new(PostmanRequest {
            method: rest_method_to_string(&endpoint.method),
            url,
            auth,
            header: headers,
            body,
            description: if endpoint.description.is_empty() {
                None
            } else {
                Some(endpoint.description.clone())
            },
        }),
    }
}

/// Builds the URL object for a request.
fn build_url(
    _base_url: &str,
    path: &str,
    path_params: &[&str],
    endpoint: &schematic_define::Endpoint,
    base_url_var: &str,
) -> PostmanUrl {
    // Build raw URL with the chosen base-URL variable and path params as :param
    let path_with_colon = path_params.iter().fold(path.to_string(), |acc, param| {
        acc.replace(&format!("{{{}}}", param), &format!(":{}", param))
    });
    let raw = format!("{{{{{}}}}}{}", base_url_var, path_with_colon);

    // Split path into segments, converting {param} to :param
    let path_segments: Vec<String> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') {
                format!(":{}", segment.trim_start_matches('{').trim_end_matches('}'))
            } else {
                segment.to_string()
            }
        })
        .collect();

    // Build path variables
    let variables: Vec<PostmanVariable> = path_params
        .iter()
        .map(|param| PostmanVariable {
            key: param.to_string(),
            value: Some(format!("<{}>", param)),
            description: None,
        })
        .collect();

    // Build query parameters
    let query_params = if let Some(ref params) = endpoint.params {
        params
            .query
            .iter()
            .map(|param| PostmanQuery {
                key: param.name.clone(),
                value: None, // Postman can populate this
                description: param.description.clone(),
            })
            .collect()
    } else {
        vec![]
    };

    PostmanUrl {
        raw,
        host: vec![format!("{{{{{}}}}}", base_url_var)],
        path: path_segments,
        query: query_params,
        variable: variables,
    }
}

/// Builds headers for a request.
fn build_headers(endpoint: &schematic_define::Endpoint, api: &RestApi) -> Vec<PostmanHeader> {
    let mut headers = Vec::new();

    // Add API-level headers
    for (key, value) in &api.headers {
        headers.push(PostmanHeader {
            key: key.clone(),
            value: value.clone(),
        });
    }

    // Add endpoint-level headers (override API-level)
    for (key, value) in &endpoint.headers {
        // Remove any API-level header with same key (case-insensitive)
        headers.retain(|h| !h.key.eq_ignore_ascii_case(key));
        headers.push(PostmanHeader {
            key: key.clone(),
            value: value.clone(),
        });
    }

    headers
}

/// Builds the body object for a request.
fn build_body(export_body: &ExportBody) -> PostmanBody {
    match export_body {
        ExportBody::Json { .. } => PostmanBody {
            mode: "raw".to_string(),
            raw: Some("{}".to_string()),
            formdata: None,
            urlencoded: None,
            options: Some(PostmanBodyOptions {
                raw: PostmanRawOptions {
                    language: "json".to_string(),
                },
            }),
        },
        ExportBody::FormData { fields } => PostmanBody {
            mode: "formdata".to_string(),
            raw: None,
            formdata: Some(fields.iter().map(build_form_param).collect()),
            urlencoded: None,
            options: None,
        },
        ExportBody::UrlEncoded { fields } => PostmanBody {
            mode: "urlencoded".to_string(),
            raw: None,
            formdata: None,
            urlencoded: Some(fields.iter().map(build_form_param).collect()),
            options: None,
        },
        ExportBody::Text { .. } => PostmanBody {
            mode: "raw".to_string(),
            raw: Some(String::new()),
            formdata: None,
            urlencoded: None,
            options: Some(PostmanBodyOptions {
                raw: PostmanRawOptions {
                    language: "text".to_string(),
                },
            }),
        },
        ExportBody::Binary => PostmanBody {
            mode: "file".to_string(),
            raw: None,
            formdata: None,
            urlencoded: None,
            options: None,
        },
    }
}

/// Builds a Postman form parameter, branching on field kind.
///
/// File and Files fields emit `type: "file"` with no `value` — Postman
/// expects users to attach a file via its `src` array, which we leave
/// unset so importers prompt for a path. Text and JSON-part fields emit
/// `type: "text"` with an empty default value (Postman has no native
/// JSON-part concept; preserving the discriminator on
/// [`FormFieldExportKind::Json`] lets future exporters opt into a richer
/// representation without re-deriving kind from the field name).
fn build_form_param(field: &FormField) -> PostmanFormParam {
    match &field.kind {
        FormFieldExportKind::File { .. } | FormFieldExportKind::Files { .. } => PostmanFormParam {
            key: field.name.clone(),
            value: None,
            description: field.description.clone(),
            type_field: "file".to_string(),
        },
        FormFieldExportKind::Text | FormFieldExportKind::Json => PostmanFormParam {
            key: field.name.clone(),
            value: Some(String::new()),
            description: field.description.clone(),
            type_field: "text".to_string(),
        },
    }
}

/// Converts RestMethod to uppercase string.
fn rest_method_to_string(method: &RestMethod) -> String {
    method.to_string()
}

/// Capitalizes the first letter of a folder name.
fn capitalize_folder_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
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
/// use schematic_gen::postman_output::build_postman_collection_grouped;
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

/// Writes a grouped Postman collection to disk.
///
/// File naming: `<module_name>.postman_collection.json`
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{RestApi, AuthStrategy};
/// use schematic_gen::postman_output::write_postman_grouped;
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
/// let path = write_postman_grouped(&[&api1], "ollama", Path::new("/tmp"), false).unwrap();
/// assert_eq!(path.file_name().unwrap(), "ollama.postman_collection.json");
/// ```
pub fn write_postman_grouped(
    apis: &[&RestApi],
    module_name: &str,
    dir: &Path,
    dry_run: bool,
) -> Result<PathBuf, GeneratorError> {
    let collection = build_postman_collection_grouped(apis, module_name);
    let filename = format!("{}.postman_collection.json", module_name);
    let path = dir.join(&filename);

    if dry_run {
        return Ok(path);
    }

    let json = serde_json::to_string_pretty(&collection).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman collection: {}", e))
    })?;

    fs::write(&path, json).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(path)
}

/// Writes a Postman collection to disk.
///
/// File naming: `<module_name>.postman_collection.json`
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use schematic_define::{RestApi, RestMethod, AuthStrategy, ApiResponse};
/// use schematic_gen::postman_output::write_postman;
///
/// let api = RestApi {
///     name: "TestApi".to_string(),
///     description: "Test API".to_string(),
///     base_url: "https://api.test.com".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: None,
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
///
/// let path = write_postman(&api, Path::new("/tmp"), false).unwrap();
/// assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");
/// ```
pub fn write_postman(api: &RestApi, dir: &Path, dry_run: bool) -> Result<PathBuf, GeneratorError> {
    let collection = build_postman_collection(api);
    let module_name = resolve_module_name(api);
    let filename = format!("{}.postman_collection.json", module_name);
    let path = dir.join(&filename);

    if dry_run {
        return Ok(path);
    }

    let json = serde_json::to_string_pretty(&collection).map_err(|e| {
        GeneratorError::CodeGenError(format!("Failed to serialize Postman collection: {}", e))
    })?;

    fs::write(&path, json).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiResponse, AuthStrategy, Endpoint};

    fn minimal_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "A test API".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            auth_policy: None,
            env_auth: vec![],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: None,
        }
    }

    #[test]
    fn build_minimal_collection() {
        let api = minimal_api();
        let collection = build_postman_collection(&api);

        assert_eq!(collection.info.name, "TestApi");
        assert_eq!(collection.info.description, Some("A test API".to_string()));
        assert_eq!(
            collection.info.schema,
            "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        );
        assert_eq!(collection.variable.len(), 1);
        assert_eq!(collection.variable[0].key, "baseUrl");
        assert_eq!(
            collection.variable[0].value,
            Some("https://api.test.com/v1".to_string())
        );
    }

    #[test]
    fn build_collection_with_endpoints() {
        let mut api = minimal_api();
        api.endpoints = vec![
            Endpoint {
                id: "GetUser".to_string(),
                method: RestMethod::Get,
                path: "/users/{id}".to_string(),
                description: "Get a user".to_string(),
                request: None,
                response: ApiResponse::json_type("User"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListUsers".to_string(),
                method: RestMethod::Get,
                path: "/users".to_string(),
                description: "List all users".to_string(),
                request: None,
                response: ApiResponse::json_type("UserList"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ];

        let collection = build_postman_collection(&api);
        assert_eq!(collection.item.len(), 1); // 1 folder: "users"

        if let PostmanItem::Folder { name, item } = &collection.item[0] {
            assert_eq!(name, "Users");
            assert_eq!(item.len(), 2);
        } else {
            panic!("Expected folder");
        }
    }

    #[test]
    fn auth_bearer_token() {
        let auth = ExportAuth::Bearer {
            variable: "bearerToken".to_string(),
        };
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "bearer");
        assert!(postman_auth.bearer.is_some());
        let bearer = postman_auth.bearer.unwrap();
        assert_eq!(bearer.len(), 1);
        assert_eq!(bearer[0].key, "token");
        assert_eq!(bearer[0].value, Some("{{bearerToken}}".to_string()));
    }

    #[test]
    fn auth_api_key() {
        let auth = ExportAuth::ApiKey {
            header: "X-API-Key".to_string(),
            variable: "apiKey".to_string(),
            location: ApiKeyLocation::Header,
        };
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "apikey");
        assert!(postman_auth.apikey.is_some());
        let apikey = postman_auth.apikey.unwrap();
        assert_eq!(apikey.len(), 3);
        assert_eq!(apikey[0].key, "key");
        assert_eq!(apikey[0].value, Some("X-API-Key".to_string()));
        assert_eq!(apikey[1].key, "value");
        assert_eq!(apikey[1].value, Some("{{apiKey}}".to_string()));
        assert_eq!(apikey[2].key, "in");
        assert_eq!(apikey[2].value, Some("header".to_string()));
    }

    #[test]
    fn auth_api_key_query_location() {
        let auth = ExportAuth::ApiKey {
            header: "api_key".to_string(),
            variable: "apiKey".to_string(),
            location: ApiKeyLocation::Query,
        };
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "apikey");
        let apikey = postman_auth.apikey.unwrap();
        assert_eq!(apikey[2].key, "in");
        assert_eq!(apikey[2].value, Some("query".to_string()));
    }

    #[test]
    fn auth_api_key_cookie_location() {
        let auth = ExportAuth::ApiKey {
            header: "api_key".to_string(),
            variable: "apiKey".to_string(),
            location: ApiKeyLocation::Cookie,
        };
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "apikey");
        let apikey = postman_auth.apikey.unwrap();
        assert_eq!(apikey[2].key, "in");
        assert_eq!(apikey[2].value, Some("cookie".to_string()));
    }

    #[test]
    fn auth_basic() {
        let auth = ExportAuth::Basic {
            username_var: "username".to_string(),
            password_var: "password".to_string(),
        };
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "basic");
        assert!(postman_auth.basic.is_some());
        let basic = postman_auth.basic.unwrap();
        assert_eq!(basic.len(), 2);
        assert_eq!(basic[0].key, "username");
        assert_eq!(basic[0].value, Some("{{username}}".to_string()));
        assert_eq!(basic[1].key, "password");
        assert_eq!(basic[1].value, Some("{{password}}".to_string()));
    }

    #[test]
    fn auth_none() {
        let auth = ExportAuth::None;
        let postman_auth = build_collection_auth(&auth).unwrap();

        assert_eq!(postman_auth.type_field, "noauth");
        assert!(postman_auth.bearer.is_none());
        assert!(postman_auth.apikey.is_none());
        assert!(postman_auth.basic.is_none());
    }

    #[test]
    fn body_json() {
        let body = ExportBody::Json {
            content_type: "application/json".to_string(),
        };
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "raw");
        assert_eq!(postman_body.raw, Some("{}".to_string()));
        assert!(postman_body.options.is_some());
        assert_eq!(postman_body.options.unwrap().raw.language, "json");
    }

    #[test]
    fn body_form_data() {
        let body = ExportBody::FormData {
            fields: vec![
                FormField {
                    name: "file".to_string(),
                    required: true,
                    description: Some("The file".to_string()),
                    kind: FormFieldExportKind::File { accept: vec![] },
                },
                FormField {
                    name: "name".to_string(),
                    required: false,
                    description: None,
                    kind: FormFieldExportKind::Text,
                },
            ],
        };
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "formdata");
        assert!(postman_body.formdata.is_some());
        let formdata = postman_body.formdata.unwrap();
        assert_eq!(formdata.len(), 2);
        assert_eq!(formdata[0].key, "file");
        assert_eq!(formdata[0].description, Some("The file".to_string()));
        assert_eq!(formdata[0].type_field, "file");
        assert!(formdata[0].value.is_none());
        assert_eq!(formdata[1].key, "name");
        assert_eq!(formdata[1].type_field, "text");
        assert_eq!(formdata[1].value, Some(String::new()));
    }

    #[test]
    fn body_url_encoded() {
        let body = ExportBody::UrlEncoded {
            fields: vec![FormField {
                name: "grant_type".to_string(),
                required: true,
                description: None,
                kind: FormFieldExportKind::Text,
            }],
        };
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "urlencoded");
        assert!(postman_body.urlencoded.is_some());
        let urlencoded = postman_body.urlencoded.unwrap();
        assert_eq!(urlencoded.len(), 1);
        assert_eq!(urlencoded[0].key, "grant_type");
        assert_eq!(urlencoded[0].type_field, "text");
    }

    #[test]
    fn form_param_file_emits_type_file() {
        let field = FormField {
            name: "audio".to_string(),
            required: true,
            description: None,
            kind: FormFieldExportKind::File {
                accept: vec!["audio/*".to_string()],
            },
        };
        let param = build_form_param(&field);
        assert_eq!(param.type_field, "file");
        assert!(param.value.is_none());
        assert_eq!(param.key, "audio");
    }

    #[test]
    fn form_param_files_emits_type_file() {
        let field = FormField {
            name: "samples".to_string(),
            required: true,
            description: None,
            kind: FormFieldExportKind::Files {
                accept: vec![],
                min: Some(1),
                max: Some(10),
            },
        };
        let param = build_form_param(&field);
        assert_eq!(param.type_field, "file");
        assert!(param.value.is_none());
    }

    #[test]
    fn form_param_text_emits_type_text() {
        let field = FormField {
            name: "name".to_string(),
            required: true,
            description: None,
            kind: FormFieldExportKind::Text,
        };
        let param = build_form_param(&field);
        assert_eq!(param.type_field, "text");
        assert_eq!(param.value, Some(String::new()));
    }

    #[test]
    fn form_param_json_emits_type_text() {
        let field = FormField {
            name: "metadata".to_string(),
            required: true,
            description: None,
            kind: FormFieldExportKind::Json,
        };
        let param = build_form_param(&field);
        assert_eq!(param.type_field, "text");
        assert_eq!(param.value, Some(String::new()));
    }

    #[test]
    fn body_form_data_real_elevenlabs_upload() {
        // Mirrors the AddVoiceSample endpoint in
        // schematic_definitions::elevenlabs: an `audio` file part with
        // an MP3/WAV accept pattern and an optional `name` text part.
        let request = schematic_define::request::ApiRequest::form_data(vec![
            schematic_define::request::FormField::file_accept("audio", vec!["audio/*".into()])
                .with_description("Audio file (mp3, wav, ogg, m4a)"),
            schematic_define::request::FormField::text("name")
                .optional()
                .with_description("Name for the sample"),
        ]);
        let body = map_body(&request);
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "formdata");
        let formdata = postman_body.formdata.expect("formdata array");
        assert_eq!(formdata.len(), 2);
        let audio = formdata
            .iter()
            .find(|p| p.key == "audio")
            .expect("audio part present");
        assert_eq!(audio.type_field, "file");
        assert!(audio.value.is_none());
        let name = formdata
            .iter()
            .find(|p| p.key == "name")
            .expect("name part present");
        assert_eq!(name.type_field, "text");
        assert_eq!(name.value, Some(String::new()));
    }

    #[test]
    fn body_form_data_real_unfolded_circle_file_upload() {
        // Mirrors the multiple unfolded_circle/core_rest endpoints that
        // accept `FormField::file("file")` with no MIME restrictions.
        let request = schematic_define::request::ApiRequest::form_data(vec![
            schematic_define::request::FormField::file("file"),
        ]);
        let body = map_body(&request);
        let postman_body = build_body(&body);

        let formdata = postman_body.formdata.expect("formdata array");
        assert_eq!(formdata.len(), 1);
        assert_eq!(formdata[0].key, "file");
        assert_eq!(formdata[0].type_field, "file");
        assert!(formdata[0].value.is_none());
    }

    #[test]
    fn body_text() {
        let body = ExportBody::Text {
            content_type: "text/plain".to_string(),
        };
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "raw");
        assert_eq!(postman_body.raw, Some(String::new()));
        assert!(postman_body.options.is_some());
        assert_eq!(postman_body.options.unwrap().raw.language, "text");
    }

    #[test]
    fn body_binary() {
        let body = ExportBody::Binary;
        let postman_body = build_body(&body);

        assert_eq!(postman_body.mode, "file");
        assert!(postman_body.raw.is_none());
        assert!(postman_body.formdata.is_none());
        assert!(postman_body.urlencoded.is_none());
    }

    #[test]
    fn url_with_path_params() {
        let endpoint = Endpoint {
            id: "GetModel".to_string(),
            method: RestMethod::Get,
            path: "/models/{model}".to_string(),
            description: "Get a model".to_string(),
            request: None,
            response: ApiResponse::json_type("Model"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        };

        let path_params = extract_path_params(&endpoint.path);
        let url = build_url(
            "https://api.test.com/v1",
            &endpoint.path,
            &path_params,
            &endpoint,
            "baseUrl",
        );

        assert_eq!(url.raw, "{{baseUrl}}/models/:model");
        assert_eq!(url.host, vec!["{{baseUrl}}"]);
        assert_eq!(url.path, vec!["models", ":model"]);
        assert_eq!(url.variable.len(), 1);
        assert_eq!(url.variable[0].key, "model");
        assert_eq!(url.variable[0].value, Some("<model>".to_string()));
    }

    #[test]
    fn folder_grouping() {
        let mut api = minimal_api();
        api.endpoints = vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: String::new(),
                request: None,
                response: ApiResponse::json_type("ModelList"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: String::new(),
                request: None,
                response: ApiResponse::json_type("Model"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetHealth".to_string(),
                method: RestMethod::Get,
                path: "/health".to_string(),
                description: String::new(),
                request: None,
                response: ApiResponse::json_type("Health"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ];

        let collection = build_postman_collection(&api);
        // Should have 2 folders: "Models" and "Health"
        assert_eq!(collection.item.len(), 2);

        // Verify folder names
        let folder_names: Vec<String> = collection
            .item
            .iter()
            .filter_map(|item| {
                if let PostmanItem::Folder { name, .. } = item {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(folder_names, vec!["Health", "Models"]);
    }

    #[test]
    fn rest_method_to_string_conversion() {
        assert_eq!(rest_method_to_string(&RestMethod::Get), "GET");
        assert_eq!(rest_method_to_string(&RestMethod::Post), "POST");
        assert_eq!(rest_method_to_string(&RestMethod::Put), "PUT");
        assert_eq!(rest_method_to_string(&RestMethod::Patch), "PATCH");
        assert_eq!(rest_method_to_string(&RestMethod::Delete), "DELETE");
        assert_eq!(rest_method_to_string(&RestMethod::Head), "HEAD");
        assert_eq!(rest_method_to_string(&RestMethod::Options), "OPTIONS");
    }

    #[test]
    fn capitalize_folder_name_cases() {
        assert_eq!(capitalize_folder_name("models"), "Models");
        assert_eq!(capitalize_folder_name("users"), "Users");
        assert_eq!(capitalize_folder_name(""), "");
        assert_eq!(capitalize_folder_name("a"), "A");
    }

    #[test]
    fn write_postman_dry_run() {
        let api = minimal_api();
        let temp_dir = std::env::temp_dir();
        let path = write_postman(&api, &temp_dir, true).unwrap();

        assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");
    }

    #[test]
    fn write_postman_creates_file() {
        use tempfile::TempDir;

        let api = minimal_api();
        let temp_dir = TempDir::new().unwrap();
        let path = write_postman(&api, temp_dir.path(), false).unwrap();

        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "testapi.postman_collection.json");

        // Verify it's valid JSON
        let content = fs::read_to_string(&path).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn postman_collection_json_structure() {
        let mut api = minimal_api();
        api.auth = AuthStrategy::BearerToken { header: None };
        api.endpoints = vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "List all models".to_string(),
                request: None,
                response: ApiResponse::json_type("ModelList"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "CreateCompletion".to_string(),
                method: RestMethod::Post,
                path: "/chat/completions".to_string(),
                description: "Create a chat completion".to_string(),
                request: Some(schematic_define::ApiRequest::json_type(
                    "CreateCompletionRequest",
                )),
                response: ApiResponse::json_type("Completion"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ];

        let collection = build_postman_collection(&api);
        let json = serde_json::to_string_pretty(&collection).unwrap();

        // Verify key structure
        assert!(json.contains("\"info\""));
        assert!(json.contains("\"variable\""));
        assert!(json.contains("\"auth\""));
        assert!(json.contains("\"item\""));
        assert!(json.contains("\"bearer\""));
        assert!(json.contains("{{baseUrl}}"));
        // The collection variable list must declare bearerToken so the
        // {{bearerToken}} reference in the auth block resolves on import.
        assert!(json.contains("\"bearerToken\""));

        // Print for manual inspection
        println!("\n{}", json);
    }

    #[test]
    fn auth_variables_for_bearer_returns_one_variable_named_bearer_token() {
        let auth = ExportAuth::Bearer {
            variable: "bearerToken".to_string(),
        };
        let vars = auth_variables(&auth);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].key, "bearerToken");
        assert_eq!(vars[0].value, Some(String::new()));
        assert!(vars[0].description.is_some());
    }

    #[test]
    fn auth_variables_for_api_key_returns_api_key_variable() {
        let auth = ExportAuth::ApiKey {
            header: "X-API-Key".to_string(),
            variable: "apiKey".to_string(),
            location: ApiKeyLocation::Header,
        };
        let vars = auth_variables(&auth);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].key, "apiKey");
    }

    #[test]
    fn auth_variables_for_basic_returns_username_and_password() {
        let auth = ExportAuth::Basic {
            username_var: "username".to_string(),
            password_var: "password".to_string(),
        };
        let vars = auth_variables(&auth);
        assert_eq!(vars.len(), 2);
        assert_eq!(vars[0].key, "username");
        assert_eq!(vars[1].key, "password");
    }

    #[test]
    fn auth_variables_for_none_returns_empty() {
        let auth = ExportAuth::None;
        let vars = auth_variables(&auth);
        assert!(vars.is_empty());
    }

    #[test]
    fn build_postman_collection_bearer_declares_bearer_token() {
        let mut api = minimal_api();
        api.auth = AuthStrategy::BearerToken { header: None };
        let collection = build_postman_collection(&api);
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"baseUrl"));
        assert!(
            keys.contains(&"bearerToken"),
            "expected bearerToken in {:?}",
            keys,
        );
    }

    #[test]
    fn build_postman_collection_basic_declares_username_and_password() {
        let mut api = minimal_api();
        api.auth = AuthStrategy::Basic;
        let collection = build_postman_collection(&api);
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"baseUrl"));
        assert!(keys.contains(&"username"), "missing username in {:?}", keys);
        assert!(keys.contains(&"password"), "missing password in {:?}", keys);
    }

    #[test]
    fn build_postman_collection_api_key_declares_api_key_variable() {
        let mut api = minimal_api();
        api.auth = AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        };
        let collection = build_postman_collection(&api);
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"baseUrl"));
        assert!(keys.contains(&"apiKey"), "missing apiKey in {:?}", keys);
    }

    #[test]
    fn build_postman_collection_none_declares_only_base_url() {
        let api = minimal_api();
        let collection = build_postman_collection(&api);
        assert_eq!(collection.variable.len(), 1);
        assert_eq!(collection.variable[0].key, "baseUrl");
    }

    #[test]
    fn merge_variables_dedupes_by_key() {
        let mut target = vec![PostmanVariable {
            key: "baseUrl".to_string(),
            value: None,
            description: None,
        }];
        merge_variables(
            &mut target,
            vec![
                PostmanVariable {
                    key: "baseUrl".to_string(),
                    value: Some("dup".to_string()),
                    description: None,
                },
                PostmanVariable {
                    key: "bearerToken".to_string(),
                    value: None,
                    description: None,
                },
            ],
        );
        assert_eq!(target.len(), 2);
        assert_eq!(target[0].key, "baseUrl");
        // First-write-wins: the original `baseUrl` (with value=None) survived.
        assert!(target[0].value.is_none());
        assert_eq!(target[1].key, "bearerToken");
    }

    #[test]
    fn build_postman_collection_grouped_uniform_bearer_declares_bearer_token() {
        let mut api1 = minimal_api();
        api1.name = "OneBearer".to_string();
        api1.auth = AuthStrategy::BearerToken { header: None };
        let mut api2 = minimal_api();
        api2.name = "TwoBearer".to_string();
        api2.base_url = "https://api2.test.com/v1".to_string();
        api2.auth = AuthStrategy::BearerToken { header: None };

        let collection = build_postman_collection_grouped(&[&api1, &api2], "test_module");
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"baseUrl"));
        assert!(keys.contains(&"baseUrl2"));
        assert!(
            keys.contains(&"bearerToken"),
            "missing bearerToken in {:?}",
            keys,
        );
        // Bearer should not be declared twice.
        let bearer_count = collection
            .variable
            .iter()
            .filter(|v| v.key == "bearerToken")
            .count();
        assert_eq!(bearer_count, 1);
    }

    /// Helper: build a minimal endpoint with a given id and path so the
    /// grouped-auth tests can construct realistic API surfaces.
    fn endpoint(id: &str, path: &str) -> Endpoint {
        Endpoint {
            id: id.to_string(),
            method: RestMethod::Get,
            path: path.to_string(),
            description: String::new(),
            request: None,
            response: ApiResponse::json_type("Resp"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }
    }

    /// Helper: collect every request item out of a built collection so
    /// tests can iterate without recursing through the folder enum.
    fn collect_requests(collection: &PostmanCollection) -> Vec<(String, &PostmanRequest)> {
        let mut out = Vec::new();
        for item in &collection.item {
            match item {
                PostmanItem::Request { name, request } => {
                    out.push((name.clone(), request.as_ref()));
                }
                PostmanItem::Folder { item, .. } => {
                    for inner in item {
                        if let PostmanItem::Request { name, request } = inner {
                            out.push((name.clone(), request.as_ref()));
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn grouped_uniform_auth_uses_collection_auth_and_no_request_auth() {
        let mut api1 = minimal_api();
        api1.name = "OneBearer".to_string();
        api1.auth = AuthStrategy::BearerToken { header: None };
        api1.endpoints = vec![endpoint("ListA", "/a"), endpoint("ListB", "/b")];

        let mut api2 = minimal_api();
        api2.name = "TwoBearer".to_string();
        api2.base_url = "https://api2.test.com/v1".to_string();
        api2.auth = AuthStrategy::BearerToken { header: None };
        api2.endpoints = vec![endpoint("ListC", "/c")];

        let collection = build_postman_collection_grouped(&[&api1, &api2], "uniform");
        let coll_auth = collection.auth.as_ref().expect("collection auth set");
        assert_eq!(coll_auth.type_field, "bearer");

        for (name, request) in collect_requests(&collection) {
            assert!(
                request.auth.is_none(),
                "request {} should inherit collection auth",
                name,
            );
            // Names are not disambiguated when auth is uniform.
            assert!(
                !name.contains('('),
                "uniform auth should not disambiguate names: {}",
                name,
            );
        }
    }

    #[test]
    fn grouped_mixed_auth_omits_collection_auth_and_emits_per_request() {
        let mut basic = minimal_api();
        basic.name = "BasicApi".to_string();
        basic.auth = AuthStrategy::Basic;
        basic.endpoints = vec![
            endpoint("ListItems", "/items"),
            endpoint("BasicOnly", "/bo"),
        ];

        let mut bearer = minimal_api();
        bearer.name = "BearerApi".to_string();
        bearer.base_url = "https://api2.test.com/v1".to_string();
        bearer.auth = AuthStrategy::BearerToken { header: None };
        bearer.endpoints = vec![
            endpoint("ListItems", "/items"),
            endpoint("BearerOnly", "/be"),
        ];

        let collection = build_postman_collection_grouped(&[&basic, &bearer], "mixed");
        assert!(
            collection.auth.is_none(),
            "mixed-auth collection must omit collection.auth",
        );

        let requests = collect_requests(&collection);
        // ListItems is duplicated; BasicOnly + BearerOnly are unique.
        assert_eq!(requests.len(), 4);

        for (name, request) in &requests {
            let auth = request
                .auth
                .as_ref()
                .unwrap_or_else(|| panic!("request {} missing auth", name));
            // Auth type is whichever the owning API uses.
            if name.contains("BasicApi") || name == "BasicOnly" {
                assert_eq!(auth.type_field, "basic", "{}", name);
            } else if name.contains("BearerApi") || name == "BearerOnly" {
                assert_eq!(auth.type_field, "bearer", "{}", name);
            } else {
                panic!("unexpected request name in mixed group: {}", name);
            }
        }
    }

    #[test]
    fn grouped_mixed_auth_declares_both_auth_variable_sets() {
        let mut basic = minimal_api();
        basic.name = "BasicApi".to_string();
        basic.auth = AuthStrategy::Basic;
        basic.endpoints = vec![endpoint("X", "/x")];

        let mut bearer = minimal_api();
        bearer.name = "BearerApi".to_string();
        bearer.base_url = "https://api2.test.com/v1".to_string();
        bearer.auth = AuthStrategy::BearerToken { header: None };
        bearer.endpoints = vec![endpoint("Y", "/y")];

        let collection = build_postman_collection_grouped(&[&basic, &bearer], "mixed");
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"username"), "missing username in {:?}", keys);
        assert!(keys.contains(&"password"), "missing password in {:?}", keys);
        assert!(
            keys.contains(&"bearerToken"),
            "missing bearerToken in {:?}",
            keys,
        );
    }

    #[test]
    fn grouped_mixed_auth_disambiguates_duplicate_request_names() {
        let mut basic = minimal_api();
        basic.name = "EmqxBasic".to_string();
        basic.auth = AuthStrategy::Basic;
        basic.endpoints = vec![endpoint("ListAlarms", "/alarms")];

        let mut bearer = minimal_api();
        bearer.name = "EmqxBearer".to_string();
        bearer.base_url = "http://localhost:18083/api/v5".to_string();
        bearer.auth = AuthStrategy::BearerToken { header: None };
        bearer.endpoints = vec![endpoint("ListAlarms", "/alarms")];

        let collection = build_postman_collection_grouped(&[&basic, &bearer], "emqx");
        let names: Vec<String> = collect_requests(&collection)
            .into_iter()
            .map(|(n, _)| n)
            .collect();

        assert!(
            names.contains(&"ListAlarms (EmqxBasic)".to_string()),
            "expected disambiguated Basic name in {:?}",
            names,
        );
        assert!(
            names.contains(&"ListAlarms (EmqxBearer)".to_string()),
            "expected disambiguated Bearer name in {:?}",
            names,
        );
        // Verify the bare name no longer appears.
        assert!(
            !names.iter().any(|n| n == "ListAlarms"),
            "bare ListAlarms must be replaced when duplicated: {:?}",
            names,
        );
    }

    #[test]
    fn grouped_mixed_auth_preserves_unique_request_names() {
        let mut basic = minimal_api();
        basic.name = "EmqxBasic".to_string();
        basic.auth = AuthStrategy::Basic;
        basic.endpoints = vec![
            endpoint("ListAlarms", "/alarms"), // duplicated
            endpoint("BasicOnly", "/bo"),      // unique to Basic
        ];

        let mut bearer = minimal_api();
        bearer.name = "EmqxBearer".to_string();
        bearer.base_url = "http://localhost:18083/api/v5".to_string();
        bearer.auth = AuthStrategy::BearerToken { header: None };
        bearer.endpoints = vec![
            endpoint("ListAlarms", "/alarms"), // duplicated
            endpoint("Login", "/login"),       // unique to Bearer
        ];

        let collection = build_postman_collection_grouped(&[&basic, &bearer], "emqx");
        let names: Vec<String> = collect_requests(&collection)
            .into_iter()
            .map(|(n, _)| n)
            .collect();

        // Unique IDs stay verbatim; duplicates are renamed.
        assert!(names.contains(&"BasicOnly".to_string()), "{:?}", names);
        assert!(names.contains(&"Login".to_string()), "{:?}", names);
        assert!(
            names.contains(&"ListAlarms (EmqxBasic)".to_string()),
            "{:?}",
            names,
        );
        assert!(
            names.contains(&"ListAlarms (EmqxBearer)".to_string()),
            "{:?}",
            names,
        );
    }

    #[test]
    fn build_postman_collection_grouped_mixed_auth_unions_variables() {
        let mut api1 = minimal_api();
        api1.name = "BasicApi".to_string();
        api1.auth = AuthStrategy::Basic;
        let mut api2 = minimal_api();
        api2.name = "BearerApi".to_string();
        api2.base_url = "https://api2.test.com/v1".to_string();
        api2.auth = AuthStrategy::BearerToken { header: None };

        let collection = build_postman_collection_grouped(&[&api1, &api2], "mixed_module");
        let keys: Vec<&str> = collection.variable.iter().map(|v| v.key.as_str()).collect();
        assert!(keys.contains(&"username"), "missing username in {:?}", keys);
        assert!(keys.contains(&"password"), "missing password in {:?}", keys);
        assert!(
            keys.contains(&"bearerToken"),
            "missing bearerToken in {:?}",
            keys,
        );
    }
}
