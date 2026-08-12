# Phase 1 baseline: launch-context capture ownership

This inventory freezes the production state before the launch-context seam is
introduced. Line numbers describe the Phase 1 tree and may move during later
phases; file paths and enclosing functions are the durable identifiers.

## Observable failure matrix

`claudine/cli/tests/ctx_launch_anchor_baseline.rs` invokes the real `claudine
compose --codex` binary from one synthetic launch package. A provider stub
captures the composed prompt, while lifecycle `warn` actions expose both an
unconditional interpolation and a guarded `when:` branch. The exact regression
inputs are `{{ ctx.area }}` and `{{ ctx.repo_root }}` in the body, a quoted
whole-value frontmatter property, the `when:` expression, and lifecycle output.

| Document location | Current `ctx.area` | Current `ctx.repo_root` | Eventual launch contract |
|---|---|---|---|
| launch repository root | empty | launch repository | `alpha-lib`, launch repository |
| launch package | `alpha-lib` | launch repository | `alpha-lib`, launch repository |
| opposing package | `beta-lib` | launch repository | `alpha-lib`, launch repository |
| external repository | empty | external repository | `alpha-lib`, launch repository |
| Rusty Biscuit `claudine-cli`, launched outside every repository | `claudine-cli` | Rusty Biscuit root | empty, absent repository |

The body, effective-frontmatter, guarded `when:`, and lifecycle observations
agree on every current value. This is useful consistency but the wrong anchor.
The test intentionally asserts the before-state; Phase 3 will flip the matrix to
the launch contract after the canonical owner exists.

The macOS fixture uses one canonical temporary-root spelling. Without that,
`/var` and `/private/var` can introduce a separate evidence/base path mismatch
that masks the anchoring regression. All fixture paths are still constructed as
`Path`/`PathBuf` values.

## Pre-selection graph behavior

`composition::sequence::preflight::tests::shell::graph_preselection_expands_target_identity_from_invocation_environment`
records that graph-phase commands currently expand all four target-dependent
roots successfully before a task target exists:

| Root | Approved before-state bytes |
|---|---|
| `ctx.agent` | `baseline-preselection-agent` |
| `ctx.model` | `baseline-preselection-model` |
| `env.AGENT` | `baseline-preselection-agent` |
| `env.MODEL` | `baseline-preselection-model` |

These are invocation-environment values, not resolved task identity. Phase 4
must replace successful graph expansion with the typed preflight rejection in
AC7; task/JIT phases with a selected target remain permitted.

## Request-local work baseline

`invocation_context::tests::relocation_matrix_reuses_request_evidence_without_ambient_fallbacks`
covers repository root, launch package, opposing package, and a nested external
repository in one request. The current accounting is:

- two Git-root discoveries and two topology probes, one for each repository;
- three topology reuses for the three sources in the launch repository;
- one OS initialization plus three reuses;
- four `repo` and four `agent` evidence reuses, with no captures for those
  already-retained groups;
- the invocation's original environment value on all four projections after
  the ambient environment is changed; and
- zero recorded ambient fallbacks.

There is not yet a counter for a prepared-context construction, a prepared
consumer observation, or a group extension. Phase 2 adds those counters so AC5
can distinguish one shared snapshot from two equal reconstructions.

## Production capture inventory

Classification vocabulary:

- **canonical prepared context** — an invocation-backed CLI route that must
  migrate to the launch capture/epoch owner;
- **live `current.ctx.*`** — intentionally event-time and outside this fix;
- **library compatibility fallback** — retained for callers that do not supply
  an invocation-owned prepared context; and
- **documentation command** — reports ambient context by design and does not
  prepare a document.

### Claudine library

| Site | Current anchor/evidence | Classification | Phase action |
|---|---|---|---|
| `system_prompt/prepare.rs:140`, `compose_prompt_markdown` | file parent or ambient CWD; ambient capture | library compatibility fallback | retain compatibility; instrument only if an invocation can reach it |
| `system_prompt/prepare.rs:223-224`, `build_shared_compose_context_with_invocation` | first file-backed prompt/appendix `SourceContext` | canonical prepared context | Phase 5 system-prompt owner |
| `system_prompt/prepare.rs:242`, built-in-only branch | launch CWD plus invocation environment only | canonical prepared context | Phase 5 launch API; preserve no fabricated source |
| `composition/sequence/preflight/mod.rs:145`, `build_preflight_graph` | sequence source parent; ambient capture | library compatibility fallback | retain public compatibility route |
| `composition/sequence/preflight/mod.rs:817-818`, `resolve_shell_bytes` | each origin's `SourceContext` | canonical prepared context | Phase 4 graph launch base plus origin file context |
| `composition/prepare.rs:177`, `derive_compose_context` | supplied launch fallback or document parent | library compatibility fallback | canonical CLI must always supply; account any invocation-backed miss |
| `composition/mod.rs:189`, `document_expression_resolution_context` | launch fallback or source parent | library compatibility fallback | canonical callers must supply prepared context |
| `composition/lifecycle/context.rs:498`, `RuntimeEventContext::capture_at_event` | event-time directory and live environment | live `current.ctx.*` | allowlist unchanged |
| `composition/lifecycle/executor.rs:773`, `early_binding_context` | launch base or prompt directory when prepared context is absent | library compatibility fallback | canonical lifecycle must never enter fallback |
| `composition/lifecycle/executor.rs:1709`, `capture_proxy_with_fallback` | proxy base when prepared context is absent | library compatibility fallback | canonical proxy must never enter fallback |

`composition/interpolation_conformance.rs` also contains a capture helper, but
the module is `#[cfg(test)]`; it is not a production capture owner.

### Claudine CLI

| Site | Current anchor/evidence | Classification | Phase action |
|---|---|---|---|
| `commands/context/mod.rs:149` | ambient CWD | documentation command | allowlist unchanged |
| `commands/compose/prep.rs:551-552` | active document `SourceContext` | canonical prepared context | Phase 3 direct/inline shell preflight epoch |
| `commands/compose/prep.rs:1138-1139` | active document `SourceContext`; second construction | canonical prepared context | Phase 3 single direct/inline/loop epoch snapshot |
| `commands/wrap/composition/pipeline.rs:1309-1310` | active document `SourceContext` | canonical prepared context | Phase 3 lifecycle consumes request snapshot |
| `commands/wrap/composition/pipeline.rs:1316` | launch CWD ambient fallback | canonical prepared-context fallback | remove from invocation-backed path and account any miss |
| `commands/wrap/overlay.rs:58-59`, `materialize_passthrough_harness_seed` | passthrough source `SourceContext` | canonical prepared context | Phase 5 overlay/passthrough migration |
| `commands/wrap/harness_orch/prompt.rs:172-173`, `build_prepare_options` | harness document `SourceContext` | canonical prepared context | Phase 5 harness prompt migration |
| `commands/wrap/harness_orch/loop_control/target_launch.rs:645` | launch area when materialized target lost its context | compatibility fallback on a canonical harness seam | require retained epoch context; account fallback |
| `commands/wrap/sequence/mod.rs:92-93`, approval options | each prompt source `SourceContext` | canonical prepared context | Phase 4 approval projection from launch base |
| `commands/wrap/sequence/mod.rs:252-253`, root graph | sequence root `SourceContext` | canonical prepared context | Phase 4 graph launch base |
| `commands/wrap/sequence/task_run.rs:209-210`, `prepare_task_context` | task origin `SourceContext` | canonical prepared context | Phase 4 task epoch plus target overrides |
| `commands/wrap/sequence/jit.rs:273-274`, template preflight | JIT source `SourceContext` | canonical prepared context | Phase 4 JIT target-adjusted launch snapshot |
| `commands/wrap/sequence/jit.rs:280` | launch area or source parent when no invocation | library-style compatibility fallback | retain only for non-invocation tests/callers |

Every production `InvocationContext::runtime_evidence` call is paired with one
of the canonical rows above. There is no production runtime-evidence call for
live `current.ctx.*`; that namespace uses `capture_at_event` deliberately.

## Canonical migration list

The reviewed minimum route list is:

1. direct/inline shell preflight and main preparation;
2. loop seed, conditions, iteration preparation, and lifecycle execution;
3. proxy entry plus retry/resume re-materialization;
4. sequence root graph preflight and referenced task/group/prompt shell bytes;
5. sequence approval composition, JIT template preflight, and task execution;
6. lifecycle pipeline re-materialization;
7. overlay and passthrough harness materialization;
8. harness prompt preparation and target launch rebuild; and
9. file-backed, appendix, and built-in system-prompt preparation.

Source contexts remain required beside the new launch snapshot for `$schema`,
transclusion, eager `file(...)`, provenance, and repository-first/source-relative
reference resolution.

## Ambient-fallback accounting audit

The four existing `record_ambient_fallback` calls cover file-resolution proxy
commit fallbacks (`composition/pipeline.rs`, `loop_control.rs`, and
`loop_control/coordinator.rs`) and environment-context fallback
(`composition/runner.rs`). They do not cover prepared-context loss.

Consumer seams that can currently fall through unobserved are:

- `PrepareOptions.prepared_context: None` reaching
  `composition/prepare.rs::derive_compose_context`, even when
  `invocation_context` is populated;
- `document_expression_resolution_context` receiving no prepared context; its
  signature carries no invocation owner with which to record the miss;
- lifecycle `StackExecutionContext::early_binding_context` and proxy-with
  fallback capture when `prepared_context` is absent;
- lifecycle pipeline re-materialization when an invocation exists but the
  source/prepared pair is incomplete;
- harness target launch rebuild when `MaterializedHarnessPrompt.compose_context`
  is absent; and
- compatibility system-prompt, preflight, and JIT branches. These are valid for
  library-only callers, but any future invocation-backed entry into them needs
  an explicit counter.

Phase 2 must wire the counter at the narrowest invocation-aware consumer seams.
It must not label intentionally live `current.ctx.*` capture or unrelated
library-only compatibility use as a canonical failure.

## Adjacent comment review

- Preserve the launch-anchor intent in `commands/compose/prep.rs` above both
  captures and in `commands/wrap/composition/pipeline.rs` above lifecycle
  capture. Those comments state the established contract; the code is wrong.
- Correct the direct-preparation sentence that says the main snapshot is
  "constructed exactly as" preflight. Phase 3 must say the exact same snapshot
  is passed to both, not merely reconstructed alike.
- Correct `composition/sequence/preflight/mod.rs` documentation that says
  early-bound `ctx.*` follows each authoring repository. File resolution does;
  launch-facing plain `ctx.*` must not.
- Preserve `composition/prepare.rs::derive_compose_context` documentation as a
  compatibility contract, while ensuring canonical command paths no longer use
  its fallback.
- Preserve comments distinguishing live `current.*` and source-relative file
  resolution from prepared plain `ctx.*`.
