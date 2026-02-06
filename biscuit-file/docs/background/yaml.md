---
prompt: >-
  - What YAML crates exist in Rust? For each:
      - what are their core features?
      - what feature flags are there and when should you use them?
      - what are the common "gotchas" when using the crate and how do people get around these gotchas?
  - What are the key problems for converting a YAML document into (and how can you resolve them):
      - JSON?
      - TOML?
  - What schema based standards are being used for YAML files to provide some type structure to editors/programs?
      - Which of the crates listed offer some amount of support for these metadata/schema standards?
---

This guide covers the current state of the Rust YAML ecosystem. The original `serde_yaml` crate is deprecated/unmaintained, while maintained forks and alternative parsers cover most needs.

### 1. What YAML Crates Exist in Rust?

The ecosystem is currently split between "High-Level" (Serde-integrated) and "Low-Level" (parsers) crates.

#### A. High-Level (Serde Integrated)

| Crate | Status | Latest | Core Features | Best For |
| --- | --- | --- | --- | --- |
| **`serde_yaml`** | **Deprecated** | `0.9.34+deprecated` | • Original Serde YAML crate (unmaintained).<br>

<br>• Full Serde support via `unsafe-libyaml`.<br>

<br>• `Value` enum for unstructured data. | Legacy projects; prefer `serde_yaml_ng` or `serde-saphyr` for new work. |
| **`serde_yaml_ng`** | **Active** | `0.10.0` | • A maintained fork of `serde_yaml`.<br>

<br>• API compatible with the original (YAML 1.1).<br>

<br>• Security and dependency updates. | **Drop-in replacement** for `serde_yaml`. |
| **`serde_yml`** | **Active** | `0.0.12` | • Another fork of `serde_yaml`.<br>

<br>• Focuses on actively adding new features. | Users needing new features beyond maintenance, though community adoption is mixed compared to `_ng`. |
| **`serde-saphyr`** | **Active** | `0.0.17` | • Uses the pure-Rust `saphyr` parser rather than `yaml-rust`.<br>

<br>• Focuses on panic-free parsing and strict typing. | Projects wanting the `saphyr` parser or a strict alternative to `unsafe-libyaml`-based crates. |

**Common Features & Flags (across Serde-based crates):**

* **Feature Flags:**
* `serde_yaml`, `serde_yaml_ng`, `serde_yml`: no public feature flags. `Value`/`Mapping` uses `IndexMap` to preserve insertion order; typed maps preserve order only if you choose an ordered map type.
* `serde-saphyr`: optional integrations such as `garde`, `validator`, `miette`, `figment`, plus `robotics` and `huge_documents` (default enables the integration features).


* **Gotchas:**
* **`#[serde(flatten)]`:** This is a notorious pain point. flattening a struct that contains an `Option<T>` or generic types often fails with cryptic errors because YAML (unlike JSON) is not self-describing enough for Serde to distinguish types during streaming.
* *Fix:* Avoid `flatten` if possible, or use a custom deserializer.


* **Enum Tagging:** Rust enums are often serialized with `!Variant` tags (YAML tags) which other languages (like Python/JS) cannot parse.
* *Fix:* Use `#[serde(tag = "type")]` on your enums to serialize them as standard maps/strings instead of using YAML-specific tags.





#### B. Low-Level (Parsers)

| Crate | Latest | Core Features | Feature Flags | Gotchas |
| --- | --- | --- | --- | --- |
| **`yaml-rust2`** | `0.11.0` | • Pure Rust implementation.<br>

<br>• No Serde support (AST only).<br>

<br>• Supports YAML 1.2. | • `encoding` (default): enables `encoding_rs`.<br>

<br>• `debug_prints`: enables parser debug output. | • **Manual Traversal:** You have to manually walk the AST (Abstract Syntax Tree).<br>

<br>• **Verbose:** Extracting a nested string requires multiple `.as_str()`, `.unwrap()`, calls. |
| **`libyaml-safer`** | `0.3.0` | • Safe Rust port of libyaml (translated from `unsafe-libyaml`). | • None. | • **Low-level API:** Not Serde-based and not a drop-in C API replacement. |

---

### 2. Key Problems Converting YAML

Converting YAML to other formats is lossy because YAML supports features that JSON and TOML do not.

#### A. Converting YAML to JSON

**Problem 1: Non-String Keys**

* **The Issue:** YAML allows arrays or maps to be keys in a map (e.g., `[1, 2]: "value"`). JSON strictly requires keys to be Strings.
* **Resolution:** You must "stringify" keys during conversion. Most Rust converters will error here. You need a recursive function to check if a key is a scalar; if not, serialize the key to a JSON string or drop the entry.

**Problem 2: Anchors and Aliases (`&` and `*`)**

* **The Issue:** YAML supports references (pointers). JSON is a tree, not a graph.
* **Resolution:**
* *Expansion:* The parser usually automatically expands aliases into deep copies of the data.
* *Circular Refs:* If the YAML has a circular reference (A points to B, B points to A), the JSON serializer will cause a Stack Overflow. You must detect cycles or error out.



**Problem 3: Infinity and NaN**

* **The Issue:** YAML supports `.inf`, `-.inf`, and `.nan`. JSON does not.
* **Resolution:** Convert these to `null` or a specific string literal (e.g., `"Infinity"`) depending on the consuming application's tolerance.

#### B. Converting YAML to TOML

**Problem 1: The `null` Value**

* **The Issue:** YAML has `null` (or `~`). TOML does not have a `null` type.
* **Resolution:**
* *Option A:* Drop the key entirely (Rust `Option::None` often does this automatically).
* *Option B:* Use a sentinel value (like an empty string or 0), though this is dangerous.



**Problem 2: Heterogeneous Arrays**

* **The Issue:** YAML arrays can mix types: `[1, "two", {three: 3}]`. TOML arrays must be homogeneous per the spec, and most parsers enforce this strictly.
* **Resolution:** Convert the array to an array of Tables (objects) or stringify all elements.

**Problem 3: Table Structure (Nesting)**

* **The Issue:** TOML is designed for config files and struggles with deeply nested arrays of arrays, which YAML handles easily.
* **Resolution:** Flatten the structure. If your YAML is deeply nested, it might not be representable in valid/readable TOML.

---

### 3. Schema Standards & Crate Support

YAML defines built-in schemas (failsafe/json/core) for tag and type resolution, but there is no widely adopted YAML-specific validation standard comparable to XSD. In practice, the industry uses **JSON Schema**. Since YAML is (mostly) a superset of JSON, JSON Schema is used to validate YAML files.

#### Standards Used

1. **JSON Schema (Draft 7 / 2020-12):** The industry standard. Most editors (VS Code, JetBrains) use the "YAML Language Server" which pulls schemas from [SchemaStore.org](https://www.schemastore.org/json/) (a collection of JSON Schemas) to validate YAML files.
2. **K8s OpenAPI:** Kubernetes uses OpenAPI v3 schemas (a JSON Schema dialect) to validate its YAML manifests.

#### Crate Support

Rust does not have a "one-stop-shop" crate that loads YAML and validates it against a schema in one go. You typically chain crates together.

| Capability | Crate | How it works |
| --- | --- | --- |
| **Validation** | **`jsonschema`** | **Best for validation.** You parse YAML into a `serde_json::Value` (using `serde_yaml_ng::from_str` or `from_reader`), then pass that JSON Value into `jsonschema` to validate. |
| **Generation** | **`schemars`** | **Best for defining schemas.** You derive `JsonSchema` on your Rust structs. This crate generates a JSON Schema document from your Rust code, which you can then publish so other editors can validate your users' YAML. |
| **Structure** | **`valico`** | An alternative to `jsonschema`, but `jsonschema` is generally faster and more up-to-date with specs. |

## Validating Against a Schema

Here is a complete, runnable example demonstrating how to validate YAML data against a JSON Schema.

This approach bridges the gap between YAML (the input format) and `jsonschema` (the validation standard) by converting the YAML data into a generic JSON Value first.

### 1. The `Cargo.toml` Dependencies

You will need four crates to make this work. We use `serde_yaml_ng` as a maintained fork of `serde_yaml` (swap in `serde_yaml` if you prefer the upstream crate).

```toml
[dependencies]
serde = "1.0"
serde_json = "1.0"
serde_yaml_ng = "0.10.0" # The maintained fork of serde_yaml
jsonschema = "0.41.0"  # The standard validator crate

```

### 2. The Rust Code

```rust
use serde_json::Value as JsonValue;

fn main() {
    // 1. The Schema (Standard JSON Schema format)
    // We require a "name" (string) and "age" (integer >= 18).
    let schema_str = r#"{
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer", "minimum": 18 },
            "tags": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["name", "age"]
    }"#;

    // 2. The Input (YAML format)
    // Note: 'age' is 16, which should trigger a validation error.
    let yaml_input = r#"
name: "Alice"
age: 16
tags:
  - "developer"
  - "rustacean"
"#;

    println!("--- Starting Validation ---");

    // Step A: Parse the Schema into a JSON Value
    let schema_json: JsonValue = serde_json::from_str(schema_str)
        .expect("Schema is invalid JSON");

    // Step B: Compile the Schema
    // This pre-processes the schema for faster validation checks later.
    let validator = jsonschema::validator_for(&schema_json)
        .expect("Schema is invalid spec");

    // Step C: Parse YAML directly into a `serde_json::Value`
    // We do NOT parse into a struct here; we want dynamic data to validate.
    let yaml_as_json: JsonValue = serde_yaml_ng::from_str(yaml_input)
        .expect("Failed to parse YAML file");

    // Step D: Validate
    if validator.is_valid(&yaml_as_json) {
        println!("✅ Document is valid!");
    } else {
        println!("❌ Document failed validation:");
        for error in validator.iter_errors(&yaml_as_json) {
            // The error gives you the path (e.g., /age) and the message.
            println!("   • Field: {} -> {}", error.instance_path(), error);
        }
    }
}

```

### 3. Key Takeaways from this Code

* **The "Bridge" Strategy:** Notice **Step C**. We used `serde_yaml_ng::from_str` but told it to output a `serde_json::Value`.
* This works because `serde_json::Value` implements the `Deserialize` trait.
* This effectively converts YAML to JSON in memory without you having to write a custom converter, solving the compatibility issue between the YAML parser and the JSON Schema validator.


* **Error Handling:** The `jsonschema` crate provides detailed pointers (e.g., `/age`) which are crucial for generating user-friendly error messages in CLI tools or editors.

## Auto Generate a JSON Schema with `schemars`

Here is how you can use **`schemars`** to generate the JSON Schema directly from your Rust code.

This effectively makes your Rust structs the "Source of Truth." If you update your code, the schema updates automatically.

### 1. The `Cargo.toml` Dependencies

You need to add `schemars` and enable its `derive` feature.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = { version = "1.2.1", features = ["derive"] }

```

### 2. The Rust Code

This example shows how to add metadata (descriptions, ranges) to your structs so they appear in the generated schema.

```rust
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

// 1. Derive `JsonSchema` alongside Serialize/Deserialize
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
// 2. Doc comments become the "description" in the schema automatically
/// A configuration for the application server
pub struct ServerConfig {
    /// The hostname or IP address to bind to
    pub host: String,

    /// The port number to listen on
    // 3. We can enforce schema validation rules (like min/max) using attributes
    #[schemars(range(min = 1024, max = 65535))]
    pub port: u16,

    /// Optional list of allowed origins for CORS
    // 4. Options are automatically marked as not "required" in the schema
    pub allowed_origins: Option<Vec<String>>,

    // 5. Enums are handled as "oneOf" or "enum" strings
    pub mode: ServerMode,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ServerMode {
    Development,
    Production,
    Testing,
}

fn main() {
    // Generate the schema for ServerConfig
    let schema = schema_for!(ServerConfig);

    // Convert the schema to a pretty-printed JSON string
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    println!("{}", schema_str);
}

```

### 3. The Output (Generated Schema)

If you run the code above, it outputs this standard JSON Schema. Notice how it captured the doc comments and the range constraint on the port.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "ServerConfig",
  "description": "A configuration for the application server",
  "type": "object",
  "required": [
    "host",
    "mode",
    "port"
  ],
  "properties": {
    "allowed_origins": {
      "description": "Optional list of allowed origins for CORS",
      "anyOf": [
        {
          "type": "array",
          "items": {
            "type": "string"
          }
        },
        {
          "type": "null"
        }
      ]
    },
    "host": {
      "description": "The hostname or IP address to bind to",
      "type": "string"
    },
    "mode": {
      "$ref": "#/$defs/ServerMode"
    },
    "port": {
      "description": "The port number to listen on",
      "type": "integer",
      "format": "uint16",
      "maximum": 65535,
      "minimum": 1024
    }
  },
  "$defs": {
    "ServerMode": {
      "enum": [
        "development",
        "production",
        "testing"
      ],
      "type": "string"
    }
  }
}

```

### 4. Why this is powerful

1. **Docs = Schema:** You don't need to maintain a separate Wiki for your config file format. Your Rust doc comments (`///`) *are* the documentation for the end-user.
2. **Editor Autocomplete:** If you save this output to a file (e.g., `schema.json`) and publish it, users editing your YAML config in VS Code will get **autocomplete** and **hover descriptions** automatically if they link the schema.
3. **Sync Guarantee:** It is impossible for your documentation to drift from your code logic, because the code *generates* the documentation.
