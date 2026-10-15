---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/claudine/features/2026-10-23-shell-template-approval/spec.md"
plan: "claudine/features/2026-10-23-shell-template-approval/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: ["darkmatter", "darkmatter-cli", "claudine", "claudine-cli", "test-toolkit"]
human_review: false
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/matching.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/schemas/simplified/subtype.rs
    - darkmatter/lib/src/markdown/schemas/simplified/mod.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
message_to_agent: "Phase 6 documentation and acceptance audit is complete. Review the Phase 6 AC/R tables before feature sign-off: first-task primary shell and task setup/teardown sites with known values retain only resolved bytes, so their optional shape choice is missing. Nested sequences and group terminal events are unavailable; serial groups validate per member. Dedicated skipped-predecessor CLI and changed-schema parse-cache cases remain unproven. Current output root is outputs, and earlier schema diagnostics may disclose rejected values. Phase 6 source edits are comments only. Do not interpret implemented=true as proof that every acceptance clause passed. Keep the feature active for author review; do not move, stage, or commit it."
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/expression/binding.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/approval.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shell_template_discovery.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/cli/src/approval.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/audit.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/coordinator/tests.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service/tests.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/invocation_context/document_tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/shell_missing_values.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/docs/topics/pre-flight-checks.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/claudine/cli-commands.md
source_files_during_phase_4:
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/harness_orch/shell_options.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/src/commands/wrap/sequence/task_run.rs
    - claudine/cli/src/commands/wrap/sequence/task_run/tests.rs
    - claudine/cli/tests/bin/argv_recorder/main.rs
    - claudine/cli/tests/bin/fake_goose/main.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/document_environment_routes.rs
    - claudine/cli/tests/l1/level1_dry_run_pty.rs
    - claudine/cli/tests/l1/lifecycle_shell_templates.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/sequence_cli.rs
    - claudine/cli/tests/l1/sequence_errors_cli.rs
    - claudine/cli/tests/l1/sequence_shell_templates.rs
    - claudine/cli/tests/l1/shell_approval_prompt.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/bindings.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/mod.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs
    - claudine/lib/src/composition/lifecycle/mod.rs
    - claudine/lib/src/composition/lifecycle/parse.rs
    - claudine/lib/src/composition/lifecycle/shell_template.rs
    - claudine/lib/src/composition/lifecycle/structured_shell.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/preflight/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/reapproval.rs
    - claudine/lib/src/composition/sequence/preflight/shape.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/shell.rs
    - claudine/lib/src/composition/sequence/task/shell/structured_tests.rs
    - claudine/lib/src/composition/sequence/task/templates.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/lib/src/harness/shell.rs
    - claudine/lib/tests/l1/agent_errors_fleet.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/approval.rs
    - darkmatter/cli/tests/bin/argv_recorder/main.rs
    - darkmatter/cli/tests/l1/approval_prompt.rs
    - darkmatter/cli/tests/l1/compose_shell_template.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/justfile
    - darkmatter/lib/src/markdown/compose/body_origin.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/target.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/shell_template_discovery.rs
    - darkmatter/lib/tests/l1/shell_template_execution.rs
    - tools/test-toolkit/src/argv_recorder.rs
    - tools/test-toolkit/src/lib.rs
docs_updated_during_phase_4:
    - claudine/docs/dependencies.md
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/sequences.md
    - claudine/docs/topics/pre-flight-checks.md
    - claudine/features/2026-10-23-shell-template-approval/implementation-log.md
    - claudine/features/2026-10-23-shell-template-approval/plan.md
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/inline/shell-expansion.md
    - tools/test-toolkit/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/architecture.md
    - .claude/skills/claudine/cli-commands.md
    - .claude/skills/darkmatter/compose.md

source_files_during_phase_5: ["claudine/cli/tests/l1/lifecycle_shell_templates.rs", "claudine/cli/tests/l1/sequence_shell_templates.rs", "darkmatter/cli/tests/l1/compose_shell_template.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/site.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/types.rs", "darkmatter/lib/tests/l1/shell_template_execution.rs", "tools/test-toolkit/src/argv_recorder.rs"]
docs_updated_during_phase_5: ["claudine/docs/topics/flow-control/sequences.md", "darkmatter/docs/inline/shell-expansion.md", "tools/test-toolkit/README.md"]
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: [".claude/skills/claudine/SKILL.md"]
source_files_during_phase_6: ["claudine/lib/src/composition/sequence/preflight/reapproval.rs", "claudine/lib/src/composition/sequence/preflight/shape.rs", "claudine/lib/src/composition/sequence/task/mod.rs"]
docs_updated_during_phase_6: ["claudine/README.md", "claudine/docs/topics/pre-flight-checks.md", "claudine/docs/topics/flow-control/sequences.md", "claudine/docs/topics/flow-control/lifecycle.md", "claudine/features/2026-10-23-shell-template-approval/plan.md", "claudine/features/2026-10-23-shell-template-approval/implementation-log.md", "claudine/features/2026-10-23-shell-template-approval/spec.md", "darkmatter/README.md", "darkmatter/docs/inline/preflight-checks.md"]
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: [".claude/skills/claudine/SKILL.md", ".claude/skills/darkmatter/compose.md"]
source_code: ["claudine/cli/Cargo.toml", "claudine/cli/src/commands/compose/prep.rs", "claudine/cli/src/commands/wrap/composition/staged_boot.rs", "claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/shell_approval.rs", "claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs", "claudine/cli/src/commands/wrap/harness_orch/shell_options.rs", "claudine/cli/src/commands/wrap/sequence/iterate.rs", "claudine/cli/src/commands/wrap/sequence/jit.rs", "claudine/cli/src/commands/wrap/sequence/jit/tests.rs", "claudine/cli/src/commands/wrap/sequence/mod.rs", "claudine/cli/src/commands/wrap/sequence/phase1c.rs", "claudine/cli/src/commands/wrap/sequence/task_run.rs", "claudine/cli/src/commands/wrap/sequence/task_run/tests.rs", "claudine/cli/tests/bin/argv_recorder/main.rs", "claudine/cli/tests/bin/fake_goose/main.rs", "claudine/cli/tests/l1/composition_seams.rs", "claudine/cli/tests/l1/document_environment_routes.rs", "claudine/cli/tests/l1/level1_dry_run_pty.rs", "claudine/cli/tests/l1/lifecycle_shell_templates.rs", "claudine/cli/tests/l1/main.rs", "claudine/cli/tests/l1/sequence_cli.rs", "claudine/cli/tests/l1/sequence_errors_cli.rs", "claudine/cli/tests/l1/sequence_shell_templates.rs", "claudine/cli/tests/l1/shell_approval_prompt.rs", "claudine/cli/tests/l1/shell_missing_values.rs", "claudine/lib/src/composition/coordinator/tests.rs", "claudine/lib/src/composition/error/mod.rs", "claudine/lib/src/composition/error/tests.rs", "claudine/lib/src/composition/lifecycle/action_shape.rs", "claudine/lib/src/composition/lifecycle/actions.rs", "claudine/lib/src/composition/lifecycle/bindings.rs", "claudine/lib/src/composition/lifecycle/executor.rs", "claudine/lib/src/composition/lifecycle/executor/tests/mod.rs", "claudine/lib/src/composition/lifecycle/executor/tests/set_shell_values.rs", "claudine/lib/src/composition/lifecycle/executor/tests/shell_templates.rs", "claudine/lib/src/composition/lifecycle/mod.rs", "claudine/lib/src/composition/lifecycle/parse.rs", "claudine/lib/src/composition/lifecycle/shell_template.rs", "claudine/lib/src/composition/lifecycle/structured_shell.rs", "claudine/lib/src/composition/lifecycle/validate.rs", "claudine/lib/src/composition/looping/seed.rs", "claudine/lib/src/composition/mod.rs", "claudine/lib/src/composition/preflight.rs", "claudine/lib/src/composition/preflight/tests.rs", "claudine/lib/src/composition/prepare.rs", "claudine/lib/src/composition/prepare/bootstrap/tests.rs", "claudine/lib/src/composition/prepare/service/tests.rs", "claudine/lib/src/composition/prepare/tests.rs", "claudine/lib/src/composition/schema/tests.rs", "claudine/lib/src/composition/sequence/preflight/mod.rs", "claudine/lib/src/composition/sequence/preflight/reapproval.rs", "claudine/lib/src/composition/sequence/preflight/shape.rs", "claudine/lib/src/composition/sequence/preflight/tests.rs", "claudine/lib/src/composition/sequence/task/mod.rs", "claudine/lib/src/composition/sequence/task/shell.rs", "claudine/lib/src/composition/sequence/task/shell/structured_tests.rs", "claudine/lib/src/composition/sequence/task/templates.rs", "claudine/lib/src/composition/sequence/task/tests.rs", "claudine/lib/src/composition/types.rs", "claudine/lib/src/harness/audit.rs", "claudine/lib/src/harness/error.rs", "claudine/lib/src/harness/shell.rs", "claudine/lib/src/invocation_context/document_tests.rs", "claudine/lib/tests/l1/agent_errors_fleet.rs", "darkmatter/cli/Cargo.toml", "darkmatter/cli/src/approval.rs", "darkmatter/cli/tests/bin/argv_recorder/main.rs", "darkmatter/cli/tests/l1/approval_prompt.rs", "darkmatter/cli/tests/l1/compose_shell.rs", "darkmatter/cli/tests/l1/compose_shell_template.rs", "darkmatter/cli/tests/l1/main.rs", "darkmatter/justfile", "darkmatter/lib/src/markdown/compose/body_origin.rs", "darkmatter/lib/src/markdown/compose/context/options.rs", "darkmatter/lib/src/markdown/compose/expression/binding.rs", "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs", "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs", "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs", "darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs", "darkmatter/lib/src/markdown/compose/inline/interpolation.rs", "darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs", "darkmatter/lib/src/markdown/compose/mod.rs", "darkmatter/lib/src/markdown/compose/nested.rs", "darkmatter/lib/src/markdown/compose/pipeline/mod.rs", "darkmatter/lib/src/markdown/compose/pipeline/phases.rs", "darkmatter/lib/src/markdown/compose/preflight/approval.rs", "darkmatter/lib/src/markdown/compose/preflight/collect.rs", "darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs", "darkmatter/lib/src/markdown/compose/preflight/mod.rs", "darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/policy.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/site.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/store.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/approval.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/fill.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/grammar.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/matching.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/target.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs", "darkmatter/lib/src/markdown/compose/shell_expansion/types.rs", "darkmatter/lib/src/markdown/compose/tests/schema.rs", "darkmatter/lib/src/markdown/schemas/mod.rs", "darkmatter/lib/src/markdown/schemas/simplified/mod.rs", "darkmatter/lib/src/markdown/schemas/simplified/subtype.rs", "darkmatter/lib/src/markdown/types.rs", "darkmatter/lib/tests/l1/data_origin.rs", "darkmatter/lib/tests/l1/main.rs", "darkmatter/lib/tests/l1/preflight_repository_sigils.rs", "darkmatter/lib/tests/l1/shell_result_values.rs", "darkmatter/lib/tests/l1/shell_template_discovery.rs", "darkmatter/lib/tests/l1/shell_template_execution.rs", "darkmatter/lib/tests/l1/shell_template_policy.rs", "darkmatter/lib/tests/l1/transcluded_shell_failure.rs", "tools/test-toolkit/src/argv_recorder.rs", "tools/test-toolkit/src/lib.rs"]
documentation: ["claudine/README.md", "claudine/docs/dependencies.md", "claudine/docs/topics/composition.md", "claudine/docs/topics/flow-control/lifecycle.md", "claudine/docs/topics/flow-control/sequences.md", "claudine/docs/topics/pre-flight-checks.md", "claudine/features/2026-10-23-shell-template-approval/implementation-log.md", "claudine/features/2026-10-23-shell-template-approval/plan.md", "claudine/features/2026-10-23-shell-template-approval/spec.md", "darkmatter/README.md", "darkmatter/docs/dependencies.md", "darkmatter/docs/inline/preflight-checks.md", "darkmatter/docs/inline/shell-expansion.md", "tools/test-toolkit/README.md"]
completed_phase: 6
implemented: true
---

# Implementation Log for 2026-10-23-shell-template-approval (6 phases)

## Phase 1

- 2026-10-14: started Phase 1 (contract review, no source changes planned).

### Dependency Gate — `2026-10-16-core-schema-types`

- The dependency is complete: all 17 plan checkboxes are checked and its log ends "implementation complete, ready for review". Its plan frontmatter still says `phase: 4`; that is bookkeeping drift in the dependency, not missing behavior.
- **The covers relation** is `darkmatter::markdown::schemas::is_subtype(a: &PropertyAtom, b: &PropertyAtom) -> bool`, defined in `darkmatter/lib/src/markdown/schemas/simplified/subtype.rs:40`. It answers "is `a` a subset of `b`?", so "the rule type covers the declared type" is `is_subtype(&declared, &rule)`.
    - Both atoms must already be resolved. An unresolved `TypeExpr::Imported` equals nothing.
    - It ignores ordinary constraints, presence markers, and descriptions.
- **Verified:** `cargo nextest run -p darkmatter -E 'test(/subtype/)'` ran 7 tests and all 7 passed. These are the 6 unit tests in `subtype.rs` and the L1 test `tests/l1/schemas_core_type_values.rs::subtype_relation_is_reachable_through_the_public_schema_api`.
- **Departure the contract must handle:** `is_subtype` returns true for any two identical types (`identical_types_are_subtypes_whatever_the_type`). Spec §7 instead says "unsupported pairs are refused even when identical", and that unions, containers, and `unknown` are never string-like.
    - Template matching therefore gates `is_subtype` behind its own check that the *rule* atom is a supported string-like, non-array, single atom.
    - The broad `string` rule is a separate check, not `is_subtype` (§7 calls it "a shell-template matching exception").
    - This adds no second subtype relation; it only restricts which inputs reach the dependency's relation.
- **Supporting API from the dependency and from Darkmatter**
    - Parse a type: `simplified::grammar::parse_type_expr(property, text) -> Result<PropertyAtom, SchemaError>`. It parses constraints after `;` and rejects options written after `;`, mixed options, unknown options, and repeated options.
    - Print a type: `serialize_property_atom(&PropertyAtom) -> String`. This is the canonical rule text once constraints and descriptions are stripped.
    - Read canonical options: `PropertyAtom::alphanumeric_options()` returns them, and `AlphanumericOptions::canonical()` drops redundant blocks.
    - Look up a declaration: `EffectiveSchema.simplified: Option<SimplifiedSchema>`, then `Single(shape).properties.get(name)`, which gives `PropertyDef::{Single, Union}`. Root unions and layered schemas go through `EffectiveSchema::property_shape` or `property_def_at_path`.
    - Validate one value against one atom: `schemas::completion::alternative_accepts(&PropertyAtom, &Value)`. It is documented as passive. For the original-value check, Phase 2/4 must confirm it enforces constraints (`min`, `pattern`) and does not coerce. If it does not, they must use the `EffectiveSchema` validator on a one-property document instead. **Open for Phase 2**, not a product question.
    - Convert a scalar to text: `compose::expression::scalar_string(&Value)`, the existing conversion engine. Null becomes `""`, and arrays/objects become JSON text. Template mode must therefore reject null, arrays, and objects *before* calling it.

### Sequence and group surfaces today (Dependency Gate, second half)

`2026-10-24-sequence-containers` is planned but unimplemented: its plan says `phase: 1`, it has no log, and it has no code. Today's names are therefore the ones this feature uses (plan rule 3).

| Spec name | Spelling today | Exists today? |
| --- | --- | --- |
| `seq.outputs` (late root) | `outputs` (`OUTPUTS_KEY`, `runtime_state.rs:43`; `ROOT_OVERLAY_KEYS`, `sequence/reserved.rs:31`) | yes |
| `seq.*` other fields (approval-time) | `state.*`, `previous`, `next`, `sequence_id` | yes |
| `loop.*` (late root) | `loop.{count,is_first,is_last,last_output,last_exit_code}` (`composition/reserved.rs:4`) | in documents yes; task/group `loop:` no |
| `group.*` (late root) | `group.<var>` (`GROUP_SCOPE_KEY`, `task/group.rs:83`); `legs`/`succeeded`/`failed`/`cancelled` do not exist | groups yes (serial or parallel, not nested, no group loop, no group lifecycle events) |
| nested sequence | rejected (`SequenceNestedSequence`, `sequence/preflight/mod.rs:779`) | **no** |
| container terminal event (`finalize`) | none | **no** |

**Deferred acceptance cases.** These depend on a surface that does not exist yet. They are recorded here rather than claimed. Phase 5 must list them as "not applicable until the container surfaces land".

- AC16 "when the first task is a nested sequence, only that sequence's own first task" gets resolved-value approval. Nested sequences are rejected today. Phase 3 should still build the position rule recursively, so that the containers feature only has to add a traversal.
- AC16 "a first group's `finalize` command is a template". Groups have no lifecycle events today. The rule "a container terminal event is always a template" is part of the contract, but there is no surface to test it on.
- AC2, AC9, and the §10 examples write `seq.outputs`. Tests use the current spelling `outputs` (for example `last(outputs)`). Whether the tests should be renamed is left to the containers feature.

These can all be tested today: first task as an action (with its stacks and its document), first task as a group (every leg), any `group.*` read, a skipped predecessor (position stays static), retry and resume, and a later group's legs.

### Boundary Inventory

The full inventory was gathered read-only, by tracing the code. Below is what each later phase owns, with file:line anchors.

#### Darkmatter (`darkmatter/lib/src/markdown/compose/`)

- **Discovery produces bytes only.** `ShellCommandEntry` (`shell_expansion/types.rs:277`) holds `{raw_command, executable, args, normalized, source_file, origin}`. It has no span, no owner schema, no authored template, and no record of which expressions it read.
    - `collect_recursive` (`preflight/collect.rs:304`) first composes the body with `Interpolation` enabled and then tokenizes the **already-interpolated** text. `scan_one_frontmatter` (`:852`) does the same.
    - Entries are de-duplicated graph-wide by `normalized` (`seen: HashSet<String>`, `collect.rs:183`), so the first origin wins. **Template mode needs per-site retention**: two sites with the same bytes can have different owner schemas and types.
- **Interpolation runs before tokenizing on every surface.** The authored shape is recovered only by masking:
    - frontmatter uses `mask_interpolations` (`frontmatter_shell_expansion.rs:1008`), which produces `__DM_INTERP_n__`;
    - the body and `::shell-block` use `DataRanges::masked` (`body_origin.rs:186/252`);
    - shape checks are `data_changed_shape` (`shell_expansion/parser.rs:183`), `reject_data_shaped_commands` (`shell_blocks/mod.rs:241`), `validate_pipeline_shape_preserved` (`frontmatter_shell_expansion.rs:1042`), and `validate_no_executable_interpolation` (`:1118`).

  These masks are the existing seam that Phase 2's template grammar replaces with real placeholder nodes.
- **The tokenizer** is `tokenize` (`shell_expansion/tokenize.rs:75`), which yields `ShellToken::{Word, And, Or, 4 curated redirects}`, followed by `parse_pipeline` (`:399`), which yields `ShellPipeline`. `{`/`}` are ordinary word characters.
- **Null handling today:** null becomes `""` through `scalar_string` (`expression/mod.rs:395`). After tokenizing, `ls {{ x }}` with `x = null` becomes `ls` with **no argument at all**. That is the §1 defect.
- **Byte-only approval keys**
    - `ComposeOptions.pre_approved_commands: Option<HashSet<String>>` (`context/options.rs:331`, setter `:1445`, identity hash `:2917`).
    - Where it is checked: `prepare_directive` (`shell_expansion/mod.rs:244-264`, error `NotPreApproved`) and `validate_pre_approved` (`preflight/mod.rs:295`).
    - `ComposePreflightApprovals.pre_approved_commands: HashSet<String>` (`preflight/lifecycle.rs:60`); `approval_set()` (`preflight/approval.rs:18`).
    - `SharedShellExpansionRuntime` (`types.rs:1118`): `allow_once` and `pending_allow_once` (`HashSet<String>`), plus `reserve_allow_once` / `complete_allow_once` (`:1261/:1320`) and `ReservationGuard` (`shell_expansion/mod.rs:67`).
- **The result cache** is separate and also uses a string key: `cache_key` (`shell_expansion/mod.rs:483`). The key is the per-action `normalize_command`, joined with the real `&&`/`||` and the redirection text, plus cwd, strip_ansi, timeout, and timeout behavior. It is read through `cached_outcome` (`types.rs:1394`).
    - **Gap for template mode:** `normalize_command` (`policy.rs:279`) escapes `\` and `"` inside quoted arguments. However, it does not quote an empty argument: `[""]` renders as a trailing space. Template mode makes empty arguments routine (spec §4). The template-mode result key is therefore built from explicit argument boundaries, never from a rendered string. (An inventory claim that `normalize_command` fails to escape `"` was checked against the source and is wrong.)
- **Display drift:** `normalized_exact` in the prompt request joins a chain with `" && "` even when the operator is `||` (`shell_expansion/mod.rs:380`). Template shapes must carry the real operators.
- **Executor** (`shell_expansion/executor.rs`):
    - It already dispatches directly: `run_actions` (`:283`) short-circuits `&&`/`||` in Rust, and `run_action` (`:414`) uses `Command::new(resolved).args(args)` with `which`/`which_in` and no `sh -c`.
    - `configure_streams` (`:614`) maps the curated redirects.
    - It does **not** refuse `.cmd`/`.bat` targets today. On Windows, Rust's `Command` routes `.bat`/`.cmd` through `cmd.exe` implicitly. Phase 4 adds the refusal for template mode only.
- **Suppression boundaries**
    - `handle_errors` (`shell_expansion/mod.rs:669`), which applies `ErrorHandling::resolve` to `ExecutionFailed` only.
    - Timeout `EmptyString` (`:646`).
    - Transclusion notices in `pipeline/phases.rs` (`fit_notice_to_span` `:515`, guarded by `transclusion_failure_is_fatal` `:~536`, which already treats `pre_approval_violation`, `unavailable_binding`, and `shell_span_failure` as fatal).
    - Nested `as_markdown` (`nested.rs:248` `is_structural`), where an ordinary shell failure can be absorbed by `||`.

  The new typed template errors must be added to `transclusion_failure_is_fatal` and `is_structural`, and must not be `ExecutionFailed`.
- **Policy store** (`shell_expansion/store.rs`): `load_ruleset` (`:85`) accepts `exact`/`prefix` and skips malformed lines with `tracing::warn!`. It reports **no line number**; the spec requires file and line. It de-duplicates by sorting. Invalid UTF-8 already becomes `PolicyIo`. All appends go through `append_rule` (`:225`).
- **Discovery seams:** `BindingView` (`expression/binding.rs:247`), `ComposeOptions::with_host_globals` (`context/options.rs:1949`), `RESERVED_ROOTS` (`expression/catalog/roots.rs:62`), and the existing refusals `DynamicCommandShape` / `UnevaluatedDependencyShape` (`collect.rs:1008/1375/1551`), pending-shell inheritance (`collect.rs:1051-1137`), and `detect_authored_dynamic_target` (`:1155`).
- **The `md` CLI prompt** is in `darkmatter/cli/src/approval.rs`: `CliShellApprovalHandler`, `approve_with_io` (`:29`), `write_prompt` (`:72`), and `can_prompt_interactively` (`:190`).
    - **Pre-existing defect:** `Markdown::compose_preflight_approvals` (`preflight/lifecycle.rs:166-171`), which is the only path `md compose` uses, treats `AllowExactPersist`/`AllowCommandPersist` like `AllowOnce` and **persists nothing**.
    - AC8 requires every choice to persist its §8 row in both prompts, so Phase 4 (Approval Prompts) must route this path through the store.

#### Claudine (`claudine/lib/src/`, `claudine/cli/src/`)

- **Byte-only sets**
    - `PreFlightResult.approved_commands: HashSet<String>` (`composition/preflight.rs:31`), built by `approve_discovered_commands` (`:198`) through `harness::shell::validate_and_approve_command_parts` (`harness/shell.rs:141`).
    - `PrepareOptions.pre_approved_commands` (`composition/prepare.rs:103`) and `CallerInputLayers.pre_approved_commands` (`types.rs:525`, with `add_approved_commands` `:632`).
    - `SharedApprovalCache = HashMap<String, CachedApprovalDecision>` (`types.rs:23`).
    - Handed to Darkmatter at `prepare.rs:427-428`; bootstrap is empty at `:938`.
    - Lifecycle `set` values: `ResolvedShellValue::commands()` goes into a fresh `ComposeOptions` (`lifecycle/executor.rs:357-364`).
- **CLI flows**
    - compose/wrap: `compose/prep.rs:844, 1524, 1535`; staged boot `wrap/composition/staged_boot.rs:85-88, 334-351`; harness lifecycle `wrap/composition/runner.rs:245`.
    - sequence: `wrap/sequence/mod.rs:65` (`approve_preflight_graph`), seeding `phase1c.rs` and then `SequenceRunContext.approved` (`mod.rs:715`). `jit.rs:167/212-236` extends it per step, and prompt tasks re-enter at `task_run.rs:611`.
- **Re-audit paths**
    - Loop: `compose/prep.rs:1761` (`audit_loop_iteration_template`) and `:1783` `frozen_loop_approvals`.
    - Retry/resume/proxy: `harness_orch/loop_control.rs:1386-1395, 1012, 1043, 1766` (freeze) and `prompt.rs:265, 286, 420-441`; thaw in `harness_orch/shell_options.rs:204-237`.
- **Sequence preflight** (`composition/sequence/preflight/`)
    - `Loader::walk_plan` (`mod.rs:300`) and `load_task` (`:389`), with groups at `load_group`/`build_group` (`:533/:592`) and external tasks at `load_external_task` (`:496`).
    - Shell bytes come from `resolve_shell_bytes` (`:886-923`). It raises `SequenceShellLateBinding` (`composition/error/mod.rs:1868`) for the roots declared `Unavailable(PREFLIGHT_UNAVAILABLE)` in `lifecycle/bindings.rs:181-199` (`outputs`, `err`, `timing`, `group`, `loop`); `current` is refused through `LATE_NAMESPACES` (`bindings.rs:79, 430-439`).
    - **Gap:** task-stack whole-value `set $( … )` is not collected by `collect_lifecycle_shell` (`:857`, `shape::collect_stack_shell_commands` `shape.rs:197`). Today it fails at run time ("pre-flight never resolved", `lifecycle/executor.rs:1556-1566`). AC16 requires `set` shell values in later tasks, so Phase 3 (Sequence Position) collects them.
    - **Static position is not tracked.** `PreflightStep.index` (`shape.rs:18`) and member indexes exist but are not threaded into approval. Phase 4 adds this.
- **Runners**
    - Sequence `TaskShellRunner` / `SystemTaskShell` (`sequence/task/shell.rs:222/279`) runs `sh -c` or `cmd /D /C`. It uses a Unix process group or a Windows kill-on-close Job Object, has a 30 s default deadline, honors interrupts, and has volume caps.
        - The structured template path **must reuse this process-tree machinery** (`:812-857, :918-941`) and change only how argv is built.
    - Lifecycle `ShellRunner` (`lifecycle/executor.rs:295`): `SystemShellRunner`/`DocumentShellRunner` go through `run_system_shell` (`:452`), which inherits stdio and has **no deadline, no process tree, and no interrupt check**.
        - "Retain its contracts" therefore means the structured lifecycle path keeps exactly that: inherited stdio, the same cwd and environment, no new deadline.
        - Adding deadlines to lifecycle shells is out of scope.
    - `DisabledShellRunner` (`:384`) must refuse the new structured method too.
- **Approval handler:** `validate_and_approve_command_parts` (`harness/shell.rs:141-351`) checks the built-in blacklist, then the user blacklist, the whitelist, the session cache, and finally the handler. It is installed in `cli/.../harness_orch/shell_options.rs:96-155` (`CliShellApprovalHandler` when there is a TTY, `YoloApprovalHandler` for `--yolo`, otherwise none). With no TTY the result is `ShellApprovalUnavailable{NoHandler|DryRun}` (`composition/preflight.rs:246-259`).
- **Comments that will drift when template mode lands.** Phases 3 and 4 must fix them in the same change that alters the behavior:
    - `sequence/preflight/mod.rs:18-21, 61-63, 850-852, 886-901, 913-917`
    - `sequence/preflight/shape.rs:99-102, 148, 261-267`
    - `sequence/task/shell.rs:3-4`
    - `sequence/task/mod.rs:703-704, 934-938, 954-959`
    - `sequence/preflight/reapproval.rs:1-11`
    - `composition/preflight.rs:124-126, 418-423`
    - `composition/prepare.rs:894-898`
    - `composition/error/mod.rs:1861-1867, 1920-1923`
    - `cli/src/commands/wrap/sequence/mod.rs:346-348` and `task_run.rs:281-285`
- **Existing tests to keep green** (AC12 corpus)
    - Darkmatter: `compose/tests/{shell,preflight}.rs`, `frontmatter_shell_expansion/tests/*`, `lib/tests/l1/{shell_*,transcluded_shell_failure,preflight_*,interpolation_literal_pipeline,error_snapshots/shell_expansion}.rs`, `cli/tests/l1/compose_shell.rs`.
    - Claudine: `composition/sequence/{preflight,task,task/shell}/tests.rs`, `composition/preflight/tests.rs`, `lifecycle/{bindings,executor}/tests*`, `cli/tests/l1/{sequence_cli,sequence_jit,sequence_groups,sequence_errors_cli,lifecycle_set_shell_values,wrap_compose_validation}.rs`.

### Contract Definition (Wave 2)

This is the shared design the later waves build against. The names are binding for the shared API. Private helpers may differ. Anything not stated here is left to the implementing wave, which records its choice in this log.

#### C1. Module placement and single owner

- All shared template types live in one new module, **`darkmatter/lib/src/markdown/compose/shell_expansion/template/`**, exported as `darkmatter::markdown::compose::shell_expansion::template`.
    - `mod.rs` holds the types and exports.
    - `grammar.rs` holds the authored parser.
    - `canonical.rs` holds the rule text encoder and decoder.
    - `typing.rs` holds the type projection.
    - `matching.rs` holds the whitelist and blacklist matching.
    - `fill.rs` holds the resolver, validator, and filled chain.
    - `target.rs` holds direct-dispatch executable resolution.
- **Owners**
    - Phase 2 "Template Grammar" owns `template/mod.rs` and the `shell_expansion/types.rs` additions: the `ShellRuleEntry::Template`, `ShellApprovalDecision::AllowTemplatePersist`, and `ShellExpansionError` variants.
    - Every later task consumes these and does not redefine them. A needed change to a shared type goes into the log first, then into `mod.rs`.

#### C2. Authored template (Phase 2)

```rust
pub struct CommandTemplate { pub authored: String, pub segments: Vec<TemplateSegment> }
pub struct TemplateSegment {
    pub operator: ChainOperator,          // existing type; None for the first
    pub program: String,                  // always literal; a hole here is a grammar error
    pub args: Vec<TemplateArg>,
    pub redirects: Vec<RedirectOp>,       // authored order retained
}
pub struct TemplateArg { pub parts: Vec<ArgPart> }  // `""` is one empty Literal part, never zero parts
pub enum ArgPart { Literal(String), Hole(Placeholder) }
pub struct Placeholder {
    pub expression: String,               // authored text inside `{{ }}`, trimmed
    pub span: std::ops::Range<usize>,     // byte span in `authored`, for diagnostics
    pub timing: HoleTiming,               // filled by discovery (C5)
    pub typing: HoleType,                 // filled by type projection (C3)
}
pub enum RedirectOp { StdoutNull, StderrNull, StderrToStdout, StdoutToStderr }
pub enum TemplateGrammarError { ProgramWordHole, RedirectHole, Pipe, Semicolon, Background,
    InputRedirect, ArbitraryRedirect, CommandSubstitution, Unterminated, ControlCharacter, Empty }
```

- `parse_authored(text) -> Result<CommandTemplate, TemplateGrammarError>` recognizes each complete `{{ … }}` span **before** it interprets quotes, whitespace, or operators.
    - It reuses the existing tokenizer's rules for quoting and for the four curated redirects.
    - `RedirectOp` lowers to the existing `RedirectionConfig` with the same dup semantics `parse_pipeline` uses. Execution is therefore unchanged.
- Resolved-byte mode keeps the existing tokenizer and its grammar exactly as they are.

#### C3. Typing (Phase 2 "Type Projection")

```rust
pub struct HoleType { pub approval: ApprovalType, pub validator: HoleValidator, pub provenance: TypeProvenance }
pub enum ApprovalType { BroadString, StringLike(PropertyAtom) } // atom: resolved, non-array, constraints+description stripped, canonical options
pub enum HoleValidator { FallbackScalar, Declared(PropertyDef) }  // Declared keeps constraints, unions, named types resolved
pub enum TypeProvenance { BareProperty { name: String }, NoProjection, Undeclared, NotBareProperty, ReservedRoot }
```

- **Supported string-like** means a single, non-array atom for which the dependency defines a string-like subset.
    - Every other declaration (numeric, boolean, union, container, `unknown`) gets `ApprovalType::BroadString` while keeping `HoleValidator::Declared`. That is the disclosure case.
    - The prompt then shows `{{ count: string (validated as number) }}`.
- **Covers:** `rule_covers(rule: &ApprovalType, declared: &ApprovalType) -> bool`.
    - A `BroadString` rule covers everything. This is the §7 exception, and execution still rejects null, arrays, and objects.
    - A `StringLike(r)` rule covers only a `StringLike(d)` declaration with `is_subtype(&d, &r)`.
    - A `StringLike` rule never covers a `BroadString` declaration.
    - The rule atom is checked to be a supported string-like type before `is_subtype` is called. This handles the departure recorded in the Dependency Gate.

#### C4. Canonical shape and rule text (Phase 2 "Policy Persistence")

```rust
pub struct TemplateShape { pub segments: Vec<ShapeSegment> }        // CommandTemplate with holes reduced to ApprovalType
impl TemplateShape {
    pub fn from_template(&CommandTemplate) -> Self;
    pub fn canonical_text(&self) -> String;                          // `template ` payload
    pub fn parse_rule(text: &str) -> Result<Self, TemplateRuleError>; // file+line added by the loader
}
pub enum ShellRuleEntry { Exact(String), Prefix(String), Template(TemplateShape) }
```

- **Reconciling the spec's text with AC11.**
    - §3 says canonical text "double-quotes literal argument segments". AC11 and every §3/§10 example persist bare words, such as `template just clean --out {{ string }}/build`.
    - Contract: the **writer** emits a literal run bare when it holds only characters that need no escaping. Otherwise it double-quotes the run using the §3 escapes (`\\`, `\"`, `\{`, `\}`, `\n`, `\r`, `\t`). Any run that contains a brace, whitespace, quote, backslash, operator character (`& | ; < > $` and backtick), or control character is quoted, and so is an empty literal argument (`""`).
    - The **reader** accepts both forms. De-duplication compares parsed `TemplateShape`, never text.
    - This satisfies AC11's literal line and §3's escaping rules together.
    - This is an interpretation, not a new ruling. It is recorded here per plan rule 2.
- **Rule placeholder text** is `{{ string }}` or `{{ <serialize_property_atom(atom)> }}`.
    - The loader rejects the following, each with file and line: constraints (with or without `;`), descriptions, unresolved or named types, arrays, unions, unsupported types, unknown options, and program-word holes. A rejected line is skipped; it does not invalidate the other lines.

#### C5. Discovery output, timing, and missing values (Phase 3 "Discovery Provenance")

```rust
pub enum HoleTiming { ApprovalTime, Deferred { roots: Vec<String> } }
pub struct ShellCandidate {
    pub site: CommandSite,                 // source file, ShellCommandOrigin, owner (document source key), stable site id
    pub template: Result<CommandTemplate, TemplateGrammarError>,
    pub resolved: Option<ResolvedCommand>, // today's normalized/executable/args/chain_executables; None when any hole is Deferred
    pub requirement: ModeRequirement,      // AnyMode | TemplateRequired (set by host position or Deferred holes)
}
```

- **Deferral is declared by the host** through a new `ComposeOptions::with_deferred_shell_roots(DeferredShellRoots)`. Claudine passes `outputs`, `loop`, and `group` for sequence approval.
    - This is separate from `BindingView`, and it does not make a root lexically available.
    - Darkmatter tracks deferral transitively through derived frontmatter and task parameters by dependency analysis, without evaluating anything.
    - A compound expression that touches a forbidden root (`err`, `timing`, `current.*`) keeps today's refusal.
- Discovery keeps **one candidate per site**, no longer de-duplicated by `normalized`. Approval prompting may still group identical keys so the user is asked once.
- **Missing values** raise one typed `ShellExpansionError::MissingValue { ctx, command, expression, origin }`. The command is the authored text; the value never appears.
    - It is raised at preflight for every approval-time hole that evaluates to null.
    - It is raised at the step barrier or at command entry for deferred holes.
    - An empty string, `0`, `false`, or a fallback that yields a present value are not missing.

#### C6. Approval transport and keys (Phase 3 "Approval Transport")

```rust
pub enum ApprovalKey {
    Exact { policy_root: PathBuf, normalized: String },
    Template { policy_root: PathBuf, shape: String /* canonical_text incl. ApprovalTypes */ },
}
pub struct ShellPreApprovals { exact: HashSet<String>, templates: HashSet<String /* canonical shape */> }
pub enum ShellApprovalDecision { AllowExactPersist, AllowCommandPersist, AllowTemplatePersist, AllowOnce, Deny, BlacklistPersist }
pub enum ApprovedMode { ResolvedBytes, Template }  // what AllowOnce means is fixed by the request's offered mode
```

- `ComposeOptions.pre_approved_commands` is **replaced** by `Option<ShellPreApprovals>`. Claudine's `PreFlightResult`, `PrepareOptions`, `CallerInputLayers`, `SharedApprovalCache`, `ResolvedShellValue`, and the sequence task records carry the same type. The old `HashSet<String>` field is removed, not kept alongside.
- **Membership at execution**
    1. If the site is `AnyMode` and its freshly resolved bytes are in `exact`, run the bytes.
    2. Otherwise, re-parse the **authored** text, project it against the owner schema, and require its canonical shape to be in `templates`. Then run in template mode.
    3. Otherwise, fail with `NotPreApproved`.
- A `TemplateRequired` site never consults `exact`.
- Freshly resolved bytes are never inserted into either set.
- Prefix and once decisions for a template candidate record its shape in `templates`.
- The `allow_once` and `pending_allow_once` sets in `SharedShellExpansionRuntime` become `HashSet<ApprovalKey>`. The existing reservation, condvar, and failure-release semantics stay unchanged.
- A failed policy write releases the reservation and returns `PolicyIo`. It caches no successful decision and is never downgraded to once.
- **The prompt request** (`ShellApprovalRequest`) gains:
    - `offered: OfferedChoices { exact, shape, prefix }`,
    - `template: Option<TemplateShape>` plus a display form (`{{ expr: type }}`, with disclosure), and
    - `resolved: Option<…>`.
  An unavailable value is `None`, never `""`.
- **Eligibility**
    - Exact is offered only when the site is `AnyMode` and resolved.
    - Shape is offered when there is at least one hole and the grammar is valid.
    - Prefix is offered when the program words are fixed. A template-required site must also pass the grammar check.
    - Once, deny, and blacklist are offered after the candidate validates.
    - A template-required site with a grammar error fails with a typed `TemplateGrammar` error before any prompt.

#### C7. Fill, validation, and dispatch (Phase 4)

```rust
pub struct FilledSegment { pub operator: ChainOperator, pub program: String, pub args: Vec<String>, pub redirects: Vec<RedirectOp> }
pub struct FilledChain { pub segments: Vec<FilledSegment> }
pub struct LateValues(/* site id + hole index -> validated, converted text */);
pub fn fill_template(t: &CommandTemplate, eval: &dyn HoleEvaluator, late: Option<&LateValues>) -> Result<FilledChain, ShellExpansionError>;
pub fn resolve_late_values(sites: &[&ShellCandidate], eval: &dyn HoleEvaluator) -> Result<LateValues, ShellExpansionError>;
pub fn check_filled_chain(chain: &FilledChain, user_blacklist: &ShellRuleSet) -> Result<(), ShellExpansionError>; // builtin then user, every segment
pub fn resolve_direct_target(program: &str, path: Option<&OsStr>, cwd: &Path) -> Result<PathBuf, ShellExpansionError>;
```

- **Order of operations**
    1. Evaluate each hole once.
    2. Reject null (`MissingValue`), arrays and objects, and embedded NUL.
    3. Validate the **original** value against `HoleValidator`.
    4. Convert it with `scalar_string`.
    5. Concatenate the parts into one argument.
    6. After every segment is filled, call `check_filled_chain`.
    7. Only then may a result-cache lookup or a spawn happen.
- **Errors** are new typed `ShellExpansionError` variants: `MissingValue`, `TemplateValueRejected { placeholder, declared_type, origin }` (never the value), `TemplateGrammar`, and `TemplateTarget`.
    - None of them is `ExecutionFailed`, so `handle_errors` / `when_error` cannot absorb them.
    - They are added to `transclusion_failure_is_fatal` and to the nested `is_structural`.
    - Claudine wraps them while keeping the source chain.
- **The step barrier** is owned by Claudine.
    - Before a step's first shell effect, Claudine calls `resolve_late_values` over every template site of that step whose holes are `Deferred`. That covers task `shell:`, the stacks, `set $( … )`, and the step document's commands, including commands on conditional branches.
    - Only deferred holes are kept, in `LateValues`, which is passed into composition (`ComposeOptions::with_late_values`) and into the runners.
    - Approval-time holes are still evaluated at command entry against current state. A lifecycle `set` uses its pre-write snapshot.
    - Mutable event-time values are therefore not frozen, and no placeholder is evaluated twice.
- **Result-cache key in template mode** is a length-prefixed encoding of the mode tag, then for each segment the operator, program, argument count, every argument with its length, and the ordered redirects, followed by the existing context (cwd, strip_ansi, timeout, behavior). No hashing is needed, so `biscuit-hash` is not involved. The key is kept separate from `ApprovalKey`.
- **Direct-target rule.** This is an interpretation of §4 "reject a target that needs an interpreter", recorded per plan rule 2.
    - On Windows, only resolved `.exe`/`.com` targets are accepted. `.cmd`, `.bat`, and any other `PATHEXT` hit are refused, because Rust would route them through `cmd.exe`, which re-parses arguments.
    - On Unix, a resolved file must start with a native executable magic number (ELF / Mach-O) or `#!`. A file with neither is refused, because `execvp` would fall back to `/bin/sh`.
    - `#!` scripts are allowed: the kernel passes argv through unchanged, so values are not re-interpreted. This keeps common tools such as `npm`/`pnpm` usable on Unix.
    - Resolution never falls back to a shell. Builtins, which are not on `PATH`, fail as `TemplateTarget`.
- **Runner adapters.** Claudine keeps operational ownership.
    - `TaskShellRunner::run_structured(&FilledChain, timeout, interrupt, live)` reuses `SystemTaskShell`'s process-group and Job Object spawn, its deadline, its interrupt handling, and its volume caps. Only argv construction changes. The `&&`/`||` short-circuit is evaluated in Rust after whole-chain validation.
    - `ShellRunner::run_structured(&FilledChain) -> Result<i32, ShellRunError>` keeps the lifecycle runner's current contract: inherited stdio, same cwd and environment, and no new deadline.
    - `DisabledShellRunner` refuses `run_structured`.
- **Native recording executable:** none exists today.
    - Phase 4 "Shared Validation" adds the recorder body as a module in `tools/test-toolkit`, with one thin `[[bin]]` per consuming package. Darkmatter and Claudine each get their own, such as `claudine/cli/tests/bin/argv_recorder/main.rs`, following the existing `claudine-fake-*` pattern.
    - Tests locate the binary at runtime through `CARGO_BIN_EXE_*` or the toolkit's lookup. The recorder writes its argv as length-prefixed records to a path taken from an environment variable.

#### C8. Sequence position (Phase 4 "Sequence Position")

- `TaskPosition::{First, Later}` is computed statically in `Loader::walk_plan` and `load_task` and stored on every prepared site.
    - Step 0's task is `First`.
    - Each leg of a group in step 0 is `First`.
    - A nested sequence would be `First` only through its own first child. This is written recursively, though there is no surface to exercise it today.
    - Everything else is `Later`.
- A `Later` site, any site reading `group.*`, and any container terminal-event site are `TemplateRequired`.
- Skips, retries, and resume never change position.
- The `SequenceShellLateBinding` refusal is removed only for `outputs`, `loop`, and `group`. It stays for `err`, `timing`, and `current.*`.

#### C9. Acceptance-to-task map

| AC | Primary task (phase) | Supporting |
| --- | --- | --- |
| 1 Missing, single doc | Discovery Provenance (P3) | Shared Validation (P4), Execution Corpus (P5) |
| 2 Missing, sequence | Sequence Position (P4) | Orchestration Corpus (P5) |
| 3 Store round trip | Policy Persistence (P2) | — |
| 4 Argument-list matching | Policy Matching (P2) | Template Grammar (P2) |
| 5 Covers table | Type Projection (P2) | — |
| 6 One argument per word | Shared Validation (P4) | Sequence / Lifecycle Dispatch (P4), Package Gates on 4 OSes (P5) |
| 7 Run-time check | Shared Validation (P4) | Execution Corpus (P5) |
| 8 Prompt | Approval Prompts (P4) | Approval Transport (P3); fixes the `md` persist defect |
| 9 Late approval end to end | Sequence Dispatch (P4) | Orchestration Corpus (P5) |
| 10 Loop re-audit | Lifecycle Dispatch (P4) | Approval Transport (P3) |
| 11 First vs later tasks | Sequence Position (P4) | Approval Prompts (P4) |
| 12 Unchanged paths | Package Gates (P5) | every task |
| 13 Runtime policy | Policy Matching (P2), Shared Validation (P4) | Execution Corpus (P5) |
| 14 Grammar and boundaries | Template Grammar (P2), Shared Validation (P4) | Package Gates on 4 OSes (P5) |
| 15 Approval vs results | Approval Transport (P3) | Execution Corpus (P5) |
| 16 State and discovery | Sequence Position (P4), Discovery Provenance (P3) | Orchestration Corpus (P5); nested-sequence and container-`finalize` cases deferred (see surfaces) |
| 17 Schema provenance | Type Projection (P2) | Execution Corpus (P5) |

#### C10. File ownership for the parallel waves

| Wave task | Owns |
| --- | --- |
| Template Grammar | `template/{mod,grammar,canonical}.rs`, new variants in `shell_expansion/types.rs` |
| Type Projection | `template/typing.rs` |
| Policy Persistence | `shell_expansion/store.rs` and its tests |
| Policy Matching | `template/matching.rs`, `shell_expansion/policy.rs` |
| Discovery Provenance | `compose/preflight/{collect,mod}.rs`, frontmatter / body / shell-block discovery masks |
| Approval Transport | `preflight/{approval,lifecycle}.rs`, `context/options.rs`, runtime `allow_once` in `types.rs` (after Grammar lands), Claudine `composition/{preflight,prepare,types}.rs`, `harness/shell.rs` decision mapping, CLI handoff and re-audit paths |
| Shared Validation | `template/{fill,target}.rs`, `shell_expansion/{mod,executor}.rs` wiring, `pipeline/phases.rs` + `nested.rs` fatal classification, recorder bins |
| Sequence Position | `composition/sequence/preflight/{mod,shape,reapproval}.rs`, prepared task records, `lifecycle/bindings.rs` |
| Sequence Dispatch | `composition/sequence/task/{shell,mod}.rs`, `cli/.../wrap/sequence/{task_run,jit}.rs` |
| Lifecycle Dispatch | `composition/lifecycle/{executor,preflight}.rs`, `cli/.../compose/prep.rs`, `harness_orch/*` re-audit |
| Approval Prompts | `darkmatter/cli/src/approval.rs`, Claudine handler install (`harness_orch/shell_options.rs`) and decision persistence |

**Checkpoint:**

- Dependency readiness is explicit: `is_subtype` was verified, along with its departure on identical pairs.
- Both execution modes (C6, C7) and both validation barriers (C7) have contracts.
- Every acceptance criterion maps to a task (C9).
- No spike was scheduled.

### Phase 1 verification and close

- Phase 1 is contract and inventory work only. No source, test, documentation, or skill file was changed. The only edits are this log and the plan's checkboxes and frontmatter.
- **Requirement-to-test mapping:** this phase changed no behavior, so it adds no tests. C9 maps every acceptance criterion to the task that will test it.
- **Gates run on macOS:**
    - `just lint` (claudine): clean.
    - `just test` (claudine): 9152 run, 9152 passed, 9 skipped, 0 failed.
    - Dependency check: `cargo nextest run -p darkmatter -E 'test(/subtype/)'` passed 7 of 7.
- **No cross-OS run:** no OS-sensitive code changed, so `just cross-check` was not needed.
- **Interpretations recorded (plan rule 2).** These are not new rulings.
    - C4: canonical rule text writes literal runs bare when safe and quotes them otherwise. This matches AC11's literal line.
    - C7: the direct-target rule allows Unix `#!` scripts and native binaries. It refuses files without a format magic number and Windows `.cmd`/`.bat`.
    - Bounded deviation: lifecycle structured dispatch keeps the lifecycle runner's current contract, which has no deadline and no process tree. It does not invent one.
- **Pre-existing defects found and assigned:**
    - The `md` batch preflight path does not persist persist-decisions. Approval Prompts (P4) fixes it.
    - Sequence task-stack `set $( … )` is never resolved at preflight. Sequence Position (P4) fixes it.
    - The policy loader's warnings carry no line number. Policy Persistence (P2) fixes it.
- Status: Phase 1 complete.

## Phase 2

- 2026-10-14: started Phase 2 (template grammar, type projection, policy persistence, policy matching). All source changes are in Darkmatter; Claudine compiles unchanged against them.

### Template Grammar (Wave 3)

- New module `darkmatter/lib/src/markdown/compose/shell_expansion/template/` (`mod.rs`, `grammar.rs`, `canonical.rs`, `typing.rs`, `matching.rs`, `tests.rs`), exported as `shell_expansion::template`.
- `parse_authored(text)` finds every `{{ … }}` span with `ExpressionFinder::scan_plain`, which is the same scanner interpolation uses, before it reads quotes, whitespace, or operators. A span's own `||`, quotes, and spaces never split or chain the command. A hole stays inside its containing argument, quoted or not. Quoting rules and the four curated redirects mirror `tokenize`.
- Arguments are normalized: adjacent literal runs merge and empty runs drop, so spelling variants have equal parts. `""` stays one empty literal argument; the resolved-byte tokenizer drops it, but template mode keeps it, as spec §4 requires.
- Redirects are kept in authored order (`RedirectOp`). `redirection_config(ops)` lowers them through `apply_redirection`, which was extracted from `tokenize::parse_pipeline`. The two modes therefore share one implementation of the dup semantics, and a test compares the two lowerings on five orderings.
- `TemplateGrammarError` variants: the contract's set plus `InterpolationLiteral` (`{{{ … }}}` and literal tokens are refused in template mode). An unquoted lone `&` is `Background`; the resolved-byte tokenizer still treats it as a literal character.
- **Departures from C1/C2, recorded per plan rule 2:**
    - `Placeholder` has `expression`, `span`, and `typing` but not yet `timing`. `HoleTiming` belongs to Discovery Provenance (Phase 3), which adds the field when it has a producer.
    - `ShellApprovalDecision::AllowTemplatePersist` and the `ShellExpansionError` variants `MissingValue`, `TemplateValueRejected`, `TemplateGrammar`, and `TemplateTarget` are **not** added yet. Each would be an unused variant that every exhaustive match (`prepare_directive`, `preflight/lifecycle.rs`, Claudine `harness/shell.rs`) would have to handle with invented behavior. Approval Transport (P3) and Shared Validation (P4) add them alongside their first producer. The phases are sequential, so the single-owner rule is not at risk.
    - `FilledChain`/`FilledSegment` (C7) are defined now because blacklist matching against filled argv is a Phase 2 task. `FilledSegment` carries the lowered `RedirectionConfig`, not `Vec<RedirectOp>`, so that `FilledChain::from_pipeline` can represent a resolved-byte pipeline (which has no authored order) in the same form.
    - `ApprovalType::StringLike` wraps a `StringLikeType`, not a bare `PropertyAtom`. `PropertyAtom` is only `PartialEq` (it holds `f64` constraints), but `ShellRuleEntry` needs `Eq + Hash`. `StringLikeType` stores the identity atom (boxed) and compares and hashes by its canonical text.

### Type Projection (Wave 4)

- `project_hole(expression, schema: Option<&SimplifiedSchema>, host_globals)` → `HoleType { approval, validator, provenance }`. `CommandTemplate::project_types` applies it to every hole. Only `Expr::Variable` naming a bare top-level property (`dir`, `fm.dir`) reads a declaration. Every other case uses the broad-string fallback with a `FallbackScalar` validator: dotted paths, calls, operators, Darkmatter reserved roots (`fm`, `ctx`, `env`, `current`), host globals (`loop`, `outputs`, …), undeclared names, and no projection. The provenance is recorded in each case.
- A root `SimplifiedSchema::Union` gives a declaration only when every arm that declares the property declares it identically. Otherwise the hole falls back as `NoProjection`. (Spec §2 names only "root union of mixed shapes"; this is the conservative reading for an all-Simplified union whose arms disagree.)
- The approval type is `StringLike` for a single, non-array, always-text atom other than `string`. Its identity keeps only canonical `alphanumeric` options, sorted and deduplicated `enum` members, and a `literal` value. A `string` declaration, numeric, boolean, union, container, and unresolved declarations get `BroadString` while keeping `HoleValidator::Declared` (boxed). `HoleType::display_type()` gives the prompt form, e.g. `string (validated as number(min(1)))`.
- A new `schemas::is_string_like(atom)` is exported from `simplified/subtype.rs`. It is `!is_array && is_always_text(atom)` and reuses the dependency's own list of always-text types instead of copying it. `rule_covers` gates `is_subtype` behind it, which handles the Phase 1 departure (`is_subtype` accepts any identical pair).
- **Open item 3 from Phase 1, resolved:** `schemas::completion::alternative_accepts` lowers the atom on its own with `required` added and validates it through the cached structural JSON-Schema validator. It **enforces** constraints (`min`, `pattern`, …) and **never coerces**: `12345` fails `alphanumeric`, and `"2"` fails `number`. Only `file` values are judged by syntax alone. `HoleValidator::accepts` therefore uses it over the flattened, non-array alternatives of the declaration, and no one-property `EffectiveSchema` document is needed. Tests: `values_are_validated_as_declared_without_conversion` and `numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator`.

### Policy Persistence (Wave 4)

- `ShellRuleEntry::Template(TemplateShape)` was added. `ShellRuleEntry` and `ChainOperator` now derive `Hash`.
- Canonical rule text (C4): `TemplateShape::canonical_text()` writes a literal run bare when every character is safe, and otherwise double-quotes it with the §3 escapes. AC11's line `template just clean --out {{ string }}/build` is produced exactly; a doc test checks it. `TemplateShape::parse_rule` accepts bare and quoted runs and decodes escapes before it recognizes holes. It finds the closing `}}` outside quoted strings, so a quoted `enum` member may contain braces.
- The rule reader rejects:
    - an unescaped brace inside quotes (`UnescapedBrace`), so a hand-written `"{{ string }}"` is reported rather than silently read as literal text;
    - a program-word hole;
    - a constraint, with or without `;`;
    - a description;
    - a named type (including `{{ 123 }}`, which the type grammar reads as a name);
    - array, union, and non-string-like types;
    - unknown options;
    - invalid escapes;
    - control characters;
    - unterminated quotes or holes.
- `store::load_ruleset_with_warnings(path)` returns `(ShellRuleSet, Vec<PolicyLineWarning { path, line, text, reason }>)`. `load_ruleset` keeps its signature and emits each warning through `tracing::warn!`, now with `file:line`. This resolves open item 4 from Phase 1.
- Deduplication now preserves order through a `HashSet<ShellRuleEntry>` instead of the old sort. The old sort could not order template entries, and nothing depended on the sorted order; the whole Darkmatter and Claudine suites pass.
- `append_whitelist_template` / `append_blacklist_template` append one line deduplicated by parsed shape (they read the file first). They refuse with `PolicyIo` (`InvalidData`) if the canonical text would not read back as the same shape. A failed write is always a typed `PolicyIo`. Only approval types are ever written, never a placeholder value.

### Policy Matching (Wave 5)

- Whitelist: `TemplateShape::covers(candidate)` requires the same segment count, operators, ordered redirects, program, argument count, byte-equal literals, and hole positions, plus `rule_covers` for each hole. `ShellRuleSet::template_covers` exposes it.
- Blacklist (`blocks_shape`, `blocks_filled`, `ShellRuleSet::template_blocks_shape`, `ShellRuleSet::blocks_filled`). **Interpretation recorded per plan rule 2**, following spec §8's "a deny list must err toward refusing":
    - holes match any hole regardless of type;
    - redirects are ignored;
    - a single-segment rule matches any segment of a chain (a multi-segment rule must match the whole chain, operators included);
    - against a filled command, each hole matches any run of its containing argument (`*` semantics, linear greedy match).
- `policy::check_template_blacklist(shape, user_blacklist)` is the preflight check. It runs the built-in blacklist on fixed program words and known literal arguments, and **keeps hole positions** (`KnownArg::{Literal, Partial}`): `git {{ x }} reset` is not read as `git reset`, while `git reset {{ x }}` and `psql -c DROP{{ x }}` are refused. It then applies user `prefix` rules, `exact` rules for hole-free segments, and `template` rules by shape. `check_builtin_blacklist` delegates to the same table walk, so its behavior is unchanged.
- `policy::check_filled_chain(chain, user_blacklist)` is the runtime check: built-in first, then user `exact`/`prefix`/`template` over **every** segment before any starts. Shared Validation (P4) calls it.
- **Behavior change now live:** `check_user_blacklist` also applies template blacklist rules to the command it is given, so a hand-written `template deploy {{ string }}` in `.darkmatter-shell-blacklist` already blocks a resolved-byte `deploy prod` in both Darkmatter's `prepare_directive` and Claudine's `validate_and_approve_command_parts`. Multi-segment template blacklist rules reach resolved-byte chains only when P4 wires `check_filled_chain` into execution.

### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| Grammar: holes before syntax, containing argument, empty args, chains, redirect order and lowering, every refusal | `template::tests::{a_placeholder_is_recognized_before_its_spaces_quotes_and_operators, literals_and_holes_stay_in_their_containing_argument, an_empty_quoted_argument_is_one_present_empty_literal, quoting_and_whitespace_spellings_have_one_shape, chains_keep_their_operators_and_redirections_their_order, redirections_lower_exactly_as_the_resolved_byte_tokenizer_does, unsupported_template_grammar_is_a_typed_error, values_are_never_parsed_only_authored_text_is}` |
| Canonical text: AC11 line, round trip with quotes, backslashes, braces, `{{` literal, newline/CR/tab, Unicode, `%`, `!`, empty args, adjacent holes, operators-as-literals; reader forms; rejections | `template::tests::{the_canonical_text_of_the_build_clean_example_is_bare, canonical_text_round_trips_every_literal_and_boundary, the_reader_accepts_bare_and_quoted_literal_runs_alike, malformed_rule_text_is_rejected, rule_types_keep_only_their_canonical_identity}` |
| AC3 store round trip; malformed lines with file and line; I/O failures | L1 `shell_template_policy::{exact_prefix_and_template_rules_survive_write_read_write_read, a_constraint_or_program_word_hole_line_is_skipped_with_its_file_and_line, unreadable_and_non_utf8_policy_files_are_policy_io_failures}` |
| Policy robustness matrix (whitelist and blacklist, 17 cells plus a blank document, control row, sentinels) | L1 `shell_template_policy::{the_whitelist_robustness_matrix, the_blacklist_robustness_matrix}`. The blacklist walk also asserts that preflight (`check_template_blacklist`) and runtime (`check_filled_chain`) agree on every cell |
| AC4 argument-list matching | L1 `whitelist_templates_match_on_the_argument_list` (whitespace/quoting variants match; a changed literal, an extra argument, a moved hole, a changed operator, a changed redirect order, dropped redirects, and an added hole do not) |
| AC5 covers table | L1 `the_covers_table_holds_for_a_declared_placeholder` (every §7 row, plus the broad `string` rule on every row), `a_broad_string_declaration_is_never_covered_by_a_narrower_rule`, `unsupported_rule_types_never_load_even_when_identical_to_the_declaration` |
| AC17 provenance (primitive portion) | L1 `a_bare_property_reads_its_resolved_named_type`, `only_a_bare_top_level_property_reads_a_declaration`, `a_local_child_schema_types_its_own_placeholders`, `numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator`, `values_are_validated_as_declared_without_conversion`; malformed constraints with and without `;` in `malformed_rule_text_is_rejected` and the store test |
| AC13 primitives | L1 `a_string_template_cannot_authorize_a_dangerous_filled_value`, `preflight_keeps_hole_positions_when_checking_known_words`, `user_blacklists_block_filled_commands_with_and_without_authored_holes`, `a_blocked_later_segment_blocks_the_whole_chain`, `a_blacklist_template_matches_on_shape_whatever_the_hole_type`; `policy::tests::a_hole_keeps_its_position_and_never_matches_a_literal_word` |

- **Matrix scope.** The plan asks that the matrix be walked through "public composition approval results". Template approvals reach composition only in P3/P4. This phase therefore observes the public library decisions instead: `load_ruleset_with_warnings`, `ShellRuleSet::template_covers`, `check_template_blacklist`, and `check_filled_chain`. The composition-level walk is listed under Phase 5's Execution Corpus ("the policy matrix").
- **Code-smell audit of touched reader paths:**
    - no `#[serde(default)]`;
    - no `Option` collapsing absent and null;
    - no `filter_map(.. as_str())` on loaded data (the only `filter_map`s select rule variants);
    - no `unwrap_or_default()` or `.ok()` on a parse.
  A missing file still defines an empty rule set, as the format specifies.
- **Tiers:** the unit tests are in the `darkmatter` lib target. The L1 tests are declared in `tests/l1/main.rs` (`mod shell_template_policy;`), and no tier marker is used. The new L1 tests take about 0.05 s in total.

### Gates (macOS)

- `just lint` (darkmatter): clean (library, CLI, DMLS, zed-dmls wasm). The first run hit `clippy::large_enum_variant` on `ApprovalType`/`ArgPart`; it was fixed by boxing the identity atom and the declared validator.
- `just test` (darkmatter): 9278 run, 9278 passed, 10 skipped.
- `just lint` (claudine): clean.
- `just test` (claudine): 8677 run, 8677 passed, 9 skipped. This is the final summary line; Phase 1 reported 9152 from its own invocation. No Claudine test or target was changed in this phase.
- Doc tests for the new public items (`parse_authored`, `TemplateShape::canonical_text`, `HoleType::display_type`, `is_string_like`): 4 passed (`cargo test --doc -p darkmatter -- template is_string_like`).
- **No cross-OS run.** This phase adds pure string parsing, typing, and matching, plus the existing line-based file read. There is no process spawn, no path comparison, and no `cfg(windows)` code. `content.lines()` already strips `\r\n`. Native argv evidence on all four OSes is Phase 5's Package Gates.
- No pre-existing failures were seen. Nothing was skipped.

### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md` gained a "File format" subsection: the three rule kinds, hole types, quoting and escapes, which template lines are skipped and why, and the blacklist template behavior that is live now. Whitelist authorization, the shape choice, and the run-time check are marked **planned**.
- `.claude/skills/darkmatter/compose.md` gained a "Typed shell templates" subsection for agents: module placement, authored parsing, the approval-type/validator split, the `rule_covers` gate, blacklist semantics, and the warnings loader.
- No Claudine skill change: no Claudine code or behavior changed in this phase.

- Status: Phase 2 complete.

## Phase 3

- 2026-10-14: started Phase 3 (Discovery Provenance, Approval Transport). One agent implemented both tasks, because both touch the shared types in `template/` and `shell_expansion/types.rs` (contract C1 single-owner rule).

### Discovery Provenance (Wave 6)

- **New shared types**, in `template/candidate.rs` and exported from `template`:
    - `ShellCandidate { site: CommandSite, template, resolved: Option<Vec<String>>, requirement: ModeRequirement }`
    - `CommandSite { source_file, origin, owner }`
    - `ModeRequirement::{AnyMode, TemplateRequired}`
    - `DeferredShellRoots`
- `Placeholder.timing: HoleTiming::{ApprovalTime, Deferred { roots }}` was added, completing the C2 departure recorded in Phase 2.
- `ComposePreflightReport.candidates` holds **one candidate per command site, never merged by bytes**.
    - `collect_effects` now returns a `CollectedEffects` struct.
    - Entries stay deduplicated by normalized bytes, as before.
- **Authored sites.** Each surface is read from the *authored* text, alongside the existing resolved discovery:
    - body `::shell`: `parser::authored_directive_commands` + `authored_template`, which drops `--when-error`/`--no-cache`/`::timeout:` exactly as `extract_options_from_tokens` does;
    - `::shell-block`: `group_logical_commands` over the authored block bodies;
    - frontmatter `$( … )`: `frontmatter_shell_expansion::authored_shell_template`. A conditional `$( a ? b : c )` is never a template, and gets the new `TemplateGrammarError::Conditional`.
- **Linking a site to its resolved bytes.** Each site is matched to the bytes discovery resolved for it:
    - body and shell-block sites by document order (interpolation inserts data, never a directive);
    - frontmatter sites by key.
    - If the counts differ, the site stays unlinked (`resolved: None`) rather than guessed. Its bytes are still approved as entries, so this fails closed.
    - **Decision (plan rule 6 asks for a recorded choice):** linking by order instead of re-filling the template. A resolved-byte command tokenizes interpolated text, so `a b` becomes two arguments there, while a template fill keeps one argument. The two forms cannot be compared byte for byte.
- **Typing.** `CandidateScope::build` types holes with `project_types(owner_schema, host_globals)`.
    - The owner schema is assembled lazily, only for a site that has a placeholder: `prepare_schemas(...).effective_for(...).simplified` for that document. A transcluded child uses its own schema.
    - Host globals come from `HostGlobals::roots()` (new, crate-private).
- **Deferral.**
    - A hole is `Deferred` when its expression reads a root from `ComposeOptions::with_deferred_shell_roots` (new). It is also `Deferred` when it reads a frontmatter key whose authored value reads such a root, directly or transitively (`derived_deferred_keys`).
    - Derived keys pass to children through `InheritedDiscovery.deferred`.
    - A deferred hole is never evaluated.
    - A site with a deferred hole is `TemplateRequired`. It contributes no resolved-byte entry, and fails with the new `ShellExpansionError::TemplateGrammar` when its authored text is not a valid template.
    - Declaring a root deferred is independent of `BindingView`/host-global availability (plan rule 4).
- **Missing values (AC1).**
    - Every approval-time hole is evaluated against the document's discovery state: `ResolvingLookup` over the prepared document state, with the discovery request's resolution context.
    - A `null` result raises the new `ShellExpansionError::MissingValue { ctx, command, expression, origin }`. The command is the authored text; the value is never included.
    - An evaluation error is left to the existing strict paths, so parse and unavailable-binding errors are unchanged.
    - When the grammar fails, holes are found with `ExpressionFinder::scan_plain`. A missing value therefore also refuses a command that runs only as resolved bytes.
    - Claudine's lifecycle `shell` and `set $( … )` resolution (`preflight::resolve_shell_command_expr`) applies the same rule. It composes the command and each placeholder as whole values in **one** `SubtreeCompose` call, because the `composition_seams` drift guard allows one call per site. It raises `LifecycleShellResolution`, whose source is Darkmatter's typed `MissingValue`.
- **Unsuppressible.** `MarkdownError::shell_input_violation()` (new) finds `MissingValue`/`TemplateGrammar` anywhere in the cause chain. It was added to `transclusion_failure_is_fatal` (`pipeline/phases.rs`) and to nested `is_structural` (`nested.rs`). Neither error is `ExecutionFailed`, so `when_error` cannot absorb them.
- **Not done here, by design (recorded for Phase 4).**
    - Claudine sequence preflight does not yet declare `outputs`/`loop`/`group` deferred, and it still raises `SequenceShellLateBinding` (C8, Sequence Position).
    - Mapping *task parameters* derived from late roots onto deferral is part of that same wiring. Darkmatter already follows derived frontmatter keys and inherited keys. A task parameter reaches the task document as data, so the host passes its name in `DeferredShellRoots` when that parameter reads a late root.
    - A frontmatter `$( … )` whose hole is deferred still meets the existing `DynamicCommandShape` refusal (`detect_dynamic_frontmatter_command_shape`) when the deferred root is not bound in discovery. This fails closed. Frontmatter template-mode execution is Phase 4 (Shared Validation).
    - Discovery's body interpolation still evaluates every body span in lenient mode. With Claudine's preflight `BindingView`, a late root is `Unavailable`, so nothing is observed. "Not evaluated" is therefore guaranteed for the candidate check, not for the pre-existing body interpolation pass.

### Approval Transport (Wave 6)

- **New shared types**, in `template/approval.rs`:
    - `ShellPreApprovals { exact: HashSet<String>, templates: HashSet<TemplateShape> }`, with `From<HashSet<String>>` (exact only) and `FromIterator<String>`;
    - `ApprovalKey::{Exact { policy_root, normalized }, Template { policy_root, shape }}`;
    - `OfferedChoices { exact, shape, prefix }`;
    - `TemplateApprovalView { shape, display }`;
    - `CommandTemplate::display_text()`, which gives `{{ expr: type }}` with the validator disclosure.
- **Departure from C6, recorded per plan rule 2:** `ApprovalKey::Template` holds the `TemplateShape` itself instead of its canonical text, so keys compare structurally. `policy_root` is an `Option`, because Claudine runs without a policy root in some paths.
- **Darkmatter**
    - `ComposeOptions.pre_approved_commands: Option<ShellPreApprovals>`.
        - `with_pre_approved_commands(impl Into<ShellPreApprovals>)`.
        - Its identity hash encodes templates only when present, so resolved-byte sets keep their fingerprint.
        - `with_deferred_shell_roots` is also hashed only when non-empty.
    - `ShellApprovalDecision::AllowTemplatePersist` was added.
    - `ShellApprovalRequest` gains `offered` and `template`. An unknown value is never encoded as bytes: a template-required request has an empty `normalized_exact`.
    - `SharedShellExpansionRuntime.allow_once`/`pending_allow_once` are now `HashSet<ApprovalKey>`; `ReservationGuard` holds keys. The reservation, condvar, and drop-release semantics are unchanged.
    - **Membership.**
        - `validate_pre_approved` checks it through `first_unapproved`. A template-required site must have its shape approved. A resolved-byte entry needs its bytes approved, unless every site resolving to those bytes is approved by its shape.
        - `prepare_directive` admits only exact members. **Template-mode execution is Phase 4**, so a site approved only by its shape is refused with `NotPreApproved` at execution. It is never run as interpolated bytes, and nothing inserts the resolved bytes.
        - A resolved-byte prompt that returns `AllowTemplatePersist` (not offered) is treated as `Deny`.
    - **`compose_preflight_approvals` (the `md` batch path) now decides per site.**
        - The order is: the blacklists for every entry; then per candidate an `exact` rule, a `template` rule (the shape is blacklist-checked by `check_template_blacklist`), a `prefix` rule, and finally the handler with the site's `offered` choices and template view.
        - Unlinked entries keep the old per-entry flow.
        - Every choice now **persists its row**: exact, prefix, template, and blacklist exact or template. This also fixes the pre-existing defect recorded in Phase 1, where this path persisted nothing. It was assigned to Approval Prompts (Phase 4), but `AllowTemplatePersist` could not be handled without it.
        - A failed write is `PolicyIo` and approves nothing.
        - A choice the request did not offer is `Denied`.
- **Claudine**
    - `PreFlightResult.approved_commands`, `PrepareOptions.pre_approved_commands`, `CallerInputLayers.pre_approved_commands`, and `add_approved_commands` now carry `ShellPreApprovals`.
    - So do every CLI handoff and re-audit set: `compose/prep.rs` loop iteration and staged stabilization, `staged_boot.rs`, sequence `mod.rs`/`phase1c.rs`/`jit.rs`/`iterate.rs`, and harness `prompt.rs`.
    - `preflight_bootstrap_shell` and `preflight_document_shell` return it too.
    - Lifecycle `set` values pass their bytes as an exact-only `ShellPreApprovals`.
    - `SharedApprovalCache` and `ShellApprovalOptions.approval_cache` are keyed by `ApprovalKey`.
    - `harness::shell` was restructured around a private `SessionPolicy`.
        - `validate_and_approve_command_parts` keeps its behavior, messages, and order. It now keys the cache by `ApprovalKey::exact(policy_root, …)`, and a failed policy write is the new `HarnessError::ShellPolicyWriteFailed { path, source }` (`io.write_failed`) **with nothing cached**. Before this change the write failure was silently ignored (`let _ =`) and the decision cached.
        - New `validate_and_approve_template_site(candidate, options) -> TemplateShape` applies, in order: the template blacklist; a template rule or prefix rules for every program; the template cache key; then the handler with the site's offered choices. An exact decision never covers it.
        - New `template_rule_covers` and `check_command_blacklists`.
    - `composition::preflight::approve_commands` now takes Darkmatter's `candidates`.
        - Template-required sites are approved by shape.
        - A resolved-byte site whose bytes no exact rule covers, and whose shape a template rule covers, is approved by shape.
        - Its bytes are still checked against both blacklists, but they are not prompted for or added to the exact set unless another site needs them as bytes.
        - Errors map through one `audit_failure` helper, keeping the existing `ShellCommandDenied`/`ShellApprovalUnavailable` families.
- **Result cache:** unchanged. The template-mode result key (C7) arrives with template execution in Phase 4.

### Comment drift fixed

- Darkmatter `preflight/mod.rs` and `preflight/approval.rs` module docs, `preflight/lifecycle.rs` steps, the `ComposeOptions.pre_approved_commands` field doc, and the `prepare_directive` fast-path comment.
- Claudine `resolve_shell_approvals` doc, `execute_set_shell_values` doc, `PreFlightResult`, and `CallerInputLayers`.
- The sequence comments listed in Phase 1 (`sequence/preflight/*`, `task/*`) still describe today's behavior, which is unchanged. They change with Sequence Position and Sequence Dispatch in Phase 4.

### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| AC1: missing value on every Darkmatter surface (null, absent, `fm.` namespace, shell-block, frontmatter, invalid grammar), typed error naming command, expression, and origin | L1 `darkmatter::l1 shell_template_discovery::a_null_or_absent_value_fails_preflight_on_every_shell_surface` |
| AC1: empty string, 0, false, fallback, and absent-with-fallback are present | `shell_template_discovery::empty_zero_false_and_fallback_values_are_present` |
| AC1: unsuppressible by `--when-error`; non-shell null still renders empty | `shell_template_discovery::a_missing_value_is_unsuppressible_and_a_non_shell_null_still_renders_empty` |
| AC1: nothing runs (frontmatter `git init` marker) | `shell_template_discovery::compose_fails_with_the_missing_value_before_any_command_runs` |
| AC1 end to end, `md compose` (body, absent, frontmatter, shell-block; marker proves nothing ran; empty string runs) | `darkmatter-cli::l1 compose_shell::test_compose_refuses_a_missing_shell_value_before_running_anything`, `test_compose_runs_a_shell_command_whose_value_is_an_empty_string` |
| AC1 end to end, `claudine compose` (body, frontmatter, lifecycle `shell`, lifecycle `set $( … )`; marker; empty string runs) | `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface`, `an_empty_string_value_is_present_and_runs` |
| AC16/AC17: one candidate per site, owner-schema typing for root vs transcluded child, owner key | `shell_template_discovery::each_site_keeps_its_authored_template_and_its_owners_types` |
| Directive options are not template arguments; a hole stays in its containing argument | `shell_template_discovery::directive_options_are_not_template_arguments_and_holes_stay_whole` |
| No shape for a hole-free site | `shell_template_discovery::a_site_without_placeholders_offers_no_shape` |
| Deferral: direct, derived through frontmatter, mixed with approval-time holes; no entry for a late site; offered choices | `shell_template_discovery::a_late_root_is_not_evaluated_and_requires_a_template` |
| A late read does not mask a missing approval-time value | `shell_template_discovery::a_late_read_with_a_missing_approval_time_value_still_fails` |
| A late site with invalid grammar fails as `TemplateGrammar` (unsuppressible) | `shell_template_discovery::a_late_command_that_is_not_a_valid_template_fails_preflight` |
| Without declared deferral, the same read is approval-time | `shell_template_discovery::without_declared_late_roots_the_same_read_is_an_approval_time_value` |
| AC15: exact and template approvals never authorize each other, including at the execution gate | `shell_template_discovery::exact_and_template_approvals_never_authorize_each_other` |
| Membership is proved and never manufactured; a changed declared type is not a member | `shell_template_discovery::the_gate_accepts_a_shape_approval_and_never_adds_its_bytes` |
| AC12: exact pre-approvals still run resolved bytes | `shell_template_discovery::exact_pre_approvals_still_run_resolved_bytes` (plus the whole unchanged corpus) |
| AC8 foundation, `md` path: offered choices, template display, each choice persists its row | `shell_template_discovery::a_resolved_site_prompt_offers_exact_and_shape_and_each_choice_persists_its_row`, `a_template_required_site_is_offered_no_exact_choice` |
| Precedence exact, then template, then prefix | `shell_template_discovery::rules_are_preferred_exact_then_template_then_prefix` (md), `claudine composition::preflight::tests::a_template_rule_approves_a_resolved_site_by_shape_and_an_exact_rule_wins` |
| A persisted template covers a late site; an exact rule does not | `shell_template_discovery::a_persisted_template_rule_covers_a_late_site_without_prompting` |
| Shape blacklist; blacklisting persists the shape | `shell_template_discovery::a_shape_blacklist_refuses_a_late_site_and_blacklisting_persists_the_shape`, `claudine … a_template_blacklist_refuses_the_site_before_any_prompt` |
| AC15: a failed write is a typed policy failure; nothing approved or cached | `shell_template_discovery::a_failed_template_write_is_a_policy_failure_and_approves_nothing`, `claudine … a_failed_policy_write_is_typed_and_caches_no_decision` (template and exact) |
| AC17: the owner schema flows into the persisted rule, with numeric disclosure | `shell_template_discovery::a_hole_type_follows_the_owner_schema_into_the_persisted_rule` |
| Mode- and policy-root-tagged runtime reservations; failure releases | `darkmatter … template::tests::session_reservations_are_keyed_by_mode_and_policy_root` |
| Claudine: a template site is offered its shape and persists it; the cache key is template | `claudine … a_template_site_is_offered_its_shape_and_persists_the_template_row` |
| Claudine: exact cache entries and rules never answer for a template site; an unoffered exact choice is denied and persists nothing | `claudine … an_exact_decision_never_answers_for_a_template_site` |
| Claudine: policy-root isolation of the session cache | `claudine … a_template_decision_is_scoped_to_its_policy_root` |
| Claudine: a shape-approved site's bytes still meet the blacklist | `claudine … a_shape_approved_site_still_checks_its_bytes_against_the_blacklist` |
| Claudine: a late document site with no approval fails pre-flight naming it | `claudine … a_late_document_site_without_an_approval_fails_pre_flight_naming_it` |

- **Tiers.**
    - Darkmatter L1 tests are declared in `tests/l1/main.rs` (`mod shell_template_discovery;`).
    - Claudine CLI L1 tests are declared in `cli/tests/l1/main.rs` (`mod shell_missing_values;`).
    - Claudine lib tests are in `composition/preflight/tests.rs`. The runtime unit test is in `template/tests.rs`.
    - No tier marker is used.
    - Timing: all new tests are fast. The two CLI process tests take about 1.7 s and 0.7 s; each lib test takes under 0.1 s.
- **Input robustness matrix:** not applicable. No configuration reader changed in this phase; the policy-file reader is the same one Phase 2 matrix-tested.

### Gates (macOS)

- `just test` (darkmatter): 10053 run, 10053 passed, 10 skipped.
- `just test` (claudine): 9162 run, 9162 passed, 9 skipped.
    - The first run failed the `composition_seams::subtree_compose_baseline_holds_the_line` drift guard: the lifecycle missing-value check had added a second `SubtreeCompose` call. It was fixed by folding both reads into the existing call.
- `just lint` (darkmatter): clean. Two fixes were needed: `#[allow(clippy::too_many_arguments)]` on `scan_one_frontmatter`, matching `collect_recursive`, and a needless borrow in a test.
- `just lint` (claudine): clean.
- **No cross-OS run.** This phase adds no `cfg(windows)` code, no new spawn path, and no path comparison except `canonicalize_simplified` for owner keys. The new process tests spawn `git` through `host_path()`, as `lifecycle_set_shell_values.rs` already does on every OS. Native argv evidence on all four OSes remains Phase 5.
- No pre-existing failures were seen. Nothing was skipped.

### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md` gained:
    - a "Missing values" section;
    - template-rule precedence;
    - the persisted `md` choices, and policy-write failure;
    - request `offered`/`template`;
    - per-site `candidates`, late values (`with_deferred_shell_roots`), and the structured membership proof.
    - **planned** markers for template-mode execution and the `md` prompt's shape choice.
- `claudine/docs/topics/pre-flight-checks.md` gained:
    - a "Missing Values" section;
    - template-rule precedence and the template-only approval of late sites;
    - `io.write_failed`;
    - the mode- and policy-root-keyed cache;
    - `ShellPreApprovals` membership.
    - **planned** markers for template execution and sequence late-root declaration.
- Skills: `.claude/skills/darkmatter/compose.md` (discovery provenance and transport) and `.claude/skills/claudine/cli-commands.md` (the shell pre-flight row).

- Status: Phase 3 complete.

## Phase 4

- 2026-10-14: started Phase 4 (Shared Validation, Sequence Position, Sequence Dispatch, Lifecycle Dispatch, Approval Prompts). The orchestrator runs the tasks as foreground subagents: Shared Validation first (every other task consumes its API), then the remaining tasks in two waves. Each task appends its own subsection below.

### Shared Validation (Wave 7)

- 2026-10-14: implemented the shared filler, validator, and direct-target resolver, and wired template-mode execution into body `::shell`, `::shell-block`, and frontmatter `$( … )`. Stage order is unchanged.

#### What was built

- **`template/fill.rs`** (exported from `shell_expansion::template`)
    - `trait HoleEvaluator { fn evaluate(&self, expression: &str) -> Result<Value, ExpressionError> }`, implemented for every `Fn(&str) -> Result<Value, ExpressionError>` and by `LookupEvaluator<'a, L: EvaluationLookup>` (`LookupEvaluator::new(&lookup)`).
    - `fill_template(&CommandTemplate, &dyn HoleEvaluator, Option<&SiteValues>) -> Result<FilledChain, HoleError>`. Per hole, in order: evaluate once; `null` → `HoleError::Missing`; array/object → `Rejected { reason: NotScalar }`; a string with NUL → `Rejected { reason: EmbeddedNul }`; `HoleValidator::accepts` on the **original** value → `Rejected { reason: DeclaredType }`; `scalar_string`; concatenate into one argument. Every hole of every segment is filled before it returns.
    - `HoleError::{Missing, Rejected { expression, declared_type, reason }, Evaluation { expression, source }}`; `HoleError::into_shell_error(ctx, command, origin)` maps to `MissingValue`, `TemplateValueRejected`, or the existing `ExpressionEvaluation` (typed cause kept). No variant carries the value.
    - `LateValues` (`site(id) -> Option<&SiteValues>`), `SiteValues` (`get(hole_index)`), `resolve_late_values(impl IntoIterator<Item = (&str, &CommandTemplate)>, &dyn HoleEvaluator) -> Result<LateValues, LateValueError { site, error }>`. Only `HoleTiming::Deferred` holes are evaluated and kept; the hole index is the placeholder's position in authored order.
    - `HoleType::declared_text()`: the type a value is checked against (`string` for the fallback).
- **`template/target.rs`**: `resolve_direct_target(program, path: Option<&OsStr>, cwd) -> Result<PathBuf, TemplateTargetError>`. It finds the program the way the executor does (`which_in` with the child's `PATH`, else `which`). Windows accepts only `.exe`/`.com`; Unix requires ELF, Mach-O (both byte orders, 32/64), universal Mach-O, or `#!`. `TemplateTargetError::{NotFound, Interpreter, NotNative, Unreadable}`.
- **New `ShellExpansionError` variants**: `TemplateValueRejected { ctx, command, placeholder: Box<str>, declared_type: Box<str>, reason: RejectReason, origin }` and `TemplateTarget { ctx, command, reason: Box<TemplateTargetError>, origin }`, each with a status block. Both join `MissingValue`/`TemplateGrammar` in `MarkdownError::shell_input_violation()`, so `transclusion_failure_is_fatal` and nested `is_structural` treat them as fatal. Neither is `ExecutionFailed`, so `handle_errors`/`when_error` never see them.
- **`shell_expansion/site.rs`** (crate-private) holds execution of one site in either mode:
    - `TemplateSite` (typed candidate + evaluator + authored context + `SiteFrame`), `SiteScope::site` (builds it through `CandidateScope::build`, so typing, timing, requirement, and `TemplateGrammar` are discovery's), and `prepare_site(resolved, site, …)`.
    - **Mode choice.** With pre-approvals: the freshly resolved bytes as exact members, else the re-typed shape as a template member, else `NotPreApproved`. Nothing is inserted into either set. Without pre-approvals: the template blacklist on the shape; then an exact rule or cached exact key (bytes); a template rule or cached template key (template); then a prefix rule (bytes) or prefix rules for every program (template-only); then the handler.
    - **Resolved-byte prompt.** `prepare_resolved` (the former body of `prepare_directive`, which now delegates to it) offers `{exact, shape, prefix}` with the template view; `AllowTemplatePersist` writes `template <shape>`, records it in the runtime whitelist, and prepares template mode.
    - **Template-only prompt.** Offers `{exact: false, shape, prefix}` with an empty `normalized_exact`, reserving the shape's `ApprovalKey` with the existing guard. `AllowOnce` records the template key, so a second site of the same shape is not asked. An unoffered `AllowExactPersist` is `Denied`. `BlacklistPersist` writes `template <shape>` to the blacklist.
    - `TemplateSite::prepare_template` fills, runs `check_filled_chain` (built-in, then user; every segment before any starts), resolves each program to a native target, and returns `ExecutionMode::Template` with the authored text as `raw_command` and display. This all happens before a result-cache lookup or a spawn.
- **Result-cache key in template mode** (`template_command_key`): length-prefixed fields: mode tag `template`, then per segment the operator, resolved program, argument count, each argument, and the rendered redirection; then the existing context (cwd, `strip_ansi`, timeout, behavior). It is separate from `ApprovalKey`.
- **Resolved-byte mode** now also runs `check_filled_chain` on its effective chain, in the pre-approved fast path too, so a chain `template` blacklist reaches resolved-byte chains (AC13 "including a resolved-byte command"; Phase 2 had left this for P4). Nothing else in resolved-byte mode changed.
- **Native recorder**: `test_toolkit::argv_recorder` (`main`, `encode_record`, `decode_records`, `read_argv_records`, env `ARGV_RECORDER_FILE`). Records are length-prefixed: `u32` count, then `u32` length + UTF-8 per argument; `argv[0]` is excluded.
    - Thin bins: `darkmatter-argv-recorder` (`darkmatter/cli/tests/bin/argv_recorder/main.rs`) and `claudine-argv-recorder` (`claudine/cli/tests/bin/argv_recorder/main.rs`), both `required-features = ["test-fixtures"]`, each enabling `test-toolkit` as an optional dependency.
    - `darkmatter-cli` gained a `test-fixtures` feature, added to `[package.metadata.ci.tests] features` and `local-features`. The darkmatter `just test` recipe (both branches) now passes `--features test-fixtures` for the CLI. `claudine-cli` already lists `test-fixtures` in both.
    - Archive mode finds the bin: the WSL2 archive run below passed.

#### How the authored text is recovered (plan rule 6 asks for a recorded choice)

- **Body `::shell`.** `BodyProvenance` gained a third map, `authored: Option<AuthoredBody>`: the pristine body plus an identity `BodyOrigin` over it, advanced by `BodyProvenance::advance` and by the interpolation stage's two edit batches.
    - The shell stage projects each directive line's `::shell` keyword (always authored) into the pristine body and reads that line with `parser::authored_directive_at`, the same prefix rules as discovery's `authored_directive_commands`.
    - Projection is exact even after page blocks remove earlier directives, which order-linking would not survive. If the map stops describing the body, no site is built and the line runs as resolved bytes only, so a shape-only approval fails closed with `NotPreApproved`.
- **`::shell-block`.** The block opener's first byte is projected into the pristine body, the authored pair with that start is found, and its commands are grouped. They are linked to resolved commands by index when the counts match.
- **Frontmatter.** `FrontmatterProvenance::authored_source(key)` + `authored_shell_template`.
- **Hole evaluation.** Holes are evaluated against the stage's current state: `ResolvingLookup` over the `EffectiveState` for body and blocks, `FrontmatterSeedState` over the post-pass-1 frontmatter for `$( … )`.
- **Owner schema.** `PreparedSchemas::effective_for(markdown).simplified`, threaded into `run_inline_pre_operation`. For frontmatter it is assembled only when a value holds `$(`.

#### Departures from the contract (C7), with reasons

1. `fill_template` returns `HoleError`, not `ShellExpansionError`; `HoleError::into_shell_error(ctx, command, origin)` converts. Reason: `ShellExpansionError` requires a `SourceContext` and `ShellCommandOrigin`, which Claudine's task and lifecycle runners do not have. They wrap `HoleError` in their own diagnostics, keeping the typed cause.
2. `fill_template`'s late input is `Option<&SiteValues>` (from `LateValues::site(id)`), and `resolve_late_values` takes `(site_id, &CommandTemplate)` pairs rather than `&[&ShellCandidate]`. Reason: sequence `shell:` tasks and lifecycle stacks are not Darkmatter-discovered candidates (Phase 3, note 3); the host chooses the site id.
3. `resolve_direct_target` returns `TemplateTargetError`; the composition wraps it in `ShellExpansionError::TemplateTarget`. `check_filled_chain` keeps its Phase 2 `Option<String>` return.
4. The result-cache key encodes each segment's lowered `RedirectionConfig`, not the authored `RedirectOp` order: `FilledSegment` carries the lowered form (Phase 2 departure). Two orders with the same lowering behave identically, so sharing a result is correct.
5. `TemplateValueRejected.placeholder`/`declared_type` are `Box<str>` and `TemplateTarget.reason` is boxed. Reason: clippy `result_large_err` (the enum reached 136 bytes). `TemplateValueRejected` also carries `reason: RejectReason`, which C7 did not list.
6. **Not added:** `ComposeOptions::with_late_values`. A document's commands evaluate their holes at command entry against the state current then, which for a late root bound as a host global at step time is the value the barrier checked. Sequence Dispatch decides whether documents need barrier values injected; the site id would be `CommandSite`-based.
7. **Not added:** a JSON-scope `EvaluationLookup`. A new lookup trips the `lookup_parity` inventory guard and would duplicate `EffectiveState`. Hosts use `LookupEvaluator::new(&state)` with `EffectiveStateBuilder::new().with_frontmatter(map).with_context(ctx).build()?`, `LookupEvaluator::new(&session)` over an `EvaluationSession`, or a closure.
8. **Not added:** a public "run a `FilledChain` with Darkmatter's executor" entry point. C7 keeps operational ownership (process tree, deadlines, cancellation) in each runner, so Claudine's `run_structured` adapters build argv from `FilledChain` themselves.

#### Interpretations (plan rule 2)

- **Unparseable resolved text means template-only.** A value holding `;`, quotes, `&&`, or a newline that splits a block can make the interpolated text unparseable or "data-shaped". Today that is an error. Now:
    - When the authored text is a valid template with a placeholder, the site is template-only: discovery marks it `TemplateRequired` with no entries (`scan_directive_lines` keeps each line's parse result; blocks fall back to their authored commands; frontmatter gets `FrontmatterShellAst::Unparsed { reason }`). Execution never runs it as bytes.
    - With no template site, the original error stands.
    - This is spec §4 ("values are never reparsed"), and is the only way AC6's values can run.
    - Consequence: three earlier tests that expected a data-shaped command to fail now see the authored template offered and assert that instead (`data_origin::{a_body_pipeline_operator_from_interpolation_is_rejected, data_cannot_split_or_close_a_shell_block}`, `shell_template_discovery::the_gate_accepts_a_shape_approval_and_never_adds_its_bytes`, the last now running in template mode). Data still never adds a segment, operator, or command.
- **Directive options for an unparseable line.** These are read from the authored words (`parser::authored_directive_options`). An option or option argument holding a placeholder has no authored value, so the line keeps its parse error.
- **A value-inserted line break.** In template mode it stays inside the directive: the replaced span runs to the first authored line break (`directive_extent`). Resolved-byte mode keeps its old span.
- **Ternaries.** A frontmatter ternary stays resolved-byte only (`TemplateGrammarError::Conditional`).
- **Display text.** A template-mode command's display and `raw_command` are its authored text, directive options included (`--when-error fallback printf %s {{ v }}`), so execution errors never show filled values.
- **Evaluation failures.** A hole that fails to evaluate is the existing `ExpressionEvaluation` (spec §1: parse and unavailable-binding errors keep their typed errors). It is not added to `shell_input_violation`.
- **Prefix choice for a template-only site.** `AllowCommandPersist` persists a `prefix` rule for each distinct program and runs template mode, mirroring `compose_preflight_approvals`.

#### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| AC6/AC14: one argument per word (`a b`, `x; rm -rf ~`, `$(id)`, `` `id` ``, empty, adjacent holes, quotes, backslashes, Unicode, newline, `%`, `!`), native recorder; directive extent over a value line break | `darkmatter-cli::l1 compose_shell_template::each_placeholder_fills_exactly_one_argument_of_its_word` |
| AC7 through `md`: array, object, NUL, null → nothing runs, typed error, value absent | `compose_shell_template::a_refused_value_runs_nothing_and_never_appears_in_the_error` |
| AC7 declared type and `min`, no coercion, full chain (Display + sources + Debug) free of the value | `darkmatter::l1 shell_template_execution::a_value_its_declaration_refuses_is_rejected_without_showing_it` |
| Missing vs present scalars; conversion | `shell_template_execution::null_is_missing_and_other_scalars_convert` |
| Every segment filled before the chain runs; each hole evaluated once | `shell_template_execution::every_segment_is_filled_once_before_the_chain_runs` |
| Late-value barrier: only deferred holes, never re-evaluated; failure names its site | `shell_template_execution::{late_values_are_validated_once_at_the_barrier, a_missing_late_value_fails_at_the_barrier_naming_its_site}` |
| AC13: `git {{ string }}` filled with `reset` refused by the built-in blacklist at run time | `compose_shell_template::a_string_template_cannot_authorize_a_builtin_blacklisted_fill` |
| AC13: exact user blacklist on a filled command; blocked later segment keeps the first from starting | `compose_shell_template::a_user_blacklist_blocks_a_filled_chain_before_its_first_segment` |
| AC13: shape-only template blacklist blocks a resolved-byte command and a resolved-byte chain | `compose_shell_template::a_template_blacklist_blocks_resolved_byte_commands_and_chains` |
| AC14 targets: a Unix script without `#!` refused (marker absent), a `#!` script runs with argv intact; Windows `.cmd`/`.bat` refused (marker absent) | `compose_shell_template::{a_script_without_a_shebang_is_refused_and_a_shebang_script_runs (unix), a_batch_file_is_refused_as_a_template_target (windows)}`; `shell_template_execution::{a_program_missing_from_the_search_path_is_not_found, a_script_without_a_shebang_is_refused_and_a_shebang_script_resolves (unix), a_batch_file_is_refused_as_a_direct_target (windows)}`; unit `template::target::tests::*` |
| AC15: shape approval reused, changed values run fresh, the same fill cached | `compose_shell_template::a_shape_approval_is_reused_but_changed_values_run_fresh` |
| AC15: exact and template rules select their own mode, never each other's; exact wins | `compose_shell_template::exact_and_template_rules_select_their_own_execution_mode`, `shell_template_execution::an_exact_rule_wins_over_a_template_rule` |
| Fatal: not suppressed by `--when-error`, `--allow-shell-timeout`, or a transclusion notice | `compose_shell_template::template_input_failures_are_never_suppressed` |
| Interactive path: shape offered beside bytes, `AllowTemplatePersist` persists `template` and runs template mode, the rule then answers silently, once runs bytes | `shell_template_execution::the_shape_choice_persists_the_template_and_runs_in_template_mode` |
| Template-only prompt: no exact offered, empty `normalized_exact`, once covers the shape for the session | `shell_template_execution::a_template_only_site_offers_no_bytes_and_once_covers_its_shape` |
| Unoffered exact denies; blacklisting persists the shape | `shell_template_execution::a_template_only_site_refuses_an_exact_choice_and_blacklists_its_shape` |
| Recorder record format round trip | `test-toolkit argv_recorder::tests::{records_round_trip_every_argument_boundary, a_truncated_record_is_invalid_data}` |
| Resolved-byte mode unchanged (AC12) | the whole darkmatter and claudine corpora (below), with the three intended updates listed above |

- **Tiers and placement.**
    - The Darkmatter lib file is declared in `tests/l1/main.rs` (`mod shell_template_execution;`) and the CLI file in `cli/tests/l1/main.rs` (`mod compose_shell_template;`).
    - No tier marker is used. CLI tests spawn `md` only through `CliProcessFixture::command()` (default minimal `PATH`, recorder copied into the fixture `bin`).
    - Each test takes under 0.4 s locally and under 1.8 s on `BUILD_WIN`.
- **Input robustness matrix:** not applicable. No configuration reader changed (the policy-file reader is Phase 2's).

#### Gates

- `just test` (darkmatter, macOS): 10076 run, 10076 passed, 10 skipped. It includes the new 11 lib and 9 CLI tests, and `darkmatter-cli` now builds with `test-fixtures`.
    - The first runs failed on `lookup_parity` (the dropped JSON lookup), on the three intended test updates above, and on `frontmatter_shell_expansion::execution_tests::non_ternary_arg_interpolation_cannot_inject_new_action`. That last one was fixed in code: `Unparsed` keeps the original parse reason, which is reported when no template site applies.
- `just lint` (darkmatter): clean. The first run hit `result_large_err` (fixed by boxing).
- `just test` (claudine, macOS): 9162 run, 9162 passed, 9 skipped. `just lint` (claudine): clean, apart from the pre-existing linker `__eh_frame` note.
- `cargo check -p claudine -p claudine-cli --features claudine-cli/test-fixtures --tests`: clean.
- `test-toolkit`: `cargo clippy -p test-toolkit --all-targets -- -D warnings` clean; `cargo nextest run -p test-toolkit --lib` 138 passed.
- Doc tests for the new public items (`fill_template`, `parse_authored`, …): `cargo test --doc -p darkmatter -- template::fill shell_expansion::template`, 5 passed.
- **Cross-OS evidence** (`just cross-check`, targeted by name; behavioral, not compile-only):
    - `darkmatter-cli … compose_shell_template`: Windows (`build-win-native`, archive) 9/9, including `a_batch_file_is_refused_as_a_template_target`; Linux (`build-linux`) 9/9; WSL2 (`build-win`, archive from ubuntu-latest) 9/9.
    - `darkmatter … shell_template data_origin`: Windows 69/69; WSL2 69/69.
    - These are a patched tree, so no CI observation was published. Full-suite Windows/WSL2 runs remain the nightly's.

#### Deferred or left for later tasks

- **Sequence Position / Sequence Dispatch.**
    - Declare `outputs`/`loop`/`group` deferred.
    - Call `resolve_late_values` at the step barrier.
    - Build `FilledChain`s for `shell:` tasks and stacks with `fill_template` and run them through `TaskShellRunner::run_structured`.
    - Relax `detect_dynamic_frontmatter_command_shape` for deferred frontmatter holes, which still fails closed at discovery when the late root is unbound.
    - Decide whether documents need `with_late_values`.
- **Lifecycle Dispatch.** Lifecycle `shell` and `set $( … )` (`frontmatter_shell_expansion/assignment.rs::execute_resolved_shell_values`) are still resolved-byte only. `DisabledShellRunner` is untouched.
- **Approval Prompts.** The `md` prompt (`darkmatter/cli/src/approval.rs`) still does not render the shape choice; the doc keeps that **planned** marker. The library flow (`prepare_site`/`prepare_resolved`) already handles every decision.
- **The Claudine recorder is unused so far.** `claudine-argv-recorder` is built under `test-fixtures` and waiting for the Claudine runner tests; locate it with `biscuit_test_harness::bin_exe!("claudine-argv-recorder")`, copy it onto the child `PATH`, set `ARGV_RECORDER_FILE`, and read with `test_toolkit::argv_recorder::read_argv_records`.
- **Observation for Execution Corpus (P5), not changed here.**
    - In a single document, a declared-type or `min` violation is caught first by frontmatter schema validation. Its pre-existing message shows the value (`"zq" is shorter than 3 characters`), so AC7's "never contains the value" holds for the template run-time check (proven at the library boundary) but not for that earlier schema error.
    - Also pre-existing: in resolved-byte mode, a value with a line break truncates the directive at the break.

#### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md`: a new "Running a command as a template" section (mode table, a worked `a b` example, the four checks, fatal classification, result caching, Mermaid flow). It replaces the template-execution **planned** marker, documents the handler's shape choice, and notes that pre-approved runs still meet the blacklists and the host API. The `md` prompt marker stays **planned**.
- `darkmatter/docs/inline/shell-expansion.md`: approval by shape, and data that holds command syntax.
- `claudine/docs/topics/pre-flight-checks.md`: document commands approved by shape now run as templates; the **planned** marker is narrowed to lifecycle `shell` and sequence `shell:`.
- `darkmatter/docs/dependencies.md` and `claudine/docs/dependencies.md`: the optional `test-toolkit` dependency for the recorder bins. `tools/test-toolkit/README.md`: `argv_recorder`.
- `.claude/skills/darkmatter/compose.md`: execution (`site.rs`), authored-text recovery, and the host API.
- **Comment drift fixed:** the `body_origin` module doc (three maps), the `FrontmatterShellAst` variant list, and the `template` module doc.

- Status: Shared Validation complete.

### Approval Prompts (Wave 8)

- 2026-10-14: the `md` prompt now lists only the offered choices and shows a template's placeholders; Claudine offers the shape choice for a known-bytes document site, and installs the same prompt.

#### What was built

- **`darkmatter/cli/src/approval.rs`** (`CliShellApprovalHandler`, also Claudine's installed handler)
    - A private `Choice` list built from `request.offered`, in the order exact, shape, prefix, once, deny, blacklist. Shape is listed only when `offered.shape` and `request.template` is present. Choices are numbered 1..n over what is listed, so a number cannot select an unoffered choice; an unlisted number is refused ("Please enter 1-n") and the prompt is shown again.
    - Each line names the rule it saves: `exact <normalized_exact>` (or "an exact rule for each command of the chain"), `template <canonical shape>`, `prefix <program>` for every distinct chain program, once ("this command shape, this session only" when no exact choice is listed), deny ("stops the run"), blacklist (`exact …`, or `template <shape>` when exact is not offered).
    - With a template, `Command:` is `template.display` (`{{ name: type }}`, with the `(validated as number)` disclosure from `display_text`), and `Resolved:` shows `normalized_exact` only when exact is offered. Without a template, `Command:` is `raw_command` as before. Labels are padded to one width. Rendering stays per-line `Prose`, as before.
    - `approve_with_io` is now `pub`: the scripted-I/O seam for both prompts. End of input returns `Deny` (before, an empty read looped forever).
    - `can_prompt_interactively` is unchanged.
- **Claudine**
    - `harness::shell::prompt_resolved_site(candidate, options) -> Result<Option<ResolvedSiteApproval>, HarnessError>` with `ResolvedSiteApproval::{Bytes, Shape(TemplateShape)}`. For an `AnyMode` site with resolved bytes and a shape, when the bytes are not blacklisted and not already decided by a whitelist rule or the session cache, it asks once with `candidate.offered()` and `approval_view()`. Rows: exact → `exact` per segment; shape → `template <shape>` (template key cached); prefix → `prefix` per distinct program; once → exact keys cached `Allowed`; blacklist → `exact` per segment in the blacklist, cached `Blacklisted`, error; deny or an unoffered choice → cached `Denied`, error. A shape blacklist refuses before the prompt. A failed write is `ShellPolicyWriteFailed` and caches nothing.
    - `composition::preflight::approve_commands` calls it after `template_rule_covers`; a `Shape` answer routes the site like a template-rule match (bytes blacklist-only), a `Bytes` answer leaves the bytes to the byte pass, which then finds the saved rule or cached decision. `user_approved`/`already_whitelisted` are adjusted for the prompted keys.
    - `cli/.../harness_orch/shell_options.rs`: `prompt_handler(can_prompt)` extracted (the installed `md` prompt, `None` without a terminal); no behavior change.
- **Library decision paths verified, unchanged:** `compose_preflight_approvals` (`decide_site`/`decide_entry`), `prepare_resolved`, and `site.rs::approve_template_only` already persist every §8 row, refuse unoffered choices, and return `PolicyIo` on a failed write (Phase 3 and Shared Validation). `validate_and_approve_template_site` likewise.

#### Departures and interpretations

- **Gap fixed outside the plan's file list:** before this change Claudine never offered the shape for a known-bytes single-document site (it went straight to the byte prompt with `OfferedChoices::resolved_bytes()`), so AC8's "single document included" held only for `md`. `prompt_resolved_site` closes it; `composition/preflight.rs` was edited for the call.
- **Numbering is dynamic**, not fixed keys. A hole-free resolved command keeps 1–5 exactly as before (exact, prefix, once, deny, blacklist); a known-bytes template gains "shape" as 2, shifting the rest by one.
- **Chain display:** the exact and blacklist lines do not print `normalized_exact` for a chain (it is joined with `&&` even for `||`, the Phase 1 display drift); they say a rule is saved per command.
- **`level1_dry_run_pty`** waited for the old `(persists to blacklist)` tail; it now waits for `the blacklist; stops the run)`.

#### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| AC8 `md` prompt: `{{ name: type }}` and `Resolved:` for a known-bytes template | `darkmatter-cli approval::tests::a_template_candidate_shows_each_placeholder_with_its_name_and_type` |
| AC8/AC17 validator disclosure | `approval::tests::the_broad_string_approval_discloses_the_original_validator`; `darkmatter-cli::l1 approval_prompt::a_numeric_declaration_is_disclosed_and_saved_as_a_string_hole` |
| Exact offered only when `offered.exact`; no `Resolved:` for an unknown value | `approval::tests::the_exact_choice_is_listed_only_when_offered` |
| Shape / prefix listed only when offered | `approval::tests::{the_shape_choice_is_listed_only_when_offered, the_prefix_choice_is_listed_only_when_offered}` |
| Unlisted input refused | `approval::tests::a_number_naming_no_listed_choice_is_refused_and_asked_again`; `approval_prompt::an_unlisted_number_is_refused_until_a_listed_choice_is_given` |
| Each choice returns its decision | `approval::tests::{each_listed_choice_returns_its_decision, all_five_choices_produce_correct_decisions}` |
| Chain executables listed for prefix | `approval::tests::{a_template_chain_lists_a_prefix_rule_for_every_program, prompt_lists_chain_executables_for_chain_request}`; `approval_prompt::a_chain_lists_and_saves_a_prefix_rule_for_every_program` |
| End of input denies | `approval::tests::input_that_ends_without_an_answer_denies` |
| AC8 persistence through `md` prompt + `compose_preflight_approvals`: exact, template, prefix, once, deny, blacklist exact (known bytes); template, prefix, once, deny, blacklist template (late) | `approval_prompt::{each_choice_for_a_resolved_command_saves_its_row, each_choice_for_a_late_command_saves_its_row}` |
| AC11 single document: exact offered, shape saves `template just clean --out {{ string }}/build` | `approval_prompt::a_single_document_command_offers_the_exact_bytes_and_saves_the_shape`; `claudine-cli bin commands::wrap::harness_orch::shell_options::tests::a_single_document_command_is_shown_resolved_and_saved_as_its_shape` |
| Round trip: saved rule answers without a prompt, file unchanged, no duplicate (twice) | `approval_prompt::a_saved_rule_answers_the_next_run_without_a_prompt_and_is_not_written_twice`; `claudine composition::preflight::tests::a_saved_choice_answers_the_next_run_without_a_prompt_and_is_not_written_twice` |
| AC15 failed write → typed failure, nothing approved/cached | `approval_prompt::a_failed_write_is_a_policy_failure_and_approves_nothing` (exact, shape, prefix, blacklist); `claudine … a_failed_write_at_a_resolved_site_is_typed_and_approves_nothing` |
| Claudine persistence, known-bytes site (all six choices, user_approved) | `claudine composition::preflight::tests::each_choice_at_a_resolved_site_saves_its_row` |
| Claudine persistence, template site (shape, prefix ×2 programs, once caches the shape, deny, blacklist template, unoffered exact) | `claudine … each_choice_at_a_template_site_saves_its_row` |
| Claudine scripted prompt session: late chain shows display, no exact, prefix list, refusal, blacklist template saved | `claudine-cli … shell_options::tests::a_late_command_offers_no_exact_choice_and_refuses_an_unlisted_number` |
| No terminal: no prompt, approval-required error, nothing written | `approval_prompt::without_a_terminal_a_command_shape_is_not_prompted_for` (`md compose`); `claudine-cli::l1 shell_approval_prompt::without_a_terminal_a_command_shape_is_not_prompted_for_and_nothing_runs` (provider not launched); `shell_options::tests::without_a_terminal_no_prompt_is_installed`; `claudine … a_resolved_site_without_a_handler_is_not_prompted_and_fails_naming_its_bytes`; existing `compose_shell::test_compose_{stdin,file}_unapproved_command_fails_with_guidance` |

- **Placement.** `darkmatter/cli/tests/l1/approval_prompt.rs` (`mod approval_prompt;`) and `claudine/cli/tests/l1/shell_approval_prompt.rs` (`mod shell_approval_prompt;`); CLI processes spawn only through `CliProcessFixture::command()`. Unit tests in `approval.rs`, `shell_options.rs`, and `composition/preflight/tests.rs`. No tier markers. Every new test is under 0.4 s. The Claudine CLI test has a Windows `claude.cmd` stub; no `cfg(unix)` gate.

#### Gates (macOS)

- `just test` (darkmatter): 10097 run, 10097 passed, 10 skipped (final run, after the concurrent work). `just lint` (darkmatter): clean (one `type_complexity` in the new test fixed with a `Row` alias).
- `just test` (claudine): 9192 run, 9192 passed, 9 skipped, after the concurrent Sequence Position work settled. Earlier runs during that work failed only in its tests and in guards naming its files (`sequence_cli`, `document_environment_routes`, `composition_seams`, `test_placement`, `override_boundary_guard`, `sequence::task::tests::byte_parity`); none was in this task's files.
- `just lint` (claudine): clean. Earlier runs failed only on a `type_complexity` in `composition/sequence/preflight/mod.rs` (theirs).
- `level1_dry_run_pty` (its marker updated) passes.
- No cross-OS run: no `cfg(windows)` code and no new spawn path; the prompt is pure I/O over a reader and writer.

#### Deferred

- The sequence first-task / later-task prompt (AC11's sequence half) uses these same paths (`prompt_resolved_site` for a known-bytes first-task site, `validate_and_approve_template_site` for later tasks) once Sequence Position routes its sites through `approve_commands` or calls them directly.

#### Docs and skills

- `darkmatter/docs/inline/preflight-checks.md`: "Interactive Approval" rewritten (choices table with when-listed and what-is-saved, a rendered example, disclosure, refusal, end of input, no-terminal); the `md` prompt **planned** marker removed.
- `claudine/docs/topics/pre-flight-checks.md`: "User Prompt" rewritten to the same choices, the `{{ name: type }}` display, and the no-terminal failure; "Whitelist Files" names the new choices.
- `.claude/skills/darkmatter/compose.md` (the prompt and its seam) and `.claude/skills/claudine/cli-commands.md` (the shell pre-flight row: `prompt_resolved_site`, `prompt_handler`).

- Status: Approval Prompts complete.

### Sequence Position (Wave 7)

- 2026-10-14: implemented static task position, per-site sequence records, sequence approval by position, the late roots, task-stack `set $( … )` collection, and the step-entry late-value barrier. Runtime template dispatch of sequence task commands is left to Sequence Dispatch (seam below).

#### What was built

- **Position (C8).** `TaskPosition::{First, Later}` (`sequence/preflight/shape.rs`), computed in `Loader::walk_plan` (`TaskPosition::First.child(index)`) and threaded through `load_step_task` → `load_task` → `load_group`/`build_group` (legs take the group's position, `in_group: true`) and `load_external_task` (inherits). `TaskPosition::child` is the recursive rule a nested sequence would use (none exist today). Stored as `PreflightTask.position`; skips, retries, and resume never touch it.
- **Per-site records.** `PreflightTask.shell_sites: Vec<PreparedShellSite>` — one per `shell:` command, stack `shell` `command`/`on_error`, and stack whole-value `set` `$( … )` — with `id` (task-local, `primary[0]`, `setup[2]`, the `LateValues` key), `stage`, `kind` (`ShellSiteKind::{Command, SetValue { key }}`), `authored`, `template: Option<CommandTemplate>` (typed against the owner schema, hole timing marked), `mode: SiteMode::{ResolvedBytes { bytes }, Template}`, `position`, `source_file`, `property` (source-rooted, e.g. `tasks[1].shell`), `label`. `PreflightTask::late_sites()` yields the sites with a deferred hole.
- **Approval by position (`Loader::prepare_site`).** Hole-free → literal bytes. Otherwise: `refuse_unapprovable_reads` (approval diagnostics in the sequence approval scope, skipping `outputs`/`loop` and `group` inside a group; `err`/`timing`/`current`, `group` outside a group, unknown functions, and target-identity roots keep their errors); `parse_authored` (or `authored_shell_template` for a `set` value) + `project_types(owner schema, SEQUENCE_HOST_GLOBALS)`; `mark_late_holes` via `DeferredShellRoots::late_roots_of` with the step's derived keys (`derived_deferred_keys` over the step frontmatter); template-required = any late hole or `Later`; a template-required grammar error is `SequenceShellResolution` carrying Darkmatter's `TemplateGrammar`; every approval-time hole (and the command bytes when nothing is late) resolves in one `compose_approval_value` call, and a `null` is `SequenceShellResolution` carrying `MissingValue` — before step 1, in every task. First task with no late value → `graph.shell_commands` (bytes) and `SiteMode::ResolvedBytes`; everything else → a `TemplateRequired` `ShellCandidate` in `graph.template_candidates` (origin `ShellCommandOrigin::Frontmatter { key: property }`, owner = the canonical authoring path), approved by `approve_commands` → `validate_and_approve_template_site`.
- **Prompt documents.** `graph.prompt_references: Vec<PromptReference { path, position, params, deferred_params }>` (one per distinct reference). `params` reading a late root (directly or via a derived step value) become `deferred_params`; the others are evaluated now (`compose_approval_value`) and carried as data. `PromptReference::deferred_shell_roots()` = `SEQUENCE_LATE_ROOTS` + deferred params, `.with_placeholders_template_required()` for `Later`. `resolve_graph_shell_approvals` now discovers per reference through a `&dyn Fn(&PromptReference)` request builder and appends `graph.template_candidates`; the CLI builder (`wrap/sequence/mod.rs::approve_preflight_graph`) adds `with_deferred_shell_roots` and the params through `LayeredOverrides::data(…).apply_to` (the override-boundary guard forbids `with_data_overrides`).
- **Darkmatter.** `DeferredShellRoots::with_placeholders_template_required()` (hashed into the compose identity when set): `CandidateScope::build` makes every site with a placeholder `TemplateRequired` (no entries, no exact offer, `TemplateGrammar` on bad grammar, approval-time presence still checked). `DeferredShellRoots::late_roots_of` is now the single root-read check; a late root matches only a bare read, so `fm.group` is the document's property (a derived key still matches either way). `derived_deferred_keys` and `compose::authored_shell_template` are public.
- **Caller values.** `build_preflight_graph_with_invocation` takes `caller_setters`; `Loader::step_frontmatter` layers them between the document and the step overlay. Before this, graph preflight never saw `key=value`/`--set`, so a sequence shell command reading a caller value resolved it as absent (silently empty bytes before Phase 3; `MissingValue` after it).
- **Task-stack `set $( … )`.** `shape::collect_stack_shell_sites` (replaces `collect_stack_shell_commands`) also returns `set` values whose string starts with `$(`, condition-blind. A first-task value with no late value is fixed into a `ResolvedShellValue` (`PreflightTask.approved_set_values: Vec<ApprovedSetValue>`, its `commands()` approved as bytes) and `TaskExecution::apply_approved_command` stamps it into the parsed `RuntimeSet`. `parse_task_action_stack_with` no longer refuses a stack `set $( … )`; a `side_effect:` task's `set` still does.
- **Step barrier.** `resolve_task_late_values(task, evaluator) -> Result<LateValues, CompositionError>` (`sequence/preflight/mod.rs`, over `task.late_sites()`), failing with the new `CompositionError::SequenceShellLateValue { command, task, message, source: Box<MarkdownError> }` whose source is Darkmatter's `MissingValue`/`TemplateValueRejected` (via `HoleError::into_shell_error`; never the value). `TaskExecution::late_values()` builds the evaluator — an `EvaluationSession` over `self.state` (step values and `outputs`) with the `TaskSetup` runtime bindings (`group` from the enclosing group) wrapped in `LookupEvaluator` — and `run_stages` calls it after `verify_preflight_approvals` and before setup; a failure is `TaskStatus::Failed` at `TaskStage::Setup` with nothing run.
- **Fail-closed seam.** `TaskExecution::undispatchable_template_site` refuses (after the barrier, before any stage) a task holding a `SiteMode::Template` site that reads a late value or is a `set` value: `SequenceTaskUnsupported`, so authored text never runs. `apply_approved_command` leaves such a template stack command unstamped instead of failing early with "not approved", so the barrier's error is the one reported.
- `SequenceShellLateBinding` now means only `err`/`timing`/`current` (also beside a late root) and `group` outside a group; its message and docs changed accordingly.

#### Departures and interpretations (plan rule 2)

1. **Interim runtime bytes for later approval-time sites.** A later task's command whose values all exist at preflight is approved only by its shape, but its preflight-resolved bytes stay in `PreflightAction::Shell.commands` / `approved_stack_commands` (with their `PreflightApproval` env re-check), so today's runner keeps running the existing corpus. These bytes are **not** in any approval set and were never shown to the operator; Sequence Dispatch must replace them with `fill_template` + `run_structured` (this is the one place a shape approval currently runs interpolated bytes through the system shell). A template `set` value has no interim value and fails closed.
2. **The barrier is per task execution**, not per step: a top-level step's task is the step; for a group, each leg runs its own barrier when it starts (its `group.*` values exist only inside the leg). A serial group's second leg can therefore fail its barrier after the first leg ran. The spec's "a step fails before any of its commands" holds exactly for non-group steps.
3. **`group` outside a group stays refused** (`SequenceShellLateBinding`), because it is not readable there; inside a group it is late.
4. **A `param` that does not evaluate at preflight is treated as late** for the document's discovery rather than failing preflight, because `params` otherwise bind only at the task's turn.
5. **`DynamicCommandShape` is left as is (fail-closed), with reason.** In sequence approval the late roots are absent properties in each document's discovery (no binding view declares them), so a frontmatter `$( … )` reading `last(outputs)` interpolates leniently and its site is `TemplateRequired` without meeting `detect_dynamic_frontmatter_command_shape` (test `a_frontmatter_command_with_a_late_placeholder_is_a_template`). A relaxation was tried and reverted: only when a host *declares* the late root unavailable does the span survive, and then skipping the shape check just moves the failure to `DiscoverySiteContext::for_frontmatter` (`document_state_builder(...).build()` interpolates strictly and raises the unavailable-binding `Interpolation` error). Supporting that host shape would need the discovery state build to skip deferred keys; no current host needs it.
6. **Owner schema.** Placeholders of tasks authored in the sequence source are typed against its authored `$schema` (`schema::load_authored_schema`); a `kind: task`/`kind: group` data file has no schema, so its holes fall back to `string`.
7. The Phase 1 log's note that a task-stack `set $( … )` "fails at run time" was inaccurate: the task-stack parser refused it outright (`refuse_task_set_shell`). That refusal is lifted for stacks only.

#### Intentional test changes (each forced by the new rule)

- `sequence/preflight/tests.rs`: `outputs_in_a_shell_command_is_rejected` → `outputs_in_a_shell_command_is_a_late_template`; `per_step_state_produces_per_step_bytes` and `sequence_owned_properties_read_the_same_under_fm` (step 2 now approved by shape; bytes asserted on the step's commands); `non_identity_ctx_and_env_references_stay_legal_in_shell_commands` (absent `env.HOME`/`ctx.area` are missing values now, so the commands use `|| 'none'`).
- `sequence/task/tests.rs::group_is_unavailable_at_sequence_approval_while_doc_group_reads_the_document`: a group member's `group.label` is now a late template, not a refusal (outside a group still refused).
- CLI: `sequence_cli::sequence_rejects_a_shell_command_depending_on_outputs` → `sequence_requires_a_template_approval_for_a_command_depending_on_outputs`; `sequence_errors_cli::a_shell_command_depending_on_outputs_is_rejected` → `…_needs_a_template_approval` (both now fail approval naming the authored command); `document_environment_routes::a_later_step_runs_approved_text_only_while_its_map_leaves_it_unchanged` → `a_step_runs_approved_text_only_while_its_map_leaves_it_unchanged` (its `>>` command, in a later task, is not a valid template, so the env re-check is now exercised on the first task).
- `composition_seams` baseline: the one sequence `SubtreeCompose` site moved from `reapproval::compose_shell_text` to `reapproval::compose_approval_value` (count 1, reason updated).
- `cli/src/commands/wrap/sequence/task_run.rs`: its inline test module moved verbatim to `task_run/tests.rs` (the three new `PreflightTask` fields pushed it past the 300-line inline budget).

#### Requirement-to-test mapping

| Requirement | Tests |
| --- | --- |
| AC11 (approval half): first task resolved, later task a template; display `{{ out: string }}`, persisted shape `just clean --out {{ string }}/build`, no exact offer | `claudine composition::sequence::preflight::tests::template_position::the_first_task_is_resolved_and_a_later_task_is_a_template` |
| AC11: an `exact` rule for the later bytes does not cover it; a `template` rule does, without asking | `template_position::an_exact_rule_never_covers_a_later_task_and_a_template_rule_does`; CLI `claudine-cli::l1 sequence_shell_templates::a_later_task_is_approved_only_by_its_shape` (no TTY: fails before step 1 naming the command, recorder empty; with the template rule step 1 runs) |
| AC9 (preflight half): `last(outputs)` approved as template; without a rule and no terminal, fails before step 1 naming the command | `sequence_shell_templates::a_late_value_is_approved_as_a_template_before_step_one`, `sequence_cli::sequence_requires_a_template_approval_for_a_command_depending_on_outputs`, `sequence_errors_cli::a_shell_command_depending_on_outputs_needs_a_template_approval`, `template_position::…` (lib) |
| AC2 (first half): absent caller value fails preflight before step 1 | `sequence_shell_templates::an_absent_caller_value_fails_preflight_before_step_one`; lib `template_position::a_missing_approval_time_value_fails_preflight_in_any_task` |
| AC2 (second half): a `null` late value fails its step before any of its commands (setup included); earlier effects stand | lib `sequence::task::tests::late_values::a_missing_late_value_fails_the_step_before_any_of_its_commands_runs`; CLI `sequence_shell_templates::a_missing_late_value_fails_its_step_before_its_commands_run` |
| Invalid late value rejected without showing it | `late_values::an_invalid_late_value_fails_the_step_without_showing_it` |
| Barrier keeps values per site/hole; only late holes read | `late_values::a_present_late_value_is_kept_for_its_site` |
| Late command never runs its authored text (until dispatch) | `late_values::a_late_command_that_passes_the_barrier_never_runs_its_authored_text` |
| AC12: `err`/`timing`/`current` refused, also beside a late root | `template_position::forbidden_roots_stay_refused_even_beside_a_late_root`, `sequence_shell_templates::forbidden_roots_are_still_refused`, existing `lifecycle_late_binding_in_a_shell_command_is_rejected` |
| AC16: first group → every leg first/resolved; `group.*` reader a template even there; later group legs templates | `template_position::a_first_group_qualifies_with_every_leg_and_group_readers_are_templates` |
| AC16: position is static | `template_position::a_task_is_later_whatever_runs_before_it` |
| Indirect deferral through derived frontmatter | `template_position::a_value_derived_from_a_late_root_is_late_not_missing` |
| Task parameter derived from `last(outputs)` is late in the document; approval-time params carried | `template_position::a_late_task_parameter_is_late_in_its_document` |
| A later task's document approved only by shape | `template_position::a_later_tasks_document_is_approved_only_by_shape`; Darkmatter `shell_template_discovery::a_host_placing_commands_late_requires_every_placeholder_site_to_be_a_template` |
| `fm.<late root>` is a document value | `template_position::an_fm_read_of_a_late_root_name_is_a_document_value`, Darkmatter `shell_template_discovery::an_fm_read_of_a_late_root_name_is_a_document_value` |
| Frontmatter `$( … )` with a late hole is a template site | Darkmatter `shell_template_discovery::a_frontmatter_command_with_a_late_placeholder_is_a_template` |
| Task-stack `set $( … )` collected and approved (first: bytes; later: shape) and runs | `template_position::stack_set_shell_values_are_collected_and_approved`; CLI `sequence_shell_templates::a_task_stack_set_value_is_approved_and_runs` |
| Unchanged corpus | the whole claudine and darkmatter suites (below), with the changes listed above |

- Placement: lib tests in `sequence/preflight/tests.rs` (`mod template_position`) and `sequence/task/tests.rs` (`mod late_values`); CLI file declared in `cli/tests/l1/main.rs` as `mod sequence_shell_templates;` under `#[cfg(feature = "test-fixtures")]` (it copies `claudine-argv-recorder` and `claudine-fake-goose` into the fixture `bin`); Darkmatter tests in `lib/tests/l1/shell_template_discovery.rs`. No tier markers. Each CLI test takes under 1 s locally; lib tests under 0.1 s. Portable: the recorder and fake provider are native binaries on every OS, spawned only through `CliProcessFixture::command()`.

#### Gates (macOS)

- `just lint` (darkmatter): clean. `just test` (darkmatter): 10097 run, 10097 passed, 10 skipped.
- `just lint` (claudine): clean (one fix: a `type_complexity` alias). `just test` (claudine): 9192 run, 9192 passed, 9 skipped. Earlier runs failed only on the intentional changes listed above, the `composition_seams` baseline, the override-boundary guard (fixed by `LayeredOverrides::data`), and the inline-test budget (fixed by moving `task_run` tests).
- No cross-OS run: no `cfg(windows)` code or new spawn path was added; the new CLI tests run on every OS and are left to Package Gates (P5).

#### Seam for Sequence Dispatch (what remains)

- **Where the barrier is called:** `TaskExecution::run_stages` (`sequence/task/mod.rs`), right after `verify_preflight_approvals`; the result is the local `_late_values: LateValues`. Dispatch threads it into `run_primary`/`run_shell` and `run_stack` and passes `late_values.site(&site.id)` to `fill_template(site.template, evaluator, late)`.
- **What carries mode/template/position:** `PreflightTask.shell_sites[i]` (`PreparedShellSite`: `id`, `stage`, `kind`, `authored`, `template`, `mode`, `position`, `property`). Primary sites are in `shell:` order (`primary[i]` ↔ `PreflightAction::Shell.commands[i]`); stack sites in `collect_stack_shell_sites` order per stage, matched to parsed actions by `(stage, authored)` as `apply_approved_command` does.
- **To remove when dispatch lands:** `undispatchable_template_site` (and its call), the interim bytes for later approval-time sites in `resolve_shell_commands`/`collect_lifecycle_shell` (`PreparedSite.bytes` for `SiteMode::Template`), and the `template_site()` skip in `apply_approved_command` once template stack commands dispatch structurally. `SiteMode::ResolvedBytes` sites keep their fixed bytes.
- **Evaluator for command-entry holes:** the same construction as `TaskExecution::late_values` (session over `self.state` + `TaskSetup` bindings), against the state current at the command; a stack `set` value uses its pre-write snapshot.
- **Prompt documents at their turn:** `wrap/sequence/task_run.rs::run_prompt` → `jit::compose_step` re-discovers the document with no deferred roots and no position, so a later document's command (approved by shape at preflight) is seen as resolved bytes there. Dispatch must pass the reference's `deferred_shell_roots()` (incl. `with_placeholders_template_required` for `Later`) into that compose so membership is proved by shape and the document runs in template mode; the document's late holes are then evaluated at command entry (no `with_late_values` exists). The step barrier does not yet cover document sites: a document's missing late value fails when the document composes, after the task's `setup:` ran.
- **Step-body steps** (no executable; the sequence document's own body) are approved by Phase 1c's `compose_step` with no position or deferral; a later body step's `::shell` with a placeholder is still approved as resolved bytes there.
- `sequence/task/shell.rs:3-4` ("approved bytes are executed bytes") changes with `run_structured`.

#### Comment drift fixed

- `sequence/preflight/mod.rs` (module doc, `DiscoveredCommand`, `resolve_shell_commands`, `collect_lifecycle_shell`, the former `resolve_shell_bytes` docs now `prepare_site`), `shape.rs` (`ApprovedCommand`, `PreflightAction::Shell`, `first_target_identity_root`), `reapproval.rs` (module doc), `composition/error/mod.rs` (`SequenceShellLateBinding`, `SequenceShellApprovalChanged`), `composition/preflight.rs` (`resolve_graph_shell_approvals`), `sequence/task/mod.rs` (`verify_preflight_approvals`, `apply_approved_commands`, `run_shell`), `lifecycle/parse.rs` (`refuse_task_set_shell`), `lifecycle/bindings.rs` (module doc), `cli/.../wrap/sequence/mod.rs` (`approve_preflight_graph`, the static-preflight comment).

#### Docs and skills

- `claudine/docs/topics/pre-flight-checks.md`: the late-root **planned** marker removed; the template-mode marker narrowed to dispatch; new "The first task and later tasks" section (positions, the `build`/`clean` example, late values, missing values, the barrier, Mermaid flow).
- `claudine/docs/topics/flow-control/sequences.md`: the "a shell string referencing `outputs` … is a typed preflight error" text replaced with first/later approval, late values, missing values, the barrier, and the planned template-mode dispatch.
- `claudine/docs/topics/composition.md` ("Approval under a later document's map") and `flow-control/lifecycle.md` (stack `set $( … )`).
- `darkmatter/docs/inline/preflight-checks.md`: `with_placeholders_template_required` and `fm.<root>`.
- Skills: `.claude/skills/claudine/architecture.md` (`sequence/preflight/`), `.claude/skills/claudine/cli-commands.md` (Sequences and Lifecycle rows), `.claude/skills/darkmatter/compose.md`.

- Status: Sequence Position complete; template-mode execution of sequence task commands is Sequence Dispatch.

### Remaining dispatch integration — test map before implementation

- Existing failing input `primary {{ last(outputs) }}` is covered by `late_values::a_late_command_that_passes_the_barrier_never_runs_its_authored_text`; replace the interim unsupported expectation with recorded structured argv and output commit assertions.
- Add task-boundary tests for current mutation state in later commands, missing authorization despite a discovered candidate, and whole-chain runtime blacklist refusal before any spawn. Native runner tests cover argument boundaries, chain short circuit, direct-target refusal, timeout/cancellation and capture through the existing owned-tree epilogue.
- Lifecycle work will verify current event state, pre-write set snapshots, three loop iterations, exact-byte refusal, disabled runners and unsuppressible validation. CLI document work will verify position-aware shape membership and a late document value failing before task setup.
- These changes execute template grammar rather than read a new configuration format; the configuration robustness matrix remains Phase 2's policy coverage.

### Lifecycle Dispatch (Wave 8)

- Implemented lifecycle `shell` and whole-value `set $( … )` template execution. Each prepared site retains its owner-typed authored template beside the resolved-byte approval keys. Approval settles its mode on a record shared by lifecycle clones: exact membership runs the stamped bytes; shape membership fills the authored template; neither membership refuses execution.
- `LifecycleContext` fills ordinary holes from the current event state and `set` holes from the pre-write snapshot. Both use the prepared `ComposeContext`, retaining captured `ctx.*` and `env.*`. Host-approved task-stack sites can carry barrier values without evaluating those holes again.
- Filled lifecycle chains meet both blacklists before dispatch. Native targets are resolved for the entire chain before the first process starts; their arguments are passed directly, with shell-style `&&` / `||` short-circuit behavior and the existing lifecycle child environment and inherited streams.
- `DisabledShellRunner` refuses `run_structured` and `ensure_permitted`, as well as byte and value execution. Template authorization, missing/rejected values, blacklists, and interpreter-dependent targets are evaluation failures, so `no_error` cannot suppress them. Initialization and early-handler shell prohibitions remain in place.
- `resolve_shell_approvals` and the signal-scoped `resolve_lifecycle_shell_approvals` both collect structured candidates and settle the approved lifecycle. This covers canonical single-document preparation and the loop/retry/resume/proxy re-audit paths without inserting filled bytes into approval sets.

#### Lifecycle requirement-to-test mapping

All tests below are L1 hermetic tests, compiled by their existing declared lib target or by `cli/tests/l1/main.rs` (`test-fixtures`, enabled by `just test-cli` and ordinary area L1). No terminal, browser, live provider, or host audio is used. Native compiled recorder/provider fixtures run on all platforms.

| Requirement | Targeted tests |
| --- | --- |
| Shape reuse across three body and lifecycle loop iterations; exact iteration-1 approval refuses iteration 2 before its process starts | CLI `lifecycle_shell_templates::a_shape_approval_covers_every_loop_iteration`, `an_exact_approval_of_the_first_iteration_refuses_the_second`, `a_lifecycle_shape_approval_covers_three_iterations`, `a_lifecycle_exact_approval_refuses_iteration_two` |
| Current lifecycle state and one-argument boundaries, including spaces, semicolons, substitutions, and backticks | CLI `a_lifecycle_shell_template_passes_each_value_as_one_argument`; lib `shell_templates::a_shape_approved_shell_fills_from_the_state_current_when_it_runs` |
| `set` pre-write snapshot and subsequent mutation visibility | CLI `a_shell_reads_the_current_state_and_a_set_value_its_pre_write_snapshot` |
| Missing/rejected values remain fatal under `no_error`, expose no filled secret, and spawn nothing | CLI `a_rejected_value_fails_the_event_whatever_no_error_says`, `a_missing_value_fails_the_event_whatever_no_error_says`; lib `a_refused_value_is_unsuppressible_and_never_shown`, `a_template_set_value_refuses_a_missing_value_before_it_runs` |
| Approved exact bytes stay fixed; unapproved sites never run | Lib `an_exact_approval_runs_the_stamped_bytes_and_never_rereads_the_value`, `a_site_approved_in_neither_mode_never_runs` |
| Built-in/user runtime blacklists and interpreter targets cannot be bypassed by a shape approval or `no_error` | CLI `the_builtin_blacklist_judges_the_filled_command_after_approval`, `the_user_blacklist_judges_the_filled_command_after_approval`, `an_interpreter_target_is_refused_before_it_starts` |
| Every structured execution path honors shell prohibition | Lib `the_disabled_runner_refuses_every_structured_path` |
| Host barrier values are not evaluated again; other holes stay current | Lib `a_host_approved_site_fills_its_late_values_without_reading_them_again` |
| Nonzero exit remains a dispatch failure naming authored text | Lib `a_non_zero_exit_stays_a_dispatch_error_naming_the_authored_command` |
| Retry re-audits under its one-attempt overlay | CLI `a_retry_reaudits_the_shape_and_reads_its_one_attempt_overlay` |
| Inline-compose proxy target gets its own structured audit and overlay | CLI `an_inline_proxy_target_audits_its_shape_and_reads_the_overlay` |
| Resume re-audits after persisted frontmatter mutation, then invokes the retained session | CLI `a_resume_reaudits_the_shape_after_the_document_changes` |
| A persisted broad string shape still enforces the current owner's original constraints | CLI `a_persisted_broad_shape_keeps_the_current_owner_constraints` (positive run, document rewrite, second read/run, unchanged whitelist, no second recorder spawn) |

#### Lifecycle verification

- Initial `just test-library shell_templates`: 8 passed; initial `just test-cli lifecycle_shell_templates`: 8 passed. Both had only the existing macOS linker warning about the compact unwind table size.
- Expanded re-entry/constraint regressions are verified below. Intermediate compile attempts encountered concurrently edited sequence/Darkmatter APIs, reported immediately to the owning orchestrator. Those intermediate failures did not indicate lifecycle runtime failures.

### Sequence Dispatch — native task runner

- Added `TaskShellRunner::run_structured(&FilledChain, timeout, interrupt, live)`. Its default refuses with `UnsupportedStructured`; an injected runner cannot accidentally convert filled values back to shell text. `SystemTaskShell` resolves every executable before the first spawn, rejects unsafe/unavailable targets with typed `Target`, and passes one filled string per native argument.
- Both execution modes share the owned-tree execution helper and its single teardown/reader-settlement/flush epilogue. Structured chains retain short-circuit order, one shared deadline, cancellation, per-pipe runaway guards, and a cumulative captured-output cap. Curated redirections choose the data/status channel explicitly; stderr routed to stdout becomes captured data and stdout routed to stderr becomes status only.
- Requirement-to-test mapping (`sequence::task::shell::structured_tests`, ordinary L1 unit target, declared by `shell.rs`):
  - No shell fallback: `custom_runners_refuse_structured_commands_without_a_shell_fallback`.
  - Whole-chain target validation before any effect: `an_invalid_later_target_refuses_the_whole_chain`, `a_later_bad_target_prevents_the_first_side_effect` (invalid second target even behind `||`).
  - Argument boundaries: `direct_arguments_preserve_empty_words_metacharacters_and_unicode` includes `$(touch forbidden)`, an absolute marker attack, empty values, spaces, quotes, semicolon, pipe, Unicode, and embedded newline; asserts exact stdout and absent marker.
  - Conditional execution/capture: `direct_chains_short_circuit_and_capture_only_segments_that_run`.
  - Redirection/capture: `direct_redirections_preserve_capture_channel_contract`, `redirected_stdout_streams_as_status_and_stderr_as_captured_data`.
  - Process ownership/deadline/cancellation: `direct_deadlines_reap_descendants_and_stop_later_segments`, `direct_interrupts_reap_a_running_tree`, `direct_wait_errors_settle_output_and_reap_the_tree`, `direct_isolation_errors_fail_closed`.
  - Volume guards: `direct_chain_capture_is_bounded_across_segments`, `direct_runaway_output_uses_the_owned_tree_guard`.
- Native argv recording across all OSes is additionally exercised by the orchestration CLI tests; the library's filesystem/process probes use Unix `#!` scripts where required and the unsupported-runner/invalid-target tests are portable. No Windows Job Object or Unix process-group implementation changed. No formatting, Git staging, or commits ran.
- Targeted verification (macOS): `just test-library structured_tests` passed all 13 tests (0.485 s total; slowest 0.476 s). Initial attempts exposed in-progress integration compile errors in other Phase 4 files, which were reported to the orchestrator and resolved before the passing run. The linker emitted its existing large `__eh_frame` compact-unwind warning. Full-area tests/lint and tier coverage are coordinated by the orchestrator.
- Comment/doc review corrected `ShellValueRequest`'s old claim that no command reads its pre-write snapshot: template placeholders now do. The lifecycle guide also incorrectly implied `err`/`timing` shell holes became legal; it now retains their preparation-time refusal and distinguishes available loop/sequence-group values.

### Sequence Dispatch — task authorization and current-state integration

- Removed interim resolved bytes from later template sites. A template-mode primary command now checks the graph audit's actual `ShellPreApprovals` membership before evaluating or dispatching; graph discovery alone is insufficient. The graph retains the policy root, and a new audit clears its previous authorization before attempting approval.
- Task stack shell actions and whole-value shell `set` values receive their approved typed lifecycle site after the late barrier. Deferred holes retain `SiteValues`; ordinary primary holes read current runtime mutations, while stack actions and `set` values use the lifecycle executor's current/pre-write state respectively. No resolved bytes are inserted to manufacture template membership.
- `PromptTaskRunner::validate` runs before setup with one retained request/snapshot per task execution. Ordinary prompt parameters refresh after setup; deferred parameters retain their entry values. Prompt and body preparation use passive shell probes during upfront validation rather than running commands as a side effect of preflight.
- Complete filled primary chains meet built-in and user blacklists before dispatch. Target refusals retain Darkmatter's typed `TemplateTarget` cause. The native adapter shares process-tree ownership, capture, cancellation, deadlines and reader settlement with exact mode.

| Changed behavior | Exact targeted test |
| --- | --- |
| A present late value executes structured argv and commits output | `composition::sequence::task::tests::late_values::a_late_command_that_passes_the_barrier_runs_its_filled_arguments` |
| Discovery cannot authorize execution | `late_values::discovery_without_shape_authorization_cannot_run_a_template` |
| Setup mutation reaches an ordinary primary hole and downstream output | `late_values::an_ordinary_hole_reads_setup_mutations_at_command_entry` |
| Runtime blacklist on a later segment prevents every spawn | `late_values::a_blacklisted_later_segment_prevents_the_entire_chain` |
| Missing/container late values fail before setup; earlier outputs remain | existing `late_values::{a_missing_late_value_fails_the_step_before_any_of_its_commands_runs,an_invalid_late_value_fails_the_step_without_showing_it}` |

- All these tests compile in the library's existing declared test target, have no tier marker, use private temporary sources, and take under 0.1 seconds individually on macOS. The existing preflight byte assertions were corrected to expect retained authored text for later sites; only first-task sites keep resolved bytes.
- Targeted development failures were fixture mistakes (required step names omitted, the fake stack recorder prefix, and case-sensitive `Blacklisted` text), not suppressed failures. Corrected fixtures retain all assertions of effects and downstream state.

### Sequence Dispatch — prompt documents and validation probes

- Before implementation, mapped document regressions to public CLI tests: late missing or invalid document inputs must fail before task setup; late task parameters must fill one native argument; body steps must use current step values with no shell effects from sequence validation. Extended the native primary-command matrix and later stack `set $( … )` coverage at the same boundary.
- `StepComposeContext` now retains static position and deferred roots through both sequence-body composition and referenced prompt composition. Runtime JIT preparation receives only preflight document approvals; it does not grant additional bytes or shapes when a live document changes.
- Added `prepare_sequence_probe`: Phase 1c uses the ordinary preparation options with all document shell execution stages disabled. Schema input prevalidation and shell discovery/approval still happen, while the probe's composed document is discarded.
- Prompt tasks pass a passive document check before setup. Darkmatter discovery validates deferred holes against each owning document's current state and schema, including transclusions. A fresh `DocumentShellValues` snapshot travels on the retained task request into JIT composition. Template dispatch fills those holes from the snapshot; an unmatched changed deferred site fails closed instead of evaluating new late inputs. Ordinary holes remain command-entry reads. Snapshot values participate in composition identity and are never added to approval sets.
- Targeted public CLI tests in the existing declared L1 `sequence_shell_templates` module (`test-fixtures`, enabled by `just test-cli`): `a_prompt_documents_missing_late_value_fails_before_task_setup`, `a_prompt_documents_invalid_late_value_fails_before_task_setup`, `a_prompt_documents_late_parameter_is_filled_as_one_argument`, `body_steps_fill_their_current_value_without_preflight_spawns`, five `a_template_primary_*` cases (spaces, substitution syntax, quotes, newline, empty argument), and `a_later_task_stack_set_value_dispatches_its_template`. Native recorder/provider executables run through `CliProcessFixture` without terminal windows.
- Darkmatter's existing declared L1 `shell_template_execution` module adds `a_document_late_value_snapshot_survives_changes_before_dispatch` (validation sees a scalar; execution sees null but uses the checked scalar) and `a_document_site_changed_after_the_barrier_cannot_read_a_new_late_value` (same approved shape, new late expression, typed refusal).
- Initial CLI targeted run: 14/16 passed; the two positive document/body cases reached the provider fixture and exposed its missing `FAKE_DIR` setup. Corrected the fixture environment and reran. Remaining verification is recorded by the orchestrator's final gates below.
- Updated current behavior docs in `composition.md` and `flow-control/sequences.md`, and the Claudine architecture skill's sequence entry. No new configuration reader or format was introduced; the input robustness matrix does not apply to these template/runtime changes.
- Targeted CLI rerun passed: `just test-cli sequence_shell_templates` ran all 17 cases in 1.843 seconds; every test was below 1.24 seconds. This includes `a_transcluded_documents_missing_late_value_fails_before_task_setup`, which checks an included owner's missing late input before the task's setup marker can execute. Only the existing macOS compact-unwind linker-size warning appeared.

- Sequence Dispatch verification: `just test-cli sequence_shell_templates` passed all 17 tests (1.843 seconds total; each at most 1.24 seconds). The native runner target passed all 13 tests; the full-area run also passed every task `late_values` regression. Sequence Dispatch checklist marked complete after these results.
- Final `just test-library shell_template`: 11 passed (8 executor regressions plus 3 approval-mode/site regressions), each at most 0.051 s. The expanded CLI run first exposed fixture setup omissions: inline composition requires both a `prompt` property and an actual provider body update. The fixtures now supply both; the portable fake Goose marker mode supports `FAKE_INLINE_DOCUMENT`, and the proxy case asserts that target's updated body as well as the child's argument vector. No lifecycle runtime change was needed for either omission.
- Snapshot-specific targeted verification passed (`just test a_document_late_value_snapshot`, Darkmatter): 1 passed, 0.077 seconds. The full Darkmatter gate additionally passed both new snapshot tests by name (`a_document_late_value_snapshot_survives_changes_before_dispatch`, `a_document_site_changed_after_the_barrier_cannot_read_a_new_late_value`) among 10,102 passing tests, 10 skipped. Cancelled the duplicate queued changed-site-only run after confirming this coverage in the full-gate report; it executed no test and is not a failure of the regression.
- Darkmatter lint's initial new finding was one `collapsible_if` in the passive validator. Replaced it with a let-chain; final lint verification is recorded by the orchestrator. The snapshot site identity includes the owner-typed template as well as source and location, so changed constraints cannot reuse a barrier's earlier validation.

- Corrected CLI lifecycle target passed all 12 cases. Concurrent package gates inflated four combined cases beyond 5 s (their earlier runs were at most 2.65 s), so the independent shape/exact loop cases, built-in/user blacklist cases, and rejected/missing value cases were split into separate tests. The `refused_values!` invocation is the single case table: every row generates its test directly, so a new row cannot be stranded in an untested table. The persisted-constraint test retains its required two-pass read/change/read boundary.

- Final `just test-cli lifecycle_shell_templates`: **15 passed**, no slow tests; all cases between 0.740 s and 2.367 s. The retained two-pass constraint roundtrip was 2.367 s. The exact lifecycle iteration-2 refusal was 1.671 s; three lifecycle iterations were 1.919 s; retry 1.757 s; resume 1.842 s; inline proxy plus document update 1.758 s. This run directly proves every new/renamed case is compiled and selected in the L1 target. Only the existing macOS compact-unwind linker warning remains.
- `just check-tier-coverage claudine` was attempted while sequence APIs were being edited; its browser listing could not compile under those temporary errors. No stranded lifecycle test was found: the final successful L1 runs above directly list every lifecycle case. Broader package tests/lints and final frontmatter are the orchestrator's closing gates.
- Lifecycle Dispatch is complete and its plan checkbox is checked. No lifecycle-specific human decision or outstanding runtime failure remains.

### Integration corrections from the broader gates

- The first Claudine area run exposed the new inline-proxy fixture's missing write-back. The native fake provider now performs the requested edit, and the test checks both shell argv and the edited body. The lifecycle suite was split along independent process cases; its final 15 tests pass, with the slowest at 2.367 seconds.
- The second area run exposed `sequence_groups::parallel_group_members_keep_separate_provider_sessions`: the document barrier and runtime composition used separate epochs, so neither epoch carried the complete consumer map. The prompt request now retains its own captured `DocumentEpoch`, and JIT execution reuses it. This preserves the barrier's captured context and the existing isolation assertion; the assertion was not weakened.
- Moved graph template membership checks ahead of the late barrier. The unapproved-template regression now also supplies a missing late input and expects authorization refusal first, proving an unaudited site cannot evaluate its holes.
- Lint found a typed state-building error collapsed into text and a collapsible conditional in deferred discovery. The state error now retains `MarkdownError::CtxMerge` as its source; the conditional was simplified without formatting surrounding code.

### Existing ignored tests retained

No Phase 4 test is ignored or moved out of L1. The area recipes retain these existing ignored identities:

- Claudine (9): `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`; `completion_perf::{perf_compose_empty_partial_meets_target, perf_compose_long_prefix_meets_target, perf_inline_compose_empty_partial_meets_target, perf_enter_compose_partial_meets_target}`; `system_prompt_perf_bench::{bench_system_prompt_resolution_cold_and_warm, bench_resolve_and_prepare_step_by_step, bench_request_topology_probe_and_reuse, bench_raw_darkmatter_compose_passes}`. All are opt-in performance measurements.
- Darkmatter (10): `layout::page::tests::f35_6_rhythm_raw_samples`, `markdown::render_tree::build_context::tests::f35_7_link_policy_raw_samples`, `markdown::render_tree::code_renderer::tests::f23_code_surface_raw_samples` (opt-in measurements); `markdown::compose::shell_expansion::alias::tests::resolve_alias_ll` (requires a user login-shell alias); DMLS `diagnostics::frontmatter::tests::a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias` and `lsp_session::{undeclared_properties_in_block_and_multi_line_expression_values_warn_at_their_own_ranges, tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored, an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote, a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix, aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved}` (existing alias/block-scalar projection gaps).

Existing passive shipped-artifact coverage remains in `claudine_schemas` and `claudine_schemas_embedded`, including the embedded YAML/JSON controls and parity corpus; the area gates run it. New command execution tests use the normal CLI invocation with the native shipped test fixtures, private CWD/home, and dry audio. No policy-format reader changed during this remaining integration; its robustness matrix remains the Phase 2 suite.

- An isolated epoch regression run confirmed all 14 document-environment routes pass once the fixture-enabled binary is restored. The group isolation assertion then identified a second capture inside the reused epoch. The prompt request now retains both the epoch and its captured `ComposeContext`; execution reuses both instead of calling `capture_launch_context` twice. The pure body-step unit expectation was updated to omit a fresh preflight consumer, because runtime now consumes the existing approval set instead of reauthorizing the document.
- A concurrent full-test/lint run produced fixture-hook failures (document maps, stalled notification, pre-warm), because lint's `lint-transport` test build overwrote the CLI without `test-fixtures` while nextest was using its runtime binary path. Final area gates run sequentially; these failures are recorded as a verification race, not waived or used to change production behavior.

- Final isolated epoch/map regression: `just test-cli parallel_group_members_keep_separate_provider_sessions sequence_step_preparation_is_one_exact_document_epoch document_environment_routes` passed all 16 tests in 2.383 seconds (max individual 2.377 seconds). The group isolation assertion is unchanged and again sees one capture with the complete consumer set. The remaining document routes preserve their distinct captured environments across proxy, retry, resume, task shell, transclusion, and serial/parallel group boundaries.
- Metadata readback caught a line-boundary mistake in the metadata updater. It was corrected immediately; earlier phase source lists were restored exactly, duplicate accidental Phase 4 properties removed, and every requested property was reread successfully with `md get` before continuing. No specification content was changed.

### Final Phase 4 verification

- macOS, final sequential `just test --no-fail-fast` from `claudine/`: **9,245 passed**, 9 existing ignored tests, no failures, 173.450 seconds. The nine slow notices are existing suite tests; the changed lifecycle CLI tests max at 3.303 seconds, sequence CLI tests at 1.656 seconds, native runner tests at 0.464 seconds, and task late-value tests at 0.070 seconds.
- macOS, `just test` from `darkmatter/`: **10,102 passed**, 10 existing ignored tests, no failures. Both public document snapshot regressions passed in this gate. `just lint` from `darkmatter/`: passed, including its Zed WASM extension check.
- `just check-tier-coverage claudine`: **0 stranded tests**. Every new Rust test is declared by an existing target or module and selected by L1; CLI recorder fixtures use the already enabled `test-fixtures` feature.
- Current-snapshot cross checks: `BUILD_WIN=build-win-native BUILD_WSL=build-win just cross-check claudine-cli --os all --features test-fixtures sequence_shell_templates lifecycle_shell_templates`: **32/32 on Linux (`build-linux`), 32/32 on native Windows (`build-win-native`), 32/32 on WSL2 (`build-win`)**. These runs exercise real native argv recording and normal CLI orchestration, not compilation alone. The overrides intentionally produce no publishable CI receipt; no CI scheduling changed. Initial older snapshots failed only the inline-proxy fake's missing update; corrected-snapshot runs are green on every rig.
- The final epoch/map regression target passed 16/16. The lifecycle target passed 15/15; native runner target 13/13; sequence CLI target 17/17. Requirement-to-test mappings are listed with each implementation subsection above.
- Reverse-consumer compile coverage comes from the two area gates: Claudine contract/generator and Darkmatter's DMLS/CLI consumers compile alongside their libraries. No workspace-wide suite was run.
- `git diff --check`: clean. All Phase 4 plan items are checked. The specification is unchanged; feature lifecycle remains active. No `cargo fmt`, staging, commits, real providers, or focused terminal/browser windows were used.

- Final sequential `just lint` from `claudine/`: **passed**, including all nine transport guards, lifecycle documentation facet guard, and linting the catalog, library, contract, CLI and generator. No unresolved test or lint failure remains. The existing macOS compact-unwind linker note remains informational.
- Final metadata checks: plan and log agree on all 88 source files, 11 updated documents, no newly created documents, four updated skill files and the five touched packages. Every path exists; list entries are unique. `md get` successfully read every requested key, and comparison against the original frontmatter confirmed that earlier-phase properties were preserved.
- **Phase 4 implementation complete, ready for Phase 5.** No human decision is required before continuing. The remaining Phase 5 acceptance/portability corpus and Phase 6 documentation/audit tasks stay unchecked. The author retains responsibility for review closure and moving the feature.


## Phase 5

### Acceptance design before changes

- This phase extends acceptance coverage; no production behavior change is assumed. Existing Phase 4 files are already staged at entry and will remain untouched in the index. No formatting, staging, commits, interactive provider, terminal, or browser invocation is authorized.
- AC3/4/5/13/17: extend the persisted-policy corpus through public composition, with one edited control fixture per matrix cell. Invalid or nonmatching whitelist rows must fail authorization before native dispatch; invalid blacklist rows must not invent a denial. Existing store roundtrip and warning-identity tests remain the loader-level companion. Include malformed constraints with and without semicolons.
- AC6/14: use native recorder CLI tests for redirection order and grammar refusals (prefix and once); retain the existing native argument boundary, NUL refusal, and direct-target corpus. Test effects and no-spawn evidence rather than parser return shapes.
- AC2/9/11/16: extend sequence CLI coverage for null callers, invalid late primary values before setup, indirect late parameters, static skipped/group positions, and changed task values. Existing retry/resume and document snapshot tests cover retained epoch/cache boundaries. Assert exact recorded argv, earlier effects, absent provider starts on preflight refusal, and absent audio publication.
- AC1/7/8/10/12/15: retain the existing missing-value, full error-chain secrecy, prompt/persistence, loop, initialization, exact/prefix, concurrent approval, and cache separation regressions; map exact identities in the closing acceptance table.
- No configuration parser is being added or changed. The persisted line-format matrix will be observed at composition as well as its existing reader boundary. New tests stay in already declared L1 modules with `test-fixtures` enabled by the area recipes.
- Known dependencies remain explicit: nested sequences and group terminal lifecycle events do not exist; serial groups currently establish late barriers per member rather than once for the whole group. These are not claimed as passing acceptance cases.


### Execution corpus completed

- `shell_template_execution::persisted_policy_matrix_decides_public_composition` passes all 21 edited-control rows on both policy sides (42 public composition results). It distinguishes template target resolution, authorization refusal, and blacklist refusal and asserts whether the approval handler was consulted. The captured context is reused outside the matrix. Existing reader tests retain warning file/line assertions, duplicate deduplication, exact/prefix sentinels, the covers table, type provenance, and write/read/write/read.
- `shell_template_execution::once_cannot_approve_invalid_late_template_grammar` refuses all seven unsupported constructs before the once handler can answer: pipe, semicolon, variable program, variable redirect, append, input redirect, and background operation. It rejects dispatch errors as an oracle, so merely trying the missing program cannot pass.
- Added five native stream-order tests to `darkmatter-cli::l1 compose_shell_template`: `native_merge_then_null_preserves_the_original_stdout_capture`, `native_null_then_merge_discards_both_streams`, `native_stderr_null_keeps_stdout`, `native_merged_capture_receives_both_streams`, and `native_stderr_null_then_stdout_to_stderr_discards_both`. All assert actual composed output and native recorded argv. They distinguish `2>&1 > /dev/null` from `> /dev/null 2>&1`.
- Shared `test_toolkit::argv_recorder` now optionally emits `ARGV_RECORDER_STDOUT`/`ARGV_RECORDER_STDERR` after a successful record, allowing both runners to prove stream routing with native binaries on every OS. The default remains silent. Its module contract and toolkit README describe the fixture options. No configuration reader changed.
- macOS targeted `just test shell_template_execution compose_shell_template` (Darkmatter): **29 passed**, maximum 0.387 s. Existing tests prove every requested argument representation, NUL/container/missing refusals, complete secret-free error chains, runtime policy before spawn, exact/template execution selection, and result cache separation. Execution Corpus checked off immediately after this result; platform evidence belongs to Package Gates below.


### Regression found by the native corpus

- Before changing production code, added and ran three public regressions: `sequence_shell_templates::native_task_merge_then_null_retains_stderr_capture` failed with recorded `[["a b"], [""]]` instead of the stderr probe; `compose_shell_template::a_native_result_keeps_the_original_channel_after_a_later_redirect` failed with stderr in `result.stderr` instead of `result.stdout`; both `lifecycle_shell_templates::native_lifecycle_*_keeps_the_original_*_destination` cases failed with output on the opposite parent channel.
- Root cause: the shared tokenizer collapsed an earlier duplicate back to each stream's own `Capture` when a later operation nulled its sibling, losing which original destination the duplicate retained. Correct the shared channel identity and ordered display; keep the already-correct Claudine channel routing and lifecycle inherited-handle dispatch. Darkmatter's executor must handle a surviving duplicate independently of its nulled sibling rather than merging both physical pipes. Both template and resolved-byte paths share this representation.
- Regression mapping before the fix: sequence test observes downstream `outputs`, native typed-result test observes `result.stdout`/`result.stderr`, lifecycle tests observe the actual parent's stdout/stderr; all record native argv and assert the discarded probe is absent. The original redirection strings are unchanged in the tests. Existing byte-mode redirection and pipeline display suites must stay green.


### Orchestration corpus completed

- Added 17 sequence cases in the declared L1 `sequence_shell_templates` module: null caller, invalid late primary before setup/teardown, first/later group position and exact refusal, first-group `group.*` deferral, seven grammar refusals under prefix, predecessor mutation reaching a prompt task's parameter/child, and three native redirection/next-step-output cases. The existing late-output case now asserts success and both native records, including the empty-but-present prior output. Every sequence invocation asserts no audio spool was published.
- Added two native lifecycle channel regressions in `lifecycle_shell_templates`; both capture parent stdout/stderr and assert exact argv plus no audio publication. All new tests use the existing native fake provider and recorder through `CliProcessFixture`, private CWD/home and minimal PATH. No terminal or provider credentials are involved. Nextest reports no leaked child.
- Fixture corrections during development: sequence steps require `name`; task parameters belong to prompt tasks rather than shell tasks; native task probes must sit after a quiet first task to exercise required template mode. Corrected the authored fixtures without changing production validation or weakening the public assertions. One initial targeted command used the root instead of area justfile and failed recipe lookup; corrected its working directory.
- After the redirection fix, macOS `just test-cli sequence_shell_templates lifecycle_shell_templates`: **51 passed**, maximum 1.817 s, total 3.088 s. MacOS Darkmatter targeted `just test shell_template_execution compose_shell_template redirection pipeline_`: **108 passed**, maximum 1.050 s, total 1.109 s. No suppression, ignored test, timeout relaxation, or tier move was added. Orchestration Corpus checked off after this integrated result.
- Corrected the redirection enum contracts and executor comments with the behavior. Updated current Darkmatter shell and Claudine sequence docs and the Claudine skill's shared-destination invariant. The spec remains unchanged.

### Requirement-to-test mapping

Identities below are existing declared L1 modules unless explicitly called new above; the full area gates exercise them. `DM` means Darkmatter's library, `md` its CLI, and `C` Claudine. Platform-specific native cases are recorded separately below.

| AC | Passing identities / remaining boundary |
| --- | --- |
| 1 | DM `shell_template_discovery::{a_null_or_absent_value_fails_preflight_on_every_shell_surface,compose_fails_with_the_missing_value_before_any_command_runs,empty_zero_false_and_fallback_values_are_present}`; C CLI `shell_missing_values::{a_missing_value_fails_pre_flight_on_every_shell_surface,an_empty_string_value_is_present_and_runs}`. |
| 2 | C CLI `sequence_shell_templates::{an_absent_caller_value_fails_preflight_before_step_one,a_null_caller_value_fails_before_step_one_and_provider_launch,a_missing_late_value_fails_its_step_before_its_commands_run,an_invalid_late_primary_value_prevents_setup_and_teardown}`; earlier effects and absent setup/teardown records asserted. |
| 3 | DM `shell_template_policy::{exact_prefix_and_template_rules_survive_write_read_write_read,a_constraint_or_program_word_hole_line_is_skipped_with_its_file_and_line}` plus new composition matrix. |
| 4 | DM `shell_template_policy::whitelist_templates_match_on_the_argument_list`; template `quoting_and_whitespace_spellings_have_one_shape`; public composition matrix literal/extra-argument controls. |
| 5 | DM `shell_template_policy::{the_covers_table_holds_for_a_declared_placeholder,a_broad_string_declaration_is_never_covered_by_a_narrower_rule}`. |
| 6 | md `compose_shell_template::each_placeholder_fills_exactly_one_argument_of_its_word`; C CLI `lifecycle_shell_templates::a_lifecycle_shell_template_passes_each_value_as_one_argument` and `sequence_shell_templates::a_template_primary_*`. Native recordings preserve boundaries. |
| 7 | DM `shell_template_execution::a_value_its_declaration_refuses_is_rejected_without_showing_it` checks Display, Debug, and every source; md `compose_shell_template::a_refused_value_runs_nothing_and_never_appears_in_the_error`; C task `late_values::an_invalid_late_value_fails_the_step_without_showing_it`. Scope caveat: earlier document schema validation can still print its rejected value, as recorded in Phase 4; these prove the shell runtime check. |
| 8 | md `approval_prompt::{each_choice_for_a_resolved_command_saves_its_row,each_choice_for_a_late_command_saves_its_row,a_single_document_command_offers_the_exact_bytes_and_saves_the_shape}`; C preflight `each_choice_at_a_resolved_site_saves_its_row` and `each_choice_at_a_template_site_saves_its_row`. |
| 9 | C CLI `sequence_shell_templates::a_late_value_is_approved_as_a_template_before_step_one` now requires both successful tasks and full argv; `shell_approval_prompt::without_a_terminal_a_command_shape_is_not_prompted_for_and_nothing_runs`. Current late root is `outputs`, not planned `seq.outputs`. |
| 10 | C CLI `lifecycle_shell_templates::{a_shape_approval_covers_every_loop_iteration,an_exact_approval_of_the_first_iteration_refuses_the_second,a_lifecycle_shape_approval_covers_three_iterations,a_lifecycle_exact_approval_refuses_iteration_two}`. |
| 11 | C CLI `sequence_shell_templates::a_later_task_is_approved_only_by_its_shape`; C preflight `the_first_task_is_resolved_and_a_later_task_is_a_template`; md prompt single-document test above. Native recorder replaces `just` in the build/clean example. |
| 12 | Full unchanged exact/prefix/approval suites; C CLI `sequence_shell_templates::forbidden_roots_are_still_refused`; initialization parser/runtime prohibition suites. |
| 13 | md `compose_shell_template::{a_string_template_cannot_authorize_a_builtin_blacklisted_fill,a_user_blacklist_blocks_a_filled_chain_before_its_first_segment,a_template_blacklist_blocks_resolved_byte_commands_and_chains}`; C CLI `lifecycle_shell_templates::{the_builtin_blacklist_judges_the_filled_command_after_approval,the_user_blacklist_judges_the_filled_command_after_approval}`; C task `late_values::a_blacklisted_later_segment_prevents_the_entire_chain`. DM `site::acceptance_tests::a_runtime_blacklist_refuses_a_preapproved_template_with_a_cached_result` populates a native result, confirms the cache hit, then injects a blacklist update and requires denial before the cached result can be served. |
| 14 | New md `native_*`, native typed-result regression, C `native_task_*`, C `native_lifecycle_*`, seven prefix refusal cases, DM `once_cannot_approve_invalid_late_template_grammar`; existing native argv and NUL corpus; Unix no-shebang and Windows batch target refusals. |
| 15 | md `compose_shell_template::{a_shape_approval_is_reused_but_changed_values_run_fresh,exact_and_template_rules_select_their_own_execution_mode}`; C CLI `lifecycle_shell_templates::a_persisted_broad_shape_keeps_the_current_owner_constraints`; DM template `session_reservations_are_keyed_by_mode_and_policy_root`; DM `reservation_cleanup::{handler_error_notifies_a_waiter_blocked_on_the_same_command,whitelist_exact_write_failure_releases_the_rest_of_the_chain,whitelist_prefix_write_failure_leaves_no_reservation}`; C preflight `a_failed_policy_write_is_typed_and_caches_no_decision`. DM `site::acceptance_tests::{a_failed_template_write_can_be_retried_without_a_cached_approval,a_failed_template_approver_wakes_its_parked_peer}` prove recovery, retry/persistence, and a genuinely parked peer reaching native execution after the first prompt fails. |
| 16 | New C group/static-position/mutated prompt parameter cases; existing indirect late parameter/transclusion barrier cases; C CLI lifecycle retry/resume/proxy tests; DM document snapshot/site-change tests; C preflight `a_task_is_later_whatever_runs_before_it`; C task `late_values::an_ordinary_hole_reads_setup_mutations_at_command_entry`. Nested sequences, group terminal events, and atomic whole-serial-group barriers remain unmet dependencies (see below). No separate CLI skipped-predecessor case or changed-schema parse-cache boundary is claimed. |
| 17 | DM `shell_template_policy::{a_bare_property_reads_its_resolved_named_type,a_local_child_schema_types_its_own_placeholders,numeric_and_union_declarations_disclose_a_string_hole_and_keep_their_validator,values_are_validated_as_declared_without_conversion,unsupported_rule_types_never_load_even_when_identical_to_the_declaration}`; md `approval_prompt::a_numeric_declaration_is_disclosed_and_saved_as_a_string_hole`; composition policy matrix constraints with/without semicolons. |

### Dependencies that remain visibly unmet

- Nested sequences and group terminal lifecycle events have no execution surface today: **not applicable until the container surfaces land**. Their AC16 promises are unproven; do not call all 17 criteria fully satisfied.
- Serial groups currently establish late barriers for each member. This phase proves static placement and native execution of their available members; it does not prove an atomic barrier across an entire serial group.
- AC16 has a static-position unit test for predecessors that do not execute a shell, and CLI coverage for later groups and state changes. A separate skipped-predecessor CLI flow and changed-schema parse-cache boundary remain unproven; no new execution surface or container traversal was added in this acceptance phase.


### Cache and concurrent recovery acceptance

- Added three internal-entry-point regressions in the declared library unit target, `markdown::compose::shell_expansion::site::acceptance_tests`:
  - `a_runtime_blacklist_refuses_a_preapproved_template_with_a_cached_result`: runs the native test executable with `--help`, verifies native exit 0 and nonempty captured output, confirms a repeated result is cached, then requires blacklist denial on the same pre-approved template before the cache can answer.
  - `a_failed_template_write_can_be_retried_without_a_cached_approval`: a directory at the whitelist path forces a real persistence failure; removing that obstruction permits the same runtime to approve, persist, and execute natively. A later repeat preserves the single saved row.
  - `a_failed_template_approver_wakes_its_parked_peer`: the first handler waits for a peer to park on the same shape key, then fails. The notified peer must approve and execute the native program. Threads synchronize on channels and the actual parked-peer condition, not a fixed sleep or an assumed overlap; both handles are joined.
- macOS `just test acceptance_tests`: **16 passed**, including all three new cases (0.031–0.056 s). The first compile caught an incorrect `ShellStatus` module import; corrected it to the executor module. No production change was needed for these tests.
- First broader Darkmatter gate (before these last three unit tests): **10,110 passed**, 10 existing ignored, 67.138 s. Four pre-existing DMLS schema-discovery tests exceeded 5 s under this gate's load; the new tests remained below 5 s. The final gate below includes the final three tests as well.


### Validation gate in progress

- Final macOS Darkmatter `just test`: **10,113 passed**, 10 existing ignored, 76.123 s. Five existing slow notices remain in DMLS schema discovery; no changed/new test exceeded 5 s. Full Claudine tests and sequential lint checks follow.
- Native cross-platform commands use the existing `cross-check` snapshot transport with `GIT_TERMINAL_PROMPT=0`; configured `BUILD_LINUX=build-linux`, and SSH-config-verified overrides `BUILD_WIN=build-win-native`, `BUILD_WSL=build-win`. Windows and WSL legs run sequentially. Filters/features intentionally narrow these to acceptance evidence; no new CI cell or evidence-reuse policy is introduced, and no publishable outgoing-head receipt is claimed. The real index/branch are not staged or committed by these snapshot checks.

- darkmatter final area gate: all tests passed. Acceptance corpus maximum: (0.548, 'darkmatter-cli::l1 compose_shell_template::a_refused_value_runs_nothing_and_never_appears_in_the_error'). Existing tests over 5 s (not suppressed):
  - `dmls::l1 claudine_schema_discovery::lifecycle_stacks_and_linking_keys_are_typed_by_the_discovered_trigger`: 5.924 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_items_and_values_complete_without_a_schema`: 7.607 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_properties_hover_without_a_schema`: 8.748 s.
  - `dmls::l1 claudine_schema_discovery::trigger_payload_keys_complete_without_a_schema`: 8.808 s.
  - `dmls::l1 claudine_schema_discovery::an_authored_redeclaration_wins_over_the_trigger_per_property`: 8.871 s.

- claudine final area gate: all tests passed. Acceptance corpus maximum: (3.262, 'claudine-cli::l1 lifecycle_shell_templates::a_persisted_broad_shape_keeps_the_current_owner_constraints'). Existing tests over 5 s (not suppressed):
  - `claudine::l1 claudine_schemas_embedded::embedded_json_action_communicate`: 5.052 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_json_action_initialize_bundle`: 5.263 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_json_action_shell`: 5.322 s.
  - `claudine::l1 claudine_schemas_embedded::disk_bundle_resolves_its_relative_imports_from_disk`: 5.961 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_json_linking_user_invocable`: 5.549 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_lifecycle_exports_resolve_without_disk_schemas`: 5.569 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_json_payload_property`: 8.132 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_bundle`: 6.432 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_flow`: 7.022 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_expression`: 7.318 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_communicate`: 7.625 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_initialize_bundle`: 7.393 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_action_shell`: 7.533 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator`: 11.498 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_lifecycle_loop`: 5.401 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_linking_user_invocable`: 5.592 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_payload_property`: 5.311 s.
  - `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator`: 7.752 s.
  - `claudine::l1 claudine_schemas_matrix::yaml_action_initialize_bundle`: 5.153 s.
  - `claudine::l1 claudine_schemas_matrix::yaml_payload_property`: 5.262 s.
  - `claudine::l1 claudine_schemas_matrix::yaml_linking_user_invocable`: 5.78 s.
  - `claudine-cli::l1 agent_text_is_data::inline_agent_values_are_stored_as_data_and_read_back_exactly`: 5.315 s.
  - `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text`: 8.965 s.
  - `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell`: 6.362 s.
  - `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory`: 5.511 s.
  - `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive`: 7.442 s.
  - `claudine-cli::l1 contamination_probes::an_uncontaminated_run_matches_the_expectations_every_probe_asserts`: 5.09 s.
  - `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents`: 5.905 s.
  - `claudine-cli::l1 loop_cli::compose_loop_rate_limit_pause_waits_then_continues`: 5.387 s.
  - `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface`: 5.296 s.
  - `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail`: 5.062 s.
  - `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known`: 5.943 s.

- Final macOS Claudine `just test`: **9,264 passed**, 9 existing ignored, 184.902 s. The acceptance corpus maximum was **3.262 s**, so every new/changed acceptance test stayed below 5 s. The suite emitted 31 slow notices; the explicit over-5-s list above contains 32 passing pre-existing tests. These are reported, not retiered, ignored, retried, or treated as acceptance performance evidence.
- macOS Darkmatter `just lint`: **passed**, including library, CLI, DMLS, Zed CLI and the Zed WASM extension compile. No formatter was invoked by these lint recipes.
- Frontmatter readback caught two missing newline separators in the handoff updater. Repaired them immediately, reread every requested property with `md get`, then compared all earlier-phase properties with the entry index snapshot through `md get`; all earlier values are preserved. No duplicate keys or parse error remains. The specification was not edited because this phase request names no spec properties to change.


### Exact new test inventory

- `darkmatter/lib/tests/l1/shell_template_execution.rs`:
  - `once_cannot_approve_invalid_late_template_grammar`
  - `persisted_policy_matrix_decides_public_composition`
- `darkmatter/lib/src/markdown/compose/shell_expansion/site.rs`:
  - `a_failed_template_approver_wakes_its_parked_peer`
  - `a_failed_template_write_can_be_retried_without_a_cached_approval`
  - `a_runtime_blacklist_refuses_a_preapproved_template_with_a_cached_result`
- `darkmatter/cli/tests/l1/compose_shell_template.rs`:
  - `a_native_result_keeps_the_original_channel_after_a_later_redirect`
  - `native_merge_then_null_preserves_the_original_stdout_capture`
  - `native_merged_capture_receives_both_streams`
  - `native_null_then_merge_discards_both_streams`
  - `native_stderr_null_keeps_stdout`
  - `native_stderr_null_then_stdout_to_stderr_discards_both`
- `claudine/cli/tests/l1/sequence_shell_templates.rs`:
  - `a_first_group_offers_resolved_bytes_for_every_member`
  - `a_group_value_is_late_even_in_the_first_group`
  - `a_later_group_keeps_template_mode_for_every_member`
  - `a_later_tasks_parameter_reads_the_mutation_from_its_predecessor`
  - `a_null_caller_value_fails_before_step_one_and_provider_launch`
  - `a_prefix_cannot_authorize_a_background_template`
  - `a_prefix_cannot_authorize_a_pipe_in_a_template`
  - `a_prefix_cannot_authorize_a_semicolon_in_a_template`
  - `a_prefix_cannot_authorize_a_variable_program_word`
  - `a_prefix_cannot_authorize_a_variable_redirect_target`
  - `a_prefix_cannot_authorize_an_append_redirect`
  - `a_prefix_cannot_authorize_an_input_redirect`
  - `an_invalid_late_primary_value_prevents_setup_and_teardown`
  - `exact_rules_cannot_authorize_a_later_groups_members`
  - `native_task_merge_then_null_retains_stderr_capture`
  - `native_task_null_then_merge_discards_both`
  - `native_task_stderr_null_retains_stdout_capture`
- `claudine/cli/tests/l1/lifecycle_shell_templates.rs`:
  - `native_lifecycle_merge_then_null_keeps_the_original_stdout_destination`
  - `native_lifecycle_route_then_null_keeps_the_original_stderr_destination`

Total: 30 new tests, each compiled by its existing declared L1 target. Macro invocations generate their own tests directly; there is no separate table whose rows can be added without generating a test.

- macOS Claudine `just lint`: **passed**, including all nine transport guards, lifecycle facet documentation guard, catalog, library, contract, CLI and generator. The existing compact-unwind linker note remains informational. Lint and full test gates ran separately, so fixture-enabled runtime binaries were not overwritten by lint during a suite.
- The two area gates compile affected reverse consumers: Claudine contract/generator, Darkmatter CLI/DMLS/Zed consumers, and the shared recorder bins. No public API signature or dependency was added, and no workspace-wide test suite was run.

- macOS toolkit checks: `just _test test-toolkit argv_recorder` **2 passed** (0.016 s); `just _lint test-toolkit` **passed**. Native probe emission is exercised through the two CLI suites, not an implementation-mirroring unit test.
- `just check-tier-coverage claudine`: **0 stranded tests**. New tests live in already declared unit/L1 modules; the recorder bins use the existing `test-fixtures` feature. No target or feature registration changed.
- `GIT_TERMINAL_PROMPT=0 BUILD_WIN=build-win-native BUILD_WSL=build-win just cross-check darkmatter-cli --os all --features test-fixtures compose_shell_template`: **15/15 on Linux (`build-linux`, 0.197 s), 15/15 on native Windows (`build-win-native`, 1.369 s), 15/15 on WSL2 (`build-win`, 0.284 s)**. The same cases passed in the macOS area gate. These are native execution runs, including argv representations, redirect routing, typed-result channel retention, NUL/container refusal, and platform-specific interpreter-target refusal; not cross-compilation or archived-run evidence.

- `just check-tier-coverage darkmatter`: **0 stranded tests**. Both area tier audits pass; all new identities are live L1 tests, including the inline library unit tests.
- `GIT_TERMINAL_PROMPT=0 BUILD_WIN=build-win-native BUILD_WSL=build-win just cross-check claudine-cli --os all --features test-fixtures sequence_shell_templates lifecycle_shell_templates`: **51/51 on Linux (`build-linux`, 2.064 s), 51/51 on native Windows (`build-win-native`, 7.006 s), 51/51 on WSL2 (`build-win`, 3.451 s)**. The same 51 identities passed locally. Native argument records, downstream `outputs`, inherited parent channels, group positions, no-terminal refusals, missing/invalid late barriers, runtime policy, retry/resume, interpreter refusal, and no audio publication all ran through the normal CLI. Native execution mode is explicit; this is not archive portability evidence.
- Native md linux maximum individual test: `darkmatter-cli::l1 compose_shell_template::a_refused_value_runs_nothing_and_never_appears_in_the_error`, 0.195 s.
- Native md windows maximum individual test: `darkmatter-cli::l1 compose_shell_template::exact_and_template_rules_select_their_own_execution_mode`, 1.361 s.
- Native md wsl maximum individual test: `darkmatter-cli::l1 compose_shell_template::a_refused_value_runs_nothing_and_never_appears_in_the_error`, 0.267 s.
- Native claudine linux maximum individual test: `claudine-cli::l1 lifecycle_shell_templates::a_persisted_broad_shape_keeps_the_current_owner_constraints`, 1.207 s.
- Native claudine windows maximum individual test: `claudine-cli::l1 lifecycle_shell_templates::a_persisted_broad_shape_keeps_the_current_owner_constraints`, 2.703 s.
- Native claudine wsl maximum individual test: `claudine-cli::l1 lifecycle_shell_templates::a_persisted_broad_shape_keeps_the_current_owner_constraints`, 1.531 s.


### Final Phase 5 verification and handoff

- `GIT_TERMINAL_PROMPT=0 BUILD_WIN=build-win-native BUILD_WSL=build-win just cross-check darkmatter --os all --features effects-instrumentation shell_template_execution shell_template_policy site::acceptance_tests`: **37/37 on Linux (`build-linux`, 0.065 s), 37/37 on native Windows (`build-win-native`, 1.163 s), 37/37 on WSL2 (`build-win`, 0.271 s)**. All three cache/persistence/concurrent-peer regressions reached actual native execution on every OS. The public composition policy matrix, complete error-chain secrecy, original validators, provenance/covers, roundtrips, and target refusals passed too.
- Final platform evidence (native execution, no cross-compilation substituted):

| Environment | Darkmatter CLI `compose_shell_template` | Claudine CLI `sequence_shell_templates` + `lifecycle_shell_templates` | Darkmatter library selected policy/execution/site corpus |
| --- | --- | --- | --- |
| macOS, current worktree | 15 passed in area L1 | 51 passed in area L1 | 37 passed in area L1 |
| Linux, `build-linux` | 15 passed | 51 passed | 37 passed |
| Native Windows, `build-win-native` | 15 passed | 51 passed | 37 passed |
| WSL2, `build-win` | 15 passed | 51 passed | 37 passed |

- All **30 new tests** have passing evidence on all four operating systems. All added/changed acceptance tests stayed below 5 s on every rig. Existing slow/ignored tests are listed above and in Phase 4; no failure was suppressed and no assertion was weakened to obtain a pass.
- Both required area `just test` and `just lint` gates pass. Toolkit targeted tests/lint pass. Both area tier audits report **0 stranded tests**. No unresolved test or lint failure remains. The requirement table explicitly preserves the unimplemented container surfaces and the separate skipped-predecessor/changed-schema cache coverage limits; these are not recast as passing tests.
- Package Gates checked off after collecting all required platform results. All three Phase 5 checkboxes are complete; Phase 6 remains unchecked. Implementation is complete for the available traversal surfaces and ready for the documentation/audit phase, with the dependencies above retained. No human design decision is needed before Phase 6.
- Plan/log metadata identifies nine changed Rust files, three updated topic/overview documents, no new documents, one updated skill, and the accumulated five-package set. Every listed path exists. The specification, feature lifecycle location, real Git index, and branch are preserved. No `cargo fmt`, real provider, focused terminal/browser, staging, branch commit, push, or CI scheduling change was performed.

- Cross-check md snapshot/run banner (the source tree tested, not a branch commit):
  - `cross-check: darkmatter-cli @ origin/feat/osc-9-4-protocol (c52e342ba) + 2645 changed file(s) as b28a8d053`
  - `os: linux(build-linux) windows(build-win-native) wsl(build-win)   extra nextest args: --features test-fixtures compose_shell_template`
  - `mode: native — --features test-fixtures overrides the plan's declared features, so no`
  - `archive can carry the key this run would report; nothing here is publishable.`
  - `clone: build-linux:/home/build/coding/shazam--feat-osc-9-4-protocol`
  - `clone: build-win-native:B:\coding\shazam--feat-osc-9-4-protocol`
  - `clone: build-win:/home/ken/coding/shazam--feat-osc-9-4-protocol`

- Cross-check claudine snapshot/run banner (the source tree tested, not a branch commit):
  - `cross-check: claudine-cli @ origin/feat/osc-9-4-protocol (c52e342ba) + 2645 changed file(s) as 909e6c7f4`
  - `os: linux(build-linux) windows(build-win-native) wsl(build-win)   extra nextest args: --features test-fixtures sequence_shell_templates lifecycle_shell_templates`
  - `mode: native — --features test-fixtures overrides the plan's declared features, so no`
  - `archive can carry the key this run would report; nothing here is publishable.`
  - `clone: build-linux:/home/build/coding/shazam--feat-osc-9-4-protocol`
  - `clone: build-win-native:B:\coding\shazam--feat-osc-9-4-protocol`
  - `clone: build-win:/home/ken/coding/shazam--feat-osc-9-4-protocol`

- Cross-check library snapshot/run banner (the source tree tested, not a branch commit):
  - `cross-check: darkmatter @ origin/feat/osc-9-4-protocol (c52e342ba) + 2645 changed file(s) as 5f4f2c410`
  - `os: linux(build-linux) windows(build-win-native) wsl(build-win)   extra nextest args: --features effects-instrumentation shell_template_execution shell_template_policy site::acceptance_tests`
  - `mode: native — --features effects-instrumentation overrides the plan's declared features, so no`
  - `archive can carry the key this run would report; nothing here is publishable.`
  - `clone: build-linux:/home/build/coding/shazam--feat-osc-9-4-protocol`
  - `clone: build-win-native:B:\coding\shazam--feat-osc-9-4-protocol`
  - `clone: build-win:/home/ken/coding/shazam--feat-osc-9-4-protocol`


## Phase 6

### Scope and success criteria

- Documentation and audit only: preserve runtime behavior, reconcile current docs and symbol comments with the existing public tests, audit all nine requirements and 17 acceptance criteria, and rerun the required local Claudine area tests and lint sequentially. No parser, schema, template artifact, prompt, configuration loader, or persisted format changes are planned; the input robustness matrix and shipped-artifact test additions are not triggered by prose/comment corrections.
- Read the Claudine, Darkmatter, Just, and Rust testing skills. Preserve prior changes and the existing index; no staging, commits, formatting, lifecycle move, new CI cell, or real-provider invocation.
- Phase 5's requirement-to-test table and native four-OS records remain evidence for unchanged runtime behavior. New Phase 6 execution evidence is recorded below separately from that inherited evidence.

### Documentation audit in progress

- Updated both area README overviews, both preflight topics, sequence/lifecycle topics, and both area skills. Added compact covers/argument matching examples, canonical policy and cache rules, direct-dispatch grammar, diagnostic scope, and an approval-to-runtime Mermaid diagram with first-task, step-entry, and command-entry branches.
- Detected drift: Claudine preflight docs described sequence compositions as cached upfront; current execution composes just in time. Removed that claim. Both preflight docs also described execution membership as sufficient by itself; clarified the runtime validation, blacklist, and target gates.
- Newly identified departure from section 10: first-task primary `shell:` and task setup/teardown sites with known values are recorded only as resolved bytes by `sequence/preflight::prepare_site`; they are absent from `template_candidates`. `resolve_graph_shell_approvals` therefore cannot offer the optional first-task shape choice, unlike first-task prompt-document discovery. Existing `the_first_task_is_resolved_and_a_later_task_is_a_template` asserts this split. This phase documents the limitation and does not claim it is fixed; restoring that choice requires a behavioral change and dedicated native task/stack regression coverage.

- Public Documentation checked off after all named pages and both skills were updated. Corrected source-comment drift in `sequence/preflight/{shape,reapproval}.rs` and `sequence/task/mod.rs`: only byte-mode sites retain approved stack commands, later ordinary-value sites are templates, prompt composition checks earlier grants, and redirected stderr can contribute to stdout. These are comment-only source edits; no runtime branch or test changed. Dependency docs need no further change because Phase 6 adds/removes no crate.
- First local Claudine `just test`: **9,264 passed**, 9 existing ignored, 150.709 s (four slow notices). This run preceded the comment corrections; the final gates below include them.


### Acceptance audit

This is an audit of existing behavior, not an assertion that documentation fixes missing behavior. The Phase 5 17-row table above supplies exact test identities; this table records the final assessment and qualifications after reading the implementation. No tests were added or renamed in Phase 6.

| AC | Audited observable evidence | Assessment |
| --- | --- | --- |
| 1 | DM `shell_template_discovery` and Claudine CLI `shell_missing_values`: absent/null refuse every shell surface before dispatch; empty/zero/false/fallback controls | Covered on available surfaces |
| 2 | CLI `sequence_shell_templates`: absent/null caller, missing late value, invalid late primary; earlier effects survive and setup/teardown markers stay absent | Covered using current `outputs` root |
| 3 | `shell_template_policy::exact_prefix_and_template_rules_survive_write_read_write_read`, both robustness matrices, public composition matrix | Covered; repeated persistence and malformed-row warnings |
| 4 | `whitelist_templates_match_on_the_argument_list`, canonical quoting/whitespace tests | Covered; argument count, literal, position, operator, redirect-order controls |
| 5 | `the_covers_table_holds_for_a_declared_placeholder`, broad-declaration refusal | Covered; original validator is separate from approval type |
| 6 | Native md, lifecycle, and task argv recorders | Covered on all four OSes in Phase 5; values are data and adjacent holes retain one word |
| 7 | Library Display/Debug/source-chain secrecy test, CLI refusal and task late-value refusal | Shell runtime covered; earlier document-schema diagnostics may disclose rejected values |
| 8 | md `approval_prompt`, Claudine preflight choice tests, no-terminal CLI refusal | Covered for document/later-task candidates; optional first-task primary/stack shape choice missing |
| 9 | CLI late-template success and no-terminal/no-rule refusal before step 1 | Covered with `outputs`; `seq.outputs` is not implemented |
| 10 | CLI lifecycle loop shape succeeds three times; exact refuses iteration two | Covered |
| 11 | Preflight build/clean test and CLI later-task native recording | First/later execution split covered; optional first-task primary/stack shape offer remains absent |
| 12 | Existing exact/prefix corpus, forbidden-root and initialization prohibition suites | Covered; full area gates preserve existing behavior |
| 13 | Filled chain blacklists, native cached-result blacklist denial, lifecycle builtin/user denial | Covered; every segment checked before first spawn and cache reuse |
| 14 | Native argv/redirect corpus, invalid grammar under prefix/once, NUL/container and interpreter refusals | Covered on all four OSes in Phase 5 |
| 15 | Fresh result after changed fill, changed constraints, mode/root keys, persistence retry and parked-peer recovery | Available paths covered; separate changed-schema parse-cache test remains unproven |
| 16 | Mutated params, child docs, late task set, later/first groups, static position, retry/resume/proxy | Partial: nested sequences/group terminal events absent; serial barriers per member; no separate skipped-predecessor CLI case |
| 17 | Named/local-child schema, fallback projection, original scalar validation, numeric/union prompt disclosure, invalid rule types | Covered |

All nine requirements were checked independently of the AC table:

| Requirement | Implementation and concrete test evidence | Qualification |
| --- | --- | --- |
| R1 Store | `store.rs`, canonical shape parsing; policy roundtrip/warning/matrix tests | Invalid lines are skipped with warnings; duplicate parsed rules deduplicate |
| R2 Matcher | `template/matching.rs`, whole filled-chain policy; argument-list, covers, runtime blacklist tests | Blacklist holes ignore types; single-segment deny rules inspect every chain segment |
| R3 Types | `template/typing.rs` projects owning effective schema; named/child/numeric/union tests | Non-bare expressions use broad scalar fallback |
| R4 Runtime fill | Shared `template/fill.rs`, `site.rs`, task and lifecycle structured adapters; native argv and redirection tests | Serial-group barrier is per member |
| R5 Missing | `HoleError::Missing` becomes typed `MissingValue`; discovery and late CLI refusal tests | Sequence wrappers preserve typed cause and authored site |
| R6 Sequence preflight | `prepare_site` static position, deferred roots and graph membership; first/later, late group and forbidden-root tests | First-task primary/stack optional shape choice missing; containers not present |
| R7 Docs | Both preflight pages, sequence/lifecycle pages, both READMEs and skills | Current limitations stated inline; no topic delegates behavior to a spec snapshot |
| R8 Transport/caches | `ShellPreApprovals`, `ApprovalKey`, authored site storage, JIT membership, per-execution late snapshots; mode/root/cache and source-owner tests | Dedicated changed-schema cache boundary remains unproven |
| R9 Persistence | Existing root resolution and canonical parsed shapes; failed write, retry, duplicate and parked-peer tests | Failed writes grant nothing; no runtime values saved in template rows |

### Robustness and bypass review

The existing policy format is line-oriented text, not YAML/JSON. The full keyword/payload row and column were reviewed on both whitelist and blacklist sides rather than inferring behavior from a single malformed line. Existing `the_whitelist_robustness_matrix`, `the_blacklist_robustness_matrix`, and `persisted_policy_matrix_decides_public_composition` cover these outcomes:

| Input shape | Whitelist keyword | Whitelist shape payload | Blacklist keyword | Blacklist shape payload |
| --- | --- | --- | --- | --- |
| Control | authorizes matching shape | authorizes matching shape | denies matching shape | denies matching shape |
| Absent | malformed warning; no grant | malformed warning; no grant | malformed warning; no deny | malformed warning; no deny |
| `null` text | unknown keyword warning | `template null` is a valid fixed-program rule, not the control shape | unknown keyword warning | valid fixed-program rule; no control denial |
| Wrong whole field/type | malformed warning | malformed collection warning | malformed warning | malformed collection warning |
| Wrong one hole | not applicable to single keyword | malformed warning; no grant | not applicable to single keyword | malformed warning; no deny |
| Wrong every hole | not applicable to single keyword | malformed warning; no grant | not applicable to single keyword | malformed warning; no deny |
| Empty | malformed warning | malformed warning; blank file has no rules | malformed warning | malformed warning; blank file has no rules |
| Duplicate rule | parsed rule deduplicates | parsed rule deduplicates | parsed rule deduplicates | parsed rule deduplicates |
| Trailing content | unknown-keyword warning | invalid syntax warns; valid extra argument changes match; bad following line warns without removing good rule | unknown-keyword warning | same parse rules; a good earlier deny remains |

- Both matrix walkers retain exact/prefix sentinels, report warning file/line, and exercise both policy decisions. The composition matrix's 21 rows on both policy sides prove authorization/denial through public composition to a deliberately unavailable target; it does not claim native-spawn evidence. Native-spawn evidence is the separate recorder corpus.
- Read `site::prepare_site` and `TemplateSite::prepare_template`: authored template parsing precedes filling; template membership precedes filling; all holes retain their positions; `check_filled_chain` and all target resolution precede result-cache access. Resolved-byte parsing is intentional only in byte mode. The three mode/cache/persistence acceptance tests in Phase 5 reach native execution, including a genuinely parked concurrent peer.
- Read sequence `prepare_site`, graph approval, task `authorized_template`/`prepare_primary_template`, task barrier, and JIT request transport. Later tasks retain authored templates, current mutations feed ordinary holes, late holes retain the task-entry snapshot, and empty/unrecorded graph authorization fails closed. No audit-time shell obtains late values. Prompt document barriers and JIT share their document epoch.
- Checked `.ok()`, `unwrap_or_default`, tokenization, and environment-read sites in the adapters. `typed_template(...).ok()` is the optional candidate path for byte-eligible lifecycle commands, not swallowed runtime validation; template-required sequence grammar returns a typed error. Structured lifecycle cwd uses the existing captured launch-directory helper. No ambient environment read, filled prepared-state reuse, or invented exact grant was introduced by this phase.
- Typed diagnostics remain `ShellExpansionError::{MissingValue,TemplateValueRejected,TemplateGrammar,TemplateTarget,PolicyIo}` underneath document errors; sequence and lifecycle wrappers retain their sources. Phase 5's secrecy test traverses Display, Debug, and every source for runtime validation. This does not establish secrecy for the earlier schema verdict.
- No parser or configuration reader changed in Phase 6. Existing shipped-schema corpus and normal CLI tests run in the area gate. No passive artifact, end-to-end, roundtrip, or matrix test was added merely to mirror prose edits.

### Portability evidence and outstanding work

- Runtime changes stopped in Phase 5. Reuse its explicitly native records: macOS area gates, Linux `build-linux`, native Windows `build-win-native`, and WSL2 `build-win`; md CLI 15, Claudine CLI 51, and Darkmatter policy/execution/site corpus 37 passing identities per OS. These include all 30 Phase 5 tests. No new OS branch, path comparison, target resolution, or process behavior changed here, so no remote rerun is warranted for Phase 6 comment/prose edits.
- Remaining work is explicit: optional first-task primary/stack shape approval; atomic whole-serial-group barriers; nested sequences and group terminal events when those execution surfaces exist; dedicated skipped-predecessor CLI and changed-schema parse-cache coverage. The output namespace is still `outputs`, and document-schema rejection is outside the runtime secrecy guarantee. Documentation does not close these gaps. Do not report all clauses of the 17 criteria as satisfied.
- Acceptance Audit checked off after recording all requirements, criteria, matrix states, bypass checks, diagnostic scope, portability evidence, and departures. No behavioral repair was attempted in this documentation phase; each missing behavior remains an actionable review finding.


### Gate interference and recovery

- The second full run reached **6,396/9,264 tests: 6,393 passed, three failed, nine existing ignored**, then nextest fail-fast left 2,868 unrun. Failures: `sequence_initialize_include_preflight::{a_direct_compose_missing_include_carries_no_sequence_note,a_dry_run_reports_the_same_preflight_failure_without_side_effects,a_prompt_whose_initialize_would_create_its_include_fails_sequence_preflight}`. Each was missing the fixture-only diagnostic snapshot (`None` instead of `composition.failed`); the command refusal and no-side-effect behavior still occurred.
- Cause: I ran `just check-tier-coverage claudine` while the suite was active. Its `cargo nextest list` rebuilds the default-feature CLI and overwrote the fixture-enabled binary. Verified the recipe uses no `test-fixtures` feature and the snapshot writer in `cli/src/main.rs` is feature-gated. This is the same build-output interference Phase 5 warned about for lint. Added the tier-listing variant to the Claudine skill; all build/list/test/lint operations are now sequential.
- After the tier audit finished, `just test sequence_initialize_include_preflight`: **5 passed**, 1.285 s. No code/assertion changes, retries configuration, ignores, or suppressed failures. This is a deliberate recovery run after restoring the required binary feature build, not a claim that the failed gate passed. An isolated full area gate follows.
- `just check-tier-coverage claudine`: **zero stranded tests**. It completed successfully before the recovery run; it will not be repeated concurrently.
- The single-property `md get spec.md implemented` command prints scalar `true`, unlike a multiple-key query's object. A verification helper initially assumed an object after both plan/log validations had already succeeded; corrected the readback and confirmed scalar `true`. No malformed frontmatter occurred.


### Skips and test scope

- No new or renamed test in Phase 6; no behavior or configuration input changed. Requirement-to-test mapping is the AC/R tables above plus Phase 5's exact identities. Existing public observable regressions, corpus controls, persistence roundtrips, and four-OS native execution records are retained rather than replaced by implementation-mirroring tests.
- The nine existing ignored L1 identities on macOS are `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`; `completion_perf::{perf_compose_empty_partial_meets_target,perf_compose_long_prefix_meets_target,perf_inline_compose_empty_partial_meets_target,perf_enter_compose_partial_meets_target}`; and `system_prompt_perf_bench::{bench_system_prompt_resolution_cold_and_warm,bench_resolve_and_prepare_step_by_step,bench_request_topology_probe_and_reuse,bench_raw_darkmatter_compose_passes}`. They remain opt-in performance diagnostics; none was changed or newly ignored here.
- Targeted recovery selected five include-preflight tests and left 9,268 other/ignored identities unselected by its filter. The failed full run's 2,868 fail-fast omissions are recorded above; the subsequent full run, rather than that partial run, supplies the final gate evidence.
- No L2/L3, browser, real-provider, workspace-wide, or new remote run was needed for comment/documentation-only changes. No window was opened or focused. Phase 5's Darkmatter full area test/lint and four-OS evidence are reused for its unchanged runtime; Phase 6 reruns the requested Claudine area gate.


### Final validation

- Initial full gate: `Summary [ 150.709s] 9264 tests run: 9264 passed (4 slow), 9 skipped`.
  - Existing tests above the five-second L1 budget (not modified, suppressed, or retiered): `claudine::l1 claudine_schemas_embedded::embedded_json_trigger_combinator` 5.599 s; `claudine::l1 claudine_schemas_embedded::embedded_yaml_trigger_combinator` 5.400 s; `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` 5.104 s; `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known` 5.142 s.
- Interfered gate: `Summary [ 117.089s] 6396/9264 tests run: 6393 passed (8 slow), 3 failed, 9 skipped`.
  - Existing tests above the five-second L1 budget (not modified, suppressed, or retiered): `claudine-cli::l1 agent_text_is_data::malformed_structured_agent_values_are_refused_not_saved_as_text` 9.113 s; `claudine-cli::l1 compose_initialize_staged_boot::early_catch_shells_cannot_run_or_enable_another_catch_shell` 5.320 s; `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory` 5.706 s; `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` 7.565 s; `claudine-cli::l1 ctx_launch_anchor::loop_route_keeps_launch_area_for_root_and_package_documents` 5.429 s; `claudine-cli::l1 loop_cli::compose_loop_rate_limit_pause_waits_then_continues` 5.452 s; `claudine-cli::l1 provider_tail_launch::embedded_credentials_reach_the_child_and_no_display_surface` 6.388 s; `claudine-cli::l1 provider_tail_launch::other_failures_are_not_attributed_to_the_tail` 6.051 s; `claudine-cli::l1 provider_tail_notice::a_switch_whose_record_is_typed_unknown_is_not_called_known` 7.153 s.
- Final isolated full gate: `Summary [ 138.480s] 9264 tests run: 9264 passed (2 slow), 9 skipped`.
  - Existing tests above the five-second L1 budget (not modified, suppressed, or retiered): `claudine-cli::l1 compose_schema_cli::compose_late_caller_file_verdict_names_the_launch_directory` 5.522 s; `claudine-cli::l1 compose_schema_cli::compose_unresolved_caller_file_fails_before_initialize_when_non_interactive` 6.531 s.
  - Shell-template/missing-value/approval corpus maximum: `claudine-cli::l1 shell_missing_values::a_missing_value_fails_pre_flight_on_every_shell_surface`, **3.544 s**; every test in this feature corpus remained below five seconds. Existing over-budget tests are performance debt, not feature acceptance evidence.
- The final isolated gate includes the comment corrections. Targeted include-preflight recovery is five passed; no final test failures remain. The existing compact-unwind linker warning is informational and was not introduced here.
- `git diff --check` passed. Read back every requested plan/log property with `md get`, verified spec `implemented=true`, compared all earlier-phase inventories/packages with the pre-edit values, and checked every listed source/document path exists. Cumulative metadata lists **123 source files** and **14 documentation files**; this phase lists three comment-only source files, nine updated documents, no new documents, and two skill files.

- Final macOS Claudine `just lint`: **passed**. All nine transport guards passed (1.689 s; 3,659 other identities unselected by the guard filter), the lifecycle facet documentation guard passed, and Clippy passed for catalog types, library, contract, CLI, and generator. No formatter was invoked.

### Review handoff

- Phase 6 documentation, audit, and review handoff are complete. All plan checkboxes now reflect completed phase work. The review package includes the nine-requirement/17-criterion mapping, exact inherited regression identities, line-format matrix review, native portability evidence, the additional first-task shape finding, and all local gate results, skips, failures, and recovery details.
- This is a handoff **with acceptance findings**, not a declaration that every clause of the full feature is implemented or proven. In particular, documentation does not resolve the optional first-task primary/stack shape choice or missing container surfaces. The requested `implemented: true` and `completed_phase: 6` metadata record the implementation-phase handoff; the explicit audit above governs feature acceptance.
- `human_review: false` means no new design decision blocks a subsequent implementation phase (there is no Phase 7). Ordinary author review still needs to address the listed behavior/coverage gaps before feature sign-off. The handoff message replaces Phase 5's message with this review context.
- Phase 6 source files changed, all comments only: `claudine/lib/src/composition/sequence/preflight/reapproval.rs`, `claudine/lib/src/composition/sequence/preflight/shape.rs`, and `claudine/lib/src/composition/sequence/task/mod.rs`. No runtime change, dependency change, test addition, or test rename was made.
- Both area READMEs, both preflight pages, sequence/lifecycle pages, plan/log/spec metadata, and both skills are updated. The active feature directory remains in place. No staging, commit, push, lifecycle closure, `cargo fmt`, real provider, or focused terminal/browser occurred.
