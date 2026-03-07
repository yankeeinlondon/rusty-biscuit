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
- `homelab/arcam-amp-integration/Cargo.toml`
- `homelab/arcam-amp-integration/src/main.rs`
- `homelab/arcam-amp-integration/src/handler.rs`
- `homelab/arcam-amp-integration/src/dispatch.rs`
- `homelab/arcam-amp-integration/src/types.rs`
- `homelab/arcam-amp-integration/src/responses.rs`
- `homelab/arcam-amp-integration/src/error.rs`
- `schematic/schema/src/unfolded_circle_integration_ws.rs`

The Arcam integration establishes the house pattern:

- the integration driver is the **WebSocket server**
- the UC Remote is the **WebSocket client**
- transport-specific I/O is isolated from the UC protocol handler
- JSON response/event builders live in a dedicated module
- entity definitions and command resolution are kept in pure types/helpers

---

## Review-Driven Guardrails

When creating the new integration, explicitly account for these lessons from the existing Arcam implementation:

1. The integration must implement the required Unfolded Circle request flow:
   - `get_driver_version`
   - `get_device_state`
   - `get_available_entities`
   - `subscribe_events`
   - `get_entity_states`
   - `entity_command`

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

5. Be honest about eventing limitations:
   - the current handler pattern is request-response oriented
   - if proactive push events are not implemented, say so
   - do not pretend polling or push updates exist unless the code actually does that

6. Model entities from user value, not protocol convenience:
   - expose only controls and states the target device can reliably support
   - prefer a smaller accurate entity set over a larger speculative one

---

## Required Output

Create or update the integration so that it includes, at minimum:

- `homelab/<integration-name>-integration/Cargo.toml`
- `homelab/<integration-name>-integration/README.md`
- `homelab/<integration-name>-integration/src/main.rs`
- `homelab/<integration-name>-integration/src/handler.rs`
- `homelab/<integration-name>-integration/src/dispatch.rs`
- `homelab/<integration-name>-integration/src/types.rs`
- `homelab/<integration-name>-integration/src/responses.rs`
- `homelab/<integration-name>-integration/src/error.rs`

Add these when needed:

- `driver.json` for installed/local-mode packaging
- `Dockerfile` and `docker-compose.yml` for external deployment
- additional modules only when the protocol complexity warrants them

---

## Implementation Process

### 1. Understand the Target

Use the design brief and any referenced docs to determine:

- the real transport: TCP, telnet, HTTP, WebSocket, serial bridge, cloud API, etc.
- the core capabilities worth exposing as UC entities
- the state model you can actually read back reliably
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

Prefer a small explicit table in the README.

### 3. Scaffold the Package

Default location:

- `homelab/<integration-name>-integration/`

Default binary name:

- `<integration-name>-integration`

Follow the Arcam package shape unless there is a strong reason not to.

### 4. Implement the Runtime

Your implementation should generally follow this structure:

- `main.rs`
    - parse CLI arguments with `clap`
    - initialize tracing
    - build config/device registry
    - construct the handler
    - start `UnfoldedCircleIntegrationWsHost::serve_addr(...)`

- `types.rs`
    - driver constants
    - entity/state structs
    - command enums
    - entity builders
    - command resolution helpers

- `responses.rs`
    - JSON builders for UC responses/events
    - keep this module pure and deterministic

- `error.rs`
    - integration error enum with `thiserror`
    - map internal failures to UC result codes

- `dispatch.rs`
    - transport-specific command execution
    - timeout handling
    - state fetching
    - translation from protocol results into UC attributes

- `handler.rs`
    - implement `WsHandler`
    - route incoming UC messages
    - return the correct response/event shape

### 5. Documentation

The README must include:

- what the integration controls
- entity table
- architecture summary
- how to run it as an external integration
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
