---
total_phases: 6
created: 2026-07-21
phase: 2
agent: "opencode/zai-coding-plan/glm-5.2"
yolo: "false"
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-21
reviewed_seed: ff6de1834fe07de9d34d9ffd3cd717d7941d54f2
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/fixes/2026-07-20-claudine-mega-merge/plan.md
docs_created_during_phase_1:
  - claudine/fixes/2026-07-20-claudine-mega-merge/acceptance-ledger.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/baselines/**/*.txt
  - claudine/fixes/2026-07-20-claudine-mega-merge/baselines/summary.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/conflict-checklist.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/dirty-worktree-inventory.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact-review.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/**/*.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/marker-baseline.txt
  - claudine/fixes/2026-07-20-claudine-mega-merge/merge-previews/**/*.txt
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase1-closeout.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase1-gates/*.txt
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase1-gates/current-gate-summary.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/reviewed-seed-audit.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/sha-ledger.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/user-owned-worktree.patch
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - .config/nextest.toml
  - biscuit-file/cli/tests/cli_tests.rs
  - biscuit-file/lib/Cargo.toml
  - biscuit-file/lib/src/**/*.rs
  - biscuit-file/lib/tests/**/*.rs
  - biscuit-test-harness/Cargo.toml
  - biscuit-test-harness/src/**/*.rs
  - claudine/cli/Cargo.toml
  - claudine/cli/src/**/*.rs
  - claudine/cli/tests/**/*.rs
  - claudine/contract/src/**/*.rs
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/gen/tests/**/*
  - claudine/justfile
  - claudine/lib/Cargo.toml
  - claudine/lib/benches/**/*.rs
  - claudine/lib/src/**/*.rs
  - claudine/lib/tests/**/*.rs
  - claudine/rendezvous/daemon/src/**/*.rs
  - claudine/reviews/_completed/2026-07-14-module-assessment/generated-artifact-baseline.json
  - darkmatter/lib/src/**/*.rs
  - darkmatter/lib/tests/**/*.rs
  - just/devops.just
  - scripts/check-error-transport.allow
  - scripts/check-error-transport.sh
  - scripts/check-lifecycle-doc-facets.sh
docs_updated_during_phase_2:
  - .claudine/memory/commits.md
  - .claudine/non-interactive.md
  - CLAUDE.md
  - biscuit-file/docs/**/*.md
  - biscuit-test-harness/README.md
  - claudine/docs/rendezvous/local-ipc.md
  - claudine/docs/topics/**/*.md
  - claudine/features/2026-07-11-sequence-plus/{plan,spec}.md
  - claudine/features/2026-07-13-error-propogation/{plan,spec}.md
  - claudine/features/2026-07-13-file-resolution/{plan,spec}.md
  - claudine/features/_completed/2026-06-14-auto-complete/*.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/{conflict-checklist,impact-review,plan}.md
  - darkmatter/docs/**/*.md
  - prompts/_implement/implement-suggestions.md
  - prompts/faster-builds-and-tests.md
docs_created_during_phase_2:
  - claudine/docs/topics/error-architecture.md
  - claudine/features/2026-07-11-sequence-plus/{gate-run-*.md,l3-ctrl-c-runbook.md,phase-1-baseline.md,review-*.md,validation-matrix.md}
  - claudine/features/2026-07-13-error-propogation/{burndown-triage.md,decisions.md,inventory.md,review-*.md}
  - claudine/features/2026-07-13-file-resolution/{decisions.md,inventory.md,review-*.md}
  - claudine/fixes/2026-07-20-claudine-mega-merge/impact/foundation-merge-detect.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-audit.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-gates.md
  - claudine/fixes/2026-07-20-claudine-mega-merge/phase2-test-map.md
  - claudine/fixes/_unscheduled/1-windows-compose-interrupt-guard/spec.md
  - prompts/_implement/implement-review-findings-plan.md
  - prompts/_implement/review-findings-plan.md
  - ~/features/2026-07-20-router-fixture/log.md
skills_files_updated_during_phase_2:
  - .claude/skills/biscuit-file/references/file-references.md
  - .claude/skills/biscuit-test-harness/SKILL.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/error-architecture.md
  - .claude/skills/claudine/timeline.md
packages:
  - biscuit-file
  - biscuit-file-cli
  - biscuit-test-harness
  - claudine
  - claudine-cli
  - claudine-contract
  - claudine-gen
  - darkmatter
  - rendezvous-daemon
---

# Claudine Mega-Merge Execution Plan

Derived from [`spec.md`](./spec.md) with the `claudine-log.md`, `error-prop-and-file-resolution-log.md`, `proxy-with-log.md`, and `conflict-report.md` inputs.

The spec defines six stages labeled Phase 0–5. Per execution-plan convention this plan renumbers the freeze stage as **Phase 1** and shifts every later stage by one, yielding six phases total. Every phase heading states its spec mapping; invariant and risk-register IDs (I1–I12, R1–R16) are preserved verbatim.

## Conventions

- Every task is observable: it produces a commit, file, recorded gate result, saved impact report, or ledger update.
- `(parallelizable with: …)` flags work that can be dispatched concurrently. Sequencing inside a phase is otherwise top-to-bottom in dependency order.
- Canonical commands: `just test` (L1, nextest), `just test-l2` (real-terminal, fail-closed), `just lint`. Never `cargo test`; never `cargo fmt` / `rustfmt` write-mode.
- Run independent gates as independent commands and record every exit status; do not use an `&&` chain that hides later evidence after the first failure.
- GitNexus `impact()` is mandatory before editing any conflicted symbol. Resolve file/module names to concrete symbols first; HIGH/CRITICAL findings halt for review.
- Only `passed` satisfies a required acceptance row. `accepted-debt` can document a deliberate code-integration exception, but it cannot close this fix or move its records to `_completed`.
- Stop conditions (spec §"Stop Conditions and Recovery") apply at every checkpoint. Recovery uses recoverable Git operations only — never a destructive reset on a broad path or dirty worktree (R13).

## Reviewed Revision Ledger

| Input | Reviewed revision | Merge base with `claudine` |
|---|---|---|
| `claudine` (latest reviewed seed) | `ff6de1834fe07de9d34d9ffd3cd717d7941d54f2` | — |
| `error-prop-and-file-resolution` | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` | `8fc8711434b01327479297af9b40a67685409d00` |
| `proxy-with` | `e348486c810969abe87a6b7209979034f5454b07` | `6cdb8bf56321c3747d5ea16a1241e47c2bff7fce` |

The SHA of the commit that contains this reviewed plan cannot be embedded in the plan itself. Phase 1 therefore records the actual execution seed in `sha-ledger.md`, audits every commit between the reviewed seed above and that execution seed, and refreshes previews against it. If either feature ref moves, the new SHA becomes a reviewed input only after the conflict inventory and branch-tip baseline evidence are regenerated (R14).

---

## Phase 1 — Freeze, protect, and baseline

Spec mapping: spec Phase 0. Goal: inputs recoverable, refs frozen, baselines recorded, acceptance ledger exists. No merge work begins in this phase.

### Tasks

- [x] Write `sha-ledger.md` in this directory. Record `git rev-parse refs/heads/claudine` as the execution seed, the two frozen feature-tip SHAs, and both merge bases. Record the command, timestamp, and operator; this artifact is the authoritative ref freeze for the rest of the plan.
- [x] Compare the execution seed with reviewed seed `ff6de1834`. Audit `ff6de1834..<execution-seed>` and confirm that every intervening commit is understood and contains no implementation from either feature branch. Any unreviewed implementation change requires a refreshed specification review before proceeding.
- [x] Verify each feature SHA and merge base against the named refs. Any divergence triggers R14: stop, update the reviewed inputs, regenerate previews, and refresh branch-tip baselines.
- [x] Rerun `git status --short --branch` and classify every modified or untracked path (R13). Preserve each user-owned path in a recoverable location or dedicated scratch commit, and record its disposition. The older research inventory is context, not an instruction to assume that the same paths are still dirty.
- [x] Confirm the actual integration worktree is clean: `git status --short --branch` shows nothing unexpected. If it cannot be clean, halt (spec Stop Conditions).
- [x] Create and name a recoverable integration branch from the execution seed recorded in `sha-ledger.md`; record its name in the ledger. Keep `claudine` and both feature source refs unchanged until final promotion, and preserve full ancestry through merge commits.
- [x] Freeze all three source refs until Phase 6 closes. The separate integration branch may advance only through commits required by this plan. Record the freeze owner/time and notify collaborators who can move those refs.
- [x] Enable `git rerere` on the integration branch in case of repeated attempts. Reused resolutions still require per-conflict review.
- [x] Before either merge, scan tracked text with `git grep -n -E '^(<<<<<<< .+|\|\|\|\|\|\|\| .+|=======|>>>>>>> .+)$'` and save the output to `marker-baseline.txt`. Classify intentional fixture hits. The reviewed seed is expected to contain only the two known `CLAUDE.md` lines; any additional unclassified hit halts Phase 1 (R16).
- [x] Refresh non-mutating merge previews with the exact SHAs from `sha-ledger.md`: execution seed ↔ foundation, execution seed ↔ proxy, and foundation ↔ proxy. Save stdout, stderr, command, and exit status under `merge-previews/`; exit `1` is expected when a preview reports conflicts and is evidence, not a gate failure by itself.
- [x] Export the refreshed feature-to-feature conflict inventory into `conflict-checklist.md`, grouped by cluster (Architecture & records; CLI prep & composition; Harness & proxy routing; Sequence integration; Library composition/lifecycle; Darkmatter; Generated/docs). Start from 36 reviewed paths, but use the refreshed output as authoritative. Add the execution-seed conflict lists and every auto-merged/one-sided hotspot from the spec. Each row gets path, cluster, owner, resolution status, rationale, and evidence slots.
- [x] Capture baseline gate results at `error-prop-and-file-resolution` tip (`43c23c6`). Save to `baselines/error-prop-and-file-resolution/`. *(parallelizable with: the proxy-with baseline below)*
    - [x] From `biscuit-file/`: `just test`, `just test-l2`, and `just lint` as three recorded commands.
    - [x] From `darkmatter/`: `just test`, `just test-l2`, and `just lint` as three recorded commands.
    - [x] From the repository root: `just test biscuit-test-harness` and `just _lint biscuit-test-harness`; this leaf crate has no area `justfile`.
    - [x] From `claudine/rendezvous/`: `just check`, `just test`, and `just lint` as three recorded commands.
    - [x] From `claudine/`: `just test`, `just test-l2 --no-fail-fast`, `just lint`, and `just check-windows` as four recorded commands.
- [x] Capture the same independent baseline gates at `proxy-with` tip (`e348486c`). Save them to `baselines/proxy-with/`. *(parallelizable with: the foundation baseline above)* Run both baselines from separate detached worktrees at the exact SHAs; never switch the dirty authoring worktree between refs.
- [x] Label red, timed-out, skipped, cross-compile-only, and backend-blocked baseline results truthfully. Backend denial (e.g. tmux setup failure on managed macOS) is recorded as debt, never silently waived (R10, R11).
- [x] Build `acceptance-ledger.md` from the owning revisions, not the older copies on the integration seed: the error-propagation, file-resolution, and Sequence Plus specs plus `validation-matrix.md` at `43c23c6`, and the proxy-with spec plus `notes/acceptance-map.md` at `e348486c`. Read them with `git show` or from the exact detached worktrees and record each full source path/SHA in the ledger. Each row records a stable criterion ID, contract, named merged-tree test or audit, tier (L1/L2/L3), platform, owner, status (`passed`/`failed`/`blocked`/`skipped`/`compile-only`/`accepted-debt`), acceptance-candidate SHA, and evidence location.
- [x] Import all 12 mandatory combined seam cases as `MM-S01` through `MM-S12`. Assign each a concrete test owner, tier, and platform before code editing; isolated feature suites cannot satisfy these rows.
- [x] Confirm GitNexus index freshness for each exact revision used by the impact reports (execution seed and both feature tips), using the detached worktrees rather than switching the authoring tree. If stale, refresh with `node .gitnexus/run.cjs analyze` (fallback `npx gitnexus analyze`; npm 11 crash → `npm i -g gitnexus`). Record the indexed revision with every report.
- [x] Resolve the following hot-spot file/module labels to concrete symbol identities with GitNexus `query()`/`context()`, then run upstream `impact()` for every symbol that may be edited and save the reports to `impact/`. Use an index at the exact branch tip that owns the symbol and record that revision; shared symbols need both feature-tip views. Ambiguous labels such as `mod`, `types`, `prep`, or `proxy` are not valid impact targets by themselves:
    - [x] `prep` (compose), `composition::{mod,pipeline,runner}` (wrap), `wrapper_stages`
    - [x] `loop_control`, `control_dispatch`, `proxy` (harness_orch), `overlay`, `prompt`, `types`
    - [x] `sequence::{iterate,mod,phase1c}`
    - [x] `composition::error::{mod,render}`, `composition::lifecycle::{context,executor}`, `composition::looping::engine`, `composition::{mod,preflight,prepare,types}`
    - [x] Darkmatter `markdown::compose::context::options`
- [x] For every HIGH/CRITICAL impact finding, open a row in `impact-review.md` with the proposed resolution owner. These must be reviewed before the corresponding Phase 2/3 edit lands.
- [ ] Stage only the Phase 1 planning/evidence artifacts, review their size and contents for secrets, run `git diff --check`, the anchored marker scans with the known baseline exception, and GitNexus `detect_changes({scope: "compare", base_ref: "main"})`, then create a documentation-only baseline checkpoint commit. Raw logs that contain secrets or are impractically large stay in an access-controlled artifact store; commit a redacted summary and stable evidence pointer instead.

### Validation Checkpoint (Phase 1 exit)

- `sha-ledger.md` exists and matches `git rev-parse` for all three refs.
- `git status --short --branch` is clean in the integration worktree.
- Integration branch exists, based on the execution seed in `sha-ledger.md`; the reviewed-seed range audit is attached.
- `marker-baseline.txt` exists, with every hit classified and the two known `CLAUDE.md` lines explicitly tracked as Phase 2 removal work.
- `baselines/error-prop-and-file-resolution/` and `baselines/proxy-with/` each contain the five area gate logs (or explicitly labeled backend debt).
- `conflict-checklist.md` contains every path from all three refreshed previews plus the auto-merged/one-sided review list from spec §"Conflict Review Scope".
- `acceptance-ledger.md` exists with every spec criterion, every Sequence Plus matrix row, every proxy-with acceptance row, and all 12 seam cases as open rows.
- `impact/` contains revision-labeled reports for every concrete hot-spot symbol on the branch tips where it exists; `impact-review.md` queues HIGH/CRITICAL findings.
- The baseline checkpoint commit contains the redacted Phase 1 artifacts and the integration worktree is clean.

---

## Phase 2 — Integrate the foundation branch (`error-prop-and-file-resolution`)

Spec mapping: spec Phase 1. Goal: a known-good foundation merge commit carrying typed diagnostics → `FileReference`/request context → Sequence Plus. Failures here MUST be resolved before proxy-with is introduced so later regressions stay attributable (R1).

### Tasks

- [x] Re-read the impact reports queued for foundation-side symbols from `impact/`. Resolve or de-scope each HIGH/CRITICAL item with a recorded rationale before touching the corresponding file.
- [ ] Start the merge non-fast-forward and uncommitted: `git merge --no-ff --no-commit 43c23c6`. (Blocked in this execution by the explicit no-stage/no-commit instruction. The exact `git merge-tree` candidate was materialized into the worktree without changing the index; see `phase2-audit.md`.)
- [x] Resolve the three predicted trunk conflicts in repository records and shared prompts:
    - [x] `.claudine/memory/commits.md` — reconcile history, keep it factual and final-architecture-aligned.
    - [x] `CLAUDE.md` — reconcile the surviving guidance and remove the two pre-existing marker lines recorded in `marker-baseline.txt`; `AGENTS.md` exposes this file through a symlink and needs no separate edit.
    - [x] `prompts/_implement/implement-suggestions.md` (and any other shared prompt files surfaced by the refreshed preview).
- [x] Manually audit the auto-merged composition pipeline, wrapper stages, composition/system-prompt topics. A clean textual merge is not semantic compatibility (R7).
- [x] Verify the trunk's transient system-prompt `.gitignore` fix survives (I11). Re-apply if the merge dropped it.
- [x] Verify trunk lifecycle-schema scaffolding, local-runner schemas/research, shared prompt changes, and topic documentation are present (I11).
- [x] Prove the typed diagnostic subsystem before any consumer edit (I1):
    - [x] Run focused tests for the diagnostic registry, snapshots/restoration, and source/transport guards.
    - [x] Confirm `DiagnosticSnapshot` is the effective selector across renderer, `err.*`, and serialization paths.
- [x] Prove the Biscuit File authority (I2):
    - [x] Grammar, kind classification, candidate planning, probes — focused tests pass.
    - [x] Repository-first bare resolution and strict explicit-relative (no fallback) — focused tests pass.
    - [x] `FileResolutionContext` derivation and completion/execution round trips — focused tests pass.
    - [x] Rooted payloads after a magic `@` prefix are rejected on every host (I2) — focused tests pass.
- [x] Prove Darkmatter composition/schema/reference/transclusion paths on the foundation side (I3, I9):
    - [x] Composition, schema, reference, transclusion focused tests pass.
- [x] Prove Sequence Plus retained behavior at its applicable tiers:
    - [x] JIT state visibility, tasks/groups, deterministic merges, process ownership.
    - [x] Source/list grammar, exact approved shell bytes, serial/parallel ordering.
- [x] Run supporting-package L1 + lint gates *(parallelizable across packages)*:
    - [x] From `biscuit-file/`, run `just test` and `just lint` independently.
    - [x] From `darkmatter/`, run `just test` and `just lint` independently. (The complete L1 set ran as four deterministic `just test --partition hash:N/4` invocations to honor the non-interactive command ceiling.)
    - [x] From the repository root, run `just test biscuit-test-harness` and `just _lint biscuit-test-harness` independently.
    - [x] From `claudine/rendezvous/`, run `just check`, `just test`, and `just lint` independently.
- [x] From `claudine/`, run full Claudine L1 and lint as independent commands: `just test`, then `just lint`. (The complete L1 set ran as eight deterministic `just test --partition hash:N/8` invocations after the monolithic command exceeded the non-interactive ceiling.)
- [ ] Run `git diff --check`, then scan both worktree and index with the anchored marker expression from Phase 1 (`git grep ...` and `git grep --cached ...`). Both scans must be empty after classified fixture exclusions, and the known `CLAUDE.md` markers must be gone (R16). (`git diff --check` and the precise worktree scan are clean. The untouched index still has the two classified Phase 1 lines because staging is prohibited; see `phase2-audit.md`.)
- [x] Inspect staged changes: `git diff --cached --stat`. Confirm only intended files are present; investigate any surprise. (Inspected: the staged diff is empty under the explicit no-stage instruction.)
- [x] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})`. Review affected symbols, processes, and modules; record in `impact/foundation-merge-detect.md`. (The cumulative foundation comparison was CRITICAL: 6,082 changed symbols across 737 files and 64 affected indexed processes; the review found that breadth consistent with the planned foundation scope.)
- [ ] Create the foundation merge commit with ancestry preserved and capture its SHA. (Requires the separately authorized history operation.)
- [ ] Refresh `git merge-tree --write-tree --name-only <foundation-merge-sha> e348486c` against the exact sequential inputs. Save command/output/status under `merge-previews/post-foundation-proxy/`, update `sha-ledger.md`, `conflict-checklist.md`, and any newly implicated impact reports. Do not treat the earlier feature-tip-to-feature-tip preview as the exact Phase 3 conflict list. (Requires the foundation merge SHA.)
- [ ] Run change detection and staged-diff/marker review for those planning-only updates, then create a documentation-only checkpoint commit so Phase 3 starts clean. (Requires separate staging/commit authorization.)

### Validation Checkpoint (Phase 2 exit)

- Foundation merge commit exists on the integration branch, ancestry preserved.
- All foundation-focused tests pass; L1/lint green across biscuit-file, darkmatter, biscuit-test-harness, rendezvous, claudine.
- Trunk `.gitignore` fix and trunk schemas/research/docs are present in the merged tree.
- `git diff --check` and both anchored marker scans are clean; the pre-existing `CLAUDE.md` marker debt is discharged.
- GitNexus change-detection report reviewed; no HIGH/CRITICAL surprise.
- Foundation merge SHA is committed in `sha-ledger.md`; the worktree is clean.
- The exact foundation-merge ↔ proxy preview is recorded, and every surfaced path is represented in the conflict/impact checklists.

---

## Phase 3 — Integrate `proxy-with` by dependency order

Spec mapping: spec Phase 2. Goal: one preparation service, one transition path, one resolution grammar, one diagnostic selector, one launch bundle, and zero conflict markers in staged or unstaged content. This is the highest-risk phase (R1–R6, R15, R16).

### Tasks

- [ ] Re-read impact reports for proxy-side symbols (coordinator, lifecycle executor, loop engine, launch plan, overlay, sequence runner). For a symbol that exists only on `proxy-with`, use the report generated from that exact feature tip; for a shared symbol, review both feature-tip reports. Clear every HIGH/CRITICAL item in `impact-review.md` before editing.
- [ ] Start the merge with ancestry preserved and without committing: `git merge --no-ff --no-commit e348486c`.

### Conflict clusters (resolve in this order)

- [ ] **Cluster 1 — Shared types, diagnostic shapes, transition errors.** Files: `composition/error/{mod,render/mod,tests}`, shared `types`. Owner per responsibility map: Claudine library lifecycle protocol + diagnostic registry. Resolve as state-machine behavior, not line-by-line text.
- [ ] **Cluster 2 — Darkmatter options and schema-stage contract.** Files: `darkmatter/lib/src/markdown/compose/context/options.rs` plus auto-merged schema validation, transclusion, reference graph, expression, schema-resolution surfaces (I9). Preserve deferred schema verdict (initialize may coerce; authoritative verdict after stabilized reread).
- [ ] **Cluster 3 — Canonical preparation and request/file-resolution context.** Files: `compose/prep.rs`, `wrap/composition/{mod,pipeline,runner}.rs`, `wrapper_stages.rs`, `composition/{mod,preflight,prepare,types}.rs` (I3, I4). One preparation service carrying `FileResolutionContext`. No reduced proxy composer or sequence-only materializer outside it.
- [ ] **Cluster 4 — Coordinator, lifecycle protocol, loop engine, transition ownership.** Files: `composition/lifecycle/{context,executor}`, `composition/looping/engine`, new `composition::coordinator`, preparation-stage, diagnostic registry, restored-diagnostic modules (I5). Coordinator is the sole owner allowed to resolve, prepare, validate, and commit a handoff. Failed handoffs must not partially activate, lose the triggering event, synthesize source completion, or emit closure twice.
- [ ] **Cluster 5 — CLI preparation, launch bundle, harness orchestration, retry/resume.** Files: `wrap/harness_orch/{loop_control,loop_control/control_dispatch,loop_control/proxy,prompt,types}.rs`, `overlay.rs`, launch adapters, environment sanitation, attempt classification, wrapper entry points, system-prompt lifetime, session reporting, new launch-plan modules (I7). The complete launch bundle is the sole source of both the compatibility key and spawn inputs. Retry/resume retain and refresh only documented facets and budgets.
- [ ] **Cluster 6 — Sequence Plus containment and task/group integration.** Files: `wrap/sequence/{iterate,mod,phase1c}.rs`, requeue/proxy tests, Sequence Plus task/group and task-stream modules (I6). A proxy selected inside a sequence step transfers active-document ownership without restarting, duplicating, or advancing the containing step. Sequence retains step/task/output ownership; adopted target owns document context, loop, lifecycle, launch, closure.
- [ ] **Cluster 7 — Terminal rendering, task framing, error emission.** Files: renderers, task-stream framing, any ad-hoc emission boundaries (I10). Exactly-once terminal behavior. `NO_COLOR`, non-TTY, Unicode fallback, OSC 8, concurrent attribution, idle flush, and close behavior remain informationally equivalent.
- [ ] **Cluster 8 — Tests, generated inventories, skills, docs, CI, repository guards.** Files: `dispatch-inventory.json`, `gen/tests/drift.rs`, `docs/topics/composition.md`, `.claude/skills/claudine/{SKILL,architecture}.md`, `.claudine/memory/commits.md`, `CLAUDE.md`, nextest profiles, `claudine-tests.yml`, guard scripts, shared `just` support. (Detailed reconciliation of generated/docs happens in Phase 4; here the goal is "no markers, no broken build, no stale guard.")

### Per-conflict protocol (applies to every file in every cluster)

For each conflict:

- [ ] Inspect merge base and both branch versions.
- [ ] Identify the final responsibility owner from the spec's Architectural Responsibility Map.
- [ ] Inspect callers, callees, and affected execution flows via `gitnexus context()` / `impact()`.
- [ ] Identify existing acceptance tests from both branches that bind this surface.
- [ ] Resolve the smallest coherent behavior unit — never broad `--ours` / `--theirs` (R15).
- [ ] Run focused tests for that unit before moving to the next cluster.
- [ ] Record rationale, owner, and evidence pointer in `conflict-checklist.md`.

### One-sided / auto-merged review (do not skip)

- [ ] Review `composition/target.rs`, launch adapters, environment sanitation, attempt classification, wrapper entry points, system-prompt lifetime, session reporting.
- [ ] Review Darkmatter schema validation, transclusion, reference graph, expression, schema-resolution paths.
- [ ] Review new `composition::coordinator`, preparation-stage, diagnostic registry, restored-diagnostic, Sequence Plus task/group, task-stream, launch-plan modules for duplicate responsibilities and stale adapters.
- [ ] Review completion adapters and all remaining private `@/` or relative-path rewrite candidates — eliminate any second grammar (I2).
- [ ] Review test harness backends, nextest config, CI filters, fail-closed tier behavior.
- [ ] Review the Rendezvous Windows `Connected` adapter and dependency gating (I12, R12).

### Invariant and responsibility audit

- [ ] Verify the Architectural Responsibility Map has exactly one owner per row in the merged tree.
- [ ] Verify invariants I1–I12 against merged source. For each, cite the appropriate merged-code test, source audit, generated guard, or platform evidence row in `acceptance-ledger.md`; do not invent a unit-test citation for documentation, ancestry, or native-platform obligations.

### Commit gates

- [ ] Run `git diff --check` and the anchored worktree/index marker scans from Phase 2. Both scans are empty after classified fixture exclusions.
- [ ] Inspect staged changes: `git diff --cached --stat`. No surprises.
- [ ] Run focused tests across all eight clusters, then Claudine L1 + lint.
- [ ] Run GitNexus `detect_changes({scope: "compare", base_ref: "main"})`. Save the report to `impact/proxy-merge-detect.md`; review unexpected symbols, execution flows, deleted guards, and duplicate paths.
- [ ] Create the proxy-with merge commit with ancestry preserved. Capture its SHA, update `sha-ledger.md`, run change detection and staged-diff/marker review for that ledger-only change, then create a documentation-only checkpoint commit so Phase 4 starts clean.

### Validation Checkpoint (Phase 3 exit)

- One preparation service, one transition path, one resolution grammar, one diagnostic selector, one launch bundle.
- `git diff --check` and the anchored worktree/index marker scans are clean.
- Every conflict row in `conflict-checklist.md` has owner, rationale, and evidence pointer.
- Responsibility map audit complete; one owner per row.
- I1–I12 each have applicable merged-tree evidence in the acceptance ledger.
- Claudine L1 + lint green.
- GitNexus change detection reviewed; no HIGH/CRITICAL surprise unresolved.
- Proxy merge SHA is committed in `sha-ledger.md`; the worktree is clean.

---

## Phase 4 — Regenerate and reconcile derived material

Spec mapping: spec Phase 3. Goal: derived artifacts describe the merged tree, not either branch. Most tasks here are independent and can be parallelized across owners.

### Tasks

- [ ] Regenerate `claudine/docs/providers/dispatch-inventory.json` with the command embedded by its owning test: `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run -p claudine-cli --test dispatch_inventory`. Review the resulting inventory before accepting it. *(parallelizable with: the generator check below)*
- [ ] Reconcile the conflict in `claudine/gen/tests/drift.rs` manually as test source; it is not generated output. Then run `cargo run -p claudine-gen -- check`. If merged inputs intentionally changed generated artifacts, run `cargo run -p claudine-gen -- generate`, review every proposed file, and rerun `check`.
- [ ] Run source-scan, generated-artifact, dispatch, and test-placement guards. Any guard that only passes by weakening scope halts work (spec Stop Conditions).
- [ ] Reconcile `.claude/skills/claudine/SKILL.md` against the merged responsibility map. *(parallelizable with: other doc reconciliations below)*
- [ ] Reconcile `.claude/skills/claudine/architecture.md` against the merged responsibility map. *(parallelizable)*
- [ ] Reconcile `claudine/docs/topics/composition.md` with the merged composition/preparation contract. *(parallelizable)*
- [ ] Reconcile lifecycle documentation with the single effective `DiagnosticSnapshot` selector and coordinator-owned transition state machine. *(parallelizable)*
- [ ] Reconcile file-reference documentation with the `FileReference` grammar, repository-first bare precedence, strict explicit-relative, and ordered probe detail (I2). *(parallelizable)*
- [ ] Reconcile schema documentation with the deferred schema-verdict ordering (I9). *(parallelizable)*
- [ ] Reconcile Sequence Plus documentation with the merged task/group containment model (I6). *(parallelizable)*
- [ ] Reconcile system-prompt topic and delivery documentation (trunk `.gitignore` fix preserved). *(parallelizable)*
- [ ] Reconcile completion documentation with the merged completion behavior. *(parallelizable)*
- [ ] Reconcile error-architecture documentation with the typed diagnostic registry and exactly-once terminal behavior (I1, I10). *(parallelizable)*
- [ ] Preserve trunk lifecycle-schema scaffolding and local-runner schemas/research (I11).
- [ ] Correct the stale proxy-with acceptance-map description of the Linux L2 workflow (called out by proxy review 18).
- [ ] Comment pass: verify inline `//` and `///` docs describe merged behavior. Treat code as authoritative unless an acceptance invariant proves the code wrong; otherwise fix or delete drift (AGENTS.md Comment Quality).
- [ ] Confirm no branch-local line-number inventories, hashes, or status claims were copied into the merged tree unchanged (R9).
- [ ] Run `git diff --check`, the anchored worktree/index marker scans, and GitNexus `detect_changes({scope: "compare", base_ref: "main"})`; review the complete staged diff and save the change report to `impact/reconciliation-detect.md`.
- [ ] Create a scoped reconciliation commit for generated artifacts, guards, skills, and documentation. Capture its SHA as the acceptance candidate; Phase 5 records it in `sha-ledger.md` alongside the evidence results.

### Validation Checkpoint (Phase 4 exit)

- The dispatch inventory and every `claudine-gen`-owned artifact match their generators from a clean merged checkout; `gen/tests/drift.rs` accurately tests the merged contract.
- All source-scan / drift / dispatch / test-placement guards pass without scope weakening.
- Every reconciled doc cites final merged behavior (responsibility map, invariants, grammar, stage matrix).
- No stale branch-local inventories remain.
- The reconciliation commit is clean and identified as the immutable acceptance-candidate SHA.

---

## Phase 5 — Layered merged-tree verification

Spec mapping: spec Phase 4. Goal: localize failures by running focused tests after each subsystem, then complete area gates. Do not debug all failures from one giant final run.

All evidence in this phase names the acceptance-candidate SHA from Phase 4. If
any production, test, generator, guard, or behavior-documentation file changes,
discard affected evidence, return to the owning earlier phase, create a new
candidate, and rerun the affected matrix.

### Tasks

- [ ] Biscuit File gates *(parallelizable with: other package-area gates below)*:
    - [ ] `cd biscuit-file && just test`
    - [ ] `cd biscuit-file && just test-l2`
    - [ ] `cd biscuit-file && just lint`
- [ ] Darkmatter gates *(parallelizable)*:
    - [ ] `cd darkmatter && just test`
    - [ ] `cd darkmatter && just test-l2`
    - [ ] `cd darkmatter && just lint`
- [ ] Biscuit Test Harness gates *(parallelizable)*:
    - [ ] From the repository root: `just test biscuit-test-harness`
    - [ ] From the repository root: `just _lint biscuit-test-harness`
- [ ] Rendezvous gates *(parallelizable)*:
    - [ ] `cd claudine/rendezvous && just check`
    - [ ] `cd claudine/rendezvous && just test`
    - [ ] `cd claudine/rendezvous && just lint`
- [ ] Claudine gates (sequential after the package-area gates above):
    - [ ] `cd claudine && just test`
    - [ ] `cd claudine && just test-l2 --no-fail-fast`
    - [ ] `cd claudine && just lint`
    - [ ] `cd claudine && just check-windows` (recorded as `compile-only`, NOT runtime evidence — I12, R11)
- [ ] Execute the 12 mandatory combined seam rows at the tier/platform assigned in Phase 1. They may live in several focused test binaries; the requirement is one stable `MM-Sxx` row per interaction and fresh merged-tree evidence, not one oversized test process:
    - [ ] `MM-S01` — A bare proxy target resolves repository-first; a missing target exposes the ordered candidate/probe diagnostic identically in terminal, `err.*`, and snapshot output.
    - [ ] `MM-S02` — An explicit-relative proxy target stays source-local without fallback; nested references derive from the target source/repository.
    - [ ] `MM-S03` — A `proxy.with` overlay enters a Sequence Plus step whose target owns a loop; the step runs once, the loop completes, and JIT state/output remain visible.
    - [ ] `MM-S04` — A proxied target's typed schema failure matches direct execution; source and target each receive exactly their owed lifecycle events.
    - [ ] `MM-S05` — Initialize-time and terminal-time handoff failures retain distinct triggering events, closure ownership, and diagnostic identity.
    - [ ] `MM-S06` — Overlay values survive retry, resume, and immediate-target loop refresh but remain absent from a downstream proxy unless supplied again.
    - [ ] `MM-S07` — Provider/model/MCP/credential changes across retry or resume rebuild the launch plan and yield typed incompatibility when a live session cannot be reused.
    - [ ] `MM-S08` — The session compatibility key derives from the exact launch bundle that is spawned.
    - [ ] `MM-S09` — Sequence/task preflight approves the exact shell bytes later executed after handoff and context re-anchoring.
    - [ ] `MM-S10` — A proxied-target failure inside a parallel group retains task attribution and stdout/stderr order, settles all children, merges state deterministically, and tears down descendants.
    - [ ] `MM-S11` — A target in another repository re-anchors authoring/nested resolution while the provider keeps the invocation-fixed child CWD; diagnostics distinguish the two contexts.
    - [ ] `MM-S12` — Dry-run reports static selection intent without lifecycle execution, dynamic proxy traversal, environment/MCP side effects, document mutation, or overlay disclosure.
- [ ] Capture L2 terminal evidence (plain / color / OSC 8 / concurrent task failure) through `just test-l2` only. Backend failures recorded separately from feature failures (R10).
- [ ] Schedule and run guarded L3 keyboard/process-interruption tests through attended/native or designated CI workflows only. Their opt-in and platform requirements remain visible in the ledger.
- [ ] Run required Linux L1 behavior and lints in CI against the acceptance-candidate SHA; run Windows L1 rows in CI where supported. Record native execution separately from cross-checks.
- [ ] Run the dedicated Linux tmux-backed L2 CI job (`.github/workflows/claudine-tests.yml`). It MUST reach assertions; a tmux setup failure is `blocked`, not a pass (R10, I12).
- [ ] Attach passing native Windows runtime evidence for Windows-specific terminal, named-pipe, console-control, HOME, and descendant-termination claims. `just check-windows` alone is `compile-only` and never satisfies a runtime row (R11, I12).
- [ ] Update every acceptance-ledger row touched in this phase with acceptance-candidate SHA, tier, platform, status, and evidence pointer.
- [ ] After every required row passes, record the acceptance-candidate SHA in `sha-ledger.md`, stage only the evidence/ledger updates, run `git diff --check`, the anchored marker scans, and GitNexus `detect_changes({scope: "compare", base_ref: "main"})`, then create a documentation-only evidence commit. Capture that commit's SHA for the Phase 6 closeout record; do not attempt to make a commit self-reference its own SHA.

### Validation Checkpoint (Phase 5 exit)

- Every required L1 and L2 assertion ran and passed; `compile-only` and `blocked` rows accurately labeled.
- All 12 seam cases reached their assertions and passed at their assigned tiers/platforms.
- Linux L2 CI reached its assertions and passed. A blocked backend is recorded accurately but prevents Phase 5 exit until a compliant rerun passes.
- Required Linux/Windows L1 and native Windows runtime rows passed; cross-check-only evidence remains labeled `compile-only`.
- All lints green.
- A documentation-only evidence commit records results against the unchanged acceptance candidate.

---

## Phase 6 — Final audit and closeout

Spec mapping: spec Phase 5. Goal: the completion definition from spec §"Completion Definition" is satisfied with a clean final tree and a fully populated acceptance ledger.

### Tasks

- [ ] From a clean merged checkout, run GitNexus `detect_changes({scope: "compare", base_ref: "main"})`. Review unexpected affected symbols, execution flows, deleted guards, and duplicate execution paths. Save the report to `impact/final-audit-detect.md`.
- [ ] Review the final `git diff` for unintended production, test, generated, and documentation changes.
- [ ] Verify generators and drift/source-scan guards pass from the clean merged checkout (not a stale worktree).
- [ ] Repeat full L1, L2, and lint gates from the clean checkout. Record fresh results.
- [ ] Attach native macOS evidence per ledger row requiring it.
- [ ] Attach native Linux evidence per ledger row requiring it (including the dedicated Linux L2 CI job).
- [ ] Attach native Windows runtime evidence per ledger row requiring it (distinct from `just check-windows`).
- [ ] Update all four source feature records (`2026-07-13-error-propogation`, `2026-07-13-file-resolution`, `2026-07-11-sequence-plus`, `2026-07-13-proxy-with`) and this mega-merge fix record with the acceptance-candidate SHA and fresh results.
- [ ] Verify completion definition from spec §"Completion Definition":
    - [ ] `claudine` contains both feature histories through reviewed merge commits.
    - [ ] All mandatory invariants (I1–I12) and the responsibility map hold in final source.
    - [ ] No duplicate legacy execution path or private resolution grammar remains.
    - [ ] Every conflict and semantic hotspot has a recorded resolution rationale.
    - [ ] All four feature specifications are mapped to fresh merged-code evidence.
    - [ ] Every required L1 and L2 test reached its assertion and passed.
    - [ ] Required L3 and native Windows evidence is attached and passed.
    - [ ] Package-area lints, drift guards, generators, source scans, and test-placement checks pass.
    - [ ] Documentation and skills describe the merged behavior.
    - [ ] A clean-checkout final audit finds no conflict markers, stale generated data, unexpected changes, or unowned acceptance rows.
- [ ] Move the four source feature records and this mega-merge fix record to their `_completed/` directories only after every required ledger row is `passed`. If the owner accepts residual debt, record its scope, consequence, mitigation, and follow-up, but leave the affected records active; accepted debt does not satisfy completion.
- [ ] Record the Phase 5 evidence-commit SHA in `sha-ledger.md`. After the record updates/moves, run `git diff --check`, anchored worktree/index marker scans, documentation/link guards, and GitNexus `detect_changes({scope: "compare", base_ref: "main"})`. Review the staged diff, then create the documentation-only closeout commit; its own SHA is the resulting branch tip and is not self-recorded.
- [ ] Re-verify that `claudine` and both feature refs still match `sha-ledger.md`. Switch to `claudine` and fast-forward it to the integration branch with `git merge --ff-only <integration-branch>`. If fast-forward is impossible, stop and review the unexpected movement; never force-update the branch.
- [ ] Close the SHA freeze on the three source branches.

### Validation Checkpoint (Phase 6 exit — merge complete)

- Clean-checkout audit clean.
- Acceptance ledger: every required row is `passed`; no `blocked`, `skipped`, `compile-only`, or `accepted-debt` row is being counted as acceptance.
- All four source feature records and this mega-merge fix record are updated with the acceptance-candidate SHA and moved to `_completed/`.
- Documentation-only evidence and closeout commits exist; all runtime evidence remains anchored to the unchanged acceptance-candidate SHA.
- `claudine` points to the integration closeout commit by fast-forward and contains both feature histories.
- Branch ref freeze lifted.
- Completion definition from spec §"Completion Definition" fully satisfied.

---

## Parallelization Summary

- **Phase 1**: baseline gate captures at the two feature tips are independent — run on separate checkouts in parallel. Impact precomputes across hot-spot symbols are independent — parallelizable.
- **Phase 2**: supporting-package L1/lint gates (biscuit-file, darkmatter, biscuit-test-harness, rendezvous) are independent — parallelizable. Claudine L1/lint runs after focused suites.
- **Phase 3**: conflict clusters are ordered by dependency; do NOT parallelize across clusters. Within a cluster, per-file `context()` / `impact()` lookups are parallelizable but resolution edits stay serialized.
- **Phase 4**: regenerate and doc-reconciliation tasks are largely independent — parallelizable across owners.
- **Phase 5**: package-area gates (biscuit-file, darkmatter, biscuit-test-harness, rendezvous) are independent — parallelizable. Claudine gates run after these. Native platform evidence collection (macOS/Linux/Windows) is parallelizable across hosts.
- **Phase 6**: inherently sequential final audit.

## Stop Conditions (apply at every checkpoint)

Pause and return to the last recoverable checkpoint if any of these occurs (spec §"Stop Conditions and Recovery"):

- Branch tips no longer match `sha-ledger.md`.
- The execution-seed range contains unreviewed implementation or the merge bases differ from the ledger.
- An unclassified conflict-marker hit appears, or a post-resolution marker scan is non-empty after documented fixture exclusions.
- Impact analysis reports HIGH or CRITICAL risk without a reviewed resolution.
- A conflict cannot be assigned to one owner in the responsibility map.
- Two preparation, transition, resolution, diagnostic-selection, or launch paths remain after resolution.
- Typed detail must be converted to text to cross a new in-process boundary.
- A proxy-in-sequence path cannot identify one owner for step advancement and one for document closure.
- Generated/drift guards pass only by weakening their scope.
- L2 or L3 infrastructure repeatedly fails before assertions and no compliant alternate backend or CI route exists.
- Unrelated dirty work prevents a reliable diff or abort.

Before any abort or retry, preserve the current conflict-resolution patch and ledger. Recoverable Git operations only.
