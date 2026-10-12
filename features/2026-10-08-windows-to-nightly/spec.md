---
kind: feature
name: windows-to-nightly
date: 2026-10-08
status: implemented
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
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-11
review_iterations: 3
completed: true
implemented: true
implemented_by: claude-code/claude-opus-5-5 (cadence, zero-test packages, Phases 5-6); a separate implementation run (Phases 1-4)
human_review: false
message_to_agent: |-
  Phase 4 is implemented (see the log). Things the next phase must know:
  1. Rig and hosted producers now publish observations: `scripts/cross-check.sh` records one `rig` observation per finished leg (`wsl2-ubuntu`, `windows-latest`) with a per-run proof directory; `hosted_observations.py` emits and publishes `hosted` ones on scheduled runs; `local_evidence.py verify --artifact-repo` vouches for hosted artifacts. A passing rig/seed observation's proof documents are rechecked byte for byte by a reader on the same host.
  2. A seed command (Phase 5) should call the same `publish_rig_observation` path in `cross-check.sh` per cell, or the `observation_notes.py publish-receipt --kind rig` CLI it wraps. Each rig run needs its own evidence directory (`<head>/<environment>/<package>-<gate>-<run_id>`): two runs sharing one would break the first run's digests.
  3. The zero-test packages are settled (author chose option B, 2026-10-11): `reaper`, `reaper-cli`, `ui`, `agent-sandbox-cli` and `research-cli` declare `[package.metadata.ci.tests] unimplemented`, so while their source has no test the planner gives them no test cell and records `unimplemented: {reason, tests_found: false}`. A full-workspace seed must therefore schedule nothing for them, not report them unproved. The first test they gain schedules them normally.
  4. Unverified on a real hosted run: that `GITHUB_TOKEN` may push `refs/notes/*`, `always()` behavior of the publisher on cancel, and re-run attempts that repeat an artifact name.
  5. Do not run cross-check from a scratch clone against the real rigs: its prune step deletes clones of worktrees it does not know about (it removed three warm clones on `build-win-native` in Phase 4).

  Earlier, from Phase 3:
  Phase 3 landed per-environment nightly anchors. DEPARTURE from the Phase 1 ruling: the resolved plan stays at version 7 (no v8): `coverage_anchors` is an OPTIONAL field present only on a nightly (like `event`), so the archive-guard fix's claim on v8 is untouched and no Rust constant or hook fixture moved. `reuse_validation.py nightly-base` is gone; `nightly-anchors` prints the anchor document, `affected_scope.py --anchors FILE` derives each environment's diff itself (calculate_scope(coverage_groups=...), select_group/merge_selections/apply_policy_delta), and `certify` writes the per-environment `ci-coverage-v1-<env>-<head>-<policy16>` artifact from the new `certify-nightly` job in ci.yml (schedule-only, gated on ci-gate success, contents+actions read only, not in ci-gate's needs). The scheduled run now runs the evidence verifier/overlay (only workflow_dispatch skips it) with the all-zero base. Legacy (pre-certification) anchors carry covered_cells=null: the planner assumes they cover whatever the diff does not reselect and adds NO policy delta, which is a stated limit until the first certified nightly. Nothing publishes Windows or hosted-cell observations yet, so Phase 4 producers decide how much the first nightlies can reuse; the first nightly after merge is a bootstrap (no nightly has ever succeeded) and runs the full gated workspace on hosted runners. Phase 4 should also smoke `observation_notes.py publish-receipt` and the Python suites on the native Windows and WSL2 rigs (still macOS-only evidence), and check whether GITHUB_TOKEN may push refs/notes/* before relying on hosted note publication. `ci-rollup` now refuses malformed reused-cell proof (no evidence, CI-origin evidence, unknown outcome); it cannot verify the host OS of a reused observation, only its shape. Run the hook test suite with `CDPATH= bash .githooks/tests/test-pre-push.sh` (a set CDPATH breaks its repo-root discovery).
related:
  - 2026-09-18-ci-cadence
  - 2026-09-19-nightly-scope
  - 2026-09-19-hosted-evidence-reuse
---

# Move Windows coverage to scoped nightlies

## Goal

Keep normal PR and merge validation fast: Linux and macOS provide immediate
coverage, while native Windows and WSL2 provide nightly coverage over changed
packages. Reuse valid test-rig evidence instead of repeating those cells on
hosted runners. Scheduling, Python enforcement, documentation, and agent
instructions must describe the same policy.

A **cell** is one package's required gate (a test tier or compile/lint check)
on one environment. A **receipt** records what a host tested and its results.
The **resolved plan** is the single document that says which cells run, reuse
verified proof, or have an approved temporary capability gap. A nightly's
**coverage anchor** is the earlier commit through which its required coverage
has been satisfied; it determines which later changes still need proof.

This specification is partially implemented. The cadence correction and the
bounded wait for an unfinished PR run landed in this worktree on 2026-10-10,
as recorded in the adjacent implementation log. Test-rig seeding, receipt
publication/consumption, migration safeguards, and regression checks remain
required. The optional risk-directed pre-push interface is deferred from the
first delivery; its evidence contracts still apply to required workspace seeds.
The document remains a draft because the evidence design needs the decisions
listed in Open Questions before implementation.

## Verified behavior and implementation status

The original inspection was on 2026-10-08, including the repository's GitHub
API. Local code and the implementation log were rechecked on 2026-10-11.
The table below describes this worktree; it does not claim the changes have
been merged or deployed.

| Surface | Current behavior |
| --- | --- |
| `.github/ci/environments.json` | Linux/macOS on PR and push; Windows and WSL2 on schedule; all four on manual dispatch. |
| `scripts/ci/affected_scope.py` | Derives event coverage from that table and produces one canonical resolved plan. |
| `scripts/ci/test_affected_scope.py` | Updated event-policy assertions cover Windows/WSL2 nightlies and an empty plan after reuse of matching PR validation on push. |
| [`nightly_base`](../../scripts/ci/reuse_validation.py), in the `repo-deps` package | Searches up to 20 successful scheduled `ci.yml` runs on `main`, newest first, and selects the first whose head is an ancestor of the current head. No qualifying run, or an API failure, returns no base. |
| `.github/workflows/ci.yml` | Diffs the selected base against the current head; no base means `--all`. Scheduled runs bypass local evidence verification and overlay. |
| `scripts/cross-check.sh` | Runs native Windows via `BUILD_WIN`, but publishes validation receipts only for WSL2. The Python cross-check recorder already accepts an environment argument; the shell publication path is WSL2-specific. |
| Skills and instructions | Primary CI docs, `AGENTS.md`, and the rust-devops/os skills now describe Windows/WSL2 nightlies and mark seeding and scheduled receipt reuse as planned. The focused cadence-drift check remains outstanding. |

### The nightly is not a fixed 24-hour window

It runs on a daily schedule, but its comparison range is **since the last
successful ancestor nightly**, not since midnight or 24 hours ago. This keeps
unproven changes in scope after failed or canceled runs. With a successful base
equal to the current head, the empty diff schedules no package test cells.
The lightweight workflow still starts to resolve and report that empty plan.

Selection uses the existing affected-scope rules. Source changes select their
owning packages and declared gates; non-source changes select only code or
tests that read them. Unchanged direct reverse dependencies are reported and
compiled in the changed package's Linux check where that environment is
scheduled; they do not receive independent test suites. This is not a promise
to retest every transitive dependent. Linux may join a nightly solely to build
archives consumed by WSL2, without Linux test, lint, or check cells.

### Observed fallback cost

In the 2026-10-08 inspection, GitHub's successful scheduled-run query returned
`total_count: 0`. The
[latest inspected nightly](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/37800062455)
used an all-zero base and recorded an explicit full-scope inventory: 66
executing WSL2 cells, 17 omitted accepted-gap cells, and 68 distinct packages.
The eight latest scheduled runs were failures, with several sharing the same
head. Thus the desired incremental mechanism exists, but has not obtained a
successful baseline in the available hosted history and repeated
full-workspace attempts in that inspected history. These are historical
observations, not a fresh diagnosis of hosted runs on the review date. Recheck
the affected cells during implementation; do not present this fallback as
efficient incremental coverage. These figures already justify resumable
seeding; no additional performance spike is required.

## Required cadence

| Event | Test environments |
| --- | --- |
| Ordinary pull request | Linux and macOS, with qualifying host evidence reused. |
| Push to `main` | Linux and macOS only; matching whole-PR validation can leave no package cells to execute. |
| Nightly `schedule` | Native Windows and WSL2 over the unproven changed scope. Linux builds WSL2 archives only when demanded. |
| Explicit full manual dispatch | Preserve the existing all-environment behavior. |
| Explicit `ci:all-os` PR opt-in | Preserve the existing opt-in exception; it must not become the ordinary cadence. |

1. Change Windows events to `schedule` and `workflow_dispatch`. Do not add
   Windows back to ordinary PR or push through another workflow, hook default,
   Python special case, matrix, or fallback.
2. Keep the environment table authoritative. Python and workflow consumers
   derive their work from the same resolved plan; no second environment or
   package scheduler is introduced. Stored identities remain
   `{package, environment, gate}`; areas remain derived groups.
3. Update Python event-policy assertions, fixtures, proven-event handling,
   producer-demand checks, and schema contracts. A matching PR-proven push
   must produce an empty execution grid, not a Windows grid. Keep unchanged
   check/lint environment policy and the policy-free `ci-gate`. For a push
   merged before its matching PR run finishes, preserve the implemented
   90-minute wait and reuse only a qualifying success for the same tree.
   Failure, cancellation, timeout, or unverifiable identity falls back to the
   ordinary Linux/macOS plan, never a Windows/WSL2 plan.
4. Preserve capability gaps as explicit accepted gaps with their existing
   governance. Windows and WSL2 evidence remain distinct. Evaluate gap and
   exact-skip
   governance at the current run's policy date. An expired gap, a newly
   declared gate, or a newly required backend cannot be treated as proved by
   an older successful baseline. The planner adds newly required coverage
   even when the source diff is empty; evidence verification still decides
   whether those cells need execution.
5. Preserve test-input selection: non-source reads select narrowed Ubuntu L1
   only when Ubuntu is a scheduled test environment. Adding Ubuntu solely as
   a WSL2 archive producer must not enable Ubuntu test cells or turn a
   document-only change into Windows/WSL2 suites. Shared CI inputs continue
   to select their existing suite owners; internal prompt folders remain
   excluded under the existing policy.

## Merge eligibility and release eligibility

Passing every native Windows or WSL2 test is **not a requirement for merging
to `main`**. Ordinary merge eligibility follows the Linux/macOS PR gates;
Windows and WSL2 coverage may be gathered after merge through nightly runs.
Default strict pre-push validation still blocks a selected local/rig failure;
that hook behavior is distinct from a repository-wide Windows/WSL2 merge
requirement and retains its explicit warn-mode exception.

**Planned release policy: no release may proceed without qualifying passing
evidence from all
supported environments for the release scope:** macOS, Linux, native Windows,
and WSL2. Evidence must apply to the release's tested inputs and cover the
packages and gates required by that release scope. A merge to `main`, green
Linux/macOS PR checks, a successful nightly baseline, or passing evidence for
unrelated packages does not independently establish release eligibility.
Native Windows and WSL2 remain separate evidence requirements.

This section states the intended release policy only; it does not change or
certify the existing release automation. Defining release scope, evidence
aggregation, enforcement points, and treatment of governed capability gaps
belongs to subsequent release-process work. Do not infer that an existing CI
gap automatically waives a release requirement or claim release enforcement
is implemented by this cadence change. This specification will be developed
further in the branch owning that work; no branch name or lifecycle path is
part of the policy.

## Nightly scope and recovery

1. Retain the last-successful-ancestor rule rather than a wall-clock cutoff.
   Failed, canceled, incomplete, foreign-workflow, or non-ancestor runs never
   advance or roll back the last successful nightly's baseline. Preserve
   already-published completed cell evidence across failures and cancellations;
   a newer qualifying failure prevents reuse of an older pass for that cell,
   without erasing unrelated proof or changing the successful baseline.
   No new source or required-coverage change since a qualifying successful
   nightly means no package test cells, builds, or package preflight runners.
2. Do not silently discard pending coverage to make a nightly smaller. When
   no baseline exists, or baseline discovery fails, report the precise reason
   for full scope. Distinguish a genuine bootstrap from an API failure or an
   exhausted bounded search. Recover the existing failing nightly and obtain
   truthful coverage; never manufacture a successful baseline.
3. An old WSL2-only successful nightly cannot prove newly nightly Windows
   coverage. For migration, use its qualifying ancestor as the WSL2 comparison
   anchor, but select the full gated Windows workspace until a successful
   nightly has certified both environments under the new policy. Apply
   qualifying cell evidence before deciding what executes in either
   environment. If no WSL2 anchor qualifies, select full scope there too.
   A rig receipt never becomes a comparison anchor by itself.

   **Reader's note:** a single old comparison base would omit unchanged
   Windows packages that have never been proved. Resetting both environments
   would needlessly select old WSL2 work. The planner therefore records the
   migration anchors and their reasons in one resolved plan, unions their
   selections by package/cell identity, and derives dispatch and producer
   demand only from that plan. Consumers never calculate a second diff.
   After one successfully audited dual-environment nightly, both environments
   use its common ancestor anchor. A required plan-field change follows the
   repository's schema-version rules.

   Verify a candidate baseline's recorded coverage policy, not just its green
   GitHub conclusion. Pre-migration runs qualify only for WSL2. Missing policy
   provenance earns no Windows baseline credit. A run that skipped required
   audits cannot establish either environment's coverage anchor.
4. A successful empty or fully reused nightly may advance a baseline only
   when every cell it was required to prove is satisfied or has a governed
   accepted gap. A green orchestration job alone is insufficient.
5. Record fallback frequency and executing/reused/gap cell counts in the
   existing summary. Repeated full-workspace fallback must be visible. A
   blanket 24-hour cutoff or an arbitrary previous commit is not a remedy.

## Required full-workspace test-rig seeding

Provide a canonical command that seeds nightly evidence for **every gated
package**, so establishing coverage does not require a cold full-workspace
hosted Windows/WSL2 run. The command must be usable again for recovery, not
just once during migration. No command name is prescribed before examining
the existing Just and cross-check interfaces.

1. Resolve one full-workspace nightly plan at a pinned, committed `main`
   revision. Scope every package's native Windows and WSL2 cells from that
   plan, including declared gates and features. Packages with no gates remain
   visibly not scheduled. Seed both environments separately; a native Windows
   pass does not prove WSL2 and vice versa.
2. Execute planned cells on the configured rigs through the existing
   cross-check/build contracts. WSL2 evidence must use its faithful archive
   mode. Reuse warm build directories and qualifying prior evidence. Resolve
   host availability through the existing `BUILD_*`/`sniff` conventions;
   missing rigs must be reported, never silently credited.
3. Make the pass resumable per `{package, environment, gate}`. Persist and
   publish completed evidence as work finishes; a later failure or interruption
   must not lose earlier packages' receipts. On resume, reverify inputs and
   run only unproven cells. Do not create one monolithic receipt that becomes
   publishable only after the entire workspace passes.
4. Preserve the pinned revision throughout a pass. If `main` advances, retain
   the older proof and revalidate it using gate-input equivalence; rerun only
   cells whose inputs changed. Never label an older rig run as testing the new
   head. Avoid unsafe simultaneous mutations of one rig checkout or build
   directory; concurrency follows existing resource isolation constraints.
5. Retain real reports, expected/observed identities, completion, native target
   or guest-runtime proof, and build/input identity. Publish receipts that
   combine without overwriting another package at the same commit. Failures,
   partial runs, unavailable backends, and accepted gaps stay distinguishable.
   A rig may provide real backend evidence for a cell hosted CI cannot run;
   that evidence must satisfy the gate's contract before it is credited.
6. Expose a preview and progress summary listing packages, cells, host,
   reuse, execution, failures, and remaining coverage. Seed scope is explicitly
   full-workspace; subsequent ordinary nightlies return to incremental scope.
7. Nightly verification must consume seed evidence even during bootstrap with
   no successful nightly or an all-zero comparison base. The current verifier's
   merge-base lookup cannot be assumed to work for that base. Keep evidence
   discovery independent of baseline availability, verify the attached commit
   and tested inputs, and handle seed receipts at the baseline boundary.
   Enumerate annotated
   reachable ancestors from fetched evidence refs, including the anchor and
   pinned seed revision; do not require a merge base with an all-zero SHA or
   traverse every commit merely to discover a small set of notes. Squashed or
   rebased commits that are not reachable are not candidates in this delivery:
   rerun or republish proof attached to the actual tested reachable revision,
   never attach an old result to a new commit without verification.
8. A fully seeded first nightly may resolve all required cells to verified
   reuse or governed gaps, run no package tests/builds, pass the normal audits,
   and establish a genuine successful nightly baseline. Do not synthesize a
   GitHub success or merely set a baseline SHA to bypass missing coverage.
   With partial seeds, hosted CI executes only uncovered cells and their
   demanded producers. No Windows producer should be scheduled solely for
   Windows cells already proved; WSL2 may still require a Windows host.

This makes Windows receipt publication and scheduled evidence consumption
part of the required delivery, independent of the optional risk-selection
interface below. The identity, completion, selective reuse, and newer-failure
rules described there apply equally to workspace seeds.

## Nightly examples

These examples describe the required behavior, not the current scheduled
workflow, which bypasses rig receipts. A baseline is a successful scheduled
run's commit; individual receipts prove cells and do not themselves change
that baseline. Let `A` be the baseline and `B` the current `origin/main` tip.
Assume all required coverage through `A` is satisfied, any supplied receipts
are complete and valid, and the stated package is the planner's entire scope
(no additional shared-input or CI-tooling changes).

| Case | Baseline lookup and scope | Evidence assessment | Nightly execution |
| --- | --- | --- | --- |
| All required Windows and WSL2 cells are already proved at `B` | If a successful nightly already exists at `B`, `nightly_base()` returns `B` and the diff is empty. If the proof is rig receipts and the latest successful nightly is `A`, the baseline remains `A` and the planner selects `A..B` before applying receipts. Bootstrap without `A` uses full scope before applying seed receipts. | All selected required cells resolve to verified reuse or governed gaps. | No package builds or tests. Normal reporting/audits may establish a successful nightly at `B`, which becomes the next baseline. |
| Coverage is through `A`; `B` changes only `darkmatter-cli` | `nightly_base()` returns `A`; the planner scopes Windows/WSL2 cells for `darkmatter-cli` from `A..B`. | Prior receipts for the changed CLI no longer qualify when its gate inputs changed. Other packages receive no test cells merely because time passed. | Execute unproven Windows/WSL2 CLI cells. Build only the planned CLI targets and their compilation dependencies, natively on Windows and as a Linux archive for WSL2. Applicable capability gaps remain explicit. A successful audit advances the baseline to `B`; failure or cancellation leaves `A` unchanged. |
| Same CLI change, but complete Windows coverage was gathered before pushing `B` | `nightly_base()` still returns `A`; a Windows pre-push receipt is not a successful nightly. The selected scope is the same as the preceding case. | Qualifying native Windows receipts resolve all required Windows CLI cells to reuse. They provide no WSL2 proof. Verify the actual tested tree/input equivalence if the merge changes the commit identity. | Execute only unproven WSL2 CLI cells and build their Linux archive. No native Windows CLI build or test executes. WSL2 still needs its Windows runner host. Success advances the baseline to `B`. |

In these examples, `darkmatter-cli` is the only selected test package. Its
Cargo build includes dependencies, enabled features, test dependencies, and
the targets required by the selected cells; this does not select independent
test suites for those dependency packages. Linux produces the CLI's archive
because WSL2 consumes Linux binaries; the Windows host provisions the guest
rather than compiling the CLI for WSL2. Actual rebuilding depends on available
build caches. Complete Windows pre-push coverage must mean every required
Windows cell was proved or explicitly resolved under existing gap policy,
using the same nightly cell contracts. An incomplete run cannot be advertised
as complete Windows coverage; it falls under partial-evidence recovery instead
of the third example.

If a run fails after one environment completes, retain that environment's
published proof and leave the successful baseline unchanged. The retry must
reverify that proof and execute only still-unproven cells. A cancellation is
not proof of completion for an interrupted cell. If `main` gains another commit,
compare pending changes from the unchanged baseline and invalidate receipts
only where independently checked inputs differ.

Required reuse includes eligible per-cell hosted nightly results, not just
rig receipts: passing producers must publish durable, verifiable proof before
an unrelated producer can make the whole run fail or cancel. Reuse that proof
through the existing cell evidence model and canonical plan. Run artifacts
alone do not satisfy this requirement unless discovery and verification can
consume them; this integration is not present in the current nightly.

## Determinism and local-failure modes

Scope resolution and evidence overlay must be idempotent for identical inputs:
comparison base/head, committed package/dependency metadata, policy and its
evaluation date, gate contracts, and verified evidence set. The Windows cells
used for rig validation and nightly reuse share one coverage contract. A
nightly's accumulated diff can select more packages than one changeset's
pre-push diff; deterministic selection does not make those different input
ranges identical. Reuse matches cells and tested inputs, not a claim that the
two entire plans must be equal.

The existing hook supports `RUSTY_BISCUIT_PRE_PUSH=warn`. It allows a push after
local failures and publishes their measured outcomes. The default `strict`
mode blocks the branch push on failure but also publishes completed failure
receipts independently of that blocked branch transfer. Thus receipts are
records of observations, not exclusively assertions that all tests passed.
No claim is made here about whether warn mode has historically been used.
The current hook can publish complete outcomes from a failed pass; preserving
individual completed cells from an interrupted pass is an extension required
by this feature, not existing hook behavior. An interrupted cell itself never
earns passing credit.

| Mode | Branch push after a required local/selected rig failure | Evidence treatment |
| --- | --- | --- |
| `strict` (default) | Block. Fix and revalidate before pushing. | Retain/publish completed results honestly; passing cells can qualify later, failing cells cannot satisfy coverage. |
| Explicit `warn` | Permit with a visible failure summary. | Publish both passing and failing completed cells. This does not certify complete Windows coverage. |
| `scope-only` | No validation runs. | Publish scope only; no newly earned test evidence. |
| `--no-verify` | Hook does not run. | Produce no new hook evidence; independently published qualifying receipts can still apply. |

For a mixed warn-mode Windows receipt, reuse only independently qualifying
passing cells. Keep failed cells unproven and execute them when Windows is
next scheduled; no older qualifying pass may hide a newer qualifying failure.
Missing, malformed, interrupted, or incomplete records earn no passing credit.
Neither the failure nor its receipt rolls back the successful nightly baseline
or removes unrelated completed evidence.

Warn mode does not move Windows back into ordinary hosted PR or push coverage.
The pending Windows cells run in the nightly if their changes reach `main`,
or through an explicit existing all-OS opt-in. Their actual execution result
controls that run's gate/audit; the earlier local failure is diagnostic evidence,
not a fabricated failing hosted result. A newer complete passing result that
qualifies for the current inputs can satisfy the cell. A fix that changes
those inputs requires fresh validation.

The optional risk-directed rig workflow must honor the selected hook mode:
strict selected-rig failures block, warn explicitly allows the push and
reports pending Windows coverage. The normal successful pre-push example
assumes strict complete Windows validation; it does not describe warn-mode
partial success. Preserve per-cell successful evidence from a failed rig
pass or a failed workspace seed without advertising full Windows coverage.

## Optional Windows-risk test-rig workflow

When a changeset is believed to carry Windows risk, retain normal host
pre-push validation and additionally seek native Windows evidence on a fast
rig. Examples include Windows-specific code, path containment or comparison,
process and handle behavior, and filesystem cleanup. This is risk-directed
additional evidence, not a requirement to run Windows for every changeset.

1. Start with an explicit reviewer/author risk decision and an auditable
   reason. Automatic risk heuristics are not required by this feature.
2. Discover configured native Windows rigs through the existing `BUILD_WIN`
   and `sniff` conventions. No hardcoded host, new hosted Windows PR matrix,
   or infrastructure provisioning is implied. An absent or unreachable rig
   is reported and leaves Windows work for nightly; a real failure is
   surfaced and cannot become passing evidence.
3. Use the canonical planned package/gate selection and existing cross-check
   transport. Run only the Windows cells needed for the changeset and the
   gates the rig can actually prove. Warm rig builds remain useful. Do not
   mark a complete package or OS proved from a narrow filtered test run.
4. Extend Windows report retention and receipt publication under
   `refs/notes/ci-local/windows-latest`. Bind evidence to the actual tested
   committed tree and native Windows target/runtime. Require real reports,
   expected/observed test identity agreement, completion, gate inputs, and
   backend proof where required. A compile check, a zero-test pass that omits expected tests, a WSL2 run,
   a dirty mismatched tree, interruption, or unsupported flags do not certify
   Windows behavioral coverage. Preserve honest failure receipts.
5. Enable scheduled local-evidence verification and apply the existing
   per-cell overlay before deriving builds and dispatch rows. A malformed or
   unavailable receipt keeps the cell scheduled and reports its rejection.
6. Accept exact-tree evidence and independently recomputed gate-input
   equivalence across a merge. A Windows rig receipt removes only its
   qualifying Windows cells from execution; missing gates still execute.
   An entirely proven Windows scope needs no hosted Windows build or test
   producer. A Windows runner may still be needed to host WSL2.
7. Ensure candidate discovery includes feature-branch commits that became
   ancestors of nightly `main`, including applicable receipts at a coverage
   boundary. Test intervening unrelated commits, dependency or lockfile
   changes, and merge/rebase behavior. Do not widen reuse without independently
   checking input identity, and do not let an older pass override a newer
   qualifying failure.
8. Preserve package receipts across sequential rig runs on the same commit;
   publishing a later package must not overwrite proof for earlier packages.
9. Opting out means an explicit `reuse` state backed by verified evidence in
   the nightly plan and report, not an environment ban, accepted gap, hidden
   skip, or successful empty test command.

## Evidence integrity and publication

A validation receipt describes an observation. A producer completion record
certifies that a particular planned execution met its coverage contract. They
are distinct documents: a hosted completion record is bound to its original
head, build, run, and reports and must never be rewritten as completion of a
later run. Verified prior evidence satisfies a later cell as `reuse`; the
later audit checks its provenance instead of demanding an executing-cell
completion record for that reused cell.

Retain each observation's environment, tested commit/tree, package and gate,
filter/features, gate-input identity, host or workflow/run/attempt identity,
report references, and completion/backend proof. Recompute input equivalence
using the existing declared closure. No changed input outside a known closure
may be assumed irrelevant. Accepted gaps are current planning decisions,
never passing receipts; expired gap and skip approvals must be reevaluated.
A zero-test report cannot replace a nonempty expected listing, but this
feature must not invent a blanket prohibition on a legitimate empty listing
that the existing gate contract accepts.

Publishing multiple packages or retrying one cell must preserve unrelated
observations and their original provenance. The existing same-head receipt
merge requires identical host, scope, and report context, so it is not a
cross-host aggregation solution. Publication must handle concurrent writers:
fetch the latest ref, merge nonconflicting observations, and retry a bounded
number of times on a ref conflict. Never force-push over unseen evidence. A
publication failure preserves local proof, reports that remote reuse is
unavailable, and leaves uncovered hosted cells scheduled. Decide the storage
and conflicting-result rules in Open Questions before implementing this path.

Hosted producers publish proof as each cell completes, before area-wide or
workflow-wide success is known. Use existing trusted workflow identities and
permissions; PR code must not gain a new write credential. Preserve the
exclusive `checks: write` ownership used for accepted-gap reporting. A canceled
job may leave proof only if its full observation was durably published before
interruption. Missing or expired reports, unavailable refs, and verification
errors cause execution of the affected cells with a visible reason. They do
not invalidate another cell's still-verifiable proof or manufacture a green
baseline. Reuse promises apply while the required evidence remains available;
this feature does not require unlimited artifact retention.

## Implementation surfaces and drift prevention

Update together:

- `.github/ci/environments.json`, `.github/workflows/ci.yml`, relevant reusable
  workflows, and `.github/ci/README.md`.
- `scripts/ci/affected_scope.py`, `reuse_validation.py`, `local_evidence.py`,
  and their Python contract suites as required by the behavior above.
- `scripts/cross-check.sh`, its tests, and the required seeding Just interface;
  add risk-directed hook integration if the optional workflow is implemented.
  Keep terminal/browser tests from gaining focus.
- `docs/topics/ci-cd.md`, `AGENTS.md`, and applicable skills, including
  rust-devops and os topic pages. Search all active docs and skills for cadence
  statements, not just those two entrypoints. Historical specs remain snapshots.

Write decided-but-unbuilt behavior as **planned** in the current topic page.
Remove that marker when code lands. Add a focused contract check that detects
conflicting active cadence guidance; avoid brittle assertions over every prose
sentence or historical measurement. Correct stale performance guidance and
identify measurements taken under the old cadence instead of inventing new
duration estimates. Follow the document-version rules if a contract changes.

## Acceptance criteria

- Guidance explicitly distinguishes merge eligibility from release eligibility:
  native Windows/WSL2 passes are not required to reach `main`, but a release
  requires qualifying passing evidence across all four supported environments
  for its release scope. Release enforcement design is deferred, not claimed
  implemented by this feature.
- Ordinary PR and push plans have no native Windows or WSL2 execution cells.
  A PR-proven push has no unnecessary test/build/preflight work.
- After migration, scheduled plans contain Windows and WSL2 cells only for
  affected scope, plus any newly required coverage under current policy;
  Linux appears only as a demanded WSL2 archive producer. Explicit opt-ins
  retain their documented behavior.
- Successful unchanged nightlies with unchanged required coverage schedule
  no package work. Failed nightlies
  preserve pending coverage. Missing baseline and API failure are diagnosed;
  the cadence migration cannot treat WSL2 history as Windows proof.
- Existing nightly failures and bootstrap status are documented during
  implementation, with a concrete recovery path and no fabricated evidence.
- A resumable full-workspace rig pass can seed all gated packages separately
  for native Windows and WSL2. Interrupted/resumed passes preserve prior
  completed cells and never overwrite another package's receipts. A later
  head reuses unchanged inputs and reruns changed inputs only.
- With no nightly baseline, a fully seeded plan schedules no package builds
  or tests and can establish its baseline through the normal nightly audit.
  Partial seeds execute exactly the remaining cells and demanded producers.
  Tests cover all-zero bases, boundary receipts, changed inputs, missing rigs,
  partial reports, sequential packages, and newer qualifying failures.
- The three nightly examples above are regression cases. A failed/canceled
  nightly preserves both its prior successful baseline and independently
  published completed cells, including hosted cells; retry does not reexecute
  those cells when their evidence still qualifies.
- Strict selected-rig failures block the push; explicit warn mode publishes
  honest mixed results and permits it. Tests prove per-cell pass reuse,
  pending failed cells, newer-failure precedence, later-pass recovery, and no
  accidental hosted Windows PR/push grid or baseline rollback.
- Python tests cover the shipped cadence, empty plans, scoped diffs,
  producer demand, baseline migration, and failure paths, including a WSL2-only
  success followed by an unchanged head, expired gaps, newly required gates,
  and the bounded wait for matching unfinished PR validation. Relevant workflow
  contract suites and actionlint pass. Validate locally; do not dispatch a
  speculative full-workspace hosted run to learn scheduling behavior.
- All active instructions, skills, and CI docs agree with implemented policy;
  planned behavior is clearly labeled until implementation.
- Evidence-publication fixtures cover concurrent ref updates, publication
  failures, artifact expiry, mixed host/run provenance, same-commit retries,
  and unordered conflicting outcomes under the author's selected design.
  No later audit credits a prior completion record as its own execution.
- End-to-end fixtures prove native
  Windows receipt publication, report/completion checks, merge-to-nightly
  discovery, selective reuse, sequential package receipt preservation, and
  fallback on missing/invalid evidence. Only unproven cells execute, and a
  newer failure cannot be hidden by older passing evidence.

## Open Questions

### How should rig and hosted observations be stored for reuse?

The current notes contain one receipt per environment/commit, and a receipt's
host and report context apply to all its cells. Required seeding and recovery
must combine independently completed cells without changing who proved them.
The author should choose this before the evidence-publication phase.

- **Versioned observation collection in the existing environment notes
  (recommended).** Keep independent receipts and immutable report references
  together under the existing ref; verify each observation separately.
  Pros: one discovery path, preserves per-run provenance, and fits rig
  publication. Cons: requires a document contract, concurrent-update handling,
  and a defined way to retain and fetch hosted reports. Recommended because
  it extends the current discovery mechanism without pretending multiple
  hosts share one execution context. Follow schema-version rules; do not
  silently change the meaning of a version-2 receipt.
- **Separate rig notes and hosted artifact discovery.** Feed both sets of
  verified observations to the same planner overlay. Pros: hosted reports
  stay in their existing artifact store; fewer note-format changes.
  Cons: two discovery paths, API/pagination work, and artifact expiry must be
  surfaced consistently. The artifact lookup is evidence discovery only,
  never another scope calculator.
- **One receipt per cell under separate refs.** Pros: independent publication
  and retry ownership. Cons: many refs to fetch and manage, and a new naming
  and retention contract. Prefer this only if aggregate-note conflicts prove
  materially problematic; no performance spike is needed to choose the
  simpler existing-ref approach initially.

### What makes one qualifying observation newer than another?

Commit ancestry does not order retries at the same commit or runs on sibling
branches. Host clocks and fetch order are not trustworthy ordering rules.
A concurrent pass must not silently erase a failure for equivalent inputs.
The existing newest-commit-first walk does not settle these cases.

- **Explicit retry lineage with unresolved conflicts requiring execution
  (recommended).** Preserve all qualifying observations. A later complete
  observation may supersede one it explicitly names as its retry; observations
  on later descendant commits follow the existing input-equivalence rule.
  Unordered conflicting pass/failure observations leave the cell unproven.
  Pros: conservative, supports later-pass recovery, and needs no global clock.
  Cons: records need observation identities and retry links; a conflict may
  cause one extra execution. Recommended because an extra cell run is cheaper
  than falsely certifying coverage. Cyclic or invalid links are rejected.
- **Repository-wide publication sequence.** Assign ordering during serialized
  ref updates. Pros: a simple total order for all observations. Cons: receipt
  publication can arrive after testing; an old delayed pass could hide a newer
  test failure unless execution lineage is also recorded.
- **Trusted completion timestamps.** Pros: familiar and easy to display.
  Cons: rig clock drift and concurrent finishes make precedence ambiguous;
  requires a clock trust policy and tie handling. Do not use local timestamps
  alone to satisfy the newer-failure rule.

### Which command should expose workspace seeding?

The first delivery excludes the optional risk-selection interface. Seeding
still needs a preview, a pinned revision, persistent progress, and an honest
nonzero exit when requested coverage remains unproved. Governed gaps are
reported separately and evaluated under the existing planner policy.

- **Dedicated Just recipe backed by the existing cross-check transport
  (recommended).** Pros: explicit full-workspace operation with resumable
  state, while preserving existing per-package commands. Cons: a small
  orchestration entry point is required. Recommended because a potentially
  long workspace seed is a different user operation from checking one package.
- **Extend `just cross-check` with a workspace mode.** Pros: one command family
  and existing host selection. Cons: broadens the current per-package
  interface and complicates resume and preview behavior.

Local resume state must be bound to repository identity, pinned revision,
policy, and cell contracts. It is a progress cache, never proof by itself;
published observations must still pass verification. Use existing rig locks,
noninteractive failure reporting, and terminal/browser tests that never take
focus. Recover actual red cells with targeted local runs; do not add unrelated
hosted cells, benchmark workloads, or repeated statistical measurements.
