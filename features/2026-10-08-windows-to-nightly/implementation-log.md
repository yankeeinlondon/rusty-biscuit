---
kind: implementation-log
feature: 2026-10-08-windows-to-nightly
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
