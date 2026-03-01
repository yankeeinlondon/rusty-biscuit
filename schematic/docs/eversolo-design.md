# Eversolo DMP-A8 API — Schematic Definition Design

> **Source document:** `homelab/docs/eversolo/eversolo-tcp-ip-api.md`
> **Target module:** `schematic/definitions/src/eversolo/`
> **Base URL:** `http://<host>:9529`
> **Authentication:** None (unauthenticated LAN HTTP)
> **HTTP methods:** All endpoints use GET (including state mutations)

## Overview

The Eversolo DMP-A8 exposes a local-network HTTP control API on port 9529. The API
surface is derived from three sources of decreasing formality:

1. **Zidoo Open API** — structured docs covering device identity, remote keys,
   playback, power, and I/O enumeration
2. **Eversolo Developer Platform** — minimal official snippet (`getModel` + `sendkey`)
3. **Community integrations** — Home Assistant and Unfolded Circle reveal undocumented
   endpoints for absolute volume, mute, brightness, VU mode, and spectrum display

This design maps every known endpoint to `schematic-define` primitives and addresses
the API's well-documented quirks.

## Design decisions

### Single definition, configurable base URL

Unlike EMQX (which has Basic and Bearer variants), Eversolo has a single auth posture
(`AuthStrategy::None`). The host and port are the only variable — callers provide the
device IP at construction time. The definition uses a placeholder base URL:

```rust
base_url: "http://192.168.1.1:9529".to_string()
```

The generated client's constructor will accept a host/IP override (standard schematic
behavior).

### Path prefix strategy

Historical documents and firmware versions expose the same endpoints under two base
prefixes:

| Prefix | Origin |
|---|---|
| `/ZidooControlCenter/...` | Zidoo Open API, Eversolo developer snippet |
| `/ControlCenter/...` | Eversolo TCP PDF, some HA integration calls |

**Decision:** Define all endpoints using the `/ZidooControlCenter/` prefix (the most
consistently documented). Path-fallback probing is a **runtime** concern handled by the
homelab client, not by the schematic definition. The definition captures the canonical
paths; the client can retry with the alternate prefix on 404.

### Endpoint grouping

Endpoints fall into five functional groups. Each group becomes a section of the endpoint
vector and gets a descriptive prefix on its `id` field:

| Group | ID prefix | Path root | Source |
|---|---|---|---|
| Device | `Device*` | `/ZidooControlCenter/` | Zidoo doc, Eversolo dev |
| Remote | `Remote*` | `/ZidooControlCenter/RemoteControl/` | Zidoo doc, Eversolo dev |
| Music | `Music*` | `/ZidooMusicControl/v2/` | Zidoo doc |
| Power | `Power*` | `/ZidooMusicControl/v2/` | Zidoo doc |
| System | `System*` | `/SystemSettings/displaySettings/` | Community (undocumented) |

### Response type mapping

The device returns JSON bodies with incorrect `Content-Type: text/plain` headers on
some endpoints. The schematic definition marks all structured endpoints as
`ApiResponse::json_type(...)` regardless — content-type leniency is a client-side
concern (the homelab crate's reqwest client will parse JSON irrespective of headers,
matching the `content_type=None` pattern used by the Home Assistant integration).

### Scope exclusions

The following are **not** modeled in this schematic definition:

- **Wake-on-LAN** — UDP broadcast, not HTTP REST
- **IR codes** — out-of-band fallback, not HTTP
- **Discovery/pairing** — protocol unknown (likely SSDP/mDNS), not documented

## Function signature

```rust
pub fn define_eversolo_api() -> RestApi
```

Returns a single `RestApi`. No variants needed.

## RestApi definition

```rust
RestApi {
    name: "Eversolo".to_string(),
    description: "Eversolo DMP-A8 local-network HTTP control API (Zidoo lineage)"
        .to_string(),
    base_url: "http://192.168.1.1:9529".to_string(),
    docs_url: Some("https://eversolo.com/Support/developer/".to_string()),
    auth: AuthStrategy::None,
    env_auth: vec![],
    env_username: None,
    headers: vec![],
    endpoints: vec![/* see below */],
    module_path: Some("eversolo".to_string()),
    request_suffix: None,
    env_mapping: Some(EnvMapping::default()),
}
```

## Endpoint catalog

### Group 1 — Device identity

| ID | Method | Path | Request | Response |
|---|---|---|---|---|
| `DeviceGetModel` | GET | `/ZidooControlCenter/getModel` | — | `GetModelResponse` |

**`GetModelResponse`** fields (from Zidoo doc + Eversolo developer snippet):

```rust
pub struct GetModelResponse {
    pub status: i32,
    pub model: String,
    pub ip: String,
    pub net_mac: String,          // wired MAC
    pub wifi_mac: String,
    pub firmware: String,
    pub android_version: Option<String>,
    pub able_remote_boot: Option<bool>,
    pub has_eq_setting: Option<bool>,
}
```

### Group 2 — Remote control (key injection)

| ID | Method | Path | Params | Response |
|---|---|---|---|---|
| `RemoteSendKey` | GET | `/ZidooControlCenter/RemoteControl/sendkey` | `key` | `StatusResponse` |
| `RemoteInputText` | GET | `/ZidooControlCenter/RemoteControl/inputtext` | `text` | `StatusResponse` |

Query parameter `key` accepts `Key.*` constants. The full key list is large —
representative keys:

- Navigation: `Key.Up`, `Key.Down`, `Key.Left`, `Key.Right`, `Key.Ok`, `Key.Back`,
  `Key.Home`, `Key.Menu`
- Volume: `Key.VolumeUp`, `Key.VolumeDown`, `Key.Mute`
- Playback: `Key.MediaPlay`, `Key.MediaPause`, `Key.MediaPlayPause`,
  `Key.MediaNext`, `Key.MediaPrev`, `Key.MediaStop`
- Power: `Key.PowerOn`, `Key.PowerOff`
- Screen: `Key.Screen.ON`, `Key.Screen.OFF`
- Output: `Key.SPDIF`, `Key.Coaxial`, `Key.XLRRCA`, `Key.Bluetooth`

**`StatusResponse`** (shared across many endpoints):

```rust
pub struct StatusResponse {
    pub status: i32,  // 200 = success
}
```

### Group 3 — Music control (playback + state)

| ID | Method | Path | Params | Request | Response |
|---|---|---|---|---|---|
| `MusicGetState` | GET | `/ZidooMusicControl/v2/getState` | — | — | `GetStateResponse` |
| `MusicPlayOrPause` | GET | `/ZidooMusicControl/v2/playOrPause` | — | — | `StatusResponse` |
| `MusicPlayNext` | GET | `/ZidooMusicControl/v2/playNext` | — | — | `StatusResponse` |
| `MusicPlayLast` | GET | `/ZidooMusicControl/v2/playLast` | — | — | `StatusResponse` |
| `MusicSeekTo` | GET | `/ZidooMusicControl/v2/seekTo` | `time` (ms) | — | `StatusResponse` |
| `MusicGetInputOutputList` | GET | `/ZidooMusicControl/v2/getInputAndOutputList` | — | — | `InputOutputListResponse` |
| `MusicSetInput` | GET | `/ZidooMusicControl/v2/setInputList` | `tag` | — | `InputOutputListResponse` |
| `MusicSetOutput` | GET | `/ZidooMusicControl/v2/setOutInputList` | `tag` | — | `InputOutputListResponse` |
| `MusicSetVolume` | GET | `/ZidooMusicControl/v2/setDevicesVolume` | `volume` | — | `StatusResponse` |
| `MusicSetMute` | GET | `/ZidooMusicControl/v2/setMuteVolume` | `isMute` (0\|1) | — | `StatusResponse` |

**`GetStateResponse`** (from Zidoo `getState` doc):

```rust
pub struct GetStateResponse {
    pub status: i32,
    pub state: i32,              // playback state enum
    pub position: Option<i64>,   // current position in ms
    pub duration: Option<i64>,   // total duration in ms
    pub playing_music: Option<PlayingMusic>,
    pub volume_data: Option<VolumeData>,
}

pub struct PlayingMusic {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<i32>,
}

pub struct VolumeData {
    pub current_volume: i32,     // note: API spells it "currenttVolume"
    pub max_volume: i32,
    pub min_volume: Option<i32>,
    pub is_mute: bool,
    pub volume_db: Option<String>, // displayed dB string
    pub is_volume_enable: Option<bool>,
}
```

**`InputOutputListResponse`**:

```rust
pub struct InputOutputListResponse {
    pub status: i32,
    pub input_data: Vec<InputItem>,
    pub output_data: Vec<OutputItem>,
    pub output_info: Option<serde_json::Value>, // varies by model/output
}

pub struct InputItem {
    pub name: String,
    pub tag: String,
    pub icon: Option<String>,
    pub sorted_index: Option<i32>,
}

pub struct OutputItem {
    pub name: String,
    pub tag: String,
    pub enable: bool,
    pub icon: Option<String>,
    pub sorted_index: Option<i32>,
}
```

### Group 4 — Power control

| ID | Method | Path | Params | Response |
|---|---|---|---|---|
| `PowerGetOptions` | GET | `/ZidooMusicControl/v2/getPowerOption` | — | `PowerOptionsResponse` |
| `PowerSetOption` | GET | `/ZidooMusicControl/v2/setPowerOption` | `tag` | `StatusResponse` |

**`PowerOptionsResponse`**:

```rust
pub struct PowerOptionsResponse {
    pub status: i32,
    pub data: Vec<PowerOption>,
}

pub struct PowerOption {
    pub name: String,   // localized display name
    pub tag: String,    // e.g., "poweroff", "reboot", "screen", "timeshutdown"
}
```

### Group 5 — System settings (community-discovered, undocumented)

| ID | Method | Path | Params | Response |
|---|---|---|---|---|
| `SystemGetScreenBrightness` | GET | `/SystemSettings/displaySettings/getScreenBrightness` | — | `BrightnessResponse` |
| `SystemSetScreenBrightness` | GET | `/SystemSettings/displaySettings/setScreenBrightness` | `index` | `StatusResponse` |
| `SystemGetKnobBrightness` | GET | `/SystemSettings/displaySettings/getKnobBrightness` | — | `BrightnessResponse` |
| `SystemSetKnobBrightness` | GET | `/SystemSettings/displaySettings/setKnobBrightness` | `index` | `StatusResponse` |
| `SystemGetVuModeList` | GET | `/SystemSettings/displaySettings/getVUModeList` | — | `DisplayModeListResponse` |
| `SystemSetVuMode` | GET | `/SystemSettings/displaySettings/setVUMode` | `index` | `StatusResponse` |
| `SystemGetSpectrumModeList` | GET | `/SystemSettings/displaySettings/getSpPlayModeList` | — | `DisplayModeListResponse` |
| `SystemSetSpectrumMode` | GET | `/SystemSettings/displaySettings/setSpPlayModeList` | `index` | `StatusResponse` |
| `SystemChangeVuDisplay` | GET | `/ZidooMusicControl/v2/changVUDisplay` | `openType` | `StatusResponse` |

**`BrightnessResponse`**:

```rust
pub struct BrightnessResponse {
    pub status: i32,
    pub index: i32,       // current brightness level
    pub max: Option<i32>, // maximum brightness
}
```

**`DisplayModeListResponse`**:

```rust
pub struct DisplayModeListResponse {
    pub status: i32,
    pub data: Vec<DisplayMode>,
    pub current_index: Option<i32>,
}

pub struct DisplayMode {
    pub name: String,
    pub index: i32,
}
```

## Query parameters via `EndpointParams`

All parameterized endpoints use query parameters (no path parameters, no request
bodies). Each endpoint that accepts parameters uses `EndpointParams`:

```rust
// Example: RemoteSendKey
Endpoint {
    id: "RemoteSendKey".to_string(),
    method: RestMethod::Get,
    path: "/ZidooControlCenter/RemoteControl/sendkey".to_string(),
    description: "Send a remote control key command to the device".to_string(),
    request: None,
    response: ApiResponse::json_type("StatusResponse"),
    headers: vec![],
    params: Some(
        EndpointParams::default()
            .with_query_param("key", QueryParamType::String, true, Some("Remote key constant (e.g., Key.VolumeUp)"))
    ),
}

// Example: MusicSeekTo
Endpoint {
    id: "MusicSeekTo".to_string(),
    method: RestMethod::Get,
    path: "/ZidooMusicControl/v2/seekTo".to_string(),
    description: "Seek to a position in the current track".to_string(),
    request: None,
    response: ApiResponse::json_type("StatusResponse"),
    headers: vec![],
    params: Some(
        EndpointParams::default()
            .with_query_param("time", QueryParamType::Integer, true, Some("Target position in milliseconds"))
    ),
}

// Example: MusicSetMute
Endpoint {
    id: "MusicSetMute".to_string(),
    method: RestMethod::Get,
    path: "/ZidooMusicControl/v2/setMuteVolume".to_string(),
    description: "Set or clear mute state".to_string(),
    request: None,
    response: ApiResponse::json_type("StatusResponse"),
    headers: vec![],
    params: Some(
        EndpointParams::default()
            .with_query_param("isMute", QueryParamType::Integer, true, Some("Mute state: 0 = unmuted, 1 = muted"))
    ),
}
```

Endpoints with **no parameters** (e.g., `MusicGetState`, `MusicPlayOrPause`) set
`params: None`.

## Serde field renaming

The device API uses camelCase JSON keys with some irregularities. Key renames needed in
response types:

```rust
// GetStateResponse
#[serde(rename = "playingMusic")]
pub playing_music: Option<PlayingMusic>,

#[serde(rename = "volumeData")]
pub volume_data: Option<VolumeData>,

// VolumeData — note the double-t typo in the actual API
#[serde(rename = "currenttVolume")]
pub current_volume: i32,

#[serde(rename = "maxVolume")]
pub max_volume: i32,

#[serde(rename = "minVolume")]
pub min_volume: Option<i32>,

#[serde(rename = "isMute")]
pub is_mute: bool,

#[serde(rename = "volumeDb")]
pub volume_db: Option<String>,

#[serde(rename = "isVolumeEnable")]
pub is_volume_enable: Option<bool>,

// InputOutputListResponse
#[serde(rename = "inputData")]
pub input_data: Vec<InputItem>,

#[serde(rename = "outputData")]
pub output_data: Vec<OutputItem>,

#[serde(rename = "outputInfo")]
pub output_info: Option<serde_json::Value>,

// InputItem / OutputItem
#[serde(rename = "sortedIndex")]
pub sorted_index: Option<i32>,

// DisplayModeListResponse
#[serde(rename = "currentIndex")]
pub current_index: Option<i32>,
```

All response structs should use `#[serde(rename_all = "camelCase")]` at the struct
level, with explicit `#[serde(rename = "...")]` only for the `currenttVolume` typo.

## File structure

```
schematic/definitions/src/eversolo/
├── mod.rs      — define_eversolo_api() + endpoint construction
└── types.rs    — all request/response types
```

### mod.rs outline

```rust
mod types;
pub use types::*;

use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, EndpointParams,
    EnvMapping, QueryParamType, RestApi, RestMethod,
};

pub fn define_eversolo_api() -> RestApi {
    RestApi {
        name: "Eversolo".to_string(),
        description: "Eversolo DMP-A8 local-network HTTP control API".to_string(),
        base_url: "http://192.168.1.1:9529".to_string(),
        docs_url: Some("https://eversolo.com/Support/developer/".to_string()),
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: build_endpoints(),
        module_path: Some("eversolo".to_string()),
        request_suffix: None,
        env_mapping: Some(EnvMapping::default()),
    }
}

fn build_endpoints() -> Vec<Endpoint> {
    let mut eps = Vec::new();
    eps.extend(device_endpoints());
    eps.extend(remote_endpoints());
    eps.extend(music_endpoints());
    eps.extend(power_endpoints());
    eps.extend(system_endpoints());
    eps
}

fn device_endpoints() -> Vec<Endpoint> { /* ... */ }
fn remote_endpoints() -> Vec<Endpoint> { /* ... */ }
fn music_endpoints() -> Vec<Endpoint> { /* ... */ }
fn power_endpoints() -> Vec<Endpoint> { /* ... */ }
fn system_endpoints() -> Vec<Endpoint> { /* ... */ }
```

### types.rs outline

```rust
use serde::{Deserialize, Serialize};

// ─── Shared ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: i32,
}

// ─── Device ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetModelResponse { /* ... */ }

// ─── Music / State ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStateResponse { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayingMusic { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeData {
    #[serde(rename = "currenttVolume")]  // typo in actual API
    pub current_volume: i32,
    /* ... */
}

// ─── I/O ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputOutputListResponse { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputItem { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputItem { /* ... */ }

// ─── Power ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerOptionsResponse { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerOption { /* ... */ }

// ─── System / Display ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrightnessResponse { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayModeListResponse { /* ... */ }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayMode { /* ... */ }
```

## Known quirks and how this design handles them

### Content-Type mismatch

**Problem:** Some endpoints return `text/plain` despite JSON bodies.

**Design response:** Mark all structured responses as `ApiResponse::json_type(...)`.
The generated client (or the homelab wrapper) must configure reqwest to ignore
content-type when parsing JSON. This matches the aiohttp `content_type=None` pattern
from the Home Assistant integration.

### All endpoints are GET

**Problem:** Even state-mutating operations (set volume, power off, switch output) use
HTTP GET.

**Design response:** Faithfully model as `RestMethod::Get` with query parameters. No
request bodies exist. This is unconventional but correct for this API.

### "currenttVolume" typo

**Problem:** The API consistently spells the field `currenttVolume` (double-t).

**Design response:** Use `#[serde(rename = "currenttVolume")]` on the Rust field
`current_volume`. Document the typo so future maintainers don't "fix" it.

### Optional fields proliferate

**Problem:** Response shapes vary by firmware version, model variant, and current
device state (e.g., `outputInfo` only populated for certain output types).

**Design response:** Use `Option<T>` liberally with
`#[serde(skip_serializing_if = "Option::is_none")]`. The `GetModelResponse` capability
flags (`able_remote_boot`, `has_eq_setting`) are optional since they may not appear on
all firmware.

### Path prefix aliases

**Problem:** `/ZidooControlCenter/` vs `/ControlCenter/` ambiguity.

**Design response:** The definition uses `/ZidooControlCenter/` consistently. Fallback
probing is a runtime concern for the homelab client, not the definition.

### Community-discovered endpoints are undocumented

**Problem:** SystemSettings endpoints (`brightness`, `VU mode`, `spectrum mode`) and
absolute volume/mute endpoints have no official documentation.

**Design response:** Include them in the definition with descriptions noting their
community-discovered status. Group them separately (`system_endpoints()`) so they can
be easily identified or excluded.

## Endpoint count summary

| Group | Count |
|---|---|
| Device | 1 |
| Remote | 2 |
| Music | 10 |
| Power | 2 |
| System | 9 |
| **Total** | **24** |

## References

- [Eversolo Developer Platform](https://eversolo.com/Support/developer/)
- [Zidoo Open API](https://apidoc.zidoo.tv/319438933e0) (getModel, sendkey, getState, I/O, power, playback)
- [Home Assistant Eversolo integration](https://github.com/hchris1/Eversolo) (community endpoints, parsing workarounds)
- [Unfolded Circle integration](https://github.com/mase1981/uc-intg-eversolo) (discovery analysis, display quirks)
- Source research: `homelab/docs/eversolo/eversolo-tcp-ip-api.md`
