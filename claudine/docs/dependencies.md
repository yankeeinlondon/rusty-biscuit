# Claudine Dependencies

## Executable Lookup

- `which` is pinned to major version `8` across both `claudine` (library) and
  `claudine-cli` so provider and tool discovery behavior stays consistent.

## Lifecycle Requeue

- `claudine-cli` depends on `rendezvous-client` and `rendezvous-core` on every
  target so the lifecycle `requeue(...)` control action can append to the
  rendezvous deferred-execution session log. On Unix the client connects over
  the existing UDS transport; on Windows it connects over a named pipe via
  `tokio::net::windows::named_pipe::ClientOptions`. When the daemon is
  unreachable on either platform, the entry is durably appended to a local
  fallback file (`<config_dir>/claudine/rendezvous/deferred-queue.jsonl`,
  overridable via `CLAUDINE_RENDEZVOUS_FALLBACK_DIR`) so the prompt is never
  lost.
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
