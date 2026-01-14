# Schematic

A two-crate code generator for producing strongly-typed Rust REST API clients from declarative definitions.

## Architecture

```
schematic/
├── define/    # Type definitions to define a Schema, API, or both (RestApi,
|             # Endpoint, AuthStrategy, etc.)
└── gen/      # Binary Code generator (tokenization, validation, formatting)
└── schema/   # generated API's and Schema's provided by Schematic and ready
              # for consumption by callers
```

## Workflow

```
┌─────────────────────────────┐
│      Define API             │
│  (schematic-define types)   │
└──────────────┬──────────────┘
               │
               ▼
┌─────────────────────────────┐
│      Generate Code          │
│  (schematic-gen binary)     │
└──────────────┬──────────────┘
               │
               ▼
┌─────────────────────────────┐
│   Compiled Rust Client      │
│  - Type-safe requests       │
│  - Auto authentication      │
│  - Error handling           │
└─────────────────────────────┘
```

## Quick Start

### 1. Define Your API

```rust
use schematic_define::{RestApi, Endpoint, RestMethod, AuthStrategy, ApiResponse};

let api = RestApi {
    name: "MyService".to_string(),
    description: "My REST API".to_string(),
    base_url: "https://api.example.com/v1".to_string(),
    docs_url: None,
    auth: AuthStrategy::BearerToken { header: None },
    env_auth: vec!["MY_API_KEY".to_string()],
    env_username: None,
    env_password: None,
    endpoints: vec![
        Endpoint {
            id: "ListItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "List all items".to_string(),
            request: None,
            response: ApiResponse::json_type("ListItemsResponse"),
        },
    ],
};
```

### 2. Generate Client Code

```bash
schematic-gen --api myservice --output src/generated
```

### 3. Use the Generated Client

```rust
use my_generated::{MyService, ListItemsRequest, ListItemsResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = MyService::new();
    let items: ListItemsResponse = client
        .request(ListItemsRequest::default())
        .await?;
    Ok(())
}
```

## Packages

| Package | Description | Documentation |
|---------|-------------|---------------|
| [schematic-define](./define/) | REST API definition types | [README](./define/README.md) |
| [schematic-gen](./gen/) | Code generator | [README](./gen/README.md) |

## Key Features

- **Type-safe requests**: Each endpoint gets a strongly-typed request struct
- **Automatic authentication**: Bearer, API Key, and Basic auth with env var fallback chains
- **Proper error handling**: `MissingCredential` errors instead of silent failures
- **Path parameters**: `{param}` syntax in paths become struct fields
- **Multiple response types**: JSON, Text, Binary, and Empty responses
- **Syntax validation**: Generated code is parsed with `syn` before writing
- **Consistent formatting**: All output is formatted with `prettyplease`

## Authentication

Authentication credentials are specified via environment variables. All strategies fail fast with `SchematicError::MissingCredential` if credentials are missing.

| Strategy | RestApi Fields | Generated Behavior |
|----------|---------------|-------------------|
| Bearer Token | `auth: BearerToken`, `env_auth: vec![...]` | `Authorization: Bearer <token>` |
| API Key | `auth: ApiKey { header }`, `env_auth: vec![...]` | `<header>: <key>` |
| Basic Auth | `auth: Basic`, `env_username`, `env_password` | `Authorization: Basic <base64>` |
| None | `auth: None` | No auth headers |

## Building

```bash
# Build both packages
just -f schematic/justfile build

# Run tests
just -f schematic/justfile test

# Install the generator binary
cargo install --path schematic/gen
```

## License

AGPL-3.0-only
