---
total_phases: 6
created: 2026-08-12
phase: 1
agent: codex/default
yolo: "true"
---

# Execution Plan: Anchor Prepared `ctx.*` to the Invocation Launch Context

Reference: [`spec.md`](spec.md)

## Goal

Make every canonical Claudine composition route project plain `ctx.*` from the
caller's immutable launch context, while preserving the active document's
`SourceContext` for file and schema resolution. Each document preparation epoch
must construct one target-adjusted early-binding snapshot and reuse that exact
snapshot through preflight, composition, loops, and lifecycle execution.

## Completion contract

The fix is complete when:

- moving a prompt, task, group, overlay, or system-prompt file cannot change
  launch-facing plain `ctx.*` values;
- launch CWD and launch repository/topology evidence can only be paired through
  an `InvocationContext` API;
- direct, inline, loop, proxy, retry/resume, sequence, harness, overlay, and
  system-prompt paths use the canonical launch capture;
- one document preparation epoch reuses one exact target-adjusted snapshot;
- `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` retain resolved-target
  precedence;
- source-relative document references, eager `file(...)` values, and `$schema`
  resolution retain their existing behavior;
- no additional ambient CWD, HOME, environment, Git, or topology discovery is
  introduced; and
- the capture-owner inventory guard and all Claudine quality gates pass.

## Dependency order

```text
Phase 1: baseline and route inventory
    -> Phase 2: invocation-owned launch capture seam
        -> Phase 3: direct/inline/loop/lifecycle epoch migration
        -> Phase 4: sequence migration
            -> Phase 5: auxiliary routes, drift guard, and documentation
                -> Phase 6: acceptance matrix and final validation
```

Phases 3 and 4 are parallelizable after Phase 2. The route tracks called out
inside Phase 5 are also parallelizable after the relevant Phase 3 or Phase 4
owner has landed.

## Phase 1 — Freeze the failure matrix and capture inventory

**Objective:** establish an observable baseline and a reviewed classification
of every production capture before changing ownership.

### Tasks

- [ ] Add CLI-seam regression fixtures that run equivalent documents from one
  launch directory while storing them at the repository root, in the launch
  package area, in an opposing package area, and in an external repository;
  record the currently wrong `ctx.area`/`ctx.repo_root` results for body,
  frontmatter, `when:`, and lifecycle surfaces (AC1–AC4).
- [ ] Add the inverse fixture: launch outside every repository while the prompt
  lives inside the Rusty Biscuit repository, and assert the eventual contract
  is an absent launch repository and package area rather than prompt-derived
  facts (AC3).
- [ ] Inventory every production `ComposeContext::capture*` and
  `InvocationContext::runtime_evidence` call under `claudine/lib/src` and
  `claudine/cli/src`; classify each as canonical prepared context, explicit
  live `current.ctx.*`, library compatibility fallback, or unrelated
  documentation command (AC12).
- [ ] Record the canonical migration list at minimum for direct compose
  preflight/preparation, sequence root and referenced-document preflight, JIT
  templates, task execution, lifecycle pipeline re-materialization, overlays,
  passthrough harnesses, harness prompt preparation, and system-prompt
  preparation.
- [ ] Add or extend request-local work-accounting assertions around the baseline
  routes so later tests can detect extra Git discovery, topology work,
  environment capture, or runtime-evidence initialization without timing-based
  assertions (AC5, AC11).
- [ ] Review the comments adjacent to the direct compose and lifecycle capture
  sites; mark comments that already state launch anchoring for preservation and
  comments that describe source-backed `ctx.*` for correction when their code
  changes.

### Validation checkpoint

- [ ] Run the focused regression set with nextest and confirm it fails for the
  source-versus-launch distinction, not because of provider availability,
  ambient user configuration, or interactive terminal behavior.
- [ ] Review the inventory against the spec's complete route list before Phase
  2; no unclassified production capture may proceed into migration.

## Phase 2 — Add the invocation-owned launch capture seam

**Objective:** make launch anchor and launch evidence an inseparable operation
owned by `InvocationContext`.

### Tasks

- [ ] Add `InvocationContext::capture_launch_context` (or an equivalently named
  API) accepting `ContextRequirements` and returning a `ComposeContext`
  captured at `launch_cwd()` from the retained launch repository/topology,
  environment, and host evidence (D1, D2).
- [ ] Refactor the internal runtime-evidence population so launch and source
  projections reuse the existing caches and work counters, but the launch path
  operates directly on the retained launch entry; do not fabricate a
  `SourceContext`, prompt path, or file-resolution context (AC11).
- [ ] Ensure demand-driven groups retain their existing semantics: Git/repo
  facts come from the launch repository, source-scanned groups use the launch
  base when requested, environment comes from the invocation snapshot, and
  host groups use the invocation's single-flight caches.
- [ ] Add request-local accounting for launch-context construction or an
  equivalent observable capture identity so tests can distinguish one reused
  snapshot from two separately constructed but equal snapshots (AC5).
- [ ] Keep `runtime_evidence(&SourceContext, ...)` available for genuinely
  source-specific compatibility consumers, and keep live `current.ctx.*`
  capture behavior unchanged.
- [ ] Add L1 tests proving launch capture reports the launch package/repository
  when the supplied document source is in the same area, an opposing area, or
  another repository, and reports no repository facts when launched outside a
  repository (AC2, AC3).
- [ ] Add L1 work-bound tests proving repeated launch projections reuse retained
  Git/topology/environment/host evidence and do not consult ambient CWD or HOME
  after invocation capture (AC11).
- [ ] Build all path fixtures with `Path`/`PathBuf` and platform-aware temporary
  directories; avoid rendered-prefix, separator, case-folding, or canonical-path
  assumptions that fail on Windows or macOS symlinked temp roots (AC13).

### Validation checkpoint

- [ ] Run `just test` and `just lint` from `claudine/`; confirm the new API is
  additive and existing compatibility callers remain green before migrations.
- [ ] Inspect request-local counters for the new unit matrix and confirm launch
  projection adds no Git root discovery or topology probe.

## Phase 3 — Make direct, inline, loop, and lifecycle execution share one epoch snapshot

**Objective:** create the target-adjusted early-binding snapshot once per active
document entry and pass it through every stage of that epoch.

**Parallelization:** may run in parallel with Phase 4 after Phase 2.

### Tasks

- [ ] Move context construction in
  `claudine/cli/src/commands/compose/prep.rs` to the canonical document-epoch
  owner after provider/model resolution and before shell preflight; call the
  launch-capture API once, then apply the resolved target's `env_overrides`
  once (D4, AC5, AC6).
- [ ] Pass that exact snapshot to the narrow/full shell preflight
  `ComposeOptions`, `PrepareOptions.prepared_context`, body and effective
  frontmatter composition, schema evaluation, loop seed/condition work, and
  lifecycle execution; remove the second capture currently performed before
  main preparation (AC1, AC2, AC5).
- [ ] Change lifecycle pipeline setup to consume the prepared epoch snapshot
  carried by the execution request instead of rebuilding a context from the
  composed prompt and active `SourceContext`; leave event-time `current.ctx.*`
  capture separate and live (AC5, AC8).
- [ ] Make loop iterations retain the epoch snapshot and prove that loop
  condition/body/lifecycle reads do not trigger a new launch capture (AC1,
  AC5, AC8).
- [ ] Route proxy target entry, retry, and resume through the same canonical
  fresh-epoch owner: each fresh read may create one new snapshot, but every
  launch-facing field must still come from the original invocation and every
  target identity override must be reapplied (AC6, AC8).
- [ ] Preserve the active `SourceContext` and its
  `FileResolutionContext` in all options used for `$schema`, transclusion,
  provenance, and eager `file(...)` handling; do not promote
  `file_ref_fallback_dir` into a file-resolution candidate (AC10).
- [ ] Correct capture-site comments that claim two separately constructed
  contexts are equivalent or that prepared `ctx.*` is source-backed; retain
  comments whose launch-area contract becomes true after this phase.

### Tests

- [ ] Add real CLI-seam direct and inline tests for repository-root versus
  package-area prompts, opposing launch/source areas, and external sources;
  assert body, effective frontmatter, `when:`, preflight-expanded command
  bytes, and lifecycle all report launch facts (AC1–AC4).
- [ ] Add a deterministic snapshot-identity/counter test proving preflight,
  body, effective frontmatter, and lifecycle consumed one capture rather than
  separately equal captures (AC5).
- [ ] Add direct, loop, proxy, retry, and resume tests asserting target-specific
  `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL`, plus launch-stable
  repository/package facts (AC6, AC8).
- [ ] Add conflicting launch/source file fixtures proving body references,
  eager `file(...)`, and `$schema` remain source-relative/repository-first
  while plain `ctx.*` remains launch-relative (AC10).

### Validation checkpoint

- [ ] Run the focused compose, inline-compose, looping, lifecycle, proxy, and
  retry/resume nextest suites; confirm the exact-snapshot counter and all
  target-identity assertions pass.
- [ ] Run `just test` from `claudine/` before merging with the sequence track.

## Phase 4 — Migrate static sequence preflight and per-task epochs

**Objective:** make every sequence phase use launch facts without sacrificing
source-specific resolution or per-task target identity.

**Parallelization:** may run in parallel with Phase 3 after Phase 2.

### Tasks

- [ ] Replace the root sequence graph capture in
  `claudine/cli/src/commands/wrap/sequence/mod.rs` with one base launch context
  from the invocation owner; use it for launch-facing graph expressions before
  per-task target selection (D4, AC7).
- [ ] Update referenced-document shell resolution in
  `composition/sequence/preflight` to use the graph's launch context while
  retaining each origin document's `SourceContext` and
  `FileResolutionContext`; remove source-anchored recapture from
  `resolve_shell_bytes` (AC7, AC10).
- [ ] Update preflight approval composition and JIT template preflight to clone
  or project the launch base, then apply the selected task target's environment
  overrides where selection is available; approved and executed command bytes
  must retain the existing equality guard (AC6, AC7).
- [ ] Update sequence task/group/prompt preparation and
  `prepare_task_context` so each task epoch gets one target-adjusted launch
  snapshot and reuses it through task composition and execution (AC5–AC7).
- [ ] Ensure a sequence file, nested prompt, task, and group distributed across
  different repositories never substitute a source repository for the launch
  repository, including serial and parallel groups (AC7).
- [ ] Keep source snapshots and source-relative selection/workspace behavior
  unchanged wherever the current contract explicitly requires them (D3).

### Tests

- [ ] Add a sequence matrix covering root graph expressions, referenced task
  and group expressions, nested prompt documents, template preflight, and task
  execution with launch/source files split across repositories (AC7).
- [ ] Add serial and parallel task tests proving per-task agent/model overrides
  are visible in both `ctx.*` and `env.*` without changing launch repo/area
  values (AC6, AC7).
- [ ] Add sequence conflict fixtures proving source-relative `$schema` and file
  references still choose the authored document's source while graph/task
  `ctx.*` chooses the invocation launch (AC10, AC13).

### Validation checkpoint

- [ ] Run the focused sequence preflight, JIT, task, serial-group, and
  parallel-group nextest suites, including approved-byte equality tests.
- [ ] Run `just test` from `claudine/` after combining Phases 3 and 4; all
  canonical document routes must now use the launch-capture seam.

## Phase 5 — Migrate auxiliary routes, install the drift guard, and update contracts

**Objective:** close every remaining canonical capture path and prevent the
source-anchored pattern from returning.

**Parallelization:** Tracks A–C are parallelizable after Phases 3 and 4 expose
the final epoch/snapshot interfaces.

### Track A — System-prompt preparation

- [ ] Replace file-backed and built-in system-prompt/appendix runtime captures
  with the invocation launch-capture API; use one shared launch snapshot for
  the composed system-prompt bundle and retain each input's source context for
  its file/schema resolution (AC9, AC10).
- [ ] Add relocation tests that move primary and appendix sources between the
  repository root, package areas, and an external repository without changing
  their launch-facing expansion (AC9).

### Track B — Overlay, harness, passthrough, and re-materialization paths

- [ ] Update overlay preparation, passthrough harness seed materialization,
  harness prompt preparation, and loop-control target launch to receive the
  active epoch snapshot or create it through the canonical launch owner; remove
  direct source-anchored prepared-context captures (AC8, AC9).
- [ ] Prove re-materialization reuses the active epoch snapshot when the active
  document has not changed and starts exactly one fresh epoch on proxy,
  retry, or resume entry (AC5, AC8).
- [ ] Add relocation/conflict tests for overlays and harness prompts that
  separately assert launch-facing `ctx.*` and source-facing file resolution
  (AC9, AC10).

### Track C — Capture-owner inventory and documentation

- [ ] Add a production-source inventory test that rejects new direct prepared
  `ComposeContext::capture*` calls outside `InvocationContext` and the canonical
  epoch owner; use explicit, reason-bearing allowlist entries only for public
  library compatibility paths, the `claudine context --values` command, and
  live `current.ctx.*` capture sites (AC12).
- [ ] Make the guard scan path-normalized Rust source locations so it runs on
  macOS, Windows, and Linux without separator or case assumptions (AC12, AC13).
- [ ] Update relevant Claudine architecture/composition/system-prompt docs and
  the mirrored `.claude/skills/claudine/` topic documents to describe the
  invocation-owned launch snapshot, per-document epoch reuse, source-context
  separation, and unchanged `current.ctx.*` semantics.
- [ ] Remove or rewrite only comments/docs that drifted from the repaired
  behavior; do not alter unrelated comments or introduce implementation
  narration.

### Validation checkpoint

- [ ] Run focused system-prompt, overlay, harness, passthrough, and
  re-materialization nextest suites and confirm relocation does not affect
  launch-facing values.
- [ ] Seed the inventory fixture with a forbidden direct capture and confirm the
  guard fails with the source path, then restore the fixture and confirm it
  passes.
- [ ] Run `just test` and `just lint` from `claudine/` after all three tracks
  merge.

## Phase 6 — Close the acceptance matrix and run final quality gates

**Objective:** demonstrate every acceptance criterion end to end and leave the
package ready for implementation review.

### Tasks

- [ ] Consolidate the direct/loop, opposing-area, external-source,
  sequence, proxy/re-entry, system-prompt, overlay, and file-resolution
  regressions into a documented AC1–AC10 matrix; every row must exercise the
  real CLI or canonical owner named by the criterion rather than inject a
  hand-built `ComposeContext` (AC4).
- [ ] Assert request-local work bounds across the full route matrix: no extra
  ambient CWD/HOME/environment capture, Git discovery, topology probe, or host
  scan, and exactly one prepared-context construction per document epoch
  (AC5, AC11).
- [ ] Run the capture-owner inventory across all production Claudine library and
  CLI source and review every allowlist reason (AC12).
- [ ] Run the path-sensitive regression matrix on macOS and ensure fixtures are
  structurally portable to Windows and Linux; where CI is available, require
  all three OS jobs before closure (AC13).
- [ ] Review all behavior-adjacent `///`, `//!`, and inline comments touched by
  the change for contract accuracy, and verify relevant README/docs/skill
  snapshots changed alongside public behavior.

### Final validation checkpoint

- [ ] Run `just test` from `claudine/` and record a clean nextest result.
- [ ] Run `just test-l2` from `claudine/` with terminal/browser fixtures kept in
  the background so no window gains focus (AC14).
- [ ] Run `just lint` from `claudine/` and record a clean result (AC14).
- [ ] Confirm the original real-world repro now reports the launch package area
  from both repository-root and package-local prompt copies, including
  lifecycle interpolation and `when:` evaluation.
- [ ] Review `git diff --check` and the final diff for unrelated edits; do not
  run write-mode `cargo fmt` under this plan.

## Acceptance-criterion traceability

| Criteria | Primary phases |
|---|---|
| AC1–AC4: direct/loop, opposing areas, external sources, real CLI seam | 1, 3, 6 |
| AC5: exact snapshot reuse | 1–5 |
| AC6: target identity | 3, 4 |
| AC7: sequence parity | 4 |
| AC8: proxy/re-entry parity | 3, 5 |
| AC9: system-prompt/overlay parity | 5 |
| AC10: file-resolution non-regression | 3–5 |
| AC11: no extra discovery | 1, 2, 6 |
| AC12: capture-owner guard | 1, 5, 6 |
| AC13: cross-platform paths | 2, 4–6 |
| AC14: final validation | 6 |
