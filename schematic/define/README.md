# schematic-define

REST and WebSocket API definition types for the Schematic code generation system.

## Overview

`schematic-define` provides a declarative way to describe REST and WebSocket APIs. These definitions are consumed by `schematic-gen` to generate strongly-typed Rust client code with automatic authentication, request serialization, and response deserialization.

The definition process is intentionally **data-driven**: you describe *what* the API looks like (endpoints, methods, schemas) rather than *how* to call it. The generator handles the implementation details.

REST definitions also support lossless Serde data transfer; see [definition serialization and migration checks](../docs/definition-snapshots.md).

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
| `UpdateStrategy` | Auth-update choice (NoChange, ChangeTo); still exported, but no generated code reads it |
| `CrossOrigin` | API-level policy for redirects and next-page links to other origins |
| `ApiRequest` | Request body type (JSON, FormData, UrlEncoded, Text, Binary) |
| `ApiResponse` | Response type (JSON, Text, Binary, Empty) |
| `FormField` | Form field definition for multipart/URL-encoded requests |
| `FormFieldKind` | Form field type (Text, File, Files, Json) |
| `Schema` | Type name and optional module path for code generation |
| `SchemaObject` | Trait bound for serializable/deserializable types |

### Header and Authentication Types

| Type | Purpose |
|------|---------|
| `Headers` | Fluent builder for HTTP headers with auth support; used by generated WebSocket clients |
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
| `PaginationStyle` | Superseded: materializes `page`/`per_page`-style query parameters; drives no pager |
| `PaginationResponse` | Superseded: read by nothing |

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

The definition only says *where* a credential goes and which environment variables can supply one. The generator translates it into the API's runtime `ApiDefinition`: each accepted method becomes a credential placement (an OAuth2 method is accepted as a bearer access token), and the environment mapping becomes the API's environment fallback. At send time [`schematic-runtime`](../runtime/README.md) resolves one credential per request, first match wins:

1. A per-call credential (`.credential(...)` on the call)
2. A client credential (`{Api}::new().api_key(...)`, `.basic_auth(...)`, or `.credentials(...)`)
3. The first set, non-empty variable of the environment fallback
4. None: the request is sent anonymously, and the server's reply (typically a 401) arrives in the response envelope

`.anonymous()` on a call skips every source. See [Credentials](../docs/runtime/credentials.md) for the full rules.

Legacy `env_auth` and `env_username` are still supported, but the generator normalizes them into `EnvMapping` before translating the fallback.

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

OAuth2 token lifecycle is managed by the `schematic-oauth` runtime crate. The `OAuth2Config` here is declarative metadata describing the provider. Generated clients do not run OAuth flows: obtain a token with `schematic-oauth` (or any other source) and pass it as `Credential::access_token(token)`, either per client or per call.

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
    # version: None,
    # env_mapping: None,
    # cross_origin: schematic_define::CrossOrigin::default(),
    # request_id_header: None,
    # secret_names: vec![],
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

A missing credential is not an error. When neither an explicit credential nor the environment fallback supplies one, the generated client sends the request without one, and the server's reply arrives in the `Response` envelope with the runtime's `AuthMethod::Anonymous` (not this crate's `AuthMethod`) recorded in `response.meta.request.auth`. See [A missing credential is not an error](../docs/runtime/credentials.md#a-missing-credential-is-not-an-error).

### Programmatic Authentication

To inject a credential without environment variables, hand a `Credential` to the generated client, for the whole client or for one call:

```rust
use schematic_schema::prelude::*;

// Token from a runtime source (Vault, OAuth, config file, etc.)
let token = get_token_from_somewhere();

// Every send of this client carries the token
let client = OpenAI::new().credentials(Credential::access_token(token));

// Or one call only, overriding the client credential
let models = client
    .list_models()
    .credential(Credential::access_token(other_token))
    .await?;
```

A credential's kind must match a placement the definition accepts: `Credential::access_token` for bearer and OAuth2, `Credential::api_key` for key headers, query parameters, and cookies, and `Credential::basic` for Basic. An explicit credential always wins over the environment fallback.

This is useful for:

- **Multi-tenant applications**: Different credentials per tenant
- **Token rotation**: Refresh tokens from a vault at runtime
- **Testing**: Inject mock credentials without setting env vars
- **OAuth flows**: Use tokens obtained from OAuth providers

### UpdateStrategy

`UpdateStrategy` (`NoChange` or `ChangeTo(AuthStrategy)`) is still exported, but no generated code reads it: the generated `variant()` method it served no longer exists. Reconfigure a client through its chained settings (`{Api}::new().base_url(..)`, `.api_key(..)`, …) instead.

## Headers Builder

The `Headers` type provides a fluent API for building HTTP headers. Generated WebSocket clients use it for their handshake headers and credentials; generated REST clients do not, since their credentials go through the runtime (see [Programmatic Authentication](#programmatic-authentication)).

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

## Cross-Origin Policy, Request IDs, and Secret Names

Three API-level fields describe how the runtime treats responses and what it
must never record. Endpoints cannot override them.

| Field | Meaning |
|-------|---------|
| `cross_origin` | Which origins a redirect or followed next-page link may reach; defaults to same-origin |
| `request_id_header` | Response header carrying the provider's request id (case-insensitive); `None` when unknown |
| `secret_names` | Extra header names (case-insensitive) and query names (exact) whose values are never stored or shown |

`CrossOrigin` has five policies:

| Constructor | Follows |
|-------------|---------|
| `CrossOrigin::same_origin()` (default) | Only the base origin (scheme, host, effective port) |
| `CrossOrigin::allow_origins([...])` | The base plus these exact origins, with credentials; no wildcards |
| `CrossOrigin::allow_without_credentials()` | Anywhere; credentials are stripped when the origin differs from the base |
| `CrossOrigin::danger_allow_any()` | Anywhere, with credentials |
| `CrossOrigin::deny()` | No redirects and no cross-origin links; same-origin next-page links still pass |

A caller may only tighten the definition's policy (to `deny`, `same_origin`,
or a subset of an `allow_origins` list; over `deny`, only `deny`). The runtime
enforces the policy.

```rust
use schematic_define::CrossOrigin;

let api = RestApi {
    // ...
    cross_origin: CrossOrigin::same_origin(),
    request_id_header: Some("x-request-id".to_string()),
    secret_names: vec![],
    default_pagination: None,
    pagination_exceptions: vec![],
};
```

`secret_names` adds to the built-in secret names and the API's own
authentication header or query name, which are always treated as secret.

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
use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse, CrossOrigin};

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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
        },
    ],
    cross_origin: CrossOrigin::default(),
    request_id_header: None,
    secret_names: vec![],
    default_pagination: None,
    pagination_exceptions: vec![],
};
```

### Example 2: REST API with Bearer Token Auth

A user management API with CRUD operations:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse, CrossOrigin
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
        },
    ],
    cross_origin: CrossOrigin::default(),
    request_id_header: None,
    secret_names: vec![],
    default_pagination: None,
    pagination_exceptions: vec![],
};
```

### Example 3: File Storage API with File Uploads

A file storage API demonstrating multipart form-data uploads:

```rust
use schematic_define::{
    RestApi, Endpoint, RestMethod, AuthStrategy, ApiRequest, ApiResponse, FormField,
    CrossOrigin,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
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
            oauth_scopes: None, cross_origin: None,
        },
    ],
    cross_origin: CrossOrigin::default(),
    request_id_header: None,
    secret_names: vec![],
    default_pagination: None,
    pagination_exceptions: vec![],
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

### Pagination

Pagination is authored once per API, not on endpoints. `RestApi` has a
default strategy for its lists and one table of exception rows keyed by
endpoint id; the types live in `schematic_define::pagination`:

```rust
use schematic_define::pagination::{
    Collection, MoreSignal, PageSize, PaginationException, Strategy, ValueSource,
};

let default = Strategy::page_number("page", MoreSignal::link_rel("next"))
    .page_size(PageSize::new("per_page").server_default(30).max(100))
    .build();

// A GET endpoint whose response is `Vec<T>` inherits `default`. Wrapper
// lists, opt-outs, overrides, and non-GET lists need a row.
let exceptions = vec![
    PaginationException::new(
        "ListWorkflowRuns",
        Collection::at("workflow_runs")
            .item_type("WorkflowRun")
            .paginate(default.clone().total(ValueSource::body("total_count"))),
    ),
    PaginationException::new("GetTagReference", Collection::root().unpaginated()),
];
// RestApi { default_pagination: Some(default), pagination_exceptions: exceptions, .. }
```

A strategy input with the same name and location as a declared parameter
adopts it (the request struct drops the field) only when the strategy
authors the knob that replaces it. If the endpoint declares that parameter
required, the strategy needs a value to send when the caller sets none
(`ManagedRole::sent_default`: the first page or offset, or the size's
`sent_default`); ordinary sends of the request send that value too. The rules are checked by
`pagination::validate_pagination`, which `schematic-gen` runs; see
[generation validation](../docs/generation/validation.md#pagination-rules).

`PaginationStyle`, `PaginationResponse`, and `EndpointParams::with_pagination`
are superseded. They remain only because existing definitions still use
`with_pagination` to add ordinary query parameters; nothing generates a pager
from them.

[Pagination](../docs/pagination.md) describes resolution, adoption, every
strategy builder, and what the generated pager does with them.

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

## Typed registration

`DefinitionContext` provides ordered typed registrations, alias paths checked by
`rust_path!`, and destination validation for Serde descriptors. This API is
additive alongside existing definitions. See [typed registration](../docs/typed-registration.md)
for authoring, container, and module-group assembly rules.

## Dependencies

- `schemars` - OpenAPI-compatible schema generation for typed registrations
- `serde_json` - Inline schemas and descriptor transfer data
- `serde` - Serialization/deserialization
- `strum` - Enum utilities (Display, FromStr, Iterator)
- `thiserror` - Error types

## License

AGPL-3.0-only

REST required-fact authoring, endpoint origin policies, secret markers, parameter locations and pager adoption are covered in [definition builders](../docs/definition-builders.md).
