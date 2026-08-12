# schematic-define

REST and WebSocket API definition types for the Schematic code generation system.

## Overview

`schematic-define` provides a declarative way to describe REST and WebSocket APIs. These definitions are consumed by `schematic-gen` to generate strongly-typed Rust client code with automatic authentication, request serialization, and response deserialization.

The definition process is intentionally **data-driven**: you describe *what* the API looks like (endpoints, methods, schemas) rather than *how* to call it. The generator handles the implementation details.

## Core Types

### REST API Types

| Type | Purpose |
|------|---------|
| `RestApi` | Complete API definition with base URL, auth, endpoints, and codegen options |
| `Endpoint` | Single endpoint with method, path, request/response schemas |
| `RestMethod` | HTTP methods (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS); supports `FromStr`, `TryFrom<String>` |
| `AuthStrategy` | Authentication configuration (Bearer, API Key header/query/cookie, Basic, OAuth2, None) |
| `AuthPolicy` | Explicit auth methods accepted by a client plus its env-fallback strategy |
| `AuthMethod` | Explicit runtime auth methods (Bearer, API Key, Basic, OAuth2 token) |
| `EnvAuthStrategy` | Environment-backed auth fallback shape for generated REST clients |
| `ApiKeyLocation` | Location for API key auth (Query, Cookie) |
| `OAuth2Config` | OAuth2 provider configuration (endpoints, grant type, PKCE) |
| `OAuth2GrantType` | OAuth2 grant type (AuthorizationCodePkce, ClientCredentials, DeviceCode) |
| `PkceRequirement` | PKCE requirement level (Required, Supported, NotUsed) |
| `OAuth2ClientAuthMethod` | Client credential delivery method (ClientSecretBasic, ClientSecretPost, None) |
| `UpdateStrategy` | Strategy for updating auth in API variants (NoChange, ChangeTo) |
| `ApiRequest` | Request body type (JSON, FormData, UrlEncoded, Text, Binary) |
| `ApiResponse` | Response type (JSON, Text, Binary, Empty) |
| `FormField` | Form field definition for multipart/URL-encoded requests |
| `FormFieldKind` | Form field type (Text, File, Files, Json) |
| `Schema` | Type name and optional module path for code generation |
| `SchemaObject` | Trait bound for serializable/deserializable types |

### Header and Authentication Types

| Type | Purpose |
|------|---------|
| `Headers` | Fluent builder for HTTP headers with auth support |
| `SensitiveString` | Secure wrapper for passwords/tokens (redacts Debug output) |
| `EnvList` | Environment variable fallback chain for credentials |
| `ApiKeyEnv` | API key header configuration with environment source |
| `EnvMapping` | Complete environment variable mapping for auth credentials |
| `HeaderError` | Errors from header validation and credential resolution |

### Model Definition Types (for API schema import)

| Type | Purpose |
|------|---------|
| `ModelCatalog` | Collection of model definitions with optional module path |
| `ModelDef` | Union of model types (struct, enum, or alias) |
| `StructDef` | Structure definition with fields |
| `EnumDef` | Enumeration definition with variants |
| `TypeAlias` | Type alias definition |
| `FieldDef` | Field definition for structs |
| `EnumVariant` | Variant definition for enums |
| `TypeRef` | Type reference (primitives, arrays, named types, combinators) |
| `PrimitiveType` | Basic primitive types |

### Parameter Definition Types (for endpoint import)

| Type | Purpose |
|------|---------|
| `EndpointParams` | Collection of endpoint parameters (query, header, cookie) |
| `ParamDef` | Single parameter definition |
| `QueryParamType` | Parameter value type |
| `ParamStyle` | Parameter serialization style |
| `PaginationStyle` | Common pagination request patterns |
| `PaginationResponse` | How APIs signal pagination state in responses |

### WebSocket API Types

| Type | Purpose |
|------|---------|
| `WebSocketApi` | Complete WebSocket API definition with base URL, auth, and endpoints |
| `WebSocketEndpoint` | Single WebSocket endpoint with path, parameters, and message schemas |
| `ConnectionParam` | Query/path parameter definition for WebSocket connections |
| `ParamType` | Parameter types (String, Integer, Boolean, Float) |
| `ConnectionLifecycle` | Open, close, and keepalive message schemas |
| `MessageSchema` | Single message type with direction (Client, Server, Bidirectional) |
| `MessageDirection` | Message flow direction enumeration |

## Definition Workflow

```
┌─────────────────────┐
│   Define RestApi     │
│  - name, base_url   │
│  - auth strategy    │
│  - endpoints[]      │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  schematic-gen      │
│  (code generator)   │
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Generated Client   │
│  - Type-safe API    │
│  - Auto auth        │
│  - Serialization    │
└─────────────────────┘
```

## Authentication Strategies

Authentication can be modeled in two ways:

1. `RestApi::auth` with legacy `env_auth` / `env_username` fields for backward-compatible single-strategy auth
2. `RestApi::auth_policy` plus `EnvMapping` for explicit multi-method auth and authoritative env fallback

Generated clients always apply this precedence:

1. Explicit auth already attached to `Headers`
2. Explicit auth injected with generated helpers such as `.api_key(...)`, `.bearer_token(...)`, `.basic_auth(...)`, or `.oauth_token(...)`
3. Environment-variable fallback from `EnvMapping`
4. `SchematicError::AuthenticationRequired`

Legacy `env_auth` and `env_username` are still supported, but the generator normalizes them into `EnvMapping` before runtime auth resolution.

### Bearer Token (Most Common)

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::BearerToken { header: None }, // Uses "Authorization" header
    env_auth: vec!["OPENAI_API_KEY".to_string()],    // Env var(s) to check
    // ...
};
```

Generates: `Authorization: Bearer <token>`

Multiple env vars can be specified as a fallback chain - the first one found is used:

```rust
env_auth: vec!["OPENAI_API_KEY".to_string(), "OPENAI_KEY".to_string()],
```

### API Key in Custom Header

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::ApiKey { header: "X-API-Key".to_string() },
    env_auth: vec!["MY_API_KEY".to_string()],
    // ...
};
```

Generates: `X-API-Key: <key>`

### Basic Authentication

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::Basic,
    env_username: Some("SERVICE_USER".to_string()),
    env_auth: vec!["SERVICE_PASSWORD".to_string()], // Password from env_auth[0]
    // ...
};
```

Generates: `Authorization: Basic <base64(user:pass)>`

### No Authentication

```rust
use schematic_define::{RestApi, AuthStrategy};

let api = RestApi {
    auth: AuthStrategy::None,
    env_auth: vec![],
    // ...
};
```

### API Key in Query Parameter or Cookie

For APIs that pass the API key in the query string or as a cookie rather than a header:

```rust
use schematic_define::{RestApi, AuthStrategy, ApiKeyLocation};

let api = RestApi {
    auth: AuthStrategy::ApiKeyParam {
        name: "api_key".to_string(),
        location: ApiKeyLocation::Query,  // or ApiKeyLocation::Cookie
    },
    env_auth: vec!["MY_API_KEY".to_string()],
    // ...
};
```

### OAuth2 Authentication

For APIs that use OAuth2, configure the provider details declaratively:

```rust
use schematic_define::{RestApi, AuthStrategy, OAuth2Config, OAuth2GrantType, PkceRequirement, OAuth2ClientAuthMethod};

let api = RestApi {
    auth: AuthStrategy::OAuth2(OAuth2Config {
        grant_type: OAuth2GrantType::AuthorizationCodePkce,
        authorization_url: Some("https://github.com/login/oauth/authorize".into()),
        token_url: "https://github.com/login/oauth/access_token".into(),
        revocation_url: None,
        device_authorization_url: None,
        default_scopes: vec!["repo".into(), "read:user".into()],
        pkce: PkceRequirement::Required,
        client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
    }),
    env_auth: vec![],
    env_username: None,
    // ...
};
```

OAuth2 token lifecycle is managed by the `schematic-oauth` runtime crate. The `OAuth2Config` here is declarative metadata describing the provider. Generated clients surface `SchematicError::AuthenticationRequired` when no acceptable credential is available and, for OAuth-enabled clients, point callers to `schematic-oauth` before injecting the token with `.oauth_token(...)` or an explicit `Headers` builder.

### Multi-Method Auth Policy

Use `auth_policy` when one client should accept more than one explicit auth method while keeping a single env fallback source:

```rust
use schematic_define::{
    AuthMethod, AuthPolicy, AuthStrategy, EnvAuthStrategy, OAuth2ClientAuthMethod,
    OAuth2Config, OAuth2GrantType, PkceRequirement, RestApi,
};

let api = RestApi {
    auth: AuthStrategy::ApiKey {
        header: "PRIVATE-TOKEN".to_string(),
    },
    auth_policy: Some(AuthPolicy {
        explicit: vec![
            AuthMethod::ApiKey {
                header: "PRIVATE-TOKEN".to_string(),
            },
            AuthMethod::OAuth2(OAuth2Config {
                grant_type: OAuth2GrantType::AuthorizationCodePkce,
                authorization_url: Some("https://example.com/oauth/authorize".into()),
                token_url: "https://example.com/oauth/token".into(),
                revocation_url: None,
                device_authorization_url: None,
                default_scopes: vec!["read_api".into()],
                pkce: PkceRequirement::Required,
                client_auth: OAuth2ClientAuthMethod::ClientSecretPost,
            }),
        ],
        env_fallback: Some(EnvAuthStrategy::ApiKey {
            header: "PRIVATE-TOKEN".to_string(),
        }),
    }),
    env_auth: vec!["GITLAB_TOKEN".to_string(), "GITLAB_PRIVATE_TOKEN".to_string()],
    // ...
    # name: "Example".to_string(),
    # description: "Example".to_string(),
    # base_url: "https://example.com/api".to_string(),
    # docs_url: None,
    # env_username: None,
    # headers: vec![],
    # endpoints: vec![],
    # module_path: None,
    # request_suffix: None,
    # env_mapping: None,
};
```

### Endpoint-Level OAuth2 Scopes

Individual endpoints can specify OAuth2 scopes that override the API-level defaults:

```rust
use schematic_define::{Endpoint, RestMethod, ApiResponse};

Endpoint {
    id: "ListRepos".to_string(),
    method: RestMethod::Get,
    path: "/repos".to_string(),
    description: "List repositories".to_string(),
    request: None,
    response: ApiResponse::json_type("ListReposResponse"),
    headers: vec![],
    params: None,
    oauth_scopes: Some(vec!["repo".into(), "read:org".into()]),
};
```

If `oauth_scopes` is `None`, the API-level `default_scopes` from `OAuth2Config` are used.

### Missing Credentials

If neither explicit auth nor environment fallback can satisfy the API requirements, the generated code returns `SchematicError::AuthenticationRequired`:

```rust
Err(SchematicError::AuthenticationRequired {
    message: "Authentication required: an explicit API key in `PRIVATE-TOKEN`, an explicit OAuth access token or set one of the fallback env vars `GITLAB_TOKEN`, `GITLAB_PRIVATE_TOKEN`.".to_string(),
    explicit_methods: vec![
        "an explicit API key in `PRIVATE-TOKEN`".to_string(),
        "an explicit OAuth access token".to_string(),
    ],
    env_fallback_vars: vec![
        "GITLAB_TOKEN".to_string(),
        "GITLAB_PRIVATE_TOKEN".to_string(),
    ],
})
```

### Programmatic Authentication

To inject tokens programmatically (without environment variables), use the `Headers` builder:

```rust
use schematic_define::Headers;

// Token from runtime source (Vault, OAuth, config file, etc.)
let token = get_token_from_somewhere();

// Create client with programmatic token - no env vars needed!
let client = OpenAI::new()
    .variant()
    .headers_builder(Headers::default().use_bearer_token(token))
    .build();

// Or with basic auth
let client = SomeApi::new()
    .variant()
    .headers_builder(Headers::default().use_basic_auth("user", "pass"))
    .build();
```

When `Headers` has explicit auth set via `use_bearer_token()`, `use_basic_auth()`, or `use_api_key()`, env-based auth fallback is automatically skipped. The generated code checks `headers.has_explicit_auth()` so explicit API keys now win over env fallback the same way bearer/basic auth already did.

This is useful for:

- **Multi-tenant applications**: Different credentials per tenant
- **Token rotation**: Refresh tokens from a vault at runtime
- **Testing**: Inject mock credentials without setting env vars
- **OAuth flows**: Use tokens obtained from OAuth providers

### UpdateStrategy

When creating API variants with the generated `variant()` method, use `UpdateStrategy` to control auth:

```rust
use schematic_define::{AuthStrategy, UpdateStrategy};

// Keep existing auth strategy
let strategy = UpdateStrategy::NoChange;

// Change to a different auth strategy
let strategy = UpdateStrategy::ChangeTo(AuthStrategy::ApiKey {
    header: "X-API-Key".to_string(),
});
```

## Headers Builder

The `Headers` type provides a fluent API for building HTTP headers:

```rust
use schematic_define::Headers;

// Bearer token authentication
let headers = Headers::default()
    .use_bearer_token("my-secret-token")
    .accept_json()
    .build()
    .unwrap();

// Basic authentication
let headers = Headers::default()
    .use_basic_auth("username", "password")
    .build()
    .unwrap();

// Custom headers
let headers = Headers::default()
    .header("X-API-Key", "my-key")
    .user_agent("MyClient/1.0")
    .build()
    .unwrap();

// Check whether any explicit auth has been attached
assert!(headers.has_explicit_auth());
```

### Headers Methods

| Method | Description |
|--------|-------------|
| `use_bearer_token(token)` | Set Bearer token authentication |
| `use_basic_auth(user, pass)` | Set Basic authentication |
| `use_api_key(key, header)` | Set API key in custom header |
| `content_type(ct)` | Set Content-Type header |
| `accept(accept)` | Set Accept header |
| `accept_json()` | Set Accept: application/json |
| `content_type_json()` | Set Content-Type: application/json |
| `user_agent(agent)` | Set User-Agent header |
| `header(name, value)` | Add custom header |
| `remove(name)` | Remove a header |
| `with_env_mapping(mapping)` | Configure env var mapping |
| `from_env()` | Load credentials from environment (permissive) |
| `try_from_env()` | Load credentials from environment (strict) |
| `has_authorization()` | Check whether the `Authorization` header is set |
| `has_explicit_auth()` | Check if any explicit auth is set |

### Environment Variable Loading

The `Headers` builder can load credentials from environment variables:

```rust
use schematic_define::{Headers, EnvMapping, EnvList};

let mapping = EnvMapping {
    bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
    basic_user: None,
    basic_pass: None,
    api_key: None,
};

let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env();
```

### SensitiveString

For secure handling of sensitive values:

```rust
use schematic_define::SensitiveString;

let secret = SensitiveString::from("my-secret-token");
// Debug output is redacted
assert_eq!(format!("{:?}", secret), "SensitiveString(\"***\")");
// Access the actual value
assert_eq!(secret.as_str(), "my-secret-token");
```

## Code Generation Options

`RestApi` includes optional fields to customize generated code:

### Module Path

By default, the generated module uses the lowercased API name. Override with `module_path`:

```rust
let api = RestApi {
    name: "OllamaOpenAI".to_string(),
    module_path: Some("ollama".to_string()),  // Use "ollama" instead of "ollamaopenai"
    // ...
};
```

This is useful when:

- Multiple APIs share a definitions module (e.g., `OllamaNative` and `OllamaOpenAI` both in `ollama/`)
- The API name doesn't match the desired module name

### Request Suffix

By default, generated wrapper structs use the "Request" suffix (e.g., `ListModelsRequest`). Customize with `request_suffix`:

```rust
let api = RestApi {
    name: "MyApi".to_string(),
    request_suffix: Some("Req".to_string()),  // Use "Req" instead of "Request"
    // ...
};
// Generates: ListModelsReq, CreateUserReq, etc.
```

**Note**: The suffix must be alphanumeric. Invalid suffixes (containing spaces, hyphens, etc.) will cause a validation error.

### Environment Variable Mapping

For more control over authentication credentials, use `env_mapping`:

```rust
use schematic_define::{RestApi, AuthStrategy, EnvMapping, EnvList};

let api = RestApi {
    name: "MyApi".to_string(),
    // Use structured env mapping instead of legacy env_auth/env_username
    env_mapping: Some(EnvMapping {
        bearer_token: Some(EnvList::from_strs(&["API_KEY", "TOKEN", "KEY"])),
        basic_user: Some(EnvList::single("SERVICE_USER")),
        basic_pass: Some(EnvList::single("SERVICE_PASS")),
        api_key: None,
        ..Default::default()
    }),
    // Legacy fields still work for backward compatibility
    env_auth: vec![],
    env_username: None,
    // ...
};
```

The `EnvMapping` struct also includes OAuth2-specific fields (`oauth_client_id`, `oauth_client_secret`, `oauth_redirect_uri`) which default to `None`.

The `RestApi::default_env_mapping()` method returns an `EnvMapping` built from:
1. Explicit `env_mapping` if set
2. Legacy `env_auth` / `env_username` fields otherwise

## Request Types

Endpoints can accept different request body formats via `ApiRequest`:

| Variant | Content-Type | Use Case |
|---------|-------------|----------|
| `ApiRequest::Json(schema)` | `application/json` | Most API requests |
| `ApiRequest::FormData { fields }` | `multipart/form-data` | File uploads, mixed data |
| `ApiRequest::UrlEncoded { fields }` | `application/x-www-form-urlencoded` | Simple form data |
| `ApiRequest::Text { content_type }` | Custom text MIME | Raw text bodies |
| `ApiRequest::Binary { content_type }` | Custom binary MIME | Raw binary bodies |

### Form Fields

`FormField` describes individual fields in multipart or URL-encoded forms:

```rust
use schematic_define::{ApiRequest, FormField, FormFieldKind, Schema};

// File upload with optional metadata
let request = ApiRequest::form_data(vec![
    FormField::file_accept("audio", vec!["audio/*".into()])
        .with_description("Audio file (mp3, wav, ogg)"),
    FormField::text("name")
        .optional()
        .with_description("Optional name for the file"),
    FormField::json("metadata", Schema::new("FileMetadata"))
        .optional(),
]);
```

### FormFieldKind Variants

| Kind | Description |
|------|-------------|
| `Text` | Plain text field |
| `File { accept }` | Single file with optional MIME restrictions |
| `Files { accept, min, max }` | Multiple files with optional count constraints |
| `Json(Schema)` | Embedded JSON data |

### Builder Methods

`FormField` provides convenient builders:

```rust
use schematic_define::FormField;

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
```

## Enum Extensibility

All public enums in `schematic-define` are marked `#[non_exhaustive]`, allowing new variants to be added in future versions without breaking downstream code. Match statements on these enums must include a wildcard arm:

```rust
use schematic_define::AuthStrategy;

let auth = AuthStrategy::BearerToken { header: None };
match auth {
    AuthStrategy::None => { /* ... */ }
    AuthStrategy::BearerToken { header } => { /* ... */ }
    AuthStrategy::ApiKey { header } => { /* ... */ }
    AuthStrategy::ApiKeyParam { name, location } => { /* ... */ }
    AuthStrategy::Basic => { /* ... */ }
    AuthStrategy::OAuth2(config) => { /* ... */ }
    _ => { /* future variants */ }
}
```

This applies to: `AuthStrategy`, `ApiKeyLocation`, `UpdateStrategy`, `ApiRequest`, `FormFieldKind`, `ApiResponse`, `RestMethod`, `QueryParamType`, `ParamStyle`, `PaginationStyle`, `PaginationResponse`, `MessageDirection`, `ParamType`, `OAuth2GrantType`, `PkceRequirement`, `OAuth2ClientAuthMethod`.

## Response Types

| Variant | Generated Return Type | Use Case |
|---------|----------------------|----------|
| `ApiResponse::Json(schema)` | Deserialized struct | Most API responses |
| `ApiResponse::Text` | `String` | Plain text endpoints |
| `ApiResponse::Binary` | `bytes::Bytes` | File downloads, images |
| `ApiResponse::Empty` | `()` | DELETE, 204 responses |

### ApiResponse Methods

The `ApiResponse` type provides convenience methods:

```rust
use schematic_define::ApiResponse;

// Check response type
let response = ApiResponse::json_type("User");
assert!(response.is_json());

// Create vec response for list endpoints
let list_response = ApiResponse::json_vec_type("ModelInfo");
// Equivalent to: ApiResponse::json_type("Vec<ModelInfo>")
```

## WebSocket APIs

WebSocket APIs use a parallel type system that shares authentication strategies with REST APIs but provides WebSocket-specific concepts like connection parameters, message direction, and lifecycle management.

### WebSocket Example: ElevenLabs Text-to-Speech

```rust
use schematic_define::{
    WebSocketApi, WebSocketEndpoint, ConnectionParam, ParamType,
    ConnectionLifecycle, MessageSchema, MessageDirection,
    AuthStrategy, Schema
};

let api = WebSocketApi {
    name: "ElevenLabsTTS".to_string(),
    description: "ElevenLabs Text-to-Speech WebSocket API".to_string(),
    base_url: "wss://api.elevenlabs.io/v1".to_string(),
    docs_url: Some("https://elevenlabs.io/docs/api-reference/websockets".to_string()),
    auth: AuthStrategy::ApiKey { header: "xi-api-key".to_string() },
    env_auth: vec!["ELEVEN_LABS_API_KEY".to_string()],
    endpoints: vec![
        WebSocketEndpoint {
            id: "TextToSpeech".to_string(),
            path: "/text-to-speech/{voice_id}/stream-input".to_string(),
            description: "Stream text and receive audio chunks".to_string(),
            connection_params: vec![
                ConnectionParam {
                    name: "model_id".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    description: Some("Model to use for synthesis".to_string()),
                },
                ConnectionParam {
                    name: "output_format".to_string(),
                    param_type: ParamType::String,
                    required: false,
                    description: Some("Audio output format".to_string()),
                },
            ],
            lifecycle: ConnectionLifecycle {
                open: Some(MessageSchema {
                    name: "BOS".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("BeginOfStreamMessage"),
                    description: Some("Begin-of-stream message".to_string()),
                }),
                close: Some(MessageSchema {
                    name: "EOS".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("EndOfStreamMessage"),
                    description: Some("End-of-stream signal".to_string()),
                }),
                keepalive: None,
            },
            messages: vec![
                MessageSchema {
                    name: "TextChunk".to_string(),
                    direction: MessageDirection::Client,
                    schema: Schema::new("TextChunkMessage"),
                    description: Some("Text to synthesize".to_string()),
                },
                MessageSchema {
                    name: "AudioChunk".to_string(),
                    direction: MessageDirection::Server,
                    schema: Schema::new("AudioChunkResponse"),
                    description: Some("Audio data chunk".to_string()),
                },
            ],
        },
    ],
};
```

### Message Direction

WebSocket messages have a direction indicating their flow:

| Direction | Description |
|-----------|-------------|
| `Client` | Sent from client to server |
| `Server` | Sent from server to client |
| `Bidirectional` | Can flow in either direction |

### Connection Lifecycle

WebSocket connections can define special lifecycle messages:

- **open**: Message sent immediately after connection (e.g., initialization/config)
- **close**: Message sent before graceful disconnection
- **keepalive**: Heartbeat message to maintain connection

## REST API Examples

### Example 1: Simple Public API (No Auth)

A basic health-check API with no authentication:

```rust
use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};

let api = RestApi {
    name: "HealthService".to_string(),
    description: "Simple health monitoring service".to_string(),
    base_url: "https://api.example.com".to_string(),
    docs_url: None,
    auth: AuthStrategy::None,
    env_auth: vec![],
    env_username: None,
    endpoints: vec![
        Endpoint {
            id: "GetHealth".to_string(),
            method: RestMethod::Get,
            path: "/health".to_string(),
            description: "Check service health".to_string(),
            request: None,
            response: ApiResponse::json_type("HealthStatus"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetVersion".to_string(),
            method: RestMethod::Get,
            path: "/version".to_string(),
            description: "Get service version".to_string(),
            request: None,
            response: ApiResponse::Text,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ],
};
```

### Example 2: REST API with Bearer Token Auth

A user management API with CRUD operations:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse
};

let api = RestApi {
    name: "UserService".to_string(),
    description: "User management REST API".to_string(),
    base_url: "https://api.myservice.com/v1".to_string(),
    docs_url: Some("https://docs.myservice.com/api".to_string()),
    auth: AuthStrategy::BearerToken { header: None },
    env_auth: vec!["MYSERVICE_API_KEY".to_string()],
    env_username: None,
    headers: vec![],
    endpoints: vec![
        // List all users
        Endpoint {
            id: "ListUsers".to_string(),
            method: RestMethod::Get,
            path: "/users".to_string(),
            description: "List all users".to_string(),
            request: None,
            response: ApiResponse::json_type("ListUsersResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Get a specific user by ID (path parameter)
        Endpoint {
            id: "GetUser".to_string(),
            method: RestMethod::Get,
            path: "/users/{user_id}".to_string(),
            description: "Retrieve a user by ID".to_string(),
            request: None,
            response: ApiResponse::json_type("User"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Create a new user (with JSON request body)
        Endpoint {
            id: "CreateUser".to_string(),
            method: RestMethod::Post,
            path: "/users".to_string(),
            description: "Create a new user".to_string(),
            request: Some(ApiRequest::json_type("CreateUserRequest")),
            response: ApiResponse::json_type("User"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Update a user
        Endpoint {
            id: "UpdateUser".to_string(),
            method: RestMethod::Put,
            path: "/users/{user_id}".to_string(),
            description: "Update an existing user".to_string(),
            request: Some(ApiRequest::json_type("UpdateUserRequest")),
            response: ApiResponse::json_type("User"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Delete a user
        Endpoint {
            id: "DeleteUser".to_string(),
            method: RestMethod::Delete,
            path: "/users/{user_id}".to_string(),
            description: "Delete a user".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ],
};
```

### Example 3: File Storage API with File Uploads

A file storage API demonstrating multipart form-data uploads:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse, FormField
};

let api = RestApi {
    name: "FileStorage".to_string(),
    description: "Cloud file storage API".to_string(),
    base_url: "https://storage.example.com/api/v2".to_string(),
    docs_url: Some("https://storage.example.com/docs".to_string()),
    auth: AuthStrategy::ApiKey { header: "X-Storage-Key".to_string() },
    env_auth: vec!["STORAGE_API_KEY".to_string()],
    env_username: None,
    headers: vec![],
    endpoints: vec![
        // List files - returns JSON
        Endpoint {
            id: "ListFiles".to_string(),
            method: RestMethod::Get,
            path: "/files".to_string(),
            description: "List all files in storage".to_string(),
            request: None,
            response: ApiResponse::json_type("FileList"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Upload file - multipart form-data
        Endpoint {
            id: "UploadFile".to_string(),
            method: RestMethod::Post,
            path: "/files".to_string(),
            description: "Upload a new file".to_string(),
            request: Some(ApiRequest::form_data(vec![
                FormField::file("file").with_description("The file to upload"),
                FormField::text("folder").optional().with_description("Target folder"),
                FormField::text("description").optional(),
            ])),
            response: ApiResponse::json_type("FileMetadata"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Download file - returns binary data
        Endpoint {
            id: "DownloadFile".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}/content".to_string(),
            description: "Download file contents".to_string(),
            request: None,
            response: ApiResponse::Binary,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Get file metadata - returns JSON
        Endpoint {
            id: "GetFileMetadata".to_string(),
            method: RestMethod::Get,
            path: "/files/{file_id}".to_string(),
            description: "Get file metadata".to_string(),
            request: None,
            response: ApiResponse::json_type("FileMetadata"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Delete file - returns empty
        Endpoint {
            id: "DeleteFile".to_string(),
            method: RestMethod::Delete,
            path: "/files/{file_id}".to_string(),
            description: "Delete a file".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ],
};
```

## Path Parameters

Paths support template parameters using curly braces. These become fields in the generated request struct:

```rust
// Path: "/users/{user_id}/posts/{post_id}"
// Generated code will require both `user_id` and `post_id` parameters
```

## Endpoint Parameters

For endpoints imported from OpenAPI, you can define query, header, and cookie parameters:

```rust
use schematic_define::params::{EndpointParams, QueryParamType, ParamStyle};

let params = EndpointParams::default()
    .with_query_param("state", QueryParamType::String, false, Some("Filter by state"))
    .with_query_param("limit", QueryParamType::Integer, false, Some("Max results"));
```

### Endpoint with Pagination

```rust
use schematic_define::{
    Endpoint, RestMethod, ApiResponse, ApiRequest,
    params::{EndpointParams, PaginationStyle, PaginationResponse}
};

Endpoint {
    id: "ListUsers".to_string(),
    method: RestMethod::Get,
    path: "/users".to_string(),
    description: "List all users with pagination".to_string(),
    request: None,
    response: ApiResponse::json_vec_type("User"),
    headers: vec![],
    params: Some(EndpointParams::default()
        .with_pagination(PaginationStyle::github())
        .with_response_pagination(PaginationResponse::LinkHeader)),
    oauth_scopes: None,
}
```

### Pagination

`PaginationStyle` provides standardized pagination patterns:

```rust
use schematic_define::params::{EndpointParams, PaginationStyle, PaginationResponse};

// GitHub-style pagination (page + per_page)
let params = EndpointParams::default()
    .with_pagination(PaginationStyle::github())
    .with_response_pagination(PaginationResponse::LinkHeader);

// Cursor-based pagination
let params = EndpointParams::default()
    .with_pagination(PaginationStyle::cursor("after", Some("limit"), 20));

// Offset/limit pagination
let params = EndpointParams::default()
    .with_pagination(PaginationStyle::offset_limit("offset", "limit", 20, 100));
```

### QueryParamType Variants

| Variant | Description |
|---------|-------------|
| `String` | UTF-8 string |
| `Integer` | 64-bit signed integer |
| `Number` | 64-bit floating-point |
| `Boolean` | true/false |
| `Array(Box<QueryParamType>)` | Array of inner type |
| `Enum(Vec<String>)` | Fixed set of allowed values |
| `Json` | Arbitrary JSON value |

### ParamStyle Variants

| Variant | Description |
|---------|-------------|
| `Form` | Default for query params (e.g., `tags=a,b` or `tags=a&tags=b`) |
| `Simple` | Comma-separated (default for path/header) |
| `SpaceDelimited` | Space-separated |
| `PipeDelimited` | Pipe-separated |
| `DeepObject` | Nested objects (`filter[name]=value`) |

## Prelude

For convenient imports, use the prelude:

```rust
use schematic_define::prelude::*;

// Now you have access to all core types:
// REST: RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse,
//       FormField, FormFieldKind, Schema, OAuth2Config, OAuth2GrantType,
//       PkceRequirement, OAuth2ClientAuthMethod
// WebSocket: WebSocketApi, WebSocketEndpoint, ConnectionParam, ParamType,
//            ConnectionLifecycle, MessageSchema, MessageDirection
```

## Pre-built API Definitions

Pre-built API definitions (like OpenAI) are in the separate `schematic-definitions` crate:

```rust
use schematic_definitions::openai::define_openai_api;

let openai = define_openai_api();
assert_eq!(openai.name, "OpenAI");
assert!(openai.endpoints.len() > 200);
```

See the [schematic-definitions README](../definitions/README.md) for available APIs.

## Naming Conventions

### Body Type Naming

When defining request body types, use a `*Body` suffix to avoid collisions with generated wrapper structs:

```rust
// ✗ BAD: Collides with generated wrapper struct
pub struct GenerateRequest { ... }  // Definition type
// Generated: pub struct GenerateRequest { body: GenerateRequest } ← Recursive!

// ✓ GOOD: Uses *Body suffix
pub struct GenerateBody { ... }  // Definition type
// Generated: pub struct GenerateRequest { body: GenerateBody } ← Works!
```

The generator creates `{EndpointId}Request` wrapper structs for each endpoint. If your body type uses the same name, you'll get a recursive struct that won't compile.

**Convention**: Name body types as `{EndpointId}Body` (e.g., `GenerateBody`, `CreateChatBody`, `EmbedBody`).

### Required Derives for Body Types

All request body types **must** derive `Default`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GenerateBody {
    pub model: String,   // Empty string by default
    pub prompt: String,  // Empty string by default
}
```

The generated wrapper structs implement `Default` and call `Default::default()` on the body type. Without this derive, generated code won't compile.

**Note**: A default with empty strings may be invalid for the API, but it's valid Rust. The API will return an error at runtime, not compile time.

## Schema with Module Paths

For types in specific modules, use `Schema::with_path`:

```rust
use schematic_define::Schema;

// Type in current scope
let simple = Schema::new("User");
assert_eq!(simple.full_path(), "User");

// Type in specific module
let qualified = Schema::with_path("User", "crate::models::user");
assert_eq!(qualified.full_path(), "crate::models::user::User");
```

### SchemaObject Trait

The `SchemaObject` trait provides bounds for types that can be used in API schemas:

```rust
use schematic_define::SchemaObject;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyRequest {
    name: String,
    count: u32,
}

// MyRequest automatically implements SchemaObject
fn accepts_schema<T: SchemaObject>(_: T) {}
```

Required bounds: `Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static`

### Usage Guide

Use `Schema::new()` when:

- The type is defined in the same generated module
- The type will be re-exported via `pub use`

Use `Schema::with_path()` when:

- The type is defined in a different crate or module
- You need explicit qualification to avoid naming conflicts

### Body Type Patterns

Request body types should follow the builder pattern for ergonomic construction:

```rust,ignore
// Core constructor with required fields, then chain optional fields
CreateMessageBody::new("claude-sonnet-4-5-20250514", messages, 1024)
    .with_system("You are a helpful assistant")
    .with_temperature(0.7)
    .with_tools(tools)
```

Recommended methods:

- `new()` - Constructor requiring all mandatory fields
- `with_*()` - Builder methods for optional fields
- `Default` - Implement when all fields have sensible defaults

## Model Definitions

For API specifications imported from OpenAPI, you can define model types:

```rust
use schematic_define::models::{
    ModelCatalog, ModelDef, StructDef, EnumDef, TypeAlias,
    FieldDef, EnumVariant, TypeRef, PrimitiveType
};

let catalog = ModelCatalog {
    module_path: Some("my_api::types".to_string()),
    types: vec![
        ModelDef::Struct(StructDef {
            name: "User".to_string(),
            description: Some("A user in the system".to_string()),
            fields: vec![
                FieldDef {
                    name: "id".to_string(),
                    serde_rename: None,
                    description: Some("Unique identifier".to_string()),
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::Integer),
                },
                FieldDef {
                    name: "name".to_string(),
                    serde_rename: None,
                    description: None,
                    required: true,
                    field_type: TypeRef::Primitive(PrimitiveType::String),
                },
            ],
            additional_properties: None,
        }),
        ModelDef::Enum(EnumDef {
            name: "Status".to_string(),
            description: None,
            variants: vec![
                EnumVariant {
                    name: "Active".to_string(),
                    value: Some("active".to_string()),
                    description: None,
                },
            ],
            untagged: false,
        }),
        ModelDef::Alias(TypeAlias {
            name: "UserId".to_string(),
            description: None,
            target: TypeRef::Primitive(PrimitiveType::String),
        }),
    ],
};
```

### TypeRef Variants

| Variant | Description |
|---------|-------------|
| `Primitive(PrimitiveType)` | Basic types (String, Integer, Number, Boolean, Bytes, Json) |
| `Array(Box<TypeRef>)` | Array of inner type |
| `Map(Box<TypeRef>)` | Map with string keys |
| `Named(String)` | Reference to named type |
| `OneOf(Vec<TypeRef>)` | OpenAPI oneOf |
| `AnyOf(Vec<TypeRef>)` | OpenAPI anyOf |
| `AllOf(Vec<TypeRef>)` | OpenAPI allOf |
| `Optional(Box<TypeRef>)` | Optional wrapper |
| `Unknown` | Unsupported type |

## Migration from Schema to ApiRequest

If you have existing code using `Option<Schema>` for `Endpoint.request`, you need to migrate to `Option<ApiRequest>`:

```rust
// Before (deprecated pattern)
Endpoint {
    request: Some(Schema::new("CreateUserRequest")),
    // ...
}

// After (new pattern)
Endpoint {
    request: Some(ApiRequest::json_type("CreateUserRequest")),
    // ...
}
```

For backward compatibility, `ApiRequest` implements `From<Schema>`:

```rust
use schematic_define::{ApiRequest, Schema};

let schema = Schema::new("MyRequest");
let request: ApiRequest = schema.into(); // Converts to ApiRequest::Json(schema)
```

## OpenAPI Support (Feature-Gated)

The `openapi` feature enables bidirectional OpenAPI 3.x integration:

```toml
[dependencies]
schematic-define = { path = "../define", features = ["openapi"] }
```

This feature provides:

- **`openapi` module** with import and export functions
- **OpenAPI 3.x parsing** via the `openapiv3` crate
- **YAML support** via `serde_yaml`

### Import

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};

let source = OpenApiSource::path("api.yaml");
let result = OpenApiImport::new(source)
    .api_name("MyApi")
    .prefer_json()
    .strict()
    .build()?;

println!("Imported {} endpoints", result.api.endpoints.len());
```

### Export

```rust
use schematic_define::openapi::{export, ExportOptions, ExportFormat};

let options = ExportOptions::new()
    .with_version("1.0.0")
    .with_format(ExportFormat::Yaml);

let openapi_doc = export(&api, &registry, &options)?;
```

See the [schematic-gen README](../gen/README.md) for CLI usage and the full import/export pipeline.

## Dependencies

- `serde` - Serialization/deserialization
- `strum` - Enum utilities (Display, FromStr, Iterator)
- `thiserror` - Error types

## License

AGPL-3.0-only
