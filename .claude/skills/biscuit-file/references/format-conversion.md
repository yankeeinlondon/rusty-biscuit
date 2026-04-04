## Format Conversion Reference

 Conversion matrix between supported data formats:

```
text
Read ┌── Write ┯── ┬────────────────┟
-----|
 TOML   | Y    | Y    | Y    | Y    | Y      | --   |
 YAML   | Y    | Y    | Y    | Y    | Y      | --   |
 JSON   | Y    | Y    | Y    | Y    | Y      | --   |
 JSON5 | Y    | Y    | Y    | Y    | Y      | --   |
 PDF   | --   | --   | --   | --   | Text  | Markdown |

 Write ─── Write ┯── Read  │ Read      | Write  |
 TOML   | Y    | Y    | Y    | Y    | --   |
 YAML  | Y    | Y    | Y    | Y    | --   |
 JSON   | Y    | Y    | Y    | Y    | --   |
 JSON5 | Y    | Y    | Y    | Y    | --   |
 PDF   | Text  | Markdown | --    |
 Markdown | Y    | --   | --   | --   |
```

### Toml

```rust
use biscuit_file::Toml;

let toml = Toml::from_str("[package]\nname = \"demo\"")?;
let json: String = toml.as_json()?;                    // pretty JSON
let json_value: serde_json::Value = toml.as_json_value()?;  // owned Value
let yaml: String = toml.as_yaml()?;             // requires `yaml` feature
let raw: &str = toml.raw();                     // original TOML text
let value: &toml::Value = toml.value();           // parsed tree
```

### Yaml

```rust
use biscuit_file::Yaml;

let yaml = Yaml::from_str("name: demo")?;
let yaml = Yaml::from_bytes(b"name: demo")?;

let json: serde_json::Value = yaml.as_json()?;
let toml_val: toml::Value = yaml.as_toml()?;  // requires `toml` feature
```

Edge cases are handled via configurable policies:

```rust
use biscuit_file::yaml::{JsonConversionOptions, NonStringKeyPolicy, NonFiniteFloatPolicy};

let yaml = Yaml::from_str("value: .nan")?;
let output = yaml.as_json_with(JsonConversionOptions {
    float_policy: NonFiniteFloatPolicy::Error, // reject non-finite floats
    ..Default::default()
}))?;
```

TOML conversion with policies:

```rust
use biscuit_file::yaml::{TomlConversionOptions, NullPolicy, HeteroArrayPolicy};

let yaml = Yaml::from_str("a: null\nb: 1")?;
let output = yaml.as_toml_with(TomlConversionOptions {
    null_policy: NullPolicy::Drop,     // drop null values
    array_policy: HeteroArrayPolicy::Error, // reject mixed arrays
    ..Default::default()
}))?;
```

### Json5

```rust
use biscuit_file::Json5;

let j = Json5::from_str("{ key: 'value', /* comment */ }")?;
let json: String = j.as_json()?;          // pretty JSON
let json_compact: String = j.as_json_compact()?;
let json5: String = j.as_json5()?;         // idiomatic JSON5 (unquoted keys, single quotes)
let json5_compact: String = j.as_json5_compact()?;
let yaml: String = j.as_yaml()?;        // requires `yaml` feature
let toml: String = j.as_toml()?;        // requires `toml` feature
```

JSON5 formatter (standalone):

```rust
use biscuit_file::json5::{to_json5_pretty, to_json5_compact};

let pretty = to_json5_pretty(&serde_json::json!({"name": "Bob"}));
// → { name: 'Bob', }

let compact = to_json5_compact(&serde_json::json!({"name": "Bob"}));
// → {name: 'bob'}
```

### Pdf

```rust,no_run
use biscuit_file::{Pdf, PdfConfig, PdfMarkdown};

let pdf = Pdf::new("document.pdf")?;
let pdf = Pdf::from_bytes(bytes)?;              // from raw bytes

let text: String = pdf.as_text()?;           // requires `extract` feature
let md: PdfMarkdown = pdf.as_markdown(Default::default())?;
let toc: PdfToc = pdf.toc()?;               // requires `lopdf` feature
```

### Conversion Gotchas

- TOML datetime types convert to RFC 3339 strings in JSON; non-finite floats become `null`
- YAML `null` is unsupported in TOML; use `NullPolicy::Drop` or `NullPolicy::Stringify`
 in `as_toml_with()`
- JSON5 `null` values fail `as_toml()` conversion; use `as_toml_with()` or filter nulls first
- YAML-to-TOML: heterogeneous arrays fail by default; use `HeteroArrayPolicy::Stringify`
 if needed
- TOML/YAML-to-JSON5: produces standard JSON representation; for idiomatic JSON5 output, parse JSON5 back through the formatter
- Non-string YAML keys: default policy is `Stringify`; use `NonStringKeyPolicy::Error` to reject
 - Deep nesting protection: `max_depth: Some(N)` in conversion options (default: 100)
