---
agent: claude/
total_phases: 7
created: 2026-07-14
phase: 7
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/cleanup/reflow.rs
  - darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/cleanup/reflow.rs
  - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/cleanup/reflow.rs
  - darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs
  - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
docs_updated_during_phase_4:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/cleanup/mod.rs
  - darkmatter/lib/src/markdown/cleanup/reflow.rs
  - darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs
  - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/tests/clean.rs
  - darkmatter/dmls/src/providers/formatting.rs
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/docs/cli/clean.md
  - darkmatter/cli/README.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/features/_completed/2026-06-19-cleanup-fixed-line-length/spec.md
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
  - darkmatter/lib/src/markdown/cleanup/mod.rs
  - darkmatter/lib/src/markdown/cleanup/reflow.rs
  - darkmatter/lib/src/markdown/cleanup/reflow/semantic.rs
  - darkmatter/lib/src/markdown/cleanup/tests/reflow.rs
  - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/cli/src/commands/clean.rs
  - darkmatter/cli/tests/clean.rs
  - darkmatter/dmls/src/providers/formatting.rs
documentation:
  - darkmatter/fixes/2026-07-13-fixed-width-lists/plan.md
  - darkmatter/docs/cli/clean.md
  - darkmatter/cli/README.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/features/_completed/2026-06-19-cleanup-fixed-line-length/spec.md
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
---

# Execution Plan — List-Aware Incidental-Newline Cleanup and Fixed-Width Reflow

Derived from [`spec.md`](./spec.md). This fix makes Darkmatter's cleanup contract apply to
prose inside ordered, unordered, and task-list items at every nesting depth — in default strip,
`--fixed-width N`, and `--ignore-incidental-newlines` modes — across the library, compose, CLI,
and DMLS surfaces, without changing any public API, CLI flag, or platform behavior.

## Guiding Constraints (apply to every phase)

- **Fail-first.** Every behavioral change lands behind a test that fails against current code
  first, then passes. At least one strip test asserts an exact string with a single ASCII join
  space; every non-overflow fixed-width fixture asserts the display width of every emitted line.
- **Parser structure is authoritative.** List-continuation vs indented-code ambiguity is resolved
  from the pulldown-cmark offset event stream (Decision 1), never from a `starts_with("    ")`
  threshold.
- **One shared soft-break decision model** feeds both `strip_incidental_newlines` and full cleanup
  (Decision 2). Do not add an unconditional second Markdown parse to the default cleanup path
  without benchmark evidence and explicit review.
- **Preserve pass order and public surface** (Decision 4). No public function, method, enum, CLI
  arg, or DMLS setting is renamed or removed. New types are private under
  `markdown/cleanup/reflow.rs` or a focused child module. No pass trait, no public list-layout
  abstraction.
- **Change surface stays narrow.** Expected files only (see spec "Change Surface"). Touching
  `lists.rs` requires written justification and focused non-regression tests.
- **Never run `cargo fmt`.** Use `just test` / `just test-l2` / `just lint` from the `darkmatter/`
  package area — not `cargo test`. Match surrounding style by hand.

---

## Phase 1 — Baseline, Blast-Radius, and Fail-First Reproduction

Goal: lock the current green baseline, quantify the risk, and codify the two core defects as
failing tests before touching any production code.

- [x] Run `just test` and `just lint` from `darkmatter/` and record a clean baseline; note any
  pre-existing failures so they are not attributed to this fix.
- [x] Run GitNexus `impact({target: "strip_incidental_newlines", direction: "upstream"})` and
  `impact({target: "reflow_to_width", direction: "upstream"})`; capture the direct/transitive
  dependent counts and confirm they match the spec's CRITICAL/HIGH classification. Report the blast
  radius before editing. *(parallelizable with the baseline run)*
- [x] Add a fail-first L1 test in `darkmatter/lib/src/markdown/cleanup/tests/reflow.rs` proving the
  **default-strip retained-indentation defect**: a `-` item whose continuation line is indented four
  spaces currently keeps the physical break / literal indentation; assert the exact single-logical-line
  output with one ASCII join space. Confirm it fails.
- [x] Add a fail-first L1 test proving the **fixed-width physical-line defect**: a pre-wrapped `-`
  item at width 24 currently wraps only the first physical line; assert the full unwrap-then-rewrap
  output from the spec ("Alpha beta gamma delta / epsilon zeta eta / theta.") with per-line width
  assertions. Confirm it fails.
- [x] Add a fail-first L1 test proving the **blockquoted-list continuation-prefix defect**
  (`> - …` continuation must stay `>   …`, not `> …`). Confirm it fails.

**Checkpoint:** three (or more) new tests fail for the documented reasons; baseline recipes are
otherwise green; impact numbers reported.

### Phase 1 Evidence

- Baseline: `just test` passed for `darkmatter`, `darkmatter-cli`, and `dmls`; `just lint` passed
  for the same package-area scope. No pre-existing failures were observed.
- Current GitNexus index: `strip_incidental_newlines` has 34 direct / 209 total upstream
  dependents at CRITICAL risk; `reflow_to_width` has 5 direct / 26 total at CRITICAL risk. This is
  slightly broader than the spec's July 13 snapshot (33 / 176 CRITICAL and 5 / 25 HIGH).
- Fail-first run: all three targeted tests failed. The default-strip case retained the physical
  newline and four spaces; the pre-wrapped width-24 case remained physically wrapped as authored;
  and the blockquoted-list continuation was emitted as `> delta...` instead of `>   delta...`.
- The three fail-first tests are ignored with phase-specific reasons until their Phase 3/4
  implementation lands, keeping the ordinary Level-1 suite green while preserving explicit
  runnable regression contracts.

---

## Phase 2 — Shared Semantic Soft-Break / List-Prose Model (Decisions 1 & 2)

Goal: build the parser-driven classifier that both strip and reflow consume. No public behavior
change yet — this phase is additive private infrastructure with its own unit coverage.

- [x] In `reflow.rs` (or a focused child module under `markdown/cleanup/`), add a private model
  built from `Parser::new_ext(content, cleanup_parser_options()).into_offset_iter()` — the **same**
  parser options as cleanup — that records, per candidate soft-break boundary:
  - the source span of the soft line ending (`Event::SoftBreak` offsets);
  - the source span of the next line's syntactic container/continuation prefix;
  - whether Darkmatter structural rules protect the boundary; and
  - the zero-or-one-character join-separator decision (reuse existing `join_separator`).
- [x] Track active `List`, `Item`, `BlockQuote`, `Paragraph`, `CodeBlock`, `Table`, and HTML
  contexts from the event stream so a four-space continuation line inside a list item is classified
  as list prose, while genuine indented code / nested lists / second paragraphs are distinguished
  from parsed structure — not from leading-space counting.
- [x] Keep the existing Darkmatter directive/fence/HTML/shell-block protection as an overlay
  (pulldown-cmark does not know `::` semantics), but ensure indentation heuristics never override
  parsed evidence that a line is list prose.
- [x] Add private unit tests for the model in isolation (boundary eligibility, protection, prefix
  spans) covering: lazy vs explicit continuation, nested parent/child items, blockquoted list items,
  second-paragraph boundaries, and protected child blocks. These pin the classifier independent of
  the strip/reflow consumers.

**Checkpoint:** model unit tests pass; `strip_incidental_newlines` and `reflow_to_width` public
behavior is still unchanged (Phase 1 defect tests still fail — intended).

### Phase 2 Evidence

- Six fail-first L1 model tests cover lazy/explicit continuation eligibility and exact spans,
  nested parent/child isolation, composite blockquote prefixes, second-paragraph containment,
  parser-structured and Darkmatter-protected children, and shared Unicode separator decisions.
- The targeted model suite passes. The three ignored Phase 1 regressions were run explicitly and
  still fail with their original output differences, confirming no public strip/reflow behavior
  changed in this additive phase.
- Scoped package-area gates pass: `just build`, `just test`, and `just lint` for `darkmatter`,
  `darkmatter-cli`, and `dmls`. No pre-existing gate failures were observed.

---

## Phase 3 — Default Stripping via the Shared Model (Decision 2, Mode row "Default")

Goal: rewrite `strip_incidental_newlines` to remove list soft breaks and their continuation-layout
prefix, inserting only the existing zero-or-one-character join separator, using non-overlapping
edits.

- [x] Rewrite `strip_incidental_newlines` to consume the Phase-2 model: for each eligible boundary,
  remove the line ending, remove the next line's container/continuation-layout prefix, join via
  `join_separator`, and emit no other whitespace. Apply non-overlapping edits from the end of the
  source into a pre-sized buffer (Decision 2) — do not repeatedly mutate the middle of a `String`.
- [x] Guarantee default strip synthesizes **no** hanging indentation and drops authored
  continuation indentation entirely (Goal 2, AC 3). A collapsed boundary yields either no character
  or exactly one `U+0020`.
- [x] Preserve all non-list strip behavior byte-for-byte (paragraph boundaries, hard breaks,
  protected blocks, blockquote-only prose, non-list Unicode join policy). CRLF / lone-CR input
  normalizes identically.
- [x] Make the Phase-1 strip fail-first test pass, then complete the **L1 stripping matrix**
  (spec "L1 library tests — stripping", items 1–12): 2/4/8-space unordered; lazy-3 / explicit-4
  ordered; `10.` / `10)`; checked+unchecked tasks; unindented lazy continuation; nested parent/child
  independent collapse; blockquoted + nested-blockquoted prefix removal; second-paragraph blank +
  indent preserved; hard breaks survive; protected fenced/indented/table/HTML/directive children;
  CRLF == LF canonical output; full Unicode separator parity (Han, Thai, Hangul, emoji, punctuation,
  ZWSP). At least one exact-string assertion contains a single ASCII join space.

**Checkpoint:** entire L1 stripping matrix green; Phase-1 strip and blockquote defect tests now
pass; `just test` from `darkmatter/` green for the library crate.

### Phase 3 Evidence

- Requirement-to-test mapping: the exact reported four-space input and repeat-strip fixed point are
  pinned by `strip_incidental_newlines_collapses_explicit_list_continuation_without_layout_spaces`;
  unordered indentation, ordered marker, and task-box representations have dedicated matrix tests;
  nested items, composite blockquotes, second paragraphs, and hard breaks each have exact-output
  tests; protected fenced/indented/table/HTML/directive children include closed and malformed
  unclosed forms; line-ending parity covers LF, CRLF, and lone CR; and Unicode parity covers Han,
  Thai, Hangul, emoji, punctuation, and zero-width spaces inside and outside lists.
- Fail-first evidence: the original input and each changed matrix group failed before the
  implementation because the newline or authored layout indentation survived. Hard-break and
  protected-child controls passed before the implementation, showing the failures were scoped to
  list-prose collapse.
- Targeted regression gate: `cargo nextest run --color=never -p darkmatter -E
  'test(/strip_incidental_newlines/)' --no-fail-fast` passed all 37 stripping tests after the final
  test-design audit.
- Broader affected-area gates passed: `just build`; `just test` (5,780 `darkmatter`, 559
  `darkmatter-cli`, and 566 `dmls` Level-1 tests); and `just lint`, all from `darkmatter/`.
  The area test recipe intentionally skipped 142 Darkmatter, 71 CLI, and 3 DMLS higher-tier tests.
  No pre-existing or new failures were observed.

---

## Phase 4 — Fixed-Width Logical-Block Reflow + Composite Prefixes (Decisions 3 & 5)

Goal: reflow complete logical prose blocks (not physical lines) to the requested display width,
emitting the full composite hanging container prefix on every created continuation line.

- [x] Give `reflow_to_width` a semantic map of reflowable prose blocks derived from parsed
  structure (Decision 3), replacing the current per-line `is_indented_code_line` protection so only
  actual parsed code/HTML/table blocks are protected — not every four-space line.
- [x] For each list prose block derive `first_prefix` and `continuation_prefix`: the first
  paragraph's `first_prefix` carries the item marker/task box/separator; `continuation_prefix`
  replaces that marker region with **equal display-width ASCII spaces** (never tabs). A subsequent
  paragraph carries its required container indentation in both prefixes and no item marker.
- [x] Make prefix parsing **compose** blockquote and list containers instead of returning after the
  first recognized family (replaces the current early-return in `line_reflow_prefix` /
  `blockquote_prefix_len` handling). `>   10. [ ] Body` yields the full byte prefix; continuation
  lines keep the quote marker and hang under the list body.
- [x] Compute each item's continuation width from that item's actual post-cleanup serialized marker
  (handle `9.`→`10.` digit growth per item; do not reuse the first item's width). Consume the
  actual post-cleanup nesting indentation from the pipeline / `--indent` policy — do not hard-code
  2 or 4 spaces.
- [x] Preserve the Decision-5 width/overflow contract: target is Unicode display columns including
  every prefix column; atomic tokens wider than the available body width (and bodies after a
  prefix that already meets/exceeds the width) are emitted intact. Word tokenization, inline-code
  protection, and spaceless-script joining are unchanged.
- [x] Make the Phase-1 fixed-width fail-first test pass, then complete the **L1 fixed-width matrix**
  (spec "L1 library tests — fixed width", items 1–11): pre-wrapped unwrap+rewrap; differing ordered
  digit widths → different hanging prefixes; task alignment after checkbox; nested 2/4/8-space
  widths; blockquoted ordered/unordered/task composite prefixes; first vs subsequent paragraph
  independence; hard breaks + valid prefixes on both sides; long token not split under tight prefix;
  wide-Unicode measured with `UnicodeWidthStr` incl. prefix; protected children byte-equivalent;
  fixed-width idempotence. **Every** non-overflow fixture asserts the display width of every emitted
  physical line including its prefix.

**Checkpoint:** full L1 fixed-width matrix green; `cleanup_to_fixed_width` and `reflow_to_width`
satisfy the width contract; running fixed-width cleanup twice is a fixed point.

### Phase 4 Evidence

- Requirement-to-test mapping:
  - logical list-paragraph unwrapping and wrapping uses the specification's original input in
    `reflow_to_width_unwraps_complete_list_paragraph_before_wrapping`;
  - per-item ordered-marker growth and task-box alignment are covered by
    `reflow_to_width_derives_ordered_prefix_width_per_item` and
    `reflow_to_width_aligns_checked_and_unchecked_task_items`;
  - actual 2/4/8-space post-cleanup nesting and composite blockquote/list prefixes are covered by
    `reflow_to_width_uses_configured_nested_list_indentation` and
    `reflow_to_width_composes_blockquote_list_prefix_families`;
  - independent subsequent paragraphs and both Markdown hard-break forms are covered by
    `reflow_to_width_keeps_first_and_subsequent_item_paragraphs_independent` and
    `reflow_to_width_preserves_list_hard_breaks_and_container_prefixes`;
  - overflow and Unicode display-column boundaries are covered by
    `reflow_to_width_keeps_long_token_intact_after_tight_prefix` and
    `reflow_to_width_measures_wide_unicode_with_list_prefix`;
  - fenced/indented code, tables, HTML, and shell blocks are covered byte-for-byte by
    `reflow_to_width_preserves_list_child_blocks_byte_for_byte`;
  - dependent pipeline state and repeat-application stability are covered by
    `cleanup_to_fixed_width_is_idempotent_for_nested_composite_lists`.
  Every non-overflow fixture checks total physical-line display width including its prefix.
- Fail-first gate: `cargo nextest run --color=never -p darkmatter -E
  'test(/reflow_to_width_|cleanup_to_fixed_width_is_idempotent/)' --run-ignored ignored-only
  --no-fail-fast` initially ran 11 tests: 8 failed for the reported list-block/prefix behaviors and
  3 control cases passed. The same command passed all 11 after implementation.
- Targeted regression gate: `cargo nextest run --color=never -p darkmatter -E
  'test(/reflow_to_width|cleanup_to_fixed_width|strip_incidental_newlines/)' --no-fail-fast`
  passed all 63 tests. The strip tests ensure the shared semantic model did not regress Phase 3.
- Broader affected-area gates passed from `darkmatter/`: `just build`, `just test`, and `just lint`
  for `darkmatter`, `darkmatter-cli`, and `dmls`. The test recipe ran 5,792 Darkmatter tests and 566
  DMLS tests; its configured higher-level exclusions remained skipped. No pre-existing or new
  failures were observed. Phase 4 changes no shipped schema, template, prompt, or configuration
  artifact, so the passive-corpus and real-artifact requirements do not apply in this phase.

---

## Phase 5 — Orchestration & Cross-Surface Parity (Decision 4)

Goal: wire the shared model into full cleanup preserving pass order, and prove byte-equivalence at
every public surface. The four parity test groups are independent and parallelizable once
orchestration lands.

- [x] In `cleanup/mod.rs`, keep the externally visible order (soft-break collapse → existing
  cleanup/list normalization → fixed-width reflow). Preferred implementation (Decision 2): lower
  eligible `Event::SoftBreak` to the chosen text separator before `pulldown-cmark-to-cmark`
  serialization, reusing the existing cleanup parse rather than adding a second full parse to the
  default path. If a different approach is chosen, demonstrate byte-equivalent output and justify.
- [x] Add the **semantic structure regression helper**: parse source and output with the cleanup
  parser options and compare a structural fingerprint (list/item count+order, ordered vs unordered,
  nesting depth, paragraph boundaries, blockquote boundaries, code/table/HTML child boundaries)
  after ignoring soft-break events and offsets. Apply it to the nested-list, second-paragraph,
  blockquoted-list, and protected-code fixtures (fingerprint supplements, never replaces, exact
  string assertions).
- [x] **Compose parity** (`compose/tests/rendering.rs`): prove all three modes through
  `ComposeOptions` — default `Strip` collapses, `Preserve` retains, `with_fixed_width(N)` forces
  collapse before reflow even when `Preserve` is also selected — and that output matches the direct
  library cleanup sequence. *(parallelizable)*
- [x] **CLI parity** (`cli/tests/clean.rs`): stdin-driven `md clean -` collapses a four-space-wrapped
  item; `md clean --fixed-width N -` reflows the whole item and satisfies total-line width;
  `md clean --ignore-incidental-newlines -` retains canonical list soft breaks; `--save` writes
  corrected wrapping and reports the delta. Existing arg-conflict / width-range tests unchanged.
  *(parallelizable)*
- [x] **DMLS parity** (`dmls/src/providers/formatting.rs`, tests only): extend formatting parity with
  a pre-wrapped list fixture; `fixed_width = N` output is byte-identical to the
  `Markdown::cleanup` + `reflow_to_width` sequence used by `md clean`. *(parallelizable)*
- [x] Confirm `--ignore-incidental-newlines` performs no list soft-break collapse and no fixed-width
  synthesis (AC 4), and that default/compact/loose list-spacing modes retain existing behavior
  (AC 11).

**Checkpoint:** library, compose, CLI stdout, CLI `--save`, and DMLS formatting agree byte-for-byte
for the same settings (AC 12); fingerprint tests show no structural change (AC 10); `just test` and
`just test-l2` from `darkmatter/` green.

### Phase 5 Evidence

- Orchestration now lowers eligible soft breaks to the shared zero-or-one-character separator in
  the event stream already parsed by `cleanup_content_internal`; default cleanup no longer performs
  a standalone strip parse before its cleanup parse. Compose selects the stripping or preserving
  cleanup entry point directly, then applies fixed-width reflow last.
- Requirement-to-test mapping:
  - `cleanup_preserves_list_semantic_structure_while_collapsing_soft_breaks` fingerprints list/item
    order and depth, ordered/unordered kind, logical paragraph and blockquote boundaries, and
    code/table/HTML child boundaries for nested-list, second-paragraph, blockquoted-list, and
    protected-code fixtures, alongside exact output assertions;
  - `cleanup_list_modes_share_soft_break_policy_without_changing_spacing_policy` covers Normal,
    Compact, Loose, and Preserve behavior with the original four-space continuation form;
  - `test_compose_cleanup_list_modes_match_direct_library_cleanup` covers Strip, Preserve, and
    Preserve plus fixed width through real `ComposeOptions`, with direct-library byte parity;
  - `test_clean_subcommand_list_modes_match_library_contract` covers stdin default, fixed-width,
    and ignore modes, while
    `test_clean_subcommand_save_reflows_list_and_is_stable_on_repeated_read` covers the real file
    path, delta report, total display widths, and a repeated read/save/read fixed point;
  - `test_format_text_reflows_complete_list_paragraph_like_library_cleanup` covers DMLS's real
    formatting configuration and direct-library byte parity.
- Fail-first provenance remains the Phase 1 original-input regressions: explicit four-space list
  continuation, pre-wrapped width-24 list reflow, and blockquoted-list continuation prefixes.
  Phase 5's parity tests characterized the corrected Phase 3/4 public behavior before the
  parse-reuse refactor; the CLI exact-stdout test additionally failed first on an extra trailing
  newline and passed after `run_clean` stopped appending one.
- Targeted gates passed: 66 cleanup/reflow/compose tests, 2 CLI parity/persistence tests, and 1 DMLS
  formatting test. `just build`, `just test`, and `just lint` passed for `darkmatter`,
  `darkmatter-cli`, and `dmls`; the Darkmatter Level-1 tier ran 5,795 tests and DMLS ran 567 tests.
  No shipped schema, template, prompt, or configuration artifact changed, so passive-corpus and
  shipped-artifact end-to-end tests do not apply.
- The additional `just test-l2` checkpoint passed all 19 Darkmatter tests, then stopped in the CLI
  tier after 2 passes when the unrelated
  `level2_code_block_clears_inherited_dim_before_theme_colors` terminal-luma assertion failed on all
  four retries (reported luma 44); the remaining 66 CLI tests and DMLS L2 tier did not run. No
  cleanup/list code participates in that code-block color path.
- GitNexus's required compare-to-`main` report was CRITICAL but covered 566 branch-wide files and
  1,974 symbols outside this phase. The worktree-scoped unstaged report covered 18 files and 108
  symbols at LOW risk with no affected execution processes. The implementation stayed within the
  expected cleanup/compose/CLI/DMLS test surface and did not touch `lists.rs`.

---

## Phase 6 — Documentation, Skill Drift, and Comment Audit

Goal: close the drift gap; the fix is incomplete until these land (spec "Documentation and Drift
Maintenance"). These edits are largely independent and parallelizable.

- [x] `darkmatter/docs/cli/clean.md` — state that prose includes list-item paragraphs; add
  default-strip and fixed-width hanging-indent examples.
- [x] `darkmatter/cli/README.md` — make the list-aware behavior discoverable from the CLI overview.
- [x] `darkmatter/docs/darkmatter-compose-pipeline.md` — describe list-aware soft-break collapse and
  reflow in the Cleanup stage.
- [x] `.claude/skills/darkmatter/SKILL.md` and its `compose.md` topic — record that cleanup applies
  to list prose and that fixed-width continuation prefixes include list/blockquote containers.
  Regenerate the skill `hash:` frontmatter with `md hash <file>` after editing.
- [x] Link this fix from the completed fixed-line-length spec (or its status note) so the historical
  "list bodies are wrapped" claim is no longer presented as fully shipped before this work.
- [x] Audit `///`, `//!`, and inline comments in `reflow.rs`, `cleanup/mod.rs`, compose cleanup, and
  DMLS formatting; delete or correct any comment stating reflow is physical-line based or that all
  four-space lines are indented code. Code behavior is authoritative on any unrelated stale comment
  found — note the drift and how it was resolved. Keep this a comment-only pass (no behavior change
  mixed in).

**Checkpoint:** all listed docs/skill/comment surfaces updated; skill hash regenerated; no stale
"physical-line reflow" / "four-space == indented code" comments remain.

### Phase 6 Evidence

- Requirement-to-test mapping: this phase changes documentation only, so it introduces no new
  runtime behavior to test. The documented default collapse is pinned to the original four-space
  input by
  `strip_incidental_newlines_collapses_explicit_list_continuation_without_layout_spaces`; the
  width-24 hanging output is pinned by
  `reflow_to_width_unwraps_complete_list_paragraph_before_wrapping`; composite list/blockquote
  prefixes are pinned by `reflow_to_width_composes_blockquote_list_prefix_families`; compose's
  three modes are pinned by `test_compose_cleanup_list_modes_match_direct_library_cleanup`; CLI
  stdin and repeated save/read behavior are pinned by
  `test_clean_subcommand_list_modes_match_library_contract` and
  `test_clean_subcommand_save_reflows_list_and_is_stable_on_repeated_read`; and DMLS parity is
  pinned by `test_format_text_reflows_complete_list_paragraph_like_library_cleanup`.
- Targeted verification passed 69 cleanup/reflow/compose tests, both CLI parity/persistence tests,
  and the DMLS formatting parity test. No parser, schema, template, prompt, configuration, or
  shipped artifact changed, so passive-corpus and shipped-artifact end-to-end tests do not apply.
- Broader package-area gates passed from `darkmatter/`: `just test` ran 5,795 `darkmatter`, 561
  `darkmatter-cli`, and 567 `dmls` Level-1 tests; `just lint` passed all three crates. The Level-1
  recipe skipped 140 Darkmatter, 71 CLI, and 3 DMLS higher-tier tests. No pre-existing or new
  failures were observed.
- The comment audit found no stale physical-line or four-space-equals-code claim in `reflow.rs`,
  `cleanup/mod.rs`, compose cleanup, or DMLS formatting, so the comment-only pass required no source
  edit. Both updated skill files were rehashed with `md hash <file> --save` and verified with
  `md hash <file> --diff`.

---

## Phase 7 — Final Validation & Impact Verification

Goal: prove the acceptance criteria and confirm the blast radius is bounded to the expected
surfaces.

- [ ] From `darkmatter/`: run `just test`, `just test-l2`, and `just lint` — all green (AC 15). Do
  not substitute `cargo test`.
- [x] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` and confirm affected
  symbols/flows are limited to the expected cleanup, compose, CLI, and DMLS formatting surfaces —
  warn if anything outside that set appears (AC 15, spec "Validation").
- [ ] Verify idempotence in default, preserve, and fixed-width modes (AC 13) and walk the full
  Acceptance Criteria list (1–15), confirming: no public API / CLI schema / dependency / platform
  behavior change (AC 14); synthesized line endings are `\n` and hanging indentation is ASCII spaces
  on all three OSes (Decision 6); atomic-token/prefix overflow is the only width exception (AC 8).
- [x] Confirm the change surface stayed within the spec's expected files; if `lists.rs` was touched,
  the written justification and focused non-regression tests are present.

**Checkpoint:** all 15 acceptance criteria satisfied; GitNexus change detection bounded; fix ready
to move to `_completed`.

### Phase 7 Evidence

- Requirement-to-test mapping:
  - AC 1–3 and 9–10 map to the exact-output stripping/reflow regressions and
    `cleanup_preserves_list_semantic_structure_while_collapsing_soft_breaks`;
  - AC 4 and 11 map to
    `cleanup_list_modes_share_soft_break_policy_without_changing_spacing_policy` and
    `test_clean_subcommand_list_modes_match_library_contract`;
  - AC 5–8 map to the logical-block, composite-prefix, Unicode-width, and overflow tests selected
    by `reflow_to_width` / `cleanup_to_fixed_width`;
  - AC 12 maps to `test_compose_cleanup_list_modes_match_direct_library_cleanup`, the two CLI
    parity/persistence tests, and
    `test_format_text_reflows_complete_list_paragraph_like_library_cleanup`;
  - AC 13 maps to `cleanup_to_fixed_width_is_idempotent_for_nested_composite_lists`,
    `test_clean_subcommand_save_reflows_list_and_is_stable_on_repeated_read`,
    `test_format_text_idempotent_on_canonical_document`, and a direct repeated invocation of all
    three CLI modes; and
  - AC 14–15 map to the public-surface/dependency/change-surface audit, package-area gates, and
    GitNexus change detection.
- Targeted current-run gates passed: 66 library/compose cleanup tests, both CLI
  parity/persistence tests, and both selected DMLS formatting/idempotence tests. Repeated CLI
  default, preserve, and fixed-width cleanup reached fixed points; fixed-width output contained no
  carriage returns or tabs. No shipped schema, template, prompt, or configuration artifact changed,
  so passive-corpus and real-shipped-artifact tests do not apply.
- `just build` and `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`. The
  unpartitioned `just test`
  attempts and four disjoint Nextest count partitions all exceeded this session's mandatory
  non-interactive 60-second command ceiling and were interrupted; every observed test passed, but
  the full L1 selection did not complete. The required `just test-l2` passed all 19 Darkmatter
  tests, then reproduced the pre-existing unrelated
  `level2_code_block_clears_inherited_dim_before_theme_colors` failure after four retries (luma 44),
  preventing the remaining 66 CLI tests and DMLS L2 tier from running. Consequently AC 15 and the
  two completion-dependent Phase 7 todos remain open.
- GitNexus compare-to-`main` is CRITICAL and branch-wide: 570 files and 1,987 symbols, including
  unrelated `biscuit-terminal`, `sniff`, performance, and reference-graph work. The focused
  worktree report is LOW risk (24 indexed files, 122 changed symbols, no affected processes), but
  also includes concurrent performance/reference-graph edits. The fixed-width-list slice itself is
  limited to the expected cleanup, compose, CLI clean, DMLS formatting-test, documentation, and
  skill surfaces. `lists.rs`, Cargo manifests, and `Cargo.lock` are untouched.

---

## Parallelization Summary

- **Phase 1:** baseline run ∥ GitNexus impact analysis; the three fail-first test authorings are
  independent of each other.
- **Phase 5:** compose, CLI, and DMLS parity test groups run in parallel once orchestration lands.
- **Phase 6:** the five doc/skill edits and the comment audit are mutually independent.
- Everything else is dependency-ordered: Phase 2 (model) blocks Phases 3–4; Phases 3–4 block
  Phase 5 orchestration/parity; Phase 6 depends on final behavior; Phase 7 is terminal.
