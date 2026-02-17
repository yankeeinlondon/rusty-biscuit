# Sony STR-AZ7000ES AV Receiver

## Network APIs

The STR-AZ7000ES (and other Sony ES receivers) expose **three** distinct network APIs:

| API | Port | Protocol | Use For |
|-----|------|----------|---------|
| JSON-RPC | 10000 | HTTP POST (JSON-RPC) | Volume, mute, input switching, sound settings, system info |
| Native Web API | 80 | HTTP POST (custom JSON) | **Power status** (reliable), input configuration, deep settings |
| SDCP / CIS | 33335 | Raw TCP (binary) | Professional control systems (Control4, Crestron) |

> **Important**: The JSON-RPC and Native Web APIs serve different purposes and have
> different capabilities. Our `homelab` library uses **both** — JSON-RPC for most
> commands and the Native Web API for power status (see [Quick Start Caveat](#quick-start--network-standby-caveat)).

## Receiver Prep

Before sending any commands, enable network control on the receiver:

1. **Network Standby**: Setup > Network Settings > Network Standby > **ON**
   - Keeps the network stack active in standby so the APIs respond even when the unit is "off"
2. **External Control**: Setup > Network Settings > External Control > **ON**
3. **Static IP**: Assign a static IP via your router or the receiver's network menu

### Connection Behavior

The Sony receiver **only accepts one concurrent TCP connection** per API port. If a
second client connects while one is already connected, the first is dropped. When
writing client code, disable HTTP connection pooling so each request opens and closes
its own connection:

```rust
let client = Client::builder()
    .pool_max_idle_per_host(0)
    .build()?;
```

## Quick Start / Network Standby Caveat

**This is the single most important thing to know about the Sony ES API.**

When **Quick Start** (also called **Network Standby**) is enabled — which is the
default and recommended setting — the JSON-RPC `getPowerStatus` method is **broken**.
It always returns `"active"` regardless of whether the receiver is truly on or in
standby. This is because the entire network stack remains powered up in standby mode,
and the JSON-RPC API cannot distinguish between "network active for standby
responsiveness" and "receiver fully powered on."

We verified this exhaustively. When the receiver is physically in standby with Quick
Start enabled:

- `getPowerStatus` → `{"status": "active"}` (WRONG)
- `getVolumeInformation` → returns volume data normally (works in standby)
- `getSoundSettings` → returns sound field data normally (works in standby)
- `getCurrentExternalTerminalsStatus` → returns input list normally (works in standby)
- `getPlayingContentInfo` → returns current input normally (works in standby)
- `getSystemInformation` → returns model/firmware normally (works in standby)

**None** of the JSON-RPC methods can distinguish active from standby.

### The Fix: Native Web API

The Native Web API on port 80 correctly reports power state regardless of Quick Start:

```bash
# Receiver in standby → correctly returns "off"
curl -s http://192.168.20.120:80/fcgi-bin/request.fcgi \
  -d '{"type":"http_get","packet":[["main.power"]]}'
# → {"type":"http_get_result","packet":[[{"feature":"main.power","id":0,"value":"off"}]]}

# Receiver powered on → correctly returns "on"
# → {"type":"http_get_result","packet":[[{"feature":"main.power","id":0,"value":"on"}]]}
```

Our `get_power_status()` method uses this API, not the broken JSON-RPC one.

### Power Off: Use "standby", Not "off"

The JSON-RPC `setPowerStatus` command also has a Quick Start caveat:

| Value | Quick Start OFF | Quick Start ON |
|-------|----------------|----------------|
| `"active"` | Powers on | Powers on |
| `"standby"` | Powers off | Powers off |
| `"off"` | Powers off | **Error 40001** |

Always use `"standby"` to power off the receiver. The value `"off"` is rejected with
error code 40001 when Quick Start is enabled.


## JSON-RPC API (Port 10000)

HTTP POST to `http://<IP>:10000/sony/<endpoint>` with JSON-RPC body.

### Request Format

```json
{
    "method": "methodName",
    "id": 1,
    "params": [],
    "version": "1.1"
}
```

### Response Format

Success:
```json
{
    "id": 1,
    "result": [{ ... }]
}
```

Error:
```json
{
    "id": 1,
    "error": [40001, "Illegal State"]
}
```

### Response Nesting Patterns

Sony responses use three nesting patterns for the `result` field:

- `result: [[{...}, ...]]` — double-nested (e.g. `getPlayingContentInfo` with zones)
- `result: [{...}]` — single-nested (e.g. `getSystemInformation`)
- `result: [["name", [...], ...], ...]` — flat array of tuples (e.g. `getMethodTypes`)

The `getMethodTypes` method returns `"results"` (plural) instead of `"result"`.

### Power Control

#### Turn Power On

```json
{
    "method": "setPowerStatus",
    "id": 1,
    "params": [{"status": "active"}],
    "version": "1.1"
}
```

#### Turn Power Off (standby)

```json
{
    "method": "setPowerStatus",
    "id": 1,
    "params": [{"status": "standby"}],
    "version": "1.1"
}
```

> Do NOT use `"off"` — see [Quick Start Caveat](#power-off-use-standby-not-off).

### Changing Inputs

Endpoint: `/sony/avContent`

List available inputs:
```json
{
    "method": "getCurrentExternalTerminalsStatus",
    "id": 1,
    "params": [],
    "version": "1.0"
}
```

Switch to a specific input:
```json
{
    "method": "setPlayContent",
    "id": 1,
    "params": [{"uri": "extInput:hdmi?port=1"}],
    "version": "1.2"
}
```

### Endpoint Reference

#### System Endpoint (`/sony/system`)

```csv
Method Name,Ver,Required Parameters,Description
getPowerStatus,1.1,[] (None),"BROKEN with Quick Start — always returns ""active"". Use Native API instead."
setPowerStatus,1.1,"{""status"": ""active""} or ""standby""","Powers on/off. Do NOT use ""off"" with Quick Start."
getSystemInformation,1.4,[] (None),"Returns Model, Serial, Mac, Firmware Ver."
getInterfaceInformation,1.0,[] (None),"Returns Product Category, Model Name."
getPowerSettings,1.0,"{""target"": ""quickStart""}",Gets specific power settings.
setPowerSettings,1.0,"{""settings"": [...]}",Sets power options.
getDeviceMiscSettings,1.0,"{""target"": ""...""}",Gets miscellaneous settings.
setDeviceMiscSettings,1.0,"{""settings"": [...]}",Sets miscellaneous settings.
getSettingsTree,1.1,"{""usage"": ""...""}",Returns menu structure.
getSWUpdateInfo,1.0,"{""network"": ""network""}",Checks for firmware updates.
actSWUpdate,1.0,[] (None),Triggers firmware update (reboots receiver).
connectBluetoothDevice,1.0,"{""bdAddr"": ""XX:XX:...""}",Connects to specific BT device.
getStorageList,1.2,"{""uri"": ""storage:usb1""}",Lists files on USB drive.
getWuTangInfo,1.0,"{""target"": ""...""}",Internal provisioning info.
getEciaDeviceInfo,1.0,[] (None),Internal device ID.
getAlexaRegistrationStatus,1.0,[] (None),Checks Alexa enrollment.
```

#### Audio Endpoint (`/sony/audio`)

```csv
Method Name,Ver,Required Parameters,Description
setAudioVolume,1.1,"{""target"": ""speaker"", ""volume"": ""25""}",Sets volume (0-100).
getVolumeInformation,1.1,"{""output"": """"}","Gets Vol, Mute, Min/Max."
setAudioMute,1.1,"{""status"": ""on""} (or ""off"")",Mutes/Unmutes.
getSoundSettings,1.1,"{""target"": ""soundField""}",Gets current Sound Field (Dolby/DTS).
setSoundSettings,1.1,"{""settings"": [...]}",Sets Sound Field.
getSpeakerSettings,1.0,"{""target"": ""level""}",Gets Speaker Level/Distance/Size.
setSpeakerSettings,1.0,"{""settings"": [...]}",Sets Speaker config.
getCustomEqualizerSettings,1.0,"{""target"": ""...""}",Gets EQ settings.
setCustomEqualizerSettings,1.0,"{""settings"": [...]}",Sets EQ settings.
```

#### AV Content Endpoint (`/sony/avContent`)

```csv
Method Name,Ver,Required Parameters,Description
getCurrentExternalTerminalsStatus,1.0,[] (None),"Lists all Inputs (HDMI, Optical, etc)."
getPlayingContentInfo,1.2,"{""output"": """"}",Gets Current Input metadata.
setPlayContent,1.2,"{""uri"": ""extInput:hdmi?port=1""}",Switches Input.
getSchemeList,1.0,[] (None),"Lists valid URIs (extInput, storage)."
getSourceList,1.2,"{""scheme"": ""extInput""}",Lists sources for a scheme.
getContentList,1.4,"{""uri"": ""..."", ""stIdx"": 0, ""cnt"": 50...}",Lists content (USB/DLNA).
getContentCount,1.3,"{""uri"": ""..."", ""type"": ""...""}",Counts items in a folder.
getPlaybackModeSettings,1.0,"{""target"": ""...""}",Gets Shuffle/Repeat status.
setPlaybackModeSettings,1.1,"{""settings"": [...]}",Sets Shuffle/Repeat.
stopPlayingContent,1.1,"{""output"": """"}",Stops playback.
pausePlayingContent,1.1,"{""output"": """"}",Pauses playback.
setPlayNextContent,1.0,"{""output"": """"}",Skips to next track.
setPlayPreviousContent,1.0,"{""output"": """"}",Skips to previous track.
getAvailablePlaybackFunction,1.0,"{""output"": """"}",Lists valid actions (Play/Stop/Pause).
```

#### App Control Endpoint (`/sony/appControl`)

```csv
Method Name,Ver,Required Parameters,Description
getApplicationList,1.2,[] (None),"Lists built-in apps (Spotify, etc)."
```

> **Zone Parameter**: Whenever you see `{"output": ""}` in the AvContent or Audio
> sections, it means the command targets a zone. Main Zone: `""` (empty string) or
> `"extOutput:zone?zone=1"`. Zone 2: `"extOutput:zone?zone=2"`. Zone 3:
> `"extOutput:zone?zone=3"`.


## Native Web API (Port 80)

The receiver hosts a web configuration interface on port 80. Behind this interface is
a JSON-based API at `/fcgi-bin/request.fcgi` that provides access to settings the
JSON-RPC API doesn't properly expose — most critically, **accurate power status**.

### GET Request

Query one or more features:

```bash
curl -s http://<IP>:80/fcgi-bin/request.fcgi \
  -d '{"type":"http_get","packet":[["feature1","feature2"]]}'
```

Response:
```json
{
  "type": "http_get_result",
  "packet": [[
    {"feature": "feature1", "id": 0, "value": "value1"},
    {"feature": "feature2", "id": 1, "value": "value2"}
  ]]
}
```

### SET Request

Set one or more features:

```bash
curl -s http://<IP>:80/fcgi-bin/request.fcgi \
  -d '{"type":"http_set","packet":[{"id":0,"feature":"feature1","value":"newvalue"}]}'
```

### Power and Zone Features

| Feature | Example Values | Description |
|---------|---------------|-------------|
| `main.power` | `"on"` / `"off"` | Main zone power (reliable, unlike JSON-RPC) |
| `main.input` | Current input URI | Currently active input |
| `main.volume` | `"50"` | Current volume level |
| `main.mute` | `"on"` / `"off"` | Mute state |
| `zone2.power` | `"on"` / `"off"` | Zone 2 power |
| `zone2.volume` | `"30"` | Zone 2 volume |
| `zone2.input` | `"SAT"` | Zone 2 input selection |
| `zone3.power` | `"on"` / `"off"` | Zone 3 power |
| `zone3.volume` | `"25"` | Zone 3 volume |

### Input Configuration Features

The native API exposes all 8 input slots with detailed configuration. Each input has a
category name and a set of sub-features.

#### Input Categories

| Category | Default Name | Description |
|----------|-------------|-------------|
| `GAME` | GAME | Gaming input |
| `STB` | MEDIA BOX | Set-top box / streaming device |
| `BD` | BD/DVD | Blu-ray / DVD player |
| `SAT` | SAT/CATV | Satellite / cable TV |
| `VIDEO` | VIDEO | Generic video input |
| `AUX` | AUX | Auxiliary input |
| `TV` | TV | Television (ARC/eARC) |
| `CD` | SA-CD/CD | Audio disc player |

#### Per-Input Features

Each category exposes these features (replace `GAME` with the category name):

| Feature | Example Values | Description |
|---------|---------------|-------------|
| `GAME.icon` | `"game"` | Icon identifier |
| `GAME.inputname` | `"GAME"` | User-facing display name |
| `GAME.hdmiassign` | `"HDMI 3"` | Which physical HDMI port is assigned |
| `GAME.show` | `"true"` / `"false"` | Whether input is visible in the UI |
| `GAME.category` | `"game"` | Internal category tag |
| `GAME.videoin` | `""` | Video input override |
| `GAME.digitalassign` | `"opt"` / `"coax"` / `""` | Digital audio input assignment (Opt/Coax) |
| `GAME.inceilingmode` | `"true"` / `"false"` | In-ceiling speaker mode |
| `GAME.inputmode` | `"auto"`, `"4ch"`, `"analog"` | Input mode setting |
| `GAME.soundfield` | `"A.F.D."` | Sound field preset for this input |
| `GAME.swlevel` | `"0"` | Subwoofer level offset (-10 to +10) |
| `GAME.swlpf` | `"off"`, `"80Hz"`, `"120Hz"` | Subwoofer low-pass filter |
| `GAME.usetrigger1` | `"true"` / `"false"` | 12V trigger 1 activation |
| `GAME.usetrigger2` | `"true"` / `"false"` | 12V trigger 2 activation |
| `GAME.usetrigger3` | `"true"` / `"false"` | 12V trigger 3 activation |
| `GAME.presetgain` | `"0"` | Input gain preset (-12dB to +12dB) |
| `GAME.avsync` | `"0"` | AV sync delay (0-300ms) |

#### Additional System Features

The native API also exposes system-wide settings not available via JSON-RPC:

| Feature | Example Values | Description |
|---------|---------------|-------------|
| `main.power` | `"on"` / `"off"` | Main zone power (reliable) |
| `main.input` | Current input URI | Currently active input |
| `main.volume` | `"50"` | Current volume level |
| `main.mute` | `"on"` / `"off"` | Mute state |
| `zone2.power` | `"on"` / `"off"` | Zone 2 power |
| `zone2.volume` | `"30"` | Zone 2 volume |
| `zone2.input` | `"SAT"` | Zone 2 input |
| `zone3.power` | `"on"` / `"off"` | Zone 3 power |
| `zone3.volume` | `"25"` | Zone 3 volume |
| `system.volumedisplay` | `"dB"`, `"linear"` | Volume display units |
| `system.dimmer` | `"off"`, `"dark"`, `"bright"` | Display dimmer |
| `system.devicename` | `"Living Room"` | Device name setting |
| `system.internetstatus` | `"connected"` / `"disconnected"` | Internet connectivity |
| `system.wiredlan` | `"connected"` / `"disconnected"` | Wired network status |
| `system.wirelesslan` | `"connected"` / `"disconnected"` | Wireless network status |

#### Audio Settings (Native API)

| Feature | Example Values | Description |
|---------|---------------|-------------|
| `audio.puredirect` | `"on"` / `"off"` | Pure Direct mode |
| `audio.soundfield` | `"A.F.D."` | Current sound field |
| `audio.frontbalance` | `"0"` | Front speaker balance |
| `audio.centerlevel` | `"0"` | Center speaker level |
| `audio.subwooferlevel` | `"0"` | Subwoofer level |
| `audio.dolbylevel` | `"0"` | Dolby volume level |
| `audio.surroundlevel` | `"0"` | Surround speaker level |

#### Querying All Inputs at Once

```bash
curl -s http://192.168.20.120:80/fcgi-bin/request.fcgi \
  -d '{"type":"http_get","packet":[["GAME.inputname","GAME.hdmiassign","STB.inputname","STB.hdmiassign","BD.inputname","BD.hdmiassign","SAT.inputname","SAT.hdmiassign","VIDEO.inputname","VIDEO.hdmiassign","AUX.inputname","AUX.hdmiassign","TV.inputname","TV.hdmiassign","CD.inputname","CD.hdmiassign"]]}'
```

### Model Type

The STR-AZ7000ES identifies as model type `"Z52"` in the native API's internal model
type system.


## SDCP / CIS (Binary Protocol)

For professional control systems requiring raw TCP communication on port 33335.

- **Port**: 33335
- **Protocol**: Raw TCP socket with hex-encoded binary messages
- **Header**: Commands start with `0x02` (STX) followed by message length
- **Delimiter**: Commands terminated with `0x03` (ETX)

### Message Format

```
[STX] [Length-H] [Length-L] [Data...] [ETX] [Checksum]
```

| Byte | Description |
|------|-------------|
| 0x02 | Start of text (STX) |
| Length | 16-bit big-endian message length |
| Data | Command payload (variable) |
| 0x03 | End of text (ETX) |
| Checksum | XOR of all bytes from STX to ETX (inclusive) |

### Known Command Categories

Based on protocol analysis of similar Sony ES receivers:

| Category | Purpose |
|----------|---------|
| Power | Power on/off, standby modes |
| Volume | Master volume, zone volumes |
| Input | Input selection, routing |
| Audio | Sound field, EQ, tone controls |
| Video | Video pass-through, picture modes |
| System | Device info, network settings |

### Command Structure Example

```
02 00 10 01 03 00 01 00 00 03 14
│  │  │  │  │  │  │  │  │  │
│  │  │  │  │  │  │  │  │  └── Checksum
│  │  │  │  │  │  │  │  └───── ETX
│  │  │  │  │  │  │  └──────── Command data
│  │  │  │  │  │  └────────── Sub-command
│  │  │  │  │  └───────────── Command
│  │  │  │  └──────────────── Source/Zone
│  │  │  └────────────────── Message type
│  │  └───────────────────── Length (16 bytes)
│  └──────────────────────── STX
```

### Capabilities vs JSON-RPC

The SDCP protocol provides:
- **Faster response**: Binary protocol with lower overhead than HTTP
- **Zone control**: Native multi-zone support (Main, Zone 2, Zone 3)
- **Real-time feedback**: Push notifications for volume/input changes
- **Professional integration**: Control4, Crestron, Savant compatibility

**Limitations**:
- Harder to debug (binary vs JSON)
- Requires specialized libraries
- Less documented than REST APIs

> **Recommendation**: Unless building a commercial driver for Control4/Crestron, use the JSON-RPC and Native Web APIs instead.


## Implementation Notes

### Which API to Use

| Operation | API | Reason |
|-----------|-----|--------|
| Get power status | **Native Web** (port 80) | JSON-RPC is broken with Quick Start |
| Set power on/off | JSON-RPC (port 10000) | Works with `"active"`/`"standby"` |
| Volume/mute | JSON-RPC (port 10000) | Full control available |
| Input switching | JSON-RPC (port 10000) | `setPlayContent` with URI |
| Input configuration | **Native Web** (port 80) | Names, HDMI assignments, visibility |
| Sound settings | JSON-RPC (port 10000) | Sound fields, EQ, speaker config |
| System info | JSON-RPC (port 10000) | Model, firmware, MAC address |

### Do NOT Retry State-Changing Commands

Commands like `setPowerStatus` must be sent **exactly once**. Retrying can cause the
receiver to toggle state (e.g. off → on → off), leading to unpredictable results.
Read-only queries (volume info, input status, etc.) are safe to retry.
