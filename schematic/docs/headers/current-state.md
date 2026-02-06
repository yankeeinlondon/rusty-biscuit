# Schematic Headers: Current State of ENV Variable Dependency

## Summary

Schematic's authentication system is **entirely ENV variable-driven**. Every authenticated API call resolves credentials by calling `std::env::var()` at request time. There is no mechanism to programmatically inject credentials (tokens, API keys, passwords) directly into a client instance. The only configurable aspect is *which* environment variable names to read from, not the actual credential values.

## How Credentials Flow Through the System

### 1. Definition Time (`schematic-define`)

API definitions declare environment variable **names** (not values):

```rust
pub struct RestApi {
    pub env_auth: Vec<String>,        // e.g., ["OPENAI_API_KEY"]
    pub env_username: Option<String>,  // e.g., Some("EMQX_API_KEY") for Basic auth
    pub auth: AuthStrategy,            // How credentials are applied to requests
    // ...
}
```

`env_auth` is a `Vec<String>` to support fallback chains. The first env var that is set wins.

### 2. Generation Time (`schematic-gen`)

The code generator copies env var **names** into generated struct fields:

```rust
// Generated struct (e.g., OpenAI)
pub struct OpenAI {
    client: reqwest::Client,
    base_url: String,
    env_auth: Vec<String>,                       // Stores ["OPENAI_API_KEY"]
    auth_strategy: schematic_define::AuthStrategy,
    env_username: Option<String>,
    headers: Vec<(String, String)>,
    variant_hooks: crate::shared::VariantHooks,
}
```

All four constructors (`new()`, `with_base_url()`, `with_client()`, `with_client_and_base_url()`) hardcode the env var names from the definition. None accept credential values.

### 3. Request Time (Generated Client)

Every request resolves credentials via `std::env::var()` in `build_and_send_request()`:

```rust
// Generated auth setup (runtime match on self.auth_strategy)
match &self.auth_strategy {
    AuthStrategy::BearerToken { header } => {
        let token = self.env_auth
            .iter()
            .find_map(|var| std::env::var(var).ok())   // <-- ENV lookup
            .ok_or_else(|| SchematicError::MissingCredential {
                env_vars: self.env_auth.clone(),
            })?;
        req_builder = req_builder.header(header_name, format!("Bearer {}", token));
    }
    AuthStrategy::ApiKey { header } => {
        let key = self.env_auth
            .iter()
            .find_map(|var| std::env::var(var).ok())   // <-- ENV lookup
            .ok_or_else(|| SchematicError::MissingCredential { ... })?;
        req_builder = req_builder.header(header.as_str(), key);
    }
    AuthStrategy::Basic => {
        let username = std::env::var(username_env)...;  // <-- ENV lookup
        let password = std::env::var(password_env)...;  // <-- ENV lookup
        req_builder = req_builder.basic_auth(username, Some(password));
    }
}
```

Credential resolution happens on **every request**, not once at construction.

## API-by-API ENV Variable Inventory

| API | Auth Strategy | `env_auth` | `env_username` | Header |
|-----|--------------|------------|----------------|--------|
| OpenAI | BearerToken | `OPENAI_API_KEY` | - | `Authorization` |
| Anthropic | ApiKey | `ANTHROPIC_API_KEY` | - | `X-Api-Key` |
| HuggingFaceHub | BearerToken | `HF_TOKEN`, `HUGGING_FACE_API_KEY`, `HF_API_KEY` | - | `Authorization` |
| ElevenLabs | ApiKey | `ELEVEN_LABS_API_KEY`, `ELEVENLABS_API_KEY` | - | `xi-api-key` |
| ElevenLabsTTS (WS) | ApiKey | `ELEVEN_LABS_API_KEY`, `ELEVENLABS_API_KEY` | - | `xi-api-key` |
| EmqxBasic | Basic | `EMQX_API_SECRET` (password) | `EMQX_API_KEY` (username) | `Authorization` |
| EmqxBearer | BearerToken | `EMQX_TOKEN` | - | `Authorization` |
| OllamaNative | None | *(empty)* | - | *(none)* |
| OllamaOpenAI | None | *(empty)* | - | *(none)* |

## What Can Be Changed at Runtime

### Via `variant()` Builder

The variant builder allows changing **env var names** and **auth strategy**, but not credential values:

```rust
let staging = client.variant()
    .base_url("https://staging.api.com/v1")
    .env_auth(vec!["STAGING_API_KEY".to_string()])  // Different env var NAME
    .auth_update(UpdateStrategy::ChangeTo(
        AuthStrategy::BearerToken { header: None }
    ))
    .build();
```

This changes *which* environment variable is read, not the credential itself. The actual token still comes from `std::env::var("STAGING_API_KEY")`.

### Via `variant_with()` Convenience

Same capability, shorthand form:

```rust
let staging = client.variant_with(
    "https://staging.api.com/v1",
    vec!["STAGING_API_KEY".to_string()],
    UpdateStrategy::NoChange,
);
```

### Via `api_key_header()`

The only method that extracts actual credentials still reads from ENV:

```rust
pub fn api_key_header(&self) -> Option<(String, String)> {
    match &self.auth_strategy {
        AuthStrategy::ApiKey { header } => {
            for env_name in &self.env_auth {
                if let Ok(value) = std::env::var(env_name) {  // <-- ENV lookup
                    return Some((header.clone(), value));
                }
            }
            None
        }
        _ => None,
    }
}
```

## What Cannot Be Done

| Capability | Supported? | Notes |
|-----------|:----------:|-------|
| Set a token directly on the client | No | No `with_token()` or `set_api_key()` method |
| Store a credential in the struct | No | Struct stores env var names, not values |
| Pass credentials to a constructor | No | All constructors hardcode env var names |
| Override credentials per-request | No | Auth is resolved in `build_and_send_request()` |
| Use a credential vault/store | No | Only `std::env::var()` is called |
| Inject credentials via variant builder | No | Builder only changes env var names |

## Workarounds Users Must Use Today

To use schematic-generated clients with credentials not in environment variables, users must:

1. **Set env vars programmatically** before making requests:
   ```rust
   std::env::set_var("OPENAI_API_KEY", token_from_vault);
   let response = client.request::<T>(req).await?;
   ```
   This is process-global and not thread-safe for different credentials.

2. **Use `http_client()` to bypass generated methods**:
   ```rust
   let resp = client.http_client()
       .get(format!("{}/models", client.api_base_url()))
       .header("Authorization", format!("Bearer {}", my_token))
       .send()
       .await?;
   ```
   This loses all generated type safety, serialization, and error handling.

3. **Use a custom reqwest client with default headers**:
   ```rust
   let mut headers = reqwest::header::HeaderMap::new();
   headers.insert("Authorization", format!("Bearer {}", token).parse().unwrap());
   let http = reqwest::Client::builder().default_headers(headers).build()?;
   // But the generated auth code STILL runs and will fail without ENV vars
   ```
   This doesn't work because generated auth code runs regardless and will return `MissingCredential`.

**In practice, workaround #1 is the only viable option**, and it has significant limitations (global mutation, not thread-safe for multi-tenant scenarios).

## Static Headers vs. Dynamic Headers

The system does support **static headers** at the API and endpoint level:

```rust
// API-level headers (applied to all requests)
RestApi { headers: vec![("X-Api-Version".to_string(), "2024-01".to_string())], ... }

// Endpoint-level headers (applied to specific endpoints)
Endpoint { headers: vec![("X-Custom".to_string(), "value".to_string())], ... }
```

These are set at definition time and baked into the generated code. Endpoint headers override API headers for matching keys (case-insensitive merge). The variant builder can also override headers at runtime. However, none of these header mechanisms help with credential injection because the auth setup runs separately from the header merge.

## Implications

1. **Testing is difficult**: Integration tests and mocked servers require setting ENV vars, which creates global state and test isolation challenges.

2. **Multi-tenant is impossible**: A single process cannot use different credentials for different tenants without global ENV mutation.

3. **Credential rotation requires ENV updates**: If a token expires, the new token must be placed in the environment variable before the next request.

4. **Vault/secret-manager integration is awkward**: Credentials from HashiCorp Vault, AWS Secrets Manager, etc. must be loaded into ENV vars rather than injected directly.

5. **The variant builder is close but not there**: It already allows changing env var names, auth strategy, base URL, and headers. Adding direct credential injection would be a natural extension.
