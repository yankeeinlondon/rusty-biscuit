---
name: create-integration
argument-hint: <integration-name> <design brief or @doc reference>
description: Create a new Unfolded Circle integration in `homelab/` using the Arcam integration as the reference implementation
---

**IMPORTANT:** You must use the `unfolded-circle` skill for this command.

Use additional skills only when they are directly relevant to the target integration. In most cases that means `rust`, `clap`, `thiserror`, `just`, and any device-specific skill that matches the transport or domain.

The user's requested action is: `$ARGUMENTS`

If the above is empty or still says `$ARGUMENTS`, stop immediately and reply with:

> You need to provide:
>
> 1. the integration name
> 2. a short design brief, protocol summary, or `@path` to the research/design doc
>
> Example:
>
> - `/create-integration sony-es use @homelab/docs/sony-es-research.md and expose power, volume, mute, and input selection`
> - `/create-integration denon-avr build a UC integration for Denon AVR control over telnet and HTTP`

Do not continue past that point if the arguments are missing.

---

## Intent

Build a production-quality Unfolded Circle integration under `homelab/<integration-name>-integration/`.

Default to an **external integration first** because it is easier to iterate on, debug, and verify. Add installed/local-mode packaging only when you actually implement and document it.

Treat the first whitespace-delimited token in `$ARGUMENTS` as the integration name. Treat the rest as the design brief.

---

## Reference Implementation

Before making changes, inspect these files and use them as the baseline shape for the new integration:

- `homelab/arcam-amp-integration/README.md`
- `homelab/arcam-amp-integration/Dockerfile`
- `homelab/arcam-amp-integration/docker-compose.yaml`
- `homelab/arcam-amp-integration/justfile`
- `homelab/unfolded-integration-helper/src/lib.rs`
- `homelab/arcam-amp-integration/Cargo.toml`
- `homelab/arcam-amp-integration/src/main.rs`
- `homelab/arcam-amp-integration/src/handler.rs`
- `homelab/arcam-amp-integration/src/driver.rs`
- `homelab/arcam-amp-integration/src/discovery.rs`
- `homelab/arcam-amp-integration/src/dispatch.rs`
- `homelab/arcam-amp-integration/src/types.rs`
- `homelab/arcam-amp-integration/src/error.rs`
- `schematic/schema/src/unfolded_circle_integration_ws.rs`

The Arcam integration establishes the house pattern:

- the integration driver is the **WebSocket server**
- the UC Remote is the **WebSocket client**
- transport-specific I/O is isolated from the UC protocol handler
- `homelab/unfolded-integration-helper` owns UC envelope parsing/building, keyed state caching, device registry, discovery, setup flow, and subscription helpers
- entity definitions and command resolution are kept in pure types/helpers
- a `DeviceDriver` trait implementation in `driver.rs` bridges device-specific logic to the generic `DeviceManager`
- a `DeviceDiscovery` trait implementation in `discovery.rs` enables network probing
- `--host` is optional (seed hint); integrations can start with zero devices and accept remote-driven setup
- `--data-dir` overrides the persistent registry location
- `PersistentRegistry` persists device/remote state across restarts

---

## Review-Driven Guardrails

When creating the new integration, explicitly account for these lessons from the existing Arcam implementation:

1. The integration must implement the required Unfolded Circle request flow:
   - `get_driver_version`
   - `get_driver_metadata`
   - `get_device_state`
   - `get_available_entities`
   - `subscribe_events`
   - `get_entity_states`
   - `entity_command`

   Do not treat mDNS discovery as sufficient by itself. A driver that advertises on the network but does not answer `get_driver_metadata` correctly can appear in the configurator and still fail to open.

2. Keep the transport adapter separate from the UC protocol layer:
   - device or service communication belongs in `dispatch.rs` or a similarly focused module
   - `handler.rs` should translate UC requests into domain operations, not perform low-level protocol work inline

3. Do not overstate authentication support:
   - verify the actual behavior of `UnfoldedCircleIntegrationWsHost` in `schematic/schema/src/unfolded_circle_integration_ws.rs`
   - only document unauthenticated operation if the underlying host actually permits it
   - only document token auth if you have confirmed the `auth-token` header behavior end-to-end

4. Do not claim installed/local-mode support unless you really build it:
   - create a real `driver.json`
   - add packaging instructions that match the actual archive layout
   - ensure the README reflects the files that exist in the package

5. Implement and describe state synchronization correctly:
    - `device_state` is for the integration's connection/availability relative to the target system, not for per-entity power/input/volume changes
    - per-entity state changes must be sent as `entity_change` events when the real device changes, even if the change did not originate from the remote
    - support unsolicited updates from the real device by either subscribing to native device events or polling and diffing against cached state
    - keep an internal entity-state cache so `get_entity_states` returns the latest known snapshot after reconnects, standby wake, or explicit refresh requests
    - parse inbound UC requests from top-level `id`
    - emit responses with top-level `req_id` and top-level `code`
    - treat `entity_command` as a synchronous `result` response plus later `entity_change` events when state changes

6. Model entities from user value, not protocol convenience:
   - expose only controls and states the target device can reliably support
   - prefer a smaller accurate entity set over a larger speculative one

7. Treat mDNS as an optional discovery layer, not as the integration contract:
   - if you expose an `--mdns` flag, document exactly what service type is advertised and how to enable it
   - explain that mDNS is link-local and may fail across VLANs or multicast-filtered networks
   - document that malformed mDNS packets from unrelated LAN devices can appear in logs and are not automatically integration bugs
   - when setting default logging filters, prefer suppressing noisy third-party mDNS parser spam while still allowing opt-in debugging via `RUST_LOG`

---

## Required Output

Create or update the integration so that it includes, at minimum:

- `homelab/<integration-name>-integration/Cargo.toml`
- `homelab/<integration-name>-integration/README.md`
- `homelab/<integration-name>-integration/justfile`
- `homelab/<integration-name>-integration/Dockerfile`
- `homelab/<integration-name>-integration/docker-compose.yaml`
- `homelab/<integration-name>-integration/src/main.rs`
- `homelab/<integration-name>-integration/src/handler.rs`
- `homelab/<integration-name>-integration/src/driver.rs`
- `homelab/<integration-name>-integration/src/discovery.rs`
- `homelab/<integration-name>-integration/src/dispatch.rs`
- `homelab/<integration-name>-integration/src/types.rs`
- `homelab/<integration-name>-integration/src/error.rs`

Add these when needed:

- `driver.json` for installed/local-mode packaging
- additional modules only when the protocol complexity warrants them

---

## Implementation Process

### 1. Understand the Target

Use the design brief and any referenced docs to determine:

- the real transport: TCP, telnet, HTTP, WebSocket, serial bridge, cloud API, etc.
- the core capabilities worth exposing as UC entities
- the state model you can actually read back reliably
- how unsolicited state changes are detected: device push notifications, subscriptions, webhooks, polling, or no mechanism at all
- configuration required to connect: host, port, credentials, device name, timeout, zones, IDs, etc.

If the user referenced local files with `@path`, read them before designing anything.

If critical details are missing, make the minimum reasonable assumptions needed to continue and state them clearly in the final response.

### 2. Design the UC Entity Surface

Before coding, define:

- entity IDs
- entity types
- supported features
- command IDs
- state attributes
- how device operations map onto UC commands
- which state transitions should emit `entity_change` events
- which backend failures should emit `device_state`

Prefer a small explicit table in the README.

### 3. Scaffold the Package

Default location:

- `homelab/<integration-name>-integration/`

Default binary name:

- `<integration-name>-integration`

Follow the Arcam package shape unless there is a strong reason not to.

Scaffold these operational files as part of the initial package, not as optional follow-up:

- `Dockerfile`
- `docker-compose.yaml`
- `justfile`

The package-local `justfile` must expose these recipe names:

- `install`
- `build-image`
- `sanity-test`
- `sanity-test-mutate`

### 4. Implement the Runtime

Your implementation should generally follow this structure:

- `main.rs`
    - parse CLI arguments with `clap` (`--host` optional, `--data-dir` for persistence)
    - initialize tracing
    - load `PersistentRegistry` from data dir
    - create `DeviceManager` with registry + subscriptions
    - seed from `--host` if provided, load persisted devices
    - create `UnfoldedCircleIntegrationWsHost::new_event_hub()`
    - construct the handler with the `DeviceManager`
    - start `UnfoldedCircleIntegrationWsHost::serve_addr_with_hub(...)`

- `driver.rs`
    - implement `DeviceDriver` trait from the helper crate
    - `build_entities()` returns UC entities for a configured device
    - `build_initial_states()` returns unknown/default states
    - `fetch_snapshot()` polls the real device and returns entity updates
    - `execute_command()` executes a command and returns updated state

- `discovery.rs`
    - implement `DeviceDiscovery` trait from the helper crate
    - `validate_host()` probes a candidate address and returns device metadata

- `types.rs`
    - driver constants
    - entity structs
    - command enums
    - entity builders
    - command resolution helpers

- `error.rs`
    - integration error enum with `thiserror`
    - map internal failures to UC result codes

- `homelab/unfolded-integration-helper`
    - use its request parser, response/event builders, keyed state cache, protocol fixtures, and subscription bridge
    - do not hand-build UC envelopes ad hoc in each integration

- `dispatch.rs`
    - transport-specific command execution
    - timeout handling
    - state fetching
    - device event subscription and/or polling loop when the protocol supports external state changes
    - translation from protocol results into UC attributes

- `handler.rs`
    - implement `WsHandler`
    - route incoming UC messages
    - parse requests with `IntegrationRequest`
    - return `result` acknowledgements for commands
    - track subscribers from `subscribe_events` and broadcast `entity_change` / `device_state` events as needed

State handling requirements:

- maintain an internal cache of the latest known entity attributes
- update that cache after successful remote-initiated commands and after externally observed device changes
- replace or merge cached attributes intentionally so stale fields are cleared when the device schema changes
- emit `entity_change` when a device powers off due to inactivity, when someone changes a setting at the physical device, or when any other non-remote action changes a mapped UC attribute
- emit `device_state` only when the integration's ability to talk to the target system changes, such as connect, disconnect, unavailable, or error transitions
- make sure `get_entity_states` returns the cached snapshot so the remote can resync after reconnect or standby wake

### 5. Documentation

The README must include:

- what the integration controls
- entity table
- architecture summary
- whether it supports mDNS discovery and how to enable it
- the distinction between mDNS discovery and the required configurator handshake, especially `get_driver_metadata`
- how state synchronization works, including how unsolicited external changes reach the remote
- how to run it as an external integration
- how to build and run it with the checked-in `Dockerfile` and `docker-compose.yaml`
- configuration flags and environment variables
- how to register it with the remote
- installed/local-mode packaging steps only if implemented
- key limitations and known gaps

Do not describe files, packaging artifacts, or features that do not exist.

### 6. Validation

Use the repo conventions:

- prefer `just` commands for verification
- run the narrowest relevant build/test commands you can from the appropriate package area

At minimum, if practical:

- build the package
- run its tests

If you could not run validation, say exactly what you did not verify.

---

## Quality Bar

The generated integration should be:

- idiomatic Rust
- aligned with monorepo conventions
- honest about limitations
- externally runnable
- easy to package later for local installation
- covered by meaningful unit tests for pure mapping logic and handler behavior

Production code must not use `unwrap()` or `expect()`.

---

## Final Response Format

When you finish, report:

1. what you created or changed
2. the entity model you chose
3. how the integration is run and validated
4. assumptions, limitations, or follow-up work

If you discovered protocol or repo-level issues while building the integration, call them out explicitly instead of hiding them.
