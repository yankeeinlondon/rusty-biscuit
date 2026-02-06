# biscuit-file

Core library for file format parsing, conversion, and detection.

## Feature Flags

| Feature | Default | Description |
|---------|:-------:|-------------|
| `toml` | yes | TOML parsing and conversion |
| `yaml` | yes | YAML parsing and conversion |
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

Recognized extensions: `.toml`, `.yaml`, `.yml`, `.json`, `.md`, `.markdown`, `.mdx`, `.pdf`

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

Each module has a dedicated error type: `TomlError`, `YamlError`, `PdfError`. All implement `std::error::Error` and integrate with `thiserror`.
