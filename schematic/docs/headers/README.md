# Headers API

The Headers API provides a type-safe, builder-style interface for managing HTTP headers with flexible credential injection. It replaces the legacy ENV variable-only approach with a system that supports both programmatic credential injection and environment-based configuration.

## Overview

The Headers API consists of four main types:

- **`Headers`**: Builder for constructing HTTP header lists with authentication
- **`EnvMapping`**: Configuration for environment variable resolution
- **`EnvList`**: Fallback chain of environment variable names
- **`SensitiveString`**: Secure wrapper for credentials (redacted in Debug output)

## Quick Start

### Programmatic Credentials

```rust
use schematic_define::Headers;

// Direct credential injection
let headers = Headers::default()
    .use_bearer_token("sk-proj-...")
    .accept_json()
    .build()?;

// Basic authentication
let headers = Headers::default()
    .use_basic_auth("username", "password")
    .content_type_json()
    .build()?;

// API key with custom header
let headers = Headers::default()
    .use_api_key("my-api-key", "X-API-Key")
    .build()?;
```

### Environment-Based Credentials

```rust
use schematic_define::{Headers, EnvMapping, EnvList};

// Automatic ENV loading with default mapping
let headers = Headers::default()
    .from_env()  // Permissive: skips missing vars
    .build()?;

// Custom ENV mapping
let mapping = EnvMapping {
    bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY", "OPENAI_KEY"])),
    basic_user: None,
    basic_pass: None,
    api_key: None,
};

let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env()
    .build()?;

// Strict ENV loading (errors on missing credentials)
let headers = Headers::default()
    .with_env_mapping(mapping)
    .try_from_env()?  // Returns Err if credentials missing
    .build()?;
```

### Hybrid Approach

```rust
// ENV with programmatic override
let headers = Headers::default()
    .from_env()                    // Load from ENV if available
    .use_bearer_token(custom_token) // Override with explicit value
    .build()?;
```

## Integration with Generated Clients

### Using with Variant Builder

```rust
use schematic_define::Headers;
use schematic_schema::OpenAI;

// Create headers with programmatic credentials
let custom_headers = Headers::default()
    .use_bearer_token("sk-proj-custom-token")
    .build()?;

// Apply to client variant
let client = OpenAI::default()
    .variant_with_headers(custom_headers);

// Make authenticated requests
let response = client.request::<ChatCompletionResponse>(req).await?;
```

### Default Environment Loading

Generated clients automatically attempt to load credentials from environment variables using their configured `EnvMapping`. You can override this by using `variant_with_headers()`.

## Environment Variable Resolution

### Fallback Chains

`EnvList` supports multiple environment variable names with priority order:

```rust
use schematic_define::EnvList;

// First non-empty value wins
let env_list = EnvList::from_strs(&[
    "OPENAI_API_KEY",    // Checked first
    "OPENAI_KEY",        // Fallback if first is unset
    "API_KEY",           // Final fallback
]);
```

### Precedence Rules

1. **Programmatic values override ENV**: Explicitly set values via builder methods take precedence
2. **ENV loading is permissive by default**: `from_env()` silently skips missing variables
3. **Strict mode available**: `try_from_env()` returns errors for missing credentials

## Security Features

### SensitiveString

All credential types use `SensitiveString` internally to prevent accidental leaks:

```rust
use schematic_define::SensitiveString;

let secret = SensitiveString::from("my-secret-token");

// Debug output is redacted
println!("{:?}", secret);  // Prints: SensitiveString("***")

// Access the value when needed
let token = secret.as_str();  // Returns: "my-secret-token"
```

**Security properties:**

- Does NOT implement `PartialEq`/`Eq` (prevents timing attacks)
- Debug output redacted to `"***"`
- Only accessible via `as_str()` or `into_inner()`

## Common Patterns

### Multi-Tenant Applications

```rust
// Different credentials for different tenants
let tenant_a_headers = Headers::default()
    .use_bearer_token(vault.get("tenant_a_token"))
    .build()?;

let tenant_b_headers = Headers::default()
    .use_bearer_token(vault.get("tenant_b_token"))
    .build()?;

let client_a = OpenAI::default().variant_with_headers(tenant_a_headers);
let client_b = OpenAI::default().variant_with_headers(tenant_b_headers);
```

### Testing with Mocked Credentials

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_call() {
        let headers = Headers::default()
            .use_bearer_token("test-token-12345")
            .build()
            .unwrap();

        let client = TestApiClient::default()
            .variant_with_headers(headers);

        // No need to set ENV vars in tests
        assert!(client.request::<Response>(req).await.is_ok());
    }
}
```

### Credential Rotation

```rust
// Fetch fresh token from vault
let new_token = credential_vault.refresh_token().await?;

// Create new headers with rotated credential
let headers = Headers::default()
    .use_bearer_token(new_token)
    .build()?;

// Update client with new credentials
let client = client.variant_with_headers(headers);
```

## API Reference

For detailed API documentation, see:

- [SensitiveString](https://docs.rs/schematic-define/latest/schematic_define/struct.SensitiveString.html)
- [Headers](https://docs.rs/schematic-define/latest/schematic_define/struct.Headers.html)
- [EnvMapping](https://docs.rs/schematic-define/latest/schematic_define/struct.EnvMapping.html)
- [EnvList](https://docs.rs/schematic-define/latest/schematic_define/struct.EnvList.html)
- [HeaderError](https://docs.rs/schematic-define/latest/schematic_define/enum.HeaderError.html)

Or generate local documentation:

```bash
cargo doc -p schematic-define --open
```

## Migration Guide

See [migration.md](./migration.md) for a comprehensive guide on migrating from the legacy ENV-only approach to the new Headers API.
