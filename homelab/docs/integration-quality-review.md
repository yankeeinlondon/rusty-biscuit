# Integration Quality Review

## Scope

This review covers:

- `homelab/eversolo-integration`
- `homelab/arcam-amp-integration`
- `homelab/unfolded-integration-helper`
- the design implications for `homelab/sony-receiver-integration`

The goal is not to restate how these integrations work today. The goal is to identify what must change for them to behave like high-quality Unfolded Circle external integrations: remote-configured, multi-device capable, multi-remote capable, and rich enough to expose meaningful device capability instead of the thinnest possible control surface.

## Review Method

This review combines:

- source inspection of the integration packages, helper crate, and relevant `homelab` client/server code
- local protocol documentation in `homelab/docs/unfolded-circle/`
- live non-destructive validation against:
    - Eversolo DMP-A8 at `192.168.20.90:9529`
    - Arcam amplifier at `192.168.20.161:50000`
- direct WebSocket probing of the current Eversolo and Arcam UC integration binaries against those real devices

## Executive Summary

The user’s central architectural criticism is correct.

All three integrations are currently designed around a process-local, startup-time device definition:

- start the integration binary
- provide `--host`
- derive one static device map
- advertise entities for that preconfigured device

That is the wrong ownership model for Unfolded Circle external integrations.

The better model is:

- start the integration server with zero required device hosts
- let the Remote drive setup
- let one running integration manage multiple physical devices
- let multiple Remotes connect to the same integration server
- persist discovered devices, validated hints, and per-remote assignments
- never force CLI startup arguments to be the only source of truth

The helper crate is not the direct cause of the current limitation, but it also does not provide any runtime for solving it. Today it is a protocol utility crate, not a multi-device integration runtime.

For Arcam specifically, the integration already supports discrete `on` and `off`; the backend is better than the current presentation suggests. The main Arcam quality problem is not missing discrete commands, it is that toggle is still exposed as a first-class feature and the whole integration remains statically configured and under-modeled.

For Eversolo specifically, the integration is materially underexposed relative to the API surface already supported by `homelab::eversolo` and `homelab-server`. It currently exposes only a power switch and a media player while omitting routing outputs, power options, brightness, VU/spectrum modes, device metadata, and several other capabilities that are already known to work.

One important nuance: against the current checkout and the live DMP-A8, I did **not** reproduce the exact “unknown” behavior shown in the screenshot when querying the integration directly over WebSocket. The current binary returned `CONNECTED`, `power=ON`, and a populated player state. That means the screenshot is likely explained by one of:

- an older binary or image
- a first-render timing/caching issue in the Remote
- a Remote/UI path that is not the same as direct `get_entity_states`
- a previously failing snapshot path that no longer fails in the current code

That does **not** invalidate the larger review. Even when the current checkout responds correctly, the Eversolo integration is still significantly under-modeled and architecturally too static.

## Live Validation Notes

### Eversolo DMP-A8

Direct HTTP validation against `192.168.20.90:9529` confirmed that the device exposes the richer capability set described by the user:

- `GET /ZidooControlCenter/getModel`
    - returned `model: "DMP-A8"`
    - returned `firmware: "v1.5.62"`
    - returned `ip: "192.168.20.90"`
    - returned `net_mac: "80:0a:80:5c:84:ac"`
    - returned `wif_mac: "02:00:00:00:00:00"`
    - returned `ableRemoteBoot: true`
- `GET /ZidooMusicControl/v2/getState`
    - returned current metadata for `"Twentieth Century Fox"`
    - returned `state: 0`
    - returned `position: 36240`
    - returned `duration: 153925`
    - returned `volumeData.currenttVolume: 160`
- `GET /ZidooMusicControl/v2/getInputAndOutputList`
    - returned 10 inputs
    - returned 6 outputs
    - included current input and output indices
- `GET /ZidooMusicControl/v2/getPowerOption`
    - returned `poweroff`
    - returned `reboot`
    - returned `screen`
    - returned `timeshutdown`
- `GET /SystemSettings/displaySettings/getScreenBrightness`
    - returned `currentValue: 52`
- `GET /SystemSettings/displaySettings/getKnobBrightness`
    - returned `currentValue: 40`
- `GET /SystemSettings/displaySettings/getVUModeList`
    - returned 14 VU modes
- `GET /SystemSettings/displaySettings/getSpPlayModeList`
    - returned 4 spectrum modes

I also ran the current `eversolo-integration` binary locally against that device and queried it over WebSocket. The current checkout responded with:

- `device_state: CONNECTED`
- `eversolo.streamer.power.state: ON`
- `eversolo.streamer.player.state: ON`
- populated media title / artist / album / duration / position / volume / source

So the current package is not dead on the wire. The larger problem is that it models far too little of what the device can do.

### Arcam Amplifier

Direct non-destructive TCP validation against `192.168.20.161:50000` confirmed the expected protocol behavior:

- power query returned a valid response frame indicating standby/off
- mute query returned a valid response frame indicating mute enabled
- system status query returned:
    - model `PA240`
    - friendly name
    - IP address `192.168.20.161`
    - amplifier mode byte
    - auto-shutdown and timeout fields

I also ran the current `arcam-amp-integration` binary locally against that device and queried it over WebSocket. The current checkout responded with:

- `device_state: CONNECTED`
- `arcam.amp.power.state: OFF`
- `arcam.amp.mute.state: ON`

This confirms that the current integration does support discrete state reporting and that the transport path is functioning.

## Cross-Cutting Design Review

## 1. Startup-Time Host Ownership Is The Wrong Design

All three integrations currently require a host at startup:

- `eversolo-integration --host ...`
- `arcam-amp-integration --host ...`
- `sony-receiver-integration --host ...`

That forces the operator to decide device identity before any Remote connects, which creates several quality problems:

- a single integration process can only represent the devices declared at startup
- the Remote cannot own configuration of the devices it wants to use
- the server cannot start in a neutral “ready to be configured” state
- the model does not scale naturally to multiple Remotes
- the model encourages one-process-per-device instead of one-process-per-category

This is opposite of the architecture that best fits the Unfolded Circle integration protocol.

### Required direction

The integration binaries should start without any required device hosts.

At minimum:

- `--host` must become optional
- optional CLI/device-file data should be treated as seed hints, not as the only configuration path
- the running server should be able to hold zero configured devices and still answer metadata/setup requests
- the Remote should be able to add devices later without restarting the process

## 2. The Correct Ownership Boundary Is “Remote Configures Device Instances”

The integration server should own:

- protocol handling
- device discovery / validation
- persistent registry of known device candidates
- persistent registry of configured device instances
- runtime polling / state fanout

The Remote should own:

- which devices a particular Remote wants to configure
- user-facing naming / selection
- deduplication rules for that Remote

### Recommended model

Use three distinct concepts:

1. `KnownDevice`
   - something the server has discovered, validated, or been told about
   - not necessarily assigned to any Remote

2. `ConfiguredDevice`
   - a concrete device instance the server can poll and control
   - identified by stable device identity plus transport metadata

3. `RemoteAssignment`
   - a binding between a Remote and one or more configured device instances
   - prevents duplicate assignment of the same physical device more than once for the same Remote

This makes the rules clear:

- the same physical device may be available to multiple Remotes if desired
- the same physical device must not be offered twice to the same Remote once already assigned
- entity IDs should be derived from configured device instance identity, not from ephemeral discovery order

## 3. `setup_data_schema` And Setup Flow Need To Be Implemented

The current integrations handle:

- `get_driver_version`
- `get_driver_metadata`
- `get_device_state`
- `get_available_entities`
- `subscribe_events`
- `get_entity_states`
- `entity_command`

They do **not** implement a real driver setup flow.

That is the missing protocol layer that would allow the Remote to configure devices instead of the process requiring `--host`.

### Required additions

The new architecture should use the Unfolded Circle setup flow to support:

- initial empty-state setup
- discovered device list presentation
- manual address entry
- validation of user-entered addresses
- confirmation of selected device identity
- optional naming and grouping
- completion of setup without restarting the integration

### Practical recommendation

Treat setup in two phases:

1. Seed candidates
   - return a screen showing:
     - discovered devices
     - previously validated hints
     - manual entry option

2. Confirm and bind
   - validate the selected address
   - resolve identity and metadata
   - create the configured device instance
   - persist the assignment

## 4. Discovery And Hinting Should Be Best-Effort, Not Mandatory

The user’s discovery/hinting direction is correct, but it needs a few guardrails.

### Good design principles

- discovery must never block startup of the server
- discovery must never prevent manual entry
- hints should be validated, not blindly trusted
- once hints exist, prefer shortlist validation over blind subnet-wide scanning
- scans should be time-bounded and cancellable

### Important network caveat

Do **not** assume “same subnet as the integration server” is always the right discovery boundary.

This review was performed from a host that is not on the same subnet as the Eversolo and Arcam, yet direct communication still worked. In real deployments, devices may sit behind routed home networks, VLANs, or other non-flat topologies.

So discovery should support:

- explicit CIDRs
- explicit host hints
- previously known device addresses
- optional local-subnet scan as a convenience, not an assumption

### Implementation suggestion

Do not make `rustscan` a hard runtime dependency unless you specifically want it for operator tooling.

For the integration runtime itself, a bounded async TCP probe plus identity verification is usually enough:

- Eversolo:
    - probe TCP `9529`
    - verify with `GET /ZidooControlCenter/getModel`
- Arcam:
    - probe TCP `50000`
    - verify with power query or system-status query
- Sony:
    - probe TCP `10000` and/or HTTP on `80`
    - verify with JSON-RPC and native web status

This keeps discovery deterministic and device-aware.

## 5. The Helper Crate Needs To Evolve Into A Runtime, Not Just Protocol Glue

`unfolded-integration-helper` is currently useful but intentionally narrow. It provides:

- request envelope parsing
- response/event builders
- state caching
- connectivity rollup
- subscription fanout
- optional mDNS advertisement for discovering the integration server itself

It does **not** provide:

- setup-data schema helpers
- setup flow orchestration
- discovered-device registry
- configured-device registry
- per-remote assignment registry
- duplicate suppression
- device hint validation
- capability/resource abstractions
- remote-scoped subscriptions

### Required helper evolution

To support the desired architecture, the helper should gain:

- a persistent registry abstraction for `KnownDevice`, `ConfiguredDevice`, and `RemoteAssignment`
- setup-flow helpers for `setup_data_schema`, setup progress, user-data submission, and completion
- discovery adapter traits
- hint validation helpers
- richer capability types for:
    - enumerations
    - numeric ranges/sliders
    - read-only metadata resources
    - action lists
- remote/device scoped subscription filtering

Without this, each integration will keep re-implementing the same runtime behaviors.

## 6. Aggregate Connectivity Is Too Coarse For Multi-Device Servers

The current helper rollup reports the integration as `CONNECTED` if any tracked device is connected.

That is acceptable for today’s one-device-per-process assumption. It is not sufficient for the architecture this review recommends.

Once a single process manages multiple devices, the integration needs:

- per-device connectivity
- per-remote configured-device health
- an aggregate integration state that does not hide partial failure

Recommended policy:

- the global integration can remain `CONNECTED` if at least one device is healthy
- but the setup/config UI must still expose per-device health clearly
- entity/state/event fanout should be device-scoped, not only integration-scoped

## Arcam Amplifier Integration Review

## Current Strengths

- transport path works against the live PA240
- the integration already supports discrete `on` and `off` for both power and mute
- state polling and UC state reporting work
- the backend is more deterministic than the current UI presentation implies

## Current Weaknesses

## 1. Static Startup Configuration

The same host-at-startup flaw applies here exactly.

Today the binary builds one device map from one required `--host`. That is the wrong ownership model and prevents:

- runtime device enrollment
- multi-device management
- remote-driven setup
- per-remote duplicate suppression

## 2. Toggle Is Over-Promoted

The Arcam integration does not lack discrete commands. It already implements:

- `power on`
- `power off`
- `mute on`
- `mute off`

However, it also advertises `toggle` as a first-class switch feature for both entities.

That is a quality problem because:

- toggle is nondeterministic from an automation author’s perspective
- the implementation is read-then-invert, not atomic
- it creates a TOCTOU race if state changes between the read and the write

### Required direction

Keep discrete `on` and `off` as the primary contract.

Options:

- best option: stop advertising toggle unless the UC UI truly requires it
- acceptable fallback: keep toggle available as a convenience action, but do not center the UI/feature model around it

## 3. The Entity Model Is Too Thin

The current Arcam model exposes only:

- `power`
- `mute`

That is serviceable, but thin.

The live amplifier and existing `homelab` support can also surface:

- system model
- friendly name
- IP address
- amplifier mode
- auto-shutdown setting
- timeout counter
- temperatures

Not all of that should necessarily become first-class UC entities. But a high-quality integration should at least consider:

- a read-only information resource for amplifier identity and model
- a read-only status resource for mode / auto-shutdown / timeout
- optional sensors if the UC UX supports them cleanly

### Priority

This is secondary to fixing setup architecture and discrete-command presentation.

## Arcam Recommendations

### Must do

- remove required startup host ownership
- add remote-driven setup
- support multiple configured Arcam devices in one process
- suppress duplicate assignment per Remote
- keep discrete power/mute semantics primary

### Should do

- demote or hide toggle
- expose minimal identity/status metadata as read-only resources
- preserve system-status probing so the user can validate they bound the correct amplifier

## Eversolo Integration Review

## Current Strengths

- transport path works against the live DMP-A8
- current checkout can answer healthy UC state over WebSocket
- power, transport, mute, volume, and input selection are implemented
- the integration already does some dynamic source-list discovery

## Current Weaknesses

## 1. Static Startup Configuration

Same root flaw as Arcam:

- required `--host`
- optional `--mac`
- one static device map at startup
- no runtime device enrollment
- no remote-driven setup

## 2. The Integration Exposes Far Too Little Of The Known API Surface

This is the largest quality problem.

The live device and `homelab::eversolo` already support:

- device info
- input routing
- output routing
- power options
- screen brightness
- knob brightness
- VU mode selection
- spectrum mode selection
- remote key sending
- text input
- seek

The current UC integration exposes only:

- one power switch
- one media player
- player input selection through `source_list`

That is not a high-quality Eversolo integration. It is a minimal proof-of-connectivity.

### Missing capability groups that should be exposed

- routing input selector
- routing output selector
- power-options action list
- display brightness controls
- VU mode selector
- spectrum mode selector
- device information resource
- wake identity resource derived from model info

## 3. WOL / MAC Handling Is Backwards

The current integration can accept `--mac`, but the live DMP-A8 already exposes the needed MAC information via `getModel`.

The current live response included:

- `net_mac: 80:0a:80:5c:84:ac`
- `wif_mac: 02:00:00:00:00:00`
- `ableRemoteBoot: true`

### Required direction

Do not require the user to know or enter the MAC address manually in the normal path.

Instead:

- user provides host or selects discovered device
- integration validates with `getModel`
- integration stores:
    - host
    - model
    - firmware
    - `net_mac`
    - `wif_mac`
    - inferred active transport
    - `ableRemoteBoot`
- WOL configuration is then derived automatically

Manual MAC entry can remain only as an expert override.

## 4. Snapshot Refresh Is Too All-Or-Nothing

The current refresh path treats routing as part of the same success boundary as player and power state.

That means one routing-related failure can suppress otherwise-good playback/power information.

### Required direction

Split refresh into partial domains:

- identity / model
- playback / transport state
- volume / mute
- routing
- display settings

Then degrade each domain independently instead of collapsing to a single failure mode.

## 5. Player State Mapping Is Too Coarse

The live DMP-A8 returned:

- `state: 0`
- current track metadata present
- current position present

The current UC mapping only treats:

- `1` as `PLAYING`
- `2` as `PAUSED`
- everything else as generic `ON` unless inferred standby is active

That is not strong enough.

### Required direction

Research and codify the meaning of Eversolo playback state values more accurately.

At minimum:

- treat `state: 0` as an explicitly modeled non-playing state instead of a generic catch-all
- keep track metadata separate from transport state
- avoid collapsing meaningful states into plain `ON`

## 6. The Current Model Hides Output Routing

The live DMP-A8 exposes current output and available outputs. That matters operationally.

Examples from the live device:

- `BAL-XLR`
- `Analog-RCA`
- `XLR/RCA`
- `IIS`
- `OPT/COAX`
- `USB DAC` with enable state

The current integration exposes only source selection. That is incomplete.

### Required direction

Expose both:

- input source selection
- output routing selection

The output selector should respect enabled/disabled state.

## 7. Brightness And Display Modes Should Be First-Class

The live device exposes:

- screen brightness
- knob brightness
- 14 VU modes
- 4 spectrum modes

These are not edge features. They are part of the device’s user-facing operating experience and are exactly the kind of thing a premium remote integration should surface.

### Required direction

Expose:

- numeric brightness controls as ranges/sliders
- VU mode selection as an enumeration
- spectrum mode selection as an enumeration

If the UC entity model does not support a clean first-class control for these today, then expose them as explicit resources/actions rather than omitting them.

## 8. Power Options Should Be Surfaced Explicitly

The live device returned these power options:

- `poweroff`
- `reboot`
- `screen`
- `timeshutdown`

These should not be hidden behind an internal command path. They should be surfaced explicitly as device actions.

This is especially important because the current integration currently overloads “power” into an inferred UX concept rather than exposing the device’s actual power/control surface.

## 9. Volume Slider Support Needs Proper Range Metadata And Debounce

The Eversolo returns:

- `minVolume`
- `maxVolume`
- current volume
- mute state
- `isVolumeEnable`

The current integration already uses `volume_steps`, but a production-quality version should also:

- expose the real range from the device
- guard against rapid-fire slider spam
- debounce or coalesce repeated `volume_set` calls
- avoid unnecessary state fetches during active slider movement

This matters even if the Remote also debounces, because the integration should not assume perfect client behavior.

## Eversolo Recommendations

### Must do

- remove required startup host ownership
- implement remote-driven setup
- auto-resolve MAC and wake metadata from `getModel`
- support multiple configured Eversolo devices in one process
- split refresh into partial domains
- expose routing input and output controls
- expose power options
- expose brightness controls
- expose VU and spectrum mode controls

### Should do

- improve playback state mapping
- add per-control debounce/coalescing for volume
- expose read-only device metadata
- preserve manual address entry and expert overrides

### Nice to have

- remote key actions and text input, if they fit the UC UX cleanly
- optional display-off / display-on actions if distinct from brightness and power options

## Sony Receiver Implications

This review did not focus on live Sony validation, but the same primary architectural flaw already exists there:

- required `--host`
- startup-time device ownership
- static entity construction

So Sony should be migrated into the same runtime model as Arcam and Eversolo.

### Sony-specific recommendation

Do not redesign Sony separately.

Build the new helper/runtime once, then port:

1. Arcam
2. Eversolo
3. Sony

That avoids three slightly different setup/discovery/persistence stacks.

## Recommended Runtime Design

## Process Model

One process per device category, not one process per physical device.

Examples:

- one `eversolo-integration` process manages many Eversolo devices
- one `arcam-amp-integration` process manages many Arcam amplifiers
- one `sony-receiver-integration` process manages many Sony receivers

## Persistence Model

Persist:

- known devices
- validated hints
- configured device instances
- remote assignments
- cached identity metadata

Do not persist only raw entity state.

## Discovery Model

At startup:

- load persisted known devices and hints
- begin background validation / discovery
- do not block WebSocket serving

During setup:

- show validated known devices first
- show newly discovered candidates second
- always allow manual entry

## Assignment Rules

For each Remote:

- do not offer a device already assigned to that same Remote
- allow re-binding only through an explicit reconfigure/remove flow

Across Remotes:

- allow the same physical device to be assigned to multiple Remotes unless a later product decision says otherwise

## Capability Model

Standardize shared helper abstractions for:

- `SelectList`
- `RangeControl`
- `ActionList`
- `InfoResource`
- `DeviceIdentity`

This will let Eversolo, Sony, and future integrations expose richer capability without reinventing raw JSON shapes each time.

## Suggested Implementation Phases

## Phase 1: Helper Runtime Foundation

- add setup-flow helpers
- add persistent registries
- add discovery/hint abstractions
- add remote assignment model
- add richer capability abstractions

## Phase 2: Arcam Retrofit

- move to remote-driven setup
- multi-device support
- per-remote dedupe
- discrete-first UI contract

## Phase 3: Eversolo Retrofit

- move to remote-driven setup
- auto-resolve MAC/identity
- partial refresh refactor
- capability expansion:
    - routing
    - power actions
    - brightness
    - display modes
    - richer metadata

## Phase 4: Sony Retrofit

- migrate into the same setup/runtime model
- align source/resource exposure with the richer helper abstractions

## Additional Quality Note: Real-Device Test Recipes

The checked-in `just sanity-test` recipes for Eversolo and Arcam currently invoke test filters that also match the destructive test names, which makes the recipe fail unless the destructive environment variable is set.

That is not a device-integration design flaw, but it is a quality issue in the validation workflow and should be corrected separately.

## Final Assessment

### Arcam

The current Arcam integration is functionally alive and already supports discrete operations, but it is architecturally too static and slightly mispresented because toggle remains over-promoted.

### Eversolo

The current Eversolo integration is functionally alive in the current checkout, but it is significantly under-modeled relative to the real DMP-A8 API surface and therefore does not yet qualify as a high-quality integration.

### Helper

The helper crate is adequate as protocol glue but insufficient as the runtime foundation needed for the architecture these integrations should evolve toward.

### Bottom line

The next major step should not be “add a couple more Eversolo commands.” It should be:

1. build the remote-driven, multi-device runtime once
2. retrofit Arcam and Eversolo onto it
3. port Sony immediately after

That is the highest-leverage path to better integrations across the entire homelab Unfolded Circle stack.
