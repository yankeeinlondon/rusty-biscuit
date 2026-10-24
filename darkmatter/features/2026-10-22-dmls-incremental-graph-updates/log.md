---
implementation_1: "2026-10-24T07:18:57-07:00"
implementation_2: "2026-10-24T07:25:04-07:00"
---

# Review-to-implement log: DMLS incremental graph updates

## Implementation of Review Findings #1

> **started at:** 2026-10-24T07:18:57-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- set the review's `implemented` flag to `false` before starting (it already read `false`)
- inventory of `review-1.md`:
        - frontmatter `findings: []`, `recurrence: false`, `has_blocked_findings: false`, `blocked: false`
        - the body's `## Findings` section reads "None recorded", and the review says it did not compare implementation to spec, reproduce anything, or run tests
        - the only open item is the `human_in_the_loop` decision on opening an unchanged, reference-only dependency (`human_resolved: false`, `## Human rulings` pending)
        - the body's "Other implementation decisions" list is document-audit conclusions with no repair required

### Finding Ledger

| ID    | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ----- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-H1 | Not a finding: the author must decide whether opening a dependency with byte-identical text should rebuild, so the file becomes a wiki candidate immediately | not a finding, awaiting the author's ruling (nonblocking for this cycle) | The review says this is "an author scope decision at this stage, not an independently verified defect finding". It asks for the ruling to be recorded under `## Human rulings` before technical review resumes. The two options produce different user-visible behavior, so this cycle does not choose between them and does not fill in a human ruling. The deferral is in `implementation-log.md` (the `decisions_for_review` entry on `open_buffer` returning `Unchanged`) |

- starting the work on 'review-1 inventory' at 07:19:30
        - no parent findings to repair; no defect class to sweep
        - did not implement the role-transition repair: the review leaves that choice to the author, and making it here would replace the author's ruling
- work completed for 'review-1 inventory' at 07:19:30

### Completion Report

- **outcome:** complete (the review has zero findings), elapsed about 1 minute
- **parent findings:** total 0 = fixed 0 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- **open items:** none in the finding ledger. R1-H1 waits on the author's ruling in `review-1.md` (`human_resolved: false`). After that ruling, the next cycle implements the chosen behavior or records that it was accepted as-is
- **changed files:** this log, plus the `log` and `implemented_by` frontmatter in `review-1.md`; no source, test, or docs changes
- **verification:** no code changed, so no package area was affected and no `just test` or `just lint` run was needed for this cycle

## Implementation of Review Findings #2

> **started at:** 2026-10-24T07:25:04-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-merman/darkmatter/features/2026-10-22-dmls-incremental-graph-updates/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- set the review's `implemented` flag to `false` before starting (it already read `false`)
- inventory of `review-2.md`:
        - frontmatter `findings`: one, `R2-F1 (high)`; `has_blocked_findings: false`, `blocked: false`, `recurrence: false`
        - body: R2-F1 with eleven child rows (R2-F1.1–R2-F1.11), a "smallest repair" paragraph (rebuild once on a role-changing equal-text open, fix the permanent test's expected disagreement, add LSP notification/request coverage, keep one universe and one table rebuild, review the hash-only publication comments)
        - R1-H1 (carried pending scope question) is not a blocker; R2-F1 cites Phase 2 Acceptance "Equivalence" (buffer opens and dependency transitions), so it has authority

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R2-F1 | Opening an unchanged dependency leaves the live graph in its old role | fixed | every required child below is fixed or clean; repair in `WorkspaceIndex::open_buffer` / `index_source` (`dmls/src/graph/invalidate.rs`); three new or rewritten tests fail with the repair disabled |
| R2-F1.1 | `open_buffer` → `set_document`: equal-text open of a parsed dependency publishes one new graph with the dependency flag cleared | fixed | unit test `graph::invalidate::tests::test_opening_a_dependency_with_equal_text_publishes_its_new_role`: `Reindexed`, generation +1, `is_dependency` false, snapshot record `dependency == false`, one universe and one table |
| R2-F1.2 | `wiki_universe` / wiki resolution: `[[dep]]` resolves immediately after the open, as in a fresh build | fixed | `incremental_graph_equivalence::dependencies_open_join_and_return_to_referenced_only_like_a_fresh_build`: `[[dep]]` goes from `Unresolved` to `Resolved(root of dep)` at the open, plus full live-vs-fresh `check`; LSP test confirms via `definition` |
| R2-F1.3 | Assembly of Markdown-link and wiki-link nodes and their edges for the opened document | fixed | same equivalence test: `Link` and `WikiLink` nodes present in the opened document, `[[source]]` resolves, edges compared by `check` |
| R2-F1.4 | Assembly of frontmatter-key, schema-use, file-use, and variable-use nodes and edges | fixed | same equivalence test: `FrontmatterKey`, `SchemaDeclaration`, `FileReference`, `Interpolation` nodes present; edges compared with a fresh build by `check` |
| R2-F1.5 | Document table basename bucket includes the opened document | fixed | same equivalence test: `basename_bucket == ("dep", [dep])`; basename counts compared by `check` |
| R2-F1.6 | Wiki file completion and Markdown path completion offer the opened document | fixed | same equivalence test: `ProviderEnv::answers` equal live vs fresh at sampled offsets plus `[[de` and `[p](de`; both completions include label `dep`; LSP test: `[[rep` offers `report` |
| R2-F1.7 | Frontmatter `file_path_completions` dependency filter no longer excludes it | fixed | code contract: the filter at `providers/frontmatter.rs` reads `DocumentRecord::dependency`, which the R2-F1.1 test asserts is false in the published snapshot after the open; the review checked this row by code trace as well |
| R2-F1.8 | Graph-symbol projection includes the opened document | fixed | same equivalence test: `graph_symbol_documents` equal live vs fresh and includes `dep.md` |
| R2-F1.9 | `portability_collisions` grouping follows the published role (sibling check) | clean | portability twins are compared by the equivalence `check` after the open; the existing logical-case-pair tests pass in the full dmls run |
| R2-F1.10 | Heading/transclusion assembly for dependencies and retained-snapshot immutability preserved (sibling check) | clean | `Heading` and `TransclusionTarget` still assembled while a dependency; retained-snapshot tests pass in the full dmls run (1,301 passed) |
| R2-F1.11 | Other ownership/update entry points still rebuild exactly when required; already-open and listed-parse equal-text opens stay no-ops | clean | unit-test controls: equal-text reopen of an open document and equal-text open of a listed parsed closed document return `Unchanged` with no generation bump; a stale dependency opened with equal text rebuilds; existing work-count tests pass; `open_buffer` is the only production `set_document` caller (`router.rs:742`) |
| R2-F1.12 | Permanent dependency-transition test asserts equivalence at the unchanged open instead of the disagreement | fixed | the "Known quirk" disagreement assertions removed; the unchanged open is now checked with `h.check(...)` and asserts one table and one universe rebuild |
| R2-F1.13 | Server-level coverage: opening an already-loaded ignored target through `didOpen` makes it wiki-eligible immediately | fixed | `workspace_file_list::opening_a_loaded_ignored_target_with_its_disk_text_makes_it_a_wiki_target_at_once`: ignored `generated/report.md` loaded as a dependency, `didOpen` with disk text, then one universe/table rebuild, `[[report]]` definition resolves, `[[rep` completion offers it, no `wiki.*` diagnostics |
| R2-F1.14 | Hash-only publication comments in the invalidation module match the new behavior | fixed | `invalidate.rs` module `//!`, `Invalidation::Unchanged`, and `open_buffer` docs; `darkmatter/docs/lsp/architecture.md`, `darkmatter/docs/lsp/indexing.md`; `.claude/skills/darkmatter/dmls.md` |

- defect class for R2-F1: a content-only equality check suppresses publication after a document's ownership role changes with byte-identical text
        - sibling sweep (orchestrator, before delegation): `open_buffer` is the only production caller of `set_document` (`src/router.rs:742`); `ensure_parsed`'s equal-hash `continue` only fires for a stale re-read, which does not change listed/open membership; `close_buffer`, `remove_document`, `disk_changed`, `install_listing` all rebuild on a role change
- starting the work on 'R2-F1 opening an unchanged dependency' at 07:26:00
        - delegated to one foreground `rust-developer` subagent (rust, rust-testing, darkmatter skills); it reported to the orchestrator, which logs here
        - repair: `open_buffer` records `is_dependency` before inserting into the open set and passes it to a new private `index_source(path, source, role_changed)`; equal hash returns `Unchanged` only when the role did not change. `set_document` passes `false` and keeps its behavior
        - subagent sibling sweep agreed with the orchestrator's: no other site publishes on a hash check alone across a role change
        - negative control: with the repair disabled, all three new tests failed (unit test at `invalidate.rs:1605`, equivalence test at `incremental_graph_equivalence.rs:465`, LSP test saw `(0, 0)` rebuilds instead of `(1, 1)`); restored afterward
        - process note: the subagent ran its negative-control check once with `--no-fail-fast`, against the brief; it affected only two targeted tests
        - orchestrator reviewed the combined diff: repair, docs (`architecture.md`, `indexing.md`, anchor `#targets-outside-the-file-list` exists), skill line, tests
        - scoped run: `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/incremental_graph_equivalence|graph::invalidate|workspace_file_list/)'` → 58 passed; adjacent `-E 'test(/referenced_targets|wiki_universe_equivalence|level1_wiki|router|graph::/)'` → 158 passed
- work completed for 'R2-F1 opening an unchanged dependency' at 07:31:30
- reconciled every ledger row against the combined diff and the full-area run at 07:35:04

### Completion Report

- **outcome:** complete, elapsed about 10 minutes (07:25:04 → 07:35)
- **parent findings:** total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - child rows: 11 fixed (R2-F1.1–F1.8, F1.12–F1.14), 3 clean (R2-F1.9–F1.11)
- **verified repair:** opening a parsed dependency with byte-identical text now rebuilds once (one wiki universe, one document table) and publishes it as an ordinary open document, equal to a fresh full build; equal-text opens whose role does not change (already open, listed parsed closed) keep the no-rebuild shortcut
- **decision recorded for the author:** this implements the "repair the transition" option of the carried R1-H1 scope question (the review recommended it under the current spec); the alternative, amending the equivalence requirement to accept the delay, was not taken because Phase 2 Acceptance already requires equivalence for buffer opens and dependency transitions. Added to the review's `human_review_items`
- **open findings:** none
- **changed files:**
        - `darkmatter/dmls/src/graph/invalidate.rs` (repair, docs, unit test) — R2-F1.1, F1.11, F1.14
        - `darkmatter/dmls/tests/l1/incremental_graph_equivalence.rs` (harness role change, rewritten transition test) — R2-F1.2–F1.10, F1.12
        - `darkmatter/dmls/tests/l1/workspace_file_list.rs` (LSP session test) — R2-F1.13
        - `darkmatter/docs/lsp/architecture.md`, `darkmatter/docs/lsp/indexing.md`, `.claude/skills/darkmatter/dmls.md` — R2-F1.14
- **verification:**
        - `cargo nextest run -p dmls --features effects-instrumentation -E 'test(/incremental_graph_equivalence|graph::invalidate|workspace_file_list/)'` → 58 passed, 0 failed (R2-F1 focused)
        - `just test --no-fail-fast` in `darkmatter/` → 10,618 passed, 43 failed, 10 skipped; **all 1,301 dmls tests passed**. The 43 failures are in `darkmatter` (42) and `darkmatter-cli` (1), neither of which depends on `dmls`, so this change cannot affect them: JSON object key-order expectations (`fn_list_formatting::as_json5_uses_the_formatter_object_idiom`, 40 `schemas_convert_snapshots::*`, `suggest_constraint_phase3::conversion_snapshot_covers_valid_and_invalid_metadata`, `schema_validate_baseline::schema_validate_legacy_json_output_is_byte_identical`) expect sorted keys while output keeps insertion order. Reproduced deterministically in isolation; they predate this cycle (likely the 2026-10-23 `biscuit-hash` canonical JSON / `preserve_order` commits `48e1c8ec5`, `bbde223e6`) and are outside this feature's scope
        - `just lint` in `darkmatter/` → pass (all crates plus zed-dmls wasm32-wasip2)
        - `just check-tier-coverage darkmatter` → 0 stranded
        - cross-OS check not run: the change is path-agnostic index bookkeeping with no OS-specific code; CI covers the other OSes
