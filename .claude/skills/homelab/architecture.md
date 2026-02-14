# Homelab Architecture

## Library (`homelab/lib`)

### Modules

| Module | Lines | Description |
|--------|-------|-------------|
| `sony_receiver` | ~1296 | Sony ES receiver JSON-RPC client (HTTP) |
| `arcam` | ~238 | Arcam PA240/PA410/PA720 binary TCP protocol |
| `network` | ~23 | `Host` enum (V4, V6, DNS) with Display |

Stub files exist for `ha`, `mqtt`, `node_red`, `ubiquiti` but are empty and not exported from `lib.rs`.

### Sony Receiver (`sony_receiver.rs`)

The largest module. Key types:

- **`SonyReceiver`** — Main client struct. Holds `Host` + port, uses `reqwest` internally
- **`SonyReceiverEndpoints`** — Enum of 8 JSON-RPC endpoints (System, Audio, AvContent, AccessControl, AppControl, Guide, Encryption, Browser)
- **Action enums** — `SystemAction`, `AudioAction`, `AvContentAction` etc. map to JSON-RPC method names
- **Data types** — `PowerStatusResult`, `VolumeInfo`, `InputSource`, `PlayingContentInfo`, `SystemInformation`, `MethodSignature`, `GenericSettingResult`, `SwUpdateInfo`, `EciaInfo`, `ContentItem`, `PlaybackFunction`, `SupportedPlaybackFunction`
- **Error type** — `SonyError` (Http, Json, Api, NoContent, InvalidResponse)
- **Helpers** — `unwrap_sony_result()`, `value_as_string()` for dealing with response format quirks

Key API methods on `SonyReceiver`:
- Power: `get_power_status()`, `set_power(bool)`
- Volume: `get_volume()`, `set_volume(u32)`, `get_mute_status()`, `set_mute(bool)`
- Input: `list_inputs()`, `get_current_input()`, `set_input(&str)`
- Content: `get_scheme_list()`, `get_source_list()`, `get_content_count()`, `get_content_list()`, `start_content_browsing()`
- Playback: `get_playing_content_info()`, `stop_playing_content()`, `pause_playing_content()`, `set_play_next_content()`, `set_play_previous_content()`, `get_available_playback_function()`, `get_supported_playback_function()`, `preset_broadcast_station()`, `seek_broadcast_station()`, `scan_playing_content()`
- Settings: `get_speaker_settings()`, `get_bluetooth_settings()`, `set_bluetooth_settings()`, `get_playback_mode_settings()`
- System: `get_system_information()`, `get_sw_update_info()`, `act_sw_update()`, `get_alexa_registration_status()`, `get_ecia_device_info()`, `get_wu_tang_info()`
- Debug: `get_supported_methods()`, `probe_endpoints()`
- Terminal: `set_active_terminal()`

### Arcam Module (`arcam.rs`)

Binary protocol over TCP port 50000 (configurable).

- **`Arcam`** struct — Connects to amplifier on each operation (no persistent connection)
- Operations: `power_on()`, `power_off()`, `request_power_state()`, `mute_on()`, `mute_off()`, `get_mute_status()`, `get_amplifier_mode()`
- Extensive `From` implementations: `Host`, `Ipv4Addr`, `Ipv6Addr`, `String`, `&str`, `IpAddr`
- **`ArcamError`** — Io, InvalidResponse, InvalidMuteStatusResponse, InvalidAmplifierModeResponse

### Network Module (`network.rs`)

Simple `Host` enum with three variants: `V4(Ipv4Addr)`, `V6(Ipv6Addr)`, `Dns(String)`. Implements `Display` with proper IPv6 bracket handling.

## CLI (`homelab/cli`)

Binary: **`homey`**

- Uses `clap` derive API with nested subcommands
- Uses `biscuit-terminal` for rich rendering (Table, Prose, UnorderedList)
- Dynamic shell completions via `clap_complete` (bash, zsh, fish)
- Global `--json` flag for machine-readable output
- Per-command `--host` flag or environment variables (`SONY_RECEIVER`, `ARCAM_AMP`)
- Per-command `--port` flag on `sony` subcommand (default: 10000)

### Command Hierarchy

```
Commands (top-level)
├── Arcam { host, action: ArcamAction }
│   ├── On / Off / PowerStatus / MuteStatus / MuteToggle
├── Sony { host, port, action: SonyAction }
│   ├── System(SonySystemAction)
│   │   ├── PowerStatus / On / Off / Info
│   │   ├── UpdateCheck / UpdateApply
│   │   └── AlexaStatus / EciaInfo / WuTangInfo { target? }
│   ├── Audio(SonyAudioAction)
│   │   ├── Volume / SetVolume { level } / MuteStatus / Mute / Unmute
│   │   └── SpeakerSettings { target: SpeakerTarget }
│   ├── Input(SonyInputAction)
│   │   ├── List / Current / Set { uri } / Schemes / Sources { scheme }
│   │   ├── ContentCount { source } / ContentList { source, --start, --count } / Browse { source }
│   │   ├── SetTerminal { uri }
│   │   └── Bluetooth { target? } / SetBluetooth { target, value } / PlaybackMode { target? }
│   ├── Playback(SonyPlaybackAction)
│   │   ├── NowPlaying / Stop / Pause / Next / Previous
│   │   ├── Functions / SupportedFunctions / Preset { uri }
│   │   └── Seek { direction } / Scan { direction }
│   └── Debug(SonyDebugAction)
│       ├── Methods { endpoint: SonyEndpoint }
│       └── Probe
└── Completions
```

### ValueEnum Types

- `SpeakerTarget` — all, level, distance, size, pattern
- `BluetoothTarget` — all, bt-standby, aac
- `PlaybackModeTarget` — all, shuffle, repeat
- `Direction` — forward (alias: fwd), backward (alias: bwd)
- `SonyEndpoint` — system, audio, av-content (alias: av), app-control (alias: app), guide, access-control (alias: access), encryption, browser

## Server (`homelab/server`)

Binary: **`homelab-server`**

### Architecture

- **Framework**: Axum 0.8 with Tower middleware
- **Config**: `~/homey.json` for multi-device support; legacy ENV vars auto-migrate on first run
- **State**: `AppState` with `Arc<RwLock<HashMap>>` for Sony receivers and Arcam hosts; also retains legacy `Option<Arc<SonyReceiver>>` for deprecated routes
- **Middleware**: TraceLayer + permissive CorsLayer
- **Shutdown**: Graceful with 10s timeout, handles Ctrl+C and SIGTERM
- **API Explorer**: Scalar UI at `/explore` (via `utoipa` + `utoipa_scalar`)

### Config File (`~/homey.json`)

```json
{
  "sony_receivers": {
    "living-room": { "host": "192.168.1.100", "port": 10000 }
  },
  "arcam_amps": {
    "office": { "host": "192.168.1.102", "port": 50000 }
  }
}
```

- Created automatically on first run
- ENV vars (`SONY_RECEIVER`, `ARCAM_AMP`) auto-migrated to config with generated petnames
- CRUD operations auto-save to disk
- Device names: lowercase alphanumeric, underscores, hyphens only

### Route Structure

```
Router::new()
    // Named device routes (primary)
    .nest("/sony_receiver", sony_receiver_crud_routes().merge(sony_receiver_control_routes()))
    .nest("/arcam_amp", arcam_amp_crud_routes().merge(arcam_amp_control_routes()))
    // Legacy routes (deprecated, ENV-based)
    .nest("/sony", legacy_sony_routes())
    .nest("/arcam", legacy_arcam_routes())
    // Utility
    .route("/", get(index))
    .route("/health", get(health))
    .route("/health/devices", get(health_devices))
    .route("/explore", Scalar::new(...))
```

### CRUD Operations

Both Sony receivers and Arcam amps support full device management:

| Operation | Method | Path | Notes |
|-----------|--------|------|-------|
| List | GET | `/sony_receiver` | All configured devices |
| Create | POST | `/sony_receiver` | `{ name, host, port? }` |
| Update | PUT | `/sony_receiver/{name}` | `{ host, port? }` |
| Rename | PATCH | `/sony_receiver/{name}` | Plain text body with new name |
| Delete | DELETE | `/sony_receiver/{name}` | Returns 204 |

### Error Handling

`ServerError` maps to HTTP status codes:

| Variant | HTTP | Code |
|---------|------|------|
| `DeviceNotConfigured` | 404 | `DEVICE_NOT_CONFIGURED` |
| `DeviceNotFound` | 404 | `DEVICE_NOT_FOUND` |
| `DeviceExists` | 409 | `DEVICE_EXISTS` |
| `InvalidDeviceName` | 400 | `INVALID_DEVICE_NAME` |
| `InvalidHost` | 400 | `INVALID_HOST` |
| `Sony(SonyError)` | 502 | `SONY_ERROR` |
| `Arcam(ArcamError)` | 502 | `ARCAM_ERROR` |
| `InvalidVolume` | 400 | `INVALID_VOLUME` |
| `Timeout` | 504 | `TIMEOUT` |
| `ConfigIo` | 500 | `CONFIG_IO_ERROR` |
| `ConfigParse` | 500 | `CONFIG_PARSE_ERROR` |

### Host Parsing

Sony host strings support multiple formats:
- IPv4: `192.168.1.100`
- IPv4 with port: `192.168.1.100:8080`
- IPv6: `[::1]`
- DNS: `receiver.local`
- Default ports: Sony 10000, Arcam 50000

## Testing

| Package | Tests | Type | Framework |
|---------|-------|------|-----------|
| `homelab` (lib) | 4 unit | Action serialization | `#[cfg(test)]` |
| `homelab-cli` (cli) | 15 integration | CLI structure, help, completions | `assert_cmd` + `predicates` |
| `homelab-server` | 39 integration | HTTP status codes, error responses | `axum-test` |

No tests communicate with actual devices. Server tests use `empty_state()` to test error paths.

## Known Issues

- **Code bug**: `homey sony audio volume` calls `get_mute_status()` instead of `get_volume()` (`cli/src/main.rs`). The command should call `get_volume()` per the clap doc comment.

## Edition and Version Notes

- All packages use `edition = "2024"`
- `reqwest` 0.13.2 (homelab uses newer than the rest of the monorepo which uses 0.12)
- `tokio`: lib uses 1.49.0, cli/server use 1.48.0
