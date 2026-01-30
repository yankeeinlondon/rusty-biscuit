# Schematic

<table>
  <tr>
    <td><img src="../assets/schematic-2.png" style="max-width='25%'" width=200px /></td>
    <td>
      <p>This package includes four sub-packages which are all aligned to create strongly typed, ergonomic API clients:</p>
      <ul>
        <li><code>define</code> - <i>provides primitives for defining an API, Request and Response schemas, and REST, Websocket, or Multi-part Form Endpoints</i></li>
        <li>
            <code>definition</code> - <i>uses the primitives from <code>define</code> to define an API surface</i>
        </li>
        <li>
            <code>gen</code> - <i>takes the definitions found in the <code>definition</code> package and generates structs and enums to represent these API definitions including a fully functioning network client</i>
        </li>
        <li>
            <code>schema</code> - <i>this is where the finalized API and schema definition go for use by external libraries</i>
        </li>
      </ul>
      <p></p>
    </td>
  </tr>
</table>

## Architecture

```sh
schematic/
├── define/       # Primitives for describing REST APIs (types, auth, endpoints)
├── definitions/  # Actual API definitions using those primitives (OpenAI, etc.)
├── gen/          # Code generator binary and library
└── schema/       # Generated API clients ready for consumption
```

## Workflow

```txt
┌─────────────────────────────┐     ┌─────────────────────────────┐
│      schematic-define       │     │   schematic-definitions     │
│  (primitives: RestApi,      │◄────│  (actual APIs: OpenAI,      │
│   Endpoint, AuthStrategy)   │     │   future: Anthropic, etc.)  │
└──────────────┬──────────────┘     └──────────────┬──────────────┘
               │                                   │
               └───────────────┬───────────────────┘
                               │
                               ▼
               ┌─────────────────────────────┐
               │       schematic-gen         │
               │    (code generator CLI)     │
               └──────────────┬──────────────┘
                              │
                              ▼
               ┌─────────────────────────────┐
               │      schematic-schema       │
               │   (generated API clients)   │
               └─────────────────────────────┘
```

## Quick Start

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), SchematicError> {
    let client = OpenAI::new()?;

    // List all models (no required params - use Default)
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    println!("Found {} models", models.data.len());

    // Retrieve a specific model - type-safe construction with new()
    let model: Model = client
        .request(RetrieveModelRequest::new("gpt-4"))
        .await?;

    println!("Model: {}", model.id);
    Ok(())
}
```

## Packages

| Package                                 | Description                    | Details                           |
|-----------------------------------------|--------------------------------|-----------------------------------|
| [schematic-define](./define/)           | REST API definition primitives | [README](./define/README.md)      |
| [schematic-definitions](./definitions/) | Pre-built API definitions      | [README](./definitions/README.md) |
| [schematic-gen](./gen/)                 | Code generator CLI/library     | [README](./gen/README.md)         |
| [schematic-schema](./schema/)           | Generated API clients          | [README](./schema/README.md)      |

## Available APIs

| API | Endpoints | Auth | Description |
|-----|-----------|------|-------------|
| Anthropic | 4 | API Key (`X-Api-Key`) | Claude Messages API with tool use |
| OpenAI | 3 | Bearer | Models API (list, retrieve, delete) |
| HuggingFace Hub | 28+ | Bearer | Models, datasets, spaces, repos |
| ElevenLabs | 45+ REST, 2 WebSocket | API Key (`xi-api-key`) | TTS, voices, audio generation |
| Ollama Native | 11 | None | Local inference (generate, chat, embed) |
| Ollama OpenAI | 4 | None | OpenAI-compatible subset |
| EMQX Basic | 30+ | Basic | MQTT broker REST API |
| EMQX Bearer | 32+ | Bearer | MQTT broker with token auth |

## Key Features

- **Type-safe requests**: Each endpoint gets a strongly-typed request struct with `new()` constructors
- **Compile-time enforcement**: Required path parameters and bodies are enforced via `new()` constructors
- **Automatic authentication**: Bearer, API Key, and Basic auth with env var fallback chains
- **Proper error handling**: `MissingCredential` errors instead of silent failures
- **Path parameters**: `{param}` syntax in paths become struct fields with `impl Into<String>` for ergonomic usage
- **Multiple response types**: JSON, Text, Binary, and Empty responses with type-specific methods
- **Per-API modules**: Each API gets its own module file with configurable paths
- **Prelude exports**: Convenient imports via `use schematic_*::prelude::*`
- **Validation**: Pre-generation checks for naming collisions and configuration errors
- **Doc examples**: Generated request structs include usage examples in doc comments

## Critical Development Requirements

> **⚠️ IMPORTANT**: Read this section before modifying schematic packages!

### 1. Response Type Verification

The generator produces different methods based on `ApiResponse` types:

| Response Type | Generated Method | Return Type |
|---------------|------------------|-------------|
| `ApiResponse::Json(schema)` | `request<T>()` | `Result<T, SchematicError>` |
| `ApiResponse::Binary` | `request_bytes()` | `Result<bytes::Bytes, SchematicError>` |
| `ApiResponse::Text` | `request_text()` | `Result<String, SchematicError>` |
| `ApiResponse::Empty` | `request_empty()` | `Result<(), SchematicError>` |

**When adding endpoints with non-JSON responses:**

1. **Verify the response type is correct** - Binary audio endpoints must use `ApiResponse::Binary`, not `ApiResponse::Json`
2. **Test the generated code compiles** - Run `cargo check -p schematic-schema`
3. **Test runtime behavior** - Unit tests only verify syntax, not that `response.bytes()` vs `response.json()` is called correctly

### 2. Module Path Configuration

When defining APIs, the generator assumes: **1 API name → 1 module name → 1 definitions module**

| Scenario | Configuration Required |
|----------|------------------------|
| Single API per module | `module_path: None` (auto-inferred) |
| Multiple APIs sharing one definitions module | **REQUIRES explicit `module_path`** |
| API name differs from definitions module | **REQUIRES explicit `module_path`** |

**Example - Ollama has two APIs sharing one definitions module:**

```rust
// ❌ WRONG - Will fail: generates "ollamanative.rs" looking for schematic_definitions::ollamanative
RestApi { name: "OllamaNative".to_string(), module_path: None, ... }

// ✅ CORRECT - Both use explicit path to shared module
RestApi { name: "OllamaNative".to_string(), module_path: Some("ollama".to_string()), ... }
RestApi { name: "OllamaOpenAI".to_string(), module_path: Some("ollama".to_string()), ... }
```

### 3. Testing Requirements

**Current tests verify:**
- ✅ Generated code is syntactically valid Rust
- ✅ Code compiles (`cargo check`)
- ✅ Unit test coverage for individual generators

**Current tests DO NOT verify:**
- ❌ Runtime behavior (binary responses actually call `.bytes()`)
- ❌ Integration with real APIs
- ❌ Module path resolution across multiple APIs

**Before submitting changes:**

```bash
# 1. Run unit tests
cargo test -p schematic-define -p schematic-definitions -p schematic-gen

# 2. Regenerate all schemas
just -f schematic/justfile generate

# 3. Verify generated code compiles
cargo check -p schematic-schema

# 4. For response type changes, manually verify correct method is generated:
grep -n "request_bytes\|request_text\|request_empty" schematic/schema/src/*.rs
```

## Building

All operations are done using the _justfile_ and the `just` runner:

```bash
# Build all schematic packages
just -f schematic/justfile build
# Run tests
just -f schematic/justfile test
# Run linter
just -f schematic/justfile lint
# Generate API clients
just -f schematic/justfile generate
# Validate API definitions (without generating)
schematic-gen validate --api openai
# Full workflow: generate and verify
just -f schematic/justfile full
```

### CLI Subcommands

The `schematic-gen` CLI supports two subcommands:

```bash
# Validate an API definition
schematic-gen validate --api openai

# Generate client code (validates first)
schematic-gen generate --api openai --output ./output
```

## License

AGPL-3.0-only
