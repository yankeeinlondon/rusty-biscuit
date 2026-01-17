# Multipart Form-Data Support

Technical design document for adding multipart/form-data endpoint definitions to the `schematic-define` package.

## Overview

This document describes the design for extending `schematic-define` to support multipart form-data endpoints, enabling file uploads and mixed file/JSON payloads while maintaining consistency with existing REST and WebSocket primitives.

## Standards Reference

### RFC 2046: MIME Multipart (Foundation)

**Boundary Format:**

- Characters: `DIGIT / ALPHA / '()+-_,./:=?`
- Length: 1-70 characters
- Delimiter: `--` prefix + boundary value
- Closing: Trailing `--` after final boundary
- Position: Must appear at beginning of line after CRLF

**Message Structure:**

```
--boundary-value
Content-Disposition: form-data; name="field_name"
Content-Type: text/plain

field value
--boundary-value
Content-Disposition: form-data; name="file"; filename="document.pdf"
Content-Type: application/pdf

[binary PDF content]
--boundary-value--
```

### RFC 7578: Multipart Form-Data (HTTP Forms)

**Required Headers per Part:**

- `Content-Disposition: form-data; name="field_name"` (REQUIRED)
- `filename="original.txt"` parameter (recommended for files)
- `Content-Type` (defaults to `text/plain` if omitted)

**Key Constraints:**

- Field names: UTF-8 encoded
- Binary data: Transmitted as-is (no base64 encoding needed)
- Multiple files: Use separate parts with same `name` attribute

## Current Architecture Analysis

### Existing Request Definition

```rust
// types.rs
pub struct Endpoint {
    pub id: String,
    pub method: RestMethod,
    pub path: String,
    pub description: String,
    pub request: Option<Schema>,  // Currently JSON-only
    pub response: ApiResponse,
    pub headers: Vec<(String, String)>,
}
```

The `request` field currently holds an `Option<Schema>`, which references a typed struct for JSON serialization. This design assumes all request bodies are JSON.

### Existing Response Types

```rust
// response.rs
pub enum ApiResponse {
    Json(Schema),
    Text,
    Binary,
    Empty,
}
```

The response enum demonstrates the pattern for multiple content types. The request side should follow a similar enumerated approach.

## Design

### New Types

#### `FormField` - Individual Form Field Definition

```rust
/// Describes a single field in a multipart form.
#[derive(Debug, Clone, PartialEq, Eq)]
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
```

#### `FormFieldKind` - Field Type Classification

```rust
/// The type of content a form field accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// - `"*/*"` - Any file type (default if empty)
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
```

#### `ApiRequest` - Request Body Types

Introduce a new enum to replace `Option<Schema>` for request bodies:

```rust
/// Describes the request body format for an endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiRequest {
    /// JSON request body.
    ///
    /// Sets Content-Type to `application/json`.
    Json(Schema),

    /// Multipart form-data request.
    ///
    /// Sets Content-Type to `multipart/form-data` with auto-generated boundary.
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
```

### Updated Endpoint Definition

```rust
pub struct Endpoint {
    pub id: String,
    pub method: RestMethod,
    pub path: String,
    pub description: String,
    pub request: Option<ApiRequest>,  // CHANGED: Option<Schema> -> Option<ApiRequest>
    pub response: ApiResponse,
    pub headers: Vec<(String, String)>,
}
```

### Builder Helpers

```rust
impl FormField {
    /// Creates a required text field.
    pub fn text(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Text,
            required: true,
            description: None,
        }
    }

    /// Creates a required file field accepting any type.
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::File { accept: vec![] },
            required: true,
            description: None,
        }
    }

    /// Creates a file field with MIME type restrictions.
    pub fn file_accept(name: impl Into<String>, accept: Vec<String>) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::File { accept },
            required: true,
            description: None,
        }
    }

    /// Creates a JSON metadata field.
    pub fn json(name: impl Into<String>, schema: Schema) -> Self {
        Self {
            name: name.into(),
            kind: FormFieldKind::Json(schema),
            required: true,
            description: None,
        }
    }

    /// Makes the field optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Adds a description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl ApiRequest {
    /// Creates a JSON request.
    pub fn json(schema: Schema) -> Self {
        Self::Json(schema)
    }

    /// Creates a JSON request from a type name.
    pub fn json_type(type_name: impl Into<String>) -> Self {
        Self::Json(Schema::new(type_name))
    }

    /// Creates a multipart form-data request.
    pub fn form_data(fields: Vec<FormField>) -> Self {
        Self::FormData { fields }
    }

    /// Creates a URL-encoded form request.
    pub fn url_encoded(fields: Vec<FormField>) -> Self {
        Self::UrlEncoded { fields }
    }
}
```

## Usage Examples

### File Upload with Metadata

```rust
Endpoint {
    id: "AddVoiceSample".to_string(),
    method: RestMethod::Post,
    path: "/v1/voices/{voice_id}/samples".to_string(),
    description: "Upload audio sample for voice cloning".to_string(),
    request: Some(ApiRequest::form_data(vec![
        FormField::file_accept("audio", vec!["audio/*".into()])
            .with_description("Audio file (mp3, wav, ogg)"),
        FormField::text("name")
            .optional()
            .with_description("Sample name"),
        FormField::json("settings", Schema::new("SampleSettings"))
            .optional()
            .with_description("Processing settings"),
    ])),
    response: ApiResponse::json_type("AddSampleResponse"),
    headers: vec![],
}
```

### Multiple File Upload

```rust
Endpoint {
    id: "UploadDocuments".to_string(),
    method: RestMethod::Post,
    path: "/v1/documents/batch".to_string(),
    description: "Upload multiple documents".to_string(),
    request: Some(ApiRequest::form_data(vec![
        FormField {
            name: "files".into(),
            kind: FormFieldKind::Files {
                accept: vec!["application/pdf".into(), "text/*".into()],
                min: Some(1),
                max: Some(10),
            },
            required: true,
            description: Some("Documents to upload (1-10 files)".into()),
        },
        FormField::text("category")
            .with_description("Document category"),
    ])),
    response: ApiResponse::json_type("BatchUploadResponse"),
    headers: vec![],
}
```

### Mixed JSON and File (Common Pattern)

```rust
Endpoint {
    id: "CreateVoice".to_string(),
    method: RestMethod::Post,
    path: "/v1/voices/add".to_string(),
    description: "Create a new voice from samples".to_string(),
    request: Some(ApiRequest::form_data(vec![
        FormField::json("metadata", Schema::new("CreateVoiceRequest"))
            .with_description("Voice configuration"),
        FormField {
            name: "samples".into(),
            kind: FormFieldKind::Files {
                accept: vec!["audio/*".into()],
                min: Some(1),
                max: None,
            },
            required: true,
            description: Some("Voice training samples".into()),
        },
    ])),
    response: ApiResponse::json_type("Voice"),
    headers: vec![],
}
```

## Rust Crate Recommendations

### Server-Side Parsing: multer

```toml
[dependencies]
multer = "3.1"
```

**Why multer:**

- Mature, Tokio-native async parsing
- Streaming API for large files (no memory buffering)
- Size constraints for DoS prevention
- Native integration with Axum via `axum::extract::Multipart`

**Usage in generated handler:**

```rust
async fn handle_upload(mut multipart: Multipart) -> Result<Response> {
    let constraints = Constraints::new()
        .with_max_num_fields(10)
        .with_max_file_size(50 * 1024 * 1024); // 50MB

    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("audio") => {
                let data = field.bytes().await?;
                // Process audio file
            },
            Some("metadata") => {
                let json = field.text().await?;
                let meta: Metadata = serde_json::from_str(&json)?;
            },
            _ => {}
        }
    }
    Ok(Response::ok())
}
```

### Client-Side Building: reqwest

```toml
[dependencies]
reqwest = { version = "0.12", features = ["multipart"] }
```

**Usage in generated client:**

```rust
use reqwest::multipart::{Form, Part};

let form = Form::new()
    .part("audio", Part::bytes(audio_data)
        .file_name("sample.mp3")
        .mime_str("audio/mpeg")?)
    .text("name", sample_name);

client.post(url)
    .multipart(form)
    .send()
    .await?;
```

### Alternative: mpart-async (Streaming Focus)

For scenarios requiring streaming uploads without buffering:

```toml
[dependencies]
mpart-async = "0.7"
```

## OpenAPI Schema Generation

The definition types map to OpenAPI 3.x as follows:

```yaml
paths:
  /v1/voices/{voice_id}/samples:
    post:
      requestBody:
        required: true
        content:
          multipart/form-data:
            schema:
              type: object
              properties:
                audio:
                  type: string
                  format: binary
                  description: Audio file (mp3, wav, ogg)
                name:
                  type: string
                  description: Sample name
                settings:
                  $ref: '#/components/schemas/SampleSettings'
              required:
                - audio
            encoding:
              audio:
                contentType: audio/*
              settings:
                contentType: application/json
```

**Mapping Rules:**

| FormFieldKind | OpenAPI Type | Format | ContentType |
|---------------|--------------|--------|-------------|
| `Text` | `string` | - | `text/plain` |
| `File { accept }` | `string` | `binary` | From `accept` |
| `Files { accept, .. }` | `array` of `string` | `binary` | From `accept` |
| `Json(Schema)` | `$ref` to schema | - | `application/json` |

## Migration Path

### Phase 1: Add New Types (Non-Breaking)

1. Add `FormField`, `FormFieldKind`, and `ApiRequest` types
2. Add `request_body: Option<ApiRequest>` to `Endpoint` (deprecated: `request`)
3. Implement `From<Schema>` for `ApiRequest` for compatibility

### Phase 2: Update API Definitions

1. Migrate existing endpoints to use `ApiRequest::json()`
2. Add new multipart endpoints using `ApiRequest::form_data()`
3. Update code generator to handle new types

### Phase 3: Remove Deprecated Field

1. Remove `Endpoint::request` field
2. Update all consumers to use `Endpoint::request_body`

## Security Considerations

### Size Limits

Generated handlers MUST enforce size limits:

```rust
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50MB
const MAX_FIELDS: u32 = 20;
const MAX_TOTAL_SIZE: u64 = 100 * 1024 * 1024; // 100MB total
```

### Filename Sanitization

Generated code MUST sanitize filenames to prevent path traversal:

```rust
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect::<String>()
        .trim_start_matches('.')
        .to_string()
}
```

### Content-Type Validation

For endpoints with `accept` restrictions, validate using magic bytes, not just declared Content-Type:

```rust
fn validate_mime_type(data: &[u8], declared: &str, allowed: &[String]) -> bool {
    // Check magic bytes match declared type
    // Verify declared type matches allowed patterns
}
```

## Future Considerations

### Streaming Progress

Consider adding progress callback support for large uploads:

```rust
pub struct UploadProgress {
    pub bytes_sent: u64,
    pub total_bytes: Option<u64>,
    pub field_name: String,
}

type ProgressCallback = Box<dyn Fn(UploadProgress) + Send + Sync>;
```

### Chunked Uploads

For very large files, consider resumable upload support:

```rust
pub enum UploadStrategy {
    Standard,
    Chunked { chunk_size: usize },
    Resumable { session_id: String },
}
```

## References

- [RFC 2046: MIME Part Two](https://datatracker.ietf.org/doc/html/rfc2046)
- [RFC 7578: Returning Values from Forms: multipart/form-data](https://datatracker.ietf.org/doc/html/rfc7578)
- [multer crate](https://docs.rs/multer)
- [reqwest multipart](https://docs.rs/reqwest/latest/reqwest/multipart/index.html)
- [OpenAPI 3.x Multipart](https://swagger.io/docs/specification/describing-request-body/multipart-requests/)
