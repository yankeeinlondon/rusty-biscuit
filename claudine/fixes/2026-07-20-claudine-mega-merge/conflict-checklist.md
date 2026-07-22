# Conflict and Semantic Hotspot Checklist

Preview revisions are frozen in [`sha-ledger.md`](./sha-ledger.md). `open` means
the row is inventoried but intentionally unresolved until its owning merge
phase. Owners are architectural responsibilities, not branch preferences.

| Path or surface | Preview/source | Cluster | Owner | Status | Rationale | Evidence |
|---|---|---|---|---|---|---|
| `.claudine/memory/commits.md` | seed↔foundation; foundation↔proxy | Architecture & records | Repository history | open | — | — |
| `CLAUDE.md` | all three previews | Architecture & records | Repository guidance | open | Must also remove the two baseline markers | `marker-baseline.txt` |
| `prompts/_implement/implement-suggestions.md` | seed↔foundation | Architecture & records | Shared implementation prompts | open | — | — |
| `.claude/skills/claudine/SKILL.md` | foundation↔proxy | Architecture & records | Final Claudine architecture | open | — | — |
| `.claude/skills/claudine/architecture.md` | foundation↔proxy | Architecture & records | Final Claudine architecture | open | — | — |
| `claudine/cli/src/commands/compose/prep.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/composition/mod.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/composition/pipeline.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/composition/runner.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/wrapper_stages.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` | foundation↔proxy | Harness & proxy routing | Command coordinator | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs` | foundation↔proxy | Harness & proxy routing | Command coordinator | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs` | foundation↔proxy | Harness & proxy routing | Command coordinator | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs` | foundation↔proxy | Harness & proxy routing | Combined transition tests | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs` | foundation↔proxy | Harness & proxy routing | Combined transition tests | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs` | foundation↔proxy | Harness & proxy routing | Combined transition tests | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/prompt.rs` | foundation↔proxy | Harness & proxy routing | Complete launch bundle | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/types.rs` | foundation↔proxy | Harness & proxy routing | Complete launch bundle | open | — | — |
| `claudine/cli/src/commands/wrap/overlay.rs` | foundation↔proxy | Harness & proxy routing | Immutable handoff overlay | open | — | — |
| `claudine/cli/src/commands/wrap/sequence/iterate.rs` | foundation↔proxy | Sequence integration | Sequence invocation | open | — | — |
| `claudine/cli/src/commands/wrap/sequence/mod.rs` | foundation↔proxy | Sequence integration | Sequence invocation | open | — | — |
| `claudine/cli/src/commands/wrap/sequence/phase1c.rs` | foundation↔proxy | Sequence integration | Sequence invocation | open | — | — |
| `claudine/docs/providers/dispatch-inventory.json` | foundation↔proxy | Generated/docs | Generator | open | Regenerate after behavior settles | — |
| `claudine/docs/topics/composition.md` | foundation↔proxy | Generated/docs | Final merged behavior | open | Reconcile after code | — |
| `claudine/gen/tests/drift.rs` | foundation↔proxy | Generated/docs | Generator drift guard | open | Test source, not generated output | — |
| `claudine/lib/src/composition/error/mod.rs` | foundation↔proxy | Library lifecycle | Diagnostic registry | open | — | — |
| `claudine/lib/src/composition/error/render/mod.rs` | foundation↔proxy | Library lifecycle | Diagnostic registry | open | — | — |
| `claudine/lib/src/composition/error/tests.rs` | foundation↔proxy | Library lifecycle | Diagnostic registry | open | — | — |
| `claudine/lib/src/composition/lifecycle/context.rs` | foundation↔proxy | Library lifecycle | Lifecycle protocol | open | — | — |
| `claudine/lib/src/composition/lifecycle/executor.rs` | foundation↔proxy | Library lifecycle | Lifecycle protocol | open | — | — |
| `claudine/lib/src/composition/lifecycle/executor/tests/mod.rs` | foundation↔proxy | Library lifecycle | Lifecycle protocol tests | open | — | — |
| `claudine/lib/src/composition/looping/engine.rs` | foundation↔proxy | Library lifecycle | Active-document loop | open | — | — |
| `claudine/lib/src/composition/mod.rs` | foundation↔proxy | Library lifecycle | Canonical preparation API | open | — | — |
| `claudine/lib/src/composition/preflight.rs` | foundation↔proxy | Library lifecycle | Canonical preflight | open | — | — |
| `claudine/lib/src/composition/prepare.rs` | foundation↔proxy | Library lifecycle | Canonical preparation service | open | — | — |
| `claudine/lib/src/composition/types.rs` | foundation↔proxy | Library lifecycle | Shared typed contracts | open | — | — |
| `darkmatter/lib/src/markdown/compose/context/options.rs` | foundation↔proxy | Darkmatter | Explicit context/schema stage | open | — | — |

## Auto-merged paths requiring semantic review

| Path or surface | Preview/source | Cluster | Owner | Status | Rationale | Evidence |
|---|---|---|---|---|---|---|
| `claudine/cli/src/commands/wrap/composition/pipeline.rs` | seed previews | CLI prep & composition | Canonical preparation service | open | Clean merge is not semantic proof | — |
| `claudine/cli/src/commands/wrap/wrapper_stages.rs` | seed previews | CLI prep & composition | Canonical preparation service | open | Clean merge is not semantic proof | — |
| `claudine/docs/topics/composition.md` | seed previews | Generated/docs | Final merged behavior | open | — | — |
| `claudine/docs/topics/system-prompt.md` | seed↔foundation | Generated/docs | Invocation-fixed prompt content | open | — | — |
| `.config/nextest.toml` | foundation↔proxy | Harness/testing | Test-tier contract | open | — | — |
| `claudine/cli/Cargo.toml` | foundation↔proxy | Harness/testing | CLI dependency boundary | open | — | — |
| `claudine/cli/src/commands/compose/mod.rs` | foundation↔proxy | CLI prep & composition | Canonical preparation service | open | — | — |
| `claudine/cli/src/commands/wrap/composition/target.rs` | foundation↔proxy | CLI prep & composition | Target authoring context | open | — | — |
| `claudine/cli/src/commands/wrap/composition/tests.rs` | foundation↔proxy | CLI prep & composition | Route-equivalence tests | open | — | — |
| `claudine/cli/src/commands/wrap/env/mod.rs` | foundation↔proxy | Harness & proxy routing | Captured ambient/credential policy | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/attempt.rs` | foundation↔proxy | Harness & proxy routing | Attempt classification | open | — | — |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/terminal_evaluation.rs` | foundation↔proxy | Harness & proxy routing | Terminal transition tests | open | — | — |
| `claudine/cli/src/commands/wrap/mod.rs` | foundation↔proxy | Harness & proxy routing | Wrapper entry point | open | — | — |
| `claudine/cli/tests/level2_lifecycle_control.rs` | foundation↔proxy | Harness/testing | L2 lifecycle seams | open | — | — |
| `claudine/cli/tests/level2_lifecycle_dispatch.rs` | foundation↔proxy | Harness/testing | L2 lifecycle seams | open | — | — |
| `claudine/docs/topics/lifecycle.md` | foundation↔proxy | Generated/docs | Final merged lifecycle | open | — | — |
| `claudine/lib/src/composition/error/render/lifecycle.rs` | foundation↔proxy | Library lifecycle | Effective diagnostic projection | open | — | — |
| `claudine/lib/src/composition/interpolation_conformance.rs` | foundation↔proxy | Library lifecycle | Interpolation contract | open | — | — |
| `claudine/lib/src/composition/lifecycle/action_shape.rs` | foundation↔proxy | Library lifecycle | Proxy action grammar | open | — | — |
| `claudine/lib/src/composition/lifecycle/control.rs` | foundation↔proxy | Library lifecycle | Typed transitions | open | — | — |
| `claudine/lib/src/composition/lifecycle/control/tests.rs` | foundation↔proxy | Library lifecycle | Typed transition tests | open | — | — |
| `claudine/lib/src/composition/lifecycle/mod.rs` | foundation↔proxy | Library lifecycle | Lifecycle protocol | open | — | — |
| `claudine/lib/src/composition/lifecycle/parse.rs` | foundation↔proxy | Library lifecycle | Lifecycle grammar | open | — | — |
| `claudine/lib/src/composition/lifecycle/tests/action_shape_control.rs` | foundation↔proxy | Library lifecycle | Proxy action grammar tests | open | — | — |
| `claudine/lib/src/composition/looping/engine/tests/lifecycle_control.rs` | foundation↔proxy | Library lifecycle | Loop transition tests | open | — | — |
| `claudine/lib/src/composition/preflight/tests.rs` | foundation↔proxy | Library lifecycle | Exact-command audit tests | open | — | — |
| `darkmatter/lib/src/markdown/compose/schema_validation.rs` | foundation↔proxy | Darkmatter | Deferred schema verdict | open | — | — |

## One-sided responsibility hotspots required by the specification

| Path or surface | Preview/source | Cluster | Owner | Status | Rationale | Evidence |
|---|---|---|---|---|---|---|
| `claudine/cli/src/commands/wrap/**/launch*` | spec one-sided review | Harness & proxy routing | Complete launch bundle | open | Resolve exact files after merge | — |
| System-prompt lifetime and session-reporting modules | spec one-sided review | Harness & proxy routing | Invocation/session state | open | — | — |
| Darkmatter transclusion/reference graph/expression/schema-resolution modules | spec one-sided review | Darkmatter | Darkmatter composition | open | — | — |
| New `composition::coordinator` modules | spec one-sided review | Library lifecycle | Command coordinator | open | — | — |
| New preparation-stage modules | spec one-sided review | Library lifecycle | Canonical preparation service | open | — | — |
| New diagnostic-registry/restored-diagnostic modules | spec one-sided review | Library lifecycle | Diagnostic registry | open | — | — |
| Sequence Plus task/group modules | spec one-sided review | Sequence integration | Sequence invocation | open | — | — |
| Task-stream modules | spec one-sided review | Terminal rendering | `TerminalRenderable` task framing | open | — | — |
| Launch-plan modules | spec one-sided review | Harness & proxy routing | Complete launch bundle | open | — | — |
| Completion adapters and private `@`/relative rewrite candidates | spec one-sided review | File resolution | `biscuit-file::FileReference` | open | Source scan required | — |
| Biscuit test-harness backends | spec one-sided review | Harness/testing | Cross-platform harness | open | — | — |
| `.github/workflows/*` Claudine/test filters | spec one-sided review | Harness/testing | Fail-closed tier behavior | open | — | — |
| Rendezvous Windows `Connected` adapter and dependency gating | spec one-sided review | Rendezvous | Cross-platform local IPC | open | — | — |
