# Audio Ducking Design

This document turns the discovery notes into a concrete, per-OS plan for Playa’s `audio-ducking` feature (feature flag name: `audio-ducking`). The feature is compiled into the CLI and conditionally includes only the code needed for the host OS. Default ducking envelope: 1_000 ms ramp down, normal playback volume for Playa, 1_000 ms ramp up to restore prior volumes.

## Goals

- Gently attenuate other system audio before Playa plays, avoiding surprises (no hard mute).
- Run Playa playback at the requested volume while other audio stays ducked.
- Restore every affected stream/device to its exact prior volume, even if Playa errors.
- Keep ducking optional, lightweight, and guarded by a feature flag and CLI toggle.

## Scope and non-goals

- Scope: macOS, Windows, Linux (PulseAudio/PipeWire). Best-effort global ducking on ALSA-only systems. No mobile platforms.
- Non-goals: precise per-app pause/resume, advanced media session coordination, or long-running audio activity detection. Ducking is bounded to the lifetime of a single Playa playback request.

## Feature flags and build matrix

- Core flag: `audio-ducking` (enabled in the CLI build). When off, the ducking code is not compiled or linked.
- OS slices (activated automatically via `cfg(target_os)` when `audio-ducking` is set):
    - `audio-ducking-macos`: CoreAudio virtual master volume path.
    - `audio-ducking-windows`: WASAPI session/endpoint volume path.
    - `audio-ducking-pulse`: PulseAudio/PipeWire path; ALSA global fallback.
- Library surface stays consistent; unimplemented platforms return a no-op ducking guard.

## Library surface

- `DuckConfig { ramp_ms: u32, floor_scalar: f32 }` with defaults `ramp_ms = 1000`, `floor_scalar = 0.2`.
- `async fn with_ducked_audio<F, Fut>(cfg: DuckConfig, f: F) -> Result<()> where F: FnOnce() -> Fut, Fut: Future<Output = Result<()>>`:
  1) Snapshot current volumes (per device/session/stream where available).
  2) Apply a fade from current volume to `floor_scalar` over `ramp_ms`.
  3) Run `f()` (Playa playback) while holding duck.
  4) Fade back to the snapshot over `ramp_ms`, even if `f()` errors. If restoration fails mid-way, continue best-effort and report.
- Internal helpers per backend (not exposed): `snapshot()`, `fade_to(floor)`, `fade_restore()`, `watchdog_on_drop`.
- Error policy: failures to duck should not block playback; return a warning-style error type but proceed with audio.

## CLI behavior

- Ducking on by default when compiled with `audio-ducking`. Flag: `--no-duck` to disable; `--duck-ramp-ms <u32>` and `--duck-floor <0.0–1.0>` to override defaults.
- If the backend is unavailable or errors, log a single warning and play normally.

## Cross-cutting implementation notes

- Time-based fades use small step intervals (e.g., 10–20 ms) to minimize zipper noise. Compute step count from `ramp_ms` and clamp minimum 3 steps.
- Store snapshots as scalar volumes (0.0–1.0). For per-channel data, keep the highest fidelity the backend provides and restore exactly.
- Identify Playa’s own session/stream and exclude it from ducking (Windows: exclude matching process ID; PulseAudio: match `application.name` / `media.name`; macOS virtual master ducking is endpoint-wide and inherently includes Playa, which is acceptable for simplicity but we can optionally skip attenuation once we know Playa’s volume path).
- Add a watchdog in `Drop` to restore volumes if the guard is dropped early due to panic or cancellation.
- Respect `NO_COLOR`/quiet modes by aligning logging with existing Playa logging controls.

## Backend specifics

### macOS (CoreAudio)

- Crates: `coreaudio` or `coreaudio-sys` (prefer safe wrappers if available). No new runtime daemons.
- Approach: endpoint-wide ducking via `kAudioHardwareServiceDeviceProperty_VirtualMasterVolume` on the default output device.
- Steps:
  1) Resolve default output `AudioObjectID`.
  2) Snapshot current virtual master volume (and balance if present).
  3) Fade volume scalar to `floor_scalar` with `AudioObjectSetPropertyData` updates.
  4) Restore with the same ramp.
- Limitations: affects Playa’s own output too; acceptable trade-off for simplicity. If future per-app exclusion is needed, reconsider MediaRemote/NowPlaying (currently out of scope).

### Windows (WASAPI)

- Crates: `wasapi` (primary). Reference `winmix` for per-process handling; avoid adding a new dependency if not necessary.
- Approach: per-session ducking via `ISimpleAudioVolume` for each active session on the default render device, excluding Playa’s session. Endpoint-wide ducking via `IAudioEndpointVolume` as a fallback if session access fails.
- Steps:
  1) `IMMDeviceEnumerator` → default render `IMMDevice`.
  2) Activate `IAudioSessionManager2`; enumerate sessions.
  3) Snapshot `GetMasterVolume` per session (and mute state). Identify Playa’s session by process ID and skip it.
  4) Fade each session to `floor_scalar`; coarse fallback: fade endpoint scalar.
  5) Restore with the same ramp.
- Limitations: Some apps may opt out of session control; endpoint fallback covers most cases.

### Linux (PulseAudio / PipeWire)

- Crates: `pulsectl` (preferred) with a potential `libpulse-binding` fallback. PipeWire commonly exposes a PulseAudio server, so the same calls usually work.
- Approach: per-sink-input ducking for all active sink inputs except Playa’s. Global sink ducking as fallback. ALSA-only: adjust the master PCM control if available.
- Steps:
  1) Connect via pulsectl; enumerate sink inputs. Identify Playa by `application.name`/process ID and skip it.
  2) Snapshot per sink input volume (per-channel). Active means state `Running` or uncorked.
  3) Fade each sink input to `floor_scalar`; fallback: fade the sink volume.
  4) Restore per input; fallback restore sink volume.
  5) ALSA-only: read current mixer control, fade master scalar, restore.
- Limitations: ALSA fallback is coarse and may not be present; treat as best-effort with warnings.

## Data structures (per backend)

- `struct VolumeSnapshot { entries: Vec<SessionVolume> }`
- `struct SessionVolume { id: SessionId, channels: Vec<f32>, mute: bool }`
- `enum SessionId { MacEndpoint, WasapiSession { pid: u32, key: String }, PulseSinkInput { index: u32, name: String }, AlsaMaster }`

## Failure handling and restoration guarantees

- Duck failures downgrade to “play without ducking” and return a warning.
- Restoration runs in `Drop` of the guard plus an explicit `restore` path; double-restore is harmless.
- If partial restoration occurs (e.g., some sessions disappear), log and proceed.

## Testing strategy

- Unit tests: envelope math (step calculation, clamping, ramp correctness), snapshot/restore data transforms.
- Integration (gated per OS, ignored on CI where unavailable):
    - macOS: mock `AudioObject` via a thin shim or feature-flagged test harness.
    - Windows: use WASAPI loopback/device in a test VM; assert Set/Get round-trips on a fake session.
    - Linux: use pulsectl against a local PulseAudio/PipeWire server; create a synthetic sink input and verify fade/restore.
- Safety: ensure Drop-based restore triggers on panic in `with_ducked_audio` body.

## Open questions / future work

- Whether to skip ducking Playa’s own output on macOS by mixing via a per-app path (requires different APIs; currently out of scope).
- Whether to surface detection (`detect_active_audio`) separately; for now, ducking runs unconditionally when enabled.
- Add a max-duck duration guard (e.g., auto-restore if playback stalls beyond N seconds).
