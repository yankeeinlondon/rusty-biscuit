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
