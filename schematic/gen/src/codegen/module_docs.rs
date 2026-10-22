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

/// Collapses a multi-line endpoint description into a single doc-list line.
///
/// Spec descriptions routinely span paragraphs and contain their own Markdown
/// lists. Emitted verbatim into a `//!` bullet they read as unindented list
/// continuations, which rustdoc renders wrongly and clippy flags as
/// `doc_lazy_continuation`.
fn summarize(description: &str) -> String {
    let first_paragraph = description.split("\n\n").next().unwrap_or(description);
    first_paragraph.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Builds module-level documentation for a generated API client.
///
/// The builder generates documentation sections including:
/// - Introduction paragraph with API name and description
/// - Authentication section explaining the auth strategy
/// - Features section listing endpoints grouped by HTTP method
/// - Rate limit and retry guidance sections, when the API declares them
///   (documentation only; the client acts on neither)
/// - Example section with a basic usage example
///
/// ## Examples
///
/// ```text
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
    /// The generated tokens are `//!` style line doc comments rather than
    /// `#![doc = "..."]` attributes, for consistency with hand-written code.
    pub fn build(&self) -> TokenStream {
        let intro = self.intro_paragraph();
        let auth_section = self.auth_section();
        let features_section = self.features_section();
        let metadata_section = self.metadata_section();
        let example_section = self.example_section();

        // Combine all sections with blank lines between them
        let full_doc = format!(
            "{}\n\n{}\n\n{}\n\n{}{}",
            intro.trim(),
            auth_section.trim(),
            features_section.trim(),
            metadata_section,
            example_section.trim()
        );

        // Convert to //! style line comments
        lines_to_doc_comments(&full_doc)
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
                "Generated API client for [{}]({}).\n\n{}",
                name, docs_url, desc
            )
        } else {
            format!("Generated API client for {}.\n\n{}", name, desc)
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
            AuthStrategy::ApiKey { header, .. } => {
                format!("Uses API key authentication via the `{}` header.", header)
            }
            AuthStrategy::Basic { .. } => "Uses HTTP Basic authentication.".to_string(),
            AuthStrategy::OAuth2(config) => {
                let flow = match config.grant_type() {
                    schematic_define::OAuth2GrantType::AuthorizationCodePkce => {
                        "Authorization Code with PKCE"
                    }
                    schematic_define::OAuth2GrantType::ClientCredentials => "Client Credentials",
                    schematic_define::OAuth2GrantType::DeviceCode => "Device Code",
                    _ => "OAuth2",
                };
                let scopes_info = if config.default_scopes().is_empty() {
                    String::new()
                } else {
                    format!(" Default scopes: `{}`.", config.default_scopes().join("`, `"))
                };
                format!("Uses OAuth2 authentication ({} flow).{}", flow, scopes_info)
            }
            // Handle future variants (non_exhaustive)
            _ => "Uses custom authentication.".to_string(),
        };

        // The variables the generated client actually reads, which can be
        // fewer than `env_auth` names: a definition's environment mapping
        // decides.
        let env_info = match crate::codegen::api_struct::env_fallback_vars(self.api) {
            Some((vars, username_vars)) if !username_vars.is_empty() => format!(
                " Without a supplied credential, the client reads the username from `{}` and \
                 the password from `{}`.",
                username_vars.join("` or `"),
                vars.join("` or `")
            ),
            Some((vars, _)) => format!(
                " Without a supplied credential, the client reads `{}`.",
                vars.join("` or `")
            ),
            None => String::new(),
        };

        format!("## Authentication\n\n{}{}", auth_desc, env_info)
    }

    /// The API-level rate-limit and retry guidance sections, each followed
    /// by a blank line, or nothing when the API declares neither. An
    /// endpoint's own value is documented on its request type instead.
    fn metadata_section(&self) -> String {
        use crate::codegen::request_structs::endpoint_docs::{rate_limit_lines, retry_guidance_lines};

        let mut sections = String::new();
        if let Some(rate_limit) = &self.api.rate_limit {
            sections.push_str(&rate_limit_lines(rate_limit).join("\n"));
            sections.push_str("\n\n");
        }
        if let Some(guidance) = &self.api.retry_guidance {
            sections.push_str(&retry_guidance_lines(guidance).join("\n"));
            sections.push_str("\n\n");
        }
        sections
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
    /// Lists all endpoints grouped by HTTP method, each description flattened
    /// by [`summarize`].
    fn features_section(&self) -> String {
        let categories = self.categorize_endpoints();
        if categories.is_empty() {
            return "## Features\n\nNo endpoints defined.".to_string();
        }

        let mut lines = vec!["## Features".to_string(), String::new()];
        for (method, endpoints) in &categories {
            lines.push(format!("**{}**:", method));
            for (id, desc) in endpoints {
                lines.push(format!("- `{}` - {}", id, summarize(desc)));
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
            return "## Example\n\nNo endpoints available for example.".to_string();
        };

        let api_name = &self.api.name;
        let call = crate::codegen::endpoint_methods::example_invocation(self.api, endpoint, "client");
        let custom_base_url = if self.api.base_url == "http://192.168.1.1:9529" {
            "http://192.168.1.50:9529"
        } else {
            "https://staging.example.com/v1"
        };

        format!(
            r#"## Examples

```text
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let client = {api_name}::new();
    let data = {call}.await?;

    // the whole response: status, headers, and request id as well
    let response = {call}.full().await?;
    println!("{{}}", response.meta);
    Ok(())
}}
```

### Override Base URL

```text
use schematic_schema::prelude::*;

let client = {api_name}::new().base_url("{custom_base_url}");
```"#
        )
    }
}

/// Converts a multi-line documentation string into `//!` style line comments.
///
/// Exposed as `pub(crate)` for use by combined module assembly.
///
/// Each line of the input becomes a separate `#![doc = "..."]` attribute.
/// When prettyplease formats these single-line doc attributes, they become
/// `//! ` style comments rather than `/*! */` block comments.
///
/// Content lines are prefixed with a space for standard formatting:
/// `//! Content here` rather than `//!Content here`.
///
/// Preserves indentation within lines (important for code blocks).
pub(crate) fn lines_to_doc_comments(doc: &str) -> TokenStream {
    let lines: Vec<TokenStream> = doc
        .lines()
        .map(|line| {
            // Use #![doc = ...] for each line - prettyplease formats single-line
            // doc attributes as //! style comments.
            // Add leading space for non-empty lines to match standard formatting,
            // but preserve internal indentation (e.g., inside code blocks).
            let doc_content = if line.trim().is_empty() {
                String::new()
            } else {
                format!(" {}", line)
            };
            quote! { #![doc = #doc_content] }
        })
        .collect();

    quote! { #(#lines)* }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiResponse, Endpoint};

    fn make_test_api() -> RestApi {
        {
            let mut api = schematic_define::RestApi::builder(&schematic_define::DefinitionContext::types_in("fixture"), "TestApi".to_string(), "https://api.test.com".to_string())
                .describes("A test API for documentation.".to_string())
                .authentication(AuthStrategy::BearerToken { header: None }, vec!["TEST_API_KEY".to_string()])
                .build().expect("valid API fixture");
            api.docs_url = Some("https://docs.test.com".to_string());
            api.endpoints = vec![{
                let mut endpoint = Endpoint::new("ListItems".to_string(), RestMethod::Get, "/items".to_string())
                    .describes("List all items".to_string())
                    .returns_empty()
                    .build().expect("valid endpoint fixture");
                endpoint.response = ApiResponse::json_type("ListItemsResponse");
                endpoint
            }];
            api
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
            value_prefix: None,
        };
        let builder = ModuleDocBuilder::new(&api);
        let auth = builder.auth_section();
        assert!(auth.contains("API key"));
        assert!(auth.contains("X-API-Key"));
    }

    #[test]
    fn auth_section_basic() {
        let mut api = make_test_api();
        api.auth = crate::test_utils::basic_auth(&["USER"], &["PASS"]);
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
        api.endpoints.push({
            let mut endpoint = Endpoint::new("CreateItem".to_string(), RestMethod::Post, "/items".to_string())
                .describes("Create a new item".to_string())
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("CreateItemResponse");
            endpoint
        });
        api.endpoints.push({
            let mut endpoint = Endpoint::new("GetItem".to_string(), RestMethod::Get, "/items/{id}".to_string())
                .describes("Get a specific item".to_string())
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("Item");
            endpoint
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
            {
                let mut endpoint = Endpoint::new("CreateItem".to_string(), RestMethod::Post, "/items".to_string())
                    .describes("Create a new item".to_string())
                    .returns_empty()
                    .build().expect("valid endpoint fixture");
                endpoint.response = ApiResponse::json_type("CreateItemResponse");
                endpoint
            },
        );

        let builder = ModuleDocBuilder::new(&api);
        let example = builder.example_section();

        // Should use ListItems (GET) not CreateItem (POST)
        assert!(example.contains("let data = client.list_items().await?;"), "{example}");
        assert!(
            example.contains("let response = client.list_items().full().await?;"),
            "{example}"
        );
        assert!(example.contains("let client = TestApi::new();"), "{example}");
        assert!(!example.contains("create_item"), "{example}");
        assert!(!example.contains("CallOptions"), "{example}");
    }

    #[test]
    fn example_section_fallback_to_first_endpoint() {
        let mut api = make_test_api();
        api.endpoints = vec![{
            let mut endpoint = Endpoint::new("CreateItem".to_string(), RestMethod::Post, "/items".to_string())
                .describes("Create a new item".to_string())
                .returns_empty()
                .build().expect("valid endpoint fixture");
            endpoint.response = ApiResponse::json_type("CreateItemResponse");
            endpoint
        }];

        let builder = ModuleDocBuilder::new(&api);
        let example = builder.example_section();

        assert!(example.contains("let data = client.create_item().await?;"), "{example}");
        assert!(
            example.contains(r#"TestApi::new().base_url("https://staging.example.com/v1");"#),
            "{example}"
        );
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

    #[test]
    fn build_produces_line_doc_attributes() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        let code = tokens.to_string();

        // quote! renders #![doc = ...] with spaces: # ! [doc = ...]
        // These become //! style when formatted by prettyplease
        assert!(
            code.contains("# ! [doc ="),
            "Expected #![doc = ...] attributes, got: {}",
            &code[..code.len().min(200)]
        );

        // Verify we have multiple separate doc attributes (one per line)
        // rather than one big block that would become /*! */
        let doc_count = code.matches("# ! [doc =").count();
        assert!(
            doc_count > 5,
            "Expected multiple doc attributes (one per line), found {}",
            doc_count
        );
    }

    #[test]
    fn build_preserves_code_block_indentation() {
        let api = make_test_api();
        let builder = ModuleDocBuilder::new(&api);
        let tokens = builder.build();
        let code = tokens.to_string();

        // The code block should have indented lines preserved
        assert!(
            code.contains("    let client"),
            "Expected indented 'let client' in code block"
        );
    }

    /// The module doc text, one entry per `//!` line.
    fn doc_text(api: &RestApi) -> String {
        let file: syn::File = syn::parse2(ModuleDocBuilder::new(api).build()).expect("module docs parse");
        file.attrs
            .iter()
            .filter_map(|attr| match &attr.meta {
                syn::Meta::NameValue(doc) => match &doc.value {
                    syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(text), .. }) => Some(text.value()),
                    _ => None,
                },
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn api_rate_limit_and_retry_guidance_are_documented_before_the_example() {
        use schematic_define::metadata::{Backoff, RateLimit, ResetUnit, RetryGuidance, StatusSelector};

        let mut api = make_test_api();
        api.rate_limit = Some(
            RateLimit::new("x-rate-limit", "x-rate-remaining", "x-rate-reset", ResetUnit::Seconds).unwrap(),
        );
        api.retry_guidance = Some(
            RetryGuidance::new([StatusSelector::Code(429)], false, 4, Backoff::Fixed { delay_ms: 100 }).unwrap(),
        );

        let docs = doc_text(&api);

        let expected = " ## Rate limit

 Response headers reporting the rate limit (documentation only; the client does not act on them):

 - limit: `x-rate-limit`
 - remaining: `x-rate-remaining`
 - reset: `x-rate-reset`, as seconds until the window resets

 ## Retry guidance

 The provider's retry guidance (documentation only; the client does not retry):

 - retry on: `429`
 - honor Retry-After: no
 - max attempts: 4, including the first
 - backoff: fixed, 100 ms

 ## Examples";
        assert!(docs.contains(expected), "{docs}");
    }

    #[test]
    fn an_api_without_metadata_has_no_metadata_sections() {
        let docs = doc_text(&make_test_api());

        assert!(!docs.contains("## Rate limit"), "{docs}");
        assert!(!docs.contains("## Retry guidance"), "{docs}");
        assert!(docs.contains("- `ListItems` - List all items\n\n ## Examples"), "{docs}");
    }
}
