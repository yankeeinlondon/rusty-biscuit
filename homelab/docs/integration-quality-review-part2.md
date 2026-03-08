# Integration Quality Review, Part 2

## Scope

This follow-up review covers the current post-refactor state of:

- `homelab/unfolded-integration-helper`
- `homelab/arcam-amp-integration`
- `homelab/eversolo-integration`
- `homelab/sony-receiver-integration`

I focused on the architectural goals from `integration-quality-review.md`: Remote-driven setup, multi-device support, multi-remote support, persistent registry correctness, and truthful entity/state modeling.

`just -f homelab/justfile test` passed during this review. The findings below are based on source inspection of the current checkout, not failing tests.

## Findings

### High: Remote-driven setup is still not implemented end-to-end

The refactor added setup helpers in `homelab/unfolded-integration-helper/src/setup.rs`, but none of the three integrations actually expose a working setup flow yet.

- All three metadata structs still omit any `setup_data_schema` field:
  - `homelab/arcam-amp-integration/src/types.rs:23`
  - `homelab/eversolo-integration/src/types.rs:25`
  - `homelab/sony-receiver-integration/src/types.rs:23`
- All three WebSocket handlers still only dispatch the legacy steady-state message set and fall through to `400` for anything else:
  - `homelab/arcam-amp-integration/src/handler.rs:62`
  - `homelab/eversolo-integration/src/handler.rs:62`
  - `homelab/sony-receiver-integration/src/handler.rs:62`
- Startup is still driven entirely by `--host` seeding and/or loading preexisting configured devices from disk:
  - `homelab/arcam-amp-integration/src/main.rs:85`
  - `homelab/eversolo-integration/src/main.rs:94`
  - `homelab/sony-receiver-integration/src/main.rs:81`

Result: the helper now contains setup primitives, but a fresh Remote still cannot configure any of these integrations through the integration protocol itself. The main architectural issue from part 1 therefore remains unresolved.

### High: Per-Remote assignment exists in storage but is ignored by the serving path

The new registry model introduces `RemoteAssignment` and assignment-aware lookups, but the runtime still serves every configured device to every connection.

- Assignment-aware APIs exist in the helper:
  - `homelab/unfolded-integration-helper/src/persistent_registry.rs:157`
  - `homelab/unfolded-integration-helper/src/persistent_registry.rs:225`
- The device manager only exposes global inventory/state methods:
  - `homelab/unfolded-integration-helper/src/device_manager.rs:208`
  - `homelab/unfolded-integration-helper/src/device_manager.rs:217`
- Each handler uses those global methods for `get_available_entities` and `get_entity_states`:
  - `homelab/arcam-amp-integration/src/handler.rs:79`
  - `homelab/eversolo-integration/src/handler.rs:79`
  - `homelab/sony-receiver-integration/src/handler.rs:79`
- Each integration also activates every configured device at startup instead of loading only the devices assigned to the connecting Remote:
  - `homelab/arcam-amp-integration/src/main.rs:96`
  - `homelab/eversolo-integration/src/main.rs:134`
  - `homelab/sony-receiver-integration/src/main.rs:97`

Result: duplicate-assignment prevention still does not happen in practice, because the runtime never filters entities or state by Remote identity.

### High: Eversolo `--host` seeding can overwrite persisted configuration for the same device

The Eversolo integration does not use `seed_from_cli_hint`. Instead, it reconstructs a fresh `ConfiguredDevice` from CLI/default values and upserts it directly into the registry.

- `homelab/eversolo-integration/src/main.rs:112`
- `homelab/eversolo-integration/src/main.rs:121`
- `homelab/unfolded-integration-helper/src/persistent_registry.rs:123`

Because `PersistentRegistry::add_configured_device()` replaces any existing record with the same `device_id`, restarting with `--host` can silently clobber a previously saved `device_name` or `driver_config` for that host:port. That reintroduces the old startup-time ownership problem for Eversolo even after the registry refactor.

### Medium: Discovery still does not produce stable physical identities

The new registry/assignment model depends on a stable physical identity, but discovery still falls back to `host:port` whenever `mac_address` is absent.

- Helper fallback:
  - `homelab/unfolded-integration-helper/src/discovery.rs:78`
- Discovery implementations currently return friendly labels but no stable physical identifier:
  - `homelab/arcam-amp-integration/src/discovery.rs:26`
  - `homelab/eversolo-integration/src/discovery.rs:26`
  - `homelab/sony-receiver-integration/src/discovery.rs:26`

Result: if a device moves to a new IP, the registry will treat it as a different device and create duplicate entries/assignments instead of reconciling it to the same physical unit.

### Medium: Eversolo still advertises incomplete player options even though the transport layer knows the real catalog

The Eversolo snapshot path already computes a dynamic source list and the real volume step range, but the entity advertisement still throws that information away.

- Dynamic catalog is fetched:
  - `homelab/eversolo-integration/src/dispatch.rs:47`
  - `homelab/eversolo-integration/src/dispatch.rs:57`
- The driver still advertises `build_entities(name, &[], DEFAULT_VOLUME_STEPS)`:
  - `homelab/eversolo-integration/src/driver.rs:48`
- Those defaults become the published `source_list` / `volume_steps` options:
  - `homelab/eversolo-integration/src/types.rs:98`

Result: the Remote still gets an empty `source_list` and a hard-coded volume range even though the integration already knows the real values. `select_source` therefore remains materially underfinished.

### Medium: Sony still advertises a hard-coded source list instead of the receiver’s actual configured inputs

Sony command execution resolves sources against the live receiver configuration, but the entity it advertises to the Remote still uses a fixed constant.

- Static source list:
  - `homelab/sony-receiver-integration/src/types.rs:94`
  - `homelab/sony-receiver-integration/src/driver.rs:29`
- Live command-time resolution against real native inputs:
  - `homelab/sony-receiver-integration/src/dispatch.rs:79`
  - `homelab/sony-receiver-integration/src/dispatch.rs:149`

Result: the Remote can present source choices that the actual receiver does not have, no longer maps, or has hidden. When that happens, the driver has advertised an option it cannot reliably execute.
