# Header Design

This document proposes an idiomatic Rust design for representing HTTP headers with ergonomic environment-variable loading and builder-style overrides.

## Goals

- Provide a typed, ergonomic API for common headers (Authorization, Content-Type, Accept, User-Agent).
- Allow environment variables to seed header values with predictable precedence rules.
- Support custom headers without losing type safety for common ones.
- Enable easy conversion into `http::HeaderMap` (or `reqwest::header::HeaderMap`).
- Keep the API chainable and minimal for typical use cases.

## Non-Goals

- Full HTTP client functionality (this is only a header builder).
- Automatic request signing or complex auth flows.
- Depend on nonstandard environment variable conventions unless configured.

## Proposed Public API

```rust
use http::header::{HeaderMap, HeaderName, HeaderValue};

#[derive(Clone, Default)]
pub struct Headers {
    authorization: Option<SensitiveString>,
    content_type: Option<String>,
    accept: Option<String>,
    user_agent: Option<String>,
    custom: Vec<(HeaderName, String)>,
}

impl Headers {
    pub fn new() -> Self;

    // Env loading (best-effort, no errors)
    pub fn from_env(self) -> Self;
    pub fn from_env_with(self, mapping: EnvMapping) -> Self;

    // Strict env loading (errors on invalid header values)
    pub fn try_from_env(self) -> Result<Self, HeaderError>;
    pub fn try_from_env_with(self, mapping: EnvMapping) -> Result<Self, HeaderError>;

    // Common header builders
    pub fn use_bearer_token(self, token: Option<impl AsRef<str>>) -> Self;
    pub fn use_basic_auth(self, username: impl AsRef<str>, password: impl AsRef<str>) -> Self;
    pub fn use_api_key(self, header: impl AsRef<str>, value: Option<impl AsRef<str>>) -> Self;
    pub fn user_agent(self, value: impl AsRef<str>) -> Self;
    pub fn content_type(self, value: impl AsRef<str>) -> Self;
    pub fn accept(self, value: impl AsRef<str>) -> Self;
    pub fn accept_json(self) -> Self;
    pub fn content_type_json(self) -> Self;

    // Custom header helpers
    pub fn header(self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self;
    pub fn remove(self, name: impl AsRef<str>) -> Self;

    // Finalize into validated HeaderMap
    pub fn build(self) -> Result<HeaderMap, HeaderError>;
}

#[derive(Clone, Default)]
pub struct EnvMapping {
    pub bearer_token: EnvList,
    pub basic_user: EnvList,
    pub basic_pass: EnvList,
    pub api_key: Option<ApiKeyEnv>,
    pub user_agent: EnvList,
    pub accept: EnvList,
    pub content_type: EnvList,
}

#[derive(Clone)]
pub struct EnvList {
    pub names: Vec<&'static str>,
}

#[derive(Clone)]
pub struct ApiKeyEnv {
    pub names: Vec<&'static str>,
    pub header: &'static str,
}

#[derive(Clone)]
pub struct SensitiveString(String);

#[derive(Debug, thiserror::Error)]
pub enum HeaderError {
    #[error("Invalid header name: {0}")]
    InvalidHeaderName(String),
    #[error("Invalid header value for {name}")]
    InvalidHeaderValue { name: String },
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
}
```

### Why this shape

- `Headers` acts as a builder with string storage, deferring validation until `build()`.
- `SensitiveString` avoids accidental token leaks via `Debug` output.
- `EnvMapping` allows a safe, explicit mapping of env names to headers.
- Two env loading modes keep the common path ergonomic while allowing strict validation when needed.

## Environment Variable Loading

### Default mapping

`Headers::from_env()` uses `EnvMapping::default()` with a conservative, explicit set of names:

```
AUTH_BEARER_TOKEN
AUTH_BASIC_USER
AUTH_BASIC_PASS
HTTP_USER_AGENT
HTTP_ACCEPT
HTTP_CONTENT_TYPE
HTTP_API_KEY (header name defaults to x-api-key)
```

Note: environment conventions vary widely. `EnvMapping` is intentionally flexible so each API can provide its own list of env names for each header.

### Multiple env names per header

Each header field accepts a list of env var names. The first non-empty value wins, making it easy to support multiple naming conventions.

```
EnvList { names: vec!["OPENAI_API_KEY", "OPEN_AI_API_KEY"] }
```

### Precedence rules

Env loading only fills unset fields. Builder overrides always win. For env lists, the first non-empty value wins.

```
Headers::new()
    .from_env()              // seed from env if empty
    .use_bearer_token(token) // overrides env if provided
    .user_agent("my-agent")  // overrides env if provided
```

This precedence is intentional and matches typical builder semantics.

### Strict vs best-effort

- `from_env` ignores missing variables and skips values that fail header validation.
- `try_from_env` returns `HeaderError` on invalid or missing values.

This allows CLI tools to be strict while library usage can remain permissive.

## Header Validation Strategy

To keep the builder chain ergonomic, `Headers` stores string values and validates only during `build()` using `HeaderName::from_bytes` and `HeaderValue::from_str`. This keeps the common path simple while preserving correctness and safety.

If immediate validation is desired, provide `try_header` or `try_use_bearer_token` methods that return `Result<Self, HeaderError>`.

## Security Considerations

- Implement `Debug` for `SensitiveString` to redact secrets (`"***"`).
- Avoid logging env contents directly.
- Prefer `Authorization` over custom headers when possible, but allow `x-api-key` for legacy APIs.

## Examples

### Basic usage with env + override

```rust
let api_key: Option<String> = std::env::var("API_KEY").ok();

let headers = Headers::new()
    .from_env()
    .use_bearer_token(api_key)
    .content_type_json();

let header_map = headers.build()?;
```

### Custom env mapping

```rust
let mapping = EnvMapping {
    bearer_token: EnvList {
        names: vec!["MYAPP_TOKEN", "MYAPP_BEARER"],
    },
    api_key: Some(ApiKeyEnv {
        names: vec!["MYAPP_API_KEY"],
        header: "x-api-key",
    }),
    user_agent: EnvList {
        names: vec!["MYAPP_UA"],
    },
    ..Default::default()
};

let headers = Headers::new()
    .from_env_with(mapping)
    .accept("application/json");
```

### OpenAI vs Anthropic mapping

```rust
let openai = EnvMapping {
    bearer_token: EnvList {
        names: vec!["OPENAI_API_KEY", "OPEN_AI_API_KEY"],
    },
    ..Default::default()
};

let anthropic = EnvMapping {
    api_key: Some(ApiKeyEnv {
        names: vec!["ANTHROPIC_API_KEY"],
        header: "x-api-key",
    }),
    ..Default::default()
};
```

### Use with `reqwest`

```rust
let header_map = Headers::new()
    .from_env()
    .user_agent("schematic/1.0")
    .build()?;

let client = reqwest::Client::builder()
    .default_headers(header_map)
    .build()?;
```

## Optional Extensions

- `Headers::merge(self, other: Headers) -> Self` for composing defaults.
- `Headers::apply_env(self, mapping: EnvMapping, mode: EnvMode)` to unify strict vs permissive.
- `Headers::from_env_prefix("MYAPP_")` to auto-build names like `MYAPP_AUTH_BEARER_TOKEN`.

## Implementation Notes

- Use `thiserror` for `HeaderError` to keep errors structured and user-friendly.
- Keep allocations minimal: build a `HeaderMap` with capacity based on the number of set fields.
- Use `HeaderName::from_bytes` and `HeaderValue::from_str` during `build` for validation.
- When encoding Basic auth, use base64 with strict ASCII and no trailing newline.
