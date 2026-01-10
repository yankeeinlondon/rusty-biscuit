//! OpenAPI 3.x specification generator.
//!
//! This module provides [`OpenApiGenerator`] for generating OpenAPI 3.1.0 specifications
//! from REST API endpoint definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::auth::ApiAuthMethod;
use crate::method::RestMethod;

/// Output format for the generated OpenAPI specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// JSON output format.
    #[default]
    Json,
    /// YAML output format.
    Yaml,
}

/// Metadata about the API for the OpenAPI `info` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    /// The title of the API.
    pub title: String,
    /// The version of the API (e.g., "1.0.0").
    pub version: String,
    /// A description of the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A URL to the Terms of Service for the API.
    #[serde(rename = "termsOfService", skip_serializing_if = "Option::is_none")]
    pub terms_of_service: Option<String>,
    /// Contact information for the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    /// License information for the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

impl OpenApiInfo {
    /// Creates a new `OpenApiInfo` with the required fields.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the terms of service URL.
    pub fn with_terms_of_service(mut self, url: impl Into<String>) -> Self {
        self.terms_of_service = Some(url.into());
        self
    }

    /// Sets the contact information.
    pub fn with_contact(mut self, contact: Contact) -> Self {
        self.contact = Some(contact);
        self
    }

    /// Sets the license information.
    pub fn with_license(mut self, license: License) -> Self {
        self.license = Some(license);
        self
    }
}

/// Contact information for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// The name of the contact person/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// A URL pointing to the contact information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The email address of the contact person/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// License information for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// The license name.
    pub name: String,
    /// A URL to the license.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A server object for the OpenAPI `servers` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// The URL to the target host.
    pub url: String,
    /// A description of the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Server {
    /// Creates a new server with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
        }
    }

    /// Sets the server description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Specification of an endpoint for OpenAPI generation.
///
/// This is a type-erased representation of an endpoint that can be used
/// for OpenAPI spec generation without needing the response format type.
#[derive(Debug, Clone)]
pub struct EndpointSpec {
    /// Unique identifier for the endpoint (used as `operationId`).
    pub id: String,
    /// HTTP method for the endpoint.
    pub method: RestMethod,
    /// URL path template (may contain `{param}` placeholders).
    pub path: String,
    /// Summary/description of the endpoint.
    pub summary: Option<String>,
    /// Detailed description of the endpoint.
    pub description: Option<String>,
    /// Tags for grouping endpoints.
    pub tags: Vec<String>,
    /// Whether the endpoint is deprecated.
    pub deprecated: bool,
}

impl EndpointSpec {
    /// Creates a new endpoint specification.
    pub fn new(
        id: impl Into<String>,
        method: RestMethod,
        path: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            method,
            path: path.into(),
            summary: None,
            description: None,
            tags: Vec::new(),
            deprecated: false,
        }
    }

    /// Sets the summary.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Sets tags (replacing any existing).
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Marks the endpoint as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Extracts path parameter names from the path template.
    fn path_params(&self) -> Vec<&str> {
        let mut params = Vec::new();
        let mut chars = self.path.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut param = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    param.push(chars.next().unwrap());
                }
                if !param.is_empty() {
                    // Find the param in the original string to return a reference
                    if let Some(start) = self.path.find(&format!("{{{param}}}")) {
                        let param_start = start + 1;
                        let param_end = param_start + param.len();
                        params.push(&self.path[param_start..param_end]);
                    }
                }
            }
        }

        params
    }
}

/// Security scheme for OpenAPI specification.
#[derive(Debug, Clone)]
pub struct SecurityScheme {
    /// The name of the security scheme (used as key in `securitySchemes`).
    pub name: String,
    /// The underlying auth method.
    pub auth_method: ApiAuthMethod,
    /// Description of the security scheme.
    pub description: Option<String>,
}

impl SecurityScheme {
    /// Creates a new security scheme from an auth method.
    pub fn new(name: impl Into<String>, auth_method: ApiAuthMethod) -> Self {
        Self {
            name: name.into(),
            auth_method,
            description: None,
        }
    }

    /// Sets the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Generator for OpenAPI 3.1.0 specifications.
///
/// ## Examples
///
/// ```rust
/// use api::openapi::{OpenApiGenerator, OpenApiInfo, Server, EndpointSpec, OutputFormat};
/// use api::RestMethod;
///
/// let spec = OpenApiGenerator::new(OpenApiInfo::new("My API", "1.0.0"))
///     .add_server(Server::new("https://api.example.com"))
///     .add_endpoint(EndpointSpec::new("get_users", RestMethod::Get, "/users"))
///     .generate(OutputFormat::Json)
///     .unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct OpenApiGenerator {
    info: OpenApiInfo,
    servers: Vec<Server>,
    endpoints: Vec<EndpointSpec>,
    security_schemes: Vec<SecurityScheme>,
    /// Global security requirements (applied to all operations).
    global_security: Vec<String>,
}

impl OpenApiGenerator {
    /// Creates a new OpenAPI generator with the given info.
    pub fn new(info: OpenApiInfo) -> Self {
        Self {
            info,
            servers: Vec::new(),
            endpoints: Vec::new(),
            security_schemes: Vec::new(),
            global_security: Vec::new(),
        }
    }

    /// Adds a server to the specification.
    pub fn add_server(mut self, server: Server) -> Self {
        self.servers.push(server);
        self
    }

    /// Adds multiple servers to the specification.
    pub fn add_servers(mut self, servers: impl IntoIterator<Item = Server>) -> Self {
        self.servers.extend(servers);
        self
    }

    /// Adds an endpoint to the specification.
    pub fn add_endpoint(mut self, endpoint: EndpointSpec) -> Self {
        self.endpoints.push(endpoint);
        self
    }

    /// Adds multiple endpoints to the specification.
    pub fn add_endpoints(mut self, endpoints: impl IntoIterator<Item = EndpointSpec>) -> Self {
        self.endpoints.extend(endpoints);
        self
    }

    /// Adds a security scheme to the specification.
    pub fn add_security_scheme(mut self, scheme: SecurityScheme) -> Self {
        self.security_schemes.push(scheme);
        self
    }

    /// Adds multiple security schemes to the specification.
    pub fn add_security_schemes(
        mut self,
        schemes: impl IntoIterator<Item = SecurityScheme>,
    ) -> Self {
        self.security_schemes.extend(schemes);
        self
    }

    /// Sets global security requirements (by scheme name).
    ///
    /// These security schemes will be applied to all operations unless
    /// overridden at the operation level.
    pub fn with_global_security(
        mut self,
        scheme_names: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.global_security = scheme_names.into_iter().map(Into::into).collect();
        self
    }

    /// Generates the OpenAPI specification as a string.
    ///
    /// ## Errors
    ///
    /// Returns an error if serialization fails.
    pub fn generate(&self, format: OutputFormat) -> Result<String, GenerateError> {
        let spec = self.build_spec();

        match format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(&spec).map_err(GenerateError::JsonSerialize)
            }
            OutputFormat::Yaml => {
                serde_yaml::to_string(&spec).map_err(GenerateError::YamlSerialize)
            }
        }
    }

    /// Generates the OpenAPI specification as a `serde_json::Value`.
    ///
    /// Useful when you need to manipulate the spec before serialization.
    pub fn generate_value(&self) -> serde_json::Value {
        self.build_spec()
    }

    /// Builds the OpenAPI spec as a JSON value.
    fn build_spec(&self) -> serde_json::Value {
        let mut spec = serde_json::json!({
            "openapi": "3.1.0",
            "info": self.info,
        });

        // Add servers if present
        if !self.servers.is_empty() {
            spec["servers"] = serde_json::to_value(&self.servers).unwrap();
        }

        // Build paths
        let paths = self.build_paths();
        if !paths.is_empty() {
            spec["paths"] = serde_json::to_value(&paths).unwrap();
        }

        // Build components (security schemes)
        let components = self.build_components();
        if let serde_json::Value::Object(obj) = &components {
            if !obj.is_empty() {
                spec["components"] = components;
            }
        }

        // Add global security if present
        if !self.global_security.is_empty() {
            let security: Vec<serde_json::Value> = self
                .global_security
                .iter()
                .map(|name| serde_json::json!({ name: [] }))
                .collect();
            spec["security"] = serde_json::Value::Array(security);
        }

        spec
    }

    /// Builds the paths object.
    fn build_paths(&self) -> BTreeMap<String, serde_json::Value> {
        let mut paths: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        for endpoint in &self.endpoints {
            let method_str = endpoint.method.to_string().to_lowercase();

            let mut operation = serde_json::json!({
                "operationId": endpoint.id,
                "responses": {
                    "200": {
                        "description": "Successful response"
                    }
                }
            });

            // Add summary if present
            if let Some(ref summary) = endpoint.summary {
                operation["summary"] = serde_json::Value::String(summary.clone());
            }

            // Add description if present
            if let Some(ref description) = endpoint.description {
                operation["description"] = serde_json::Value::String(description.clone());
            }

            // Add tags if present
            if !endpoint.tags.is_empty() {
                operation["tags"] = serde_json::to_value(&endpoint.tags).unwrap();
            }

            // Add deprecated flag if true
            if endpoint.deprecated {
                operation["deprecated"] = serde_json::Value::Bool(true);
            }

            // Add path parameters
            let path_params = endpoint.path_params();
            if !path_params.is_empty() {
                let parameters: Vec<serde_json::Value> = path_params
                    .iter()
                    .map(|&param| {
                        serde_json::json!({
                            "name": param,
                            "in": "path",
                            "required": true,
                            "schema": {
                                "type": "string"
                            }
                        })
                    })
                    .collect();
                operation["parameters"] = serde_json::Value::Array(parameters);
            }

            // Insert or merge with existing path entry
            paths
                .entry(endpoint.path.clone())
                .and_modify(|existing| {
                    if let serde_json::Value::Object(obj) = existing {
                        obj.insert(method_str.clone(), operation.clone());
                    }
                })
                .or_insert_with(|| serde_json::json!({ &method_str: operation }));
        }

        paths
    }

    /// Builds the components object.
    fn build_components(&self) -> serde_json::Value {
        if self.security_schemes.is_empty() {
            return serde_json::json!({});
        }

        let mut security_schemes = serde_json::Map::new();

        for scheme in &self.security_schemes {
            let scheme_value = match &scheme.auth_method {
                ApiAuthMethod::BearerToken => {
                    let mut obj = serde_json::json!({
                        "type": "http",
                        "scheme": "bearer"
                    });
                    if let Some(ref desc) = scheme.description {
                        obj["description"] = serde_json::Value::String(desc.clone());
                    }
                    obj
                }
                ApiAuthMethod::ApiKey(header_name) => {
                    let mut obj = serde_json::json!({
                        "type": "apiKey",
                        "in": "header",
                        "name": header_name
                    });
                    if let Some(ref desc) = scheme.description {
                        obj["description"] = serde_json::Value::String(desc.clone());
                    }
                    obj
                }
                ApiAuthMethod::QueryParam(param_name) => {
                    let mut obj = serde_json::json!({
                        "type": "apiKey",
                        "in": "query",
                        "name": param_name
                    });
                    if let Some(ref desc) = scheme.description {
                        obj["description"] = serde_json::Value::String(desc.clone());
                    }
                    obj
                }
                ApiAuthMethod::None => {
                    // Skip None auth methods
                    continue;
                }
            };

            security_schemes.insert(scheme.name.clone(), scheme_value);
        }

        if security_schemes.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::json!({
                "securitySchemes": security_schemes
            })
        }
    }
}

/// Errors that can occur during OpenAPI generation.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    /// JSON serialization failed.
    #[error("failed to serialize OpenAPI spec to JSON: {0}")]
    JsonSerialize(#[source] serde_json::Error),

    /// YAML serialization failed.
    #[error("failed to serialize OpenAPI spec to YAML: {0}")]
    YamlSerialize(#[source] serde_yaml::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_spec_generation() {
        let info = OpenApiInfo::new("Test API", "1.0.0");
        let generator = OpenApiGenerator::new(info)
            .add_server(Server::new("https://api.example.com"));

        let spec_json = generator.generate(OutputFormat::Json).unwrap();
        let spec: serde_json::Value = serde_json::from_str(&spec_json).unwrap();

        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["info"]["title"], "Test API");
        assert_eq!(spec["info"]["version"], "1.0.0");
        assert_eq!(spec["servers"][0]["url"], "https://api.example.com");
    }

    #[test]
    fn test_spec_with_endpoints() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoint(
                EndpointSpec::new("get_users", RestMethod::Get, "/users")
                    .with_summary("Get all users"),
            )
            .add_endpoint(
                EndpointSpec::new("get_user", RestMethod::Get, "/users/{id}")
                    .with_summary("Get user by ID"),
            );

        let spec = generator.generate_value();

        // Verify endpoints are present
        assert!(spec["paths"]["/users"]["get"].is_object());
        assert!(spec["paths"]["/users/{id}"]["get"].is_object());

        // Verify operation IDs
        assert_eq!(spec["paths"]["/users"]["get"]["operationId"], "get_users");
        assert_eq!(spec["paths"]["/users/{id}"]["get"]["operationId"], "get_user");

        // Verify path parameters
        let params = &spec["paths"]["/users/{id}"]["get"]["parameters"];
        assert!(params.is_array());
        assert_eq!(params[0]["name"], "id");
        assert_eq!(params[0]["in"], "path");
        assert_eq!(params[0]["required"], true);
    }

    #[test]
    fn test_multiple_methods_same_path() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoint(EndpointSpec::new("get_users", RestMethod::Get, "/users"))
            .add_endpoint(EndpointSpec::new("create_user", RestMethod::Post, "/users"));

        let spec = generator.generate_value();

        // Both methods should be under the same path
        assert!(spec["paths"]["/users"]["get"].is_object());
        assert!(spec["paths"]["/users"]["post"].is_object());
    }

    #[test]
    fn test_bearer_auth_scheme() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_security_scheme(
                SecurityScheme::new("bearerAuth", ApiAuthMethod::BearerToken)
                    .with_description("JWT Bearer token"),
            );

        let spec = generator.generate_value();

        let scheme = &spec["components"]["securitySchemes"]["bearerAuth"];
        assert_eq!(scheme["type"], "http");
        assert_eq!(scheme["scheme"], "bearer");
        assert_eq!(scheme["description"], "JWT Bearer token");
    }

    #[test]
    fn test_api_key_auth_scheme() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_security_scheme(SecurityScheme::new(
                "apiKey",
                ApiAuthMethod::ApiKey("X-API-Key".to_string()),
            ));

        let spec = generator.generate_value();

        let scheme = &spec["components"]["securitySchemes"]["apiKey"];
        assert_eq!(scheme["type"], "apiKey");
        assert_eq!(scheme["in"], "header");
        assert_eq!(scheme["name"], "X-API-Key");
    }

    #[test]
    fn test_query_param_auth_scheme() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_security_scheme(SecurityScheme::new(
                "queryAuth",
                ApiAuthMethod::QueryParam("key".to_string()),
            ));

        let spec = generator.generate_value();

        let scheme = &spec["components"]["securitySchemes"]["queryAuth"];
        assert_eq!(scheme["type"], "apiKey");
        assert_eq!(scheme["in"], "query");
        assert_eq!(scheme["name"], "key");
    }

    #[test]
    fn test_global_security() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_security_scheme(SecurityScheme::new("bearerAuth", ApiAuthMethod::BearerToken))
            .with_global_security(["bearerAuth"]);

        let spec = generator.generate_value();

        assert!(spec["security"].is_array());
        assert!(spec["security"][0]["bearerAuth"].is_array());
    }

    #[test]
    fn test_yaml_output() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_server(Server::new("https://api.example.com"));

        let yaml = generator.generate(OutputFormat::Yaml).unwrap();

        assert!(yaml.contains("openapi: 3.1.0"));
        assert!(yaml.contains("title: Test API"));
        assert!(yaml.contains("version: 1.0.0"));
    }

    #[test]
    fn test_endpoint_tags() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoint(
                EndpointSpec::new("get_users", RestMethod::Get, "/users")
                    .with_tags(["users", "admin"]),
            );

        let spec = generator.generate_value();
        let tags = &spec["paths"]["/users"]["get"]["tags"];

        assert!(tags.is_array());
        assert_eq!(tags[0], "users");
        assert_eq!(tags[1], "admin");
    }

    #[test]
    fn test_deprecated_endpoint() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoint(
                EndpointSpec::new("old_endpoint", RestMethod::Get, "/old")
                    .deprecated(),
            );

        let spec = generator.generate_value();
        assert_eq!(spec["paths"]["/old"]["get"]["deprecated"], true);
    }

    #[test]
    fn test_info_with_all_fields() {
        let info = OpenApiInfo::new("Full API", "2.0.0")
            .with_description("A comprehensive API")
            .with_terms_of_service("https://example.com/tos")
            .with_contact(Contact {
                name: Some("API Support".to_string()),
                url: Some("https://example.com/support".to_string()),
                email: Some("support@example.com".to_string()),
            })
            .with_license(License {
                name: "MIT".to_string(),
                url: Some("https://opensource.org/licenses/MIT".to_string()),
            });

        let generator = OpenApiGenerator::new(info);
        let spec = generator.generate_value();

        assert_eq!(spec["info"]["description"], "A comprehensive API");
        assert_eq!(spec["info"]["termsOfService"], "https://example.com/tos");
        assert_eq!(spec["info"]["contact"]["name"], "API Support");
        assert_eq!(spec["info"]["license"]["name"], "MIT");
    }

    #[test]
    fn test_multiple_path_params() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoint(EndpointSpec::new(
                "get_user_post",
                RestMethod::Get,
                "/users/{user_id}/posts/{post_id}",
            ));

        let spec = generator.generate_value();
        let params = &spec["paths"]["/users/{user_id}/posts/{post_id}"]["get"]["parameters"];

        assert!(params.is_array());
        assert_eq!(params.as_array().unwrap().len(), 2);
        assert_eq!(params[0]["name"], "user_id");
        assert_eq!(params[1]["name"], "post_id");
    }

    #[test]
    fn test_spec_is_valid_json() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_server(Server::new("https://api.example.com"))
            .add_endpoint(EndpointSpec::new("get_users", RestMethod::Get, "/users"))
            .add_security_scheme(SecurityScheme::new("bearerAuth", ApiAuthMethod::BearerToken));

        let json_str = generator.generate(OutputFormat::Json).unwrap();

        // Should be valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_spec_is_valid_yaml() {
        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_server(Server::new("https://api.example.com"))
            .add_endpoint(EndpointSpec::new("get_users", RestMethod::Get, "/users"));

        let yaml_str = generator.generate(OutputFormat::Yaml).unwrap();

        // Should be valid YAML
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(&yaml_str);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_all_endpoints_represented() {
        let endpoints = vec![
            EndpointSpec::new("list_users", RestMethod::Get, "/users"),
            EndpointSpec::new("create_user", RestMethod::Post, "/users"),
            EndpointSpec::new("get_user", RestMethod::Get, "/users/{id}"),
            EndpointSpec::new("update_user", RestMethod::Put, "/users/{id}"),
            EndpointSpec::new("delete_user", RestMethod::Delete, "/users/{id}"),
        ];

        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_endpoints(endpoints);

        let spec = generator.generate_value();

        // Verify all endpoints are present
        assert!(spec["paths"]["/users"]["get"].is_object());
        assert!(spec["paths"]["/users"]["post"].is_object());
        assert!(spec["paths"]["/users/{id}"]["get"].is_object());
        assert!(spec["paths"]["/users/{id}"]["put"].is_object());
        assert!(spec["paths"]["/users/{id}"]["delete"].is_object());

        // Verify operation IDs
        assert_eq!(spec["paths"]["/users"]["get"]["operationId"], "list_users");
        assert_eq!(spec["paths"]["/users"]["post"]["operationId"], "create_user");
        assert_eq!(spec["paths"]["/users/{id}"]["get"]["operationId"], "get_user");
        assert_eq!(spec["paths"]["/users/{id}"]["put"]["operationId"], "update_user");
        assert_eq!(spec["paths"]["/users/{id}"]["delete"]["operationId"], "delete_user");
    }

    #[test]
    fn test_all_security_schemes_documented() {
        let schemes = vec![
            SecurityScheme::new("bearerAuth", ApiAuthMethod::BearerToken),
            SecurityScheme::new("apiKeyHeader", ApiAuthMethod::ApiKey("X-API-Key".to_string())),
            SecurityScheme::new("apiKeyQuery", ApiAuthMethod::QueryParam("api_key".to_string())),
            SecurityScheme::new("none", ApiAuthMethod::None), // Should be skipped
        ];

        let generator = OpenApiGenerator::new(OpenApiInfo::new("Test API", "1.0.0"))
            .add_security_schemes(schemes);

        let spec = generator.generate_value();
        let security_schemes = &spec["components"]["securitySchemes"];

        // Should have 3 schemes (None is skipped)
        assert!(security_schemes["bearerAuth"].is_object());
        assert!(security_schemes["apiKeyHeader"].is_object());
        assert!(security_schemes["apiKeyQuery"].is_object());
        assert!(security_schemes["none"].is_null()); // None auth should be skipped
    }
}
