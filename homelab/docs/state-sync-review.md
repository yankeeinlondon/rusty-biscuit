# State Sync Review

Review date: 2026-03-07

Scope:

- `homelab/arcam-amp-integration/`
- `homelab/eversolo-integration/`
- `homelab/sony-receiver-integration/`

Validation run:

- `cargo test -p arcam-amp-integration -p eversolo-integration -p sony-receiver-integration`
- Result: all existing tests passed

This review focuses first on Unfolded Circle state synchronization, then on adjacent issues that materially affect correctness, ergonomics, performance, and test quality.

## Cross-Cutting Findings

### 1. The shared WebSocket host cannot emit real unsolicited events today

All three integrations acknowledge `subscribe_events`, but the shared host only writes a message when `WsHandler::handle_message(...)` returns a value while servicing an inbound frame.

Relevant code:

- `schematic/schema/src/unfolded_circle_integration_ws.rs:437-452`

Impact:

- True out-of-band `entity_change` and `device_state` events cannot be emitted today.
- Polling loops can keep an internal cache fresh, but they cannot proactively update the remote.
- This especially affects the Eversolo integration, which already has a useful polling loop but no broadcast path.

Recommended fix:

- Extend `UnfoldedCircleIntegrationWsHost` with connection/subscriber tracking plus an async send API or broadcast channel.
- Keep `entity_command` acknowledgements as normal `resp` messages.
- Use `entity_change` and `device_state` only for actual event delivery after `subscribe_events`.

### 2. Response envelope handling appears to be spec-inconsistent across all three integrations

Based on the local schema definitions, requests use top-level `id`, responses use top-level `req_id` and top-level `code`, and events are separate envelopes.

Relevant code:

- `schematic/definitions/src/unfolded_circle/integration_ws/types.rs:90-130`
- `homelab/arcam-amp-integration/src/handler.rs:103-105`
- `homelab/eversolo-integration/src/handler.rs:250-255`
- `homelab/sony-receiver-integration/src/handler.rs:150-152`
- `homelab/arcam-amp-integration/src/responses.rs:8-18`
- `homelab/eversolo-integration/src/responses.rs:8-18`
- `homelab/sony-receiver-integration/src/responses.rs:8-18`
- `homelab/arcam-amp-integration/src/responses.rs:71-77`
- `homelab/eversolo-integration/src/responses.rs:75-81`
- `homelab/sony-receiver-integration/src/responses.rs:75-81`

Impact:

- Each handler reads `req_id` from inbound requests instead of `id`.
- Each `result` response puts `code` inside `msg_data` instead of at the top level.
- `entity_command` success paths are modeled as immediate `entity_change` event payloads rather than an acknowledged response plus a separate event path.

Recommended fix:

- Add a shared envelope builder/parser layer instead of hand-building JSON in each package.
- Update tests to use spec-shaped request envelopes so the same mistake does not keep being copied.

### 3. There is too much copy-pasted integration scaffolding

The same handler structure, response builders, cached state handling, and envelope mistakes are repeated in all three drivers.

Impact:

- Bugs in one integration are easy to clone into the others.
- State-sync behavior diverges in confusing ways.
- Small protocol fixes must be applied three times.

Recommended fix:

- Extract a shared helper crate or internal module for:
  - DECISION: create a small library called `unfolded-integration-helper` which will be defined at homelab/unfolded-integration-helper and will provide the following features as a high quality and reusable package to all integrations:
      - envelope parsing/building
      - subscriber tracking
      - entity cache management
      - common request routing
      - test fixtures for UC request/response shape
  - The @.claude/commands/create-integration.md command will be updated to ensure that this helper library is used going forward and we will update the existing three integrations to ensure they use this too

## Arcam

### Summary

Arcam currently has the weakest state synchronization of the three integrations. It has no background refresh, no on-demand refresh in `get_entity_states`, and no truthful device connectivity model. The cache starts as `OFF` for both entities and only changes after commands sent through this integration.

### Findings

#### High: `get_entity_states` returns a static cache that is never refreshed from the amplifier

Relevant code:

- `homelab/arcam-amp-integration/src/handler.rs:128-133`
- `homelab/arcam-amp-integration/src/types.rs:71-83`
- `homelab/arcam-amp-integration/src/main.rs:54-66`
- `homelab/arcam-amp-integration/src/dispatch.rs:23-61`

Why it matters:

- If the amplifier powers down due to inactivity, is muted from the front panel, or is changed by another controller, the UC Remote will not learn about it.
- Even the initial `get_entity_states` response is speculative because the cache is hard-coded to `OFF`.

Recommended fix:

- Add a real state refresh path that queries `request_power_state()` and `get_mute_status()`.
- Run that refresh on startup before serving connections.
- Run it again in `get_entity_states`.
- If the host gains async broadcast support, also poll or subscribe in the background and emit `entity_change` when state diffs are detected.

#### High: `subscribe_events` is acknowledged, but Arcam has no event source at all

Relevant code:

- `homelab/arcam-amp-integration/src/handler.rs:128`
- `schematic/schema/src/unfolded_circle_integration_ws.rs:437-452`

Why it matters:

- The integration claims event subscription support but does nothing with it.
- This makes the state-sync story look more complete than it is.

Recommended fix:

- After the host supports broadcasts, track subscribers and emit:
  - `entity_change` when power or mute changes outside the remote
  - `device_state` when amplifier connectivity changes
- Until then, document the limitation more plainly in the README.

#### Medium: `get_device_state` reports `CONNECTED` whenever a device is configured

Relevant code:

- `homelab/arcam-amp-integration/src/handler.rs:117-123`

Why it matters:

- `device_state` should reflect whether the integration can currently talk to the amplifier.
- Today it only reflects whether CLI configuration exists.

Recommended fix:

- Track last successful refresh/probe and derive `device_state` from that.
- Use states such as `CONNECTED` only after a successful TCP query.
- Return `DISCONNECTED` or `ERROR` after repeated failures or explicit connection loss.

#### Medium: The README normalizes the current non-syncing behavior

Relevant code:

- `homelab/arcam-amp-integration/README.md:35-40`
- `homelab/arcam-amp-integration/README.md:202-205`

Why it matters:

- The documentation shows `entity_change` as the direct answer to `entity_command`.
- It also frames proactive push as only a future concern, which risks copying the same design into new integrations.

Recommended fix:

- Update the README to describe:
  - cached state versus live state
  - the absence of unsolicited updates
  - the desired future behavior once the host supports broadcasts

#### Medium: Test coverage is not adequate for sync correctness

Relevant code:

- `homelab/arcam-amp-integration/src/handler.rs:163-260`
- `homelab/arcam-amp-integration/src/dispatch.rs:76-88`

Current state:

- Existing tests cover helper functions and basic request routing.
- They do not cover any real state refresh behavior because none exists yet.

Recommended tests:

- A state refresh unit or integration test that updates both power and mute from queried device data.
- A startup-refresh test proving the initial cache is not left at all-`OFF`.
- A reconnect test proving `device_state` flips after connectivity loss and recovery.
- A drift test proving external state changes overwrite the cached state on the next refresh.
- Spec-shape tests using inbound request `id` rather than `req_id`.

#### Low: Documentation drift outside state sync

Relevant code:

- `homelab/arcam-amp-integration/README.md:128-179`

Observation:

- The README documents installed/local-mode packaging and `driver.json`, but this package directory does not currently contain `driver.json`.

Recommended fix:

- Either add the real packaging assets or remove the installed-mode section until it exists.

### Ergonomics and Performance Notes

- The cache is stored as `Vec<ArcamEntityState>` and updated via repeated linear scans in `handler.rs`. With only two entities this is not a performance problem, but a `HashMap<String, ArcamEntityState>` would simplify updates and reduce repeated copy-paste logic.
- A small `refresh_device_state(host, port, timeout)` helper returning both power and mute state would make the sync path clearer and easier to test.

## Eversolo

### Summary

Eversolo has the best internal sync story today because it does keep an internal cache fresh with a polling loop. The main gap is that the fresh cache never reaches the remote proactively, and some command paths still leave companion entities or stale attributes behind.

### Findings

#### High: Polling refreshes the cache, but unsolicited changes still never reach the remote

Relevant code:

- `homelab/eversolo-integration/src/handler.rs:53-115`
- `homelab/eversolo-integration/src/handler.rs:264-272`
- `schematic/schema/src/unfolded_circle_integration_ws.rs:437-452`
- `homelab/eversolo-integration/README.md:133-145`

Why it matters:

- Front-panel changes, mobile app changes, and offline-to-online recovery are visible only in the driver's in-memory cache.
- The remote still has to ask again before it sees the update.

Recommended fix:

- Keep the poller.
- Add subscriber/broadcast support once the shared host allows it.
- Emit `entity_change` only when a poll detects an actual diff.
- Emit `device_state` when connectivity flips between reachable and unreachable.

#### Medium: Power commands update two entities internally, but only one `entity_change` is surfaced

Relevant code:

- `homelab/eversolo-integration/src/dispatch.rs:199-239`
- `homelab/eversolo-integration/src/handler.rs:198-238`

Why it matters:

- `power_on` and `power_off` update both the `power` switch and the `player` entity.
- The handler selects only one update to return.
- The companion entity can stay stale on the remote until the next poll or explicit state fetch.

Recommended fix:

- Once event broadcasting exists, emit one event per changed entity.
- In the meantime, prefer a full state refresh after power transitions and ensure `get_entity_states` immediately reflects both entities.

#### Medium: Power transitions leave stale player metadata in the cache

Relevant code:

- `homelab/eversolo-integration/src/dispatch.rs:206-216`
- `homelab/eversolo-integration/src/dispatch.rs:228-238`
- `homelab/eversolo-integration/src/handler.rs:203-212`

Why it matters:

- `player_power_attrs(...)` only returns `state`.
- The handler merges attributes key-by-key.
- After power-off, old `volume`, `source`, and track metadata remain in memory until the next poll failure or refresh overwrites them.

Recommended fix:

- Replace the full entity attribute map on state-shape-changing updates such as power transitions.
- Alternatively, make `player_power_attrs(false)` explicitly clear or null out stale fields.

#### Medium: Offline player state shape is too lossy

Relevant code:

- `homelab/eversolo-integration/src/handler.rs:282-287`

Why it matters:

- Offline replacement collapses the player entity to `{state: OFF}`.
- That may be acceptable, but it is an abrupt schema change compared with the richer normal shape.

Recommended fix:

- Decide on a stable offline schema and document it.
- If the remote benefits from stable attributes, keep `muted`, `source`, and maybe volume fields present with explicit neutral values.

#### Medium: Test coverage misses the important sync paths

Relevant code:

- `homelab/eversolo-integration/src/handler.rs:290-392`
- `homelab/eversolo-integration/src/dispatch.rs:199-258`

Current state:

- Existing tests cover many helpers and routing behavior.
- They do not cover the polling lifecycle or multi-entity sync behavior.

Recommended tests:

- A `refresh_device` success test that updates cache, catalog, and connectivity together.
- A `refresh_device` failure test that marks the device disconnected and applies the offline state shape.
- A power-on/power-off command test that proves both entities are updated and stale metadata is cleared.
- A poll-diff test that proves only changed attributes would be broadcast once the host supports it.
- A reconnect/resync test proving the first successful poll after failure restores `device_state` and entity cache coherently.

### Ergonomics and Performance Notes

- `states` is a `Vec` with repeated linear scans. Moving to a map keyed by `entity_id` would simplify mutation code and make multi-entity updates less error-prone.
- `refresh_device` currently acquires separate write locks for `catalogs`, `connectivity`, and `states`. That is acceptable at this scale, but a small aggregated device-cache struct would make the refresh path easier to reason about.

## Sony

### Summary

Sony is materially better than Arcam because `get_entity_states` does attempt a fresh device read, but it still has meaningful sync gaps. The biggest ones are incorrect `device_state`, no proactive update path, and incomplete state refresh for the advertised `source` attribute.

### Findings

#### High: `device_state` does not reflect actual receiver reachability

Relevant code:

- `homelab/sony-receiver-integration/src/handler.rs:164-170`

Why it matters:

- The integration reports `CONNECTED` whenever a device is configured.
- If the receiver is offline or HTTP calls are failing, the remote still gets a healthy device-state answer.

Recommended fix:

- Track last successful receiver probe or refresh and derive `device_state` from that.
- Update that connectivity state during `get_entity_states` and after command failures/successes.
- Broadcast `device_state` transitions once the host supports async events.

#### High: External source changes are never synchronized

Relevant code:

- `homelab/sony-receiver-integration/src/types.rs:93-99`
- `homelab/sony-receiver-integration/src/dispatch.rs:91-120`
- `homelab/sony-receiver-integration/src/handler.rs:117-145`

Why it matters:

- The media-player entity advertises and stores a `source` attribute.
- `fetch_receiver_state(...)` only refreshes power, volume, and mute.
- If someone changes the input from the Sony remote, front panel, or another controller, `get_entity_states` still will not repair the cached `source`.

Recommended fix:

- Extend `fetch_receiver_state(...)` to query the current input URI and map it back to the advertised category or display label.
- Update both the `receiver` entity cache and returned state snapshot with that source value.

#### Medium: There is still no proactive update path for unsolicited changes

Relevant code:

- `homelab/sony-receiver-integration/src/handler.rs:175-177`
- `homelab/sony-receiver-integration/README.md:116-121`
- `schematic/schema/src/unfolded_circle_integration_ws.rs:437-452`

Why it matters:

- External changes only become visible when the remote asks for `get_entity_states`.
- That is better than Arcam, but it still is not true subscription-driven state sync.

Recommended fix:

- After the shared host is upgraded, add a polling loop or subscription strategy and emit `entity_change` for drift.
- Until then, at least keep a warm background cache so `get_entity_states` can answer quickly without doing all network work inline.

#### Medium: Command paths can leave related entity state stale

Relevant code:

- `homelab/sony-receiver-integration/src/dispatch.rs:28-79`
- `homelab/sony-receiver-integration/src/handler.rs:80-108`

Why it matters:

- Power commands only update the `power` switch entity.
- The `receiver` media-player entity can still show stale `state`, `volume`, `muted`, or `source` until the next `get_entity_states`.

Recommended fix:

- After successful power commands, fetch and apply a full receiver snapshot.
- When async events are possible, emit separate changes for both `power` and `receiver` if both changed.

#### Medium: `get_entity_states` silently preserves stale cache on refresh failures

Relevant code:

- `homelab/sony-receiver-integration/src/handler.rs:117-145`

Why it matters:

- If a refresh fails, the old cache remains untouched and the caller receives it.
- Without a separate connectivity model, stale cached state can look current.

Recommended fix:

- Record per-device refresh failure.
- Update `device_state` accordingly.
- Consider marking entity state as `UNKNOWN` or otherwise stale after repeated failures.

#### Medium: Test coverage does not cover the real sync risks

Relevant code:

- `homelab/sony-receiver-integration/src/handler.rs:207-297`
- `homelab/sony-receiver-integration/src/dispatch.rs:197-240`

Current state:

- Existing tests cover helpers and request routing.
- They do not cover real refresh semantics or connectivity transitions.

Recommended tests:

- A `get_entity_states` refresh test that updates power, volume, mute, and source together.
- A failure-path test proving stale cache is either marked stale or paired with a disconnected device state.
- A power command test proving the `receiver` entity state is repaired after a power transition.
- A source-change drift test covering a physical or third-party source change.
- Spec-shape tests using request `id` and top-level response `code`.

### Ergonomics and Performance Notes

- `handle_get_entity_states(...)` fetches and mutates per device sequentially. With few devices this is fine, but a future multi-device version could fetch snapshots first, then acquire the state lock once and apply them.
- Like the other integrations, using a map for state storage would simplify updates and reduce repeated linear scans.

## Test Coverage Assessment

### Adequacy today

- Arcam: inadequate for state synchronization
- Eversolo: moderate helper coverage, inadequate sync coverage
- Sony: moderate helper coverage, inadequate sync coverage

### Why the current suites are still insufficient

- They prove that the current happy-path JSON builders and message routers work.
- They do not prove that the remote stays correct when the physical device changes on its own.
- They do not prove reconnect or standby resync behavior.
- They do not prove that `device_state` and per-entity `entity_change` are kept distinct.
- They do not prove that state caches are repaired after transport failures.

### Highest-value new tests across the board

- Add mocked or fake-device tests for successful refresh of every advertised attribute.
- Add drift tests where the physical device changes outside the UC command path.
- Add reconnect tests where the device disappears and later comes back.
- Add protocol-envelope tests using the local schema definitions instead of ad hoc JSON shapes.
- Add integration tests for `subscribe_events` and event fan-out once the shared host supports it.

## Recommended Implementation Order

1. Fix the shared host so integrations can actually broadcast unsolicited events.
2. Fix request/response envelope handling and stop using `entity_change` as a synchronous command reply.
3. Repair Arcam state refresh first because it currently has no real resync path.
4. Repair Sony source synchronization and truthful device connectivity next.
5. Tighten Eversolo multi-entity update behavior and stale-field clearing.
6. Add shared test fixtures so the same envelope and sync mistakes do not regress.
