//! Normalized HTTP endpoint representation for export.

use schematic_define::RestMethod;

use super::{ExportAuth, ExportBody};

/// A normalized HTTP endpoint ready for export to OpenAPI or Postman.
#[derive(Debug, Clone)]
pub struct ExportEndpoint {
    /// Endpoint identifier (e.g., "ListModels").
    pub id: String,
    /// HTTP method.
    pub method: RestMethod,
    /// URL path template (e.g., "/models/{model}").
    pub path: String,
    /// Human-readable description.
    pub description: String,
    /// Folder key inferred from path (for Postman folders).
    pub folder_key: Option<String>,
    /// Path parameter names extracted from the path template.
    pub path_params: Vec<String>,
    /// Query parameters.
    pub query_params: Vec<ExportParam>,
    /// Additional headers for this endpoint.
    pub headers: Vec<(String, String)>,
    /// Request body, if any.
    pub body: Option<ExportBody>,
    /// Per-endpoint auth override (when different from collection-level).
    pub auth_override: Option<ExportAuth>,
}

/// A query or header parameter for export.
#[derive(Debug, Clone)]
pub struct ExportParam {
    /// Parameter name.
    pub name: String,
    /// Default value, if any.
    pub value: String,
    /// Human-readable description.
    pub description: Option<String>,
}
