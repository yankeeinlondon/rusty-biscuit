---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/claudine/fixes/2026-10-20-handle-indented-frontmatter/spec.md"
plan: "claudine/fixes/2026-10-20-handle-indented-frontmatter/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - biscuit-file
  - darkmatter
  - claudine
human_review: false
source_files_during_phase_2:
  - biscuit-file/lib/src/yaml/analyze/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/engine.rs
  - biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/tests/mod.rs
  - biscuit-file/lib/src/yaml/analyze/tests/root_indentation.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/tests/l1/inline_document_text.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
  - darkmatter/cli/tests/l1/clean_frontmatter.rs
  - darkmatter/dmls/tests/l1/formatting_transaction.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/timeline.md
source_files_during_phase_4:
  - claudine/cli/tests/bin/fake_goose/main.rs
  - claudine/cli/tests/l1/inline_compose_indented_frontmatter.rs
  - claudine/cli/tests/l1/main.rs
docs_updated_during_phase_4:
  - darkmatter/docs/cli/clean.md
  - darkmatter/docs/cli/hash.md
  - biscuit-file/lib/README.md
  - claudine/docs/topics/composition.md
  - darkmatter/cli/README.md
  - claudine/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/cli-commands.md
  - .claude/skills/os/windows.md
  - .claude/skills/darkmatter/cleanup.md
  - .claude/skills/darkmatter/frontmatter.md
source_code:
  - biscuit-file/lib/src/yaml/analyze/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/engine.rs
  - biscuit-file/lib/src/yaml/analyze/tests/diagnostic.rs
  - biscuit-file/lib/src/yaml/analyze/tests/mod.rs
  - biscuit-file/lib/src/yaml/analyze/tests/root_indentation.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/tests/l1/inline_document_text.rs
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - darkmatter/lib/tests/l1/clean_frontmatter_repair.rs
  - darkmatter/cli/tests/l1/clean_frontmatter.rs
  - darkmatter/dmls/tests/l1/formatting_transaction.rs
  - claudine/cli/tests/bin/fake_goose/main.rs
  - claudine/cli/tests/l1/inline_compose_indented_frontmatter.rs
  - claudine/cli/tests/l1/main.rs
documentation:
  - darkmatter/docs/cli/clean.md
  - darkmatter/docs/cli/hash.md
  - biscuit-file/lib/README.md
  - claudine/docs/topics/composition.md
  - darkmatter/cli/README.md
  - claudine/README.md
  - .claude/skills/claudine/timeline.md
  - .claude/skills/claudine/cli-commands.md
  - .claude/skills/darkmatter/cleanup.md
  - .claude/skills/darkmatter/frontmatter.md
  - .claude/skills/os/windows.md
completed_phase: 4
implemented: true
---

# Implementation Log for 2026-10-20-handle-indented-frontmatter (4 phases)

## Phase 1

Phase 1 is a bounded contract audit; no source, test, or documentation files
were changed. Three read-only audits (biscuit-file, Darkmatter/DMLS, Claudine)
were run in parallel and their findings consolidated below. Paths are relative
to the repository root.

### biscuit-file YAML analyzer (`biscuit-file/lib/src/yaml/analyze/`)

- `analyze_yaml(source: &str) -> YamlAnalysis` (`engine.rs:65`). Parseable input
  runs `s1_candidates` then `prove_drafts`; unparseable input runs
  `bom_recovery` → `tab_indentation_recovery` → `recover::recover`.
  `YamlAnalysis::apply() -> EditSetOutcome` (`analysis.rs:142`) applies every
  auto-apply-eligible repair.
- `s1_candidates` (`engine.rs:137`) order: BOM, line endings, final newline,
  trailing whitespace, block whitespace, flow whitespace. The new candidate
  belongs here.
- `Draft` carries exactly **one** `YamlRepair { span, replacement, explanation }`.
  `prove_drafts` (`engine.rs:155-181`) applies all drafts together; on failure
  it re-tests each draft alone, then the survivors together.
  **Consequence:** per-line dedent drafts cannot pass alone, so a combined
  failure would drop all of them. Phase 2 must either let one diagnostic carry
  several prefix repairs (`YamlAnalysis::apply` already flattens repairs), or
  prove the dedent as one unit, as `tab_indentation_recovery`
  (`engine.rs:236-272`) does. The equality proof (`parses_equal`) and overlap
  rejection (`apply_edit_set`, `edit_set.rs:73/94`) stay authoritative.
- Overlap with existing edits when removing N leading spaces:
  - line-ending (`\r\n` at line end) and missing-final-newline (zero-width at
    `len`): no overlap.
  - trailing whitespace (`engine.rs:359-390`): **overlaps** on whitespace-only
    `Blank` lines (span starts at the line start). It skips `BlockContent`.
    The dedent must skip `Blank` lines outside block scalars, matching the spec.
  - extra trailing blank lines (`engine.rs:336-351`): overlaps if a dedent
    touches that run; skip it.
  - flow whitespace (`engine.rs:442-559`): overlaps on multi-line flow
    continuation lines starting with `]`, `}`, `,`, `:`. Flow roots are
    excluded, but nested multi-line flow values inside an eligible root also
    need the dedent to stay out of their leading runs, or to let the proof
    refuse.
- `scan.rs`: `LineKind` = `Blank`, `Comment`, `DocumentStart`, `DocumentEnd`,
  `Content`, `BlockContent`. `Comment` means `#` is the first byte after the
  indent, at any depth. `---`/`...` count as markers only at column 0. **There
  is no directive kind**: `%YAML` is `Content`, so eligibility must detect
  directives itself. `detect_block_scalars` marks `Blank`, deeper `Comment`,
  and deeper `Content` lines after a `|`/`>` header as `BlockContent`, so
  scalar lines beginning with `#` are already `BlockContent`, not `Comment`.
  `Line.indent` counts spaces **and tabs**, so the dedent must count only ASCII
  spaces.
- Code inventory (`diagnostic.rs`), four places: enum + `#[serde(rename)]`
  (:51-127), `ALL: [Self; 24]` → 25 (:131-156), `as_str` (:160-187), and the
  position-sensitive `test_code_spellings_pinned`
  (`src/yaml/analyze/tests/diagnostic.rs:88-124`). The other inventory tests
  loop over `ALL`. Downstream code compares codes directly; there is no
  exhaustive external match.
- Tests: `tab_indentation.rs` is a **unit** module at
  `src/yaml/analyze/tests/tab_indentation.rs`, declared in
  `src/yaml/analyze/tests/mod.rs` with helper `assert_untouched_bytes_preserved`.
  New tests go in `src/yaml/analyze/tests/root_indentation.rs` with
  `mod root_indentation;` (between `reserved_indicator` and `scan`). Because
  the package sets `autotests = false`, an integration file would need `mod` in
  `tests/l1/main.rs`.
- Docs: `biscuit-file/lib/README.md` "YAML Source Analysis and Repair" (:145;
  `yaml.tab-indentation` at :217).

### Darkmatter editor gate (`darkmatter/lib/src/markdown/hash/write.rs`)

- `validate_block_mapping` (:737) has three callers:
  - `parse_text_frontmatter` (:164) → used by `restore_properties_text` (:87)
    for **both** current (:92) and snapshot (:93), before any no-op; it also
    reparses after each restore (:109, :143). No block → empty result
    without validation.
  - `apply_hash_save_text` (:314): **the early return at :320-322
    (`new_stored == None`) precedes validation (:340).** The spec requires the
    check before any no-op, so Phase 2 must move validation ahead of that return
    (only when a block exists). The `last_updated` pass (:368-391) does not
    revalidate.
  - `locate_frontmatter_leaves` (:543): empty `paths` returns `Ok(vec![])`
    before validation (:551); otherwise validates at :565 →
    `LeafLocateError::Document`. Claudine's `persist.rs:192-196` also returns
    early on empty paths. The spec's "leaf location returns the same cause" is
    tested with non-empty paths; keeping the empty-paths short-circuit avoids
    widening behavior.
- Existing messages to retain: flow root (`"flow-style frontmatter roots are not
  supported"`, detected after `trim_start`), scalar/sequence (`"frontmatter must
  be a top-level block mapping"`), parse failure (`"frontmatter YAML could not
  be parsed: …"`). `Value::Null` is accepted (:748). **Comment-only frontmatter
  has no test**, and `frontmatter_values` only special-cases whitespace (:185).
  Phase 2 adds an accepted-comment-only control and fixes the path if it fails.
- `MarkdownError::FrontmatterTextEdit { reason: String }` (`types.rs:142`),
  built by `text_edit_error` (:690).
- `Markdown::stamp_baseline(&self, source, &MdHashOptions, now: DateTime<Utc>,
  Change) -> MarkdownResult<BaselineStamp>` (`baseline.rs:69`); `Change::Known`
  forces `bump_last_updated`. External callers: `md hash` (`Change::Detect`),
  `effects/verbs.rs:45`, Claudine `closure.rs:177`.
- Tests: in-file `mod tests` (:1177; `assert_refused` helpers :1746/:1764
  also assert unchanged input); `darkmatter/lib/tests/l1/inline_document_text.rs`
  (public restore API); `schemas_source_projection.rs:986` (leaf location). All
  are declared in `darkmatter/lib/tests/l1/main.rs` (`autotests = false`).

### Darkmatter cleanup and DMLS

- `repair_frontmatter` (`cleanup/frontmatter.rs:176`): first pass
  `analyze_yaml` + `apply`, then a **rescan only when the text changed and
  parses** (:243-266), keeping only new findings. A tab-indented root fails the
  first parse, is fixed by tab recovery, and then gets the root dedent from the
  rescan. That covers "verify cleanup's rescan enables the new repair". An
  already-parseable indented root gets the repair in the first pass.
- `clean_document` (`cleanup/document.rs:98`) is shared by `md clean`
  (`darkmatter/cli/src/commands/clean.rs`) and DMLS formatting
  (`darkmatter/dmls/src/providers/formatting.rs:50`).
- Test homes: `darkmatter/lib/tests/l1/clean_frontmatter_repair.rs`;
  `darkmatter/cli/tests/l1/clean_frontmatter.rs` (`CliProcessFixture::named`,
  `doc()` / `clean_to_string` helpers); `darkmatter/dmls/tests/l1/formatting_transaction.rs`.
  All are already declared in their `l1/main.rs`.
- Docs: `darkmatter/docs/cli/clean.md` (:148), `darkmatter/docs/inline/cleaning.md`,
  `darkmatter/docs/lsp/frontmatter.md`, `.claude/skills/darkmatter/cleanup.md`
  (:21-22). There is no `darkmatter/docs/cleanup.md`; the spec's "cleanup topic
  page" is `docs/cli/clean.md`.

### Claudine inline preparation and closure

- `prepare_inline_on` (`claudine/lib/src/composition/prepare.rs:1018`) order:
  removed-grammar scans (:1024-1036, no side effects) → prompt read (:1038) →
  request setup (:1049-1065) → **`compose_selecting` (:1066, runs frontmatter
  `$(…)`)** → schema verdict → **`load_or_create_guardrails` (:1099, writes
  `<repo>/.claudine/inline-compose.md`)** → hash capture (:1106) →
  `InlineClosurePlan` (:1131). The preflight goes right after the
  removed-grammar scans and before `compose_selecting`.
- `source.original_text` is captured in `resolve.rs:211`
  (`resolve_composition_source`) and re-read by `reload_composition_source`
  (:361). The drop-optionals clone keeps it unchanged.
- Callers that pass through `prepare_inline_on`, and so all get the check:
  `prepare_inline`/`_selecting`, `prepare_loop_seed`, `prepare_sequence_probe`,
  `prepare_inline_with_schema` (and its drop-optional retry, which runs the
  check twice harmlessly), `prepare_document` (service). From the CLI: the
  ordinary run, `--dry-run` (`compose/prep.rs:1739`), the staged initialize
  reread, loop iterations, proxy/retry/resume through
  `harness_orch/prompt.rs:336`, and sequence JIT. All of these are inline
  preparation, so ordinary `compose`/`sequence` stays unaffected.
- **Ordering caveat (resolved):** `eager_shell_preflight` →
  `approve_document_shell` runs before `prepare_inline_on`, but it calls
  Darkmatter's `compose_preflight`, a condition-blind *discovery* walk
  (`preflight.rs:81-110`). It approves commands and does not execute them. A
  non-TTY run needs `--yolo` (or prior approval) to reach preparation. Phase 3's
  sentinel test proves no execution empirically.
- Closure helpers: `CLOSURE_OWNED_PROPERTIES` (`closure.rs:32`),
  `inline_hash_options()` (:224), `restore_properties_text` (:158) and
  `stamp_baseline(..., Change::Known)` (:176) inside
  `reconcile_inline_artifact_with_evidence` (:121), `restore_inline_baseline`
  (:208). There is no clock seam: closure takes `now: DateTime<Utc>`, and
  production passes `Utc::now()` (`harness_orch/loop_control.rs:2390`). Since
  `PrepareOptions` has no time field, `prepare_inline_on` passes `Utc::now()`
  into the helper, which takes `now` as a parameter so helper tests can pin it.
- Error integration (`composition/error/`):
  - New `InlineBaselineUnstampable { path, #[source] source: Box<MarkdownError> }`.
    Box it, because `MarkdownError` sits at the `result_large_err` limit; the
    existing precedent is `LifecycleShellTemplateRefused`.
  - Add arms in `render/mod.rs::status_block`, in the typed-Darkmatter semantic
    arm (:36-40).
  - `role()` defaults to `Semantic`.
  - `code()` (:398-472) defaults to `composition.failed`. A dedicated code would
    need a catalog `code_spec` and a corpus entry in
    `claudine/cli/tests/l1/error_guards.rs` (~:1100-1180) because of the
    `the_corpus_covers_every_code…` guard.
  - Check `detail()` path projection (:649) and the batch detail list in
    `error/tests.rs:2361`.
  - None of these matches is fully exhaustive (all have `_`), so the compiler
    will not flag missing arms. Phase 3 must add them deliberately.
- Tests: `closure/tests.rs` (`plan()`/`agent_run()` helpers, fixed-timestamp
  pattern); `prepare/tests.rs` (source builder :55); CLI
  `inline_compose_hash.rs` (cross-platform). `inline_compose_cli.rs` is
  `#[cfg(unix)]`, so new cross-platform CLI tests must not go there. The
  cross-platform launch marker is `install_marker_provider` / `provider_runs` /
  `run_with_marker` in `claudine/cli/tests/l1/wrap_compose_validation.rs:615-642`
  (Unix `sh` stub; Windows copies `claudine-fake-goose`). That module is gated
  `#[cfg(feature = "test-fixtures")]`. Model test:
  `inline_compose_rejects_a_nested_span_before_any_provider_starts` (:714). A
  guardrail-absence assertion needs a git-repo fixture (`init_git_repo`).
- Docs: `claudine/docs/topics/composition.md` "## Inline Composition" (:377),
  "### The inline guard and rollback" (:514).

### Requirement-to-test checklist (owners: later phases)

| # | Requirement | Public observation | Test home | Phase |
| - | --- | --- | --- | --- |
| 1 | Indented ordinary mapping is dedented; parsed value unchanged (positive control, reproduction fixture) | `analyze_yaml(src).apply()` → column-0 keys, `parsed_value` equal, code `yaml.root-indentation` | `biscuit-file/lib/src/yaml/analyze/tests/root_indentation.rs` | 2 |
| 2 | Column-0 mapping and excluded roots (empty, comment-only, sequence, scalar, flow, directive, `---` marker, complex key, tagged root) get no root-indentation repair | no diagnostic with the new code | same | 2 |
| 3 | Less-indented comments retained; qualifying comments dedented | applied bytes | same | 2 |
| 4 | Whitespace-only `BlockContent` lines at, below, and above the content indent keep the scalar value; whitespace-only lines outside scalars are left to `yaml.trailing-whitespace` | value equality + no overlap rejection | same | 2 |
| 5 | LF/CRLF terminators preserved; combines with line-ending, trailing-whitespace, and final-newline edits | combined `apply()` parse-equivalent, no rejected edits | same | 2 |
| 6 | Scalar safety: literal/folded, indentation indicators, blank scalar lines, multiline quoted, scalar `#` lines | exact value preserved or no repair offered | same | 2 |
| 7 | Invalid documents (duplicate key, trailing garbage, second document) keep their existing outcomes with no new repair; tab-indented root is fixed by recovery and then the rescan | analyzer result; `repair_frontmatter` / `clean_document` result | `root_indentation.rs` + `darkmatter/lib/tests/l1/clean_frontmatter_repair.rs` | 2 |
| 8 | Code inventory/serialization consistent | `ALL` (25), pinned spellings | `src/yaml/analyze/tests/diagnostic.rs` | 2 |
| 9 | Editor rejects indented root with the spec's reason, before any no-op, text untouched | `apply_hash_save_text`, `restore_properties_text` (each input), `locate_frontmatter_leaves` (non-empty paths) → `FrontmatterTextEdit` | write.rs `mod tests` + `darkmatter/lib/tests/l1/inline_document_text.rs` | 2 |
| 10 | Editor controls: column-0 accepted; comments before keys do not count as the root key; empty/comment-only accepted; existing flow/scalar/parse errors unchanged | same APIs | same | 2 |
| 11 | DMLS formatting offers the same edit; stable on a second pass | formatting result + second pass no root edit | `darkmatter/dmls/tests/l1/formatting_transaction.rs` | 3 |
| 12 | `md clean --save` then `md hash --save` succeeds (read/write/read round trip); `md hash --save` on the uncleaned doc fails with the reason and leaves bytes unchanged | CLI exit, stderr, file bytes | `darkmatter/cli/tests/l1/clean_frontmatter.rs` (+ hash CLI test) | 3 |
| 13 | Baseline rehearsal helper refuses an indented baseline; accepts column-0; fixed `now` | helper `Result`, typed `MarkdownError` cause | `claudine/lib/src/composition/closure/tests.rs` | 3 |
| 14 | `prepare_inline_on` refuses before compose: no `$(…)` sentinel, no guardrail file, source path + typed cause, bytes unchanged | `InlineBaselineUnstampable { path, source }` | `claudine/lib/src/composition/prepare/tests.rs` | 3 |
| 15 | `inline-compose` (run and `--dry-run`) on an indented doc exits non-zero, zero provider launches, document byte-identical | CLI via `install_marker_provider` | `claudine/cli/tests/l1/wrap_compose_validation.rs` (or a sibling under the `test-fixtures` gate) | 4 |
| 16 | After `md clean --save`, the same `inline-compose` completes and stamps `hash` at column 0 | CLI + file read-back | Claudine CLI L1 | 4 |
| 17 | Column-0 baseline + agent-indented candidate → closure refuses and restores the original bytes | `reconcile_inline_artifact*` result + file bytes | `closure/tests.rs` | 3/4 |
| 18 | Existing inline closure tests unchanged | existing suite | — | every phase |
| 19 | Error rendering/catalog: status block, code, detail, corpus | `error/tests.rs`, `error_guards.rs` | Claudine | 3 |

**Checkpoint met:** every success criterion has an assigned public observation.
Operation ordering (preflight before `compose_selecting` and the guardrail
write; approval is discovery-only) and fixture portability
(`install_marker_provider`, not `launch_recorder`/`inline_compose_cli.rs`) are
established. No spike was run.

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: ["darkmatter"]`, `change_class: package`. The selection comes from
  uncommitted changes that predate this phase (`codebook.toml`, biscuit-terminal
  and biscuit-visualized docs), not from this phase's plan/log edits. Because
  `packages` is non-empty, the gates were run, not skipped.
- `just test` (claudine area): 10448 passed, 9 skipped, 0 failed (336.5 s).
- `just lint` (claudine area): passed.
- No source changed, so no dependent-package tests or cross-OS check apply.
- The Claudine skill needs no update: no behavior changed in this phase.

## Phase 2

Both Wave 2 tasks were implemented directly (no subagents), biscuit-file first,
then Darkmatter, in separate file sets.

### Repair Root Indentation (biscuit-file)

- `YamlDiagnosticCode::RootIndentation` (`yaml.root-indentation`) added to the
  enum, `ALL` (now 25, placed after `TabIndentation`), `as_str`, and the pinned
  spelling list in `src/yaml/analyze/tests/diagnostic.rs`. The `FromStr`/serde
  round-trip tests loop over `ALL` and cover it.
- `Draft` now carries `repairs: Vec<YamlRepair>` and is proven as a unit;
  `prove_drafts` and `tab_indentation_recovery` share a new `proves_equal`
  helper. The combined/individual/survivor proof sequence is unchanged, so the
  per-line trap from Phase 1 is avoided without weakening the proof.
- `root_indentation_candidate` runs inside `s1_candidates` (after trailing
  whitespace). Eligibility: parsed value is a nonempty mapping; no document
  markers; the first `Content` line is a non-dash mapping entry whose key does
  not start with `? ! & * % [ {`. Common indent is the minimum leading
  ASCII-space count over nonblank `Content`/`BlockContent` lines. Edits:
  `Content` lines lose the common indent, `Comment` lines only when they have
  that many spaces, `BlockContent` lines (including whitespace-only) lose up to
  it, `Blank` lines are left to `yaml.trailing-whitespace`.
- **Departure, recorded here:** `Content` lines that start inside a flow
  collection or a quoted scalar keep their indentation. The flow-whitespace
  candidate removes leading runs before `]`/`}`/`,`, so dedenting those lines
  would overlap it, and an overlap drops every survivor on the combined
  re-proof (it would have cost unrelated line-ending repairs). Folding makes
  their indentation insignificant, and the second pass offers nothing because
  the root keys are at column 0. The spec does not require dedenting them; it
  requires parse equivalence, which holds.
- Diagnostic span: first edit start to last edit end; message
  `root mapping is indented by N spaces`.

### Guard Text Editors (Darkmatter `markdown/hash/write.rs`)

- `validate_block_mapping` keeps its flow-root, scalar/sequence, and parse
  errors, and for a parsed mapping refuses a first non-blank, non-comment line
  with leading spaces, with the spec's exact reason (singular "1 space").
  Helpers: `first_content_line`, `root_indent`, `indented_root_error`.
- `apply_hash_save_text` no-op (`new_stored == None`): previously returned
  before any validation, and the existing test
  `textual_no_change_does_not_parse_unsupported_source` pins that a no-op never
  parses unsupported sources. To satisfy both that and the spec's
  "even if the operation would otherwise be a no-op", the no-op branch runs
  only `refuse_indented_root`, which parses only when the first key line is
  indented, and only for a block that extracts cleanly (a no-op never reported
  near-miss fences). The write path still runs the full gate before key lookup.
- `restore_properties_text` and `locate_frontmatter_leaves` already reach the
  gate through `parse_text_frontmatter` / their own call; no code change.
- Comment-only frontmatter was already accepted (serde gives `Null`, and the
  map deserializer accepts it); the new control confirms it, no fix needed.
- API docs on `apply_hash_save_text`, `restore_properties_text`, and
  `locate_frontmatter_leaves` now state the column-0 constraint.

### Requirement-to-test mapping (Phase 2 rows 1–10 of the Phase 1 checklist)

| # | Requirement | Test |
| - | --- | --- |
| 1 | Reproduction dedented, value kept | `root_indentation::test_reproduction_prompt_block_moves_to_column_zero`, `test_common_indent_is_removed_preserving_order_nesting_and_comments` |
| 2 | Column-0 and excluded roots get no repair | `test_column_zero_mapping_gets_no_root_repair`, `test_excluded_root_forms_get_no_root_repair` (empty, whitespace, comment-only, sequence, scalar, flow mapping/sequence, `?` complex key, `!!map` tag, `&` anchor, `---`, `%YAML`, `...`) |
| 3 | Comments | `test_less_indented_comments_are_kept`, row 1 tests |
| 4 | Whitespace-only lines | `test_whitespace_only_scalar_lines_at_below_and_above_content_indent`, `test_blank_lines_outside_block_scalars_are_left_to_trailing_whitespace` |
| 5 | LF/CRLF and combined edits | `test_crlf_terminators_survive_prefix_edits_and_combine_with_line_endings`, `test_combines_with_trailing_whitespace_and_final_newline_repairs`, `test_missing_final_newline_combines_with_dedent` |
| 6 | Scalar safety | `test_literal_and_folded_scalars_keep_their_values`, `test_explicit_indentation_indicator_keeps_value`, `test_scalar_lines_beginning_with_hash_are_content_not_comments`, `test_multiline_quoted_and_plain_scalars_keep_their_values`, `test_nested_multiline_flow_value_keeps_its_value` |
| 7 | Invalid documents; tab recovery then rescan | `test_invalid_documents_keep_their_outcomes_without_a_root_repair`, `test_tab_indented_root_is_recovered_first_then_dedented_on_rescan` (cleanup-level rescan is Phase 3's) |
| 8 | Code inventory | `diagnostic::test_code_spellings_pinned` (+ existing `ALL` loops) |
| 9 | Editor refusal before any no-op, text untouched | `write::tests::hash_save_refuses_the_indented_reproduction_with_the_indentation_reason`, `hash_save_refuses_an_indented_root_even_when_no_write_is_needed`, `restore_refuses_indentation_in_either_input`, `leaf_location_refuses_an_indented_root_with_the_same_cause`, `l1::inline_document_text::restore_refuses_an_indented_root_through_the_public_api` |
| 10 | Editor controls | `comments_and_nested_values_do_not_count_as_the_root_key`, `empty_and_comment_only_frontmatter_stay_editable`, `indented_unsupported_roots_keep_their_existing_reasons`, column-0 controls inside row 9 tests |

Every root-indentation test asserts audit-clean application, byte preservation
outside applied spans (`assert_untouched_bytes_preserved`), parsed-value
equality, and that a second analysis offers no root repair. Boundary: these are
in-process L1 tests at each crate's public API (`analyze_yaml` /
`YamlAnalysis::apply`; `apply_hash_save_text`, `restore_properties_text`,
`locate_frontmatter_leaves`). The CLI round trip (`md clean --save` then
`md hash --save`) and DMLS are Phase 3.

Robustness smell grep over the changed parsing paths: no `#[serde(default)]`,
`unwrap_or_default()`, or `filter_map(..as_str())` added. One intentional
`if let Ok(Some(..)) = extract_frontmatter_block(..)` in the no-op branch, with
a comment: the no-op path never reported fence errors, and the write path
still propagates them.

### Gates

- biscuit-file: `just test` 1161 passed; `just lint` clean.
- darkmatter (includes DMLS): `just test` 10490 passed, 10 skipped; `just lint` clean.
- content-policy (only other analyzer consumer, filters on `TabIndentation`):
  `cargo nextest run -p content-policy -E 'test(/tab/)'` 9 passed.
- claudine: `just test` 10448 passed (12 slow), 9 skipped; `just lint` clean
  (only the pre-existing `__eh_frame` linker warning).
- Scope calculator: `packages: ['biscuit-file', 'darkmatter', 'test-toolkit']`,
  `change_class: package`, so the gates were run. `test-toolkit` is a reverse
  dependency of biscuit-file and does not use the analyzer or editor; not run
  separately.
- No failures, skips introduced, or load-sensitive reruns.
- Cross-OS check not run: the changes are byte-level string logic with no
  path, process, or `cfg` code; CRLF is covered by tests.
- No docs or skills changed in this phase; Phase 4 owns them (see
  `message_to_agent`). The claudine skill needs no update: Claudine code did
  not change.

## Phase 3

The two Wave 3 tasks ran as parallel subagents in disjoint file sets
(Darkmatter cleanup/CLI/DMLS tests; Claudine closure/prepare/error), each
verifying only its own slice. The area gates were run once afterwards.

### Verify Cleanup Consumers (Darkmatter, test-only)

No product code changed and no defect was found: `repair_frontmatter`,
`clean_document`, `md clean`, and DMLS formatting all receive the analyzer's
`yaml.root-indentation` repair through the existing shared path.

- `darkmatter/lib/tests/l1/clean_frontmatter_repair.rs`
  - `indented_root_reproduction_is_dedented_by_one_root_indentation_finding`:
    exact output, a single finding carrying many space-only edits, all of them
    applied, and an equal parsed value.
  - `clean_document_applies_the_root_repair_and_keeps_the_parsed_value`
  - `indented_root_combines_with_trailing_whitespace_and_final_blank_lines`
  - `crlf_indented_root_keeps_prefix_edits_off_terminators`: every
    root-indentation edit removes only spaces. The frontmatter's switch to LF
    comes from a separate `yaml.line-ending` finding, and the fences and body
    keep CRLF under the existing cleanup rules. This keeps prefix-edit
    terminator fidelity apart from the other line-ending rules.
  - `tab_indented_root_is_recovered_then_dedented_by_the_rescan`:
    `yaml.tab-indentation` first, then one `yaml.root-indentation` from the
    rescan.
  - Stability: a second pass through `repair_frontmatter` and
    `clean_document` reports no `RootIndentation`, applies nothing, and
    returns `Unchanged`. The check uses the reported codes, not text equality.
- `darkmatter/cli/tests/l1/clean_frontmatter.rs`
  - `test_indented_root_is_refused_by_hash_save_until_clean_save_dedents_it`
    (five spawns through `CliProcessFixture`):
    1. `md hash --save` fails with the exact indentation reason, and the bytes
       are unchanged.
    2. `md clean --save` runs; the reread has column-0 keys and the same
       parsed data.
    3. `md hash --save` succeeds; the reread has a column-0 `hash:` stamp and
       the frontmatter, minus the stamps, equals the original.
    4. `md hash --diff` exits 0.

    The reason is compared after stripping whitespace and backticks, because
    the terminal renderer styles the code span and wraps lines.
- `darkmatter/dmls/tests/l1/formatting_transaction.rs`
  - `an_indented_root_is_dedented_once_and_keeps_its_value`: the edit matches
    `clean_document`. After `didChange`, a second formatting request returns
    `[]`.
  - `a_tab_indented_root_is_recovered_and_dedented_in_one_transaction`
- Scoped runs: darkmatter `clean_frontmatter_repair::` 15 passed;
  darkmatter-cli `clean_frontmatter::test_indented_root` passed (one rerun,
  after a test-assertion fix for the rendered code span); dmls
  `formatting_transaction::` 16 passed; `just lint` (darkmatter) clean.
  `clean_frontmatter.rs` was already over the 500-line guideline (739 → 852;
  report-only).

### Preflight Inline Baselines (Claudine)

- `closure::rehearse_inline_baseline(original, now) -> Result<(), MarkdownError>`
  restores `CLOSURE_OWNED_PROPERTIES` from the baseline into itself, then
  stamps it with `inline_hash_options()` and `Change::Known`. It discards the
  result and writes nothing.
- `prepare_inline_on` calls it on `source.original_text` (a plain `String`)
  with `Utc::now()`, right after `reject_removed_sequence_reads_in_source`.
  That is before `inline_prompt_text`, `compose_selecting` (frontmatter
  `$(…)`), and `load_or_create_guardrails`.
- New `CompositionError::InlineBaselineUnstampable { path, source: Box<MarkdownError> }`:
  - Display names the path and includes the cause.
  - `status_block` renders it in the schema family, not in the typed-Darkmatter
    arm, because a `FrontmatterTextEdit` cause has no path or excerpt and the
    block must name the document. A comment at that arm records why.
  - `code()` is `composition.failed` (Author/Correctable, detail
    `source_path`). No dedicated code was added, because the cause already
    names the fix. So no catalog or corpus entry was needed.
  - `detail()` projects `source_path`; `role()` is the default `Semantic`.
  - `frontmatter_block_spec` highlights `hash` only for a
    `MalformedStoredHash` cause.
  - The existing batch-detail test list now includes the new variant.
- Every inline entry reaches `prepare_inline_on`: `prepare_inline`/`_selecting`,
  `prepare_inline_with_schema` and its drop-optionals retry,
  `prepare_loop_seed`, `prepare_sequence_probe`, the service
  `prepare_document`, `--dry-run`, the staged reread, retry/resume/proxy
  through harness_orch, and sequence JIT. The only production
  `InlineClosurePlan` is built in `prepare.rs`. Direct compose/sequence use
  `prepare_direct_on` and are unaffected.
- **Behavior consequences, recorded here:**
  - A document whose stored `hash` is malformed is now refused at launch with
    `InlineBaselineUnstampable(MalformedStoredHash)` instead of after the agent
    runs with `InlineHashMalformed`. This follows from rehearsing the real
    stamp, which is what the spec requires.
  - An unstampable baseline is reported before a missing-`prompt` error.
  - No existing test depended on either order.
- Tests:

  | Requirement | Test |
  | --- | --- |
  | Helper refuses an indented baseline with a typed cause; input unchanged; fixed timestamp | `composition::closure::tests::rehearsal_refuses_an_indented_baseline_with_the_editors_reason` |
  | Helper accepts column 0, with managed properties present and absent | `rehearsal_accepts_a_column_zero_baseline_with_every_managed_property`, `rehearsal_accepts_a_column_zero_baseline_with_only_a_prompt` |
  | Valid baseline, then an agent-indented candidate: closure refuses (`InlineArtifactEditFailed(FrontmatterTextEdit)`), nothing is written, rollback restores exact bytes | `a_rehearsed_baseline_still_refuses_and_rolls_back_an_agent_indented_candidate` |
  | Preparation refuses before composing (exact path, matched cause, bytes unchanged, `$(mkdir ran)` sentinel did not run, no `.claudine/inline-compose.md`) through `prepare_inline`, `prepare_loop_seed(Inline)`, and `prepare_sequence_probe(Inline)` | `composition::prepare::tests::inline_preparation_refuses_an_indented_baseline_before_composing` |
  | Control: column 0 composes, the sentinel runs, the guardrail file is created | `inline_preparation_of_a_column_zero_baseline_composes_and_writes_guardrails` |
  | Rendering names the path and keeps the editor reason; code/detail/role; typed source chain (`Box<MarkdownError>`) | `composition::error::tests::inline_baseline_unstampable_renders_the_path_and_the_editor_reason`, `…_owns_composition_failed_with_its_path`, `…_retains_the_typed_markdown_cause` |

  A mutation check (the preflight call stubbed out) made the refusal test
  fail. The sentinel is `$(mkdir ran)` because it works the same under `sh`
  and `cmd`, and the shell policy refuses redirection. Commands are pre-approved
  so the result never depends on the host's shell whitelist.
- The CLI-level acceptance (`inline-compose` with zero provider launches,
  `md clean --save` then a successful run) is Phase 4's per the plan.

### Robustness smell check

No `#[serde(default)]`, `unwrap_or_default()`, `filter_map(..as_str())`, or
`.ok()` was added on a parse path. The editor and analyzer loaders are
unchanged in this phase; their document-shape matrix was covered in Phase 2.

### Gates

- claudine `just test`: 10457 passed (31 slow), 9 skipped, 0 failed (296.7 s).
- claudine `just lint`: passed.
- `just check-tier-coverage claudine` (repo root): 0 stranded.
- Darkmatter: only test files changed in this phase. The scoped runs above
  plus `just lint` cover them, and the full darkmatter suite runs in Phase 4's
  Wave 5. No Darkmatter product code changed, so no dependents are affected.
- Claudine dependents: the change is internal to inline preparation
  (`pub(crate)` helper and a new error variant). Its in-repo consumer crates
  (`claudine-cli`, `claudine-contract`, `claudine-gen`) are part of the claudine
  area suite above. No reverse dependency outside the area does inline
  preparation, so none was run separately.
- No failures, load-sensitive reruns, or skipped tests introduced.
- Skill: `.claude/skills/claudine/timeline.md` gained a 2026-10-20 entry for
  the preparation invariant. User docs are Phase 4's.
- Cross-OS: `just cross-check claudine --os windows inline_preparation rehears a_rehearsed inline_baseline_unstampable`
  ran the 9 new Claudine tests on native Windows, and all passed. The
  `$(mkdir ran)` sentinel works under `cmd`. A first attempt that passed a
  `-E '…'` filter failed in the remote shell's quoting (parentheses), before
  any test ran. The cross-check published no observation because the remote
  tree is not a committed head. The Darkmatter tests are string or
  `CliProcessFixture` based and were not run cross-OS.
- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: ['biscuit-file', 'claudine', 'darkmatter', 'darkmatter-cli', 'dmls', 'test-toolkit']`,
  `change_class: package`. The working tree still holds the uncommitted Phase 2
  changes. The packages list is non-empty, so the gates were run.

## Phase 4

The acceptance tests were written here. A Documenter subagent wrote the docs
at the same time and ran no Cargo commands. The gates ran afterwards, one at a
time.

### Prove Inline Acceptance (Claudine CLI)

New file `claudine/cli/tests/l1/inline_compose_indented_frontmatter.rs` (L1,
registered in `tests/l1/main.rs` behind `#[cfg(feature = "test-fixtures")]`;
`test-fixtures` is the package's declared `local-features` and is in CI's
`features`). It drives `claudine inline-compose --goose` through
`CliProcessFixture`, using the compiled `claudine-fake-goose` on every OS in
its marker mode, so no shell script or test-time `rustc` compile is needed:
one line per launch in `PROVIDER_MARKER`, and an edit of `FAKE_INLINE_DOCUMENT`.
The fixture initializes a repository, so the guardrail file has a place to go,
and writes `.darkmatter-shell-whitelist` with `exact argv-recorder ran`. The
recording prompt command is the frontmatter value
`probe: "$(argv-recorder ran)"`, run by the compiled `claudine-argv-recorder`.
The test reads its record through `ARGV_RECORDER_FILE`. A first version used
`$(mkdir ran)` like Phase 3's in-process test, and it failed on native Windows
with `ShellExpansionError: command not found` (`mkdir` is a `cmd` builtin with
no executable on PATH, and the CLI looks the command up before running it).
Phase 3's in-process tests did not hit this.

Stub extension: `tests/bin/fake_goose/main.rs` marker mode now accepts
`FAKE_INLINE_REPLACEMENT` (a file whose bytes overwrite the target). Existing
callers that set only `FAKE_INLINE_DOCUMENT` behave as before.

| Requirement | Test | Observation |
| --- | --- | --- |
| Indented baseline refused: nonzero exit, path and cause shown, zero launches, byte-identical, no guardrail, command not run, no audio | `an_indented_baseline_is_refused_before_anything_runs` | stderr (squeezed of whitespace, gutters, and backticks) has "inline baseline not editable", `notes.md`, "indented by 2 spaces", and `md clean --save <file>` |
| Preview path | `an_indented_baseline_is_refused_by_a_dry_run_too` | `--dry-run` refuses; zero launches; bytes unchanged; command not run |
| Recovery and control: after `md clean --save` the document runs | `the_cleaned_baseline_runs_and_is_stamped_at_column_zero` | refusal first; then cleaning gives column-0 keys with equal data; the run launches once, runs the command, writes guardrails, keeps the agent's body edit, and has a column-0 `hash:` that passes the `md hash --diff` comparison, with the data unchanged |
| Valid baseline, then an agent-indented candidate: closure fails and restores the bytes | `an_agent_indented_candidate_fails_closure_and_restores_the_baseline` | one launch; nonzero exit; the file equals the original baseline exactly; stderr has the indentation reason |

Departure noted: the `md clean --save` step runs the Darkmatter library
transaction behind it (`darkmatter::markdown::cleanup::clean_document`, then
writing the text back) in-process instead of spawning `md`. This is the route
`inline_compose_hash.rs` already uses ("unlike a sibling CLI process it remains
available in a nextest archive"). The `CliProcessFixture` PATH has no `md`, and
no Claudine L1 test spawns one. Spawning `md` itself is covered by
darkmatter-cli's `test_indented_root_is_refused_by_hash_save_until_clean_save_dedents_it`
(Phase 3).

The first run failed one assertion. The test expected the document path in the
`md clean --save` hint, but the editor reason (Phase 2, as specified) prints the
literal `md clean --save <file>`. The assertion was corrected; the product text
was not changed. Targeted run: `just test-cli inline_compose_indented_frontmatter`,
4 passed.

The YAML input-robustness matrix was covered in Phases 2 and 3 at the analyzer
and editor public APIs. These CLI tests only prove wiring (control plus one
indented shape per path) and do not repeat the matrix.

### Document Repair Workflow

- `darkmatter/docs/cli/clean.md`: new section "Indented Root Mappings
  (`yaml.root-indentation`)": eligibility, when no repair is offered, the
  parse-equality limit (blank lines in block scalars), tab recovery then
  dedent, DMLS parity, and a before/after example.
- `darkmatter/docs/cli/hash.md`: new section "Indented Frontmatter" covering
  the `--save` refusal (exit 1, file unchanged, even when nothing would be
  written), that read-only hashing and `--diff` still work, and recovery.
- `biscuit-file/lib/README.md`: a `yaml.root-indentation` entry beside the tab
  repair. This README is the analyzer's only reference, so no topic page was
  created.
- `claudine/docs/topics/composition.md`: new "Launch-time baseline check"
  section (what is refused and what does not happen, recovery, `--dry-run`,
  malformed stored hash, entry points, candidate validation and rollback still
  apply, Mermaid flow). Inline step 4 links to it. The "Malformed existing
  hash" bullet was corrected for drift: such a hash is now refused at launch.
- READMEs: one overview line each in `darkmatter/cli/README.md` and
  `claudine/README.md`, linking into `docs/`.
- Skills: `.claude/skills/claudine/cli-commands.md` (new "Inline baseline
  rehearsal" invariant row), `.claude/skills/darkmatter/cleanup.md`, and
  `.claude/skills/darkmatter/frontmatter.md`. The latter also fixes a drifted
  sentence: a hash save with nothing to write no longer returns "without
  parsing", because it now runs the indentation check. `timeline.md`'s
  2026-10-20 entry was reviewed and is accurate. `.claude/skills/os/windows.md`:
  the builtin bullet now also covers a CLI frontmatter `$(mkdir …)` sentinel,
  which fails on Windows.
- No `docs/` page names the fix directory.
- Found but out of scope (pre-existing drift, left alone):
  `darkmatter/docs/topics/markdown-formatting.md` § frontmatter still says
  tab-indented frontmatter is never repaired, which contradicts `clean.md`
  and the analyzer's existing `yaml.tab-indentation` recovery.

### Gates

- Scope calculator (`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`):
  `packages: ['claudine-cli', 'test-toolkit', 'darkmatter']`, `change_class: package`,
  so the gates were run.
- claudine `just test`: 10461 passed (10 slow), 9 skipped, 0 failed (210.6 s).
- claudine `just lint`: passed, both before and after the Windows fix to the
  test (the only warning is the existing linker note about the binary's
  `__eh_frame` size).
- `just check-tier-coverage claudine`: 0 stranded.
- darkmatter `just test` (includes darkmatter-cli and DMLS): 10498 passed,
  10 skipped, 0 failed. This is the full Darkmatter run Phase 3 deferred here.
  darkmatter `just lint`: passed.
- biscuit-file `just test`: 1161 passed. biscuit-file `just lint`: passed.
- After the full claudine run, the only edit was the Windows fix inside
  `inline_compose_indented_frontmatter.rs`, which changed how the frontmatter
  command is recorded. That file was rerun (4 passed on macOS) and linted;
  the full suite was not rerun.
- Cross-OS: `just cross-check claudine-cli --os windows inline_compose_indented_frontmatter`
  passed all 4 on native Windows after the argv-recorder fix. The first
  attempt named package `claudine` (the library), which selected 0 tests.
  The second failed as described above.
- Dependents: Phase 4 changed only Claudine CLI tests and fixtures, plus docs.
  Nothing outside the area depends on them, so no reverse dependency was run.
- No load-sensitive reruns, skipped tests, or pre-existing failures.

### Requirement-to-test summary (all phases)

| Success criterion | Proof |
| --- | --- |
| `md clean --save` dedents while keeping the parsed value | biscuit-file `tests/root_indentation.rs` (Phase 2); darkmatter `clean_frontmatter_repair.rs`, darkmatter-cli `test_indented_root_is_refused_by_hash_save_until_clean_save_dedents_it`, DMLS `formatting_transaction.rs` (Phase 3) |
| `md hash --save` refuses an uncleaned document without changing it | darkmatter `inline_document_text.rs` (Phase 2); darkmatter-cli test above (Phase 3) |
| Inline preparation refuses before commands or providers run | claudine `prepare::tests::inline_preparation_refuses_an_indented_baseline_before_composing` (Phase 3); CLI `an_indented_baseline_is_refused_before_anything_runs` and `…_by_a_dry_run_too` (Phase 4) |
| The cleaned document completes with a valid column-0 hash | CLI `the_cleaned_baseline_runs_and_is_stamped_at_column_zero` (Phase 4) |
| Agent-indented candidate: closure refuses and restores the exact bytes | `closure::tests::a_rehearsed_baseline_still_refuses_and_rolls_back_an_agent_indented_candidate` (Phase 3); CLI `an_agent_indented_candidate_fails_closure_and_restores_the_baseline` (Phase 4) |

Implementation complete, ready for review. Nothing was committed or moved.
