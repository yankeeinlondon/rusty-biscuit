---
kind: feature
name: windows-to-nightly
date: 2026-10-08
status: draft-spec
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

This is a specification, not an implemented change. Cadence correction,
test-rig seeding, receipt publication/consumption, and regression checks are
required. Only the risk-directed pre-push workflow is optional.

## Verified current behavior

Inspected on 2026-10-08 in this worktree and through the repository's GitHub API:

| Surface | Current behavior |
| --- | --- |
| `.github/ci/environments.json` | Linux/macOS on PR and push; Windows on push; WSL2 on schedule; all four on manual dispatch. |
| `scripts/ci/affected_scope.py` | Derives event coverage from that table and produces one canonical resolved plan. |
| `scripts/ci/test_affected_scope.py` | Explicitly asserts Windows-on-push and WSL2-only nightlies, including a reused PR leaving Windows work on the subsequent push. |
| `scripts/ci/reuse_validation.py::nightly_base` | Searches up to 20 successful scheduled `ci.yml` runs on `main`, newest first, and selects the first whose head is an ancestor of the current head. No qualifying run, or an API failure, returns no base. |
| `.github/workflows/ci.yml` | Diffs the selected base against the current head; no base means `--all`. Scheduled runs bypass local evidence verification and overlay. |
| `scripts/cross-check.sh` | Runs native Windows via `BUILD_WIN`, but publishes validation receipts only for WSL2. The Python cross-check recorder already accepts an environment argument; the shell publication path is WSL2-specific. |
| Skills and instructions | `AGENTS.md`, CI docs, and the rust-devops skill describe Windows-on-push. The rust-devops skill additionally has stale prose claiming nightly Linux/Windows full-workspace execution. |

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

GitHub's successful scheduled-run query returned `total_count: 0`. The
[latest inspected nightly](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/37800062455)
used an all-zero base and recorded an explicit full-scope inventory: 66
executing WSL2 cells, 17 omitted accepted-gap cells, and 68 distinct packages.
The eight latest scheduled runs were failures, with several sharing the same
head. Thus the desired incremental mechanism exists, but has not obtained a
successful baseline in the available hosted history and currently repeats
full-workspace attempts. Do not present that as efficient incremental coverage.

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
   check/lint environment policy and the policy-free `ci-gate`.
4. Preserve capability gaps as explicit accepted gaps with their existing
   governance. Windows and WSL2 evidence remain distinct.

## Merge eligibility and release eligibility

Passing every native Windows or WSL2 test is **not a requirement for merging
to `main`**. Ordinary merge eligibility follows the Linux/macOS PR gates;
Windows and WSL2 coverage may be gathered after merge through nightly runs.
Default strict pre-push validation still blocks a selected local/rig failure;
that hook behavior is distinct from a repository-wide Windows/WSL2 merge
requirement and retains its explicit warn-mode exception.

**No release may proceed without qualifying passing evidence from all
supported environments for the release scope:** macOS, Linux, native Windows,
and WSL2. Evidence must apply to the release's tested inputs and cover the
packages and gates required by that release scope. A merge to `main`, green
Linux/macOS PR checks, a successful nightly baseline, or passing evidence for
unrelated packages does not independently establish release eligibility.
Native Windows and WSL2 remain separate evidence requirements.

This section states the release policy only. Defining release scope, evidence
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
   No new change since a successful nightly means no
   package test cells, builds, or package preflight runners.
2. Do not silently discard pending coverage to make a nightly smaller. When
   no baseline exists, or baseline discovery fails, report the precise reason
   for full scope. Distinguish a genuine bootstrap from an API failure or an
   exhausted bounded search. Recover the existing failing nightly and obtain
   truthful coverage; never manufacture a successful baseline.
3. An old WSL2-only successful nightly cannot prove newly nightly Windows
   coverage. Define and test the migration baseline for Windows explicitly,
   using qualifying Windows evidence where available and bootstrapping missing
   cells where necessary. Do not force already-proven WSL2 cells to rerun merely
   to initialize Windows coverage. Preserve one canonical plan even if this
   requires distinct environment coverage anchors inside it.
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
   backend proof where required. A compile check, zero-test pass, WSL2 run,
   dirty mismatched tree, interruption, or unsupported flags do not certify
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
- Scheduled plans contain Windows and WSL2 cells only for affected scope;
  Linux appears only as a demanded WSL2 archive producer. Explicit opt-ins
  retain their documented behavior.
- Successful unchanged nightlies schedule no package work. Failed nightlies
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
  producer demand, baseline migration, and failure paths. Relevant workflow
  contract suites and actionlint pass. Validate locally; do not dispatch a
  speculative full-workspace hosted run to learn scheduling behavior.
- All active instructions, skills, and CI docs agree with implemented policy;
  planned behavior is clearly labeled until implementation.
- End-to-end fixtures prove native
  Windows receipt publication, report/completion checks, merge-to-nightly
  discovery, selective reuse, sequential package receipt preservation, and
  fallback on missing/invalid evidence. Only unproven cells execute, and a
  newer failure cannot be hidden by older passing evidence.

## Open implementation decisions

- Whether to include the optional risk-directed rig workflow in the first
  delivery. Workspace seeding and its receipt publication/consumption are
  required either way.
- Which existing command surface should expose resumable workspace seeding,
  and how to recover the currently failing cells on the rigs.
- Which explicit risk-selection interface fits the existing hook and
  cross-check workflow. Avoid an automatic heuristic or new configuration
  system unless an actual need justifies it.
