---
spec: ./spec.md
plan: ./plan.md
packages:
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - claudine/cli/tests/level2_initialize_generated_transclusion.rs
    - claudine/cli/Cargo.toml
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-15-initialize-after-proxy/spec.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-15-initialize-after-proxy/design.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/tests/frontmatter_surface_projection.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/bootstrap.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/cli/tests/composition_seams.rs
docs_updated_during_phase_3:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_4:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/tests/compose_initialize_staged_boot.rs
    - claudine/cli/tests/composition_seams.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_5:
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/sequence_initialize_include_preflight.rs
    - claudine/cli/tests/level2_initialize_generated_transclusion.rs
docs_updated_during_phase_5:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_6:
    - prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/cli/tests/shipped_prompt_route_drift.rs
docs_updated_during_phase_6:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/cli/tests/compose_initialize_acceptance.rs
    - darkmatter/lib/src/effects/fs_write.rs
docs_updated_during_phase_7:
    - claudine/fixes/2026-09-15-initialize-after-proxy/spec.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_7:
    - claudine/fixes/2026-09-15-initialize-after-proxy/evidence.md
skills_files_updated_during_phase_7: []
human_review: false
message_to_agent: |-
    Phase 8 is complete; implementation is ready for author review, and the fix remains in its active directory.
    This phase changed documentation/skills and comments only (loop_control.rs and composition/preflight.rs).
    Both areas passed just test, just lint, and just test-l2. Claudine L1 passed 7,258 tests after removing
    inherited HOME/USERPROFILE from the test command; the ordinary inherited environment repeats seven
    pre-existing HOME-overlay assertions. Use the exact commands and skip inventory in evidence.md.
    No new cross-OS evidence is claimed; Phase 7's Linux/Windows/WSL gaps remain for normal CI qualification.
    Sequence includes must exist before the sequence starts; an earlier task cannot satisfy static preflight.
    No files were formatted, staged, committed, or moved to _completed. The temporary build target was removed.

source_files_during_phase_8:
    - claudine/lib/src/composition/preflight.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs

docs_updated_during_phase_8:
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/execution-flow.md
    - claudine/README.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/lifecycle.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/spec.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/evidence.md

docs_created_during_phase_8: []

skills_files_updated_during_phase_8:
    - .claude/skills/claudine/composition.md
    - .claude/skills/claudine/lifecycle.md

source_code:
    - claudine/cli/tests/level2_initialize_generated_transclusion.rs
    - claudine/cli/Cargo.toml
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/tests/frontmatter_surface_projection.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/bootstrap.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/cli/tests/composition_seams.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/tests/compose_initialize_staged_boot.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/sequence_initialize_include_preflight.rs
    - prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/cli/tests/shipped_prompt_route_drift.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/cli/tests/compose_initialize_acceptance.rs
    - darkmatter/lib/src/effects/fs_write.rs

documentation:
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/execution-flow.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/spec.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/design.md
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/evidence.md
    - claudine/README.md
    - claudine/docs/pipeline.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/lifecycle.md

completed_phase: "8"
implemented: true
---

# Implementation Log — Run Initialization Before Body Discovery

## Phase 1

Phase 1 covers rulings, the reproduction, and spikes. No production code
changed. The phase's deliverables are
[`design.md`](./design.md) and the red reproduction test
`claudine/cli/tests/level2_initialize_generated_transclusion.rs`.

### Spec frontmatter

- `implemented: true`, `implemented_by: claude/default`.

### Reproduction (red)

- I reproduced the failure by hand on the current build first, from a
  hermetic `env -i` fixture: a router proxies to a target whose `initialize`
  runs `ensure_file: "{{log}}"`, and whose body has the nested
  `file_exists(log)` / `phase > 1` guards around `::file {{log}}`. The `log`
  path is built with `dirname(spec)`, which is valid. The result is
  `TransclusionError: I/O failure` / `File not found:
  fixes/2026-09-14-demo/implementation-log.md`, exit 1, and the log is never
  created. Invoking the target directly (no proxy) fails the same way.
- **Wiring control.** With the log pre-created, the identical fixture
  succeeds: `initialize` runs once, before the provider, and caller `phase`
  reaches the target through the proxy. The tests are therefore red only
  because of the ordering defect.
- I added `level2_initialize_generated_transclusion.rs` and registered it in
  `claudine/cli/Cargo.toml` with `required-features = ["terminal-tests"]`.
  The spawn guard requires the `level2_` prefix for a tmux-owning file, so the
  plan's example name gained it. The file has three tests, all of which assert
  the fixed behavior and fail now:
  - `level2_proxied_initialize_creates_log_before_guarded_phase_one_body_is_discovered`
    (AC1/AC2, phase 1: the log is excluded from the prompt).
  - `level2_proxied_initialize_creates_log_before_guarded_later_phase_body_includes_it`
    (AC1/AC2, phase 2: the prompt includes the log content `initialize` wrote).
  - `level2_direct_initialize_shell_has_no_effect_when_approval_is_unavailable`
    (R2/AC5; see the finding below).
- Command: `just test-l2 level2_` from `claudine/`. This run was stopped after
  69 of 229 tests: 66 pre-existing L2 tests passed, and all three new tests
  failed for the documented reasons (two with `TransclusionError` /
  `File not found`, one with "the unapproved initialize shell command ran
  before the approval gate").

### Finding: pre-existing approval-boundary defect (R2)

- A direct document run non-interactively without `-y` executes an
  `initialize` `shell:` command (its effect file is created) *before* the
  lifecycle audit fails with "requires approval but no approval handler is
  available".
- Cause: `route_initialize` (`pipeline.rs:1776`) and the loop engine
  (`engine.rs:375`) emit `initialize` before any lifecycle shell audit. Only
  `run_initialize_stages` (adopted targets) has the narrow gate.
- Phase 4's narrow gate must cover the direct and loop routes. The third red
  test is the regression for this.

### Spikes

Background research agents did the spikes read-only. I spot-checked their key
citations against the source (`prep.rs:67`, `prep.rs:662`,
`pipeline/mod.rs:273`, `options.rs:654`, `engine.rs:375`, `pipeline.rs:1776`).

- **Darkmatter projection.** The contract is recorded in `design.md`. The key
  hazard: `validate_pre_approved` (`darkmatter/.../pipeline/mod.rs:273-282`)
  runs the full condition-blind body collector whenever pre-approved commands
  are set and `FrontmatterShellExpansion` is enabled. A naive
  `only([FrontmatterInterpolation, FrontmatterShellExpansion])` would
  therefore reproduce the bug. Phase 2 needs a guard plus a public
  frontmatter-only command collector.
- **Loop path.** The decision is recorded in `design.md`. Pre-`initialize`
  body reads are the command audit (A) plus the loop seed's full compose (B).
  The seed must consume the `BootstrapPreparation`, and the stabilized reread
  lands in iteration 1's branch (`prep.rs:966`). Reread and audit errors must
  be routed through the guard (R5), and the stabilized source must be kept
  for iterations 2+ (AC6).
- **Regression archaeology.** This is a latent defect, not a regression. The
  command-level scan has preceded `initialize` since `922a25027` (2026-06-22),
  the first commit where the CLI emits `initialize`.

### Rulings

- All seven plan rulings are recorded in `design.md`, each with its spec
  citation.
- **Clarification on ruling 2.** The staged-boot predicate (authored
  `initialize` key) applies to both direct documents and adopted targets.
  "Adopted targets always stage" means the existing harness boot is retained.
  It does not mean targets without `initialize` lose eager discovery
  (R6/AC11).

### Finding for Phase 6

- `prompts/_implement/implement-plan.md` already uses
  `dirname(spec || plan)` for `log`; `47e9259ab` removed `parent_dir(spec)`
  from it. Phase 6 should verify this and record it rather than re-edit.

### Environment

- The first `just lint` / `just test` attempt failed with `No space left on
  device` (25 GiB free on `/Volumes/coding`). This was an environment failure,
  not a code failure.
- I applied the repo sweep policy (`just sweep`). It removed a 15.9 GiB
  incremental cache under `target/ci-local` and thinned snapshots, leaving
  38 GiB free. I also removed my own `/tmp` scratch fixtures.

### Requirement-to-test mapping (Phase 1)

| Requirement | Test | State |
|---|---|---|
| AC1/AC2, reported guarded shape, phase 1, via proxy | `level2_proxied_initialize_creates_log_before_guarded_phase_one_body_is_discovered` | red, as intended until Phase 4 |
| AC1/AC2, later phase includes the generated log | `level2_proxied_initialize_creates_log_before_guarded_later_phase_body_includes_it` | red, as intended until Phase 4 |
| R2/AC5, an unapproved `initialize` shell has no effect (direct route) | `level2_direct_initialize_shell_has_no_effect_when_approval_is_unavailable` | red, as intended until Phase 4 |

The original failing input is preserved: a spec argument with the
`fixes/<dated>/spec.md` spelling, a router `initialize` proxy, `ensure_file`,
and nested `file_exists(log)` / `phase > 1` guards. Each test asserts the
dependent outputs: exit status, the ordered events log (exactly one
`initialize` before `provider-ran`), creation of the generated file, and the
prompt the fake provider received.

### Gates

- `just lint` (claudine): **pass**, after the disk sweep.
- `just test` (claudine L1): **pass**. 7204 tests run, 7204 passed, 9 skipped.
- `cargo clippy -p claudine-cli --features terminal-tests --test level2_initialize_generated_transclusion -- -D warnings`:
  **pass**. `just lint` does not enable `terminal-tests`, so this command is
  the only lint for the new file.
- `just test-l2 level2_` (claudine): the three new tests fail as intended; the
  66 pre-existing L2 tests that ran before cancellation passed. A full L2 run
  was not completed in this phase.
- OS coverage: the reproduction is `#![cfg(unix)]` (tmux). No production code
  changed, so no cross-OS run was needed for Phase 1. Windows L2 parity for
  the eventual regression is Phase 7's concern.
- Claudine skill: not updated. Phase 1 changes no behavior, and the ordering
  docs change in Phase 8.

## Phase 2

- **Projection API.** `ComposeOptions::only_frontmatter_surface()` intersects the
  enabled operation set with `{FrontmatterInterpolation,
  FrontmatterShellExpansion}` (a frontmatter operation the caller disabled stays
  disabled); `is_frontmatter_surface_only()` is derived from the operation set, so
  the options fingerprint needs no new field.
- **Pre-approval gate.** Rather than skipping `validate_pre_approved` for the
  projection (design.md's first option), the gate now validates against
  `collect_frontmatter_shell_commands` when `is_frontmatter_surface_only()`. This
  keeps "no `$(...)` runs until every command this compose could run is approved"
  while never walking the body graph. Full compose still uses the condition-blind
  graph collector.
- **Collector.** `preflight::collect::collect_frontmatter_shell_commands` (re-exported
  from `compose`) wraps the private `scan_one_frontmatter`; excluded keys contribute
  no commands.
- **Gate finding.** Frontmatter `$(...)` expansion already prepares (approval-checks)
  every candidate, both ternary branches included, before executing any. A mutation
  that skipped the up-front check for the projection left
  `projection_refuses_an_unapproved_frontmatter_command_before_any_runs` green, so for
  frontmatter commands that check is defense-in-depth. It still earns its place: it
  keeps `validate_pre_approved`'s documented invariant and surfaces dynamic-shape
  errors first. The collector is needed anyway for Claudine's bootstrap approval
  (Phase 3).
- **Docs.** `//!` section "Frontmatter-surface projection" on `compose/mod.rs`
  (what it projects, what it never does, why it exists); method docs with a doctest
  on `only_frontmatter_surface` and `collect_frontmatter_shell_commands`; the
  `validate_pre_approved` doc and the pipeline gate comment now describe the
  frontmatter-only branch. Added short notes to `darkmatter/docs/inline/preflight-checks.md`
  and the darkmatter skill (`.claude/skills/darkmatter/compose.md`) because the
  public API changed. No Claudine skill change: Claudine code is untouched this phase.
- **Impact analysis.** GitNexus upstream impact was LOW for `validate_pre_approved`,
  `run_compose_pipeline_internal`, and `scan_one_frontmatter`. A repo scan of
  `.only(` callers found none that combine a frontmatter-only operation set with
  pre-approved commands, so no existing caller changes behavior.

### Requirement-to-test mapping

All tests are in `darkmatter/lib/tests/frontmatter_surface_projection.rs` (L1,
public API through `Markdown::compose_with`, temp-dir fixtures, no host state).

| Requirement | Test |
|---|---|
| Missing include does not fail the projection; body verbatim; no page-block, body interpolation, or lifecycle effect | `projection_reads_frontmatter_without_dereferencing_missing_includes` |
| Frontmatter interpolation: caller-set `spec` → `dirname` → derived `log`; `file_exists` false before the file exists | same |
| Lifecycle subtree keeps `{{ }}`; `deferred_frontmatter_keys` populated | same |
| `file_exists` true once the generated file exists (dependent output) | `projection_sees_the_generated_file_once_it_exists` |
| Full compose still fails on the missing include (with and without an approval set); `compose_preflight` still fails | `full_compose_and_preflight_still_fail_on_the_missing_include` |
| Pre-approved set (empty) does not trigger the body graph walk | `projection_with_pre_approved_commands_skips_the_body_graph_walk` |
| Approved frontmatter command runs; unapproved body `::shell` never runs; missing include tolerated | `projection_runs_approved_frontmatter_commands_but_never_body_commands` |
| Unapproved frontmatter command refused before any runs (negative) | `projection_refuses_an_unapproved_frontmatter_command_before_any_runs` |
| Full compose still refuses an unapproved body command before any command runs; the same set suffices for the projection | `full_compose_still_refuses_an_unapproved_body_command` |
| `deferred_schema_verdict` honored; owned verdict still reported (negative) | `projection_honors_the_schema_verdict_setting` |
| Caller-disabled frontmatter op stays disabled (`$(...)` literal, no effect) | `projection_keeps_a_caller_disabled_frontmatter_operation_disabled` |
| `is_frontmatter_surface_only` classifies every operation | `frontmatter_surface_only_reflects_every_body_operation` |
| Collector: frontmatter commands incl. both ternary branches; no body `::shell`/`::shell-block`; excluded lifecycle key contributes none; missing include tolerated | `frontmatter_collector_enumerates_frontmatter_commands_only` |

Mutation check: forcing `validate_pre_approved` back to the graph collector turned
three tests red (`..._skips_the_body_graph_walk`,
`..._runs_approved_frontmatter_commands_but_never_body_commands`,
`full_compose_still_refuses_an_unapproved_body_command`). Restored afterwards.

The plan's passive corpus and shipped-artifact e2e requirements do not apply to this
phase. It changes no parser, schema, template, or shipped prompt; the shipped-prompt
e2e is Phase 7 (AC10). The existing preflight acceptance tests in
`compose/preflight/mod.rs` (`preflight_blocks_frontmatter_shell_when_body_unapproved`,
`preflight_blocks_shell_blocks_when_later_block_unapproved`, …) keep guarding full
compose.

### Gates (macOS host)

- `darkmatter/`: `just lint` passed (darkmatter, darkmatter-cli, dmls, zed-dmls-cli,
  wasm32 check). `just test`: 7935 passed, 7 skipped, 0 failed. Doctests for the
  two new examples passed (`cargo test -p darkmatter --doc`). `cargo doc` with
  `-D rustdoc::broken_intra_doc_links`: the new links resolve. A redundant explicit
  link target in the new `//!` text was fixed; the other rustdoc warnings in that run
  are pre-existing and unrelated.
- `claudine/`: `just lint` passed. `just test`: 7204 passed, 9 skipped, 0 failed.
  Claudine code is unchanged; this run guards the Darkmatter dependency change.
- Not run: `just test-l2` (plan Checkpoint 2 asks only for L1 and lint). Cross-OS
  runs: none. The new tests use absolute portable paths, `touch`, and `echo`,
  the same way the existing `preflight/mod.rs` acceptance tests do, and they
  assert on behavior, never on separator text. I judged the OS risk low and left
  it to CI.
- Disk: 24 GiB free at the start. `just sweep` reclaimed nothing; no ENOSPC occurred.


## Phase 3

- **`BootstrapPreparation`** (`claudine/lib/src/composition/prepare/bootstrap.rs`,
  re-exported from `claudine::composition`). Fields: `mode`, `entry`,
  `resolved_path`, `source_repo_root`, `input_layers` (`CallerInputLayers`),
  `effective_frontmatter`, `selection_hints`, C3-stamped `lifecycle`,
  `deferred_lifecycle_keys`, `compose_context`, and `document_epoch`. It has no
  prompt, closure plan, launch schema, or warnings, and it is not a
  `PreparedComposition` variant. The plan's "exactly" list omits
  `effective_frontmatter` and `compose_context`. I kept them because they are
  frontmatter-only, not body-derived, and Phase 4 needs them: event-time
  interpolation of `initialize` (`ensure_file: "{{log}}"`) reads the effective
  frontmatter, and the stabilized reread must extend the same context snapshot
  rather than recapture it. Frontmatter warnings are left for the stabilized
  reread, so they are not reported twice.
- **`prepare_bootstrap(BootstrapRequest { entry, mode, source, options })`** in
  `prepare/service.rs`. It forces `defer_schema_verdict = true` and delegates to
  `prepare.rs::compose_bootstrap`, which:
  - runs the authored removed-validation-key scan;
  - records the epoch consumer `EffectiveFrontmatter` only (no `Body`, because
    no body is read);
  - records one compose operation;
  - composes `source.markdown` with
    `canonical_compose_options(..).with_deferred_schema_verdict(true).only_frontmatter_surface()`.
    The schema phase is `None` for chained and `Launch` for inline, matching
    full preparation.
  - runs the shared post-compose steps.

  Both `ChainedDocument` and `InlineFrontmatterPrompt` are supported. The inline
  bootstrap does not require `prompt`, because the prompt is body-equivalent;
  the stabilized reread still raises `PromptPropertyMissing`. A
  `debug_assert!` on `entry.stages().emits_initialize` rejects Retry, Resume,
  and LoopIteration (ruling 2). This is the first production caller of
  `DocumentEntryReason::stages()`.
- **Shared post-compose steps (refactor).** `prepare_direct_with_prompt` and
  `prepare_inline` duplicated the same block: effective removed-key scan →
  agent/model/interactive hints → `parse_lifecycle_config` →
  `validate_no_nested_spans_in_literals` → `resolve_lifecycle_shell_commands`
  (C3) → `validate_no_err_in_no_error_events`. It is now one private
  `effective_surface` helper used by direct, inline, and bootstrap, so the
  bootstrap cannot drift from full preparation (R4). The error order is
  unchanged. GitNexus impact on `prepare_direct_with_prompt`, `prepare_inline`,
  and `canonical_compose_options` was **HIGH** (17–18 upstream symbols, 4
  modules). They are central to all preparation, so the change is limited to
  the behavior-preserving extraction. The full L1 suite guards it.
- **Narrow-gate integration.** `resolve_lifecycle_shell_approvals(&b.lifecycle,
  &b.resolved_path, &[LifecycleSignal::Initialize], opts)` works directly on the
  bootstrap fields. The frontmatter `$(...)` commands run *during* the
  bootstrap compose, so they need approval first. I added
  **`preflight_bootstrap_shell(source, mode, options, approval_options)`**, the
  bootstrap counterpart of `preflight_document_shell`. It collects via
  Darkmatter's `collect_frontmatter_shell_commands` using the same
  `bootstrap_compose_options` helper as the compose, and approves through
  `preflight::approve_discovered_commands` (now `pub(super)`). A collection
  failure returns an empty set, so the compose surfaces the typed error, the
  same way `preflight_document_shell` treats a discovery failure.
  - Unlike `preflight_document_shell`, it applies `env_overrides` to the
    context, so the approved bytes equal the executed bytes.
    `preflight_document_shell` does not apply them. That looks like a latent
    inconsistency; I left it untouched (out of scope).
- **Drift guard.** `claudine/cli/tests/composition_seams.rs::compose_with_allowlist_holds_the_line`
  failed on the new `compose_with` site. I added a reasoned
  `COMPOSE_WITH_ALLOWLIST` entry for `composition::prepare::compose_bootstrap`.
  This is the only `claudine/cli` change. The plan says Phase 3 touches only
  `claudine/lib`, but the guard exists to force exactly this acknowledgement.
- **Docs.** Updated the `canonical_compose_options` doc to list the bootstrap
  read. Added a paragraph to `.claude/skills/claudine/architecture.md` next to
  the canonical-service description. `.opencode/skill/claudine/`, named by
  Phase 8, does not exist in this worktree.
- **No CLI behavior change.** Nothing outside tests calls `prepare_bootstrap`
  or `preflight_bootstrap_shell` yet (Checkpoint 3).

### Requirement-to-test mapping (Phase 3)

All tests are in `claudine/lib/src/composition/prepare/bootstrap/tests.rs`. They
are L1 and use the public `claudine::composition` API with temp-dir fixtures, a
temp approval `policy_root`, and no host state. The reported caller spelling is
kept: `spec=fixes/2026-09-14-demo/spec.md`, a launch-relative path, with
`log: "{{ dirname(spec) + '/implementation-log.md' }}"`.

| Requirement | Test |
|---|---|
| Bootstrap succeeds on a body that includes the missing file (nested `file_exists(log)`/`phase > 1` guards plus an unconditional include); caller layers retained; `log` derived; initialize shell C3-stamped; lifecycle subtree keeps `{{log}}`; no effect runs. The same document through `prepare_document` still fails (`ComposeFailed`); after the file exists (standing in for `initialize`), full preparation includes its content at both sites | `bootstrap_reads_the_lifecycle_surface_of_a_body_that_includes_a_missing_file` |
| Inline-mode parity: body and `prompt` include missing files; bootstrap succeeds; full inline preparation fails, then succeeds after creation | `inline_bootstrap_reads_the_lifecycle_surface_of_a_prompt_that_includes_a_missing_file` |
| Narrow gate approves only the `initialize` command, in its stamped bytes; full preparation stamps identical lifecycle bytes; the post-stabilization full audit with the shared cache prompts only for the command the gate never saw | `initialize_gate_approves_stamped_bytes_and_the_full_audit_reuses_them` |
| Frontmatter `$(...)` is approved via `preflight_bootstrap_shell` and runs; body `::shell` never runs; the missing include is tolerated. Negative: an empty pre-approved set refuses the frontmatter command (`ShellExpansionFailed`) | `bootstrap_runs_approved_frontmatter_commands_and_never_body_commands` |
| Negative: no handler and no whitelist → `ShellApprovalUnavailable` | `bootstrap_frontmatter_preflight_without_a_handler_refuses_an_unlisted_command` |
| A malformed lifecycle (`say`/`say_first` conflict) and a late-binding root in an initialize shell command raise the same typed errors (`LifecycleSayConflict`; `LifecycleShellResolution` with property `initialize.stack[0].action[0].command`) from bootstrap and full preparation, in both modes | `a_malformed_lifecycle_fails_bootstrap_and_full_preparation_identically` |
| Bootstrap and full preparation agree for a body-independent document: effective frontmatter (including type coercion and a caller value), selection hints, lifecycle (C3 bytes), deferred keys, path, repo root, and caller layers, in both modes | `bootstrap_and_full_preparation_agree_on_a_body_independent_document` |
| The verdict is always withheld (the caller's `defer_schema_verdict: false` is overridden); the validating read still fails `MissingProperties` | `bootstrap_always_withholds_the_schema_verdict` |
| Epoch observation: exactly one `effective-frontmatter` consumer and no `body` | `bootstrap_observes_the_prepared_context_as_frontmatter_only` |
| Ruling 2: a retry never takes a bootstrap read (debug assertion) | `a_retry_never_takes_a_bootstrap_read` |

Mutation checks, each restored afterwards:

- Dropping `.only_frontmatter_surface()` turned the chained,
  frontmatter-command, epoch, and (after tightening) inline tests red. The
  first inline fixture had no body include and stayed green, so I added one.
- Recording a `Body` consumer turned the epoch test red.

Passive corpus and shipped-artifact e2e do not apply. This phase changes no
parser, schema, template, or shipped prompt, and the new entry has no CLI
caller yet. The shipped-prompt e2e is Phase 7 (AC10).

### Gates (macOS host)

- `claudine/`: `just lint` **pass** (exit 0), including after the allowlist
  edit. `just test` **pass**: 7214 run, 7214 passed, 9 skipped (Phase 2's 7204
  plus 10 new).
  - The first `just test` run failed only on
    `composition_seams::compose_with_allowlist_holds_the_line`, which the
    allowlist entry fixed.
- `cargo doc -p claudine --no-deps`: no warnings from the new docs.
- GitNexus `detect-changes --scope all`: medium. Its affected flows are Phase
  2's Darkmatter `run_compose_pipeline_internal`.
- Not run: `just test-l2`. Checkpoint 3 asks for L1 and lint only, and the
  three Phase 1 L2 reproductions stay red until Phase 4. No cross-OS run was
  done. The new tests assert strings and identities built from forward-slash
  literals, execute only `echo` (already executed without cfg gates by the
  existing `preflight/tests.rs` and Darkmatter projection tests), and never run
  the `touch`/`mkdir` strings they approve. I judged the OS risk low and left it
  to CI.
- Disk: 59 GiB free at the start; no ENOSPC.

## Phase 4

Phase 4 wires the staged boot into the command coordinator, the pipeline
boundary, the loop route, and the harness's in-place adoption. `design.md`'s
loop-path decision is followed. `initialize` now runs before any
`PreparedComposition` is built for a live document that authors it.

### Design as implemented

- **One predicate.** `defers_schema_verdict_to_initialize` (prep.rs) moved to
  `wrap::composition::staged_boot::authors_initialize`. It is still keyed on the
  authored `initialize` key. The coordinator and the harness now both call this
  one function (ruling 2).
- **Coordinator (`compose/prep.rs`).** `prepare_and_run_active_document`
  computes `staged = !dry_run && authors_initialize(&source)` for the caller's
  document *and* for an adopted target (ruling 2 clarification).
  - A staged document skips the command-level body audit (A). The audit now
    lives in `eager_shell_preflight` and runs only for
    `DocumentBoot::Eager`.
  - `execute_loop_or_single` takes the bootstrap read and the narrow gate
    (`staged_boot::bootstrap_document`: `preflight_bootstrap_shell` →
    `prepare_bootstrap` → `resolve_lifecycle_shell_approvals(&[Initialize])`)
    before loop recognition picks a route. A failure here happens before the
    document's lifecycle exists, so it is not routed to any catch stack.
- **Single route (`run_staged_single`).**
  1. It builds a `LifecycleRunGuard` over the bootstrap lifecycle
     (`StagedLifecycleRuntime`: settings, effect engine, and the epoch snapshot
     extended for the effective frontmatter plus the identity env).
  2. It emits `initialize` through the pipeline's existing `route_initialize`
     (`staged_boot::route_staged_initialize`). The process CWD is switched to
     `child_cwd` for the event, as the pipeline route did, and restored to the
     launch area afterwards.
  3. A proxy commits through `pipeline::commit_initialize_proxy`. That helper was
     extracted from `provider_run_handoff`, which now calls it too, so there is
     one commit and refusal routing (AC29). A `skip` returns `Done(0)`.
  4. On proceed, `reread_and_audit` loads the document fresh from disk
     (`harness_orch::load_overlaid_document`, extracted from
     `load_overlaid_source`) and re-merges the proxy overlay. It reuses the
     bootstrap options (caller layers, file-resolution context, epoch,
     snapshot). The snapshot is extended for context groups the reread newly
     names and the identity env is reapplied, so approved bytes equal executed
     bytes; see Phase 3 note (5). The full body audit runs through
     `preflight_document_shell`, and approvals accumulate.
  5. `prepare_collecting_missing(Validate)` runs next. This is the eager route's
     interactive `MissingProperties` retry, now shared by both routes.
  6. The guard is repointed at the stabilized lifecycle. The pipeline then runs
     through `execute_staged_composition`, the external-guard seam the loop
     route already used. The pipeline therefore never routes `initialize`
     again, and the runner still audits the stabilized lifecycle. The request
     carries no `adopted_handoff`.
  7. A reread, audit, or verdict failure goes through
     `route_stabilized_failure`, which fires the document's `blocked`/`finalize`
     once (R5/AC8).
- **Loop route (`build_and_run_loop`).**
  - The seed and lifecycle come from
    `claudine::composition::build_loop_seed_from_bootstrap` (new lib function;
    it shares the control-variable lift with `build_loop_seed_with_lifecycle`).
  - The engine stays the only emitter of `initialize`.
  - Iteration 1 is the stabilized reread (`reread_and_audit` → `prepare_staged(Validate)`).
    Its failure is routed through the engine's guard explicitly, because the
    engine only records an iteration `Err`.
  - Iterations 2+ compose from the stabilized source and its approvals, never
    from the pre-`initialize` snapshot (AC6).
- **Harness adoption (`loop_control.rs`).** Sequence *tasks* reach this path:
  `task_run.rs` passes no handoff ledger, so an `initialize` proxy is adopted
  inside the harness (`coordinator.adopt`).
  - The new `initialize_adopted_target_phase` handles a target that authors
    `initialize` and still owes a `Full` boot. It takes
    `bootstrap_harness_prompt` (a `MaterializedHarnessPrompt` built from the
    bootstrap read: no prompt, closure plan, or launch schema), runs
    `run_initialize_stages` on it, and marks the coordinator
    `BootstrapStage::TargetInitialized`.
  - The normal flow then reads the post-`initialize` document, and
    `bootstrap_adopted_document_phase` finishes from stage 4, including the R6
    launch rebuild (`stage != StabilizeOnly`).
  - A target without `initialize` keeps the eager `Full` boot.
- **Dry run.** Unchanged: `staged` is false, so the eager audit may still
  report a transclusion only `initialize` would create (ruling 5).

### Finding: sequence tasks had the same defect

A `sequence` step with `prompt: router.md` whose router proxied to a
generated-include target failed with `Transclusion error: I/O error: File not
found`. It did so in the in-harness adopt path's full read (D), not in the
coordinator. I confirmed this by disabling `initialize_adopted_target_phase`:
the manual run failed that way, and it succeeded with the phase enabled. It is
now covered by an L1 test. This is the proxied-target path of a sequence task,
not OQ1's static preflight of the task's own document, which Phase 5 still
owns.

### Deviations and notes

- **GitNexus impact ran after editing, not before.** That is a process miss on
  my part. Upstream impact: `prepare_and_run_active_document` HIGH (8),
  `execute_loop_or_single` HIGH (6), `build_and_run_loop` LOW,
  `prepare_attempt_phase` LOW, `bootstrap_adopted_document_phase` LOW,
  `build_loop_seed_with_lifecycle` LOW, `load_overlaid_source` LOW.
  `provider_run_handoff` was UNKNOWN; a text search shows its one caller is
  `execute_composition_request_inner_with_guard` in `pipeline.rs`. The HIGH
  symbols are the coordinator entry points for compose, inline-compose, and
  sequence proxy steps. The full L1 and L2 suites guard them.
  `detect-changes --scope all`: medium. The affected flows are Phase 2's
  Darkmatter compose pipeline.
- **Tests location.** The plan suggested extending existing seam files. The
  entry-path rows went into a new focused L1 file
  (`cli/tests/compose_initialize_staged_boot.rs`) instead. The existing guard
  (`composition_seams.rs`) gained a reasoned `PROXY_TRANSITION_SITE_BASELINE`
  entry for `route_staged_initialize`, and `staged_boot.rs` joined
  `TRANSITION_PATH_FILES`.
- **Mechanical move.** `harness_orch/prompt.rs` inline tests moved to
  `harness_orch/prompt/tests.rs`. The two new tests pushed the module past the
  `test_placement` 300-line limit.
- **`dispatch_inventory`.** A first draft used `Provider::Claude` as a fallback
  label, which added a dispatch site. I replaced it with an explicit error
  (live runs always resolve a target eagerly), and the committed inventory is
  unchanged.
- **Known redundancy (perf only, not fixed).** A `TargetInitialized` harness
  target still takes the post-`initialize` full audit and deferred read in
  `prepare_attempt_phase` before stage 4 rereads again.
- **Loop iteration 1** does not offer the interactive `MissingProperties`
  collection. The loop route never did.
- **Comment drift fixed:** `runner.rs` (`stabilize_after_initialize` rationale),
  `pipeline.rs` (adopted-target and `initialize`-event comments; commit comment),
  `prep.rs` (schema-stage and preflight comments), and the `load_overlaid_source`
  doc. The architecture skill paragraph now describes the wired ordering. It
  replaces the "belongs to Phase 4" sentence.

### Requirement-to-test mapping (Phase 4)

| Requirement | Test(s) | Level |
|---|---|---|
| Reported shape: router `initialize` proxies to a target whose `initialize` creates the guarded, included log (AC1/AC2, phase 1 and a later phase) | `level2_proxied_initialize_creates_log_before_guarded_phase_one_body_is_discovered`, `level2_proxied_initialize_creates_log_before_guarded_later_phase_body_includes_it` (Phase 1 reproductions, now green) | L2 |
| R2/AC5: an unapproved `initialize` shell has no effect (direct route) | `level2_direct_initialize_shell_has_no_effect_when_approval_is_unavailable` (now green); `an_unapproved_initialize_shell_is_refused_before_the_body_is_discovered` (gate precedes discovery; no catch events) | L2, L1 |
| Direct `compose` staged boot: `initialize` once, before the provider; prompt includes the created file | `compose_initialize_creates_a_file_the_body_includes` | L1 |
| `inline-compose` parity (prompt includes the created file; closure keeps the lifecycle) | `inline_compose_initialize_creates_a_file_the_prompt_includes` | L1 |
| Proxy adoption staged boot (each document's `initialize` once) | `a_proxied_target_initialize_creates_a_file_the_target_body_includes` | L1 |
| Loop-owning target: seed without body read, one `initialize`, iterations 1 and 2 include the file | `a_looping_target_first_iteration_includes_what_initialize_created` | L1 |
| Harness-adopted target (sequence task, no command ledger) | `a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes`; unit: `bootstrap_harness_prompt_reads_an_initialize_target_without_its_body`, `bootstrap_harness_prompt_leaves_a_target_without_initialize_to_the_eager_read`, `a_target_initialized_from_its_bootstrap_read_owes_only_the_stabilized_tail` | L1 |
| AC8/R5: a file still missing after `initialize` fails once, via `blocked`/`finalize`, with no provider | `a_file_still_missing_after_initialize_blocks_once_without_launching` (exactly one `File not found`; events `initialize, blocked, finalize`) | L1 |
| AC11/R6: no-`initialize` document keeps eager failure and diagnostic (`TransclusionError` + `File not found: generated/notes.md`, no lifecycle event) | `a_document_without_initialize_still_fails_eagerly_on_a_missing_include` | L1 |
| Ruling 5 dry run: no `initialize` effect; missing include still reported | `a_dry_run_never_runs_initialize_and_still_reports_the_missing_include` | L1 |
| `skip` ends the run with no body discovery | `an_initialize_skip_ends_the_run_before_the_body_is_discovered` | L1 |
| Existing proxy/lifecycle semantics (cycles, hop limit, missing target → source `blocked`/`finalize`, target `skip`/`error`, schema verdict after `initialize` on direct/loop/proxy/recovery routes, overlay-installed gate, reread after mutation) | the 34 pre-existing `initialize`-related L2 tests in `level2_lifecycle_control.rs`, `level2_lifecycle_loop.rs`, `level2_typed_error_render_capture.rs`, `level2_provided_partial_file_capture.rs` | L2 |
| New proxy endpoint is baselined | `composition_seams::every_production_proxy_route_carries_the_typed_handoff` | L1 |

The original failing input is preserved in the L2 reproductions. The L1 rows
use a smaller document with the same structure (an `initialize` that creates
a file, and a body `::file` of it).

**Mutation checks** (each restored afterwards):

- Forcing `staged = false` turned 7 of the 9 L1 rows then present red. The 2
  that stayed green are the unchanged-boundary guards (eager, dry run), as
  intended.
- Seeding the staged loop from a full compose turned the loop row red.
- Skipping `route_stabilized_failure` turned the AC8 row red.
- Disabling `initialize_adopted_target_phase` made the sequence-task scenario
  fail with `File not found`. The L1 row was added afterwards and passes.

Passive corpus and shipped-artifact e2e do not apply to this phase. It changes
runtime ordering, not a parser, schema, template, or shipped prompt. The
shipped `implement-plan.md` e2e belongs to Phase 7 (AC10).

### Gates (macOS host)

- `claudine/`: `just test` **pass**: 7227 run, 7227 passed, 9 skipped.
  - Earlier runs failed on guard tests the change legitimately tripped:
    `composition_seams` proxy baseline, `error_guards` boxed-diagnostic,
    `dispatch_inventory`, and `test_placement`. Each was fixed as described
    above, not by relaxing the guard.
- `claudine/`: `just lint` **pass**. The first attempt failed on
  `clippy::result_large_err` in `staged_boot.rs`, fixed with the same
  file-level allow and rationale as `harness_orch/prompt.rs`.
- `claudine/`: `just test-l2` **pass**: claudine-cli 229/229, claudine-gen 3/3.
  This includes the three Phase 1 reproductions (red before this phase).
- Checkpoint 4 AC11 posture: a no-`initialize` document with a missing
  transclusion still fails at the command-level audit with `TransclusionError`
  / `File not found: <path>` and fires no lifecycle event (L1 row above).
- **Cross-OS: not obtained (environment, not code).**
  - `./scripts/cross-check.sh --os linux claudine-cli -E '…staged/initialize/bootstrap…'`
    gave up after 1800 s. `build-linux`'s `ci-verification/.cross-check.lock`
    is held by `{"purpose": "nightly-reward-spike", "owner":
    "reward-20260914-c3e60d0", "started": "2026-09-14T18:25:30Z"}`.
  - `--os windows` failed during the clone sync with `fatal: write error: No
    space left on device` on `build-win-native`.
  - Per the script's guidance, I did not remove the other run's lock or delete
    anything on the shared host.
  - OS risk assessment:
    - The new code switches the process CWD with the existing cross-platform
      `switch_process_cwd` / `set_current_dir` pair, the same way the pipeline
      route did.
    - It reads the document through the existing overlay loader, and it
      resolves references through the retained `FileResolutionContext`: no
      new path spelling or prefix checks.
  - Coverage gap: the new L1 file is `#![cfg(unix)]` (sh fake provider), so
    Windows coverage of the staged boot comes only from CI and from the
    platform-neutral unit tests.
- Disk: 40 GiB free on `/Volumes/coding` at the start; no ENOSPC locally.

## Phase 5

Phase 5 implements OQ1 Option A at the sequence static-preflight boundary. The
failure contract does not change: same error, same code, same abort-all
timing. The phase adds a bounded, actionable note plus characterization and
negative coverage.

### Characterization (before any code change)

A manual run with the built binary against a sequence whose `prompt:` step
names a document whose `initialize` creates `generated/notes.md`, which its
body includes via `::file`, failed at static preflight with
`⤫ TransclusionError: I/O failure / File not found: generated/notes.md` and
no guidance. The failing site is `approve_preflight_graph`
(`cli/src/commands/wrap/sequence/mod.rs`) →
`composition::resolve_graph_shell_approvals` → Darkmatter `compose_preflight`
per prompt document, raised as
`CompositionError::PreFlightDiscoveryFailed(MarkdownError::Transclusion(TransclusionError::Io(NotFound)))`.

### Finding: a prior step cannot create the include either (plan deviation)

The plan's diagnostic wording suggested "or add a prior task that creates
it". I probed that: a sequence whose first step is
`shell: "mkdir -p generated && echo hi > generated/notes.md"` and whose
second step is the prompt fails identically. Static preflight composes every
prompt document before step 1 runs. The note therefore tells authors to
create the file **before starting the sequence**, and says that neither an
earlier step nor the document's own `initialize` can create it. Suggesting a
prior task would have been wrong advice.

### Design as implemented

- `approve_preflight_graph` records the document the graph walk is composing
  (a `RefCell` set inside the existing per-document `compose_options`
  closure). This is the only typed way to name the failing prompt document:
  the Darkmatter I/O error carries no source context, and parsing `Display`
  is prohibited by the error architecture.
- On failure, `is_missing_include` matches exactly
  `PreFlightDiscoveryFailed(Transclusion(Io(kind == NotFound)))`. On a match,
  `emit_missing_include_note` writes one Prose `note:` line to stderr
  (naming the document's file name), and the **unchanged** error is then
  returned. The top-level renderer prints the typed block after the note.
- Rejected alternatives:
  - A new `CompositionError` wrapper variant with a render appendix. It would
    touch the exhaustive code/detail/role/status-block matches and the error
    guards for guidance-only prose, which is too broad for "no contract
    change".
  - Wrapping the error in an eyre context. The walker renders only the
    selected diagnostic, so the context would be invisible.
- Not relaxed: no staging in sequences, no new approval prompt, no change to
  `resolve_graph_shell_approvals`. `--dry-run` takes the identical walk, so
  it gets the same error plus the note, and nothing else changes.

### Observations for review (pre-existing, not changed)

1. **A sequence step's prompt is composed before its own `initialize` runs.**
   With the include pre-seeded, the live run succeeds and `initialize` runs
   once at the step's turn. The prompt the provider receives contains the
   file as it was *before* `initialize` appended to it. This is the sequence
   contract that Option A keeps. The L1 success row pins it with a comment
   saying OQ1 Option B/C would change it.
2. **`sequence --dry-run --yolo` executes a `shell:` step.** The row showed
   `step-ran.txt` created during a dry run (`✓ step 1/2 rendered (dry-run)`).
   Phase 5 only touches the preflight error path, so this is pre-existing.
   The Dry Run docs promise "no filesystem side effects of its own" and carve
   out only `::shell` spans in the document graph, so this may be a docs/code
   gap worth a separate fix. The test deliberately does not pin it either
   way.

### Requirement-to-test mapping (Phase 5)

| Requirement | Test(s) | Level |
|---|---|---|
| Characterization: a sequence prompt whose `initialize` would create an included file fails static preflight with the typed missing-file error before any step starts; no side effects (no `initialize` marker, no `generated/`, earlier `shell:` step did not run, sequence/target documents byte-identical); no audio published | `sequence_initialize_include_preflight::a_prompt_whose_initialize_would_create_its_include_fails_sequence_preflight` | L1 (cross-platform; no provider needed) |
| Same boundary under `--dry-run` (unchanged dry-run failure plus note, no side effects) | `…::a_dry_run_reports_the_same_preflight_failure_without_side_effects` | L1 (cross-platform) |
| Actionable diagnostic: the full note text renders exactly once, precedes the error block, and the selected diagnostic code (`composition.failed`, via `CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT`) is unchanged; exactly one `File not found` | both rows above (`assert_missing_include_failure`) | L1 |
| Diagnostic renders in a real terminal (Checkpoint 5 capture; detached tmux, no focus) | `level2_initialize_generated_transclusion::level2_sequence_preflight_note_renders_for_an_include_initialize_would_create` | L2 |
| Note is bounded: only `PreFlightDiscoveryFailed` + transclusion `Io(NotFound)`; not `PermissionDenied`, not other transclusion errors, not the same I/O error under `ComposeFailed` | `commands::wrap::sequence::tests::only_a_not_found_include_at_preflight_earns_the_missing_include_note` | L1 unit |
| Note never appears outside sequence preflight (direct `compose` missing include keeps its eager failure, same code, no note) | `…::a_direct_compose_missing_include_carries_no_sequence_note` | L1 (cross-platform) |
| Negative: include exists → sequence runs unchanged (earlier step runs, `initialize` once, provider receives body + include, no note) | `…::existing_include::a_sequence_whose_include_exists_runs_unchanged` | L1 (unix) |
| Negative: include exists → `--dry-run` still succeeds, no note, no `initialize`, no prompt delivered, include untouched | `…::existing_include::a_dry_run_whose_include_exists_still_succeeds_without_side_effects` | L1 (unix) |
| No collision with Phase 4's proxied sequence-task path | `compose_initialize_staged_boot::a_sequence_task_proxy_target_initialize_creates_a_file_its_body_includes` (still green) | L1 |

The original failing input shape (`initialize` → `ensure_file` + content
append; body `::file generated/notes.md`; referenced from a sequence `prompt:`
step) is used verbatim in the L1 rows. The representation variants covered
are live vs `--dry-run`, missing vs present include, and sequence vs direct
compose. The passive corpus / shipped-artifact e2e requirement does not apply:
no parser, schema, template, or shipped prompt changed.

**Mutation check.** Replacing the `emit_missing_include_note(document)` call
with a no-op turned both failure rows red (2/5); restored afterwards.

### Gates (macOS host)

- `claudine/`: `just test` **pass**: 7233 run, 7233 passed, 9 skipped.
- `claudine/`: `just test-l2` **pass**: claudine-cli 230/230 (includes the new
  tmux row), claudine-gen 3/3.
- `claudine/`: `just lint` **pass** (exit 0).
- The first L2 attempt failed only because the pane soft-wraps the one-line
  note mid-word ("seq/uence"). The assertion now compares with whitespace
  removed. The rendering itself was correct.
- GitNexus: `impact approve_preflight_graph` upstream **LOW** (1 direct
  caller, `execute_sequence`; 1 process), run before editing.
  `detect-changes --scope all` reports **high** for the branch's cumulative
  diff (29 files, 70 symbols, dominated by the Phase 2–4 coordinator
  changes). Phase 5's only changed production symbol is
  `approve_preflight_graph`, plus two new private helpers.
- **Cross-OS: not obtained (environment, not code).**
  - `just cross-check claudine-cli --os windows -E 'binary(sequence_initialize_include_preflight)'`
    failed during upload/fetch on `build-win-native`: `scp: write remote … Failure`,
    then `fatal: write error: No space left on device` (same as Phase 4).
  - `--os linux` waited on `build-linux`'s
    `~/ci-verification/.cross-check.lock/owner`, still
    `{"purpose": "nightly-reward-spike", "owner": "reward-20260914-c3e60d0", "started": "2026-09-14T18:25:30Z"}`.
    I stopped my waiting run after ~11 minutes.
  - Per the script's guidance, I did not remove the other run's lock or delete
    anything on either shared host.
  - OS risk assessment:
    - The production change is platform-neutral: a typed `ErrorKind::NotFound`
      match (Darkmatter constructs that kind itself, so it does not depend on
      the OS error mapping), a `Path::file_name` display, and a Prose line.
    - The three failure rows in `sequence_initialize_include_preflight.rs` are
      deliberately not `cfg(unix)`. They need no provider, and their `shell:`
      step never runs, so Windows CI exercises the note and the unchanged code
      there.
    - The two success rows are `cfg(unix)` (sh fake provider). The L2 row is
      `cfg(unix)` (tmux).

## Phase 6

### Findings

- **The planned expression repair had already landed.** Commit `47e9259ab`
  changed `log` to `dirname(spec || plan) + '/implementation-log.md'` in both the
  shipped prompt and its Level 2 fixture, so no edit was needed. Every
  `parent_dir(...)` display use is untouched, and `parent_dir` keeps its meaning.
- **The logging instructions were wrong, and that is the fix in this phase.**
  `initialize` runs `ensure_file: "{{log}}"` before the body composes. So
  `::block when="!file_exists(log)"` (the "create the log, add title +
  metadata" instructions) could never render. A fresh implementation was told
  "the log file already exists" and was never asked for the title or metadata.
  - Reproduced: the new e2e test, run against the pre-change fixture, fails at
    `a fresh log must be started`.

### Changes

- `prompts/_implement/implement-plan.md`, with the fixture mirrored byte for byte
  in the body:
  - Unstarted log block: `!file_exists(log) || (markdown_body_empty(log) &&
    is_empty(frontmatter(log)))`. The wording is now "has not been started yet;
    make sure the log file exists — it may already exist as an empty file — and
    then write to it".
  - Started log block: `file_exists(log) && (!markdown_body_empty(log) ||
    !is_empty(frontmatter(log)))`. The wording is now "already has content from
    earlier work; keep it and append to it". The nested `phase > 1`
    prior-entries pointer is unchanged.
  - Guarding with `file_exists` matters. `markdown_body_empty` and `frontmatter`
    raise a typed file-reference error on a missing file, and Darkmatter `||` /
    `&&` short-circuit (see `condition_or_short_circuits_on_true`). This keeps
    composition safe where the body composes without `initialize`, such as
    sequence static preflight and dry run.
- `shipped-hashes.json` was refreshed with
  `CLAUDINE_UPDATE_SHIPPED_PROMPT_HASHES=1`. The new body hash
  `962d16d860e2561a-19b48aa3de0ec0fb` equals `md hash
  prompts/_implement/implement-plan.md`.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| Fresh run: `initialize` creates an empty log, and the prompt asks the agent to start it (title + metadata) | `shipped_prompt_contract::shipped_implement_plan_logging_instructions_follow_log_content` (L1 e2e: `claudine compose --goose` over the fixture, fake provider captures the delivered prompt) |
| An empty or whitespace-only log counts as unstarted (boundary variants `""` and `"\n  \n"`) | same test |
| Frontmatter-only, body-only, or both: the log counts as started, `ensure_file` preserves the bytes (read/write/read across repeated invocations), and there is no re-title | same test |
| The prior-phase pointer appears only when `phase > 1` | same test (phase 1 vs phase 2 variants) |
| The fixture body is the shipped body, so the e2e exercises the shipped text | new `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body` |
| Passive corpus: every shipped block condition parses | existing `shipped_prompt_contract::shipped_prompts_have_parseable_schemas_and_expressions` |
| Shipped change is reviewed and pinned | existing `shipped_prompt_route_drift::shipped_implement_prompts_have_not_drifted_from_their_fixture` (hash refreshed) |
| Loop/schema semantics unchanged across the route | existing `fixture_preserves_the_shipped_schema_and_loop_semantics` and the L2 rows below |

### Gates

- Targeted: `cargo nextest run -p claudine-cli --test shipped_prompt_route_drift
  --test shipped_prompt_contract --test shipped_prompts --test
  compose_caller_file_provenance`: **36/36 pass**.
- Regression proof: the new e2e test **fails** against the `HEAD` fixture and
  passes after the change.
- `claudine/`: `just lint` **pass** (exit 0).
- `claudine/`: `just test` first run 7234/7235. The one failure was a LEAK-FAIL
  in the unrelated
  `claudine composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree`
  (32 s), which ran while `just lint` compiled concurrently. It passed 3/3 in
  isolation. The clean rerun was **7235/7235 pass** (9 skipped).
- `claudine/`: `just test-l2 level2_shipped_implement_plan
  level2_lifecycle_shipped_implement_route`: **3/3 pass**. These are the L2 rows
  that execute the edited fixture.
- Checkpoint 6 deviation: beyond the prompt, fixture, and hash manifest, the
  work group also touched two test files (the new e2e test and the body-identity
  guard). Both are test-only.

### Cross-OS

- No `just cross-check` was run. The change is prompt text plus tests, with no
  platform-dependent code. The Phase 5 log records `build-win-native` out of
  disk and `build-linux` locked by another run.
- The e2e test is `cfg(unix)` (`sh` fake provider), matching its neighbors in
  `shipped_prompt_contract.rs`.
- `fixture_body_matches_the_shipped_body` is portable. Both files check out
  under the same line-ending policy, which the existing hash guard already
  depends on.

## Phase 7

Phase 7 closes the AC1–AC12 acceptance matrix and records full package-area
evidence in [`evidence.md`](./evidence.md).

### Regression-first design

Before changing implementation code, three AC12 tests were enabled against the
old behavior. They failed on the exact authored references
`./generated/notes.md`, `&generated/notes.md`, and the shadowed implicit
`generated/notes.md`. The first two could not create their intended targets;
the third mutated the repository-root decoy rather than the existing
document-local file. Each test asserts the created or preserved identity,
resulting bytes, downstream provider prompt, and decoy exclusion.

The completed matrix adds repository-scoped `^generated/notes.md`, interpolated
`{{ ctx.repo_root }}/generated/notes.md`, missing implicit creation, and the
invalid escape `&../outside.md`. The negative case asserts failure before a
file or provider effect. Existing Phase 4–6 tests supply direct, inline, proxy,
loop, sequence, approval, dry-run, shipped-corpus, and repeated persistence
coverage. The exact AC-to-test mapping is in `evidence.md`.

### Implementation

- Lifecycle filesystem effects now parse document-authored path arguments as
  `biscuit_file::FileReference` and resolve them with the captured request
  context. Existing targets use normal document resolution and shadowing.
  Missing explicit-relative, repository-root, and repository-scoped targets use
  their request-scoped identities. A missing implicit relative target retains
  the prior mutation-root policy.
- Darkmatter's filesystem write guard now canonicalizes the deepest existing
  ancestor and reattaches any missing tail before containment comparison. This
  handles equivalent macOS spellings such as `/var` and `/private/var` while
  preserving the boundary: an in-root symlink to an outside directory is
  rejected before creation.
- The shipped-router end-to-end helper quotes the transclusion reference, so
  the real fixture also works when the checkout or temporary root contains a
  space.

### Verification

- Targeted Claudine acceptance: 23/23 pass after a 3/3 red regression proof.
  The final row covers `ensure_dir`, both `ensure_file` forms, `append_line`,
  and `append_jsonl` through one document-relative identity.
- Targeted Darkmatter containment: 2/2 pass.
- Claudine L2: 231/231 CLI and 3/3 generator tests pass.
- Darkmatter L1: 7,937/7,937 pass. Darkmatter L2: 18/18 library,
  69/69 CLI, and 3/3 DMLS tests pass.
- Claudine and Darkmatter `just lint`: pass; the latter includes the
  `wasm32-wasip2` Zed extension check.
- Claudine L1 ran all 7,257 tests: 7,250 passed, including every Phase 7 test;
  seven pre-existing spawn tests failed because inherited host state sets
  `HOME=/Users/ken/.claudine`. The failure is unrelated to the changed paths
  and is recorded rather than hidden by changing externally owned HOME state.
- Remote qualification was attempted. Linux was locked by another build,
  native Windows was out of disk, and WSL reset the connection. CI remains the
  qualification path for those three environments. See `evidence.md` for the
  exact commands and gap handling.

An intermediate rerun was blocked before tests by the shared target directory's
write permissions. The first draft of the all-effects fixture also used a
`.jsonl` transclusion and correctly received Darkmatter's unsupported-file-type
error; changing only that fixture target to `.md` made its appended JSON record
observable through the supported composition path. Neither is a product
failure; both are recorded in `evidence.md`.

No skill file changed. Phase 8 still owns the planned repository documentation,
Claudine skill update, and final comment-drift review.

## Phase 8

### Scope and test design before edits

This phase changes documentation and comments only; it introduces no runtime,
parser, schema, prompt, or persistence behavior. Existing Phase 7 working-tree
changes and unrelated work are preserved. The plan's doubled `claudine/` path
is a spelling error; this phase uses the fix directory named in the spec.
The `.opencode/skill/claudine` files resolve to the canonical
`.claude/skills/claudine` files, which are edited once.

| Documented requirement | Concrete verification |
|---|---|
| Live bootstrap precedes body discovery, including inline/proxy/loop routes | `compose_initialize_staged_boot` and `compose_initialize_acceptance`; L2 `level2_initialize_generated_transclusion` |
| Approval denial, post-initialize mutations, typed missing include, once-only initialization | Named AC5–AC8 tests in `evidence.md` |
| Eager no-initialize path, retry/resume, dry run remain unchanged | Named AC7/AC11 tests in `evidence.md` |
| Sequence static preflight requires includes before the sequence starts | `sequence_initialize_include_preflight` plus its generated-transclusion L2 diagnostic test |
| Shipped guarded log, exact original spec spelling, persisted contents | AC4/AC10 tests in `evidence.md`, `shipped_prompt_contract`, and `shipped_prompt_route_drift` |
| Frontmatter-only projection does not discover body dependencies | Darkmatter `frontmatter_surface_projection` |

No new test is necessary for prose-only corrections. Final gates are `just
test`, `just lint`, and `just test-l2` in both Claudine and Darkmatter; results
and any failures/skips will be appended to `evidence.md`. Markdown hashes use
Darkmatter. No formatting, staging, commits, or completion-directory move.

### Documentation and comment review

Updated pipeline, composition, lifecycle, and README guidance, plus the
canonical Claudine composition/lifecycle skill pages. The docs explain staged
eligibility, frontmatter-only bootstrap, narrow approvals, stabilized reread,
full condition-blind audit, schema verdict, failure ownership, once-only
initialization, dry-run behavior, and the sequence boundary. Existing-file
sequence mutation timing is explicitly documented; no claim is made that an
earlier sequence task can create an include in time for static preflight.

Reviewed the plan's named symbols: `prepare_and_run_active_document`,
`execute_loop_or_single`, `build_and_run_loop`, `resolve_shell_approvals`,
`prepare_document`, `prepare_bootstrap`, `run_initialize_stages`,
`bootstrap_adopted_document_phase`, `route_initialize`, and Darkmatter's
projection/options/preflight comments. One correction was needed:
`run_initialize_stages` claimed every failure bypassed lifecycle routing,
although only bootstrap/narrow-gate failures precede config installation.
Corrected the comment, keeping code unchanged. GitNexus upstream impact was
LOW, four impacted symbols, two direct callers
(`bootstrap_adopted_document_phase`, `initialize_adopted_target_phase`), and
zero recorded processes. The local registered index was dated 2026-09-17 at
commit `30d9802`; source inspection confirmed the current behavior.

Initial `just test --no-fail-fast` could not compile: the shared target contains
read-only `.rmeta` artifacts. No tests ran in that attempt. Validation is being
retried with isolated artifact placement; no shared permissions are changed.

The final review also corrected `resolve_shell_approvals`'s argument docs:
callers provide source Markdown, not necessarily already composed Markdown;
the condition-blind walker dereferences includes, so staged live callers must
pass the stabilized reread. GitNexus reports CRITICAL risk (18 impacted symbols,
four direct callers across compose, sequence, and harness paths, including the
`run_phase_1c_with_schema` process family). This was reported before editing;
only its argument documentation changed. No approval behavior changed.

The edited skill's pre-existing `reconcile_inline_artifact` relative link was
repaired. All eight edited topic/skill documents pass `md validate refs
--fragments`. README-wide validation reaches an existing shell directive and
requires approval; its new initialization link targets the independently
validated composition heading. No shell directive was executed for this check.

### L1 environment resolution

The isolated-target ordinary Claudine run repeated all seven Phase 7 HOME
assertion failures. Removing inherited `HOME`/`USERPROFILE` only from the test
command lets the existing spawn fixture use its intended temporary-directory
fallback: all nine targeted spawn tests passed. The full, unfiltered rerun
with that environment then passed **7,258/7,258**, with nine pre-existing
performance/diagnostic opt-ins skipped. No fixture, product code, or persistent
home configuration was changed. Both runs and exact failures are retained in
`evidence.md`.

### Final gates and closure

- Claudine: `just test` **7,258/7,258 pass** in the clean inherited environment;
  `just lint` **pass**; `just test-l2` **231/231 CLI + 3/3 generator pass**.
- Darkmatter: `just test` **7,937/7,937 pass**; `just lint` **pass**, including
  `wasm32-wasip2`; `just test-l2` **18/18 library + 69/69 CLI + 3/3 DMLS pass**.
- Exact commands, the initial shared-target permission error, all seven
  inherited-HOME failures, the resolving nine-test probe, and the nine Claudine
  and seven Darkmatter L1 skips are recorded in `evidence.md`. L2's non-L2
  exclusions are identified separately. No new targeted tests were added:
  this phase's changes are documentation/comments only, and the existing
  mapped acceptance suites all passed.
- No new OS-specific behavior was introduced. Phase 7's Linux, native Windows,
  and WSL qualification gaps remain explicit for normal CI; no new remote run
  is claimed and no human decision is needed to complete this documentation
  phase.
- Plan tasks are all checked. Plan/log frontmatter records the Phase 8 files,
  cumulative source/documentation inventories, touched packages, phase `8`,
  `implemented: true`, and `human_review: false`. The spec retains the requested
  `implemented: true` and `implemented_by: codex/default` and now links to the
  completed implementation/evidence rather than describing it as future work.
- The session-owned isolated target measured 29 GiB and was removed after the
  gates. Shared target files and pre-existing working-tree changes were left
  intact. No formatting, staging, commits, or `_completed` move occurred.

**Implementation complete, ready for review.**
