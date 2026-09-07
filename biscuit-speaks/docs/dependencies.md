# Biscuit Speaks Dependencies

## Playback and detached delivery

The optional `playa` feature enables both Playa's async API and
`playa/native-playback`. This is explicit because the Playa library keeps native
device playback opt-in. On Linux, the resulting `rodio`/`cpal` route links ALSA;
`biscuit-speaks/lib/Cargo.toml` therefore declares `libasound2-dev` in
`[package.metadata.ci.native]`, alongside `espeak-ng`. macOS uses CoreAudio and
Windows uses WASAPI without additional system packages.

`fs4` provides the test and detached-helper coordination locks shared with the
Playa spool. `biscuit-hash` supplies xxHash for stable content-addressed audio
cache names; the key inputs remain provider, voice, text, format, and
provider-dependent speed. `sniff` remains the authority for cross-platform
provider/executable discovery.

The `biscuit-speaks-cli` crate enables the library's `playa` feature. Its
executable installs Playa's scheduler/delegated-play entrypoints and the private
biscuit-speaks preparation entrypoint before normal CLI parsing, allowing both
worker modes to re-exec the same absolute binary without exposing internal
arguments.
