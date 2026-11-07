## Module Organization

Source: `biscuit-file/lib/src/`

| Module | Feature Flag | Key Types |
|--------|-------------|-----------|
| `toml_impl` | `toml` | `Toml`, `TomlError`, `TomlSource`, `ValidationReport` |
| `yaml` | `yaml` | `Yaml`, `YamlError`, `YamlSource`, `ConversionOutput` |
| `json5` | `json5` | `Json5`, `Json5Error`, `Json5Source` |
| `pdf` | `extract` / `lopdf` / `pdfium` | `Pdf`, `PdfConfig`, `PdfError`, `PdfMarkdown`, `PdfToc` |
| `file_reference` | `file-reference-syntax` | `FileReference` (parse, `class`, `payload`, builders), `FileReferenceKind`, `FileReferenceClass`, `FileReferenceError`, `ResolutionFailure`, `ResolutionCandidate`, `RootProvenance`, `PathPosition`, `MagicPathTier`, `GlobReference` (`new`, `patterns`, `escape`), `GlobReferenceError`, `FileResolutionContext`, `BaseDirOrigin`, `LaunchMagicScope`, `home_dir`, `user_cache_dir`; from `file_reference::portable` (`path_identity`, `strategy`): `PathIdentity`, `RelativeRoute`, `PortabilityPreference`, `IntentForms` |
| `file_reference` (resolution) | `file-reference` | `resolve.rs` and every `resolve*`/`candidate_plan*`/`complete_partial*`/`validate_*_candidate` method, `DetailedResolution`, `PartialCompletion`, `Resolved`, `MagicSearchRoot`, glob listing (`glob/{list,matches,roots}.rs`: `GlobListing`, `SkippedEntry`, `PreparedGlobReference`), `find_git_root`, `FileReferenceError::Git`; from `file_reference::portable` (`env_anchor`, `diagnostics`, `evaluate`, crate-internal `text`): `PortablePath`, `PortableReference`, `PORTABLE_ENV_VARIABLES`, `PortablePathError` and its diagnostics types |
| `frontmatter` | `yaml` | `extract_frontmatter_block`, `FrontmatterExtraction`, `FrontmatterEnvelopeError`, `line_spans`, `LineSpan`, `detect_near_miss_fence` |
| `detect` | (always) | `FileType`, `detect_file_type`, `detect_file_type_from_bytes`, `detect_from_extension` |
| `json_lines` | (always) | `read_json_lines`, `JsonLines`, `JsonLineRecord`, `JsonLinesError` |
| `delimited` | `delimited` | `read_delimited`, `DelimitedData`, `DelimitedRecord`, `DelimitedField`, `DelimitedError` |
| `data_locations` | (always; `locate_toml` needs `toml`, `locate_json5` and `json5_parse_error_offset` need `json5`) | `locate_json`, `locate_jsonl`, `locate_toml`, `locate_json5`, `json5_parse_error_offset` |
| `source_position` | (always) | `source_position` |
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
| `file-reference-syntax` | `globset`, `url` | Passive file and glob reference grammar and data types |
| `file-reference` | `file-reference-syntax`, `gix`, `walkdir` | File reference resolution, glob listing, portable references, git root discovery |
| `schema` | `jsonschema` | JSON Schema validation (not yet implemented) |
| `delimited` | `csv`, `csv-core` | Strict CSV/TSV reader with raw field offsets (off by default; darkmatter enables it; tests in the `l1-delimited` target) |
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
