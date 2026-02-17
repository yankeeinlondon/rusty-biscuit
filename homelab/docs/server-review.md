# Arcam Amplifier Code Review

## Executive Summary

The Arcam amplifier integration has several functional issues and inefficiencies, with the most significant being the **failure to use the System Status (0x5D) command** which would reduce multiple network round-trips to a single call.

---

## Critical Issues

### 1. System Status Command (0x5D) Not Utilized

**Location:** `homelab/server/src/lib.rs:409-477` (probe_arcam function)

**Problem:** The server makes **10 separate TCP requests** to get amplifier status when a single System Status (0x5D) command would provide all information:

| Current Implementation | Protocol Capability |
|-----------------------|-------------------|
| Power state (0x00) | System Status (0x5D) returns ALL of these |
| Mute status (0x0E) | in a single response |
| Amplifier mode (0x61) | plus additional parameters |
| Auto shutdown (0x58) | including: |
| System model (0x5E) | - Software version |
| Timeout counter (0x55) | - Friendly name |
| | - IP address |
| | - Input detect |
| | - DC offset |
| | - Short circuit status |

**Impact:**
- 10x network latency for status queries
- Each request creates a new TCP connection (no pooling)
- Dashboard polling is unnecessarily slow

**Recommendation:** Implement a `get_system_status()` function that sends the 0x5D command and parses the multi-part response. The protocol documentation (arcam.md:307-319) confirms:

```
Cmd: 21 01 5D 01 F0 0D
Resp: 21 01 5D 00 01 F0 0D
      (followed by individual status messages for each parameter)
```

### 2. Amplifier Mode Mapping Bug

**Location:**
- `homelab/server/src/handlers/arcam.rs:199-204` and `homelab/server/src/handlers/arcam.rs:376-381`

**Problem:** The handler maps mode bytes incorrectly:

```rust
let mode = match mode_byte {
    0 => "Stereo",
    1 => "Bridged",
    2 => "DualMono",
    _ => "Unknown",
};
```

**Reality (from arcam.md:443-451):**
- `0x01` = Stereo (physical switch in "ST" position)
- `0x02` = Bridged
- `0x03` = Dual Mono (physical switch in "DM" position)

**Impact:** Mode is always reported incorrectly - "Stereo" shows as "Unknown", etc.

**Recommendation:** Fix the mapping to:
```rust
let mode = match mode_byte {
    1 => "Stereo",
    2 => "Bridged",
    3 => "DualMono",
    _ => "Unknown",
};
```

### 3. Heartbeat Only Targets Legacy Device

**Location:** `homelab/server/src/main.rs:167-212`

**Problem:** The heartbeat keepalive task only sends heartbeats to `state.arcam_host` (the legacy single device):

```rust
if let Some(host) = &state.arcam_host {  // Only legacy device!
    let arcam = Arcam::from(host.as_str());
    // ...
}
```

In the loop (lines 190-206), it does iterate over `state.arcam_hosts`, but only if that HashMap has entries. The startup query (lines 171-185) only queries the legacy device.

**Impact:** Multi-device configurations may not have heartbeat keepalive working properly.

**Recommendation:** Ensure the startup model query also runs for all configured devices in `arcam_hosts`.

---

## Functional Issues

### 4. Missing Temperature Endpoints

**Problem:** The protocol supports temperature queries (0x56 for lifter temperature, 0x57 for output temperature) but these aren't exposed in the REST API.

**Recommendation:** Add endpoints:
- `GET /arcam_amp/{name}/temperature/lifter`
- `GET /arcam_amp/{name}/temperature/output`

### 5. Missing Timeout Counter REST Endpoint

**Location:** `homelab/server/src/lib.rs:521-536`

**Problem:** There's an internal `/arcam/timeout` endpoint that returns raw JSON, but no proper REST endpoint for the timeout counter.

**Recommendation:** Add:
- `GET /arcam_amp/{name}/timeout` - returns `{ "seconds_remaining": number, "formatted": "X hours Y minutes" }`

### 6. No Support for PA410 (No Mode)

**Problem:** The PA410 amplifier doesn't support the amplifier mode command (0x61), but the server assumes all amps support it.

**Location:** `homelab/server/src/handlers/arcam.rs:192-207` and `368-384`

**Recommendation:** Check for `ParameterNotRecognised` error (0x84) and return "N/A" for PA410 units.

---

## UI/UX Issues

### 8. No Friendly Name Support

**Problem:** The protocol supports getting/setting a friendly name (0x53), but this isn't exposed in the server API or dashboard.

**Recommendation:** Add:
- `GET /arcam_amp/{name}/name`
- `PUT /arcam_amp/{name}/name`

---

## Code Quality Issues

### 9. Duplicate Host Parsing Logic

**Locations:**
- `homelab/server/src/handlers/arcam.rs:409-422` (parse_host)
- `homelab/server/src/state.rs:384-401` (parse_host)

**Problem:** Identical host parsing logic is duplicated.

**Recommendation:** Extract to a shared function in the library.

### 10. Connection Not Reused

**Location:** `homelab/lib/src/arcam.rs:68-79` (send_raw)

**Problem:** Each command creates a new TCP connection:

```rust
let addr = format!("{}:50000", ip_addr);
let mut sock = TcpStream::connect(addr).await?;  // New connection every time
sock.write_all(cmd_bytes).await?;
```

**Recommendation:** Consider a connection pool or connection reuse for commands that are sent in quick succession (like in probe_arcam).

---

## Minor Issues

### 11. Inconsistent Error Handling in Dashboard

**Location:** `homelab/server/src/lib.rs:959-966`

The timeout counter polling has its own error handling that logs to console but doesn't update the UI state.

THIS IS INTENDED AS A DEBUGGING MEASURE. WE CURRENTLY DO NOT UNDERSTAND THE RESULTS WE'RE GETTING on the TIMEOUT MEASUREMENTS.

Suggestion: investigate if the server really is returning the number of seconds as the documentation suggests. Identify if our constant polling is interfering with the timing interval.

### 12. No Graceful Handling of Missing Model Query

**Location:** `homelab/server/src/main.rs:171-185`

If the initial model query fails, there's no retry logic, and the model stays unknown until server restart.

NOTES:

- the Amplifier model will change almost never; once someone has setup their system it's like to stay static so whenever we DO get a reading on this it is reasonable to cache it as a fallback from any future failures. That said if there is a failure in getting the model we should have some sort of incremental fallback approach.

---

## Summary

| Priority | Issue | Effort | Status |
|----------|-------|--------|--------|
| Critical | System Status (0x5D) not used | Medium | ✅ Implemented |
| Critical | Amplifier mode mapping bug | Low | ✅ Fixed |
| High | Heartbeat only targets legacy device | Medium | ✅ Fixed |
| Medium | Missing temperature endpoints | Low | ✅ Implemented |
| Medium | Missing timeout endpoint | Low | ✅ Implemented |
| Medium | Missing friendly name endpoints | Low | ✅ Implemented |
| Medium | PA410 mode not supported | Low | ✅ Fixed |
| Low | Duplicate host parsing | Low | ✅ Fixed |
| Low | Connection not reused | Medium | ⚠️ Skipped (complex) |

## New Endpoints Added

- `GET /arcam_amp/{name}/status` - Full system status (single call)
- `GET /arcam_amp/{name}/temperature/{lifter|output}` - Temperature sensors
- `GET /arcam_amp/{name}/timeout` - Timeout counter with formatted string
- `GET /arcam_amp/{name}/name` - Get friendly name
- `PUT /arcam_amp/{name}/name` - Set friendly name

---

## References

- Protocol documentation: `homelab/docs/arcam.md`
- Library implementation: `homelab/lib/src/arcam.rs`
- Server handlers: `homelab/server/src/handlers/arcam.rs`
- Server lib (probe function): `homelab/server/src/lib.rs`
