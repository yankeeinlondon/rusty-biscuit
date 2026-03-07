# unfolded-integration-helper

Shared runtime helpers for the Unfolded Circle integration drivers in `homelab/`.

This crate exists to hold the integration concerns that are the same across the device-specific drivers, so Arcam, Sony, and Eversolo do not each need their own copy of the same UC protocol glue.

## What It Handles

Functionally, this crate owns five shared concerns:

1. Parsing incoming Unfolded Circle request envelopes
   It normalizes the UC request shape into a small `IntegrationRequest` / `RequestEnvelope` API so handlers can work with top-level `id`, `msg`, and `msg_data` consistently.

2. Building outgoing UC responses and events
   It provides helpers for the common response and event shapes used by the integrations, including:
   `driver_version`, `available_entities`, `entity_states`, `result`, `entity_change`, and `device_state`.

3. Tracking entity state snapshots
   `StateCache` keeps a keyed snapshot of UC entity state, detects meaningful changes, and supports replace/merge workflows so integrations can diff poll results before broadcasting updates.

4. Aggregating device connectivity
   `ConnectivityTracker` stores per-device connectivity and computes the integration-level state the UC Remote expects, instead of every driver inventing its own rollup rules.

5. Managing event subscriptions
   `SubscriptionRegistry` bridges integration handlers to the shared WebSocket host so `subscribe_events` registration and subscriber-only broadcasts stay consistent across drivers.

It also includes `test_fixtures` for generating realistic UC request payloads in handler tests.

## Modules

- `envelope`
  Request parsing plus response/event builders for the UC Integration WebSocket protocol.
- `state_cache`
  Keyed entity-state storage and diff-friendly update helpers.
- `connectivity`
  Per-device and aggregate connectivity state handling.
- `subscriptions`
  Thin helper around `UnfoldedCircleEventHub` and `WsConnectionContext`.
- `test_fixtures`
  Small builders for common request payloads used in unit tests.

## What It Does Not Handle

This crate is intentionally not the place for:

- device transport code
- vendor-specific command mapping
- polling loops
- CLI argument parsing
- driver setup UX
- integration-specific entity definitions

Those responsibilities stay inside the concrete integration crates.

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
- `subscribe_events` controls unsolicited broadcasts
- `entity_change` and `device_state` events are emitted in one consistent shape

When the UC protocol rules change, this crate should be the first place to update so the integrations stay in sync.
