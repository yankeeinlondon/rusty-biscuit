# homelab

Core library for controlling home automation devices over the local network.

## Modules

| Module | Description |
|--------|-------------|
| `arcam` | Arcam AV receiver control |
| `ha` | Home Assistant integration |
| `mqtt` | MQTT messaging |
| `network` | Host discovery and network utilities |
| `node_red` | Node-RED flow automation |
| `sony_receiver` | Sony ES receiver control (JSON-RPC) |
| `ubiquiti` | UniFi network device control |

## Lessons Learned

### Sony JSON-RPC Response Format

The Sony Audio Control API (used by ES receivers like the STR-AZ7000ES) has several quirks that make naive `serde` deserialization unreliable:

1. **`result` vs `results`** -- Most methods return `"result"` (singular), but `getMethodTypes` returns `"results"` (plural). The `send_command` helper normalizes this by copying `results` → `result` when present.

2. **Three nesting patterns** -- Responses use three different shapes for the `result` array:
   - `result: [[{...}, ...]]` — double-nested, one entry per zone (e.g. `getPlayingContentInfo`)
   - `result: [{...}]` — single-nested object (e.g. `getSystemInformation`)
   - `result: [["name", [...], ...], ...]` — flat array of tuples (e.g. `getMethodTypes`)

   The `unwrap_sony_result()` helper distinguishes these by checking whether `result[0]` is an array whose first element is an object (double-nested data) vs. a tuple-style array.

3. **Mixed field types across endpoints** -- The same logical concept (e.g., "what's playing") returns different field shapes depending on the endpoint or content source. For example, `getCurrentExternalTerminalsStatus` returns `title` as a string, while `getPlayingContentInfo` omits `title` entirely and uses `source`/`uri` instead. Some fields like `stateInfo` are objects, not strings.

4. **Field name inconsistencies** -- The schema reported by `getMethodTypes` may not match the actual response. For example, `getSystemInformation` v1.4 schema says `serial` but the receiver returns `serialNumber`.

5. **Ghost methods** -- Methods listed by `getMethodTypes` may return error code 12 ("No Such Method") at runtime if the feature is not enabled or provisioned on the receiver. For example, `getAlexaRegistrationStatus` appears in the system method catalog but fails if Alexa has never been configured. Handle these gracefully in the CLI with user-friendly messages.

**Recommended approach**: Deserialize Sony responses as `serde_json::Value` and extract fields manually with a helper like `value_as_string()` rather than relying on typed structs. This avoids "invalid type: map, expected a string" errors when the API returns an object where a string was assumed.
