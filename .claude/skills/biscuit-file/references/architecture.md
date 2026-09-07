## Module Organization

Source: `biscuit-file/lib/src/`

| Module | Feature Flag | Key Types |
|--------|-------------|-----------|
| `toml_impl` | `toml` | `Toml`, `TomlError`, `TomlSource`, `ValidationReport` |
| `yaml` | `yaml` | `Yaml`, `YamlError`, `YamlSource`, `ConversionOutput` |
| `json5` | `json5` | `Json5`, `Json5Error`, `Json5Source` |
| `pdf` | `extract` / `lopdf` / `pdfium` | `Pdf`, `PdfConfig`, `PdfError`, `PdfMarkdown`, `PdfToc` |
| `file_reference` | `file-reference` | `FileReference`, `FileReferenceError`, `PathPosition` |
| `detect` | (always) | `FileType`, `detect_file_type`, `detect_file_type_from_bytes` |
| `format` | (always) | `DataFormat` |
| `path_text` | (always) | `to_portable_string`, `try_portable_string` |
| `error` | (always) | `BiscuitFileError` |

## Feature Flags

Default features: `toml`, `yaml`, `json5`, `extract`, `lopdf`, `file-reference`

| Flag | Dependencies | Description |
|------|-------------|-------------|
| `toml` | `toml` | TOML parsing, conversion, `Toml` struct |
| `yaml` | `serde_yaml_ng` | YAML parsing, conversion, `Yaml` struct |
| `json5` | `json-five` | JSON5 parsing, conversion, `Json5` struct |
| `extract` | `pdf-extract` | PDF text extraction |
| `lopdf` | `lopdf` | PDF TOC extraction |
| `pdfium` | `pdfium-render` | High-fidelity PDF extraction |
| `file-reference` | `gix`, `walkdir`, `dirs`, `url` | File reference parsing and resolution |
| `schema` | `jsonschema` | JSON Schema validation (not yet implemented) |
| `full` | all above | All features enabled |

## Re-exports

The library re-exports underlying crates for convenience, allowing consumers to migrate from direct dependencies:

```rust
// TOML
use biscuit_file::toml_crate;          // crate re-export
use biscuit_file::TomlDeError;          // toml::de::Error

// YAML
use biscuit_file::serde_yaml_ng;       // crate re-export
use biscuit_file::YamlValue;           // serde_yaml_ng::Value
use biscuit_file::YamlMapping;         // serde_yaml_ng::Mapping
use biscuit_file::YamlParseError;      // serde_yaml_ng::Error
```

## Error Handling Pattern

All error types use `thiserror` derive macros. Library code never uses `unwrap()` or `expect()` outside of tests. All public functions return `Result` types with module-specific errors:

- `TomlError` — IO, Parse, Serialize, Json, Yaml variants
- `YamlError` — IO, Parse, JsonConversion, TomlConversion, SchemaParse, CycleDetected, MaxDepthExceeded
- `Json5Error` — Io, Parse(String), Json, Yaml, Toml variants
- `PdfError` — Io, BackendUnavailable, Parse, Encrypted, Image, Unsupported
- `FileReferenceError` — InvalidSyntax, MissingEnvironmentVariable, CurrentDirectory, Git, Workspace, VaultNotConfigured, RelativePath, Io
