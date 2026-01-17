# schematic-schema

Generated REST API clients produced by `schematic-gen`.

> **Note**: This package contains auto-generated code. Do not edit files in `src/` directly - they will be overwritten on regeneration.

## Overview

`schematic-schema` provides ready-to-use, strongly-typed Rust HTTP clients for various REST APIs. Each API is in its own module with:

- A client struct (e.g., `OpenAI`)
- Request structs for each endpoint
- A unified request enum
- Re-exported response types from `schematic-definitions`

## Usage

### Quick Start with Prelude

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), SchematicError> {
    // Create client (reads OPENAI_API_KEY from environment)
    let client = OpenAI::new()?;

    // List all models
    let models: ListModelsResponse = client
        .request(ListModelsRequest::default())
        .await?;

    for model in models.data {
        println!("{}: owned by {}", model.id, model.owned_by);
    }

    Ok(())
}
```

### Direct Module Access

```rust
use schematic_schema::openai::{
    OpenAI, OpenAIRequest,
    ListModelsRequest, RetrieveModelRequest,
    Model, ListModelsResponse,
};

#[tokio::main]
async fn main() -> Result<(), schematic_schema::openai::SchematicError> {
    let client = OpenAI::new()?;

    // Retrieve a specific model
    let gpt4: Model = client
        .request(RetrieveModelRequest {
            model: "gpt-4".to_string(),
        })
        .await?;

    println!("Model {} created at {}", gpt4.id, gpt4.created);
    Ok(())
}
```

## Available APIs

| API | Module | Client Struct | Auth |
|-----|--------|---------------|------|
| OpenAI | `openai` | `OpenAI` | Bearer token (`OPENAI_API_KEY`) |

## Prelude Exports

The prelude (`schematic_schema::prelude`) exports:

- **Client structs**: `OpenAI`
- **Request enums**: `OpenAIRequest`
- **Error type**: `SchematicError`
- **Response types**: `Model`, `ListModelsResponse`, `DeleteModelResponse` (from definitions)

## Client Configuration

### Default Configuration

```rust
// Uses default base URL and reads credentials from environment
let client = OpenAI::new()?;
```

### Custom Base URL

```rust
// Use a different API endpoint (for testing, proxies, etc.)
let client = OpenAI::with_base_url("http://localhost:8080/v1");
```

### Custom HTTP Client

```rust
// Use a pre-configured reqwest client
let http_client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(60))
    .build()?;

let client = OpenAI::with_client(http_client);

// Or with both custom client and base URL
let client = OpenAI::with_client_and_base_url(
    http_client,
    "https://api.example.com/v1"
);
```

### API Variants

Create variants with different configurations:

```rust
use schematic_define::UpdateStrategy;

let production = OpenAI::new()?;

// Staging environment with different credentials
let staging = production.variant(
    "https://staging.openai.com/v1",
    vec!["STAGING_OPENAI_KEY".to_string()],
    UpdateStrategy::NoChange,
);
```

## Error Handling

All API calls return `Result<T, SchematicError>`:

```rust
use schematic_schema::prelude::*;

match client.request(ListModelsRequest::default()).await {
    Ok(response) => println!("Got {} models", response.data.len()),
    Err(SchematicError::Http(e)) => eprintln!("Network error: {}", e),
    Err(SchematicError::Json(e)) => eprintln!("Parse error: {}", e),
    Err(SchematicError::ApiError { status, body }) => {
        eprintln!("API error {}: {}", status, body)
    }
    Err(SchematicError::MissingCredential { env_vars }) => {
        eprintln!("Set one of: {:?}", env_vars)
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Regenerating

To regenerate the API clients:

```bash
# From the schematic directory
just generate

# Or with verification
just full
```

## File Structure

```
schema/
├── Cargo.toml      # Auto-generated manifest
└── src/
    ├── lib.rs      # Module declarations
    ├── prelude.rs  # Convenient re-exports
    └── openai.rs   # OpenAI API client
```

## Dependencies

The generated code requires (automatically managed):

- `reqwest` - HTTP client
- `serde` / `serde_json` - Serialization
- `thiserror` - Error types
- `tokio` - Async runtime
- `schematic-define` - Auth strategy types
- `schematic-definitions` - Response types

## License

AGPL-3.0-only
