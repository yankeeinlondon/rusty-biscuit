---
kind: fix
name: archive-guard-full-tree
date: 2026-10-06
status: draft-spec
related:
  - 2026-09-19-less-brittle
  - 2026-10-05-worker-git-processes
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: "2026-10-06"
review_iterations: 0
clarified: false
implemented: false
human_review: true
$schema:
  status: |-
    enum(
        draft-spec,
        finalized-spec,
        planned,
        implemented,
        review-findings,
        human-in-the-loop,
        completed,
        on-hold,
        abandoned
    ) -> an indicator of progress for this specification
  reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
---

# The archive-path guard always scans the full tree

## Summary

The archive-path guard stops reading its scan scope from the resolved plan and
always scans the full eligible corpus: the Rust files admitted by the existing
scanner's exclusions. The planner keeps deciding **whether**
the guard runs (its scheduling rules are unchanged) but no longer tells it
**what** to scan, so the plan's `archive_guard` section, its Python emitter and
validator, its Rust reader, and the two cross-language contract tests that run
the whole planner to exercise that handoff are removed. The resolved plan's
schema version moves from 7 to 8.

This reverses one part of `2026-09-19-less-brittle`: the table that scoped a
pull request's guard scan to the changed eligible files. That spec's real goal,
reliable **selection** of the guard, is kept as it is.

## Background

### What the guard is

The [archive guard library](../../tools/test-toolkit/src/archive_guard.rs) in
the `test-toolkit` package lexes Rust source and refuses
compile-time paths that do not survive an archived run:
`env!("CARGO_MANIFEST_DIR")`, `env!("CARGO_BIN_EXE_…")`, and literal
`/home/runner/work/…` paths, except through the runtime-first macros
(`biscuit_test_harness::manifest_dir!`, `bin_exe!`) or an `ALLOWED` entry with a
reason. It also recognizes the existing, narrowly defined inline runtime-first
fallbacks; this fix does not change which expressions are accepted. Its
[driver](../../tools/test-toolkit/tests/archive_path_guard.rs) is run by
`just archive-path-guard` in `tools/test-toolkit`. The driver has two tests:

| Test | Scope today |
| --- | --- |
| `no_archive_executed_target_bakes_in_a_producer_path` | the plan's scope (full tree, or a changed-file list) |
| `every_allowlist_entry_still_names_a_live_site` | **always the full tree** |

### Where it runs

- **CI (authoritative):** as the `archive-path-guard` companion suite of
  `test-toolkit`'s `lint` cell on `ubuntu-latest` (`_package-ci.yml`, step
  guarded by `contains(fromJSON(steps.cell.outputs.companion_suites),
  'archive-path-guard')`), with `BISCUIT_ARCHIVE_GUARD_PLAN` pointing at the
  downloaded resolved plan.
- **Locally (advisory):** `just/ci-local.just` runs it in the pre-push hook when
  the plan attaches it, and says a non-Ubuntu pass leaves the planned
  `ubuntu-latest` execution outstanding.

### What the plan carries today

`affected_scope.py::archive_guard_scope` returns exactly one of three shapes,
stored as the plan's required top-level `archive_guard` field (schema version 5
onward):

```json
{"selected": false, "reason": "…"}
{"selected": true, "mode": "full", "reason": "…"}
{"selected": true, "mode": "changed", "paths": ["…"], "reason": "…"}
```

The same function also decides **selection**: a changed path that is eligible
Rust source or one of `ARCHIVE_GUARD_OWN_INPUTS` triggers the guard; the guard
is not selected when the event schedules no `ubuntu-latest` cell; a full-scope
run selects it. When the owner (`test-toolkit`) is not otherwise impacted,
`archive_guard_only_selection` adds a `test-toolkit` lint cell that runs the
guard alone. Actual scheduled execution is already visible to every reader as
a lint cell's `companions` entry named `archive-path-guard`, independently of
the `archive_guard` section. There is one important distinction: an ordinary
`test-toolkit` lint cell always includes that companion through the package's
manifest, even when the separate guard-trigger decision is false. This fix
preserves that attachment. After the change, such an invocation checks the
whole eligible tree instead of doing an empty violation scan plus full-tree
exemption maintenance.

## Problem

### The scope handoff costs more than it saves

Measured on CI (PR #116, run 37537695506, `test-toolkit` lint on
`ubuntu-latest`):

| Test | Scope | Time |
| --- | --- | ---: |
| `every_allowlist_entry_still_names_a_live_site` | full tree, 4,038 files | 3.9 s |
| `no_archive_executed_target_bakes_in_a_producer_path` | `changed(1 listed)` | 0.03 s |

The cell already pays for a full-tree pass on every guard run, so narrowing
the other test removes only one of two source passes. The 3.9 s measurement
gives an approximate scale for the added work, not an upper bound on the
violation scan or the whole suite. Recent full-tree runs report
`files-checked=4038 … violations=5` (all five allowed).

### The handoff's contract tests time out

Two tests in `tools/test-toolkit/tests/ci_workflow_contracts.rs` exist only to
prove that the Python emitter and the Rust reader agree:

- `the_shipped_planner_emits_plans_the_guards_reader_accepts` runs the shipped
  planner three times against the real workspace, once per shape
  (`-- README.md` → not selected, `--all` → full, `-- tools/test-toolkit/src/archive_guard.rs`
  → changed), and parses each result with `GuardPlan::from_plan_json`.
- `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
  checks that deleted paths are omitted from `changed` and that the reader
  refuses a listed path that does not exist.

Their cost, for the same two tests:

| Host and mode | Time |
| --- | ---: |
| macOS development host, native | ~4 s |
| `BUILD_LINUX`, native | 16 s |
| `BUILD_WSL`, native | 1.9 s + 5.9 s |
| `BUILD_WSL`, nextest archive mode (as CI runs it) | 51 s + 52 s |
| `ubuntu-latest` CI | 64–82 s each |
| `windows-latest` CI | **timed out at 90 s** (runs 37529082394, 37537695506) |

In archive mode the `-- README.md` plan alone took 30.1 s against about 3 s
from a checkout; `--all` and a `.rs` change took about 1.2 s and
`cargo metadata` about 1.1 s. The `README.md` plan is the expensive one
because a non-source change runs the planner's test-input scan
(`scripts/ci/test_inputs.py`) over every test-reachable Rust file, which the
guard's answer does not depend on. Why archive mode makes that scan about ten
times slower is unexplained (see Not in scope). These timeouts keep
`test-toolkit` L1 red on `windows-latest`, on `main` and on any `ci:all-os`
pull request.

### Why the original reason no longer holds

`2026-09-19-less-brittle` scoped a pull request's scan to its changed files.
The currently passing full-tree scans and maintained exemption list make
finding an inherited violation unlikely. They do not guarantee its absence:
event scheduling, canceled runs, and policy changes can leave a tree unchecked.
The intended behavior is now that a selected scan detects every unexempted
violation in the eligible tree, including one outside the changed files. The
small expected increase in scan time is preferable to maintaining a second
scan policy and a plan reader solely for it.

## Decision

1. Every guard invocation scans the full eligible corpus. The scanner reads
   no plan and no scope variable. It continues to use the existing runtime
   checkout lookup, so an archived binary checks the consumer's checkout.
2. Scheduling is unchanged: the same changed paths trigger a guard-only
   `test-toolkit` lint cell, the same ordinary owner lint cells carry the
   companion, and no guard cell forces `ubuntu-latest` into an event that does
   not schedule it. A scheduled guard can now fail for a violation in an
   untouched file; that wider detection is intentional.
3. The resolved plan no longer carries the guard's scope. Its lint cells'
   `archive-path-guard` companions describe scheduled execution, and a
   guard-only cell keeps its human-readable selection reason.
4. The resolved plan's schema version moves from 7 to 8 under the
   [schema version rules](../../docs/cicd/schema-versions.md). Validation
   receipt and scope receipt versions do not change.
5. Guard lint cells remain non-reusable. Removing the mode distinction does
   not make package-local receipts identify all repository-wide scan inputs.

> **Review note — preserve execution ownership.** Removing the scanner's
> plan reader does not remove the workflow's need for the plan. The workflow
> still resolves its cell, decides which companions to execute, and records
> their results from that document. Likewise, retaining the ordinary lint
> cell's companion avoids an unrelated scheduling change. The simplification
> removes the file-list handoff; it does not create a second scheduler or
> weaken the merge gate.

## Requirements

### Guard library and driver (`test-toolkit`)

- Change [scan](../../tools/test-toolkit/src/archive_guard.rs), the
  `test-toolkit` function that discovers and checks source, to
  `scan(root: &Path) -> Result<ScanReport, GuardError>`. It always uses the
  existing full-tree discovery. Keep the eligibility policy, directory and
  own-source exclusions, exemption list, matcher, traversal behavior, and
  filesystem error handling unchanged.
- Remove [GuardPlan and ScanMode](../../tools/test-toolkit/src/archive_guard.rs),
  the `test-toolkit` types that parse the plan and represent selectable scan
  modes. Remove their helpers, the plan constants (`PLAN_ENV`, `PLAN_KEY`,
  `PLAN_SCHEMA_VERSION`, `PLAN_SCOPE_FIELDS`), and changed-list-only error
  variants and validation helpers once no caller needs them. Do not keep a
  one-variant mode type or a compatibility reader.
- Simplify [ScanReport](../../tools/test-toolkit/src/archive_guard.rs), the
  `test-toolkit` scan result: retain the checked-file count and violations;
  remove stored mode and changed-list-only `ineligible` state. Its summary
  still identifies `mode=full-tree` and the actual checked-file count.
- Keep both tests in the [real-checkout driver](../../tools/test-toolkit/tests/archive_path_guard.rs).
  The violation test prints the summary and explicitly fails if it checked
  zero files. Printing a zero count alone does not prevent a false pass.
  This assertion belongs in the repository driver; a generic scan of an empty
  fixture tree may still return a valid empty report.
- Keep exemption maintenance's raw full-tree lookup. It must still recognize
  accepted runtime-first fallbacks as live sites, so exemptions are not
  incorrectly removed just because those sites produce no violation. Keep
  the two passes and two driver tests; sharing or caching their work is not
  needed for this fix.
- Update callers and relevant module, function, and driver comments, including
  `raw_live_form_files` documentation and the scanner fixture tests. Remove
  stale claims about a plan, changed-file scans, or selectable modes. Update
  any affected public exports; `src/lib.rs` currently exposes the module
  rather than re-exporting the removed types.
- Move `serde_json` back to `[dev-dependencies]` in
  [the package manifest](../../tools/test-toolkit/Cargo.toml): the plan reader
  is its only current library user, while contract tests still need JSON.
  Update the dependency comment and dependency documentation together.

### Planner selection (`repo-deps`)

The Python planner in the `repo-deps` package owns scheduling.

- Replace [archive_guard_scope](../../scripts/ci/affected_scope.py), the helper
  that currently combines selection and scan scope, with an internal
  selection decision and a readable reason. It is not serialized as a second
  plan section. Remove branches and arguments used only to choose scan mode
  or produce a scan list. Keep deletion input if needed to preserve triggers.
- Preserve the existing eligible-source trigger, owned-input list, skipped
  directories, build-script exclusion, full-scope selection, and check for
  `ubuntu-latest` among the environments actually scheduled. In particular,
  an eligible deletion still triggers maintenance even though there is no
  remaining source file to scan; pushes that have already proved Linux do
  not gain another Linux execution.
- Preserve [archive_guard_only_selection](../../scripts/ci/affected_scope.py),
  the `repo-deps` helper that schedules only the guard when its owner has no
  other required work. It still adds exactly one
  `{test-toolkit, ubuntu-latest, lint}` cell with `companions_only: true`,
  no Clippy work, no unrelated companions, and no dependent compile work.
  Its reason identifies a changed trigger or a full-scope request without
  claiming a changed-file scan was performed.
- When the owner already has an ordinary lint cell, retain its guard
  companion, with no duplicate cell or companion and no `companions_only`
  flag. This also preserves existing attachment when the separate
  guard-specific trigger is false. A documentation-only input does not add
  a guard-only cell, but a separate ownership rule can still select an
  ordinary lint cell carrying the guard.
- Keep `SUITE_REGISTRY["archive-path-guard"]`, its lint-only recipe and Just
  tuple, and the manifest's companion binding unconditionally. They are how
  CI still executes the guard. Remove only the exact
  `.github/ci/schemas/archive_guard_cases.json` ownership mapping in
  `SUITE_OWNER_PATHS` when its retired corpus is deleted. Keep
  `contract.json` ownership for its surviving cross-language readers.
- Refresh selection and registry comments that explain the former scope
  contract; keep the eligibility and owned-input agreement tests.

### Resolved plan schema (version 7 → 8)

Follow the [schema version rules](../../docs/cicd/schema-versions.md) in one
implementation change:

- In [the Python schema owner](../../scripts/ci/schema.py), remove the
  top-level `archive_guard` field, its validation call, `_archive_guard`,
  `ARCHIVE_GUARD_FIELDS`, `ARCHIVE_GUARD_MODES`, and its exported contract
  metadata. Set `RESOLVED_PLAN_SCHEMA_VERSION` to 8 and regenerate
  `.github/ci/schemas/contract.json` with `python3 scripts/ci/schema.py`.
- Delete `.github/ci/schemas/archive_guard_cases.json` and tests whose only
  purpose is validating that retired scope shape. Keep tests of the rest of
  the resolved plan contract.
- Update every version mirror listed in the changelog, including the
  `repo-deps` [rollup reader](../../scripts/ci-rollup.rs), the pre-push hook's
  `plan-*.json` fixtures and `affected_scope_stub.py`, and
  `scripts/ci/plan_fixtures.py`. Remove the guard's mirror altogether.
  Search other plan fixtures and assertions for hard-coded version 7.
- **Keep `change_inventory.deleted` and the `--deleted` argument.** They are
  part of the existing declared change inventory, validated independently
  of scan scope. Keep the shared `diff_scope.py` parser and the deletion and
  rename declarations in CI, the local runner, and the pre-push hook. Removing
  them is a separate contract change with no benefit to this simplification.
- Update the changelog's current-version table and mirror list, and add a
  version 8 history row naming `2026-10-06-archive-guard-full-tree`. Record
  removal of the guard scope and its Rust version mirror, and retention of
  deletion inventory. This history entry is the exception to the rule that
  current behavior docs do not name a fix.
- Readers that enforce the plan version still reject older generations
  before validating the remaining fields. A version-7 scope receipt misses
  once as `scope-schema`; CI calculates a fresh plan. Do not upgrade stored
  receipts in place or change validation receipt versions. Existing passing
  cell evidence remains eligible under its usual input-equivalence rules;
  the plan bump alone is not grounds to reject it.

### Workflows, local runner, and plan display

- In [the package workflow](../../.github/workflows/_package-ci.yml), remove
  `BISCUIT_ARCHIVE_GUARD_PLAN` and its scanner-specific comment from the lint
  companion step. **Keep the required resolved-plan download and cell
  resolution.** A missing or unreadable execution plan still fails the job;
  the guard itself simply no longer reads that document.
- Keep companion selection, the companions-only Clippy condition, producer
  status, completion records, area coverage auditing, and the existing
  `ci-gate` failure propagation. Do not add jobs, matrix cells, or receipt
  formats for the full-tree scan.
- In [the local runner](../../just/ci-local.just), remove the scope-variable
  export. Keep executing the canonical recipe only when a planned lint cell
  carries the guard companion and the local run requests lint. Keep the
  `companions_only` behavior and the advisory message that the planned
  `ubuntu-latest` execution remains `OUTSTANDING` on another host. A local
  pass does not replace a non-reusable CI lint cell, including on Linux.
- Replace the old scan-scope summary on both display surfaces:
  [ci-plan](../../scripts/ci-plan.rs), the `repo-deps` terminal renderer, and
  the local runner's `jq` fallback. They derive guard presence from lint-cell
  companions and describe a full-tree scan when attached, or say that no
  guard execution is planned when none is attached. Cell selection reasons
  and execution states remain visible in the existing cell display. A
  prohibited cell must not be presented as a completed scan. Do not invent
  an unselected reason that is no longer carried in the plan.
- Remove `ci-plan`'s `ArchiveGuard` scope model and use its existing
  `TerminalRenderable` components for the replacement summary. Add defaulted
  companion-name deserialization to its cell model, which currently omits
  `companions`, so older accepted cell documents still render. Rendering
  must not run the scanner, inspect Git, or recalculate selection. Older
  plans that this advisory renderer already accepts retain their ordinary
  cell display; their retired scope field is ignored.
- Update [the guard recipe](../../tools/test-toolkit/justfile) comments and
  keep `--no-capture`, so passing runs still expose their file counts.

### Tests

- In the `test-toolkit` [workflow contract suite](../../tools/test-toolkit/tests/ci_workflow_contracts.rs),
  delete the two expensive tests named in the Problem section, plus the
  passive field-set, shared scope-corpus, and guard-specific schema-version
  tests whose reader no longer exists. Do not replace them with other
  invocations of the shipped planner over the real workspace.
- Adapt shared contracts rather than deleting them just because they mention
  the removed variable. Keep coverage of guard-only execution, required plan
  download and cell resolution, canonical recipe agreement, absence of
  consumer-side diffs, eligibility and owned-input agreement, deletion
  declaration, and failure propagation through the merge gate.
- Convert Python guard-scope tests in `scripts/ci/test_affected_scope.py` to
  internal selection tests and assertions about actual lint-cell companions.
  Use fixture metadata containing the guard owner for execution assertions:
  a fixture without `test-toolkit` cannot prove that a cell is attached.
  Cover eligible changes inside and outside workspace members, owned inputs,
  excluded paths, deletions, full-scope requests, no scheduled Linux, the
  ordinary owner lint cell, and the guard-only cell. Include an ordinary
  owner selection with no separate guard trigger to pin existing attachment.
- Update `test_schema.py`, `test_resolved_plan.py`, `test_ci_local.py`,
  `test_evidence_reuse.py`, `test_local_evidence.py`, and shared plan fixtures
  for the retired field and version. Retain version-first refusal, the
  `scope-schema` miss, and unchanged validation receipt behavior. Guard lint
  cells remain non-reusable.
- Adapt existing temporary-tree scanner tests to the mode-free API and retain
  full-tree traversal, matcher, filesystem failure, and exemption checks.
  Demonstrate that violations in multiple eligible files are found without
  a path list and that the real-checkout driver rejects a zero-file pass.
  Remove tests solely about parsing plans or validating changed-path lists.
- Update `scripts/ci-plan-tests.rs` to cover both companion-present and
  companion-absent summaries without a scope block, and keep
  `scripts/ci-rollup-tests.rs` version and companion-result coverage.
- No timeout, retry policy, eligibility rule, or exemption is relaxed to make
  validation pass.

### Documentation

Update current behavior in `.github/ci/README.md`,
`.github/ci/schemas/README.md`, `.claude/skills/rust-devops/ci-cd.md`, and
`docs/dependencies.md` alongside implementation. Update any affected area
README or topic page that describes the public scan API. Explain full-tree
scanning, ordinary versus guard-only attachment, the still-required execution
plan, non-reusable guard evidence, and the retained exclusions. Describe the
guard's environment rule in terms of the environments actually scheduled,
rather than depending on a hard-coded claim about the nightly event. The
nightly policy discrepancy is recorded in Resolved Questions below; this fix
does not change the environment table. Current docs state behavior directly
and do not name this fix. Historical specs and schema history rows remain
snapshots.

## Acceptance

1. A targeted `rg` audit finds no active code, test, fixture, or current
   behavior documentation still using `BISCUIT_ARCHIVE_GUARD_PLAN`,
   `GuardPlan`, `ScanMode` in the archive guard, `archive_guard_cases`, or
   the plan's removed `archive_guard` field. Historical specs, this spec,
   and schema history rows are excluded; the `archive_guard` module name
   remains valid. All current plan writers and required-version readers
   agree on version 8, and the generated contract matches the schema owner.
2. Selection and companion tests prove unchanged scheduling, including
   guard-only and ordinary owner cells. The scanner detects violations across
   the full eligible fixture tree, preserves exemption maintenance, and the
   real-checkout driver fails on a zero-file scan. Both human plan displays
   agree with the lint-cell companions without reading a scope block.
3. Implementation validation passes: `python3 -m unittest discover` in
   `scripts/ci`; `just archive-path-guard`, `just test`, and `just lint` in
   `tools/test-toolkit`; `just test repo-deps` from the root, with its default
   `local-tools` feature so the plan renderer is tested; and
   `just test-githooks` from the root. These are implementation checks, not
   prerequisites for this document-only review.
4. `just cross-check test-toolkit --os windows` passes without the removed
   planner-reader timeouts. Reuse qualifying evidence for required cells on
   each environment and run cells lacking it. Review `just ci-local --plan`
   before an implementation push. Normal event scheduling applies; no
   `ci:all-os` label or full-workspace CI run is required just to review or
   validate this simplification. Existing execution bans remain binding.
5. The selected Ubuntu lint companion reports `mode=full-tree` and a positive
   file count; a failing or unreported required companion cannot pass its
   cell. Record durations from the normal validation run for context. The
   existing measurements justify removing the handoff without a performance
   spike or a new timing threshold: 3.9 s measured one raw-form pass, not the
   total duration of two tests, compilation, or CI setup.

## Resolved Questions

### Which environments should the nightly event schedule?

**Decided 2026-10-06 by the author: keep the checked-in environment table as
it is for this fix, and resolve nightly policy separately.** This fix does not
change `.github/ci/environments.json`; a nightly run continues to schedule
WSL2 alone, so it does not execute the guard. The options as reviewed:

The repository instructions describe a nightly run covering Linux, Windows,
and WSL2. However, the checked-in
[environment table](../../.github/ci/environments.json) assigns `schedule`
only to WSL2, and [the workflow](../../.github/workflows/ci.yml) describes
that same rule. This matters because the guard is hosted on `ubuntu-latest`:
a WSL2-only nightly cannot run it. This is a pre-existing policy discrepancy,
not a consequence of removing scan scope.

- **Preserve the checked-in table for this fix; resolve nightly policy
  separately (chosen).** Pros: keeps scheduling unchanged and avoids
  adding CI work unrelated to the guard's plan reader. Cons: the disagreement
  with the repository instructions remains, and the current nightly does not
  execute the guard. Recommended because this fix can satisfy its design
  goal without deciding or expanding the nightly matrix. The author should
  confirm the intended policy and correct its owning documents separately.
- **Include a separately justified nightly-policy change.** Add `schedule`
  to Linux and Windows in the environment table, then update the event tests
  and current documentation together. Pros: brings the executable schedule
  into agreement with the stated repository policy and runs the guard on
  the nightly event. Cons: adds Linux and Windows work across the nightly
  full-workspace plan, increasing runner time and making this fix responsible
  for a broader scheduling change. It requires an explicit author decision
  and a scope-based cost estimate before adding those cells.

Either option leaves the guard rule unchanged: it runs only when the resolved
plan contains a lint cell carrying its companion on `ubuntu-latest`. This
policy question does not block implementing the scan simplification against
the current table. The archive-mode planner slowdown remains an independent
investigation below.

## Not in scope

These are open and recorded so they are not lost:

- **Nightly environment policy.** The repository instructions and the
  `rust-devops` CI page describe a nightly run covering Linux, Windows, and
  WSL2, while `.github/ci/environments.json` schedules `schedule` on WSL2
  alone. Deciding the intended policy and correcting whichever side is wrong
  is a separate change (see Resolved Questions).

- **Archive-mode planner slowdown.** The `-- README.md` plan takes about 30 s
  in nextest archive mode against about 3 s from a checkout on the same WSL
  guest; `cargo metadata` and the other plan kinds are normal, and file reads
  are plain `read_text`. CI's own scope job plans from a normal checkout, so it
  is not affected; tests that run the planner are. Not yet explained.
- **Planner in Rust.** `scripts/ci/test_inputs.py` is a Rust lexer written in
  Python and the expensive part of planning; moving it into Rust, sharing the
  guard's lexer, is the proposed first step of an incremental migration. It
  needs its own spec.
- **`repo-deps` Windows MAX_PATH.** Seven `ci-build` archive tests fail on
  `windows-latest` with `LNK1104` because the nested fixture build reaches a
  266-character path; four more time out. Partial uncommitted work is on
  branch `fix/ci-build-fixture-max-path` (worktree
  `.claude/worktrees/agent-aaa3ce04badad1120`).
- **Fake-Gitea record-and-replay.** `list_prs::a_changed_origin_drops_the_old_head_checks_fallback_notice_and_caption`
  misses the 3 s wait on `windows-latest`; about half its transport time is the
  test fixture running `git http-backend` per request. Partial uncommitted work
  is on branch `test/gitea-replay` (worktree
  `.claude/worktrees/agent-ad15a63a110599e32`), with a probe test to delete.
- **Proving `2026-10-05-worker-git-processes` on Windows CI.** Re-run `main`'s
  run 37517419405 once the items above are merged; its `worktree` cell passed
  521/521 last time and failed only on the UTF-8 crash fixed by PR #116.
