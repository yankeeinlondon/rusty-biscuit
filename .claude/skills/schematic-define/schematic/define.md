# schematic-define

REST and WebSocket API definition primitives for the Schematic code generation system.

## Core Types

### REST API Types

| Type | Purpose |
|------|---------|
| `RestApi` | Complete API definition with base URL, auth, endpoints, codegen options |
| `Endpoint` | Single endpoint with method, path, request/response schemas |
| `RestMethod` | HTTP methods: `Get`, `Post`, `Put`, `Patch`, `Delete`, `Head`, `Options` |
| `AuthStrategy` | Authentication: `BearerToken`, `ApiKey`, `ApiKeyParam`, `Basic`, `None` |
| `ApiRequest` | Request body: `Json`, `FormData`, `UrlEncoded`, `Text`, `Binary` |
| `ApiResponse` | Response type: `Json`, `Text`, `Binary`, `Empty` |
| `FormField` | Form field definition for multipart/URL-encoded requests |
| `FormFieldKind` | Field type: `Text`, `File`, `Files`, `Json` |
| `Schema` | Type name and optional module path for code generation |

### Header and Authentication Types

| Type | Purpose |
|------|---------|
| `Headers` | Fluent builder for HTTP headers with auth support |
| `SensitiveString` | Secure wrapper for passwords/tokens (redacts Debug output) |
| `EnvList` | Environment variable fallback chain for credentials |
| `ApiKeyEnv` | API key header configuration with env source |
| `EnvMapping` | Complete env var mapping for auth credentials |
| `HeaderError` | Errors from header validation and credential resolution |

### Model Definition Types (for OpenAPI import)

| Type | Purpose |
|------|---------|
| `ModelCatalog` | Collection of model definitions |
| `ModelDef` | Union: struct, enum, or type alias |
| `StructDef` | Structure definition with fields |
| `EnumDef` | Enumeration definition with variants |
| `TypeAlias` | Type alias definition |
| `FieldDef` | Field definition for structs |
| `EnumVariant` | Variant definition for enums |
| `TypeRef` | Type reference (primitives, arrays, named types) |

### Parameter Definition Types

| Type | Purpose |
|------|---------|
| `EndpointParams` | Collection of endpoint parameters (query, header, cookie) |
| `ParamDef` | Single parameter definition |
| `QueryParamType` | Parameter value type |
| `ParamStyle` | Parameter serialization style |

### WebSocket API Types

| Type | Purpose |
|------|---------|
| `WebSocketApi` | Complete WebSocket API definition |
| `WebSocketEndpoint` | Single endpoint with path, params, and message schemas |
| `ConnectionParam` | Query/path parameter (name, type, required, description) |
| `ParamType` | Parameter types: `String`, `Integer`, `Boolean`, `Float` |
| `ConnectionLifecycle` | Open, close, and keepalive message schemas |
| `MessageSchema` | Message type with direction and schema |
| `MessageDirection` | Flow direction: `Client`, `Server`, `Bidirectional` |

## RestApi Struct

```rust
pub struct RestApi {
    /// Unique identifier (becomes struct name: "OpenAI" → `struct OpenAI`)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Base URL for all endpoints
    pub base_url: String,

    /// Link to API documentation
    pub docs_url: Option<String>,

    /// Authentication strategy
    pub auth: AuthStrategy,

    /// Env var names for credentials (fallback chain)
    pub env_auth: Vec<String>,

    /// Env var for Basic auth username
    pub env_username: Option<String>,

    /// Default headers for all requests
    pub headers: Vec<(String, String)>,

    /// All endpoint definitions
    pub endpoints: Vec<Endpoint>,

    /// Custom module path (default: name.to_lowercase())
    pub module_path: Option<String>,

    /// Custom suffix for request structs (default: "Request")
    pub request_suffix: Option<String>,

    /// Environment variable mapping for flexible auth (optional)
    pub env_mapping: Option<EnvMapping>,

    /// Global query/header/cookie parameters (optional)
    pub params: Option<EndpointParams>,
}
```

## Endpoint Struct

```rust
pub struct Endpoint {
    /// Identifier (becomes enum variant and struct name)
    pub id: String,

    /// HTTP method
    pub method: RestMethod,

    /// Path template with {param} placeholders
    pub path: String,

    /// Human-readable description
    pub description: String,

    /// Request body definition (None for GET/DELETE typically)
    pub request: Option<ApiRequest>,

    /// Expected response type
    pub response: ApiResponse,

    /// Endpoint-specific headers (merged with API headers)
    pub headers: Vec<(String, String)>,

    /// Query/header/cookie parameters (optional)
    pub params: Option<EndpointParams>,
}
```

## AuthStrategy Enum

```rust
pub enum AuthStrategy {
    /// No authentication required
    None,

    /// Bearer token in Authorization header
    /// header: custom header name (None = "Authorization")
    BearerToken { header: Option<String> },

    /// API key in custom header
    ApiKey { header: String },

    /// HTTP Basic authentication
    Basic,

    /// API key in query parameter or cookie
    ApiKeyParam { location: ApiKeyLocation, name: String },
}

pub enum ApiKeyLocation {
    Query,
    Cookie,
}
```

## ApiRequest Enum

```rust
pub enum ApiRequest {
    /// JSON body with typed schema
    Json(Schema),

    /// Multipart form-data
    FormData { fields: Vec<FormField> },

    /// URL-encoded form
    UrlEncoded { fields: Vec<FormField> },

    /// Raw text body
    Text { content_type: String },

    /// Raw binary body
    Binary { content_type: String },
}

// Convenience constructors
impl ApiRequest {
    pub fn json_type(name: &str) -> Self;
    pub fn json(schema: Schema) -> Self;
    pub fn form_data(fields: Vec<FormField>) -> Self;
    pub fn url_encoded(fields: Vec<FormField>) -> Self;
    pub fn text(content_type: &str) -> Self;
    pub fn binary(content_type: &str) -> Self;
}
```

## ApiResponse Enum

```rust
pub enum ApiResponse {
    /// JSON response with typed schema
    Json(Schema),

    /// Plain text response
    Text,

    /// Binary data (files, images, audio)
    Binary,

    /// No content (204 responses)
    Empty,
}

// Convenience constructors
impl ApiResponse {
    pub fn json_type(name: &str) -> Self;
    pub fn json(schema: Schema) -> Self;
}
```

## FormField Builders

```rust
// Required text field
let name = FormField::text("name");

// Optional text field with description
let bio = FormField::text("bio")
    .optional()
    .with_description("User biography");

// File upload accepting any type
let doc = FormField::file("document");

// File upload with MIME restrictions
let image = FormField::file_accept("avatar", vec!["image/*".into()]);

// Multiple files with constraints
let samples = FormField::files_with_constraints(
    "audio_samples",
    vec!["audio/*".into()],
    Some(1),   // min
    Some(10),  // max
);

// JSON metadata field
let meta = FormField::json("metadata", Schema::new("FileMetadata"))
    .optional();
```

## Schema Type

```rust
pub struct Schema {
    pub type_name: String,
    pub module_path: Option<String>,
}

impl Schema {
    /// Type in current scope
    pub fn new(type_name: &str) -> Self;

    /// Type in specific module
    pub fn with_path(type_name: &str, module: &str) -> Self;

    /// Returns fully qualified path
    pub fn full_path(&self) -> String;
}

// Usage
let simple = Schema::new("User");                           // full_path() → "User"
let qualified = Schema::with_path("User", "crate::models"); // full_path() → "crate::models::User"
```

## Headers Builder

See [Headers Builder](./headers.md) for complete documentation.

```rust
use schematic_define::Headers;

// Bearer token auth
let headers = Headers::default()
    .use_bearer_token("my-token")
    .accept_json()
    .build()?;

// Basic auth
let headers = Headers::default()
    .use_basic_auth("user", "pass")
    .build()?;

// API key in custom header
let headers = Headers::default()
    .use_api_key("my-key", "X-API-Key")
    .content_type_json()
    .build()?;

// Load from environment
let mapping = EnvMapping {
    bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
    basic_user: None,
    basic_pass: None,
    api_key: None,
};
let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env();
```

## Prelude

```rust
use schematic_define::prelude::*;

// Exports all core types:
// REST: RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse,
//       FormField, FormFieldKind, Schema, Headers, EnvList, EnvMapping
// WebSocket: WebSocketApi, WebSocketEndpoint, ConnectionParam, ParamType,
//            ConnectionLifecycle, MessageSchema, MessageDirection
// Models: ModelCatalog, ModelDef, StructDef, EnumDef, TypeRef
// Params: EndpointParams, ParamDef, QueryParamType
```
