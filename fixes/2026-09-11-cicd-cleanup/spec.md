---
created: 2026-09-11
status: proposed
implemented: false
reviewed: true
reviewed_by: claude/fable
reviewed_on: 2026-09-11
review_iterations: 3
area: repository-ci
depends-on:
  - fixes/2026-09-10-local-affected-scope/spec.md
---

# CI Cleanup: Impacted Areas, Reused Results, and Area-Owned Outcomes

## Objective

A viewer of a CI run must be able to identify the impacted package areas being
tested and inspect a complete environment matrix for each area. Verified local
results appear in that matrix without executing those tests again in CI. CI
runs only the remaining authorized cells. Each area owns its outcome; a
separate global verdict job must not reinterpret successful area results.

Only areas selected for testing receive functional tests, compile checks, and
applicable lint gates. Other areas receive no package jobs, including
compile-only jobs for unchanged reverse dependencies.

## Relationship to Prior Specifications

> **Reader's note (review, 2026-09-11).** Three earlier decisions touch the
> same surfaces. This section states which of them this specification
> inherits, which it deliberately reverses, and what each reversal costs, so
> that an implementer does not treat a reversal as an accident or an
> inheritance as open for renegotiation.

**Inherited from `fixes/2026-09-10-local-affected-scope/spec.md`** (the
`depends-on` above; its items are assumed complete before this work starts):

- scope evidence and validation evidence are distinct versioned documents;
- validation evidence is keyed per `{package, environment, gate or tier}`
  cell and carries outcome, exit code, duration, completion, and provenance;
- a complete failed local run is reusable evidence; interrupted, partial, or
  dirty-tree runs are not;
- a matching local scope document is authoritative in CI, which then computes
  scope itself only on a miss;
- `scope-only` is the intentional no-test hook mode.

Where that specification says results feed "the normal final verdict", read
"the owning area's outcome" once section 5 below is implemented.

**Superseded by this specification:**

- the 2026-09-10 rule that an unchanged direct reverse dependency receives a
  compile-check job (section 1.4 and Open Question 1);
- the one-environment receipt: `verified_environment()` returning the first
  match and the planner's single `--exclude-environment` (section 3);
- `ci-verdict` as the single required check (section 5).

**Intended reversal of `fixes/2026-08-06-cicd/spec.md`.** That specification
retired the package area as a CI concept: `.github/ci/README.md` still states
that "there is no concept of a package area in CI", and every artifact name,
JUnit manifest record, baseline entry, and rollup cell is keyed on
`{package, environment, tier}`. This specification reintroduces the area
**only as the presentation and outcome-ownership unit**. It does not re-key
evidence. Side effects and their mitigation:

- Package remains the identity of every artifact, manifest record, baseline
  entry, and receipt cell. Area is derived from package at rollup and
  presentation time, never stored as an identity. Re-keying the baseline or
  artifacts is out of scope and would reopen the 2026-08-06 migration.
- The README sentence quoted above becomes wrong and must be rewritten to
  distinguish identity (package) from grouping (area).
- The pre-push hook's `RUSTY_BISCUIT_PRE_PUSH_AREAS` override and the root
  justfile's `areas :=` convenience list already use area names; they remain
  local conveniences and are not the canonical mapping (see Scope and
  Terminology).

**Intended reversal of the 2026-09-09 ruling that `ci-verdict` is the
permanent single required check.** The reasons for that ruling are real and
are addressed in section 5 and Open Question 3: required checks match static
names, so a dynamically selected producer cannot be a required check, and a
required producer would bypass baseline and policy-gap acceptance. Removing
the job without replacing what those two properties protect is forbidden by
this specification.

## Observed Problems

PR [#76](https://github.com/yankeeinlondon/rusty-biscuit/pull/76), including
[CI run 34638047631](https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/34638047631),
exposed the following failures in the current contract:

- Unchanged dependent packages such as DMLS appeared as green top-level entries
  after only a Windows compile check. No functional tests ran for those entries.
- Top-level entries represented individual Cargo packages rather than package
  areas, separating libraries and CLIs that belong to the same area.
- Unscheduled jobs appeared as skipped placeholders with unresolved labels such
  as `${{ inputs.package }}` and `${{ matrix.environment }}`. This is a known
  GitHub behavior already recorded in `ci.yml`: when a matrix job is skipped
  through `needs:` or `if:`, the matrix context is never evaluated and the
  whole matrix collapses into one skipped job named with the raw expression
  (measured on run 30323254931).
- The macOS receipt suppressed hosted execution but did not inject local results
  into the visible results matrix.
- All executed test jobs passed, but `ci-verdict` failed because it classified
  six macOS L1 cells and one macOS L2 cell as missing. Those cells had passed
  locally. The Windows and WSL L2 policy gaps were accepted notes, not the
  blocking reason for that verdict.
- The current verifier returns the first matching environment receipt, and the
  scope calculator accepts one excluded environment. Evidence from multiple
  environments cannot be combined.
- A request not to rerun WSL was incorrectly treated as applying only to manual
  reruns. A subsequent push automatically scheduled WSL anyway.
- Documentation still prescribes `ci-verdict` as the sole required merge check,
  conflicting with the requested removal. It also incorrectly claimed that
  `--no-verify` prevents reuse of already-published valid receipts. (The
  `--no-verify` wording was corrected on 2026-09-11 in `.github/ci/README.md`,
  `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/os/SKILL.md`, and
  `CLAUDE.md`; the `ci-verdict` prescription remains and is this
  specification's to remove.)

The successful macOS validation of `0156096ff` took 160.696 seconds for 28 gates,
including 7,546 tests. Claudine L2 passed 248 tests with 14 workers in 35.494
seconds. These are recorded observations of the existing run, not performance
thresholds for this change.

## Scope and Terminology

- **Area:** the repository's package-area identity, such as `claudine` or
  `playa`. The canonical mapping is the one `sniff repo package-areas` reports:
  a workspace member's area is derived from its manifest directory relative to
  the repository root, and nested areas such as `claudine/rendezvous`,
  `darkmatter/dmls`, and `homelab/server` are **distinct areas**, not children
  of their parent. An area may contain more than a library/CLI pair. Do not
  guess an area from a crate-name prefix.
- **Package:** a Cargo workspace member within an area. Package-level feature,
  native dependency, tier, and backend declarations remain authoritative, and
  package remains the identity of every stored result.
- **Cell:** an area's result for an environment and gate/tier, with package-level
  contributions available underneath it.
- **Environment:** macOS, Linux, native Windows, or WSL2. WSL2 remains distinct
  from native Linux and native Windows for execution and evidence.
- **Selected area:** an impacted area selected by the canonical scope planner
  for testing. Merely depending on changed code does not independently select
  an unchanged area for a compile-only job.
- **Accepted policy gap:** a cell whose environment cannot host its tier, where
  the inability is governed in `.github/ci/environments.json` by an owner,
  reason, and unexpired expiry. The two open features
  `features/_unscheduled/windows-l2-ci-leg` and
  `features/_unscheduled/wsl2-l2-ci-leg` are the work that closes the current
  gaps; per Ken's 2026-09-08 ruling the gaps are temporary and no acceptance
  criterion is amended to exclude those environments.

**Area mapping in CI.** The scope job runs on a hosted runner with Python and
no `sniff` binary. The planner therefore replicates sniff's directory rule in
`scripts/ci/affected_scope.py` and emits an `area` field on every matrix and
policy record. A contract test compares that mapping against sniff on hosts
that have it, namely the local `ci-local` self-test and the preflight leg:
sniff exposes a package's area only per directory (`sniff repo package-area`
run from each workspace member's manifest directory; the aggregate JSON and
`sniff repo packages` list names only), and `sniff repo package-areas` gives
the area universe. If that per-directory loop proves too slow for preflight,
add a per-package area field to sniff's aggregate JSON rather than weakening
the test. A committed mapping file is rejected: it would be a second policy
store.

This specification changes selection, scheduling, evidence, reporting, and
merge-check ownership together. Renaming existing jobs alone is insufficient.
It does not authorize a full-workspace validation run, removal of platform
support, or bypassing real failures to make a run green.

## Required Behavior

### 1. Select and Execute Only Impacted Test Areas

1. Compute scope once from the event's actual changes and canonical package-area
   mapping. Local validation and hosted CI consume the same resolved plan (the
   2026-09-10 scope-evidence contract).
2. Select the impacted test areas and their applicable package targets, preserving
   declared features, test tiers, native dependencies, and backend requirements.
   The plan must state why each area and package target was selected.
3. Selected areas receive functional tests and compile checks on the required
   environments, with valid reused results satisfying their corresponding cells.
   Applicable lint gates remain included and identify their execution environment.
4. Remove compile-only selection and top-level entries for unchanged reverse
   dependent areas. Ordinary dependency compilation needed to build a selected
   package remains normal build work, not another selected area or result cell.
   Whether the downstream seam (does the change still compile its unchanged
   consumers?) is checked at all, and where, is Open Question 1; the
   presentation rule here holds under every option there.
5. Remove the hard-coded Windows-only compile-check policy. Today this is the
   planner constant `CHECK_OS = ["windows-latest"]` plus the rule that a
   Windows receipt drops reverse dependencies entirely
   (`scheduled_reverse_ids`); both go. Linux and Windows compile coverage must
   remain when macOS coverage comes from local evidence. Respect WSL's archive
   execution contract rather than adding compilation inside its toolchain-free
   guest: WSL compile coverage is the `ubuntu-latest` archive build, reported as
   such.
6. Do not use blanket `--all-targets` as the default check contract. Select the
   relevant library, binary, test, example, or benchmark targets explicitly when
   their coverage is required. `--all-targets` does not mean all packages or all
   operating systems, but its target breadth still requires a concrete reason.
   The one concrete reason on record is that benches and examples compile in
   the check job and nowhere else; if that coverage is kept, say so per target
   kind rather than by the blanket flag.
7. Avoid a redundant compile step when an executed gate demonstrably covers the
   same target, features, and environment. An L1 job already compiles the
   library and test targets for its environment and feature set; a separate
   check on that environment is justified only for target kinds the L1 build
   does not produce. Report where compile coverage came from; do not equate a
   compile success with functional-test success.
8. Infrastructure and documentation changes run their applicable contract checks
   without making unrelated Cargo areas appear as tested areas. Explicit full
   runs remain a separate, intentional operation. The 2026-09-09 per-gate
   global-input rule (a global input widens only the gates it can change) is
   retained unchanged.

### 2. Present an Area Matrix

The normal package portion of the CI run shows one top-level entry per selected
area. Packages, target details, and build plumbing are subordinate to that area.
Necessary infrastructure checks may remain, clearly identified as infrastructure.

Each area's matrix must expose:

| Field | Required information |
|---|---|
| Environment | macOS, Linux, Windows, or WSL2 |
| Gate/tier | Compile, lint, L1, L2, or another applicable declared suite |
| State | Pending, running, passed, failed, or accepted-policy-gap cancellation |
| Origin | Local validation, prior verified validation, or current CI execution |
| Evidence | Validated revision/input identity and a link to the report or logs |
| Measurements | Recorded test counts and execution duration where applicable |

- A locally satisfied cell remains visible and completed; it is not removed or
  represented as missing, skipped, or newly executed in CI.
- A valid receipt requires no test runner for that cell. Publishing its result
  may take CI bootstrap/API time, but must not rebuild or rerun its tests.
- No unscheduled browser, L2, or other gate appears as a placeholder job with
  an unresolved label. Accepted policy gaps are the deliberate visible
  exception described below.
- No reader-facing label exposes unexpanded workflow expressions.
- Lint and compile labels identify their environment. Build/archive steps do not
  masquerade as functional-test results.
- Area rollups retain enough package detail to locate a failure without making
  each package a separate top-level area.

**Evidence links for local-origin cells (decision).** CI cannot link to a file
on a developer host. A local-origin cell's evidence link is the Git notes ref
that carried it (`refs/notes/ci-local/<environment>` at the tested head),
rendered in the area's step summary together with the counts, duration, and
host identity the receipt records. The full local report stays on the
producing host under `~/.rusty-biscuit/ci-evidence/` (the convention PR #74
established), and the receipt records that path so a reviewer can ask for it.
Uploading local reports into the run is rejected: the host has no run to
upload into at hook time, and a later upload would be unverifiable.

**GitHub presentation facts the design must respect** (verified in the
controlled fixture of Validation step 2 before the workflow is restructured):

- A job skipped through `if:` still appears in the run graph as skipped. The
  achievable target is therefore "no skipped placeholder carries an unresolved
  label", reached by hoisting tier selection into planner-generated top-level
  matrices so that only real work becomes a job, and by giving any whole-stage
  skip a static, human-readable name. This is why acceptance criterion 10 is
  worded as it is.
- Reusable workflows nest at most four levels deep including the caller. An
  area-level reusable workflow calling the package-level one, which calls the
  WSL one, is three levels and fits; adding a fourth is not.
- A called reusable workflow's jobs render as `<caller job name> / <called job
  name>`, which is how the area becomes the top-level identity without any
  display-name parsing.

### 3. Reuse Verified Evidence Across Multiple Environments

Replace the single-excluded-environment model with evidence resolved per cell.
Local macOS and prior WSL evidence must be usable together when both qualify.

The evidence contract must retain:

- area, package, environment, gate/tier, feature and relevant target selection;
- tested revision/tree and the scope/input identity used to establish validity;
- actual outcome, counts, start/end or elapsed duration, and report provenance;
- applicable backend execution proof and schema/toolchain/policy compatibility.

Requirements:

1. The scheduler, area outcome, and summary consume one verified result model.
   Evidence accepted for scheduling must also satisfy the corresponding result
   cell. Recalculating a conflicting expectation in the reporting path is forbidden.
   Concretely: `local_evidence.py verify` returns the full set of accepted
   cells across all environments, the planner accepts that set (not an
   environment name) and omits exactly those cells, and the same set is written
   into the scope/policy artifact so the area rollup expects a local-origin
   result for each of them rather than a CI-origin one.
2. Valid results from multiple environments combine without overwriting one
   another. Mixed local and CI results contribute to the same area matrix.
3. Older results are not reusable merely because a run was recent. Either their
   existing exact identity matches, or a defined and tested input-equivalence
   rule proves the relevant execution inputs unchanged. Never restamp old
   results as if they tested a new tree.

   **Input-equivalence rule (decision).** A cell's *gate-input identity* is the
   hash of the `git ls-tree` entries for (a) the manifest directories of every
   package in the tested package's build closure (dev-dependencies included,
   as `build_closure` already computes) and (b) the global inputs the planner
   already assigns to that gate under the 2026-09-09 per-gate rule
   (`Cargo.lock`, `rust-toolchain.toml`, `.cargo/`, `.config/nextest.toml`,
   the workflow files, `environments.json`, and the planner itself). Two
   heads with equal gate-input identity for a cell may share that cell's
   result. The identity is stored in the receipt at record time, so
   verification is a comparison, not a recomputation over an old tree. This
   reuses the existing per-gate input classification instead of inventing a
   second one; if that classification is later found incomplete, both the
   planner and this rule are fixed in the same change.
4. Missing, malformed, incompatible, stale, interrupted, or insufficient evidence
   cannot become a passing result. Report the specific reason it was rejected.
5. A complete failed local result remains a failed result. Reuse must not turn
   failure into success, conceal it, or prevent independent environments from
   reporting their own results.
6. Define migration behavior for existing receipts, which currently lack detailed
   outcomes, counts, and duration. **Decision:** `schema_version: 1` notes are
   accepted only on exact tree identity (never through the equivalence rule),
   only as pass-only whole-environment evidence exactly as today, and are
   rendered with measurements marked "not recorded (v1 receipt)". They are
   never upgraded in place and expire naturally because they bind to exact
   heads. Never invent measurements or silently rerun tests to fill a legacy
   receipt's gaps.
7. `--no-verify` produces no new hook evidence, but does not invalidate receipts
   already published and verified for the outgoing work.
8. **Cross-check output becomes a receipt (decision).** `scripts/cross-check.sh`
   already runs the WSL suite as CI does (archive mode, builder target hidden).
   When the remote clone's tested tree equals the outgoing head's tree and the
   remote worktree is clean, the run publishes a `wsl2-ubuntu` receipt with the
   same schema the hook publishes, so "prior WSL evidence" is an exact-identity
   or gate-input-identity receipt like any other. A cross-check run against a
   dirty or patched tree publishes nothing and says so.

### 4. Honor Execution Constraints Before Triggering CI

A restriction such as "do not rerun WSL" applies to direct commands, automatic
push-triggered jobs, dispatches, and retries. Repush authorization retains that
restriction unless the user explicitly changes it.

Before a push or other workflow trigger, make the resolved execution plan
reviewable and reconcile it with every active environment/tier restriction.
Show which cells are reused, which will execute, and which are accepted gaps.

- Verify all requested exclusions; checking macOS alone does not establish that
  WSL is excluded.
- If reusable evidence is insufficient and an environment is prohibited from
  running, stop before the trigger and explain the unsatisfied constraint.
- Do not automatically convert a user restriction into a policy-gap acceptance,
  silently rerun the environment, or fabricate evidence to satisfy the plan.
- Carry explicit execution constraints through the supported local/CI scheduling
  interface so this boundary has regression coverage rather than depending only
  on an agent remembering a chat instruction. Where the constraint is persisted
  between sessions is Open Question 2; the interface is `just ci-local --plan`
  (extending today's `--dry-run`), which prints the plan above and exits
  non-zero when a prohibited environment would still be scheduled.
- Constraints are enforced at the trigger boundary only. CI never reads them:
  a constraint without qualifying evidence must block the push, never cause CI
  to silently skip required coverage.
- `ci-infra-retry.yml` reruns only jobs whose hosted runner was lost, on
  attempt 1 only. Such a rerun re-executes a cell the original trigger already
  authorized and is within that authorization. A manual "re-run failed jobs"
  is a new trigger and is subject to the same constraints; the tooling cannot
  prevent a human clicking it, so documentation states this plainly.
- Classify job failures by stage. Passing tests followed by an artifact-upload
  failure do not establish a test regression or justify rerunning the suite.

### 5. Remove the Standalone Verdict Job

Remove `ci-verdict` as a standalone job and merge authority. Do not replace it
with the same global verdict mechanism under a different name.

- Each selected area owns the correctness of its combined local/CI result.
- Real failed or missing required results prevent that area's success.
- A reporting-only summary presents those same area results without applying a
  second, conflicting policy evaluation or becoming another verdict gate.
- Update branch-protection/ruleset requirements, validation-reuse code, workflow
  dependencies, baseline/policy ownership, and documentation together.
- Verify how required checks work for dynamically selected areas. A deselected
  area must not leave an expected check pending, and a failed selected area must
  not be mergeable simply because no static required-check name covers it.
- Failure of CI infrastructure that prevents required coverage must remain
  visible and blocking through the appropriate owning check. Removing the global
  verdict does not authorize merging incomplete validation.
- Preserve detailed machine-readable results where they support reporting and
  future reuse; the removal concerns the redundant global decision job.

**What "area-owned outcome" means mechanically (decision).** Each selected
area gets one rollup job of its own (`if: always()`, `needs:` that area's
producers) that: reads the area's produced cells plus the local-origin cells
the plan assigned to it; applies the baseline, governed policy gaps, and the
missing-cell rule **for that area only**; writes the area's slice of the
machine-readable results; and fails when any required cell of that area
failed, is missing, or is an unexcused gap. This is `ci-rollup rollup` and
`ci-rollup verdict` run per area with `--scope` narrowed to the area, not a
new tool. The merge gate is then whatever mechanism Open Question 3 selects;
no job re-evaluates policy across areas.

**Migration checklist for removing the job** (all in the same change, because
each item alone leaves either every PR blocked or nothing blocked):

- ruleset `protect-your-bacon` (id 19747338): replace the `ci-verdict`
  required-status-check context with the selected mechanism; a stale required
  context leaves every PR "Expected — waiting for status" forever;
- the admin `pull_request` bypass actor on that ruleset is a separate open
  item for Ken and is neither removed nor relied upon by this specification;
- `scripts/ci/runner_loss.py attribute` and the `ci-results` artifact move
  into the per-area rollup jobs; `ci-infra-retry.yml`'s `decide` path keeps
  working because it reads job-level check-run annotations, not the verdict;
- `scripts/ci/reuse_validation.py` and `release-plz.yml` both key on the `ci`
  workflow run's conclusion being `success`, which after this change is the
  conjunction of every job. Advisory jobs must therefore never fail, and
  policy-gap presentation must not alter the run conclusion (section 6);
- `pr-health.yml`, `_package-ci.yml`, and `_wsl-ci.yml` comments,
  `.github/ci/README.md` ("the single required check" section and the
  "no package area" sentence), `ci-baseline.toml`'s header,
  `.claude/skills/rust-devops/ci-cd.md`, `.claude/skills/os/SKILL.md`, and the
  memory note recording the 2026-09-09 ruling.

### 6. Represent Accepted Policy Gaps Explicitly

An applicable, accepted policy gap appears immediately in the area's environment
matrix as a **cancelled** cell, without starting a test or archive build.

Its visible description/details must provide:

- the reason and affected environment, packages, and gate/tier;
- who owns/accepted the gap and its expiration;
- a direct link to the policy entry;
- instructions for changing or revoking acceptance and the coverage needed to
  close the gap (for the current gaps, the two `_unscheduled` L2-leg features).

An accepted gap counts as neither a pass nor a test failure. Area evaluation
recognizes its explicit, unexpired acceptance. An absent, expired, or revoked
acceptance cannot silently excuse missing required coverage and blocks the area.
An ordinary user cancellation, runner cancellation, or interrupted test is not
an accepted policy gap.

Validate GitHub's native job/check capabilities before choosing the mechanism:
synthetic cancelled results, descriptions/details links, required-check behavior,
and whether cancellation propagates to the parent workflow must be proven. Do
not cancel the whole workflow to produce this presentation or silently substitute
an ambiguous skipped placeholder. If GitHub cannot express the requested UI,
present the verified limitation and a concrete alternative before adopting it.

**Known constraints going in** (to be confirmed by the fixture, not assumed):

- A GitHub Actions *job* can only conclude `cancelled` by being cancelled,
  which is forbidden above. A *check run* created through the Checks API
  (`checks: write`, GITHUB_TOKEN) may be created directly with a conclusion of
  `cancelled`, `neutral`, or `skipped`, with a title, summary, Markdown text,
  and a `details_url`. Such check runs appear on the commit and in the PR
  Checks tab, and do not change the workflow run's conclusion.
- The existing contract treats a cancelled scheduled cell as blocking
  interruption (`ci-baseline.toml` rules; `runner_loss.py`). Whatever
  conclusion is chosen, the machine-readable state must be a distinct
  `ACCEPTED GAP` value carried in results, never inferred from GitHub's
  conclusion, so retry and rollup logic cannot mistake it for an interruption.
- The choice between `cancelled` (as requested) and `neutral` is Open
  Question 4.

### 7. Preserve the Agreed Concurrency Policy

- Local runs: `max(1, logical cores - 2)` workers.
- CI with four or fewer logical cores: use all available logical cores.
- CI with more than four logical cores: `logical cores - 2` workers.
- Keep required shared-resource serialization and explicitly documented group
  limits. Do not reintroduce CI caps into local isolated suites accidentally.

The two-slot allowance preserves capacity for other host work; small CI runners
use their full capacity because reserving two would discard half or more of it.
This controls test workers, not CPU affinity or a guarantee of reserved cores.

This policy already exists (commit `0156096ff`: the `_test_threads` recipe in
`just/devops.just`, its forwarding in `just/ci-local.just`, and
`scripts/ci/test_ci_local.py`). The requirement here is preservation under
the scheduling redesign, not new behavior.

## Design Decisions

Decisions made during review that are recorded here so they are not re-derived
during implementation. Each is reversible by editing this section before work
starts.

1. **Package is identity, area is grouping.** No stored result, artifact name,
   baseline entry, or receipt is re-keyed by area. Area is a derived field.
2. **Nested areas are distinct areas.** `claudine/rendezvous` is not rolled
   into `claudine`. This matches sniff's report and each nested area's own
   justfile and canonical recipes.
3. **Area mapping is derived, not committed.** The planner replicates sniff's
   directory rule; a drift test against sniff guards it. No mapping file.
4. **Gate-input identity is the equivalence rule** (section 3.3), built from
   the existing per-gate global-input classification and the existing build
   closure. One classification, two consumers.
5. **Version-1 receipts are exact-identity, pass-only, unmeasured** (section
   3.6). No in-place upgrade.
6. **Local-origin evidence links to the notes ref**, not to a file (section
   2). Reports stay under `~/.rusty-biscuit/ci-evidence/`.
7. **Cross-check publishes receipts** when its tested tree equals the outgoing
   head tree (section 3.8). Otherwise it publishes nothing.
8. **Constraints are enforced at the trigger boundary only** (section 4). CI
   never reads them.
9. **Per-area rollup jobs apply policy for their area** (section 5). The
   existing `ci-rollup` binary is reused with a narrowed scope; no new verdict
   tool.
10. **Accepted gaps are a distinct machine-readable state** regardless of the
    GitHub conclusion used to display them (section 6).
11. **Presentation fixtures run outside this repository's rulesets.** Job and
    check-run presentation is validated on a throwaway branch; required-check
    and ruleset semantics are validated in a scratch repository under the same
    account, because ruleset edits on this repository affect every open PR.

## Open Questions

Each question lists the options considered, and a recommendation with the
reason for it. Implementation must not start on the affected section until
Ken rules; unaffected sections may proceed.

### OQ1 — Where does the downstream seam get compiled, if anywhere?

Section 1.4 removes the compile-only job for unchanged direct reverse
dependencies. PR #75 introduced that job for a reason: a change to a library's
public API that breaks an unchanged consumer (a darkmatter change breaking
DMLS) is otherwise invisible until a `workflow_dispatch` full run or the
weekly `rust-latest-stable.yml`, because a push to `main` computes scope from
the same diff and selects the same packages. Removing the job removes the only
seam check.

- **Option A — remove the seam check entirely** (the specification as
  drafted). Pros: simplest; no unchanged area ever appears; smallest matrix.
  Cons: API breakage in unchanged consumers reaches `main` unseen; the first
  signal is a red full-scope run or a broken developer checkout days later.
- **Option B — compile direct reverse dependencies as a step inside the
  changed package's own check job**, on `ubuntu-latest` only, reported in that
  cell's details as "also compiled N dependents". Pros: keeps the seam check;
  the unchanged area gets no job, no entry, no cell, satisfying the objective;
  failure lands on the area that caused it, which is where a reviewer looks;
  cost is one `cargo check` of a warm dependency graph. Cons: the check job's
  duration grows with dependent count for hub crates (renderable,
  biscuit-terminal); the cell's compile evidence now spans packages outside
  the area, which the report must label honestly.
- **Option C — compile direct reverse dependencies in a single infrastructure
  job** ("dependent seams"), listed under infrastructure, not under any area.
  Pros: seam kept; hub-crate cost isolated from the changed area's timing.
  Cons: a failure is attributed to nobody's area, which is exactly the
  ownership problem section 5 is removing; one more top-level entry.

**Recommendation: Option B.** It preserves the only seam coverage the repo
has while meeting every presentation rule in sections 1 and 2, and it puts
the failure on the area whose change caused it. The hub-crate cost is
bounded (a check, not a test run) and measurable in the first real run; if it
proves material, the step can be limited to a declared dependent list in
`[package.metadata.ci]` without changing the presentation.

### OQ2 — Where is an execution constraint persisted between sessions?

Section 4 requires constraints such as "do not rerun WSL" to survive beyond a
chat instruction and to be checked by `just ci-local --plan` and the pre-push
hook.

- **Option A — environment variable** (`RUSTY_BISCUIT_CI_FORBID=wsl2-ubuntu`).
  Pros: trivial; already the hook's configuration style
  (`RUSTY_BISCUIT_PRE_PUSH`). Cons: dies with the shell; a second agent
  session or a later push from a fresh terminal does not see it, which is the
  failure PR #76 exhibited.
- **Option B — per-branch constraint file on the host**
  (`~/.rusty-biscuit/ci-constraints/<repo>/<branch>.json` with environment,
  reason, who set it, and an expiry). Pros: survives sessions and terminals
  on the host that received the instruction; sits beside the evidence
  directory already in use; expiry prevents a forgotten constraint from
  blocking pushes weeks later; `--plan` prints it. Cons: not visible from
  another host; one more file convention to document.
- **Option C — a Git note on the branch tip.** Pros: travels with the branch
  to any host. Cons: notes are per-commit, so every new commit needs the note
  re-attached; it is also pushed to the remote where it means nothing and
  invites the mistake of letting CI read it.

**Recommendation: Option B.** It fixes the observed failure (a later push
from a different session) at the boundary the constraint actually guards,
and the expiry field keeps it from becoming an ambient trap. Cross-host
loss is acceptable: the constraint is about not consuming a specific shared
host's time, and the host that would do the consuming is the one that holds
the file.

### OQ3 — What is the merge gate once `ci-verdict` is gone?

GitHub required status checks match a static check name; a required name that
never reports leaves the PR pending forever; wildcards are unsupported; a
`skipped` or `neutral` conclusion counts as passing for a required check.
Selected areas vary per run, so no per-area job name can be required. Section
5 still requires that a failed selected area blocks and a deselected area does
not hang the PR.

- **Option A — per-area required checks with the full area list published
  every run.** A fixed-name job (or the planner) posts a check run per area
  on every run: `success`/`failure` from the area's rollup for selected
  areas, `skipped` with "not selected" for the rest; every area name is a
  required check. Pros: the PR checks list reads exactly like the area
  matrix; the required set is explicit and reviewable. Cons: the job that
  posts "not selected" for unselected areas is a global fan-in that must know
  every area, which is the shape section 5 is removing; adding or renaming an
  area requires a ruleset edit; the ruleset needs ~30 required contexts.
- **Option B — one fixed-name conjunction job** (`if: always()`, needs every
  area rollup) that applies no policy of its own and fails if any needed job
  failed; it is the single required check. Pros: static name; trivially
  correct; deselected areas are skipped jobs, which the conjunction ignores.
  Cons: it is a global job that decides mergeability, so it must be
  demonstrably policy-free (a pure fold of `needs.*.result`) to honor the
  "not the same mechanism under another name" rule; MISSING detection still
  has to live in the per-area rollups, because a matrix that never created an
  area's job is invisible to a conjunction.
- **Option C — ruleset "Require workflows to pass"** pointing at
  `.github/workflows/ci.yml`. Pros: no gate job at all; GitHub's own fold of
  the run conclusion is the gate; `reuse_validation.py` and `release-plz.yml`
  already key on that same conclusion, so three consumers share one signal.
  Cons: the rule's availability and its behavior for `workflow_dispatch` and
  for reruns must be proven on this repository's plan; any advisory job that
  ever fails becomes a merge blocker, so advisory jobs need `continue-on-error`
  discipline; the whole-run conclusion is coarser to explain in the PR checks
  list than a named check.

**Recommendation: Option C, with Option B as the fallback if the ruleset rule
is unavailable or misbehaves in the scratch-repository fixture.** Option C is
the only one with no global decision job at all, which is the literal
requirement of section 5, and it aligns the merge gate with the two existing
consumers of the run conclusion instead of adding a third signal. MISSING
detection lives in the per-area rollup under either option, so the property
`ci-verdict`'s `--scope` protected is kept.

### OQ4 — `cancelled` or `neutral` for accepted policy gaps?

Section 6 asks for a cancelled cell. The Checks API can produce either
conclusion for a synthetic check run.

- **Option A — `cancelled`, as requested.** Pros: matches the requested UI;
  visually distinct from pass and skip. Cons: collides with the meaning the
  repo already gives cancellation (interruption, blocking); `runner_loss.py`,
  the retry workflow, and the rollup must all be taught the distinct marker
  (Design Decision 10 makes that mandatory anyway); the PR merge box reports
  "some checks were not successful" for a cancelled non-required check,
  which reads as a problem on every PR that touches an L2 package.
- **Option B — `neutral`.** Pros: GitHub's own semantics for "neither pass
  nor fail"; treated as passing for required checks; does not trip the
  merge-box wording. Cons: not the requested UI; grey is easier to overlook
  than the cancelled glyph.
- **Option C — a skipped job with a descriptive static name.** Pros: native
  Actions graph presentation, no Checks API. Cons: a skipped job runs no
  steps, so it cannot carry the owner, expiry, link, and instructions the
  section requires; this is the "ambiguous skipped placeholder" the section
  rejects.

**Recommendation: Option A if the fixture confirms the merge-box wording is
tolerable to Ken, otherwise Option B.** The request was explicit, the
mechanism exists, and Design Decision 10 removes the only correctness risk
(misreading the conclusion as an interruption). The merge-box wording is a
presentation cost only Ken can weigh, so the fixture must show it before the
choice is final.

## Implementation Boundaries

Review and update these existing surfaces as one contract:

- `scripts/ci/affected_scope.py` and its tests: area derivation and drift test,
  targets, the execution plan, removal of `CHECK_OS` and compile-only
  dependent-area selection, and per-cell (not per-environment) exclusion.
- `scripts/ci/local_evidence.py`, its tests, `.githooks/pre-push`, and
  `.githooks/tests/`: multi-environment per-cell evidence, gate-input
  identity, v1 receipt handling, execution restrictions, and pre-trigger
  validation.
- `scripts/cross-check.sh`: receipt publication for exact-tree remote runs.
- `just/ci-local.just` and `just/devops.just`: `--plan`, constraint checking,
  and unchanged `_test_threads` forwarding.
- `.github/workflows/ci.yml`, `_package-ci.yml`, and `_wsl-ci.yml`: area
  grouping, scheduling only real work, visible reused results, per-area rollup
  jobs, and removal of `ci-verdict`.
- `.github/workflows/ci-infra-retry.yml`, `release-plz.yml`, and
  `pr-health.yml`: consumers of the run conclusion and of verdict-era text.
- `scripts/ci-rollup.rs`, `scripts/ci-rollup-tests.rs`, and
  `scripts/ci/runner_loss.py`: one result model, local-origin cells, area
  narrowing, accepted-gap state, and reporting-only aggregation.
- `scripts/ci/reuse_validation.py`: unchanged in logic, re-verified against
  the new run-conclusion semantics.
- `.github/ci/` policy records, `.github/ci/README.md`, and ruleset
  `protect-your-bacon` (19747338): explicit policy-gap ownership and migration
  away from a global verdict.
- Shared Just recipes, `CLAUDE.md`, `.claude/skills/rust-devops/ci-cd.md`,
  `.claude/skills/os/`, and human documentation: matching scope, concurrency,
  evidence, and execution-authorization semantics.

Determine the smallest coherent schema/workflow changes after examining these
surfaces. Avoid parallel planners, duplicate policy stores, or cosmetic job
renaming that preserves the conflicting underlying behavior.

## Acceptance Criteria

1. A fixture changing only Claudine and Playa source selects those test areas.
   Unchanged reverse-dependent areas such as DMLS get no jobs and no top-level
   entry, under every OQ1 option.
2. Each selected area has one top-level identity; applicable package contributions
   appear under it. Compile-only work does not create additional tested areas.
   Nested areas appear as their own entries.
3. Selected areas receive required functional and compile coverage on Linux and
   Windows when macOS is satisfied locally. Relevant-target coverage replaces
   unexplained blanket `--all-targets` checks, and each cell's details say
   which target kinds its compile coverage came from.
4. A valid macOS receipt immediately supplies visible completed cells with actual
   outcome, counts, duration, and provenance. No corresponding test runner starts.
5. Valid macOS and WSL evidence can both be reused in one run; the remaining
   authorized environments execute normally. A WSL receipt published by
   `cross-check` qualifies exactly like a hook-published one.
6. The exact PR #76 regression is covered: excluding seven proven macOS cells
   from execution does not turn any of them into `MISSING` in area results.
7. Stale/malformed/partial evidence is rejected with the specific reason. A
   prohibited environment with rejected evidence blocks the trigger with an
   explanation rather than rerunning. Gate-input identity accepts a receipt
   from an older head only when the tested cell's inputs are byte-identical,
   and rejects it on any change to those inputs (fixtures for both).
8. A complete failed local result and a failed CI result both fail their owning
   area. Independent areas/environments still complete and remain visible.
9. An accepted policy gap creates an immediate, clearly explained cell with
   owner, expiry, policy link, and change instructions, carrying the distinct
   `ACCEPTED GAP` state in machine-readable results. Expired/revoked acceptance
   blocks; unrelated cancellations are not treated as accepted gaps, and an
   accepted gap does not alter the workflow run's conclusion.
10. No unscheduled gate appears as a placeholder job whose name contains a
    literal `${{ ... }}` expression. A stage skipped as a whole may appear once
    under a static, human-readable name. Lint's environment is visible.
11. No standalone `ci-verdict` or renamed global policy job remains. The merge
    gate selected under OQ3 blocks a failed selected area, does not wait on an
    unselected area, and is exercised by the scratch-repository fixture before
    the ruleset is edited. The combined summary applies no policy.
12. Current successful PR-validation reuse continues to work with the new area
    results; failure, cancellation, changed inputs, and missing evidence cannot
    be reused as successful validation.
13. Tests cover local/CI worker boundaries, isolated L2 forwarding, and explicit
    overrides so the scheduling redesign preserves the concurrency correction.
14. Documentation and skills state both actual behavior and its rationale, remove
    obsolete verdict/single-environment/compile-only-area guidance, and clearly
    distinguish any remaining limitations from implemented capabilities.
15. The planner's area mapping matches sniff's `package-area` answer for
    every workspace member's manifest directory, and its area universe matches
    `sniff repo package-areas`, enforced by a test that runs wherever sniff is
    present.
16. A `schema_version: 1` receipt is accepted only on exact tree identity, is
    rendered with unrecorded measurements, and is never accepted through the
    equivalence rule.
17. `just ci-local --plan` shows reused, executing, accepted-gap, and
    prohibited cells, and exits non-zero when a persisted constraint (OQ2)
    would be violated by the resolved plan.

## Validation and Rollout

1. Establish fixtures for scope, area mapping, mixed evidence, execution
   restrictions, policy gaps, and outcome propagation before changing scheduling.
2. Validate the GitHub presentation and dynamic required-check design with a
   minimal controlled fixture: job and check-run presentation on a throwaway
   branch of this repository; required-check and ruleset semantics (OQ3, OQ4)
   in a scratch repository, because ruleset edits here affect every open PR.
   Do not use the workspace's full test matrix just to discover check/job UI
   semantics.
3. Run focused planner, evidence, hook, workflow, and report contract tests, plus
   appropriate Nextest tests for changed Rust tooling. Do not run `cargo fmt`.
4. Prepare a reviewable pre-push plan showing selected areas, per-cell origin,
   actual executions, accepted gaps, and prohibited executions. The plan must
   satisfy the user's active restrictions before any push.
5. Run only authorized impacted-area validation locally, retain its actual
   reports, and publish verified evidence for the outgoing work. No WSL rerun is
   authorized by this specification; reuse qualifying prior evidence or surface
   the gap before triggering CI.
6. Coordinate the workflow and required-check migration so obsolete checks do
   not block every PR and incomplete coverage is not accidentally allowed. The
   ruleset edit and the workflow change that removes `ci-verdict` land in the
   same PR, with the ruleset edited after the PR's own run is green under the
   old gate.
7. Verify the resulting CI UI and machine-readable results end to end. Record
   local validation wall time separately from remaining CI wall time; distinguish
   test execution from setup, builds, queueing, and artifact publication. Never
   attribute the overall local duration to an individual area's cell.
8. Preserve existing valid results during rollout. This specification does not
   itself authorize committing, pushing, or rerunning completed suites.
