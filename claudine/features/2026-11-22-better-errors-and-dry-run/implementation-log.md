---
spec: "/home/build/wt/rusty-biscuit/feat-fix-sequence/claudine/features/2026-11-22-better-errors-and-dry-run/spec.md"
plan: "claudine/features/2026-11-22-better-errors-and-dry-run/plan.md"
implemented-by: "claude/opus"
started-phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/findings/mod.rs
    - darkmatter/lib/src/findings/trail.rs
    - darkmatter/lib/src/findings/sink.rs
    - darkmatter/lib/src/findings/report.rs
    - darkmatter/lib/src/findings/reference.rs
    - darkmatter/lib/src/lib.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/cli/src/output/mod.rs
    - darkmatter/lib/tests/l1/findings.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/diagnostic_projection.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/shared_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - claudine/lib/src/composition/sequence/model.rs
    - claudine/cli/src/main.rs
    - claudine/cli/src/commands/mod.rs
    - claudine/cli/src/commands/sequence.rs
    - claudine/cli/src/commands/document_check/mod.rs
    - claudine/cli/src/commands/document_check/outcome.rs
    - claudine/cli/src/commands/document_check/selection.rs
    - claudine/cli/src/commands/document_check/preview.rs
    - claudine/cli/src/commands/document_check/tests.rs
    - claudine/cli/tests/l1/dry_run_check.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/composition_seams.rs
    - claudine/cli/tests/l1/context_construction_guard.rs
    - claudine/cli/tests/l1/switch_catalog_guard.rs
    - claudine/cli/tests/l1/error_guards/transport-allow.toml
    - claudine/cli/tests/fixtures/check/fleet-run1/renderers/_fleet.yaml
    - claudine/cli/tests/fixtures/check/fleet-run1/renderers/_find.md
    - claudine/cli/tests/fixtures/check/fleet-run2/renderers/_fleet.yaml
    - claudine/cli/tests/fixtures/check/fleet-run2/renderers/_find.md
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run1_reports_every_problem_and_previews_the_composable_task.snap
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_human.snap
    - claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_json.snap
docs_updated_during_phase_1:
    - claudine/docs/cli/sequence.md
    - darkmatter/docs/cli/data-contracts.md
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/claudine/SKILL.md
    - darkmatter/docs/use-skills/use-darkmatter/library.md
source_files_during_phase_2:
    - claudine/cli/src/commands/document_check/mod.rs
    - claudine/cli/tests/l1/dry_run_check.rs
    - claudine/cli/tests/l1/sequence_groups.rs
    - claudine/lib/generated/shipped.json
    - claudine/lib/src/composition/sequence/formal.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/removed.rs
    - claudine/lib/src/composition/sequence/source.rs
    - claudine/lib/tests/fixtures/schema_keys/expected.json
    - claudine/lib/tests/fixtures/schema_keys/sequence.md
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs
    - claudine/lib/tests/l1/main.rs
    - claudine/lib/tests/l1/registration_inventory.rs
    - claudine/lib/tests/l1/schema_key_parity.rs
    - claudine/lib/tests/l1/sequence_schema.rs
    - claudine/registration/formal-sequence-file.yaml
    - claudine/registration/group-catalog-file.yaml
    - claudine/registration/group-file.yaml
    - claudine/registration/sequence-document.yaml
    - claudine/registration/sequence.yaml
    - claudine/registration/task-file.yaml
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/claudine_schema_bundle.rs
    - darkmatter/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/schema_key_hints.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
    - darkmatter/dmls/src/diagnostics/observed.rs
    - darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
    - darkmatter/dmls/tests/l1/claudine_schema_key_parity.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/schema_key_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
    - darkmatter/lib/tests/l1/findings.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_key_findings.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/outcome.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/composition.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/src/validation/suggest.rs
    - simplified-schema/lib/src/yaml_documents/load.rs
    - simplified-schema/lib/tests/l1/key_declarations.rs
    - simplified-schema/lib/tests/l1/main.rs
docs_updated_during_phase_2:
    - claudine/docs/topics/schemas.md
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/docs/cli/schema.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - simplified-schema/docs/topics/diagnostics.md
    - simplified-schema/docs/topics/language/objects-and-named-types.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/simplified-schema/SKILL.md
    - claudine/docs/use-skills/use-claudine/sequences.md
    - darkmatter/docs/use-skills/use-dmls/schemas.md
source_files_during_phase_3:
- claudine/lib/src/composition/mod.rs
- claudine/lib/src/composition/sequence/normalize.rs
- claudine/lib/src/composition/sequence/grammar.rs
- claudine/lib/src/composition/sequence/declarations.rs
- claudine/lib/src/composition/sequence/formal.rs
- claudine/lib/src/composition/sequence/source.rs
- claudine/lib/src/composition/sequence/preflight/check.rs
- claudine/lib/src/composition/sequence/preflight/mod.rs
- claudine/lib/src/composition/sequence/preflight/inputs.rs
- claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs
- claudine/cli/src/commands/document_check/mod.rs
- claudine/cli/src/commands/document_check/tasks.rs
- claudine/cli/src/commands/document_check/preview.rs
- claudine/cli/src/commands/document_check/tests.rs
- claudine/cli/src/commands/wrap/sequence/providers.rs
- claudine/cli/tests/l1/sequence_preflight_baselines.rs
- claudine/cli/tests/l1/dry_run_check.rs
- claudine/cli/tests/l1/main.rs
- claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs
docs_updated_during_phase_3:
- claudine/docs/cli/sequence.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/claudine/SKILL.md
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/passive.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/src/findings/mod.rs
- darkmatter/lib/tests/l1/passive_compose.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/findings.rs
- darkmatter/lib/tests/l1/dynamic_schema_slot_positions.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/composition/prepare/tests.rs
- claudine/lib/src/composition/guardrails.rs
- claudine/lib/src/composition/error/mod.rs
- claudine/lib/src/composition/error/render/provider.rs
- claudine/lib/src/composition/error/tests.rs
- claudine/lib/src/composition/sequence/preflight/check.rs
- claudine/lib/src/composition/sequence/preflight/mod.rs
- claudine/cli/src/commands/document_check/mod.rs
- claudine/cli/src/commands/document_check/preview.rs
- claudine/cli/src/commands/document_check/tasks.rs
- claudine/cli/src/commands/compose/prep.rs
- claudine/cli/tests/l1/dry_run_check.rs
- claudine/cli/tests/l1/composition_seams.rs
- claudine/lib/src/doctor/registration_tests.rs
docs_updated_during_phase_4:
- darkmatter/docs/inline/preflight-checks.md
- darkmatter/docs/topics/caching.md
- claudine/docs/cli/sequence.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- darkmatter/docs/use-skills/use-darkmatter/library.md
- .claude/skills/claudine/SKILL.md
source-code:
- claudine/lib/src/doctor/registration_tests.rs
- darkmatter/lib/src/findings/mod.rs
- darkmatter/lib/src/findings/trail.rs
- darkmatter/lib/src/findings/sink.rs
- darkmatter/lib/src/findings/report.rs
- darkmatter/lib/src/findings/reference.rs
- darkmatter/lib/src/lib.rs
- darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/cli/src/output/mod.rs
- darkmatter/lib/tests/l1/findings.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/diagnostic_projection.rs
- darkmatter/lib/tests/l1/report_diagnostics.rs
- darkmatter/lib/tests/l1/shared_diagnostics.rs
- darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
- claudine/lib/src/composition/sequence/model.rs
- claudine/cli/src/main.rs
- claudine/cli/src/commands/mod.rs
- claudine/cli/src/commands/sequence.rs
- claudine/cli/src/commands/document_check/mod.rs
- claudine/cli/src/commands/document_check/outcome.rs
- claudine/cli/src/commands/document_check/selection.rs
- claudine/cli/src/commands/document_check/preview.rs
- claudine/cli/src/commands/document_check/tests.rs
- claudine/cli/tests/l1/dry_run_check.rs
- claudine/cli/tests/l1/main.rs
- claudine/cli/tests/l1/composition_seams.rs
- claudine/cli/tests/l1/context_construction_guard.rs
- claudine/cli/tests/l1/switch_catalog_guard.rs
- claudine/cli/tests/l1/error_guards/transport-allow.toml
- claudine/cli/tests/fixtures/check/fleet-run1/renderers/_fleet.yaml
- claudine/cli/tests/fixtures/check/fleet-run1/renderers/_find.md
- claudine/cli/tests/fixtures/check/fleet-run2/renderers/_fleet.yaml
- claudine/cli/tests/fixtures/check/fleet-run2/renderers/_find.md
- claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run1_reports_every_problem_and_previews_the_composable_task.snap
- claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_human.snap
- claudine/cli/tests/l1/snapshots/l1__dry_run_check__fleet_run2_json.snap
- claudine/cli/tests/l1/sequence_groups.rs
- claudine/lib/generated/shipped.json
- claudine/lib/src/composition/sequence/formal.rs
- claudine/lib/src/composition/sequence/grammar.rs
- claudine/lib/src/composition/sequence/mod.rs
- claudine/lib/src/composition/sequence/preflight/mod.rs
- claudine/lib/src/composition/sequence/removed.rs
- claudine/lib/src/composition/sequence/source.rs
- claudine/lib/tests/fixtures/schema_keys/expected.json
- claudine/lib/tests/fixtures/schema_keys/sequence.md
- claudine/lib/tests/l1/claudine_schemas.rs
- claudine/lib/tests/l1/claudine_schemas_embedded.rs
- claudine/lib/tests/l1/lifecycle_one_implementation_guard.rs
- claudine/lib/tests/l1/main.rs
- claudine/lib/tests/l1/registration_inventory.rs
- claudine/lib/tests/l1/schema_key_parity.rs
- claudine/lib/tests/l1/sequence_schema.rs
- claudine/registration/formal-sequence-file.yaml
- claudine/registration/group-catalog-file.yaml
- claudine/registration/group-file.yaml
- claudine/registration/sequence-document.yaml
- claudine/registration/sequence.yaml
- claudine/registration/task-file.yaml
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/tests/l1/claudine_schema_bundle.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/schema_key_hints.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
- darkmatter/dmls/src/diagnostics/observed.rs
- darkmatter/dmls/tests/l1/claudine_schema_discovery.rs
- darkmatter/dmls/tests/l1/claudine_schema_key_parity.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/schema_key_diagnostics.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/schema_validation.rs
- darkmatter/lib/src/markdown/errors/blocks.rs
- darkmatter/lib/src/markdown/schemas/bound_operands.rs
- darkmatter/lib/src/markdown/schemas/clean.rs
- darkmatter/lib/src/markdown/schemas/mod.rs
- darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
- darkmatter/lib/tests/l1/schema_key_findings.rs
- darkmatter/lib/tests/l1/schema_phase_validation.rs
- darkmatter/lib/tests/l1/shipped_schema_corpus.rs
- simplified-schema/lib/src/lib.rs
- simplified-schema/lib/src/outcome.rs
- simplified-schema/lib/src/prepared.rs
- simplified-schema/lib/src/resolve.rs
- simplified-schema/lib/src/resolve/composition.rs
- simplified-schema/lib/src/schema/export.rs
- simplified-schema/lib/src/schema/mod.rs
- simplified-schema/lib/src/simplified/convert.rs
- simplified-schema/lib/src/simplified/mod.rs
- simplified-schema/lib/src/simplified/source.rs
- simplified-schema/lib/src/simplified/types.rs
- simplified-schema/lib/src/validate.rs
- simplified-schema/lib/src/validation.rs
- simplified-schema/lib/src/validation/suggest.rs
- simplified-schema/lib/src/yaml_documents/load.rs
- simplified-schema/lib/tests/l1/key_declarations.rs
- simplified-schema/lib/tests/l1/main.rs
- claudine/lib/src/composition/mod.rs
- claudine/lib/src/composition/sequence/normalize.rs
- claudine/lib/src/composition/sequence/declarations.rs
- claudine/lib/src/composition/sequence/preflight/check.rs
- claudine/lib/src/composition/sequence/preflight/inputs.rs
- claudine/lib/src/composition/sequence/preflight/tests/check_tree.rs
- claudine/cli/src/commands/document_check/tasks.rs
- claudine/cli/src/commands/wrap/sequence/providers.rs
- claudine/cli/tests/l1/sequence_preflight_baselines.rs
- darkmatter/lib/src/markdown/compose/passive.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/schemas/slots/session.rs
- darkmatter/lib/tests/l1/passive_compose.rs
- darkmatter/lib/tests/l1/dynamic_schema_slot_positions.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/composition/prepare/tests.rs
- claudine/lib/src/composition/guardrails.rs
- claudine/lib/src/composition/error/mod.rs
- claudine/lib/src/composition/error/render/provider.rs
- claudine/lib/src/composition/error/tests.rs
- claudine/cli/src/commands/compose/prep.rs
documentation:
- claudine/docs/cli/sequence.md
- darkmatter/docs/cli/data-contracts.md
- darkmatter/docs/inline/preflight-checks.md
- claudine/docs/topics/schemas.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/docs/cli/schema.md
- darkmatter/docs/topics/schemas/dmls-schema-support.md
- simplified-schema/docs/topics/diagnostics.md
- simplified-schema/docs/topics/language/objects-and-named-types.md
- darkmatter/docs/topics/caching.md
completed-phase: 4
packages:
    - darkmatter
    - darkmatter-cli
    - darkmatter-lsp
    - claudine
    - claudine-cli
    - simplified-schema
decisions-for-review:
    - |-
      The passive check is built but NOT yet `--dry-run`. Switching the dispatch made 285 existing L1 tests fail, 208 of them
      for real reasons: the old preview runs the library preflight (dynamic and file-referenced sequence sources, list
      classification, groups, task files, catalogs, preflight errors), and this phase's walker reports those as not checked,
      so a dry run would have said "fine" where a real run fails. `main.rs` routes a document dry run to
      `commands/document_check` only when `document_check::enabled()`, today the `test-fixtures` seam
      `CLAUDINE_TEST_PASSIVE_DRY_RUN` (`1` human, `json` JSON). `--dry-run` and `--dry-run-shell` behave as before.
      Alternatives: ship the switch and migrate about 40 test files now, or keep the old preview as a fallback for what the
      walker cannot reach. Redo: once the walker reuses `Loader::walk_plan`/preflight (plan Rule 9), delete `enabled()`,
      route every document dry run there, add `--json`, refuse `--dry-run-shell`, and migrate the old preview's L1
      suites (`sequence_sources_cli`, `sequence_errors_cli`, `entry_point_parity`, `argv_normalization`,
      `compose_prompt_tiers`, `wrap_sequence_composition`, `level1_dry_run_pty`, `wrap_compose_preflight`, …).
    - |-
      `--json` is not declared. Declaring it on `SharedComposeArgs` (spec) took a provider's own `--json` out of the caller
      tail after the file (`provider_tail_launch::a_resume_carries_the_tail_exactly_once` failed). Shipping the spec's flag
      is a behavior change for callers who forward `--json` to a provider; it needs either acceptance of that, or a different
      name. Redo: add the flag with the switch and update that test.
    - |-
      The sequence walker reads the root's authored `sequence` list directly (string and mapping items, executables,
      `with`, `agent`, `model`, `prompt`) instead of `normalize.rs`/the preflight loader. It does not walk nested groups,
      task files, nested or referenced sequences, or command-backed lists (each is an `info` not-checked row). Redo:
      Rule 9's shared walker; keep the finding codes and messages.
    - |-
      The provider chain is written out in `document_check/selection.rs` (flag, `agent=` setter, `with.agent`, prompt
      frontmatter `agent`, root `agent`) rather than recorded inside `wrap/sequence/providers.rs::select_target` and
      `wrap/composition/target.rs` (Rule 10). List-valued `agent`, `preferred_agent`, and the review rules are not
      modeled. Not installed = error; unresolved = error on a non-terminal stderr, warning on a terminal. Redo: record
      attempts at the selector's decision points and render them.
    - |-
      The model requirement is a presence check over `-m`, the provider's model variables, `MODEL`, `with.model`, the
      prompt's `model`, and the profile's configured default, instead of reusing `resolve_model_and_validate`'s pure
      precedence. Redo with Rule 10.
    - |-
      The dry-run guard sits in Darkmatter's `prepare_resolved` and `prepare_site` (after the built-in blacklist, before
      approval) and again in `execute_prepared_outcome`. It refuses with `ShellExpansionError::SkippedInDryRun`, so a
      preview depending on any command is `not checked` as a whole. Rule 6's deferred-value propagation (skip only the
      dependent parts) and the remote/ask/read-approval passive policy are not built. Remote reads in a dry run follow
      `allow_remote_transclusion` plus the `network` permission, as a real compose does.
    - |-
      The preview composes `Markdown::compose_with` directly (`document_check::preview::compose_passively`, added to the
      `compose_with` allowlist) with `with_dry_run()` and `CacheAccessMode::Off`. Caller setters, `--set`, and a task's
      `with:` are inserted into the document's frontmatter. The canonical `prepare_*` path was not used because its staged
      initialize and approval are effects. Alternative: a `dry_run` field on `PrepareOptions` applied in
      `canonical_compose_options`. Redo if the canonical preparer gains a passive mode.
    - |-
      The three new Darkmatter codes live in `darkmatter::findings::FINDING_CODES`, not in `COMPOSE_DIAGNOSTIC_CODES` (the
      `md --deny` catalog), because `md compose` does not emit them yet. Claudine's dry-run codes (`composition.*`,
      `internal.bug`) are in a test-only `CHECK_CODES` list, not in `diagnostics/registry.rs`, which holds error codes with
      lifecycle facets. Redo when Rule 13 wires `md compose`, and if these become lifecycle errors.
    - |-
      `Diagnostic` gained `hint`, `notes` (used for the provider chain lines), and `trail`. `ReferenceDetails` gained `base`,
      `resolved`, `searched`, and `suggestions`. All are omitted when empty, so existing JSON is byte-identical. The legacy
      `iteration_path`/`included_via` are not yet serialized from `trail` (Rule 2). `RelatedLocation` reuses the existing
      `DiagnosticRelated`. The reference renderer of `md validate refs` was not extracted (Rule 4); `FindingReport` has its
      own renderer.
    - |-
      `missing_document`'s hint names the path relative to the launch directory (`DocumentConstruct::launch_dir`). For a
      sequence step the host overrides `base` with the sequence file, as the spec's example shows.
    - |-
      Phase 2: `removed(<replacement>)` is block-form only (a property's whole value), stored apart from `properties` in
      `SchemaShape::key_declarations` (`KeyDeclarations`, boxed with the opt-in to stay under clippy's enum-size limit), and
      lowered to a `false` property plus the owner's `x-darkmatter-removed` annotation. It is refused on a pattern key, a
      `$` key, a `types` entry, and inside an inline `{ … }`. A removed key's problem points at the owner (`path`) with the
      key as `offending_property`, so positions land on the key token; the old `/fail_fast`-style path is gone.
    - |-
      Phase 2: `$advise-unknown-keys` is an object-level block key whose list names exact keys never advised on; it is
      read from the object's own declaration and not inherited through `$extends`/spreads. Advice needs a unique best
      candidate; the `<string>` catch-all never silences it, an intentional pattern key does.
    - |-
      Phase 2: the shared ranker lives at `simplified_schema::validation::suggest` (moved verbatim from `md clean`; its
      message text is unchanged). DMLS shows the hint by appending it to the message (`"{message}; {hint}"`) and in
      `data.hint`, since LSP has no hint field; `md schema validate` writes `hint` only when present.
    - |-
      Phase 2: no related declaration location is attached to a removed or near-miss finding. The prepared JSON Schema
      carries no declaration positions, so nothing proves one; `related` stays empty and the parity fixture asserts no
      related position. Adding it means threading the schema source map into `annotate_key_problems`.
    - |-
      Phase 2: root `fail_fast: null` is now a schema error (previously valid to the schema and refused only by the
      parser), as Rule 14's null-presence rule requires. Group legs are now the closed `leg` (a leg's undeclared key such as
      `owner:` is a schema error, as the parser already refused it), while a sequence step stays open.
    - |-
      Phase 2: the parser's removal table (`composition/sequence/removed.rs`) classifies each annotated object of the
      prepared `sequence-document.yaml` payload by what it declares: the root, `tasks` → group, `side_effect` → task. It
      panics if a key the parser refuses is not declared removed for that owner, so the schema and the parser cannot drift.
      Group and task `execution` share the spec's replacement text; `execution: serial` still appends the parser's
      value-specific example.
    - |-
      Phase 2: new `kind: yaml-schema` members `task-file.yaml` (a closed `leg`), `group-file.yaml` (a closed `group`),
      and `group-catalog-file.yaml` (open root, `groups:` of groups). The catalog root stays open because the parser reads
      only `kind` and `groups`. No tracked file uses these kinds yet.
    - |-
      Phase 2: near-miss advice is a compose warning too (`ComposeWarning::schema_key_advice`), so `md compose` and
      Claudine's passive check show it; `md schema validate` reports only problems (advice is not in its record), which a
      later phase may add if wanted.
    - |-
      Phase 2: the passive check judges only the sequence root (Markdown frontmatter or a YAML file by kind) and reports
      only key findings (removed, unknown, near-miss). Typed values are left to the parser and each prompt's composition,
      because a templated value is not final in a dry run. Task, group, and sequence files loaded by reference are not
      judged: the hand-written walker does not load them (Phase 3 moves this check onto the shared walker).
    - |-
      Phase 2: two Phase 1 lint failures were fixed in passing because the Darkmatter gate needs them: a clippy
      `type_complexity` in `darkmatter/lib/tests/l1/findings.rs` (a `DryRunCase` alias), and `ContractVerdict::Rows` in
      `call_check.rs` now boxes its `PropertyDef` (the shape grew by one pointer).
    - |-
      Phase 3: completion flags. The pipeline prompt treats Phase 3 as the final phase and asks for `implemented: true`,
      but the plan has `total-phases: 8` and Phases 4–8 are unstarted. Set: `completed-phase: 3`, the phase-3 file lists,
      and the cumulative `source-code` / `documentation` lists on the plan and this log; the spec keeps `implemented-by:
      "claude/opus"` and is NOT marked `implemented: true`. Set it when Phase 8 closes.
    - |-
      Phase 3: a check-mode root task's property is its authored key, `sequence[i]`; a real run's stays `tasks[i]`, and
      `TaskIdentity` is `tasks[i]` in both. Changing the real-run label would change every preflight message (Phase 8's
      Rule 16 sweep is the place to decide it). Likewise a referenced data file's child property is `sequence[i]` even for
      a bare list or an offset selection, as the real run labels it; the CLI then locates the finding at the file only.
    - |-
      Phase 3: missing-input grading assumes `prompt_for_missing: true` (with the observed stdin/stderr terminals) instead
      of reading the user's configuration, because the passive check reads no configuration until Phase 5's passive
      projection. With the setting off, a non-interactive verdict is an error either way; only an interactive run with the
      setting off is graded too leniently (warning).
    - |-
      Phase 3: recovering input discovery (`InputDiscoveryOptions::recovering`) skips an item, leg, or owner it cannot
      read; the walk reports that failure at its task, so its inputs are simply absent from the inventory. A root
      `sequence:` that is a file reference or `{{ }}` expression is still planned fail-fast
      (`resolve_sequence_plan_with`): one bad row there leaves the whole root list unread with one root failure. Nested
      references recover row by row (`plan_referenced_sequence_recovering`); giving the root reference the same treatment
      means a recovering `resolve_sequence_plan_with`.
    - |-
      Phase 3: a referenced document that fails to parse is reported with its loader's message, which carries the resolved
      path and the line and column as text; the finding has no structured source location in that file. Giving loader
      errors a `DiagnosticSource` is an error-site change (Phase 8 **Complete Sequence Sites**).
    - |-
      Phase 3: `compose`/`inline-compose` checks now parse the document's lifecycle and judge its literals
      (`document_check::check_single`), two of the refusals `prepare.rs` makes before composition. The others there
      (removed validation keys, removed loop and sequence reads, the inline baseline rehearsal) are still not run by the
      check. Redo: Phase 4 **Mark Previews** composes through the canonical preparer's passive mode, which makes all of
      them, and then deletes this stand-in.
    - |-
      Phase 3: a task's preview is left unchecked only for its own missing inputs; an owner's missing value passed down
      through `with:` (the frozen-complete reproduction) still previews the nested prompt and adds its schema errors.
      Phase 4's placeholder marking settles this.
    - |-
      Phase 4: deferral is carried by authored text, not by a new value type. A deferred frontmatter value keeps its
      authored text and the request's `PassiveSession` maps that text to its `DeferralReason`; a read of it raises
      `ExpressionError::Unavailable`, which propagates like any evaluation error (lookups, conditions, collections,
      arguments, paths, `with:`) and is never recovered by `try`/`||`. Alternative: a `Value::Deferred` variant in the
      evaluator, which is exact per site but touches every value consumer and serializer. Trade-off: two values with
      identical authored text in one request share deferral, which is harmless because identical text evaluates
      identically. Redo if a case needs per-site identity: replace the text map with a site-keyed one in
      `passive.rs::check_read`.
    - |-
      Phase 4: in a dry run the body shell stages (`ShellExpansion`, `ShellBlocks`) report as disabled
      (`ComposeOptions::is_enabled`), and `pipeline/mod.rs` records each `::shell` / `::shell-block` site as a deferral
      before skipping, instead of running the stage in a "skip" mode. Alternative: thread the session through the stage.
      Trade-off: the stage's own approval and "did not run" internals are never reached, which is the point. Redo if a
      stage gains effect-free work a dry run should still do.
    - |-
      Phase 4: the plan's network `ask` does not exist: `NetworkPermission` is `allow`/`deny` (plus invalid), so a dry
      run fetches an allowed remote read with no persistent store and keeps a denied one's typed refusal; nothing asks.
      The plan's `info` finding for a read needing approval is a `SchemaTemplate` deferral (the only read that asks is a
      schema slot's), surfaced by Claudine as a `not checked` row. Redo if a network `ask` is added: map it to
      `DeferralReason::ReadNeedsApproval` in `build_remote_fetch_runtime`.
    - |-
      Phase 4: provider-query functions (`pr`, `pr_list`, `cicd`, `cicd_list`) are deferred, like shell and ping probes,
      rather than queried under the network policy. Alternative: treat them as allowed remote reads. Trade-off: a dry
      run never reaches a provider's API, so it cannot preview a body that reads one. Redo in
      `functions/mod.rs::passive_refusal` if previews should query providers.
    - |-
      Phase 4: `dm.compose.empty_body` is emitted by Claudine's preview (with the cause from
      `ComposeReport.excluded_blocks`) and by the canonical preparer's `ComposedBodyEmpty { cause }`; `md compose` does
      not yet emit it, which is Phase 6 "Combine Document Checks". Only top-level excluded blocks are recorded
      (children's are not merged) and the cause claims nothing it did not observe. `composition.no_executable`
      suppresses the empty-body finding structurally: the no-executable error path returns before any preview. Redo in
      Phase 6 if `md` needs the finding.
    - |-
      Phase 4: an `Interpolation` deferral carries no line (the rewrite pass sees span text, not file positions);
      `ShellDirective`, `ShellBlock`, and `BlockCondition` carry their file line, and a `FrontmatterValue` from a
      command carries its line when the shell candidate knows it. Redo if Phase 6's shared report needs located spans:
      thread `located()` through `interpolation/rewrite.rs`.
    - |-
      Phase 4: a dry run's schema-slot session is `SessionMode::ReferenceCheck` (resolves what it can without asking);
      an unresolved template records a `SchemaTemplate` deferral whose `authored` is the slot error's text, since the
      template has no single authored span. Redo if the report should show the slot reference itself.
    - |-
      Phase 4 (preview decision, settled): the canonical preparer has a passive mode (`PrepareOptions.passive`) and the
      preview composes through `prepare_direct` / `prepare_inline`; the `fm_insert` of setters and `with:` and the
      `compose_with` call in `preview.rs` are gone (its `composition_seams` allowlist entry removed), as is
      `check_single`'s lifecycle stand-in, since the preparer makes those refusals plus the inline baseline rehearsal.
      The inline guardrails are read without being created (`load_guardrails_passively`). The input check
      (`prompt_inputs`) prepares passively too, so a command value no longer skips it. Alternative kept open by the
      plan: a separate passive composer, rejected because it duplicates composition (R1). Redo only if the preparer
      gains an effect a dry run cannot skip.
    - |-
      Phase 4: a nested prompt task's `with:` now reaches its preview as the real walk evaluates it:
      `Loader::record_prompt_reference` notes the evaluated values and late keys on the open `CheckedTask`
      (`with_evaluated`, `with_late`), and the preview passes them as data beneath the caller's setters (the task-turn
      precedence). A task with late `with:` keys skips the input check and preview as before; a body task's `with:` is
      split by whether it holds `{{`. A prompt whose owner misses an input (`inventory.prompts` with a `missing` owner)
      is held: `inputs` and `preview` are not checked, instead of repeating the owner's missing input. Alternative:
      evaluate `with:` again in the CLI, which would be a second evaluator. Redo if body tasks' `with:` should be
      evaluated by the walk too.
    - |-
      Phase 4: the human preview marks each deferred construct by appending `[not evaluated in a dry run]` after a
      deferred `{{ … }}` span and at the end of a deferred directive line (`::shell`, `::shell-block`, `::block`); the
      JSON envelope carries no preview, so the rows are the machine-readable form. Rows sharing a reason are grouped by
      the shared renderer (`not checked: \`::block\` condition, interpolation (…)`). Alternative: styled (dim) text,
      which would be lost when stdout is piped. Redo in Phase 5/6 if the preview format is settled differently.
message-to-agent: |-
    Phase 4 is closed: dry runs compose passively (`darkmatter::markdown::compose::passive`), and Claudine's preview
    prepares through the canonical preparer's passive mode (`PrepareOptions.passive`), marking each deferral and
    reporting it as its own not-checked row. With the seam forced on, 218 CLI L1 tests still fail: they assert the old
    `--dry-run` preview's output (rendered shell tasks, `--perf` notes, provider-free previews, the old missing-input
    wording), which is Phase 5's "Migrate Preview Suites". The only Phase 3 kind (c) failures still red there are
    old-preview expectations only (the inline baseline refusal reads "Cannot edit frontmatter text safely"; the missing-input report is
    a finding per input); a `$( … )` root list is reported as not checked by design. The seam
    `CLAUDINE_TEST_PASSIVE_DRY_RUN` remains until Phase 5. The Phase 2 checkpoint **Integrate Schema Checks** is still
    unticked (items 3 and 4). The working tree holds the uncommitted Phase 1–4 changes.
---

# Implementation Log for 2026-11-22-better-errors-and-dry-run (3 phases)

## Phase 1

- 2026-11-26: started Phase 1. Read the spec and plan; surveyed the
  Darkmatter `Diagnostic` record, Claudine's compose/sequence argument
  model, and the dry-run path.
- 2026-11-26: **Build Findings (partial).** Added `darkmatter::findings`
  (`findings/{mod,trail,sink,report,reference}.rs`): tagged `Trail`/
  `TrailFrame`, `FindingSink` with opaque `SubjectId`/`FindingId`,
  equivalence dedup and `not_checked`, `FindingReport` with counts and a
  biscuit-terminal human renderer, `EmptyBodyCause`, and the
  `missing_document` builder over biscuit-file's `DetailedResolution` with a
  shared `ListingMemo` and up to three suggestions. `Diagnostic` keeps its
  public path and gains `hint`, `notes`, `trail`, `actionable`/`projected`
  constructors and setters; `ReferenceDetails` gains `base`, `resolved`,
  `searched`, `suggestions`. New fields are omitted when empty, so existing
  JSON is unchanged.
- 2026-11-26: **Guard Composition (partial).** `ComposeOptions::with_dry_run()`
  (part of the options identity). Darkmatter refuses every document command
  under it with `ShellExpansionError::SkippedInDryRun`, after the built-in
  blacklist and before approval (`prepare_resolved`, `prepare_site`), and
  again before the cache and spawn (`execute_prepared_outcome`).
- 2026-11-26: **Integrate Claudine Checks (partial).** New
  `claudine/cli/src/commands/document_check/` (mod, outcome, selection,
  preview, tests). Added `ExecutableField::noun`. Dispatched from
  `main.rs::dispatch` before tracing, the Darkmatter config load, the wrapper,
  and `ensure_config_exists`, but only under `document_check::enabled()`.
- 2026-11-26: switched `--dry-run` to the check first; the full CLI suite
  then failed 285 tests (208 still failing with provider findings
  downgraded). These are real coverage gaps of the walker, so I held the
  switch back. I restored `--dry-run-shell`, the task-runner hook, the old
  `sequence_dry_run.rs`, and the Claudine docs and skills I had rewritten.
  The new tests moved to `cli/tests/l1/dry_run_check.rs` behind the seam.
  See `decisions-for-review`.
- 2026-11-26: **Isolate Startup (not done).** `--json` is not declared (it
  shadowed a provider's `--json` in the caller tail). The passive config
  projection (Rule 11) was not built; the check reads configuration only
  through `InvocationContext` without `with_darkmatter_config`, and
  `dry_run_check::a_dry_run_creates_no_configuration` proves nothing is
  created.
- 2026-11-26: **Prove Passive Reports (partial).** Fixtures
  `cli/tests/fixtures/check/fleet-run1/` and `fleet-run2/`; insta snapshots
  of the human report and the JSON envelope reproduce the spec's Run 1 and
  Run 2 (2 errors, 2 warnings, 3 info; previewed 1 of 2). Also covered:
  continuation past a failed task, an unknown provider, commands and side
  effects never run, no configuration created, provider not on PATH,
  OpenCode's model requirement, the inline file left intact, a missing root
  as a finding, and `--dry-run-shell` refused by the check. Darkmatter
  `lib/tests/l1/findings.rs` (16 tests) covers the record, trail, sink,
  renderer, reference builder, and the guard with and without pre-approval.
- 2026-11-26: guards updated with reasons: `compose_with` allowlist,
  context-construction allowance for the seam's `var_os`, the ownership-site
  list, and the error-collapse allowlist. The module was split to stay under
  the 800-line placement limit.
- 2026-11-26: gates. Claudine `just test`: 13089 passed, 16 skipped
  (the CLI suite alone: 5403 passed). `just lint` is clean. Dependents,
  serially: darkmatter lib, full, 12903 passed; darkmatter-cli, scoped to
  output/json/diagnostic/validate/graph/compose, 1507 passed;
  darkmatter-lsp, scoped to diagnostics, 187 passed. biscuit-file was not
  changed. Cross-OS: no Windows run. The new code uses `/`-normalized display
  paths and `EXE_SUFFIX` fakes, and the switch is not live.
- 2026-11-26: `just check-tier-coverage`: Stranded (0). A first attempt
  failed with "listing failed" for 49 cells because the disk was full (0 B
  free). `just sweep` freed 71 GiB of incremental caches, and the retry
  passed. `just publish-skills` (darkmatter) failed the same way and then
  succeeded.
- 2026-11-26: docs: `claudine/docs/cli/sequence.md` gains a "Passive check
  (planned)" section. Darkmatter `docs/cli/data-contracts.md` documents the
  new record fields and finding codes, and `docs/inline/preflight-checks.md`
  documents dry runs. The `use-darkmatter` caller skill (`library.md`) gains
  `with_dry_run`, and the repo `claudine` skill describes the held-back check.

## Phase 2

### Simplified Schema: `removed(...)`, `$advise-unknown-keys`, shared ranker

- `removed(<replacement>)` is read in the block-form shape parser (`simplified/mod.rs::removed_declaration`) into the new
  `SchemaShape::removed` map, never into `properties`, so completion, suggestion pools and defaults never see a removed key.
  Refused (grammar error at the key) with an empty replacement, on a pattern key or `$`-key, and as a `types` table entry.
  The inline `{ … }` form is not a placement for it (the whole value must be the declaration).
- Composition (`resolve/composition.rs::assemble`) carries removals: a removal and a literal of the same key replace each
  other in entry order; the receiver's own removals apply last. Resolution keeps both new fields
  (`resolve.rs::expand_shape`/`expand_assembled`).
- Lowering (`convert.rs::object_body_from_shape`): each removed key is a `false` property schema, joins the
  pattern-property literal exclusion (so an overlapping pattern never reports it again), and the owner carries
  `x-darkmatter-removed: { key: replacement }`; the opt-in lowers to `x-darkmatter-advise-unknown-keys: [excluded…]`.
  Layer/baseline merging (`resolve.rs::merge_layer`) moves a copied removal's advice with its `false` property, and both
  annotations are admitted by `validate_simple_object_root`.
- `ValidationProblem` gained `hint`; `ValidationProblemCode` gained `RemovedKey` (`dm.schema.removed_key`) and
  `NearMissKey` (`dm.schema.near_miss_key`). `prepared::describe_problems` now first runs
  `validate::annotate_key_problems`: a `false`-schema failure whose owner annotation names the key becomes one
  `RemovedKey` problem at the owner (offending key = the key, so positions land on the key token), and an
  `UnknownKey` gains a `did you mean` hint from the effective object schemas the location reaches (references and
  value-selected union arms; arms that hold no object add nothing).
- Advice: `validate::key_advice` walks the value only when the lowered schema mentions the opt-in; it is returned on
  `ValidationSuccess::key_advice`, `Prepared::key_advice` and `SimplifiedSchema::key_advice`, never among a failure's
  problems. A key is skipped when declared (removed included), excluded, or matched by an intentional
  `patternProperties` pattern; the `<string>` catch-all (`additionalProperties`) is admission and never silences it.
  Only a unique best candidate is advised (`suggest::unique_nearest`).
- Ranker: `validation/suggest.rs` (`near_key_match`, `edit_distance`, `nearest`, `unique_nearest`, `did_you_mean`),
  moved verbatim from Darkmatter's `md clean`.
- Export round-trips both declarations (`schema/export.rs::write_shape`); the inline-expression writer refuses them as
  lossy. Source projection skips them (`simplified/source.rs::is_key_metadata`).
- Tests: `simplified-schema/lib/tests/l1/key_declarations.rs` (16 cells: absence, null and every non-null value
  presence, key-token position, free-text replacement, empty replacement, pattern/table placement, pattern overlap
  reported once, nested + union-reached owner, composition + later-member-wins, export round trip, lowering, closed
  unknown-key hint and no-match, opt-in advice with exclusions/intentional pattern/catch-all, ambiguous tie, absent
  opt-in, opt-in value shapes null/scalar/mapping/mixed/all-invalid/empty-string and `[]`), plus
  `validation::suggest` unit tests. `just test` in `simplified-schema`: 2221 passed.

### Darkmatter and DMLS: projecting key findings

- `ValidationReport` gained `key_advice`, computed by `validate::key_advice` and located with the same position map as
  problems; it never affects `valid`. `awaits_rendering` treats `RemovedKey` like `UnknownKey`.
- `md clean`'s suggestion (`schemas/clean.rs::nearest_declared_key`) now ranks with `simplified_schema::validation::suggest`
  and excludes `false` (removed) schemas; its local ranker was deleted and its message text is unchanged.
- Compose: a schema problem's record carries `hint` (`report_diagnostics.rs`), and near-miss advice is reported as a
  warning (`ComposeWarning::schema_key_advice`, identity `WarningSubject::SchemaKey { document, path, key }`, record
  from `report_diagnostics::key_advice_diagnostic`), emitted beside schema advisories in `compose/schema_validation.rs`.
- `md schema validate`: the JSON problem carries `hint` only when present; human output adds a `hint:` row.
- DMLS: `dm.schema.deprecated_key` (never emitted) was removed from the catalog; `dm.schema.removed_key` (error) and
  `dm.schema.near_miss_key` (warning) were added. `diagnostics/frontmatter.rs` chains `key_advice` after `problems` (the
  YAML-document adapter shares the same function), ranges a removed key and advice on the key token, appends the hint
  to the message (`problem_message`: `"{message}; {hint}"`), and puts it in `data.hint`; `observed.rs` matches.
  Unknown keys keep their existing grade (a warning in non-strict frontmatter).
- Tests: `darkmatter/lib/tests/l1/schema_key_findings.rs` (report location and hint, unknown-key hint, advice beside a
  valid report, `md clean` text kept and no removed suggestion), `darkmatter/cli/tests/l1/schema_key_hints.rs`
  (validate JSON and human hint, compose JSON code and hint, compose advice as a warning on a successful run),
  `darkmatter/dmls/tests/l1/schema_key_diagnostics.rs` (removed key at the key token for UTF-8/UTF-16 × LF/CRLF with a
  non-ASCII key before it, near-miss warning, excluded key silent, completion never offers a removed key).
  `schema_phase_validation.rs`'s passive corpus walk now skips object-level entries (`$extends`,
  `$advise-unknown-keys`, `removed(…)`), as it already skipped `$constraints`.

### Claudine: authored roles and the parser's removal table

- `registration/sequence.yaml`: the task fields moved to `task-fields`, with `setup`, `teardown`, `execution`, and
  `fail_fast` declared `removed(…)` (the spec's texts). `task` (a sequence step) is `task-fields` plus the open
  catch-all and `$advise-unknown-keys: [agent, model]`; the new closed `leg` (`task-fields` plus a task file's
  `kind`) types a group's `tasks:`; `group` declares `execution` removed. `sequence-document.yaml` and
  `formal-sequence-file.yaml` declare the root `fail_fast` removed.
- New kinds: `task-file.yaml` (`kind: task`, a closed `leg`), `group-file.yaml` (`kind: group`, a closed `group`),
  and `group-catalog-file.yaml` (`kind: group-catalog`, an open root whose `groups:` entries are groups). The parser
  reads a catalog root's `kind` and `groups` only, so its other keys stay legal.
- `composition/sequence/removed.rs` replaces the hand-written dispatch (`grammar.rs::removed_task_key`, deleted): it
  reads every removal annotation of the prepared `sequence-document.yaml` payload into an owner table (root; an
  object declaring `tasks` is a group; one declaring `side_effect` is a task) and keeps the parser's typed
  `RemovedGrammar` at the old boundaries: `mod.rs::resolve_sequence_plan_with`, `formal.rs`, `source.rs`,
  `grammar.rs::validate_task_grammar`, and `preflight::Loader::build_group`. `execution: serial` keeps its
  value-specific example sentence. The lifecycle guard's ParserTwin allowance moved to `removed.rs::removed_key`.
- `just shipped-schemas` regenerated `claudine/lib/generated/shipped.json` only (the Simplified Schema and Darkmatter
  artifacts were already current).
- Tests updated to the intended messages: `sequence_schema.rs` (group `execution` is "key `execution` has been
  removed"; root `fail_fast` points at the root; root `fail_fast: null` is now invalid), `claudine_schemas_embedded.rs`,
  `registration_inventory.rs` (root `fail_fast` problem path is the root), and `sequence_groups.rs` (the two root
  `fail_fast` runs assert the spec's replacement instead of the old "failure" wording). New:
  `registration_inventory.rs::{a_task_file_is_judged_as_a_closed_task, a_group_file_is_judged_as_a_closed_group,
  a_group_catalog_file_types_each_entry_as_a_group}` through the registered host, and `removed.rs` unit tests.

### Checkpoint: Integrate Schema Checks

- The passive check (`document_check::check_schema_keys`) judges the sequence root as authored (Markdown frontmatter,
  or a YAML file by its kind) with Claudine's registered schemas and reports removed and unknown keys as errors and
  near-miss advice as warnings, each once per owner and key, with the schema's hint. This runs before the step walk,
  so a removal is reported even where the parser would abort first. Tests:
  `dry_run_check.rs::{removed_unknown_and_near_miss_keys_are_findings_with_their_hints,
  a_formal_sequence_files_removed_root_key_is_a_finding}`.
- Editor/runtime parity: one fixture, `claudine/lib/tests/fixtures/schema_keys/{sequence.md, expected.json}`, covers
  root (`null`), group, leg, and step removals, a closed group's misspelled key, near-miss advice, unrelated step data,
  the never-advised `agent`, and a legal `group.fail_fast`, with non-ASCII text before two keys.
  `claudine/lib/tests/l1/schema_key_parity.rs` proves Claudine's embedded bundle reports exactly `expected.json` for LF
  and CRLF; `darkmatter/dmls/tests/l1/claudine_schema_key_parity.rs` proves DMLS's registered host publishes the same
  code, message, hint, and instance path, with the range at the same authored key under UTF-8 and UTF-16, LF and CRLF.
- The real parser's typed failure at the old boundary: `sequence_groups::document_fail_fast_{true,false}_is_refused_before_anything_runs`
  (`RemovedGrammar`, nothing runs) and the grammar tests in `sequence_schema.rs`.

### Lint follow-up: `KeyDeclarations`

- `just lint` in `simplified-schema` failed on `large_enum_variant`: the two new inline `SchemaShape` fields grew every
  enum holding a shape past the 200-byte difference limit. Both now live in one boxed
  `SchemaShape::key_declarations: Option<Box<KeyDeclarations>>` (re-exported), read through `shape.removed()` and
  `shape.advise_unknown_keys()`; the earlier notes in this phase that name `SchemaShape::removed` and
  `SchemaShape::advise_unknown_keys` mean these accessors. Darkmatter's `call_check.rs::ContractVerdict::Rows` boxes
  its `PropertyDef` for the same reason, and a Phase 1 `type_complexity` lint in `darkmatter/lib/tests/l1/findings.rs`
  was fixed so the Darkmatter lint gate passes.
- Darkmatter tests that pinned the old identities were updated: `shipped_schema_corpus.rs` (the three new kinds are
  `YamlSchemaDefinition`s, a `removed(…)` value is not parsed as a property, and `sequence-document.yaml` no longer
  declares `null`), `cli/tests/l1/claudine_schema_bundle.rs` and `dmls/tests/l1/claudine_schema_discovery.rs` (a removed
  key is reported at its owner with the replacement), and DMLS's `position_parity_tests.rs` (the published message is
  `problem_message`, the message plus hint).

### Gates

- `simplified-schema`: `just lint` clean; `just test` 2221 passed (plus 2645 in the lint recipe's test pass).
- `darkmatter` (darkmatter, darkmatter-cli, darkmatter-lsp, zed-dmls-cli): `just lint` clean; `just test` 18378
  passed, 10 skipped, none at or above 5 s.
- `claudine`: `just lint` clean; `just test` 13097 passed, 16 skipped, none at or above 5 s.
  `just check-tier-coverage claudine`: Stranded (0).
- Root `just shipped-schemas`: only `claudine/lib/generated/shipped.json` changes.
- Cross-OS: no `just cross-check`. The change adds no path, process, or `cfg` code; fixtures are LF on every
  checkout (`.gitattributes` `eol=lf`), and the CRLF cases are built in the tests.
- `just publish-skills` run in `claudine` and `darkmatter`.
- Phase 2 incomplete: **Integrate Schema Checks** stays unticked (see `message-to-agent`).

## Phase 3

Started 2026-11-26. Read Phase 2's gate result first: everything but the checkpoint **Integrate Schema Checks**
landed; its open items (1) and (2) (judge every task, group, and sequence site, including referenced files, and
check typed fields passively) are owned by this phase's walker.

Note on phase numbering: the pipeline prompt calls this "Phase 3 of 3", but the plan's frontmatter says
`total-phases: 8` and Phases 4–8 are unstarted. This run implements Phase 3 only (see the end of this section for
how the completion flags were set).

### Capture Run Baselines

`cli/tests/l1/sequence_preflight_baselines.rs` (gated `test-fixtures`, the compiled `claudine-fake-goose` as a
launch marker) runs the real `claudine sequence` before any library change and pins exit status, the root
`finalize`'s `err.code err.category err.disposition err.origin` (written to `events.log`), and that nothing launched,
for a graph refusal (empty group `tasks`), a missing `prompt:` reference, an unknown `with.agent` provider, a missing
root input (Phase 1c), and a case where the dry run's task-by-task order differs from the real failure order (task 2's
missing file wins over task 1's missing input). Phase 7 compares against these.

Recorded values (all exit 1, nothing launched): the graph, reference, and provider cases all reach `finalize` as
`composition.failed composition correctable author` today, so each case also pins the text naming its typed cause
(`` `tasks` must declare at least one task``, `` failed to load external sequence file: `./missing.md` ``,
`references an invalid Agent provider 'nosuch'`); the Phase 1c case is `composition.missing_properties composition
correctable author`. The catch-all `composition.failed` on these preflight refusals is pre-existing (Phase 8's
Rule 16 sweep owns it); the baselines pin it so that change is visible, not accidental.

### Recover Graph

The recovering walk is the real preflight `Loader` in a check mode, not a second parser:
`sequence/preflight/check.rs::check_sequence_tree` builds the root plan the way `resolve_sequence_plan_with` does,
runs input discovery, then `Loader::walk_plan` with `Loader::check = Some(CheckState)`. A real run never sets
`check`, so every fail-fast entry point and its error order are unchanged (the library's 1001 sequence, preflight,
normalize, formal, and declaration tests pass unchanged, and the five baselines above still pass).

- `normalize.rs`: `normalize_plan_at` is split into per-item `parse_item` and `assemble_plan`;
  `normalize_plan_recovering_at` returns a `RecoveredPlan { plan, ordinals, failures }` in which an unreadable item
  keeps its authored index. Recovery reads an item shallowly (`grammar::validate_task_grammar_shallow`), so a bad
  child of an inline group or nested list fails that child, not its parent.
- `formal.rs::normalize_formal_plan_recovering` applies the template (a template failure still fails the whole
  list) and validates `$state` per row (`declarations::validate_step`); `prepare_state_schema` is shared with the
  fail-fast path.
- `source.rs::plan_referenced_sequence_recovering` shares `plan_referenced` with the fail-fast
  `plan_referenced_sequence`, so a referenced YAML, JSON5, JSONL, or Markdown sequence recovers item by item exactly
  as an inline one does (found by the coverage tests: before this, one bad row of a referenced file failed its
  whole referencing task).
- `Loader::check_task` wraps every root step, sequence child, and group leg: it records the task before its
  children (authored order, container first), pushes the enclosing frame, and turns a failure into a record at that
  task. `recover_root` / `recover_in_task` keep root- and container-level failures (removed keys, lifecycle parse,
  container events) from ending the walk.
- `resolve_reference` takes the `ExecutableField` it resolves for; in check mode a no-match is also turned into a
  `darkmatter::findings::reference::missing_document` finding (`source.rs::reference_finding`, construct `prompt:`,
  `task:`, `group:`, or `sequence:`, base = the referencing file). Every loaded data file is recorded once
  (`note_loaded_file`) for the schema check; the loader's per-invocation source cache is shared as before.
- `inputs.rs::InputDiscoveryOptions::recovering()` makes discovery skip a list item, leg, or owner it cannot read
  instead of failing, so one bad task does not hide every other task's inputs.
- The library's public site type is `TaskSite`; the CLI provider selector's private type is renamed `SelectionSite`.

### Rehost Document Check

`commands/document_check/tasks.rs` replaces the hand-written reading of the root list (`check_sequence`,
`check_step`, `load_prompt`, and the root-only `check_schema_keys` are gone from `mod.rs`). It calls
`check_sequence_tree` and registers one subject per `CheckedTask` in authored order, with a `group`/`sequence` host
frame per enclosing task in its trail and a label that names the enclosing tasks (`first → inner`), so each
occurrence of a file referenced twice reads apart. Finding codes and messages are unchanged (the fleet snapshots
pass unchanged). A reference finding is located at the reference only when the task's own file wrote it (a missing
prompt inside a `task:` file is not located at the referencing site).

- Schema checks moved onto the walk: every file in `SequenceCheck::files` is judged against its kind's definition
  (Phase 2 checkpoint item 1). Typed values are judged by the passive value preparation below (item 2).
- Inputs: `prompt_inputs` prepares each prompt task's document values (`prepare_sequence_document_values`) under the
  setters and the task's `with:`; a value from a frontmatter `$( … )` or a `with:` `{{ … }}` leaves inputs not
  checked. Owner inputs (root, root reference, nested owners) come from the walk's inventory. Each missing value is
  one `composition.missing_properties` row under its task, graded once the whole cohort is known
  (`missing_input_severity`: warning only when the run could prompt and every missing shape is promptable). A task
  with a missing input leaves its preview not checked.
- `not checked` remains only for a command-backed list (root `$( … )`), a value known at launch (a command-backed
  or `{{ }}` input), a check blocked by another finding (provider and inputs after an unreadable prompt), and the
  `shell:`/`side_effect:` tasks a dry run never runs.

### Measure Coverage

Behavior → test map (L1 unless noted):

| Behavior | Test |
| --- | --- |
| A failed second of three tasks leaves the third read | `check_tree_tests::a_failed_second_of_three_tasks_leaves_the_third_read`, `dry_run_check::a_failed_task_does_not_stop_the_tasks_after_it` |
| Nested groups: bad leg at its own identity, frames outermost first | `check_tree_tests::nested_groups_record_a_bad_leg_at_its_own_identity` |
| Catalog identity: authored index and name kept; an undefined entry fails only its task; `groups` not a list | `check_tree_tests::a_catalog_entry_keeps_its_authored_place`, `…::a_catalog_without_a_groups_list_fails_its_task` |
| Cycles fail only the closing task | `check_tree_tests::a_cycle_fails_only_the_task_that_closes_it` |
| Repeated references: one record and one finding per occurrence, file read once | `check_tree_tests::a_repeated_reference_is_checked_at_each_occurrence`, `dry_run_check::nested_occurrences_and_missing_inputs_are_reported_under_their_task` |
| Dynamic (command-backed) root list left unread | `check_tree_tests::a_command_backed_root_list_is_left_unread` |
| Root shape: wrong whole type, authored null, empty list, absent key | `check_tree_tests::the_root_list_shape_decides_what_is_read` |
| Item validators: null, number, list, nameless, two executables (incl. `sequence`), empty and null references, empty and wrong-type names; all-invalid list | `check_tree_tests::invalid_root_items_keep_their_positions`, `…::an_all_invalid_root_list_reports_every_item` |
| Task and group fields: `with: {}`, wrong-type `with`/`when`, mixed and all-invalid inline legs, `tasks` not a list | `check_tree_tests::task_and_group_fields_fail_at_their_property` |
| A control and one invalid child per entry point: referenced group, external task, catalog entry, referenced sequence | `check_tree_tests::every_entry_point_reports_its_bad_child_beside_its_control` |
| Lenient rows: bare name, number, boolean, nameless ordinal; null refused alone | `check_tree_tests::lenient_rows_keep_their_projection_and_refuse_only_null` |
| Loaders: YAML duplicate / trailing garbage / two documents, `.json` duplicate, JSON5 garbage, JSONL duplicate after a blank line (line 3 kept), NDJSON garbage, Markdown duplicate frontmatter, `.yml`; unreadable vs. failed-to-parse | `check_tree_tests::a_document_that_fails_to_parse_fails_only_its_referencing_task` |
| JSON5 comments and trailing commas; explicit YAML null at a referenced root | `check_tree_tests::loader_controls_read_their_admitted_syntax` |
| Spawned CLI per loader family: located cause, control still previewed, no command run | `dry_run_check::a_document_that_fails_to_parse_fails_only_its_task` |
| Root refusals before the list is read (removed reads with a command-backed list; reported once) | `check_tree_tests::root_refusals_are_made_before_the_list_is_read`, `dry_run_check::a_setter_naming_a_declaration_is_a_finding` |
| Referenced prompt lifecycle literals refused at the task; `compose` root lifecycle and literals | `check_tree_tests::a_prompts_nested_lifecycle_literal_is_refused_at_its_task`, `dry_run_check::a_nested_lifecycle_literal_is_a_finding`, `dry_run_check::compose_reports_a_renamed_lifecycle_event` |
| Missing input cohort grading; property paths | `document_check::tests::a_missing_input_warns_only_when_the_whole_cohort_can_be_collected`, `…::property_segments_follow_keys_and_indexes` (unit) |
| Real runs keep failing fast with the same facets | `sequence_preflight_baselines::*` |

The library cases call `check_sequence_tree` in process (`preflight/tests/check_tree.rs`, 17 tests, about 1.5 s
each including fixture setup); only the loader, occurrence, literal, and setter cases spawn the CLI.

### Exit gate

Run 1: the Claudine CLI L1 suite with the seam forced on for the whole run (a temporary edit making
`document_check::enabled()` return the human report whenever the variable is not `json`, because the fixture scrubs
inherited `CLAUDINE_*` variables; reverted afterwards, verified by `grep`): 5414 run, 231 failed (the Phase 1 switch
attempt recorded 285). Classifying them found three kind (d) gaps, all fixed in this phase:

1. A root refusal made before any list is read (`commands/sequence.rs`: a setter naming a declaration, root
   grammar, lifecycle, removed `seq.*` reads in frontmatter, body, and lifecycle) was reported only when the list
   was read; with a `$( … )` root list the check said nothing (`sequence_errors_cli::a_rename_error_precedes_dynamic_sequence_source_effects`,
   `sequence_sources_cli::a_setter_cannot_write_a_sequence_declaration`). `check.rs::check_root` now makes them
   first, and the walk no longer repeats them in check mode.
2. A referenced prompt's single-pass lifecycle literal (`PreflightGraph::validate_prompt_lifecycle_literals`, run
   after the graph) was not checked (`wrap_compose_validation::sequence_referencing_the_incident_in_step_two_starts_no_step`).
   The per-document `validate_lifecycle_literals` is now shared and recorded at each task composing the prompt.
3. A referenced sequence file's bad row failed its whole referencing task instead of only that row (found by the
   coverage tests, not the suite): `plan_referenced_sequence_recovering`.

The same pass also fixed the `compose` root's lifecycle parse and literals
(`lifecycle_renamed_events::compose_dry_run_refuses_a_composed_key_naming_start`,
`wrap_compose_validation::compose_rejects_the_pre_fix_incident_before_any_provider_starts`; not the walker, but a
refusal the check missed) and the `not checked` reason after a task failure, which said "the prompt file could not be
read" for any failure (it now says "the task could not be read" unless a reference failed).

Run 2, after the fixes (same forced seam, reverted afterwards): 5417 run, 230 failed, zero of kind (d):

- **(b) provider, model, flag, or startup — 157.** Almost all are a dry run with no provider on `PATH` or no
  provider chosen, which the old preview allowed ("decided at launch") and the check reports as "no provider for
  this task/document; a non-interactive run fails here" (exit 2); plus provider tails, `--perf`, argv handling, the
  launch model notice, and the provider list. By file: `entry_point_parity` 54, `sequence_sources_cli` 31,
  `compose_prompt_tiers` 17, `sequence_errors_cli` 13, `authored_text_rendering` 6, `argv_normalization` 5,
  `wrap_sequence_composition` 4, `ctx_launch_anchor` 4, `provider_tail_launch` 4, `completion_resolution_round_trip`
  3, and 1–2 each in 15 more files.
- **(a) the old preview's text, format, or effects — 59.** Rendered `shell:`/`side_effect:` tasks, `Sequence: N
  step(s)` and other status lines, the stdout/stderr layout, shell approval prompts and gates (including the two
  PTY approval tests), `--dry-run-shell` (now refused), shell `timeout:` values (parsed when the task runs, not in
  preflight), and refusals the check reports in its own words (removed `seq.*` grammar, two executables, a
  dynamic `with:` key, the nested-span incident).
- **(c) deferred values or preview marking, Phase 4 — 14.** A preview that depends on a command or a launch value
  (`ctx_per_run` ×6, the shell-expanded root list, launch-directory and repository transclusion, task `with:` values
  known at launch); the frozen-complete reproduction, where the root's missing `spec` is reported but the nested
  preview still composes with it absent; and `inline_compose_indented_frontmatter::an_indented_baseline_is_refused_by_a_dry_run_too`:
  the inline baseline rehearsal (`prepare.rs`, before composition) is not run by the preview, which Phase 4's
  **Mark Previews** decision (compose through the canonical preparer's passive mode) settles.

Without the seam, `just test` and `just lint` in `claudine` were run after these changes (results below). The fleet
snapshots are unchanged: the inputs check added no row to either fleet fixture, since neither has a missing input.

Gate results, without the seam (2026-11-26):

- `just test` in `claudine`: 13126 tests run, 13126 passed, 16 skipped; "none at or above 5.0s". The first run
  failed one guard, `lifecycle_one_implementation_guard`, whose ParserTwin allowlist named `validate_task_grammar`;
  the `RemovedGrammar` site now sits in `validate_task_grammar_at`, so the entry was renamed. A second run had six
  `completion_delayed` failures caused by running `just lint` beside it (its `lint-transport` step rebuilt
  `target/debug/claudine` without `test-fixtures` mid-run); the third run, alone, is the one recorded here.
- `just lint` in `claudine`: passes (clippy `-D warnings` on `claudine-catalog-types`, `claudine`,
  `claudine-contract`, `claudine-cli`; transport and lifecycle-doc-facet guards). Four clippy findings in new code
  were fixed first (`clone_on_copy`, `type_complexity` via a `RecoveredItems` alias, `obfuscated_if_else`, and a
  `result_large_err` closure in `check_single`).
- `just check-tier-coverage claudine`: no stranded or stubbed tier.
- Dependents: `claudine` has two in-workspace dependents, `claudine-cli` and `claudine-contract`, both inside
  `just test` here; no other package needed a run. No cross-OS run: the change is path-neutral (no new
  `#[cfg]`, path comparison, or process code; reference bases reuse the existing `to_forward_slash_string`).
- `affected_scope.py` over the working tree: `claudine`, `claudine-cli`, plus `darkmatter`, `darkmatter-cli`,
  `darkmatter-lsp`, `simplified-schema`, and `test-toolkit` from the uncommitted Phase 1 and 2 changes in the same
  tree (Phase 3 changed only Claudine files).
- Fleet snapshots (`dry_run_check` fleet-run1/run2) unchanged.

Phase 3 closed: all four tasks ticked. The Phase 2 checkpoint **Integrate Schema Checks** stays unticked (see
`message-to-agent`).

Completion flags: `completed-phase: 3` and the cumulative `source-code`/`documentation` lists are set on the plan and
this log; the spec gains `implemented-by: "claude/opus"` but not `implemented: true`, because the plan has eight
phases and Phases 4–8 are unstarted (recorded in `decisions-for-review`).

## Phase 4

Started 2026-11-26. Phase 3's exit gate is met (log above: zero kind (d) failures); Phase 4 starts from its 14
kind (c) failures.

### Defer Unavailable Values

- New `darkmatter::markdown::compose::passive` (`PassiveSession`, `Deferral`, `DeferredConstruct`,
  `DeferralReason`). `ComposeOptions::with_dry_run()` now installs a session (shared by every
  clone) and forces `CacheAccessMode::Off`; `dry_run: bool` is gone. A deferred value keeps its
  authored text; a read of it raises `ExpressionError::Unavailable`, enforced at the shared variable
  read (`expression/mod.rs`, the interpolation fast path) and at function dispatch for every
  `SlotClass::Rejected` function except `as_markdown` (`functions/mod.rs::passive_refusal`).
- Surfaces keep their text: `{{ }}` spans (`interpolation/rewrite.rs`), whole `::block` regions
  (`page_blocks/engine.rs`), frontmatter values, which are deferred in turn
  (`frontmatter_interpolation.rs`, `frontmatter_shell_expansion.rs`). Body `::shell` /
  `::shell-block` stages report disabled in a dry run and their sites are recorded
  (`inline/shell_expansion.rs::record_deferred_command_sites`).
- Read policy: schema slots resolve in `SessionMode::ReferenceCheck` and an unresolved template is a
  `SchemaTemplate` deferral; the remote runtime gets no persistent store; `deny` keeps its refusal.
  No network `ask` exists (decision recorded).
- Tests: `darkmatter/lib/tests/l1/passive_compose.rs` (11 tests: the exit-gate document with one
  `$( … )` value, body shell, effectful vs pure functions, callback and fallback, a transcluded
  child, blacklist, unapproved command, a rejected call, and wiremock `allow`/`deny`/remote include
  with request counts and cache-folder control), `dynamic_schema_slot_positions.rs`
  (`a_dry_run_leaves_an_unapproved_slot_read_unchecked_without_asking`), and the `passive.rs`
  unit tests. The old `SkippedInDryRun` dry-run tests in `findings.rs` were removed.

### Prove Empty Bodies

- `EmptyBodyCause::ExcludedBlocks { blocks }` with `ExcludedBlock { line, condition }`, recorded
  from page-block evaluation into `ComposeReport.excluded_blocks` (top-level, proven false only);
  `EmptyBodyCause::classify_composed`. Claudine's `ComposedBodyEmpty` carries `cause` from both
  `prepare.rs` sites; `render/provider.rs` explains it from the cause, and the guessed "every block
  was stripped" text is gone.
- Tests: `claudine/lib/src/composition/error/tests.rs`
  (`composed_body_empty_names_each_excluded_block_and_claims_nothing_else`),
  `prepare/tests.rs` (whitespace and excluded-block causes), and the CLI
  `dry_run_check::an_empty_body_names_the_blocks_that_were_left_out`.

### Mark Previews

- `document_check/preview.rs` prepares through `composition::prepare_direct` /
  `prepare_inline` with `PrepareOptions.passive`; the `fm_insert` of setters and `with:`, the
  `compose_with` call (and its `composition_seams` allowlist entry), `skipped_command`, and
  `check_single`'s lifecycle stand-in are deleted. Each deferral is a `not checked` row; the human
  preview appends `[not evaluated in a dry run]` to each deferred span or directive line;
  `ComposedBodyEmpty` becomes `dm.compose.empty_body`.
- `prompt_inputs` prepares passively too, so a command value no longer skips the input check.
- A prompt task's `with:` reaches the preview as the walk evaluated it
  (`CheckedTask::{with_evaluated, with_late}`, recorded by `Loader::note_prepared_with`), as data
  beneath the setters. A prompt whose owner misses an input is held (inputs and preview not
  checked).
- Tests: `dry_run_check::{commands_and_side_effects_never_run (updated),
  a_preview_leaves_what_needs_an_effect_as_written_and_marks_it,
  a_nested_task_previews_with_values_known_at_launch_and_waits_for_a_missing_input,
  an_inline_preview_refuses_a_baseline_the_run_could_not_edit}` and the `preview.rs` unit test
  `each_construct_left_as_written_is_marked_and_nothing_else_is`.
- Seam forced on (temporary `enabled()` edit, reverted): 220 → 218 CLI L1 failures, none new. Of
  Phase 3's kind (c) items, `frozen_complete_reproduction` setter-before/after now pass; the
  `ctx_per_run`, repository-context, launch-directory, and inline-baseline tests now preview
  correctly and fail only on old-preview expectations (`--perf` notes, exit status 2 for a
  cycle, no provider requirement, the old refusal wording); a `$( … )` root list is not checked by
  design. Those suites move in Phase 5 "Migrate Preview Suites".

### Prove Passivity

- Darkmatter fixtures cover each effect (frontmatter shell, body shell, effectful function,
  approval-needing slot read, remote include/data reads under `allow`/`deny` with a counting
  loopback responder, a transcluded child, a callback, and a refused call). Claudine's
  `a_preview_leaves_what_needs_an_effect_as_written_and_marks_it` asserts both marker files absent
  and the fixture home unchanged.
- Cross-OS: no `just cross-check`; the change adds no path comparison or OS-specific code (the
  preview marker splits lines on `\n` and trims, so CRLF reads the same).

### Exit gate

- The exit-gate document is `passive_compose::one_command_value_defers_only_what_reads_it` and,
  through Claudine, `a_preview_leaves_what_needs_an_effect_as_written_and_marks_it`.
- `darkmatter`: `just test` 18390 passed, none at or above 5 s; `just lint` clean.
- `claudine`: `just test` 13132 passed, none at or above 5 s; `just lint` clean;
  `just check-tier-coverage claudine` clean.
- `affected_scope.py` over the working tree: packages `claudine`, `claudine-cli`, `darkmatter`,
  `darkmatter-cli`, `test-toolkit`; `cargo check --all-targets` of the 11 reverse dependencies is
  clean. `just test test-toolkit` failed once on a pre-existing archive-path violation in
  `claudine/lib/src/doctor/registration_tests.rs:1123` (committed in `cf255aa5d`, not this phase):
  fixed to `biscuit_test_harness::manifest_dir!()`, then 591 passed.
- Docs: `darkmatter/docs/inline/preflight-checks.md` ("Dry runs": what is left as written, how
  deferral spreads, what is still refused), `darkmatter/docs/topics/caching.md`,
  `claudine/docs/cli/sequence.md` (planned passive check: preview marking); skills:
  `use-darkmatter` `library.md` (published with `just publish-skills`), `.claude/skills/claudine`.

## Phase 5

Started 2026-11-26. The Phase 4 exit gate is recorded as met (log above), so no earlier gate is open.

### Route Dry Runs (switch)

- `main.rs::dispatch` routes every `compose`/`inline-compose`/`sequence` `--dry-run` to
  `commands/document_check::run`; `document_check::enabled()` and the `test-fixtures` seam
  `CLAUDINE_TEST_PASSIVE_DRY_RUN` are deleted. `--json` is declared on `SharedComposeArgs` with
  `requires = "dry_run"`; `--dry-run-shell` is hidden and refusal-only (no `requires`), refused in
  `dispatch` right after the `--output` refusal, with or without `--dry-run`.
