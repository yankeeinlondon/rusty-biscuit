# Playa dependencies

## Native OS prerequisites

Playa's default feature set plays audio through native OS backends, which link
system audio libraries. These must be present to **build and test** the default
configuration.

| OS | System libraries | Provided by | Install |
|----|------------------|-------------|---------|
| Linux | ALSA dev headers (`libasound`) | `rodio` → `cpal` (feature `sfx-native`), and `alsa` (feature `audio-ducking-linux`) | `apt-get install libasound2-dev` |
| Linux | PulseAudio dev headers (`libpulse`) | `libpulse-binding` / `pulsectl-rs` (features `sfx-native-audio`, `audio-ducking-linux`) | `apt-get install libpulse-dev` |
| macOS | CoreAudio | `coreaudio-sys` (system framework) | none — ships with macOS |
| Windows | Windows SDK bindings | `windows` crate | none — no system package |

The Linux packages are declared once, by the `native` policy in
`.github/ci/areas.json` (`{"ubuntu-latest": ["libasound2-dev", "libpulse-dev"]}`),
and installed from there by one implementation — the root `justfile`'s
`_ensure-native-libs`, which probes each declared library with pkg-config and
installs only what is missing, translating the apt names to `dnf` / `pacman` /
`apk` equivalents on non-Debian hosts. Both consumers run that same recipe:

- CI — `just _ensure-native-libs playa` before every build, test, and lint
  command. A missing system library fails that named provisioning step, not a
  product test (D9).
- Developer hosts — `just init`, which runs it with no argument to cover every
  area's declared libraries.

### Building without native audio

The `playa` library defaults to no features, so a consumer that does not opt into
`sfx-native` / `sfx-native-audio` / `native-playback` needs no ALSA or PulseAudio
headers; playback falls back to host CLI players (e.g. `afplay`, `ffplay`, `mpv`).

`playa-cli` enables those by default, but `--no-default-features` drops them and
builds with no ALSA / PulseAudio requirement:

```sh
cargo install --path playa/cli --no-default-features
```

This is the escape hatch for environments that cannot install the system
libraries. It works because the OS-native backends are reached through the single
`sfx-native-audio` feature rather than through per-target dependency entries —
Cargo cannot gate a dependency entry on a feature, so a `[target.'cfg(…)']` entry
requesting a native backend would be unconditional.

## Rust dependencies

See `playa/lib/Cargo.toml` and `playa/cli/Cargo.toml` for the authoritative crate
list and the feature gating (`sfx-native-audio` for OS-native SFX routing on every
platform; `audio-ducking-{linux,macos,windows}` still split per OS).
