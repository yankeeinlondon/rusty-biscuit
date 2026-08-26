---
total_phases: 6
created: 2026-08-12
updated: 2026-08-26
phase: 6
agent: codex/default
yolo: true
---

# Execution Plan: Anchor Prepared `ctx.*` to the Invocation Launch Context

Reference: [`spec.md`](spec.md)

## Goal

Make every canonical Claudine composition route project plain `ctx.*` from the
caller's immutable launch context, while preserving the active document's
`SourceContext` for file and schema resolution. Each document preparation epoch
must construct one target-adjusted early-binding snapshot and reuse that exact
snapshot through preflight, composition, loops, and lifecycle execution.

## Current implementation baseline

This plan starts from `main` at
`bd6c305a89ddf81c721649d48eb1428b497bf25d`. PR #54 is an evidence source for
requirements and test cases, not an implementation base; do not cherry-pick or
revive it.

The 2026-08-25 source audit established:

- `InvocationContext` already owns immutable launch inputs and reusable host,
  repository, topology, environment, and file-resolution evidence;
- `DocumentPreparation` is the canonical preparation service and
  `PreparedComposition::compose_context` is the existing exact-snapshot carrier;
- canonical routes still call `runtime_evidence(&SourceContext, ...)` and pair
  it with `source_context.base_dir()`, including direct preparation, sequence,
  system-prompt, overlay, harness, and composition-pipeline paths; and
- Darkmatter has `capture_with_evidence`, but no missing-requirements or
  same-snapshot evidence-extension API.

No implementation from the previous branch is credited as complete. Phase 1
therefore begins with a clean failure baseline and a fully classified route
inventory on current main.

## Completion contract

The fix is complete when:

- moving a prompt, task, group, overlay, or system-prompt file cannot change
  launch-facing plain `ctx.*` values;
- launch CWD and launch repository/topology evidence can only be paired through
  an `InvocationContext` API;
- direct, inline, loop, proxy, retry/resume, sequence, harness, overlay,
  composition-pipeline, and system-prompt paths use the canonical launch
  capture;
- one document preparation epoch reuses one exact target-adjusted snapshot;
- the post-`initialize` stabilized reread stays inside its epoch: missing
  requirement groups are extended from retained launch evidence, and the
  anchor, environment capture, and applied target overrides never change;
- a sequence graph-phase command referencing `ctx.agent`, `ctx.model`,
  `env.AGENT`, or `env.MODEL` fails graph preflight with a typed
  target-identity rejection instead of expanding a pre-selection value;
- `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` retain resolved-target
  precedence;
- source-relative document references, eager `file(...)` values, and `$schema`
  resolution retain their existing behavior;
- no additional ambient CWD, HOME, environment, Git, or topology discovery is
  introduced; and
- the capture-owner inventory guard and all affected Claudine and Darkmatter
  quality gates pass locally on every required operating-system environment.

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

- [x] Add CLI-seam regression fixtures that run equivalent documents from one
  launch directory while storing them at the repository root, in the launch
  package area, in an opposing package area, and in an external repository;
  record the currently wrong `ctx.area`/`ctx.repo_root` results for body,
  frontmatter, `when:`, and lifecycle surfaces (AC1–AC4).
- [x] Add the inverse fixture: launch outside every repository while the prompt
  lives inside the Rusty Biscuit repository, and assert the eventual contract
  is an absent launch repository and package area rather than prompt-derived
  facts (AC3).
- [x] Inventory every production `ComposeContext::capture*` and
  `InvocationContext::runtime_evidence` call under `claudine/lib/src` and
  `claudine/cli/src`; classify each as canonical prepared context, explicit
  live `current.ctx.*`, library compatibility fallback, or unrelated
  documentation command (AC12).
- [x] Record the canonical migration list at minimum for direct compose
  preflight/preparation, sequence root and referenced-document preflight, JIT
  templates, task execution, lifecycle pipeline re-materialization, overlays,
  harness prompt preparation, composition-pipeline re-materialization, and
  system-prompt preparation.
- [x] Record the current graph-phase behavior for a pre-selection sequence
  command that references `ctx.agent`/`ctx.model`/`env.AGENT`/`env.MODEL`, so
  the Phase 4 typed rejection has a frozen before-state (AC7).
- [x] Add or extend request-local work-accounting assertions around the baseline
  routes so later tests can detect extra Git discovery, topology work,
  environment capture, or runtime-evidence initialization without timing-based
  assertions (AC5, AC11).
- [x] Audit the `record_ambient_fallback` accounting for blind spots as part of
  the capture inventory: identify every consumer seam that could drop its
  prepared context and fall through to darkmatter's ambient capture unobserved
  (D2, AC5).
- [x] Review the comments adjacent to the direct compose and lifecycle capture
  sites; mark comments that already state launch anchoring for preservation and
  comments that describe source-backed `ctx.*` for correction when their code
  changes.

### Validation checkpoint

- [x] Run the focused regression set with nextest and confirm it fails for the
  source-versus-launch distinction, not because of provider availability,
  ambient user configuration, or interactive terminal behavior.
- [x] Review the inventory against the spec's complete route list before Phase
  2; no unclassified production capture may proceed into migration.

## Phase 2 — Add the invocation-owned launch capture seam

**Objective:** make launch anchor and launch evidence an inseparable operation
owned by `InvocationContext`.

### Tasks

- [x] Add `InvocationContext::capture_launch_context` (or an equivalently named
  API) accepting `ContextRequirements` and returning a `ComposeContext`
  captured at `launch_cwd()` from the retained launch repository/topology,
  environment, and host evidence (D1, D2).
- [x] Add the minimal Darkmatter APIs for calculating missing
  `ContextRequirements` and extending an existing `ComposeContext` from supplied
  evidence. Preserve its original datetime, environment, anchor, diagnostics,
  and previously captured values; do not add snapshot identity or ownership to
  Darkmatter (D2, D4, AC5).
- [x] Refactor the internal runtime-evidence population so launch and source
  projections reuse the existing caches and work counters, but the launch path
  operates directly on the retained launch entry; do not fabricate a
  `SourceContext`, prompt path, or file-resolution context (AC11).
- [x] Ensure demand-driven groups retain their existing semantics: Git/repo
  facts come from the launch repository, source-scanned groups use the launch
  base when requested, environment comes from the invocation snapshot, and
  host groups use the invocation's single-flight caches.
- [x] Add request-local work accounting for launch-context construction and
  group extension — Claudine-only, per the ratified AC5 mechanism: no
  darkmatter identity field and no `Arc` plumbing refactor — so tests can
  count exactly one construction per epoch (plus reread extensions) and assert
  an ambient-fallback count of zero, rather than accepting two separately
  constructed but usually equal snapshots (AC5).
- [x] Complete the `record_ambient_fallback` wiring wherever the Phase 1 audit
  found a consumer seam that could drop its prepared context and reach
  darkmatter's ambient capture unobserved; AC5's zero-fallback assertion
  depends on this accounting having no blind spot (D2, AC5).
- [x] Expose the Claudine epoch-extension operation: project only the missing
  `ContextRequirements` groups from the same retained launch evidence into an
  existing snapshot, without re-anchoring, re-capturing environment, or
  re-applying target overrides (D4, AC5).
- [x] Keep `runtime_evidence(&SourceContext, ...)` available for genuinely
  source-specific compatibility consumers, and keep live `current.ctx.*`
  capture behavior unchanged.
- [x] Add L1 tests proving launch capture reports the launch package/repository
  when the supplied document source is in the same area, an opposing area, or
  another repository, and reports no repository facts when launched outside a
  repository (AC2, AC3).
- [x] Add L1 work-bound tests proving repeated launch projections reuse retained
  Git/topology/environment/host evidence and do not consult ambient CWD or HOME
  after invocation capture (AC11).
- [x] Build all path fixtures with `Path`/`PathBuf` and platform-aware temporary
  directories; avoid rendered-prefix, separator, case-folding, or canonical-path
  assumptions that fail on Windows or macOS symlinked temp roots (AC13).

### Validation checkpoint

- [x] Run the affected Darkmatter L1 tests and lint, then `just test` and
  `just lint` from `claudine/`; confirm the new API is additive and existing
  compatibility callers remain green before migrations.
- [x] Inspect request-local counters for the new unit matrix and confirm launch
  projection adds no Git root discovery or topology probe.

## Phase 3 — Make direct, inline, loop, and lifecycle execution share one epoch snapshot

**Objective:** create the target-adjusted early-binding snapshot once per active
document entry and pass it through every stage of that epoch.

**Parallelization:** may run in parallel with Phase 4 after Phase 2.

### Tasks

- [x] Make the existing `DocumentPreparation` service the canonical
  document-epoch owner. Its CLI entry in
  `claudine/cli/src/commands/compose/prep.rs` must request the snapshot after
  provider/model resolution and before shell preflight, call the launch-capture
  API once, and apply the resolved target's `env_overrides` once. Do not add a
  parallel route-local coordinator (D4, AC5, AC6).
- [x] Pass that exact snapshot to the narrow/full shell preflight
  `ComposeOptions`, `PrepareOptions.prepared_context`, body and effective
  frontmatter composition, schema evaluation, loop seed/condition work, and
  lifecycle execution; remove the second capture currently performed before
  main preparation (AC1, AC2, AC5).
- [x] Change lifecycle and composition-pipeline setup to consume
  `PreparedComposition::compose_context` instead of rebuilding a context from
  the composed prompt and active `SourceContext`; leave event-time
  `current.ctx.*` capture separate and live (AC5, AC8).
- [x] Keep the post-`initialize` stabilized reread inside the same epoch: when
  the reread's demand-driven requirements exceed the stored snapshot's groups,
  extend the snapshot through the epoch owner's extension operation; the
  capture anchor, environment capture, and applied target overrides remain
  immutable for the life of the epoch (D4, AC5).
- [x] Make loop iterations retain the epoch snapshot and prove that loop
  condition/body/lifecycle reads do not trigger a new launch capture (AC1,
  AC5, AC8).
- [x] Route proxy target entry, retry, and resume through the same canonical
  fresh-epoch owner: each fresh read may create one new snapshot, but every
  launch-facing field must still come from the original invocation and every
  target identity override must be reapplied (AC6, AC8).
- [x] Preserve the active `SourceContext` and its
  `FileResolutionContext` in all options used for `$schema`, transclusion,
  provenance, and eager `file(...)` handling; do not promote
  `file_ref_fallback_dir` into a file-resolution candidate (AC10).
- [x] Correct capture-site comments that claim two separately constructed
  contexts are equivalent or that prepared `ctx.*` is source-backed; retain
  comments whose launch-area contract becomes true after this phase.

### Tests

- [x] Add real CLI-seam direct and inline tests for repository-root versus
  package-area prompts, opposing launch/source areas, and external sources;
  assert body, effective frontmatter, `when:`, preflight-expanded command
  bytes, and lifecycle all report launch facts (AC1–AC4).
- [x] Add a deterministic work-accounting test proving preflight, body,
  effective frontmatter, and lifecycle consumed one launch-capture
  construction (plus zero or more reread extensions), with an
  ambient-fallback count of zero and every consumer seam observing a
  populated prepared context, so separately constructed but equal captures
  cannot pass (AC5).
- [x] Add a stabilized-reread test in which `initialize` rewrites the document
  to demand context groups the stored snapshot was not captured with; assert
  the epoch snapshot is extended from retained launch evidence — counters
  show one construction plus the extension, zero ambient fallbacks — and the
  anchor and target overrides are unchanged (AC5, AC11).
- [ ] Add direct, loop, proxy, retry, and resume tests asserting target-specific
  `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL`, plus launch-stable
  repository/package facts (AC6, AC8).
- [ ] Add conflicting launch/source file fixtures proving body references,
  eager `file(...)`, and `$schema` remain source-relative/repository-first
  while plain `ctx.*` remains launch-relative (AC10).

### Validation checkpoint

- [x] Run the focused compose, inline-compose, looping, lifecycle, proxy, and
  retry/resume nextest suites; confirm the exact-snapshot counter and all
  target-identity assertions pass.
- [x] Run `just test` from `claudine/` before merging with the sequence track.

## Phase 4 — Migrate static sequence preflight and per-task epochs

**Objective:** make every sequence phase use launch facts without sacrificing
source-specific resolution or per-task target identity.

**Parallelization:** may run in parallel with Phase 3 after Phase 2.

### Tasks

- [x] Replace the root sequence graph capture in
  `claudine/cli/src/commands/wrap/sequence/mod.rs` with one base launch context
  from the invocation owner; use it for launch-facing graph expressions before
  per-task target selection (D4, AC7).
- [x] Update referenced-document shell resolution in
  `composition/sequence/preflight` to use the graph's launch context while
  retaining each origin document's `SourceContext` and
  `FileResolutionContext`; remove source-anchored recapture from
  `resolve_shell_bytes` (AC7, AC10).
- [x] Update preflight approval composition and JIT template preflight to clone
  or project the launch base, then apply the selected task target's environment
  overrides where selection is available; where an approval/execution pairing
  exists, approved and executed command bytes retain the existing equality
  guard (AC6, AC7).
- [x] Reject, at graph preflight, any pre-selection command whose expansion
  references `ctx.agent`, `ctx.model`, `env.AGENT`, or `env.MODEL`, with a
  typed error that mirrors the existing late-binding rejection: it names the
  offending root and directs the author to task-scoped commands. Sequence
  preflight resolves shell bytes once and execution runs those bytes, so this
  rejection is the only safety net — there is no second resolution that could
  detect a wrong-value expansion. Per-task and JIT audits, where the selected
  target is available, continue to permit those roots (D4, AC6, AC7).
- [x] Update sequence task/group/prompt preparation and
  `prepare_task_context` so each task epoch gets one target-adjusted launch
  snapshot and reuses it through task composition and execution (AC5–AC7).
- [x] Ensure a sequence file, nested prompt, task, and group distributed across
  different repositories never substitute a source repository for the launch
  repository, including serial and parallel groups (AC7).
- [x] Keep source snapshots and source-relative selection/workspace behavior
  unchanged wherever the current contract explicitly requires them (D3).

### Tests

- [x] Add a sequence matrix covering root graph expressions, referenced task
  and group expressions, nested prompt documents, template preflight, and task
  execution with launch/source files split across repositories (AC7).
- [x] Add serial and parallel task tests proving per-task agent/model overrides
  are visible in both `ctx.*` and `env.*` without changing launch repo/area
  values (AC6, AC7).
- [x] Add graph-phase rejection tests: a root-graph command referencing each of
  `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` fails preflight with
  the typed target-identity rejection — a preflight error, never a
  wrong-value expansion — while the same command inside a task with a
  selected target still expands (AC6, AC7).
- [x] Add sequence conflict fixtures proving source-relative `$schema` and file
  references still choose the authored document's source while graph/task
  `ctx.*` chooses the invocation launch (AC10, AC13).

### Validation checkpoint

- [x] Run the focused sequence preflight, JIT, task, serial-group, and
  parallel-group nextest suites, including approved-byte equality tests.
- [x] Run `just test` from `claudine/` after combining Phases 3 and 4; all
  canonical document routes must now use the launch-capture seam.

## Phase 5 — Migrate auxiliary routes, install the drift guard, and update contracts

**Objective:** close every remaining canonical capture path and prevent the
source-anchored pattern from returning.

**Parallelization:** Tracks A–C are parallelizable after Phases 3 and 4 expose
the final epoch/snapshot interfaces.

### Track A — System-prompt preparation

- [x] Replace file-backed and built-in system-prompt/appendix runtime captures
  with the invocation launch-capture API; use one shared launch snapshot for
  the composed system-prompt bundle and retain each input's source context for
  its file/schema resolution (AC9, AC10).
- [ ] Add relocation tests that move primary and appendix sources between the
  repository root, package areas, and an external repository without changing
  their launch-facing expansion (AC9).

### Track B — Overlay, harness, composition-pipeline, and re-materialization paths

- [x] Update overlay preparation, harness prompt preparation,
  composition-pipeline re-materialization, and loop-control target launch to
  receive the active epoch snapshot or create it through the canonical launch
  owner; remove direct source-anchored prepared-context captures (AC8, AC9).
- [ ] Prove re-materialization reuses the active epoch snapshot when the active
  document has not changed and starts exactly one fresh epoch on proxy,
  retry, or resume entry (AC5, AC8).
- [ ] Add relocation/conflict tests for overlays and harness prompts that
  separately assert launch-facing `ctx.*` and source-facing file resolution
  (AC9, AC10).

### Track C — Capture-owner inventory and documentation

- [x] Add a production-source inventory test that rejects new direct prepared
  `ComposeContext::capture*` calls outside `InvocationContext` and the canonical
  epoch owner; use explicit, reason-bearing allowlist entries only for public
  library compatibility paths, the `claudine context --values` command, and
  live `current.ctx.*` capture sites (D5, AC12).
- [x] Make the guard scan path-normalized Rust source locations so it runs on
  macOS, Windows, and Linux without separator or case assumptions (AC12, AC13).
- [x] Update relevant Claudine architecture/composition/system-prompt docs and
  the mirrored `.claude/skills/claudine/` topic documents to describe the
  invocation-owned launch snapshot, per-document epoch reuse, source-context
  separation, and unchanged `current.ctx.*` semantics.
- [x] Remove or rewrite only comments/docs that drifted from the repaired
  behavior; do not alter unrelated comments or introduce implementation
  narration.

### Validation checkpoint

- [x] Run focused system-prompt, overlay, harness, composition-pipeline, and
  re-materialization nextest suites and confirm relocation does not affect
  launch-facing values.
- [x] Seed the inventory fixture with a forbidden direct capture and confirm the
  guard fails with the source path, then restore the fixture and confirm it
  passes.
- [x] Run `just test` and `just lint` from `claudine/` after all three tracks
  merge.

## Phase 6 — Close the acceptance matrix and run final quality gates

**Objective:** demonstrate every acceptance criterion end to end and leave the
package ready for implementation review.

### Tasks

- [x] Consolidate the direct/loop, opposing-area, external-source, sequence
  (including the graph-phase target-identity rejection), stabilized-reread
  extension, proxy/re-entry, system-prompt, overlay, and file-resolution
  regressions into a documented AC1–AC10 matrix; every row must exercise the
  real CLI or canonical owner named by the criterion rather than inject a
  hand-built `ComposeContext` (AC4).
- [x] Assert request-local work bounds across the full route matrix: no extra
  ambient CWD/HOME/environment capture, Git discovery, topology probe, or host
  scan, and exactly one prepared-context construction per document epoch
  (AC5, AC11).
- [x] Run the capture-owner inventory across all production Claudine library and
  CLI source and review every allowlist reason (AC12).
- [ ] Run the path-sensitive regression matrix on macOS and ensure fixtures are
  structurally portable to Windows and Linux; where CI is available, require
  all three OS jobs before closure (AC13).
- [ ] Review all behavior-adjacent `///`, `//!`, and inline comments touched by
  the change for contract accuracy, and verify relevant README/docs/skill
  snapshots changed alongside public behavior.

### Final validation checkpoint

- [ ] Run the affected Darkmatter `just test` and `just lint` gates and record
  clean results.
- [ ] Run `just test` from `claudine/` and record a clean nextest result.
- [ ] Run `just test-l2` from `claudine/` with terminal/browser fixtures kept in
  the background so no window gains focus (AC14).
- [ ] Run `just lint` from `claudine/` and record a clean result (AC14).
- [ ] Repeat the complete affected L1 and L2 validation in `build-linux`,
  `build-win` (WSL), and `build-win-native`, in addition to the macOS host.
  Re-run L1 after L2 fixes to catch regressions; do not start hosted CI until
  all four local environments are green (AC13, AC14).
- [ ] After all local environments are green, run one full hosted CI workflow.
  Treat any failure as a new local reproduce/fix/validate cycle before pushing
  another CI attempt; do not increase timeouts without first exhausting a more
  efficient deterministic test design.
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
