# DMLS v1 — Closure Summary

Phase 11 closes the DMLS v1 feature. All 11 phases are complete; the language
server is a credible standalone Markdown LSP with the full Darkmatter overlay
(wiki links, frontmatter schema intelligence, DSL awareness), safe editing
(rename/code-actions/formatting), and the no-side-effects guarantee.

## What Phase 11 delivered

- **Editor setup docs** — `darkmatter/dmls/docs/editors/{README,vscode,neovim,helix,zed}.md`
  from the R-7 capability matrix, plus a manual `smoke-checklist.md`.
- **Zed extension scaffold** — `darkmatter/dmls/zed-dmls/` (`extension.toml`,
  `cdylib` `Cargo.toml` on `zed_extension_api`, `src/lib.rs` with PATH →
  settings → GitHub-release binary resolution). Workspace-excluded; extract to a
  standalone repo to publish (AD-7).
- **Release recipe** — `just dist` builds a per-platform release archive named
  for the distribution matrix the Zed extension resolves against.
- **Performance sign-off** — `phase11-bench-results.md`; AD-2 verdict (no cache)
  written into `design.md`.
- **Docs drift closure** — `dmls/README.md` rewritten to shipped reality;
  `SKILL.md` DMLS section extended; `spec.md` marked `v1-delivered` with an
  acceptance-criteria proof table; `design.md` AD-2 updated.

## Acceptance criteria

All 11 confirmed with pointers to proving tests/artifacts — see
[spec.md § v1 Status](./spec.md) for the full mapping. Headlines:

- Lifecycle, source-map, Layer-0/1/2/3, formatting, and rename criteria are each
  pinned by an L1 or L2 test in `dmls/tests/`.
- Criterion 7 (no side effects) is proven by `tests/no_side_effects.rs`.
- Criterion 11 (performance) is met on the release build: full repo (3,141
  files) ~1.9 s cold, `vault-5k` ~0.5 s — both inside the R-6 budget.

## Cross-platform posture

The path/case/NFC fixtures (Phase 5 / R-8) and the CRLF/lone-CR source-map
matrix (Phase 2 / R-2) are written platform-neutrally: wiki fixtures use fixed
absolute paths and assert case-sensitivity at L1 (so a case-insensitive
filesystem can't collapse the two files), and URI↔path conversion routes through
`url`'s cross-platform `to_file_path`/`from_file_path`. The only `cfg(windows)`
in the crate is a drive-letter unit test. These pass on the macOS dev host; the
first Windows/Linux CI run is a confirmation, not a discovery. The indexer has
no platform-conditional core path.

## Feature-folder artifacts

- `ad1-negative-check.md` — AD-1 negative-check note.
- `phase3-bench-results.md` — Phase 3 cold-start bench.
- `phase11-bench-results.md` — release-build performance sign-off.
- `phase11-editor-smoke.md` — smoke-checklist status + automated-proof mapping.
- `closure-summary.md` — this file.

## Deferred to post-v1

Per spec § Out of Scope: compose/render preview + side-effecting commands,
embedded-language delegation, remote URL validation, Obsidian vault parity,
TOML/JSON frontmatter, semantic tokens, incremental sync, persistent on-disk
index, and extract/inline refactors. Implementation deviations logged in the
plan (Rule 2/3): the Neovim file-rename command path, `ChangeAnnotation`s, and a
few request-time-vs-materialized-edge choices in the DSL/frontmatter overlays.
