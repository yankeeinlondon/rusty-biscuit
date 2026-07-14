# Error Definition Patterns

Patterns for defining custom error types with thiserror.

## Enum Error Types

The most common pattern - define variants for each error case:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataError {
    // Unit variant - no additional data
    #[error("Operation not supported")]
    NotSupported,

    // Tuple variant - positional fields
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    // Struct variant - named fields (preferred for clarity)
    #[error("Resource limit of {limit} exceeded for user '{user}'")]
    ResourceLimitExceeded { limit: usize, user: String },
}
```

**When to use:** Most library error types should be enums to allow callers to match on specific cases.

## Struct Error Types

For single-purpose errors or when an enum would have only one variant:

```rust
#[derive(Error, Debug)]
#[error("Connection failed to {host}:{port}")]
pub struct ConnectionError {
    pub host: String,
    pub port: u16,
    #[source]
    pub source: std::io::Error,
}
```

**When to use:** Rare - prefer enums for extensibility.

## Field Interpolation

Reference fields in error messages using format string syntax:

```rust
#[derive(Error, Debug)]
pub enum FormatExamples {
    // Tuple field by index
    #[error("Error code: {0}")]
    Code(i32),

    // Multiple tuple fields
    #[error("Range error: {0} not in [{1}, {2}]")]
    Range(i32, i32, i32),

    // Named fields
    #[error("User '{name}' (id: {id}) not found")]
    UserNotFound { name: String, id: u64 },

    // Using Display trait
    #[error("Parse error: {value}")]
    Parse { value: String },

    // Using Debug trait
    #[error("Unexpected state: {state:?}")]
    UnexpectedState { state: Vec<u8> },
}
```

## Accessing Nested Fields

You can access methods or nested fields:

```rust
#[derive(Error, Debug)]
pub enum MyError {
    // Call a method on the field
    #[error("IO error (kind: {0:?})")]
    Io(std::io::Error),

    // Access nested struct field (requires the field to impl Display)
    #[error("Config error in section '{section}'")]
    Config { section: String },
}
```

## Visibility and Public APIs

Error types in library public APIs should be:

```rust
/// Errors that can occur during parsing.
#[derive(Error, Debug)]
#[non_exhaustive]  // Allow adding variants without breaking changes
pub enum ParseError {
    #[error("Unexpected token '{token}' at position {position}")]
    UnexpectedToken { token: String, position: usize },

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
}
```

**Best practice:** Use `#[non_exhaustive]` on public error enums to maintain backward compatibility.

## Related

- [Error Chaining](./error-chaining.md) - Connecting errors with #[from] and #[source]
