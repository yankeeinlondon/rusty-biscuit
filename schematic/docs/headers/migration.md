# Headers Migration Guide

This guide explains how to migrate from Schematic's legacy ENV variable-only authentication to the new Headers API.

## Overview

The legacy system required all credentials to be stored in environment variables and resolved at request time. The new Headers API provides three credential injection modes:

1. **Programmatic**: Direct credential injection (new)
2. **Environment-based**: Load from ENV variables (legacy compatibility)
3. **Hybrid**: ENV with programmatic overrides (recommended for flexibility)

## What Changed

### Before: ENV Variable-Only

The legacy system:

- **Required** credentials to be in environment variables
- Resolved credentials at **request time** (on every request)
- Allowed changing ENV variable **names** but not **values**
- Made multi-tenant scenarios impossible without global ENV mutation

```rust
// Old approach: ENV variable required
std::env::set_var("OPENAI_API_KEY", "sk-proj-...");

let client = OpenAI::default();
let response = client.request::<T>(req).await?;
```

### After: Flexible Credential Injection

The new system:

- **Supports** direct credential injection
- Resolves credentials **once** during client variant construction
- Allows both programmatic and ENV-based credentials
- Enables multi-tenant and testing scenarios

```rust
// New approach: Direct injection
let headers = Headers::default()
    .use_bearer_token("sk-proj-...")
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);

let response = client.request::<T>(req).await?;
```

## Migration Scenarios

### Scenario 1: Simple ENV-Based Usage

**Before:**
```rust
// Set ENV variable
std::env::set_var("OPENAI_API_KEY", token);

// Create client (reads ENV at request time)
let client = OpenAI::default();
```

**After (Option A: ENV compatibility mode):**
```rust
// Set ENV variable (same as before)
std::env::set_var("OPENAI_API_KEY", token);

// Create client (automatic ENV loading)
let client = OpenAI::default();
```

Generated clients automatically attempt ENV loading with their default `EnvMapping`. No changes required.

**After (Option B: Explicit ENV loading):**
```rust
// Set ENV variable
std::env::set_var("OPENAI_API_KEY", token);

// Explicit ENV resolution
let headers = Headers::default()
    .from_env()
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

**After (Option C: Programmatic injection - RECOMMENDED):**
```rust
// No ENV variable needed
let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### Scenario 2: Custom ENV Variable Names

**Before:**
```rust
// Set custom ENV variable
std::env::set_var("MY_CUSTOM_TOKEN", token);

// Create variant with custom env_auth names
let client = OpenAI::default()
    .variant()
    .env_auth(vec!["MY_CUSTOM_TOKEN".to_string()])
    .build();
```

**After:**
```rust
// Option A: Custom ENV mapping
std::env::set_var("MY_CUSTOM_TOKEN", token);

let mapping = EnvMapping {
    bearer_token: Some(EnvList::single("MY_CUSTOM_TOKEN")),
    basic_user: None,
    basic_pass: None,
    api_key: None,
};

let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env()
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);

// Option B: Direct injection (no ENV needed)
let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### Scenario 3: Multi-Tenant Applications

**Before (NOT POSSIBLE without global ENV mutation):**
```rust
// Tenant A
std::env::set_var("OPENAI_API_KEY", tenant_a_token);
let client_a = OpenAI::default();
let resp_a = client_a.request::<T>(req).await?;

// Tenant B (OVERWRITES global ENV - race condition!)
std::env::set_var("OPENAI_API_KEY", tenant_b_token);
let client_b = OpenAI::default();
let resp_b = client_b.request::<T>(req).await?;

// client_a now uses tenant_b_token!
```

**After:**
```rust
// Tenant A
let headers_a = Headers::default()
    .use_bearer_token(tenant_a_token)
    .build()?;
let client_a = OpenAI::default()
    .variant_with_headers(headers_a);

// Tenant B (independent, no race condition)
let headers_b = Headers::default()
    .use_bearer_token(tenant_b_token)
    .build()?;
let client_b = OpenAI::default()
    .variant_with_headers(headers_b);

// Both clients maintain their own credentials
```

### Scenario 4: Testing

**Before:**
```rust
#[test]
fn test_api_call() {
    // Global ENV mutation (affects other tests)
    std::env::set_var("OPENAI_API_KEY", "test-token");

    let client = OpenAI::default();
    // Test using ENV variable
}
```

**After:**
```rust
#[test]
fn test_api_call() {
    // No ENV mutation needed
    let headers = Headers::default()
        .use_bearer_token("test-token-12345")
        .build()
        .unwrap();

    let client = OpenAI::default()
        .variant_with_headers(headers);

    // Test with explicit credentials (isolated)
}
```

### Scenario 5: Credential Vaults/Secret Managers

**Before:**
```rust
// Load from vault and inject into ENV
let token = vault_client.get_secret("openai_token").await?;
std::env::set_var("OPENAI_API_KEY", token);

let client = OpenAI::default();
```

**After:**
```rust
// Load from vault and inject directly
let token = vault_client.get_secret("openai_token").await?;

let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### Scenario 6: Credential Rotation

**Before:**
```rust
// Update global ENV variable
std::env::set_var("OPENAI_API_KEY", new_token);

// Existing client picks up new token on next request
let response = client.request::<T>(req).await?;
```

**After:**
```rust
// Create new headers with rotated credential
let headers = Headers::default()
    .use_bearer_token(new_token)
    .build()?;

// Create new client variant with updated credentials
let client = client.variant_with_headers(headers);

let response = client.request::<T>(req).await?;
```

### Scenario 7: Basic Authentication

**Before:**
```rust
// Set ENV variables for Basic auth
std::env::set_var("EMQX_API_KEY", "username");
std::env::set_var("EMQX_API_SECRET", "password");

let client = EmqxBasic::default();
```

**After:**
```rust
// Option A: ENV-based (same as before)
std::env::set_var("EMQX_API_KEY", "username");
std::env::set_var("EMQX_API_SECRET", "password");

let client = EmqxBasic::default();

// Option B: Direct injection
let headers = Headers::default()
    .use_basic_auth("username", "password")
    .build()?;

let client = EmqxBasic::default()
    .variant_with_headers(headers);
```

### Scenario 8: API Key with Custom Header

**Before:**
```rust
// Set ENV variable
std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-...");

let client = Anthropic::default();
// Uses X-Api-Key header automatically
```

**After:**
```rust
// Option A: ENV-based
std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-...");
let client = Anthropic::default();

// Option B: Direct injection
let headers = Headers::default()
    .use_api_key("sk-ant-...", "X-Api-Key")
    .build()?;

let client = Anthropic::default()
    .variant_with_headers(headers);
```

### Scenario 9: Fallback ENV Variable Chains

**Before:**
```rust
// Definition specifies fallback chain
RestApi {
    env_auth: vec![
        "HUGGINGFACE_API_KEY".to_string(),
        "HF_TOKEN".to_string(),
        "HF_API_KEY".to_string(),
    ],
    // ...
}

// Client tries each in order at request time
let client = HuggingFaceHub::default();
```

**After:**
```rust
// Option A: ENV-based with same fallback chain
let mapping = EnvMapping {
    bearer_token: Some(EnvList::from_strs(&[
        "HUGGINGFACE_API_KEY",
        "HF_TOKEN",
        "HF_API_KEY",
    ])),
    basic_user: None,
    basic_pass: None,
    api_key: None,
};

let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env()
    .build()?;

let client = HuggingFaceHub::default()
    .variant_with_headers(headers);

// Option B: Direct injection (no fallback needed)
let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;

let client = HuggingFaceHub::default()
    .variant_with_headers(headers);
```

## API Changes

### Generated Client Methods

#### New Methods

| Method | Description |
|--------|-------------|
| `variant_with_headers(headers: Headers)` | Create a client variant with custom headers |

#### Unchanged Methods

| Method | Behavior |
|--------|----------|
| `new()` | Still attempts automatic ENV loading |
| `with_base_url(url)` | Still attempts automatic ENV loading |
| `variant()` | Returns variant builder (can set `headers_builder()`) |
| `variant_with(base_url, env_auth, strategy)` | Legacy method (ENV variable names only) |

### EnvMapping Structure

The `EnvMapping` type replaces scattered `env_auth`, `env_username` fields:

```rust
pub struct EnvMapping {
    pub bearer_token: Option<EnvList>,  // For BearerToken strategy
    pub basic_user: Option<EnvList>,    // For Basic auth username
    pub basic_pass: Option<EnvList>,    // For Basic auth password
    pub api_key: Option<ApiKeyEnv>,     // For ApiKey strategy
}
```

Each field supports a fallback chain via `EnvList`.

## Best Practices

### 1. Prefer Programmatic Injection for Applications

```rust
// GOOD: Direct injection
let headers = Headers::default()
    .use_bearer_token(get_token_from_vault())
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

```rust
// AVOID: Global ENV mutation
std::env::set_var("OPENAI_API_KEY", get_token_from_vault());
let client = OpenAI::default();
```

### 2. Use ENV Loading for CLI Tools

```rust
// GOOD for CLI: ENV with error on missing
let headers = Headers::default()
    .try_from_env()?  // Error if OPENAI_API_KEY not set
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### 3. Hybrid Approach for Flexibility

```rust
// GOOD: ENV with programmatic override
let headers = Headers::default()
    .from_env()               // Load defaults from ENV
    .use_bearer_token(override_token)  // Override if provided
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### 4. Always Use `variant_with_headers()` for Tests

```rust
// GOOD: Isolated test credentials
#[test]
fn test_api() {
    let headers = Headers::default()
        .use_bearer_token("test-token")
        .build()
        .unwrap();

    let client = OpenAI::default()
        .variant_with_headers(headers);

    // Test...
}
```

```rust
// AVOID: ENV mutation in tests
#[test]
fn test_api() {
    std::env::set_var("OPENAI_API_KEY", "test-token");
    let client = OpenAI::default();
    // Global state affects other tests
}
```

## Troubleshooting

### Issue: `MissingCredential` Error

**Problem:**
```
Error: MissingCredential { env_vars: ["OPENAI_API_KEY"] }
```

**Solution:**

Either set the ENV variable or use programmatic injection:

```rust
// Option A: Set ENV
std::env::set_var("OPENAI_API_KEY", token);

// Option B: Direct injection (recommended)
let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;

let client = OpenAI::default()
    .variant_with_headers(headers);
```

### Issue: Headers Not Applied

**Problem:**

Custom headers set via `.header()` don't appear in requests.

**Solution:**

Make sure to call `.build()` before passing to client:

```rust
// WRONG: Missing .build()
let headers = Headers::default()
    .use_bearer_token(token);
let client = OpenAI::default()
    .variant_with_headers(headers);  // Error: wrong type

// CORRECT: Call .build()
let headers = Headers::default()
    .use_bearer_token(token)
    .build()?;  // <-- Returns Vec<(String, String)>
let client = OpenAI::default()
    .variant_with_headers(headers);
```

### Issue: ENV Variables Not Loaded

**Problem:**

ENV variables are set but credentials not found.

**Solution:**

Use explicit ENV loading or check variable names:

```rust
// Explicit ENV loading
let headers = Headers::default()
    .from_env()  // or .try_from_env()? for errors
    .build()?;

// Or check ENV mapping
let mapping = EnvMapping {
    bearer_token: Some(EnvList::from_strs(&["OPENAI_API_KEY"])),
    // ...
};

let headers = Headers::default()
    .with_env_mapping(mapping)
    .from_env()
    .build()?;
```

### Issue: Multi-Tenant Credentials Leaking

**Problem:**

Different tenants using same credentials.

**Solution:**

Ensure each tenant gets its own client variant:

```rust
// WRONG: Shared client
let client = OpenAI::default();
// Both tenants use same client (and same ENV-based credentials)

// CORRECT: Separate variants per tenant
let tenant_a_client = OpenAI::default()
    .variant_with_headers(tenant_a_headers);

let tenant_b_client = OpenAI::default()
    .variant_with_headers(tenant_b_headers);
```

## Summary

| Scenario | Before (Legacy) | After (Headers API) |
|----------|----------------|-------------------|
| **Simple usage** | ENV variable required | ENV auto-loaded OR programmatic |
| **Multi-tenant** | Not possible (global ENV) | Separate headers per tenant |
| **Testing** | ENV mutation (global state) | Direct injection (isolated) |
| **Credential rotation** | Update global ENV | Create new variant |
| **Secret vaults** | Awkward (load into ENV) | Natural (inject directly) |
| **Fallback chains** | Defined in API definition | EnvList with priority order |

**Migration Path:**

1. **No changes required** for basic ENV-based usage (auto-compatible)
2. **Recommended**: Switch to programmatic injection for applications
3. **Testing**: Always use `variant_with_headers()` for test isolation
4. **Multi-tenant**: Use separate client variants with distinct headers
