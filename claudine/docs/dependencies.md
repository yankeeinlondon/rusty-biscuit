# Claudine Dependencies

## Native Path Test Fixtures

- The `claudine` library uses `dunce` as a dev dependency so tests that need a
  canonical filesystem identity can remove safely reducible Windows verbatim
  prefixes without converting paths through display text.
- The `claudine` library and `claudine-cli` reach Playa's worker and queue locks
  through `test-toolkit`'s `LockedAudioSpool` fixture while device-free tests
  inspect durable queued audio. Neither crate declares `fs4` itself.
- `claudine-gen` keeps its optional `terminal-tests` dependency on
  `test-toolkit` and also declares it as a dev dependency, for the
  `test_layout` gate in its consolidated `l1` test binary. The dev entry adds
  no crate to the workspace; it brings `fs4` into the dev graph, which is
  already in `Cargo.lock`.
- The `claudine` library enables Tokio's `test-util` feature in its dev
  dependencies. It gives the delivery-tracker unit tests a paused clock, so a
  10-second drain deadline is asserted exactly and costs no wall time.
- The `claudine` library takes `proc-macro2` (with `span-locations`, as
  `claudine-cli` already does) as a dev dependency for the messaging spawn
  guard. The guard lexes `src/messaging/` instead of searching its text, so a
  spawn named in a comment or string cannot trip it, and it reports the line
  and column of a real one. Both crates are already in `Cargo.lock`.
- All three crates take `biscuit-test-harness` as a dev dependency for
  `manifest_dir!`, which resolves the crate directory at run time. A fixture
  path baked in at compile time names the *building* host's checkout, which is
  the wrong directory when a `cargo nextest archive` is executed elsewhere.
- `claudine-cli` takes `xpty` 0.3.6 as a Windows-only dev dependency. It opens
  a ConPTY pseudoconsole with no window, so a test can type Ctrl+C into a real
  console during the exit drain. `unchained-ai` and `worktree-cli` already
  build it, so it adds no crate to `Cargo.lock`.

## Audio Handoff

- Both Claudine crates enable Playa's `native-playback` feature. Linux CI
  installs `libasound2-dev` for the resulting ALSA build; Windows uses WASAPI
  and macOS uses CoreAudio without additional native packages.
- Claudine does not reimplement Playa's spool dependencies. Production audio
  handoff reaches `fs4`, `biscuit-hash`, and the private-path rules through the
  Playa dependency; test-only worker-lock ownership comes from `test-toolkit`.
  `biscuit-speaks/playa` carries the same native feature edge for TTS.

## Budget Ledger Process Identity

- `claudine-cli` depends on `sysinfo` (`0.38`, the version `sniff` already
  brings into the graph). Budget-ledger crash recovery uses it to read a
  recorded worker's process start time, and signals the PID only when that
  start time still matches, because PIDs are reused. On Windows it also
  terminates a verified survivor. See [Shared execution budgets](cli/budget.md).

## Executable Lookup

- `which` is pinned to major version `8` across both `claudine` (library) and
  `claudine-cli` so provider and tool discovery behavior stays consistent.

## Rendezvous Local IPC

See [`rendezvous/local-ipc.md`](rendezvous/local-ipc.md) for the contract these
edges exist to serve.

- `rendezvous-core` depends on `sniff` (`default-features = false`) for
  `sniff::os::current_user_id`, which qualifies the per-user default endpoint
  with the effective UID or the process token's account SID. The edge is
  one-directional by design — Sniff must never depend on Rendezvous — so it
  stays acyclic. Sniff discovers the principal; Rendezvous authorizes with it.
  Nothing else in `rendezvous-core` touches the OS: it models and resolves the
  endpoint and performs no filesystem mutation.
- `rendezvous-daemon` depends on `sniff` for the same discovery (ownership
  checks on the runtime directory, the data root, and the endpoint) and on
  `dirs` for `data_local_dir()`, which roots the default durable data directory
  at `<local-data-dir>/claudine/rendezvous`. That replaced the former
  `<tempdir>/rendezvous-data` default, which was not an ownership boundary.
- `rendezvous-daemon` declares `windows = "0.62"` under
  `[target.'cfg(windows)'.dependencies]` with exactly four features, each
  carrying a specific call:
  - `Win32_Security` — `SECURITY_ATTRIBUTES` and `PSECURITY_DESCRIPTOR`, shared
    by the named-pipe endpoint and the data root
  - `Win32_Security_Authorization` — the SDDL conversion
    (`ConvertStringSecurityDescriptorToSecurityDescriptorW`) plus
    `GetNamedSecurityInfoW`/`ConvertSidToStringSidW` for owner inspection
  - `Win32_Storage_FileSystem` — `CreateDirectoryW`, which applies the DACL at
    creation and so leaves no permissive window
  - `Win32_Foundation` — `LocalFree` for the RAII-owned descriptor

  The CLI's optional `daemon-tests` feature enables `rendezvous-daemon` on every
  target, so daemon-spawning tests compile and run on Windows through
  `spawn_local_server`. Ordinary local L1 tests leave the feature disabled to
  avoid compiling bundled DuckDB; CI and `just test-daemon` retain the
  cross-platform live-daemon contract.

- `claudine-cli`'s `terminal-tests` feature exposes its L2/L3 integration
  targets, and `claudine-gen` uses the same feature for its terminal report
  target. Required-feature declarations keep those binaries out of local L1;
  CI and the tier recipes enable them. The contract and CLI `real-tests`
  features similarly keep live-provider targets opt-in.

## Steering Routing

See [Steering Routing](topics/steering-routing.md).

- `claudine` (library) depends on `uuid` (`v4`, the version the CLI already
  uses) for random steering execution and request identifiers.
- `claudine-cli` depends on `tokio-stream` (already in the graph through
  `rendezvous-daemon`) for the owner's outbound `SteeringControl` request
  stream. Its dev-dependency on `tokio` adds `test-util` so the control link's
  retry backoff is asserted on a paused clock. The wrapper's process-start
  identity reuses the `sysinfo` edge described under Budget Ledger Process
  Identity (`cli_utils::process_start`).
- `rendezvous-client` adds `tokio-stream` as a dev-dependency for the
  steering round-trip test's owner stream.

## Lifecycle Requeue

- `claudine-cli` depends on `rendezvous-client` and `rendezvous-core` on every
  target so the lifecycle `requeue(...)` control action can append to the
  rendezvous deferred-execution session log. The call site is platform-neutral:
  it hands a `LocalEndpoint` to `rendezvous_client::connect`, which dispatches
  to `tokio::net::UnixStream` or a named pipe without a `cfg` branch at the
  caller. When the daemon is unreachable on either platform, the entry is
  durably appended to a local fallback file
  (`<config_dir>/claudine/rendezvous/deferred-queue.jsonl`, overridable via
  `CLAUDINE_RENDEZVOUS_FALLBACK_DIR`) so the prompt is never lost.
- `tonic` is a direct CLI dependency for the rendezvous RPC status type
  surfaced by that enqueue path.
- `dirs` resolves the per-user config directory for the fallback file.
- `thiserror` is used by the CLI's internal enqueue error type so the typed
  composition error can preserve a clear, source-aware failure message.

## Build Dependencies

- `rendezvous-core` compiles `proto/rendezvous.proto` at build time via
  `tonic-prost-build`, which shells out to `protoc`. To avoid requiring a
  system-installed protobuf compiler, its `build.rs` uses `protoc-bin-vendored`
  to supply a bundled `protoc` on macOS, Windows, and Linux. CI workflows also
  install `protoc` (`arduino/setup-protoc`) as a backstop.

## Provider-Catalog Generation (Phase A1)

- `claudine-catalog-types` (`claudine/catalog-types`) is a leaf crate — serde
  and `strum` only — holding the coerced catalog enums (`ModelCatalogSource`),
  the shared detection vocab (`Unit`/`Zone`/`Confidence`), and the
  `DisplayPolicy`/`EventClass` render-policy shells. Both `claudine` (library)
  and `claudine-gen` depend on it; `strum`'s variant-name introspection backs
  the generator's schema↔catalog enum-subset gate.
- `claudine-gen` (`claudine/gen`) depends on `darkmatter` (frontmatter parsing
  plus SimplifiedSchema sidecar validation), `biscuit-file` (file-reference
  resolution for empirical research fixtures), `sniff` (focused repository
  observation at the generator command boundary), `serde`/`serde_json`/
  `serde_yaml_ng`, `clap`, `thiserror`, and `regex` (generate-time
  compilation check for `match_op: regex` signal-detection records) — and
  deliberately NOT on the `claudine` library or CLI (bootstrap rule: a broken
  generated catalog must never block building the tool that regenerates it).
  `claudine-cli` shells out to the `claudine-gen` binary for
  `claudine providers generate`.

## Content Hashing

- `claudine-cli` depends on `biscuit-hash` (`xx_hash` feature only) for the
  resume session-compatibility key's content digests (system-prompt content and
  MCP config env). This is the repository's non-crypto hashing authority and the
  same hasher the `claudine` library uses for MCP catalog IDs
  (`biscuit_hash::xx_hash`), so digests are comparable across the two crates. It
  replaced an ad-hoc `std::collections::hash_map::DefaultHasher`, whose output is
  not a stable, cross-crate authority.

## Multi-Target Render Components

- `claudine` (library) depends on `renderable` (`../../renderable`) directly, in
  addition to `biscuit-terminal`. The `lib/src/render/` components implement both
  `TerminalRenderable` (re-exported by `biscuit-terminal`) and, for report-class
  components, `BrowserRenderable`, whose return types (`BrowserFragment<Ready>`,
  `HtmlPage`, `PageOptions`) and composition primitives (`BlockTag`,
  `ComposableNode`) live in `renderable` — so the crate must be a direct
  dependency rather than reached transitively through `biscuit-terminal`. This
  mirrors how `biscuit-terminal` and `darkmatter` declare the `renderable`
  path dependency.
