# biscuit-file

Core library for file format parsing, conversion, and detection.

## Feature Flags

| Feature | Default | Description |
|---------|:-------:|-------------|
| `toml` | yes | TOML parsing and conversion |
| `yaml` | yes | YAML parsing and conversion |
| `json5` | yes | JSON5 parsing and conversion via `json-five` |
| `extract` | yes | PDF text extraction via `pdf-extract` |
| `lopdf` | yes | PDF table-of-contents extraction via `lopdf` |
| `pdfium` | no | High-fidelity PDF extraction via `pdfium-render` |
| `schema` | no | JSON Schema validation for TOML/YAML |
| `full` | no | All features enabled |

## Key Types

### `FileType` / `detect_file_type`

Automatic file type detection using magic bytes (PDF) and file extensions.

```rust
use biscuit_file::{detect_file_type, FileType};

let ft = detect_file_type("config.toml")?;
assert_eq!(ft, FileType::Toml);
assert_eq!(ft.extension(), Some("toml"));
assert_eq!(ft.mime_type(), Some("application/toml"));
```

Recognized extensions: `.toml`, `.yaml`, `.yml`, `.json`, `.json5`, `.md`, `.markdown`, `.mdx`, `.pdf`

### `Toml`

Parse TOML from files, strings, or readers. Convert to JSON or YAML.

```rust
use biscuit_file::Toml;

// From a file
let toml = Toml::new("config.toml")?;

// From a string
let toml = Toml::from_str("[package]\nname = \"demo\"")?;

// Convert
let json: String = toml.as_json()?;
let json_value: serde_json::Value = toml.as_json_value()?;
let yaml: String = toml.as_yaml()?;       // requires `yaml` feature
let raw: &str = toml.raw();                // original TOML text
```

### `Json5`

Parse JSON5 from files, strings, or bytes. Convert to JSON, YAML, or TOML.

```rust
use biscuit_file::Json5;

let j = Json5::new("config.json5")?;
let j = Json5::from_str("{ key: 'value', /* comment */ }")?;

// Convert
let json: String = j.as_json()?;
let json_compact: String = j.as_json_compact()?;
let json5: String = j.as_json5();            // pretty, idiomatic JSON5
let json5_compact: String = j.as_json5_compact(); // single-line JSON5
let yaml: String = j.as_yaml()?;             // requires `yaml` feature
let toml: String = j.as_toml()?;             // requires `toml` feature
```

The JSON5 formatter outputs idiomatic syntax: unquoted keys (when valid identifiers), single-quoted strings, and trailing commas in pretty mode.

### `Yaml`

Parse YAML from files, strings, or bytes. Convert to JSON or TOML.

```rust
use biscuit_file::Yaml;

let yaml = Yaml::new("config.yaml")?;
let yaml = Yaml::from_str("name: demo")?;
let yaml = Yaml::from_bytes(b"name: demo")?;

let json: serde_json::Value = yaml.as_json()?;
let toml: toml::Value = yaml.as_toml()?;  // requires `toml` feature
```

YAML-to-JSON conversion supports configurable policies for edge cases via `as_json_with()`:

- `NonStringKeyPolicy` -- handling of non-string map keys
- `NonFiniteFloatPolicy` -- handling of NaN/Infinity values

### `Pdf`

Extract text, Markdown, or table-of-contents from PDFs.

```rust
use biscuit_file::Pdf;

let pdf = Pdf::new("document.pdf")?;
let pdf = Pdf::from_bytes(bytes)?;

let text: String = pdf.as_text()?;
let md: PdfMarkdown = pdf.as_markdown(Default::default())?;
let toc: PdfToc = pdf.toc()?;
```

Backend selection is automatic based on enabled features, or can be specified via `PdfConfig`.

## Error Types

Each module has a dedicated error type: `TomlError`, `YamlError`, `Json5Error`, `PdfError`. All implement `std::error::Error` and integrate with `thiserror`.

### `to_json5_pretty` / `to_json5_compact`

Standalone formatters for converting any `serde_json::Value` to JSON5 output, available via `biscuit_file::json5::{to_json5_pretty, to_json5_compact}`.

## Re-Exporting Underlying Crates

For convenience, this crate re-exports types from its underlying dependencies. This allows other packages in this monorepo to migrate from direct dependencies to using `biscuit-file`, while maintaining the same import paths.

> **Note:** When enabling multiple features (e.g., `features = ["toml", "yaml"]`), the corresponding re-exports become available for each.

### TOML Re-Exports

```toml
# Instead of this in your Cargo.toml:
toml = "1.0"

# Use this:
biscuit-file = { path = ".../biscuit-file/lib", features = ["toml"] }
```

```rust
// Instead of:
use toml::Value;

// Use:
use biscuit_file::toml_crate::Value;

// Or access the full crate:
use biscuit_file::toml_crate;

// For parsing errors:
use biscuit_file::TomlDeError;
```

### YAML Re-Exports

```toml
# Instead of this in your Cargo.toml:
serde_yaml = "0.10"

# Use this:
biscuit-file = { path = ".../biscuit-file/lib", features = ["yaml"] }
```

```rust
// Instead of:
use serde_yaml::Value;
use serde_yaml::Mapping;

// Use:
use biscuit_file::serde_yaml_ng::Value;
use biscuit_file::serde_yaml_ng::Mapping;

// For direct access to the crate:
use biscuit_file::serde_yaml_ng;

// For parsing errors:
use biscuit_file::YamlParseError;
```

### Migration Example

Before (direct dependency):
```rust
use toml::Value;
let value: Value = toml::from_str(&content)?;
```

After (via biscuit-file):
```rust
use biscuit_file::toml_crate::Value;
let value: Value = biscuit_file::toml_crate::from_str(&content)?;
```

Or use the wrapper types directly:
```rust
use biscuit_file::Toml;
let toml = Toml::from_str(&content)?;
let value = toml.value();
```

