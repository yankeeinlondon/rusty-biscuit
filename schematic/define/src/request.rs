//! API request body type definitions.
//!
//! This module defines the types of request bodies an API endpoint can accept.
//! The request type determines how the generated client serializes and sends
//! the request body.
//!
//! ## Request Types
//!
//! - [`ApiRequest::Json`] - JSON request body (most common)
//! - [`ApiRequest::FormData`] - Multipart form-data for file uploads
//! - [`ApiRequest::UrlEncoded`] - URL-encoded form data
//! - [`ApiRequest::Text`] - Raw text body
//! - [`ApiRequest::Binary`] - Raw binary body

use serde::{Deserialize, Serialize};

use crate::schema::Schema;

/// The type of content a form field accepts.
///
/// Used with [`ApiRequest::FormData`] and [`ApiRequest::UrlEncoded`] to
/// describe individual fields in a form submission.
///
/// ## Examples
///
/// ```
/// use schematic_define::FormFieldKind;
///
/// // Simple text field
/// let text = FormFieldKind::Text;
///
/// // File upload accepting any type
/// let file = FormFieldKind::File { accept: vec![] };
///
/// // File upload with MIME restrictions
/// let audio = FormFieldKind::File {
///     accept: vec!["audio/*".into()],
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormFieldKind {
    /// Plain text field (Content-Type: text/plain).
    ///
    /// Suitable for simple string values like IDs, names, descriptions.
    Text,

    /// File upload field.
    ///
    /// The `accept` patterns follow MIME type syntax with wildcards:
    /// - `"image/*"` - Any image type
    /// - `"audio/mpeg"` - Specific MIME type
    /// - `"application/pdf, application/msword"` - Multiple types
    /// - Empty vec means any file type is accepted
    File {
        /// Accepted MIME type patterns.
        ///
        /// Empty vec means any file type is accepted.
        accept: Vec<String>,
    },

    /// Multiple files with same field name.
    ///
    /// Per RFC 7578, multiple files are sent as separate parts
    /// sharing the same `name` attribute.
    Files {
        /// Accepted MIME type patterns (same as File).
        accept: Vec<String>,

        /// Minimum number of files required.
        min: Option<u32>,

        /// Maximum number of files allowed.
        max: Option<u32>,
    },

    /// Structured JSON data embedded as a form field.
    ///
    /// The field's Content-Type is set to `application/json`.
    /// Useful for metadata accompanying file uploads.
    Json(Schema),
}

/// Describes a single field in a multipart or URL-encoded form.
///
/// Form fields have a name, a type (kind), and optionally a description.
/// By default, fields are required unless marked as optional.
///
/// ## Examples
///
/// Create fields using builder methods:
///
/// ```
/// use schematic_define::FormField;
///
/// // Required text field
/// let name = FormField::text("name");
///
/// // Optional text field with description
/// let nickname = FormField::text("nickname")
///     .optional()
///     .with_description("User's preferred nickname");
///
/// // File upload with MIME restrictions
/// let avatar = FormField::file_accept("avatar", vec!["image/*".into()])
///     .with_description("Profile picture");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormField {
    /// Field name as it appears in Content-Disposition.
    ///
    /// Example: `"file"`, `"metadata"`, `"user_id"`
    pub name: String,

    /// The kind of field (text, file, or structured JSON).
    pub kind: FormFieldKind,

    /// Whether this field is required.
    ///
    /// Required fields must be present in the request.
    pub required: bool,

    /// Human-readable description of this field.
    pub description: Option<String>,
}

impl FormField {
    /// Creates a required text field.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::FormField;
    ///
    /// let field = FormField::text("username");
    /// assert!(field.required);
    /// ```
    pub fn text(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Text,
            required: true,
            description: None,
        }
    }

    /// Creates a required file field accepting any type.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{FormField, FormFieldKind};
    ///
    /// let field = FormField::file("document");
    /// assert!(field.required);
    /// assert!(matches!(field.kind, FormFieldKind::File { accept } if accept.is_empty()));
    /// ```
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::File { accept: vec![] },
            required: true,
            description: None,
        }
    }

    /// Creates a required file field with MIME type restrictions.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{FormField, FormFieldKind};
    ///
    /// let field = FormField::file_accept("audio", vec!["audio/*".into()]);
    /// assert!(field.required);
    /// if let FormFieldKind::File { accept } = &field.kind {
    ///     assert_eq!(accept, &vec!["audio/*".to_string()]);
    /// }
    /// ```
    pub fn file_accept(name: impl Into<String>, accept: Vec<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::File { accept },
            required: true,
            description: None,
        }
    }

    /// Creates a required multi-file field accepting any type.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{FormField, FormFieldKind};
    ///
    /// let field = FormField::files("attachments");
    /// assert!(field.required);
    /// assert!(matches!(field.kind, FormFieldKind::Files { .. }));
    /// ```
    pub fn files(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Files {
                accept: vec![],
                min: None,
                max: None,
            },
            required: true,
            description: None,
        }
    }

    /// Creates a required multi-file field with constraints.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{FormField, FormFieldKind};
    ///
    /// let field = FormField::files_with_constraints(
    ///     "samples",
    ///     vec!["audio/*".into()],
    ///     Some(1),
    ///     Some(10),
    /// );
    /// if let FormFieldKind::Files { accept, min, max } = &field.kind {
    ///     assert_eq!(accept, &vec!["audio/*".to_string()]);
    ///     assert_eq!(*min, Some(1));
    ///     assert_eq!(*max, Some(10));
    /// }
    /// ```
    pub fn files_with_constraints(
        name: impl Into<String>,
        accept: Vec<String>,
        min: Option<u32>,
        max: Option<u32>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Files { accept, min, max },
            required: true,
            description: None,
        }
    }

    /// Creates a required JSON metadata field.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{FormField, FormFieldKind, Schema};
    ///
    /// let field = FormField::json("metadata", Schema::new("MetadataRequest"));
    /// assert!(field.required);
    /// assert!(matches!(field.kind, FormFieldKind::Json(_)));
    /// ```
    pub fn json(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Json(schema),
            required: true,
            description: None,
        }
    }

    /// Makes the field optional.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::FormField;
    ///
    /// let field = FormField::text("nickname").optional();
    /// assert!(!field.required);
    /// ```
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Adds a description to the field.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::FormField;
    ///
    /// let field = FormField::text("email")
    ///     .with_description("User's email address");
    /// assert_eq!(field.description, Some("User's email address".to_string()));
    /// ```
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Describes the request body format for an API endpoint.
///
/// Each variant indicates a different request format, which affects
/// how the generated client serializes and sends the request body.
///
/// ## Examples
///
/// JSON request (most common):
///
/// ```
/// use schematic_define::ApiRequest;
///
/// let request = ApiRequest::json_type("CreateUserRequest");
/// ```
///
/// Multipart form-data with file upload:
///
/// ```
/// use schematic_define::{ApiRequest, FormField};
///
/// let request = ApiRequest::form_data(vec![
///     FormField::file_accept("audio", vec!["audio/*".into()])
///         .with_description("Audio file"),
///     FormField::text("name").optional(),
/// ]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiRequest {
    /// JSON request body.
    ///
    /// Sets Content-Type to `application/json`.
    Json(Schema),

    /// Multipart form-data request.
    ///
    /// Sets Content-Type to `multipart/form-data` with auto-generated boundary.
    /// Use for file uploads and mixed file/data payloads.
    FormData {
        /// Fields in the multipart form.
        fields: Vec<FormField>,
    },

    /// URL-encoded form data.
    ///
    /// Sets Content-Type to `application/x-www-form-urlencoded`.
    /// Only suitable for simple text fields (no files).
    UrlEncoded {
        /// Fields in the form (text only).
        fields: Vec<FormField>,
    },

    /// Raw text body.
    ///
    /// Sets Content-Type based on provided MIME type.
    Text {
        /// MIME type (e.g., "text/plain", "text/csv").
        content_type: String,
    },

    /// Raw binary body.
    ///
    /// Sets Content-Type based on provided MIME type.
    Binary {
        /// MIME type (e.g., "application/octet-stream").
        content_type: String,
    },
}

impl ApiRequest {
    /// Creates a JSON request with the given schema.
    ///
    /// Use this when you have a pre-built [`Schema`] with a module path.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiRequest, Schema};
    ///
    /// let schema = Schema::with_path("CreateUser", "crate::models");
    /// let request = ApiRequest::json(schema);
    /// ```
    pub fn json(schema: Schema) -> Self {
        Self::Json(schema)
    }

    /// Creates a JSON request with just a type name.
    ///
    /// This is the most common way to specify a JSON request. The type
    /// name should match a struct in the generated or imported code.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::ApiRequest;
    ///
    /// let request = ApiRequest::json_type("CreateUserRequest");
    ///
    /// // Verify the schema was created correctly
    /// if let ApiRequest::Json(schema) = request {
    ///     assert_eq!(schema.type_name, "CreateUserRequest");
    /// }
    /// ```
    pub fn json_type(type_name: impl Into<String>) -> Self {
        Self::Json(Schema::new(type_name))
    }

    /// Creates a multipart form-data request.
    ///
    /// Use for file uploads and mixed file/data payloads.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiRequest, FormField};
    ///
    /// let request = ApiRequest::form_data(vec![
    ///     FormField::file("document"),
    ///     FormField::text("title"),
    /// ]);
    ///
    /// if let ApiRequest::FormData { fields } = request {
    ///     assert_eq!(fields.len(), 2);
    /// }
    /// ```
    pub fn form_data(fields: Vec<FormField>) -> Self {
        Self::FormData { fields }
    }

    /// Creates a URL-encoded form request.
    ///
    /// Only suitable for simple text fields (no files).
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::{ApiRequest, FormField};
    ///
    /// let request = ApiRequest::url_encoded(vec![
    ///     FormField::text("username"),
    ///     FormField::text("password"),
    /// ]);
    /// ```
    pub fn url_encoded(fields: Vec<FormField>) -> Self {
        Self::UrlEncoded { fields }
    }

    /// Creates a raw text request.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::ApiRequest;
    ///
    /// let request = ApiRequest::text("text/csv");
    ///
    /// if let ApiRequest::Text { content_type } = request {
    ///     assert_eq!(content_type, "text/csv");
    /// }
    /// ```
    pub fn text(content_type: impl Into<String>) -> Self {
        Self::Text {
            content_type: content_type.into(),
        }
    }

    /// Creates a raw binary request.
    ///
    /// ## Examples
    ///
    /// ```
    /// use schematic_define::ApiRequest;
    ///
    /// let request = ApiRequest::binary("application/octet-stream");
    ///
    /// if let ApiRequest::Binary { content_type } = request {
    ///     assert_eq!(content_type, "application/octet-stream");
    /// }
    /// ```
    pub fn binary(content_type: impl Into<String>) -> Self {
        Self::Binary {
            content_type: content_type.into(),
        }
    }
}

/// Provides backward compatibility for code using `Schema` directly.
///
/// Converts a [`Schema`] to [`ApiRequest::Json`].
impl From<Schema> for ApiRequest {
    fn from(schema: Schema) -> Self {
        ApiRequest::Json(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_field_text_is_required_by_default() {
        let field = FormField::text("name");
        assert_eq!(field.name, "name");
        assert!(field.required);
        assert!(matches!(field.kind, FormFieldKind::Text));
        assert!(field.description.is_none());
    }

    #[test]
    fn form_field_optional_makes_not_required() {
        let field = FormField::text("nickname").optional();
        assert!(!field.required);
    }

    #[test]
    fn form_field_with_description_sets_description() {
        let field = FormField::text("email").with_description("User email");
        assert_eq!(field.description, Some("User email".to_string()));
    }

    #[test]
    fn form_field_builder_chaining() {
        let field = FormField::text("bio")
            .optional()
            .with_description("User biography");

        assert_eq!(field.name, "bio");
        assert!(!field.required);
        assert_eq!(field.description, Some("User biography".to_string()));
    }

    #[test]
    fn form_field_file_creates_file_kind() {
        let field = FormField::file("document");
        assert!(matches!(field.kind, FormFieldKind::File { accept } if accept.is_empty()));
    }

    #[test]
    fn form_field_file_accept_sets_mime_types() {
        let field = FormField::file_accept("image", vec!["image/png".into(), "image/jpeg".into()]);
        if let FormFieldKind::File { accept } = field.kind {
            assert_eq!(accept, vec!["image/png", "image/jpeg"]);
        } else {
            panic!("Expected File kind");
        }
    }

    #[test]
    fn form_field_files_creates_files_kind() {
        let field = FormField::files("attachments");
        assert!(matches!(
            field.kind,
            FormFieldKind::Files {
                accept,
                min: None,
                max: None
            } if accept.is_empty()
        ));
    }

    #[test]
    fn form_field_files_with_constraints() {
        let field =
            FormField::files_with_constraints("samples", vec!["audio/*".into()], Some(1), Some(5));

        if let FormFieldKind::Files { accept, min, max } = field.kind {
            assert_eq!(accept, vec!["audio/*"]);
            assert_eq!(min, Some(1));
            assert_eq!(max, Some(5));
        } else {
            panic!("Expected Files kind");
        }
    }

    #[test]
    fn form_field_json_creates_json_kind() {
        let schema = Schema::new("Metadata");
        let field = FormField::json("meta", schema);
        assert!(matches!(field.kind, FormFieldKind::Json(_)));
    }

    #[test]
    fn api_request_json_type() {
        let request = ApiRequest::json_type("CreateUserRequest");
        if let ApiRequest::Json(schema) = request {
            assert_eq!(schema.type_name, "CreateUserRequest");
        } else {
            panic!("Expected Json variant");
        }
    }

    #[test]
    fn api_request_form_data() {
        let request = ApiRequest::form_data(vec![
            FormField::file("doc"),
            FormField::text("name").optional(),
        ]);

        if let ApiRequest::FormData { fields } = request {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "doc");
            assert_eq!(fields[1].name, "name");
        } else {
            panic!("Expected FormData variant");
        }
    }

    #[test]
    fn api_request_url_encoded() {
        let request = ApiRequest::url_encoded(vec![FormField::text("user"), FormField::text("pass")]);

        if let ApiRequest::UrlEncoded { fields } = request {
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected UrlEncoded variant");
        }
    }

    #[test]
    fn api_request_text() {
        let request = ApiRequest::text("text/csv");
        if let ApiRequest::Text { content_type } = request {
            assert_eq!(content_type, "text/csv");
        } else {
            panic!("Expected Text variant");
        }
    }

    #[test]
    fn api_request_binary() {
        let request = ApiRequest::binary("application/octet-stream");
        if let ApiRequest::Binary { content_type } = request {
            assert_eq!(content_type, "application/octet-stream");
        } else {
            panic!("Expected Binary variant");
        }
    }

    #[test]
    fn api_request_from_schema() {
        let schema = Schema::new("TestRequest");
        let request: ApiRequest = schema.into();

        assert!(matches!(request, ApiRequest::Json(_)));
    }

    #[test]
    fn form_field_kind_serde_roundtrip_text() {
        let kind = FormFieldKind::Text;
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: FormFieldKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }

    #[test]
    fn form_field_kind_serde_roundtrip_file() {
        let kind = FormFieldKind::File {
            accept: vec!["image/*".into()],
        };
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: FormFieldKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }

    #[test]
    fn form_field_kind_serde_roundtrip_files() {
        let kind = FormFieldKind::Files {
            accept: vec!["audio/*".into()],
            min: Some(1),
            max: Some(10),
        };
        let json = serde_json::to_string(&kind).unwrap();
        let parsed: FormFieldKind = serde_json::from_str(&json).unwrap();
        assert_eq!(kind, parsed);
    }

    #[test]
    fn form_field_serde_roundtrip() {
        let field = FormField::text("username")
            .optional()
            .with_description("The username");

        let json = serde_json::to_string(&field).unwrap();
        let parsed: FormField = serde_json::from_str(&json).unwrap();
        assert_eq!(field, parsed);
    }

    #[test]
    fn api_request_serde_roundtrip_json() {
        let request = ApiRequest::json_type("TestRequest");
        let json = serde_json::to_string(&request).unwrap();
        let parsed: ApiRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, parsed);
    }

    #[test]
    fn api_request_serde_roundtrip_form_data() {
        let request = ApiRequest::form_data(vec![
            FormField::file_accept("audio", vec!["audio/*".into()]),
            FormField::text("name").optional(),
        ]);

        let json = serde_json::to_string(&request).unwrap();
        let parsed: ApiRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, parsed);
    }

    // =========================================================================
    // Integration Tests - Realistic API Usage Patterns
    // =========================================================================

    #[test]
    fn integration_mixed_endpoint_types() {
        use crate::{AuthStrategy, Endpoint, RestApi, RestMethod, ApiResponse};

        // Create an API with mixed request types
        let api = RestApi {
            name: "FileService".to_string(),
            description: "File upload and management service".to_string(),
            base_url: "https://api.example.com/v1".to_string(),
            docs_url: Some("https://docs.example.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["FILE_SERVICE_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![
                // JSON endpoint
                Endpoint {
                    id: "CreateFolder".to_string(),
                    method: RestMethod::Post,
                    path: "/folders".to_string(),
                    description: "Create a new folder".to_string(),
                    request: Some(ApiRequest::json_type("CreateFolderRequest")),
                    response: ApiResponse::json_type("Folder"),
                    headers: vec![],
                },
                // FormData endpoint with file upload
                Endpoint {
                    id: "UploadFile".to_string(),
                    method: RestMethod::Post,
                    path: "/folders/{folder_id}/files".to_string(),
                    description: "Upload a file to a folder".to_string(),
                    request: Some(ApiRequest::form_data(vec![
                        FormField::file("file").with_description("The file to upload"),
                        FormField::text("name").optional().with_description("Override filename"),
                        FormField::json("metadata", Schema::new("FileMetadata")).optional(),
                    ])),
                    response: ApiResponse::json_type("File"),
                    headers: vec![],
                },
                // GET endpoint with no request body
                Endpoint {
                    id: "ListFiles".to_string(),
                    method: RestMethod::Get,
                    path: "/folders/{folder_id}/files".to_string(),
                    description: "List files in a folder".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListFilesResponse"),
                    headers: vec![],
                },
                // Binary download
                Endpoint {
                    id: "DownloadFile".to_string(),
                    method: RestMethod::Get,
                    path: "/files/{file_id}/content".to_string(),
                    description: "Download file content".to_string(),
                    request: None,
                    response: ApiResponse::Binary,
                    headers: vec![],
                },
            ],
        };

        assert_eq!(api.name, "FileService");
        assert_eq!(api.endpoints.len(), 4);

        // Verify endpoint types
        let create_folder = &api.endpoints[0];
        assert!(matches!(create_folder.request, Some(ApiRequest::Json(_))));

        let upload_file = &api.endpoints[1];
        if let Some(ApiRequest::FormData { fields }) = &upload_file.request {
            assert_eq!(fields.len(), 3);
            assert!(fields[0].required);
            assert!(!fields[1].required);
            assert!(!fields[2].required);
        } else {
            panic!("Expected FormData request");
        }

        let list_files = &api.endpoints[2];
        assert!(list_files.request.is_none());
    }

    #[test]
    fn integration_multifile_upload_pattern() {
        // Test the multi-file upload pattern from the spec
        let request = ApiRequest::form_data(vec![
            FormField::files_with_constraints(
                "documents",
                vec!["application/pdf".into(), "text/*".into()],
                Some(1),
                Some(10),
            )
            .with_description("Documents to process (1-10 files)"),
            FormField::text("category").with_description("Document category"),
            FormField::text("tags").optional().with_description("Comma-separated tags"),
        ]);

        if let ApiRequest::FormData { fields } = &request {
            assert_eq!(fields.len(), 3);

            // Verify multi-file field
            if let FormFieldKind::Files { accept, min, max } = &fields[0].kind {
                assert_eq!(accept.len(), 2);
                assert_eq!(*min, Some(1));
                assert_eq!(*max, Some(10));
            } else {
                panic!("Expected Files kind");
            }

            // Verify required/optional
            assert!(fields[0].required);
            assert!(fields[1].required);
            assert!(!fields[2].required);
        } else {
            panic!("Expected FormData");
        }
    }

    #[test]
    fn integration_all_request_variants_serializable() {
        // Ensure all ApiRequest variants can be serialized
        let variants = vec![
            ApiRequest::json_type("TestRequest"),
            ApiRequest::form_data(vec![
                FormField::file("doc"),
                FormField::text("name"),
            ]),
            ApiRequest::url_encoded(vec![
                FormField::text("username"),
                FormField::text("password"),
            ]),
            ApiRequest::text("text/plain"),
            ApiRequest::binary("application/octet-stream"),
        ];

        for variant in variants {
            let json = serde_json::to_string(&variant).expect("Should serialize");
            let parsed: ApiRequest = serde_json::from_str(&json).expect("Should deserialize");
            assert_eq!(variant, parsed);
        }
    }

    #[test]
    fn integration_backward_compatibility_from_schema() {
        // Code that used Schema directly should still work via From trait
        let schema = Schema::new("LegacyRequest");
        let request: ApiRequest = schema.into();

        if let ApiRequest::Json(s) = &request {
            assert_eq!(s.type_name, "LegacyRequest");
        } else {
            panic!("From<Schema> should create ApiRequest::Json");
        }

        // Verify it serializes correctly
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("LegacyRequest"));
    }
}
