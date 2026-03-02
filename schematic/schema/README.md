# schematic-schema

Generated API outputs produced by `schematic-gen`.

> Auto-generated source in `src/` should not be edited manually.

## Overview

`schematic-schema` currently provides:

- Generated Rust REST clients (request structs, enums, typed response handling)
- Generated Rust WebSocket clients and hosts (transport runtimes, event streams, correlated request-response)
- Typed API definitions and message model types for consumption

## Available Modules

### REST client modules

- `anthropic`
- `bitbucket`
- `elevenlabs`
- `emqx`
- `eversolo`
- `gitea`
- `github`
- `gitlab`
- `huggingface`
- `lmstudio`
- `ollama`
- `openai`
- `unfolded_circle_core_rest`

### WebSocket runtime modules

- `elevenlabs_ws`
- `unfolded_circle_core_ws`
- `unfolded_circle_dock_ws`
- `unfolded_circle_integration_ws`

## Quick Start

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), SchematicError> {
    let client = OpenAI::new();
    let response = client.list_models().await?;
    println!("{}", response.data.len());
    Ok(())
}
```

## WebSocket Usage

```rust
use schematic_schema::prelude::*;

let core_ws = define_unfolded_circle_core_ws_api_definition();
let dock_ws = define_unfolded_circle_dock_ws_api_definition();
let integration_ws = define_unfolded_circle_integration_ws_api_definition();

assert_eq!(core_ws.name, "UnfoldedCircleCoreWs");
assert_eq!(dock_ws.name, "UnfoldedCircleDockWs");
assert_eq!(integration_ws.name, "UnfoldedCircleIntegrationWs");
```

## Notes

- There is currently no `Dock REST` API definition in `schematic-definitions`; only `Dock WebSocket` is defined.
- Use API modules directly (for example `schematic_schema::openai::Model`) when you need response/model types.

## Regenerating

```bash
cd schematic
just generate
```
