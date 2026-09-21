---
kind: feature
name: direct-cell-execution
date: 2026-09-19
status: implemented
supersedes:
  - 2026-09-12-better-cicd-flow
related:
  - 2026-09-11-cicd-cleanup
  - 2026-09-19-hosted-evidence-reuse
  - 2026-09-19-nightly-scope
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
reviewed_by: codex/default
reviewed_on: 2026-09-19
review_iterations: 1
implemented: true
human_review: true
human_review_items:
  - |-
    **Please confirm the "all areas at once, undo by revert" approach, and
    run one real CI trial before this is merged.**

    *Background.* This feature changes how CI decides what to run. The old
    design gave each package a list of operating systems to test on. The new
    design gives each workflow one row per unit of work, taken directly from
    the plan. All nine phases are built. Every local check passes, and
    `acceptance.md` in this feature's folder maps each acceptance criterion
    to the tests that prove it. No hosted CI run has happened yet, because
    none of the implementing sessions was allowed to push.

    *Why decide before merging.* Local tests cannot show the real job names
    on GitHub, the per-cell completion files uploaded by real runners, or the
    area audits reading them. One ordinary pull-request run shows all three.
    If that run is skipped, the first real run happens on `main`, where a
    problem would affect everyone until a revert lands.

    * **A — Accept as built, and merge after one watched CI run.** Push the
      branch, watch one ordinary pull-request run, and compare it with the
      plan. The commands and pass conditions are in `acceptance.md` under
      "What is not proven", item 1. Revert if they disagree.
      *Pros:* matches what is built. Real evidence before merge. One way of
      deciding what runs.
      *Cons:* needs a push and a few hours of CI. This branch has no WSL2
      work to run, so the WSL2 job name and completion file are proven only
      by the next change that has some.
    * **B — Restore a per-area switch first.** Bring back the old
      operating-system lists beside the rows, so areas can move one at a
      time.
      *Pros:* a gradual rollout.
      *Cons:* rebuilds what Phases 5–8 removed. It also keeps two
      descriptions of what CI runs, which can drift apart while both look
      green. That drift is the problem this feature exists to fix.
    * **C — Accept as built and merge without a trial.**
      *Pros:* fastest.
      *Cons:* the first real run is on `main`.

    **Recommendation: A.** It is what is built and tested, and it adds the
    one piece of evidence no local test can give before anyone depends on
    it.
message_to_agent: |-
    All nine phases are implemented; the state is "implementation complete,
    ready for review". Start with `acceptance.md` in this folder, then the
    "## Phase 9" section of `implementation-log.md`.

    1. **No hosted claim has been made.** The Phase 7 trial, S1's label
       questions, and the WSL2 guest's completion record are all unproven on
       GitHub. `acceptance.md` § "What is not proven" has the commands. The
       one unchecked plan box (Phase 7, Wave 3, "One ordinary affected-area
       CI run") is unchecked on purpose. Check it only with a real run's
       evidence.
    2. **S1 question 3 cannot close on this branch.** Both WSL2 cells here
       are accepted gaps, so no WSL2 row runs under any label. It closes on
       the first later change that touches a package with an executing WSL2
       cell. Record the answer in `spikes/s1-labels.md`.
    3. **Phase 9 fixed one pre-existing drift.** The docs said `ci.yml` has
       six top-level jobs; it has eight (`build`, `area-drift`). The fix is
       pinned by `the_ci_documentation_states_the_implemented_behavior`.
    4. **Optional follow-ups, not defects:** retire the redundant
       `execution_path` field (plan schema v5 → v6); the `local_evidence.py`
       version-1 `scope["matrix"]` read; the unreachable `package_cells`
       `KeyError`. See `acceptance.md` § "Known gaps and follow-ups".
    5. **Run the hook suite with `env -u CDPATH`** on the development Mac.
    6. **Do not move this feature to `_completed`** and do not run
       `just complete`; the author does that after review.

    Green on this tree (macOS, 2026-09-20): 13/13 Python suites (905 tests),
    `just _test repo-deps` (445), `just _test test-toolkit` (228), both
    lints, the hook suite (67/67), `actionlint` on the four workflows, and
    `just ci-local --plan` (9 rows).
---

# Direct cell execution: hosted matrices built from the plan's cells

## Why

A cell is one required gate for one package on one environment, identified by
`{package, environment, gate}`. Test gates are L1 (unit tests), L2 (integration
tests), and browser; check and lint are separate gates. Stored results use
`tier` for the same gate dimension. Area groups packages for presentation and
scheduling; it never replaces package identity.

The planner already decides which cells execute, reuse passing evidence, or
represent governed capability gaps. Hosted workflows currently consume a
projection of those decisions into separate environment lists. Keeping those
lists aligned with workflow inputs creates unnecessary opportunities to lose
required work. The seven missing macOS cells reported in PR #76 illustrate
why scheduling and expected coverage must agree.

This feature removes the environment-list interface and moves test-completeness
validation into the job that ran the tests. It supersedes the remaining direct
scheduling and producer-completeness work in 2026-09-12-better-cicd-flow.
It preserves the canonical plan, package identities, archive sharing,
event-based environment policy, and the policy-free merge gate.

## Existing contracts and review corrections

The relevant sources are the [CI contracts](../../.github/ci/README.md),
[resolved-plan schema](../../scripts/ci/schema.py), and the shipped workflows:

| Surface | Current responsibility |
|---|---|
| [ci.yml](../../.github/workflows/ci.yml) | Scope, preflight, native build owners, area calls, area-mapping check, merge gate, reporting |
| [_area-ci.yml](../../.github/workflows/_area-ci.yml) | Package calls, accepted-gap publication, area coverage audit |
| [_package-ci.yml](../../.github/workflows/_package-ci.yml) | Check, lint, native L1/L2/browser consumers, WSL2 delegation |
| [_wsl-ci.yml](../../.github/workflows/_wsl-ci.yml) | Archive consumption and tests inside the WSL2 guest |

The root `repo-deps` package's
[`legacy_scope_document`](../../scripts/ci/affected_scope.py) already reads
execution decisions from the resolved plan; it does not independently select
scope. The change removes its hosted environment-list interface, not a second
planner. Its policy and build projections, and local consumers of its package
matrix, must remain until their readers have migrated.

> Reader's note: the draft proposed skipping an entire area when no cell
> executes. That would remove its blocking audit and accepted-gap publisher.
> This revision retains the area call and skips only execution workflows.
> Eliminating even the audit runner requires a separate ownership decision;
> advisory reporting cannot replace a blocking coverage check.

The call chain currently has four levels. Keeping that depth is a design
constraint here, not GitHub.com's current maximum: GitHub documents ten levels
and 50 unique reusable workflows. Matrix expansion remains limited to 256 jobs
per matrix. Validate limits without scheduling a full workspace merely to
probe platform behavior. See [reusable-workflow limits](https://docs.github.com/en/actions/reference/workflows-and-actions/reusing-workflow-configurations)
and [matrix limits](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idstrategymatrix).

Exact-test skip approvals currently live in
[ci-baseline.toml](../../.github/ci/ci-baseline.toml), not in plan cells. Expected
test manifests are supported by the existing rollup contract but are not yet
wired into producers. Both are implementation work, not existing guarantees.

## Decisions

### One matrix row per executing cell

The planner's output adapter emits area-local row sets derived only from the
final resolved plan, after evidence reuse has been applied. Each executing cell
appears exactly once; reused, accepted-gap, prohibited, and deferred work never
creates an execution row. A prohibited or otherwise invalid omission must not
be mistaken for a valid gap. Event-deferred environments remain outside the
plan's required cells.

Use one row per cell, not a row bundling L1 and L2. Separate jobs preserve
failure attribution and retries per gate. Archive creation already happens
once per build record, so separate downloads do not require separate builds.
Do not introduce a new L1 prerequisite that suppresses otherwise required L2
or browser execution after a test failure.

Rows carry only dispatch identity and the fields needed to expand the workflow:
package, environment, gate, and runner. Job steps read the remaining execution
contract from the immutable resolved-plan artifact using the exact cell key.
The runner comes from the plan's environment table; `wsl2-ubuntu` is an
execution environment hosted by a Windows runner, not a runner label.
Consumers verify row identity and dispatch fields against the plan and refuse
unknown, duplicate, non-executing, or mismatched cells.

The plan must provide every required execution input: feature and test arguments,
canonical tier/profile selection, slow-test policy, build reference, native
prerequisites, runner tools, toolchain and Node requirements, L2 backends,
companion suites, and check selectors and dependent compile requirements.
Resolve missing fields in the planner, not from live manifests in downstream
workflows. Store package-wide inputs on package records and cell-specific
inputs on cells; do not duplicate complete package records into every row.

Add the exact-skip policy snapshot to the resolved plan with its per-cell and
backend applicability and provenance. Preserve existing owner, reason,
source-run, and optional-expiry rules. The plan artifact is authoritative;
rows do not carry another copy and producers do not reload a potentially
different baseline. Because carried scope receipts must still work without
reading manifests or policy, this addition requires a resolved-plan schema
version increment, synchronized Python/Rust contracts, and rejection of older
scope receipts through the existing schema-miss fallback. Do not invalidate
validation receipts merely for a scheduling representation change; preserve
existing gate-input checks and strengthen them where new completeness evidence
is required.

### A concrete workflow layout

`ci.yml` continues to call `_area-ci.yml` for every selected area requiring an
audit, including areas containing only reused cells or accepted gaps. The area
workflow calls `_package-ci.yml` once with its native test, check, and lint row
sets, rather than once per package. Inside that workflow, three jobs expand
the corresponding disjoint row sets. The native test job branches by tier only
where execution differs, including backend proof and browser serialization.
A fourth job delegates each WSL2 row to `_wsl-ci.yml`; the guest executes only
the row's gate, never an additional internal tier matrix.

The union of those four row sets must equal the area's executing cells, with
no duplicate keys. All matrices use `fail-fast: false`. Planner-emitted scalar
flags guard empty row sets before matrix expansion, including skipping the
entire execution-workflow call for an all-reused or gap-only area. The area
audit resolves its package membership from the plan, not the execution rows.

The plan's existing build-owner schedule stays separate. Every executing test
cell retains exactly one valid build reference; owners compile and publish
once per planned build record even when their native test cell is reused but
a WSL2 consumer still needs the archive. Consumers preserve revision/tree,
build-key, archive, sidecar, and path verification. They never rebuild a missing
archive or derive a new build key. A failed build must not prevent consumers of
unrelated successful builds from running. Check and lint retain their existing
independent compilation, including the changed package's dependent compile
requirements inside its check cell.

Keep artifact/status/JUnit/receipt identities keyed by package, environment,
and gate, with backend or suite dimensions where already required. A build
record remains a build identity, never a result cell. Preserve run-attempt
selection, current-result precedence, and retained passing evidence on retries.
Do not collect cell results through a matrix reusable workflow's last output.

Job labels are part of runner-loss attribution. Update the root `repo-deps`
package's [runner-loss tooling](../../scripts/ci/runner_loss.py) and its
workflow-derived fixtures with the new labels so a lost native or WSL2 runner
still maps to exactly one cell. Labels must expose package, environment, and
gate without dumping the row JSON. Jobs skipped before matrix expansion must
not display unevaluated matrix expressions. Keep producer tokens read-only;
only the existing accepted-gap publisher receives `checks: write`.

### Producers prove completeness

Before a test producer succeeds, it must:

1. Verify its planned archive, provision runtime requirements, and list tests
   on the execution target using the same archive and workspace remapping as
   the run. Use Nextest's machine-readable listing. Provisioning precedes
   listing because listing executes test binaries and may need runtime
   libraries. Archive consumers do not require a compiler solely to list tests;
   packages whose tests require a toolchain still receive the declared one.
2. Construct the expected set from the canonical package, tier, profile,
   feature configuration, slow-test policy, and declared backend invocations.
   Derive it independently of any ad hoc invocation filter; an extra filter
   must not shrink both the expected and observed sets and pass undetected.
   Record ignored tests and planned exclusions explicitly. Never compare an
   L1 report with every tier in a shared archive.
3. Run the complete planned gate and every attached companion suite. Preserve
   canonical recipes, timeout behavior, concurrency limits, serialization,
   and compiler-work measurement. L2 and browser execution must not bring a
   terminal or browser window into focus.
4. Compare expected and observed identities, not counts. Use the existing
   suite/test identity convention, scoped by cell, backend, and companion
   suite; retain enough binary identity to detect collisions. Normalize
   retries into one final outcome without hiding failures or accepting
   duplicate reports. Fail on missing or malformed reports, missing expected
   tests, unexpected identities, unapproved skips, failed tests or companions,
   and absent required backend proof.
5. Publish a versioned completion record, expected manifest, JUnit reports,
   and diagnostics under the cell identity. The record binds the tested
   revision, build key where applicable, gate inputs, run and attempt, and
   report inventory. `complete` is set only after validation succeeds.
   Upload required artifacts before the producer can succeed; an upload
   failure fails the job. Failure-path publication remains best effort under
   the existing cancellation rules.

The listing and execution must use the repository's pinned Nextest version;
verify its archive listing and ignored-test behavior with fixtures before
selecting the parser. See [Nextest machine-readable lists](https://www.nexte.st/docs/machine-readable/list/)
and [archive execution](https://nexte.st/docs/ci-features/archiving/).

Tests compiled out by `cfg` are absent from that target's expected set, not
skips. A producer-side list from a different target is not a valid comparison.
Retain source-tier ownership checks, but do not claim they prove runtime
coverage on every OS. An empty expected set needs a specific, plan-recorded
reason; a companion-only cell is valid only if its declared suites complete.
An empty JUnit document alone is never proof of successful coverage.

> Reader's note: missing results now fail even when their identities appear in
> a skip approval. The old baseline also allowed expected-but-unreported tests
> to count as skips. This intentional tightening distinguishes an observed,
> approved skip from a lost test. Migration must reject or replace any reliance
> on silent absence with explicit skip evidence; it must not silently retain
> the old interpretation. The baseline file was empty at review time.

Check and lint producers validate and upload their own completion records,
including both halves of a check that compiles unchanged dependents. This
feature grants neither gate new evidence-reuse eligibility.

### The area audit retains a blocking role

The audit no longer recomputes expected test identities or exact skips for
new-format executing cells. It checks that every executing cell has a valid,
complete, correctly bound result and required report inventory; every reused
cell has qualifying evidence; and every accepted gap has valid governance.
It rejects duplicate/conflicting results and unplanned evidence according to
the existing result-selection rules. An explicit current execution continues
to outrank a reused claim and the discrepancy is reported.

The audit must not accept a green status record unsupported by its required
artifacts. Older reused evidence must remain subject to its existing checks;
removing producer-side skip checks from the audit does not retroactively certify
old receipts. During migration, keep the legacy validation path for records
without the new completion contract, or schedule the cell when required proof
cannot be established. Do not introduce cross-tree hosted reuse in this feature.

Render the area slice even after a producer failure, without adding a second
coverage-policy failure for that same producer error. Enforce the verdict when
execution producers succeeded **or when none were required**. A skipped producer
call despite nonempty execution rows is missing coverage, not permission to
skip the audit. A cancellation or producer failure still blocks through the
existing workflow result. Unreadable audit inputs remain an infrastructure
failure rather than an invented test failure.

Accepted gaps retain one neutral check per cell with owner, expiry, reason,
and remediation. Publish them without waiting for test completion. The area
summary and advisory run summary both include reused and accepted-gap cells;
only the area audit owns the verdict. A failure remains stronger than a gap.

`ci-gate` remains byte-identical: a policy-free fold of its existing blocking
job results. `ci-reporting` remains advisory. Shared audit or validator defects
can still affect multiple areas; moving validation does not eliminate that
risk, so this spec makes no such reliability guarantee.

## Migration and validation

1. Add the versioned plan inputs and deterministic row adapter beside the
   current projection. Prove identity equality **and uniqueness**, dispatch
   fields, area membership, and complete joins back to the plan. Compare both
   scheduling forms against the same plan without executing tests twice.
2. Implement the row-driven workflow and producer completion contract together.
   Preserve archive owners and all existing execution behavior. Move workflow
   contract assertions onto the new interfaces; remove obsolete shape checks
   only after their behavioral replacement passes.
3. Introduce explicit version-aware audit handling before removing any old
   completeness checks. Exercise all-reused, gap-only, mixed, and unexpectedly
   skipped-producer areas. Keep selected areas separate from executing rows.
4. Trial the new path on the root area while other areas use the existing path.
   Choose the path once per area; never schedule both. Use local comparisons
   and an ordinary affected-area CI run for reporting and artifact behavior.
   If needed, use a minimal scratch workflow to test nesting and naming rules.
   Do not mandate three duplicate executions or full-workspace dispatches.
5. Switch remaining areas after the focused contracts pass and the trial's
   observed cells match the plan. Retain a temporary area-level rollback switch
   until this review closes. Rollback must restore compatible workflow and
   audit readers together and must never erase evidence or weaken enforcement.
6. Remove environment-list inputs and only the legacy projections with no
   remaining readers. Migrate local hook/Just readers before deleting their
   package matrix. Retain policy and build outputs until their consumers move.
   Update CI documentation and rust-devops/OS skills to describe the new
   topology and intentional skip-rule change; update root instructions where
   they describe audit responsibility. Do not move this feature to `_completed`.

Validate full-workspace and nightly **plans offline** for every individual
matrix's cardinality, four-level call depth, unique-workflow count, job estimate,
and serialized output sizes. Include area audit and build-owner overhead in
estimates; area grouping alone is not proof of staying within limits. Fail
clearly before dispatch if an area exceeds the supported row capacity or output
budget; never truncate it. Design further partitioning only if measured plan
sizes require it. Any larger live run needs an unanswered execution question
and must respect existing evidence reuse and explicit execution constraints.

## Open Questions

### Should valid draft status be normalized before planning?

The requested frontmatter schema allows `draft-spec`, while the existing
`status: draft` is outside that enum. This review preserves that property as
requested rather than silently changing lifecycle state.

- **Recommended: change the value to `draft-spec` before finalization.** This
  matches the supplied schema and the document's current intent. It requires
  an explicit metadata correction but no repository-wide schema expansion.
- **Allow `draft` as an alias in the shared schema.** This can accommodate older
  documents, but expands the lifecycle vocabulary and conflicts with the exact
  schema requested for this review. It belongs in a separate schema decision.

The scheduling design decisions formerly left open are resolved above: one row
per cell, target-derived expected tests, both area and run summaries, and skip
policy stored in the plan artifact. Eliminating audit runners for areas with no
executing cells is deferred because it changes blocking ownership, not merely
presentation.

## Acceptance criteria

1. Every executing cell appears exactly once across the shipped dispatch row
   sets, with matching identity and fields. Reused, accepted-gap, prohibited,
   and event-deferred work creates no execution row. Duplicate, dropped, and
   altered rows fail the relevant contracts.
2. Hosted area/package workflows accept no independent environment lists.
   Consumers verify row-to-plan binding and do not recalculate scope. Old scope
   receipts fall back cleanly after the schema change; unchanged qualifying
   validation evidence remains reusable under its applicable checks.
3. All-reused and gap-only areas create no execution-workflow call but retain
   their blocking audit, area slice, and summary. Neutral gap checks remain
   visible. Unexpectedly skipped execution with required cells cannot pass.
4. Native macOS, Linux, Windows, and WSL2 consumers preserve archive verification,
   runtime provisioning, canonical gate behavior, artifact identity, and retry
   attribution. Multiple consumers share one build; unrelated build failures
   do not suppress them. Check/lint and dependent compile behavior are unchanged.
5. Producer fixtures cover missing and malformed reports, missing tests, extra
   filters, duplicate identities, retries, explicit ignored/approved skips,
   expired approvals, empty suites, required backend failure, companion failure,
   upload failure, and cancellation. A missing test cannot use a skip approval
   to pass. Canonical tier exclusions do not create false missing-test failures.
6. Audit fixtures reject absent or mismatched completion records and artifacts,
   invalid reuse, invalid gaps, and partial rerun evidence. Legacy evidence
   cannot silently bypass checks removed from the new execution path.
7. Runner-loss fixtures derive actual job labels from the new workflows and
   identify native and WSL2 cells. Matrix failures do not cancel siblings;
   producer failures reach the unchanged `ci-gate` without duplicate policy
   failures. Producer permissions remain read-only.
8. Offline full-workspace and nightly plans satisfy matrix, nesting, and output
   budgets. The focused root-area trial matches the plan without duplicate
   gate runs. The Python suites, `ci_workflow_contracts`, rollup and hook suites,
   and actionlint pass using canonical repository recipes where available.
   No implementation or CI run is claimed by this specification review.
