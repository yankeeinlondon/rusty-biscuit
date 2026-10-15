# Headers Builder

The `Headers` builder provides a fluent API for constructing HTTP headers with secure credential handling.

**Scope.** Generated **WebSocket** clients carry a `Headers` in their public
`headers` field and send `headers.build()` when connecting. Generated **REST**
clients do not use `Headers` at all: they send through `schematic-runtime`,
which takes `Credential`s (see "Generated Clients" below).

## Quick Start

```rust
use schematic_define::{Headers, EnvList, EnvMapping};

// Bearer token authentication
let headers = Headers::default()
    .use_bearer_token("my-secret-token")
    .accept_json()
    .build()?;

// Basic authentication
let headers = Headers::default()
    .use_basic_auth("username", "password")
    .build()?;

// API key in custom header
let headers = Headers::default()
    .use_api_key("my-key", "X-API-Key")
    .build()?;
```

## Core Types

### SensitiveString

Secure wrapper for passwords and tokens that prevents accidental logging.

```rust
use schematic_define::SensitiveString;

let secret = SensitiveString::from("my-secret-token");

// Debug output is redacted
println!("{:?}", secret);  // SensitiveString("***")

// Access the actual value when needed
let value = secret.as_str();  // "my-secret-token"
```

What it guarantees, and no more:
- **Redacted Debug output**: prints `SensitiveString("***")`
- **No `Display`**: it cannot be printed by accident

It has no `PartialEq`/`Eq` (values simply cannot be compared; this is not a
timing-attack defense) and no zeroize-on-drop; `as_str()` and `into_inner()`
expose the value.

### EnvList

Environment variable fallback chain. First non-empty value wins.

```rust
use schematic_define::EnvList;

// Single variable
let env = EnvList::single("OPENAI_API_KEY");

// Fallback chain (first match wins)
let env = EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY", "API_KEY"]);

// Access names
let names = env.names();  // &["OPENAI_API_KEY", "OPENAI_KEY", "API_KEY"]
```

### EnvMapping

Complete environment variable mapping for all authentication types.

```rust
use schematic_define::{EnvMapping, EnvList, ApiKeyEnv};

// Bearer token only
let mapping = EnvMapping {
    bearer_token: Some(EnvList::single("OPENAI_API_KEY")),
    basic_user: None,
    basic_pass: None,
    api_key: None,
    ..Default::default() // the oauth_* fields
};

// Basic authentication
let mapping = EnvMapping {
    bearer_token: None,
    basic_user: Some(EnvList::single("API_USERNAME")),
    basic_pass: Some(EnvList::single("API_PASSWORD")),
    api_key: None,
    ..Default::default() // the oauth_* fields
};

// API key with custom header
let mapping = EnvMapping {
    bearer_token: None,
    basic_user: None,
    basic_pass: None,
    api_key: Some(ApiKeyEnv {
        names: EnvList::from_strs(&["HF_TOKEN", "HUGGINGFACE_KEY"]),
        header: "Authorization".to_string(),
    }),
    ..Default::default()
};
```

## Headers Builder Methods

### Authentication

```rust
// Bearer token (sets Authorization: Bearer <token>)
.use_bearer_token("my-token")

// Basic auth (sets Authorization: Basic <base64>)
.use_basic_auth("user", "pass")

// API key in custom header (sets <header>: <key>)
.use_api_key("my-key", "X-API-Key")
```

### Standard Headers

```rust
// Content-Type
.content_type("application/json")
.content_type_json()  // Convenience

// Accept
.accept("application/json")
.accept_json()  // Convenience

// User-Agent
.user_agent("MyClient/1.0")
```

### Custom Headers

```rust
// Add custom header (last value wins if duplicate)
.header("X-Request-ID", "12345")
.header("X-Custom", "value")

// Remove a header
.remove("X-Custom")
```

### Environment Resolution

```rust
// Permissive: skip missing vars
let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env();

// Strict: error on missing vars
let headers = Headers::default()
    .with_env_mapping(mapping)
    .try_from_env()?;

// Custom mapping (ignores builder's mapping)
let headers = Headers::default()
    .from_env_with(custom_mapping);
```

### Build and Check

```rust
// Build final header list
let headers: Vec<(String, String)> = builder.build()?;

// Check if authorization is set
if builder.has_authorization() {
    // Skip environment-based auth
}
```

## Generated Clients

### WebSocket clients

A generated WebSocket client creates its `Headers` with the API's
`EnvMapping` attached but does not resolve it. Set explicit auth, or call
`from_env()` / `try_from_env()`, before connecting:

```rust
use schematic_schema::elevenlabs_ws::ElevenLabsTTSWs;

let mut api = ElevenLabsTTSWs::new();
api.headers = api.headers.clone().use_api_key(key, "xi-api-key"); // explicit
// or
api.headers = api.headers.clone().try_from_env()?;               // from the mapping
```

### REST clients

REST clients take a `schematic_runtime::Credential` instead, per call or per
client; the runtime reads the environment fallback itself at send time and
sends anonymously when nothing resolves (there is no missing-credential
error):

```rust
use schematic_schema::prelude::*;

let token = get_token_from_somewhere(); // Vault, OAuth, config file, ...
let client = OpenAI::new().credentials(Credential::access_token(token));
let models = client
    .list_models()
    .credential(Credential::access_token(other_token)) // this call only
    .await?;
```

This covers multi-tenant credentials, token rotation, test credentials, and
OAuth tokens. Precedence and refusal rules: `schematic/docs/runtime/credentials.md`.

## Error Handling

```rust
use schematic_define::HeaderError;

// Strict env resolution reports which variables to set
let builder = match builder.try_from_env() {
    Ok(builder) => builder,
    Err(HeaderError::MissingCredential(vars)) => {
        eprintln!("Set one of: {:?}", vars);
        return;
    }
    Err(other) => panic!("{other}"),
};

// build() validates header names
match builder.build() {
    Ok(headers) => { /* use headers */ }
    Err(HeaderError::InvalidHeaderName(name)) => eprintln!("Bad header name: {}", name),
    Err(_) => { /* other errors */ }
}
```

| Error | Cause |
|-------|-------|
| `InvalidHeaderName` | Non-ASCII or invalid characters in header name (`build()`) |
| `InvalidHeaderValue` | Invalid characters in header value |
| `MissingEnv` | Single env var not set |
| `MissingCredential` | `try_from_env()` found none of a fallback chain's vars set |

These are `Headers` errors only. A REST send never fails for a missing
credential; it is sent anonymously.

## Best Practices

1. **Use SensitiveString for secrets**: Never log raw credentials
2. **Prefer fallback chains**: Support multiple env var names
3. **Check has_explicit_auth()**: env resolution is skipped once any explicit auth is set
4. **Use try_from_env() in production**: Fail fast on missing credentials
5. **Use from_env() in development**: Permissive for easier iteration
