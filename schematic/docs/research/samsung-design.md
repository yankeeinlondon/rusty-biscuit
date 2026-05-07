# Samsung Smart TV API (S95C-First) — Schematic Definition Design

> **Primary source precedence:** `homelab/docs/samsung-s95c.md` > `homelab/docs/samsung/samsung-smart-tv-ip-apis.md`
> **Target generated REST client:** `SamsungSmartTv`
> **Package area:** `schematic`
> **Definition primitives:** `schematic-define` (`RestApi`, `Endpoint`, `ApiRequest`, `ApiResponse`, `EndpointParams`, `WebSocketApi`, `MessageSchema`, etc.)

## Source Reconciliation

### Priority rule application

The requested precedence is to prefer `homelab/docs/samsung-s95c.md` over `homelab/docs/samsung/samsung-smart-tv-ip-apis.md` when conflicts exist.

Current state:

- `homelab/docs/samsung-s95c.md` currently contains only the title and no technical constraints.
- `homelab/docs/samsung/samsung-smart-tv-ip-apis.md` contains the actionable protocol and endpoint detail.

Design implication:

- There are no explicit conflicts to resolve.
- The design uses S95C-first assumptions from the detailed Samsung IP API document, while explicitly marking model/firmware-variant behavior as non-guaranteed.

## Goal

Define Samsung LAN control APIs in `schematic-definitions` so that:

1. The primary generated REST client is named `SamsungSmartTv`.
2. The most reliable modern-Tizen control surfaces are modeled (S95C-focused).
3. The definition follows Schematic design best practices for typing, docs, and performance.

## Non-Goals

This design does **not** attempt to model protocols that cannot be represented by `schematic-define` REST/WebSocket primitives:

- SSDP/mDNS discovery (UDP multicast flows)
- UPnP SOAP control URL discovery process itself
- SDB developer tooling protocol (`:26101`)

Those belong in runtime/device-discovery libraries (for example `homelab`), not generated HTTP/WS clients.

## S95C-Focused Surface Selection

For modern Samsung TVs (including S95C-class behavior), the design models:

1. Smart View HTTP API (`:8001`) for device info and app launch attempts.
2. Samsung remote control WebSocket channel (`/api/v2/channels/samsung.remote.control`) with tokenized reconnect support.

The design intentionally excludes legacy TCP `:55000` control because S95C is in the modern Tizen era.

## Proposed Definitions

## 1. Primary REST API Definition

### Function signature

```rust
pub fn define_samsung_smart_tv_api() -> RestApi
```

### `RestApi` shape

```rust
RestApi {
    name: "SamsungSmartTv".to_string(),
    description: "Samsung Smart TV LAN API (S95C-focused modern Tizen subset)".to_string(),
    base_url: "http://192.168.1.1:8001".to_string(),
    docs_url: Some(
        "https://developer.samsung.com/smarttv/develop/extension-libraries/smart-view-sdk/receiver-apps/debugging.html".to_string()
    ),
    auth: AuthStrategy::None,
    env_auth: vec![],
    env_username: None,
    headers: vec![],
    endpoints: build_samsung_smart_tv_endpoints(),
    module_path: Some("samsung_smart_tv".to_string()),
    request_suffix: None,
    env_mapping: Some(EnvMapping::default()),
}
```

### Why `AuthStrategy::None`

- The modeled REST endpoints do not require a standard bearer/basic/API-key header auth scheme.
- Approval/token behavior is handled on the remote-control WebSocket side via query parameters and TV-side user approval flow.

### Endpoint catalog (REST)

Use `Vec::with_capacity(4)` in endpoint builders.

| ID                        | Method | Path                            | Request | Response                                              | Notes                                    |
|---------------------------|--------|---------------------------------|---------|-------------------------------------------------------|------------------------------------------|
| `GetDeviceInfo`           | `GET`  | `/api/v2/`                      | none    | `ApiResponse::json_type("SamsungDeviceInfoResponse")` | Official Smart View info endpoint        |
| `GetServerLogs`           | `GET`  | `/logs/`                        | none    | `ApiResponse::Text`                                   | Requires dev mode on some firmware       |
| `LaunchApplicationById`   | `POST` | `/api/v2/applications/{app_id}` | none    | `ApiResponse::Empty`                                  | Pattern A observed in ecosystem          |
| `LaunchApplicationByName` | `POST` | `/ws/apps/{app_name}`           | none    | `ApiResponse::Empty`                                  | Pattern B fallback observed in ecosystem |

### Rationale for response typing

- `GetDeviceInfo`: JSON object expected.
- `GetServerLogs`: textual log output.
- App launch endpoints: response body format is firmware-variable; `Empty` avoids over-constraining client deserialization while still enforcing success status handling.

## 2. Companion Remote Control WebSocket Definition

To model remote key transport, add a second definition:

```rust
pub fn define_samsung_smart_tv_remote_ws_api() -> WebSocketApi
```

This is separate from REST because `schematic-define` models HTTP and WebSocket independently.

### `WebSocketApi` shape

```rust
WebSocketApi {
    name: "SamsungSmartTvRemote".to_string(),
    description: "Samsung Smart TV remote-control websocket API (S95C-focused)".to_string(),
    base_url: "wss://192.168.1.1:8002".to_string(),
    docs_url: Some(
        "https://developer.samsung.com/smarttv/develop/extension-libraries/smart-view-sdk/receiver-apps/debugging.html".to_string()
    ),
    auth: AuthStrategy::None,
    env_auth: vec![],
    endpoints: vec![/* RemoteControl channel */],
    runtime: Some(WebSocketRuntimeHints {
        frame_format: FrameFormat::JsonText,
        supports_reconnect: true,
        request_id_type: RequestIdType::String,
    }),
}
```

### Why WSS `:8002` default

- S95C-first posture should favor encrypted channel by default.
- Generated clients can still use `with_base_url("ws://<tv-ip>:8001")` when older or local-only behavior requires non-TLS.

### Endpoint catalog (WebSocket)

Single endpoint:

| ID              | Path                                      | Connection Params                                       | Notes                                                 |
|-----------------|-------------------------------------------|---------------------------------------------------------|-------------------------------------------------------|
| `RemoteControl` | `/api/v2/channels/samsung.remote.control` | `name` (required `String`), `token` (optional `String`) | `name` is base64 client name in known implementations |

Connection params should use:

```rust
connection_params: vec![
    ConnectionParam {
        name: "name".to_string(),
        param_type: ParamType::String,
        required: true,
        description: Some("Base64-encoded client name used by Samsung remote channel".to_string()),
    },
    ConnectionParam {
        name: "token".to_string(),
        param_type: ParamType::String,
        required: false,
        description: Some("Previously approved remote token to bypass repeated on-TV prompts".to_string()),
    },
],
```

### Message schema catalog

Model both typed command payloads and envelope-style server events:

| Message name               | Direction     | Schema                           |
|----------------------------|---------------|----------------------------------|
| `RemoteControlCommand`     | Client        | `SamsungRemoteControlCommand`    |
| `ChannelConnectEvent`      | Server        | `SamsungRemoteConnectEvent`      |
| `ChannelUnauthorizedEvent` | Server        | `SamsungRemoteUnauthorizedEvent` |
| `ChannelErrorEvent`        | Server        | `SamsungRemoteErrorEvent`        |
| `ChannelEnvelope`          | Bidirectional | `SamsungRemoteEnvelope`          |

Use `ConnectionLifecycle::default()` (no documented mandatory open/close control frames).

## Type Design (Best-Practices Aligned)

## REST types

### `SamsungDeviceInfoResponse`

Because `/api/v2/` fields vary by firmware, model a stable typed core plus extensibility:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamsungDeviceInfoResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<SamsungDeviceInfo>,
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}
```

`Eq`/`Hash` are intentionally omitted due `serde_json::Value` in `extra`.

### `SamsungDeviceInfo`

Use optional fields only where data is firmware-dependent; document reasons per field.

Fields to include initially:

- `model`
- `model_name` (`#[serde(rename = "modelName")]`)
- `os`
- `network_type` (`#[serde(rename = "networkType")]`)
- `resolution`
- `token_auth_support` (`#[serde(rename = "TokenAuthSupport")]` if required by observed payload)
- `extra: BTreeMap<String, Value>` flatten

## WebSocket types

### Remote command model

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteControlCommand {
    pub method: SamsungRemoteMethod,
    pub params: SamsungRemoteControlParams,
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SamsungRemoteControlParams {
    #[serde(rename = "Cmd")]
    pub cmd: SamsungRemoteCommandAction,
    #[serde(rename = "DataOfCmd")]
    pub data_of_cmd: SamsungRemoteKey,
    #[serde(rename = "Option")]
    pub option: String,
    #[serde(rename = "TypeOfRemote")]
    pub type_of_remote: SamsungRemoteType,
}
```

Design notes:

- Keep upstream wire-key casing with `serde(rename = ...)`.
- `option` remains `String` because real payloads frequently use string booleans (`"false"`) instead of strict JSON boolean.
- Use enums for fixed discriminants, with passthrough fallback where needed.

### Envelope/event models with deferred payload parsing

Per best-practices, use `RawValue`:

```rust
use serde_json::value::RawValue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamsungRemoteEnvelope {
    pub event: Option<SamsungRemoteEventName>,
    pub method: Option<SamsungRemoteMethod>,
    pub data: Option<Box<RawValue>>,
}
```

This avoids eagerly parsing all event payloads into `serde_json::Value` before routing.

### Suggested discriminant enums

- `SamsungRemoteMethod`
  - `#[serde(rename = "ms.remote.control")] MsRemoteControl`
  - `#[serde(rename = "ms.channel.emit")] MsChannelEmit`
  - `Other(String)` via untagged wrapper pattern
- `SamsungRemoteEventName`
  - `MsChannelConnect`
  - `MsChannelUnauthorized`
  - `MsError`
  - `Other(String)`
- `SamsungRemoteType`
  - `SendRemoteKey`
  - `Other(String)`
- `SamsungRemoteCommandAction`
  - `Click`, `Press`, `Release`, `Other(String)`

## Request/Endpoint Naming Strategy

Use explicit, non-ambiguous endpoint IDs and preserve a stable future expansion path.

REST prefix strategy:

- `Get*` for reads
- `Launch*` for app launch actions

WebSocket message type strategy:

- `SamsungRemote*` prefix for all model types

Body-type naming:

- If future POST JSON bodies are added, use `*Body` suffix to avoid collision with generated `*Request` types.

## Module Layout Plan

## `schematic-definitions`

```text
schematic/definitions/src/
  samsung_smart_tv/
    mod.rs
    types.rs
    remote_ws/
      mod.rs
      types.rs
```

Exports from `samsung_smart_tv/mod.rs`:

- `define_samsung_smart_tv_api()`
- `define_samsung_smart_tv_remote_ws_api()`
- REST and WS model re-exports

Top-level wiring updates:

- `schematic/definitions/src/lib.rs`
  - `pub mod samsung_smart_tv;`
  - re-export the new define functions

## `schematic-gen` wiring

REST API registration updates in `schematic/gen/src/main.rs`:

- Add `samsung-smart-tv` to `AVAILABLE_APIS`
- Add `resolve_api("samsung-smart-tv")`
- Add to `resolve_all_apis()`

WebSocket codegen wiring updates in `schematic/gen/src/output.rs`:

- Add a WS definition module entry, for example `samsung_smart_tv_remote_ws`
- Add `assemble_ws_definition_module` match arm to call `define_samsung_smart_tv_remote_ws_api()`
- Re-export path should reference `schematic_definitions::samsung_smart_tv::remote_ws::*`

WebSocket lowering test updates:

- Include new WS definition in `schematic/gen/src/ws_codegen/plan.rs` `lower_all_known_ws_apis` test list.

## `schematic-schema` expected generated modules

- `samsung_smart_tv.rs` (REST; primary client `SamsungSmartTv`)
- `samsung_smart_tv_remote_ws.rs` (WS runtime client)

## Endpoint Details

### REST endpoint detail blocks

#### 1. `GetDeviceInfo`

- Method: `GET`
- Path: `/api/v2/`
- Request: none
- Response: `SamsungDeviceInfoResponse`
- Description should include:
  - This is the canonical Smart View info endpoint for modern Samsung TVs.
  - Payload fields vary by model/firmware; unknown fields are preserved.

#### 2. `GetServerLogs`

- Method: `GET`
- Path: `/logs/`
- Response: `Text`
- Description should include:
  - Debug/developer-facing endpoint.
  - May require developer mode and can return non-success when disabled.

#### 3. `LaunchApplicationById`

- Method: `POST`
- Path: `/api/v2/applications/{app_id}`
- Response: `Empty`
- Description should include:
  - Firmware-dependent support.
  - Use when Samsung app ID is known (for example package-like IDs).

#### 4. `LaunchApplicationByName`

- Method: `POST`
- Path: `/ws/apps/{app_name}`
- Response: `Empty`
- Description should include:
  - Compatibility fallback for firmware that does not honor `/api/v2/applications/{id}`.
  - Recommended runtime strategy: attempt ID endpoint first, fallback to name endpoint.

### WebSocket endpoint detail block

#### `RemoteControl`

- Path: `/api/v2/channels/samsung.remote.control`
- Required connection param: `name`
- Optional connection param: `token`
- Expected behavior:
  - First connect may trigger on-TV approval prompt.
  - Approved sessions provide/reuse token for future reconnects.

## Performance and Robustness Requirements

1. Pre-allocate endpoint/message vectors with `Vec::with_capacity(...)`.
2. Use `RawValue` for WebSocket envelope payloads (`data`), not `serde_json::Value`.
3. Keep `String` ownership in generated models, but document high-frequency polling concerns for `/api/v2/` if repeatedly called.
4. Treat all non-standard/firmware-variable fields as optional or flattened extras with clear docs.

## Documentation Requirements in Code

Model and endpoint docs should explicitly include:

- Meaning of remote key examples (for example `KEY_VOLUP`, `KEY_HOME`, `KEY_POWER`).
- Why certain fields are optional (firmware variance).
- How to parse envelope payload by event name when using `RawValue`.
- Runtime guidance for fallback behavior:
  - WSS `:8002` first when available
  - fallback to WS `:8001`
  - app-launch path fallback order

## Validation and Test Plan

### Unit tests in `schematic-definitions`

1. Metadata/auth tests

2. REST API name/base/docs/auth

3. WS API name/base/docs/auth/runtime hints

4. Endpoint integrity tests

5. REST endpoint count and path/method assertions

6. WS endpoint count and required connection params

7. Type serde tests

8. Remote command serialization key casing (`Cmd`, `DataOfCmd`, `TypeOfRemote`)

9. Envelope parse with deferred `RawValue`

10. Device info parse with unknown-field flatten behavior

### Generator and workspace verification

Run from `schematic/`:

```bash
just test
just lint
just generate-one samsung-smart-tv
cargo check -p schematic-schema
```

If WS module wiring is added as part of this work, also run full generation:

```bash
just generate
```

## Rollout Phases

1. **Phase 1: REST baseline (`SamsungSmartTv`)**

- Implement REST definition and core types.
- Register API in `schematic-gen` as `samsung-smart-tv`.

1. **Phase 2: Remote WS companion**

- Implement `define_samsung_smart_tv_remote_ws_api` and WS models.
- Wire WS generation module list and tests.

1. **Phase 3: Hardening and docs**

- Expand known key enums and event payload specializations based on real capture data.
- Add richer docs/examples in definition modules.

## Open Questions (Must Be Settled by Device Validation)

1. Whether S95C firmware in target environment accepts both `ws://:8001` and `wss://:8002` for remote control.
2. Which app-launch endpoint is actually honored on installed firmware (`/api/v2/applications/{id}` vs `/ws/apps/{name}`).
3. Exact `/api/v2/` field guarantees for strict nullability tightening in later revisions.
4. Whether additional Samsung WS channels should be modeled (for example art-mode-specific channels) after empirical confirmation.

## Final Design Summary

- The primary generated API client will be `SamsungSmartTv` via a new REST definition.
- A dedicated companion WebSocket definition will model tokenized remote control without forcing header-based auth.
- The type system will prioritize strong discriminants and deferred payload parsing (`RawValue`) while preserving compatibility with firmware variability through documented optional/flattened fields.
