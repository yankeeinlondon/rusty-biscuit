# TOML Library Design

## Context

The Biscuit File library will provide consistent TOML utilities alongside YAML and PDF.
Per `docs/core-structs-and-features.md`, the TOML core is a `Toml` struct with:

- `as_json()`
- `as_yaml()`
- `validate()`

This design focuses on the library implementation required to deliver those features
and a clean, extensible API surface.

## Goals

- Read a TOML file from disk and parse into a stable internal representation.
- Provide ergonomic conversions to JSON and YAML output.
- Provide a validation API with clear errors and optional schema support.
- Offer `TryFrom` implementations for string variants and paths.
- Keep the API consistent with YAML and PDF modules (similar constructors and errors).

## Non-goals

- Format-preserving TOML editing (comments, whitespace, ordering) in v1.
- TOML formatting or pretty-printing beyond conversion output.
- Language-server or editor tooling features.
- Auto-repair or auto-fix of invalid TOML.

## Core Decisions

- Use `toml` crate for parsing into `toml::Value` (TOML 1.0 compliant).
- Use serde-based conversion to `serde_json` and `serde_yaml`.
- Treat TOML date/time types as strings when converting to JSON/YAML.
- Keep schema validation optional via a feature flag and a separate validation module.

## Public API

### Primary type

```rust
pub struct Toml {
    source: TomlSource,
    raw: String,
    value: toml::Value,
}
```

### Source tracking

```rust
pub enum TomlSource {
    Path(std::path::PathBuf),
    Inline,
}
```

### Construction

```rust
impl Toml {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self, TomlError>;
    pub fn from_str(input: impl AsRef<str>) -> Result<Self, TomlError>;
    pub fn from_reader<R: std::io::Read>(reader: R) -> Result<Self, TomlError>;

    pub fn value(&self) -> &toml::Value;
    pub fn raw(&self) -> &str;
    pub fn source(&self) -> &TomlSource;
}
```

### Conversion

```rust
impl Toml {
    pub fn as_json(&self) -> Result<String, TomlError>;
    pub fn as_yaml(&self) -> Result<String, TomlError>;

    pub fn as_json_value(&self) -> Result<serde_json::Value, TomlError>;
    pub fn as_yaml_value(&self) -> Result<serde_yaml::Value, TomlError>;
}
```

### Validation

```rust
impl Toml {
    pub fn validate(&self) -> Result<ValidationReport, TomlError>;
    pub fn validate_with_schema(&self, schema: SchemaSource)
        -> Result<ValidationReport, TomlError>;
}
```

### Trait implementations

```rust
impl TryFrom<&str> for Toml;
impl TryFrom<String> for Toml;
impl TryFrom<&std::path::Path> for Toml;
impl TryFrom<std::path::PathBuf> for Toml;
impl TryFrom<&std::ffi::OsStr> for Toml;
impl TryFrom<std::ffi::OsString> for Toml;
```

Notes:

- `TryFrom<&str>` and `TryFrom<String>` interpret inputs as file paths.
- `from_str` is the explicit constructor for raw TOML content.

## Internal Representation

- Parse input with `toml::from_str` into `toml::Value`.
- Store the raw content to enable diagnostics and re-validation.
- If needed later, a `Document`-style variant can be added for
  format-preserving edits (via `toml_edit`), without breaking the API.

## Conversion Design

### TOML to JSON

Mapping rules:

- TOML tables -> JSON objects
- Arrays -> JSON arrays
- Strings/bools -> JSON strings/bools
- Integers/floats -> JSON numbers
- Datetime/date/time -> JSON string (RFC 3339 or TOML canonical form)

Implementation approach:

- Convert `toml::Value` into `serde_json::Value` using serde.
- Serialize with `serde_json::to_string_pretty` (default) and provide
  compact output in a future options struct.

Gotchas:

- Dotted keys expand into nested objects; original dotted syntax is not preserved.
- Inline tables lose inline formatting (expected for conversions).

### TOML to YAML

Mapping rules are analogous to JSON, using `serde_yaml`.
Datetime/date/time values must be stringified before serialization.

Implementation approach:

- Convert to `serde_yaml::Value` using serde.
- Serialize with `serde_yaml::to_string`.

### Ordering

By default `toml::Value` uses map ordering that may not reflect input order.
If deterministic output is needed for tests, enable a crate feature that
turns on `toml` with `preserve_order` (backed by `indexmap`).

## Validation Design

### Levels

1. Syntactic validation
   - Parsing during construction already guarantees basic validity.

2. Semantic validation (optional feature)
   - Validate duplicates and structural correctness beyond parse errors.
   - Implement via `taplo` or a lightweight semantic pass.

3. Schema validation (optional feature)
   - Validate against JSON Schema.
   - Support schema sources:
     - Inline directive: `# schema: <url or path>` at top of file
     - Explicit schema source passed to `validate_with_schema`

### Validation report

```rust
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

pub struct ValidationIssue {
    pub level: ValidationLevel,
    pub message: String,
    pub span: Option<Span>,
}

pub enum ValidationLevel {
    Error,
    Warning,
}
```

If schema validation is not enabled, `validate_with_schema` returns a
feature-disabled error.

## Error Model

```rust
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Schema validation error: {0}")]
    Schema(String),

    #[error("Schema support is not enabled")]
    SchemaFeatureDisabled,
}
```

## Module Layout

```
src/
  lib.rs
  toml/
    mod.rs
    parse.rs
    convert.rs
    validate.rs
    types.rs
```

- `toml/mod.rs` exposes `Toml` and public error/validation types.
- `parse.rs` handles I/O and construction.
- `convert.rs` handles JSON/YAML conversion logic.
- `validate.rs` contains schema and semantic validation logic.

## Dependencies

Required:

- `toml`
- `serde`
- `serde_json`
- `serde_yaml`
- `thiserror`

Optional (feature-gated):

- `taplo` for semantic validation and schema handling
- `jsonschema` for direct JSON Schema validation if preferred
- `indexmap` via `toml` `preserve_order` feature

## Testing Strategy

- Unit tests for:
  - `TryFrom` constructors and path handling
  - Parsing valid/invalid TOML
  - Conversion outputs for typical TOML structures
  - Datetime conversion behavior
- Golden tests for JSON and YAML output using `insta` to lock output.
- Schema validation tests (feature-gated) using sample schemas and inputs.

## Example Usage

```rust
let toml = Toml::new("config/app.toml")?;
let json = toml.as_json()?;
let yaml = toml.as_yaml()?;
let report = toml.validate()?;
```

## Future Extensions

- Add a format-preserving `TomlEdit` type backed by `toml_edit`.
- Add conversion from JSON/YAML into TOML via a shared intermediate model.
- Provide `as_json_pretty` and `as_yaml_pretty` options or a formatter config.
