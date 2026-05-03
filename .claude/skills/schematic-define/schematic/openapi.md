# OpenAPI Support

Schematic supports bidirectional OpenAPI 3.x integration: import OpenAPI specs to generate Rust clients, or export existing API definitions to OpenAPI format.

**Feature Gate**: OpenAPI functionality requires the `openapi` feature in `schematic-define`.

## Import OpenAPI Specs

Transform any OpenAPI 3.x specification into a type-safe Rust client.

### CLI Usage

```bash
# Basic import
schematic-gen import --input petstore.yaml --output generated/src

# With custom API name
schematic-gen import --input api.json --api-name MyPetStore --output src

# Strict mode (fail on warnings)
schematic-gen import --input api.yaml --output src --strict

# Dry run (preview without writing)
schematic-gen import --input api.yaml --output src --dry-run
```

### Import Options

| Option | Description |
|--------|-------------|
| `--input` | Path to OpenAPI spec file (JSON or YAML) |
| `--api-name` | Override API name (default: derived from spec title) |
| `--module-path` | Override module path for generated code |
| `--output` | Output directory for generated code |
| `--dry-run` | Preview generated code without writing files |
| `--strict` | Fail on any warning-level diagnostic |

### Library API

```rust
use schematic_define::openapi::{OpenApiImport, OpenApiSource};
use schematic_gen::import_pipeline::{run_import, ImportOptions};

// Option 1: High-level pipeline
let options = ImportOptions {
    input: "api.yaml".to_string(),
    api_name: Some("MyApi".to_string()),
    module_path: None,
    output: "generated/src".to_string(),
    dry_run: false,
    strict: false,
    verbose: true,
};
let result = run_import(&options)?;

// Option 2: Low-level builder
let source = OpenApiSource::from_file("api.yaml")?;
let result = OpenApiImport::new(source)
    .api_name("MyApi")
    .prefer_json()
    .strict()
    .build()?;

println!("Generated {} endpoints, {} models",
    result.api.endpoints.len(),
    result.models.models.len());
```

### Type Mappings

| OpenAPI Type | Rust Type |
|--------------|-----------|
| `string` | `String` |
| `string` + `format: date-time` | `String` (or chrono types) |
| `integer` | `i64` |
| `integer` + `format: int32` | `i32` |
| `number` | `f64` |
| `boolean` | `bool` |
| `array` | `Vec<T>` |
| `object` | Named struct |
| `object` + `additionalProperties` | `HashMap<String, T>` |
| `oneOf`/`anyOf` | Enum (untagged) |
| `$ref` | Named type reference |

### Diagnostics

Import produces diagnostics for potential issues:

```rust
pub enum DiagnosticSeverity {
    Info,   // Informational messages
    Warn,   // Non-blocking issues
    Error,  // Blocking issues (fail in strict mode)
}

pub struct OpenApiDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub location: Option<String>,  // JSON path
}
```

Use `--strict` to fail on warnings.

## Export to OpenAPI

Generate OpenAPI 3.0.3 specs from existing Schematic API definitions.

### CLI Usage

```bash
# Export as JSON (default)
schematic-gen generate --api openai --openapi-out specs/

# Export as YAML
schematic-gen generate --api openai --openapi-out specs/ --openapi-format yaml

# Generate all with OpenAPI export
schematic-gen generate --api all --openapi-out specs/
```

### Library API

```rust
use schematic_define::openapi::{export, ExportOptions, ExportFormat};
use schematic_definitions::openai::define_openai_api;
use schematic_definitions::registry::get_registry;

let api = define_openai_api();
let registry = get_registry("openai").unwrap();

let options = ExportOptions::new()
    .with_version("1.0.0")
    .with_format(ExportFormat::Yaml);

let openapi_doc = export(&api, &registry, &options)?;
```

### Schematic Extensions

OpenAPI specs include `x-schematic` extensions for round-trip fidelity:

```yaml
# Document level
x-schematic:
  module_path: "my_api"
  request_suffix: "Request"
  env_mapping:
    bearer_token: ["API_KEY"]
  headers:
    - ["X-Custom", "value"]

# Operation level
x-schematic:
  request: "CreateUserBody"
  response: "User"
  headers:
    - ["X-Request-ID", "12345"]
```

## Feature Configuration

In `Cargo.toml`:

```toml
[dependencies]
schematic-define = { path = "../define", features = ["openapi"] }
```

The `openapi` feature enables:
- `openapi` module with import/export functions
- OpenAPI 3.x parsing via `openapiv3` crate
- YAML support via `serde_yaml`
