# Arcam PA Series IP/Serial Protocol

Protocol reference for the **Arcam PA720, PA240, and PA410** power amplifiers.

Source: [Custom Installation Notes SH305E Issue 3](arcam-pa-series-ip-serial-protocol.pdf)

## Connection

### IP (TCP)

- **Port**: 50000
- **Protocol**: TCP to the unit's IP address

### RS232 (Serial)

- **Cable**: Null modem (DB9F ↔ DB9F)
- **Baud**: 38,400 bps
- **Format**: 8N1 (8 data bits, no parity, 1 stop bit, no flow control)
- **Pinout**: Pin 2 ↔ Pin 3 (Rx/Tx crossover), Pin 5 ↔ Pin 5 (ground)

### Timing

- The amp responds to each command within **3 seconds**.
- The controller may send further commands before a previous response arrives.

## Frame Format

### Command (Controller → Amp)

```
St  Zn  Cc  Dl  Data...  Et
```

| Byte | Name | Description |
|------|------|-------------|
| St | Start | `0x21` (ASCII `!`) |
| Zn | Zone | `0x01` = Zone 1, `0x02` = Zone 2 |
| Cc | Command code | See command table below |
| Dl | Data length | Number of data bytes (excludes Et) |
| Data | Payload | Command-specific parameters |
| Et | End | `0x0D` (carriage return) |

### Response (Amp → Controller)

```
St  Zn  Cc  Ac  Dl  Data...  Et
```

Same as command, but with an **Answer Code (Ac)** inserted between Cc and Dl.

### Answer Codes

| Ac | Meaning |
|----|---------|
| `0x00` | Status update (success) |
| `0x82` | Zone invalid |
| `0x83` | Command not recognised |
| `0x84` | Parameter not recognised |
| `0x86` | Invalid data length |

### Unsolicited Messages

The amp sends status updates when state changes from front panel switches or external events (e.g. faults). These use the standard response frame format.

## Command Reference

### Power — `0x00`

Set or query the standby state.

| Data (command) | Meaning |
|----------------|---------|
| `0x00` | Power off (standby) |
| `0x01` | Power on |
| `0xF0` | Query state |

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | Standby |
| `0x01` | Powered on |

Example — query power state of zone 1 (result: on):

```
Cmd:  21 01 00 01 F0 0D
Resp: 21 01 00 00 01 01 0D
```

### Software Version — `0x04`

Query firmware version. Response has 2 data bytes: major, minor.

| Data (command) | Meaning |
|----------------|---------|
| `0xF0` | Query |

Example — version 1.2:

```
Cmd:  21 01 04 01 F0 0D
Resp: 21 01 04 00 02 01 02 0D
```

### Factory Reset — `0x05`

Reset to factory defaults. Requires confirmation pattern `0xAA 0xAA`.

```
Cmd:  21 01 05 02 AA AA 0D
Resp: 21 01 05 00 00 0D
```

### Mute — `0x0E`

Set or query mute status.

| Data (command) | Meaning |
|----------------|---------|
| `0x00` | Mute |
| `0x01` | Unmute |
| `0xF0` | Query |

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | Muted |
| `0x01` | Unmuted |

Example — query mute (result: unmuted):

```
Cmd:  21 01 0E 01 F0 0D
Resp: 21 01 0E 00 01 01 0D
```

### Heartbeat — `0x25`

Check connectivity. Also resets the EuP (Energy-using Products) standby timer.

| Data (command) | Meaning |
|----------------|---------|
| `0xF0` | Heartbeat ping |

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | Alive |

```
Cmd:  21 01 25 01 F0 0D
Resp: 21 01 25 00 01 00 0D
```

### Reboot — `0x26`

Force reboot. Requires ASCII `"REBOOT"` (`0x52 0x45 0x42 0x4F 0x4F 0x54`) as confirmation.

```
Cmd:  21 01 26 06 52 45 42 4F 4F 54 0D
Resp: 21 01 26 01 00 0D
```

### DC Offset — `0x51`

Query output DC offset status.

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | OK (no DC offset) |
| `0x01` | DC offset detected |

```
Cmd:  21 01 51 01 F0 0D
Resp: 21 01 51 00 01 00 0D
```

### Short Circuit Status — `0x52`

*PA720 and PA240 only.*

Query output short circuit status.

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | No short circuit |
| `0x01` | Short circuit detected |

```
Cmd:  21 01 52 01 F0 0D
Resp: 21 01 52 00 01 00 0D
```

### Friendly Name — `0x53`

*PA720 and PA240 only.*

Get or set the unit's friendly name. Uppercase A–Z, digits 0–9, and space only. Max 10 characters.

**Query:**

```
Cmd:  21 01 53 01 F0 0D
Resp: 21 01 53 00 05 41 4D 50 20 31 0D   (="AMP 1")
```

**Set** (send ASCII bytes directly as data):

```
Cmd:  21 01 53 05 41 4D 50 20 31 0D      (="AMP 1")
Resp: 21 01 53 00 05 41 4D 50 20 31 0D
```

### IP Address — `0x54`

*PA720 and PA240 only.*

Get or set the IP address. 4 data bytes = the 4 octets.

**Query:**

```
Cmd:  21 01 54 01 F0 0D
Resp: 21 01 54 00 04 C0 A8 01 04 0D      (=192.168.1.4)
```

**Set** (e.g. 192.168.1.4 = `0xC0 0xA8 0x01 0x04`):

```
Cmd:  21 01 54 04 C0 A8 01 04 0D
Resp: 21 01 54 00 04 C0 A8 01 04 0D
```

### Timeout Counter — `0x55`

Query seconds remaining until auto-standby. 2-byte big-endian response. Range: 0–14400 (`0x0000`–`0x3840`).

```
Cmd:  21 01 55 01 F0 0D
Resp: 21 01 55 00 02 38 40 0D            (=14400 seconds)
```

### Lifter Temperature — `0x56`

*PA720 and PA240 only.*

Query lifter circuitry temperature in degrees Celsius.

| Data (command) | Meaning |
|----------------|---------|
| `0xF0` | Sensor 1 |
| `0xF1` | Sensor 2 |

Response: 2 bytes — sensor ID + temperature.

```
Cmd:  21 01 56 01 F0 0D
Resp: 21 01 56 00 02 F0 4B 0D            (sensor 1 = 75°C)
```

### Output Temperature — `0x57`

Query output stage temperature in degrees Celsius.

| Data (command) | Meaning |
|----------------|---------|
| `0xF0` | Sensor 1 |
| `0xF1` | Sensor 2 |

Response: 2 bytes — sensor ID + temperature.

```
Cmd:  21 01 57 01 F0 0D
Resp: 21 01 57 00 02 F0 4B 0D            (sensor 1 = 75°C)
```

### Auto Shutdown Control — `0x58`

Set or query the signal-sense auto shutdown timer.

| Data | Meaning |
|------|---------|
| `0x00` | Disabled |
| `0x01` | 20 min (default) |
| `0x02` | 30 min |
| `0x03` | 1 hour |
| `0x04` | 2 hours |
| `0x05` | 4 hours |
| `0xF0` | Query |

```
Cmd:  21 01 58 01 F0 0D
Resp: 21 01 58 00 01 01 0D               (=20 min)
```

### Input Detect — `0x5A`

Query whether audio signal is present on the active input.

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | No input signal |
| `0x01` | Input signal present |

```
Cmd:  21 01 5A 01 F0 0D
Resp: 21 01 5A 00 01 01 0D               (signal present)
```

### System Status — `0x5D`

Triggers the amp to send individual status responses for **all** parameters:

- Power, Software Version, Mute, Friendly Name*, IP Address*, Timeout Counter, Lifter Temperature*, Output Temperature, Auto Shutdown, Input Detect, System Model, Amplifier Mode**

*PA720/PA240 only. **PA240 only.

```
Cmd:  21 01 5D 01 F0 0D
Resp: 21 01 5D 00 01 F0 0D
      (followed by individual status messages for each parameter)
```

### System Model — `0x5E`

Query model name. Returned as ASCII characters (max 10).

```
Cmd:  21 01 5E 01 F0 0D
Resp: 21 01 5E 00 05 50 41 37 32 30 0D   (="PA720")
```

### Amplifier Mode — `0x61`

*PA240 only.*

Query the amplifier operating mode.

| Data (response) | Meaning |
|-----------------|---------|
| `0x00` | Stereo |
| `0x01` | Bridged |
| `0x02` | Dual Mono |

```
Cmd:  21 01 61 01 F0 0D
Resp: 21 01 61 00 01 00 0D               (=Stereo)
```

## Command Summary

| CC | Name | Models | Dl (cmd) | Dl (resp) | Query |
|----|------|--------|----------|-----------|-------|
| `0x00` | Power | All | 1 | 1 | `0xF0` |
| `0x04` | Software Version | All | 1 | 2 | `0xF0` |
| `0x05` | Factory Reset | All | 2 | 0 | N/A (`0xAA 0xAA`) |
| `0x0E` | Mute | All | 1 | 1 | `0xF0` |
| `0x25` | Heartbeat | All | 1 | 1 | `0xF0` |
| `0x26` | Reboot | All | 6 | 1 | N/A (`"REBOOT"`) |
| `0x51` | DC Offset | All | 1 | 1 | `0xF0` |
| `0x52` | Short Circuit | PA720, PA240 | 1 | 1 | `0xF0` |
| `0x53` | Friendly Name | PA720, PA240 | 1 or n | n (max 10) | `0xF0` |
| `0x54` | IP Address | PA720, PA240 | 1 or 4 | 4 | `0xF0` |
| `0x55` | Timeout Counter | All | 1 | 2 | `0xF0` |
| `0x56` | Lifter Temperature | PA720, PA240 | 1 | 2 | `0xF0`/`0xF1` |
| `0x57` | Output Temperature | All | 1 | 2 | `0xF0`/`0xF1` |
| `0x58` | Auto Shutdown | All | 1 | 1 | `0xF0` |
| `0x5A` | Input Detect | All | 1 | 1 | `0xF0` |
| `0x5D` | System Status | All | 1 | 1+ | `0xF0` |
| `0x5E` | System Model | All | 1 | n (max 10) | `0xF0` |
| `0x61` | Amplifier Mode | PA240 | 1 | 1 | `0xF0` |

Commands `0xF0`–`0xFF` are reserved for test functions — **do not use**.

## AMX Discovery (DDDP)

The amp supports AMX Duet Dynamic Device Discovery Protocol.

```
Cmd:  AMX\r
Resp: AMXB<Device-SDKClass=Amplifier><Device-Make=ARCAM><Device-Model=PA720, PA240, PA410><Device-Revision=x.y.z>\r
```
