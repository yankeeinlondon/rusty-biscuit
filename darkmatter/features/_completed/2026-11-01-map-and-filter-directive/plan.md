---
kind: plan
created: 2026-10-27
total_phases: 6
phase: 7
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1:
- darkmatter/cli/src/commands/render.rs
- darkmatter/lib/src/markdown/compose/body_origin.rs
- darkmatter/lib/src/markdown/compose/directive_targets.rs
- darkmatter/lib/src/markdown/compose/expression/ast.rs
- darkmatter/lib/src/markdown/compose/expression/binding.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/parser.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/parse_utils.rs
- darkmatter/lib/src/markdown/compose/perf.rs
- darkmatter/lib/src/markdown/compose/pipeline/operations.rs
- darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
- darkmatter/lib/src/markdown/compose/type_tests.rs
- darkmatter/lib/src/markdown/errors/mod.rs
- darkmatter/lib/src/markdown/types.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/types.rs
- darkmatter/lib/src/markdown/compose/expression/iteration.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/tests/l1/expression_scopes.rs
- claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
- darkmatter/lib/src/markdown/cleanup/protect.rs
- darkmatter/lib/src/markdown/compose/block_pairs.rs
- darkmatter/lib/src/markdown/compose/call_precheck.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/types.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/context/capture/groups.rs
- darkmatter/lib/src/markdown/compose/context/effective_state.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/directive_targets.rs
- darkmatter/lib/src/markdown/compose/directives_api.rs
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/binding.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs
- darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
- darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
- darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
- darkmatter/lib/src/markdown/compose/expression/iterable.rs
- darkmatter/lib/src/markdown/compose/expression/lexer.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/parser.rs
- darkmatter/lib/src/markdown/compose/expression/prepared.rs
- darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/expression/scope/lexical.rs
- darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
- darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/page_blocks/guards.rs
- darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/parse_utils.rs
- darkmatter/lib/src/markdown/compose/policy.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
- darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
- darkmatter/lib/tests/l1/binding_contract.rs
- darkmatter/lib/tests/l1/collection_analysis.rs
- darkmatter/lib/tests/l1/expression_type_scope.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/scope_site_conformance.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- darkmatter/lib/src/markdown/compose/expression/iteration.rs
- darkmatter/lib/src/markdown/compose/expression/binding.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/compose/body_origin.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs
- darkmatter/lib/src/markdown/compose/inline/mod.rs
- darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
- darkmatter/lib/src/markdown/compose/inline/interpolation.rs
- darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/directive_targets.rs
- darkmatter/lib/src/markdown/compose/site_scope.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/compose/nested.rs
- darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
- darkmatter/lib/tests/l1/collection_execution.rs
- darkmatter/lib/tests/l1/collection_effects.rs
- darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
- darkmatter
- darkmatter-cli
- dmls
- claudine
source_files_during_phase_4:
- darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
- darkmatter/lib/src/markdown/compose/body_origin.rs
- darkmatter/lib/src/markdown/compose/cache/hashing.rs
- darkmatter/lib/src/markdown/compose/cache/runtime.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/expression/iteration.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/preflight/mod.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/reference/graph.rs
- darkmatter/lib/tests/l1/collection_discovery.rs
- darkmatter/lib/tests/l1/main.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
- darkmatter/cli/src/args/command.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/parsers.rs
- darkmatter/cli/src/args/policy.rs
- darkmatter/cli/src/collection_failure.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/compose/report.rs
- darkmatter/cli/src/commands/mod.rs
- darkmatter/cli/src/io/mod.rs
- darkmatter/cli/src/lib.rs
- darkmatter/cli/tests/l1/collection_blocks.rs
- darkmatter/cli/tests/l1/compose_collection_controls.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/overlay/collections.rs
- darkmatter/dmls/src/overlay/directives.rs
- darkmatter/dmls/src/overlay/expressions.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/providers/collections.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/providers/mod.rs
- darkmatter/dmls/src/providers/semantic_tokens.rs
- darkmatter/dmls/src/semantic_legend.rs
- darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
- darkmatter/dmls/tests/l1/call_check_agreement.rs
- darkmatter/dmls/tests/l1/collection_editor.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
- darkmatter/lib/src/markdown/compose/call_precheck.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
- darkmatter/lib/tests/fixtures/collection_blocks/filter.expected.md
- darkmatter/lib/tests/fixtures/collection_blocks/filter.md
- darkmatter/lib/tests/fixtures/collection_blocks/map.expected.md
- darkmatter/lib/tests/fixtures/collection_blocks/map.md
- darkmatter/lib/tests/l1/collection_blocks.rs
- darkmatter/lib/tests/l1/collection_blocks/budget.rs
- darkmatter/lib/tests/l1/collection_blocks/docs_corpus.rs
- darkmatter/lib/tests/l1/collection_blocks/grammar.rs
- darkmatter/lib/tests/l1/collection_blocks/pipeline.rs
- darkmatter/lib/tests/l1/collection_blocks/scope_schedule.rs
- darkmatter/lib/tests/l1/collection_blocks/support.rs
- darkmatter/lib/tests/l1/collection_blocks/types_values.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/entry_point_parity.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/scope_site_conformance.rs
- darkmatter/lib/tests/l1/semantic_results_never_persist.rs
- darkmatter/lib/tests/l1/shared_diagnostics.rs
- claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
- claudine/lib/src/composition/prepare/bootstrap/tests.rs
- claudine/lib/tests/l1/collection_host_compat.rs
- claudine/lib/tests/l1/main.rs
docs_updated_during_phase_5:
- darkmatter/docs/cli/compose.md
- darkmatter/docs/cli/policy.md
- darkmatter/docs/cli/report-json.md
- darkmatter/dmls/docs/autocomplete.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/dmls/docs/features.md
- darkmatter/dmls/docs/hover.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
- darkmatter/README.md
- darkmatter/docs/darkmatter-compose-pipeline.md
- darkmatter/docs/inline/filter.md
- darkmatter/docs/inline/map.md
- darkmatter/docs/inline/page-blocks.md
- darkmatter/docs/inline/shell-blocks.md
- darkmatter/docs/inline/shell-expansion.md
- darkmatter/docs/iterables.md
- darkmatter/docs/topics/compose-requests.md
- darkmatter/docs/topics/dmls.md
- darkmatter/docs/topics/expression-engine.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/testing.md
source_code:
- claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs
- claudine/lib/src/composition/prepare/bootstrap/tests.rs
- claudine/lib/tests/l1/collection_host_compat.rs
- claudine/lib/tests/l1/main.rs
- darkmatter/cli/src/args/command.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/parsers.rs
- darkmatter/cli/src/args/policy.rs
- darkmatter/cli/src/collection_failure.rs
- darkmatter/cli/src/commands/compose.rs
- darkmatter/cli/src/commands/compose/report.rs
- darkmatter/cli/src/commands/mod.rs
- darkmatter/cli/src/commands/render.rs
- darkmatter/cli/src/io/mod.rs
- darkmatter/cli/src/lib.rs
- darkmatter/cli/tests/l1/collection_blocks.rs
- darkmatter/cli/tests/l1/compose_collection_controls.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/dmls/src/diagnostics/codes.rs
- darkmatter/dmls/src/overlay/collections.rs
- darkmatter/dmls/src/overlay/directives.rs
- darkmatter/dmls/src/overlay/expressions.rs
- darkmatter/dmls/src/overlay/mod.rs
- darkmatter/dmls/src/providers/collections.rs
- darkmatter/dmls/src/providers/dsl.rs
- darkmatter/dmls/src/providers/frontmatter.rs
- darkmatter/dmls/src/providers/mod.rs
- darkmatter/dmls/src/providers/semantic_tokens.rs
- darkmatter/dmls/src/semantic_legend.rs
- darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
- darkmatter/dmls/tests/l1/call_check_agreement.rs
- darkmatter/dmls/tests/l1/collection_editor.rs
- darkmatter/dmls/tests/l1/expression_scopes.rs
- darkmatter/dmls/tests/l1/main.rs
- darkmatter/dmls/tests/l1/type_scope_ownership_guard.rs
- darkmatter/lib/src/lifecycle/preflight/shell_walk.rs
- darkmatter/lib/src/markdown/cleanup/protect.rs
- darkmatter/lib/src/markdown/compose/block_pairs.rs
- darkmatter/lib/src/markdown/compose/body_origin.rs
- darkmatter/lib/src/markdown/compose/cache/hashing.rs
- darkmatter/lib/src/markdown/compose/cache/runtime.rs
- darkmatter/lib/src/markdown/compose/call_precheck.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/engine.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/error.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/scopes.rs
- darkmatter/lib/src/markdown/compose/collection_blocks/types.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/context/capture/groups.rs
- darkmatter/lib/src/markdown/compose/context/effective_state.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/report.rs
- darkmatter/lib/src/markdown/compose/directive_targets.rs
- darkmatter/lib/src/markdown/compose/directives_api.rs
- darkmatter/lib/src/markdown/compose/expression/absence.rs
- darkmatter/lib/src/markdown/compose/expression/ast.rs
- darkmatter/lib/src/markdown/compose/expression/binding.rs
- darkmatter/lib/src/markdown/compose/expression/call_check.rs
- darkmatter/lib/src/markdown/compose/expression/catalog/roots.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/functions/fs.rs
- darkmatter/lib/src/markdown/compose/expression/functions/paths.rs
- darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
- darkmatter/lib/src/markdown/compose/expression/iterable.rs
- darkmatter/lib/src/markdown/compose/expression/iteration.rs
- darkmatter/lib/src/markdown/compose/expression/lexer.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/parser.rs
- darkmatter/lib/src/markdown/compose/expression/prepared.rs
- darkmatter/lib/src/markdown/compose/expression/scope/facts.rs
- darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
- darkmatter/lib/src/markdown/compose/expression/scope/lexical.rs
- darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
- darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
- darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
- darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs
- darkmatter/lib/src/markdown/compose/inline/interpolation.rs
- darkmatter/lib/src/markdown/compose/inline/mod.rs
- darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
- darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
- darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/nested.rs
- darkmatter/lib/src/markdown/compose/page_blocks/guards.rs
- darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/parse_utils.rs
- darkmatter/lib/src/markdown/compose/perf.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/operations.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/compose/policy.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/preflight/mod.rs
- darkmatter/lib/src/markdown/compose/report_diagnostics.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/site.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/mod.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/compose/site_scope.rs
- darkmatter/lib/src/markdown/compose/tests/lookup_parity.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/compose/type_tests.rs
- darkmatter/lib/src/markdown/errors/mod.rs
- darkmatter/lib/src/markdown/reference/graph.rs
- darkmatter/lib/src/markdown/types.rs
- darkmatter/lib/tests/fixtures/collection_blocks/filter.expected.md
- darkmatter/lib/tests/fixtures/collection_blocks/filter.md
- darkmatter/lib/tests/fixtures/collection_blocks/map.expected.md
- darkmatter/lib/tests/fixtures/collection_blocks/map.md
- darkmatter/lib/tests/l1/binding_contract.rs
- darkmatter/lib/tests/l1/collection_analysis.rs
- darkmatter/lib/tests/l1/collection_blocks.rs
- darkmatter/lib/tests/l1/collection_blocks/budget.rs
- darkmatter/lib/tests/l1/collection_blocks/docs_corpus.rs
- darkmatter/lib/tests/l1/collection_blocks/grammar.rs
- darkmatter/lib/tests/l1/collection_blocks/pipeline.rs
- darkmatter/lib/tests/l1/collection_blocks/scope_schedule.rs
- darkmatter/lib/tests/l1/collection_blocks/support.rs
- darkmatter/lib/tests/l1/collection_blocks/types_values.rs
- darkmatter/lib/tests/l1/collection_discovery.rs
- darkmatter/lib/tests/l1/collection_effects.rs
- darkmatter/lib/tests/l1/collection_execution.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/entry_point_parity.rs
- darkmatter/lib/tests/l1/expression_type_scope.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/scope_site_conformance.rs
- darkmatter/lib/tests/l1/semantic_results_never_persist.rs
- darkmatter/lib/tests/l1/shared_diagnostics.rs
documentation:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/compose.md
- .claude/skills/darkmatter/dmls.md
- .claude/skills/darkmatter/expressions.md
- .claude/skills/darkmatter/testing.md
- darkmatter/README.md
- darkmatter/dmls/docs/autocomplete.md
- darkmatter/dmls/docs/diagnostics.md
- darkmatter/dmls/docs/features.md
- darkmatter/dmls/docs/hover.md
- darkmatter/docs/cli/compose.md
- darkmatter/docs/cli/policy.md
- darkmatter/docs/cli/report-json.md
- darkmatter/docs/darkmatter-compose-pipeline.md
- darkmatter/docs/inline/filter.md
- darkmatter/docs/inline/map.md
- darkmatter/docs/inline/page-blocks.md
- darkmatter/docs/inline/shell-blocks.md
- darkmatter/docs/inline/shell-expansion.md
- darkmatter/docs/iterables.md
- darkmatter/docs/topics/compose-requests.md
- darkmatter/docs/topics/dmls.md
- darkmatter/docs/topics/expression-engine.md
completed_phase: 6
implemented: true
---

# Map and Filter Directive Implementation Plan

## Summary and Definition of Done

Implement `::map` and `::filter` as two modes of one typed collection-region
executor in Darkmatter's existing composition pipeline. The work includes
callback syntax, immutable lexical bindings, Markdown-aware structural regions,
retained body scopes and provenance, condition-blind child discovery, existing
shell-template approval, request-wide budgets, typed diagnostics, CLI controls,
and passive DMLS analysis. Expression-level `map()` and `filter()` are outside
this delivery; their shared foundations are included here because they do not
exist in the current code.

Completion means:

- Both directives meet R1–R10 and the settled pipeline ruling in the functional
  specification, through library composition and the normal `md compose` route.
  Map bodies receive transformed values; filter bodies retain original values.
- Frontmatter runs once per document. Structural callbacks finish before body
  interpolation and ordinary shell stages. Transclusions retain their existing
  concurrency and placement. Locals, authored positions, inserted-data markings,
  and original item identities survive those stages.
- Passive checking rejects static defects before effects, including dead
  branches. Preflight discovers every candidate child before selection and
  stopping, approves shell templates without evaluating local placeholders, and
  rejects execution-dependent or newly encountered targets without late approval.
- The 100,000-visit and 16,777,216-byte defaults apply across nested directives,
  child composition, explicit nested composition, and cache reuse. Positive
  library/CLI overrides agree; exhaustion is request-fatal.
- One acceptance corpus proves ordering, laziness, typing, scope, structural
  boundaries, budgets, failure policy, diagnostic parity, and existing behavior.
  L1 tests and lint pass, with macOS, Linux, native Windows, and WSL2 evidence
  recorded by environment. No new terminal/browser behavior requires a higher
  test tier.
- Both directive topic pages have `kind: canonical-documentation`, no Planned
  notice, complete examples, and the shipped controls. Related topic pages,
  overview links, relevant skill topics, and implementation-log evidence agree.

Execution is non-interactive from Phase 1 through Phase 6. Checkpoints are
automated validation, never human gates. Final human judgment occurs after the
complete implementation is ready for review. Keep changes in this worktree;
do not make an author commit, push, move the feature, or run `just complete`.

## Phase 1 — Establish Shared Contracts

### Necessary Rules

Paths below are relative to `darkmatter/` unless explicitly rooted elsewhere.
These decisions govern every later task; they are implementation contracts,
not questions for a subsequent session.

1. **This feature owns its missing shared substrate.** Add callback/pattern
   syntax, lexical frames, iterable/type projection, and iteration accounting
   once, and expose them for the later expression functions. Do not implement
   the functions or broaden executable function-valued catalog parameters here.
   Evidence: `expression/ast.rs::Expr`, `parser.rs::Parser`, and
   `binding.rs::EvaluationSession` currently have no callbacks or lexical frames;
   the current handoff in `2026-11-01-map-and-filter-expression` explicitly calls
   for shared retainable frames and patterns. Its historical Phase 1 gates are
   superseded by that handoff. The separate schema-narrowing metadata migration
   is not a dependency; use `call_binding/registration.rs::NarrowingFact`.

2. **Use one non-value callback representation.** In
   `lib/src/markdown/compose/expression/ast.rs`, introduce `BindingPattern`
   (identifier or nonempty ordered shallow fields), its spanned counterpart,
   `Callback { pattern, body }`, and `SpannedCallback` with pattern, arrow, and
   body spans plus `erase()`. Expose
   `parser::{parse_callback, parse_callback_spanned}` returning the corresponding
   callback or `ParseError`. Both arrows use the same parser and consume one
   complete expression body. Keep callbacks outside `Expr` value positions and
   `serde_json::Value`; ordinary `parse` rejects an arrow in a value position.
   Evidence: `ast.rs` already separates erased/spanned expressions, and
   `parser.rs::parse_expression` supplies ternary/call precedence. No
   `FunctionCall` argument migration is required for rendering directives.

3. **Share quoting and retain decoded offsets.** Extend
   `parse_utils.rs::Cursor` with a span-carrying decoded quoted-value result
   containing a decoded-byte-boundary to authored-byte-boundary map. Keep
   `read_value` behavior for existing callers. Collection callback/condition
   parsing uses the mapped result; errors and DMLS ranges project through it.
   Source parsing accepts exactly a lexer-valid dotted variable path, including
   dashed segments. The optional positional callback precedes options. Reject
   unknown/duplicate options, bare flags, extra positionals, unquoted expression
   values, empty expressions, `fn=`, `while` with `until`, and `sort-order`
   without `sort`; order values are exactly `asc` and `desc`.
   Evidence: `Cursor::read_quoted_value` already owns escapes and nested-quote
   compatibility but currently returns only `String`; spec R1 requires decoding
   once and authored offset preservation. Extend object-expression parsing to
   lower unquoted shorthand to the existing ordered `ObjectLiteral` entries;
   retain duplicate-key and no-trailing-comma rejection in
   `Parser::parse_object_literal`.

4. **Add lexical identity to the existing binding model.** Put retainable
   `LexicalScope` frames and a delegating `ScopedLookup` in
   `expression/binding.rs`; frames own bindings and retain their parent through
   `Arc`, with separate runtime-value/origin and passive-type views. Extend
   `ResolvedBinding`, `VariableClass`, `BindingId`, and
   `DeclaredTypes`/`TypeScope` to represent locals. A local subject uses a frame
   identity and slot, never just its spelling. Explicit `local.x` and implicit
   `x` resolve to that same subject; `fm.x` remains a document subject. Preserve
   `TypeScope`'s borrowing of the owning declarations while its lexical layers
   retain owned type definitions. Evidence: `binding.rs` owns root
   classification, `scope/projection.rs::Subject` owns stable subjects, and
   `scope/mod.rs::TypeScope` already has immutable `Arc` fact links.
   Keep `RootClass`'s existing namespace/global/document cases: `BindingView`
   classifies host declarations, while the scoped lookup and declared lexical
   layer classify local reads. Thus Claudine's exhaustive host-view classifier
   in `composition/lifecycle/prewarm/classify.rs` needs no new enum branch.

5. **Reserve `local` without changing host APIs unnecessarily.** Add
   `RootMembers::LexicalBindings` and the `local` descriptor to
   `expression/catalog/roots.rs`. Lookup order is reserved roots/registered
   globals protected from binding, then nearest permitted local, then normal
   document lookup. Local presence shadows even at null; missing members of a
   present local do not fall through. Outside a scope `local.*` is absent/null.
   Reject a pattern name colliding with any reserved root, `null`, or a global
   in the actual `BindingView`; ordinary shadowing has no warning. Extend
   `EvaluationLookup` with default lexical-scope/budget accessors and make
   `ScopedLookup` forward every context, formatting, observation, binding-view,
   and expression-scope hook. Evidence: `binding.rs::check_root`,
   `BindingView::classify_root`, and `expression/mod.rs::EvaluationLookup`
   already centralize these contracts. A host registering `local` must receive
   the existing reserved-root registration error.

6. **Give aliases of an original item one analysis identity.** Each authored
   collection has distinct abstract input and map-result subjects. Callback
   parameter names alias the input subject; object-pattern fields alias its
   field subjects. Apply selection/prefix guard facts through
   `TypeScope::assume`, then expose retained input facts in filter bodies and
   map callback input scopes. A map body binds the result subject afresh,
   including identity-map syntax; input facts do not migrate by spelling.
   Runtime iteration frames remain distinct for every source index/key.
   Evidence: spec R3–R4 explicitly requires different callback names to share
   input facts, while `BindingId` currently distinguishes document, namespace,
   and global subjects and needs a lexical arm.

7. **Project iterable and result types without sampling values.** Add
   `expression/iterable.rs` for one `IterableSource` adapter and
   `expression/scope/iterable.rs` for declared item/entry projection. Arrays
   use their element declaration, tuples join their declared members, and
   objects yield present `{key, value}` entries. Closed heterogeneous objects
   join property types; open objects include unknown additional values. Resolve
   named definitions through the effective declaration owner. Nullable/optional
   iterable sources are accepted, null elements are preserved, and actual
   absent optional object properties never become entries. Extend
   `call_check.rs::expression_type` to preserve computed object/array literal
   shapes from their members, including shorthand; other currently unknown
   expressions stay unknown. Evidence: `TypeExpr::Tuple`, `DefinitionTable`,
   `scope::Shape`, and `expression_type` exist, but the latter currently knows
   only all-literal object shapes. Recovery permits unknown source shape only;
   it never excuses a known scalar or a union still containing scalar arms.

8. **Keep the structural scanner authoritative.** Extend
   `block_pairs.rs::{BlockOpenKind, BlockPair, scan_block_pairs_in}` with map
   and filter and their distinct closers; retain page and shell on the same
   stack. Add container ancestry from `pulldown_cmark` offset events to the
   existing Markdown scan in `parse_utils.rs`, including list/item identity and
   quote depth. Do not replace it with `trim()` or a quote boolean. Closers must
   match kind and complete container path, with trailing whitespace only.
   Unclosed errors anchor the opener; crossing errors identify the closer and
   still-open inner region. Quoted collections and shell blocks are active;
   quoted page-block examples and their closers remain literal. Reject
   collection openers in a shell-block command body. Preserve body bytes,
   LF/CRLF, empty removal, indentation, and EOF closers. Evidence: the current
   scanner already owns Page/Shell pairing but stores only `quoted: bool`;
   `parse_utils.rs::scan_code_regions` already uses CommonMark offsets for
   fenced and indented code, and spec R10 settles differing activation rules.

9. **Add one independent collection operation.** Append
   `ComposeOperation::CollectionBlocks` at stable index 16, default enabled,
   with label `collection blocks` and a corresponding perf metric. Schedule it
   beside `PageBlocks`, between replacement and interpolation. Group these two
   registry entries into one structural traversal, using the existing grouped
   transclusion dispatch pattern so traversal runs once with both enablement
   bits. A disabled wrapper retains its own authored markers and contributes
   neither its condition nor its bindings; its body is traversed once in the
   enclosing scope for other enabled structural kinds, then ordinary enabled
   body stages proceed. Register `--disable collection-blocks`; page-block
   disabling does not disable collections. Frontmatter-only projection omits
   both and never traverses collection bodies. Evidence:
   `pipeline/operations.rs` preserves existing discriminants 0–15,
   `pipeline/mod.rs::run_compose_pipeline_node` already groups transclusion
   operations, and `ComposeOptions::only_frontmatter_surface` owns projection.

10. **Retain scoped authored segments through the pipeline.** Extend
    `body_origin.rs::BodyProvenance` with scope/iteration/expansion-owner ranges
    and add a copy/splice builder that can project the same authored range
    repeatedly. Ordinary `TextEdit` alone cannot represent repetition:
    `BodyOrigin::after_edits` advances through source runs only once. Preserve
    `BodyOrigin`, `AuthoredBody`, and `DataRanges` separately, and fail closed on
    data-map mismatch. Parse structural bodies once and copy their authored
    spans with immutable frame handles. Interpolation, target rewriting,
    shell/template preparation, link processing, and transclusion preparation
    obtain the scope at the authored execution site. Never compose each item
    as a document or rescan produced text as executable syntax. Evidence:
    `inline/interpolation.rs::run_stage` currently builds one lookup,
    `inline/page_blocks.rs::run_stage` emits deletion edits, and
    `pipeline/phases.rs` assumes one phase-wide state. These are explicit
    handoff changes, not a new pipeline driver.

11. **Use the settled serial callback schedule.** Gate and source resolve in
    the enclosing scope, gate first, source once. Without sort, visit →
    positional filter predicate → extra filter → prefix predicate → map →
    nested structural preparation, sequentially for each reached item. With
    sort, visit/select/key each item in source order, stable-sort the selected
    records, then prefix/map/prepare bodies in output order. Each option gets
    an independent parameter frame. Map defaults to `i => i`; filter defaults
    to `i => true`. Callbacks A and B finish before the bodies' ordinary
    interpolation/shell work. Explicit `as_markdown()` executes at expression
    evaluation time. Evidence: spec R5 and the 2026-10-27 pipeline ruling.
    Keep existing ordinary interpolation order, shell-stage ordering, and
    transclusion concurrency; this feature does not promise source-order child
    effects or add separators.

12. **Use stable homogeneous sort keys.** Store one admitted numeric or string
    key alongside each selected original entry; validate even one selected
    item. Use stable sorting with a descending comparator, never list reversal.
    Numeric comparison uses `markdown::numeric` admission; strings use Rust
    string ordering. Null/missing, booleans, containers, and mixed families fail;
    numeric strings remain strings. Retain original indices/keys separately
    from sorted positions. Evidence: spec R6 and the existing expression
    ordering's numeric coercion make the latter unsuitable for string sorting.

13. **Preserve existing ordering and extend semantic identities.** Keep the
    existing `serde_json` preserve-order maps: `frontmatter.rs::
    parse_yaml_map_rejecting_duplicates`, `MappingOrders`,
    `context/shared_frontmatter.rs::SharedFrontmatter`,
    `effective_state.rs::{deep_merge, deep_merge_override}`, and coercion's
    object conversion already preserve base order and append new keys. No new
    map representation or loader is needed. Add an order fingerprint alongside
    existing canonical value hashes wherever state, context, overlays, caller
    inputs, and locals affect semantic cache/graph keys. Encode nested object
    key sequences with their structural paths and array positions using
    `biscuit-hash`; preserve canonical treatment of genuinely unordered policy
    sets. Evidence: `cache/hashing.rs::{effective_state_hash, context_hash,
    set_overlay_hash}` and `context/options.rs::classify_options` canonicalize
    objects and otherwise conflate differently ordered iterable inputs.
    Share parsed bodies, not evaluated locals; add no persistent result cache.

14. **Separate passive analysis from graph discovery.** Introduce public
    `collection_blocks::{CollectionKind, CollectionRegion, BodyScopes}` and
    passive `analyze_collection_blocks`, backed by the shared scanner,
    expression parser, declared types, and `TypeScope`. `BodyScopes` is the
    common cursor/site scope inventory; `page_blocks::BodyGuards` remains the
    conditional projection of that traversal. Composition call checking,
    directive-target analysis, shell typing, and DMLS consume it. Validation
    runs no handlers or resolution I/O. Preflight alone enumerates typed
    sources and evaluates the dependency slice needed for targets/explicit
    child parameters, without evaluating selection, sort, prefix gates, or
    ordinary bodies. Mapping expressions needed for those targets run with
    discovery capabilities. Evidence: `call_precheck.rs::check_document`,
    `directive_targets.rs::analyze_directive_targets`, DMLS
    `providers/dsl.rs::with_site_scope`, and `preflight/collect.rs::
    collect_document` currently use page-only scopes; R8 permits read-side
    target discovery, not callback evaluation during passive analysis.
    The analysis signature is
    `analyze_collection_blocks(source: &str, site: &TypeScope<'_>, options: &CollectionAnalysisOptions<'_>) -> Result<CollectionAnalysis, CollectionError>`.
    `CollectionAnalysisOptions` supplies the enabled operation set, binding
    view, two untyped-source recovery booleans, and optional source path;
    it contains no runtime provider or resolution service. `CollectionAnalysis`
    owns regions, `BodyScopes`, and shared diagnostics. Match the existing
    guard API with
    `BodyScopes::scope_at<'d>(&self, site: TypeScope<'d>, offset: usize) -> TypeScope<'d>`.
    Incomplete editor headers use the public lenient directive scanner and
    conservative scopes up to the malformed region, never a second grammar.

15. **Track unavailable dependencies transitively.** Extend preflight's
    pending-frontmatter/host-root tracking through collection source reads,
    mapped local definitions, target expressions, and explicit child values.
    A target derived from shell output, execution-dependent host globals, or
    discovery placeholder results such as uncomposed `as_markdown` content is
    unavailable and rejected at its authored target. Preserve the original
    dependency chain in the typed discovery error; do not treat a placeholder
    null as a missing optional collection. Use the current discovery slots for
    supported effects, never invoke shell or infer authorization from
    selection. Evidence: `preflight/collect.rs::{DiscoverySiteContext,
    pending_shell_literals, detect_dynamic_command_shape}` and
    `nested.rs::NestedComposeSlot::Discover` already distinguish discovery
    results from execution. Spec R8 requires indirect dependency rejection.

16. **Key child edges by site and scope.** Extend `PreflightGraphEdge` and
    non-Markdown target edges with authored site, original iteration path,
    explicit-parameter value/origin identity, and resolved opening identity.
    Replace raw-target resolution-cache lookup and source-only child lookup
    with this key in `transclusion/engine.rs::{build_resolution_cache,
    prepare_block_transclusions, render_markdown_transclusion}`. For a graph-backed
    request, missing or differing scoped targets fail with the existing
    `NotPreApproved` meaning; do not fall back to late resolution/approval.
    Requests without a retained graph still use the normal root preflight gate.
    Child documents inherit existing document state/global policy but no local
    frames; only explicit `set` values cross, with existing origin semantics.
    Evidence: current `build_resolution_cache` keys on `raw_target`,
    `PreflightGraphNode::child_for_source` selects by source, and
    `markdown_child_options` already owns child request derivation. Preserve
    its resolution context and the request's remote policy.

17. **Extend shell timing without another permission system.** Local reads,
    including explicit aliases and reads inside expressions, make the affected
    placeholder `HoleTiming::Deferred` and candidate `TemplateRequired`.
    Type-only discovery still visits every authored shell site in empty/false
    regions. Unrelated approval-time holes retain ordinary missing-value
    checks. Extend `template/typing.rs::project_hole` to local subjects and
    owned declarations/refinements, and candidate timing via shared static
    reads. Numeric locals retain broad `string` approval plus numeric
    validation; unknown locals retain the existing scalar fallback. Runtime
    null/container arguments fail existing shell input diagnostics. Membership,
    blacklist, argument validation, cache lookup, and execution keep their
    current order. Evidence: `template/{candidate,typing,fill}.rs` already
    separate `ModeRequirement`, timing, `ApprovalType`, and `HoleValidator`.
    Preserve `::shell --no-cache` and `::shell-block no_cache=true`; identical
    approved commands may reuse the existing request-wide shell result cache.

18. **Use positive portable limit types.** Add public
    `IterationLimits` in `expression/iteration.rs`, defaulting to 100,000 visits
    and 16,777,216 expansion bytes as `NonZeroU64`. Add
    `ComposeOptions::{with_max_iteration_visits, with_max_expansion_bytes}`
    taking `NonZeroU64` and same-named getters without `with_`. Add
    `EvaluationSession::with_max_iteration_visits(NonZeroU64)` for standalone
    evaluation. CLI fields are `Option<NonZeroU64>` with the two specified
    flags; omission uses library defaults. Zero, negatives, and u64 overflow
    are argument errors, never clamped. Include limits in both exhaustive
    options identity products; consumed counters are runtime state, not key
    material. Evidence: `ComposeOptions::with_max_transclusion_depth` uses
    builder naming but clamps zero, which R9 explicitly forbids here.

19. **Share one fatal budget authority.** Add `IterationBudget`, held through
    `Arc` by `PipelineRuntime` and its `clone_for_child`, propagated into
    `ResolutionContext` and scoped lookups. A mutex protects checked u64
    counters and a sticky first-exhaustion record; it is never held while a
    callback runs. Standalone evaluation uses its session authority, or a fresh
    default authority for one top-level evaluation with all recursive calls
    borrowing it. Preflight uses a discovery mode that charges neither budget.
    Charge a visit before any predicate, including rejection and the stopping
    item; sort visits the whole source. Evidence: `PipelineRuntime` already
    shares request state across Rayon children, `nested.rs::prepare_child`
    clones that runtime, and R9 requires no fresh nested allowance.

20. **Account for materialization, including reuse.** Implement an
    `ExpansionWriter` that reserves bytes before capacity growth/append.
    Structural copies charge authored emitted body text; later replacements
    charge newly inserted expression, shell, and child bytes to every enclosing
    expansion owner exactly once. Already copied text, ordinary stage passes,
    reallocations, preflight copies, and unrelated text do not charge; consumed
    authored template text is not refunded. Copying nested expansion output
    charges each enclosing copy; expression arrays themselves charge no bytes.
    Cache entries carry `ExpansionUsage { visits, bytes }` for their subtree:
    compute paths charge normally, reuse paths reserve the recorded usage
    before cloning/inserting, and enclosing insertion is charged separately.
    Track subtree usage with per-node ledgers, not global counter subtraction,
    which races concurrent children. Evidence: `cache/runtime.rs::ComposeResult`
    currently stores only content/report/context groups and
    `transclusion/engine.rs::render_markdown_transclusion` clones cached content;
    R9 explicitly forbids cache-based budget bypass. Exactly-full succeeds.

21. **Keep typed failures and existing handlers.** Add boxed
    `MarkdownError::Collection` and shared `CollectionError` variants for
    syntax, source typing/shape, protected binding/pattern, sort, callback, and
    budget failure. Each holds authored `SourceRef`/span and an
    `IterationPath` of original indices/keys; callback errors retain the
    underlying `ExpressionError` as a typed cause. Collection failures are
    fatal in `pipeline/phases.rs::transclusion_failure_is_fatal`; ordinary
    child read failures retain existing policy. Explicit expression handlers
    can handle ordinary callback/nested-expression failures normally, but no
    partial collection result is returned after an unhandled failure. Add
    budget exhaustion to nested fatal restoration alongside blocked
    diagnostics, checking the sticky authority before the next expression,
    stage, effect, and final success. Evidence: `nested.rs::{Outcome,
    take_blocked, fold_child}` already restore selected request-fatal causes
    that `as_markdown()` would otherwise turn into recoverable expression
    errors. Preserve warnings reported before every failure. Budget causes
    use shared `expression/iteration.rs::IterationError`, exposed by
    `ExpressionError::IterationLimit(Box<IterationError>)` and the collection
    budget variant; do not make the expression layer depend on region types.

22. **Fix diagnostic identities now.** Define codes once in
    `collection_blocks/diagnostics.rs`, and re-export shared iteration/callback
    constants from their owning expression modules. Use the following mapping:

    | Code | Meaning and severity |
    | --- | --- |
    | `dm.collection.malformed` | Invalid region/header/options; error |
    | `dm.collection.untyped_map_source` | Missing iterable declaration; error, warning under map recovery |
    | `dm.collection.untyped_filter_source` | Missing iterable declaration; error, warning under filter recovery |
    | `dm.collection.invalid_source` | Known unsuitable source or reached scalar; error |
    | `dm.expression.invalid_callback` | Callback grammar or protected parameter; error |
    | `dm.collection.pattern_mismatch` | Incompatible input/result destructuring; error |
    | `dm.collection.invalid_sort_key` | Unsupported/mixed/null ordering keys; error |
    | `dm.collection.callback_failed` | Unhandled reached callback failure, with underlying typed cause; error |
    | `dm.iteration.visit_limit` | Shared visit exhaustion; unrecoverable error |
    | `dm.iteration.expansion_limit` | Shared byte exhaustion; unrecoverable error |

    Existing expression call/parse diagnostics inside valid callbacks keep
    their existing codes; wrapper context does not invent a second underlying
    finding. Register codes in `policy.rs::COMPOSE_DIAGNOSTIC_CODES`, library
    `report_diagnostics.rs`, and DMLS `diagnostics/codes.rs`. Add library
    `with_allow_untyped_map_variables(bool)` and
    `with_allow_untyped_filter_variables(bool)`, default false, wired to the
    exact CLI recovery names `untyped-map-variables` and
    `untyped-filter-variables`. Recovered warnings identify authored source
    sites, not iterations; use `WarningIdentity`/`WarningSubject` and
    `ComposeReport::add_warning`. Runtime errors include iteration paths.
    Evidence: spec R2/R9 leaves code/API spelling to existing registries;
    `context/report.rs`, `policy.rs`, CLI `args/policy.rs`, and
    `report_diagnostics.rs` already own identity and presentation.

    Extend the library's located `expression::Diagnostic` with
    `span: Option<SourceSpan>` in authored-file bytes and
    `iteration_path: Vec<IterationFrame>`. An `IterationFrame` contains the
    source reference spelling as `collection` and a tagged item
    `position: {kind: "index", value: <zero-based u64>}` or
    `position: {kind: "key", value: <original string>}`. Preserve file/start
    line/character column in `DiagnosticSource`; the typed error retains the
    complete underlying source reference. Serialize these fields in report-json
    and DMLS diagnostic data, with null/empty defaults for unscoped existing
    diagnostics. Render paths such as `teams[2] → members[4]` from these frames.
    Update every in-crate shared `Diagnostic` constructor, parity test, and
    documented report shape together. Static findings have no iteration path.

23. **Limit downstream edits to actual consumers.** Update exhaustive matches
    in Darkmatter and DMLS; retain default trait methods so existing lookup
    adapters compile. Add compatibility coverage for Claudine's loop/sequence
    globals and frontmatter-only preparation, without implementing new
    Claudine features. Evidence: Sniff identifies Darkmatter's direct consumers;
    repository searches find external `ResolvedBinding` construction in
    `claudine/lib/src/composition/{looping/expression.rs,sequence/expr.rs}`, and
    `EvaluationSession::associate` consumers in sequence tasks/templates and
    lifecycle bindings. They construct/classify bindings rather than
    exhaustively matching the new local variant. `composition/preflight.rs`
    requests frontmatter-only operations. No external host named `local` was
    found; registration rejection remains the explicit migration behavior.

24. **No spike, loader expansion, or dependency addition.** Ruling: implement
    with existing Rust, CommonMark, JSON, schema, numeric, file, and hash
    infrastructure; no performance benchmark or prototype phase. Spec's
    architecture already settles borrowing/copy constraints, and the source
    trace settles pipeline feasibility. The Input Robustness Matrix does not
    apply: this delivery adds a Markdown/expression text grammar and numeric
    CLI arguments, and reuses existing file-format loaders unchanged. Source
    absence/null/shape tests below still prove directive semantics. Do not
    refactor YAML/JSON loaders or policy storage. No new crates are required.

25. **Share discovery with reference validation.** Add a discovery-only
    collection-site inventory in `collection_blocks/discovery.rs` and use it
    from shell preflight and `lib/src/markdown/reference/graph.rs`. It yields
    authored reference/shell sites with scope and dependency metadata, not
    concatenated repeated bodies. For local-dependent reference destinations,
    evaluate only the supported destination dependency slice; retain existing
    literal-reference validation and child traversal. Do not add normal
    `CollectionBlocks` execution to `reference/graph.rs::prepare_content`'s
    restricted compose list. That function currently invokes composition over
    TextReplacement/PageBlocks/Interpolation/ShellExpansion, so naively adding
    the operation would execute callbacks a second time before the real run.
    Carry scoped site provenance through `build_node` and
    `prepare_content_for_validation`, and preserve heading/dependency snapshots
    from the same discovery observation. Runtime-only reference/heading text
    remains unresolved during discovery; it cannot fabricate a definite broken
    reference or an executable target. This follows R7's prohibition on running
    repeated bodies during discovery and R8's supported target evaluation.

### Wave 1 — Record and scaffold

This wave is serial. It establishes shared interfaces before parallel edits.

- [x] **Record contracts**
  - Create `implementation-log.md` beside this plan with the decisions above,
    source findings, shared ownership with `2026-11-01-map-and-filter-expression`,
    and a phase/wave validation ledger. Record that no product rulings or spikes
    remain. Preserve the specification as a dated snapshot.
  - Record package scope (`darkmatter`, `darkmatter-cli`, `dmls`; compatibility
    checks for `claudine`) and the current unrelated lifecycle moves as excluded
    work. Check off plan tasks only after their stated evidence exists.
- [x] **Establish interfaces**
  - Add the shared callback/pattern, lexical identity, collection region/error,
    iteration limit/usage, and body scope-range types with the module ownership
    and names in the rules. Add registry/re-export scaffolding and compile all
    exhaustive consumers as one integration edit. Keep scaffolded paths
    non-executing until their owning tasks implement them; do not report the
    feature as shipped or remove Planned notices.
  - Keep errors boxed where they enter recursive pipeline frames. Review the
    existing comments on changed symbols alongside each implementation; update
    their behavior contracts, not surrounding unrelated prose.

Checkpoint: the new shared types compile through `darkmatter`, `darkmatter-cli`,
and `dmls`; the log records the fixed interfaces. No runtime feature tests are
claimed at this foundation checkpoint.

## Phase 2 — Implement Grammar and Lexical Analysis

### Wave 2 — Build independent foundations

Depends on Phase 1. Three owners can work concurrently: expression syntax,
binding/type scope, and Markdown container scanning. The coordinator owns common
module exports; workers edit only their listed implementation files.

- [x] **Parse callback syntax**
  - Implement both arrows, `parse_callback[_spanned]`, shallow patterns, mapped
    quote offsets, and object shorthand in `expression/{lexer,parser,ast}.rs`
    and `parse_utils.rs::Cursor`. The scanner owner handles the other
    `parse_utils` functions; integrate their patches sequentially.
  - Prove ternary/nested-call bodies, arrows inside strings, dashed identifiers,
    object field order, nested callback text rejection in ordinary value
    positions, malformed fields, aliases/rest/defaults, empty patterns,
    duplicate fields, missing arrows, surplus text, and multibyte escaped spans.
    Keep existing subtraction and condition parsing intact.
- [x] **Implement lexical frames**
  - Implement immutable frames, `ScopedLookup`, reserved `local`, local root
    classification, passive declarations, and identity-aware fact lookup in
    `expression/{binding,mod,absence}.rs`, `expression/scope/`, and
    `expression/catalog/roots.rs`. Runtime frames retain values/origins and
    parents without copying all document state.
  - Make static read inventories understand lexical definitions and free
    captures. Extend prepared validation and context-requirement discovery so
    locals are never document reads and outer `ctx`/`current`/host dependencies
    still register. Preserve existing observer and per-expression refresh hooks.
  - Test explicit/implicit alias equality, null shadowing, missing member
    shadowing, nested restoration, sibling isolation, retained-frame lifetime,
    `fm.local`, every protected root, an attempted host `local`, and existing
    host-global registration/availability behavior.
- [x] **Track Markdown containers**
  - Extend `parse_utils`' CommonMark offset scan with code regions plus complete
    container ancestry; upgrade `block_pairs` to typed matched regions.
    Preserve existing page/shell parsing through adapters over the shared pairs.
  - Exercise lists with indented continuation bodies, quote depths, nested
    list/quote combinations, literal quoted page examples, active quoted shell
    blocks, mismatched/crossed closers, shell-body collection rejection, fences,
    indented code, LF/CRLF, and no-final-newline closers.

### Wave 3 — Integrate passive regions

Depends on Wave 2. The two tasks run concurrently in separate collection parser
and expression type-projection files; integrate their public boundary together.

- [x] **Parse collection regions**
  - Implement `collection_blocks/{parser,types}.rs` over shared block pairs and
    the mapped directive cursor. Add Map/EndMap/Filter/EndFilter to
    `directives_api.rs::{DirectiveKind, KEYWORDS}` and expose the optional
    positional callback with its own span on `ParsedDirective`.
  - Implement all header validation from Rule 3 in enabled authored regions,
    including false/empty ones. Preserve lenient best-effort editor scanning
    while composition returns strict located structural/header errors.
  - Update public block scans, shell-block command scans, and existing parser
    adapters together so no consumer retains a competing region stack.
- [x] **Project collection types**
  - Implement typed `IterableSource` entries with original index/key, captured
    source ownership, per-item borrowed views, and declared entry types. Capture
    a source once; retain per-item values only when later body frames need them.
    Allocate no rejected-item list; sorting stores selected entries and one key.
  - Implement iterable/tuple/object/union/named/open projection, pattern
    projection, computed object/array result shapes, local alias remapping, and
    selected-input facts through the shared type engine. Check the source on
    the true side of its own gate plus enclosing guards.
  - Change `functions/paths.rs`'s `file_exists` registration from true-implies
    `string` to true-implies `file`, matching `is_file_type` in
    `functions/predicates.rs`. Update `fs.rs`'s string-only doc comment and
    predicate tests. Do not transfer facts to independent read-side calls.

### Wave 4 — Connect passive consumers

Depends on Wave 3; serial because the scope inventory is a shared authority.

- [x] **Unify site analysis**
  - Implement `BodyScopes`/`analyze_collection_blocks` and make `BodyGuards` its
    conditional view. Extend `call_precheck.rs::check_body`,
    `directive_targets.rs`, expression absence/lint/prepared validation, and
    context-requirement collection to discover every enabled authored surface
    under its exact gate, callback, or body scope. Frontmatter remains outside
    body locals. Static errors occur once per site, before any effects.
  - Check source declaration suitability, protected names, pattern input/result
    suitability, provably unsupported sort keys, and calls in skipped bodies.
    Unknown result types continue through existing uncertain-call diagnostics.
    Recovery warnings enter the blocking policy before subsequent evaluation.
  - Add public analysis tests with handlers that fail on invocation, proving
    analysis neither evaluates callbacks nor resolves files/network services.
    Include a source narrowed by its own gate, independent option parameter
    names, map result fact isolation, and host globals.

Checkpoint: syntax, scope, and passive-analysis suites pass; existing page/shell
scans and type-scope ownership guards remain green. The shared grammar corpus
accepts shipped artifacts. Callback/static typing is ready before runtime work.

## Phase 3 — Execute Scoped Bodies and Enforce Limits

### Wave 5 — Implement runtime primitives

Depends on Phase 2. Budget and provenance owners run concurrently in their
respective modules; the coordinator integrates pipeline/context handoffs.

- [x] **Enforce request budgets**
  - Implement `IterationLimits`, shared `IterationBudget`, discovery mode,
    checked visit/byte reservations, sticky typed exhaustion, and per-node
    `ExpansionUsage` ledgers. Add positive options/builders/getters and standalone
    session visit controls. Thread one authority through pipeline construction,
    `clone_for_child`, `ResolutionContext`, scoped lookups, and nested composition.
  - Extend exhaustive options identity classification for limits and recovery
    controls; no counters or lexical values are part of persistent state.
    Implement request-fatal budget restoration through `as_markdown` and its
    `||` handlers using the existing typed fatal restoration channel.
  - Test exactly-full reservations, overflow, exhaustion before excessive
    append, shared children, nested calls, empty bodies consuming visits, and
    preflight remaining uncharged. Use small configured limits for boundary
    tests; retain one defaults parity assertion.
- [x] **Retain body scopes**
  - Extend `BodyProvenance` with copy/splice projection and scope/iteration/
    expansion-owner ranges. Implement `ExpansionWriter`, preserving exact
    authored substrings and inserted data when ranges repeat or are rewritten.
  - Teach interpolation/target rewriting to choose a lookup at each site,
    rather than concatenate bodies and rediscover local meaning. Carry all
    provenance maps across edits; include copied source-span and fail-closed
    data-range tests through composed public results.

### Wave 6 — Integrate structural execution

Depends on Wave 5; tasks are sequential because they share the pipeline driver.

- [x] **Execute collection regions**
  - Implement `collection_blocks/engine.rs` as one executor parameterized by
    Map/Filter, using the iterator and scope model already built. Integrate
    page conditions in the same recursive structural traversal; preserve shell
    blocks as authored templates for their later stage.
  - Implement gate/source timing, callback defaults, independent option frames,
    fused unsorted selection/prefix/map, immediate per-selected-item key
    computation, stable sorting, prefix exclusion, result rebinding, and
    recursive per-iteration preparation. Report original indices/keys and
    nested iteration paths after sorting.
  - Count effects using controlled handlers to prove short-circuiting and the
    exact schedule, including callbacks before ordinary body effects, failure
    before the first shell stage, one key evaluation, and no callback after
    the unsorted stopping item. Do not parallelize callbacks.
- [x] **Wire staged composition**
  - Add grouped structural dispatch to `pipeline/{operations,mod,phases}.rs`
    and perf/report metadata. Replace the page-only inline execution handoff
    with the shared traversal while retaining public page-block adapters.
  - Make body shell expansion, shell blocks, conditions, directive targets,
    and transclusion preparation read retained scopes and original origins.
    Replace the phase-wide assumption only for scoped site data; document
    frontmatter/schema/state still prepare once.
  - Honor every operation subset and frontmatter-only projection. Keep
    replacement document-scoped, then structural preparation, interpolation,
    existing shell/link stages, concurrent children, existing cleanup/heading
    normalization, and root-only link normalization.
  - Charge materialized bytes at each collection copy/insertion boundary with
    the writer. Preserve typed errors and earlier warnings, extend fatal
    collection classification, and prevent successful partial output.

Checkpoint: in-process composition proves both directive semantics, nested
conditional/collection ordering, scope survival through later stages, typed
failure propagation, and budget boundaries. Shell integration tests here use
explicit preapproved fixtures; full condition-blind discovery follows next.

## Phase 4 — Integrate Approval, Child Discovery, and Caches

### Wave 7 — Extend independent integration owners

Depends on Phase 3. Two owners run concurrently: shell templates, and
graph/cache integration. The integration owner completes **Retain scoped
graphs** before **Preserve cache semantics**, since both touch the transclusion
engine. The coordinator owns `preflight/collect.rs` in Wave 8, avoiding parallel
edits to that shared collector.

- [x] **Type local templates**
  - Extend shell template candidate timing and `project_hole` with local
    subjects, type definitions, constraints, and shared guard refinements.
    Classify local-dependent holes through the shared static read inventory;
    explicit and implicit reads agree even inside larger expressions.
  - Preserve template-only requirement without converting locals into host
    globals. Test broad numeric holes with numeric validators, unknown scalar
    fallback, null/container rejection, unsupported command grammar, unrelated
    missing approval-time placeholders, and actual argv boundaries.
  - Preserve whitelist/blacklist and membership checks before request-wide
    shell result-cache reuse, plus both existing no-cache spellings and every
    logical command in a repeated shell block.
- [x] **Retain scoped graphs**
  - Add authored-site/iteration/parameter/origin keys to preflight graph edges,
    non-Markdown targets, and runtime prepared transclusions. Implement strict
    scoped membership and child-subgraph selection; retire raw-target and
    source-only selection for repeated sites.
  - Derive child requests through existing `markdown_child_options` and
    file-resolution context machinery. Reset lexical frames at child entry;
    preserve explicitly passed local values as data and their source origin.
  - Carry collection container prefixes into prepared transclusions and apply
    them at the final splice, reusing existing prefix/indent helpers from shell
    blocks and TOC insertion. `transclusion/engine.rs::
    render_markdown_transclusion` currently documents missing directive indent
    capture; collection list/quote output must capture it rather than inherit
    that defect. Cover multiline child output inside both containers.
  - Test equal target bytes with different parameters, recursive child
    parameters, distinct file-opening contexts, root-only overrides, child
    diagnostics, and rejection of a runtime target absent from its scoped graph.
- [x] **Preserve cache semantics**
  - Add semantic object-order fingerprints in `cache/hashing.rs` and the
    options/graph identity encoders without altering unordered-set semantics.
    Include explicit scoped values and origins where evaluated work is cached.
  - Extend `cache/runtime.rs::ComposeResult` with subtree usage and an explicit
    compute-versus-reuse outcome from `get_or_compute_compose`. Reserve replay
    usage on reuse only; charge parent insertions separately. Preserve typed
    collection/budget failures across single-flight failures instead of
    downgrading them to a formatted `Transform` error for waiting consumers.
  - Test repeated child/shell cache hits, simultaneous child reuse, nesting
    charges, rejected-item visits, same bytes under different local values,
    different object insertion orders, and no charges for unrelated stage
    passes. Keep semantic results memory-only and existing cache guards green.

### Wave 8 — Extend condition-blind discovery

Depends on Wave 7; this is one collector owner followed by serial integration.

- [x] **Discover scoped children**
  - Extend `preflight/collect.rs::{collect_document, collect_recursive,
    DiscoverySiteContext}` with `BodyScopes`, typed collection enumeration,
    target/explicit-parameter dependency slicing, and transitive unavailable
    dependency records. Evaluate supported read-side target-producing mapping
    expressions in discovery mode; do not evaluate irrelevant mappings or
    selection/sort/prefix/gate callbacks merely to discover children.
  - Scan shell templates passively for all authored collection bodies even
    with no candidate items. Enumerate all available candidate items for child
    targets before `when`, positional predicates, extra filters, and stopping.
    Recurse using explicit child parameters so skipped candidates' commands
    remain subject to the same approval rules.
  - Preserve existing handling of a supported target's null/missing read and
    ordinary child read errors; reject unavailable execution dependencies at
    the authored target before any shell work. Discovery retains graph
    metadata, not evaluated body/callback result caches.
- [x] **Integrate approval membership**
  - Wire local-aware shell candidates, graph keys, budget authority, strict
    runtime target matching, and preflight warnings through
    `preflight/{approval,lifecycle,mod}.rs`, the in-pipeline preapproval gate,
    and `nested.rs`. Keep every analysis-only surface passive.
  - Implement Rule 25's shared discovery inventory and update
    `reference/graph.rs::{build_node, prepare_content,
    prepare_content_for_validation}` and its validation caller to use it
    without enabling shell or repeated-body execution;
    preserve preflight graph identity checks and failed-walk warning retention.
  - Cover `find_files()`-produced file arrays, target-producing read callbacks,
    false outer conditions, empty collections, excluded/stopped items, indirect
    pending shell dependencies, execution-dependent globals, recursion,
    repeated targets, and changed runtime targets. Use controlled temp files,
    effect counters, explicit whitelist fixtures, and the native argv recorder.

Checkpoint: approval remains a condition-blind superset of execution. No shell
command runs during discovery, passive analysis invokes no callback, new
runtime targets receive no late approval, and cache hits preserve budgets and
scoped meaning. Existing transclusion and shell-template suites pass.

## Phase 5 — Deliver CLI, Editor, and Acceptance Coverage

### Wave 9 — Implement public surfaces

Depends on Phase 4. CLI, DMLS, and library acceptance tests can proceed
concurrently; each owner controls its own crate's production files/tests.

- [x] **Expose compose controls**
  - Add `Option<NonZeroU64>` flags to `cli/src/args/command.rs::Command::Compose`,
    wire them through `commands/mod.rs` to `commands/compose.rs::run_compose`,
    and apply library options before reference validation, lifecycle/preflight,
    or composition starts. Keep defaults solely in the library.
  - Register recoveries and `collection-blocks` in `args/policy.rs` with help,
    completions, conflicts, `all` behavior, blocking codes, and specific policy
    precedence matching existing entries. No unlimited or budget recovery flag.
  - Extend `report_diagnostics`/CLI reporting so structured errors retain
    spans, causes, and iteration paths. Reuse existing terminal components and
    report sinks; report-json emits exactly one object with no stray stdout.
  - Add CLI wiring tests: positive custom limits, omitted defaults, zero,
    negative/overflowing flags, each recovery, blocked recovery before effects,
    normal Markdown output, typed failure location, and report-json parity.
- [x] **Project editor scopes**
  - Extend DMLS `overlay/{directives,expressions}.rs`,
    `providers/dsl.rs::{with_site_scope, interpolation_completions,
    expression_diagnostics}`, `diagnostics/call_check.rs`, and code exports to
    consume the shared region/scope analysis. `providers/{completion,hover}.rs`
    remain routers; do not implement a second type resolver there.
  - Offer source/header/option completions and cursor-visible locals,
    including `local.*`, destructured fields, mapped result members, and outer
    captures. Exclude callback siblings, later/expired locals, and shadowed
    outer bindings. Hover uses the same declared/result/refined types.
  - Extend region folds, directive keyword inventory, structural diagnostics,
    target analysis, and semantic token classification for the four new
    markers. Project escaped/multibyte callback spans into LSP ranges.
  - Prove library/DMLS static code/location agreement and zero callback I/O
    with analysis tripwires. Extend `dmls/tests/l1/call_check_agreement.rs`
    rather than maintaining another expected type/code registry.
- [x] **Prove directive semantics**
  - Add `lib/tests/l1/collection_blocks.rs` and focused submodules, registered
    in the existing consolidated L1 binary. Build a shared case corpus under
    `lib/tests/fixtures/collection_blocks/` with both kinds parameterized where
    behavior is shared and explicit map/filter differences.
  - Grammar coverage: both arrows in every slot, defaults, strict option
    combinations/order, quoted arrows, nested calls/ternaries, malformed
    patterns, escaped source spans, every invalid closer/container, LF/CRLF,
    and no final newline.
  - Type/value coverage: declared arrays/tuples/dictionaries, heterogeneous and
    open/named types, unions with/without sufficient guards, nullable/optional
    sources, missing declarations, unknown recovery versus known scalars,
    present null fields/elements versus missing pattern fields, map result
    incompatibility, and filtered original types/facts.
  - Scope/schedule coverage: protected roots/globals, explicit/implicit local
    equivalence, `fm` escape, null shadowing, outer captures, independent option
    names, restoration/siblings, nested map/filter/conditionals, source capture,
    false gates, selection short-circuit, prefix exclusion, stable asc/desc
    ties, one key evaluation, locale-independent strings, and invalid key
    families including a single selected item.
  - Pipeline/failure coverage: exact whitespace with cleanup disabled for
    byte-exact assertions, normal cleanup separately, frontmatter once,
    callback/body stage schedule, inserted fake interpolation/closers/fences
    remaining data, repeated authored shell blocks, operation subsets,
    frontmatter-only requests, lenient child fatal collections, ordinary child
    read policy, explicit nested expression handlers, fatal budgets through
    fallback, prior warning retention, and sorted original iteration paths.
  - Budget coverage: defaults, small positive limits, rejected visits,
    boundary visits, empty bodies, shared nested/child/session allowances,
    concurrent children, exactly-full success, preappend failure, UTF-8 bytes,
    nested copied-output charges, cached insertions, usage replay, no ordinary
    pass/preflight charges, and expression values remaining uncharged bytes.

### Wave 10 — Verify cross-surface contracts

Depends on Wave 9. One integration owner connects the corpus to process and
compatibility tests; this wave is serial.

- [x] **Prove invocation wiring**
  - Add a small `cli/tests/l1/collection_blocks.rs` suite using
    `CliProcessFixture`. Run a representative successful map and filter through
    actual `md compose`, plus typed invalid input, preapproval denial in a
    skipped candidate, and budget exhaustion. Assert original file/line/column,
    structured iteration path, failure exit, no successful document, and no
    unexpected recorder invocation. Keep the shape cross product in-process.
  - Reuse the existing thin `darkmatter-argv-recorder` binary backed by
    `test_toolkit::argv_recorder`; copy it to fixture-owned PATH and explicitly
    configure whitelist rules. Prove spaces, operators, redirection-looking
    strings, repeated commands, no-cache controls, and null arguments without
    POSIX-only tools, network services, or interactive approval.
  - Add a passive shipped-artifact corpus test reading the actual two directive
    docs and iterables page; parse their directive/callback examples through
    public scanners. Execute positive examples from stable fixtures separately.
    Declare reads with `include_str!` or `manifest_dir!().join("...")` in the
    tested file, using the runtime harness macros for fixture paths/binaries.
    Internal prompts remain opt-in inputs.
- [x] **Verify host compatibility**
  - Extend existing scope-site conformance, shared diagnostics, entry-point
    parity, context/path construction, and semantic-cache guards with collection
    sites rather than broad exemptions. Add focused Claudine L1 cases using
    its real loop/sequence `BindingView`: protected global rejection, no local
    free-read leakage, and frontmatter-only preparation with dormant collection
    callbacks.
  - Keep existing `EvaluationLookup` implementations source compatible using
    the default new hooks. Verify `claudine` via its scoped package recipe;
    compile other unchanged direct consumers through the normal affected-plan
    check cells. This feature does not rename host globals or alter lifecycle
    function catalogs.

Checkpoint: every specification acceptance row has named tests in the log;
library/CLI/report-json/DMLS agree on code, authored location, and scoped type.
The executable corpus uses the normal request, approval, and composition paths.

## Phase 6 — Publish Current Documentation and Validate

### Wave 11 — Synchronize documentation

Depends on Phase 5. The documentation owner handles public docs and README;
another owner can update skill topics and the test-evidence ledger in parallel.
Avoid parallel full test/lint jobs while these source-input docs are changing.

- [x] **Document shipped behavior**
  - Review `docs/inline/map.md` and `docs/inline/filter.md` in full. Add
    `kind: canonical-documentation` frontmatter and remove Planned notices only
    now that behavior is complete. Cover grammar/defaults, dotted source forms,
    destructuring, result versus original bindings, guards, ordering, scope,
    container rules, discovery/approval, failures, both limits/builders/flags,
    recoveries, caching, and concise executable examples.
  - Update `docs/iterables.md`, `docs/darkmatter-compose-pipeline.md`,
    `docs/topics/{darkmatter-expressions,expression-engine,compose-requests,dmls}.md`,
    `docs/inline/{page-blocks,shell-expansion,shell-blocks}.md`,
    `docs/cli/{policy,report-json}.md`, and README overview links to reflect the
    final APIs. Preserve their overview/reference boundaries and US English.
    Add a Mermaid flow to explain structural preparation versus later body
    stages and one compact example for each novel rule. Topic pages never
    name this feature or another lifecycle directory.
- [x] **Update implementation evidence**
  - Update `.claude/skills/darkmatter/{SKILL,compose,expressions,testing,dmls}.md`
    with the shared modules, retained scopes, budgets, discovery boundary, and
    tests. Preserve the requirement to consult biscuit-file for resolution and
    biscuit-hash for fingerprints. Dependency docs remain unchanged because
    this plan adds no crates.
  - Complete the acceptance-to-test ledger, record actual spec departures with
    their technical reasons, and fix current docs to describe final behavior.
    The dated specification stays unchanged. Record any newly learned OS fact
    in the OS skill in the same implementation change.

### Wave 12 — Run final verification

Depends on Wave 11. Run these tasks sequentially on the integrated tree;
individual feature-slice tests from earlier phases are not a substitute.

- [x] **Run scoped gates**
  - From `darkmatter/`, run `just test`, `just lint`, `just doctest`, and
    `just build` sequentially, fixing failures and rerunning only the affected
    checks after each fix. All use the area's existing nextest/test-feature
    recipes; never run `cargo test` or `cargo fmt`.
  - From the repository root, run the opt-in `just test-prompts` once after
    final grammar/doc edits because the expression skill requires that grammar
    corpus check. It remains outside mandatory CI inputs. Run `just test
    claudine` for the shared binding/type-scope compatibility changes.
    Inspect the grammar target's test results explicitly: this opt-in recipe
    exits zero even after a failed subcommand. Grammar failures must be fixed;
    unrelated draft-prompt schema findings remain informational and are logged.
  - Audit relevant source for local reads falling through to document lookup,
    callback AST splitting, name-based predicate narrowing, raw canonicalize,
    ambient context recapture, ignored parse failures, and unchecked expansion
    appends. Treat found defects as implementation work and close them before
    completion; do not file unscheduled follow-ups.
- [ ] **Validate supported environments**
  - Read the normal evidence plan with `just ci-local --plan`; preserve its
    canonical per-package/per-environment/per-gate scope. Reuse qualifying
    evidence and run the missing scoped L1 coverage on Linux, native Windows,
    and WSL2 through `just cross-check <package> --os <environment>` for
    `darkmatter`, `darkmatter-cli`, and `dmls`. The planning host declares
    `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL`; local execution supplies macOS.
    Do not hardcode their current SSH aliases. Use the existing recipe's
    unattended transport and archive mode, with BatchMode authentication.
  - Preserve the recipe's distinction between uncommitted-tree test evidence
    and published exact-HEAD receipts; record the actual tested tree and do
    not claim receipt reuse without a qualifying receipt. Run no full-workspace
    suite, speculative CI matrix, real terminal/browser suite, or window-focus
    operation. Keep the WSL archive relocation check distinct from native
    Windows behavior. Inspect PASS counts and structured failures, not just
    command exits; fix environment-specific defects in this worktree and repeat
    affected coverage.
  - A transport/provisioning failure produces an explicit failed/unproven
    environment record and a failing validation status, never a false completion
    claim. Continue all other independent checks without a human gate. Do not
    modify CI scheduling or create an execution ban to hide missing evidence.
- [x] **Close implementation ledger**
  - Verify every task and acceptance case against its artifact/test evidence;
    check off only achieved items. Re-read both directive documents for
    canonical-documentation frontmatter, stale planned claims, inconsistent
    examples, and their exact control defaults.
  - Record changed public APIs, files, tests and environments, source-comment
    drift fixes, cache/budget behavior, and any remaining external validation
    failure. End with implementation complete and ready for review only when
    all required checks passed; otherwise report incomplete validation and the
    concrete failure without seeking a ruling or changing scope. Keep the
    feature active for the author's later review cycle.

Final checkpoint: all six phases execute without an undecided design choice or
human gate, the implementation and current docs meet the Definition of Done,
and validation evidence describes the final integrated tree.