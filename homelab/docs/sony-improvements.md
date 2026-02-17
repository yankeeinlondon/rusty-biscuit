# Sony STR-AZ7000ES API Improvements

This document outlines recommendations for improving the Sony API exposed by `homelab-server` based on a gap analysis of the three receiver APIs.

---

## Executive Summary

The current implementation exposes approximately **30%** of the available functionality from the Sony receiver's three APIs. Significant gaps exist in:

1. **Native Web API (Port 80)** — Currently only fetches 5 of 18+ per-input features
2. **Audio Settings** — No access to sound fields, Pure Direct, EQ, or speaker levels
3. **System Settings** — No access to volume display, dimmer, device name, network status
4. **Device Details** — MAC addresses, connectivity status not fully exposed

---

## Gap Analysis

### Current Server Exposure

| Endpoint | Methods Exposed |
|----------|----------------|
| `/power` | get, set |
| `/volume` | get, set |
| `/mute` | get, set |
| `/inputs` | list (JSON-RPC) |
| `/inputs/config` | list (Native API - limited) |
| `/input/current` | get |
| `/input` | set |
| `/system/info` | get |

### Missing from Native API (Port 80)

The current `get_native_inputs()` only retrieves:
- `inputname`
- `hdmiassign`
- `show` (visible)
- `icon`
- `soundfield`

**Missing per-input features:**
| Feature | Description | Priority |
|---------|-------------|----------|
| `digitalassign` | Optical/Coax input selection | High |
| `inputmode` | Auto/4ch/Analog input mode | Medium |
| `swlevel` | Subwoofer level offset | Medium |
| `swlpf` | Subwoofer low-pass filter | Medium |
| `inceilingmode` | In-ceiling speaker mode | Low |
| `videoin` | Video input override | Low |
| `usetrigger1/2/3` | 12V trigger control | Medium |
| `presetgain` | Input gain preset | Medium |
| `avsync` | AV sync delay | High |

### Missing System Features

The Native API exposes system settings not available via JSON-RPC:

| Feature | Description | API Source | Priority |
|---------|-------------|------------|----------|
| Volume display units | dB vs linear | Native | Medium |
| Display dimmer | Off/dark/bright | Native | Medium |
| Device name | User-defined name | Native | Medium |
| Wired LAN status | Connected/disconnected | Native | High |
| Wireless LAN status | Connected/disconnected | Native | High |
| Internet connectivity | Online/offline | Native | High |
| Wired MAC address | Physical address | JSON-RPC* |
| Wireless MAC address | Physical address | JSON-RPC* |

*JSON-RPC returns this in `getSystemInformation` but it's not exposed in our API

### Missing Audio Features (JSON-RPC)

| Feature | Method | Priority |
|---------|--------|----------|
| Sound field (global) | `getSoundSettings` | High |
| Set sound field | `setSoundSettings` | High |
| Pure Direct | Implemented via sound settings | High |
| Custom EQ | `getCustomEqualizerSettings` | Medium |
| Speaker levels | `getSpeakerSettings` (level) | Medium |
| Speaker distance | `getSpeakerSettings` (distance) | Low |
| Speaker size | `getSpeakerSettings` (size) | Low |

---

## Recommendations

### Priority 1: Expand Native Input Configuration

**Current state:**
```rust
const FEATURES: &[&str] = &["inputname", "hdmiassign", "show", "icon", "soundfield"];
```

**Recommended:**
```rust
const FEATURES: &[&str] = &[
    "inputname", "hdmiassign", "show", "icon", "soundfield",
    "digitalassign", "inputmode", "swlevel", "swlpf",
    "inceilingmode", "videoin", "usetrigger1", "usetrigger2",
    "usetrigger3", "presetgain", "avsync"
];
```

**New struct fields:**
```rust
pub struct NativeInputConfig {
    pub category: String,
    pub name: String,
    pub hdmi_assign: String,
    pub icon: String,
    pub visible: bool,
    pub sound_field: String,
    // NEW:
    pub digital_assign: String,    // "opt", "coax", or ""
    pub input_mode: String,        // "auto", "4ch", "analog"
    pub subwoofer_level: String,  // "-10" to "+10"
    pub subwoofer_lpf: String,     // "off", "80Hz", etc.
    pub in_ceiling_mode: bool,
    pub trigger_1: bool,
    pub trigger_2: bool,
    pub trigger_3: bool,
    pub preset_gain: String,       // "-12" to "+12"
    pub av_sync: String,          // "0" to "300"
}
```

### Priority 2: Add System Settings Endpoint

Create a new endpoint `/system/settings` that queries:

```
system.volumedisplay, system.dimmer, system.devicename,
system.internetstatus, system.wiredlan, system.wirelesslan
```

**Proposed response:**
```json
{
  "volume_display": "dB",
  "dimmer": "off",
  "device_name": "Living Room",
  "network": {
    "wired": "connected",
    "wireless": "connected",
    "internet": "connected"
  }
}
```

### Priority 3: Expose Audio Settings

Add endpoints for sound field control:

| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/audio/soundfield` | get, set | Current sound field |
| `/audio/pure_direct` | get, set | Pure Direct mode |
| `/audio/equalizer` | get, set | Custom EQ |
| `/audio/speakers` | get | Speaker levels/distances |

### Priority 4: Add Zone 2/3 Support

The Native API supports Zone 2 and Zone 3 control:
- Power state
- Volume
- Input selection

Add endpoints:
| Endpoint | Methods |
|----------|--------|
| `/zones/2/power` | get, set |
| `/zones/2/volume` | get, set |
| `/zones/2/input` | get, set |
| `/zones/3/power` | get, set |
| `/zones/3/volume` | get, set |
| `/zones/3/input` | get, set |

### Priority 5: Add Native Set Operations

Currently only reading is implemented via Native API. Add setters:

```rust
pub async fn native_set(&self, feature: &str, value: &str) -> Result<(), SonyError>
```

Required for:
- Setting input names
- Configuring HDMI assignments
- Setting trigger states
- Changing display settings

### Priority 6: Expose Device Details

Ensure full system info is exposed:

```rust
pub struct SystemInformation {
    pub model: String,
    pub serial: Option<String>,
    pub mac_addr: String,           // Wired - ALREADY IN STRUCT
    pub wireless_mac_addr: Option<String>,  // WIRELESS - NEED TO EXPOSE
    pub bd_addr: Option<String>,
    pub name: Option<String>,
    pub generation: Option<String>,
    pub version: String,
    pub region: Option<String>,
    pub product: Option<String>,
}
```

The `wireless_mac_addr` field exists in the struct but is not exposed via the server API.

---

## Implementation Roadmap

### Phase 1: Data Model Updates
1. Expand `NativeInputConfig` struct with all 18 features
2. Add `NativeSystemSettings` struct
3. Add `ZoneStatus` struct for multi-zone support
4. Add audio settings structs

### Phase 2: Library Updates
1. Update `get_native_inputs()` to fetch all features
2. Add `get_system_settings()` method
3. Add `set_native_input()` method
4. Add zone-specific methods

### Phase 3: Server Endpoints
1. Add `/inputs/config` expansion (automatic with lib update)
2. Add `/system/settings` endpoint
3. Add `/audio/soundfield` get/set
4. Add `/zones/:id/*` endpoints

---

## Appendix: API Feature Matrix

| Category | JSON-RPC | Native API | Currently Exposed |
|----------|----------|------------|-------------------|
| Power (main) | ✅ | ✅ | ✅ (Native) |
| Power (zones) | ❌ | ✅ | ❌ |
| Volume | ✅ | ✅ | ✅ |
| Mute | ✅ | ✅ | ✅ |
| Inputs (list) | ✅ | ✅ | ✅ |
| Input config | Partial | Full | Partial |
| Sound field | ✅ | ✅ | Partial |
| Pure Direct | ✅ | ❌ | ❌ |
| EQ | ✅ | ❌ | ❌ |
| Speaker settings | ✅ | ❌ | ❌ |
| System info | ✅ | ✅ | Partial |
| Display settings | ❌ | ✅ | ❌ |
| Network status | ❌ | ✅ | ❌ |
| Triggers | ❌ | ✅ | ❌ |

---

## Notes

- The JSON-RPC `getSystemInformation` already returns both `macAddr` and `wirelessMacAddr` — the struct handles this but the server doesn't serialize `wireless_mac_addr` in responses (the field exists but needs verification)
- For SDCP (port 33335), no implementation is recommended unless building commercial integrations
- All Native API writes should be idempotent and handle errors gracefully
