# Integration Tests

Integration tests verify your public API from an external perspective. They live in the `tests/` directory.

## Structure

```
my_project/
├── src/
│   └── lib.rs
└── tests/
    ├── common/
    │   └── mod.rs      # Shared utilities (not a test file)
    ├── api_tests.rs    # Compiled as separate crate
    └── workflow_tests.rs
```

## Basic Integration Test

```rust
// tests/api_tests.rs
use my_crate::Calculator;

#[test]
fn calculator_adds_correctly() {
    let calc = Calculator::new();
    assert_eq!(calc.add(2, 3), 5);
}

#[test]
fn calculator_handles_negative() {
    let calc = Calculator::new();
    assert_eq!(calc.add(-5, 3), -2);
}
```

## Key Differences from Unit Tests

| Aspect | Unit Tests | Integration Tests |
|--------|------------|-------------------|
| Location | Same file as code | `tests/` directory |
| Access | All items (public + private) | Public API only |
| Compilation | Part of main crate | Separate crate per file |
| `use` statements | `use super::*;` | `use my_crate::...;` |

## Sharing Test Utilities

Place shared code in `tests/common/mod.rs` (not directly in `tests/`):

```rust
// tests/common/mod.rs
pub fn setup_test_env() {
    // Initialize logging, create temp dirs, etc.
}

pub fn create_test_data() -> Vec<Item> {
    vec![
        Item { id: 1, name: "Test".into() },
        Item { id: 2, name: "Sample".into() },
    ]
}
```

Use in test files:

```rust
// tests/workflow_tests.rs
mod common;

use my_crate::process_items;

#[test]
fn processes_all_items() {
    common::setup_test_env();
    let items = common::create_test_data();

    let result = process_items(&items);
    assert_eq!(result.len(), 2);
}
```

## Running Integration Tests

```bash
# Run all tests (unit + integration + doc)
cargo test

# Run only integration tests
cargo test --test api_tests

# Run specific test in integration file
cargo test --test api_tests calculator_adds

# Run all tests in tests/ directory
cargo test --tests
```

## Binary Crate Testing

For binary crates (`src/main.rs`), extract logic into a library:

```rust
// src/lib.rs - testable logic
pub fn run_app(config: Config) -> Result<(), Error> {
    // Application logic here
}

// src/main.rs - minimal wrapper
use my_crate::{Config, run_app};

fn main() {
    let config = Config::from_args();
    if let Err(e) = run_app(config) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

Then test the library:

```rust
// tests/app_integration.rs
use my_crate::{Config, run_app};

#[test]
fn app_runs_with_valid_config() {
    let config = Config::default();
    assert!(run_app(config).is_ok());
}
```

## Testing with External Resources

For tests requiring databases, APIs, or other services, consider:

1. **Test containers** - Spin up Docker containers
2. **Mock servers** - Use `mockito` for HTTP
3. **In-memory alternatives** - SQLite instead of Postgres

```rust
// tests/database_integration.rs
use testcontainers::{clients, images::postgres::Postgres};

#[test]
fn database_operations() {
    let docker = clients::Cli::default();
    let postgres = docker.run(Postgres::default());

    let connection_string = format!(
        "postgres://postgres:postgres@localhost:{}",
        postgres.get_host_port_ipv4(5432)
    );

    // Run tests against real database
    let pool = create_pool(&connection_string).unwrap();
    // ... test database operations
}
```

## Related

- [Unit Tests](./unit-tests.md) - Testing internal code
- [Mocking](./mocking.md) - Isolating external dependencies
- [nextest](./nextest.md) - Faster test execution
