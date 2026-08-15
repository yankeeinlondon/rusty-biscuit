# Darkmatter Dependencies

## Remote URL Referencing

- `biscuit-file` with the `fetch` feature supplies the shared HTTP fetch
  primitive and host allowlist policy used by compose remote reads and
  side-effect network writes.
- `reqwest` provides the shared HTTP client for compose remote fetches and the
  side-effect `http_post` verb.
- `tokio` runs remote fetch tasks and the blocking wrapper used by synchronous
  callers.
- `url` parses and normalizes HTTP(S) references before policy checks.

These dependencies are required so every network egress path goes through the
same scheme validation and deny-all-by-default host policy.

## Compose Text Replacement

- `aho-corasick` powers the `replace:` map matcher in
  `markdown/compose/replacement.rs`: a `MatchKind::LeftmostLongest` automaton
  gives a single linear pass with the same leftmost / non-overlapping /
  longest-key-wins / no-rescan contract the previous hand-rolled per-character
  scanner had, but without its `O(content × rules × keylen)` cost. Already
  compiled transitively via `regex`, so it adds no build cost.

## Shell Directive Execution

- `shared_child` backs the child-process wait in
  `markdown/compose/shell_expansion/executor.rs`. Both executor variants block
  on an OS wait event (a helper thread calls `SharedChild::wait`, the caller
  consumes it with `recv_timeout`) instead of the previous 10 ms
  `try_wait`/`sleep` poll loop, so a fast command's completion is observed
  immediately rather than up to one poll interval later. `SharedChild` is what
  makes the timeout arm safe: it can kill and reap through the same handle the
  waiter thread is blocked on, which `std::process::Child` cannot express.
  Depended on with `default-features = false` — the default `timeout` feature's
  `wait_timeout` is built on a process-wide SIGCHLD handler (pulling
  `sigchld`/`signal-hook`), and Darkmatter is a library that must not hijack a
  host application's signal disposition. The ungated `wait`/`kill` core we use
  is `waitid`/`WaitForSingleObject`-based and installs nothing, so Darkmatter
  carries no platform split of its own.
- `libc` (Unix only) supplies `setsid` for the alias-lookup child in
  `markdown/compose/shell_expansion/alias.rs`. That lookup must run the shell
  interactively to see rc-file aliases, and an interactive shell left on the
  caller's controlling terminal stops itself with `SIGTTIN` whenever it is not
  the terminal's foreground process group — a hang, not an error. A new session
  is the only thing that detaches the controlling terminal; a new process group
  alone does not. One syscall, so the raw binding rather than a wrapper crate.

## DMLS (`darkmatter/dmls`)

The Darkmatter Language Server keeps its dependency surface small and
protocol-focused:

- `lsp-server` — stdio framing, the synchronous `Connection` loop, and
  `Connection::memory()` for in-memory L2 session tests.
- `lsp-types` — LSP 3.17 protocol types (0.97; its `Uri` carries no
  file-path conversion).
- `crossbeam-channel` — the channel family `lsp-server`'s `Connection`
  exposes; also the worker-pool channel from Phase 3 on.
- `line-index` — rust-analyzer's line/offset index behind the `source_map`
  module; its types never leak past that module.
- `rlsp-yaml-parser` — lossless, byte-span YAML 1.2 parse behind the Phase-7
  `overlay::frontmatter::FrontmatterAst` facade (AD-4 / R-3); its types never
  leak past that module.
- `url` — battle-tested `file://` URI ↔ path conversion (percent decoding,
  Windows drive letters) that `lsp-types` 0.97 lacks.
- `toml` — `.dmls.toml` config parsing.
- `serde` / `serde_json` — LSP payloads and the config overlay merge.
- `thiserror` — error types.
- `tracing` / `tracing-subscriber` — logging to stderr or `--log-file`
  only; stdout is reserved for LSP framing.
- `ignore` — gitignore-aware workspace discovery walk (Phase 3); symlinks
  are not followed (the R-8 v1 policy).
- `globset` — include/exclude glob matching over discovered paths (Phase 3).
- `unicode-normalization` — NFC normalization of wiki-link targets and logical
  paths (Phase 5 / R-8), so a vault resolves identically on macOS, Windows, and
  Linux. Already in the tree transitively via `url`/`idna`.
- `darkmatter` (lib) — the semantic authority (span vocabulary,
  frontmatter block extraction, single-document reference extraction, and the
  later-phase parsing surfaces).
- `biscuit-file` (`file-reference` feature, default features off) —
  file-reference resolution conventions.
- `biscuit-hash` — xxHash content-hash identity for the Phase 3 invalidation
  engine (`WorkspaceIndex`).

## CLI (`darkmatter/cli`)

- `serde` (`derive`) — the `md clean --json` diagnostic envelope. The
  per-diagnostic body is `biscuit_file::YamlDiagnostic`'s own `Serialize`, so
  the wire format stays pinned to the shared diagnostic vocabulary; the derive
  is only needed for the CLI-local envelope and stage tag that wrap it.
- `dunce` — canonicalizes the `md schema validate` document-link href without
  Windows `\\?\` verbatim prefixes, so one document yields one stable
  `file://` URL whether its CLI path was spelled with symlinks or 8.3 short
  names. Same crate the workspace already uses in `biscuit-file` and
  `claudine` for this exact boundary.
