---
ready: false
agent: codex/default
created: 2026-07-07T14:06:41
implemented: true
---

# DMLS Review 1

## Verdict

DMLS is not ready for production. The implementation compiles and the current
DMLS nextest suite is green, but several v1 requirements are either not
implemented as specified or are covered only by narrow in-memory happy paths.

Verification run:

- `cargo check -p dmls --color=never` - passed
- `cargo nextest run -p dmls --color=never` - passed, 251 tests

## Findings

### High - Server-side file-watch fallback is documented but not implemented

Spec acceptance criterion 2 requires watched-file changes to update the graph
for unopened files. The design also calls out a server-side rescan/hash fallback
for clients without reliable watchers, especially Neovim on Linux. The code
selects `WatchMode::ServerRescan`, but no path ever schedules or runs a rescan
when that mode is active. `register_watchers` simply returns without
registration, and the only graph update path for unopened files is
`workspace/didChangeWatchedFiles`.

Evidence:

- `darkmatter/dmls/src/workspace/watch.rs:21` defines `ServerRescan`.
- `darkmatter/dmls/src/router.rs:151` says server-side rescan covers the rest.
- `darkmatter/dmls/src/router.rs:803` only reacts to client watch
  notifications.
- `darkmatter/dmls/src/workspace/startup.rs:77` exposes a discovery helper
  "for the rescan fallback", but it is never called from the router.

Impact: Neovim/Linux and any client without usable watched-file notifications
will keep stale indexes for unopened files. Broken-link diagnostics,
completion, definition, backlinks, wiki resolution, rename impact analysis,
and transclusion cycle checks can all operate on stale workspace state.

Required fix: implement an actual fallback trigger, such as save/config-driven
or timed rescan with content-hash comparison, and add an LSP-session test with a
client profile that selects `ServerRescan` and proves an unopened file
create/change/delete affects diagnostics/navigation.

Verification level: strongest current coverage is Level 1 unit tests for watch
mode selection and event coalescing. There is no end-to-end LSP coverage for the
fallback behavior required by the spec.

### High - Config reload does not reindex or rebuild wiki roots

The spec requires config changes to be reloadable without server restart,
invalidating only affected indexes where possible. The router applies
`workspace/didChangeConfiguration` only to the config object and does not
recompute wiki roots, rebuild the graph, clear stale overlay state, or refresh
diagnostics. This breaks runtime changes to `wiki.wiki_root`,
`workspace.include`/`exclude`, and schema-extension activation.

Evidence:

- `darkmatter/dmls/src/router.rs:794` handles
  `workspace/didChangeConfiguration`.
- `darkmatter/dmls/src/router.rs:801` only calls
  `apply_client_settings`.
- `darkmatter/dmls/src/router.rs:114` computes wiki roots once during
  initialize.
- `darkmatter/dmls/src/graph/invalidate.rs:64` has `set_wiki_roots`, but it is
  not called after config changes.
- `darkmatter/dmls/src/config/mod.rs:336` documents reload behavior but has no
  caller-side invalidation hook.

Impact: a user changing `.dmls.toml` or editor settings can see settings appear
accepted while the workspace graph and diagnostics continue using the old
resolution universe. This is especially visible for wiki links and schema
extension globs.

Required fix: make config reload return an invalidation summary, rebuild
affected indexes/wiki roots, clear or recompute overlay caches where needed,
and publish refreshed diagnostics. Add an LSP-session test that toggles
`wiki.wiki_root` or schema extensions and observes changed diagnostics or
completion without restart.

Verification level: current coverage is Level 1 config-merge tests and one
Level 1-ish notification smoke path that only verifies the server stays alive.
It does not verify user-observable behavior after config reload.

### High - The promised single graph does not carry several required edge kinds

The spec and design commit to one graph carrying `uses_schema`, `uses_file`,
and `uses_variable` edges in addition to references/transclusions. The current
graph model declares all eight edge kinds, but the builder only materializes
heading definition, Markdown/wiki references, and `::file`/`::code`
transclusion edges. Frontmatter `$schema`, frontmatter `file(...)`, images,
style assets, directive path uses beyond transclusion, and interpolation
variables are handled request-time or not represented at all.

Evidence:

- `darkmatter/features/2026-07-04-dmls/spec.md:242` requires one workspace graph
  with schema, file, and interpolation references as typed edges.
- `darkmatter/features/2026-07-04-dmls/design.md:124` lists those edge kinds as
  graph sources.
- `darkmatter/dmls/src/graph/edge.rs:23` declares `UsesSchema`, `UsesFile`,
  and `UsesVariable`.
- `darkmatter/dmls/src/graph/substrate.rs:95` stores only headings, links,
  wiki links, and transclusions in `DocumentIndex`.
- `darkmatter/dmls/src/graph/arena.rs:191` builds only `DefinesAnchor`,
  `DefinesSymbol`, `References`, and `Transcludes` edges.
- `darkmatter/dmls/src/providers/frontmatter.rs:279` resolves `$schema` and
  `file(...)` navigation outside the graph.
- `darkmatter/dmls/src/providers/dsl.rs:313` resolves interpolation definition
  outside the graph.

Impact: this undermines the architecture the refactor/edit/invalidation story
depends on. Invalidation fan-out for `uses_file` cannot work, workspace
references cannot see schema/file/variable uses consistently, and future rename
or code-action logic will have to keep duplicating request-time scanners.

Required fix: either implement graph nodes/edges for the remaining committed
sources or explicitly amend the spec/design to narrow v1. Given the v1 status
claims "all 11 acceptance criteria are delivered", implementation is the better
direction. Add graph-level tests asserting each edge kind that v1 claims is
actually emitted from representative documents.

Verification level: current tests cover graph edges for Markdown links, wiki
links, and transclusions. They do not verify `uses_schema`, `uses_file`, or
`uses_variable` because those edges are not produced.

### Medium - The benchmark harness reports stage timings that do not match the R-6 contract

The R-6 design requires per-stage timings for discovery, read, hash,
frontmatter, Markdown parsing, directive scanning, graph build, reverse index,
diagnostics, and snapshot swap. The shipped `bench_index` measures discovery
once, then calls `collect_indices`, which performs discovery a second time and
folds read/hash/frontmatter/parse work into `parse_markdown_ms`. Several
published fields remain zero even though the work happened.

Evidence:

- `darkmatter/dmls/src/bench.rs:25` exposes the full per-stage report shape.
- `darkmatter/dmls/src/bench.rs:120` times one discovery pass.
- `darkmatter/dmls/src/bench.rs:130` calls `collect_indices`, which discovers
  again before indexing.
- `darkmatter/dmls/src/bench.rs:124` notes that finer stage split is deferred,
  despite spec acceptance criterion 11 requiring per-stage timings.

Impact: the benchmark can still indicate total cold-start time, but it is not a
reliable source for the stage-level performance decisions described in the
design. It can hide whether I/O, hashing, frontmatter extraction, directive
scanning, reverse-index construction, or diagnostics are the bottleneck.

Required fix: split discovery from indexing so the file list is reused, time
read/hash/frontmatter/directives/parse/build/reverse/swap separately, and add a
test that nonzero stage fields correspond to work performed on a corpus.

Verification level: current coverage is Level 1 JSON-shape/count tests. It does
not verify timing semantics.

### Medium - Editor and packaging claims are mostly documentation smoke, not executable verification

The spec targets VS Code, Zed, Neovim, and Helix and requires cross-platform
release artifacts. Phase 11 documents editor setup and includes a Zed extension
scaffold, but the automated suite only exercises in-memory `lsp_server`
sessions and unit tests. There is no executable check that the Zed WASM
extension builds, that `just dist` produces the claimed archives, or that the
native binary can be launched through any editor integration path.

Evidence:

- `darkmatter/features/2026-07-04-dmls/spec.md:42` lists editor targets.
- `darkmatter/features/2026-07-04-dmls/spec.md:315` requires cross-platform
  release artifacts and editor setup docs.
- `darkmatter/features/2026-07-04-dmls/phase11-editor-smoke.md:13` states the
  editor smoke was not run in this non-interactive environment.
- `darkmatter/dmls/tests/level2_lsp_session.rs:1` uses in-memory LSP
  conversations, not real editor/client launch paths.

Impact: the core LSP protocol paths are well covered, but release readiness for
the advertised editor integrations is not proven. A production cut could ship a
server that works in tests but fails from the Zed extension or from the archive
layout clients expect.

Required fix: add non-interactive packaging checks where practical:
`cargo check --manifest-path darkmatter/dmls/zed-dmls/Cargo.toml --target
wasm32-wasip1` or the actual supported target, archive-shape tests for
`just dist`, and at least one stdio subprocess smoke that initializes the
compiled `dmls` binary over real pipes.

Verification level: current strongest coverage is in-memory LSP session tests.
For editor packaging behavior, there is no executable verification.

## Notes

- The current test suite is valuable and broad for in-process logic and
  in-memory LSP protocol behavior. The gaps above are about unimplemented
  spec contracts and mismatched verification levels, not about compilation or
  existing happy-path regressions.
- No Level 3 keyboard/terminal testing appears necessary for DMLS v1 because
  DMLS is a stdio LSP server, not an interactive terminal UI. The user-visible
  behaviors here should be verified through LSP protocol sessions, subprocess
  stdio sessions, and packaging/editor launch smoke tests rather than terminal
  emulator keyboard injection.
