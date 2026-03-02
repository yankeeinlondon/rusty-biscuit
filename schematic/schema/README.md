# schematic-schema

Generated API outputs produced by `schematic-gen`.

> Auto-generated source in `src/` should not be edited manually.

## Overview

`schematic-schema` currently provides:

- Generated Rust REST clients (request structs, enums, typed response handling)
- Generated WebSocket **definition helper modules** for APIs that are defined as `WebSocketApi` in `schematic-definitions`

Important: WebSocket runtime/client code generation is not implemented yet. WS modules in this crate expose typed API definitions and message model types, not a live WS transport client.

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

### WebSocket definition helper modules

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

## WebSocket Definition Usage

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
