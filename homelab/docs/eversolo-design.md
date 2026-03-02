# Eversolo DMP-A8 Integration Design

This document describes how to integrate the Eversolo DMP-A8 music streamer into the homelab ecosystem, following the established patterns used by SonyReceiver and ArcamAmp.

## Background

The `schematic-schema` crate already contains a fully generated API client (`schematic_schema::eversolo::Eversolo`) with 24 endpoints covering device identity, remote control, music playback, power management, and display settings. All endpoints use `GET` with query parameters and require no authentication. Default port is **9529**.

## 1. Homelab Library: `Eversolo` Struct

### Location

`homelab/lib/src/eversolo.rs` (new file), re-exported from `homelab/lib/src/lib.rs`.

### Design

The `Eversolo` struct wraps the generated `schematic_schema::eversolo::Eversolo` client, providing a homelab-native interface that:

- Accepts a URL (constructed from host + port) rather than relying on the generated client's default `192.168.1.1:9529`
- Exposes high-level methods grouped by domain (device, music, power, display, remote)
- Returns homelab-friendly types (re-exports from `schematic_definitions::eversolo`)
- Follows the existing pattern where Sony uses a custom struct and Arcam uses a custom struct with TCP

```rust
// homelab/lib/src/eversolo.rs

use schematic_schema::eversolo::{
    Eversolo as EversoloClient,
    // Request types
    DeviceGetModelRequest, MusicGetStateRequest, MusicPlayOrPauseRequest,
    MusicPlayNextRequest, MusicPlayLastRequest, MusicSeekToRequest,
    MusicGetInputOutputListRequest, MusicSetInputRequest, MusicSetOutputRequest,
    MusicSetVolumeRequest, MusicSetMuteRequest, PowerGetOptionsRequest,
    PowerSetOptionRequest, RemoteSendKeyRequest, RemoteInputTextRequest,
    SystemGetScreenBrightnessRequest, SystemSetScreenBrightnessRequest,
    SystemGetKnobBrightnessRequest, SystemSetKnobBrightnessRequest,
    SystemGetVuModeListRequest, SystemSetVuModeRequest,
    SystemGetSpectrumModeListRequest, SystemSetSpectrumModeRequest,
    SystemChangeVuDisplayRequest,
};

// Re-export response types for consumers
pub use schematic_definitions::eversolo::*;

pub const DEFAULT_PORT: u16 = 9529;

pub struct Eversolo {
    client: EversoloClient,
    host: String,
    port: u16,
}
```

### Constructor

```rust
impl Eversolo {
    /// Creates a new Eversolo client for the given host and port.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        let host = host.into();
        let base_url = format!("http://{}:{}", host, port);
        Self {
            client: EversoloClient::with_base_url(base_url),
            host,
            port,
        }
    }

    /// Host accessor for display purposes.
    pub fn host(&self) -> &str { &self.host }

    /// Port accessor.
    pub fn port(&self) -> u16 { self.port }
}
```

### Method Groups

Each method delegates to `self.client.request::<ResponseType>(RequestType)`. Methods are organized into `impl` blocks by domain for clarity:

**Device**
- `get_model() -> Result<GetModelResponse>` — device identity, firmware, capabilities

**Music Control**
- `get_state() -> Result<GetStateResponse>` — playback state, position, track info, volume
- `play_or_pause() -> Result<StatusResponse>` — toggle playback
- `play_next() -> Result<StatusResponse>` — skip forward
- `play_previous() -> Result<StatusResponse>` — skip backward
- `seek_to(time_ms: i64) -> Result<StatusResponse>` — seek to millisecond position
- `get_inputs_outputs() -> Result<InputOutputListResponse>` — list audio routing
- `set_input(tag: &str) -> Result<InputOutputListResponse>` — select input
- `set_output(tag: &str) -> Result<InputOutputListResponse>` — select output
- `set_volume(level: i64) -> Result<StatusResponse>` — absolute volume (0 to max_volume)
- `set_mute(muted: bool) -> Result<StatusResponse>` — mute/unmute (converts bool to 0/1)

**Power**
- `get_power_options() -> Result<PowerOptionsResponse>` — available power actions
- `set_power_option(tag: &str) -> Result<StatusResponse>` — execute power action

**Remote**
- `send_key(key: &str) -> Result<StatusResponse>` — remote control key press
- `input_text(text: &str) -> Result<StatusResponse>` — text entry

**Display (community-discovered)**
- `get_screen_brightness() -> Result<BrightnessResponse>`
- `set_screen_brightness(index: i64) -> Result<StatusResponse>`
- `get_knob_brightness() -> Result<BrightnessResponse>`
- `set_knob_brightness(index: i64) -> Result<StatusResponse>`
- `get_vu_modes() -> Result<DisplayModeListResponse>`
- `set_vu_mode(index: i64) -> Result<StatusResponse>`
- `get_spectrum_modes() -> Result<DisplayModeListResponse>`
- `set_spectrum_mode(index: i64) -> Result<StatusResponse>`
- `change_vu_display(open_type: i64) -> Result<StatusResponse>`

### Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum EversoloError {
    #[error("Eversolo API error: {0}")]
    Api(#[from] schematic_schema::SchematicError),

    #[error("Eversolo returned error status: {0}")]
    Status(i32),
}
```

### Dependencies

Add to `homelab/lib/Cargo.toml`:

```toml
schematic-schema = { path = "../../schematic/schema" }
schematic-definitions = { path = "../../schematic/definitions" }
```

---

## 2. Homelab CLI: `eversolo` Subcommand

### Location

New `Eversolo` variant in the `Commands` enum in `homelab/cli/src/main.rs`.

### Subcommand Definition

```rust
#[derive(Subcommand)]
enum Commands {
    // ... existing Arcam, Completions, Sony ...

    /// Eversolo DMP-A8 music streamer control
    Eversolo {
        /// Device name from ~/homey.json
        #[arg(long)]
        name: Option<String>,

        /// Eversolo host IP or DNS name (overrides config)
        #[arg(long, env = "EVERSOLO")]
        host: Option<String>,

        #[command(subcommand)]
        action: EversoloAction,
    },
}
```

### Action Hierarchy

The subcommands are grouped to provide a self-documenting CLI. Shell completions will expose this hierarchy, guiding users through available operations:

```
homey eversolo [--host <addr> | --name <device>] <group> <action> [args]
```

```rust
#[derive(Subcommand)]
enum EversoloAction {
    /// Device information and identity
    #[command(subcommand)]
    Device(EversoloDeviceAction),

    /// Music playback control
    #[command(subcommand)]
    Music(EversoloMusicAction),

    /// Audio routing and volume
    #[command(subcommand)]
    Audio(EversoloAudioAction),

    /// Power management
    #[command(subcommand)]
    Power(EversoloPowerAction),

    /// Display settings (screen, knob, VU meter, spectrum)
    #[command(subcommand)]
    Display(EversoloDisplayAction),

    /// Remote control key commands
    #[command(subcommand)]
    Remote(EversoloRemoteAction),
}

#[derive(Subcommand)]
enum EversoloDeviceAction {
    /// Get device model, firmware, and network information
    Info,
}

#[derive(Subcommand)]
enum EversoloMusicAction {
    /// Get current playback state and track info
    Status,
    /// Toggle play/pause
    PlayPause,
    /// Skip to next track
    Next,
    /// Go back to previous track
    Previous,
    /// Seek to position in current track
    Seek {
        /// Position in seconds
        seconds: u64,
    },
}

#[derive(Subcommand)]
enum EversoloAudioAction {
    /// Get current volume and mute state (from music status)
    Volume,
    /// Set volume level
    SetVolume {
        /// Volume level (0 to device max)
        level: u32,
    },
    /// Mute the device
    Mute,
    /// Unmute the device
    Unmute,
    /// List available audio inputs and outputs
    Routing,
    /// Set the active audio input
    SetInput {
        /// Input tag (from `routing` output)
        tag: String,
    },
    /// Set the active audio output
    SetOutput {
        /// Output tag (from `routing` output)
        tag: String,
    },
}

#[derive(Subcommand)]
enum EversoloPowerAction {
    /// List available power options
    Options,
    /// Execute a power action
    Set {
        /// Power action tag: poweroff, reboot, screen, timeshutdown
        #[arg(value_enum)]
        action: PowerActionTag,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PowerActionTag {
    /// Power off the device
    Poweroff,
    /// Reboot the device
    Reboot,
    /// Toggle screen on/off
    Screen,
    /// Set timed shutdown
    #[value(alias = "timer")]
    Timeshutdown,
}

#[derive(Subcommand)]
enum EversoloDisplayAction {
    /// Get current screen brightness
    ScreenBrightness,
    /// Set screen brightness level
    SetScreenBrightness {
        /// Brightness index (0 to max from screen-brightness output)
        index: u32,
    },
    /// Get current knob LED brightness
    KnobBrightness,
    /// Set knob LED brightness level
    SetKnobBrightness {
        /// Brightness index (0 to max from knob-brightness output)
        index: u32,
    },
    /// List available VU meter modes
    VuModes,
    /// Set VU meter display mode
    SetVuMode {
        /// Mode index (from `vu-modes` output)
        index: u32,
    },
    /// List available spectrum display modes
    SpectrumModes,
    /// Set spectrum display mode
    SetSpectrumMode {
        /// Mode index (from `spectrum-modes` output)
        index: u32,
    },
}

#[derive(Subcommand)]
enum EversoloRemoteAction {
    /// Send a remote control key command
    Key {
        /// Remote control key name
        key: String,
    },
    /// Send text input to the device
    Text {
        /// Text to input
        text: String,
    },
}
```

### Host Resolution

Follows the established three-priority pattern:

```rust
/// Resolves the Eversolo host from --host, --name, or config auto-select.
fn resolve_eversolo(host: Option<String>, name: Option<String>) -> Result<(String, DeviceSource)> {
    // 1. Explicit --host flag (or EVERSOLO env)
    if let Some(h) = host {
        return Ok((h, DeviceSource::Flag));
    }

    let config = HomeyConfig::load().unwrap_or_default();

    // 2. --name lookup
    if let Some(ref n) = name {
        if let Some(service) = config.eversolo_devices.get(n) {
            return Ok((service.host.clone(), DeviceSource::Name(n.clone())));
        }
        let available = device_names(&config.eversolo_devices);
        return Err(color_eyre::eyre::eyre!(
            "Eversolo device '{}' not found in config.{}",
            n,
            available
        ));
    }

    // 3. Auto-select if only one device
    if config.eversolo_devices.len() == 1 {
        let (dev_name, service) = config.eversolo_devices.iter().next().unwrap();
        return Ok((service.host.clone(), DeviceSource::Auto(dev_name.clone())));
    }

    // 4. Error with helpful message
    let available = device_names(&config.eversolo_devices);
    Err(color_eyre::eyre::eyre!(
        "Host required: use --host <IP>, --name <device>, or set EVERSOLO env var.{}",
        available
    ))
}
```

### Missing Host Error Message

When neither `--host` nor `EVERSOLO` nor config entry is available:

```
Error: Host required: use --host <IP>, --name <device>, or set EVERSOLO env var.
```

If config has devices but the name doesn't match:

```
Error: Eversolo device 'bedroom' not found in config. Available: living-room, office
```

### Handler Pattern

```rust
async fn handle_eversolo(
    name: Option<String>,
    host: Option<String>,
    action: EversoloAction,
    json: bool,
) -> Result<()> {
    let (resolved_host, source) = resolve_eversolo(host, name)?;
    let eversolo = homelab::eversolo::Eversolo::new(&resolved_host, homelab::eversolo::DEFAULT_PORT);
    let suffix = device_suffix(&resolved_host, homelab::eversolo::DEFAULT_PORT, &source);
    let err_ctx = format!("Eversolo at {resolved_host}:{}", homelab::eversolo::DEFAULT_PORT);

    match action {
        EversoloAction::Device(a) => handle_eversolo_device(&eversolo, a, json, &suffix).await,
        EversoloAction::Music(a) => handle_eversolo_music(&eversolo, a, json, &suffix).await,
        EversoloAction::Audio(a) => handle_eversolo_audio(&eversolo, a, json, &suffix).await,
        EversoloAction::Power(a) => handle_eversolo_power(&eversolo, a, json, &suffix).await,
        EversoloAction::Display(a) => handle_eversolo_display(&eversolo, a, json, &suffix).await,
        EversoloAction::Remote(a) => handle_eversolo_remote(&eversolo, a, json, &suffix).await,
    }
    .wrap_err(err_ctx)
}
```

### Output Modes

All subcommands support the global `--json` flag:

- **Human mode** (default): Styled output using `Prose` and `Table` from `biscuit-terminal`, with device suffix showing connection info
- **JSON mode** (`--json`): Machine-readable `serde_json` output matching the API response structure

### Example CLI Usage

```bash
# Using environment variable
export EVERSOLO=192.168.1.50
homey eversolo device info
homey eversolo music status
homey eversolo audio volume
homey eversolo audio set-volume 45
homey eversolo audio mute
homey eversolo music play-pause
homey eversolo music next
homey eversolo power options
homey eversolo power set reboot
homey eversolo display vu-modes
homey eversolo display set-vu-mode 2
homey eversolo remote key Power

# Using --host override
homey eversolo --host 10.0.0.50 device info

# Using --name with ~/homey.json
homey eversolo --name living-room music status

# JSON output
homey eversolo --json music status
```

### Shell Completions

The nested subcommand hierarchy provides tab-completion guidance:

```
$ homey eversolo <TAB>
audio    device   display  music    power    remote

$ homey eversolo music <TAB>
next        play-pause  previous    seek        status

$ homey eversolo power set <TAB>
poweroff      reboot        screen        timeshutdown
```

---

## 3. Configuration: `~/homey.json`

### Config Extension

Add `eversolo_devices` field to `HomeyConfig` in `homelab/lib/src/config.rs`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HomeyConfig {
    #[serde(default)]
    pub sony_receivers: HashMap<String, SonyReceiverService>,

    #[serde(default)]
    pub arcam_amps: HashMap<String, ArcamAmpService>,

    /// Eversolo music streamer configurations keyed by device name
    #[serde(default)]
    pub eversolo_devices: HashMap<String, EversoloService>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EversoloService {
    /// Hostname or IP address of the Eversolo
    pub host: String,

    /// Port number (default: 9529)
    #[serde(default = "default_eversolo_port")]
    pub port: u16,
}

fn default_eversolo_port() -> u16 {
    9529
}
```

### Example Config

```json
{
  "sony_receivers": {
    "living-room": { "host": "192.168.1.100", "port": 10000 }
  },
  "arcam_amps": {
    "office": { "host": "192.168.1.101", "port": 50000 }
  },
  "eversolo_devices": {
    "living-room": { "host": "192.168.1.50" }
  }
}
```

The `#[serde(default)]` attribute ensures backward compatibility — existing config files without `eversolo_devices` will deserialize correctly with an empty map.

---

## 4. Homelab Server: OpenAPI Playground

### OpenAPI Tag

Add the Eversolo tag to the `ApiDoc` struct in `homelab/server/src/lib.rs`:

```rust
#[derive(OpenApi)]
#[openapi(
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "sony_receiver", description = "Sony ES receiver management and control"),
        (name = "arcam_amp", description = "Arcam amplifier management and control"),
        (name = "eversolo", description = "Eversolo DMP-A8 music streamer management and control"),
    )
)]
struct ApiDoc;
```

### Route Nesting

Add the Eversolo route group alongside Sony and Arcam:

```rust
pub fn build_router(state: AppState) -> Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(utoipa_axum::routes!(health))
        .routes(utoipa_axum::routes!(health_devices))
        .nest("/sony_receiver", /* ... */)
        .nest("/arcam_amp", /* ... */)
        .nest(
            "/eversolo",
            handlers::crud::eversolo_crud_routes()
                .merge(handlers::eversolo::routes_with_name()),
        )
        .split_for_parts();
    // ... rest unchanged
}
```

### CRUD Endpoints

Following the existing CRUD pattern in `homelab/server/src/handlers/crud.rs`:

```
GET    /eversolo/              — List all configured Eversolo devices
POST   /eversolo/              — Add a new Eversolo device
PUT    /eversolo/{name}        — Update an Eversolo device
DELETE /eversolo/{name}        — Remove an Eversolo device
PATCH  /eversolo/{name}/rename — Rename an Eversolo device
```

### Device Control Endpoints

New handler module `homelab/server/src/handlers/eversolo.rs`:

```
GET /eversolo/{name}/info           — Device model and firmware
GET /eversolo/{name}/state          — Current playback state
PUT /eversolo/{name}/play-pause     — Toggle play/pause
PUT /eversolo/{name}/next           — Skip to next track
PUT /eversolo/{name}/previous       — Go to previous track
GET /eversolo/{name}/volume         — Current volume
PUT /eversolo/{name}/volume         — Set volume (body: { "level": N })
PUT /eversolo/{name}/mute           — Set mute (body: { "muted": bool })
GET /eversolo/{name}/routing        — List inputs/outputs
PUT /eversolo/{name}/input          — Set input (body: { "tag": "..." })
PUT /eversolo/{name}/output         — Set output (body: { "tag": "..." })
GET /eversolo/{name}/power-options  — List power actions
PUT /eversolo/{name}/power          — Execute power action (body: { "tag": "..." })
GET /eversolo/{name}/brightness     — Screen + knob brightness
PUT /eversolo/{name}/brightness     — Set brightness (body: { "target": "screen"|"knob", "index": N })
GET /eversolo/{name}/display-modes  — VU + spectrum modes
PUT /eversolo/{name}/display-mode   — Set display mode (body: { "target": "vu"|"spectrum", "index": N })
```

All endpoints will have `#[utoipa::path(...)]` annotations for automatic OpenAPI spec generation, making them available in the Scalar interactive UI at `/explore`.

### AppState Extension

Add Eversolo fields to `homelab/server/src/state.rs`:

```rust
pub struct AppState {
    // ... existing fields ...

    /// Multi-device: Eversolo devices keyed by name
    pub eversolo_devices: Arc<RwLock<HashMap<String, EversoloService>>>,
}
```

Since the Eversolo uses HTTP (not persistent TCP connections like Sony), device connections are created per-request — same pattern as Arcam. The `EversoloService` stores host/port, and an `Eversolo` client is created on each request.

Add CRUD helper methods:

```rust
impl AppState {
    pub async fn get_eversolo(&self, name: &str) -> Option<(String, u16)> {
        let devices = self.eversolo_devices.read().await;
        devices.get(name).map(|s| (s.host.clone(), s.port))
    }

    pub async fn add_eversolo(&self, name: String, service: EversoloService) -> Result<(), ConfigError> { ... }
    pub async fn remove_eversolo(&self, name: &str) -> Result<bool, ConfigError> { ... }
    pub async fn rename_eversolo(&self, old: &str, new: String) -> Result<bool, ConfigError> { ... }
}
```

### Server Config Migration

In `homelab/server/src/config.rs`, add migration from `EVERSOLO` env var (same pattern as Sony/Arcam):

```rust
pub fn migrate_from_env(config: &mut HomeyConfig) -> bool {
    // ... existing Sony/Arcam migration ...

    if config.eversolo_devices.is_empty() {
        if let Ok(host) = std::env::var("EVERSOLO") {
            let name = generate_petname();
            let (host, port) = parse_host_port(&host, 9529);
            config.eversolo_devices.insert(name, EversoloService { host, port });
            modified = true;
        }
    }

    modified
}
```

---

## 5. What This Plan Does NOT Cover

These items are explicitly deferred to future plans:

- **Dashboard widget** on `GET /`  — the Eversolo will not be represented on the server's HTML dashboard yet
- **Heartbeat/polling** — no background polling or cached status for Eversolo (unlike Arcam which has heartbeat logic)
- **Remote control key constants** — the API accepts string keys but the known key names need research; the CLI will accept freeform strings for now
- **Seek-by-percentage** — only absolute millisecond seek is exposed by the API

---

## 6. Implementation Order

1. **Config** — Add `EversoloService` and `eversolo_devices` to `HomeyConfig` (with tests)
2. **Library** — Create `homelab/lib/src/eversolo.rs` with the `Eversolo` wrapper struct
3. **CLI** — Add `eversolo` subcommand with all action groups and host resolution
4. **Server state** — Extend `AppState` with Eversolo CRUD helpers
5. **Server CRUD routes** — Add list/create/update/delete/rename endpoints
6. **Server control routes** — Add device control endpoints with utoipa annotations
7. **Server config migration** — Add `EVERSOLO` env var migration

Each step builds on the previous and can be tested independently.

---

## 7. Dependencies Summary

| Crate | Where | Why |
|-------|-------|-----|
| `schematic-schema` | homelab/lib | Generated Eversolo API client |
| `schematic-definitions` | homelab/lib | Response types (re-exported) |
| `thiserror` | homelab/lib | `EversoloError` type |
| (existing) `clap` | homelab/cli | Subcommand parsing |
| (existing) `utoipa` | homelab/server | OpenAPI annotations |
| (existing) `utoipa-scalar` | homelab/server | Interactive API playground |
