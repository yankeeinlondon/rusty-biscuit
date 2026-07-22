---
total_phases: 6
created: 2026-07-21
phase: 1
agent: "opencode/zai-coding-plan/glm-5.2"
yolo: "true"
---

# Claudine Mega-Merge Execution Plan

Derived from [`spec.md`](./spec.md) with the `claudine-log.md`, `error-prop-and-file-resolution-log.md`, `proxy-with-log.md`, and `conflict-report.md` inputs.

The spec defines six stages labelled Phase 0–5. Per execution-plan convention this plan renumbers the freeze stage as **Phase 1** and shifts every later stage by one, yielding six phases total. Phase naming in this document therefore differs from the spec by exactly one; invariant and risk-register IDs (I1–I12, R1–R15) are preserved verbatim.

## Conventions

- Every task is observable: it produces a commit, a file, a recorded gate result, a saved impact report, or an explicit `accepted-debt` ledger row.
- `(parallelizable with: …)` flags work that can be dispatched concurrently. Sequencing inside a phase is otherwise top-to-bottom in dependency order.
- Canonical commands: `just test` (L1, nextest), `just test-l2` (real-terminal, fail-closed), `just lint`. Never `cargo test`; never `cargo fmt` / `rustfmt` write-mode.
- GitNexus `impact()` is mandatory before editing any conflicted symbol; HIGH/CRITICAL findings halt for review (R14, R15).
- Stop conditions (spec §"Stop Conditions and Recovery") apply at every checkpoint. Recovery uses recoverable Git operations only — never a destructive reset on a broad path or dirty worktree (R13).

## SHA Ledger (authoritative inputs)

| Branch | Reviewed tip | Merge base with `claudine` |
|---|---|---|
| `claudine` (integration seed) | `d2d0a8fc4467230ed78e2a1d3146b7c336cc17fd` | — |
| `error-prop-and-file-resolution` | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` | `8fc8711434b01327479297af9b40a67685409d00` |
| `proxy-with` | `e348486c810969abe87a6b7209979034f5454b07` | `6cdb8bf56321c3747d5ea16a1241e47c2bff7fce` |

The pre-merge preview MUST be refreshed against the integration seed in Phase 1 because the conflict report predates it. If any ref moves after the ledger is written, the new SHA becomes a reviewed input only after the conflict inventory and baseline evidence are regenerated (R14).

---

## Phase 1 — Freeze, protect, and baseline

Spec mapping: spec Phase 0. Goal: inputs recoverable, refs frozen, baselines recorded, acceptance ledger exists. No merge work begins in this phase.

### Tasks

- [ ] Write `sha-ledger.md` in this directory recording the three tip SHAs and both merge bases from the table above. This is the authoritative ref freeze for the rest of the plan.
- [ ] Verify each SHA against `git rev-parse <branch>` and confirm no divergence from the spec.
- [ ] Quarantine and preserve dirty/untracked worktree inputs (R13): back up `.claudine/memory/commits.md`, `CLAUDE.md`, untracked `.claude/settings.local.json`, and this entire `claudine/fixes/2026-07-20-claudine-mega-merge/` directory plus `darkmatter/fixes/2026-07-20-mega-merge/` to a recoverable copy or a dedicated commit on a scratch branch. Do not overwrite.
- [ ] Confirm the actual integration worktree is clean: `git status --short --branch` shows nothing unexpected. If it cannot be clean, halt (spec Stop Conditions).
- [ ] Create the recoverable integration branch from the frozen `claudine` seed (`d2d0a8f`). Keep all three source branches unchanged; full ancestry must survive through merge commits.
- [ ] Freeze the three branch refs (stop feature work on them until Phase 6 closes). Announce the freeze to anyone with push access.
- [ ] Enable `git rerere` on the integration branch in case of repeated attempts. Reused resolutions still require per-conflict review.
- [ ] Refresh non-mutating merge previews against the frozen SHAs: `git merge-tree --write-tree --name-only d2d0a8f 43c23c6` and `git merge-tree --write-tree --name-only d2d0a8f e348486c`, plus the two-feature preview `git merge-tree --write-tree --name-only 43c23c6 e348486c`. Save raw output to `merge-previews/`.
- [ ] Export the 36-path conflict inventory into `conflict-checklist.md`, grouped by cluster (Architecture & records; CLI prep & composition; Harness & proxy routing; Sequence integration; Library composition/lifecycle; Darkmatter; Generated/docs). Each row gets: path, cluster, owner per responsibility map, resolution status, rationale slot, evidence slot.
- [ ] Capture baseline gate results at `error-prop-and-file-resolution` tip (`43c23c6`). Save to `baselines/error-prop-and-file-resolution/`. *(parallelizable with: the proxy-with baseline below)*
    - [ ] `cd biscuit-file && just test && just test-l2 && just lint`
    - [ ] `cd darkmatter && just test && just test-l2 && just lint`
    - [ ] `cd biscuit-test-harness && just test && just lint`
    - [ ] `cd claudine/rendezvous && just check && just test && just lint`
    - [ ] `cd claudine && just test && just test-l2 --no-fail-fast && just lint && just check-windows`
- [ ] Capture baseline gate results at `proxy-with` tip (`e348486c`). Save to `baselines/proxy-with/`. *(parallelizable with: the error-prop-and-file-resolution baseline above)* — same command set, run against the `proxy-with` ref (clean worktree or dedicated checkout).
- [ ] Label red, timed-out, skipped, cross-compile-only, and backend-blocked baseline results truthfully. Backend denial (e.g. tmux setup failure on managed macOS) is recorded as debt, never silently waived (R10, R11).
- [ ] Build `acceptance-ledger.md` from the four feature specs + Sequence Plus validation matrix + proxy-with acceptance map. Each row: criterion ID, contract, test/matrix row, tier (L1/L2/L3), platform, owner, status (`passed`/`failed`/`blocked`/`skipped`/`compile-only`/`accepted-debt`), merged SHA slot, evidence slot.
- [ ] Import the 10 mandatory combined seam cases (spec §"Mandatory combined seam cases") as their own ledger block — these cannot be satisfied by isolated feature suites.
- [ ] Confirm the GitNexus index is current against the frozen `claudine` seed. If stale, refresh with `node .gitnexus/run.cjs analyze` (fallback `npx gitnexus analyze`; npm 11 crash → `npm i -g gitnexus`).
- [ ] Precompute impact analyses for hot-spot symbols and save to `impact/`. Required before any Phase 2/3 edit:
    - [ ] `prep` (compose), `composition::{mod,pipeline,runner}` (wrap), `wrapper_stages`
    - [ ] `loop_control`, `control_dispatch`, `proxy` (harness_orch), `overlay`, `prompt`, `types`
    - [ ] `sequence::{iterate,mod,phase1c}`
    - [ ] `composition::error::{mod,render}`, `composition::lifecycle::{context,executor}`, `composition::looping::engine`, `composition::{mod,preflight,prepare,types}`
    - [ ] Darkmatter `markdown::compose::context::options`
- [ ] For every HIGH/CRITICAL impact finding, open a row in `impact-review.md` with the proposed resolution owner. These must be reviewed before the corresponding Phase 2/3 edit lands.

### Validation Checkpoint (Phase 1 exit)

- `sha-ledger.md` exists and matches `git rev-parse` for all three refs.
- `git status --short --branch` is clean in the integration worktree.
- Integration branch exists, based on `d2d0a8f`.
- `baselines/error-prop-and-file-resolution/` and `baselines/proxy-with/` each contain the five area gate logs (or explicitly labeled backend debt).
- `conflict-checklist.md` has all 36 paths plus the auto-merged/one-sided review list from spec §"Conflict Review Scope".
- `acceptance-ledger.md` exists with every spec criterion, every Sequence Plus matrix row, every proxy-with acceptance row, and all 10 seam cases as open rows.
- `impact/` contains a report per hot-spot symbol; `impact-review.md` queues HIGH/CRITICAL findings.

---

## Phase 2 — Integrate the foundation branch (`error-prop-and-file-resolution`)

Spec mapping: spec Phase 1. Goal: a known-good foundation merge commit carrying typed diagnostics → `FileReference`/request context → Sequence Plus. Failures here MUST be resolved before proxy-with is introduced so later regressions stay attributable (R1).

### Tasks

- [ ] Re-read the impact reports queued for foundation-side symbols from `impact/`. Resolve or de-scope each HIGH/CRITICAL item with a recorded rationale before touching the corresponding file.
- [ ] Start the merge non-fast-forward and uncommitted: `git merge --no-ff --no-commit 43c23c6`.
- [ ] Resolve the three predicted trunk conflicts in repository records and shared prompts:
    - [ ] `.claudine/memory/commits.md` — reconcile history, keep it factual and final-architecture-aligned.
    - [ ] `CLAUDE.md` — keep trunk content; merge in foundation additions where they describe behavior that survives.
    - [ ] `prompts/_implement/implement-suggestions.md` (and any other shared prompt files surfaced by the refreshed preview).
- [ ] Manually audit the auto-merged composition pipeline, wrapper stages, composition/system-prompt topics. A clean textual merge is not semantic compatibility (R7).
- [ ] Verify the trunk's transient system-prompt `.gitignore` fix survives (I11). Re-apply if the merge dropped it.
- [ ] Verify trunk lifecycle-schema scaffolding, local-runner schemas/research, shared prompt changes, and topic documentation are present (I11).
- [ ] Prove the typed diagnostic subsystem before any consumer edit (I1):
    - [ ] Run focused tests for the diagnostic registry, snapshots/restoration, and source/transport guards.
    - [ ] Confirm `DiagnosticSnapshot` is the effective selector across renderer, `err.*`, and serialization paths.
- [ ] Prove the Biscuit File authority (I2):
    - [ ] Grammar, kind classification, candidate planning, probes — focused tests pass.
    - [ ] Repository-first bare resolution and strict explicit-relative (no fallback) — focused tests pass.
    - [ ] `FileResolutionContext` derivation and completion/execution round trips — focused tests pass.
    - [ ] Rooted payloads after a magic `@` prefix are rejected on every host (I2) — focused tests pass.
- [ ] Prove Darkmatter composition/schema/reference/transclusion paths on the foundation side (I3, I9):
    - [ ] Composition, schema, reference, transclusion focused tests pass.
- [ ] Prove Sequence Plus retained behavior at its applicable tiers:
    - [ ] JIT state visibility, tasks/groups, deterministic merges, process ownership.
    - [ ] Source/list grammar, exact approved shell bytes, serial/parallel ordering.
- [ ] Run supporting-package L1 + lint gates *(parallelizable across packages)*:
    - [ ] `cd biscuit-file && just test && just lint`
    - [ ] `cd darkmatter && just test && just lint`
    - [ ] `cd biscuit-test-harness && just test && just lint`
    - [ ] `cd claudine/rendezvous && just check && just test && just lint`
- [ ] Run full Claudine L1 + lint: `cd claudine && just test && just lint`.
- [ ] Scan both worktree and staged blobs for conflict markers: `git diff --check` and a staged-blob scan (e.g. `git ls-files -s | ...` grep for `<<<<<<<`/`=======`/`>>>>>>>`). Both must be clean.
- [ ] Inspect staged changes: `git diff --cached --stat`. Confirm only intended files are present; investigate any surprise.
- [ ] Run `gitnexus detect_changes` against `main`. Review affected symbols, processes, and modules; record in `impact/foundation-merge-detect.md`.
- [ ] Create the foundation merge commit with ancestry preserved. Record the new SHA in `sha-ledger.md`.

### Validation Checkpoint (Phase 2 exit)

- Foundation merge commit exists on the integration branch, ancestry preserved.
- All foundation-focused tests pass; L1/lint green across biscuit-file, darkmatter, biscuit-test-harness, rendezvous, claudine.
- Trunk `.gitignore` fix and trunk schemas/research/docs are present in the merged tree.
- `git diff --check` clean; staged-blob marker scan clean.
- GitNexus change-detection report reviewed; no HIGH/CRITICAL surprise.
- Foundation SHA recorded in `sha-ledger.md`.

---

## Phase 3 — Integrate `proxy-with` by dependency order

Spec mapping: spec Phase 2. Goal: one preparation service, one transition path, one resolution grammar, one diagnostic selector, one launch bundle, and zero conflict markers in staged or unstaged content. This is the highest-risk phase (R1–R6, R15).

### Tasks

- [ ] Re-read impact reports for proxy-side symbols (coordinator, lifecycle executor, loop engine, launch plan, overlay, sequence runner). Clear every HIGH/CRITICAL item in `impact-review.md` with a recorded rationale before editing.
- [ ] Start the merge uncommitted: `git merge --no-commit e348486c`.

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
- [ ] Verify invariants I1–I12 hold against merged source. For each, cite the merged-code test that proves it in `acceptance-ledger.md`.

### Commit gates

- [ ] Scan both worktree and staged blobs for conflict markers. Both clean.
- [ ] Inspect staged changes: `git diff --cached --stat`. No surprises.
- [ ] Run focused tests across all eight clusters, then Claudine L1 + lint.
- [ ] Run `gitnexus detect_changes` against `main`. Save to `impact/proxy-merge-detect.md`. Review unexpected symbols, execution flows, deleted guards, duplicate paths.
- [ ] Create the proxy-with merge commit with ancestry preserved. Record SHA in `sha-ledger.md`.

### Validation Checkpoint (Phase 3 exit)

- One preparation service, one transition path, one resolution grammar, one diagnostic selector, one launch bundle.
- Zero conflict markers in unstaged or staged content.
- Every conflict row in `conflict-checklist.md` has owner, rationale, and evidence pointer.
- Responsibility map audit complete; one owner per row.
- I1–I12 each have a merged-code test citation in the acceptance ledger.
- Claudine L1 + lint green.
- GitNexus change detection reviewed; no HIGH/CRITICAL surprise unresolved.

---

## Phase 4 — Regenerate and reconcile derived material

Spec mapping: spec Phase 3. Goal: derived artifacts describe the merged tree, not either branch. Most tasks here are independent and can be parallelized across owners.

### Tasks

- [ ] Regenerate `claudine/docs/providers/dispatch-inventory.json` with its owning tool. *(parallelizable with: other regenerate tasks below)*
- [ ] Regenerate `claudine/gen/tests/drift.rs` fixture with `claudine-gen`. *(parallelizable)*
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

### Validation Checkpoint (Phase 4 exit)

- Regenerated files match their generators from a clean merged checkout.
- All source-scan / drift / dispatch / test-placement guards pass without scope weakening.
- Every reconciled doc cites final merged behavior (responsibility map, invariants, grammar, stage matrix).
- No stale branch-local inventories remain.

---

## Phase 5 — Layered merged-tree verification

Spec mapping: spec Phase 4. Goal: localize failures by running focused tests after each subsystem, then complete area gates. Do not debug all failures from one giant final run.

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
    - [ ] `cd biscuit-test-harness && just test`
    - [ ] `cd biscuit-test-harness && just lint`
- [ ] Rendezvous gates *(parallelizable)*:
    - [ ] `cd claudine/rendezvous && just check`
    - [ ] `cd claudine/rendezvous && just test`
    - [ ] `cd claudine/rendezvous && just lint`
- [ ] Claudine gates (sequential after the package-area gates above):
    - [ ] `cd claudine && just test`
    - [ ] `cd claudine && just test-l2 --no-fail-fast`
    - [ ] `cd claudine && just lint`
    - [ ] `cd claudine && just check-windows` (recorded as `compile-only`, NOT runtime evidence — I12, R11)
- [ ] Run the 10 mandatory combined seam cases (spec §"Mandatory combined seam cases") as a single suite at the appropriate tier. Each must reach its assertion:
    - [ ] 1. Bare proxy target resolves repository-first; missing target exposes ordered candidate/probe diagnostic identically in terminal, `err.*`, snapshot.
    - [ ] 2. Explicit relative proxy target stays source-local; nested references derive from target source/repository.
    - [ ] 3. `proxy.with` overlay into a Sequence Plus step whose target owns a loop: step runs once, loop completes, JIT state/output visible.
    - [ ] 4. Proxied target typed schema failure matches direct execution; source and target each receive their owed lifecycle events.
    - [ ] 5. Initialize-time vs terminal-time handoff failures retain distinct triggering events, closure ownership, diagnostic identity.
    - [ ] 6. Overlay values survive retry, resume, immediate-target loop refresh; absent from downstream proxy unless resupplied.
    - [ ] 7. Provider/model/MCP/credential changes across retry/resume rebuild launch plan; typed incompatibility when a live session cannot be reused.
    - [ ] 8. Session compatibility key derived from the exact spawned launch bundle.
    - [ ] 9. Sequence/task preflight approves the exact shell bytes later executed after handoff and context re-anchoring.
    - [ ] 10. Failure from a proxied target inside a parallel group: correct attribution, stdout/stderr ordering preserved, all children settled, deterministic state merge, process descendants torn down.
- [ ] Capture L2 terminal evidence (plain / color / OSC 8 / concurrent task failure) through `just test-l2` only. Backend failures recorded separately from feature failures (R10).
- [ ] Schedule and run guarded L3 keyboard/process-interruption tests through attended/native or designated CI workflows only. Their opt-in and platform requirements remain visible in the ledger.
- [ ] Run the dedicated Linux tmux-backed L2 CI job (`.github/workflows/claudine-tests.yml`). It MUST reach assertions; a tmux setup failure is `blocked`, not a pass (R10, I12).
- [ ] Attach native Windows runtime evidence for Windows-specific terminal, named-pipe, console-control, HOME, and descendant-termination claims. `just check-windows` alone is `compile-only` and never satisfies a runtime row (R11, I12).
- [ ] Update every acceptance-ledger row touched in this phase with merged SHA, tier, platform, status, and evidence pointer.

### Validation Checkpoint (Phase 5 exit)

- Every required L1 and L2 assertion ran and passed; `compile-only` and `blocked` rows accurately labeled.
- All 10 seam cases reached their assertions.
- Linux L2 CI job reached assertions; any backend denial recorded as debt.
- Native Windows runtime evidence attached where the platform matrix requires it.
- All lints green.

---

## Phase 6 — Final audit and closeout

Spec mapping: spec Phase 5. Goal: the completion definition from spec §"Completion Definition" is satisfied with a clean final tree and a fully populated acceptance ledger.

### Tasks

- [ ] From a clean merged checkout, run `gitnexus detect_changes` against `main`. Review unexpected affected symbols, execution flows, deleted guards, and duplicate execution paths. Save to `impact/final-audit-detect.md`.
- [ ] Review the final `git diff` for unintended production, test, generated, and documentation changes.
- [ ] Verify generators and drift/source-scan guards pass from the clean merged checkout (not a stale worktree).
- [ ] Repeat full L1, L2, and lint gates from the clean checkout. Record fresh results.
- [ ] Attach native macOS evidence per ledger row requiring it.
- [ ] Attach native Linux evidence per ledger row requiring it (including the dedicated Linux L2 CI job).
- [ ] Attach native Windows runtime evidence per ledger row requiring it (distinct from `just check-windows`).
- [ ] Update all four feature records (`2026-07-13-error-propogation`, `2026-07-13-file-resolution`, `2026-07-11-sequence-plus`, `2026-07-13-proxy-with`) with the final merged SHA and fresh results.
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
- [ ] Move feature/fix records to `_completed/` only after every required criterion is `passed` or explicitly accepted as documented residual debt by the owner (`accepted-debt` with scope, consequence, mitigation, follow-up — never silently waived).
- [ ] Close the SHA freeze on the three source branches.

### Validation Checkpoint (Phase 6 exit — merge complete)

- Clean-checkout audit clean.
- Acceptance ledger: every required row `passed` or explicit `accepted-debt` with documented residual.
- All four feature records updated with merged SHA and moved to `_completed/` (or residual debt precisely documented and owner-accepted).
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
- Impact analysis reports HIGH or CRITICAL risk without a reviewed resolution.
- A conflict cannot be assigned to one owner in the responsibility map.
- Two preparation, transition, resolution, diagnostic-selection, or launch paths remain after resolution.
- Typed detail must be converted to text to cross a new in-process boundary.
- A proxy-in-sequence path cannot identify one owner for step advancement and one for document closure.
- Generated/drift guards pass only by weakening their scope.
- L2 or L3 infrastructure repeatedly fails before assertions and no compliant alternate backend or CI route exists.
- Unrelated dirty work prevents a reliable diff or abort.

Before any abort or retry, preserve the current conflict-resolution patch and ledger. Recoverable Git operations only.
