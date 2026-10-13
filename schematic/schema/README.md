# schematic-schema

Generated API outputs produced by `schematic-gen`.

> Auto-generated source in `src/` should not be edited manually.

## Overview

`schematic-schema` currently provides:

- Generated Rust REST clients (request structs, enums, typed response handling), executed by [`schematic-runtime`](../runtime/README.md)
- Generated Rust WebSocket clients and hosts (transport runtimes, event streams, correlated request-response)
- Typed API definitions and message model types for consumption

## Available Modules

### REST client modules

- `anthropic`
- `artificial_analysis`
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
- `samsung_smart_tv`
- `unfolded_circle_core_rest`

### WebSocket runtime modules

- `elevenlabs_ws`
- `unfolded_circle_core_ws`
- `unfolded_circle_dock_ws`
- `unfolded_circle_integration_ws`
- `samsung_smart_tv_remote_ws`

## Quick Start

Each REST API has a marker type (for example `OpenAI`) whose `builder()` returns an `OpenAIClient`. Every endpoint is a request struct, and every request goes through the same `send`:

```rust
use schematic_schema::openai::ListModelsRequest;
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAI::builder()
        .credentials(Credential::access_token(std::env::var("OPENAI_API_KEY")?))
        .build()?; // BuildError

    let models = client
        .send(ListModelsRequest::default(), &CallOptions::new())
        .await? // TransportError: no response head arrived
        .into_result()?; // ApiError, with the status and raw body
    println!("{}", models.data.len());
    Ok(())
}
```

`send` returns a `Response { meta, outcome }` envelope for every reply that arrived, whatever its status; `into_result()` drops the metadata when only the value matters. Text endpoints decode to `String`, binary endpoints to `bytes::Bytes`, and empty ones to `()`. When no credential is supplied and the API has no environment fallback, the request is sent anonymously rather than failing.

See the [`schematic-runtime` README](../runtime/README.md) and the [runtime topic pages](../docs/runtime/README.md) for credentials, timeouts, redirects, and `send_raw`.

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
