---
area: repo
status: implemented
created: 2026-09-22
clarified: false
reviewed: false
review_iterations: 0
implemented: true
implemented_by: claude-code/claude-opus-5-5
owner: Ken Snyder <ken@ken.net>
origin: darkmatter L1 red on main, found by a manual full-suite run, 2026-09-22
related:
    - 2026-09-19-less-brittle
    - 2026-09-21-ci-build-feature-divergence
packages:
    - repo-deps
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

# Schedule the tests that read a changed file, and nothing else

## Outcome

A change to a file that compiled code reads, rather than compiles, runs the
code that reads it:

- A file embedded into **shipped** code (`include_str!` / `include_bytes!`
  outside tests) is that package's source, and selects it the ordinary way.
- A file read by a **test** schedules one `{package, ubuntu-latest, L1}` cell,
  narrowed by a nextest filter to exactly the tests that read it.

A file nothing reads (a README typo, the canonical case) still schedules
nothing. No other cell kind is added: no lint, no check, no other environment,
no dependents' seam. The pushing host's exact-tree run of the narrowed tests
satisfies that cell from any OS, so a push through the hook costs CI nothing.

The live rule, for readers new to this repository, is
[`docs/cicd/test-inputs.md`](../../docs/cicd/test-inputs.md). This spec keeps
the reasoning, the alternatives, and the measurements.

## Problem and evidence

`darkmatter::l1 schema_phase_validation::public_docs_and_skill_describe_required_and_eager_as_independent_axes`
reads three documents by path and asserts their wording. It is red on `main`:
it still names `darkmatter/docs/topics/schema-definition.md`, which no longer
exists. Three documentation-only commits broke it, and each planned nothing:

| Commit | What it did | Plan at the time |
|---|---|---|
| `869cbb225` | edited `docs/inline/schema-validation.md`, copied the definition doc into `topics/schema/` | `documentation`, no package |
| `aa1f03c70` | deleted `topics/schema-definition.md` | `documentation`, no package |
| `330805a84` | edited `schema-validation.md` again, renamed `topics/schema/definition.md` into `topics/schemas/` | `documentation`, no package |

The pre-push hook plans with the same planner, so it ran nothing either.

The cause is deliberate policy. `SOURCE_SUFFIXES` omits `.md`, and so do
`.yaml`, `.json`, `.svg`, `.scm`, and every other non-code suffix, so that a
documentation edit does not rebuild unchanged packages. That default is what
keeps CI cheap, and this fix keeps it. What the default could not see is the
subset of those files that code reads.

### The blind spot is wider than documentation, and was latent in half of all merges

Measured over `main` since 2026-08-01 with
[`spikes/history_cost.py`](spikes/history_cost.py): **32 of 62 first-parent
merges** changed a non-source file that a test of an otherwise-unselected
package reads. None of those tests ran. The darkmatter test happened to break;
the others happened not to. At commit granularity, which is the most a
pre-push hook sees, it was 253 of 1,174 commits.

The dominant readers are CI's own documentation contracts, for example
`test-toolkit::ci_workflow_contracts` `the_ci_documentation_states_the_implemented_behavior`,
which asserts the wording of `.github/ci/README.md`,
`.claude/skills/rust-devops/ci-cd.md`, and `docs/topics/ci-cd.md`. Also in the
set: claudine's shipped-prompt contracts (`prompts/*.md`), darkmatter's
feature-spec corpus tests (`darkmatter/features/**`), and the live instance.

## Inventory

A static index over every `lib`, `bin`, and `test` target's `mod` tree
([`scripts/ci/test_inputs.py`](../../scripts/ci/test_inputs.py)). Candidates
were all 8,379 tracked non-source files; the scan takes 3.7 s.

| Measure | Count |
|---|---|
| References, total | 6,755 |
| Embedded into shipped code | 298 (6 packages: biscuit-icon `.svg` 129, tree-hugger `.scm` 68, playa audio 88, and darkmatter/research/biscuit-terminal-cli docs and schemas 13) |
| Embedded into test code only | 141 references to 96 files |
| Read by tests at run time | 6,316 references to 1,759 files |
| Distinct files read by tests | 1,819, by 415 test units in 26 packages |
| …of those, outside the reading package's own directory | 1,441 |
| Documentation files (`docs/`, `.claude/`, READMEs) read by tests | 52, by 58 test units |

**The size decides the design.** A hand-declared `doc-inputs` table would hold
about 1,800 path-to-test mappings. That table is a second copy of every path,
and a second copy of a path going stale *is* this defect. So the mapping is
derived from source, not declared.

### What counts as a read

The index was built precision-first against real history, because over-selection
is paid in CI time. Each rule below removed measured false positives:

| Rule | Why | Merges with narrowed cells |
|---|---|---|
| Any literal equal to a tracked path, bare names and directories included | first cut | 49 of 62 |
| Model suite-owner selection as "already selected" (`.github/workflows/**` already selects `test-toolkit`) | double counting | 43 |
| A directory, or a single-component name, counts only when joined onto a root | `.claude/skills` and `README.md` are written into tempdirs by hundreds of tests | 43 |
| A free multi-component literal counts only in a file that reads through a root | sniff's classifier tests mock `".github/workflows/ci.yml"` | 32 |
| Resolve the `.join()` receiver: known roots, same-file bindings, `.parent()` steps; an unknown receiver (`fixture.cwd()`, `dir.path()`) is a fixture | `.claudine/memory/commits.md` written into a fixture; `manifest_dir!().parent().join("justfile")` resolved to the wrong justfile | **32** (final; every remaining driver was checked and is a genuine read) |

The recognized forms, now documented for test authors in the `rust-testing`
skill, are:

- `include_str!` / `include_bytes!`
- a `.join("…")` rooted at `manifest_dir!()`, `CARGO_MANIFEST_DIR`, a
  `repo_root()` / `repository_root()` / `workspace_root()` helper, or a name the
  same file binds to one of those
- a full repository-relative literal in a file that reads through a root

A path assembled at run time is invisible. That is the status quo, not a
regression.

## Design

### Chosen: derived static index, one narrowed Linux L1 cell

1. `diff_scope.py` also emits `--renamed-from <old>` for renames. The old name
   enters neither the changed nor the deleted list, so the change inventory and
   the archive guard keep their contracts. It is read only by this search, to
   catch a test still naming a file that moved away.
2. For every changed, deleted, or renamed-away **non-source** path, the planner
   runs `test_inputs.scan` over the head tree (1.3 to 1.7 s in the live replay).
3. A reference from shipped code makes the path package source (reason:
   `embedded file … changed`). Reverse dependencies are handled like any other
   source change.
4. Test references are grouped per package. A package whose L1 the plan
   already runs gains nothing. Any other package gets one cell from
   `package_cells` with its policy reduced to L1 and its environments reduced
   to `TEST_INPUT_ENVIRONMENT` (`ubuntu-latest`), so execution, profile,
   build, and prohibition handling are exactly an ordinary L1 cell's. Two
   properties differ:
   - `test_filter` is the union of the readers' exact nextest identities:
     `binary_id(<bin>) & test(=<module>::<fn>)` for a literal inside a
     `#[test]`, or `test(/^<module>::/)` for one in a helper. Units the L1 tier
     excludes (`level2_`/`level3_`/`browser_`/`real_`, `slow_` unless
     `l1-include-slow`, and `level2*` binaries) are dropped.
   - Its evidence rule. Only a receipt cell carrying the identical
     `test_filter`, recorded on the head's exact tree, satisfies it, and that
     receipt may come from any host (see "Local evidence from any host"
     below). Whole-tier evidence never satisfies it, and narrowed evidence
     never satisfies a whole-tier cell.
5. The producer passes the filter as `BISCUIT_TEST_NARROW`. `_tier_filter`
   intersects it with, and never replaces, the tier expression. Both the gate
   and its expected-test listing read `_tier_filter`, so they narrow
   identically. A narrowed run uses `--no-tests=fail`, so a stale filter fails
   instead of passing having run nothing.
6. `completion.py` admits exactly one extra conjunct, the cell's own
   `(<tier expression>) & (<test_filter>)`, and still validates the tier half
   canonically.
7. `just ci-local` runs a narrowed cell on **any** host, before the push, and
   `record_cells` records it with its `test_filter`. `verify_cells` credits it
   to the plan's narrowed cell only (`_resolve_narrowed`), keyed by the plan's
   environment, with its evidence naming the host's ref. This closes the
   "local validation ran nothing either" half of the defect, and removes the
   CI cost of this rule for every push that goes through the hook.
8. `_test` consumes `BISCUIT_TEST_NARROW` before starting nextest. Left
   exported, it reached tests that run `just _test` themselves (`repo-deps`'
   archive fixtures) and narrowed their selection to nothing. The local sweep
   below found this; `L1ThreadForwardingTests` now pins it.

### Local evidence from any host

Whether a *test* is host-independent cannot be decided soundly from its code,
and a maintained annotation would drift, which is the same objection as to
declared `doc-inputs`. The exception is justified by the *change* instead,
which the planner knows exactly: a narrowed cell exists only when none of the
package's source changed. So the code under test is what last passed the full
matrix on every OS, and the only new variable is the content of the file it
reads. The residual risk is content that drives an existing OS-specific branch
differently (a fixture gaining a Windows-style path). That is the risk the
repository already accepts for Windows and WSL2, and the nightly full run on
Linux, Windows, and WSL2 surfaces it within a day.

The exception is deliberately narrow: identical filter, exact tree. A document
is not a gate input, so the gate-input equivalence that lets an ordinary
receipt carry forward to a later head would vouch for a run against old
wording.

**Why Linux alone in CI.** Whether a document's wording or a fixture's content still
satisfies the test that reads it does not depend on the OS. Linux is where
lint and check already live. The whole-tier run on every environment still
happens the next time the package's source changes.

**Identity.** The cell is `{package, ubuntu-latest, L1}`; `test_filter` is an
attribute of it, not part of its key. Every stored name (artifact, completion
record, receipt cell) keeps the existing key, and `affected_scope.py` remains
the only place scope is decided.

### Cost of the chosen design (measured)

| Measure | Value | Source |
|---|---|---|
| Frequency | 49 narrowed cells over 62 merges (about 7 per week), 21 of them `test-toolkit` | `spikes/history_cost.py merges` |
| Embedded-source selections | 1 of 1,174 commits | same, commit mode |
| One narrowed `test-toolkit` cell | ubuntu owner build 4.8 to 8.4 min (built with `repo-deps`), consumer job 2 to 2.7 min | runs `35641847006`, `35634811758`, `35618767264` |
| One narrowed `darkmatter` cell | 381 s warm compile, plus the same consumer overhead | `2026-09-21-ci-build-feature-divergence` |
| Weekly total | at most about 50 to 80 runner-minutes, before local receipts; each push through the hook satisfies its narrowed cells locally at zero CI cost. For comparison, one nightly ubuntu build is 102 min. | estimate from the rows above |
| Planner overhead | +1.3 to 1.7 s per plan with a non-source change; 3.7 s worst case (every non-source file changed) | `spikes/live_replay.py` |
| README typo | 0 | `RealWorkspaceTestInputTests` |

Compile dominates every narrowed cell. The filter saves only the rest of the
suite's run time (tens of seconds), but that is not its main value: a doc PR
is no longer blocked by an unrelated red or flaky test in the same suite.
When the package's archive is already being built for another cell, the
narrowed cell adds only the consumer job.

### Alternatives and why not

| Option | Question it answers | Cost per triggering change | Verdict |
|---|---|---|---|
| Add `.md` (and friends) to `SOURCE_SUFFIXES` | "Does anything in this package still work?" for every doc edit | darkmatter: lint + check + L1 on 2 to 3 environments + seam, about 25 to 30 runner-min, **including README typos** | Rejected: it gives back the savings this repository was built around. |
| Declared `doc-inputs` in `[package.metadata.ci.tests]` | Same as chosen, for declared paths | Same as chosen, plus the upkeep of about 1,800 mappings | Rejected: the declaration is a second copy of every path, the very thing that went stale here. |
| Path-existence guard in `lint` (ubuntu) | "Does every path a test names exist?" | Seconds, no compile | Rejected as the primary fix. It answers only the deletion case (`aa1f03c70`), not wording drift (`869cbb225`, `330805a84`). Over arbitrary literals it is also imprecise: a heuristic scan found 129 "dangling" repo-shaped literals, of which only the darkmatter one is real; the rest are tempdir fixtures and deliberate absences. The chosen cell catches deletion too, because the test fails with `ENOENT`, as the end-to-end run below shows. |
| Runtime-recorded inputs (harness logs reads; manifest checked in) | Exact file-to-test mapping | Instrumenting every read path, plus a checked-in manifest that itself drifts | Deferred: the static index reached precision on real history without it. Worth revisiting if run-time path assembly turns out to hide real readers. |
| Compile-time coupling (`include_str!`) | Turns a missing file into a build error | None added by itself | Adopted where it fits: shipped embeds now select their package, and the `rust-testing` skill recommends it. On its own it schedules nothing for a docs-only change; it only shortens the fuse. |

## Verification

- **Hermetic replay** (`HistoricalDocumentationBlindSpotReplayTests`). Each
  commit's exact `--name-status -M` diff goes through the real `diff_scope`
  parser, over the test as it stood (its own binary, local `repo_root()`).
  Every commit now plans exactly `darkmatter/ubuntu-latest/L1` narrowed to that
  test. Nothing else is planned: no lint, no check, no other OS, no
  dependents. A `darkmatter/README.md` edit still plans nothing and builds
  nothing. With the index disabled, these tests fail (checked).
- **Live replay** ([`spikes/live_replay.py`](spikes/live_replay.py), real
  worktrees and `cargo metadata`). For all three commits: before, `documentation`
  with no package; after, one narrowed `darkmatter/ubuntu-latest/L1` cell with
  a schema-valid plan, in 1.3 to 1.7 s.
- **End to end on today's tree.** `BISCUIT_TEST_NARROW=<the planned filter> just _test darkmatter`
  starts `1 test across 1 binary (753 tests … skipped)`. That test fails with
  `darkmatter/docs/topics/schema-definition.md is unreadable: No such file or directory`,
  which is the defect caught. A filter matching nothing exits 1 with
  `no tests to run`.
- **Unit coverage.**
  - `TestInputIndexTests`: 13 cases, one per reference form and per
    false-positive class above.
  - `TestInputSelectionTests`: 11 planner cases (deletion, rename source,
    embedded source, already covered, event without Linux, reuse refused,
    contract, schema refusals).
  - `RealWorkspaceTestInputTests`: the live docs select the live test, README
    typos select nothing, and a guard-only `test-toolkit` gains the L1 cell on
    its one record.
  - `test_completion` (narrowed selection accepted only as planned),
    `test_evidence_reuse` (narrowed run never recorded; fails without the
    exclusion), `DiffScopeParserTests` (`--renamed-from`), `test_ci_local`,
    and the pre-push hook suite (rename source declared, never
    changed/deleted).
- **Existing tests that changed their expectation.** Four assertions used
  `docs/topics/ci-cd.md` as "a document that selects nothing". It is read by
  `test-toolkit`'s contracts, so each was itself an instance of this blind
  spot. They now use `docs/comment-quality.md`, which nothing reads, and the
  corpus test states both columns: ownership selection and test-input
  readers.

Suites run: `test_affected_scope` (366), `test_resolved_plan` (102),
`test_ci_local` (100), `test_completion` (62), `test_evidence_reuse`,
`test_local_evidence`, `test_schema`, `test_cross_check`, `test_runner_loss`,
`test_reuse_validation`, `test_constraints`, `test_publish_gaps`,
`test_build_key`, `just test test-toolkit` (345), `just test repo-deps` (449),
all passing. `just test-pre-push-hook` shows 63 passing and 5 failing. The same
5 fail on an untouched `HEAD` worktree: the harness does not copy
`scripts/ci/test_consolidation.py` into its fixture repository. That failure
is pre-existing and unrelated.

## Changes

| File | Change |
|---|---|
| `scripts/ci/test_inputs.py` | New. The static index. |
| `scripts/ci/affected_scope.py` | `renamed_from` / `--renamed-from`, embedded-source selection, `select_test_inputs`, `test_input_only_cells`, area reason for input readers |
| `scripts/ci/diff_scope.py` | Emits `--renamed-from` |
| `scripts/ci/schema.py`, `.github/ci/schemas/contract.json` | Optional cell `test_filter`, valid only on an executing or reused L1 cell; optional receipt-cell `test_filter` |
| `scripts/ci/cell_contract.py` | `test_filter` output |
| `.github/workflows/_package-ci.yml` | `BISCUIT_TEST_NARROW` on the listing and gate steps |
| `just/devops.just` | `_tier_filter` intersection; `--no-tests=fail` when narrowed; `cargo test` fallback refuses a narrowing |
| `just/ci-local.just` | `--renamed-from` pairs; narrowed cells run locally on any host, staged apart |
| `scripts/ci/completion.py` | Admits exactly the planned narrowing |
| `scripts/ci/local_evidence.py` | `record_cells` records a narrowed run with its filter; `_resolve_narrowed` credits it across hosts, exact tree only |
| Docs | `docs/cicd/test-inputs.md` (the home); pointers in `CLAUDE.md`, `.github/ci/README.md`, `.github/ci/schemas/README.md`, `docs/topics/ci-cd.md`, `docs/cicd/schema-versions.md`, and the `rust-testing` skill; reusable lessons in `rust-devops/ci-cd.md` |

`test_filter` amends plan version 7 without a bump; the rationale is recorded
in `docs/cicd/schema-versions.md`. `ci-rollup.rs` reads plan cells without
`deny_unknown_fields`, so no mirror moves.

## Local sweep of accumulated failures

Every test the index attributes to any tracked non-source file (278 test units
in 26 packages) was run locally with its narrowed filter, to find what had
drifted while unscheduled. See the summary for the result.

## Not done here, and follow-ups

- **The red test itself is not fixed.** Repointing it at
  `darkmatter/docs/topics/schemas/definition.md` is a one-line change; all five
  expected claims and none of the retired ones are present there, verified with
  whitespace collapsed. It is a darkmatter source change, so it belongs to that
  area's next change rather than to this CI fix, where it would pull
  darkmatter's full matrix into this pull request.
- **Python and TypeScript suites are not indexed.** For example,
  `test_schema.py` reads `docs/cicd/schema-versions.md`. CI's own suites are
  partly covered by `SUITE_OWNER_PATHS`; extending the index to
  `scripts/ci/test_*.py` is the natural next step.
- **Source read as text across packages** is not selected (for example
  `claudine::boundary_lint` walking the claudine tree). Measured: including
  source paths would add 7 more merges of 62, almost all from whole-tree
  walkers.
- **A narrowed cell builds its package's whole archive.** Restricting the owner
  build to the named test binaries would cut most of the compile, but it
  changes build-key semantics, so it is a separate question.
