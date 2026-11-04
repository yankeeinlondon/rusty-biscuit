---
kind: plan
title: Scoped Windows nightlies and resumable evidence seeding
created: 2026-10-11
total_phases: 6
phase: 4
agent: codex/gpt-6.1-sol
yolo: true
spec: 2026-10-08-windows-to-nightly
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

# Implementation plan

## Phase 1 — Scope, rulings, and contracts

### Necessary Rules

- **Delivery boundary:** preserve the implemented cadence and 90-minute PR wait. Deliver native Windows publication, scheduled evidence reuse, hosted partial-result recovery, migration safeguards, and full-workspace rig seeding. Defer optional risk-directed pre-push selection and release enforcement. Document the intended four-environment release policy without claiming enforcement.
- **Existing work:** the implementation log's cadence phase is complete in this worktree, with recorded Python/workflow validation. This plan starts at Phase 1 for the remaining delivery; its six phases replace the earlier three-phase outline for planning purposes. Do not renumber or rewrite historical log entries, redo cadence implementation, or treat worktree results as deployed behavior.
- **Open storage ruling:** provisionally use a versioned observation collection under existing environment notes, with independently verified immutable observations and report references. This is the spec's recommendation, not an author-approved decision. Record the selected contract before Phase 2; do not reinterpret version-2 receipts or combine hosts into one receipt context.
- **Open precedence ruling:** provisionally use explicit retry lineage, descendant-commit input equivalence, and execution for unordered conflicting outcomes. Record identities, valid supersession rules, and cycle rejection before Phase 2. Host timestamps, fetch order, and publication order cannot hide a qualifying failure.
- **Open interface ruling:** provisionally expose a dedicated Just recipe backed by cross-check transport. Phase 1 selects its concrete name after inspecting existing interfaces; do not imply the author has already chosen a name. Require preview, pinned revision, resume, and nonzero exit for unproved requested coverage; governed gaps remain separate.
- **Baseline certification:** define the durable policy/audit proof that qualifies a successful scheduled run, including a historical WSL2-only run. Missing Windows policy provenance earns no Windows anchor credit; skipped required audits earn neither anchor. Record how current policy adds expired-gap, newly declared gate, and newly required backend work independently of a source diff.
- **Publication trust:** decide the concrete report-retention/fetch mechanism, immutable report identity, bounded conflict retry count, and trusted hosted publication permission boundary. Prefer existing artifact retention and note discovery; expired/unavailable proof causes visible execution. No new write credential for PR code, force-push over unseen notes, or second owner of `checks: write`.
- **No performance spike:** the spec's 66 executing WSL2 cells and repeated full-scope fallback already justify resumable seeding. No repeated timing study, cross-OS benchmark, or full hosted dispatch is needed. Wider measurement would require an author ruling.
- **Noninteractive implementation:** where no author response is possible, record the recommended choices as explicit implementation assumptions and their consequences. Do not claim approval. If repository policy or unavailable credentials prevents a chosen publication path, preserve local proof and report the blocker rather than fabricate successful remote reuse.

### Work and successful completion

The remaining work makes evidence durable per `{package, environment, gate}`, teaches the canonical planner to combine migration scope with verified proof, and provides a resumable seed operation. Success means an ordinary PR/push schedules Linux/macOS only; a scheduled run executes precisely unproved Windows/WSL2 cells and demanded producers; completed rig and hosted observations survive unrelated failure or cancellation; and a fully seeded bootstrap can establish a truthful audited nightly baseline without package builds/tests.

Every phase has an exit checkpoint. Waves have globally unique numeric indices. Tasks in a wave may run concurrently only where their file ownership and prerequisites permit it; a single-task wave is sequential. Implementation may use subagents for the explicitly parallel waves, with narrow ownership, noninteractive instructions, and one integrator for shared contracts. No phase authorizes commits, pushes, hosted dispatch, or moving the feature to `_completed`.

### Wave 1 — Independent discovery

- [x] **Contract inventory**
  - Inspect `schema.py`, `local_evidence.py`, `reuse_validation.py`, `affected_scope.py`, completion/audit readers, and `docs/cicd/schema-versions.md`. Record current versions, reader/writer ownership, legacy compatibility, and every field whose value affects a decision.
  - Inspect ancestor-note discovery, report fetchability, same-head merge limitations, failure precedence, and all-zero/boundary behavior. Produce concrete proposed observation, migration-anchor, policy-certification, and resume-state contracts.
  - Ownership: design notes in `implementation-log.md`; no shared production edits in this wave.
- [x] **Transport inventory**
  - Independently inspect `cross-check.sh`, its tests, root Just interfaces, `BUILD_*`/sniff discovery, rig locks, WSL archive execution, hosted reusable workflows, and artifact publication helpers.
  - Identify where each cell can persist reports and completion before the next cell starts; identify trusted hosted note/report publication and safe native Windows execution paths. Consult `os`, `sniff`, and `rust-testing` skills as applicable.
  - Ownership: return findings to the integrator; do not mutate the same implementation log concurrently.
- [x] **Recovery inventory**
  - Read current nightly run metadata and existing artifacts through authenticated read-only tooling when available; record observed failing cells, bootstrap status, and exact recovery targets with inspection date and run identity.
  - Distinguish current findings from the spec's historical 2026-10-08 numbers. If the API is unavailable, record that limitation and the targeted read-only command to retry; do not infer current failures or start a hosted run.
  - Ownership: return a recovery report to the integrator. This is diagnosis, not a performance spike.

### Wave 2 — Resolve design

- [x] **Record rulings**
  - Integrate Wave 1 findings and resolve the open choices above in the implementation log, including assumptions requiring later author review and any departures from the spec. Leave the spec as its decision snapshot.
  - Freeze observation identity/lineage, report availability, current-policy certification, preview/resume interface, and exit semantics. Specify a failing completed observation separately from successful producer completion; interrupted work never earns proof.
  - Define fallback reason codes for genuine bootstrap, API failure, search exhaustion, missing certification, and no qualifying ancestor. Retain bounded search and the newest qualifying ancestor rule.
- [x] **Complete input matrix**
  - Use the matrix contract below to enumerate **every** load-bearing field on each changed JSON reader, including nested existing receipt, completion, expected-manifest, API, and resume fields. Add individual columns for any fields the final design adds; do not hide them behind one object column.
  - Define legacy absence semantics per supported generation, unknown-version rejection before fields, and fixture provenance. No implicit defaulting or malformed-element filtering may grant proof.
  - Check unchanged JSON/TOML/YAML readers for whether this feature actually changes their behavior. Extend the matrix to each affected format if it does; workflow YAML contract inspection alone does not justify redesigning the workflow parser.

**Checkpoint:** implementation log contains concrete contracts and rulings; the matrix has no undecided load-bearing inputs; transport/publication prerequisites and recovery targets are recorded. Shared implementation starts only after this checkpoint.

## Phase 2 — Durable observation storage and verification

### Wave 3 — Shared schema foundation

- [x] **Version contracts**
  - Implement the selected collection contract in `schema.py` and its validators. Preserve independent receipt/run/host contexts; distinguish observation completion, outcome, producer completion, and accepted gaps.
  - Keep version-1/2 receipt reads only where their existing evidence contracts qualify; do not upgrade provenance in place. Add migration-plan fields with an appropriate resolved-plan version bump when required.
  - Move all affected schema mirrors, regenerate `.github/ci/schemas/contract.json`, and update `docs/cicd/schema-versions.md` together. Inspect current versions rather than assuming the next number remains free.
  - Add the matrix tests through public verification/planning results, starting with actual recorder/producer output as the positive fixture.

### Wave 4 — Parallel evidence services

- [x] **Publish observations**
  - Implement fetch/merge/bounded retry of note updates without force overwrites. Preserve other packages, hosts, attempts, and original proof; repeated publication of one immutable observation is idempotent.
  - Persist reports and observations locally before publication. Surface fetch, retention, upload, and ref-update errors; remote publication failure leaves proof local and uncovered cells scheduled.
  - Ownership: a dedicated publication helper and its tests; the integrator owns changes to shared schema/CLI files.
- [x] **Verify observations**
  - Refactor discovery to enumerate annotated reachable ancestors from fetched refs, including the head and coverage boundary. Bootstrap/all-zero bases must not require a merge base or a walk of every commit.
  - Independently verify tested commit/tree, gate-input closure, features/filter, expected/observed identities, reports, completion, target/runtime, backends, and build identity. Reject unreachable squash/rebase candidates; unknown closures require execution.
  - Resolve equivalent-input outcomes by the frozen lineage rules; reject invalid/cyclic links, preserve newer failure precedence, and require execution for unresolved conflicts. Later qualifying retry passes may restore reuse.
  - Ownership: `local_evidence.py` discovery/verification and focused tests; call the publication helper through its frozen interface.

### Wave 5 — Evidence integration

- [x] **Integrate evidence**
  - Connect collection reads/writes to existing recorder and hook publication paths without changing strict/warn/scope-only/no-verify meanings. Completed failures remain observations, not passing proof; preserve completed cells independently of a failed or interrupted pass.
  - Update module/function docs and inline comments changed by the new discovery, publication, or precedence behavior. Explain rejection reasons through existing summaries.
  - Prove sequential packages, mixed hosts/runs, same-commit retries, concurrent ref conflicts, delayed publication, invalid lineage, expired reports, missing notes, newer failure, and later-pass recovery in isolated local fixtures.

**Checkpoint:** publication preserves unrelated proof under concurrency; verifier grants reuse only for independently valid cells; malformed inputs never shrink required coverage; legacy qualification remains explicit. Focused schema/local-evidence/evidence-reuse/hook suites pass.

## Phase 3 — Migration-aware nightly planning and audits

### Wave 6 — Canonical scope

- [x] **Resolve migration**
  - Extend `reuse_validation.py` to verify recorded coverage policy and required audit results, returning per-environment anchors and precise fallback reasons. Green orchestration alone is insufficient.
  - Have `affected_scope.py` calculate each environment's selection, union cells once, and record anchors/reasons in one resolved plan. Old WSL2-only history anchors WSL2 while Windows selects the full gated workspace; after a certified dual-environment nightly both use the common anchor.
  - Evaluate policy at the current run date and add newly required coverage even with an empty source diff. Receipts never become comparison anchors. Failed/canceled/incomplete/foreign/non-ancestor runs never advance or roll back the successful anchor.
  - Preserve package identities, area derivation, internal prompt exclusions, suite ownership, direct-dependent seam policy, and narrowed test-input behavior. Archive-only Ubuntu must not enable Ubuntu tests, lint, or check.
- [x] **Overlay scheduled proof**
  - Enable verification for schedules, including no-baseline/all-zero and boundary receipts. Apply one verified evidence overlay before deriving dispatch rows, builds, and preflight demand.
  - Derive all workflow consumers from the resolved plan; remove any single-base workflow diff assumption incompatible with migration. Consumers do not calculate their own selections.
  - Record execute/reuse/gap counts and fallback frequency in the existing summary. Windows reuse removes native producers; WSL2 still demands its Linux archive and Windows guest host where necessary.

### Wave 7 — Parallel consumers

- [x] **Audit reused proof**
  - Update `scripts/ci-rollup.rs` and relevant contracts to verify reused-cell provenance without crediting an old completion as this run's execution. Require actual completion records for executing cells.
  - Ensure fully reused/gap-only plans launch necessary normal audits despite empty execution rows; truly unchanged and unchanged-policy plans launch no package builds/tests/preflight. Preserve producer failures and policy-free `ci-gate` folding.
  - Ownership: rollup/results/audit code and tests; no concurrent workflow edits.
- [x] **Preserve cadence**
  - Update workflow wiring and `ci_workflow_contracts` for canonical migration selection, scheduled overlay, producer demand, explicit opt-ins, and empty plans.
  - Keep existing event-table authority, Linux-only check/lint policy, 90-minute same-tree PR wait, and failure/cancellation/timeout fallback to ordinary Linux/macOS. Do not reintroduce Windows/WSL2 through a fallback or another workflow.
  - Ownership: workflow orchestration and workflow contracts; coordinate shared fixtures with the integrator.

**Checkpoint:** local planner fixtures prove all three spec examples, WSL2-only success at unchanged head, bootstrap, exhausted/API-failed lookup, skipped audits, non-ancestors, expired gaps, new gates/backends, unchanged successful dual-policy scope, and empty PR-proven pushes. Repeat identical inputs, policy date, and evidence to assert identical resolved plans. Actionlint and focused workflow/audit suites pass.

## Phase 4 — Rig and hosted cell producers

### Wave 8 — Parallel producer paths

- [x] **Record rig cells**
  - Extend native Windows cross-check publication alongside WSL2, using the canonical planned gates/features/filter and native target/runtime proof. Retain expected manifests, real reports, completion, backend/build proof, and failure outcomes per completed cell.
  - Publish as each cell finishes; interruption preserves earlier proof and gives the interrupted cell no passing credit. Keep WSL2 faithful archive mode, existing warm directories/locks, and configured `BUILD_*` discovery.
  - Test unsupported flags, dirty/mismatched trees, zero reports against nonempty expectations, legitimate empty expectations, missing backends/rigs, and publication failure. Compile-only or WSL2 results cannot prove native Windows behavior.
  - Ownership: `cross-check.sh`, rig recorder adapters, and cross-check tests.
- [x] **Record hosted cells**
  - Make trusted scheduled producers durably publish each completed observation with immutable artifact/report references before area/workflow success is known. Publish honest completed failures through the observation path even where no passing completion exists.
  - Bind proof to original head/build/workflow/run/attempt. Retrying at a new head uses verified reuse; never rewrite original completion. Ensure discovery actually fetches/validates hosted artifacts, not just uploads them.
  - Keep PR permissions unchanged and accepted-gap reporting's exclusive `checks: write` owner. Test publication denial, artifact expiry, unrelated producer failure, and cancellation after versus before durable publication.
  - Ownership: producer reusable workflows and hosted publication adapter/tests, excluding rig files.

### Wave 9 — Recovery integration

- [x] **Exercise partial recovery**
  - Feed both producer paths through the same verifier and overlay. Simulate a pass in one environment followed by failure/cancellation elsewhere; retain the prior successful anchor and execute only remaining unproved cells on retry.
  - Cover a later `main` commit with changed and unchanged gate inputs, sequential packages at one revision, mixed warn-mode outcomes, and unresolved same-input conflicting observations.
  - Reproduce concrete current red cells from Phase 1 on matching configured rigs with targeted gates; record fixes or unresolved blockers honestly. Do not change unrelated packages or broaden hosted scope to diagnose them.

**Checkpoint:** native Windows and WSL2 each have real producer-to-discovery-to-reuse fixtures; hosted proof survives unrelated failure/cancellation; no missing report or canceled cell gets credit. Matching native/guest smoke runs establish transport/report integrity without a full hosted run.

## Phase 5 — Resumable workspace seeding

### Wave 10 — Seed operation

- [x] **Implement seed command**
  - Add the dedicated Just interface chosen in Phase 1, backed by a small orchestrator using the existing planner, verifier, and cross-check transport. Preview resolves one full-workspace schedule plan at a pinned committed `main` revision; it creates no proof.
  - List every gated package's separate Windows/WSL2 cells, declared gates/features, current gaps, chosen host, reuse/execution, and remaining coverage. Packages without gates stay visibly unscheduled.
  - Refuse dirty/mismatched execution trees and unsafe concurrent checkout/build mutations. Honor rig locks and native Windows storage contracts; missing/unreachable rigs are explicit and leave requested coverage unproved.
- [x] **Persist seed progress**
  - Bind resume state to repository, pinned revision, policy/date, and cell contracts. Treat it as a progress cache only; reverify published/local observations and actual inputs on every resume.
  - Publish each completed cell immediately using Phase 4. Preserve local proof if publication fails and distinguish local success from remote availability. Failure/interruption must not discard earlier cells or wait for workspace-wide success to publish.
  - Keep a pass pinned when `main` advances. A new pass may reuse older equivalent-input observations and execute changed cells; never relabel an old run as testing the new head.
  - Report completed failures, interruptions, unavailable backends, gaps, and publication blockers separately; exit nonzero when requested coverage remains unproved.

### Wave 11 — Seed acceptance

- [ ] **Verify seed recovery**
  - Use isolated multi-package fixtures for preview, full/partial seeds, stopped/resumed passes, changed head/inputs, missing rigs, concurrency locks, sequential package preservation, and stale/malformed resume state.
  - Prove a fully seeded all-zero bootstrap resolves required cells to reuse/gaps, schedules no package builds/tests, performs normal audits, and becomes eligible for a genuine successful scheduled baseline. A partial seed executes exactly unproved cells and demanded producers.
  - Validate the full-workspace preview against actual metadata. Run one small native Windows/WSL2 sample through the command, then use the resumable operation for actual recovery when configured rigs are available. This operational seed is required coverage gathering, not a performance spike or speculative hosted run.
  - Retain a recovery inventory of still-unproved cells and exact commands if availability prevents completing operational coverage. Do not call that state a certified workspace or successful nightly baseline.

**Checkpoint:** command preview and resume are observable and deterministic; actual proof survives interruption; integration fixtures demonstrate truthful bootstrap. Record separately whether code acceptance, rig coverage gathering, and a normal scheduled baseline have completed.

## Phase 6 — Documentation and final validation

### Wave 12 — Parallel closure preparation

- [x] **Align active guidance**
  - Update `.github/ci/README.md`, schema documentation, `docs/topics/ci-cd.md`, `AGENTS.md`, rust-devops/os skill topic pages, and other active cadence/evidence guidance found by search. Add a developer-facing seed/recovery example and a Mermaid diagram of scope → verify → execute/reuse/gap → audit → baseline.
  - Remove planned markers only for delivered behavior. Keep risk-directed selection and release enforcement planned/deferred. Distinguish merge eligibility from intended four-environment release eligibility, and incremental scope from historical full-scope fallback costs.
  - Historical specs remain snapshots; current docs do not name/link feature or fix directories. Record new OS-specific discoveries in the matching skill page alongside the fix.
  - Ownership: docs/skills/instructions only; integrate implementation log updates serially.
- [x] **Check policy drift**
  - Add a focused contract check for conflicting active cadence guidance using a bounded set of policy claims/locations, excluding historical specs and timing measurements. Avoid exact sentence snapshots and a new independent CI gate.
  - Extend existing planner, evidence, schema, cross-check, hook, audit, and workflow suites rather than adding unrelated CI cells. Use the test toolkit and canonical test-input read spellings so edits select their actual suite owners.
  - Ownership: focused tests and fixtures; agree with the documentation task on the policy assertions before editing.

### Wave 13 — Final verification

- [x] **Run scoped checks**
  - Run relevant Python suites through their registered companion recipes; confirm the expected test count so invoking a module that collects zero tests cannot be reported as validation.
  - Run `just test repo-deps` and `just test test-toolkit` from the repository root; use area `just lint` where supplied and actionlint for changed workflows. Run affected hook fixtures through their canonical noninteractive runner. Use nextest, never `cargo test`; do not run `cargo fmt`.
  - Inspect `SUITE_REGISTRY` and recipe ownership before choosing exact companion commands. Run relevant L2 via area `just test-l2` only if changed behavior requires it; terminal/browser windows must never gain focus.
  - Record OS evidence explicitly: local contract checks plus configured native Windows and WSL2 targeted transport checks, Linux archive production, and macOS behavior where changed. Reuse qualifying evidence; do not add a hosted matrix cell or dispatch a workspace run to answer a fixture-level question.
- [x] **Record review readiness**
  - Map every spec acceptance criterion to a test or operational result, including strict/warn outcomes and hosted partial-result recovery; mark unavailable operational evidence/blockers explicitly. Record assumptions/departures and actual run identities in `implementation-log.md`.
  - Inspect final diffs for unintended cadence changes, policy-free gate drift, scope recalculation, loss of provenance, defaulted malformed inputs, and stale comments. Confirm only necessary files changed and active documentation matches shipped behavior.
  - Stop at implementation complete, ready for review. Do not manufacture a baseline, claim release enforcement, commit, publish a branch, or move the feature to `_completed`.

**Checkpoint:** required focused suites/lint/actionlint pass; the acceptance mapping is complete and separates simulated behavior from actual collected evidence; any remaining operational blocker has a concrete recovery path.

### Input robustness matrix contract

This feature changes JSON evidence and planning readers, so the matrix is required. The following columns freeze the proposed new collection and migration/resume inputs; Phase 1 expands them to the exact selected names and **every nested load-bearing field** from existing contracts. Each table uses the same outcomes, asserted through the public command's resolved plan, reuse/rejection summary, or seed exit/progress result, never only a validator return value.

`R` = reject this input with a field/shape reason; evidence receives no credit and the affected required cell remains scheduled. Invalid planning input stops resolution with a nonzero exit instead of emitting a reduced plan. Invalid resume state stops resume with a repair/restart diagnostic and preserves observations. Invalid API/certification data produces a visible full-scope fallback for the affected environment. `P` = accept only after semantic and provenance verification. `E` = a real empty collection, never absence. `N/A → R` = the shape is inapplicable to this scalar/object but mutation to that shape must still reject. `D` = explicit documented absence semantics for an optional field, distinct from null.

JSON collection envelope and observation extensions (existing receipt fields retain their own columns in the expanded matrix):

| Shape | `schema_version` | `observations` | observation `id` | observation `retry_of` | observation `source` | observation `report_refs` |
| --- | --- | --- | --- | --- | --- | --- |
| Unedited real-tool control | P, known version | P, nonempty proof | P, unique immutable identity | P, valid lineage | P, trusted rig/workflow context | P, available immutable reports |
| Absent | R | R | R | D: no declared retry | R | R |
| Explicit null | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R |
| Wrong one element | N/A → R | R, no filtering | N/A → R | R | R, malformed member | R |
| Wrong every element | N/A → R | R | N/A → R | R | R | R |
| Empty | R | E: zero proof, all required cells unproved | R | E: no supersession claim | R | E: cannot prove a report-requiring gate |
| Duplicate key | R | R | R | R | R | R |
| Trailing/invalid document | R | R | R | R | R | R |

JSON nightly certification/migration inputs (proposed names, finalized in Wave 2):

| Shape | `coverage_policy` | `required_cells` | `audit_results` | `anchors` | anchor `environment` | anchor `commit` | anchor `reason` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Unedited real-tool control | P, current or recognized historical policy | P, exact required identities | P, required audits satisfied | P, explicit environments | P, known environment | P, successful qualifying ancestor | P, enumerated diagnosis |
| Absent | R: no Windows credit; WSL2 requires independent historical qualification | R | R | R | R | R; use explicit no-anchor state | R |
| Explicit null | R | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R | R |
| Wrong one element | R, malformed member | R | R | R | N/A → R | N/A → R | N/A → R |
| Wrong every element | R | R | R | R | N/A → R | N/A → R | N/A → R |
| Empty | R | E only when canonical required scope is actually empty | E only when no audits are required by the contract | R: scheduled environments must be explicit | R | R; no-anchor has a tagged state and reason | R |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing/invalid document | R | R | R | R | R | R | R |

JSON resume inputs (proposed names, finalized in Wave 2):

| Shape | `schema_version` | `repository` | `pinned_revision` | `policy` | `policy_date` | `cell_contracts` | `progress` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Unedited real-tool control | P | P, matching repository | P, actual committed revision | P, matching policy | P, explicit evaluation date | P, matching planned contracts | P, progress only; reverify proof |
| Absent | R | R | R | R | R | R | R |
| Explicit null | R | R | R | R | R | R | R |
| Wrong whole type | R | R | R | R | R | R | R |
| Wrong one element | N/A → R | N/A → R | N/A → R | R, malformed member | N/A → R | R | R |
| Wrong every element | N/A → R | N/A → R | N/A → R | R | N/A → R | R | R |
| Empty | R | R | R | R | R | E only for genuinely ungated workspace scope | E: nothing completed, no reuse credit |
| Duplicate key | R | R | R | R | R | R | R |
| Trailing/invalid document | R | R | R | R | R | R | R |

- [x] **Assert complete matrix**
  - Finalize columns for receipt `environment/base/head/tree/scope_identity/completion/host/cells`; each host, cell, report, source/run, build, expected-manifest, completion, anchor, policy, API, and progress field actually read. Include cells' package/gate/outcome/exit/completion/counts/duration/input identity/backends/report/build/filter and nested values that affect credit. Treat optional legacy absence by that generation's contract; explicit null is a separate row.
  - One parameterized matrix test per supported format walks recorder/producer-generated fixtures, makes one edit per field/shape cell, and asserts the public decision. Use multiple real-tool control fixtures inside that test where different document sides need them; each control must prove that the edited field changes the result.
  - Reject duplicate keys at document loading, not after a last-wins parse; reject valid JSON followed by garbage. A malformed candidate cannot hide a conflicting failure or silently remove a required cell. Test valid empty scopes alongside invalid missing scopes.
  - Before closure, inspect changed readers for `#[serde(default)]`, ambiguous `Option<T>`, `filter_map(.. as_str())`, `unwrap_or_default()`, and `.ok()` on load-bearing parses. Any matrix review finding reports every other state in that field's column and shape's row, rather than fixing one isolated case.
