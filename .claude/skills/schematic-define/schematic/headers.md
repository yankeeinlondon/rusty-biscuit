# Headers Builder

The `Headers` builder provides a fluent API for constructing HTTP headers with secure credential handling.

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

Security features:
- **Redacted Debug output**: Prevents secrets from appearing in logs
- **No PartialEq/Eq**: Prevents timing attacks in comparisons
- **Clone support**: Can be safely cloned

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
};

// Basic authentication
let mapping = EnvMapping {
    bearer_token: None,
    basic_user: Some(EnvList::single("API_USERNAME")),
    basic_pass: Some(EnvList::single("API_PASSWORD")),
    api_key: None,
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

## Integration with Generated Clients

Use `Headers` with the variant builder for programmatic authentication:

```rust
use schematic_define::Headers;
use schematic_schema::OpenAI;

// Token from runtime source (Vault, OAuth, config file, etc.)
let token = get_token_from_somewhere();

// Create client with programmatic token
let client = OpenAI::new()?
    .variant()
    .headers_builder(Headers::default().use_bearer_token(token))
    .build();
```

When `Headers` has authorization set via `use_bearer_token()` or `use_basic_auth()`, the generated client skips environment variable lookup. This enables:

- **Multi-tenant applications**: Different credentials per tenant
- **Token rotation**: Refresh tokens from a vault at runtime
- **Testing**: Inject mock credentials without setting env vars
- **OAuth flows**: Use tokens obtained from OAuth providers

## Error Handling

```rust
use schematic_define::HeaderError;

let result = builder.build();
match result {
    Ok(headers) => { /* use headers */ }
    Err(HeaderError::InvalidHeaderName(name)) => {
        eprintln!("Bad header name: {}", name);
    }
    Err(HeaderError::MissingCredential(vars)) => {
        eprintln!("Set one of: {:?}", vars);
    }
    _ => { /* other errors */ }
}
```

| Error | Cause |
|-------|-------|
| `InvalidHeaderName` | Non-ASCII or invalid characters in header name |
| `InvalidHeaderValue` | Invalid characters in header value |
| `MissingEnv` | Single env var not set |
| `MissingCredential` | None of the fallback chain vars set |

## Best Practices

1. **Use SensitiveString for secrets**: Never log raw credentials
2. **Prefer fallback chains**: Support multiple env var names
3. **Check has_authorization()**: Skip env lookup when auth is programmatic
4. **Use try_from_env() in production**: Fail fast on missing credentials
5. **Use from_env() in development**: Permissive for easier iteration
