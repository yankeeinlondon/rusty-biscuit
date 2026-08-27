---
total_phases: 8
created: 2026-08-27
phase: 8
agent: openai/codex
yolo: "true"
source_files_during_phase_1:
    - darkmatter/cli/tests/common/mod.rs
docs_updated_during_phase_1:
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_1:
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
    - biscuit-file/lib/tests/precedence_flip.rs
    - biscuit-file/lib/tests/reference_grammar.rs
    - biscuit-file/lib/tests/resolution_context.rs
    - biscuit-file/cli/tests/cli_tests.rs
docs_updated_during_phase_2:
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/completion_round_trip.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
    - biscuit-file/lib/tests/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
    - biscuit-file/lib/tests/precedence_flip.rs
    - biscuit-file/lib/tests/repository_scope_catalog.rs
    - biscuit-file/lib/tests/resolution_context.rs
    - biscuit-file/cli/tests/cli_tests.rs
docs_updated_during_phase_3:
    - biscuit-file/cli/README.md
    - biscuit-file/docs/tech-spec/file-reference-struct.md
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/lib/README.md
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/architecture.md
    - .claude/skills/biscuit-file/references/cli.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/claudine/composition.md
source_files_during_phase_4:
    - biscuit-file/lib/src/file_reference/context.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/clean_schema.rs
    - darkmatter/cli/tests/compose_transclusion.rs
    - darkmatter/cli/tests/schema_triggers.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/repository_scope.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/reference.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/link_interpolation_integration.rs
    - darkmatter/lib/tests/reference_integration.rs
docs_updated_during_phase_4:
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/composition.md
source_files_during_phase_5:
    - darkmatter/cli/tests/compose_transclusion.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/compose/context/capture/invocation.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/base_schema_end_to_end.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/completion/composition/magic_at.rs
    - claudine/cli/src/completion/composition/mod.rs
    - claudine/cli/tests/completion_compose.rs
    - claudine/cli/tests/compose_cli.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/lib/src/composition/coordinator/commit.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/expression/tests/resolution_context.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/harness/audit.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/resolve/tests.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/system_prompt/prepare.rs
    - claudine/lib/src/system_prompt/resolve.rs
    - claudine/lib/src/system_prompt/resolve/tests.rs
docs_updated_during_phase_5:
    - claudine/features/2026-08-26-finalized-references/plan.md
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/docs/topics/context-variables.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/composition.md
source_files_during_phase_6:
    - Cargo.lock
    - claudine/cli/src/commands/providers.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/completion/scopes.rs
    - claudine/cli/src/completion/scopes/tests.rs
    - claudine/cli/tests/ctx_launch_anchor.rs
    - claudine/cli/tests/inline_compose_cli.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/gen/Cargo.toml
    - claudine/gen/src/inputs.rs
    - claudine/gen/tests/pipeline.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/resolve/tests.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/resolve/tests.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/invocation_context/tests.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
docs_updated_during_phase_6:
    - claudine/docs/dependencies.md
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
    - claudine/cli/src/commands/config_tui/mod.rs
    - claudine/cli/src/commands/init_wizard.rs
    - claudine/cli/src/commands/providers.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/exec/spawn/setup.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/mod.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/commands/wrap/exec/wiring/tests.rs
    - claudine/cli/src/main.rs
    - claudine/cli/tests/agent_cwd.rs
    - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
    - claudine/cli/tests/spawn_inventory.rs
    - claudine/lib/src/child_environment.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/dispatch/runner/bash.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/model_catalog/provider_sources.rs
docs_updated_during_phase_7:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/topics/execution-flow.md
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_7:
    - claudine/docs/providers/spawn-seam-inventory.json
skills_files_updated_during_phase_7:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
source_files_during_phase_8:
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/system_prompt/prepare.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
docs_updated_during_phase_8:
    - biscuit-file/docs/topics/file-references.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
    - claudine/features/2026-08-26-finalized-references/plan.md
docs_created_during_phase_8:
    - claudine/features/2026-08-26-finalized-references/acceptance.md
skills_files_updated_during_phase_8:
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/claudine/cli-reference.md
    - .claude/skills/darkmatter/compose.md
source_code:
    - Cargo.lock
    - biscuit-file/cli/tests/cli_tests.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/parse.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/completion_round_trip.rs
    - biscuit-file/lib/tests/detailed_resolution.rs
    - biscuit-file/lib/tests/finalized_reference_resolution.rs
    - biscuit-file/lib/tests/implicit_relative.rs
    - biscuit-file/lib/tests/precedence_flip.rs
    - biscuit-file/lib/tests/reference_grammar.rs
    - biscuit-file/lib/tests/repository_scope_catalog.rs
    - biscuit-file/lib/tests/resolution_context.rs
    - claudine/cli/src/commands/config_tui/mod.rs
    - claudine/cli/src/commands/init_wizard.rs
    - claudine/cli/src/commands/providers.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/wrap/env/mod.rs
    - claudine/cli/src/commands/wrap/env/tests.rs
    - claudine/cli/src/commands/wrap/exec/spawn/setup.rs
    - claudine/cli/src/commands/wrap/exec/spawn/tests/mod.rs
    - claudine/cli/src/commands/wrap/exec/wiring/session.rs
    - claudine/cli/src/commands/wrap/exec/wiring/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/retry_resume.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/completion/composition/magic_at.rs
    - claudine/cli/src/completion/composition/mod.rs
    - claudine/cli/src/completion/scopes.rs
    - claudine/cli/src/completion/scopes/tests.rs
    - claudine/cli/src/main.rs
    - claudine/cli/tests/agent_cwd.rs
    - claudine/cli/tests/completion_compose.rs
    - claudine/cli/tests/compose_cli.rs
    - claudine/cli/tests/ctx_launch_anchor.rs
    - claudine/cli/tests/inline_compose_cli.rs
    - claudine/cli/tests/level2_file_resolution_capture.rs
    - claudine/cli/tests/snapshots/wrap_basics__wrapper_reports_removed_sensitive_env_names.snap
    - claudine/cli/tests/spawn_inventory.rs
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/gen/Cargo.toml
    - claudine/gen/src/inputs.rs
    - claudine/gen/tests/pipeline.rs
    - claudine/lib/src/child_environment.rs
    - claudine/lib/src/composition/coordinator/commit.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/control.rs
    - claudine/lib/src/composition/lifecycle/control/tests.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/filesystem_lookup.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/looping/expression.rs
    - claudine/lib/src/composition/looping/expression/tests/resolution_context.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/resolve.rs
    - claudine/lib/src/composition/resolve/tests.rs
    - claudine/lib/src/composition/sequence/expr.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/dispatch/runner/bash.rs
    - claudine/lib/src/harness/audit.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - claudine/lib/src/harness/resolve.rs
    - claudine/lib/src/harness/resolve/tests.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/invocation_context.rs
    - claudine/lib/src/invocation_context/tests.rs
    - claudine/lib/src/lib.rs
    - claudine/lib/src/model_catalog/provider_sources.rs
    - claudine/lib/src/system_prompt/prepare.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - claudine/lib/src/system_prompt/resolve.rs
    - claudine/lib/src/system_prompt/resolve/tests.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/clean_schema.rs
    - darkmatter/cli/tests/common/mod.rs
    - darkmatter/cli/tests/compose_transclusion.rs
    - darkmatter/cli/tests/schema_triggers.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/compose/context/capture/invocation.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/capture/snapshot.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/repository_scope.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/reference.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/base_schema_end_to_end.rs
    - darkmatter/lib/tests/link_interpolation_integration.rs
    - darkmatter/lib/tests/reference_integration.rs
documentation:
    - biscuit-file/cli/README.md
    - biscuit-file/docs/tech-spec/file-reference-struct.md
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/lib/README.md
    - claudine/docs/dependencies.md
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/docs/providers/spawn-seam-inventory.json
    - claudine/docs/topics/execution-flow.md
    - claudine/features/2026-08-26-finalized-references/acceptance.md
    - claudine/features/2026-08-26-finalized-references/consumer-audit.md
    - claudine/features/2026-08-26-finalized-references/plan.md
    - darkmatter/docs/schemas/darkmatter.yaml
    - darkmatter/docs/topics/context-variables.md
packages:
    - biscuit-file
    - darkmatter
    - claudine
    - claudine-cli
---

# Execution Plan: Finalized file-reference grammar — one sigil catalog, one CWD model

Reference: [`spec.md`](spec.md) · Delta chronicle: [`sigil-delta.md`](sigil-delta.md) ·
Design-intent (normative): `claudine/docs/topics/file-referencing.md`

## Goal

Land the finalized file-reference grammar across `biscuit-file`, `darkmatter`,
and `claudine`: add `&` and `^`, remove `!`, flip implicit-relative ordering to
composition-CWD first, re-anchor multi-homed sigil bases to the reference's own
scope through a single `RepositoryScopeCatalog` projection, materialize
caller-passed file parameters as anchored values, add `ctx.cwd`, and set
`AGENT_CWD` in every spawned child's environment — with repo containment
enforced for `&`/`^` on both lexical and resolved targets.

Dependency direction is fixed: `biscuit-file` ← `darkmatter` ← `claudine`.
Phases follow that order. No backward-compatibility shims; call sites and
fixtures are updated directly.

## Dependency and parallelism map

```
Phase 1 (baseline + audit)
  └─> Phase 2 (biscuit-file grammar) ─> Phase 3 (biscuit-file catalog/resolve)
                                           └─> Phase 4 (darkmatter projection + resolver retirement)
                                                  └─> Phase 5 (darkmatter ctx.cwd + materialization)
                                                          └─> Phase 6 (claudine source-scope + ctx.cwd)
Phase 7 (claudine AGENT_CWD + spawn-seam guard)  -- implementation may start after Phase 1;
                                                      final inventory regenerates after Phase 6
Phase 8 (docs, audit proofs, L2, cross-platform) -- after Phases 2-7
```

- **Phase 7 is mostly parallelizable** with Phases 2–6 after Phase 1 records
  the complete spawn census. Its shared CLI/invocation wiring must be
  coordinated with Phase 6, and its committed inventory must be regenerated
  after all production-source changes land.
- Phases 2–6 are one coordinated public-contract migration. Public enum/error
  additions can make downstream exhaustive matches fail between phase
  checkpoints; do not commit or hand off a knowingly broken workspace at one
  of those intermediate boundaries. Within Phase 4, the projection and the
  resolver-site replacements are separable but sequenced (sites consume the
  projection).
- Phases 2–3 are biscuit-file-only; Phase 4–5 are darkmatter-only. One engineer
  can hold Phase 7's helper/scanner work while another holds Phases 2–5; Phase
  6/7 CLI and invocation-context wiring needs explicit coordination.

## Ground rules (from spec + AGENTS.md)

- Explicit resolution stays snapshot-only: no ambient CWD, HOME, Git, Cargo
  metadata, or topology discovery after context capture.
- One owner per responsibility per the spec's "Ownership and reuse boundaries"
  table; a lower layer never grows a dependency to satisfy a higher one.
- US English; comments follow the repo's comment-quality rules (no HOW-narration,
  fix drifted comments in the same change as behavior changes).
- Tests run via `just test` / `just test-l2` / `just lint` per package area
  (nextest underneath). L2/L3 must not steal terminal focus.
- After a phase changes a public biscuit-file or Darkmatter contract, restore
  the complete workspace consumer set through the dependent phase before any
  commit or review handoff; a package-local green check is not evidence that
  downstream exhaustive consumers compile.
- Never `cargo fmt` unless told; never commit unless told.

---

## Phase 1 — Baseline, dependency gate, and consumer audit

The spec's `depends-on` (fixes/2026-08-12-ctx-launch-anchor) requires that
fix's contract and acceptance criteria complete before implementation starts,
and the baseline must be verified green rather than assumed.

- [x] Confirm `claudine/fixes/2026-08-12-ctx-launch-anchor` is complete: its spec ACs are satisfied and it has been moved to `fixes/_completed/` (or record the owner's explicit go-ahead in this feature's notes); note from its `log.md` that the Linux/WSL/native-Windows gates were partially deferred — decide with the owner whether those gates block this feature's Phase 8 or run concurrently.
- [x] Record the baseline commit (`git rev-parse HEAD`) in this feature directory and verify green: `just test` and `just lint` in `biscuit-file/`, `darkmatter/`, and `claudine/`; the Claudine area recipe already includes its library, CLI, contract, catalog-types, and generator L1 suites.
- [x] Run GitNexus impact analysis (`impact` upstream) for the symbols the spec names: `FileReferenceKind`, `RootProvenance`, `FileReferenceError`, `candidate_plan`, `resolve_in_context`, `complete_partial`, `document_resolution_context`, `find_package_area_from`, `find_git_root_from`, `derive_source`, `prompt_magic_roots`, `build_child_env`. Save the blast-radius summary to `claudine/features/2026-08-26-finalized-references/consumer-audit.md`; where an inline/free function is not indexed (currently `build_child_env` may not resolve by name), record the source inventory rather than treating a not-found graph result as zero impact.
- [x] Complete the compiler-exhaustiveness consumer inventory in `consumer-audit.md`: every match/constructor of `FileReferenceKind`, internal `ReferenceKind`, `RootProvenance`, `FileReferenceError`, candidate-plan, completion-root, and `resolve_in_context` consumers across the workspace. The spec's known blast radius includes: Darkmatter library/CLI/DMLS schema resolution/rewrite/detect/format, transclusion, expression helpers (`path_projection`, `functions/mod.rs` git-root fns), preflight, TOC linking (`link_resolve`, `link_normalization`), Claudine library/CLI/generator sequence source resolution, harness diagnostics/resolution, system prompts, and CLI composition completion. Include every external consumer of Darkmatter's public `find_git_root_from`, notably `darkmatter/cli`, `darkmatter/dmls`, and `claudine/gen`; verify none are missed.
- [x] Build a separate production process-spawn census for Phase 7 across `claudine/lib/src` and `claudine/cli/src`. Classify std/Tokio process commands, helper-returned commands, inline `status`/`output` calls, and platform-gated production modules; exclude `#[cfg(test)]` bodies, test-only files, and `clap::Command`. Record whether each site already receives a launch snapshot and how the shared helper will reach it. The `claudine-contract` and rendezvous crates are outside the spec's lib/CLI guard scope and must not be silently folded into or mistaken for that inventory.
- [x] Locate and list in `consumer-audit.md` the existing conflict fixtures that lock repository-first implicit ordering (cited by ctx-launch-anchor AC10 — darkmatter `resolve_ctx.rs`, `schema_validation.rs`, `options.rs` area, and any claudine fixtures) so Phase 3/6 flip them deliberately.

**Checkpoint:** `consumer-audit.md` exists with the symbol inventory and fixture list; all three areas green at the recorded baseline commit. Exit criteria: dependency complete or explicitly waived; audit reviewed against the spec's Scope section.

---

## Phase 2 — biscuit-file: grammar layer (parse and parse errors)

Implements D1, D2, D9, D10 (grammar parts) in
`biscuit-file/lib/src/file_reference/{parse.rs, error.rs, mod.rs}`. This phase
changes parsing but deliberately leaves the now-unreachable legacy Package
enum/provenance/error arms for Phase 3, where the resolver code that consumes
them is removed in the same package-local change. New public variants can still
break downstream exhaustive matches; those are restored through Phases 4–6
before any commit or review handoff.

- [x] Add `FileReferenceKind::RepositoryRoot` (`&`) and `FileReferenceKind::RepositoryScoped` (`^`) plus the internal `ReferenceKind` variants and `DetectedKind` arms in `parse.rs::detect_kind`, inserted at the documented D9 position (after `@`, before `~`).
- [x] Implement defensive-sigil handling for `@`, `&`, `^` exactly once (generalize the existing `@` logic): consume at most one following `/`; reject a payload that remains rooted (`is_rooted_magic_payload`-style check) with `InvalidSyntax`; reject `&\x`-style backslash separators on every host (`/` is the only portable sigil separator); reject empty payloads as `InvalidSyntax`.
- [x] Stop parsing the `!`/Package kind: delete `DetectedKind::Package` and make a leading `!` return `FileReferenceError::InvalidSyntax` with dedicated text that names the removed sigil and suggests `^`; it must not fall through to implicit-relative. Retire the unreachable internal/public Package and provenance variants with their resolver arms in Phase 3 so this phase remains package-local compilable.
- [x] Implement D9 reserved-introducer/scheme rejection in `detect_kind` with the documented host-independent order: HTTP(S) → `vault::`/`vault:` → `@`/`&`/`^`/`!`/`~` → explicit Windows device-prefix rejection → supported native absolute classification (POSIX, bare drive root, drive-with-separator, UNC) → generic RFC-scheme guard (`[A-Za-z][A-Za-z0-9+.-]*:` that is not a supported scheme → typed `UnsupportedScheme` error, `file:` included) → explicit-relative → implicit. Drive-relative `C:path` is a typed error, never implicit; bare `C:`, `C:/x`, and `C:\x` retain the design-intent document's supported behavior.
- [x] Make a second leading `%` invalid (`%%x` → `InvalidSyntax`) rather than a recursive filename — enforce in `strip_recursive`/`parse`.
- [x] Update `resolve.rs::injected_sigil` (grammar set): interpolation must not inject `&`, `^`, or the removed `!` either; keep the existing rejection message shape.
- [x] Add the parse-layer `UnsupportedScheme` error plus the typed outside-repository and repository-escape variants Phase 3 will consume. Outside-repository names the sigil and reference CWD; repository-escape identifies the sigil, authored reference, repository root, and escaped candidate without leaking an unrelated ambient path. Re-map `classify_error` when the resolution variants become reachable in Phase 3; do not add a `RemovedSigil` variant because D2 requires the dedicated `!` diagnostic to remain `InvalidSyntax`.
- [x] Update the module-level docs in `mod.rs` (the `!README.md` example in the `//!` header must go; show `&`/`^` instead).
- [x] L1 parse tests (host-independent, run everywhere): empty/rooted/backslash payloads after `@`/`&`/`^`; `&/x` ≡ `&x` equivalence; `~user`; `%%`; removed `!` `InvalidSyntax` diagnostic text; drive-relative `C:path` rejection; `file:`, `file:///`, `\\?\`, `\\.\`, and misspelled schemes as typed errors; bare `C:`, `C:/abs`, and `C:\abs` preserved; reserved-introducer filenames reachable via `./` (for example `./!weird-name.md`) and POSIX colon-bearing filenames via `./name:part`; interpolation-sigil-injection rejections for `&`/`^`.

**Checkpoint:** `just test` and `just lint` pass in `biscuit-file/`. Downstream crates are expected to fail compilation — capture the full compiler error list into `consumer-audit.md` as the Phase 4–6 work list. Verify no parse test reads the filesystem.

---

## Phase 3 — biscuit-file: scope catalog, resolution, containment, completion

Implements D3–D7 core + AC1, AC2 (engine level), AC8, AC11 in
`biscuit-file/lib/src/file_reference/{context.rs, resolve.rs, mod.rs}`.

- [x] Retire the unreachable Package surface together with its resolver consumers: delete `ReferenceKind::Package`, `FileReferenceKind::Package`, their match arms, `FileReference::with_package_area_magic_path`, `RootProvenance::Package`, `MissingPackageContext`, and the `Workspace(cargo_metadata::Error)` variant; add distinct `RootProvenance::PackageRoot` and `RootProvenance::PackageArea` variants and finish the Phase 2 error-classification mapping. Update all biscuit-file tests and detailed-resolution fixtures in the same step so the package remains green.
- [x] Add the pure-data `RepositoryScopeCatalog` type to `context.rs` (exported from `mod.rs`): repository root, monorepo package-area fallback policy, package-area roots, package roots. Validated constructor accepts only absolute, lexically normalized roots; rejects a package/package-area root outside its repository; dedupes roots without filesystem I/O; no `sniff` dependency. Component-aware, most-specific-first scope selection (`scope_for(base) -> {package_root?, package_area_root?, repository_root?}`).
- [x] Extend `FileResolutionContext`: add the distinct package-root anchor (alongside the existing `package_area`), a `with_repository_scope_catalog` builder carrying the caller-supplied catalog, and recomputation in `for_source`/`for_base`/`for_trusted_external_*` — source-specific anchors are re-derived from the catalog by component-aware containment, never blindly copied from the previous document. A trusted-external derivation not covered by the catalog clears repository/package/package-area anchors. `validate()` semantics updated so normal and trusted-external derivations cannot retain stale repository scopes.
- [x] Delete `find_package_area` from `context.rs`, remove `cargo_metadata` from the `file-reference` feature and dependency table in `biscuit-file/lib/Cargo.toml`, refresh the workspace lockfile if no other package retains it, and drop the re-export from `mod.rs`. Delete or rewrite the `Workspace`/malformed-`Cargo.toml` detailed-resolution tests and comments that existed solely for this shell-out. `find_git_root` and `ResolutionContext::from_ambient` stay (ambient convenience + `bf` CLI only).
- [x] Flip `implicit_relative_roots` in `resolve.rs` to composition-CWD first, then repo root; collapse when equal; keep it the single authority for direct + recursive + anchoring paths. Update the doc comment that currently cites the repository-first "Phase 4 precedence", and update the in-file tests (`caller_supplied_repository_root_is_tried_before_base` becomes CWD-first).
- [x] Implement `&` resolution in `collect_roots`/candidate building: single candidate `{repo-root}/payload`, no package/area consultation, no fallback; typed outside-repository error when the reference's CWD is not inside a repository; lexical containment on the normalized payload (`&../outside.md` → typed escape error).
- [x] Implement `^` resolution: candidate order package root → package-area root → repo root (missing levels skipped, duplicates collapsed), selected from the reference's own base via the catalog; never consults home; same typed outside-repository error as `&`; every candidate passes containment; an escape is a typed error that stops resolution (does not advance to the next scope).
- [x] Implement the `@` intrinsic scope chain (D6): registered prepends → package root → package-area root → repo root → home → registered appends. Package/area/repo/home come from the intrinsic list exactly once (no caller should need to double-register them as convention roots — Phase 6 removes Claudine's double registration).
- [x] Implement one shared containment helper for `&`/`^` (lexical check via `normalize_components` + canonical check on an existing target or deepest existing ancestor via `canonicalize`/`dunce` handling for junctions/reparse points) and route direct, recursive (`%`) and completion through it. Expose one narrow biscuit-file API for Phase 5's lazy first-candidate materialization to invoke the same deepest-existing-ancestor check; do not make `candidate_plan()` itself probe the filesystem, because its contract remains an unprobed plan. Other kinds retain current symlink behavior.
- [x] Completion (AC8): add direct `&`/`^` entry forms to `classify_token`/`CompletionEntryForm`/`completion_roots` enumerating the same roots in execution order; `%` completion remains `Ok(None)` without reinterpreting the token; malformed rooted payloads keep typed parse errors; implicit completion order flips with the engine; magic completion mirrors the new intrinsic chain.
- [x] L1 resolution tests: for each D1 sigil, fixtures prove documented candidate order, first-match-wins, miss-as-`Ok(None)`, and typed errors; conflict fixtures where CWD and repo root both hold the file prove CWD wins (engine-level AC2); containment matrices — lexical `..` escapes, in-repo symlink still resolving, symlink-to-outside rejected, deepest-existing-ancestor behavior; catalog-constructor invariants — relative or non-normalized roots rejected, package/area roots outside the repository rejected, duplicates collapsed, and a work counter proving construction performs no I/O; catalog scope-selection matrices (root package, nested package, area-only, outside repo, second catalog); completion/execution parity tests per token form.
- [x] Windows-correctness pass on the new code paths: `Path`/`PathBuf` and portable-path helpers only (no manual separator replacement); junction/reparse-target containment tests written now, gated to run on native Windows (AC9 — execution happens in Phase 8).

**Checkpoint:** `just test` and `just lint` pass in `biscuit-file/`; `rg 'cargo_metadata' biscuit-file/lib biscuit-file/cli` and `rg 'find_package_area\b' biscuit-file/lib biscuit-file/cli` return no production or stale test/doc consumers. Update `consumer-audit.md` with the resolved downstream breakage list for Phases 4–6; do not commit until those downstream consumers compile again.

---

## Phase 4 — Darkmatter: single catalog projection and resolver retirement

Implements the ownership table's Darkmatter rows + D7 wiring + AC4 (adapter
half). Centered on `darkmatter/lib/src/markdown/compose/util.rs` and the
request boundary in `compose/context/`.

- [x] Implement the retained-observation → `RepositoryScopeCatalog` projection exactly once in Darkmatter, at the request boundary beside the existing repository capture group (`compose/context/capture/snapshot.rs` / `repo.rs` area): input is the retained `RepoInfo` plus the already-observed repository root expressed in a spelling lexically compatible with the reference base. Rebuild package and area roots from their repository-relative identities under that root spelling — never copy a foreign-spelled absolute `Package::path`. No filesystem observation inside the projection.
- [x] Preserve Sniff topology semantics in the projection: nested-package ownership, `RepoInfo::package_area_label_for_dir`'s first-component fallback for newly scaffolded directories, known-area vs root behavior. Export the projection (pub) so Claudine's `derive_source` calls it — this is the only sniff→catalog adapter in the workspace.
- [x] Make `ComposeOptions` (the Darkmatter request authority) carry the captured catalog alongside its `FileResolutionContext`. Supplied-evidence entry points receive the already-retained observation; ambient compatibility entry points perform Git/topology observation once at the top-level request boundary and project once, independently of whether the document requests the demand-driven `ctx.repo` group. Every nested resolver receives the same request snapshot through options/context derivation. The current `compose/context/options.rs` fallbacks at ~lines 1025/1081 become catalog reads, not alternate capture sites.
- [x] Delete `find_package_area_from` and `package_area_for_reference` from `compose/util.rs`; update the re-exports in `compose/mod.rs`.
- [x] Replace every per-reference `find_git_root_from` fallback that feeds a resolution candidate with reads from the request's catalog: `document_resolution_context` (`compose/util.rs`), `markdown/reference/mod.rs`, transclusion resolver (`compose/transclusion/resolver.rs:153`), `link_resolve.rs:166`, `link_normalization.rs:381`, `schema_validation.rs:54`, `schemas/resolve.rs:550`, `schemas/rewrite.rs:395`, `schemas/detect.rs:129`, `schemas/format.rs:389`, `expression/path_projection.rs:68`, and the `git_root`-style expression functions (`expression/functions/mod.rs:2214/2242`). `find_git_root_from` survives in the library only for the explicitly retained display helper (`abbreviate_path`) and the distinct `file_links/discovery.rs` boundary contract. `schemas/clean.rs` and `shell_expansion/store.rs` must receive a request boundary/catalog root or use a separately justified non-resolution owner; they may not retain an unreviewed ambient walk merely because they do not return a `ResolutionCandidate`.
- [x] Rework `document_resolution_context` to build `FileResolutionContext` from the catalog (repository root + package/area anchors per source base) so `for_source` recomputation (Phase 3) drives nested documents; drop the now-dead `package_area` parameter plumbing at call sites.
- [x] Update the reference graphing, TOC linking, preflight, expression functions, transclusion, and schema surfaces to consume the new grammar through `FileReference` (no prefix checks) — remove any surviving `starts_with('!')`-style dispatch found in the Phase 1 audit.
- [x] Update Darkmatter's other package-area consumers of the public helper: `darkmatter/cli/src/commands/schema/{triggers,validate}.rs` must capture/project once at their command boundary, and `darkmatter/dmls/src/overlay/schema.rs` must receive a passive supplied snapshot rather than discovering Git during completion/validation. Keep DMLS validation, completion, and hover free of filesystem observation beyond the explicit editor/request capture seam.
- [x] Seeded work-counter/inventory guards (AC4): explicit resolution performs no ambient CWD, HOME, Git, Cargo metadata, or topology discovery — extend the existing work-counter test seams to fail on ambient discovery during `resolve_in_context`/`candidate_plan` driven from a document context.
- [x] L1 tests: projection fixtures comparing the catalog's selected scopes with the retained `RepoInfo` across repository root, known area outside a package, newly scaffolded area, root and nested packages, a second repository, and symlink-equivalent root spellings (macOS `/var` vs `/private/var` exercised locally); a source-inventory guard test proving Darkmatter contains the only sniff-observation → catalog adapter.

**Checkpoint:** `just test` and `just lint` pass in `darkmatter/` (library, CLI, and DMLS); repository search shows no `find_package_area_from` / `package_area_for_reference` and no resolver-side `find_git_root_from` (only `abbreviate_path`, the explicit `file_links` boundary, and test fixtures recorded in `consumer-audit.md`). A standalone `md compose` of a monorepo document resolves `^`/implicit/`@` references against the document's own scopes, and supplied passive DMLS/schema paths do not recapture topology.

---

## Phase 5 — Darkmatter: `ctx.cwd` and caller parameter materialization

Implements D8.1–D8.6 + AC5 + AC6 (Darkmatter half) + AC12. Centered on
`compose/context/` (groups, catalog, capture), `darkmatter/docs/schemas/darkmatter.yaml`,
and the caller-file binding in `compose/schema_validation.rs`.

- [x] Add the no-I/O `ContextGroup::Invocation` to `compose/context/capture/groups.rs`, its `KEYS` table entry, and every exhaustive group registry/name/dependency/capture match; requesting it never triggers repository capture and works outside a repo. Add a catalog-parity assertion so a future context group cannot be added to one registry while omitted from another.
- [x] Add `ctx.cwd` end to end: typed descriptor in `compose/context/catalog.rs`, `darkmatter/docs/schemas/darkmatter.yaml`, context help text, and every single-sourced projection. Value is the captured absolute launch directory converted with biscuit-file's portable-path helpers (no ad hoc separator replacement). Ambient compatibility entry points capture the process CWD exactly once at the request boundary; downstream composition never calls `current_dir()` to populate it. Ambient capture failure projects `null` plus the existing partial-capture diagnostic.
- [x] Extend caller-file binding to both schema arms (D8.2–D8.5): a string is a file parameter only when the effective SimplifiedSchema selects `file` or `file(eager)` for that property; arrays and unions recurse into the selected file arm. Eager probes the complete ordered candidate list, requires an existing regular local file, and materializes the winning absolute native path. Lazy (non-recursive, local) builds the unprobed candidate plan and materializes the first candidate as a lexically normalized absolute path — no existence probing, absence is not an error; `&`/`^` invokes Phase 3's public biscuit-file lazy-containment seam against an existing target or deepest existing ancestor rather than duplicating containment in Darkmatter. Recursive lazy is a typed parameter-binding error suggesting `file(eager)`; lazy HTTP(S) stays a typed remote reference.
- [x] Implement origin-decided anchoring (D8.3): CLI key/value and `--set` file values use the immutable launch file-resolution context; document frontmatter and defaults use that document's `SourceContext`; `proxy.with` evaluates/materializes in the proxying source before handoff; sequence task parameters use the sequence document that authored them. Once materialized, an absolute value is never re-anchored by a proxy target, retry, resume, loop iteration, or sequence task.
- [x] Separate raw input from effective value (D8.6): the input layer retains the caller's raw override plus origin so a fresh epoch can reapply schema selection; every downstream expression, body, lifecycle, proxy, and launch-plan consumer sees the materialized effective value (frontmatter/lifecycle keeps native identity; Markdown presentation uses the existing portable sidecar). No downstream consumer reparses the raw relative string.
- [x] Lower-layer regression test (AC5): model the 2026-08-26 failure with distinct caller and target `FileResolutionContext`/compose-source inputs, materialize a relative caller `spec`, and prove `parent_dir(spec)`/`dirname(spec)`-derived sibling reads use the retained caller anchor after the target context changes. The full proxy/success-guard route belongs to Phase 6.
- [x] AC5 materializer unit matrix: synthetic caller/document/`proxy.with`/sequence origins; scalar/array/union arms; lazy first-plan selection (launch candidate wins even when absent); eager first-existing selection; lazy recursive rejection; lazy remote preservation; ordinary string overrides untouched; and repeated materialization remaining idempotent. Phase 6 owns the direct/proxy/retry/resume/loop/sequence route matrix.
- [x] Passive-contract tests (AC12): validation-only schema APIs remain non-mutating and do not call the lazy-containment/probing seam; only canonical successful composition materializes caller file values.
- [x] Seeded inventory/work guards: downstream composition never calls `current_dir()` for `ctx.cwd`; outside-repository documents still get `ctx.cwd` without requesting the repository group; a forced ambient CWD failure yields `null` + typed diagnostic.

**Checkpoint:** `just test` and `just lint` pass in `darkmatter/`; `md compose` of a fixture using `{{ ctx.cwd }}` and a lazy/eager `spec` parameter behaves per D8; the lower-layer two-context regression passes without requiring Claudine proxy machinery.

---

## Phase 6 — Claudine: source-scope integration, conventions, and `ctx.cwd` projections

Implements D7's Claudine consumers + AC4 (parity half) + AC6 (projection half)
+ AC7. Centered on `claudine/lib/src/invocation_context.rs`,
`claudine/lib/src/composition/resolve.rs`, and the prepared-context catalog.

- [x] Rework `InvocationContext::derive_source` (`invocation_context.rs:777`): stop computing `package_area_root`/`package_root` via the local `package_roots()` helper; call Darkmatter's projection (Phase 4) on the retained `RepoInfo` plus the source-compatible repository-root spelling, then build the `FileResolutionContext` from the resulting catalog. Delete the now-unused local `package_roots` helper. A source in another repository continues to get its context from the invocation's per-repository cache.
- [x] Provision anchors per D7 through the catalog-based `FileResolutionContext`: document-authored references resolve against the document's own scopes; caller-passed parameters use the immutable launch file-resolution context (`launch_file_resolution_context`). Source-relative convention roots are recomputed with the source's scope — not stored as launch-derived absolute prepend roots a nested document inherits. Request-stable roots (captured home, explicitly application-global prepend/append) remain stable.
- [x] Review convention registration against D6 (`composition/resolve.rs::prompt_magic_roots` and its callers at resolve.rs:92/177/442 and `invocation_context.rs:1425`): the package root, package-area root, repository root, and home directory are no longer also registered as convention roots — they come from `@`'s intrinsic list exactly once. Claudine's conventions (`<package>/prompts`, `<area>/prompts`, `<repo>/prompts`, `<repo>/.claudine/prompts`, `<repo>/docs`, peer-agent skills directories, `~/.claudine/prompts`) stay as registered prepend roots. Update `cli/src/completion/scopes.rs` and `cli/completion` surfaces that enumerate the same roots.
- [x] Update `composition/resolve.rs::with_prompt_magic_paths` (the ambient legacy path at resolve.rs:420) to the catalog projection, or retire it if the Phase 1 audit shows it redundant with the context-aware path — record the ruling. Ruling: retired; the only ambient enrichment caller now captures the catalog-backed context and resolves through `resolve_in_context`, matching canonical paths.
- [x] Add `ctx.cwd` to Claudine's prepared-context catalog inputs and its schema/single-sourcing projections (the same surfaces `ctx.area`/`ctx.current_package` flow through), including every exhaustive `ContextGroup` name/descriptor projection; launch-CWD capture failure remains the existing invocation error. Route fixtures cover direct, inline-compose, proxy, retry/resume, loop, sequence, overlay, harness, and system-prompt paths and prove one immutable value reaches preflight, body, effective frontmatter, and lifecycle.
- [x] Switch the last ambient biscuit-file call in Claudine — `cli/src/commands/providers.rs:410` `find_git_root(&cwd)` — to the invocation context.
- [x] Sweep the consumer-audit list: sequence source resolution (`composition/sequence/preflight`), harness diagnostics/resolution (`harness/`), system prompts (`system_prompt/prepare.rs`), overlay (`cli/src/commands/wrap/overlay.rs`), and CLI composition completion consume the correct source context and materialized parameter values; remove `!`-grammar handling everywhere (AC3 — no workspace code produces or consumes `Package`).
- [x] Implement the Claudine integration half of the AC5 matrix: prove the Darkmatter-owned raw/effective materialization survives direct, proxy, retry, resume, loop, sequence-task, harness, system-prompt, and overlay boundaries without any Claudine-side reparser. Include the canonical 2026-08-26 regression — repo-root shared prompt, launch from a package area, relative `spec`, and `parent_dir(spec)`/`dirname(spec)`-derived sibling read inside the proxied prompt's `success` guard. Phase 5 owns the materializer's unit matrix; this phase owns the real Claudine route matrix.
- [x] Replace `claudine/gen/src/inputs.rs`'s public `darkmatter::find_git_root_from` dependency with its own command-boundary repository observation (using Sniff, already the repository-observation authority for Claudine) or an explicitly supplied repository root. Update its pipeline fixtures; the generator must not keep a non-display consumer alive solely because Darkmatter still exports the display helper.
- [x] Reconcile ctx-launch-anchor AC10's conflict fixtures explicitly (AC2): update the fixtures that locked repository-first so the composition-CWD copy wins for document-authored references and the launch-directory copy wins for caller-passed parameters; document the supersession in the fixture comments (spec D3 re-rules review-3 Finding 4).
- [x] Parity test (AC4): a standalone `md compose` and a `claudine compose` of the same document produce identical `^` and implicit candidate plans and identical intrinsic `@` package/area/repository/home segments; Claudine's registered convention roots asserted separately at their D6 prepend/append positions so they cannot fail the parity test.
- [x] Collision fixtures (AC7): the skill example `@.claude/skills/name/SKILL.md` finds the repo's copy first and falls back to `~/.claude/skills/...`; prompt-lookup conventions keep working; lock the complete effective prepend → intrinsic → append order; prove intrinsic roots occur exactly once; prove a nested document gets its own source-relative convention roots.

**Checkpoint:** `just test` and `just lint` pass in `claudine/` (all area L1 suites, including `dispatch_inventory`); parity and canonical materialization-regression tests are green; no `FileReferenceKind::Package` / `RootProvenance::Package` / `!`-file-reference usage remains in production code. Phase 8 performs the prompt/doc/skill proof.

---

## Phase 7 — Claudine: `AGENT_CWD` and the spawn-seam inventory guard

Implements D8.7 + AC6 (`AGENT_CWD` half). Core helper and wiring work may start
after Phase 1's spawn census. Coordinate shared CLI/invocation files with Phase
6, and regenerate/freeze the inventory only after Phase 6's production-source
changes are complete.

- [x] Add one shared child-environment contribution helper in the `claudine` lib that sets `AGENT_CWD` to the captured absolute launch directory, overwriting any inherited value. This is the only place the variable is written for Claudine's children.
- [x] Define the capture rule: an ordinary top-level or nested Claudine invocation captures its own absolute entry CWD before any process-directory mutation and does not adopt inherited `AGENT_CWD`; the hidden provider-hook `handle` entry point (`cli/src/commands/handle.rs`) instead adopts the wrapper-supplied absolute `AGENT_CWD` as retained launch evidence, rejects a present non-absolute value, and uses the hook process's entry CWD only when the variable is absent. Any CLI route that can spawn a child captures this launch fact at entry even without building a composition `InvocationContext`.
- [x] Wire the helper into every production spawn seam from the Phase 1 census. Known sites include the provider launches (`cli/src/commands/wrap/exec/{spawn/setup,wiring/session}.rs` via `build_child_env`/`build_child_env_with_launch`), hook runners (`lib/src/dispatch/runner/bash.rs`), `::shell` execution (`lib/src/harness/shell.rs`, `lib/src/composition/sequence/task/shell.rs`, and `cli/src/commands/sequence.rs`), the lifecycle executor (`lib/src/composition/lifecycle/executor.rs`), dynamic model discovery (`lib/src/model_catalog/provider_sources.rs`), and CLI administrative/probe children (`commands/config_tui`, `init_wizard`, `providers`, and production wrapper helpers such as `overlay`). Do not list `lib/src/system_prompt/context.rs` or `lib/src/linking/paths.rs` as production seams: their current `Command` calls are inside `#[cfg(test)]` modules and the guard intentionally excludes them. Each production site either calls the helper directly or is recorded as an indirect governed path; an allowlist may describe indirection but may not exempt a child from `AGENT_CWD`.
- [x] Extend `debug_assert_child_env` (`cli/src/commands/wrap/exec/spawn/setup.rs`) — or add a sibling assertion — so the provider seam asserts `AGENT_CWD` presence and absoluteness.
- [x] Build the spawn-seam inventory guard as a new test in the style of `cli/tests/dispatch_inventory.rs`: scan production sources in `claudine/lib/src` and `claudine/cli/src`, classify every `std::process::Command` and `tokio::process::Command` construction (including imports/aliases, fully qualified paths, helper-returned builders, platform-gated modules, and inline `spawn`/`status`/`output` forms), and fail when a child can execute without the shared contribution helper. Exclude `#[cfg(test)]` bodies, test-only files, and clap's unrelated `Command` builder. Re-run the census after Phase 6, then commit the generated inventory artifact next to `dispatch-inventory.json` with its regenerate command.
- [x] Scanner unit fixtures prove the guard is non-vacuous: for each supported construction form, present one governed and one deliberately ungoverned seam and assert pass/fail — without mutating production source during the test.
- [x] Behavioral fixtures (AC6): `AGENT_CWD` is present in the provider, hook, `::shell`, lifecycle-executor, and sequence-shell child environments as the captured absolute launch directory; add a focused assertion for every additional indirect wiring class found by the census (for example administrative/probe commands) so the source inventory is not the only evidence. Prove it overwrites an inherited stale value; remains stable across retry/resume/loop/sequence re-entry; a wrapper → provider → `handle` → hook-action chain retains the wrapper's launch value despite the provider child CWD; an ordinary nested invocation with a stale inherited value publishes its own entry CWD; missing and non-absolute inherited values on `handle` cover the fallback/error rule.
- [x] Document the environment contract (D8.7 residual risk): `AGENT_CWD` is un-namespaced, Claudine overwrites it for its own children, and an unrelated tool reading it with different expectations is an accepted risk — note it wherever the child environment is documented (`claudine/docs/topics/execution-flow.md` environment section).

**Checkpoint:** `just test` and `just lint` pass in `claudine/`; the new inventory guard runs green and fails when temporarily pointed at an ungoverned fixture; all behavioral `AGENT_CWD` fixtures pass.

---

## Phase 8 — Documentation, audit proofs, L2, and the cross-platform matrix

Implements the documentation scope, AC9, AC10, AC13, and closes AC12.

- [x] Refresh `biscuit-file/docs/topics/file-references.md` to the finalized grammar: the D1 sigil table with candidate orders, `&`/`^` containment guarantees (lexical + resolved target, with the time-of-check/time-of-use limitation stated — biscuit-file is not a sandbox), the D9 reserved-introducer rules, the flipped implicit order, the `@` effective order under registered conventions, and the `&` shell-control-operator quoting note (`spec='&docs/plan.md'`).
- [x] Verify rather than repeat the AC13/OQ1 design-intent amendment: commit `aa38252c8` already changed `claudine/docs/topics/file-referencing.md` to distinguish forms authors may *encounter* (`file:` URIs, device prefixes, drive-relative paths) from forms the grammar accepts. Confirm the implemented diagnostics still match that ratified text; any additional design-intent wording change still requires Ken's approval.
- [x] Apply the drift rule to READMEs, topic docs, prompts, and every `.claude/skills/` entry that describes file referencing or composition context, including at minimum the biscuit-file `references/file-references.md`, Darkmatter `compose.md`, Claudine composition/CLI references, and shell-completion documentation where it states magic-root order. Update only consumers that actually describe the changed public behavior. For every edited Markdown file carrying a `hash:` frontmatter field, refresh it with Darkmatter's `md hash <file>` rather than hand-editing the hash. Update `biscuit-file/docs/dependencies.md` and any other affected per-area dependency references when `cargo_metadata` leaves biscuit-file; if Phase 6 adds Sniff directly to `claudine-gen`, record that edge in Claudine's dependency documentation too.
- [x] Run the repository-search proof set with `rg` and record results in `consumer-audit.md`: zero `!` file references in prompts (every `!` is expression negation); no `cargo_metadata` in biscuit-file production/tests/docs; no `find_package_area_from` / resolver-side `find_git_root_from` in Darkmatter library/CLI/DMLS; no external non-display consumer of Darkmatter's surviving display helper; no `FileReferenceKind::Package` / `RootProvenance::Package` / `MissingPackageContext` anywhere; Darkmatter holds the only Sniff-observation → catalog adapter and Claudine calls it.
- [x] Add/extend L2 coverage on the real compose and sequence surfaces per the repo's testing taxonomy (L2 fixtures must not take terminal focus): compose with `&`/`^`/`@`/implicit references from a nested package document, a proxied prompt with a materialized `spec`, a sequence task passing file parameters, and completion parity through the CLI completion surface.
- [ ] Cross-platform validation (AC9/AC10): host-independent parser tests run everywhere; filesystem-specific containment tests (junctions/reparse points) run on native Windows. Drive the matrix: local macOS full suites, then `build-linux`, `build-win`, and `build-win-native` for the three affected areas — coordinate with the Phase 1 ruling on the ctx-launch-anchor deferred gates so both land green before hosted CI.
- [x] Final validation sweep: `just test`, `just test-l2`, and `just lint` pass in `biscuit-file/`, `darkmatter/`, and `claudine/`; no L2 fixture took focus during the runs.
- [x] Walk the spec's acceptance criteria AC1–AC13 one by one against the implemented state and record the verdict matrix in this feature directory (e.g. `acceptance.md`); flag any criterion that needs a spec correction because the design-intent document disagrees (spec: the design-intent document wins).

**Checkpoint:** all three areas green on macOS + Linux + Windows environments; documentation and skills updated; acceptance matrix complete with every AC satisfied or explicitly escalated.
