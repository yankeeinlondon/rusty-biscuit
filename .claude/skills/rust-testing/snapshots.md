# Snapshot Testing with Insta

Snapshot testing captures output and compares against stored "golden" files. Use `insta` for this in Rust.

## Setup

```toml
# Cargo.toml
[dev-dependencies]
insta = "1"

# For JSON snapshots
insta = { version = "1", features = ["json"] }

# For YAML snapshots
insta = { version = "1", features = ["yaml"] }
```

Install CLI for reviewing snapshots:

```bash
cargo install cargo-insta
```

## Basic Usage

```rust
use insta::assert_snapshot;

#[test]
fn test_greeting() {
    let greeting = format_greeting("World");
    assert_snapshot!(greeting);
}
```

First run creates `snapshots/module__test_greeting.snap`:

```
---
source: src/lib.rs
expression: greeting
---
Hello, World!
```

## Reviewing Snapshots

```bash
# Review pending snapshots interactively
cargo insta review

# Accept all pending snapshots
cargo insta accept

# Reject all pending snapshots
cargo insta reject

# Show pending changes
cargo insta pending-snapshots
```

## Snapshot Types

### String Snapshots

```rust
#[test]
fn test_html_output() {
    let html = render_page("Home");
    assert_snapshot!(html);
}
```

### Debug Snapshots

```rust
#[test]
fn test_struct_debug() {
    let config = Config::default();
    assert_debug_snapshot!(config);
}
```

### JSON Snapshots

```rust
use insta::assert_json_snapshot;

#[test]
fn test_api_response() {
    let response = api::get_user(1);
    assert_json_snapshot!(response);
}
```

### YAML Snapshots

```rust
use insta::assert_yaml_snapshot;

#[test]
fn test_config_serialization() {
    let config = Config::load("test.yaml");
    assert_yaml_snapshot!(config);
}
```

## Named Snapshots

```rust
#[test]
fn test_multiple_outputs() {
    let (header, body, footer) = render_page();

    assert_snapshot!("header", header);
    assert_snapshot!("body", body);
    assert_snapshot!("footer", footer);
}
```

## Inline Snapshots

Store snapshot in the test file:

```rust
#[test]
fn test_inline() {
    let result = calculate(42);
    assert_snapshot!(result, @"expected value here");
}
```

Run `cargo insta review` to populate the inline value.

## Redacting Dynamic Values

Remove timestamps, IDs, or other changing values:

```rust
use insta::{assert_json_snapshot, sorted_redaction};

#[test]
fn test_with_dynamic_id() {
    let user = create_user();

    assert_json_snapshot!(user, {
        ".id" => "[ID]",
        ".created_at" => "[TIMESTAMP]",
        ".sessions[].token" => "[TOKEN]",
    });
}
```

## Settings

Configure snapshot behavior:

```rust
use insta::Settings;

#[test]
fn test_with_settings() {
    let mut settings = Settings::clone_current();
    settings.set_snapshot_path("custom_snapshots/");
    settings.set_prepend_module_to_snapshot(false);

    settings.bind(|| {
        assert_snapshot!(compute_result());
    });
}
```

## Snapshot Location

Default location: `src/snapshots/` or `tests/snapshots/`

Structure:

```
src/
├── lib.rs
└── snapshots/
    ├── lib__test_greeting.snap
    └── lib__test_html_output.snap
```

## CI Integration

Fail CI if snapshots are pending:

```bash
# In CI, run tests then check for pending snapshots
cargo test
cargo insta pending-snapshots --check
```

GitHub Actions example:

```yaml
- name: Run tests
  run: cargo test

- name: Check snapshots
  run: |
    cargo install cargo-insta
    cargo insta pending-snapshots --check
```

## When to Use Snapshots

**Good for:**

- Complex output (HTML, JSON responses)
- Compiler/transpiler output
- Formatted text
- Serialized data structures
- Error messages

**Less suitable for:**

- Simple values (use `assert_eq!`)
- Floating point numbers
- Order-dependent collections
- Frequently changing output

## Best Practices

1. **Review changes carefully** - Don't blindly accept
2. **Keep snapshots small** - Split large outputs
3. **Use redactions** for dynamic values
4. **Name snapshots meaningfully** when using multiple per test
5. **Commit snapshots** to version control
6. **Update snapshots intentionally** - Document why in commit message

## Example: Testing a Formatter

```rust
use insta::assert_snapshot;

fn format_code(input: &str) -> String {
    // Your formatting logic
    todo!()
}

#[test]
fn test_format_function() {
    let input = r#"
fn main(){let x=1;println!("{}",x);}
"#;
    assert_snapshot!(format_code(input));
}

#[test]
fn test_format_struct() {
    let input = r#"
struct Point{x:i32,y:i32}
"#;
    assert_snapshot!(format_code(input));
}
```

## Related

- [Unit Tests](./unit-tests.md) - Basic testing
- [Property Testing](./property-testing.md) - Testing with many inputs
