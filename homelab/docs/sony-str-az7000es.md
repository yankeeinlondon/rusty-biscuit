# Sony STR AZ7000es AV Receiver

## Network APIs

The ES (Elevated Standard) receivers from Sony all support control via TCP/IP. There are two distinct ways to communicate:

1. Sony Audio Control API (REST/JSON): The modern, human-readable method. This uses standard HTTP requests (JSON-RPC) and is the easiest way to start.
2. SDCP / CIS (Common Item Set): The legacy binary protocol used by professional control systems (Control4, Crestron, etc.) on a specific TCP port.


## Receiver Prep

Before sending any code, you must enable network control on the receiver itself.

Network Standby:

- Go to Setup > Network Settings > Network Standby.
- Set this to ON. (This ensures the API works even when the unit is "off").

External Control:

- Go to Setup > Network Settings > External Control.
- Set this to ON.

Static IP:

It is highly recommended to assign a Static IP to the receiver via your router or the receiver's network menu so the address doesn't change.

Example for this guide: 192.168.1.50

## Rest API

This is the most accessible method. You send HTTP POST requests to the receiver.

- Port: Typically 80 or 10000 (Try 10000 first for Sony Audio devices, otherwise 80).
- Endpoint: `http://<Receiver-IP>:<Port>/sony/system` (for power/system) or /sony/audio (for volume).

### Get Power Status

URL: `http://192.168.1.50:10000/sony/system`
Method: POST
Body:

```json
{
    "method": "getPowerStatus",
    "id": 1,
    "params": [],
    "version": "1.1"
}
```

### Turn Power On

URL: `http://192.168.1.50:10000/sony/system`
Method: POST
Body:

```json
{
    "method": "setPowerStatus",
    "id": 1,
    "params": [{"status": "active"}],
    "version": "1.1"
}
```

### Turn Power Off (standby)

URL: `http://192.168.1.50:10000/sony/system`
Method: POST
Body:

```json
{
    "method": "setPowerStatus",
    "id": 1,
    "params": [{"status": "off"}],
    "version": "1.1"
}
```

### Changing Inputs

- You use the endpoint `http://192.168.1.50:10000/sony/avContent`
- To list the available inputs use:

    ```
    {
        "method": "getCurrentExternalInputStatus",
        "id": 1,
        "params": [],
        "version": "1.1"
    }
    ```

### CIS / SDCP (Binary Protocol)

If you are writing a driver for a home automation system and require the raw TCP stream, you use the binary protocol.

- **Port:** `33335` (Standard for modern Sony ES receivers).
- **Protocol:** Raw TCP socket.

This protocol relies on sending specific Hex byte strings. It is faster but harder to debug.

- **Header:** Commands usually start with a specific header (often `0x02` start byte).
- **Example (Conceptual):** A "Power On" command might look like a stream of bytes: `0x02 0x03 0x00 0x01 ...`

*Note: Unless you are building a commercial driver, I strongly recommend sticking to Method 1.*


## REST Versioning per Action (with required params)

### System Endpoint

```csv
Method Name,Ver,Required Parameters (JSON Object),Description
getPowerStatus,1.1,[] (None),"Returns ""active"" or ""standby""."
setPowerStatus,1.1,"{""status"": ""active""} (or ""off"")",Turns receiver On/Off.
getSystemInformation,1.4,[] (None),"Returns Model, Serial, Mac, Firmware Ver."
getInterfaceInformation,1.0,[] (None),"Returns Product Category, Model Name."
getPowerSettings,1.0,"{""target"": ""quickStart""}",Gets specific power settings.
setPowerSettings,1.0,"{""settings"": [...]}",Sets power options.
getDeviceMiscSettings,1.0,"{""target"": ""...""}",Gets miscellaneous settings.
setDeviceMiscSettings,1.0,"{""settings"": [...]}",Sets miscellaneous settings.
getSettingsTree,1.1,"{""usage"": ""...""}",Returns menu structure.
getSWUpdateInfo,1.0,"{""network"": ""network""}",Checks for firmware updates.
actSWUpdate,1.0,[] (None),Triggers firmware update.
connectBluetoothDevice,1.0,"{""bdAddr"": ""XX:XX:...""}",Connects to specific BT device.
getStorageList,1.2,"{""uri"": ""storage:usb1""}",Lists files on USB drive.
getWuTangInfo,1.0,"{""target"": ""...""}",Internal provisioning info.
getEciaDeviceInfo,1.0,[] (None),Internal device ID.
getAlexaRegistrationStatus,1.0,[] (None),Checks Alexa enrollment.
```

### Audio Endpoint

```csv
Method Name,Ver,Required Parameters (JSON Object),Description
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

### AV Content Endpoint

```csv
Method Name,Ver,Required Parameters (JSON Object),Description
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

### App Control Endpoint

```csv
Method Name,Ver,Required Parameters (JSON Object),Description
getApplicationList,1.2,[] (None),"Lists built-in apps (Spotify, etc)."
```


> Critical Note on "Output" Parameter:
>
> Whenever you see {"output": ""} in the AvContent or Audio section, it means the command requires you to specify the Zone.
>
> Main Zone: "" (Empty string usually works) OR "extOutput:zone?zone=1"
> Zone 2: "extOutput:zone?zone=2"
> Zone 3: "extOutput:zone?zone=3"
