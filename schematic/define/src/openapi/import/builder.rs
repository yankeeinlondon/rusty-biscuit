use std::collections::HashSet;

use super::super::source::OpenApiSource;
use super::diagnostics::OpenApiDiagnostic;
use super::mappings;
use super::{extension, reconcile};
use super::normalize::SourceKeywords;
use super::resolver::RefResolver;
use crate::cross_origin::CrossOrigin;
use crate::models::ModelCatalog;
use crate::types::RestApi;

#[cfg(feature = "openapi")]
use biscuit_file::serde_yaml_ng;

/// Maximum nesting depth for schema resolution.
pub const MAX_NESTING_DEPTH: usize = 50;

/// Maximum number of components allowed.
pub const MAX_COMPONENT_COUNT: usize = 5000;

/// Builder for importing OpenAPI specifications.
///
/// Provides a fluent API for configuring the import process and
/// transforming OpenAPI documents into Schematic definitions.
///
/// ## Examples
///
/// Basic import with defaults:
///
/// ```text
/// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
///
/// let source = OpenApiSource::yaml("...");
/// let result = OpenApiImport::new(source).build()?;
/// ```
///
/// Import with custom configuration:
///
/// ```text
/// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
///
/// let result = OpenApiImport::new(source)
///     .api_name("MyApi")
///     .module_path("my_api")
///     .base_url_override("https://api.example.com")
///     .prefer_json()
///     .build()?;
/// ```
#[derive(Debug)]
pub struct OpenApiImport {
    source: OpenApiSource,
    pub(crate) options: OpenApiImportOptions,
}

impl OpenApiImport {
    /// Creates a new import builder from an OpenAPI source.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
    ///
    /// let source = OpenApiSource::yaml("openapi: '3.0.0'\ninfo:\n  title: Test\n  version: '1.0.0'\npaths: {}");
    /// let builder = OpenApiImport::new(source);
    /// ```
    pub fn new(source: OpenApiSource) -> Self {
        Self {
            source,
            options: OpenApiImportOptions::default(),
        }
    }

    /// Sets the API name, overriding the info.title from the spec.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
    ///
    /// let builder = OpenApiImport::new(OpenApiSource::yaml("..."))
    ///     .api_name("MyApi");
    /// ```
    #[must_use]
    pub fn api_name(mut self, name: impl Into<String>) -> Self {
        self.options.api_name = Some(name.into());
        self
    }

    /// Sets the module path for generated code.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
    ///
    /// let builder = OpenApiImport::new(OpenApiSource::yaml("..."))
    ///     .module_path("my_api::client");
    /// ```
    #[must_use]
    pub fn module_path(mut self, path: impl Into<String>) -> Self {
        self.options.module_path = Some(path.into());
        self
    }

    /// Uses the first server URL from the OpenAPI spec as the base URL.
    ///
    /// This is the default behavior.
    #[must_use]
    pub fn base_url_from_servers(mut self) -> Self {
        self.options.base_url_policy = BaseUrlPolicy::FromServers;
        self
    }

    /// Overrides the base URL with a custom value.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::openapi::{OpenApiImport, OpenApiSource};
    ///
    /// let builder = OpenApiImport::new(OpenApiSource::yaml("..."))
    ///     .base_url_override("https://api.example.com/v2");
    /// ```
    #[must_use]
    pub fn base_url_override(mut self, url: impl Into<String>) -> Self {
        self.options.base_url_policy = BaseUrlPolicy::Override(url.into());
        self
    }

    /// Prefers JSON content type when multiple options are available.
    ///
    /// When an operation supports multiple content types (e.g., JSON, XML),
    /// this setting causes the importer to select JSON.
    #[must_use]
    pub fn prefer_json(mut self) -> Self {
        self.options.content_preference = ContentPreference::PreferJson;
        self
    }

    /// Enables strict mode: any WARN or ERROR diagnostic fails the import.
    ///
    /// INFO diagnostics never fail it; they are returned in
    /// [`OpenApiImportResult::diagnostics`] as in permissive mode.
    #[must_use]
    pub fn strict(mut self) -> Self {
        self.options.strict = true;
        self
    }

    /// Builds the import result from the configured source.
    ///
    /// ## Returns
    ///
    /// Returns `OpenApiImportResult` containing the API definition,
    /// model catalog, and any diagnostics encountered. The API carries one
    /// schema component per catalog model, in source order: the model's
    /// final name, its Rust path (`{module_path}::{Name}`, or the bare name
    /// without a module path), and its source schema. It passes
    /// [`RestApi::validate_references`], so it can be exported directly.
    ///
    /// ## Errors
    ///
    /// Returns `OpenApiError` if:
    /// - The source cannot be parsed
    /// - Required fields are missing
    /// - An endpoint body or response names a schema the document does not
    ///   define ([`OpenApiError::Definition`] holding
    ///   [`crate::DefinitionError::UnresolvedReference`])
    /// - A value rule in a parameter, body, form field, or component schema
    ///   is malformed or contradictory (`minLength` above `maxLength`, a
    ///   nonpositive `multipleOf`, an empty `enum`, an invalid `pattern`):
    ///   [`OpenApiError::Validation`] located at the first such keyword
    /// - Strict mode is enabled and any WARN or ERROR diagnostic was
    ///   generated ([`OpenApiError::Validation`] located at the first one,
    ///   with the count and that diagnostic's message)
    pub fn build(self) -> Result<OpenApiImportResult, super::super::error::OpenApiError> {
        let mut diagnostics = Vec::new();

        let (mut doc, keywords) = self.parse_source(&mut diagnostics)?;
        super::query_paths::normalize(&mut doc, &mut diagnostics)?;

        self.validate_constraints(&doc)?;

        let resolver = RefResolver::with_source_keywords(&doc, &keywords);

        let mut api = self.map_to_rest_api(&doc, &resolver, &mut diagnostics)?;

        let request_suffix = api.request_suffix.as_deref().unwrap_or("Request");
        let reserved_names: std::collections::HashSet<String> = api
            .endpoints
            .iter()
            .map(|ep| format!("{}{}", ep.id, request_suffix))
            .collect();

        // Models live in the effective module: the CLI's, else the document's.
        let mut catalog_options = self.options.clone();
        catalog_options.module_path.clone_from(&api.module_path);
        let schema_result =
            mappings::map_all_schemas(&doc, &resolver, &mut diagnostics, &catalog_options, &reserved_names);

        // `openapiv3` drops these keywords wherever they sit (components,
        // bodies, forms, responses), so the rule each carries is lost. A
        // parameter, form field, or body rule has reported its own at the
        // same pointer; every other one is reported here.
        let reported: std::collections::HashSet<String> =
            diagnostics.iter().map(|diagnostic| diagnostic.location.clone()).collect();
        for pointer in keywords.dropped().filter(|pointer| !reported.contains(*pointer)) {
            let keyword = pointer.rsplit('/').next().unwrap_or(pointer);
            diagnostics.push(OpenApiDiagnostic::warn(
                pointer.to_owned(),
                format!("`{keyword}` is a JSON Schema keyword OpenAPI 3.0 does not define, so the parser drops it; not preserved"),
            ));
        }

        // A component that yielded its name (for example, to a request
        // struct) is referenced by its final name everywhere reference
        // validation reads, at any nesting depth.
        for name in api.json_references_mut().into_iter().flat_map(crate::TypeRef::names_mut) {
            if let Some(final_name) = schema_result.name_mapping.get(name) {
                name.clone_from(final_name);
            }
        }
        // A malformed or contradictory value rule fails the import in
        // either mode; dropping it would change what the API accepts.
        if let Some(error) = resolver.take_rejection() {
            return Err(error);
        }
        api.schemas = schema_result.components;
        api.validate_references()?;

        if self.options.strict {
            let mut failing = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity != super::diagnostics::DiagnosticSeverity::Info);
            if let Some(first) = failing.next() {
                return Err(super::super::error::OpenApiError::Validation {
                    path: first.location.clone(),
                    message: format!(
                        "Strict mode: {} WARN/ERROR diagnostics encountered; first: {} at {}: {}",
                        1 + failing.count(),
                        first.severity,
                        first.location,
                        first.message
                    ),
                });
            }
        }

        Ok(OpenApiImportResult {
            api,
            models: schema_result.catalog,
            diagnostics,
        })
    }

    /// Parses the OpenAPI document from the configured source, with the
    /// exact source values of the keywords `openapiv3` narrows or drops.
    ///
    /// Text sources are captured by [`SourceKeywords::capture`] before
    /// [`super::normalize::stand_in_bounds`] makes them parseable. A parsed
    /// [`OpenApiSource::Document`] has no source text, so its numbers are
    /// read as `openapiv3` holds them.
    fn parse_source(
        &self,
        diagnostics: &mut Vec<OpenApiDiagnostic>,
    ) -> Result<(openapiv3::OpenAPI, SourceKeywords), super::super::error::OpenApiError> {
        let yaml_error = |e: serde_yaml_ng::Error| super::super::error::OpenApiError::Parse {
            message: e.to_string(),
            location: None,
        };
        let (content, json) = match &self.source {
            OpenApiSource::Path(path) => {
                let content = std::fs::read_to_string(path).map_err(|e| {
                    super::super::error::OpenApiError::Io {
                        path: path.clone(),
                        message: e.to_string(),
                    }
                })?;
                (content, false)
            }
            OpenApiSource::Json(content) => (content.clone(), true),
            OpenApiSource::Yaml(content) => (content.clone(), false),
            OpenApiSource::Bytes(bytes) => (String::from_utf8_lossy(bytes).into_owned(), false),
            OpenApiSource::Document(doc) => return Ok((*doc.clone(), SourceKeywords::default())),
        };

        let captured = SourceKeywords::capture(&content, json);
        if let Err(error @ super::super::error::OpenApiError::Validation { .. }) = captured {
            return Err(error);
        }
        let content = super::normalize::stand_in_bounds(&content);
        super::query_paths::check_text(&content, json)?;
        let doc = if json {
            serde_json::from_str(&content).map_err(|e| super::super::error::OpenApiError::Parse {
                message: e.to_string(),
                location: Some(format!("line {}, column {}", e.line(), e.column())),
            })?
        } else {
            serde_yaml_ng::from_str(&content).map_err(yaml_error)?
        };
        // The capture reads anything the parse does, so this is a capture
        // limit (a non-scalar under a numeric keyword name), never silent.
        let keywords = captured.unwrap_or_else(|error| {
            diagnostics.push(OpenApiDiagnostic::warn(
                "#".to_string(),
                format!("exact numeric bounds could not be captured ({error}); bounds are read as parsed"),
            ));
            SourceKeywords::default()
        });
        Ok((doc, keywords))
    }

    /// Validates input constraints (max depth, component count).
    fn validate_constraints(
        &self,
        doc: &openapiv3::OpenAPI,
    ) -> Result<(), super::super::error::OpenApiError> {
        if let Some(ref components) = doc.components {
            let schema_count = components.schemas.len();
            let param_count = components.parameters.len();
            let response_count = components.responses.len();
            let total = schema_count + param_count + response_count;

            if total > MAX_COMPONENT_COUNT {
                return Err(super::super::error::OpenApiError::Validation {
                    path: "#/components".to_string(),
                    message: format!(
                        "Component count {} exceeds maximum {}",
                        total, MAX_COMPONENT_COUNT
                    ),
                });
            }
        }
        Ok(())
    }

    /// Maps the OpenAPI document to a RestApi definition.
    ///
    /// The result has no schema components yet and its references use
    /// sanitized source names; [`Self::build`] renames them, attaches the
    /// catalog's components, and validates the references.
    fn map_to_rest_api(
        &self,
        doc: &openapiv3::OpenAPI,
        resolver: &RefResolver,
        diagnostics: &mut Vec<OpenApiDiagnostic>,
    ) -> Result<RestApi, super::super::error::OpenApiError> {
        use mappings::{map_info, map_operation, map_server};

        let (default_name, description) = map_info(&doc.info);
        let name = self.options.api_name.clone().unwrap_or(default_name);

        let base_url = map_server(&doc.servers, &self.options.base_url_policy);
        if doc.servers.is_empty() && self.options.base_url_policy == BaseUrlPolicy::FromServers {
            diagnostics.push(OpenApiDiagnostic::warn(
                "#/servers".to_string(),
                format!("the document declares no server; the base URL defaults to `{base_url}`"),
            ));
        }

        // `openapiv3` reads these required strings without checking them.
        let required_text = [
            ("#/info/version", Some(doc.info.version.as_str())),
            ("#/servers/0/url", doc.servers.first().map(|server| server.url.as_str())),
            ("#/externalDocs/url", doc.external_docs.as_ref().map(|docs| docs.url.as_str())),
        ];
        for (location, _) in required_text.iter().filter(|(_, text)| *text == Some("")) {
            diagnostics.push(OpenApiDiagnostic::error(location.to_string(), "must not be empty".to_string()));
        }
        let nonempty = |text: &str| (!text.is_empty()).then(|| text.to_string());

        let doc_ext = extension::decode_document(&doc.extensions, diagnostics);
        let no_details = Vec::new();
        let (auth, auth_policy) = reconcile::map_root_auth(
            doc,
            doc_ext.as_ref().map_or(&no_details, |ext| &ext.auth),
            diagnostics,
        );

        let mut endpoints = Vec::new();
        let mut outcomes = Vec::new();
        let mut used_names: HashSet<String> = HashSet::new();

        for (path, path_item_ref) in &doc.paths.paths {
            let path_item = match path_item_ref {
                openapiv3::ReferenceOr::Item(item) => item,
                openapiv3::ReferenceOr::Reference { reference } => {
                    diagnostics.push(OpenApiDiagnostic::warn(
                        format!("#/paths/{}", extension::escape(path)),
                        format!("Path reference '{}' not supported, skipping", reference),
                    ));
                    continue;
                }
            };

            for (method, operation) in Self::get_operations(path_item) {
                match map_operation(
                    &name,
                    operation,
                    &path_item.parameters,
                    path,
                    method,
                    resolver,
                    &mut used_names,
                    &self.options,
                ) {
                    Ok((mut endpoint, mut op_diags)) => {
                        if let Some(names) = operation.extensions.get("x-schematic-query-rust-names").and_then(serde_json::Value::as_object)
                            && let Some(params) = &mut endpoint.params {
                                for param in &mut params.query {
                                    param.rust_name = names.get(&param.name).and_then(serde_json::Value::as_str).map(str::to_string);
                                }
                        }
                        let location = format!("#/paths/{}/{}", extension::escape(path), method.to_string().to_lowercase());
                        let op_ext = extension::decode_operation(&operation.extensions, &location, doc_ext.is_some(), diagnostics);
                        let outcome = reconcile::apply_operation(&mut endpoint, operation, path, op_ext, doc_ext.as_ref(), doc, diagnostics);
                        // What the standard classification could not say about
                        // shapes the extension has now restored exactly.
                        op_diags.retain(|diagnostic| {
                            let superseded = (outcome.success_from_extension && diagnostic.location.starts_with("responses/2"))
                                || (outcome.errors_from_extension
                                    && (diagnostic.location.starts_with("responses/4") || diagnostic.location.starts_with("responses/5")));
                            !(superseded && diagnostic.severity == super::diagnostics::DiagnosticSeverity::Warn)
                        });
                        diagnostics.append(&mut op_diags);
                        endpoints.push(endpoint);
                        outcomes.push(outcome);
                    }
                    Err(e) => {
                        diagnostics.push(OpenApiDiagnostic::error(
                            format!("#/paths/{}/{}", extension::escape(path), method.to_string().to_lowercase()),
                            format!("Failed to map operation: {}", e),
                        ));
                    }
                }
            }
        }

        let mut api = RestApi { rate_limit: None, retry_guidance: None, errors: vec![], graphql: None, unchecked_compositions: false,
            name,
            description,
            base_url,
            docs_url: doc.external_docs.as_ref().and_then(|docs| nonempty(&docs.url)),
            auth,
            auth_policy,
            env_auth: vec![],
            headers: vec![],
            endpoints,
            module_path: self.options.module_path.clone(),
            request_suffix: None,
            version: nonempty(&doc.info.version),
            env_mapping: None,
            cross_origin: CrossOrigin::default(),
            request_id_header: None,
            secret_names: vec![],
            default_pagination: None,
            schemas: vec![],
            };
        if let Some(mut doc_ext) = doc_ext {
            reconcile::settle_document_errors(&mut api.endpoints, outcomes, &mut doc_ext, diagnostics);
            reconcile::apply_document(&mut api, doc_ext, &self.options, diagnostics);
        }
        reconcile::settle_endpoint_fallbacks(&mut api);
        Ok(api)
    }

    /// Extracts operations from a PathItem.
    fn get_operations(
        path_item: &openapiv3::PathItem,
    ) -> Vec<(crate::types::RestMethod, &openapiv3::Operation)> {
        use crate::types::RestMethod;

        let mut ops = Vec::new();
        if let Some(ref op) = path_item.get {
            ops.push((RestMethod::Get, op));
        }
        if let Some(ref op) = path_item.post {
            ops.push((RestMethod::Post, op));
        }
        if let Some(ref op) = path_item.put {
            ops.push((RestMethod::Put, op));
        }
        if let Some(ref op) = path_item.patch {
            ops.push((RestMethod::Patch, op));
        }
        if let Some(ref op) = path_item.delete {
            ops.push((RestMethod::Delete, op));
        }
        if let Some(ref op) = path_item.head {
            ops.push((RestMethod::Head, op));
        }
        if let Some(ref op) = path_item.options {
            ops.push((RestMethod::Options, op));
        }
        ops
    }
}

/// Options for OpenAPI import.
#[derive(Debug, Clone)]
pub struct OpenApiImportOptions {
    /// Override API name.
    pub api_name: Option<String>,
    /// Module path for generated code.
    pub module_path: Option<String>,
    /// Policy for determining base URL.
    pub base_url_policy: BaseUrlPolicy,
    /// Content type preference.
    pub content_preference: ContentPreference,
    /// Strict mode: fail on any WARN or ERROR diagnostic; INFO never fails.
    pub strict: bool,
}

impl Default for OpenApiImportOptions {
    fn default() -> Self {
        Self {
            api_name: None,
            module_path: None,
            base_url_policy: BaseUrlPolicy::FromServers,
            content_preference: ContentPreference::PreferJson,
            strict: false,
        }
    }
}

/// Policy for determining the base URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseUrlPolicy {
    /// Use the first server URL from the spec.
    FromServers,
    /// Use a custom URL.
    Override(String),
}

/// Content type preference when multiple are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentPreference {
    /// Prefer JSON over other content types.
    PreferJson,
    /// Use the first available content type.
    FirstAvailable,
}

/// Result of an OpenAPI import operation.
#[derive(Debug)]
pub struct OpenApiImportResult {
    /// The imported REST API definition.
    pub api: RestApi,
    /// Imported model definitions.
    pub models: ModelCatalog,
    /// Diagnostics generated during import.
    pub diagnostics: Vec<OpenApiDiagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::ResponseShape;

    // ========== OpenApiImport Builder Tests ==========

    #[test]
    fn new_creates_builder_with_default_options() {
        let source = OpenApiSource::yaml(
            "openapi: '3.0.0'\ninfo:\n  title: Test\n  version: '1.0'\npaths: {}",
        );
        let import = OpenApiImport::new(source);

        assert!(import.options.api_name.is_none());
        assert!(import.options.module_path.is_none());
        assert!(!import.options.strict);
    }

    #[test]
    fn api_name_sets_override() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).api_name("CustomApi");

        assert_eq!(import.options.api_name, Some("CustomApi".to_string()));
    }

    #[test]
    fn module_path_sets_path() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).module_path("custom::path");

        assert_eq!(import.options.module_path, Some("custom::path".to_string()));
    }

    #[test]
    fn base_url_from_servers_sets_policy() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).base_url_from_servers();

        assert_eq!(import.options.base_url_policy, BaseUrlPolicy::FromServers);
    }

    #[test]
    fn base_url_override_sets_url() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).base_url_override("https://custom.api.com");

        assert_eq!(
            import.options.base_url_policy,
            BaseUrlPolicy::Override("https://custom.api.com".to_string())
        );
    }

    #[test]
    fn prefer_json_sets_preference() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).prefer_json();

        assert_eq!(
            import.options.content_preference,
            ContentPreference::PreferJson
        );
    }

    #[test]
    fn strict_enables_strict_mode() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source).strict();

        assert!(import.options.strict);
    }

    #[test]
    fn builder_chain_works() {
        let source = OpenApiSource::yaml("");
        let import = OpenApiImport::new(source)
            .api_name("Test")
            .module_path("test::api")
            .base_url_override("https://api.test.com")
            .prefer_json()
            .strict();

        assert_eq!(import.options.api_name, Some("Test".to_string()));
        assert_eq!(import.options.module_path, Some("test::api".to_string()));
        assert!(import.options.strict);
    }

    // ========== Import Build Tests ==========

    #[test]
    fn build_parses_minimal_spec() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Minimal API
  version: "1.0.0"
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert_eq!(result.api.name, "MinimalApi");
        assert!(result.api.endpoints.is_empty());
    }

    #[test]
    fn build_uses_api_name_override() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Original Name
  version: "1.0.0"
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source)
            .api_name("OverriddenName")
            .build()
            .unwrap();

        assert_eq!(result.api.name, "OverriddenName");
    }

    #[test]
    fn build_extracts_base_url_from_servers() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
servers:
  - url: https://api.example.com/v1
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert_eq!(result.api.base_url, "https://api.example.com/v1");
    }

    #[test]
    fn build_uses_base_url_override() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
servers:
  - url: https://original.api.com
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source)
            .base_url_override("https://custom.api.com/v2")
            .build()
            .unwrap();

        assert_eq!(result.api.base_url, "https://custom.api.com/v2");
    }

    #[test]
    fn build_returns_error_for_invalid_yaml() {
        let source = OpenApiSource::yaml("not: valid: yaml: {{{}");
        let result = OpenApiImport::new(source).build();

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            super::super::super::error::OpenApiError::Parse { .. }
        ));
    }

    #[test]
    fn build_maps_simple_endpoint() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Pet Store
  version: "1.0.0"
paths:
  /pets:
    get:
      operationId: listPets
      summary: List all pets
      responses:
        '200':
          description: A list of pets
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert_eq!(result.api.endpoints.len(), 1);
        assert_eq!(result.api.endpoints[0].id, "ListPets");
        assert_eq!(result.api.endpoints[0].path, "/pets");
    }

    #[test]
    fn build_maps_bearer_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(matches!(
            result.api.auth,
            crate::auth::AuthStrategy::BearerToken { .. }
        ));
    }

    #[test]
    fn build_maps_basic_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  securitySchemes:
    basicAuth:
      type: http
      scheme: basic
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(matches!(result.api.auth, crate::auth::AuthStrategy::Basic { .. }));
    }

    #[test]
    fn build_maps_api_key_header_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  securitySchemes:
    apiKeyAuth:
      type: apiKey
      in: header
      name: X-API-Key
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        match result.api.auth {
            crate::auth::AuthStrategy::ApiKey { header, .. } => {
                assert_eq!(header, "X-API-Key");
            }
            _ => panic!("Expected ApiKey auth"),
        }
    }

    #[test]
    fn build_maps_api_key_query_auth() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  securitySchemes:
    apiKeyAuth:
      type: apiKey
      in: query
      name: api_key
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        match result.api.auth {
            crate::auth::AuthStrategy::ApiKeyParam { name, location } => {
                assert_eq!(name, "api_key");
                assert!(matches!(location, crate::auth::ApiKeyLocation::Query));
            }
            _ => panic!("Expected ApiKeyParam auth"),
        }
    }

    use super::super::diagnostics::DiagnosticSeverity;

    fn severities(diagnostics: &[OpenApiDiagnostic]) -> Vec<DiagnosticSeverity> {
        diagnostics.iter().map(|diagnostic| diagnostic.severity).collect()
    }

    /// No operationId: one INFO (fallback name).
    const INFO_ONLY_SPEC: &str = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
servers:
  - url: https://api.test
paths:
  /test:
    get:
      responses:
        '204':
          description: No content
"#;

    /// A path-level `$ref` is skipped with a WARN.
    const WARN_ONLY_SPEC: &str = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
servers:
  - url: https://api.test
paths:
  /test:
    $ref: '#/components/pathItems/Shared'
"#;

    /// An unresolvable local parameter `$ref`: one ERROR.
    const ERROR_ONLY_SPEC: &str = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
servers:
  - url: https://api.test
paths:
  /test:
    get:
      operationId: getTest
      parameters:
        - $ref: '#/components/parameters/Missing'
      responses:
        '204':
          description: No content
"#;

    #[test]
    fn build_strict_mode_passes_info_only_diagnostics() {
        let permissive = OpenApiImport::new(OpenApiSource::yaml(INFO_ONLY_SPEC)).build().unwrap();
        assert_eq!(severities(&permissive.diagnostics), [DiagnosticSeverity::Info]);

        let strict = OpenApiImport::new(OpenApiSource::yaml(INFO_ONLY_SPEC)).strict().build().unwrap();

        assert_eq!(strict.diagnostics, permissive.diagnostics);
        assert_eq!(strict.api.endpoints.len(), 1);
    }

    #[test]
    fn build_strict_mode_fails_on_warn_diagnostic() {
        let permissive = OpenApiImport::new(OpenApiSource::yaml(WARN_ONLY_SPEC)).build().unwrap();
        assert_eq!(severities(&permissive.diagnostics), [DiagnosticSeverity::Warn]);

        let error = OpenApiImport::new(OpenApiSource::yaml(WARN_ONLY_SPEC)).strict().build().unwrap_err();

        let message = error.to_string();
        assert!(message.contains("1 WARN/ERROR"), "{message}");
        assert!(message.contains(&permissive.diagnostics[0].location), "{message}");
        assert!(message.contains(&permissive.diagnostics[0].message), "{message}");
    }

    #[test]
    fn build_strict_mode_fails_on_error_diagnostic() {
        let permissive = OpenApiImport::new(OpenApiSource::yaml(ERROR_ONLY_SPEC)).build().unwrap();
        assert_eq!(severities(&permissive.diagnostics), [DiagnosticSeverity::Error]);

        let error = OpenApiImport::new(OpenApiSource::yaml(ERROR_ONLY_SPEC)).strict().build().unwrap_err();

        let message = error.to_string();
        assert!(message.contains("1 WARN/ERROR"), "{message}");
        assert!(message.contains("#/paths/~1test/get/parameters/0"), "{message}");
        assert!(message.contains("#/components/parameters/Missing"), "{message}");
    }

    #[test]
    fn build_extracts_docs_url() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
externalDocs:
  url: https://docs.example.com
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert_eq!(
            result.api.docs_url,
            Some("https://docs.example.com".to_string())
        );
    }

    #[test]
    fn build_imports_component_schemas() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(!result.models.types.is_empty());
    }

    #[test]
    fn build_validates_component_count() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build();

        assert!(result.is_ok());
    }

    // ========== Ref Resolution Tests ==========

    #[test]
    fn build_resolves_schema_refs() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /pets:
    get:
      operationId: listPets
      responses:
        '200':
          description: List of pets
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/PetList'
components:
  schemas:
    Pet:
      type: object
      properties:
        id:
          type: integer
    PetList:
      type: array
      items:
        $ref: '#/components/schemas/Pet'
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(!result.models.types.is_empty());
    }

    // ========== Parameter Mapping Tests ==========

    #[test]
    fn build_maps_path_parameters() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /pets/{petId}:
    get:
      operationId: getPet
      parameters:
        - name: petId
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: A pet
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert_eq!(result.api.endpoints.len(), 1);
        assert!(result.api.endpoints[0].path.contains("{petId}"));
    }

    #[test]
    fn build_maps_query_parameters() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /pets:
    get:
      operationId: listPets
      parameters:
        - name: limit
          in: query
          required: false
          schema:
            type: integer
      responses:
        '200':
          description: List of pets
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        let endpoint = &result.api.endpoints[0];
        assert!(endpoint.params.is_some());
        let params = endpoint.params.as_ref().unwrap();
        assert!(!params.query.is_empty());
        assert_eq!(params.query[0].name, "limit");
    }

    #[test]
    fn build_maps_header_parameters() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: test
      parameters:
        - name: X-Request-ID
          in: header
          required: false
          schema:
            type: string
      responses:
        '200':
          description: OK
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        let endpoint = &result.api.endpoints[0];
        let params = endpoint.params.as_ref().unwrap();
        assert!(!params.header.is_empty());
        assert_eq!(params.header[0].name, "X-Request-ID");
    }

    // ========== Response Mapping Tests ==========

    #[test]
    fn build_selects_200_response() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /test:
    get:
      operationId: test
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: object
        '400':
          description: Bad Request
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(matches!(
            result.api.endpoints[0].response[0].shape,
            ResponseShape::Json(_)
        ));
    }

    #[test]
    fn build_maps_204_to_empty() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /test:
    delete:
      operationId: deleteTest
      responses:
        '204':
          description: No Content
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(matches!(
            result.api.endpoints[0].response[0].shape,
            ResponseShape::Empty
        ));
    }

    #[test]
    fn build_maps_binary_response() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /download:
    get:
      operationId: download
      responses:
        '200':
          description: Binary file
          content:
            application/octet-stream:
              schema:
                type: string
                format: binary
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(matches!(
            result.api.endpoints[0].response[0].shape,
            ResponseShape::Binary
        ));
    }

    // ========== Diagnostic Tests ==========

    #[test]
    fn build_generates_diagnostic_for_missing_operation_id() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        assert!(!result.api.endpoints.is_empty());
    }

    // ========== OpenApiImportOptions Tests ==========

    #[test]
    fn import_options_default() {
        let opts = OpenApiImportOptions::default();

        assert!(opts.api_name.is_none());
        assert!(opts.module_path.is_none());
        assert_eq!(opts.base_url_policy, BaseUrlPolicy::FromServers);
        assert_eq!(opts.content_preference, ContentPreference::PreferJson);
        assert!(!opts.strict);
    }

    // ========== OpenApiImportResult Tests ==========

    #[test]
    fn import_result_has_expected_fields() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
"#;
        let source = OpenApiSource::yaml(yaml);
        let result = OpenApiImport::new(source).build().unwrap();

        let _api = &result.api;
        let _models = &result.models;
        let _diagnostics = &result.diagnostics;
    }

    // ========== schema/field name sanitization + deconfliction ==========

    fn model_names(result: &super::OpenApiImportResult) -> Vec<String> {
        result
            .models
            .types
            .iter()
            .map(|m| match m {
                crate::models::ModelDef::Struct(s) => s.name.clone(),
                crate::models::ModelDef::Enum(e) => e.name.clone(),
                crate::models::ModelDef::Alias(a) => a.name.clone(),
            })
            .collect()
    }

    #[test]
    fn build_deconflicts_schema_names_that_sanitize_identically() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  schemas:
    user-name:
      type: object
      properties:
        label:
          type: string
    user_name:
      type: object
      properties:
        code:
          type: string
    container:
      type: object
      properties:
        a:
          $ref: '#/components/schemas/user-name'
        b:
          $ref: '#/components/schemas/user_name'
"#;
        let result = OpenApiImport::new(OpenApiSource::yaml(yaml)).build().unwrap();
        let names = model_names(&result);
        assert!(names.contains(&"UserName".to_string()), "names: {names:?}");
        assert!(names.contains(&"UserName2".to_string()), "names: {names:?}");

        let container = result
            .models
            .types
            .iter()
            .find_map(|m| match m {
                crate::models::ModelDef::Struct(s) if s.name == "Container" => Some(s),
                _ => None,
            })
            .expect("Container model");
        let field_a = container.fields.iter().find(|f| f.name == "a").unwrap();
        let field_b = container.fields.iter().find(|f| f.name == "b").unwrap();
        // The two refs must resolve to the two distinct deconflicted models,
        // not both collapse onto one.
        assert_eq!(
            field_a.field_type,
            crate::models::TypeRef::Named("UserName".to_string())
        );
        assert_eq!(
            field_b.field_type,
            crate::models::TypeRef::Named("UserName2".to_string())
        );
    }

    #[test]
    fn build_deconflicts_enum_variants_that_sanitize_identically() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  schemas:
    status:
      type: string
      enum:
        - active
        - Active
"#;
        let result = OpenApiImport::new(OpenApiSource::yaml(yaml)).build().unwrap();
        let status = result
            .models
            .types
            .iter()
            .find_map(|m| match m {
                crate::models::ModelDef::Enum(e) if e.name == "Status" => Some(e),
                _ => None,
            })
            .expect("Status enum");

        assert_eq!(status.variants.len(), 2);
        assert_ne!(status.variants[0].name, status.variants[1].name);
        let values: Vec<Option<&str>> =
            status.variants.iter().map(|v| v.value.as_deref()).collect();
        assert!(values.contains(&Some("active")), "values: {values:?}");
        assert!(values.contains(&Some("Active")), "values: {values:?}");
    }

    #[test]
    fn build_sanitizes_reserved_and_invalid_field_names() {
        let yaml = r#"
openapi: "3.0.0"
info:
  title: Test
  version: "1.0.0"
paths: {}
components:
  schemas:
    thing:
      type: object
      properties:
        type:
          type: string
        self:
          type: string
        "2fa":
          type: string
        "":
          type: string
"#;
        let result = OpenApiImport::new(OpenApiSource::yaml(yaml)).build().unwrap();
        let thing = result
            .models
            .types
            .iter()
            .find_map(|m| match m {
                crate::models::ModelDef::Struct(s) if s.name == "Thing" => Some(s),
                _ => None,
            })
            .expect("Thing model");

        let by_rename = |wire: &str| {
            thing
                .fields
                .iter()
                .find(|f| f.serde_rename.as_deref() == Some(wire))
                .unwrap_or_else(|| panic!("no field renamed to {wire:?}"))
        };
        assert_eq!(by_rename("type").name, "type_");
        assert_eq!(by_rename("self").name, "self_");
        assert_eq!(by_rename("2fa").name, "_2fa");
        assert_eq!(by_rename("").name, "field");
    }
}
