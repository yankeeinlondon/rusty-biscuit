# Sony JSON-RPC Quirks

The Sony Audio Control API (used by ES receivers like the STR-AZ7000ES) has several quirks that make naive `serde` deserialization unreliable.

## 1. `result` vs `results`

Most methods return `"result"` (singular), but `getMethodTypes` returns `"results"` (plural). The `send_command` helper normalizes this by copying `results` -> `result` when present.

## 2. Three Nesting Patterns

Responses use three different shapes for the `result` array:

- `result: [[{...}, ...]]` — double-nested, one entry per zone (e.g. `getPlayingContentInfo`)
- `result: [{...}]` — single-nested object (e.g. `getSystemInformation`)
- `result: [["name", [...], ...], ...]` — flat array of tuples (e.g. `getMethodTypes`)

The `unwrap_sony_result()` helper distinguishes these by checking whether `result[0]` is an array whose first element is an object (double-nested data) vs. a tuple-style array.

## 3. Mixed Field Types

The same logical concept returns different field shapes depending on the endpoint or content source:

- `getCurrentExternalTerminalsStatus` returns `title` as a string
- `getPlayingContentInfo` omits `title` entirely and uses `source`/`uri` instead
- Some fields like `stateInfo` are objects, not strings

## 4. Field Name Inconsistencies

The schema reported by `getMethodTypes` may not match the actual response. Example: `getSystemInformation` v1.4 schema says `serial` but the receiver returns `serialNumber`.

## 5. Ghost Methods

Methods listed by `getMethodTypes` may return error code 12 ("No Such Method") at runtime if the feature is not enabled. Example: `getAlexaRegistrationStatus` appears in the catalog but fails if Alexa has never been configured. Handle gracefully in the CLI with user-friendly messages.

## Recommended Approach

Deserialize Sony responses as `serde_json::Value` and extract fields manually with `value_as_string()` rather than relying on typed structs. This avoids "invalid type: map, expected a string" errors when the API returns an object where a string was assumed.

## API Endpoints

The Sony receiver exposes 8 JSON-RPC endpoints:

| Endpoint | Path | Key Methods |
|----------|------|-------------|
| System | `/sony/system` | power, info, firmware, Alexa |
| Audio | `/sony/audio` | volume, mute, speaker settings |
| AvContent | `/sony/avContent` | inputs, playback, content browsing |
| AccessControl | `/sony/accessControl` | device pairing |
| AppControl | `/sony/appControl` | app management |
| Guide | `/sony/guide` | EPG data |
| Encryption | `/sony/encryption` | key exchange |
| Browser | `/sony/browser` | browser control |

## Output Zone Parameter

Many methods accept an `output` parameter for multi-zone receivers:

- `""` (empty) — main zone
- `"extOutput:zone?zone=2"` — zone 2
- `"extOutput:zone?zone=3"` — zone 3
