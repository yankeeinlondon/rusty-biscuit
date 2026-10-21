---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-iterm-graphics/claudine/fixes/2026-10-20-handle-indented-frontmatter/spec.md"
plan: "claudine/fixes/2026-10-20-handle-indented-frontmatter/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
human_review: false
message_to_agent: |-
  Phase 1 was an audit only; read the "## Phase 1" section of this log before starting. Key traps for Phase 2:
  - biscuit-file `Draft` carries one repair and `prove_drafts` re-tests drafts singly on combined failure, so per-line dedent drafts would all be dropped. Carry several prefix repairs on one diagnostic, or prove the dedent as a unit as `tab_indentation_recovery` does.
  - Skip whitespace-only `Blank` lines outside block scalars (the trailing-whitespace edit spans them from line start) and the trailing blank-line run; dedent `BlockContent` whitespace-only lines. `scan.rs` has no directive kind (`%YAML` is `Content`) and `Line.indent` counts tabs too.
  - Darkmatter `apply_hash_save_text` returns `Ok(None)` at write.rs:320-322 BEFORE `validate_block_mapping` (:340); the spec requires the indentation check before any no-op, so move validation ahead when a block exists. Comment-only frontmatter acceptance is untested; add a control.
  - Adding a `YamlDiagnosticCode` touches four places, including the position-sensitive pinned spelling list in src/yaml/analyze/tests/diagnostic.rs.
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
