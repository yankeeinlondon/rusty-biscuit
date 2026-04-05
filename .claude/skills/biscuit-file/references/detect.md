## File Type Detection

### API

```rust
use biscuit_file::{detect_file_type, detect_file_type_from_bytes, FileType};

// From file path (magic bytes first, then extension fallback)
let ft: FileType = detect_file_type("config.toml")?;

// From bytes (magic bytes only; no extension fallback)
let ft: FileType = detect_file_type_from_bytes(b"%PDF-1.7\n");
```

### FileType Enum

| Variant | Extension | MIME |
|---------|----------|------|
| `Toml` | `toml` | `application/toml` |
| `Yaml` | `yaml` | `application/yaml` |
| `Json` | `json` | `application/json` |
| `Json5` | `json5` | `application/json5` |
| `Markdown` | `md` | `text/markdown` |
| `Pdf` | `pdf` | `application/pdf` |
| `Unknown` | `None` | `None` |

Methods: `.extension() -> Option<&str>`, `.mime_type() -> Option<&str>`, `.as_data_format() -> Option<DataFormat>`

Detection order: read first 512 bytes for magic bytes (`%PDF-` for PDF), then fall back to extension.

Recognized extensions: `.toml`, `.yaml`, `.yml`, `.json`, `.json5`, `.md`, `.markdown`, `.mdx`, `.pdf`

### DataFormat Enum

`DataFormat` has the `Text` variant (unlike `FileType::Unknown`). Methods: `.extension()`, `.mime_type()`, `.requires_utf8()`, `.is_structured()`, `Display`.

A `From<DataFormat>` for `FileType` conversion exists (maps `Text` to `Unknown`).

Source: `biscuit-file/lib/src/detect.rs`, `biscuit-file/lib/src/format.rs`
