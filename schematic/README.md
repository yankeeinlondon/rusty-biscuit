# Schematic

A modular code generation system for producing strongly-typed Rust REST API clients from declarative definitions.

## Architecture

```
schematic/
├── define/       # Primitives for describing REST APIs (types, auth, endpoints)
├── definitions/  # Actual API definitions using those primitives (OpenAI, etc.)
├── gen/          # Code generator binary and library
└── schema/       # Generated API clients ready for consumption
```

## Workflow

```
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

    // List all models
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    println!("Found {} models", models.data.len());
    Ok(())
}
```

## Packages

| Package | Description | Details |
|---------|-------------|---------|
| [schematic-define](./define/) | REST API definition primitives | [README](./define/README.md) |
| [schematic-definitions](./definitions/) | Pre-built API definitions | [README](./definitions/README.md) |
| [schematic-gen](./gen/) | Code generator CLI/library | [README](./gen/README.md) |
| [schematic-schema](./schema/) | Generated API clients | [README](./schema/README.md) |

## Key Features

- **Type-safe requests**: Each endpoint gets a strongly-typed request struct
- **Automatic authentication**: Bearer, API Key, and Basic auth with env var fallback chains
- **Proper error handling**: `MissingCredential` errors instead of silent failures
- **Path parameters**: `{param}` syntax in paths become struct fields
- **Multiple response types**: JSON, Text, Binary, and Empty responses
- **Per-API modules**: Each API gets its own module file
- **Prelude exports**: Convenient imports via `use schematic_*::prelude::*`

## Building

```bash
# Build all schematic packages
just -f schematic/justfile build

# Run tests
just -f schematic/justfile test

# Run linter
just -f schematic/justfile lint

# Generate API clients
just -f schematic/justfile generate

# Full workflow: generate and verify
just -f schematic/justfile full
```

## License

AGPL-3.0-only
