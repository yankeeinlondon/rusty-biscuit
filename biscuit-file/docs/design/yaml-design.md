# YAML Library Design

## Context

The core requirements define a `Yaml` struct that:

- Reads a YAML file and parses to an internal representation.
- Exports to JSON and TOML.
- Validates YAML (syntax and optionally schema).
- Provides ergonomic `From<T>` and `TryFrom<T>` implementations for common inputs.

This design translates those requirements into a concrete library API, error model, and conversion behavior while acknowledging known YAML conversion pitfalls.

## Goals

- Provide a stable `Yaml` struct with clear, typed APIs for parsing and conversion.
- Make conversion behavior explicit and configurable (lossy by default with warnings).
- Offer schema validation via JSON Schema (industry standard for YAML).
- Keep the public API ergonomic (multiple constructors, `TryFrom` implementations, minimal boilerplate).

## Non-Goals

- Format-preserving editing (comments, anchors, whitespace). This library is read/convert/validate.
- Full YAML 1.2 feature parity beyond what the selected parser supports.
- Round-trip fidelity across YAML <-> JSON/TOML conversions.

## Crate Selection

Primary choices are based on ecosystem status and compatibility:

- `serde_yaml_ng`: Maintained fork of `serde_yaml`, drop-in compatible and actively updated.
- `serde_json`: Required for JSON conversion and JSON Schema validation.
- `toml`: Provides `toml::Value` for TOML conversion targets.
- `jsonschema`: Standard JSON Schema validator used by YAML tooling (enabled by the `schema` feature).

Optional and future candidates:

- `serde-saphyr`: Alternative parser focused on panic-free parsing; possible backend feature in the future.
- `schemars`: For schema generation (used in examples, not required for core YAML library).

## Feature Flags

- `schema`: Enables JSON Schema validation via `jsonschema` and exposes `validate_schema` plus related types.

## Public API Design

### Core Type

```rust
pub struct Yaml {
    source: YamlSource,
    value: serde_yaml_ng::Value,
}

pub enum YamlSource {
    Path(std::path::PathBuf),
    Text(String),
    Bytes(Vec<u8>),
}
```

`Yaml` stores the parsed `Value` plus a lightweight source descriptor to improve error messages and debugging.

### Constructors

```rust
impl Yaml {
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self, YamlError>;
    pub fn from_str(input: impl AsRef<str>) -> Result<Self, YamlError>;
    pub fn from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, YamlError>;

    pub fn from_value(value: serde_yaml_ng::Value) -> Self;
    pub fn value(&self) -> &serde_yaml_ng::Value;
}
```

### Conversions

```rust
impl Yaml {
    pub fn as_json(&self) -> Result<serde_json::Value, YamlError>;
    pub fn as_json_with(&self, options: JsonConversionOptions)
        -> Result<ConversionOutput<serde_json::Value>, YamlError>;

    pub fn as_toml(&self) -> Result<toml::Value, YamlError>;
    pub fn as_toml_with(&self, options: TomlConversionOptions)
        -> Result<ConversionOutput<toml::Value>, YamlError>;
}
```

`as_json` and `as_toml` are convenience defaults. The `_with` variants return a `ConversionOutput` that includes warnings and policy decisions.

### Validation

```rust
impl Yaml {
    pub fn validate(&self) -> Result<(), YamlError>;
    #[cfg(feature = "schema")]
    pub fn validate_schema(&self, schema: JsonSchemaInput)
        -> Result<ValidationReport, YamlError>;
}

#[cfg(feature = "schema")]
pub enum JsonSchemaInput {
    Path(std::path::PathBuf),
    Text(String),
    Json(serde_json::Value),
}
```

- `validate` ensures the YAML is syntactically valid and parsed. It is effectively a no-op after construction but keeps the API consistent with other formats.
- `validate_schema` compiles and applies JSON Schema to the YAML after conversion to `serde_json::Value` (only available with the `schema` feature).

### TryFrom Implementations

```rust
impl TryFrom<&str> for Yaml;
impl TryFrom<String> for Yaml;
impl TryFrom<std::path::PathBuf> for Yaml;
impl TryFrom<&std::path::Path> for Yaml;
impl TryFrom<Vec<u8>> for Yaml;

impl From<serde_yaml_ng::Value> for Yaml;
```

The goal is to cover the typical entry points: file path, raw YAML, or pre-parsed values.

## Conversion Design

YAML conversion is inherently lossy. The library should make these loss points explicit via policies and warnings.

### YAML to JSON

#### Known Pitfalls

- Non-string map keys (YAML allows complex keys; JSON does not).
- Anchors and aliases (graph vs tree); cycles can cause stack overflows.
- NaN and infinity values; JSON does not support them.

#### Conversion Strategy

1. Parse YAML into `serde_yaml_ng::Value`.
2. Convert to `serde_json::Value` using a custom traversal that normalizes unsupported constructs.
3. Produce a `ConversionOutput` that includes warnings for lossy decisions.

#### Options

```rust
pub struct JsonConversionOptions {
    pub key_policy: NonStringKeyPolicy,
    pub float_policy: NonFiniteFloatPolicy,
    pub alias_policy: AliasPolicy,
    pub max_depth: Option<usize>,
}

pub enum NonStringKeyPolicy {
    Error,
    Stringify,
    Drop,
}

pub enum NonFiniteFloatPolicy {
    Error,
    Null,
    Stringify,
}

pub enum AliasPolicy {
    Expand,
    ErrorOnCycle,
}
```

Defaults should be `Stringify` for keys, `Null` for non-finite floats, and `ErrorOnCycle` to avoid stack overflow on circular aliases.

### YAML to TOML

#### Known Pitfalls

- YAML `null` has no TOML equivalent.
- YAML arrays can be heterogeneous; TOML arrays must be homogeneous.
- Deeply nested YAML structures may not map cleanly to TOML tables.

#### Conversion Strategy

1. Traverse the YAML value, attempting to coerce into `toml::Value` types.
2. Apply policies for `null`, heterogeneous arrays, and deep nesting.
3. Produce a `ConversionOutput` with warnings.

#### Options

```rust
pub struct TomlConversionOptions {
    pub null_policy: NullPolicy,
    pub array_policy: HeteroArrayPolicy,
    pub table_policy: TableNestingPolicy,
    pub max_depth: Option<usize>,
}

pub enum NullPolicy {
    Drop,
    Stringify,
    Error,
}

pub enum HeteroArrayPolicy {
    Error,
    Stringify,
    ArrayOfTables,
}

pub enum TableNestingPolicy {
    Error,
    Flatten,
}
```

Defaults should be `Drop` for nulls, `Error` for heterogeneous arrays, and `Error` for overly deep nesting.

### Conversion Output

```rust
pub struct ConversionOutput<T> {
    pub value: T,
    pub warnings: Vec<ConversionWarning>,
}

pub enum ConversionWarning {
    NonStringKey { path: String },
    NonFiniteFloat { path: String },
    NullDropped { path: String },
    HeteroArrayCoerced { path: String },
    TableFlattened { path: String },
}
```

Warnings are optional but make lossy decisions visible to callers.

## Validation Design

### Schema Validation Flow

1. Accept schema input as a path, raw JSON text, or `serde_json::Value`.
2. Parse and compile using `jsonschema::validator_for`.
3. Convert YAML to `serde_json::Value` using `as_json_with` and fail on conversion errors.
4. Validate, returning a `ValidationReport` with structured errors.

This section is gated behind the `schema` feature. Without it, only `validate` is available.

### Validation Types

```rust
#[cfg(feature = "schema")]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<SchemaError>,
}

#[cfg(feature = "schema")]
pub struct SchemaError {
    pub instance_path: String,
    pub message: String,
}
```

This mirrors `jsonschema` error structures and keeps output stable for CLI or API consumers.

## Error Model

Use a single public error type with internal variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum YamlError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Parse(#[from] serde_yaml_ng::Error),
    #[error("JSON conversion error: {0}")]
    JsonConversion(String),
    #[error("TOML conversion error: {0}")]
    TomlConversion(String),
    #[error("Schema parse error: {0}")]
    SchemaParse(#[from] serde_json::Error),
    #[error("Schema invalid: {0}")]
    SchemaInvalid(String),
}
```

## Module Layout

Proposed library structure:

```
src/
  yaml/
    mod.rs
    convert.rs
    schema.rs
    error.rs
```

- `mod.rs`: public API (`Yaml`, constructors, conversion methods).
- `convert.rs`: conversion logic and policies.
- `schema.rs`: JSON Schema validation types and logic (behind the `schema` feature).
- `error.rs`: `YamlError` and shared error types.

## Testing Plan

- Unit tests for parsing (valid/invalid YAML, edge cases).
- Conversion tests for JSON and TOML with policy variations.
- Schema validation tests using `jsonschema` with YAML inputs (behind the `schema` feature).
- Snapshot tests for schemas and conversion output using the golden test approach described in `docs/testing/golden-tests.md`.

## Open Questions

- Should we expose a `YamlSchema` type to precompile and reuse validators?
- Do we need a strict backend option (`serde-saphyr`) for untrusted inputs?

## Summary

This design provides a clear `Yaml` API, explicit conversion policies, and schema validation that matches real-world YAML tooling. It acknowledges unavoidable lossiness in YAML conversion and makes those decisions visible and configurable while keeping the default path simple for common use cases.
