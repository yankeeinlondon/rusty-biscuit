# File System Test Isolation

Cross-process test serialization using file-based locking.

## When to Use file_serial

| Scenario | Use `#[serial]` | Use `#[file_serial]` |
|----------|-----------------|----------------------|
| Unit tests in same crate | Yes | No |
| Integration tests (`tests/`) | Maybe | Often needed |
| Doctests (`///`) | No | Yes |
| Different crates in workspace | No | Yes |
| Same test binary | Yes | Either works |

Key distinction: `#[serial]` uses in-memory locks that only work within the same process. Doctests and integration tests run as separate processes and need file-based locking.

## Basic Usage

Requires the `file_locks` feature:

```toml
[dev-dependencies]
serial_test = { version = "3.3", features = ["file_locks"] }
```

```rust
use serial_test::file_serial;

#[test]
#[file_serial]
fn test_writes_to_shared_file() {
    std::fs::write("/tmp/shared.txt", "test data").unwrap();
    // ...
    std::fs::remove_file("/tmp/shared.txt").ok();
}

#[test]
#[file_serial]
fn test_reads_shared_file() {
    // This test won't run until the above completes
    let data = std::fs::read_to_string("/tmp/shared.txt").unwrap_or_default();
    // ...
}
```

## Custom Lock File Path

Override the default temp directory lock location:

```rust
#[test]
#[file_serial(shared_file, path => "/tmp/my-project-lock")]
fn test_with_custom_lock() {
    // Lock file at /tmp/my-project-lock
}
```

## Named Groups with File Locks

Like `#[serial]`, file locks support named groups:

```rust
#[test]
#[file_serial(database)]
fn test_db_write_1() { }

#[test]
#[file_serial(database)]
fn test_db_write_2() { }  // Serializes with test_db_write_1

#[test]
#[file_serial(cache)]
fn test_cache_clear() { }  // Can run in parallel with database tests
```

## file_parallel

Allow concurrent execution within a group while blocking file_serial tests:

```rust
#[test]
#[file_serial(data)]
fn test_modifies_data() { }

#[test]
#[file_parallel(data)]
fn test_reads_data_1() { }

#[test]
#[file_parallel(data)]
fn test_reads_data_2() { }  // Can run with test_reads_data_1
```

## Mixing serial and file_serial (AVOID)

There are **no guarantees** when mixing `#[serial]` and `#[file_serial]`:

```rust
// DANGEROUS: These may run concurrently!
#[test]
#[serial(shared)]
fn test_a() { }  // In-memory lock

#[test]
#[file_serial(shared)]
fn test_b() { }  // File-based lock

// These use different locking mechanisms and don't coordinate!
```

**Best Practice**: Choose one locking mechanism per shared resource and use it consistently.

## Lock File Location

| Platform | Default Location |
|----------|------------------|
| macOS | `$TMPDIR/serial_test_*` or `/tmp/serial_test_*` |
| Linux | `/tmp/serial_test_*` |
| Windows | `%TEMP%\serial_test_*` |

The lock files are automatically cleaned up when the lock is released.

## Doctest Example

Use file_serial in documentation tests:

```rust
/// Opens the shared database.
///
/// ```
/// # use serial_test::file_serial;
/// #[file_serial(database)]
/// fn test_db_open() {
///     let db = Database::open("/tmp/test.db").unwrap();
///     // ...
/// }
/// ```
pub fn open_database() { }
```

## Integration Test Pattern

For tests in `tests/` directory that share resources:

```rust
// tests/integration_tests.rs
use serial_test::file_serial;

#[test]
#[file_serial]
fn test_creates_state_file() {
    std::fs::write("./test-state.json", "{}").unwrap();
}

#[test]
#[file_serial]
fn test_reads_state_file() {
    let state = std::fs::read_to_string("./test-state.json").unwrap();
    assert_eq!(state, "{}");
}
```

## Best Practices

1. **Use for cross-process isolation** - Integration tests, doctests, multi-crate workspaces
2. **Prefer #[serial] for unit tests** - Lower overhead, simpler
3. **Don't mix locking mechanisms** - Pick one per resource
4. **Use custom paths for clarity** - Self-documenting lock files
5. **Clean up test artifacts** - Lock files are cleaned, but your test files aren't

## Checking Lock Status

```rust
use serial_test::is_locked_file_serially;

fn some_test() {
    if is_locked_file_serially() {
        // Currently holding a file lock
    }
}
```
