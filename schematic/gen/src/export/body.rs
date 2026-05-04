//! Normalized request body for export.

use schematic_define::request::{ApiRequest, FormFieldKind};

/// Normalized request body for export formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportBody {
    /// JSON request body.
    Json {
        /// Content type (typically "application/json").
        content_type: String,
    },
    /// Multipart form data.
    FormData {
        /// Form fields.
        fields: Vec<FormField>,
    },
    /// URL-encoded form data.
    UrlEncoded {
        /// Form fields.
        fields: Vec<FormField>,
    },
    /// Plain text body.
    Text {
        /// Content type.
        content_type: String,
    },
    /// Binary body.
    Binary,
}

/// A form field for export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    /// Field name.
    pub name: String,
    /// Whether the field is required.
    pub required: bool,
    /// Human-readable description.
    pub description: Option<String>,
    /// Field kind discriminator preserved from the source definition.
    ///
    /// Carries through whether the original `schematic_define::FormField`
    /// was a text, file, files, or JSON-part field so downstream exporters
    /// (Postman, OpenAPI, etc.) can emit format-appropriate output without
    /// re-deriving the kind from the field name.
    pub kind: FormFieldExportKind,
}

/// Discriminator for export-side form fields.
///
/// Mirrors [`schematic_define::FormFieldKind`] but is decoupled from the
/// source enum so the export layer is free to evolve independently. JSON
/// metadata parts collapse to a unit variant here because exporters only
/// need to know "this is a JSON-typed part", not the full schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldExportKind {
    /// Plain text field.
    Text,
    /// Single file upload field with optional MIME accept patterns.
    File {
        /// Accepted MIME type patterns (empty = any type).
        accept: Vec<String>,
    },
    /// Multiple files sharing one field name with optional count constraints.
    Files {
        /// Accepted MIME type patterns (empty = any type).
        accept: Vec<String>,
        /// Minimum number of files required, if any.
        min: Option<u32>,
        /// Maximum number of files allowed, if any.
        max: Option<u32>,
    },
    /// Structured JSON part embedded in the form.
    Json,
}

/// Maps a schematic `ApiRequest` to an `ExportBody`.
///
/// ## Examples
///
/// ```
/// use schematic_define::request::ApiRequest;
/// use schematic_define::Schema;
/// use schematic_gen::export::body::{ExportBody, map_body};
///
/// let body = map_body(&ApiRequest::Json(Schema::new("TestRequest")));
/// assert_eq!(body, ExportBody::Json { content_type: "application/json".to_string() });
/// ```
pub fn map_body(request: &ApiRequest) -> ExportBody {
    match request {
        ApiRequest::Json(_) => ExportBody::Json {
            content_type: "application/json".to_string(),
        },
        ApiRequest::FormData { fields } => ExportBody::FormData {
            fields: fields.iter().map(map_form_field).collect(),
        },
        ApiRequest::UrlEncoded { fields } => ExportBody::UrlEncoded {
            fields: fields.iter().map(map_form_field).collect(),
        },
        ApiRequest::Text { content_type } => ExportBody::Text {
            content_type: content_type.clone(),
        },
        ApiRequest::Binary { .. } => ExportBody::Binary,
        // Handle any future variants by defaulting to binary
        _ => ExportBody::Binary,
    }
}

fn map_form_field(field: &schematic_define::request::FormField) -> FormField {
    FormField {
        name: field.name.clone(),
        required: field.required,
        description: field.description.clone(),
        kind: map_form_field_kind(&field.kind),
    }
}

fn map_form_field_kind(kind: &FormFieldKind) -> FormFieldExportKind {
    match kind {
        FormFieldKind::Text => FormFieldExportKind::Text,
        FormFieldKind::File { accept } => FormFieldExportKind::File {
            accept: accept.clone(),
        },
        FormFieldKind::Files { accept, min, max } => FormFieldExportKind::Files {
            accept: accept.clone(),
            min: *min,
            max: *max,
        },
        FormFieldKind::Json(_) => FormFieldExportKind::Json,
        // FormFieldKind is #[non_exhaustive]; future variants fall back
        // to plain text so exporters keep working until explicit support
        // is added.
        _ => FormFieldExportKind::Text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::Schema;

    #[test]
    fn map_body_json() {
        assert_eq!(
            map_body(&ApiRequest::Json(Schema::new("TestRequest"))),
            ExportBody::Json {
                content_type: "application/json".to_string()
            }
        );
    }

    #[test]
    fn map_body_text() {
        assert_eq!(
            map_body(&ApiRequest::Text {
                content_type: "text/plain".to_string()
            }),
            ExportBody::Text {
                content_type: "text/plain".to_string()
            }
        );
    }

    #[test]
    fn map_body_binary() {
        assert_eq!(
            map_body(&ApiRequest::Binary {
                content_type: "application/octet-stream".to_string()
            }),
            ExportBody::Binary
        );
    }

    #[test]
    fn map_body_form_data() {
        let request = ApiRequest::FormData {
            fields: vec![
                schematic_define::request::FormField::text("name").with_description("User name"),
                schematic_define::request::FormField::file("avatar").optional(),
            ],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "name");
                assert!(fields[0].required);
                assert_eq!(fields[0].description.as_deref(), Some("User name"));
                assert_eq!(fields[0].kind, FormFieldExportKind::Text);
                assert_eq!(fields[1].name, "avatar");
                assert!(!fields[1].required);
                assert_eq!(
                    fields[1].kind,
                    FormFieldExportKind::File { accept: vec![] }
                );
            }
            _ => panic!("Expected FormData"),
        }
    }

    #[test]
    fn map_body_url_encoded() {
        let request = ApiRequest::UrlEncoded {
            fields: vec![schematic_define::request::FormField::text("grant_type")],
        };
        let body = map_body(&request);
        match body {
            ExportBody::UrlEncoded { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "grant_type");
                assert_eq!(fields[0].kind, FormFieldExportKind::Text);
            }
            _ => panic!("Expected UrlEncoded"),
        }
    }

    #[test]
    fn map_body_form_data_preserves_file_kind() {
        let request = ApiRequest::FormData {
            fields: vec![schematic_define::request::FormField::file("audio")],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "audio");
                assert_eq!(
                    fields[0].kind,
                    FormFieldExportKind::File { accept: vec![] }
                );
            }
            _ => panic!("Expected FormData"),
        }
    }

    #[test]
    fn map_body_form_data_preserves_file_accept_patterns() {
        let request = ApiRequest::FormData {
            fields: vec![schematic_define::request::FormField::file_accept(
                "audio",
                vec!["audio/*".into()],
            )],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(
                    fields[0].kind,
                    FormFieldExportKind::File {
                        accept: vec!["audio/*".to_string()]
                    }
                );
            }
            _ => panic!("Expected FormData"),
        }
    }

    #[test]
    fn map_body_form_data_preserves_files_constraints() {
        let request = ApiRequest::FormData {
            fields: vec![
                schematic_define::request::FormField::files_with_constraints(
                    "samples",
                    vec!["audio/*".into()],
                    Some(1),
                    Some(10),
                ),
            ],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(
                    fields[0].kind,
                    FormFieldExportKind::Files {
                        accept: vec!["audio/*".to_string()],
                        min: Some(1),
                        max: Some(10),
                    }
                );
            }
            _ => panic!("Expected FormData"),
        }
    }

    #[test]
    fn map_body_form_data_preserves_text_for_text() {
        let request = ApiRequest::FormData {
            fields: vec![schematic_define::request::FormField::text("name")],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(fields[0].kind, FormFieldExportKind::Text);
            }
            _ => panic!("Expected FormData"),
        }
    }

    #[test]
    fn map_body_form_data_preserves_json_part() {
        let request = ApiRequest::FormData {
            fields: vec![schematic_define::request::FormField::json(
                "metadata",
                Schema::new("MetadataRequest"),
            )],
        };
        let body = map_body(&request);
        match body {
            ExportBody::FormData { fields } => {
                assert_eq!(fields[0].kind, FormFieldExportKind::Json);
            }
            _ => panic!("Expected FormData"),
        }
    }
}
