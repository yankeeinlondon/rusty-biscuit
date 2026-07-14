---
agent: claude/
total_phases: 7
created: 2026-07-14
phase: 1
yolo: "true"
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

- [ ] Run `just test` and `just lint` from `darkmatter/` and record a clean baseline; note any
  pre-existing failures so they are not attributed to this fix.
- [ ] Run GitNexus `impact({target: "strip_incidental_newlines", direction: "upstream"})` and
  `impact({target: "reflow_to_width", direction: "upstream"})`; capture the direct/transitive
  dependent counts and confirm they match the spec's CRITICAL/HIGH classification. Report the blast
  radius before editing. *(parallelizable with the baseline run)*
- [ ] Add a fail-first L1 test in `darkmatter/lib/src/markdown/cleanup/tests/reflow.rs` proving the
  **default-strip retained-indentation defect**: a `-` item whose continuation line is indented four
  spaces currently keeps the physical break / literal indentation; assert the exact single-logical-line
  output with one ASCII join space. Confirm it fails.
- [ ] Add a fail-first L1 test proving the **fixed-width physical-line defect**: a pre-wrapped `-`
  item at width 24 currently wraps only the first physical line; assert the full unwrap-then-rewrap
  output from the spec ("Alpha beta gamma delta / epsilon zeta eta / theta.") with per-line width
  assertions. Confirm it fails.
- [ ] Add a fail-first L1 test proving the **blockquoted-list continuation-prefix defect**
  (`> - …` continuation must stay `>   …`, not `> …`). Confirm it fails.

**Checkpoint:** three (or more) new tests fail for the documented reasons; baseline recipes are
otherwise green; impact numbers reported.

---

## Phase 2 — Shared Semantic Soft-Break / List-Prose Model (Decisions 1 & 2)

Goal: build the parser-driven classifier that both strip and reflow consume. No public behavior
change yet — this phase is additive private infrastructure with its own unit coverage.

- [ ] In `reflow.rs` (or a focused child module under `markdown/cleanup/`), add a private model
  built from `Parser::new_ext(content, cleanup_parser_options()).into_offset_iter()` — the **same**
  parser options as cleanup — that records, per candidate soft-break boundary:
  - the source span of the soft line ending (`Event::SoftBreak` offsets);
  - the source span of the next line's syntactic container/continuation prefix;
  - whether Darkmatter structural rules protect the boundary; and
  - the zero-or-one-character join-separator decision (reuse existing `join_separator`).
- [ ] Track active `List`, `Item`, `BlockQuote`, `Paragraph`, `CodeBlock`, `Table`, and HTML
  contexts from the event stream so a four-space continuation line inside a list item is classified
  as list prose, while genuine indented code / nested lists / second paragraphs are distinguished
  from parsed structure — not from leading-space counting.
- [ ] Keep the existing Darkmatter directive/fence/HTML/shell-block protection as an overlay
  (pulldown-cmark does not know `::` semantics), but ensure indentation heuristics never override
  parsed evidence that a line is list prose.
- [ ] Add private unit tests for the model in isolation (boundary eligibility, protection, prefix
  spans) covering: lazy vs explicit continuation, nested parent/child items, blockquoted list items,
  second-paragraph boundaries, and protected child blocks. These pin the classifier independent of
  the strip/reflow consumers.

**Checkpoint:** model unit tests pass; `strip_incidental_newlines` and `reflow_to_width` public
behavior is still unchanged (Phase 1 defect tests still fail — intended).

---

## Phase 3 — Default Stripping via the Shared Model (Decision 2, Mode row "Default")

Goal: rewrite `strip_incidental_newlines` to remove list soft breaks and their continuation-layout
prefix, inserting only the existing zero-or-one-character join separator, using non-overlapping
edits.

- [ ] Rewrite `strip_incidental_newlines` to consume the Phase-2 model: for each eligible boundary,
  remove the line ending, remove the next line's container/continuation-layout prefix, join via
  `join_separator`, and emit no other whitespace. Apply non-overlapping edits from the end of the
  source into a pre-sized buffer (Decision 2) — do not repeatedly mutate the middle of a `String`.
- [ ] Guarantee default strip synthesizes **no** hanging indentation and drops authored
  continuation indentation entirely (Goal 2, AC 3). A collapsed boundary yields either no character
  or exactly one `U+0020`.
- [ ] Preserve all non-list strip behavior byte-for-byte (paragraph boundaries, hard breaks,
  protected blocks, blockquote-only prose, non-list Unicode join policy). CRLF / lone-CR input
  normalizes identically.
- [ ] Make the Phase-1 strip fail-first test pass, then complete the **L1 stripping matrix**
  (spec "L1 library tests — stripping", items 1–12): 2/4/8-space unordered; lazy-3 / explicit-4
  ordered; `10.` / `10)`; checked+unchecked tasks; unindented lazy continuation; nested parent/child
  independent collapse; blockquoted + nested-blockquoted prefix removal; second-paragraph blank +
  indent preserved; hard breaks survive; protected fenced/indented/table/HTML/directive children;
  CRLF == LF canonical output; full Unicode separator parity (Han, Thai, Hangul, emoji, punctuation,
  ZWSP). At least one exact-string assertion contains a single ASCII join space.

**Checkpoint:** entire L1 stripping matrix green; Phase-1 strip and blockquote defect tests now
pass; `just test` from `darkmatter/` green for the library crate.

---

## Phase 4 — Fixed-Width Logical-Block Reflow + Composite Prefixes (Decisions 3 & 5)

Goal: reflow complete logical prose blocks (not physical lines) to the requested display width,
emitting the full composite hanging container prefix on every created continuation line.

- [ ] Give `reflow_to_width` a semantic map of reflowable prose blocks derived from parsed
  structure (Decision 3), replacing the current per-line `is_indented_code_line` protection so only
  actual parsed code/HTML/table blocks are protected — not every four-space line.
- [ ] For each list prose block derive `first_prefix` and `continuation_prefix`: the first
  paragraph's `first_prefix` carries the item marker/task box/separator; `continuation_prefix`
  replaces that marker region with **equal display-width ASCII spaces** (never tabs). A subsequent
  paragraph carries its required container indentation in both prefixes and no item marker.
- [ ] Make prefix parsing **compose** blockquote and list containers instead of returning after the
  first recognized family (replaces the current early-return in `line_reflow_prefix` /
  `blockquote_prefix_len` handling). `>   10. [ ] Body` yields the full byte prefix; continuation
  lines keep the quote marker and hang under the list body.
- [ ] Compute each item's continuation width from that item's actual post-cleanup serialized marker
  (handle `9.`→`10.` digit growth per item; do not reuse the first item's width). Consume the
  actual post-cleanup nesting indentation from the pipeline / `--indent` policy — do not hard-code
  2 or 4 spaces.
- [ ] Preserve the Decision-5 width/overflow contract: target is Unicode display columns including
  every prefix column; atomic tokens wider than the available body width (and bodies after a
  prefix that already meets/exceeds the width) are emitted intact. Word tokenization, inline-code
  protection, and spaceless-script joining are unchanged.
- [ ] Make the Phase-1 fixed-width fail-first test pass, then complete the **L1 fixed-width matrix**
  (spec "L1 library tests — fixed width", items 1–11): pre-wrapped unwrap+rewrap; differing ordered
  digit widths → different hanging prefixes; task alignment after checkbox; nested 2/4/8-space
  widths; blockquoted ordered/unordered/task composite prefixes; first vs subsequent paragraph
  independence; hard breaks + valid prefixes on both sides; long token not split under tight prefix;
  wide-Unicode measured with `UnicodeWidthStr` incl. prefix; protected children byte-equivalent;
  fixed-width idempotence. **Every** non-overflow fixture asserts the display width of every emitted
  physical line including its prefix.

**Checkpoint:** full L1 fixed-width matrix green; `cleanup_to_fixed_width` and `reflow_to_width`
satisfy the width contract; running fixed-width cleanup twice is a fixed point.

---

## Phase 5 — Orchestration & Cross-Surface Parity (Decision 4)

Goal: wire the shared model into full cleanup preserving pass order, and prove byte-equivalence at
every public surface. The four parity test groups are independent and parallelizable once
orchestration lands.

- [ ] In `cleanup/mod.rs`, keep the externally visible order (soft-break collapse → existing
  cleanup/list normalization → fixed-width reflow). Preferred implementation (Decision 2): lower
  eligible `Event::SoftBreak` to the chosen text separator before `pulldown-cmark-to-cmark`
  serialization, reusing the existing cleanup parse rather than adding a second full parse to the
  default path. If a different approach is chosen, demonstrate byte-equivalent output and justify.
- [ ] Add the **semantic structure regression helper**: parse source and output with the cleanup
  parser options and compare a structural fingerprint (list/item count+order, ordered vs unordered,
  nesting depth, paragraph boundaries, blockquote boundaries, code/table/HTML child boundaries)
  after ignoring soft-break events and offsets. Apply it to the nested-list, second-paragraph,
  blockquoted-list, and protected-code fixtures (fingerprint supplements, never replaces, exact
  string assertions).
- [ ] **Compose parity** (`compose/tests/rendering.rs`): prove all three modes through
  `ComposeOptions` — default `Strip` collapses, `Preserve` retains, `with_fixed_width(N)` forces
  collapse before reflow even when `Preserve` is also selected — and that output matches the direct
  library cleanup sequence. *(parallelizable)*
- [ ] **CLI parity** (`cli/tests/clean.rs`): stdin-driven `md clean -` collapses a four-space-wrapped
  item; `md clean --fixed-width N -` reflows the whole item and satisfies total-line width;
  `md clean --ignore-incidental-newlines -` retains canonical list soft breaks; `--save` writes
  corrected wrapping and reports the delta. Existing arg-conflict / width-range tests unchanged.
  *(parallelizable)*
- [ ] **DMLS parity** (`dmls/src/providers/formatting.rs`, tests only): extend formatting parity with
  a pre-wrapped list fixture; `fixed_width = N` output is byte-identical to the
  `Markdown::cleanup` + `reflow_to_width` sequence used by `md clean`. *(parallelizable)*
- [ ] Confirm `--ignore-incidental-newlines` performs no list soft-break collapse and no fixed-width
  synthesis (AC 4), and that default/compact/loose list-spacing modes retain existing behavior
  (AC 11).

**Checkpoint:** library, compose, CLI stdout, CLI `--save`, and DMLS formatting agree byte-for-byte
for the same settings (AC 12); fingerprint tests show no structural change (AC 10); `just test` and
`just test-l2` from `darkmatter/` green.

---

## Phase 6 — Documentation, Skill Drift, and Comment Audit

Goal: close the drift gap; the fix is incomplete until these land (spec "Documentation and Drift
Maintenance"). These edits are largely independent and parallelizable.

- [ ] `darkmatter/docs/cli/clean.md` — state that prose includes list-item paragraphs; add
  default-strip and fixed-width hanging-indent examples.
- [ ] `darkmatter/cli/README.md` — make the list-aware behavior discoverable from the CLI overview.
- [ ] `darkmatter/docs/darkmatter-compose-pipeline.md` — describe list-aware soft-break collapse and
  reflow in the Cleanup stage.
- [ ] `.claude/skills/darkmatter/SKILL.md` and its `compose.md` topic — record that cleanup applies
  to list prose and that fixed-width continuation prefixes include list/blockquote containers.
  Regenerate the skill `hash:` frontmatter with `md hash <file>` after editing.
- [ ] Link this fix from the completed fixed-line-length spec (or its status note) so the historical
  "list bodies are wrapped" claim is no longer presented as fully shipped before this work.
- [ ] Audit `///`, `//!`, and inline comments in `reflow.rs`, `cleanup/mod.rs`, compose cleanup, and
  DMLS formatting; delete or correct any comment stating reflow is physical-line based or that all
  four-space lines are indented code. Code behavior is authoritative on any unrelated stale comment
  found — note the drift and how it was resolved. Keep this a comment-only pass (no behavior change
  mixed in).

**Checkpoint:** all listed docs/skill/comment surfaces updated; skill hash regenerated; no stale
"physical-line reflow" / "four-space == indented code" comments remain.

---

## Phase 7 — Final Validation & Impact Verification

Goal: prove the acceptance criteria and confirm the blast radius is bounded to the expected
surfaces.

- [ ] From `darkmatter/`: run `just test`, `just test-l2`, and `just lint` — all green (AC 15). Do
  not substitute `cargo test`.
- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})` and confirm affected
  symbols/flows are limited to the expected cleanup, compose, CLI, and DMLS formatting surfaces —
  warn if anything outside that set appears (AC 15, spec "Validation").
- [ ] Verify idempotence in default, preserve, and fixed-width modes (AC 13) and walk the full
  Acceptance Criteria list (1–15), confirming: no public API / CLI schema / dependency / platform
  behavior change (AC 14); synthesized line endings are `\n` and hanging indentation is ASCII spaces
  on all three OSes (Decision 6); atomic-token/prefix overflow is the only width exception (AC 8).
- [ ] Confirm the change surface stayed within the spec's expected files; if `lists.rs` was touched,
  the written justification and focused non-regression tests are present.

**Checkpoint:** all 15 acceptance criteria satisfied; GitNexus change detection bounded; fix ready
to move to `_completed`.

---

## Parallelization Summary

- **Phase 1:** baseline run ∥ GitNexus impact analysis; the three fail-first test authorings are
  independent of each other.
- **Phase 5:** compose, CLI, and DMLS parity test groups run in parallel once orchestration lands.
- **Phase 6:** the five doc/skill edits and the comment audit are mutually independent.
- Everything else is dependency-ordered: Phase 2 (model) blocks Phases 3–4; Phases 3–4 block
  Phase 5 orchestration/parity; Phase 6 depends on final behavior; Phase 7 is terminal.
