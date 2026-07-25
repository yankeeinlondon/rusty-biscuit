# Playa dependencies

## Native OS prerequisites

Playa's default feature set plays audio through native OS backends, which link
system audio libraries. These must be present to **build and test** the default
configuration.

| OS | System libraries | Provided by | Install |
|----|------------------|-------------|---------|
| Linux | ALSA dev headers (`libasound`) | `rodio` → `cpal` (feature `sfx-native`), and `alsa` (feature `audio-ducking-linux`) | `apt-get install libasound2-dev` |
| Linux | PulseAudio dev headers (`libpulse`) | `libpulse-binding` / `pulsectl-rs` (features `sfx-native-linux`, `audio-ducking-linux`) | `apt-get install libpulse-dev` |
| macOS | CoreAudio | `coreaudio-sys` (system framework) | none — ships with macOS |
| Windows | Windows SDK bindings | `windows` crate | none — no system package |

CI installs the Linux packages via the `native` policy in
`.github/ci/areas.json` (`{"ubuntu-latest": ["libasound2-dev", "libpulse-dev"]}`),
provisioned by the shared `install-native` action before build/test. A missing
system library fails that named provisioning step, not a product test (D9).

### Building without native audio

The CLI defaults to `sfx-native` + `native-playback`. Build with
`--no-default-features` to drop the native audio backends entirely; playback then
falls back to host CLI players (e.g. `afplay`, `ffplay`, `mpv`) and no ALSA /
PulseAudio headers are required. This is the escape hatch for environments that
cannot install the system libraries.

## Rust dependencies

See `playa/lib/Cargo.toml` and `playa/cli/Cargo.toml` for the authoritative crate
list and the per-OS feature gating (`sfx-native-{linux,macos,windows}`,
`audio-ducking-{linux,macos,windows}`).
