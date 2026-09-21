---
kind: rulings
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 1
status: complete
---

# Rulings — Direct cell execution

Each entry records the decision taken in Phase 1, the reasoning grounded in this
tree, and the consequence if the ruling is later reversed. Where a ruling
contradicts or reinterprets the specification (`../spec.md`), that is said
explicitly.

## R1 — Row key set and job-label legibility

**Ruling: keep `runner` in the row and accept the four-token label.**

GitHub builds a matrix job's label from *every* value in the row. That is why
`ci.yml`'s `area-ci` matrix is a plain vector of area names rather than an
`include:` of whole package records (the comment at
`.github/workflows/ci.yml:739` records this), and why a job that can be skipped
as a whole may carry **no** `name:` — the matrix context is never evaluated for
a skipped job, so a `name:` containing `${{ matrix.* }}` reaches the Checks tab
as raw expression text (proven in this repository; see `preflight`'s comment at
`ci.yml:505` and the `no_skippable_job_is_labelled_with_an_unresolved_expression`
contract).

A four-key row (`package`, `gate`, `environment`, `runner`) therefore renders a
producer leg as `test (homelab-server, L1, ubuntu-latest, ubuntu-latest)`. A
sibling runner-lookup map (row without `runner`, runner resolved from the plan
elsewhere) would be a second document to keep aligned with the rows — the class
of defect this feature exists to remove — so the duplicate token is accepted.

Row keys are ordered `package, gate, environment, runner` so the parsed prefix
is stable, and `runner_loss.py` must accept **both** the three-token
(`test (ubuntu-latest)`) and four-token (`test (homelab-server, L1,
ubuntu-latest, ubuntu-latest)`) forms, so a later reversal of this ruling is a
label-format change, never a silent attribution loss.

**If reversed** (runner dropped from rows): every shipped row set, the
`cell_contract.py` dispatch-field check, and `runner_loss.py`'s four-token arm
change together; archived completion records are unaffected because their key
stays `{package, environment, gate}`.

## R2 — Where the completeness validator runs, and in what language

**Ruling: implement the validator as `scripts/ci/completion.py` (Python 3
stdlib only, like every other `scripts/ci` tool) and add `python3` to the WSL2
guest's declared apt provisioning with a `python3 --version` reachability check
in that same named step.**

An archive consumer has no compiler, and the WSL2 guest today installs exactly
`ca-certificates curl git xz-utils jq`
(`.github/workflows/_wsl-ci.yml`, first-boot step) — verified on this tree.
Native runners already depend on `python3` in this exact job:
`companion_suites.py` runs inside `_package-ci.yml`'s producer on all three
OSes, so the ruling adds one declared package on one leg.

Rejected alternative: a subcommand on the archive-staged `ci-build` binary. It
would reach the guest for free, but `check` and `lint` cells consume no archive
and would need either a second implementation (drift) or a release build of
`repo-deps` inside every check/lint gate job (cost).

**If reversed** (validator moves into a Rust tool): `completion.py`, its suite
`test_completion.py`, and the guest's `python3` provisioning are removed
together; the completion-record contract itself is unchanged, so Phase 6's
audit reader survives.

## R3 — Job layout inside the execution workflow, and the fate of D4 staging

**Ruling: follow the specification — one native test job over the L1, L2, and
browser rows, branching by tier only where execution differs (backend
provisioning and proof, browser serialization, toolchain and Node
provisioning).**

Today L2 (`test-l2`) and browser (`test-browser`) are separate jobs ordered
behind L1 through `needs: test` with `!cancelled()`; the contract
`expensive_tiers_stage_behind_l1_but_lint_never_gates_it`
(`tools/test-toolkit/tests/ci_workflow_contracts.rs`) asserts that shape. The
ordering was resource staging, never correctness: a red L1 must not suppress
required L2 or browser evidence, and the specification forbids a new L1
prerequisite that would. One job over all three tiers cannot express
`needs:`-ordered staging, so the contract test is **rewritten to assert the
negative**: no test row's job depends on another test job's success. This
reinterprets the D4 contract rather than the specification.

Consequence to watch: peak concurrent runners per area rises. Phase 7's
capacity report records it; the fallback — a second `needs:`-ordered job for
the L2 and browser rows — is a two-line change if a measured run shows queue
damage.

**If reversed** (staging restored): the negative contract is replaced by the
old `needs: test` assertion and the L2/browser rows split out of the `test`
job; row sets and the plan schema do not move.

## R4 — "The repository's pinned Nextest version"

**Ruling: read the specification's pin requirement as "the same
`cargo-nextest` binary the gate command uses, in the same job", enforce it by
construction — listing and running in one job with one installed binary — and
record the resolved `cargo-nextest --version` in the completion record so a
cross-host mismatch is visible after the fact.**

The repository pins no nextest version: all four reader-facing workflows
install through `taiki-e/install-action@nextest`, and every build contract in
`.github/ci/environments.json` declares `"nextest": "latest"` (verified on this
tree; resolved locally as `cargo-nextest 0.9.136 (1d5bf1ec9 2026-05-16)`).
Introducing a pin is a separate decision touching the build-key identity of
every archive (see `scripts/ci/build_key.py`), so it is noted as a follow-up
candidate and **not** taken here. The producer/consumer version equality check
in `affected_scope._validate_compatibility` is unaffected.

**If reversed** (a pin is adopted): the pinned version enters every build key;
all published archives and validation receipts invalidate at once, by design.

## R5 — `_package-ci.yml` keeps its filename

**Ruling: do not rename it.**

After this change `_package-ci.yml` is called once per *area*, so its name is
no longer descriptive. It stays anyway: the path is named in
`ORCHESTRATION_PATHS` and `GLOBAL_PATHS_ALL_GATES`
(`scripts/ci/affected_scope.py`), and that pairing is what keeps a workflow
edit from invalidating every published local cell (before that rule existed
one such edit cost a 45-minute pre-push). It is also named in
`runner_loss.py`'s comments, `READER_FACING_WORKFLOWS`, the four-level depth
contract, the CI README, and two skills. Renaming would be a multi-file churn
with no behavioral gain (repository Rule 3). Its header documentation is
rewritten instead, to describe what it now is: the area's execution workflow.

**If reversed** (renamed): the new path must be added to both
`GLOBAL_PATHS_ALL_GATES` and `ORCHESTRATION_PATHS` in the same change, exactly
as R11 prescribes for `_area-ci.yml`.

## R6 — The specification's `status` value

**Ruling: set `status: planned` — a value the spec's own frontmatter enum
defines and the accurate lifecycle state once this plan lands — replacing the
out-of-enum `draft`.**

This resolves the specification's Open Question ("normalize `draft-spec`
before finalization?") without expanding the shared lifecycle vocabulary with
a `draft` alias. The author may prefer `draft-spec`; either is a one-word
change and neither affects implementation. Applied to `../spec.md` in this
phase, per the Phase 1 rule that the spec's own frontmatter is the one
production file that changes.

**If reversed** (value becomes `draft-spec` or the enum grows a `draft`
alias): a one-word frontmatter edit; no code, workflow, or test reads the
spec's status.

## R7 — Row transport and the output budget

**Ruling: the planner is the only place a budget is enforced.**

Row sets travel as `scope`-job outputs indexed per area in the `with:` block,
exactly as `area_matrix` does today, and the immutable `ci-resolved-plan`
artifact carries everything else. The planner fails with a named, actionable
error **before** emitting a plan whose largest row set exceeds `MATRIX_LIMIT`
(256, `affected_scope.py:436`) or whose serialized per-area payload exceeds a
declared byte budget. It never truncates, never silently splits an area, and
CI never re-derives the budget. Measured headroom on this tree (see
`spikes/s3-capacity.md`): largest row set 21 of 256; largest per-area payload
~2.1 KB; budget constants recorded in S3 for Phase 7 to enforce.

**If reversed** (budget moves into the workflow): a second enforcement point
that can drift from the planner's, plus a mid-run failure mode the planner
exists to prevent; the row-set shape itself does not change.

## R8 — The skip policy stays hand-edited and is snapshotted

**Ruling: `.github/ci/ci-baseline.toml` remains the human-owned source of
truth; the planner reads it once and writes a `skip_policy` snapshot into the
plan with per-cell and backend applicability, owner, reason, `source_run`, and
optional expiry, plus provenance naming the file and its content hash. An
expired approval, or an approval naming a cell the plan does not carry, is a
planner-time error — not a producer-time surprise — because the plan is the
artifact a carried scope receipt reuses.**

The file is empty on this tree (verified), so the migration moves no data. The
interpretation change — a missing result is a **failure**, never an approved
skip — is encoded in `completion.py` and the audit, and pinned by fixtures in
both. Producers and the audit read the plan only; they never reload a
potentially different baseline.

**If reversed** (producers read the live baseline): a plan/receipt boundary
opens where two runs can disagree about skip policy for the same carried plan;
every completion record would need its own baseline copy to stay auditable.

## R9 — Per-area path selection and rollback

**Ruling: a committed allowlist, `.github/ci/direct-execution.json`
(`{schema_version, areas: [...]}`), names the areas on the new path; the
planner reads it and stamps `execution_path: "rows" | "lists"` on each area
record. Exactly one path per area per run, asserted by a contract test that
fails if any area would emit both row sets and environment lists. The workflow
branches on the area record, never on a workflow-level input. The trial value
is `["root"]`.**

Phase 8 deletes the file, the field, and the `lists` branch together once
every area is switched and the review closes. Rollback must restore compatible
workflow and audit readers together and never erase evidence or weaken
enforcement.

**If reversed** (no per-area switch — all areas move at once): the trial
phase loses its rollback; any defect becomes all-areas at once, which is
precisely the blast radius the migration steps exist to avoid.

## R10 — Completion records are their own artifact

**Ruling: `status-<package>-<gate>-<environment>` keeps its `always()`
semantics and stays the failure-path diagnostic. The completion record ships
as `completion-<package>-<gate>-<environment>`, written and uploaded **only
after** validation succeeds, so `complete: true` cannot exist without the
evidence behind it. The audit adds `completion-*` to its download pattern, and
a green status whose completion artifact is absent or mismatched is an audit
failure. Keys stay `{package, environment, gate}` with backend and suite
dimensions where already required.**

**If reversed** (completion folded into the status artifact): a failed
validation could publish a half-written record unless the status writer
becomes transactional; keeping them separate is what makes
"validated-then-uploaded" a simple ordering.

## R11 — `_area-ci.yml` becomes a scheduling input; the selection tables must say so

**Ruling: add `.github/workflows/_area-ci.yml` to both
`GLOBAL_PATHS_ALL_GATES` and `ORCHESTRATION_PATHS` in one change, and pin the
pairing with a fixture: a path that forces workspace scope because it
schedules work must also be excluded from gate-input identity.**

Verified on this tree: `_area-ci.yml` is in **neither** list today — it selects
only `test-toolkit`, through the `.github/workflows/**` suite-owner prefix.
Under this design it carries the row sets, so it decides what runs. Adding it
to one list only is the failure mode to guard against: the first spelling
(only `GLOBAL_PATHS_ALL_GATES`) costs a needless full-workspace pre-push on
every area-workflow edit; the second (only `ORCHESTRATION_PATHS`) invalidates
every published local cell on a workflow edit.

**If reversed** (the file returns to non-scheduling status): both entries and
the paired fixture are removed together; a partial removal reintroduces one of
the two failure modes above.

## R12 — `_expected_manifest` must become a CI entry recipe

**Ruling: add `_expected_manifest` to `CI_RECIPES_BY_GATE["test"]` in the same
change that makes a producer call it, with a fixture asserting an edit to it
selects the test gate and moves the test cells' gate-input identity.**

Verified on this tree: `CI_RECIPES_BY_GATE["test"]` is
`("_test", "_test_l2", "_test_browser")` (`affected_scope.py:89`), and
`local_evidence.just_gate_inputs` / `just_change_gates` work from that closure.
A producer calling `just _expected_manifest` directly would put a recipe on
the critical path of what a gate *proves* while leaving it outside the closure
that selects and identifies the gate: editing the expected-set logic would
change every producer's verdict and select nothing.

**If reversed** (`_expected_manifest` stays unregistered): an edit to the
expected-set logic can change what "complete" means for every producer without
scheduling a single cell — the exact silent-drift class the selection rules
exist to prevent.
