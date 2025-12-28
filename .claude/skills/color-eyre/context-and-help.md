# Context and Help

Adding rich context, help text, and notes to errors for better debugging.

## The WrapErr Trait

The core mechanism for building error chains. Added to all `Result` types.

### Basic Usage

```rust
use color_eyre::eyre::{WrapErr, Result};

fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .wrap_err("Failed to read config file")?;

    toml::from_str(&content)
        .wrap_err("Invalid config format")
}
```

### Dynamic Context with wrap_err_with

Use when context needs runtime values:

```rust
fs::read_to_string(path)
    .wrap_err_with(|| format!("Failed to read file: {}", path.display()))?
```

**Why wrap_err instead of map_err?**
- Preserves the original error (accessible via downcasting)
- Creates a chain visible in output (`Caused by:` sections)
- Cleaner, more ergonomic syntax

## The Help Trait

Attach actionable suggestions to errors.

### with_help

```rust
use color_eyre::eyre::{Result, Help};

fn get_api_key() -> Result<String> {
    env::var("API_KEY")
        .map_err(|e| eyre::Report::new(e))
        .with_help(|| "Set the API_KEY environment variable:\n  export API_KEY=your_key")
}
```

### with_suggestion

Alias for `with_help`, use for alternative approaches:

```rust
config.get("port")
    .ok_or_else(|| eyre!("Missing port in config"))
    .with_suggestion(|| "Add 'port = 8080' to your config.toml")
```

## The Note Trait

Add debugging information that helps diagnose issues.

### with_note

```rust
fn parse_config(content: &str) -> Result<Config> {
    serde_json::from_str(content)
        .wrap_err("Invalid JSON config")
        .with_note(|| format!("File content was:\n---\n{}\n---", content))
}
```

Notes appear in the error report and help developers understand what went wrong.

## SectionExt Trait

Add custom sections to error reports.

```rust
use color_eyre::Section;

fn load_user(id: u64) -> Result<User> {
    db.find_user(id)
        .wrap_err("User not found")
        .section(format!("User ID: {}", id))
        .section("Check that the user exists in the database")
}
```

## Creating Ad-hoc Errors

Use the `eyre!` macro for one-off errors:

```rust
use color_eyre::eyre::eyre;

fn validate(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(eyre!("Value cannot be empty"));
    }
    if value.len() > 100 {
        return Err(eyre!("Value too long: {} chars (max 100)", value.len()));
    }
    Ok(())
}
```

## Combining Context Techniques

```rust
fn process_order(order_id: u64) -> Result<Invoice> {
    let order = db.get_order(order_id)
        .wrap_err_with(|| format!("Failed to fetch order {}", order_id))
        .with_note(|| format!("Database: {}", db.connection_string()))
        .with_help(|| "Ensure the database is accessible and order exists")?;

    let invoice = billing::create_invoice(&order)
        .wrap_err("Failed to create invoice")
        .with_suggestion(|| "Check billing service status")?;

    Ok(invoice)
}
```

## Error Output Structure

When printed, errors display:

```text
Error:
   0: Failed to process order 123
   1: Failed to create invoice
   2: Billing service unavailable

  ============================================================

Caused by:
    Connection refused (os error 111)

Note:
    Database: postgres://localhost:5432/orders

Help:
    Check billing service status

Backtrace omitted. Run with RUST_BACKTRACE=1 to display it.
```

## Related

- [Installation and Setup](./setup.md) - Basic setup
- [Comparison Guide](./comparison.md) - When to use color-eyre
