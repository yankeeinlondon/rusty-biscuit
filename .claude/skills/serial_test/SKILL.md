---
name: serial_test
description: Expert knowledge for the Rust serial_test crate — isolating tests that share global state such as environment variables, files, and singletons. Use when parallel tests interfere via shared state, tests pass alone but fail together, or you need #[serial]/#[parallel] to force serialized execution.
---

# serial_test

Rust crate for serializing test execution when tests share mutable global state.

## Quick Reference

```rust
use serial_test::{serial, parallel, file_serial, file_parallel};

// Basic serial - runs exclusively, never concurrently with other serial tests
#[test]
#[serial]
fn test_env_var_manipulation() {
    std::env::set_var("MY_VAR", "value");
    // ... test logic
    std::env::remove_var("MY_VAR");
}

// Named groups - tests with same key serialize together, different keys can run in parallel
#[test]
#[serial(database)]
fn test_db_write() { }

#[test]
#[serial(cache)]
fn test_cache_clear() { }  // Can run with test_db_write

#[test]
#[serial(database, cache)]
fn test_db_and_cache() { }  // Blocks both groups

// Parallel - runs concurrently with other parallel tests, but not with serial tests
#[test]
#[parallel]
fn test_read_only() { }

// File-based locking - for cross-process isolation (doctests, integration tests)
#[test]
#[file_serial]
fn test_shared_file() { }

#[test]
#[file_serial(key, path => "/tmp/my-lock")]
fn test_with_custom_lock() { }
```

## Cargo.toml

```toml
[dev-dependencies]
serial_test = "3.3"          # Default: async + logging
serial_test = { version = "3.3", features = ["file_locks"] }  # Add file_serial/file_parallel
```

## When to Use Which Attribute

| Scenario | Attribute | Why |
|----------|-----------|-----|
| Environment variables | `#[serial]` | `env::set_var` is process-global |
| Singleton/static mut | `#[serial]` | Shared mutable state |
| File system paths | `#[serial]` or `#[file_serial]` | Depends on test process model |
| Read-only operations | `#[parallel]` | Safe for concurrent access |
| Database writes | `#[serial(db)]` | Named group for DB tests |
| Mixed read/write | `#[parallel]` for reads, `#[serial]` for writes | Explicit concurrency control |

## Details

- [Environment Variable Testing](env-vars.md) - Patterns for safe env var manipulation
- [Named Groups](named-groups.md) - Organizing tests by resource type
- [File System Isolation](file-isolation.md) - Cross-process test serialization
- [Test Runner Integration](test-runners.md) - cargo test and nextest compatibility
