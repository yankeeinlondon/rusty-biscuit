---
created: 2026-09-16
spec: ./spec.md
plan: ./plan.md
---

# Design Note — Run Initialization Before Body Discovery

> **Superseded safety design (2026-09-17):** the shell-free ruling in
> [spec R2](spec.md#r2-initialization-is-shell-free-binding-ruling-2026-09-17)
> and [its handoff](shell-free-ruling.md) replaces every initialization shell
> approval step below. Initialization and early catches cannot execute shells,
> regardless of approval. Earlier phase results do not verify this amendment.


Phase 1 output: rulings, the reproduction, the Darkmatter projection contract,
the loop-path decision, and regression archaeology. Later phases treat this
file as their contract. Line numbers are from the worktree at `e4ae0139a`.

## Rulings

Each plan ruling is reaffirmed below. Where research sharpened a ruling, the
clarification is recorded with it.

1. **OQ1: sequences use Option A (binding).** Spec → Open Questions → OQ1
   (recommendation). Sequence execution keeps its recursive, side-effect-free
   static preflight. If a transclusion is absent because `initialize` has not
   created it yet, that stays a static-preflight failure for the sequence task.
   Phase 5 adds a characterization test and an actionable diagnostic. This fix
   does not adopt Option B and builds no part of Option C.
2. **The staged-boot predicate is the authored `initialize` key (binding).**
   Spec → Design Decisions → "Live document entry uses one staged boot"; R4;
   R6. The one predicate is `defers_schema_verdict_to_initialize`
   (`claudine/cli/src/commands/compose/prep.rs:67`). It is keyed on the
   authored frontmatter map, so a malformed or empty `initialize:` still
   stages. No second detection mechanism may be introduced. Retry, resume, and
   later loop iterations never stage again.
   - **Clarification.** The plan says "a newly adopted proxy target always
     stages". Its only valid reading is that the harness's existing
     adopted-target boot stays in place (`bootstrap_adopted_document_phase`).
     It does not mean the predicate is bypassed for adopted targets. The
     spec's staged order applies to a direct document or adopted target
     "*that declares `initialize`*". R6/AC11 require a document without
     `initialize` to keep its eager body discovery and failure timing.
     Phase 4 therefore applies the predicate to first and non-first documents
     alike.
   - **Existing split to handle.** `prep.rs:488` currently computes
     `schema_stage` as `Validate` only when `first && !defers`, so every
     adopted target defers its schema verdict even without `initialize`. That
     is verdict timing, not transclusion timing. Phase 4 must not widen it into
     a body-discovery deferral for targets without `initialize`.
3. **The projection lives in Darkmatter (binding).** Spec R1, R4 and Design
   Decisions ("If Darkmatter lacks a frontmatter/lifecycle-only projection, add
   that projection at the shared composition boundary"). The contract is
   below. Claudine does not reimplement interpolation.
4. **Parsing is not discovery (binding).** Spec → "Body discovery means
   dereferencing, not parsing". These may run before `initialize`:
   - loading and parsing the root document;
   - assembling caller inputs and proxy overlays;
   - resolving target identity and capturing the document epoch;
   - `ContextRequirements::for_document` (`prep.rs:601`);
   - `parse_selection_hints_from_frontmatter` (`prep.rs:518`);
   - `resolve_loop_config` (`claudine/lib/src/composition/looping/config.rs:61-66`,
     which reads the authored `loop` map lexically).

   Dereferencing any body dependency before `initialize` is not permitted.
5. **Dry run is unchanged (binding).** Spec → "Dry run remains
   side-effect-free"; AC11. `--dry-run` runs no `initialize` side effects and
   does not traverse a dynamic proxy. It may still report a missing
   transclusion that only `initialize` would create.
6. **`parent_dir` keeps its meaning (binding).** Spec → Separate prompt defect;
   Scope and Completion. Only the shipped `log` expression changes. See
   "Finding for Phase 6".
7. **Header placement stays (advisory).** Spec R3. Status and header rendering,
   including the proxy-handoff announcement, is not body discovery and stays
   where it is. The reproduction shows both headers render before the failure.

## Reproduction

Test file: `claudine/cli/tests/level2_initialize_generated_transclusion.rs`,
registered as `required-features = ["terminal-tests"]`. It runs in a detached
tmux pane with `CliProcessFixture` state, launched under `env -i`: fixture
home, fixture `bin` plus the minimal system `PATH`, `PLAYA_DRY_RUN=1` with a
private spool, `CLAUDINE_RENDEZVOUS_REPORT=false`, and a fake `claude`.

Run it with `just test-l2 level2_proxied_initialize` and
`just test-l2 level2_direct_initialize` from `claudine/`. The spawn guard
requires the `level2_` file prefix for a tmux-owning test, so the plan's
example name gained that prefix.

| Test | Shape | Spec |
|---|---|---|
| `level2_proxied_initialize_creates_log_before_guarded_phase_one_body_is_discovered` | Router `initialize` proxies to `prompts/_implement/implement-plan.md`. The target's `initialize` runs `ensure_file: "{{log}}"` plus a marker `shell`. The body has nested `file_exists(log)` / `phase > 1` guards around `::file {{log}}`. `log` is `dirname(spec) + '/implementation-log.md'`. `phase=1`. | AC1, AC2, AC10 shape |
| `level2_proxied_initialize_creates_log_before_guarded_later_phase_body_includes_it` | Same as above with `phase=2`. The prompt must contain the log content `initialize` wrote. | AC1, AC2 |
| `level2_direct_initialize_shell_has_no_effect_when_approval_is_unavailable` | Direct document, no `-y`, stdin closed, `initialize` runs `shell: touch <effect>`. | R2, AC5 |

All three tests assert the fixed behavior, so they stay red until Phase 4.

**Observed failure, proxied tests (both phases), current build:**

```text
Claudine ▸ Claude  YOLO   Compose  prompt sourced from prompts/router.md
ℹ Starting pre-flight checks
ℹ flow control redirected to implement-plan.md
Claudine ▸ Claude  YOLO   Compose  prompt sourced from ./_implement/implement-plan.md
⤫ TransclusionError: I/O failure
┃
┃ File not found: fixes/2026-09-14-demo/implementation-log.md
┃
┃ Check file existence and permissions.
```

- The exit status is 1.
- `implementation-log.md` is never created.
- `events.log` stays empty: `initialize` never ran and the provider never
  launched.
- The same error occurs when `prompts/_implement/implement-plan.md` is
  invoked directly without the router (checked by hand).
- **Wiring control (by hand):** with the log pre-created, the same fixture
  succeeds on the current build. `initialize` runs once before the provider,
  caller `phase` reaches the target through the proxy, and at phase 2 the
  prompt includes the log content written by this run's `initialize`. So the
  only thing red in the tests is the ordering defect.

**Observed failure, direct approval test, current build:** the run exits 1
with:

```text
⤫ CompositionError: composition failed
┃ pre-flight shell approval failed: Shell command 'touch …' at …/direct.md
┃ requires approval but no approval handler is available. …
```

However, **the effect file exists**. The unapproved `initialize` shell command
ran before the lifecycle audit refused it.

### Finding: approval gap on the direct and loop routes (pre-existing, R2)

On the single direct route, `route_initialize`
(`claudine/cli/src/commands/wrap/composition/pipeline.rs:1776`) emits
`initialize` before any lifecycle shell audit. On the loop route it is
`looping/engine.rs:375`. The lifecycle audit runs later (`runner.rs:238`), and
the lifecycle `ShellRunner` assumes its commands were already approved
(`lifecycle/executor.rs:273-276`). Only `run_initialize_stages`, for adopted
targets, has the narrow `[Initialize]` gate (`loop_control.rs:852-857`). The
direct test above demonstrates this for a non-interactive run without `-y`.
Phase 4's narrow gate before `initialize` must cover the direct and loop routes
too. The third reproduction test is the regression for that.

## Phase 2 contract — Darkmatter frontmatter-surface projection

### Operation registry and prior art

- `ComposeOperation` is defined in
  `darkmatter/lib/src/markdown/compose/pipeline/operations.rs:17`. Its default
  order, by phase:
  - InlinePre: `FrontmatterInterpolation`, `FrontmatterShellExpansion`,
    `TextReplacement`, `PageBlocks`, `Interpolation`, `ShellExpansion`,
    `ShellBlocks`, `LinkResolve`.
  - Transclusion: `BlockTransclusion`, `FrontmatterTransclusion`,
    `CodeTransclusion`, `TocLinking`, `FileLinks`.
  - InlinePost: `Cleanup`, `Normalization`.
  - Finalization: `LinkNormalization`.
- `ComposeOptions::only(&[ComposeOperation])` is at
  `compose/context/options.rs:654`, and `is_enabled` at `:660`.
  `enabled_operations` already participates in the options fingerprint, so a
  preset that only narrows the operation set needs no new field.
- The prior art is `compose/preflight/collect.rs:218-240`. Discovery composes
  each document with `only([FrontmatterInterpolation, TextReplacement,
  Interpolation])`, excludes `$schema`, and defers shell-pending schema
  problems.

### What runs regardless of the operation set

`run_compose_pipeline_internal` (`compose/pipeline/mod.rs`) always runs a
frontmatter stage:

- **Remote prefetch (`:64`).** Runs only when `BlockTransclusion` or
  `Interpolation` is enabled, so the projection skips it.
- **Schema preparation (`:160-167`).** Loads `$schema`, the baseline and
  trigger schemas, and the caller file-input projection. This is frontmatter
  I/O and is permitted by R1.
- **Frontmatter interpolation pass 1 (`:186`).** Honors `exclude_keys`, so
  lifecycle subtrees keep their `{{ }}` spans.
- **Schema validation (`:252`).** Stops at the deferred verdict.
- **Hazard: `validate_pre_approved` (`:273-282`).** It runs when
  `pre_approved_commands.is_some()` and any of `FrontmatterShellExpansion`,
  `ShellExpansion` or `ShellBlocks` is enabled. It calls the full
  condition-blind collector (`collect_recursive`, which dereferences every
  `::file`/`::url`/prologue/epilogue at `collect.rs:318-440`). Claudine's
  `canonical_compose_options` sets pre-approved commands
  (`claudine/lib/src/composition/prepare.rs:247`), so a naive
  `only([FrontmatterInterpolation, FrontmatterShellExpansion])` **reproduces
  the bug**.
- **Frontmatter `$(...)` expansion and pass 2 (`:290`, `:315`).** Skip
  excluded keys.

### Chosen API shape

In `darkmatter/lib/src/markdown/compose/context/options.rs`:

```rust
/// Effective-frontmatter projection: composes frontmatter (lifecycle subtrees
/// excluded by the caller keep their `{{ }}` spans) and never touches the body.
#[must_use]
pub fn only_frontmatter_surface(self) -> Self;   // only([FrontmatterInterpolation, FrontmatterShellExpansion])
pub fn is_frontmatter_surface_only(&self) -> bool; // derived from enabled_operations
```

- **Required guard at `pipeline/mod.rs:273`.** Run `validate_pre_approved`
  only when a *body* shell or transclusion operation is enabled
  (`ShellExpansion`, `ShellBlocks`, or `BlockTransclusion`), or equivalently
  skip it when `is_frontmatter_surface_only()` is true.
  - A companion test must prove the full compose path still fails on an
    unapproved body command, so the guard does not weaken full compose.
- **Required companion collector.** Expose a public frontmatter-only command
  collector in `compose/preflight/`, for example
  `collect_frontmatter_shell_commands(&Markdown, &ComposeOptions)`, wrapping
  the private `scan_one_frontmatter` (`collect.rs:506`). This lets the
  bootstrap approve frontmatter `$(...)` before it executes. The alternative,
  disabling `FrontmatterShellExpansion` in the bootstrap, would break
  lifecycle command resolution whenever a lifecycle command depends on a
  shell-derived key.

### Invariants: what the projection never does

- It never parses or resolves `::file`, `::url`, `::code`, or
  prologue/epilogue.
- It never parses or runs `::shell` / `::shell-block`.
- It never evaluates `::block when`.
- It never interpolates the body, runs remote prefetch, or runs the full
  body collector.
- The composed `content()` is the unchanged root body and must never be used
  as a prompt. This is why the bootstrap output is a distinct
  `BootstrapPreparation`, not a `PreparedComposition`.
- `report.deferred_frontmatter_keys` is still populated.

### Shared option assembly (Phase 3)

`canonical_compose_options` (`prepare.rs:215-259`) is the single assembly point
for direct preparation (`:448`), inline preparation (`:619`) and
`preflight_document_shell` (`:345`). It handles the `LIFECYCLE_EVENT_KEYS`
exclusion, initialized-output overrides, caller records, pre-approved commands,
the file-resolution context, and `with_deferred_schema_verdict`.

`prepare_bootstrap` calls it with `defer_schema_verdict = true`, applies
`.only_frontmatter_surface()`, and then reuses the post-compose lifecycle steps
of `prepare_direct_with_prompt` (`:416-543`):

- `parse_lifecycle_config`
- `validate_no_nested_spans_in_literals`
- `resolve_lifecycle_shell_commands`
- `validate_no_err_in_no_error_events`
- the selection hints

It skips the empty-body and prompt steps. The bootstrap stage sits beside
`prepare_document` (`prepare/service.rs:100-135`) as its own entry, not as a
fifth `PreparedComposition` arm.

The command-level preflight in `prep.rs:623-662` builds its own
`ComposeOptions` and bypasses `canonical_compose_options`, so it lacks the
workspace binding and initialized outputs. Phase 4 should replace it, after
`initialize`, with `preflight_document_shell` (the harness already uses that
path at `harness_orch/prompt.rs:249`) rather than carry a second assembly.

### Open points for Phase 2/3

- **Remote reads from frontmatter expressions.** `frontmatter_resolution_context`
  (`options.rs:1122`) attaches remote fetch whenever `remote_reads_enabled()`.
  No Claudine call enables remote reads today. Treat "no remote fetch" as
  inherited: do not add a projection-specific enforcement unless Phase 2 finds
  a caller enabling it.

## Loop-path decision (drives Phase 4)

### Body reads before `initialize` today

**Looping document that declares `initialize`** (direct, or an adopted target,
which `prep.rs:1011` forces through the loop path with `adopted_handoff: None`):

- **(A)** The command audit at `prep.rs:662`. It calls
  `resolve_shell_approvals` → `compose_preflight`, which dereferences every
  transclusion. It passes `lifecycle: None`.
- **(B)** The loop seed at `prep.rs:873`. `build_loop_seed_with_lifecycle`
  (`looping/seed.rs:87-92`) calls `prepare_direct`/`prepare_inline` →
  `compose_with`, a full body compose used only for control variables and the
  lifecycle config.
- `initialize` fires at `looping/engine.rs:375`, outside the iteration loop.
- Iteration 1's `prepare_staged` (`prep.rs:967`) then composes the
  **pre-`initialize` root text** against post-`initialize` disk, using
  approvals from (A).

**Non-looping direct document:**

- (A) above.
- **(C)** `prep.rs:1336` → `prepare_document` → `compose_with`, which composes
  the body and runs body `::shell`.
- `initialize` fires at `pipeline.rs:1776`.

**Newly adopted non-looping target:**

- (A) and (C), with the verdict deferred.
- **(D)** `loop_control.rs:560` → `preflight_harness_document`.
- **(E)** `loop_control.rs:574`, a `DeferToStabilizedReread` materialization,
  which is a full `prepare_document`.
- Only then does `run_initialize_stages` run (`:958`). It emits `initialize`
  at `:862`, and its stabilized reread is at `:969-1012`.

**No duplicate `initialize` today.** `take_bootstrap_pending` (`:947`) consumes
the pending stage, `arm_stabilization` (`coordinator.rs:110`) steps aside when
a full boot is armed, and loop iterations 2+ return before `route_initialize`
(`pipeline.rs:1678`).

### Decision

1. **Before the loop engine or pipeline**, for a document satisfying ruling 2:
   - skip audit (A);
   - run `prepare_bootstrap` to get the `BootstrapPreparation`;
   - run the narrow gate, `resolve_lifecycle_shell_approvals(&[Initialize])`,
     with the shared-cache `approval_options` (`prep.rs:649`). This also closes
     the approval gap above.
2. **Loop seed.** `build_loop_seed_with_lifecycle` takes the
   `BootstrapPreparation` (effective frontmatter plus lifecycle) instead of
   calling `prepare_direct`/`prepare_inline`. Loop ownership needs nothing
   from the body: `resolve_loop_config` reads the authored `loop` map, and the
   per-iteration condition reads frontmatter only.
   - **Accepted limitation (unchanged from today).** If `initialize` rewrites
     `loop:` or a control variable in the file, the pre-`initialize` reading
     wins for loop recognition.
3. **`looping/engine.rs:375` stays the only emission point** for looping
   documents. It already handles skip, error, proxy, retry, resume and defer,
   and it does not re-fire on later iterations.
4. **The stabilized reread lands in iteration 1's branch at `prep.rs:966`.**
   It replaces the call that composes the pre-`initialize` snapshot:
   1. Reload the root from disk and reapply the proxy overlay, the same way as
      `load_overlaid_source` (`harness_orch/prompt.rs:100-126`).
   2. Reuse `loop_prepare_options`: layered set overrides, caller records,
      `file_resolution_context`, `document_epoch`, and the same-epoch context
      extended for newly demanded groups. Launch state is never recaptured.
   3. Run the full body and transclusion audit on that read.
   4. Call `prepare_staged(entry, SchemaStage::Validate)` with those approvals.

   The result has `schema_verdict_deferred == false`, so `runner.rs:160` does
   not arm a second reread, and `runner.rs:238` audits the stabilized
   lifecycle.
5. **Constraints on that site:**
   - **R5 routing.** An `Err` from the iteration closure is only recorded by
     the engine (`engine.rs:557-570`) and does not fire `blocked`/`finalize`.
     Route reread and audit failures through the guard explicitly, exactly
     once.
   - **AC6.** Keep the stabilized source in a cell so iterations 2+
     (`prep.rs:978`) compose from it rather than from the pre-`initialize`
     `source`.
   - **R4.** Extract "reread + audit + prepare" into one helper. The
     iteration-1 site and the harness tail (`loop_control.rs:969-1012`) must
     both call it, so there are not two copies.
   - **Why not only the harness tail.** The pipeline needs a prepared
     composition before the harness runs (selection hints, inline closure
     plan, argv, system prompt, MCP tags). The single direct route therefore
     needs the same split: `initialize` must move ahead of building
     `request.prepared` (C).
6. **Adopted targets.** Reads (D) and (E) become the bootstrap projection,
   since (E) is where `run_initialize_stages` reads its lifecycle (`:841`).
7. **Retry and resume** keep `preflight_fresh_document_phase` plus a
   `Validate` read (`loop_control.rs:1343-1352`) with no `initialize`.
   Iterations 2+ keep `skip_preflight = true`.

Additional observation: the `STAGE_MATRIX` rows (`entry.rs:88-131`) have no
production callers of `.stages()`. The "design authority" named in R4 is
currently documentation, not enforced behavior.

## Finding for Phase 6: the shipped `log` repair is already in place

`prompts/_implement/implement-plan.md` currently reads
`log: "{{ dirname(spec || plan) + '/implementation-log.md' }}"`. The router
`prompts/implement.md` already uses `dirname(spec)` for both `ensure_file` and
the proxy's `with: log`. The last change touching these files was `dd2bccf39`
("fix(prompts): guard optional router inputs and repair review notifications").
`git log -S"parent_dir(spec)"` identifies `47e9259ab` ("fix(claudine): repair
shipped implement prompts and the tests that pin them") as the commit that
removed `parent_dir(spec)` from the `log` expression.

`parent_dir` remains only in display strings: `parent_dir(plan)` in the
`start`/`success`/`failure` messages and `parent_dir(spec)` in the body's log
title instruction. Ruling 6 leaves these untouched. Phase 6 should verify and record
this rather than re-edit, then complete the remaining items: fixture and hash
manifests, and the logging-instruction review for an already-created empty log.

## Regression archaeology

This is a **latent defect, not a regression** (high confidence, based on
source reads at each commit, not on rebuilding old versions).

- `21d69ff19` (2026-04-01) first placed the pre-flight shell approval scan
  ahead of execution in compose.
- `35cd28e25` (2026-06-16) switched it to Darkmatter `compose_preflight`,
  which fails on a missing transclusion.
- `6939ed660` (2026-06-18) moved it into `prep.rs`, before
  `execute_loop_or_single`.
- `922a25027` (2026-06-22) is the first commit where the CLI emits
  `initialize`. The command-level scan already preceded it, so an include
  that `initialize` would create has failed since `initialize` shipped.
- The staged harness work (`01d0d7739` 2026-07-17
  `bootstrap_adopted_document_phase`; `df864234c` 2026-07-18
  `run_initialize_stages`; `9cb508c72` 2026-07-18 deferred schema verdict)
  added the correct sequence downstream but never moved the earlier scan.

No first bad commit exists to bisect to. If one must be named, it is
`922a25027`, where the conflicting contract was introduced.
