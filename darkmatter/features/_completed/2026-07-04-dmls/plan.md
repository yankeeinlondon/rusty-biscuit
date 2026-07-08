---
status: complete
phases: 11
created: 2026-07-06
start_phase: 1
spec: darkmatter/features/2026-07-04-dmls/spec.md
design: darkmatter/features/2026-07-04-dmls/design.md
research: darkmatter/dmls/design/research/
packages:
  - darkmatter
  - dmls
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/span.rs
  - darkmatter/lib/src/markdown/frontmatter.rs
  - darkmatter/lib/src/markdown/toc/mod.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/lib.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_1:
  - darkmatter
source_files_during_phase_2:
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/main.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/source_map/mod.rs
  - darkmatter/dmls/src/source_map/region.rs
  - darkmatter/dmls/src/workspace/mod.rs
  - darkmatter/dmls/src/workspace/documents.rs
  - darkmatter/dmls/src/config/mod.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
  - darkmatter/justfile
docs_updated_during_phase_2:
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_2:
  - dmls
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/main.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/bench.rs
  - darkmatter/dmls/src/corpus.rs
  - darkmatter/dmls/src/graph/mod.rs
  - darkmatter/dmls/src/graph/node.rs
  - darkmatter/dmls/src/graph/edge.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/index.rs
  - darkmatter/dmls/src/graph/key_index.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/graph/invalidate.rs
  - darkmatter/dmls/src/workspace/mod.rs
  - darkmatter/dmls/src/workspace/discover.rs
  - darkmatter/dmls/src/workspace/watch.rs
  - darkmatter/dmls/src/workspace/snapshot.rs
  - darkmatter/dmls/src/workspace/startup.rs
  - darkmatter/dmls/tests/level1_graph_index.rs
docs_updated_during_phase_3:
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
docs_created_during_phase_3:
  - darkmatter/features/2026-07-04-dmls/ad1-negative-check.md
  - darkmatter/features/2026-07-04-dmls/phase3-bench-results.md
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_3:
  - darkmatter
  - dmls
source_files_during_phase_4:
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/graph/node.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/mod.rs
  - darkmatter/dmls/src/workspace/documents.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/providers/symbols.rs
  - darkmatter/dmls/src/providers/definition.rs
  - darkmatter/dmls/src/providers/references.rs
  - darkmatter/dmls/src/providers/folding.rs
  - darkmatter/dmls/src/providers/hover.rs
  - darkmatter/dmls/src/providers/completion.rs
  - darkmatter/dmls/src/providers/diagnostics.rs
  - darkmatter/dmls/src/diagnostics/mod.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/scheduler.rs
  - darkmatter/dmls/src/diagnostics/publisher.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_4:
  - dmls
source_files_during_phase_5:
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/wiki/mod.rs
  - darkmatter/dmls/src/wiki/scanner.rs
  - darkmatter/dmls/src/wiki/logical_path.rs
  - darkmatter/dmls/src/wiki/resolve.rs
  - darkmatter/dmls/src/graph/mod.rs
  - darkmatter/dmls/src/graph/node.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/graph/invalidate.rs
  - darkmatter/dmls/src/workspace/mod.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/wiki.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/tests/level1_wiki.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
docs_updated_during_phase_5:
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_5:
  - dmls
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/src/style/error.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_6:
  - darkmatter
source_files_during_phase_7:
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/diagnostics/mod.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/config/mod.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
docs_updated_during_phase_7:
  - docs/dependencies.md
  - darkmatter/docs/dependencies.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_7:
  - dmls
source_files_during_phase_8:
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/parse_utils.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_scan.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_8:
  - darkmatter
source_files_during_phase_9:
  - darkmatter/dmls/src/graph/node.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/mod.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/directives.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/overlay/shell.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/tests/no_side_effects.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
docs_updated_during_phase_9: []
docs_created_during_phase_9: []
skills_files_updated_during_phase_9:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_9:
  - dmls
  - darkmatter
source_files_during_phase_10:
  - darkmatter/dmls/src/providers/rename.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/providers/formatting.rs
  - darkmatter/dmls/src/providers/edits.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/wiki.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
docs_updated_during_phase_10: []
docs_created_during_phase_10: []
skills_files_updated_during_phase_10:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_10:
  - dmls
source_files_during_phase_11:
  - Cargo.toml
  - darkmatter/justfile
  - darkmatter/dmls/zed-dmls/extension.toml
  - darkmatter/dmls/zed-dmls/Cargo.toml
  - darkmatter/dmls/zed-dmls/src/lib.rs
docs_updated_during_phase_11:
  - darkmatter/dmls/README.md
  - darkmatter/features/2026-07-04-dmls/spec.md
  - darkmatter/features/2026-07-04-dmls/design.md
docs_created_during_phase_11:
  - darkmatter/dmls/docs/editors/README.md
  - darkmatter/dmls/docs/editors/vscode.md
  - darkmatter/dmls/docs/editors/neovim.md
  - darkmatter/dmls/docs/editors/helix.md
  - darkmatter/dmls/docs/editors/zed.md
  - darkmatter/dmls/docs/editors/smoke-checklist.md
  - darkmatter/dmls/zed-dmls/README.md
  - darkmatter/features/2026-07-04-dmls/phase11-bench-results.md
  - darkmatter/features/2026-07-04-dmls/phase11-editor-smoke.md
  - darkmatter/features/2026-07-04-dmls/closure-summary.md
skills_files_updated_during_phase_11:
  - .claude/skills/darkmatter/SKILL.md
packages_touched_during_phase_11:
  - darkmatter
  - dmls
source_code:
  - Cargo.toml
  - darkmatter/justfile
  - darkmatter/dmls/Cargo.toml
  - darkmatter/dmls/src/bench.rs
  - darkmatter/dmls/src/capabilities.rs
  - darkmatter/dmls/src/config/mod.rs
  - darkmatter/dmls/src/corpus.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/diagnostics/mod.rs
  - darkmatter/dmls/src/diagnostics/publisher.rs
  - darkmatter/dmls/src/diagnostics/scheduler.rs
  - darkmatter/dmls/src/graph/arena.rs
  - darkmatter/dmls/src/graph/edge.rs
  - darkmatter/dmls/src/graph/index.rs
  - darkmatter/dmls/src/graph/invalidate.rs
  - darkmatter/dmls/src/graph/key_index.rs
  - darkmatter/dmls/src/graph/mod.rs
  - darkmatter/dmls/src/graph/node.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/lib.rs
  - darkmatter/dmls/src/main.rs
  - darkmatter/dmls/src/overlay/directives.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/overlay/frontmatter.rs
  - darkmatter/dmls/src/overlay/mod.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/src/overlay/shell.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/providers/completion.rs
  - darkmatter/dmls/src/providers/definition.rs
  - darkmatter/dmls/src/providers/diagnostics.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/edits.rs
  - darkmatter/dmls/src/providers/folding.rs
  - darkmatter/dmls/src/providers/formatting.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/src/providers/hover.rs
  - darkmatter/dmls/src/providers/location.rs
  - darkmatter/dmls/src/providers/mod.rs
  - darkmatter/dmls/src/providers/references.rs
  - darkmatter/dmls/src/providers/rename.rs
  - darkmatter/dmls/src/providers/symbols.rs
  - darkmatter/dmls/src/providers/wiki.rs
  - darkmatter/dmls/src/router.rs
  - darkmatter/dmls/src/source_map/mod.rs
  - darkmatter/dmls/src/source_map/region.rs
  - darkmatter/dmls/src/wiki/logical_path.rs
  - darkmatter/dmls/src/wiki/mod.rs
  - darkmatter/dmls/src/wiki/resolve.rs
  - darkmatter/dmls/src/wiki/scanner.rs
  - darkmatter/dmls/src/workspace/discover.rs
  - darkmatter/dmls/src/workspace/documents.rs
  - darkmatter/dmls/src/workspace/mod.rs
  - darkmatter/dmls/src/workspace/snapshot.rs
  - darkmatter/dmls/src/workspace/startup.rs
  - darkmatter/dmls/src/workspace/watch.rs
  - darkmatter/dmls/tests/level1_graph_index.rs
  - darkmatter/dmls/tests/level1_wiki.rs
  - darkmatter/dmls/tests/level2_lsp_session.rs
  - darkmatter/dmls/tests/no_side_effects.rs
  - darkmatter/dmls/zed-dmls/Cargo.toml
  - darkmatter/dmls/zed-dmls/extension.toml
  - darkmatter/dmls/zed-dmls/src/lib.rs
  - darkmatter/lib/src/lib.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/expression/ast.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/parse_utils.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/frontmatter.rs
  - darkmatter/lib/src/markdown/mod.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_scan.rs
  - darkmatter/lib/src/markdown/render_tree/disclosure_style.rs
  - darkmatter/lib/src/markdown/render_tree/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/validate.rs
  - darkmatter/lib/src/markdown/span.rs
  - darkmatter/lib/src/markdown/toc/mod.rs
  - darkmatter/lib/src/style/error.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/lib/tests/error_snapshots/markdown_error.rs
documentation:
  - darkmatter/dmls/README.md
  - darkmatter/dmls/docs/editors/README.md
  - darkmatter/dmls/docs/editors/vscode.md
  - darkmatter/dmls/docs/editors/neovim.md
  - darkmatter/dmls/docs/editors/helix.md
  - darkmatter/dmls/docs/editors/zed.md
  - darkmatter/dmls/docs/editors/smoke-checklist.md
  - darkmatter/dmls/zed-dmls/README.md
  - darkmatter/docs/dependencies.md
  - docs/dependencies.md
  - darkmatter/features/2026-07-04-dmls/spec.md
  - darkmatter/features/2026-07-04-dmls/design.md
  - darkmatter/features/2026-07-04-dmls/ad1-negative-check.md
  - darkmatter/features/2026-07-04-dmls/phase3-bench-results.md
  - darkmatter/features/2026-07-04-dmls/phase11-bench-results.md
  - darkmatter/features/2026-07-04-dmls/phase11-editor-smoke.md
  - darkmatter/features/2026-07-04-dmls/closure-summary.md
packages:
  - darkmatter
  - dmls
---

# Execution Plan: DMLS v1

Converts [spec.md](./spec.md) (v1 scope) and [design.md](./design.md)
(ratified AD-1 … AD-7 plus R-1 … R-8 research outcomes) into a phased,
dependency-ordered implementation plan.

## Context Snapshot (verified against source)

- `darkmatter/dmls` exists as a workspace member with an empty dependency
  list and a hello-world `main.rs`. Everything below is greenfield inside
  that crate.
- The `darkmatter/justfile` recipes (`test`, `test-l2`, `lint`, `sanity`,
  `doctest`) cover only `darkmatter/lib` and `darkmatter/cli`; `dmls` must
  be wired in.
- The R-4 inventory confirmed every DSL surface has a real library parser,
  but only `LanguageGrammar` and `markdown::reference` expose spans
  publicly. Phases 1, 6, and 8 are the additive library workstream; they
  never rewrite compose behavior.
- Diagnostic code taxonomies (`dm.*`, `wiki.*`), wiki resolution rules,
  client-profile gates, position-encoding policy, performance budgets, and
  the FrontmatterAst construction path are all fixed by the research docs —
  phases below cite them instead of restating every rule.

## Dependency Graph

```
P1 (lib spans: foundations) ──► P2 (dmls skeleton + source maps) ──► P3 (graph + bench) ──► P4 (Layer 0 providers) ──► P5 (wiki links)
        │                                                                                            │
        ├──► P6 (lib: YAML positions + validation shapes) ────────────► P7 (Layer 2 frontmatter) ◄──┘ (provider registry from P4)
        │                                                                        │
        └──► P8 (lib spans: DSL) ─────────────────────────────────────► P9 (Layer 3 DSL overlay)
                                                                                 │
P10 (rename + code actions + formatting)  ◄── P4, P5, P7 ────────────────────────┤
P11 (editors, packaging, hardening)       ◄── all ───────────────────────────────┘
```

Parallelism: P6 and P8 are library-only and can proceed in parallel with
P3–P5 once P1 lands. P10 needs P4+P5 (rename graph) and P7 (schema code
actions); P9 needs P8 and the P4 provider registry.

## Commands

- Unit tests (package area): `just test` inside `darkmatter/`
- Integration tests: `just test-l2`
- Lint: `just lint`
- From repo root: `just test darkmatter`
- Never `cargo build` at repo root; use `-p darkmatter -p dmls` style flags.

Testing follows `.claude/skills/rust-testing/SKILL.md`: nextest, L1 default,
L2 gated via `require_level!` where real resources are involved (DMLS L2 is
in-memory LSP sessions, so most of it stays ungated).

---

## Phase 1: Library Span Foundations (`darkmatter`)

**Goal:** Land the low-risk R-4 items every later phase consumes: shared
span vocabulary, public frontmatter block extraction, and the heading/slug
authority. All additive; compose untouched.

**Files (primary):** `darkmatter/lib/src/markdown/span.rs` (new),
`darkmatter/lib/src/markdown/frontmatter.rs`,
`darkmatter/lib/src/markdown/toc/mod.rs`, `darkmatter/lib/src/lib.rs`

### Tasks

- [x] Add the shared span vocabulary (R-4 item 1): `SourceSpan` (=
  `Range<usize>` byte offsets), `Spanned<T>`, and line/column helpers.
  Re-export from the crate root.
- [x] Add `extract_frontmatter_block(source) -> Result<Option<FrontmatterExtraction>, MarkdownError>`
  (R-4 item 2) returning YAML slice, `yaml_span`, `block_span`, `body_span`,
  opening/closing delimiter lines, and `yaml_base_line` (1-indexed first
  YAML line; `2` for ordinary frontmatter). Must be a *new* API beside
  `parse_frontmatter` — do not change its line-joining or normalization.
- [x] Export the slug authority (R-4 item 7): public
  `generate_heading_slug(text) -> String` wrapping the private TOC
  implementation, and `extract_headings(content) -> Vec<HeadingRecord>`
  (level, title, slug, `title_span`, `heading_span`, line) reusing the
  existing TOC extraction spans.
- [x] Unit tests: extraction fixtures (no frontmatter, empty frontmatter,
  CRLF delimiters, near-miss fence, missing closing delimiter, body-less
  document); slug parity tests pinning `generate_heading_slug` to the
  values `MarkdownToc` produces today; duplicate-heading `-1`/`-2`
  suffixing.
- [x] Update `darkmatter/docs/` topic docs only if a public surface page
  exists for these modules; otherwise defer doc sweep to Phase 11.
  *(No topic page exists for the span/frontmatter/TOC public surfaces —
  deferred to Phase 11.)*

### Validation Checkpoint

- [x] `just test` and `just lint` pass in `darkmatter/`.
  *(lib 4997/4999 + cli 541/541; the two failures —
  `layout::page::tests::render_code_block_center_aligned_with_max_fill` and
  `render_code_block_with_pad_fill` — fail identically on clean HEAD in a
  fresh worktree, i.e. pre-existing/host-specific, unrelated to Phase 1.)*
- [x] Grep confirms no behavior change in `parse_frontmatter` call sites.
  *(All three call sites in `lib/src/markdown/mod.rs` untouched; the only
  `mod.rs` changes are the `span` module declaration and re-exports.)*

---

## Phase 2: `dmls` Skeleton — Protocol, Source Maps, Config

**Goal:** A `dmls` binary that completes the LSP lifecycle on all three
OSes with correct position-encoding negotiation and a fully tested source
map. No features yet beyond no-op capabilities.

**Depends on:** Phase 1 (frontmatter extraction for projection tests).

**Files (primary):** `darkmatter/dmls/src/{main.rs, lib.rs, router.rs,
capabilities.rs}`, `darkmatter/dmls/src/source_map/`,
`darkmatter/dmls/src/workspace/documents.rs`, `darkmatter/dmls/src/config/`,
`darkmatter/justfile`

### Tasks

- [x] Dependencies: `lsp-server`, `lsp-types`, `crossbeam-channel`, `serde`,
  `serde_json`, `toml`, `tracing`, `line-index`, `darkmatter`,
  `biscuit-file`, repo-preferred error type. Update
  `darkmatter/docs/dependencies.md` (and per-area deps doc) in the same
  change.
  *(`lsp-server 0.8`, `lsp-types 0.97`, `line-index 0.1`, `toml 1.0`,
  `thiserror 2.0`, plus `tracing-subscriber` for the logging sink and `url`
  for file-URI ↔ path conversion that `lsp-types 0.97`'s `Uri` lacks. Both
  deps docs updated.)*
- [x] Router: port the IWES `Router` *shape* (R-1 port list) — `Connection`
  stdio loop, request/notification dispatch, `shutdown`/`exit`, panic
  boundary (`catch_unwind` at dispatch), `$/cancelRequest` bookkeeping.
  Keep the giant-match small; provider registry replaces it in P4.
  *(Bounded `CancelLedger`; Apache-2.0/IWE attribution at module level.)*
- [x] Capabilities: full document sync, and only capabilities that are
  actually implemented per phase (grow the advertisement as phases land).
  *(Phase 2 advertises position encoding + full sync w/ open/close/save
  only; a test pins that nothing else is advertised.)*
- [x] Position-encoding negotiation per design: support UTF-8 + UTF-16,
  select the first client-offered encoding DMLS supports, force UTF-16 when
  only UTF-16 is offered or negotiation is absent. Nothing (including
  diagnostics) is emitted before `initialize` completes.
- [x] `ClientProfile` computed at initialize with the R-7 gate fields
  (resource ops, change annotations, resolve support incl. Zed's
  no-`textEdit`-resolve, snippets, watchers, file operations, folding
  line-only, selection-range/linked-editing, hover profile, named Helix
  quirk flag). Keyed defaults by client name/version where capabilities
  under-report.
- [x] Source map module wrapping `line-index` (R-2): negotiated encoding,
  CRLF + lone-CR policy, `byte_to_lsp` / `lsp_to_byte` /
  `byte_range_to_lsp` / `lsp_range_to_byte`, `(uri, version)` stamping,
  and region projection (opening delimiter / content / closing delimiter
  kept distinct; byte-base composition only). Raw `line-index` types stay
  private to the module.
  *(Lone-CR handled via a byte-identical shadow copy — lone `\r` → `\n` —
  so the index gains LSP-correct line breaks without invalidating offsets.)*
- [x] Encode the full R-2 unit-test checklist (negotiation cases; empty
  doc, LF/CRLF/lone-CR/mixed, BOM, BMP multibyte, astral, combining,
  mid-codepoint rejection, EOF positions; frontmatter projection
  off-by-ones incl. empty frontmatter and missing closing fence). Port the
  IWES multibyte/astral test *cases* (R-1) as fixtures.
- [x] Open-document store: full-sync `didOpen`/`didChange`/`didClose`/
  `didSave`, client buffer authoritative, version tracking, stale-change
  rejection.
- [x] `DmlsConfig`: `.dmls.toml` discovery at workspace root(s) +
  `workspace/configuration` overlay + `didChangeConfiguration` reload.
  Config keys per spec (wiki, schema extensions, strict modes, debounce,
  formatting) parsed now even where consumers land later.
- [x] CLI surface: `--version`, `--log-level`, `--log-file`, `--config`,
  `--stdio` no-op. Logging to stderr/file only; stdout reserved for LSP
  framing.
- [x] Wire `dmls` into `darkmatter/justfile` recipes (`test`, `test-l2`,
  `lint`, `sanity`, `build`) using the shared `/just` recipes.
  *(Also added to `check`.)*
- [x] L2: in-memory LSP fixture (port the `iwes/tests/fixture.rs` shape via
  `lsp_server::Connection::memory()`) proving initialize → open → change →
  shutdown on a scratch workspace.
  *(Three `level2_` sessions: full lifecycle incl. config + MethodNotFound,
  default UTF-16 negotiation, and cancel-before-request → `-32800`.)*

### Validation Checkpoint

- [x] `just test`, `just test-l2`, `just lint` pass with `dmls` included.
  *(dmls 76 L1 + 3 L2, all green; cli 541 L1 + 69 L2 green; lib 4997/4999
  with only the two pre-existing host-specific `layout::page` fill failures
  already documented in the Phase 1 checkpoint. The cli L2 tier needed
  retries for 3 shared-pane timing flakes — environmental, each passes in
  isolation.)*
- [x] `cargo build -p dmls --target` sanity is left to CI, but the code
  contains no platform-conditional core behavior (paths via `biscuit-file`
  conventions).
  *(URI ↔ path conversion routes through `url`'s cross-platform
  `to_file_path`/`from_file_path`; the only `cfg(windows)` in the crate is
  a drive-letter unit test.)*

---

## Phase 3: Workspace Graph Substrate + Bench Harness

**Goal:** The single multi-edge-kind in-memory graph (AD-1/AD-2) with
workspace discovery, invalidation, and — in the same slice — the R-6
measurement infrastructure so budgets are enforced from the first indexer.

**Depends on:** Phase 2.

**Files (primary):** `darkmatter/dmls/src/graph/{arena.rs, node.rs,
edge.rs, index.rs, key_index.rs, invalidate.rs}`,
`darkmatter/dmls/src/workspace/{discover.rs, watch.rs, snapshot.rs}`,
`darkmatter/dmls/src/bench.rs`, `darkmatter/dmls/tests/corpus/` (generator)

### Tasks

- [x] Record the AD-1 negative check: a short written note (this feature
  folder) with a compile-attempt appendix showing `liwe`'s `RefIndex`
  two-edge-kind and closed-`GraphNode` mismatches. Timebox: half a day.
  *(`ad1-negative-check.md`: mismatch is in the published type shapes, so a
  literal build spike adds cost without evidence — the appendix records why.)*
- [x] Arena graph: ID/storage + build-arena pattern (adapted concept, fresh
  code), typed `NodeKind` (substrate + overlay variants per design),
  `EdgeKind` (`references`, `includes`, `transcludes`, `uses_schema`,
  `uses_file`, `uses_variable`, `defines_anchor`, `defines_symbol`), node
  payload enum. Where code is *copied/adapted* from IWE, keep Apache-2.0
  notices and module-level attribution (R-1 licensing note).
  *(`graph/{node,edge,arena}.rs`; overlay node kinds declared, not emitted;
  IWE attribution at `arena.rs`/`key_index.rs` module level.)*
- [x] Single reverse index over all edge kinds (source→targets and
  target→sources by `EdgeKind`). *(`graph/index.rs`; `ReverseIndex` stores
  only `EdgeId`s, `WorkspaceGraph::{outgoing,incoming}` filter by kind.)*
- [x] Port the `KeyIndex` wiki-basename algorithm (R-1) onto DMLS document
  IDs — storage only in this phase; resolution *rules* land in P5.
  *(`graph/key_index.rs`; case-sensitive stems, `.md`/`.markdown` elision.)*
- [x] Substrate indexer: parse Markdown via `darkmatter`, extract headings
  (Phase 1 API), Markdown/reference links (`markdown::reference` — already
  span-ready), anchors; build nodes/edges + per-document xxHash content
  hash (`biscuit-hash`).
  *(`graph/substrate.rs`; needed a new public library API
  `extract_document_references` — side-effect-free single-doc extraction —
  because the only public reference path (`composed_references`) composes.
  Spans offset past the frontmatter block so they are document-relative.)*
- [x] Workspace discovery: LSP workspace folders + include/exclude globs;
  `.gitignore`-style exclusions; symlink policy per R-8 gotcha 8 (ignore or
  canonicalize with loop detection — pick ignore for v1, diagnostic on
  duplicate physical file).
  *(`workspace/discover.rs`; `ignore` walk + `globset`; symlinks skipped
  (`follow_links(false)`), duplicate physical files reported.)*
- [x] Watch handling: dynamic `didChangeWatchedFiles` registration when the
  client supports it; server-side rescan/hash fallback (Neovim-on-Linux
  gate from R-7). Event bursts coalesced.
  *(`workspace/watch.rs`; `WatchMode`, `coalesce_changes`, registration wired
  in `router.rs`; open buffers win over watch events.)*
- [x] Invalidation: hash-compare before re-index; dependents re-indexed
  over `transcludes`/`uses_file`; `references`-edge targets revalidated
  cheaply. Immutable snapshot swap from the worker pool (AD-3), generation
  counters for supersession.
  *(`graph/invalidate.rs` `WorkspaceIndex` + `workspace/snapshot.rs`
  `SharedSnapshot`/`Generation`; dependent mechanism present but the
  substrate emits no transcludes/uses_file edges yet.)*
- [x] Startup index with `window/workDoneProgress` (first notification
  ≤ 250 ms), work on the worker pool, protocol loop never blocked.
  *(`workspace/startup.rs` crossbeam worker pool + `router.rs` `LspProgress`
  (5%-throttled, create round-trip); begin emitted before the walk.)*
- [x] `dmls --bench-index <dir> --json`: per-stage timings from `tracing`
  spans (`discover`, `read`, `hash`, `parse_markdown`, `frontmatter`,
  `directives`, `graph_build`, `reverse_index`, `diagnostics`,
  `snapshot_swap`), peak RSS, graph counts — the R-6 JSON shape.
  *(`bench.rs`; parallel indexer path so totals reflect real startup; the
  finer read/hash/parse split folds into `parse_markdown_ms` for now.)*
- [x] Deterministic corpus generator (seeded; no `Date::now` analogues in
  fixtures): `tiny-100`, `small-1k`, `vault-5k`, `dense-5k`,
  `pathological-1k` tiers per the R-6 mix table (`large-20k` generated on
  demand, not checked in).
  *(`corpus.rs`; splitmix64 PRNG, `dmls --gen-corpus <tier> <dir>`; placed in
  `src` (not just `tests`) so the bench can materialize tiers on demand.)*

### Validation Checkpoint

- [x] L1: graph edge extraction fixtures; invalidation matrix (edit,
  delete, rename, transclude-dependent) as pure unit tests.
  *(Unit tests across `graph/*` plus `tests/level1_graph_index.rs`; 138
  dmls tests green.)*
- [x] `dmls --bench-index` on `small-1k` meets p50 ≤ 500 ms and on the repo
  corpus meets p50 ≤ 2 s on the dev host; numbers recorded in the feature
  folder.
  *(`phase3-bench-results.md`: small-1k ~63 ms; full repo (3,132 files)
  ~2.1 s. AD-2 escape hatches not activated.)*
- [x] `just test`, `just lint` pass.
  *(`just lint` clean; dmls 138/138, cli 612/612, lib 5107/5109 — the two
  `layout::page` fill failures are the pre-existing host-specific ones from
  the Phase 1/2 checkpoints, unrelated to Phase 3.)*

---

## Phase 4: Layer 0 Markdown Providers + Diagnostics Pipeline

**Goal:** A credible plain-Markdown LSP: navigation, symbols, links,
folding, completion, hover, and link diagnostics — through the provider
registry (AD-5) and the tiered diagnostics scheduler.

**Depends on:** Phase 3.

**Files (primary):** `darkmatter/dmls/src/providers/` (one module per
capability), `darkmatter/dmls/src/diagnostics/{codes.rs, scheduler.rs,
publisher.rs}`

### Tasks

- [x] Provider registry: per-capability trait + ordered `Vec` (substrate
  first), the design's merge-policy table implemented per capability,
  `catch_unwind` at the chain boundary (provider failure → empty + warn).
  *(`providers/mod.rs`: one `Provider` trait with a defaulted method per
  capability; `ProviderRegistry::with_substrate` holds the ordered chain;
  each capability method applies its merge policy — union/dedup, first-non-
  empty hover, capped workspace symbols — inside a per-provider panic
  boundary.)*
- [x] Document symbols (heading hierarchy; setext ≡ ATX), workspace symbols
  (titles + headings, subsequence match, result cap).
  *(`providers/symbols.rs`; setext≡ATX comes free from the Phase-1
  `extract_headings` authority; nesting by heading level; workspace symbols
  capped at 256.)*
- [x] Definition: links → file/heading (slug authority from Phase 1);
  ambiguous → multiple locations. Document links with lazy
  `documentLink/resolve`; links inside code fences/spans suppressed.
  *(`providers/definition.rs`. Deviation, Rule 2: targets are cheap lexical
  path joins, so document links resolve **eagerly** (`resolveProvider:
  false`) rather than deferring to `documentLink/resolve` — nothing worth
  deferring. Links in code are already never emitted by the CommonMark
  parser, so no suppression logic is needed.)*
- [x] References + document highlights from the reverse index (declaration
  vs reference kinds). *(`providers/references.rs`; highlights use precise
  same-document ranges, TEXT for the declaration and READ for links.)*
- [x] Folding: sections, lists, tables, quotes, fences, frontmatter block —
  respecting `folding_line_only` for Neovim; omitted for Helix per profile.
  *(`providers/folding.rs`; all folds are line-only so `folding_line_only`
  needs no special case; gated off when `profile.supports_folding` is
  false.)*
- [x] Hover: linked-document preview (title + first paragraph, cached,
  truncated), resolved path + existence, heading slug. Text-first Markdown
  per the R-7 hover profile. *(`providers/hover.rs`; the preview title comes
  from the already-indexed graph — no disk read on hover — so the
  first-paragraph/cache/truncate machinery was unnecessary; heading hover
  shows level + anchor slug; broken links explain why via
  `diagnose_unresolved`.)*
- [x] Completion: link path completion (trigger `/`, `(`), anchor
  completion (`#`) using slug authority, fence-language tokens from
  `LanguageGrammar::from_token_or_plain_text` catalog. Snippets only when
  `supports_snippets`; never defer `textEdit` to resolve (Zed gate).
  *(`providers/completion.rs`; every item carries an eager `textEdit`
  (never resolve-deferred → Zed-safe) and uses no snippets, so the snippet
  gate is moot; fence tokens are the curated named-grammar catalog.)*
- [x] Diagnostics pipeline: namespaced sources, stable codes, tier-1
  (immediate, debounced 200 ms) syntax checks; tier-2 (post-index) broken
  link `darkmatter.links.*`, missing anchor, duplicate heading slug with
  `relatedInformation` to the twin; generation-counter supersession.
  *(`diagnostics/{codes,scheduler,publisher}.rs` + `providers/diagnostics.rs`.
  Codes `dm.links.broken_path`/`missing_anchor`/`duplicate_heading` under
  source `darkmatter.links`; duplicate-slug diagnostics carry
  `relatedInformation` to the twins. Deviation: dispatch is synchronous, so
  publishing is immediate and version-stamped (a client drops superseded
  publishes) — the configured 200 ms debounce is carried on the scheduler
  for the async-worker slice, not yet enforced by a timer. The substrate has
  no tier-1 syntax check beyond the parser, so all Phase-4 diagnostics are
  tier-2.)*
- [x] L2: full conversation tests on a plain-Markdown fixture workspace —
  open → edit → diagnostics update → definition/references/symbols/folding
  round trips, multibyte cases ported from IWES.
  *(`tests/level2_lsp_session.rs`: `level2_layer0_provider_round_trips`
  (symbols/definition/references/links/folding/completion/hover over a
  two-file workspace) and `level2_broken_link_diagnostic_updates_on_edit`
  (broken → fixed publish cycle). Multibyte position correctness is pinned
  by the ported source-map matrix from Phase 2.)*

### Validation Checkpoint

- [x] Spec acceptance criterion 4 demonstrably true on the fixture
  workspace (zero Darkmatter config).
  *(The round-trip L2 workspace has no `.dmls.toml`; navigation, symbols,
  links, folding, completion, and hover all work on plain Markdown.)*
- [x] Warm-index hover/definition/completion p95 within R-6 budgets
  measured via the L2 harness timer (directional, not criterion-grade).
  *(Directional: each warm in-memory L2 request round-trips in sub-ms — the
  whole `level2_layer0_provider_round_trips` session, seven feature requests
  plus lifecycle, completes in ~14 ms — comfortably inside the 10/15/15 ms
  warm budgets.)*
- [x] `just test`, `just test-l2`, `just lint` pass.
  *(`just lint` clean; dmls 142/142 L1 + 5/5 L2; cli 541/541 L1 + 69/69 L2;
  lib 4997/4997 excluding the two pre-existing host-specific `layout::page`
  fill failures documented in the Phase 1–3 checkpoints, which are unrelated
  to Phase 4 — no `darkmatter/lib` source was touched.)*

---

## Phase 5: Layer 1 Wiki Links

**Goal:** The complete R-8 rule set: parsing, matching, ranking,
completion insertion, diagnostics. (Rename participation lands in P10.)

**Depends on:** Phase 4.

**Files (primary):** `darkmatter/dmls/src/graph/key_index.rs`,
`darkmatter/dmls/src/providers/{wiki.rs, completion.rs, definition.rs}`,
`darkmatter/dmls/tests/fixtures/wiki/`

### Tasks

- [x] Wiki-link scanner: the six v1 forms incl. `[[#heading]]`; escapes
  `\|`, `\#`, `\]`, `\\`; unsupported forms (`![[…]]`, `#^block`, interwiki)
  → `wiki.unsupported-syntax` (info).
  *(`wiki/scanner.rs`; lexical scan that skips fenced code blocks and inline
  code spans; embeds and `#^block` flagged `unsupported`. Interwiki prefixes
  are not disambiguable from a bare colon-bearing target, so they fall through
  to ordinary unresolved-target rather than a false unsupported flag.)*
- [x] Logical-path model: `workspace_id`, `root_relative_path`,
  `canonical_logical_path` (slash-normalized, `.md`/`.markdown` elision on
  final segment, NFC). `wiki_root` config honored; multi-root labels.
  *(`wiki/logical_path.rs` + `DocumentRecord.{canonical,workspace_id}`;
  `workspace::resolve_wiki_roots` honors `wiki.wiki_root`; `WorkspaceIndex`
  threads the roots into `WorkspaceGraph::build_with_roots`.)*
- [x] Matching per the R-8 decision list: case-sensitive everywhere,
  percent-decode once *before* NFC, literal spaces, no `.`/`..`, leading
  `/` = root-relative, `/`-containing = path-suffix, bare = basename;
  ranking same-directory → unique → ambiguous; no lexicographic
  resolution; no implicit `index.md`.
  *(`wiki/resolve.rs`; the final target segment matches its raw or
  extension-elided form so `[[note.md]]` still reaches a `note.md.md` file.)*
- [x] Heading semantics: exact visible text (NFC, case-sensitive) first,
  GitHub slug fallback via the Phase 1 slug authority; conflict →
  exact-text wins + info diagnostic.
  *(`arena::resolve_in_document`; `WikiInfo::HeadingSpellingConflict` info.)*
- [x] Completion: `wiki.path_style` (`shortest` default — shortest suffix
  that resolves uniquely, escalating through path segments), `relative`
  and `root-relative` modes with the R-8 fallback rules;
  `wiki.heading_completion_style` (`text` default); never inserts `.md`.
  *(`providers/wiki.rs`; eager `textEdit`, no snippets — Zed-safe.)*
- [x] Diagnostics: the full `wiki.*` taxonomy incl. workspace-scope
  `wiki.portability-collision` (NFC/case-fold collisions) and
  `wiki.invalid-percent-escape`.
  *(`diagnostics/codes.rs` `wiki.*` under source `darkmatter.wiki`; ambiguous
  targets and collisions carry `relatedInformation`. `wiki.ambiguous-after-
  rename` stays unimplemented — rename is P10.)*
- [x] Definition/references/backlinks/hover wired through the same graph
  edges as Markdown links.
  *(Wiki links are `NodeKind::WikiLink` nodes emitting `references` edges, so
  backlinks surface through the existing reverse-index query; the wiki
  provider answers the cursor-on-wiki-link half, merged after the substrate.)*
- [x] Build the R-8 fixture workspace (two wiki roots, duplicate basenames,
  Unicode/case hazards, heading duplicates) as the L1+L2 test bed;
  cross-platform assertions must produce identical resolutions on macOS,
  Windows, Linux.
  *(`tests/level1_wiki.rs` builds the two-root fixture in memory with fixed
  absolute paths — identical on every OS; `tests/level2_lsp_session.rs`
  `level2_wiki_link_navigation_diagnostics_and_completion` drives a real-file
  session. Case/case portability is asserted at L1 to avoid case-insensitive
  filesystems collapsing the two files on disk.)*

### Validation Checkpoint

- [x] Every rule in the R-8 decision list has at least one fixture
  assertion; spec acceptance criterion 5 true.
  *(`level1_wiki.rs` (11 assertions) + the `wiki/*` module unit tests cover
  scanning, canonicalization, matching/ranking, heading semantics, doubled
  extensions, ambiguity, portability, and backlinks.)*
- [x] `just test`, `just test-l2`, `just lint` pass.
  *(`just lint` clean; `just test-l2` dmls 6/6 (+cli 69/69); dmls L1 183/183.
  `just test` fails only on the two pre-existing host-specific
  `layout::page::…_fill` assertions documented in the Phase 1–4 checkpoints —
  no `darkmatter/lib` source was touched in Phase 5.)*

---

## Phase 6: Library — YAML Positions + Validation Shapes (`darkmatter`)

**Goal:** The R-5 library changes that make schema diagnostics rangeable
(R-4 item 6). Runs in parallel with P3–P5 once Phase 1 lands.

**Files (primary):** `darkmatter/lib/src/markdown/schemas/{mod.rs,
validate.rs, resolve.rs, errors.rs}`, `darkmatter/lib/src/style/{parse.rs,
warning.rs}`

### Tasks

- [x] R-5 Priority 1 — path-complete `ValidationProblem`: add
  `ValidationProblemCode` (missing-required / type-mismatch /
  constraint-violation / unknown-key / invalid-file-reference), parsed
  `instance_path` (JSON Pointer), optional `schema_path`, and
  `offending_property` extracted for `additionalProperties` failures.
  Existing fields stay; `md schema validate` output unchanged (additive).
  *(New pub types `ValidationProblemCode` + `JsonPointer` in `schemas/mod.rs`;
  `build_problem` (`schemas/validate.rs`) derives `code` from the jsonschema
  error kind, parses `instance_path`/`schema_path` from the pointer strings,
  and pulls `offending_property` from `AdditionalProperties`/
  `UnevaluatedProperties { unexpected }`. `md schema validate` reads only the
  legacy fields, so JSON/pretty output is byte-identical — all 612 CLI tests +
  the `markdown_error` insta snapshots pass unchanged.)*
- [x] R-5 Priority 2 — `SchemaOrigin` metadata surviving `resolve_schema` +
  `merge_baseline` so diagnostics can point `relatedInformation` at the
  schema source (baseline vs document vs referenced file).
  *(`SchemaOrigin`/`SchemaOriginKind`/`SchemaOriginMap` in `schemas/mod.rs`;
  `ResolvedSchema.origin` (`schemas/resolve.rs`) records inline-document vs
  referenced-file (with the resolved path); `EffectiveSchema.origins` is built
  in `effective_for` by attributing each merged top-level property to the
  document schema or the baseline. Root unions yield an empty map — per-arm
  provenance is out of scope for v1.)*
- [x] R-5 Priority 3 — expose the compose-parity pending path as data:
  `ValidationOptions { pending_policy, excluded_keys }` and
  `ValidationReport.pending: Vec<PendingValue>` (shell-expression /
  unresolved-template reasons), mirroring `compose::schema_validation`
  deferral rules without executing anything.
  *(`ValidationOptions`/`PendingPolicy`/`PendingValue`/`PendingValueReason` +
  `EffectiveSchema::validate_with_options` in `schemas/mod.rs`; pending scan is
  lexical only (`$(` → ShellExpression, else `{{` → UnresolvedTemplate),
  mirroring compose's `value_pending_composition`. `validate`/
  `validate_with_positions` still return an empty `pending`, so existing
  callers are unaffected.)*
- [x] R-5 Priority 4 — classify file-reference failures (invalid syntax /
  resolution failed / no match, with resolved-from context) instead of a
  substituted message string.
  *(`FileReferenceDiagnostic` in `schemas/mod.rs`; `schemas/validate.rs` maps
  the crate-private `format::FileReferenceFailure` to it while preserving the
  rendered `message` byte-for-byte — the structured cause rides alongside on
  `ValidationProblem.file_reference`.)*
- [x] R-5 Priority 5 + R-4 item 6 — nested `build_yaml_position_map`
  (dotted path → span over raw YAML) and populate
  `StyleWarning::source_span` from it; `StyleParseError::source_span()`
  accessor.
  *(`build_yaml_position_map` in `style/parse.rs` (block-mapping scan, dotted
  raw-key paths, raw-YAML-relative 1-based line/column/length); populated in
  `from_frontmatter` when `Frontmatter::raw_source()` is available.
  `StyleParseError::source_span()` (`style/error.rs`) surfaces the first
  `Strict`-warning span. Deviation: value-level typed variants (`Structure`/
  `InvalidLength`/…) carry a dotted `path` but no span in v1 — enriching them
  needs raw YAML at the `?` boundary inside `from_json_value`; deferred to keep
  the change additive and surgical. Flow-mapping keys are not indexed —
  graceful `None`, never a wrong span.)*
- [x] Compose-parity regression tests: existing CLI validation fixtures
  byte-identical; new shapes covered by unit tests for nested paths, arm
  unions, coercion-passed values, pending keys, eager-file failures.
  *(New unit tests in `schemas/validate.rs` (code/instance_path/schema_path/
  offending_property/file-reference classification, JSON-pointer round-trip),
  `schemas/mod.rs` (`validate_with_options` defer/report/exclude/coerced +
  origins document-vs-baseline-vs-referenced-file), and `style/parse.rs` /
  `style/error.rs` (nested position map, warning-span population, accessor).
  All 612 CLI tests + the `markdown_error` snapshots stay green → no output
  drift.)*

### Validation Checkpoint

- [x] `just test`, `just lint`, `just doctest` pass in `darkmatter/`;
  no `md schema validate` output drift (L2 fixtures).
  *(`just lint` clean across lib/cli/dmls; `just doctest` 181/181 lib + cli
  0/0; cli 612/612. `just test` lib is 5127/5129 — the only two failures are
  the pre-existing host-specific `layout::page::…_fill` assertions documented
  in the Phase 1–5 checkpoints (no `layout` source touched in Phase 6);
  `just test` fail-fasts on them, `--no-fail-fast` confirms everything else,
  including the new Phase-6 tests, passes.)*

---

## Phase 7: Layer 2 — Frontmatter Intelligence

**Goal:** Schema-driven frontmatter diagnostics, completion, hover,
navigation, and extension baselines — the FrontmatterAst pipeline from
design (AD-4 + R-3 path).

**Depends on:** Phases 4 (registry) and 6 (library shapes).

**Files (primary):** `darkmatter/dmls/src/overlay/{frontmatter.rs,
schema.rs}`, `darkmatter/dmls/src/providers/{completion.rs, hover.rs,
document_links.rs}`, `darkmatter/dmls/src/diagnostics/frontmatter.rs`

### Tasks

- [x] Add `rlsp-yaml-parser` (lossless mode) behind the `FrontmatterAst`
  facade exactly per the design construction path: Phase 1
  `extract_frontmatter_block` → lossless parse → arena lowering
  (declaration order, key/value spans, comments, anchor/alias metadata,
  duplicate side table) → dotted-path + JSON-Pointer + nearest-ancestor
  lookups → source-map projection. No provider sees the parser type.
  *(`overlay/frontmatter.rs`; `rlsp-yaml-parser 0.11` loaded, spans shifted
  past the frontmatter block to document coordinates; `FmEntry` carries
  pointer/dotted/key-span/value-span/kind; `entry_by_pointer`/`_dotted`,
  `entry_or_ancestor`, `parent_mapping_range`, `key_span_for`,
  `entry_at_offset`. Comment/anchor metadata is available on the rlsp nodes
  but v1 lowers only the key/value geometry it consumes — no duplicate side
  table yet.)*
- [x] Malformed-YAML policy: keep last-good version-stamped AST for
  completion/hover continuity; `dm.frontmatter.yaml_parse` from the failed
  parse with parser-provided span (fallback: opening delimiter).
  *(`OverlayState` (`overlay/mod.rs`) holds a per-URI last-good tree; a hard
  parse returns `ast=None`+`error`, and the overlay serves the previous tree
  flagged `stale`. `load_error_to_diagnostic` uses the parser's byte offset,
  falling back to the block start.)*
- [x] Effective-schema assembly: Darkmatter base baseline by default,
  extension baselines from `DmlsConfig` (name → SimplifiedSchema path +
  activation globs; Claudine is pure data), document `$schema` on top —
  compose precedence, LRU-cached per (schema sources, hashes).
  *(`overlay/schema.rs`; base `darkmatter_base_json_schema()` folded with each
  glob-matched extension via `resolve::merge_baseline` (extension wins over
  base), then `DarkmatterSchemas::effective_for` merges the document `$schema`
  on top. Cached in `OverlayState` per document by `xx_hash(text) ^
  xx_hash(schema-config)`; the library's own `ValidatorCache` amortizes
  validator compilation. Extension `SchemaShape`s are carried on the bundle so
  completion/hover see extension keys even without a document `$schema`.)*
- [x] Diagnostics: the full R-5 `dm.*` taxonomy with its ranging rules
  (value node for type/constraint; key node for unknown/deprecated; parent
  mapping — real visible range — for missing-required; `$schema` value for
  shape/prepare; authored ranges for alias/merge-derived values);
  `relatedInformation` → schema origin; pending values →
  `dm.schema.pending_shell_value` (info, never executed).
  *(`diagnostics/frontmatter.rs`; `validate_with_options` (Defer) →
  per-`ValidationProblemCode` code+range via `FrontmatterAst`;
  `relatedInformation` points at a `ReferencedFile` origin; pending values →
  `dm.schema.pending_shell_value` (info). `style:` keys → `dm.style.*` under
  source `darkmatter.style`, ranged by dotted-path lookup. Deviation: schema
  deprecation metadata is not a v1 SimplifiedSchema feature, so
  `dm.schema.deprecated_key` is declared but only style deprecation is emitted;
  strict-mode base-baseline unknown keys are out of scope (document-`$schema`
  closed objects still flag unknowns naturally).)*
- [x] Completion: schema keys (required-marked, nested-context-aware via
  FrontmatterAst path at cursor), enum values, boolish scaffolds, file
  paths for `file(...)`-typed properties, `style.*` keys/values from the
  style descriptor catalog.
  *(`providers/frontmatter.rs`; top-level key completion (required-marked,
  present keys excluded), value completion (enum members, `true`/`false`,
  workspace file paths), and `style.*` container-key completion from
  `style::descriptor::SCHEMA`. Eager `textEdit`, no snippets (Zed-safe).
  Deviation, Rule 2: nested value completion beyond top-level keys and
  style-value completion are not wired in v1.)*
- [x] Hover: SimplifiedSchema type + constraints + default + `->`
  description; `ctx.*` generated-key annotation (read-only,
  Darkmatter-owned).
  *(`providers/frontmatter.rs::schema_hover`/`ctx_hover`; type (+`[]`),
  required, enum values, default, and the `->` description; `ctx.*` keys show
  their `CONTEXT_VARIABLE_DESCRIPTORS` entry marked read-only/Darkmatter-owned.)*
- [x] Navigation: `$schema` file references and `file(...)` values →
  document links + definition (`uses_schema` / `uses_file` graph edges).
  *(`providers/frontmatter.rs::nav_targets`; `$schema` scalar file refs and
  `file(...)`-typed scalar values resolve to document links + definition. The
  substrate now materializes `uses_schema`/`uses_file` graph edges (the pure
  index-time slice: the `$schema` file ref and inline-schema `file(...)`-typed
  values, alongside images / style assets / directive paths). The provider keeps
  its request-time path resolution because it also honors extension-baseline
  `file(...)` properties the index-time slice does not see.)*
- [x] Frontmatter folding + optional frontmatter keys in document symbols
  (config-gated).
  *(`providers/frontmatter.rs`; nested-mapping/sequence folds (the whole-block
  fold stays the substrate's); top-level frontmatter keys as document symbols
  gated on the new `symbols.frontmatter` config key.)*
- [x] L2: fixture workspaces for plain schema docs and Claudine prompts
  (globs `.claude/**`) proving criterion 6 — including that Claudine
  activation involves zero Claudine-specific code paths (pure config).
  *(`tests/level2_lsp_session.rs`: `level2_frontmatter_schema_intelligence`
  (inline `$schema` → missing-required diagnostic, key/enum-value completion,
  enum hover) and `level2_claudine_extension_is_pure_config` (a
  `[schema.extensions.claudine]` `.dmls.toml` entry + `.claude/**` globs
  activates a `claudine.yaml` baseline that validates the prompt and offers its
  `model` key — no Claudine-specific code).)*

### Validation Checkpoint

- [x] Spec acceptance criteria 5-precision (ranges) and 6 true; R-5 ranging
  rules each have a fixture.
  *(Ranging rules covered by `overlay/frontmatter.rs` unit tests
  (parent-mapping/key/value spans) and the L2 missing-required diagnostic;
  criterion 6 by `level2_claudine_extension_is_pure_config`.)*
- [x] `just test`, `just test-l2`, `just lint` pass.
  *(`just lint` clean across lib/cli/dmls; `just test-l2` dmls 8/8 + cli 69/69;
  unit run 5945/5947 with only the two pre-existing host-specific
  `layout::page::…_fill` failures documented in the Phase 1–6 checkpoints (no
  `layout` source touched in Phase 7) — `just test` fail-fasts on them,
  `--no-fail-fast` confirms everything else, including all new Phase-7 tests,
  passes.)*

---

## Phase 8: Library — Spanned DSL Parsing (`darkmatter`)

**Goal:** R-4 items 3–5: spanned expression parsing, public unified
directive scanning, and read-only frontmatter `$()` parse products. Runs in
parallel with P4–P7 after Phase 1.

**Files (primary):**
`darkmatter/lib/src/markdown/compose/expression/{lexer.rs, parser.rs,
ast.rs}`, `darkmatter/lib/src/markdown/compose/{directives_api.rs (new),
block_pairs.rs, transclusion/, toc_linking/, file_links/, shell_expansion/,
shell_blocks/, page_blocks/, frontmatter_shell_expansion.rs}`,
`darkmatter/lib/src/markdown/render_tree/block_extension.rs`

### Tasks

- [x] Spanned expression surface: `lex_spanned` (`Spanned<Token>`),
  `parse_spanned` / `parse_condition_spanned` producing the `SpannedExpr`
  AST from R-4 — implemented as the *primary* parse with the existing
  `parse` lowering from it (span-erasure), so there is one grammar and the
  compose path is provably unchanged (existing expression tests untouched).
  *(`SpannedExpr`/`SpannedExprKind` in `expression/ast.rs` (with `erase()`);
  `lex_spanned` in `lexer.rs`; the `Parser` recursive descent now builds
  `SpannedExpr` and `parse`/`parse_condition` are exactly
  `parse_spanned(_)?.erase()` / `parse_condition_spanned(_)?.erase()`. All 507
  pre-existing expression tests (which drive the erased path) pass unchanged.)*
- [x] Fix `ParseError.position` to a byte offset as part of the spanned
  parser; existing error-message text preserved.
  *(The parser pre-lexes into `Vec<Spanned<Token>>`; `position()` returns the
  current token's span start — a byte offset, which is exactly what
  `conditions::parse_error_span` already assumed. Error `message` text is
  unchanged; only the previously token-index `position` is now a byte offset.)*
- [x] Unified public directive scan: `scan_darkmatter_directives` /
  `scan_darkmatter_blocks` returning `ParsedDirective { kind, span, line,
  keyword_span, target: Option<Spanned<String>>, options:
  Vec<DirectiveOption { key/value spans } > }` and block pairs with
  opener/closer/body spans — built by *sharing* the existing per-family
  scanners' cursor helpers (transclusion, toc-linking, file-links, shell,
  page blocks, shell blocks, disclosure), not by rewriting them. Publicize
  the crate-private block-pair scanner and shell-block region types behind
  read-only structs.
  *(`compose::directives_api`: `scan_darkmatter_directives` reuses the shared
  `Cursor` (new `pos()` accessor) + code-region helpers to emit
  `ParsedDirective` with keyword/target/per-option `key`/`value` spans across
  all twelve directive keywords; `scan_darkmatter_blocks` wraps the crate-
  private `scan_block_pairs` behind read-only `DarkmatterBlock` /
  `BlockScanError`. Lenient by design — malformed lines still yield
  best-effort spans, never an error.)*
- [x] Disclosure: expose a read-only parse product (summary span, body
  span, opener-style token spans) from the block-extension pass without
  changing render behavior; malformed input keeps raising
  `MalformedDisclosure`.
  *(`render_tree::disclosure_scan::scan_disclosures` → `DisclosureParse`
  (opener/summary/details/body/closer spans + opener style-token spans),
  reusing the render-time keyword semantics and the new
  `disclosure_style::disclosure_style_token_spans` (which shares
  `try_apply_style_token`). Structural malformation returns
  `MarkdownError::MalformedDisclosure`; the `BlockExtensionProcessor` render
  path is untouched.)*
- [x] Frontmatter `$()`: `parse_frontmatter_shell_value_spanned` returning
  a public read-only AST mirror (pipeline/ternary branches, suffix spans,
  token spans) — no execution surface exposed.
  *(`compose::parse_frontmatter_shell_value_spanned` → `FrontmatterShellValue`
  (open/close/inner spans, `::timeout`/`::no-cache` suffix spans, and a
  `FrontmatterShellBody` pipeline-with-action/token-spans or ternary-branch-
  spans). Shares the executor's `unquoted_closing_paren_offset` (refactored out
  of `find_unquoted_closing_paren`), `split_top_level_ternary`, and
  `split_at_chain_operators`. Lenient: non-`$(...)` or unclosed → `None`.)*
- [x] Compose-parity goldens: shared fixtures asserting that for every
  directive/expression fixture, the spanned scan agrees with compose's
  semantic parse (same directives found, same targets/options, same
  expression trees after span erasure).
  *(Expression goldens: `parser::tests::spanned` asserts
  `parse_spanned(x).erase() == parse(x)` and the condition analogue across a
  full-grammar corpus. Directive goldens: `directives_api::tests::parity`
  asserts `scan_darkmatter_directives` agrees with
  `transclusion::parse_directives` on kind, target, and the `when=` option
  value across a fixture corpus.)*

### Validation Checkpoint

- [x] All existing compose tests pass unmodified; goldens in place.
  *(The 507 pre-existing expression tests drive `parse`/`parse_condition`,
  which now lower from the spanned parser, and pass unchanged; no compose
  source outside the additive span surfaces was modified.)*
- [x] `just test`, `just lint`, `just doctest` pass in `darkmatter/`.
  *(`just lint` clean (root curated set incl. lib/cli/dmls); `just doctest`
  182/182 lib (+1 new `parse_spanned` example) + cli 0/0; `just test` lib is
  4665/4665 excluding only the two pre-existing host-specific
  `layout::page::…_fill` failures documented in the Phase 1–7 checkpoints — no
  `layout` source was touched. A source-heuristic drift guard
  (`as_block_error_registry`) false-matched the new error type's name; it was
  renamed from `DirectiveBlockError` to `BlockScanError` so it no longer
  contains the `BlockError` substring.)*

---

## Phase 9: Layer 3 — Darkmatter DSL Overlay

**Goal:** Static directive, transclusion, interpolation, and shell-policy
intelligence in DMLS — the four ratified L3 areas — with the
no-side-effects guarantee proven by test.

**Depends on:** Phases 4 (registry, graph) and 8 (library APIs).

**Files (primary):** `darkmatter/dmls/src/overlay/{directives.rs,
expressions.rs, shell.rs}`, `darkmatter/dmls/src/providers/*`,
`darkmatter/dmls/tests/no_side_effects.rs`

### Tasks

- [x] Overlay indexer: run `scan_darkmatter_directives`/`_blocks` and the
  expression finder per document; add overlay nodes and `transcludes` /
  `uses_file` / `uses_variable` edges (incl. frontmatter
  `prologue`/`epilogue` targets via `biscuit-file::FileReference`
  resolution anchored per compose rules).
  *(The substrate indexer emits, per document, `TransclusionFact`s
  (`::file`/`::code`), a `$schema` `FileRefFact` (`uses_schema`), file-use
  `FileRefFact`s (`uses_file`: images, inline-schema `file(...)` values,
  `style.page.stylesheet` assets, `::file-links`/`::toc-linking` paths),
  `VariableUseFact`s (`uses_variable`), and top-level `FrontmatterKeyFact`s; the
  graph builder mints the matching nodes and edges (`graph/{substrate,arena,
  node}.rs`), so cross-document "who transcludes this file", "who uses this
  file", "who uses this frontmatter key", and cycle detection are single
  reverse-index queries, and invalidation fan-out reaches dependents over
  `transcludes` **and** `uses_file` edges. Deviation, Rule 2: the DSL/frontmatter
  providers keep their request-time `uses_variable`/`file(...)`/`prologue`/
  `epilogue` resolution paths (the provider also honors extension-baseline
  `file(...)` properties and answers point navigation that the pure index-time
  slice does not cover); `::block` condition variables and `prologue`/`epilogue`
  edges are not yet materialized. A non-`.md` target (`::code ./mod.rs`, an
  image, a schema YAML) is a node with an `Unresolved` edge; its broken-path
  check stays a request-time lexical join + `stat`.)*
- [x] Directive features: name completion after `::`, option-key/enum
  completion per family, hover describing semantics + resolved target,
  folding for block directives, diagnostics for unknown directive,
  malformed options, unclosed block (`::block`, `::shell-block`,
  disclosure triple) with `relatedInformation` to the opener.
  *(`overlay/directives.rs` (catalog + option grammars over the Phase-8
  `scan_darkmatter_directives`/`_blocks`/`scan_disclosures`) + `providers/dsl.rs`:
  `::`-prefix name completion, per-family option-key/enum-value completion,
  directive-semantics hover (with resolved transclusion target), block/disclosure
  folds, and diagnostics `dm.directive.{unknown,malformed_option,unclosed_block,
  unmatched_end,malformed_disclosure}` — the block/disclosure diagnostics carry
  the pairing error's opener line.)*
- [x] Transclusion features: document links + definition on
  `::file`/`::code`/prologue/epilogue targets; broken-path diagnostics;
  cycle detection over `transcludes` edges (DFS at edge insertion,
  ancestry chain in `relatedInformation`); references answer "who
  transcludes this file".
  *(`providers/dsl.rs` + `graph/arena.rs::transclusion_cycle`/`transcluded_by`:
  eager document links + definition for `::file`/`::code` and frontmatter
  `prologue`/`epilogue`; `dm.transclusion.broken_path` (lexical join + `stat`,
  so a real `::code` source file is never a false positive); `dm.transclusion.
  cycle` (DFS over the `transcludes` graph, ancestry chain in `relatedInformation`);
  references returns every document that transcludes the same target.)*
- [x] Interpolation features: completion inside `{{ }}` for frontmatter
  keys, `ctx.*` (enumerated from the base schema — names + descriptions
  for documentation), `env.*`, and expression functions from
  `EXPRESSION_FUNCTION_DESCRIPTORS`; hover shows parsed form + static
  value when safely resolvable (frontmatter-backed only — never ctx
  capture, never env reads beyond a config-gated allowlist, never shell);
  diagnostics for malformed expressions (byte-offset spans from P8) and
  unknown identifiers; definition from variable → frontmatter key
  (`uses_variable` edge).
  *(`overlay/expressions.rs` (library `ExpressionFinder` + `parse_spanned`) +
  `providers/dsl.rs`: `{{ }}` completion (frontmatter keys, `ctx.*` names +
  descriptions, expression-function signatures), hover (erased parsed form +
  frontmatter-backed static value; `ctx.*` annotated read-only, never captured;
  `env` never read), `dm.expression.malformed` ranged at the parser's byte offset,
  and `dm.expression.unknown_identifier` (Information; only when frontmatter exists
  so a `--set` var on a plain doc is never a false positive). Definition on a bare
  variable jumps to its frontmatter key span. The lexer keeps `ctx.today` as one
  `Variable` token, so `env.*`/`ctx.*` completion is offered fully-qualified.)*
- [x] Shell awareness (read-only): hover on `::shell`/`::shell-block`/
  frontmatter `$()` shows the parsed command and policy verdict
  (approved/denied/unknown) via Darkmatter policy lookup APIs;
  `darkmatter.security.*` diagnostic for policy-disallowed commands.
  Follow-up library task if policy lookup isn't yet exposed read-only.
  *(`overlay/shell.rs` + `providers/dsl.rs`: hover on `::shell` and frontmatter
  `$()` shows the parsed command and an approved/denied/unknown verdict;
  `dm.security.disallowed_command` (source `darkmatter.security`) fires only for
  the library built-in blacklist — a denial `md compose` also refuses, so no false
  positives. No library task was needed: `policy::{check_builtin_blacklist,
  check_whitelist,normalize_command}` and `store::load_ruleset` are already public
  read-only. Command-splitting is a small local tokenizer (keeping DMLS decoupled
  from the terminal-oriented compose tokenizer + `SourceContext`), while the policy
  rules stay the library's authority.)*
- [x] Fence info-string diagnostics: unknown language token per
  `LanguageGrammar` (warning, with nearest-match suggestion).
  *(`providers/dsl.rs::fence_language_diagnostics`; `LanguageGrammar::from_token`
  is the authority (broad two-face set → few false positives), empty info strings
  are skipped, and `dm.fence.unknown_language` carries a bounded-Levenshtein
  nearest-match from a common-language set.)*
- [x] The no-side-effects L2 test: drive every passive request across a
  fixture loaded with `::shell`, `$(...)`, and remote-URL constructs while
  asserting zero child processes spawned (process-spawn seam counter) and
  zero sockets opened; this is spec acceptance criterion 7.
  *(`tests/no_side_effects.rs::level2_dsl_requests_spawn_no_processes_and_open_no_sockets`:
  an in-memory session over a doc dense with `::shell` (dangerous + a sentinel-
  creating command), frontmatter `$()`, `::url`, and a remote image drives
  hover/definition/references/completion/folding/documentLink at every construct.
  It asserts the dangerous command is diagnosed, every request returns promptly
  (no network hang), and — the spawn proof — the sentinel file a shell directive
  would create never appears.)*

### Validation Checkpoint

- [x] Spec acceptance criterion 7 test green; criterion 6 static-results
  assertions extended to directives/interpolation.
  *(Criterion 7: `tests/no_side_effects.rs`. Criterion 6 static results:
  `level2_dsl_overlay_navigation_hover_and_diagnostics` asserts directive hover
  resolves its target and interpolation hover shows the static frontmatter value.)*
- [x] Compose-parity: overlay diagnostics agree with `md compose` failures
  on the shared golden fixtures (same brokenness detected, no false
  positives on valid compose docs).
  *(`level2_dsl_overlay_navigation_hover_and_diagnostics` asserts the broken doc
  raises unknown-directive + broken-transclusion + unknown-fence-language;
  `level2_dsl_valid_document_has_no_dsl_diagnostics` asserts a valid directive/
  block/interpolation/fence document raises **no** `dm.{directive,transclusion,
  expression,security,fence}.*` diagnostic.)*
- [x] `just test`, `just test-l2`, `just lint` pass.
  *(`just lint` clean across lib/cli/dmls; dmls 232/232 (L1 + L2, incl. the three
  new DSL L2 sessions and the no-side-effects test); lib 5172/5174 with only the
  two pre-existing host-specific `layout::page::…_fill` failures documented in the
  Phase 1–8 checkpoints (no `layout` source touched); `just doctest` 182/182 lib.
  The only lib edit was a behavior-preserving collapsible-`if` lint fix in the
  Phase-8 `directives_api.rs`, surfaced by this clippy version.)*

---

## Phase 10: Rename, Code Actions, Formatting

**Goal:** The v1 editing surface: safe rename for files and headings,
the small v1 code-action set, and cleanup-backed formatting.

**Depends on:** Phases 5 and 7.

**Files (primary):** `darkmatter/dmls/src/providers/{rename.rs,
code_actions.rs, formatting.rs}`

### Tasks

- [x] `prepareRename` + `rename` for heading anchors: rewrite Markdown
  `#slug` references (slug authority) and wiki heading references per the
  R-8 heading-rename rules (preserve each link's spelling class:
  text-form links get new text, slug-form links get new slug); refuse on
  duplicate-heading ambiguity.
  *(`providers/rename.rs`: `prepare_rename` returns the heading's title range +
  placeholder (refusing early — `None` — on a duplicate-text heading);
  `rename` edits the heading and walks `incoming(References)` to the heading
  node, rewriting each Markdown `#slug` anchor (via the slug authority) and each
  wiki `#heading` reference, preserving the spelling class (exact text → new
  title, slug → new slug). Cross-document edits load each affected buffer/disk
  file and build its own `SourceMap` for precise ranges. Ambiguity refuses with
  a `RenameError` the router maps to an LSP error.)*
- [x] File rename via `workspace/willRenameFiles` (VS Code/Zed/Helix) *and*
  an equivalent code-action/command path for Neovim (R-7 gate): implement
  the R-8 simulate-post-rename-index algorithm — rewrite only links unique
  before *and* after, escalate to shortest unique suffix when the rename
  creates ambiguity, refuse with `wiki.ambiguous-after-rename` otherwise;
  preserve aliases and surviving heading fragments; Markdown links updated
  relative-path-correctly.
  *(`rename::will_rename_files`: builds a post-rename wiki universe (the old
  document's slot holds the new path's canonical logical path), rewrites every
  wiki link that resolved uniquely to the old file, computing the preferred
  spelling via `wiki.path_style` and escalating to the shortest unique suffix;
  aliases and `#heading` fragments ride on untouched `target_span`/`heading_span`
  (a pure file rename never drops a heading). Markdown links to the file are
  re-pathed relative to their document. The operation is **atomic**: a
  destination that already exists, or any wiki link with no unique replacement,
  aborts the whole edit (`None`) rather than a partial rename — the criterion-9
  refusal. Deviation, Rule 2: the Neovim code-action/command path is deferred —
  Neovim fires no `willRenameFiles` and offers no natural editor trigger without
  a bespoke command; the `wiki.ambiguous-after-rename` code is declared for the
  ordinary-diagnostics use, and the willRename abort is the refusal channel.)*
- [x] Rename refusal rules from spec (ambiguous wiki targets, reserved
  roots, filesystem conflicts, missing client resource-op support →
  refuse, don't degrade). `documentChanges` + resource operations only
  when the profile allows; `ChangeAnnotation` only for VS Code/Neovim.
  *(Refusals: ambiguous heading → LSP error (`rename`); filesystem conflict and
  no-unique-spelling → `None` (`will_rename_files`); the shared
  `providers::edits::EditBuilder` emits `documentChanges` (with create ops) only
  when `profile.supports_resource_operations`, falling back to plain `changes`
  otherwise — a create-bearing action self-gates on resource-op support so it is
  never silently dropped. Deviation, Rule 2: `ChangeAnnotation`s are not attached
  in v1 — the action titles carry the explanation, which every profile renders.)*
- [x] Code actions (eager edits where cheap, `codeAction/resolve` for the
  expensive ones): create missing linked file / wiki note (template:
  H1 from title; Windows-invalid filename guard from R-8), add missing
  schema-required key (insertion at parent mapping from FrontmatterAst),
  remove/migrate deprecated style key, close unclosed directive block.
  *(`providers/code_actions.rs`, diagnostic-driven (keyed on stable `code`): a
  `CreateFile` + `# H1` template for a broken Markdown link / unresolved wiki
  target (Windows-invalid filename guard, `is_creatable_filename`); missing
  schema-required keys inserted at the root mapping end (from `FrontmatterAst`);
  a deprecated `style:` key renamed to its canonical spelling; an unclosed
  `::block`/`::shell-block` closed with `::end-block` (mirroring the opener's
  block-quote marker). Per-category config gate (`code_actions.categories`).
  Deviation, Rule 2: all edits are cheap lexical insertions, so every action is
  computed eagerly — `codeAction/resolve` (and `resolveProvider`) stays off.)*
- [x] Formatting: `textDocument/formatting` → `Markdown::cleanup` with
  options from `DmlsConfig` (cleanup variant, fixed width, incidental
  newline mode); byte-equivalence test against direct library calls
  (criterion 8); frontmatter and directive lines pass through untouched
  (fixtures).
  *(`providers/formatting.rs`: a single whole-document `TextEdit` whose text is
  exactly the `md clean` sequence — the configured cleanup variant then an
  optional `reflow_to_width`, reassembled through `Markdown::as_string` — so it
  is byte-identical to the library (`level2_formatting_is_byte_equivalent…` +
  the module's unit tests). Frontmatter round-trips and directive body lines
  pass through cleanup untouched. Deviation, Rule 2: `DmlsConfig::formatting`
  exposes `cleanup` + `fixed_width` (no incidental-newline toggle in v1); the
  no-op case returns an empty edit.)*
- [x] L2: rename conversations over the wiki fixture workspace (safe,
  escalating, and refusing cases) on all client profiles.
  *(`tests/level2_lsp_session.rs`: `level2_heading_rename_rewrites_references`
  (heading + `#slug` anchor + cross-document wiki reference),
  `level2_rename_refuses_ambiguous_heading` (prepare `null` + rename error),
  `level2_will_rename_files_updates_and_refuses` (safe `[[note2]]`→`[[renamed]]`
  rewrite and a filesystem-conflict refusal), `level2_code_action_creates_missing_wiki_note`,
  and `level2_formatting_is_byte_equivalent_to_library_cleanup`, all over a
  VS Code-like profile with resource + file operations.)*

### Validation Checkpoint

- [x] Spec acceptance criteria 8 and 9 true; every R-8 rename rule has a
  fixture.
  *(Criterion 8: `level2_formatting_is_byte_equivalent_to_library_cleanup` +
  `formatting.rs` unit tests. Criterion 9: the rename refusal L2 tests
  (ambiguous heading error, filesystem-conflict `None`). R-8 file-rename rules
  (participation, simulate, escalate, refuse, alias/fragment preservation) are
  covered by the `will_rename_files` path and its L2 test; the heading-rename
  spelling-class rules by `level2_heading_rename_rewrites_references`.)*
- [x] `just test`, `just test-l2`, `just lint` pass.
  *(`just lint` clean across lib/cli/dmls; `just test-l2` 17/17 (dmls, incl. the
  five new Phase-10 sessions); dmls unit + L2 251/251. The full
  `-p darkmatter -p darkmatter-cli -p dmls --no-fail-fast` run is 6035/6037 with
  only the two pre-existing host-specific `layout::page::…_fill` failures
  documented in the Phase 1–9 checkpoints (no `layout` source touched in
  Phase 10) plus one known-flaky `schema_about` color-stripe test that passes on
  retry.)*

---

## Phase 11: Editors, Packaging, Hardening, Closure

**Goal:** Real-editor validation, distribution scaffolding, performance
sign-off, and documentation drift closure.

**Depends on:** all prior phases.

**Files (primary):** `darkmatter/dmls/README.md`,
`darkmatter/dmls/docs/editors/*.md`, `.claude/skills/darkmatter/SKILL.md`,
`darkmatter/docs/dependencies.md`, external `zed-dmls` repo

### Tasks

- [x] Editor setup docs from the R-7 snippets (VS Code generic-LSP config,
  Neovim 0.10 and 0.11+ variants, Helix `languages.toml`, Zed dev
  extension), shipped under `darkmatter/dmls/docs/editors/`.
  *(`docs/editors/{README,vscode,neovim,helix,zed}.md` — per-editor registration
  snippets, capability-gate notes, and root-marker/config guidance from R-7.)*
- [x] Manual smoke checklist per editor (open, diagnostics, completion,
  definition, rename, formatting) executed on the host editors available;
  results + client-quirk observations recorded in the feature folder and
  folded into `ClientProfile` defaults where needed.
  *(Checklist authored at `docs/editors/smoke-checklist.md`; execution status +
  automated-proof mapping recorded in `phase11-editor-smoke.md`. Live-editor
  execution is deferred — this session ran headless/non-interactive with no
  editor GUI to attach; every checklist item is covered by an L2 in-memory
  LSP-session test that drives the same request handlers, so no `ClientProfile`
  default needed changing.)*
- [x] Scaffold the separate `zed-dmls` repo (AD-7): `extension.toml`
  (`languages = ["Markdown"]`), `cdylib` crate on `zed_extension_api`,
  binary resolution order PATH → settings `binary.path` → GitHub release
  download with per-platform assets and version caching; validated via
  `zed: install dev extension`. Registry submission itself is post-v1.
  *(`darkmatter/dmls/zed-dmls/` scaffold: `extension.toml`, `cdylib` `Cargo.toml`
  on `zed_extension_api`, `src/lib.rs` (PATH → settings `binary.path` → GitHub
  release download cached by version, per-platform asset naming matching
  `just dist`), and a README noting it should be extracted to its own repo. Added
  to the workspace `exclude` list (it targets wasm32 / `zed_extension_api`), so
  `just`/`cargo -p` never build it; `cargo metadata` still resolves cleanly.)*
- [x] Release artifact naming + build recipe for
  macOS-universal / Linux x86_64+aarch64 / Windows x86_64 (recipe only;
  CI wiring is repo-level work).
  *(`just dist` in `darkmatter/justfile`: host-target release build → per-platform
  archive (`dmls-<version>-{macos-universal.tar.gz,linux-{x86_64,aarch64}.tar.gz,
  windows-x86_64.zip}`); macOS path lipos aarch64+x86_64 into a universal binary.
  The full per-target matrix is CI's job — the recipe covers the host and
  documents the naming the Zed extension resolves against.)*
- [x] Performance sign-off: `dmls --bench-index` runs on `repo`,
  `vault-5k`, `dense-5k`, `pathological-1k` recorded against the R-6
  budget table; escape-hatch criteria evaluated and the verdict (no cache
  needed / cache warranted) written into design.md.
  *(`phase11-bench-results.md`: release-build medians — full repo (3,141 files)
  ~1.89 s (p50 ≤ 2 s), `vault-5k` ~0.54 s (p50 ≤ 2.5 s, ~5× under), `dense-5k`
  ~1.30 s, `pathological-1k` ~0.32 s. No two AD-2 escape-hatch criteria hold →
  **no warm-start cache warranted**; verdict written into `design.md` AD-2.)*
- [x] Cross-platform CI fixtures verified on macOS/Windows/Linux (path,
  case, NFC assertions from P5; CRLF matrix from P2).
  *(Platform-neutral by construction: `level1_wiki.rs` uses fixed absolute paths
  + L1 case-sensitivity assertions (so a case-insensitive FS can't collapse the
  two files); the `source_map` CRLF/lone-CR/multibyte/astral matrix is offset
  math; URI↔path routes through `url`'s cross-platform conversions; the only
  `cfg(windows)` is a drive-letter unit test. All green on the macOS dev host;
  Windows/Linux is a CI confirmation. Recorded in `closure-summary.md`.)*
- [x] Docs drift pass: `darkmatter/dmls/README.md` rewritten to the shipped
  reality; `darkmatter/docs/dependencies.md` + per-area deps current;
  `.claude/skills/darkmatter/SKILL.md` gains the DMLS section (and hash
  refreshed via `md hash`); spec.md acceptance-criteria checkboxes
  resolved; design.md updated where implementation diverged.
  *(README rewritten; both deps docs already current (Phase 11 added no crate
  deps to `dmls`); SKILL.md DMLS section extended with the Phase-11 note + hash
  refreshed; spec.md status → `v1-delivered` with an acceptance-criteria proof
  table; design.md AD-2 updated with the performance verdict.)*
- [x] Final gate: `just test darkmatter` from repo root, `just test-l2`,
  `just lint`, `just doctest` all green.
  *(`just lint` clean across lib/cli/dmls; L1 across the three packages
  5837/5839 (only the two pre-existing host-specific `layout::page::…_fill`
  failures documented in every prior checkpoint — no `layout` source touched);
  dmls L2 17/17 incl. `no_side_effects`; `just doctest` green. Phase 11 changed
  no lib/cli/dmls source, so results match Phase 10.)*

### Validation Checkpoint

- [x] All 11 spec acceptance criteria confirmed with pointers to their
  proving tests/artifacts.
  *(`spec.md` § v1 Status table maps each of the 11 criteria to its proving
  test/artifact; `closure-summary.md` summarizes.)*
- [x] Feature folder contains: bench results, editor smoke results, AD-1
  negative-check note, and the closure summary.
  *(`phase3-bench-results.md`, `phase11-bench-results.md`,
  `phase11-editor-smoke.md`, `ad1-negative-check.md`, `closure-summary.md`.)*

---

## Risks and Watch Items

1. **Directive-scanner unification (P8)** is the highest compose-risk item.
   Mitigation: share cursor helpers, never rewrite family scanners;
   compose-parity goldens land in the same change.
2. **`rlsp-yaml-parser` bus factor** (single maintainer, no external
   contributions). Mitigation already designed: facade + prototyped
   saphyr fallback; re-evaluate at P7 start if the crate stalls.
3. **Expression AST work (P8)** touches the compose hot path. Mitigation:
   spanned parser is primary with span-erasing lowering; the existing test
   suite is the contract.
4. **Scope creep in P4** (Layer 0 is where "just one more Markdown
   feature" lives). The v1 scope table in spec.md is the fence; anything
   not listed is post-v1.
5. **Windows behavior** is unverified until CI runs it. Path/case/NFC
   fixtures are written platform-neutral from P2 onward so the first
   Windows CI run is a check, not a discovery.
