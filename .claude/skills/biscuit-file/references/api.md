## Toml

Source: `biscuit-file/lib/src/toml_impl/types.rs`

### Construction

```rust
use biscuit_file::Toml;

// From file
let toml = Toml::new("config.toml")?;

// From string
let toml = Toml::from_str("[package]\nname = \"demo\"")?;

// From reader
let toml = Toml::from_reader(reader)?;

// From existing value
let toml = Toml::from_value(value)?;

// Validate without constructing
assert!(Toml::is_valid("[package]\nname = \"demo\""));
```

### Conversion

```rust
let json: String = toml.as_json()?;              // pretty JSON
let json_value: serde_json::Value = toml.as_json_value()?;  // owned Value
let yaml: String = toml.as_yaml()?;             // requires `yaml` feature
let yaml_value: serde_yaml_ng::Value = toml.as_yaml_value()?; // requires `yaml`
let raw: &str = toml.raw();                     // original TOML text
let value: &toml::Value = toml.value();           // parsed value tree
let source: &TomlSource = toml.source();       // Path or Inline
```

### TOML-to-JSON Datetime Handling

TOML datetime types convert to RFC 3339 strings in JSON. Non-finite floats become `null`.

## Yaml

Source: `biscuit-file/lib/src/yaml/types.rs`

### Construction
```rust
use biscuit_file::Yaml;

// From file, string, or or bytes
let yaml = Yaml::new("config.yaml")?;
let yaml = Yaml::from_str("name: demo")?;
let yaml = Yaml::from_bytes(b"name: demo")?;

// From existing value
let yaml = Yaml::from_value(serde_yaml_ng::Value);

// Validate
assert!(Yaml::is_valid("name: demo"));
```

### Conversion
```rust
// To JSON (returns serde_json::Value)
let json: serde_json::Value = yaml.as_json()?;

// To JSON with custom policies
let output: ConversionOutput<serde_json::Value> = yaml.as_json_with(options)?;
// output.value = the JSON, output.warnings = any warnings

// To TOML (requires `toml` feature)
let toml: toml::Value = yaml.as_toml()?;
```

### Edge Case Policies (via `as_json_with` / `as_toml_with`)
```rust
use biscuit_file::yaml::{JsonConversionOptions, NonStringKeyPolicy, NonFiniteFloatPolicy};
use biscuit_file::yaml::{TomlConversionOptions, NullPolicy, HeteroArrayPolicy};
```

## Json5
Source: `biscuit-file/lib/src/json5/types.rs`

### Construction
```rust
use biscuit_file::Json5;

let j = Json5::new("config.json5")?;           // from file
let j = Json5::from_str("{ key: 'value' }")?;    // from string
let j = Json5::from_bytes(b"{ key: 'value' }")?; // from bytes

// Validate
assert!(Json5::is_valid("{ key: 'value' }"));       // JSON5
assert!(Json5::is_valid_json(r#"{"key":"value"}"#)); // strict JSON only
```

### Conversion
```rust
let json: String = j.as_json()?;               // pretty JSON
let json_compact: String = j.as_json_compact()?;   // single-line JSON
let json_value: &serde_json::Value = j.as_json_value(); // reference
let json5: String = j.as_json5();            // pretty idiomatic JSON5
let json5_compact: String = j.as_json5_compact(); // single-line JSON5
let yaml: String = j.as_yaml()?;             // requires `yaml` feature
let toml: String = j.as_toml()?;             // requires `toml` feature
```

### Standalone JSON5 Formatters
```rust
use biscuit_file::json5::{to_json5_pretty, to_json5_compact};

let pretty: String = to_json5_pretty(&serde_json_value);
let compact: String = to_json5_compact(&serde_json_value);
```

## PDF
Source: `biscuit-file/lib/src/pdf/types.rs`

### Construction
```rust
use biscuit_file::Pdf;

let pdf = Pdf::new("document.pdf")?;
let pdf = Pdf::from_bytes(bytes)?;
let pdf = Pdf::from_bytes_with_config(bytes, config)?;
```

### Extraction
```rust
let text: String = pdf.as_text()?;      // requires `extract` feature
let md: PdfMarkdown = pdf.as_markdown(Default::default())?; // text wrapped in markdown
let toc: PdfToc = pdf.toc()?;   // requires `lopdf` feature
```

### Configuration
```rust
use biscuit_file::{PdfConfig, PageRange, MarkdownOptions, ImageMode, BackendPreference};

let config = PdfConfig::default()
    .with_backend(BackendPreference::Extract)
    .with_password("secret")
    .with_page_range(PageRange::new(1, 5))
    .with_max_pages(10)
    .with_normalize_text(false)
    .with_remove_headers_footers(false);
```

## Portable Path Text
Source: `biscuit-file/lib/src/path_text.rs` (unfeatured — available with `--no-default-features`)

```rust
use std::path::Path;
use biscuit_file::{to_portable_string, try_portable_string};

// Portable text when a faithful slash-separated spelling exists;
// otherwise the NATIVE spelling, unchanged.
let s: String = to_portable_string(Path::new(r"docs\file.md")); // "docs/file.md"

// Same policy, with the fallback exposed so a caller can branch on it.
let maybe: Option<String> = try_portable_string(Path::new(r"\\server\share\f.md")); // None
```

Which to use:

| Consumer | Function | Why |
|----------|----------|-----|
| Markdown link destination, generated URL-adjacent text | `try_portable_string` | CommonMark eats backslash escapes; a native spelling does not survive a parse. Error or preserve on `None`. |
| Diagnostics, completion candidates, YAML scalars | `to_portable_string` | Native text is still correct output for these. |

Declined (`None` / native fallback) on Windows: UNC, device-namespace, and any
verbatim path `dunce::simplified` will not reduce — reserved DOS names,
trailing dot/space, over-`MAX_PATH`, and literal `.`/`..` names under `\\?\`.
No lexical `.`/`..` collapse happens; `dunce`'s refusal is authoritative.

Lossy by design: non-Unicode data becomes U+FFFD (`Path::to_string_lossy`), and
on Unix a literal `\` in a filename renders as `/`.

Never use rendered text as a path-identity key — build a comparison
representation instead. A short root can simplify while its long descendant
cannot.

## File Detection
Source: `biscuit-file/lib/src/detect.rs`

```rust
use biscuit_file::{detect_file_type, detect_file_type_from_bytes, FileType};

let ft: FileType = detect_file_type("config.toml")?;
let ft: FileType = detect_file_type_from_bytes(b"%PDF-1.7\n");

// FileType variants: Toml, Yaml, Json, Json5, Markdown, Pdf, Unknown
assert_eq!(ft.extension(), Some("toml"));
assert_eq!(ft.mime_type(), Some("application/toml"));
assert_eq!(ft.as_data_format(), Some(DataFormat::Toml));
```
