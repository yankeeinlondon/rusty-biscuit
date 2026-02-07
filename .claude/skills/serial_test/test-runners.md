# Test Runner Integration

How serial_test works with cargo test and nextest.

## cargo test

serial_test is designed for `cargo test` out of the box.

```bash
# Normal test run - serial tests are serialized automatically
cargo test

# Run specific serial test
cargo test test_name

# Run all tests in a module
cargo test module_name::
```

### Parallel Test Threads

By default, `cargo test` runs tests in parallel using multiple threads. Tests marked with `#[serial]` use in-memory locks to coordinate.

```bash
# Reduce parallelism (useful for debugging)
cargo test -- --test-threads=1

# Maximum parallelism (default)
cargo test -- --test-threads=<num_cpus>
```

Setting `--test-threads=1` makes ALL tests serial, but is slower than using `#[serial]` selectively.

## cargo-nextest

[Nextest](https://nexte.st) runs each test in a separate process, which affects how serial_test works.

### Key Difference: Process Isolation

| Feature | cargo test | nextest |
|---------|------------|---------|
| Test isolation | Threads in one process | Separate processes |
| `#[serial]` works | Yes | No (different processes) |
| `#[file_serial]` works | Yes | Yes |

**Important**: `#[serial]` only works within a single process. Since nextest runs each test in its own process, you must use `#[file_serial]` for cross-test serialization.

### Nextest Configuration

Nextest has built-in test groups that provide similar functionality:

```toml
# .config/nextest.toml

[test-groups]
# Create a serial group with max 1 concurrent test
serial-env = { max-threads = 1 }

[[profile.default.overrides]]
# Apply to all tests containing "env" in their name
filter = 'test(/env/)'
test-group = 'serial-env'

[[profile.default.overrides]]
# Or apply to a specific module
filter = 'package(my-crate) & test(/config_tests/)'
test-group = 'serial-env'
```

This is equivalent to using `#[serial]` but works across processes.

### Choosing Between Approaches

| Scenario | cargo test | nextest |
|----------|------------|---------|
| Env vars in unit tests | `#[serial]` | `#[file_serial]` or test-groups |
| Shared files | `#[serial]` or `#[file_serial]` | `#[file_serial]` or test-groups |
| Database tests | `#[serial(db)]` | `#[file_serial(db)]` or test-groups |
| Maximum parallelism | Named groups | test-groups |

### Migration Guide: cargo test to nextest

If you're switching from `cargo test` to nextest:

1. **Replace `#[serial]` with `#[file_serial]`** for tests that need isolation:

```rust
// Before (cargo test only)
#[test]
#[serial]
fn test_env_var() { }

// After (works with both)
#[test]
#[file_serial]
fn test_env_var() { }
```

2. **Or use nextest's test-groups** in `.config/nextest.toml`:

```toml
[test-groups]
env-tests = { max-threads = 1 }

[[profile.default.overrides]]
filter = 'test(#[serial])'  # Won't work - need filter by name/module
test-group = 'env-tests'
```

### Best Practice: Support Both Runners

Use `#[file_serial]` if you need to support both `cargo test` and nextest:

```rust
use serial_test::file_serial;

#[test]
#[file_serial]
fn test_works_everywhere() {
    // Works with cargo test and nextest
}
```

The overhead of file locking is minimal, and this approach is the most portable.

## CI Considerations

### GitHub Actions

```yaml
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test
        # Or with nextest:
        # run: cargo nextest run
```

### Parallelism in CI

CI environments often have many CPU cores. Consider:

- **cargo test**: Tests parallelize well with `#[serial]` groups
- **nextest**: Configure `max-threads` per test group appropriately

### Flaky Test Detection

If tests fail intermittently, check for:

1. Missing `#[serial]` on tests that share state
2. Mixing `#[serial]` and `#[file_serial]` for the same resource
3. Tests that depend on global state not properly isolated

```bash
# Run tests multiple times to detect flakiness
for i in {1..10}; do cargo test || exit 1; done
```

## Feature Flags Summary

```toml
[dev-dependencies]
# Default: async + logging
serial_test = "3.3"

# Add file locking for nextest compatibility
serial_test = { version = "3.3", features = ["file_locks"] }

# Minimal (no async, no logging)
serial_test = { version = "3.3", default-features = false }
```

## MSRV

Minimum Supported Rust Version: **1.68.2**

The crate tracks a fairly recent MSRV and may bump it in minor versions.
