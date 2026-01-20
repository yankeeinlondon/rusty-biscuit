//! Module documentation generation for API clients.
//!
//! This module provides the [`ModuleDocBuilder`] struct for generating rich
//! module-level documentation for generated API modules. The documentation
//! includes an introduction, authentication details, feature list, and
//! usage examples.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{AuthStrategy, RestApi, RestMethod};
use std::collections::BTreeMap;

/// Builds module-level documentation for a generated API client.
///
/// The builder generates documentation sections including:
/// - Introduction paragraph with API name and description
/// - Authentication section explaining the auth strategy
/// - Features section listing endpoints grouped by HTTP method
/// - Example section with a basic usage example
///
/// ## Examples
///
/// ```ignore
/// use schematic_define::RestApi;
/// use schematic_gen::codegen::ModuleDocBuilder;
///
/// let api: RestApi = /* ... */;
/// let builder = ModuleDocBuilder::new(&api);
/// let doc_tokens = builder.build();
/// ```
pub struct ModuleDocBuilder<'a> {
    api: &'a RestApi,
}

impl<'a> ModuleDocBuilder<'a> {
    /// Creates a new module documentation builder for the given API.
    pub fn new(api: &'a RestApi) -> Self {
        Self { api }
    }

    /// Builds the complete module documentation as a token stream.
    ///
    /// The generated tokens include `#![doc = "..."]` attributes that
    /// form the module-level documentation.
    pub fn build(&self) -> TokenStream {
        let intro = self.intro_paragraph();
        let auth_section = self.auth_section();
        let features_section = self.features_section();
        let example_section = self.example_section();

        quote! {
            #![doc = #intro]
            //!
            #![doc = #auth_section]
            //!
            #![doc = #features_section]
            //!
            #![doc = #example_section]
        }
    }

    /// Generates the introduction paragraph.
    ///
    /// If a documentation URL is available, the API name is rendered as
    /// a markdown link. Otherwise, just the name is used.
    fn intro_paragraph(&self) -> String {
        let name = &self.api.name;
        let desc = &self.api.description;

        if let Some(docs_url) = &self.api.docs_url {
            format!(
                " Generated API client for [{}]({}).\n\n {}",
                name, docs_url, desc
            )
        } else {
            format!(" Generated API client for {}.\n\n {}", name, desc)
        }
    }

    /// Generates the authentication section.
    ///
    /// Documents the authentication strategy and any environment variables
    /// used for credentials.
    fn auth_section(&self) -> String {
        let auth_desc = match &self.api.auth {
            AuthStrategy::None => "No authentication required.".to_string(),
            AuthStrategy::BearerToken { header } => {
                let header_name = header.as_deref().unwrap_or("Authorization");
                format!(
                    "Uses Bearer token authentication via the `{}` header.",
                    header_name
                )
            }
            AuthStrategy::ApiKey { header } => {
                format!("Uses API key authentication via the `{}` header.", header)
            }
            AuthStrategy::Basic => "Uses HTTP Basic authentication.".to_string(),
        };

        let env_info = if !self.api.env_auth.is_empty() {
            format!(
                " Set via environment variable: `{}`.",
                self.api.env_auth.join("` or `")
            )
        } else {
            String::new()
        };

        format!(" ## Authentication\n\n {}{}", auth_desc, env_info)
    }

    /// Groups endpoints by their HTTP method.
    ///
    /// Returns a map from method name (e.g., "GET") to a list of
    /// (endpoint_id, description) pairs.
    fn categorize_endpoints(&self) -> BTreeMap<String, Vec<(String, String)>> {
        let mut categories: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for endpoint in &self.api.endpoints {
            let method = endpoint.method.to_string();
            categories
                .entry(method)
                .or_default()
                .push((endpoint.id.clone(), endpoint.description.clone()));
        }
        categories
    }

    /// Generates the features section.
    ///
    /// Lists all endpoints grouped by HTTP method.
    fn features_section(&self) -> String {
        let categories = self.categorize_endpoints();
        if categories.is_empty() {
            return " ## Features\n\n No endpoints defined.".to_string();
        }

        let mut lines = vec![" ## Features".to_string(), String::new()];
        for (method, endpoints) in &categories {
            lines.push(format!(" **{}**:", method));
            for (id, desc) in endpoints {
                lines.push(format!(" - `{}` - {}", id, desc));
            }
            lines.push(String::new());
        }
        lines.join("\n")
    }

    /// Generates the example section.
    ///
    /// Creates a usage example using the first GET endpoint, or the first
    /// endpoint if no GET endpoint exists.
    fn example_section(&self) -> String {
        // Find first GET endpoint, or fallback to first endpoint
        let endpoint = self
            .api
            .endpoints
            .iter()
            .find(|e| e.method == RestMethod::Get)
            .or_else(|| self.api.endpoints.first());

        let Some(endpoint) = endpoint else {
            return " ## Example\n\n No endpoints available for example.".to_string();
        };

        let api_name = &self.api.name;
        let method_name = to_snake_case(&endpoint.id);

        format!(
            r#" ## Example

 ```ignore
 use schematic_schema::prelude::*;

 #[tokio::main]
 async fn main() -> Result<(), SchematicError> {{
     let client = {}::new();
     let response = client.{}().await?;
     println!("{{:?}}", response);
     Ok(())
 }}
 ```"#,
            api_name, method_name
        )
    }
}

/// Converts a PascalCase string to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiResponse, Endpoint};

    fn make_test_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "A test API for documentation.".to_string(),
            base_url: "https://api.test.com".to_string(),
            docs_url: Some("https://docs.test.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List all items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
            }],
        }
    }

    #[test]
    fn builder_creates_valid_tokenstream() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn builder_includes_api_name() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        let code = tokens.to_string();
        assert!(code.contains("TestApi"));
    }

    #[test]
    fn builder_includes_docs_url_when_present() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        let code = tokens.to_string();
        assert!(code.contains("https://docs.test.com"));
    }

    #[test]
    fn builder_omits_docs_url_when_none() {
        let mut api = make_test_api();
        api.docs_url = None;
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        let code = tokens.to_string();
        // Should not have a markdown link format
        assert!(!code.contains("]("));
    }

    #[test]
    fn to_snake_case_converts_pascal_case() {
        assert_eq!(to_snake_case("ListItems"), "list_items");
        assert_eq!(to_snake_case("GetUser"), "get_user");
        assert_eq!(to_snake_case("CreateAPIKey"), "create_a_p_i_key");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn auth_section_bearer_token_default_header() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("Bearer token"));
        assert!(auth.contains("Authorization"));
        assert!(auth.contains("TEST_API_KEY"));
    }

    #[test]
    fn auth_section_bearer_token_custom_header() {
        let mut api = make_test_api();
        api.auth = AuthStrategy::BearerToken {
            header: Some("X-Auth-Token".to_string()),
        };
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("X-Auth-Token"));
    }

    #[test]
    fn auth_section_api_key() {
        let mut api = make_test_api();
        api.auth = AuthStrategy::ApiKey {
            header: "X-API-Key".to_string(),
        };
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("API key"));
        assert!(auth.contains("X-API-Key"));
    }

    #[test]
    fn auth_section_basic() {
        let mut api = make_test_api();
        api.auth = AuthStrategy::Basic;
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("Basic authentication"));
    }

    #[test]
    fn auth_section_none() {
        let mut api = make_test_api();
        api.auth = AuthStrategy::None;
        api.env_auth = vec![];
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("No authentication required"));
    }

    #[test]
    fn features_section_groups_by_method() {
        let mut api = make_test_api();
        api.endpoints.push(Endpoint {
            id: "CreateItem".to_string(),
            method: RestMethod::Post,
            path: "/items".to_string(),
            description: "Create a new item".to_string(),
            request: None,
            response: ApiResponse::json_type("CreateItemResponse"),
            headers: vec![],
        });
        api.endpoints.push(Endpoint {
            id: "GetItem".to_string(),
            method: RestMethod::Get,
            path: "/items/{id}".to_string(),
            description: "Get a specific item".to_string(),
            request: None,
            response: ApiResponse::json_type("Item"),
            headers: vec![],
        });

        let builder = ModuleDocBuilder::new(&api);
        let features = builder.features_section();

        assert!(features.contains("**GET**:"));
        assert!(features.contains("**POST**:"));
        assert!(features.contains("`ListItems`"));
        assert!(features.contains("`GetItem`"));
        assert!(features.contains("`CreateItem`"));
    }

    #[test]
    fn features_section_empty_endpoints() {
        let mut api = make_test_api();
        api.endpoints = vec![];
        let builder = ModuleDocBuilder::new(&api);
        let features = builder.features_section();
        assert!(features.contains("No endpoints defined"));
    }

    #[test]
    fn example_section_uses_first_get_endpoint() {
        let mut api = make_test_api();
        api.endpoints.insert(
            0,
            Endpoint {
                id: "CreateItem".to_string(),
                method: RestMethod::Post,
                path: "/items".to_string(),
                description: "Create a new item".to_string(),
                request: None,
                response: ApiResponse::json_type("CreateItemResponse"),
                headers: vec![],
            },
        );

        let builder = ModuleDocBuilder::new(&api);
        let example = builder.example_section();

        // Should use ListItems (GET) not CreateItem (POST)
        assert!(example.contains("list_items"));
        assert!(!example.contains("create_item"));
    }

    #[test]
    fn example_section_fallback_to_first_endpoint() {
        let mut api = make_test_api();
        api.endpoints = vec![Endpoint {
            id: "CreateItem".to_string(),
            method: RestMethod::Post,
            path: "/items".to_string(),
            description: "Create a new item".to_string(),
            request: None,
            response: ApiResponse::json_type("CreateItemResponse"),
            headers: vec![],
        }];

        let builder = ModuleDocBuilder::new(&api);
        let example = builder.example_section();

        assert!(example.contains("create_item"));
    }

    #[test]
    fn example_section_no_endpoints() {
        let mut api = make_test_api();
        api.endpoints = vec![];
        let builder = ModuleDocBuilder::new(&api);
        let example = builder.example_section();
        assert!(example.contains("No endpoints available"));
    }

    #[test]
    fn env_auth_multiple_vars() {
        let mut api = make_test_api();
        api.env_auth = vec!["PRIMARY_KEY".to_string(), "FALLBACK_KEY".to_string()];
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("`PRIMARY_KEY` or `FALLBACK_KEY`"));
    }
}
