# unfolded-integration-helper

Shared runtime helpers for the Unfolded Circle integration drivers in `homelab/`.

This crate exists to hold the integration concerns that are the same across the device-specific drivers, so Arcam, Sony, and Eversolo do not each need their own copy of the same UC protocol glue.

## What It Handles

Functionally, this crate owns ten shared concerns:

1. Parsing incoming Unfolded Circle request envelopes
   It normalizes the UC request shape into a small `IntegrationRequest` / `RequestEnvelope` API so handlers can work with top-level `id`, `msg`, and `msg_data` consistently.

2. Building outgoing UC responses and events
   It provides helpers for the common response and event shapes used by the integrations, including:
   `driver_version`, `driver_metadata`, `available_entities`, `entity_states`, `result`,
   `entity_change`, and `device_state`.

3. Tracking entity state snapshots
   `StateCache` keeps a keyed snapshot of UC entity state, detects meaningful changes, and supports replace/merge workflows so integrations can diff poll results before broadcasting updates.

4. Aggregating device connectivity
   `ConnectivityTracker` stores per-device connectivity and computes the integration-level state the UC Remote expects, instead of every driver inventing its own rollup rules.

5. Managing event subscriptions
   `SubscriptionRegistry` bridges integration handlers to the shared WebSocket host so `subscribe_events` registration and subscriber-only broadcasts stay consistent across drivers.

6. Persistent device registry
   `PersistentRegistry` stores known devices, configured devices, and remote assignments in a JSON file. Writes are atomic (write-to-tmp then rename). Supports auto-seeding from CLI `--host` hints.

7. Device discovery infrastructure
   `DeviceDiscovery` trait, `local_ipv4_candidates`, and `bounded_scan` let each integration probe persisted hosts plus the local LAN with bounded parallelism and timeout management.

8. Multi-device runtime management
   `DeviceManager` owns the lifecycle of multiple `DeviceDriver` instances, polling each independently, routing entity commands, enriching live capability metadata, lazily activating devices as they become relevant, and filtering served entities/state by assigned Remote.

9. Setup flow orchestration
   `SetupState`, `SetupSessions`, `device_selection_schema`, and `device_selection_setup_data` implement the UC Remote-driven device configuration protocol, including per-connection setup state and configurator-compatible `setup_data_schema` plus initialized `setup_data` payloads.

10. Registry data model
    `KnownDevice`, `ConfiguredDevice`, `RemoteAssignment`, and `DeviceMetadata` types represent the full device lifecycle from discovery through configuration and remote binding.

It also includes `test_fixtures` for generating realistic UC request payloads in handler tests.

## Modules

- `envelope`
  Request parsing plus response/event builders for the UC Integration WebSocket protocol.
- `registry`
  Data model types: `KnownDevice`, `ConfiguredDevice`, `RemoteAssignment`, `DeviceMetadata`, `DiscoverySource`.
- `persistent_registry`
  JSON-file-backed persistent storage for the registry with atomic writes and thread-safe access.
- `device_manager`
  Multi-device runtime with `DeviceDriver` trait, per-device polling, entity command routing, and connectivity tracking.
- `discovery`
  `DeviceDiscovery` trait and `bounded_scan` for async device probing with concurrency and timeout bounds.
- `setup`
  Setup flow state machine and schema builder for UC Remote-driven device configuration.
- `state_cache`
  Keyed entity-state storage and diff-friendly update helpers.
- `connectivity`
  Per-device and aggregate connectivity state handling.
- `subscriptions`
  Thin helper around `UnfoldedCircleEventHub` and `WsConnectionContext`.
- `mdns`
  Optional mDNS/DNS-SD advertisement support for external integrations via the `_uc-integration._tcp.local.` service type.
- `test_fixtures`
  Small builders for common request payloads used in unit tests.

## What It Does Not Handle

This crate is intentionally not the place for:

- device transport code
- vendor-specific command mapping
- integration-specific entity definitions

Those responsibilities stay inside the concrete integration crates, each implementing `DeviceDriver` and optionally `DeviceDiscovery`.

## Current Consumers

This helper is used by:

- `arcam-amp-integration`
- `sony-receiver-integration`
- `eversolo-integration`

All three rely on it for the shared UC protocol layer while keeping their actual device behavior separate.

## Design Goal

The point of this crate is not abstraction for its own sake. It exists to keep the integrations aligned on protocol behavior:

- requests use top-level `id`
- responses use top-level `req_id`
- responses use top-level `code`
- configurator compatibility depends on both `driver_version` and `driver_metadata`
- `subscribe_events` controls unsolicited broadcasts
- `entity_change` and `device_state` events are emitted in one consistent shape
- `--host` is optional; integrations start with zero devices and accept remote-driven setup
- device state persists across restarts via `PersistentRegistry`
- a single integration process manages multiple physical devices via `DeviceManager`
- setup schemas must follow the documented UC field contract such as `select` plus `options` and `value`; ad hoc shapes like `dropdown` or `items` are not compatible with the configurator
- initial setup metadata must remain valid even when discovery returns zero candidates, which means omitting device-selection controls that cannot yet be populated
- dynamic setup screens must send initialized `setup_data` entries for every rendered field; sending only the schema can leave the configurator in an invalid state
- long-running setup operations must acknowledge `setup_driver` / `set_driver_user_data` first, then send `driver_setup_change` progress or follow-up input events; delaying the ack can make the configurator time out before it renders the next screen
- inbound UC frames must be classified by `kind` before enforcing request-only fields such as top-level `id`, because setup flows can emit event messages like `abort_driver_setup`

When the UC protocol rules change, this crate should be the first place to update so the integrations stay in sync.

## mDNS Notes

The optional `mdns` feature exists for external integrations that want zero-config discovery from the UC configurator.

- The helper advertises `_uc-integration._tcp.local.` with UC-compatible TXT properties. The visible configurator label and developer line come from mDNS TXT fields, so publish human-facing `name`, `ver`, and `developer` values.
- mDNS only helps the Remote find the driver. After discovery, the configurator still opens the WebSocket connection and asks for protocol data such as `get_driver_metadata`.
- If a driver advertises over mDNS but does not implement `get_driver_metadata`, the integration can appear in discovery lists but fail to open in the configurator with a conflict-style error.
- mDNS traffic on a home LAN can include malformed or partial packets from unrelated devices. Those parser errors are usually observational noise, not proof that the UC integration advertisement is broken.
- Because mDNS is link-local multicast, VLAN boundaries, multicast filtering, or router policy can prevent discovery even when direct WebSocket connectivity works.
