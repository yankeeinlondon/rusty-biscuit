# Claudine Dependencies

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

  The daemon's `rendezvous-daemon` dev-dependency in `claudine-cli` is no longer
  in a `cfg(unix)` target section: the daemon-spawning tests compile and run on
  Windows through `spawn_local_server`.

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
  resolution for empirical research fixtures), `serde`/`serde_json`/
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
