---
kind: implementation-log
feature: 2026-10-08-windows-to-nightly
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - scripts/ci/schema.py
  - scripts/ci/local_evidence.py
  - scripts/ci/observation_notes.py
  - scripts/ci/completion.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_completion.py
  - scripts/ci/test_cross_check.py
  - scripts/cross-check.sh
  - .githooks/pre-push
  - .githooks/tests/test-pre-push.sh
  - .github/ci/schemas/contract.json
docs_updated_during_phase_2:
  - docs/cicd/schema-versions.md
  - docs/topics/ci-cd.md
  - .github/ci/schemas/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/ci-evidence.md
source_files_during_phase_3:
  - scripts/ci/schema.py
  - scripts/ci/reuse_validation.py
  - scripts/ci/affected_scope.py
  - scripts/ci/test_schema.py
  - scripts/ci/test_reuse_validation.py
  - scripts/ci/test_affected_scope.py
  - scripts/ci-rollup.rs
  - scripts/ci-rollup-tests.rs
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/ci.yml
  - .github/ci/schemas/contract.json
docs_updated_during_phase_3:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - docs/cicd/schema-versions.md
  - docs/topics/ci-cd.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/ci-evidence.md
source_files_during_phase_4:
  - scripts/cross-check.sh
  - scripts/ci/schema.py
  - scripts/ci/local_evidence.py
  - scripts/ci/observation_notes.py
  - scripts/ci/hosted_observations.py
  - scripts/ci/affected_scope.py
  - scripts/ci/test_evidence_reuse.py
  - scripts/ci/test_cross_check.py
  - scripts/ci/test_hosted_observations.py
  - scripts/ci/test_affected_scope.py
  - scripts/Cargo.toml
  - tools/test-toolkit/tests/ci_workflow_contracts.rs
  - .github/workflows/ci.yml
  - .github/workflows/_package-ci.yml
  - .github/workflows/_wsl-ci.yml
  - .github/ci/schemas/contract.json
docs_updated_during_phase_4:
  - .github/ci/README.md
  - .github/ci/schemas/README.md
  - docs/topics/ci-cd.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/rust-devops/ci-cd.md
  - .claude/skills/os/ci-evidence.md
  - .claude/skills/os/build-hosts.md
packages:
  - repo-deps
  - test-toolkit
---

# Implementation log

## Decisions taken before implementation (2026-10-10)

- Delivered in three phases on one branch, each committed for review:
  1. cadence correction, plus waiting for an unfinished PR run on a push;
  2. native Windows receipt publication and scheduled evidence consumption,
     including the all-zero-base bootstrap and the Windows migration baseline;
  3. resumable full-workspace rig seeding.
- The optional risk-directed Windows rig workflow is deferred.
- A push merged before its pull request's CI finished waits for that run
  (bounded at 90 minutes) instead of falling back to the full grid at once.
  This is an addition to the spec, which did not cover early merges.

## Phase 1 — cadence (2026-10-10)

- `environments.json`: `windows-latest` events are now `schedule` and
  `workflow_dispatch`. A PR-proven push plans no environment, cell, build, or
  preflight runner; a nightly plans native Windows and WSL2 cells, with Linux
  as WSL2's cell-less archive producer.
- `reuse_validation.py check --wait-minutes N` polls an unfinished PR run of
  the merged head once a minute; `ci.yml`'s `validation` job waits up to 90
  minutes (step timeout 95, job 100).
- **Departure:** `schema.py` refused a gating package with an empty gate list
  unless it carried an exclusion. A fully proven push is the first plan to
  select packages while scheduling no environment, so the rule now admits an
  empty gate list without an exclusion exactly when the plan schedules no
  environment. `scheduled_areas` already ignored such packages, so no area
  job launches.
- Docs and skills updated; seeding, receipt publication, and scheduled
  evidence consumption are marked **planned**.
- Not yet done from the spec: the contract check for conflicting cadence
  guidance, the Windows migration baseline, and the bootstrap/recovery
  diagnostics (phase 2).

### Validation

- `scripts/ci` Python suites: all pass (`test_inputs.py` runs no tests under
  `unittest`, unchanged from `main`).
- `ci_workflow_contracts`: 165 passed. `actionlint`: clean.

## Phase 1

Design and discovery only: no production file was edited. Waves 1–2 were run by
three read-only inventory agents (contracts, transport, recovery) and
integrated here. Rulings below are **implementation assumptions** recorded
because no author response is possible in this session; none claims author
approval, and each names its consequence. The spec is left as its snapshot.

### Recovery inventory (inspected 2026-10-11, read-only `gh`)

- `gh run list --workflow ci.yml --event schedule --limit 8`: all eight are
  attempt 1, conclusion `failure` (run ids 38059157100 (2026-10-10, head
  7c0b5612), 37948949074 (2026-10-09, 7c0b5612), 37800062455 (2026-10-08),
  37642656386, 37482581152, 37344514586, 37206926023, 37125197081).
  A query for `status=success` scheduled runs returns `total_count` 0: **no
  nightly has ever succeeded for either environment, so there is no baseline**
  and the planner is in genuine bootstrap.
- Only the two latest runs were drilled into. Run 38059157100: 15 failed, 107
  skipped, 102 succeeded jobs; nothing cancelled; 62 WSL2 cells passed. The
  only native-Windows job was `preflight (windows-latest)` (passed); the
  current hosted nightly runs no native Windows test cells.
- Failing cells, 38059157100 (all L1, `wsl2-ubuntu`): `claudine-cli` (test
  failure in "L1 tests from the archive"); `reaper`, `reaper-cli`, `ui`
  (tabby), `agent-sandbox-cli`, `research-cli` (fail at "Certify this cell's
  completeness"). The same six failed in 37948949074. The coverage-audit job
  failures (e.g. "Download the resolved package policy") vary run to run and are
  probably consequences of the cell failures; this was not confirmed.
- No all-zero base or "bootstrap" text appears in the failing run's log; that
  is absence of evidence, not confirmation (the planner step output and
  artifacts were not inspected). The all-zero bootstrap is inferred from the
  empty success history.
- **Historical, not current:** the spec's 2026-10-08 figures (66 executing WSL2
  cells, 17 accepted-gap cells, 68 packages) were not re-measured.
- Recovery targets for Phase 4: `claudine-cli`, `reaper`, `reaper-cli`, `ui`,
  `agent-sandbox-cli`, `research-cli` L1 on WSL2 (reproduce via
  `just cross-check <pkg> --os wsl`). Retry command:
  `gh run list --workflow ci.yml --event schedule --limit 8`.

### Contract inventory (findings)

Current versions: resolved plan 7; validation receipt 2 (reads 1); scope
receipt 1; expected manifest 2 (Rust mirror stays 1, legacy only); completion
record 1; result doc 5; skip baseline 3; environment table 3. Owners and mirrors
are the table in `docs/cicd/schema-versions.md`.

- **Receipt v2 reader** (`schema.validate_receipt`, `local_evidence.verify_cells`):
  an invalid receipt contributes no cell; a non-`complete` cell is rejected
  alone; a complete failure resolves the cell without credit and an older pass
  cannot override it. `scope_identity`, `base`, `duration_s`, `counts`,
  `backends` are shape/display only; the verifier recomputes
  `gate_input_identity`.
- **Discovery** (`verify_cells`, `local_evidence.py:288-441`): candidates are
  `rev-list merge_base..head`; the merge base itself is excluded; an all-zero
  base fails `merge-base` and yields one `missing-receipt`; reports are never
  opened (trust is in the note); same-head `merge_prior_receipt` requires
  identical host/scope/base, so it cannot aggregate across hosts.
- **Schedule**: `ci.yml:249-286` skips scope-receipt and cell verification for
  `schedule` and `workflow_dispatch`; `nightly_base` checks only the GitHub
  conclusion (first of 20, no pagination, no environment, audit, or policy
  check), so a WSL2-only pre-migration success would qualify for Windows today.
  Bootstrap, API failure, and exhausted search are indistinguishable (stderr
  only).
- **`cross-check.sh`**: publishes only WSL2, one package's L1 per receipt, with
  `git notes add -f` and no fetch/merge/retry, so a second package at the same
  head overwrites the first; no Windows receipt; receipt `host.report_dir` is a
  deleted temp dir; honest failures never publish (`record_cross_check` raises
  on non-complete); no interruption trap; no up-front rig reachability probe;
  base is `merge-base origin/main HEAD` (not bootstrap-safe).
- **Defects found in readers (Phase 2 must fix with the new version):**
  1. Python readers check the key set before `schema_version`, contradicting
     the documented version-first rule (Rust checks first).
  2. Python version checks use `==`, so `true` reads as 1 and `2.0` as 2.
  3. `validate_receipt` can raise `UnboundLocalError` (`label` used before
     assignment for a bad `test_filter` on the first cell) and `TypeError` for a
     non-dict cell; `verify_cells` does not catch them.
  4. Expected-manifest `nextest_version: null` becomes `"None"` and passes;
     non-iterable `backends`/`companion_suites` raise uncaught `TypeError`.
  5. JSON loaders are last-wins on duplicate keys everywhere (Python
     `json.loads`, Rust `serde_json::Value`) and accept `NaN`/`Infinity`.
  6. `attempt`/`run` in the completion record are unchecked by the Python
     validator.

### Transport inventory (findings)

- Existing interfaces: `just cross-check <pkg> [--os …]` (`justfile:153`, one
  package per call), `just ci-local` (`just/ci-local.just`, has `--plan`,
  `--dry-run`, `--plan-out`), `ci-diff`. No `ci-seed` recipe or alias exists.
- Persist points: after a leg's `cross-check-exit` marker and before the next
  `run_unix`/`run_windows` (`cross-check.sh:1006-1022`). Today Unix legs
  discard the staged report immediately (`:728-735`) except WSL (fetched after
  all legs), and the Windows leg deletes its report and `manifest.jsonl` over
  SSH after the leg. A seed must fetch report + manifest into
  `${BISCUIT_CI_EVIDENCE_DIR:-~/.rusty-biscuit/ci-evidence}/<sha>/<env>/` before
  those deletions, then `record-cells --prior-receipt`.
- Locks: `<clone>/.cross-check.lock` (mkdir/New-Item, 30-min wait, exit 75).
  A seed must treat exit 75 as "rig busy", not a test failure. Storage
  preflight runs inside `_test`, after the `ci-build` compile, so a seed must
  preflight each leg first. Dirty local trees are shipped as a synthetic commit
  by design; a seed must refuse them and require a committed pinned revision.
- Hosted: only `accepted-gaps` holds `checks: write` (`_area-ci.yml:128-134`);
  `package-ci` is `contents: read`. No workflow writes git notes. Completion
  artifacts upload only on success; `junit`/`status` upload on `!cancelled()`/
  `always()`; build archives keep 1 day; `ci-resolved-plan` keeps 3 days;
  default retention is the repository setting (not checked).
- Stale doc: `.claude/skills/os/build-hosts.md` (~183-187) reports the old
  report path; to be corrected in Phase 4 once confirmed on a rig.

### Rulings (assumptions, pending author review)

1. **Storage — versioned observation collection** under the existing
   `refs/notes/ci-local/<env>` refs, as the spec recommends. Notes body becomes
   **receipt version 3** (a collection); v1/v2 notes stay readable as
   single-observation collections and are never upgraded in place. Fields:
   - collection: `schema_version`(3), `environment`, `head`, `tree`,
     `observations[]` sorted by id, size-capped (256 KiB, else refuse to publish
     and keep local proof);
   - observation: `observation_id` = identity of the canonical observation
     without the id (same id + different content is corruption; republishing the
     same observation is idempotent), `retry_of?`, `producer`
     `{kind: hook|rig|hosted|seed, host{os,kernel,label}, workflow?, run_id?,
     attempt?, seed_id?}`, `base`, `scope_identity`, `cell` (the existing receipt
     cell contract plus optional `features`), `proof` `{nextest_version, target,
     expected_listing_digest, completion_record_digest}`, `reports[]`
     `{path, sha256, bytes, store: host-dir|artifact|git}`.
   - Consequence: older code meeting a v3 note fails safe (cells execute);
     `host-dir` report references cannot be verified from a hosted runner and are
     accepted with provenance shown, exactly as rig receipts are today;
     expired/missing `artifact` references cause execution with a visible reason.
   - Distinction kept: observation `completion` (complete/interrupted/partial)
     and `outcome` (pass/fail) are separate from a producer completion record
     (`completion-*` artifact) and from an accepted gap (a plan decision, never
     an observation). A completed failure is a stored observation, not proof.
2. **Precedence.** Observation A supersedes B if (a) A names B (transitively) in
   `retry_of`, both on the same commit, A complete, same environment/package/
   gate/`test_filter`; or (b) A's commit is a descendant of B's and both are
   input-equivalent to the head under the existing closure rule. Unordered
   pairs with differing outcomes leave the cell unproven (`conflicting-evidence`,
   execute); unordered passes agree and the lowest id is reported. Incomplete
   observations never supersede or conflict. Cyclic, dangling, or cross-cell
   links are rejected. Timestamps, fetch order, and publication order are never
   used. Consequence: one extra execution on a genuine conflict. All
   observations are gathered before resolving, removing the current dependency
   on environment loop order for narrowed cells.
   New rejection codes: `observation-link-invalid`, `observation-id-mismatch`,
   `observation-report-unavailable`, `search-exhausted`.
3. **Interface — `just ci-seed`**, a dedicated recipe (placed in
   `just/ci-local.just` or a new imported module), backed by cross-check
   transport. No collision found. Flags: `--plan`/`--dry-run` (preview; creates no
   proof), `--os windows|wsl|all`, `--revision <sha>` (pinned, committed, clean
   tree required), `--resume`. Exit nonzero while any requested cell is
   unproved; governed gaps are reported separately; exit-75 lock and unreachable
   rigs are "unavailable", not failures.
4. **Baseline certification.** An audited scheduled run publishes a
   **coverage certification** (version 1) artifact per certified environment:
   `schema_version`, `run_id`, `attempt`, `workflow_path`, `event`(schedule),
   `head`, `tree`, `coverage_policy_version`, `environments_digest`,
   `plan_identity`, `anchors_used`, `environments` per env
   `{certified, cells{executing,reused,accepted_gap,omitted}, audits{required,
   passed}}`; written only after the final gate passes and no
   `producers-skipped`. Artifact name
   `ci-coverage-v1-<environment>-<head>-<policy16>`, looked up by name (no
   download needed to find it).
   - A run is eligible if scheduled, on `main`, this workflow/repository,
     successful, and its head is an ancestor. It is credited for environment E
     only with E's certification at or above E's minimum policy version.
     Legacy runs derive provenance from `environments.json` at the run's head
     (`git show`): environments whose `events` included `schedule` there earn
     credit, so a pre-migration run anchors WSL2 only; missing provenance earns
     Windows nothing; skipped required audits earn neither.
   - Fallback reason codes: `bootstrap-no-run`, `api-failure`,
     `search-exhausted` (bounded, with pagination beyond the current 20),
     `policy-uncertified`, `no-qualifying-ancestor`, `certified-ancestor`,
     `legacy-wsl2-ancestor`. Newest qualifying ancestor rule retained.
   - Plan **version 8** adds `coverage_anchors[]`
     `{environment, mode: incremental|full, anchor: sha|null, reason, source{kind:
     nightly-certified|nightly-legacy|none, run_id, head}}` (`anchor` null iff
     `mode` is `full`); the planner selects per environment, unions by
     `{package, environment, gate}`, and evaluates current policy at the run date
     so expired gaps, newly declared gates, and newly required backends add work
     independent of any source diff. Receipts never become anchors. Inspect the
     live version again before bumping (another branch may claim 8).
5. **Publication trust.** Reports: existing artifact retention for hosted cells
   (referenced by artifact name + sha256), rig-durable evidence directory for rig
   cells. Notes are updated by fetch (`+` force fetch of the ref) → union by
   observation id → `git notes add -f` → **non-force** push, bounded at **5**
   attempts with jittered backoff; final failure keeps local proof and reports
   remote reuse unavailable. Hosted note publication is done by a scheduled-only
   publisher job running default-branch code with `contents: write` scoped to that
   job (not `area-ci`/`package-ci`, which keep `contents: read`; `accepted-gaps`
   stays the sole `checks: write` owner). PR code gets no new credential.
   **Unverified, to check in Phase 4 before relying on it:** whether the
   `protect-your-bacon` ruleset or repository settings permit `GITHUB_TOKEN`
   to push `refs/notes/*`. If not, hosted observations stay artifact-only and the
   blocker is reported (assumption, not a fabricated success).
6. **Resume state** (`$BISCUIT_CI_EVIDENCE_DIR/seed/<seed_id>/state.json`, atomic
   write, rig-lock guarded): `schema_version`(1), `seed_id`, `repository{id,
   remote, root_commit}`, `pinned{revision, tree}`, `plan{identity,
   schema_version, cell_count}`, `policy{coverage_policy_version,
   environments_digest, skip_policy_digest}`, `contracts{receipt, completion,
   expected_manifest}`, `cell_contracts` (per cell: `cell_digest`, `status`,
   `attempts[]{n, exit_code, observation_id?, published, publish_error?,
   retry_of?}`), `progress`. Progress cache only: every resume re-resolves the
   plan, rechecks identity/policy/contracts/digests, and re-verifies each
   recorded pass through `verify_cells`. `main` advancing never relabels a pass;
   a new pass reuses equivalent-input observations and runs changed cells.
7. **No performance spike**: confirmed; no timing study, benchmark, or hosted
   dispatch was or will be run.

### Fixed input matrix (expanded from the plan)

Outcome key as in the plan (`R` reject, `P` accept after verification, `E` real
empty, `D` documented absence, `N/A→R`). Public decision = resolved plan /
rejection summary / seed exit. Planning input errors exit nonzero; evidence
errors reject the one observation/cell and never shrink required coverage.

**Existing receipt-cell fields (v1/v2 generations, still read)**

| Field | absent | null | wrong type | element wrong | empty |
|---|---|---|---|---|---|
| `schema_version` | R | R | R (bool/float included) | n/a | n/a |
| `environment` | R | R | R, not member / ref mismatch | n/a | R |
| `head`,`tree` | R | R | R, not 40-hex or ≠ commit | n/a | R |
| `base`,`scope_identity` | R | R | R, shape | n/a | R |
| `completion` | R | R | R, not enum; non-`complete` ⇒ no credit | n/a | R |
| `host` | R | R | R | n/a | R (needs `os`,`kernel`,`report_dir`) |
| `cells` | R | R | R | R, no filtering | R |
| cell `package`,`gate`,`outcome` | R | R | R, no crash | n/a | R |
| cell `exit_code`,`report` | R | R | R | n/a | R |
| cell `counts` | R | R | R, `total`≠sum | R | R |
| cell `duration_s` | R | R | R (bool rejected) | n/a | n/a |
| cell `gate_input_identity` | R | R | R | n/a | R |
| cell `backends` | R | R | R | R | E |
| cell `failed_tests` | D (only legal on pass) | R | R | R | E |
| cell `build` | D | R | R | n/a | R |
| cell `test_filter` | D (whole tier) | R | R, no crash | n/a | R |
| duplicate key / cell | R | | | | |
| trailing content / NaN / Infinity | R | | | | |

**Completion record** (`schema_version`,`package`,`environment`,`gate`,
`complete`,`head`,`build`,`run`,`attempt`,`reports`; `backends`,`companions`,
`gate_inputs`,`nextest_version` optional and unread): absent R, null R, wrong
type R (`attempt`/`run` now typed), element non-string in `reports` R, empty
`reports` E only where expected listing is legitimately empty, duplicate key R,
trailing R.

**Expected manifest** (`schema_version`,`environment`,`tier`,`target`,
`nextest_version`,`from_archive`,`selection{filter,profile,test_args}`,
`packages[pkg]{tests,ignored,excluded}`,`backends`,`companion_suites`): absent
R, null R (including `nextest_version: null`, no `"None"`), wrong type R (no
uncaught `TypeError`), element wrong R, empty `tests` E only when the gate
contract accepts an empty listing, duplicate R, trailing R.

**Observation collection (v3)**: matrix from the plan, with columns
`observations`, `observation_id`, `retry_of`, `producer`, `reports[]`
(`path`,`sha256`,`bytes`,`store`), `proof`
(`nextest_version`,`target`,`expected_listing_digest`,`completion_record_digest`)
and `features`. `retry_of` and `features` absent = D; everything else absent R.
Wrong-element and wrong-every-element reject without filtering; empty
`observations` is E (zero proof); empty `reports` is E only for a gate needing
none; `observation_id` mismatch ⇒ `observation-id-mismatch`.

**Certification / migration**: columns `schema_version`, `run_id`, `attempt`,
`workflow_path`, `event`, `head`, `tree`, `coverage_policy_version`,
`environments_digest`, `plan_identity`, `anchors_used`, `environments[].certified`,
`environments[].cells{…}`, `environments[].audits{required,passed}`, plan
`coverage_anchors[].{environment,mode,anchor,reason,source.kind,source.run_id,
source.head}`. Absent/null/wrong-type R ⇒ that environment earns no credit and
selects full scope with a reason (visible), never a reduced plan;
`anchor` null iff `mode` is `full`; empty `anchors_used` only for a bootstrap
tagged state. API failure or malformed list ⇒ `api-failure` full-scope fallback.

**Resume**: columns per the plan (`schema_version`,`repository`,
`pinned_revision`→`pinned.revision`,`policy`,`policy_date`,`cell_contracts`,
`progress`) plus `seed_id`,`contracts`,`plan`. Any R stops resume with a
repair/restart diagnostic and preserves observations; `cell_contracts` empty is
E only for a workspace with no gated packages; `progress` empty is E (nothing
completed).

Scope check of unchanged readers: `environments.json` and the skip baseline TOML
are read by the same changed code but their shape does not change; the plan
(v8) is JSON validated by `validate_resolved_plan`, extended with the anchors
column. Workflow YAML is contract-inspected only, so no YAML matrix is added.

### Phase 1 checkpoint

- Contracts, rulings, matrix, transport prerequisites, and recovery targets are
  recorded above; no load-bearing input is undecided.
- Requirement-to-test mapping: not applicable this phase (no behavior changed);
  Phase 2+ owns the matrix tests named above. No tests, lint, or OS rigs were
  run, because no source changed.
- Skipped / not done: no hosted run, no rig run, no `gh` mutation. Recovery
  diagnosis covered the two latest nightly runs only.

## Phase 2

Durable observation storage and verification (Waves 3-5). Python only; no
workflow, planner (`affected_scope.py`), or plan-version change, so the plan
stays at version 7 and `RECEIPT_SCHEMA_VERSION` at 2.

### What changed

- `schema.py`: observation collection (`COLLECTION_SCHEMA_VERSION = 3`) with
  `validate_collection`, `validate_observation`, `observation_identity`,
  `is_version`, `loads_strict`; four new rejection codes
  (`observation-link-invalid`, `observation-id-mismatch`,
  `observation-report-unavailable`, `observation-proof-missing`);
  `receipt_cell_problems` extracted from `validate_receipt` so a receipt and an
  observation share one cell contract. Reader defects 1, 2, 3, 5 (for every
  note, plan, prior-receipt, and manifest loader), and 6 from the Phase 1
  inventory are fixed; 4 is fixed in `completion.py`.
- `local_evidence.py::verify_cells` rewritten as gather-then-resolve
  (`_read_note`/`_read_collection`/`_read_legacy`, `_plan_keys`,
  `_qualification`, `_supersedes`). v1, v2, and v3 notes feed one resolver.
  `candidate_commits` now includes the coverage boundary commit and, for an
  all-zero base, enumerates annotated reachable ancestors (cap 500, one
  `merge-base --is-ancestor` per noted commit, no merge base, no full walk).
- `observation_notes.py` (new): `receipt_observations`, `merge_observations`,
  `publish` (local write -> fetch into a scratch ref -> union -> rebase local
  notes onto the fetched tip -> non-force push, 5 attempts, jittered backoff),
  and the `publish-receipt` CLI (exit 0 published, 4 local only, 3 nothing
  recorded).
- `pre-push` and `cross-check.sh` publish through `publish-receipt`; the hook no
  longer force-fetches the notes ref or feeds a prior receipt (the collection
  carries earlier cells); `cross-check.sh` copies its WSL report to the evidence
  directory first so the observation's digest names something that outlives the
  script.
- `completion.py`: version via `is_version`, strict JSON, `nextest_version`
  must be a string (`null` used to become the text `"None"`), `backends` and
  `companion_suites` must be lists of strings (used to raise `TypeError`); the
  completion record now types `run` and `attempt`.

### Departures from the Phase 1 rulings (all assumptions, pending author review)

- **`proof` is forbidden for `hook` and required for `rig`/`hosted`/`seed`.**
  Both existing publishers (hook, `cross-check.sh`) prove through the report
  counts of their cell and have no expected-listing or completion-record digest
  to cite, so they publish `hook` observations. A `rig` observation is only
  accepted once Phase 4 produces real proof. The ruling listed `proof` as always
  present.
- **`features` is a single string** (the plan's build identity spells isolated
  features as one string), not a list. Absent means no feature arguments.
- **`reports[].store` is `host-dir` or `artifact`**; `git` was dropped (nothing
  would write or verify it). An `artifact` reference is unavailable unless the
  caller passes a `report_probe`; the probe is Phase 4's seam.
- **`producer.host`** is `{os, kernel, label?, report_dir?}`; `report_dir` is
  required only when a `host-dir` report is cited.
- **Automatic retry links.** `publish` links a new observation of a cell to the
  single unretried observation already in the collection (none or several =
  unlinked). A republication of an identical run is dropped before linking.
  Content-addressed ids make retry cycles unconstructible; the cycle, self-link,
  dangling, and cross-cell checks remain as defense and are tested for the
  constructible cases (dangling, cross-cell).
- **Boundary commit now consulted.** The merge base itself was excluded
  before; the spec asks for the coverage boundary, so it is included.
- **Backend and feature checks are v3-only.** `_observation_refusal` rejects an
  observation whose proved backends miss the plan cell's `backends` or whose
  `features` differ from the plan build's isolated features. v2 cells keep the
  checks they had; their `receipt_cell` already marks an unproved L2 `partial`.
- `search-exhausted` is not added yet: nothing in this phase can exhaust a
  bounded search into a decision (Phase 3 owns the fallback reasons).
- An existing v1 note, or a v2 note whose reports are no longer retained,
  cannot be carried into a collection; publishing then fails locally with the
  reason instead of dropping the older proof.

### Requirement-to-test mapping

| Behavior | Test (`scripts/ci/`) |
|---|---|
| Collection credits every package it holds; evidence names the observation | `test_evidence_reuse.ObservationCollectionTests.test_a_collection_credits_every_package_it_holds` |
| Boundary commit is a candidate; changed inputs refused | `...test_the_coverage_boundary_commit_is_a_candidate`, `...test_a_changed_gate_input_on_the_boundary_is_refused` |
| Newer failure beats older pass; later pass restores reuse | `...test_a_newer_failure_beats_an_older_pass_of_equivalent_inputs`, `...test_a_later_passing_commit_restores_reuse_over_an_older_failure` |
| Retry lineage (pass supersedes failure; failure supersedes pass) | `...test_a_retry_that_passes_supersedes_the_failure_it_names`, `...test_a_retry_that_fails_supersedes_the_pass_it_names` |
| Unordered disagreement executes; agreeing passes credit once | `...test_unlinked_disagreeing_observations_on_one_commit_conflict`, `...test_results_on_unordered_commits_conflict` (real merge), `...test_unlinked_agreeing_passes_credit_the_cell_once` |
| Incomplete observations neither credit nor supersede | `...test_an_incomplete_observation_neither_credits_nor_supersedes` |
| Invalid links / forged ids / wrong ref | `...test_a_dangling_retry_link_rejects_the_whole_collection`, `...test_a_retry_across_cells_is_rejected`, `...test_an_id_that_is_not_the_identity_of_its_content_is_rejected`, `...test_a_note_filed_under_the_wrong_environment_is_credited_to_neither`; schema: `CollectionValidationTests` |
| Reports: missing, altered, other host, artifact probe, failure still blocks, must be cited | `ObservationReportTests` (6 tests) |
| Feature and backend refusal | `...test_features_other_than_the_plans_are_refused`, `...test_an_observation_that_misses_a_planned_backend_is_refused` (backend case calls `_observation_refusal` directly: building an executing L2 plan fixture was out of proportion) |
| Bootstrap: all-zero base reads head, finds ancestor proof, ignores non-ancestors, needs no merge base, bounded by notes | `BootstrapDiscoveryTests` (5 tests) |
| Strict JSON in notes: duplicate key, trailing content, NaN, boolean version | `StrictDocumentTests`; schema: `StrictJsonTests`, `ReceiptReaderRobustnessTests` |
| Input matrix, collection side (envelope, observation, producer/proof, report ref, cell fields), one edit per cell with an unedited control for each | `ObservationMatrixTests` (6 tests, every assertion through `verify_cells`) |
| Publication: local+remote, sequential packages, idempotence, retry linking, second host union, lost race retried, unrelated commit's notes survive a rebase, unreachable remote keeps proof local, retry bound, oversize writes nothing, same id different content | `PublicationTests` (11 tests, real bare remote and second clone) |
| Recorder round trip (read/write/read), v2 note carried not overwritten, missing reports publish nothing, CLI exit 4 local-only | `RecorderPublicationTests` |
| Hook publishes a one-observation v3 collection from a `hook` producer; follow-ups, failures, stacked PR bases still behave | `.githooks/tests/test-pre-push.sh` (all pass; reads through the `receipt_view` jq projection) |
| `cross-check.sh` publishes via the collection and retains its report first | `test_cross_check.WslObservationPublicationTests` (static contract: the shell path itself was not run end to end) |
| Reader defects: version first, bool/float versions, crash on non-dict cell and bad first `test_filter`, bool duration/exit code, null build/report | `test_schema.ReceiptReaderRobustnessTests` |
| Completion/manifest defects | `test_completion.ManifestReaderRobustnessTests` (null/non-string `nextest_version`, bool/float version, non-list declarations, duplicate key), `test_schema.CompletionRecordValidationTests` (version first, typed `run`/`attempt`) |

Matrix cells still open: the nightly-certification, anchor, and resume columns
belong to Phases 3 and 5; the expected-manifest per-field walk (absent/null/wrong
type for every field) was not done as a full table, only the defects the Phase 1
inventory named. The plan's final "Assert complete matrix" task therefore stays
unchecked.

### Gates run (macOS, local)

- `scripts/ci/suite_runner.py` counts: `test_schema.py` 190, `test_completion.py`
  73-74, `test_cross_check.py` 30, `test_local_evidence.py` 23,
  `test_evidence_reuse.py` 135 - all pass.
- Unittest runs, all pass: `test_resolved_plan`, `test_ci_local`,
  `test_consolidation`, `test_constraints`, `test_runner_loss`, `test_build_key`,
  `test_publish_gaps`, `test_reuse_validation` (432 + 310 tests combined).
- `.githooks/tests/test-pre-push.sh`: pass. `shellcheck -S warning` on the hook
  and `cross-check.sh`: clean. `bash -n scripts/cross-check.sh`: clean.
- `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 165 passed.
- `cargo nextest run -p repo-deps`: 471 passed, **1 timed out**:
  `ci-build archive::tests::slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`
  (90 s `slow_` test that archives a real package). I did not touch `ci-build`
  or archives and did not re-run it on the unmodified tree, so I cannot say it is
  pre-existing; treat it as unverified. The frozen-contract tests
  (`plan_fields_match_the_frozen_contract`, `completion_record_fields_match_the_frozen_contract`,
  `the_rejection_vocabulary_matches_the_frozen_cross_language_contract`) pass.
- Not run: `test_affected_scope.py` (the planner is unchanged), `just lint`
  (no Python linter is wired; Rust is unchanged), `just test-l2`.

### OS evidence

macOS only. Nothing in this phase is compiled or `cfg`-gated; the Windows-facing
choices are deliberate: git is called with a list argv and stdin (no shell
quoting), the ancestor list goes through `git rev-list --stdin` so a long list
cannot overflow the 32 KiB Windows command line, paths are only joined with
`pathlib`, and no `canonicalize`. `just cross-check` runs a package's Rust
tiers, not the Python companion suites, so it would not exercise this code;
native Windows and WSL2 runs of the Python suites and of `publish-receipt`
against a real remote were **not** performed. Phase 4 should smoke
`observation_notes.py publish-receipt` on both rigs.

### Notes for later phases

- `verify_cells(..., report_probe=...)` is the seam for hosted `artifact`
  reports (Phase 4); `publish(..., before_push=..., sleep=...)` are test seams.
- The resolver has no `schedule` special case yet: `ci.yml` still skips
  cell verification for `schedule` and `workflow_dispatch`, and `nightly_base`
  is unchanged (Phase 3).
- `.githooks/pre-push` no longer force-fetches `refs/notes/ci-local/<env>`.
  The scope note still uses `attach_note`'s `+ref:ref` fetch; that is a
  different ref, so it was left alone.

## Phase 3

Migration-aware nightly planning and audits (Waves 6-7). Waves 6 (planner, anchor
resolution, scheduled overlay) were done in the orchestrator; Wave 7 ran as two
foreground subagents with disjoint ownership (`ci-rollup.rs`; `ci.yml` plus the
workflow contract test).

### What changed

- `schema.py`: coverage vocabulary (`ANCHOR_REASONS`, `ANCHOR_MODES`,
  `ANCHOR_SOURCE_KINDS`, `COVERAGE_POLICY_VERSION`), the plan's optional
  `coverage_anchors` (validated by `_coverage_anchor`: reason, mode, anchor, and
  source must agree), the certification document
  (`CERTIFICATION_SCHEMA_VERSION = 1`, `validate_certification`,
  `policy_digest`, `certification_name`), and the planner's anchor input
  (`validate_anchor_input`). `contract.json` regenerated.
- `reuse_validation.py`: `nightly_base` is gone. `resolve_anchors` finds, per
  scheduled environment, the newest successful nightly on `main` that is an
  ancestor and proved *that* environment: a valid certification artifact
  (looked up by name, downloaded and validated, bound to run, head, environment,
  digest, policy version, and passed audits), or, for a run with no
  certification for any scheduled environment, the environment's `schedule`
  membership in `environments.json` at that commit plus every `coverage-audit`
  job succeeding. Bounded, paginated search (4 pages of 50). Coded fallbacks:
  `bootstrap-no-run`, `api-failure`, `search-exhausted`, `policy-uncertified`,
  `no-qualifying-ancestor`. New CLI modes `nightly-anchors` and `certify`.
- `affected_scope.py`: `calculate_scope` selection was extracted into a nested
  `select_group` (behavior-preserving; the full 733-test planner/schema suite
  passed after the extraction). `coverage_groups` runs it once per distinct
  anchor, `merge_selections` unions cells once per `{package, environment,
  gate}`, `apply_policy_delta` adds cells current policy requires that the
  anchor's certification never covered and records `inherited_cells`. New
  `--anchors FILE` derives each diff itself (`git_changes`); malformed anchors or
  anchors that do not cover exactly the scheduled environments stop planning.
- `ci.yml`: the schedule path calls `nightly-anchors` and passes `--anchors`; the
  evidence verifier/overlay now runs on `schedule` (only `workflow_dispatch`
  skips it); the scope job reads `actions: read`; the scope summary prints
  execute/reuse/gap counts and the anchor reasons; the `certify-nightly` job
  (schedule-only, after `ci-gate` success, contents+actions read) uploads each
  certification under its exact name with 90-day retention.
- `ci-rollup.rs`: reused-cell proof is checked (`check_reused_proof`); optional
  `coverage_anchors` is read and summarized; all-reused, gap-only, and empty
  plans roll up cleanly.

### Departures from the Phase 1 rulings (assumptions, pending author review)

- **No plan version bump.** The ruling said plan v8. `coverage_anchors` is
  optional and absent outside a nightly, so the version follows the required set
  (the same rule `event`/`deferred_environments` use). This also leaves v8 to the
  archive-guard fix that names it, and no Rust constant or hook fixture moved.
- **Certification is one artifact per environment** (the ruling's per-environment
  `{certified, cells, audits}` map became a top-level document), and it carries
  `covered_cells` plus `cells.inherited` so coverage stays transitive across
  nightlies that execute nothing; the ruling carried counts only, which could not
  support the policy-delta comparison.
- **Legacy anchors prove no cell list** (`covered_cells = null`): the planner
  takes them to cover what the diff does not reselect and adds no delta. Expired
  gaps and newly required backends since a legacy run are therefore invisible
  until the first certified nightly.
- **Audit counting uses job names** (`coverage-audit` at the end of the job
  name, `area-ci (<area>) / coverage-audit` in the real run) from the Actions
  jobs API; `required` is the number of scheduled areas. This was not run
  against a real hosted run.
- **`certify-nightly` fails the run** if it cannot publish, so an uncertified run
  can never be mistaken for a baseline.

### Requirement-to-test mapping

| Behavior | Test |
|---|---|
| Bootstrap, API failure, exhausted search, non-ancestor, newest wins, failed/incomplete/foreign/other-event/other-branch runs never anchor | `test_reuse_validation.NightlyAnchorTests` |
| WSL2-only legacy history anchors WSL2 only, Windows stays full | `...test_a_legacy_wsl2_only_success_anchors_wsl2_and_leaves_windows_in_full_scope` (resolver); `test_affected_scope.CoverageAnchorTests.test_wsl2_only_history_anchors_wsl2_and_runs_windows_in_full` (plan) |
| Certified dual nightly shares one anchor/diff | `...test_a_certified_dual_environment_nightly_anchors_both_environments`; `...test_a_certified_dual_nightly_shares_one_anchor_and_one_diff` |
| Unchanged certified head plans nothing, inherits all | `CoverageAnchorTests.test_an_unchanged_certified_head_plans_no_cell_and_inherits_everything` |
| Policy delta (expired gap / new gate or backend) with an empty diff | `CoverageAnchorTests.test_current_policy_adds_uncovered_cells_whatever_the_diff_says`, `...test_a_delta_cell_joins_a_package_the_diff_already_selected` |
| Skipped audits earn neither anchor | `NightlyAnchorTests.test_a_legacy_run_with_a_skipped_audit_earns_neither_anchor`; certification `audits` edits in the matrix |
| Different anchors use their own diffs; identical inputs give identical plans; bootstrap equals the unanchored full plan | `CoverageAnchorTests` (3 tests) |
| Certification input matrix (absent/null/wrong type/empty per field, unknown field, wrong run/head/environment/digest, uncertified, short audits, duplicate key, trailing content, unreadable artifact), through `resolve_anchors` | `NightlyAnchorTests.test_certification_input_matrix_every_edit_loses_the_anchor` (control row proves the unedited document is credited); schema side `test_schema.CertificationValidationTests` |
| Anchor-input matrix (planner side), exit via `RuntimeError`/`SystemExit` | `CoverageAnchorTests.test_anchor_input_matrix_every_malformed_document_stops_planning`, `...test_certified_and_legacy_entries_must_agree_with_their_coverage_shape`, `...test_the_command_refuses_anchors_beside_a_file_list_or_all` |
| Plan `coverage_anchors` shape/contradiction rejection | `test_schema.CoverageAnchorValidationTests` |
| Certification content: transitive coverage, gaps cover nothing, failed gate/short audits never certify, command writes one artifact per anchored environment | `test_reuse_validation.CertificationTests` |
| Reused-proof audit, stale completion not credited, all-reused/gap-only/empty plans, anchors summary | `ci-rollup-tests.rs` (10 tests named in the Wave 7 report; 285 `ci-rollup` tests pass) |
| Workflow wiring: anchors not single base, verifier on schedule not dispatch, scope permissions, certification job read-only/gated/not in `ci-gate`, PR/push cadence unchanged | `ci_workflow_contracts.rs` (5 new tests; 170 pass) |

### Gates run (macOS, local)

- Planner/schema/evidence Python suites: `test_affected_scope`, `test_resolved_plan`,
  `test_schema` (733 after the refactor, before new tests), then `test_evidence_reuse`,
  `test_local_evidence`, `test_completion`, `test_cross_check`, `test_ci_local`,
  `test_resolved_plan`, `test_consolidation`, `test_constraints`, `test_runner_loss`,
  `test_build_key`, `test_publish_gaps`, `test_reuse_validation`, `test_schema`
  (936) - all pass after two summary-string test failures I caused (an
  unneeded rewording of the dispatch row, reverted). Counts: `test_reuse_validation`
  42, `test_schema` 200, `CoverageAnchorTests` 13.
- `CDPATH= bash .githooks/tests/test-pre-push.sh`: 68 passed (a set `CDPATH`
  breaks the script's repo-root discovery with a misleading "hook not
  executable"; unrelated to this change).
- `actionlint .github/workflows/*.yml`: clean.
- `cargo nextest run -p test-toolkit`: 468 passed. `cargo nextest run -p repo-deps`:
  489 passed, 1 timed out - the same `slow_real_package_archives_...` test as
  Phase 2, unrelated and still unverified against the baseline.
- Real-workspace smoke: `affected_scope.py --event schedule --anchors` with a
  full Windows and a legacy WSL2 anchor produced a plan that validates
  (88 cells, WSL2 inherits 68, 7 s).
- Not run: `just lint` (no Python linter is wired; Rust changes are tests and
  rollup only), `just test-l2`, a hosted run, any rig run.

### OS evidence

macOS only. The changes are Python, workflow YAML, and Rust rollup/test code with
no `cfg`-gated paths; git is invoked with a list argv and `zipfile`/`pathlib`
only. Native Windows and WSL2 runs of the Python suites and of
`observation_notes.py publish-receipt` against a real remote remain **not done**
(Phase 4). The certification download path (`gh api .../zip`) and the audit job
name pattern have never run against a real hosted run.

### Matrix status

Done in this phase: certification document and anchor input (planner and
resolver sides), plan `coverage_anchors`. Still open for the final "Assert
complete matrix" task: resume-state columns (Phase 5), the expected-manifest
per-field walk, and a duplicate-key walk of the plan reader.

### Notes for later phases

- The first nightly after merge is a bootstrap (no nightly has ever succeeded),
  so both environments run the full gated workspace on hosted runners until a
  rig or hosted producer publishes observations (Phase 4) or a nightly is
  certified.
- `certify` needs the real job names; if `coverage-audit` ever gets a `name:`,
  update `reuse_validation.AUDIT_JOB`.
- `workflow_dispatch` never anchors and never certifies.

## Phase 4

Rig and hosted cell producers (Waves 8-9). The rig side (`cross-check.sh`,
`observation_notes.py` rig helpers) and the hosted side (`hosted_observations.py`,
workflows, contracts) ran as two foreground subagents with disjoint file
ownership; I owned the shared schema/verifier contract, the verifier wiring, the
no-op republication fix, the recovery tests, the docs reconciliation, and the
red-cell diagnosis. Rulings below are implementation assumptions pending author
review.

### What changed

- **Proof contract** (`schema.py`, `contract.json`): `completion_record_digest` is
  required of a *passing* `rig`/`hosted`/`seed` observation only, and a *failing*
  one may omit `proof` entirely. Reason: `completion.py` refuses a run with a
  failing test, so a completed failure has no completion record, yet it must stay
  an observation that blocks reuse. New `PROOF_EXPECTED_FILE` /
  `PROOF_COMPLETION_FILE` name the two documents a `rig`/`seed` proof cites,
  inside `producer.host.report_dir`.
- **Verifier** (`local_evidence.py`): `_proof_problem` rechecks those documents
  byte for byte (and that the record is `complete: true` for the same
  package/gate) when the producer's directory is on the reading host; another
  host's directory is accepted with provenance. `verify --artifact-repo` builds
  `hosted_observations.gh_probe` as `verify_cells`' `report_probe`; `ci.yml`
  passes it.
- **Publication** (`observation_notes.py`): republishing what a note already holds
  no longer moves the notes ref (both the local write and the post-fetch union
  are skipped when unchanged). Rig helpers `record_rig_cell` / `publish_rig_cell`
  and `publish-receipt --kind rig`.
- **Rig producers** (`cross-check.sh`): both remote legs build `expected-L1.json`
  with `_expected_manifest` before the tests (a failure prints
  `cross-check-manifest-exit` and does not stop the tests), copy the staging dir
  to a per-run path, and `publish_rig_observation <os>` runs for WSL2 *and* native
  Windows, **right after each leg**. It fetches the staging dir, qualifies the run
  (exact tree, clean, unfiltered, archive mode), validates with `completion`
  in-process on the orchestrating host (native Windows has no usable `python3`),
  and publishes under the leg's own environment. A completed failure is published
  as a failing observation; an incomplete run (missing/unexpected tests, zero
  reports against a nonempty expectation, contradictory exit code) publishes
  nothing. INT/TERM are trapped: an interrupted leg records nothing and earlier
  legs' proof survives. Exit 75 prints as BUSY, ssh 255 as UNREACHABLE.
- **Hosted producers** (`hosted_observations.py`, `_package-ci.yml`,
  `_wsl-ci.yml`, `ci.yml`): scheduled-only `emit` step (`continue-on-error`,
  `!cancelled()`) after the completion step uploads
  `observation-<package>-<gate>-<environment>` for passes *and* honest failures;
  a scheduled-only `publish-observations` job (`contents: write`, `actions: read`,
  `always()`, not in `ci-gate`) validates and publishes them through
  `observation_notes.publish`. `accepted-gaps` stays the sole `checks: write`
  owner; PR permissions are unchanged.

### Departures from the Phase 1 rulings (assumptions)

- `proof` / `completion_record_digest` are conditional on outcome (above); the
  ruling listed them as always required for non-hook kinds.
- Rig proof documents live beside the report in a per-(cell, run) directory;
  `producer.run_id` is set on rig observations and a second run of a cell at one
  head links as a retry.
- The hosted observation's environment is carried by an envelope
  `{schema_version, environment, head, observation}` and `producer.host.label`
  (the probe derives the conventional artifact name from it).
- Hosted report artifacts keep the repository's default retention, so hosted proof
  is reusable only as long as the artifacts live; an expired artifact makes the
  cell execute with `observation-report-unavailable`.

### Requirement-to-test mapping

| Behavior | Test |
|---|---|
| Proof contract: pass needs all four; fail may omit record or proof; matrix of absent/null/wrong type per proof field | `test_evidence_reuse.ObservationReportTests` (`..._without_a_record_or_proof_is_invalid`, `test_a_rig_failure_*`), `ObservationMatrixTests.test_producer_and_proof_fields` |
| Rig proof rechecked byte for byte; record for another cell is not proof | `ObservationReportTests.test_a_rig_pass_is_rechecked_against_the_documents_its_proof_cites`, `...test_a_record_that_certifies_another_cell_is_not_proof` |
| WSL2 and native Windows each credited only in their own environment; sequential packages unioned; declined later cell leaves earlier proof | `test_evidence_reuse.RigProducerToReuseTests` |
| Completed failure published and blocks an older pass; incomplete runs publish nothing; qualification refusals; publication failure keeps proof local; separate per-run directories; tampered proof not credited | `RigFailureAndIncompleteTests`, `RigQualificationTests` |
| `cross-check.sh` end to end with stub ssh/scp, real publisher and `completion.py`, bare origin: per-leg publication before the next leg, interruption keeps the first leg, BUSY/UNREACHABLE, native mode publishes nothing | `test_cross_check.RigPublicationRunTests`, `RigPublicationContractTests` |
| Hosted emit (pass, WSL2, honest failure, 14-row refusal matrix), publish (idempotent, unrelated failure, cancellation-before-publication, invalid observation, push denied keeps proof local), discovery (matching bytes credit; altered/expired/deleted/API-down do not; failure still blocks), field matrix | `test_hosted_observations` (33 tests) |
| Workflow structure: emit/upload on `!cancelled()` + schedule, read-only producers, publisher trusted/outside the gate, verifier gets `--artifact-repo` | `ci_workflow_contracts.rs` (5 new tests) |
| Republishing does not move the notes ref | `PublicationTests.test_republishing_does_not_move_the_notes_ref` |
| Partial recovery through one verifier and overlay: pass survives a failure and a cancellation and the retry runs only the rest; later `main` commit reuses unchanged and reruns changed inputs; warn-mode failure does not hide another environment's pass; unordered rig/hosted disagreement executes until a later commit settles it; expired hosted artifacts execute while the rig pass stays reusable | `PartialRecoveryTests` (5 tests) |

### Recovery targets re-read (2026-10-11)

`gh run view 38059157100 --log-failed` (read-only) and targeted rig runs:

- `reaper`, `reaper-cli`, `ui`, `agent-sandbox-cli`, `research-cli`: each ran
  **"0 tests run"** on the hosted guest. These packages contain no tests at all
  (0 `#[test]` in `reaper/lib`, `reaper/cli`, `tabby/ui`, `agent-sandbox/cli`,
  `research/cli`), the plan still schedules an L1 cell, and `completion.py`
  refuses an empty expectation (`completion-expected-empty`). Reproduced on
  `build-win` with `just cross-check reaper --os wsl`: 0 expected, 0 run. **Not
  fixed**: relaxing the check changes a deliberate safety contract. Raised as a
  human-review item in the spec with options and a recommendation.
- `claudine-cli`: one test timed out on the hosted guest at 100 s
  (`commands::wrap::exec::spawn::descendants::tests::a_descendant_outside_the_group_is_terminated_after_its_parent_dies`).
  On the rig the same package ran **3617 passed, 304 skipped in 52 s**. Not
  reproduced; hosted-WSL-specific and load-dependent. Unresolved.

### Real rig results (from the rig subagent)

`just cross-check biscuit-file` on both rigs produced `expected-L1.json` (WSL2: 1062
tests, `x86_64-unknown-linux-gnu`; native Windows: 1054, `x86_64-pc-windows-msvc`;
`build-win-native` has Cygwin `jq`), and fetched the staging dir. From a pristine
scratch clone with a local bare remote, each rig published a passing `rig`
observation and `local_evidence.py verify --cells` credited exactly the matching
environment's cell; a deliberately failing test published a failing observation.
Nothing was pushed to GitHub. A native Windows failing test, a real Ctrl-C, and a
real busy lock were not exercised on a rig (stub-tested only).

### Gates run (macOS, local)

- Python unittest: `test_evidence_reuse` 172, `test_schema` 200, `test_cross_check`
  52, `test_completion` 74, `test_hosted_observations` 33, `test_reuse_validation`
  42, `test_local_evidence` 23, `test_resolved_plan` 103 - all pass;
  `test_affected_scope` 453 passed (hosted agent, after the registry change).
- `CDPATH= bash .githooks/tests/test-pre-push.sh`: 0 failed. `shellcheck -S warning`
  on `cross-check.sh` and the hook: clean. `actionlint .github/workflows/*.yml`: clean.
- `cargo nextest run -p test-toolkit`: 473 passed. `-p repo-deps`: 490 passed
  (the `slow_real_package_archives_...` test took 75 s but passed this time).
- Not run: `just lint` (no Python linter wired; Rust changes are tests only),
  `just test-l2`, a hosted run, a real Ctrl-C on a rig.

### Skipped / unverified

- Whether rulesets let `GITHUB_TOKEN` push `refs/notes/*`; publisher behavior on a
  real cancellation; re-run attempts repeating an artifact name; the Windows-host
  path of the WSL `emit` step; `gh api .../zip` on a real runner.
- A side effect to know: the rig subagent's scratch rehearsal deleted three warm
  clones on `build-win-native` (the prune keeps only clones of worktrees the local
  repo lists). They hold no unique work and re-clone cold; recorded in
  `build-hosts.md`.

### OS evidence

macOS locally; WSL2 and native Windows via `just cross-check` and the scratch-clone
rehearsal above. Linux was not exercised (the Linux rig does not produce a scheduled
environment). Python code uses list argv and `pathlib` only; hosted Windows runs set
`PYTHONUTF8=1`.

### Notes for later phases

See `message_to_agent` in the spec. Phase 5's seed should reuse
`publish_rig_observation` and keep one evidence directory per (cell, run).

### Phase 4 close-out (2026-10-11)

- Phase 4 left `test_ci_local.py` red: 29 `WorkflowScopeStepTests` failed
  because the scope step now passes `--artifact-repo "$GITHUB_REPOSITORY"` and
  the step harness did not emulate that Actions variable. The harness now sets
  it; 103 pass.
- The spec frontmatter carried two `message_to_agent` keys; they are merged,
  Phase 4's notes first.

## Human review resolved: packages with no tests (2026-10-11)

The author chose **option B**, with a constraint: these packages have no tests
because they are not implemented yet, and must be tested as soon as they are.

- New `[package.metadata.ci.tests] unimplemented = "<why>"`. For a declaring
  package, the planner scans its own source (not `target/`) for a test
  (`#[test]` and path-qualified forms, `rstest`, `test_case`,
  `wasm_bindgen_test`, `proptest!`, any `.rs` under `tests/`). None found: the
  package has no tiers and no test cell; lint still runs on Linux. One found:
  it is scheduled normally and the plan reports the marker stale. The package
  record carries `unimplemented: {reason, tests_found}` (resolved plan 7,
  amended without a bump).
- Declared on `reaper`, `reaper-cli`, `ui`, `agent-sandbox-cli`,
  `research-cli`. A full nightly plan of the real workspace gives them no
  cell and validates.
- **Bug found in Phase 1 and fixed here:** `policy_record` read an empty gate
  list as `gates = false` and indexed the missing exclusion, so a fully
  proven push that selected a package would have crashed in the scope step's
  projection. The projection now keys on the exclusion's presence. The schema
  likewise admits an empty gate list without an exclusion for an
  `unimplemented` package with no tests, besides the fully proven push.
- Trade-off: until its first test, such a package's compile coverage is
  Linux lint (`clippy --all-targets`) only.

### Validation

- `scripts/ci` Python suites: all pass. `actionlint`: clean. Hook tests: 68
  passed, 0 failed.
- `cargo nextest run -p test-toolkit -p repo-deps`: 962 of 963 passed. The
  remaining one, `ci-build`'s
  `slow_real_package_archives_read_their_fixtures_from_the_consumers_checkout`
  (two full relocated dependency builds, 75 s when it passed in Phase 4 against
  a 90 s limit), timed out alone at a host load average of 89. This change
  touches neither `ci-build` nor the two packages it builds; it was not
  re-run on a quiet host.

## Phase 5 — Resumable workspace seeding (2026-10-11)

### What changed

- `just ci-seed` (`scripts/ci/seed.py`): pins a clean checkout reachable from
  `origin/main`, fetches observation notes without force, resolves the
  full-workspace `schedule` plan through `affected_scope.py`, asks
  `local_evidence.verify_cells` (all-zero base) which `windows-latest` and
  `wsl2-ubuntu` cells are proved, and runs `cross-check.sh <package> --os
  windows|wsl` for each package still owing one. Phase 4's
  `publish_rig_observation` publishes each finished leg; the package's cells
  are verified again after every leg.
- Outcomes reported apart: `reused`, `proved`, `gap`, `busy` (exit 75),
  `unreachable` (255), `failed`, `unproved` (exit 0, nothing qualifying
  published), `no-host`. Exit 0 only when nothing requested is owed.
- `--plan` previews and proves nothing; `--os`, `--package` narrow a pass.

### Departures from the Phase 1 rulings

- **No `--resume` flag; resume is the default.** What a pass owes is always
  re-derived from published proof, so the state file cannot decide anything
  and a separate resume mode would only add a way to disagree with it.
- **State file reduced** to `{schema_version, seed_id, repository.remote,
  pinned{revision, tree}, plan{sha256, cell_count}, attempts, cells}`. The
  ruling's policy/contract digests are covered by `seed_id`, which binds the
  pinned commit and the resolved plan's bytes; a different plan is a
  different seed directory, never a rewritten state.
- **Observations are `rig`, not `seed`.** The seed drives the same
  `cross-check.sh` legs, which already publish `rig` observations with proof;
  a separate producer kind would duplicate that path for no reader.
- **`--plan` is allowed off `main`** (warning only); it creates no proof.

### Requirement-to-test mapping

| Behavior | Test |
|---|---|
| Fully seeded bootstrap: every cell reused, no build, no dispatch row | `test_affected_scope.EventSchedulingTests.test_a_fully_seeded_bootstrap_nightly_builds_and_runs_nothing` |
| Partial seed runs exactly the rest; no Windows producer for proved Windows cells | `...test_a_partly_seeded_nightly_runs_exactly_the_rest` |
| Pinning: clean, the checkout, on `origin/main`; preview off main | `test_seed.PinTests` |
| Reuse, per-leg outcomes, missing rig, interruption then resume, retry after failure, attempts accumulate, package filter, preview | `test_seed.SeedRunTests` |
| Atomic state, foreign/unreadable state ignored, lock, seed id binding | `test_seed.StateTests` |

### Validation

- `test_seed` 18, `test_affected_scope` all pass; `test-toolkit` 473 passed;
  `just check-canonical` clean.
- `just ci-seed --plan` on this branch's real metadata: 162 cells, 128 owed,
  34 governed gaps, in about 7 s.

### Operational seeding: not performed

The operational seed cannot run yet. It must run at a clean `origin/main`
checkout, and `main` has neither `seed.py` nor Phase 4's rig publication in
`cross-check.sh`; a pass from this branch would publish proof on a commit no
nightly descends from. It also pushes `refs/notes/ci-local/*` to GitHub,
which needs the author's go-ahead. Recovery inventory once this merges:

```sh
git fetch origin && git switch --detach origin/main
just ci-seed --plan                    # expect 128 owed cells, 34 gaps
just ci-seed --os windows --package biscuit-file   # one small sample first
just ci-seed                           # the rest; rerun to resume
```

No nightly baseline exists, and nothing here is a certified workspace.

## Phase 6 — Documentation and final validation (2026-10-11)

### What changed

- **Guidance:** `docs/topics/ci-cd.md` gains a per-cell flow diagram (proof
  sources → one plan → gap/reuse/execute → audits → certify or keep the
  anchor) and states that a pass counts only for its own environment.
  `docs/cicd/test-inputs.md` no longer says the nightly runs every package's
  full L1 on Linux; `.github/ci/README.md` no longer calls the nightly
  WSL2-only; `CLAUDE.md` marks the nightly duration as measured under the old
  cadence. Release enforcement and the risk-directed workflow stay planned.
- **Drift check:** `ci_workflow_contracts::
  active_cadence_guidance_agrees_with_the_environment_table` reads the
  environment table and refuses a bounded set of contradicting phrases in
  `CLAUDE.md`, `docs/topics/ci-cd.md`, `.github/ci/README.md`, and the
  rust-devops and os skills (normalized for case, backticks, and wrapping).
  Verified to fail on a planted claim. An edit to any listed document selects
  the contracts binary on Linux.
- **Matrix close-out:** duplicate keys and trailing content are now refused at
  load by the readers this feature changed: `affected_scope.py`
  `--apply-to`, `--accepted-cells`, `--evidence-rejections`;
  `reuse_validation.py certify --plan`; `completion.py`'s `manifest.jsonl`
  lines; `seed.py`'s plan and state. The seed state has a per-field matrix
  (absent, null, wrong type, wrong version, boolean version, empty id,
  another seed's id), each with an unedited control.
- **Departure:** a malformed seed state is *ignored and replaced*, not a
  stop-and-repair error as the matrix row says. The state decides nothing, so
  refusing it would block a pass for no protection; only its attempt history
  is lost.

### Acceptance criteria

| Criterion | Evidence |
|---|---|
| Merge eligibility vs release eligibility | `docs/topics/ci-cd.md` ("Planned release evidence policy"); release enforcement deferred, not claimed |
| PR/push plans have no Windows/WSL2 cells; PR-proven push runs nothing | `EventSchedulingTests.test_each_event_schedules_its_environments_and_defers_the_rest`, `test_a_reused_pull_request_leaves_its_push_nothing_to_run` (plus its projection) |
| Nightly: Windows + WSL2 for affected scope, Linux only as WSL2 producer; opt-ins kept | `test_a_scheduled_guest_brings_its_producer_without_cells`, `test_a_full_scope_run_preflights_only_the_scheduled_runners`, `test_the_label_plans_every_environment_for_a_pull_request` |
| Unchanged nightly schedules nothing; failures keep pending coverage; bootstrap/API failure diagnosed; WSL2 history is not Windows proof | `CoverageAnchorTests.test_an_unchanged_certified_head_plans_no_cell_and_inherits_everything`, `test_wsl2_only_history_anchors_wsl2_and_runs_windows_in_full`; `NightlyAnchorTests.test_no_successful_nightly_is_a_bootstrap_for_every_scheduled_environment`, `test_an_api_failure_is_a_visible_full_scope_fallback_not_a_bootstrap`, `test_failed_incomplete_foreign_and_non_ancestor_runs_never_anchor` |
| Existing nightly failures documented with a recovery path | Phase 1 recovery inventory; five zero-test packages fixed (`unimplemented`); `claudine-cli` hosted-WSL2 timeout **unresolved** (passes on the rig) |
| Resumable full-workspace seed; interrupted passes keep completed cells; no receipt overwrite | `test_seed.SeedRunTests` (interruption/resume, retry, accumulation), Phase 4 `RigProducerToReuseTests` (sequential packages unioned) |
| Fully seeded bootstrap schedules nothing; partial seed runs exactly the rest | `test_a_fully_seeded_bootstrap_nightly_builds_and_runs_nothing`, `test_a_partly_seeded_nightly_runs_exactly_the_rest` |
| The three nightly examples | 1: `test_an_unchanged_certified_head_plans_no_cell_and_inherits_everything`; 2: Phase 4 `PartialRecoveryTests` (later commit reuses unchanged, reruns changed); 3: `test_a_partly_seeded_nightly_runs_exactly_the_rest` |
| Strict blocks, warn publishes honest mixed results | hook suite (`test_warn_passing_tests_exits_zero` and strict cases, 68 passed); `test_a_warn_mode_failure_in_one_environment_does_not_hide_a_pass_in_another`; `test_a_newer_failure_beats_an_older_pass_of_equivalent_inputs` |
| Python tests for cadence, empty plans, scoped diffs, producer demand, migration, failures; contracts + actionlint | suites below |
| Active guidance agrees with the shipped policy | `active_cadence_guidance_agrees_with_the_environment_table` |
| End-to-end fixtures: Windows receipt publication, completion checks, discovery, selective reuse, sequential receipts, fallback | Phase 4 `test_cross_check.RigPublicationRunTests`, `test_evidence_reuse.RigProducerToReuseTests`, `RigFailureAndIncompleteTests`, `test_hosted_observations` |

### Simulated vs collected

Everything above is fixture or contract evidence on macOS, plus Phase 4's
real `biscuit-file` legs on both rigs against a scratch bare remote. **Not
collected:** an operational seed (blocked until this branch is on `main`),
any hosted run of the new nightly, and whether `GITHUB_TOKEN` may push
`refs/notes/*`. No nightly baseline exists.

### Gates run (macOS, local, final tree)

Every `SUITE_REGISTRY` Python companion with its count: `test_affected_scope`
462, `test_build_key` 15, `test_ci_local` 103, `test_completion` 75,
`test_constraints` 39, `test_cross_check` 52, `test_evidence_reuse` 172,
`test_hosted_observations` 33, `test_local_evidence` 23, `test_publish_gaps`
19, `test_resolved_plan` 103, `test_reuse_validation` 43, `test_runner_loss`
50, `test_schema` 202, `test_seed` 20, all OK. `just test repo-deps`: 489
passed. `just test test-toolkit`: 472 passed. `actionlint`, `shellcheck -S
warning` (`cross-check.sh`, the hook): clean. Hook fixtures: 68 passed, 0
failed. Host load averaged 150+ (other sessions' language servers) during the
run; nothing timed out.

Implementation complete, ready for review. The operational seed, a hosted
nightly, and the `refs/notes/*` push permission remain open (above).
