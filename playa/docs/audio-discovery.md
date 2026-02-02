# Audio Environment Discovery

Goals specific to Playa:

- Detect when the system is already playing audio so we can avoid double‑mixing or choose a lower‑impact effect path.
- Optionally pause or duck (lower volume) existing playback while Playa injects its own audio, then restore the prior state.

## Cross‑platform takeaways

- There is no single cross‑platform crate that exposes “is the system playing audio?” or “duck all other apps.” We need per‑OS backends and conditional compilation.
- Our playback stack already depends on host players; the discovery/ducking layer should stay lightweight, use system APIs directly, and make best effort without blocking playback.

## macOS (CoreAudio)

- Detect active playback: query the default output device’s `kAudioDevicePropertyDeviceIsRunningSomewhere` via `AudioObjectGetPropertyData`. If true, some client has an IOProc running (i.e., audio is active).
- Volume/ducking:
    - System‑wide: set `kAudioHardwareServiceDeviceProperty_VirtualMasterVolume` (or per‑channel `VirtualMasterBalance`) on the default output device to apply a temporary attenuation; restore afterward.
    - Per‑app pause is not exposed through CoreAudio. MediaRemote/NowPlaying APIs can control media sessions for music apps, but they are private/unsupported for general use and inconsistent across apps.
- Crates to use:
    - `coreaudio` (RustAudio) or `coreaudio-sys` for the raw bindings; `objc2-core-audio` if we need newer symbols in a safer wrapper.
- Implementation sketch:
  1) Resolve default output `AudioObjectID`.
  2) Read `kAudioDevicePropertyDeviceIsRunningSomewhere` → bool.
  3) For ducking, read and cache current virtual master volume, set a reduced scalar (e.g., 0.35–0.5), then restore.

## Windows (WASAPI)

- Detect active playback: use `IAudioSessionManager2` → `IAudioSessionEnumerator` → per session `AudioSessionState` and `IAudioMeterInformation::GetPeakValue`. Any session in `AudioSessionStateActive` with peak > ~0.01 implies current audio.
- Volume/ducking options:
    - Per‑session: `ISimpleAudioVolume::SetMasterVolume` or `SetMute` on each active session except Playa’s own session.
    - Endpoint‑wide: `IAudioEndpointVolume` to lower the default render device (simpler but affects the whole system).
- Crates to use:
    - `wasapi` crate for safe access to device + session managers; includes examples for listing devices and processes.
    - `winmix` crate (wraps `windows` bindings) if we want a ready‑made per‑process volume control helper (enumerate sessions, set mute/volume). Useful as a reference even if we roll our own to avoid unsafe exposure.
- Implementation sketch:
  1) Create `DeviceEnumerator` → default render `IMMDevice`.
  2) Activate `IAudioSessionManager2`, enumerate sessions.
  3) For detection, read `SessionState` + `IAudioMeterInformation` peak.
  4) For ducking, call `ISimpleAudioVolume::SetMasterVolume` on non‑Playa sessions; cache and restore values. For coarse ducking, adjust `IAudioEndpointVolume` scalar.

## Linux (PulseAudio / PipeWire)

- Detection (PulseAudio): subscribe to sink input events (`pa_context_subscribe` with `SUBSCRIPTION_MASK_SINK_INPUT`). Inspect `SinkInputInfo.state` (`PA_STREAM_RUNNING` or non‑zero `corked` flag) and `volume` to detect active playback.
- Detection (PipeWire): enumerate nodes via the registry; check `PW_KEY_MEDIA_ROLE`, `PW_KEY_NODE_STATE`, and monitor param events. PipeWire commonly exposes a PulseAudio compatibility layer, so using PulseAudio APIs is the simplest path for now.
- Volume/ducking:
    - PulseAudio: `pa_context_set_sink_input_volume` to attenuate specific sink inputs; `pa_context_set_sink_input_mute` to mute. For global ducking, adjust the sink volume (`pa_context_set_sink_volume_by_index/name`).
    - PipeWire native: set `SPA_PARAM_Props` on nodes (e.g., `volume`, `mute`) via `pw_node_set_param`, or use the PulseAudio compatibility layer to avoid PipeWire‑specific plumbing.
- Crates to consider:
    - `pulsectl` / `pulsectl-rs`: higher‑level, synchronous control of sinks and sink inputs (volumes, mute, subscribe to events) with less boilerplate than `libpulse`.
    - `libpulse-binding`: full PulseAudio API (async + threaded mainloop) if we need finer control or monitoring.
    - `pipewire` crate: safe bindings to libpipewire if we choose native PipeWire control instead of the PulseAudio shim.
- Implementation sketch (PulseAudio path):
  1) Connect to server; subscribe to sink input events.
  2) Enumerate sink inputs, treat any with state `Running` as active audio.
  3) On duck request, scale volumes for non‑Playa sink inputs (store per‑input volume, apply factor, restore later). If PipeWire is present, the same calls work via its PulseAudio server.

## Other notes and gaps

- ALSA‑only systems: limited visibility into other apps; mixer controls (`alsa` crate) can change the hardware PCM/MASTER volume but cannot reliably enumerate per‑app streams. Treat as best‑effort global ducking.
- Privacy/UX: muting other apps can be surprising. Default to gentle ducking with a short timeout and restoration guard in case Playa crashes before restoring.
- Failure strategy: discovery/ducking must be optional; if system APIs are unavailable, fall back to normal playback.

## Suggested approach for Playa

1) Add a feature‑gated “coordination” module with per‑OS backends:
   - macOS: `coreaudio` backend (running flag + virtual master duck).
   - Windows: `wasapi` backend (session enumerate + per‑session or endpoint duck). Consider `winmix` as reference code.
   - Linux: PulseAudio‑first backend using `pulsectl` (simpler) with a `libpulse-binding` fallback; PipeWire piggybacks via its PA server.
2) Expose two async hooks in the library/CLI:
   - `detect_active_audio()` → enum { None, Light (non‑zero peak but low), Active }.
   - `with_ducked_audio(async fn)` that applies attenuation, runs Playa playback, and restores, with a watchdog to restore on error.
3) Keep dependencies optional via cargo features (`coordination-macos`, `coordination-windows`, `coordination-pulse`, `coordination-pipewire`).
4) In the CLI, add a `--duck` flag defaulting to on when effects are short; allow `--no-duck` to disable system changes.
