## Overview

The `darkmatter` branch combined four overlapping bodies of work: a broad performance follow-up whose implementation largely predated the branch split, a focused removal of redundant reference-graph verification, a progressively widened correction to list-aware cleanup and fixed-width reflow, and a cross-package invalid-frontmatter repair pipeline. The post-fork history is therefore not four clean, independent streams. In particular, fixed-width cleanup review findings expanded into list indentation, blockquote ownership, and opaque Darkmatter directive protection; invalid-frontmatter introduced a reusable YAML analysis layer in `biscuit-file`; and redundant-walk added a graph-owned heading snapshot so fresh validation remained coherent without weakening caller-supplied graph checks. The status at `14dd391f45206d58383ba9d84adbf53c65520534` is mixed: fixed-width lists and redundant-walk were reviewed as production-ready, while performance-followup and invalid-frontmatter still carry evidence gaps described below.

### `2026-07-15-performance-followup`

This feature audited all 35 findings from the earlier performance review and closed implementation, compatibility, measurement, and evidence gaps without reopening work already proven complete. Its goals included restoring the bare Sniff timezone API's full behavior while retaining Darkmatter's no-NTP call; proving terminal OSC 10 caching and single terminal discovery; replacing incomparable TOC measurements with identical hashed fixtures; reducing frontmatter interpolation, expression, replacement, shell, remote-discovery, rendering, hashing, and copy costs; restoring directory-hash membership; and giving every remaining finding an implemented, reverted, separated, or evidence-backed no-win disposition. Compatibility was a hard constraint: Markdown, validation, rendering, graph/CLI JSON, diagnostics, exit status, shell ordering, cache identity, and owned public facades were to remain stable across macOS, Linux, and Windows.

Acceptance required all retained findings to have honest dispositions, Finding 22's directory-membership regression to be reverted, Finding 18's graph correctness work to remain owned by the separate opaque-reference-graph feature, same-byte benchmark artifacts to meet predeclared thresholds, scoped behavioral/L1/L2/browser/lint/platform gates to pass, and the feature-local manifest and typed identity architecture to be in place. The implementation findings were dispositioned and substantial wins were retained, including roughly 27x faster replacement matching, an approximately 104x skipped-work improvement for interpolation-free bodies, and an 82.5% remote-discovery improvement. However, Review 10 still classified the feature as not production-ready: the integrated compose-regression gate needs two admissible quiet-host captures, and repeated attempts were rejected because the one-minute host load far exceeded the predeclared ceiling of 2.0. Thus acceptance criteria 5 and 6 remain open; no benchmark was run during the high-contention periods.

Packages and modules changed over the feature's lifetime:

- `darkmatter` library: `markdown::compose` cache identity/runtime, context/options/runtime, frontmatter interpolation, expressions, replacement, conditions, shell expansion, remote discovery/fetch, transclusion, and pipeline phases; `markdown::hash`, `markdown::fs`, `markdown::toc`, reference/provenance support, render-tree code/build context, page layout, performance harnesses, and Criterion benches.
- `darkmatter-cli`: compose terminal-detection coverage, hash orchestration and directory-hash behavior, plus focused CLI tests and public hash documentation.
- `sniff`: `os::time`, restoring the public timezone/NTP compatibility boundary while exposing a test seam.
- `biscuit-terminal`: OSC-query discovery/caching, tracing-based query-count evidence, PTY/real-emulator tests, and the package-area L2 recipe.
- Feature-local benchmark fixtures, manifests, raw samples, recomputation/report scripts, specifications, reviews, results, and compliance records.

### `2026-07-16-redundant-walk`

This fix separated two trust boundaries that had incorrectly shared one expensive path. `Markdown::validate_references` and `FileTree::ensure_built` construct a graph and immediately consume that coherent snapshot, so they should use an internal fresh-graph seam without reopening and rehashing every visited descendant. By contrast, `Markdown::validate_references_with_graph` accepts caller-supplied state and must remain fail-closed: it still verifies root document/source identity, full mode, graph options, and every visited local descendant before flattening. Both routes were required to converge on one validation/report engine, preserve all public signatures and serialized views, and prove the difference with a changed-child test.

The initial estimate attributed much of a roughly 4.15 ms validation floor to the redundant walk, but same-session decomposition corrected that premise: the verification walk was closer to 159 microseconds, about 1.5% of the measured path. The amended acceptance guard therefore required mechanism proof, at least a 100-microsecond improvement, no fixture regression, and preservation of the prebuilt-path gap. Those guards passed. Later review also found fragment validation could reread a transcluded child's headings after graph construction; `PreparedHeadingSnapshot` was added privately to `ReferenceGraph`, populated from build-time heading data, omitted from `Debug` and JSON views, and used to keep fresh validation snapshot-coherent while checked reuse still rejects stale graphs. Review 7 marked the fix production-ready, leaving only a low-severity cache-first optimization for descendant `path#fragment` targets.

Packages and modules changed:

- `darkmatter` library only in production code: `markdown::reference::validate`, the public reference facade, graph construction/types, the new `snapshot` module, and `reference::file_tree` model/build routing.
- `darkmatter` reference-graph Criterion benchmark and focused reference/file-tree tests.
- `darkmatter-cli` and `dmls` were downstream verification scope through the area recipes, but their production modules were not changed for this fix.
- The fix's specification, plan, evidence, results, seven review cycles, and implementation log.

### `2026-07-13-fixed-width-lists`

This fix made cleanup treat prose inside ordered, unordered, task, nested, and blockquoted list items with the same incidental-newline contract as top-level prose. Default cleanup must remove eligible source soft breaks and their continuation-layout prefix; preserve mode must leave them untouched; fixed-width cleanup must first reconstruct a complete logical prose block and then wrap it so every synthesized continuation carries the full list/blockquote container prefix in Unicode display columns. Paragraph, sibling-item, nested-list, hard-break, code, table, HTML, directive, and other protected-block boundaries must remain structurally intact, and direct library cleanup, compose inline-post cleanup, `md clean` stdout/`--save`, and DMLS formatting must agree and remain idempotent.

Acceptance covered all three modes, nested and blockquoted lists, changing ordered-marker widths, task boxes, configured indentation, per-line width limits except documented atomic overflows, structural fingerprints, stable list-spacing modes, no public API or CLI-schema change, fixed parse counts, targeted performance evidence, and area build/L1/L2/lint gates. Review cycles widened the implementation beyond the spec's originally expected `reflow`-only surface: wide markers and configured indentation required a stack-based and then parser-derived list context; blockquoted task lists required separating syntactic marker width from display prefixes; and shell-block payload corruption required a shared opaque-body model. The final implementation derives opaque ownership from the existing Darkmatter block-pair scanner, including mixed page/shell nesting, keyword boundaries, quoted ownership, code exclusions, and source-preserving fallback for malformed structures. Review 6 marked the fix production-ready. Its baseline/candidate/baseline Criterion vector remains deferred because the shared host was too busy, and the review explicitly treats that missing performance claim as non-blocking.

Packages and modules changed:

- `darkmatter` library: `markdown::cleanup::{mod,reflow,reflow::semantic,lists,blockquote,opaque,perf_profile}`, cleanup tests and parse counters, `Markdown` cleanup routing, compose inline-post routing/tests, and the `clean_hot_paths` benchmark.
- `darkmatter-cli`: clean command/argument parsing and documentation, cleanup tests, and the shared tmux-backed L2 harness routing.
- `dmls`: formatting-provider parity and structural-integrity tests; no separate formatting algorithm was introduced.
- Darkmatter public/skill documentation and the earlier fixed-line-length specification were updated to describe the corrected cleanup contract.
- The fix's specification, plan, six review cycles, deferred-performance record, and implementation log.

### `2026-07-14-invalid-frontmatter`

This feature folded YAML validation and safe repair into `md clean` for frontmatter only; fenced YAML in the Markdown body remains intentionally untouched. Its reusable foundation is source-first and lives in `biscuit-file`: it analyzes parseable and unparseable YAML, retains structured locations and authored source, produces uniform diagnostics/repair candidates, applies non-overlapping UTF-8-safe edits from end to start, and supports deterministic normalization, whitespace cleanup, bounded reserved-indicator recovery, duplicate/anchor/multi-document detection, and report-only linting. Darkmatter adds schema-aware analysis, principally schema-proven scalar quoting, and the CLI composes baseline, trigger, document, and explicit schema choices once per clean invocation.

The ratified acceptance matrix requires S1 parse-equivalent normalization, S2 schema-proven invalid-to-valid quoting, S3 bounded invalid-YAML reserved-indicator recovery, and safe combined edits; all less-certain findings must be report-only. Untouched bytes and delimiters must be preserved, repairs must be idempotent, authored byte/line/column spans must remain correct after earlier length-changing edits, and `--json` must emit the stable version-1 envelope as the sole stdout payload. No-frontmatter input must perform zero YAML/schema/trigger work, clean frontmatter must parse once, the corpus must include a pinned YAML Test Suite subset plus mutation/property invariants, and macOS/Linux/Windows package gates plus no-regression timings must be retained.

The post-fork review cycles fixed important integration gaps: `md clean` was wired to the analyzer, schema safety began detecting deliberate scalar type changes instead of requiring value equality, shared analysis avoided accidental double work, the pinned upstream corpus was added, BOM/lone-CR documents entered the frontmatter path, invalid YAML gained a stable JSON envelope, diagnostic spans were projected back to authored-document coordinates after multiple edits, and raw delimiter bytes were preserved. Review 4 found no remaining functional defect but still called the feature not production-ready because the admissible no-frontmatter/already-clean timing bracket and native Linux/Windows runtime evidence were absent; its high evidence finding was explicitly labeled non-blocking for the moment.

Packages and modules changed:

- `biscuit-file`: shared `SourceSpan`; `yaml::analyze` analysis, diagnostic, edit-set, engine, locate, recover, report, and scanner modules; YAML location/source-retention/types; public re-exports; unit, corpus, mutation, safety, parse-count, and compatibility tests; public README/API documentation.
- `darkmatter` library: shared span compatibility, frontmatter extraction, `markdown::schemas::clean` and validation integration, schema quoting/suggestion tests, safety properties/counters, and `clean_hot_paths` benchmarks.
- `darkmatter-cli`: clean arguments and dispatch, `commands::clean::frontmatter_repair`, schema/trigger resolution, raw-preserving assembly, version-1 JSON and human diagnostic presentation, plus clean/frontmatter/schema/JSON tests and public clean documentation.
- `dmls` is a downstream consumer in the affected scope but received no invalid-frontmatter production change in this period.
- The feature's specification, decisions, acceptance matrix, baselines, impact report, deferred-performance record, reviews, and log.

## Timeline

The timeline includes every commit from the fork point `d672388dd0fed4196295e7f21514cac6fa59f0ae` through `14dd391f45206d58383ba9d84adbf53c65520534`, in chronological order. The fork-point commit itself is included as requested.

- `d672388dd` (2026-07-17) — docs(darkmatter): record performance-followup review 8 implementation cycle
  - This is the stated fork point. It records that no new implementation finding was fixed and that the integrated compose-regression measurement remained deferred because host load violated the benchmark contract.
- `a1d8ff844` (2026-07-18) — feat(dmls): add fixed-width list reflow parity test
- `ada1e6e98` (2026-07-18) — docs(darkmatter): clarify cleanup scope and fixed-width reflow in inline-post
- `05df99be7` (2026-07-18) — docs(darkmatter): forward-reference fixed-width-lists fix from cleanup spec
- `dc6a68974` (2026-07-18) — feat(darkmatter): re-export biscuit_file::SourceSpan and pin compat
  - Established one shared byte-offset span vocabulary across `biscuit-file` and Darkmatter, a prerequisite for uniform YAML/schema diagnostics.
- `96c6616e9` (2026-07-18) — docs(darkmatter): ratify invalid-frontmatter phase 1 deliverables
- `4d0dd908e` (2026-07-18) — refactor(darkmatter): split reflow parser model and add validate fresh seam
  - The first major post-fork implementation commit: it introduced parser-derived list prose modeling and the internal fresh-reference-graph validation route while preserving pass order and the checked public path.
- `4c2573060` (2026-07-18) — feat(darkmatter-cli): support list-item paragraphs in clean
- `cffa3a142` (2026-07-18) — docs(darkmatter): record fixed-width list review cycle
- `c6a13b778` (2026-07-18) — docs(darkmatter): record performance-followup review 9 assessment
- `82f015cc5` (2026-07-18) — docs(darkmatter): close redundant-walk planning cycle
- `afe035914` (2026-07-18) — feat(darkmatter): add schema-aware clean analysis module and tests
  - Added Darkmatter's schema-aware half of invalid-frontmatter analysis, including quoting/suggestion logic and focused tests, while keeping the schema-agnostic engine in `biscuit-file`.
- `d383839d7` (2026-07-18) — docs(darkmatter): complete invalid-frontmatter phases 3 and 4
- `5a59fb723` (2026-07-18) — docs(darkmatter): describe list-item paragraph cleanup support
- `8a0ccd900` (2026-07-18) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `aab72bfb9` (2026-07-18) — test(biscuit-file): add integration tests for yaml analyzer
- `7aaa9dccc` (2026-07-18) — feat(biscuit-file): add shared SourceSpan byte-offset vocabulary
- `95bf05b99` (2026-07-18) — test(biscuit-file): add unit tests for yaml analyze module
- `2ad880c2d` (2026-07-18) — feat(biscuit-file): add source-first YAML analyze module
  - Landed the reusable schema-agnostic analyzer, repair engine, raw-source retention, structured locations, and diagnostics API that made repair of unparseable frontmatter possible.
- `f82221e0c` (2026-07-19) — feat(darkmatter): add PreparedHeadingSnapshot to ReferenceGraph
  - Closed the fresh-graph coherence gap for fragment validation by retaining build-time heading slugs privately, without changing `Debug`, JSON, errors, or public graph construction.
- `b9e3d3d04` (2026-07-19) — test(darkmatter): add invalid-frontmatter library safety + counter suites
- `577a869d1` (2026-07-19) — test(darkmatter-cli): add L1 CLI suites for invalid-frontmatter pipeline
- `7d6556f61` (2026-07-19) — feat(darkmatter-cli): wire md clean into invalid-frontmatter repair
  - Connected file/stdin clean flows to frontmatter analysis, schema resolution, repair, diagnostics, and raw-preserving output; this turned the prior library work into user-visible behavior.
- `690b2ecc3` (2026-07-19) — planning(darkmatter): close invalid-frontmatter review-1 cycle
- `152ea6b84` (2026-07-19) — planning(darkmatter): complete redundant-walk review cycle 3
- `a7bee0cf3` (2026-07-19) — fix(darkmatter): detect type changes in schema clean safety gate
  - Corrected the safety model so schema-proven quoting may intentionally change a scalar's parsed type while still requiring the full candidate schema result to improve safely.
- `4c903c586` (2026-07-19) — planning(darkmatter): close redundant-walk cycle 3, open cycle 4
- `3355343e7` (2026-07-19) — planning(darkmatter): close redundant-walk cycle 4, open cycle 5
- `243ea24f8` (2026-07-19) — planning(darkmatter): close redundant-walk cycle 5, open cycle 6
- `bbc8ccd77` (2026-07-20) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `6abe0827f` (2026-07-20) — planning(darkmatter): close redundant-walk cycle 6, open cycle 7
- `16461dc79` (2026-07-20) — docs: document fix-review cycle commit convention
- `24949be46` (2026-07-20) — feat(biscuit-file): share YAML analysis across diagnostics and repairs
  - Removed duplicate analysis paths and added corpus/mutation infrastructure, supporting the parse-once acceptance contract.
- `1438dbaf0` (2026-07-20) — test(dmls): add fixed-width reference-definition integrity suite
- `a521054ef` (2026-07-20) — planning(darkmatter): initialize 2026-07-20-dm-mega-merge fix planning
- `0993c32a3` (2026-07-20) — planning(darkmatter): close performance-followup cycle 9, open cycle 10
- `9ddff77e7` (2026-07-20) — test(darkmatter-cli): cover fixed-width list review findings
- `1098913fe` (2026-07-20) — fix(darkmatter): implement review-1 findings for fixed-width-lists fix
  - Strengthened parse-count, cross-surface parity, idempotency, and structural regression coverage around the initial list-aware reflow implementation.
- `386165c14` (2026-07-20) — planning(darkmatter): close fixed-width-lists review-1 cycle
- `6a8aac192` (2026-07-20) — fix(darkmatter): stack-based fix_list_indentation from review-2
  - Replaced width assumptions with stack-based container tracking for changing ordered markers and nested list indentation.
- `fed4d1e95` (2026-07-20) — fix(darkmatter-cli): reject --indent 8; library still accepts any usize
- `c91b73806` (2026-07-20) — test(darkmatter-cli): cover indent rejection and preserve idempotence
- `87580e164` (2026-07-20) — test(darkmatter): cover review-2 wide-marker list regressions
- `f1ff23c92` (2026-07-20) — planning(darkmatter): close fixed-width-lists review-2 cycle
- `a6f035354` (2026-07-20) — test(darkmatter): cover review-3 fixed-width list regressions
- `2e4ef475a` (2026-07-20) — test(dmls): cover nested lists inside blockquotes for format_text
- `2e0e02553` (2026-07-20) — fix(darkmatter): parser-derived cleanup context from review-3
  - Moved list indentation normalization to parser-derived block context so marker-looking code and container ownership were no longer inferred from raw leading spaces.
- `10a363bc2` (2026-07-20) — fix(darkmatter-cli): restore --indent 8 after structural cleanup fix
- `9b55ea39b` (2026-07-20) — planning(darkmatter): close fixed-width-lists review-3 cycle
- `c786a6828` (2026-07-20) — test(darkmatter-cli): route L2 tests via tmux harness; add indent-8
- `3c415182e` (2026-07-20) — fix(darkmatter-cli): align --indent docs with structural cleanup from review-4
- `674aa6be4` (2026-07-20) — test(dmls): cover review-4 fixed-width list cleanup regressions
- `42b7667bd` (2026-07-20) — test(darkmatter-cli): cover review-4 fixed-width list cleanup regressions
- `0c74e2fec` (2026-07-20) — planning(darkmatter): close fixed-width-lists review-3 cycle, open cycle 4
- `c5eee4d1c` (2026-07-20) — fix(darkmatter): parser-derived cleanup context from review-4
  - Extended structural context across blockquotes, task-list prefixes, and cleanup orchestration, replacing remaining raw-prefix heuristics.
- `895c22506` (2026-07-20) — test(darkmatter): cover review-4 fixed-width list cleanup regressions
- `3267e6358` (2026-07-20) — test(darkmatter): cover review-5 shell-block payload preservation
- `0a30ad463` (2026-07-20) — planning(darkmatter): close fixed-width-lists review-4 cycle, open cycle 5
- `8fb45cf5a` (2026-07-20) — test(darkmatter-cli): cover review-5 cleanup and schema gates
- `358171689` (2026-07-20) — test(dmls): cover review-5 shell-block payload preservation in format_text
- `0deff3584` (2026-07-20) — fix(darkmatter): mask opaque directive bodies across cleanup
  - Prevented list and reflow passes from rewriting literal shell-block payloads; later review found the local recognizer needed to be unified with the shared scanner.
- `8fe7e894e` (2026-07-21) — test(dmls): cover review-6 mixed-stack shell-payload preservation in format_text
- `aa249a405` (2026-07-21) — docs(commits): note gpg-agent pinentry risk and quoted-commit bodies
- `c7a825fb4` (2026-07-21) — test(darkmatter-cli): cover invalid-frontmatter stdin trigger isolation
- `5462146f6` (2026-07-21) — planning(darkmatter): implement invalid-frontmatter review-1 findings; defer Phase 7 timing
- `31a2235f2` (2026-07-21) — test(darkmatter): cover review-6 mixed-stack shell-payload preservation
- `da81b7e19` (2026-07-21) — fix(darkmatter): derive opaque body ownership from shared block scanner
  - Unified cleanup protection with Darkmatter's authoritative block-pair scanner and defined source-preserving fallback for malformed or unterminated structures, resolving the final blocking fixed-width-list review finding.
- `4deff9973` (2026-07-21) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `9b590ba29` (2026-07-21) — planning(darkmatter): close fixed-width-lists review-5 cycle, open cycle 6
- `fda518ba4` (2026-07-21) — feat(biscuit-file): vendor pinned YAML Test Suite subset for corpus coverage
  - Replaced locally analogous fixtures with exact, licensed, SHA-pinned upstream YAML Test Suite cases and stable upstream IDs.
- `2b81b4082` (2026-07-21) — test(darkmatter-cli): cover review-6 cleanup mixed-stack shell-payload preservation
- `8724e3eb2` (2026-07-21) — feat(darkmatter): correct clean_hot_paths benchmark vehicle
  - Repaired the benchmark harness needed for the invalid-frontmatter no-regression evidence, but did not itself supply an admissible quiet-host comparison.
- `7278b1ccb` (2026-07-21) — docs(commits): add git verify-commit signature verification guidance
- `9509570eb` (2026-07-21) — planning(darkmatter): close invalid-frontmatter review cycle 2
- `513bdae24` (2026-07-21) — docs(darkmatter): document md clean frontmatter repair
- `43fc04704` (2026-07-21) — planning(darkmatter): close fixed-width-lists review-6 cycle, ready
- `cb95e4e50` (2026-07-21) — fix(darkmatter): BOM/lone-CR frontmatter + v1 clean JSON envelope
  - Ensured stream-start BOM and lone-CR documents enter frontmatter analysis and made the stable version-1 JSON envelope available even for unrepaired invalid YAML.
- `7b43851c3` (2026-07-21) — planning(darkmatter): close invalid-frontmatter review-2, open cycle 3
- `422726c02` (2026-07-21) — fix(darkmatter): JSON spans index authored YAML; clean keeps delimiters
  - Corrected later-pass spans after expanding/shrinking edits and reconstructed output from authored delimiter slices, closing Review 3's remaining functional defects.
- `b49549734` (2026-07-21) — chore: refresh GitNexus symbol/relationship counts in CLAUDE.md
- `c94f156f7` (2026-07-21) — docs(biscuit-file): document YAML source analysis APIs
- `69055a56b` (2026-07-21) — planning(darkmatter): close invalid-frontmatter cycle 3, open cycle 4
- `14dd391f4` (2026-07-21) — docs(commits): record shared staged worktree risk

## File Blast Radius

This is the union of every path mutated by the 83 commits in `d672388dd0fed4196295e7f21514cac6fa59f0ae^..14dd391f45206d58383ba9d84adbf53c65520534`, collected with rename detection disabled so historical mutations are not hidden by the final net diff. It contains 154 paths.

### Repository metadata and agent guidance

- `.claude/skills/darkmatter/SKILL.md`
- `.claude/skills/darkmatter/compose.md`
- `.claudine/memory/commits.md`
- `CLAUDE.md`

### `biscuit-file`

- `biscuit-file/README.md`
- `biscuit-file/lib/Cargo.toml`
- `biscuit-file/lib/README.md`
- `biscuit-file/lib/src/lib.rs`
- `biscuit-file/lib/src/span.rs`
- `biscuit-file/lib/src/yaml/analyze/analysis.rs`
- `biscuit-file/lib/src/yaml/analyze/diagnostic.rs`
- `biscuit-file/lib/src/yaml/analyze/edit_set.rs`
- `biscuit-file/lib/src/yaml/analyze/engine.rs`
- `biscuit-file/lib/src/yaml/analyze/locate.rs`
- `biscuit-file/lib/src/yaml/analyze/mod.rs`
- `biscuit-file/lib/src/yaml/analyze/recover.rs`
- `biscuit-file/lib/src/yaml/analyze/report.rs`
- `biscuit-file/lib/src/yaml/analyze/scan.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/analysis.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/anchors.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/classification_gate.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/duplicate_keys.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/edit_set.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/line_endings.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/lints.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/locate.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/mod.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/multi_document.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/normalization.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/reserved_indicator.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/scan.rs`
- `biscuit-file/lib/src/yaml/analyze/tests/whitespace.rs`
- `biscuit-file/lib/src/yaml/location.rs`
- `biscuit-file/lib/src/yaml/mod.rs`
- `biscuit-file/lib/src/yaml/tests/diagnose.rs`
- `biscuit-file/lib/src/yaml/tests/location.rs`
- `biscuit-file/lib/src/yaml/tests/mod.rs`
- `biscuit-file/lib/src/yaml/tests/retained_source.rs`
- `biscuit-file/lib/src/yaml/types.rs`
- `biscuit-file/lib/tests/corpus/YAML-TEST-SUITE-NOTICE.md`
- `biscuit-file/lib/tests/corpus/yaml_corpus.json`
- `biscuit-file/lib/tests/parse_count.rs`
- `biscuit-file/lib/tests/span_compat.rs`
- `biscuit-file/lib/tests/yaml_corpus.rs`
- `biscuit-file/lib/tests/yaml_mutation.proptest-regressions`
- `biscuit-file/lib/tests/yaml_mutation.rs`
- `biscuit-file/lib/tests/yaml_safety.rs`

### `darkmatter-cli`

- `darkmatter/cli/Cargo.toml`
- `darkmatter/cli/README.md`
- `darkmatter/cli/src/args/command.rs`
- `darkmatter/cli/src/args/completion.rs`
- `darkmatter/cli/src/args/parsers.rs`
- `darkmatter/cli/src/commands/clean.rs`
- `darkmatter/cli/src/commands/clean/frontmatter_repair.rs`
- `darkmatter/cli/src/commands/compose.rs`
- `darkmatter/cli/src/commands/mod.rs`
- `darkmatter/cli/src/main.rs`
- `darkmatter/cli/tests/clean.rs`
- `darkmatter/cli/tests/clean_frontmatter.rs`
- `darkmatter/cli/tests/clean_json.rs`
- `darkmatter/cli/tests/clean_schema.rs`
- `darkmatter/cli/tests/common/level2.rs`
- `darkmatter/cli/tests/level2_code_block_styling.rs`
- `darkmatter/cli/tests/level2_errors.rs`
- `darkmatter/cli/tests/level2_schema_about.rs`

### `dmls`

- `darkmatter/dmls/src/providers/formatting.rs`

### Darkmatter public documentation

- `darkmatter/docs/cli/clean.md`
- `darkmatter/docs/cli/render.md`
- `darkmatter/docs/darkmatter-compose-pipeline.md`
- `darkmatter/docs/dependencies.md`

### Feature and fix records

- `darkmatter/features/2026-07-14-invalid-frontmatter/acceptance-matrix.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/baselines.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/baselines/clean-fm.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/baselines/coercible.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/baselines/invalid-reserved.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/baselines/no-fm.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/decisions.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/deferred-performance.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/impact-report.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/log.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/plan.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/review-1.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/review-2.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/review-3.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/review-4.md`
- `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`
- `darkmatter/features/2026-07-15-performance-followup/log.md`
- `darkmatter/features/2026-07-15-performance-followup/performance-compliance.md`
- `darkmatter/features/2026-07-15-performance-followup/review-10.md`
- `darkmatter/features/2026-07-15-performance-followup/review-8.md`
- `darkmatter/features/2026-07-15-performance-followup/review-9.md`
- `darkmatter/features/2026-07-15-performance-followup/spec.md`
- `darkmatter/features/_completed/2026-06-19-cleanup-fixed-line-length/spec.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/deferred-performance-tests.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/log.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-1.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-2.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-3.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-4.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-5.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/review-6.md`
- `darkmatter/fixes/2026-07-13-fixed-width-lists/spec.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/log.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/phase-1-evidence.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/plan.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/results.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-1.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-2.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-3.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-4.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-5.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-6.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/review-7.md`
- `darkmatter/fixes/2026-07-16-redundant-walk/spec.md`
- `darkmatter/fixes/2026-07-20-dm-mega-merge/_research.md`
- `darkmatter/fixes/2026-07-20-dm-mega-merge/spec.md`

### `darkmatter` library

- `darkmatter/lib/Cargo.toml`
- `darkmatter/lib/benches/clean_hot_paths.rs`
- `darkmatter/lib/benches/reference_graph.rs`
- `darkmatter/lib/src/markdown/cleanup/blockquote.rs`
- `darkmatter/lib/src/markdown/cleanup/lists.rs`
- `darkmatter/lib/src/markdown/cleanup/mod.rs`
- `darkmatter/lib/src/markdown/cleanup/opaque.rs`
- `darkmatter/lib/src/markdown/cleanup/perf_profile.rs`
- `darkmatter/lib/src/markdown/cleanup/reflow.rs`
- `darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs`
- `darkmatter/lib/src/markdown/cleanup/tests/lists.rs`
- `darkmatter/lib/src/markdown/cleanup/tests/mod.rs`
- `darkmatter/lib/src/markdown/cleanup/tests/parse_count.rs`
- `darkmatter/lib/src/markdown/cleanup/tests/reflow.rs`
- `darkmatter/lib/src/markdown/compose/pipeline/phases.rs`
- `darkmatter/lib/src/markdown/compose/tests/rendering.rs`
- `darkmatter/lib/src/markdown/frontmatter.rs`
- `darkmatter/lib/src/markdown/mod.rs`
- `darkmatter/lib/src/markdown/reference/file_tree/mod.rs`
- `darkmatter/lib/src/markdown/reference/file_tree/model.rs`
- `darkmatter/lib/src/markdown/reference/graph.rs`
- `darkmatter/lib/src/markdown/reference/mod.rs`
- `darkmatter/lib/src/markdown/reference/snapshot.rs`
- `darkmatter/lib/src/markdown/reference/types.rs`
- `darkmatter/lib/src/markdown/reference/validate.rs`
- `darkmatter/lib/src/markdown/schemas/clean.rs`
- `darkmatter/lib/src/markdown/schemas/mod.rs`
- `darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs`
- `darkmatter/lib/src/markdown/schemas/tests/clean_suggestions.rs`
- `darkmatter/lib/src/markdown/schemas/tests/mod.rs`
- `darkmatter/lib/src/markdown/schemas/validate.rs`
- `darkmatter/lib/src/markdown/span.rs`
- `darkmatter/lib/tests/clean_counters.rs`
- `darkmatter/lib/tests/schema_quoting_safety.proptest-regressions`
- `darkmatter/lib/tests/schema_quoting_safety.rs`
- `darkmatter/lib/tests/span_compat.rs`
