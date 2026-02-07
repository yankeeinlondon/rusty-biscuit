# Environment Variable Testing

Patterns for safely testing code that reads or modifies environment variables.

## The Problem

Environment variables are process-global mutable state. When cargo test runs tests in parallel (default), tests that modify env vars can interfere with each other.

```rust
// DANGEROUS: These tests may run concurrently and fail randomly
#[test]
fn test_config_from_env() {
    std::env::set_var("API_KEY", "test-key");
    let config = Config::from_env();
    assert_eq!(config.api_key, "test-key");
    std::env::remove_var("API_KEY");  // May never run if assertion fails
}

#[test]
fn test_config_missing_env() {
    std::env::remove_var("API_KEY");  // Removes the var another test just set!
    let config = Config::from_env();
    assert!(config.api_key.is_none());
}
```

## Solution: serial_test

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_config_from_env() {
    unsafe { std::env::set_var("API_KEY", "test-key"); }
    let config = Config::from_env();
    assert_eq!(config.api_key, "test-key");
    unsafe { std::env::remove_var("API_KEY"); }
}

#[test]
#[serial]
fn test_config_missing_env() {
    unsafe { std::env::remove_var("API_KEY"); }
    let config = Config::from_env();
    assert!(config.api_key.is_none());
}
```

## RAII Pattern for Cleanup

Use an RAII guard to ensure env vars are restored even if the test panics.

```rust
use serial_test::serial;
use std::env;

/// RAII helper for temporarily setting environment variables
struct ScopedEnv {
    key: String,
    original: Option<String>,
}

impl ScopedEnv {
    fn set(key: &str, value: &str) -> Self {
        let original = env::var(key).ok();
        // SAFETY: Only used in serial tests - no concurrent env access
        unsafe { env::set_var(key, value); }
        Self { key: key.to_string(), original }
    }

    fn remove(key: &str) -> Self {
        let original = env::var(key).ok();
        // SAFETY: Only used in serial tests - no concurrent env access
        unsafe { env::remove_var(key); }
        Self { key: key.to_string(), original }
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        // SAFETY: Only used in serial tests - no concurrent env access
        unsafe {
            match &self.original {
                Some(val) => env::set_var(&self.key, val),
                None => env::remove_var(&self.key),
            }
        }
    }
}

#[test]
#[serial]
fn test_with_scoped_env() {
    let _env = ScopedEnv::set("MY_VAR", "test-value");
    // Test logic here...
    // _env automatically restores original value when dropped
}

#[test]
#[serial]
fn test_with_removed_env() {
    let _env = ScopedEnv::remove("MY_VAR");
    // MY_VAR is unset for this test
    // Restored when _env drops
}
```

## Named Groups for Env Var Categories

When tests modify different env vars, use named groups for better parallelism.

```rust
#[test]
#[serial(api_config)]
fn test_api_key() {
    unsafe { std::env::set_var("API_KEY", "key1"); }
    // ...
}

#[test]
#[serial(api_config)]
fn test_api_url() {
    unsafe { std::env::set_var("API_URL", "https://api.example.com"); }
    // ...
}

#[test]
#[serial(logging)]
fn test_log_level() {
    unsafe { std::env::set_var("RUST_LOG", "debug"); }
    // ...
}
```

Tests in `api_config` group serialize with each other, but can run in parallel with `logging` tests.

## Async Tests

serial_test works with async tests out of the box.

```rust
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_async_with_env() {
    unsafe { std::env::set_var("DATABASE_URL", "postgres://localhost/test"); }
    let pool = create_pool().await;
    // ...
    unsafe { std::env::remove_var("DATABASE_URL"); }
}
```

## Best Practices

1. **Always use RAII guards** - Tests that panic won't clean up manually removed env vars
2. **Use named groups** when tests modify non-overlapping env vars
3. **Document the reason** for `#[serial]` in a comment
4. **Prefer dependency injection** - Pass config structs instead of reading env vars directly
5. **Keep serial tests minimal** - Move non-env-var logic to helper functions

## Common Pitfalls

### Forgetting to cleanup

```rust
// BAD: If assertion fails, MY_VAR stays set
#[test]
#[serial]
fn test_bad_cleanup() {
    unsafe { std::env::set_var("MY_VAR", "value"); }
    assert!(some_check());  // If this fails...
    unsafe { std::env::remove_var("MY_VAR"); }  // ...this never runs
}

// GOOD: RAII ensures cleanup
#[test]
#[serial]
fn test_good_cleanup() {
    let _env = ScopedEnv::set("MY_VAR", "value");
    assert!(some_check());  // Even if this fails, cleanup happens
}
```

### Missing serial attribute

```rust
// BAD: Tests may run in parallel and interfere
#[test]
fn test_missing_serial() {
    unsafe { std::env::set_var("MY_VAR", "value"); }
    // ...
}
```

### Rust 2024 Edition Note

Starting with Rust 2024 edition, `std::env::set_var` and `std::env::remove_var` require `unsafe` blocks because they are inherently not thread-safe. The `#[serial]` attribute ensures only one test runs at a time, making the `unsafe` usage sound.
