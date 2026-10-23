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

Each REST API is one client struct (for example `OpenAI`): `OpenAI::new()` returns a usable client, and settings are chained onto it. Every endpoint is a method on that struct, so nothing beyond the prelude needs importing:

```rust
use schematic_schema::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = OpenAI::new().api_key(std::env::var("OPENAI_API_KEY")?);

    let models = client.list_models().await?; // CallError: no reply, or a failed one
    println!("{}", models.data.len());

    let model = client.retrieve_model("gpt-4").await?; // required values are arguments
    println!("{}", model.id);
    Ok(())
}
```

Awaiting an endpoint call returns the data or a `CallError`; `.full()` returns the `Response { meta, outcome }` envelope for every reply that arrived, whatever its status. Text endpoints decode to `String`, binary endpoints to `bytes::Bytes`, and empty ones to `()`; a streaming endpoint returns an `ItemStream<T>` or `ByteStream` read item by item, and an endpoint with several success shapes returns its `{Id}Response` enum (see [success shapes](../docs/responses-and-errors.md#generated-success-and-failure-types)). When no credential is supplied and the API has no environment fallback, the request is sent anonymously rather than failing.

See [Calling an API](../docs/runtime/calling-an-api.md) for endpoint methods, setters, per-call overrides, and `CallError`, and the [`schematic-runtime` README](../runtime/README.md) and the [runtime topic pages](../docs/runtime/README.md) for credentials, timeouts, redirects, and `send_raw`.

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
