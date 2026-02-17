# homelab

Core library for controlling home automation devices over the local network.

## Modules

| Module | Description |
|--------|-------------|
| `arcam` | Arcam PA240/PA410/PA720 amplifier control (binary protocol over TCP) |
| `config` | Configuration file management (`~/homey.json`) for device settings |
| `network` | Host addressing (IPv4, IPv6, DNS) |
| `sony_receiver` | Sony ES receiver control (JSON-RPC over HTTP) |

> **Stub modules** (empty files, not yet implemented): `ha`, `mqtt`, `node_red`, `ubiquiti`

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

### CLI Error Handling

1. **Human-readable errors** -- CLI errors must be styled with `Prose` (via `fallback_render`) and presented as a single line. Never expose raw error chains, file locations, or backtrace hints to the user. The `main()` function catches errors and formats them with `<red><b>Error:</b></red>` prefix, deduplicating causes that repeat.

2. **Enumerated parameter hints** -- When a command takes an enumerated target parameter (e.g. `speaker-settings`, `bluetooth`, `playback-mode`) and the user omits it (uses the `all` default), the CLI intercepts the error and shows valid values: `Error: speaker-settings requires a target. Valid targets: level, distance, size, pattern`. When a specific target is provided and fails, the real API error propagates normally — do not mask it with a list of "valid" values that may themselves be wrong.
