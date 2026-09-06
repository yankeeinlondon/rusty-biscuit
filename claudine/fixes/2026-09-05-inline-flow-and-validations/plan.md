---
total_phases: 8
created: 2026-09-05
phase: 7
agent: codex/default
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/final_response_contract.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/golden_stderr.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/sections_and_output.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/opencode/tests.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/phase.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
  - darkmatter/lib/tests/inline_document_text.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
docs_updated_during_phase_2:
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/architecture.md
  - .claude/skills/darkmatter/schema.md
source_files_during_phase_3:
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
docs_updated_during_phase_3:
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/compose/mod.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/schema_interactive/status.rs
  - claudine/cli/src/commands/schema_interactive/tests.rs
  - claudine/cli/src/commands/wrap/composition/pipeline.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/launch_plan.rs
  - claudine/cli/src/commands/wrap/launch_plan/tests.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/sequence/jit.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/write_grant.rs
  - claudine/cli/src/commands/wrap/write_grant/tests.rs
  - claudine/cli/tests/compose_schema_cli.rs
  - claudine/cli/tests/dispatch_inventory.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/coordinator/tests.rs
  - claudine/lib/src/composition/guardrails.rs
  - claudine/lib/src/composition/inline_prompt.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/src/composition/schema/tests.rs
  - claudine/lib/src/composition/schema/translate.rs
  - claudine/lib/src/composition/select/tests.rs
  - claudine/lib/src/composition/types.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
docs_updated_during_phase_4:
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/composition.md
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/compose_caller_file_provenance.rs
  - claudine/cli/tests/compose_header_first.rs
  - claudine/cli/tests/compose_schema_cli.rs
  - claudine/cli/tests/composition_outputs.rs
  - claudine/cli/tests/error_guards.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/inline_compose_hash.rs
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
  - claudine/cli/tests/loop_cli.rs
  - claudine/cli/tests/sequence_groups.rs
  - claudine/cli/tests/sequence_prompt_property.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/completion.rs
  - claudine/lib/src/composition/completion/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/diagnostics/registry.rs
docs_updated_during_phase_5:
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/composition.md
source_files_during_phase_6:
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/harness_orch/types.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/tests/error_guards/transport-allow.toml
  - claudine/cli/tests/inline_completion_lifecycle.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
  - claudine/cli/tests/level2_sequence_task_stream_capture.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/completion.rs
  - claudine/lib/src/composition/completion/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_6:
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/composition.md
source_files_during_phase_7:
  - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
  - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
  - claudine/cli/tests/inline_completion_lifecycle.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
docs_updated_during_phase_7:
  - claudine/cli/README.md
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/docs/topics/lifecycle.md
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
  - prompts/_implement/implement-plan.md
  - prompts/_reviews/review-implementation.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/composition.md
  - .claude/skills/claudine/lifecycle.md
  - .claude/skills/claudine/timeline.md
  - .claude/skills/darkmatter/frontmatter.md
  - .claude/skills/darkmatter/schema.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
source_code:
  - claudine/cli/src/commands/compose/mod.rs
  - claudine/cli/src/commands/compose/prep.rs
  - claudine/cli/src/commands/schema_interactive/mod.rs
  - claudine/cli/src/commands/schema_interactive/status.rs
  - claudine/cli/src/commands/schema_interactive/tests.rs
  - claudine/cli/src/commands/wrap/composition/pipeline.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/target_launch/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/active_state_wiring.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs
  - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key.rs
  - claudine/cli/src/commands/wrap/harness_orch/session_key/tests.rs
  - claudine/cli/src/commands/wrap/harness_orch/types.rs
  - claudine/cli/src/commands/wrap/inline.rs
  - claudine/cli/src/commands/wrap/launch_plan.rs
  - claudine/cli/src/commands/wrap/launch_plan/tests.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/final_response_contract.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/golden_stderr.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/sections_and_output.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/overlay.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/sequence/jit.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/commands/wrap/write_grant.rs
  - claudine/cli/src/commands/wrap/write_grant/tests.rs
  - claudine/cli/tests/common/mod.rs
  - claudine/cli/tests/compose_caller_file_provenance.rs
  - claudine/cli/tests/compose_header_first.rs
  - claudine/cli/tests/compose_schema_cli.rs
  - claudine/cli/tests/composition_outputs.rs
  - claudine/cli/tests/dispatch_inventory.rs
  - claudine/cli/tests/error_guards.rs
  - claudine/cli/tests/error_guards/transport-allow.toml
  - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
  - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
  - claudine/cli/tests/inline_completion_lifecycle.rs
  - claudine/cli/tests/inline_compose_cli.rs
  - claudine/cli/tests/inline_compose_hash.rs
  - claudine/cli/tests/inline_compose_sequence_mismatch.rs
  - claudine/cli/tests/level2_lifecycle_control.rs
  - claudine/cli/tests/level2_schema_prompt_pty.rs
  - claudine/cli/tests/level2_sequence_task_stream_capture.rs
  - claudine/cli/tests/loop_cli.rs
  - claudine/cli/tests/sequence_groups.rs
  - claudine/cli/tests/sequence_prompt_property.rs
  - claudine/cli/tests/shipped_prompt_contract.rs
  - claudine/cli/tests/wrap_inline_compose.rs
  - claudine/cli/tests/wrap_inline_compose_interactive.rs
  - claudine/cli/tests/wrap_perf.rs
  - claudine/lib/src/composition/closure.rs
  - claudine/lib/src/composition/closure/tests.rs
  - claudine/lib/src/composition/completion.rs
  - claudine/lib/src/composition/completion/tests.rs
  - claudine/lib/src/composition/coordinator/tests.rs
  - claudine/lib/src/composition/error/mod.rs
  - claudine/lib/src/composition/error/render/mod.rs
  - claudine/lib/src/composition/error/render/schema.rs
  - claudine/lib/src/composition/error/tests.rs
  - claudine/lib/src/composition/guardrails.rs
  - claudine/lib/src/composition/inline_prompt.rs
  - claudine/lib/src/composition/mod.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/prepare/tests.rs
  - claudine/lib/src/composition/schema/classify.rs
  - claudine/lib/src/composition/schema/mod.rs
  - claudine/lib/src/composition/schema/tests.rs
  - claudine/lib/src/composition/schema/translate.rs
  - claudine/lib/src/composition/select/tests.rs
  - claudine/lib/src/composition/types.rs
  - claudine/lib/src/diagnostics/registry.rs
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/providers/opencode/tests.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/errors.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/phase.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/simplified/serialize.rs
  - darkmatter/lib/src/markdown/schemas/simplified/types.rs
  - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
  - darkmatter/lib/tests/inline_document_text.rs
  - darkmatter/lib/tests/meta_schema_phase3.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
documentation:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/cli-reference.md
  - .claude/skills/claudine/composition.md
  - .claude/skills/claudine/lifecycle.md
  - .claude/skills/claudine/timeline.md
  - .claude/skills/darkmatter/frontmatter.md
  - .claude/skills/darkmatter/schema.md
  - claudine/cli/README.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/execution-flow.md
  - claudine/docs/topics/flow-control/sequences.md
  - claudine/docs/topics/frontmatter-properties.md
  - claudine/docs/topics/lifecycle.md
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
  - prompts/_implement/implement-plan.md
  - prompts/_reviews/review-implementation.md
packages:
  - claudine
  - darkmatter
  - dmls
---

# Execution Plan — Inline Flow and Completion Validation

This plan implements [`spec.md`](spec.md) by making `inline-compose` file-aware,
adding phase-aware Darkmatter schema validation, and routing both composition
modes through one completion verdict before lifecycle `success` or `failure`.
It preserves the reviewed rulings: no response-frontmatter channel, no
completion-time composition or I/O inside validation, schema failure keeps a
successfully written inline artifact, and terminal lifecycle hooks remain
unrestricted.

## Delivery map

| Phase | Outcome | Depends on | Parallelization |
|---|---|---|---|
| 1 | Baseline, impact inventory, and acceptance matrix | — | Read-only discovery can be split by package |
| 2 | Darkmatter schema, document-text, and stream foundations | Phase 1 | Three independent workstreams |
| 3 | DMLS per-property diagnostics and catalog parity | Phase 2 schema workstream | Can overlap Phase 4 after its dependency is ready |
| 4 | File-aware inline preparation and minimum writable launch | Phase 2 schema + text workstreams | Provider-profile tests can be split by provider/OS |
| 5 | Shared completion evaluator and inline artifact closure | Phases 2 and 4 | Pure evaluator and renderer/error work can overlap |
| 6 | Lifecycle routing, rollback, retry/resume, proxy, sequence, and loop integration | Phase 5 | Scenario tests can be split after orchestration compiles |
| 7 | End-to-end acceptance coverage and documentation | Phases 3 and 6 | Docs and independent test suites can run concurrently |
| 8 | Full validation and close-out | Phase 7 | Package gates can run concurrently where resources allow |

## Risk and implementation constraints

- GitNexus reports `prepare_inline` as **CRITICAL** risk: 56 upstream symbols,
  11 direct callers, and eight affected modules. Migrate its result/state
  contract atomically and audit direct, loop, sequence, and canonical-service
  callers together.
- GitNexus reports `apply_inline_closure` as **HIGH** risk: 22 upstream symbols
  and 17 direct callers. Replace it only after the Darkmatter text primitive is
  tested, and keep the old call path compiling until the shared completion seam
  is ready.
- `handle_tool_use_completed` is LOW risk and independent of the composition
  migration, which is why its repair is isolated as a parallel Phase 2 track.
- Before editing any additional symbol, run scoped upstream GitNexus impact
  analysis. Warn before proceeding on any new HIGH or CRITICAL result. Before
  handoff, run `detect_changes(scope: "all")` against this worktree.
- Preserve the user's existing modifications to `spec.md` and
  `homelab/docs/unifi/products/voip.md`; do not fold them into implementation
  edits or use them as disposable fixtures.
- Do not run `cargo fmt`. All terminal/browser tests must remain headless and
  must not focus a host window.

## Phase 1 — Establish the baseline and executable acceptance matrix

**Goal:** turn the specification and current code into an agreed migration
surface before changing behavior.

### Tasks

- [x] Record scoped GitNexus `context` and upstream `impact` results for every
  existing public or orchestration symbol that will change, starting with
  `EffectiveSchema::validate*`, `prepare_inline`, `prepare_document`,
  `apply_inline_closure`, `try_inline_closure`, and the loop-control terminal
  path; list direct callers that must migrate atomically.
- [x] Inventory the existing schema preparation, interactive collection,
  closure, active-document/proxy, retry/resume, sequence, loop, launch-profile,
  and final-response test suites; map AC1–AC19 to a target L1, DMLS session, CLI
  integration, or L2 provider-stub test file.
- [x] Preserve the saved 2026-09-05 OpenCode transcript as a deterministic test
  fixture and identify a fixture location that does not depend on the modified
  `voip.md` working copy.
- [x] Inventory every shipped inline guardrail default (2026-03-17, 2026-03-27,
  2026-09-01, and 2026-09-05), the materialized migration logic, and the
  superseded response-block parser/status/tests so removal is complete and
  customized guardrail files remain untouched.
- [x] Capture current targeted baselines for Darkmatter schema/DMLS tests and
  Claudine composition/stream tests, noting any pre-existing failures without
  modifying unrelated files.

### Validation checkpoint

- [x] Confirm every acceptance criterion has an owner, target test tier, and
  prerequisite phase, with no unresolved design choices or open questions.
- [x] Confirm the source/caller inventory covers direct compose,
  `inline-compose`, sequence steps, loop iterations, proxy adoption, retry, and
  resume.

### Phase 1 evidence

#### GitNexus impact and atomic caller inventory

The worktree index was current for this branch. Counts below exclude test-only
upstream symbols; tests were included separately in the suite inventory. No
Phase 1 implementation symbol was edited.

| Symbol | Risk / upstream / direct | Production callers or atomic migration boundary |
|---|---:|---|
| `EffectiveSchema::validate` | LOW / 0 / 0 | Public compatibility entry point; retain unphased semantics. |
| `EffectiveSchema::validate_with_positions` | CRITICAL / 50 / 3 | `validate`, `validate_with_options`, and their schema suites. |
| `EffectiveSchema::validate_raw` | LOW / 4 / 3 | `analyze_frontmatter`, `schema_result_set_identical`, `schema_proven_quoting`. |
| `EffectiveSchema::validate_raw_with_positions` | LOW / 5 / 1 | `validate_raw`. |
| `EffectiveSchema::validate_with_options` | MEDIUM / 5 / 5 | Direct callers are existing unit-test variants. |
| `to_json_schema` | CRITICAL / 111 / 24 | Schema emission, catalog/expression schemas, baseline/meta schemas, standalone/root-union resolution, shape conversion, and tests. |
| `apply_hash_save_text` | CRITICAL / 33 / 10 | `run_hash_save`, `apply_inline_closure`, and hash/closure tests. |
| `schema_prepare_diagnostic` | LOW / 17 / 1 | DMLS frontmatter diagnostics projection. |
| `prepare_inline` | CRITICAL / 43 / 11 | `prepare_without_schema`, `build_loop_seed_with_lifecycle`, `prepare_document`, `run_prepare`, and preparation tests. |
| `prepare_document` | HIGH / 27 / 11 | `prepare_staged`, `materialize_harness_prompt`, and service tests. |
| `prepare_inline_with_schema` | CRITICAL / 34 / 5 | `compose_step`, `prepare_document`, and schema-preparation tests. |
| `post_shell_validate` | HIGH / 40 / 1 | `prepare_with_schema`. |
| `apply_inline_closure` | HIGH / 22 / 17 | `try_inline_closure` and closure tests. |
| `try_inline_closure` | HIGH / 6 / 4 | `classify_attempt_phase` and wrapper tests. |
| `dispatch_terminal_control` | HIGH / 6 / 3 | `start_lifecycle_phase`, `run_finalize_with_recovery`, `drive_terminal_recovery`. |
| `run_harness_loop_inner` | HIGH / 5 / 1 | `run_harness_loop`. |
| `classify_attempt_phase` | HIGH / 4 / 1 | `run_harness_loop_inner`. |
| `prepare_and_run_active_document` | HIGH / 8 / 2 | `run_composition_inner`, `run_step_proxy_loop`. |
| `execute_loop_or_single` | HIGH / 6 / 1 | `prepare_and_run_active_document`. |
| `build_and_run_loop` | LOW / 4 / 1 | `execute_loop_or_single`. |
| `build_launch_plan` | HIGH / 54 / 18 | `rebuild_launch_identity` plus launch-plan/provider tests. |
| `session_compat_key` | HIGH / 16 / 4 | `execute_attempt_phase` plus retry/resume tests. |
| `LiveSemanticSink::on_semantic_event` | LOW / 0 / 0 | Provider-neutral semantic event sink. |
| `handle_tool_use_completed` | LOW / 3 / 1 | OpenCode `feed_line`. |

The migration boundary therefore covers direct compose, inline compose,
sequence `compose_step`, loop seed/iterations, active-document preparation,
proxy adoption, and retry/resume compatibility. The high/critical preparation,
closure, schema-conversion, hash-write, launch-plan, and lifecycle paths must be
migrated with those callers rather than piecemeal.

#### Existing suite and artifact inventory

- Darkmatter schema coverage lives in
  `darkmatter/lib/src/markdown/schemas/tests/mod.rs`, the simplified-schema
  conversion/grammar/serialization modules, trigger tests, and
  `darkmatter/lib/tests/schemas_*` plus `meta_schema_repo_schemas.rs`.
- Darkmatter byte-preserving hash coverage lives in
  `darkmatter/lib/src/markdown/hash/write.rs` and already exercises LF/CRLF,
  ordering, duplicate keys, and byte preservation.
- DMLS projection coverage lives in
  `darkmatter/dmls/src/diagnostics/frontmatter.rs` and
  `darkmatter/dmls/tests/lsp_session.rs`.
- Claudine preparation/collection coverage lives in
  `composition/prepare/tests.rs`, `composition/prepare/service/tests.rs`, and
  `composition/schema/tests.rs`; closure coverage lives in
  `composition/closure/tests.rs` and `claudine/cli/tests/wrap_inline_compose.rs`.
- Provider stream coverage lives in `claudine/lib/tests/protocol_fixture_replay.rs`,
  `semantic_fidelity.rs`, and `claudine/cli/tests/wrap_structured_stream.rs`.
  The complete 2026-09-05 OpenCode assistant event stream is preserved at
  `claudine/lib/tests/fixtures/providers/opencode-voip-2026-09-05.ndjson` (75
  valid envelopes, ending in `step_finish`).
- Active-document, proxy, retry/resume, and terminal lifecycle coverage lives
  in the loop-control unit modules and
  `claudine/cli/tests/level2_lifecycle_control.rs`; sequence/loop coverage lives
  in `sequence_jit.rs`, `sequence_schema.rs`, `loop_cli.rs`, and
  `level2_lifecycle_loop.rs`; launch/session compatibility has colocated unit
  suites in `launch_plan.rs` and `session_key.rs`.

The inline guardrail inventory is: 2026-03-17 file-aware edit-file text,
2026-03-27 body-only response text, 2026-09-01 response-property allowlist,
and 2026-09-05 response-block text. `load_or_create_guardrails_with` migrates a
materialized file only when its bytes equal a known shipped default, writes the
replacement atomically, preserves custom files, and falls back to the current
in-memory protocol without truncating a file after migration failure. Phase 4
must retain 2026-09-05 in that known-default set when it becomes historical.
The superseded response channel is localized in `composition/closure.rs`
(`InlineReplacementParts`, response extraction/rewrite/node helpers,
harvested-property status and `frontmatter_changed`), its closure tests, the
CLI `wrap/inline.rs` wrapper/status tests, and response-block wording in the
composition docs and READMEs. Those are the Phase 5/7 removal surfaces.

#### Executable acceptance matrix

All choices below are fixed by the specification: validation is passive,
inline completion uses the on-disk document plus live overlays, direct compose
does no completion I/O, owned-property restoration is textual, and lifecycle
terminal hooks remain unrestricted.

| AC | Owner / prerequisite | Target observable test |
|---|---|---|
| AC1 | Darkmatter / Phase 2, DMLS / Phase 3 | L1 `schema_phase_validation`: exact eager string/number/object/datetime, nested/union/array-placement, native and quoted YAML, missing/present/null; passive shipped-schema corpus and real DMLS LSP artifact. |
| AC2 | DMLS / Phase 3 | `lsp_session`: two invalid property constraints produce two exact ranges while valid-neighbor hover remains type documentation. |
| AC3 | Claudine preparation / Phase 4 | Schema/prepare L1 plus CLI provider-stub L2: exact supplied prompt bypasses interaction; required non-eager inline value is deferred; launch evidence/status is retained. |
| AC4 | Claudine completion / Phase 5 | Pure evaluator L1: missing/invalid completion property produces structured failure and dependent nonzero outcome. |
| AC5 | Claudine launch / Phase 4 | Provider/OS launch-plan table plus provider-stub L2: exact native path, spaces/backslashes, writable root, and pre-spawn unsupported/denied errors. |
| AC6 | Darkmatter text + Claudine closure / Phases 2, 5 | Text-editor L1 and closure integration: agent-edited owned keys are restored, warned, freshly stamped, written once, then read/write/read stable. |
| AC7 | Claudine completion / Phase 5 | Evaluator/closure tests: identical and whitespace-only bodies fail; internal changes succeed and persist with dependent stamps. |
| AC8 | Claudine lifecycle / Phase 6 | Headless provider-stub L2: valid edit exits zero; malformed/invalid edit exits nonzero with typed diagnostic and no window focus. |
| AC9 | Claudine lifecycle / Phase 6 | Loop-control unit + L2: exact `initialize→launch validation→start→provider→closure→verdict→success|failure→finalize` order and exclusive branch effects. |
| AC9a | Claudine preparation / Phase 4 | Existing headless interactive harness plus provider-stub L2: missing eager is collected, absent required non-eager is not prompted inline, prompt not persisted. |
| AC9b | Claudine closure / Phase 5 | Closure integration invokes real `md hash --diff`; accepted output is coherent and repeated read/write/read is stable. |
| AC10 | Claudine completion / Phases 5, 6 | Evaluator L1 plus `sequence_schema`, `loop_cli`, and lifecycle L2: transient CLI/sequence/proxy values validate without leaking into source. |
| AC11 | Claudine streams / Phase 2 | `protocol_fixture_replay`: every adapter replays `text→tool activity→trailing text`; saved voip stream yields only trailing summary with stable tool counts. |
| AC12 | Darkmatter text / Phase 2 | Text-editor L1 corpus: LF/CRLF, block scalar, trailing spaces, four-space indent, order, native paths, add/replace/delete, duplicate/malformed errors, and repeated round trip. |
| AC13 | Claudine preparation / Phase 4 | Prepare L1 + headless interactive test: caller prompt wins before first interpolation/eager collection and is not persisted. |
| AC14 | Claudine completion / Phase 5 | Pure evaluator + overlay integration: direct uses live instance, inline applies disk delta then owned/live overlays, absent schema succeeds, and no completion I/O/expression runs. |
| AC15 | Darkmatter / Phase 2 | `schema_phase_validation`: launch/completion matrix for native/quoted scalars, missing/present/null, nested/union/file placement, raw JSON Schema, and generated-required boundary behavior. |
| AC16 | Darkmatter / Phase 2 | Schema L1: eager is accepted for active schemas and rejected for every passive trigger type/placement. |
| AC17 | Claudine completion / Phases 5, 6 | Closure malformed/duplicate/type-error L1 plus provider-stub L2: typed diagnostics, preserved successful write, failure lifecycle, and nonzero unrecovered exit. |
| AC18 | Claudine lifecycle / Phase 6 | Retry/resume/proxy unit + L2: baseline rollback before recovery, baseline retention on retry/resume, replacement on proxy, stale-file and failed-rollback outcomes. |
| AC19 | Claudine launch / Phase 4 | Launch-plan table covers every provider and macOS/Windows/Linux path/argument/environment shape; provider-stub L2 proves denial before spawn. |

Phase 7 supplies the required passive corpus sweep of all shipped schemas,
guardrails, and stream fixtures plus end-to-end tests through real shipped
artifacts and normal invocation paths. No acceptance criterion has an
unassigned owner, tier, prerequisite, or unresolved design choice.

#### Targeted baseline captured on 2026-09-05

- Darkmatter schema filter: 901 passed, 4,777 skipped, zero failures.
- DMLS schema-intelligence/session filter: 3 passed, 85 skipped, zero failures.
- Claudine composition/OpenCode/structured-stream filter: 1,947 passed, 4,561
  skipped, zero failures. The macOS linker emitted its existing oversized
  compact-unwind warning for the CLI test binary; it did not fail the build.
- Claudine package-area gates: `just lint` passed; `just test` ran 6,721 tests,
  with 6,721 passed and 11 intentionally skipped.

No existing targeted test reproduces the new completion contract; the exact
regression inputs and representation variants are assigned to the tests in the
matrix above before any Phase 2 implementation change. No unrelated file was
modified to establish these baselines.

## Phase 2 — Build the independent foundations

**Goal:** land the reusable Darkmatter contracts and the independent semantic
stream correction before Claudine orchestration depends on them.

The following workstreams are parallelizable after Phase 1. Each workstream
must pass its own checkpoint before a dependent phase begins.

### Workstream A — Phase-aware SimplifiedSchema validation

- [x] Add public `SchemaPhase::{Launch, Completion}` and phase-aware validation
  entry points that operate on a working instance without mutation, expression
  execution, schema resolution, file access, or network access; retain the
  existing unphased `validate*` semantics unchanged.
- [x] Accept `eager` as a universal SimplifiedSchema constraint and derive the
  recursive phase projection from the resolved schema shape: launch requires
  eager properties, completion requires required-or-eager properties, and
  present values remain type-checked in both phases.
- [x] Implement nested-property presence, union hoisting, `file(eager)[]`
  versus `file[](eager)` ownership, explicit-null-as-absence, and the existing
  eager-file existence semantics without encoding phase rules into authored or
  compiled JSON Schema.
- [x] Reject `eager` for passive trigger-match schemas across every type and
  retain raw JSON Schema's existing launch-time `required` behavior.
- [x] Preserve `generated; required` compatibility: unphased validation keeps
  its current exemption, launch allows the host-supply opportunity, and runtime
  completion rejects an absent generated required value.
- [x] Replace first-error conversion with a typed aggregate of independent
  per-property failures whose children retain property, message, span/origin,
  and referenced-schema provenance; keep structurally fatal root errors single.
- [x] Update every typed descriptor/per-type constraint catalog and serializer
  round trip so `eager` is advertised and preserved consistently.

### Workstream B — Darkmatter text-preserving inline document primitive

- [x] Add `restore_properties_text(current, snapshot, properties)` and
  `RestoredDocument { text, restored_properties, frontmatter_delta }` beside
  `apply_hash_save_text`, using the existing format-preserving frontmatter
  machinery instead of a second YAML editor.
- [x] Define semantic `FrontmatterDelta` entries for addition, replacement, and
  deletion while treating value-preserving reformatting as no semantic change;
  exclude restored closure-owned properties from the delta exposed to
  Claudine.
- [x] Preserve untouched bytes across block scalars, trailing spaces,
  four-space indentation, LF/CRLF, property order, and platform-native paths;
  return typed parse/duplicate-key/edit failures without writing.
- [x] Expose or reuse Darkmatter's non-strict `Simple` body hash comparison so
  leading/trailing whitespace and blank-line changes are ignored while internal
  whitespace remains significant.

### Workstream C — Provider-neutral final-response accumulation

- [x] Reset the live semantic sink's final-response accumulator on either
  `SemanticEvent::ToolCall` or `SemanticEvent::ToolResult`, while recording tool
  names exactly once and retaining trailing output text as the summary.
- [x] Repair OpenCode's completed `tool_use` adapter to emit its documented
  `ToolCall` immediately before `ToolResult`; update drifted module comments to
  describe the paired event contract.
- [x] Add one replay contract case per stream adapter (`text → tool activity →
  trailing text`) and assert the final response contains only trailing text;
  use the saved voip transcript for the OpenCode regression.

### Validation checkpoint

- [x] Run focused Darkmatter schema tests for AC1, AC15, and AC16, including
  nested, union, array-placement, null, raw JSON Schema, generated, and trigger
  cases.
- [x] Run Darkmatter text-editor round-trip tests covering all AC12 byte forms
  and semantic delta operations.
- [x] Run the cross-adapter replay suite and confirm the OpenCode voip fixture
  satisfies AC11 without altering other providers' tool counts.

## Phase 3 — Project schema-definition errors correctly in DMLS

**Goal:** expose the Phase 2 schema contract as precise editor diagnostics,
hover text, and completion data.

### Tasks

- [x] Teach `schema_prepare_diagnostic` to anchor `SchemaError::Convert` at the
  offending `$schema.<property>` value span, using the whole `$schema` span only
  for `<root>` or an explicitly unmappable referenced source.
- [x] Recursively project aggregate conversion children into one DMLS
  diagnostic per invalid property without attaching a neighbor's error to a
  valid property.
- [x] Preserve referenced-schema URI/origin information when available and use
  the documented reference-span fallback when the source cannot be mapped.
- [x] Drive frontmatter completion and hover from Darkmatter's descriptor
  catalog so every applicable type offers `eager` with the phase wording and
  no DMLS-local constraint list can drift.

### Validation checkpoint

- [x] Run DMLS unit and LSP session tests proving AC2: two invalid properties
  yield two correctly ranged diagnostics and hover on a valid neighbor remains
  type documentation.
- [x] Open the screenshot-equivalent schema fixture through the headless DMLS
  harness and confirm valid eager string/number/object/datetime declarations
  produce zero diagnostics.

## Phase 4 — Make inline preparation file-aware and writable

**Goal:** launch every inline agent with the real active document identity,
schema obligations, and the narrowest safe permission posture required to edit
that file.

### Tasks

- [x] Refactor schema-aware preparation so the retained launch-resolved schema,
  effective frontmatter, launch report, active native absolute path, and inline
  guard snapshot travel together in prepared/runtime state without resolving
  the schema again later.
- [x] Move inline's intrinsic `prompt` lookup behind caller overlays, first-pass
  interpolation, and eager collection; read the actual delivered prompt from
  effective frontmatter so a CLI-supplied or interactively collected prompt can
  satisfy the launch gate.
- [x] Apply `SchemaPhase::Launch` after the effective instance stabilizes:
  collect/fail missing eager properties in both modes, collect/fail required
  non-eager properties only for direct compose after authored expressions have
  resolved, never prompt inline mode for those properties, and retain the raw
  JSON Schema fail-fast path.
- [x] Replace inline response-block guardrails with a file-aware prompt header
  containing the absolute native document path and, when present, a schema
  property table with type, launch status, and completion requirement.
- [x] State the three owned properties (`prompt`, `hash`, `last_updated`), direct
  file-write/re-read duty, schema type duty, and two-to-three-paragraph summary
  contract using renderable prompt components where terminal rendering is
  involved.
- [x] Migrate materialized guardrails only when byte-equal to a known shipped
  default; preserve customized files and remove response-frontmatter harvesting
  language from every default.
- [x] Extend provider launch-plan construction to grant only the document's
  required writable root, honor explicit denies, reject unsupported/out-of-root
  cases before spawn with a typed provider/path/capability error, and include
  the effective sandbox/permission facet in retry/resume compatibility state.
- [x] Add table-driven launch-plan tests for every supported provider across
  macOS, Windows, and Linux path/argument/environment shapes without starting a
  real provider.

### Validation checkpoint

- [x] Prove AC3, AC5, AC13, and AC19 with focused preparation, guardrail
  migration, and launch-plan tests.
- [x] Confirm the delivered prompt contains the exact native absolute path,
  retains spaces/backslashes without JSON escaping, and uses a transient
  caller-supplied prompt without persisting it.

## Phase 5 — Implement one completion evaluator and inline artifact closure

**Goal:** make one passive completion verdict serve both modes while inline
mode reconciles the file atomically and direct compose performs no write.

### Tasks

- [x] Add `composition::completion` with `CompletionContext`, a pure
  `evaluate_completion(schema, instance, body_evidence)` function, typed
  `CompletionVerdict`/`CompletionOutcome`, and one
  `complete_active_document` orchestration entry point.
- [x] For inline mode, read and parse the agent-authored file after a successful
  provider exit, clean the candidate body in memory, and reject empty or
  non-strict-hash-unchanged bodies before stamping.
- [x] Restore `prompt`, `hash`, and `last_updated` textually from the active
  guard snapshot, emit one warning for each property the agent touched, compute
  a fresh Darkmatter `Simple` hash/date, and persist the accepted artifact with
  exactly one atomic write.
- [x] Build the inline completion instance by layering the semantic on-disk
  additions/replacements/deletions over live effective frontmatter, then apply
  restored owned values and fresh stamps; preserve transient CLI, sequence, and
  `proxy.with` inputs without writing them to the source.
- [x] For direct compose, pass live effective frontmatter unchanged and perform
  no source-file read or write.
- [x] Validate both instances against the retained schema at
  `SchemaPhase::Completion` without recomposition, coercion, expression/shell
  execution, schema re-resolution, or filesystem work inside the validator.
- [x] Reuse the schema status data/rendering path for declaration-ordered valid,
  missing, and invalid entries, and add typed
  `composition.body_unchanged` / `composition.completion_schema` diagnostics
  with structured property details.
- [x] Delete the closure-local YAML node editor, response-block extraction,
  harvested-property statuses, and response-capture fallback after all callers
  use Darkmatter's primitive and the agent final response is summary/output
  only.

### Validation checkpoint

- [x] Unit-test the pure evaluator for identical inline/direct schema status,
  absent schema success, missing/type/nested failures, transient overlays, and
  passive behavior (AC4, AC9, AC10, AC14).
- [x] Prove the closure's owned-node restoration, unchanged/empty rejection,
  semantic delta, byte preservation, fresh date, and `md hash --diff` coherence
  (AC6, AC7, AC9b, AC12).

### Phase 5 evidence

`composition::completion` owns the verdict; `composition::closure` is now the
file-aware artifact reconciler. `try_inline_closure` in the CLI reads the
document back instead of parsing provider output, which is what let the
response-block channel be deleted; the loop-control call site still routes
failures through the old `inline_closure` error kind, and Phase 6 replaces it
with `complete_active_document` and the typed `err`.

| Requirement | Targeted tests |
|---|---|
| Pure evaluator, absent schema, missing/type/null failures | `composition::completion::tests::{a_document_without_a_schema_completes_when_its_body_changed, a_satisfied_instance_completes_and_reports_every_property, a_required_property_the_agent_never_set_fails_completion, an_explicit_null_is_absence_at_completion, a_value_of_the_wrong_type_fails_completion_with_the_type_message}` |
| Nested / array-indexed property paths (AC4) | `composition::completion::tests::{a_nested_property_failure_names_its_full_path, dotted_pointer_renders_nested_and_indexed_paths}` |
| Declaration-ordered status reuse (AC4, AC10) | `composition::completion::tests::problems_follow_the_status_tables_declaration_order` (proved non-vacuous by neutering the sort: validator order is alphabetical, the table's is not) |
| Identical inline/direct status (AC10) | `composition::completion::tests::inline_and_direct_reach_identical_status_for_the_same_instance` |
| Transient overlays survive; agent delta is observable (AC14) | `composition::completion::tests::{a_transient_caller_value_satisfies_completion_and_removing_it_does_not, inline_layers_the_agent_delta_over_the_live_effective_instance, an_agent_deletion_is_observable_at_completion}` |
| Passive validation — no filesystem work (AC14) | `composition::completion::tests::validation_performs_no_filesystem_work_for_an_eager_file_property` |
| Direct compose performs no read/write | `composition::completion::tests::direct_compose_judges_the_live_instance_and_never_touches_the_file` |
| Typed `composition.body_unchanged` / `composition.completion_schema` | `composition::completion::tests::{body_rejection_outranks_schema_problems, an_empty_body_is_reported_as_its_own_reason}`; `claudine-cli::error_guards` corpus |
| Owned-node restoration + one warning each (AC6) | `composition::closure::tests::restores_every_owned_property_the_agent_touched_and_warns_once_each`; `claudine-cli::wrap_inline_compose::inline_compose_preserves_frontmatter_and_restores_owned_properties` |
| Unchanged / empty rejection, no stamp, no write (AC7) | `composition::closure::tests::{refuses_an_untouched_document_without_writing, refuses_a_whitespace_only_body_change, refuses_an_empty_body_even_when_frontmatter_changed, refuses_a_baseline_that_was_never_cleanup_stable}`; `claudine-cli::wrap_inline_compose::inline_compose_rejects_a_document_the_agent_never_updated` |
| Semantic delta incl. reformat-is-not-a-change (AC12) | `composition::closure::tests::{reports_the_agents_semantic_delta_excluding_owned_properties, value_preserving_reformatting_is_not_a_semantic_change}` |
| Byte preservation, fresh date, `md hash --diff` coherence (AC9b, AC12) | `composition::closure::tests::{accepts_the_agents_body_and_stamps_a_coherent_hash, preserves_crlf_and_the_authored_last_updated_quote_style, cleans_the_agents_body_and_hashes_the_cleaned_text}`; `claudine-cli::inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff` |
| Read/write/read round-trip stability | `composition::closure::tests::{read_write_read_is_stable_and_the_second_pass_refuses, repeated_reconciliation_of_identical_inputs_is_byte_deterministic}` |
| Typed read/edit/hash failures leave the document alone | `composition::closure::tests::{reports_a_malformed_stored_hash_without_mutating_the_document, reports_a_duplicate_owned_key_without_mutating_the_document, reports_a_document_the_agent_deleted}`; `claudine-cli::wrap_inline_compose::inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document` |
| Agent-authored frontmatter survives across runs | `claudine-cli::wrap_inline_compose::inline_compose_keeps_agent_written_frontmatter_across_runs` |

Every `inline-compose` CLI fixture migrated from response capture to a
file-aware agent stub (`common::InlineAgentStub`, `common::INLINE_BODY_REWRITE`,
`common::INLINE_DOC_FROM_PROMPT`). The stubs use shell builtins only, because
inline fixtures routinely set `PATH` to the stub directory alone.

Gates: `just test` in `claudine/` — 6,766 passed, 11 skipped, zero failures.
`just lint` in `claudine/` — clean. No L2/L3 tier was run in this phase; the
provider-stub lifecycle coverage belongs to Phase 6.

## Phase 6 — Integrate lifecycle routing, rollback, and repeated compositions

**Goal:** place the completion verdict at the correct lifecycle boundary and
preserve recovery semantics across every active-document transition.

### Tasks

- [x] Replace the inline-only `try_inline_closure` branch in loop control with
  the single `complete_active_document` call site used by both modes, ordered as
  `initialize → launch validation → start → provider → inline closure → verdict
  → success|failure → finalize`.
- [x] Publish the provider's final summary into run output before terminal
  lifecycle handling so CLI display and `last(outputs)` receive the summary but
  the document never does.
- [x] Route a failed completion verdict through ordinary `failure` recovery
  with the typed `err` payload; prevent `success` from firing first and exit
  non-zero only when retry, resume, or proxy does not recover.
- [x] Capture the full inline baseline when an active document is invoked or
  adopted; retain it across retry/resume, discard it on proxy, and capture the
  target's own baseline after adoption.
- [x] Atomically restore the baseline before failure handling on provider
  non-zero exit, exit 130/interrupt, post-start launch failure, closure
  parse/edit failure, duplicate owned keys, empty body, or unchanged body; do
  not stamp a new hash/date on rollback.
- [x] When rollback fails, retain the initiating diagnostic and attach a typed
  rollback cause containing path and I/O detail; never print a restoration
  success message.
- [x] Keep a successfully written inline artifact on completion-schema failure,
  and carry operation-level body-change evidence across metadata-only
  retry/resume so recovery is not rejected as unchanged.
- [x] Apply the verdict once per sequence step and loop iteration under existing
  `fail_fast` behavior, while keeping success/failure/finalize hooks unrestricted
  after the verdict as ruled.
- [x] Recompute and compare the complete launch/session compatibility facet for
  retry/resume, including writable-root permission state, while keeping
  invocation CWD/system-prompt inputs immutable.

### Validation checkpoint

- [x] Add provider-stub coverage for successful file writes and summaries,
  missing/wrong/correct completion properties, provider exits 1 and 130,
  malformed/duplicate/half-written documents, rollback failure, and artifact
  retention on schema failure (AC4–AC9b, AC17).
- [x] Add lifecycle tests proving failure-before-success ordering, structured
  `err`, retry/resume metadata-only recovery, provider-failure rollback before
  retry, proxy guard replacement, sequence step-2 `fail_fast`, and per-iteration
  loop verdicts (AC9, AC10, AC18).

### Phase 6 evidence

`classify_attempt_phase` now has one terminal seam for both modes:
provider failure (rollback → `failure` recovery) → summary published to
`outputs` → `complete_active_document` → `failure` recovery or `success`. The
inline-only `try_inline_closure` wrapper is deleted; `wrap::inline` retains only
the artifact *reporting*, and its typed error now reaches the lifecycle
directly (the `format_context` transport-allow entry for it was removed, which
is the D10 routing change that entry deferred).

The guard became **operation-level** state on the harness loop
(`InlineOperation`): captured from the read the provider runs against, retained
across `retry`/`resume`, and never carried across a `proxy` — a hand-off ends
the loop, and the target's own run captures its own baseline.
`MaterializedHarnessPrompt` now carries `launch_schema`, so the retained schema
reaches the verdict without a second resolution.

Two behaviors were corrected while wiring this up:

- An owned property absent from the file is no longer treated as absent at
  completion. A transient caller-supplied `prompt` is deliberately never
  persisted, so overwriting the live effective value with the on-disk absence
  failed a run whose launch gate the same value had satisfied.
- `reconcile_inline_artifact_with_evidence` suppresses the `Unchanged`
  rejection when the operation already produced a body, which is what makes a
  metadata-only `retry`/`resume` recoverable (AC18). `Empty` is never rescued.

| Requirement | Targeted tests |
|---|---|
| One call site, both modes; summary reaches the caller, never the document (AC5, AC10) | `claudine-cli::inline_completion_lifecycle::a_satisfied_inline_run_exits_zero_and_keeps_the_summary_out_of_the_document` |
| Missing / wrong-typed / satisfied completion property (AC4) | `inline_completion_lifecycle::{a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact, a_wrong_typed_completion_property_reports_the_type_mismatch}` |
| Artifact retention on schema failure (AC9b) | `inline_completion_lifecycle::a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact`; `composition::completion::tests::a_required_property_the_agent_never_set_fails_completion` |
| Failure before success, typed `err`, then `finalize` (AC9) | `inline_completion_lifecycle::{a_failed_inline_verdict_fires_failure_with_the_typed_err_then_finalize, a_refused_body_fires_failure_with_the_body_unchanged_code, a_direct_compose_run_fails_completion_when_a_start_effect_invalidates_a_property, a_direct_compose_run_with_a_satisfied_property_reaches_success}` |
| Rollback on provider exit 1 / 130 / empty body / duplicate owned key / parse failure (AC8, AC17) | `inline_completion_lifecycle::{a_provider_exit_one_restores_the_captured_baseline, a_provider_exit_130_restores_the_captured_baseline, an_empty_candidate_body_is_refused_and_rolled_back, a_duplicate_owned_key_is_refused_and_rolled_back}`; `claudine-cli::wrap_inline_compose::inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document`; `composition::closure::tests::{restoring_the_baseline_rewrites_the_captured_bytes_exactly, restoring_twice_is_byte_identical}` |
| Failed rollback keeps the initiating diagnostic and adds a typed cause | `inline_completion_lifecycle::a_failed_rollback_reports_the_typed_cause_and_keeps_the_initiating_error`; `composition::closure::tests::a_restoration_that_cannot_write_reports_the_typed_rollback_failure`; `composition::error::tests::every_batch_3_variant_projects_a_catalog_shaped_detail` |
| Metadata-only recovery carries body-change evidence; provider failure rolls back first (AC18) | `inline_completion_lifecycle::{a_metadata_only_retry_recovers_a_completion_schema_failure, a_provider_failure_rolls_back_before_the_retry_reads_the_document, a_failed_provider_attempt_hands_the_retry_a_restored_document}`; `composition::closure::tests::{carried_body_change_evidence_accepts_a_metadata_only_candidate, carried_evidence_still_refuses_an_empty_body}`; `composition::completion::tests::carried_body_change_evidence_reaches_the_inline_closure` |
| Proxy discards the source guard and captures the target's own baseline (AC18) | `inline_completion_lifecycle::a_proxy_out_of_a_failed_verdict_captures_the_targets_own_baseline`; `claudine-cli::level2_lifecycle_control::level2_lifecycle_inline_compose_proxy_closure_rewrites_only_final_target` |
| One verdict per sequence step and loop iteration under `fail_fast` (AC10) | `inline_completion_lifecycle::{a_sequence_applies_the_verdict_per_step_and_fail_fast_stops_at_step_two, a_loop_applies_the_verdict_per_iteration}` |
| Writable-root permission state is recomputed per attempt for retry/resume (AC19) | `loop_control::target_launch::tests::{the_write_posture_is_rebuilt_from_the_active_document, a_direct_run_records_no_write_posture}`; `harness_orch::session_key::tests::write_posture_moves_the_permission_facet` |
| Transient caller inputs survive the owned-property overlay (AC13, AC14) | `composition::completion::tests::a_transient_caller_prompt_survives_the_owned_property_overlay`; `claudine-cli::wrap_inline_compose::inline_compose_uses_a_caller_supplied_prompt_without_persisting_it` |

Non-vacuity was proved by neutering both new guards at once — forcing
`prior_body_change: false` and making `rollback_inline_document` a no-op — which
turned 7 of the 15 new integration tests red; restoring them returned all 15 to
green.

Gates: `just test` in `claudine/` — 6,796 passed, 11 skipped, zero failures.
`just lint` — clean. `claudine/docs/providers/dispatch-inventory.json` was
regenerated (`CLAUDINE_UPDATE_INVENTORY=1`); it had drifted from the phase-2/4
untracked source files as well as this phase's one new `Provider::` reference.

`just test-l2` found seven failures, all fixtures encoding contracts phases 2-5
retired (L2 was not run in those phases):

- `level2_lifecycle_inline_compose_proxy_closure_rewrites_only_final_target` —
  `write_inline_body_goose` was still a response-capture stub, so the file-aware
  closure correctly refused it as an unchanged body. Migrated to the shared
  file-aware stub (`INLINE_DOC_FROM_PROMPT` + `INLINE_BODY_REWRITE`).
- `level2_pty_inline_compose_{interactive_flag,frontmatter_interactive}_collects_before_launch`
  — both fixtures declared `topic: 'string(required)'` and expected inline mode
  to collect it before launch. Under AC13 inline mode never prompts for a
  required-but-not-eager property, so the fixtures now declare it `eager`,
  which is the half of the split that *is* collected at launch.
- `level2_sequence_task_stream_capture::{level2_parallel_prompt_streams_keep_task_attribution_in_tmux, level2_prompt_idle_flush_keeps_the_task_bar_in_tmux}`
  — the member documents carried a root `prompt:`, making them inline documents
  whose completion verdict demands the agent write the file, while the `claude`
  stream stubs only emit a transcript. These rows are about task stream
  attribution, so the members are now direct-compose documents.
- `level2_shipped_implement_plan_*` — see the open finding below.

After those fixes L2 is 235/236. The one remaining red,
`level2_typed_error_render_capture::level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`,
is blocked by the host rather than by this change: the spawned WezTerm login
shell sits on Atuin's interactive "Atuin AI is not yet configured" onboarding
dialog, so the pane never executes the `claudine` command and no Claudine
output appears at all. It reproduces in isolation and is unrelated to the
composition path.

One further intermittent red was observed and is also pre-existing:
`level2_shipped_implement_plan_supplied_commit_message_runs_exact_commit_branch`
occasionally reads a mangled `events.log` line (`git-committts-ran`) because the
fixture's `git`, `say`, and `just` doubles append to one file concurrently
without locking. It passed 3/3 in isolation immediately afterwards; the fixture's
append pattern, not the verdict, is the cause.

#### Open finding — `eager` without `required` is no longer expressible as "optional"

Running `just test-l2` surfaced this against a *shipped* artifact, not a test
fixture. `prompts/_implement/implement-plan.md` declares

```yaml
spec: file(eager; match(**/*spec*.md))
```

and derives it with `spec: "{{ … dirname(plan) + '/spec.md' … : null }}"`, so a
plan with no sibling `spec.md` legitimately yields `null`. The ratified rule
(spec §D1: "`eager` without `required` is accepted and equivalent to
`required; eager` for validation purposes") makes that property present-required
at completion, so the run now fails with
``spec — null is not of type "string"``. There is no longer a way to say
"optional, but resolve and check it eagerly when supplied", which is exactly
what this prompt means.

The two `level2_shipped_implement_plan_*` rows were unblocked by staging the
sibling `feature/spec.md` their subject (commit-message branching) always
implied. That is a fixture fix, not a resolution: the shipped prompt is still
mis-declared under the new vocabulary, and reconciling it is a Phase 7 task
because it touches an outward-facing artifact and its drift hash pin. Ken's
call is needed on which way it goes — relax the prompt to a non-eager `file`
(losing eager reference resolution for a caller-supplied `spec`), or revisit
the `eager`-implies-`required` ruling.

## Phase 7 — Complete acceptance coverage and drift maintenance

**Goal:** close cross-package compatibility gaps and update every public
contract that changed.

### Tasks

- [x] Run the full AC1–AC19 matrix and add any missing regression cases for
  caller/sequence/proxy overlays, agent delta deletion, raw JSON Schema,
  generated properties, interactive-denied launch, and all three OS path
  shapes.
- [x] Reconcile `prompts/_implement/implement-plan.md` (and its drift-pinned
  fixture copy) with the ratified `eager` semantics — see *Open finding:
  optional eager files* below — then refresh the shipped-prompt hash pin.
- [x] Add passive shipped-artifact corpus coverage for SimplifiedSchema/trigger
  changes and an end-to-end normal invocation test using a real shipped schema,
  as required by the Darkmatter skill.
- [x] Update Darkmatter schema-definition and inline-validation docs with the
  launch/completion table, universal `eager`, nested/union/array placement,
  trigger rejection, raw JSON Schema behavior, and generated interplay.
- [x] Update Claudine composition, frontmatter-property, agent, CLI README, and
  lifecycle docs for file-aware inline operation, owned-property restoration,
  summary output, writable permissions, completion ordering, artifact
  retention, recovery, and per-composition sequence/loop validation.
- [x] Update the Darkmatter and Claudine skill references/catalogs so their
  architecture and workflow descriptions match the implemented contracts;
  remove stale response-frontmatter/transcription guidance.
- [x] Review every changed symbol's `///`, `//!`, and inline comments; correct
  behavior drift (especially OpenCode's event comment and closure narration)
  without unrelated comment cleanup.

### Validation checkpoint

- [x] Confirm documentation examples and descriptor output agree with the same
  typed catalogs used by implementation and DMLS.
- [x] Search for retired response-block terminology and closure-local YAML
  editor symbols; only historical specs may retain them.

### Phase 7 evidence

#### AC1–AC19 matrix

Every row was executed, not merely mapped. The Darkmatter/DMLS rows ran under
`cargo nextest -p darkmatter -p dmls`; the Claudine rows under
`cargo nextest -p claudine -p claudine-cli`; both also ran inside their package
`just test` gates.

| AC | Evidence |
|---|---|
| AC1 | `darkmatter::schema_phase_validation::{launch_requires_eager_and_completion_requires_required_or_eager, all_eager_scalar_representations_are_coerced_and_checked, nested_and_union_eager_presence_is_recursive_and_hoisted, eager_array_placement_owns_items_or_property, original_voip_schema_launches_with_outputs_absent_and_enforces_them_at_completion}`; `dmls::lsp_session::{eager_schema_fixture_is_clean_and_catalog_driven, original_voip_schema_definitions_are_clean}` |
| AC2 | `dmls::lsp_session::{schema_definition_errors_are_independent_and_property_ranged, referenced_schema_conversion_error_keeps_origin_and_reference_fallback}` |
| AC3 | `claudine-cli::wrap_inline_compose::{inline_compose_without_an_eager_prompt_fails_before_launch_naming_it, inline_compose_uses_a_caller_supplied_prompt_without_persisting_it}`; `claudine::composition::prepare::tests` |
| AC4 | `claudine::composition::completion::tests::{a_required_property_the_agent_never_set_fails_completion, a_value_of_the_wrong_type_fails_completion_with_the_type_message, an_explicit_null_is_absence_at_completion, a_nested_property_failure_names_its_full_path}`; `claudine-cli::inline_completion_lifecycle::{a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact, a_wrong_typed_completion_property_reports_the_type_mismatch, a_satisfied_inline_run_exits_zero_and_keeps_the_summary_out_of_the_document}` |
| AC5 | `claudine-cli::wrap_inline_compose::{inline_compose_delivers_the_native_document_path_and_file_aware_guardrails, inline_compose_writes_the_agents_file_and_reports_only_the_final_summary}`; `claudine-cli::inline_completion_lifecycle::a_satisfied_inline_run_exits_zero_and_keeps_the_summary_out_of_the_document` |
| AC6 | `claudine::composition::closure::tests::restores_every_owned_property_the_agent_touched_and_warns_once_each`; `claudine-cli::wrap_inline_compose::inline_compose_preserves_frontmatter_and_restores_owned_properties` |
| AC7 | `claudine::composition::closure::tests::{refuses_an_untouched_document_without_writing, refuses_a_whitespace_only_body_change, refuses_an_empty_body_even_when_frontmatter_changed}`; `claudine-cli::wrap_inline_compose::inline_compose_rejects_a_document_the_agent_never_updated` |
| AC8 | `claudine-cli::inline_completion_lifecycle::a_provider_exit_130_restores_the_captured_baseline` |
| AC9 | `claudine-cli::inline_completion_lifecycle::{a_failed_inline_verdict_fires_failure_with_the_typed_err_then_finalize, a_refused_body_fires_failure_with_the_body_unchanged_code, a_direct_compose_run_fails_completion_when_a_start_effect_invalidates_a_property, a_direct_compose_run_with_a_satisfied_property_reaches_success}` |
| AC9a | `claudine-cli::compose_schema_cli::compose_conditional_required_property_fails_before_launch_when_interaction_is_denied` (interaction-denied *and* the resolvable-input control); `claudine-cli::inline_completion_lifecycle::a_conditional_required_property_never_blocks_an_inline_launch`; `claudine-cli::schema_interactive::tests::pre_validate_with_interactive_returns_missing_when_not_allowed` |
| AC9b | `claudine-cli::inline_completion_lifecycle::a_missing_completion_property_fails_the_run_and_keeps_the_written_artifact`; `claudine-cli::inline_compose_hash::inline_compose_writes_hash_that_passes_md_diff` |
| AC10 | `claudine::composition::completion::tests::{inline_and_direct_reach_identical_status_for_the_same_instance, problems_follow_the_status_tables_declaration_order}`; `claudine-cli::inline_completion_lifecycle::{a_sequence_applies_the_verdict_per_step_and_fail_fast_stops_at_step_two, a_loop_applies_the_verdict_per_iteration}` |
| AC11 | `claudine-cli::live_semantic_sink::tests::final_response_contract` (one case per adapter) plus the saved `opencode-voip-2026-09-05.ndjson` replay |
| AC12 | `darkmatter::markdown::hash::write::tests` byte-preservation matrix (`textual_save_treats_indentless_sequence_as_one_node` and the representation/newline matrix); `claudine::composition::closure::tests::{reports_the_agents_semantic_delta_excluding_owned_properties, value_preserving_reformatting_is_not_a_semantic_change, preserves_crlf_and_the_authored_last_updated_quote_style}` |
| AC13 | `claudine-cli::wrap_inline_compose::inline_compose_uses_a_caller_supplied_prompt_without_persisting_it`; `claudine::composition::completion::tests::a_transient_caller_prompt_survives_the_owned_property_overlay`; `claudine-cli::compose_schema_cli::compose_conditional_required_property_fails_before_launch_when_interaction_is_denied` |
| AC14 | Caller: `claudine::composition::completion::tests::a_transient_caller_value_satisfies_completion_and_removing_it_does_not`. Proxy: `claudine-cli::inline_completion_lifecycle::a_proxy_with_overlay_satisfies_the_targets_completion_schema`. **Sequence (added this phase):** `claudine-cli::inline_completion_lifecycle::sequence_step_params_satisfy_the_referenced_documents_completion_schema`. Delta: `…::{inline_layers_the_agent_delta_over_the_live_effective_instance, an_agent_deletion_is_observable_at_completion}`. Passivity: `…::validation_performs_no_filesystem_work_for_an_eager_file_property` |
| AC15 | `darkmatter::schema_phase_validation::raw_json_schema_keeps_required_at_launch`; `claudine::composition::completion::tests::raw_json_schema_required_is_enforced_at_completion` |
| AC16 | `darkmatter::schema_phase_validation::generated_required_keeps_authoring_compatibility_but_fails_completion`; `claudine::composition::completion::tests::a_generated_required_property_must_be_supplied_by_completion` |
| AC17 | `claudine-cli::inline_completion_lifecycle::{a_provider_exit_one_restores_the_captured_baseline, an_empty_candidate_body_is_refused_and_rolled_back, a_duplicate_owned_key_is_refused_and_rolled_back, a_failed_rollback_reports_the_typed_cause_and_keeps_the_initiating_error}`; `claudine-cli::wrap_inline_compose::inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document` |
| AC18 | `claudine-cli::inline_completion_lifecycle::{a_metadata_only_retry_recovers_a_completion_schema_failure, a_provider_failure_rolls_back_before_the_retry_reads_the_document, a_failed_provider_attempt_hands_the_retry_a_restored_document, a_proxy_out_of_a_failed_verdict_captures_the_targets_own_baseline}` |
| AC19 | `claudine-cli::wrap::write_grant::tests::{every_provider_plans_inside_and_outside_on_every_path_shape, containment_follows_the_native_path_shape, windows_verbatim_documents_are_granted_with_the_legacy_spelling, explicit_denies_refuse_before_spawn_and_are_never_widened, kilo_outside_its_worktree_is_a_typed_unsupported_capability}`; `claudine-cli::wrap_inline_compose::inline_compose_refuses_an_explicit_write_deny_before_spawn` |

Only one row had a genuine gap. AC14 named three transient channels — caller
input, sequence state, and `proxy.with` — and only two were proved. The new
`sequence_step_params_satisfy_the_referenced_documents_completion_schema` runs a
real `claudine sequence` whose step supplies `researched_by` through `params:`;
the referenced document's completion schema is satisfied for that step and the
value never reaches the file. It carries its own control: re-running the same
document standalone fails naming `researched_by`. The first draft of its
"never persisted" assertion matched the `$schema` declaration line as well as a
value, which is how it was proved non-vacuous — it failed against the real
written document before being narrowed to the value.

#### Checkpoint results

- **Descriptor/documentation agreement.** `md schema about` built from this
  worktree reports `eager` with `target_types: all types` and the phase wording,
  matching `SCHEMA_CONSTRAINT_DESCRIPTORS`, the per-type
  `accepted_constraints` rows, and the DMLS completion item
  (`dmls::providers::dsl::tests::text_edit_item_carries_eager_edit_and_markdown_documentation`).
  `darkmatter/docs/topics/schema-definition.md` agrees, including the
  launch/completion table and the explicit note that an optional-but-eager
  property is not expressible. **Trap worth recording:** the *installed*
  `~/.cargo/bin/md` (2026-09-05) still printed the old `eager | file | Require
  the referenced file to exist` row, which reads exactly like drift. Confirm
  descriptor output against a binary built from the worktree.
- **Retired terminology sweep.** No production symbol, comment, or doc retains
  `response_frontmatter`, response-block extraction, `InlineReplacementParts`,
  `semantic_top_level_key`, `rewrite_harvested_frontmatter`, or a
  closure-local YAML node editor. Two deliberate survivors:
  `composition::guardrails::SHIPPED_GUARDRAILS_2026_09_01` / `_2026_09_05` are
  the historical shipped defaults the migration matches byte-for-byte, and
  `default_guardrails_state_the_file_aware_contract` asserts the retired
  phrases are *absent* from the live default. One incidental use was cleaned
  up: a Darkmatter hash-write fixture used `response_frontmatter:` as an
  arbitrary indentless-sequence key and is now `reviewers:`.

#### Gates

- `just test` in `claudine/` — 6,805 passed, 11 skipped, zero failures (6,804
  before this phase's added test).
- `just lint` in `claudine/` — clean.
- `just test` in `darkmatter/` (includes DMLS and the Zed CLI) — 7,689 passed,
  51 skipped, zero failures.
- `just lint` in `darkmatter/` — clean, including the `wasm32-wasip2` Zed
  extension check.
- No L2/L3 tier was run in this phase. Phase 6 left L2 at 235/236 with one
  host-blocked red (`level2_initialize_proxy_block_auto_detects_osc8_in_wezterm`,
  an Atuin onboarding dialog in the spawned WezTerm login shell), and this
  phase changed no terminal-surface behavior; `just test-l2` remains Phase 8's
  task.

#### Note on the working tree

Phases 1–6 and the first four Phase 7 tasks were committed to this branch by a
separate process while this phase was running (`8f02d3e83`…`5bc65559e`). No
commit or stage was performed by this phase; its changes are uncommitted.

## Phase 8 — Run final gates and prepare handoff

**Goal:** demonstrate that the implementation is complete, scoped, portable,
and ready for review without committing.

### Tasks

- [x] From `darkmatter/`, run `just build`, `just test`, and `just lint`; ensure
  the included DMLS suites and shipped-artifact corpus pass.
- [x] From `claudine/`, run `just build`, `just test`, and `just lint`.
- [ ] Run `just test-l2` from `claudine/` for provider-stub/terminal-harness
  coverage, ensuring no terminal or browser window gains focus.
- [ ] From the repository root, run `just ci-local` before push; record any
  platform coverage that remains construction-only because the host is macOS.
- [ ] Run GitNexus `detect_changes(scope: "all", worktree: <this worktree>)` and
  verify changed symbols/flows match schema, DMLS, composition, launch, stream,
  and documentation scope; investigate any unexpected module.
- [ ] Review `git diff --check` and `git status --short`, confirming no unrelated
  user edits were overwritten, no generated/provider data was hand-edited, and
  no `cargo fmt` or commit was performed.
- [ ] Report the completed AC1–AC19 matrix, exact commands/results, known
  macOS-only execution limits, and any follow-up that requires native Windows
  or Linux execution.

### Validation checkpoint

- [ ] All package and repository gates pass, all AC1–AC19 rows are evidenced,
  GitNexus reports only expected affected scope, and the worktree contains only
  intentional implementation/documentation changes plus the user's preserved
  pre-existing edits.
