//! Normalized request body for export.

use schematic_define::request::ApiRequest;

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
                assert_eq!(fields[1].name, "avatar");
                assert!(!fields[1].required);
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
            }
            _ => panic!("Expected UrlEncoded"),
        }
    }
}
