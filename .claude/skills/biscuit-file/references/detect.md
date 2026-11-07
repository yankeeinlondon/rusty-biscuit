## File Type Detection

### API

```rust
use biscuit_file::{detect_file_type, detect_file_type_from_bytes, detect_from_extension, FileType};

// From file path (magic bytes first, then extension fallback)
let ft: FileType = detect_file_type("config.toml")?;

// From bytes (magic bytes only; no extension fallback)
let ft: FileType = detect_file_type_from_bytes(b"%PDF-1.7\n");

// From a path's extension only (no I/O; works for URL paths too)
let ft: FileType = detect_from_extension(std::path::Path::new("DATA.CSV")); // Csv
```

### FileType Enum

| Variant | Extension | MIME |
|---------|----------|------|
| `Toml` | `toml` | `application/toml` |
| `Yaml` | `yaml` | `application/yaml` |
| `Json` | `json` | `application/json` |
| `Json5` | `json5` | `application/json5` |
| `Jsonl` | `jsonl` (also `.ndjson`) | `application/x-ndjson` |
| `Csv` | `csv` | `text/csv` |
| `Tsv` | `tsv` | `text/tab-separated-values` |
| `Markdown` | `md` | `text/markdown` |
| `Pdf` | `pdf` | `application/pdf` |
| `Unknown` | `None` | `None` |

Methods: `.extension() -> Option<&str>`, `.mime_type() -> Option<&str>`, `.as_data_format() -> Option<DataFormat>`

Detection order: read first 512 bytes for magic bytes (`%PDF-` for PDF), then fall back to extension.

Recognized extensions (case-insensitive): `.toml`, `.yaml`, `.yml`, `.json`, `.json5`, `.jsonl`, `.ndjson`, `.csv`, `.tsv`, `.md`, `.markdown`, `.mdx`, `.pdf`

`bf` detects `Jsonl`/`Csv`/`Tsv` but rejects them as conversion inputs; the library reads them with `json_lines::read_json_lines` and (feature `delimited`) `delimited::read_delimited`. See `biscuit-file/docs/topics/data-readers.md`.

### DataFormat Enum

`DataFormat` has the `Text` variant (unlike `FileType::Unknown`) and the same `Jsonl`, `Csv`, `Tsv` variants (UTF-8, structured). Methods: `.extension()`, `.mime_type()`, `.requires_utf8()`, `.is_structured()`, `Display`.

A `From<DataFormat>` for `FileType` conversion exists (maps `Text` to `Unknown`).

Source: `biscuit-file/lib/src/detect.rs`, `biscuit-file/lib/src/format.rs`
