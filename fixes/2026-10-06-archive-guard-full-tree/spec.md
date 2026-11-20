---
kind: fix
name: archive-guard-full-tree
date: 2026-10-06
status: draft-spec
related:
  - 2026-09-19-less-brittle
  - 2026-10-05-worker-git-processes
  - 2026-10-06-archive-key-helper
  - 2026-11-17-too-much-work
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
the guard runs (this fix does not change its selection rules) but no longer tells it
**what** to scan, so the plan's `archive_guard` section, its Python emitter and
validator, its Rust reader, and the two cross-language contract tests that run
the whole planner to exercise that handoff are removed. The resolved plan's
schema version moves from 7 to 8.

This reverses one part of `2026-09-19-less-brittle`: the table that scoped a
pull request's guard scan to the changed eligible files. That spec's real goal,
reliable **selection** of the guard, is kept as it is.

*Refreshed 2026-11-19* against the current tree: the nightly environment
policy, the landed key helper, the scanner's tokenization prefilter, the plan
renderer's new file, and two further plan-fixture consumers. See
[Coordination with `2026-11-17-too-much-work`](#coordination-with-2026-11-17-too-much-work)
for how this fix relates to that selection change.

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
| `no_archive_executed_target_bakes_in_a_producer_path` | the plan's scope: full tree for a push, a full-scope request, or a run with no change inventory; otherwise (a pull request) a changed-file list |
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

Since that measurement, the scanner skips tokenizing a file whose text holds
none of the guarded path forms (commit `85c7da9c6`, 2026-11-16). Most eligible
files contain none, so a full-tree violation scan should now cost less than
the figure above. It has not been re-measured; acceptance 5 records it.

### The handoff's contract tests are expensive in archive mode

Two tests in `tools/test-toolkit/tests/ci_workflow_contracts.rs` exist only to
prove that the Python emitter and the Rust reader agree:

- three `the_shipped_planner_emits_a_{documentation,full_scope,changed_file}_plan_the_guards_reader_accepts`
  tests run the shipped planner against the real workspace, once per shape
  (`-- README.md` → not selected, `--all` → full, `-- tools/test-toolkit/src/archive_guard.rs`
  → changed), and parse each result with `GuardPlan::from_plan_json`.
- `the_shipped_planner_omits_deletions_and_the_reader_refuses_an_unexpected_absence`
  checks that deleted paths are omitted from `changed` and that the reader
  refuses a listed path that does not exist.

On hosted CI, each test took 64–82 s on `ubuntu-latest`, and both timed out
at 90 s on `windows-latest` (runs 37529082394, 37537695506). At the time those
timeouts kept `test-toolkit` L1 red on `windows-latest`. `windows-latest` is now
scheduled only by the nightly, `workflow_dispatch`, and `ci:all-os` pull
requests, so that is where such a timeout would show.

Archive-mode runs on the build hosts (`BUILD_WSL` and `BUILD_WIN` through
`just cross-check`) located the cause. Their durations do not predict hosted CI
durations, so they are not quoted here. On both hosts, most of each test's time
was the planner's first build-key call. An archive checkout has no `target/`,
so `scripts/ci/build_key.py` falls back to `cargo run` and compiles `ci-build`.
Every plan computes at least one key, including a plan for a `README.md`-only
change. In the same runs, the test-input scan was a small share of the time.
Giving test 1 an input that skips the scan left both tests above nextest's
30 s slow mark on `BUILD_WIN`.

`2026-10-06-archive-key-helper` has since been implemented: an archive
consumer binds the artifact's verified `ci-build` as the key helper, and
`build_key.py` no longer falls back to `cargo run`. That removes the compile
these tests paid for. No hosted `windows-latest` duration for them has been
recorded since. The timeouts are therefore not, on their own, a
reason for this spec. Deleting the two tests removes their cost along with the
handoff they prove, but the case for removing the handoff rests on the
subsections before and after this one: the scan cost and the detection goal. Whether the `windows-latest`
timeout clears is judged from `windows-latest` runs.

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
2. This fix does not change scheduling: the same changed paths trigger a guard-only
   `test-toolkit` lint cell, the same ordinary owner lint cells carry the
   companion, and no guard cell forces `ubuntu-latest` into an event that does
   not schedule it. "Unchanged" means unchanged *by the scan simplification*;
   it does not freeze today's ordinary-cell attachment against separate
   ownership changes (see
   [Coordination](#coordination-with-2026-11-17-too-much-work)). A scheduled
   guard can now fail for a violation in an untouched file; that wider
   detection is intentional.
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
  contract; keep the eligibility and owned-input agreement tests. The current
  `archive_guard_scope` docstring says a `schedule` run schedules WSL2 alone;
  that is stale (the nightly schedules `windows-latest` and `wsl2-ubuntu`), and
  its replacement states the rule in terms of the scheduled environments.
- Selection tests pin the selection policy in force when this fix is
  implemented. If `2026-11-17-too-much-work` lands first and narrows when an
  ordinary `test-toolkit` lint cell exists, these tests follow that result
  rather than restoring the broader selection.

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
  [the plan renderer](../../scripts/ci-plan-render.rs), which the `repo-deps`
  `ci-plan` binary uses, and the local runner's `jq` fallback. They derive guard presence from lint-cell
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
  `test_evidence_reuse.py`, `test_local_evidence.py`, `test_cross_check.py`,
  `test_hosted_observations.py`, and shared plan fixtures
  (`plan_fixtures.archive_guard` is removed)
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
rather than depending on a hard-coded claim about the nightly event. This fix
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
4. `just cross-check test-toolkit --os windows` passes. Reuse qualifying
   evidence for required cells on each environment and run cells lacking it. Review `just ci-local --plan`
   before an implementation push. Normal event scheduling applies; no
   `ci:all-os` label or full-workspace CI run is required just to review or
   validate this simplification. Existing execution bans remain binding.
5. The selected Ubuntu lint companion reports `mode=full-tree` and a positive
   file count; a failing or unreported required companion cannot pass its
   cell. Record durations from the normal validation run for context,
   including the violation test's full-tree time with the tokenization
   prefilter. The
   existing measurements justify removing the handoff without a performance
   spike or a new timing threshold: 3.9 s measured one raw-form pass, not the
   total duration of two tests, compilation, or CI setup.

## Coordination with `2026-11-17-too-much-work`

The two specs overlap in the planner and its test contracts but answer
different questions. Neither supersedes the other, and neither depends on the
other being implemented first.

| | This fix | `2026-11-17-too-much-work` |
| --- | --- | --- |
| Question | What does the guard scan once selected? | Which checks should a change select? |
| Change | Always scan the full eligible tree; remove the file-list handoff | Replace directory-based selection with checks justified by the affected behavior |
| Scheduling | Not changed by this fix | Intentionally changed |
| Cost removed | Scope machinery and its contract tests | Unrelated gates, tests, and compilation |

Where they meet:

- **Planner ownership and companion attachment.** This fix keeps both guard
  routes: an ordinary `test-toolkit` lint cell carries the companion, and an
  explicit trigger without other owner work adds the guard-only cell. The
  newer spec may reduce how often an ordinary owner lint cell exists (for
  example, after a comment-only `justfile` change or a maintenance-script
  change under `tools/test-toolkit`). That incidentally reduces guard runs and
  is acceptable. What must survive either change: an eligible-source change or
  a change to one of `ARCHIVE_GUARD_OWN_INPUTS` (deletions included) still
  selects the guard whenever the event schedules `ubuntu-latest`.
- **Contract-test selection.** Both specs agree on the rule: a test is deleted
  when the contract it proves disappears (this fix's cross-language scope
  tests), never merely to lower a test count. Surviving guard contracts listed
  under Tests stay.
- **One canonical plan.** Both keep the planner as the only scheduler. After
  this fix the scanner reads no plan, but the workflow still reads the
  resolved plan to decide whether to execute the companion.

Full-tree scanning does not conflict with the newer spec's rule that every
selected gate answers a concrete question. A selected guard answers: "does the
eligible tree contain any unexempted archive-unsafe path?" Its repository-wide
scope is intentional coverage; whether a given change should select it is the
newer spec's question.

## Resolved Questions

### Which environments should the nightly event schedule?

**Settled outside this fix.** When this spec was first written, the nightly
scheduled WSL2 alone while the repository instructions described Linux,
Windows, and WSL2. The environment table and the repository instructions now
agree: `schedule` runs `windows-latest` and `wsl2-ubuntu`, and pull requests
and pushes to `main` prove `ubuntu-latest` and `macos-latest`. The nightly
therefore schedules no `ubuntu-latest` cell and never executes the guard;
Linux coverage of the guard comes from pull requests and pushes. This fix does
not change `.github/ci/environments.json`.

The guard rule is unchanged: it runs only when the resolved plan contains a
lint cell carrying its companion on `ubuntu-latest`.

## Not in scope

- **Planner in Rust.** `scripts/ci/test_inputs.py` is a Rust lexer written in
  Python and the expensive part of planning; moving it into Rust, sharing the
  guard's lexer, is the proposed first step of an incremental migration. It
  needs its own spec.
- **Selection by affected behavior.** Narrowing which changes select ordinary
  `test-toolkit` lint cells belongs to `2026-11-17-too-much-work`.
