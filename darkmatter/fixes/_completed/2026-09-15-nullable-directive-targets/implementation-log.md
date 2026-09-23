---
phase: 5
completed_phase: "5"
implemented: true
source_code:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/lib/tests/directive_target_analysis.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
documentation:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/simplified-schemas.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/directives_api.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/cli/tests/compose_transclusion.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/spec.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/tests/directive_target_analysis.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/dmls/src/diagnostics/codes.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_4:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/simplified-schemas.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md
  - darkmatter/fixes/2026-09-15-nullable-directive-targets/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
human_review: false
human_review_items: []
message_to_agent: >-
  Phase 5 is implementation-complete and ready for review. There is no subsequent
  implementation phase; do not move this fix to `_completed` until the author closes review.
---

# Implementation Log — Nullable Directive Targets

## Phase 1

- Initialized Phase 1 tracking before implementation changes.
- Baseline revision: `b1bef6839814378420b54b0be6c8bbda98c8ee61` on
  `feat/dark-fixes`.
- Untouched local baseline gates:
  - `just build`: failed before compiling Darkmatter because shared `target/`
    artifacts were read-only; build-script linking also failed because the
    host has not accepted the Xcode license.
  - `just test`: failed during dependency compilation because the same shared
    artifacts were read-only.
  - `just lint`: failed during dependency checking because the same shared
    artifacts were read-only.
  - These are host-state failures, not test or lint failures in the package.
- Authoritative minimal reproduction, run with the checkout's existing
  `target/debug/md` against the exact fixture from the specification: exit 1,
  `TransclusionError: directive parse failed`, `Expected value, found end of
  directive`, and an excerpt pointing to frontmatter line 3 (`log: file`)
  instead of the `::file {{log}}` directive at file line 7.
- Live-prompt evidence was omitted. The historical nullable working-tree state
  cannot be reconstructed honestly from a clean revision, so no diff or result
  is attributed to `6f5b06251`.
- Refreshed the GitNexus index with `just gitnexus`; it matches all 7,333
  covered files at `b1bef683`. Upstream impact results:
  - transclusion `parse_directives`: HIGH, 31 affected symbols, 20 direct
    callers, and the `resolve_prepared_transclusion`, `run_compose_pipeline`,
    and `run_transclusion_phase` process families;
  - interpolation `interpolate_text`: HIGH, 42 affected, 28 direct callers;
  - `scan_darkmatter_directives`: HIGH, 58 affected, 16 direct callers across
    Compose, DMLS Providers/Overlay, and Graph;
  - preflight `collect_recursive`: MEDIUM, 49 affected, one direct caller;
  - DMLS `transclusion_diagnostics`: LOW, three affected, one direct caller.
  The HIGH-risk parser/scanner surfaces remain a review gate: Phase 1 changes
  tests only and pins every intended call path before later production edits.
- Span spike:
  - `nullable_target_raw_span_contract` covers bare, quoted-empty,
    whole-value, and mixed targets for `::file`, `::code`, and `::url`, with
    CRLF input and a fenced-code negative control.
  - The promoted contract fails on the current code because quoted-empty
    targets are collapsed to `None`; this is the exact parser-API gap Phase 2
    must close. Whole-value and mixed raw spans are already preserved.
  - `directive_lines_can_be_removed_end_to_start_without_invalidating_spans`
    proves full lines can be removed in reverse document order while all
    earlier scanner spans remain valid.
  - The shared callable surface for both terminal interpolation and preflight
    is `markdown::compose::directives_api`; no second directive recognizer is
    needed.
- Typed-evaluation spike:
  - `whole_value_evaluation_preserves_nullable_target_types` proves the
    existing `interpolate_value` path retains JSON null, `""`, and a concrete
    string before stringification.
  - `authored_target_and_pending_shell_data_identify_the_same_dependency`
    proves the authored target span's expression root can be matched to the
  existing pending-shell `(key, literal)` data. The security guard can reject
  that target before any absence rewrite; no dependency-model redesign is
  needed.
- Requirement-to-test mapping and fail-first evidence:
  - Runtime condition split: `runtime_page_blocks_suppress_guarded_null_target`
    passes today, while
    `condition_blind_preflight_skips_null_edge_but_keeps_concrete_sibling`
    fails when promoted with `Expected value, found end of directive`; its
    concrete false-condition sibling proves approval discovery remains
    condition-blind and must still discover `echo sibling-command`.
  - Unguarded evaluated absence:
    `unguarded_null_target_is_skipped_with_one_warning` and
    `unguarded_empty_string_target_keeps_its_typed_reason` both fail when
    promoted because the current parser aborts before a typed warning exists.
  - Authored boundaries:
    `authored_empty_targets_remain_errors_and_mixed_targets_remain_paths`
    passes for bare, quoted-empty, and mixed targets;
    `malformed_directive_in_false_block_has_distinct_runtime_and_preflight_outcomes`
    passes and locks the runtime/preflight difference.
  - Security boundary:
    `pending_shell_target_is_rejected_before_child_command_approval` and the
    CLI `compose_rejects_pending_target_before_child_command_can_execute`
    fail when promoted because the current code reports a generic directive
    parse error instead of a dynamic-shape rejection. Both prove the child
    command is not approved or executed; the CLI case uses
    `CliProcessFixture` and a fixture-owned sentinel.
  - Real CLI incident:
    `compose_guarded_nullable_target_through_real_preflight_lifecycle` uses the
    exact minimal reproduction and fails when promoted with the incorrect
    frontmatter line-3 excerpt.
  - Coordinates: `preflight_parse_errors_use_file_relative_lines` covers
    three distinct frontmatter lengths and fails when promoted at the first
    row (`line == 3`, expected file line 6).
  - DMLS current defect:
    `interpolated_transclusion_target_currently_reports_broken_path` executes
    the in-memory LSP path and observes one false warning ranged on `{{log}}`.
    `nullable_transclusion_diagnostic_matrix` reserves optional, guarded,
    outer-nested guard, required, default metadata, and literal-missing cases;
    promoted execution sees six `broken_path` warnings rather than only the
    concrete missing path, and no nullable warning.
  - Passive corpus:
    `shipped_transclusion_fixture_targets_are_scanned_passively` reads all 13
    shipped performance Markdown fixtures and checks all 41 transclusion
    targets through `scan_darkmatter_directives`, with no compose, resolution,
    process, or network path.
- Targeted normal-mode results: 13/13 Darkmatter library tests, 2/2 CLI tests,
  and 2/2 DMLS tests passed. With `BISCUIT_PROMOTE_PENDING=1`, all six library
  pending contracts, both CLI contracts, and the DMLS matrix failed for the
  expected pre-fix reasons recorded above.
- Required area gates:
  - `just test`: 7,837 tests passed, seven intentionally skipped, zero failed.
  - `just lint`: passed for `darkmatter`, `darkmatter-cli`, `dmls`,
    `zed-dmls-cli`, and the `zed-dmls` `wasm32-wasip2` compile check.
  - Both commands used an isolated target directory and Homebrew clang with
    an explicit macOS SDK path to avoid the host's read-only shared artifacts
    and unaccepted Xcode license. No formatter command was run.
- Cross-platform review used the `os` skill. The code added in Phase 1 is test
  evidence only and introduces no target-specific production behavior. Local
  remote checks could not run: Linux was occupied by another worktree's lock,
  native Windows had no free build-drive space, and WSL reset the SSH
  connection. These are environment limitations, not product failures.
- Final GitNexus `detect-changes --scope all` reported eight changed indexed
  files, 28 symbols, no affected execution processes, and LOW risk. The
  separately present unrelated working-tree changes were left untouched.

## Phase 2

- Initialized Phase 2 tracking before implementation changes. The specification already records
  `implemented: true` and `implemented_by: codex/default` as requested.
- Pre-implementation requirement-to-test mapping:
  - The shared target model and raw authored/evaluated distinction are covered by
    `nullable_target_raw_span_contract`,
    `whole_value_evaluation_preserves_nullable_target_types`, and
    `authored_empty_targets_remain_errors_and_mixed_targets_remain_paths`.
  - Runtime null/empty skipping, typed warning reasons, guard suppression, and unchanged malformed
    syntax behavior are covered by `unguarded_null_target_is_skipped_with_one_warning`,
    `unguarded_empty_string_target_keeps_its_typed_reason`,
    `runtime_page_blocks_suppress_guarded_null_target`, and
    `malformed_directive_in_false_block_has_distinct_runtime_and_preflight_outcomes`.
  - Condition-blind preflight edge omission plus concrete sibling discovery is covered by
    `condition_blind_preflight_skips_null_edge_but_keeps_concrete_sibling`.
  - Pending-shell fail-closed behavior and downstream non-execution are covered at library and real
    CLI boundaries by `pending_shell_target_is_rejected_before_child_command_approval` and
    `compose_rejects_pending_target_before_child_command_can_execute`.
  - The normal CLI lifecycle is covered by
    `compose_guarded_nullable_target_through_real_preflight_lifecycle`, using the exact minimal
    fixture from the specification through `CliProcessFixture`.
  - Full-file line coordinates across three frontmatter lengths are covered by
    `preflight_parse_errors_use_file_relative_lines`, including an assertion that the selected
    source line contains the malformed directive.
  - DMLS interpolated-path suppression is covered through the real LSP session by the existing
    executed defect test, which will be inverted to require no `dm.transclusion.broken_path`;
    `nullable_transclusion_diagnostic_matrix` remains pending for Phase 3 except for its shared
    broken-path expectation.
  - The passive shipped-artifact corpus remains
    `shipped_transclusion_fixture_targets_are_scanned_passively`, covering all 13 shipped Markdown
    fixtures and their 41 transclusion targets without effects.
  - `compose_shipped_transclusion_fixture_through_normal_cli_path` composes the repository's real
    shipped schema/transclusion fixture and child through `md compose`, covering the effectful
    normal invocation path without reconstructing the artifact.
- Pre-edit GitNexus impact: `scan_darkmatter_directives` is HIGH risk (58 affected symbols, 16
  direct callers across Compose, DMLS Providers/Overlay, and Graph); `collect_recursive` is MEDIUM
  risk (49 affected symbols); `transclusion_diagnostics` is LOW risk (three affected symbols).
  `inline::interpolation::run_stage` returned UNKNOWN because module-qualified calls are not linked
  by the index; text search confirmed its pipeline call in `pipeline/phases.rs`.
- Implemented `directive_targets`, which retains directive kind and source span while distinguishing
  concrete, null, and empty-string results. The shared rewrite runs before ordinary body
  interpolation, rewrites concrete targets, removes absent directive lines from end to start, and
  emits one file-relative typed warning for an unguarded absence.
- Preserved quoted-empty target spans in the passive directive scanner so authored `::file ""`
  remains distinguishable from an evaluated empty string and continues through the established
  transclusion syntax error path.
- Wired the rewrite through the existing interpolation stage. Ordinary composition remains
  condition-aware because page blocks run first; condition-blind preflight uses the same stage and
  therefore omits only evaluated-absent graph edges while retaining concrete sibling discovery.
- Added authored-expression dependency detection for transclusion targets before preflight's
  interpolation rewrite, plus a post-rewrite target-span check for directives introduced by earlier
  stages. Pending frontmatter-shell values now return the existing `DynamicCommandShape` error and
  cannot reveal a child command after approval.
- Added an offset-aware transclusion-parser entry point while preserving the zero-offset public API.
  Only preflight's full-file source-context call supplies `frontmatter_line_count()`, so body-only
  callers remain unchanged and the offset is applied exactly once.
- DMLS now suppresses `dm.transclusion.broken_path` whenever a transclusion target contains a real
  interpolation span. Concrete missing paths retain the existing warning.
- Promoted all Phase 2 pending contracts. The Phase 3 nullable-diagnostic matrix remains pending;
  its shared broken-path expectation is already satisfied by the executed Phase 2 LSP regression.
- Targeted verification: the passive scanner corpus/span tests pass; the null and empty-string
  runtime warning tests pass; preflight sibling discovery, authored-boundary, malformed false-block,
  three-length coordinate, and pending-target tests pass; all ten `compose_transclusion` CLI tests
  pass; and the real LSP interpolated-target test passes. One assertion was corrected to distinguish
  the phrase `evaluated to null` rather than searching for `null` inside the required word
  `nullable`.
- Final area gates, using the isolated writable target, Homebrew clang, and the Command Line Tools
  SDK: `just build` passed for `darkmatter`, `darkmatter-cli`, `dmls`, and `zed-dmls-cli`; `just
  test` passed all 7,838 executed tests with seven intentional skips; and `just lint` passed all four
  packages plus the `zed-dmls` `wasm32-wasip2` check. No formatter command was run.
- An earlier broad test attempt exposed an unrelated intermittent property failure in
  `escape::accepted_destinations_are_always_inert` for the generated input `&#0;`. Its generated
  regression artifact was removed, no out-of-scope source was changed, and the required clean full
  rerun passed all 7,838 tests.
- Phase 2 introduces no OS-specific branches, path comparison, or platform APIs; remote cross-OS
  execution was therefore not needed for this phase's pure parsing, evaluation, and diagnostic
  changes.
- Final GitNexus `detect-changes --scope all` reported 14 changed indexed files, 43 symbols, no
  affected execution processes, and LOW risk. The separately present unrelated working-tree changes
  were left untouched.

## Phase 3

- Initialized Phase 3 tracking. The specification already records `implemented: true` and
  `implemented_by: codex/default` as requested.
- Pre-implementation requirement-to-test mapping:
  - Schema/frontmatter nullability is covered by a focused classifier matrix for optional `file`,
    optional `string(default(...))`, required `file`, missing values, explicit null, concrete
    native and quoted YAML scalars, `doc.*`, raw JSON Schema, baseline/trigger origins, root unions,
    nested paths, complex expressions, and missing schema authority.
  - Cataloged `ctx.*` behavior is covered by required, optional, and unknown descriptor cases using
    the shipped descriptor catalog; no currently shipped `ctx.*` descriptor carries a default.
  - Guard narrowing is covered by an evaluator-backed truth table for `file_exists(x)`, bare `x`,
    `!!x`, both supported inequality forms and their false outcomes, parentheses, `a && b`, and
    negative controls for `||`, single `!`, equality, and unsupported expressions.
  - Integrated passive target analysis is covered across `::file`, `::code`, and `::url`, direct
    and outer-nested guards, unguarded nullable targets, required/concretely bound targets, mixed
    interpolation and unknown schema/expression cases. Assertions include
    directive kind, expression root, target span, nullability, and accumulated narrowing.
  - Passive behavior is covered by asserting the source/frontmatter inputs remain byte-for-byte
    unchanged and by analyzing missing/remote-looking targets and shell-looking values without
    filesystem resolution, network access, expression execution, or shell execution.
  - Phase 3 changes no shipped parser/schema/template/configuration artifact and persists no value,
    so it adds neither a new shipped-artifact corpus nor a read/write/read persistence test. The
    existing passive directive corpus and real `md compose` shipped-fixture test continue to cover
    the unchanged authored-directive path.
- Pre-edit GitNexus impact was bound to this worktree. The Phase 2
  `rewrite_directive_targets` symbol was not yet indexed and returned `UNKNOWN`; text search
  confirmed its only production caller is the body interpolation stage. The reused
  `scan_darkmatter_directives` and `scan_darkmatter_blocks` surfaces are HIGH risk (58 and 19
  affected symbols respectively), while the `EffectiveSchema` struct is LOW risk after
  disambiguation.
  Phase 3 changes neither scanner/parser; the implementation adds a passive consumer of the shared
  directive and block scanners and covers all three analyzed directive kinds plus nested blocks.
- Fail-first evidence: the new `directive_target_analysis` integration target failed to compile
  before implementation because `TargetNullability`, `classify_target_nullability`,
  `narrowed_expression_paths`, and `analyze_directive_targets` did not exist.
- Added the passive public analysis surface in `compose::directive_targets`:
  - `ExpressionPath` normalizes `doc.x` and bare `x` to the same document path while retaining
    cataloged `ctx.*` paths.
  - `classify_target_nullability` uses only supplied `EffectiveSchema`, property origins, static
    frontmatter, and the context descriptor catalog. It treats document schema defaults as
    metadata, honors `required`, accepts document/referenced origins, and returns `Unknown` for
    missing authority, raw JSON Schema, root unions, baseline/trigger properties, nested paths, and
    complex expressions.
  - `narrowed_expression_paths` walks the parsed condition AST for the specification's closed
    forms. `analyze_directive_targets` unions every enclosing page-block guard and retains directive
    kind, authored target span, whole-expression span, parsed expression, root, nullability, and
    narrowed paths without evaluation.
- Targeted verification: all four `directive_target_analysis` tests pass. They cover the exact
  `::file {{log}}` input; `::file`, `::code`, and `::url`; native and quoted YAML scalars; missing,
  explicit-null, and concrete values; optional/defaulted/required declarations; supported and
  unknown `ctx.*`; document/referenced/baseline/trigger origins; raw JSON Schema/root unions;
  evaluator truth tables; direct and outer-nested guards; mixed and complex expressions; unchanged
  inputs; no effect-engine/network-attempt delta; and no command-created sentinel.
- The first lint run found that the test's effect counters were feature-gated in the library but not
  at their call sites. Matching the existing passive-analysis test pattern fixed the test-only
  configuration; the full lint rerun then passed.
- Required area gates, using the isolated writable target, Homebrew clang, and explicit macOS SDK:
  `just test` passed all 7,842 executed tests with seven intentional skips, and `just lint` passed
  `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, plus the `zed-dmls` `wasm32-wasip2`
  compile check. No formatter command was run.
- Phase 3 adds only in-memory AST/schema classification and no OS-specific branches, path
  comparisons, platform APIs, or persistence. Remote OS execution was therefore not needed.
- Final GitNexus `detect-changes --scope all` reported 14 changed indexed files, 46 symbols, no
  affected execution processes, and LOW risk. This includes the existing Phase 1/2 worktree; the
  separately present unrelated changes were left untouched.

## Phase 4

- Initialized Phase 4 tracking. The specification already records `implemented: true` and
  `implemented_by: codex/default` as requested.
- Pre-implementation requirement-to-test mapping:
  - `nullable_transclusion_diagnostic_matrix` is the real LSP end-to-end regression test. Its
    shipped document syntax includes the original failing input `::file {{optional}}`, optional
    and `default(...)` metadata declarations, `file(required)`, direct and outer-nested
    `file_exists(...)` guards, and a concrete missing path.
  - The matrix asserts dependent diagnostic state: exactly two nullable warnings; exact code,
    `darkmatter.compose` source, warning severity, expression-token range, and both expression
    roots; exactly one concrete `broken_path`; and no coexistence of `broken_path` on interpolated
    targets.
  - `interpolated_transclusion_target_does_not_report_broken_path` independently pins the original
    `$schema: { log: file }` / `::file {{log}}` false-positive input and downstream suppression.
  - The Phase 3 `directive_target_analysis` matrix remains the exhaustive passive boundary for
    native versus quoted YAML scalars, missing versus explicit-null versus concrete values,
    directive variants, malformed/unknown expressions, schema-authority variants, and guard
    truth tables. Phase 4 consumes that public analysis without duplicating it in DMLS.
  - This phase changes no parser, shipped schema, template, prompt, or configuration artifact and
    persists no values, so no new corpus or read/write/read test is required. The existing passive
    shipped-directive corpus and real `md compose` shipped-fixture test continue to cover those
    unchanged paths.
- Pre-edit GitNexus identity: repository/worktree
  `/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes`, indexed and current at `b1bef68` but stale for
  the prior phase's uncommitted additions. `transclusion_diagnostics` has LOW upstream risk: one
  direct caller (`diagnostics`), three total affected symbols, no indexed execution process, and
  the Providers module only. The new Phase 3 `analyze_directive_targets` symbol is not in the stale
  index, so GitNexus reports UNKNOWN; text search resolves the uncertainty by showing its definition
  and integration tests only, with Phase 4 adding the first DMLS production caller.
- Fail-first evidence: with `BISCUIT_PROMOTE_PENDING=1`,
  `nullable_transclusion_diagnostic_matrix` failed with only the concrete
  `dm.transclusion.broken_path` diagnostic and the controlled oracle "nullable target diagnostics
  are not implemented". The initial shared-target attempt stopped earlier on read-only cached
  `.rmeta` files; the isolated writable target reached the behavioral failure.
- Added the stable `dm.transclusion.nullable_target` code and integrated
  `analyze_directive_targets` into the Layer-3 DSL provider. DMLS passes the overlay bundle's
  already assembled `EffectiveSchema` and `frontmatter_json`, emits only for nullable analyses not
  narrowed by an enclosing supported guard, and ranges the warning on `expression_span`.
- The warning message names the normalized expression root and gives all three remedies: guard
  with `file_exists(...)`, bind a non-null value, or make the parameter required. Concrete,
  required, guarded, mixed, and unknown targets stay quiet; interpolated targets cannot also
  receive `dm.transclusion.broken_path`.
- Promoted and expanded `nullable_transclusion_diagnostic_matrix`. It now includes mixed and
  unsupported whole-expression negative controls and asserts exactly two warnings, exact code,
  source, severity, complete message, expression range, and roots, alongside exactly one concrete
  broken-path warning. The entire real-LSP `lsp_session` target passed 100/100 tests.
- Updated interpolation, expression, Simplified Schema, and DMLS diagnostic documentation for
  evaluated-absence skipping, defaults-as-metadata, non-null mechanisms, narrowing, diagnostic
  range/severity, and interpolated broken-path exclusion. `darkmatter/README.md` has no diagnostic
  inventory and required no change.
- Updated the Darkmatter compose skill with the condition-aware runtime versus condition-blind
  approval invariant, absent-target edge behavior, pending-target fail-closed rule, and recommended
  `file_exists(...)` idiom. `md hash` verifies its recorded hash as
  `ef46db3751d8e999-149976b01d0952ef`.
- Required gates used `/tmp/rusty-biscuit-darkmatter-phase4-target`, Homebrew clang, and the Command
  Line Tools SDK. `just test` passed all 7,842 executed L1 tests with seven intentional skips.
  `just lint` passed `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, and the `zed-dmls`
  `wasm32-wasip2` compile check. No formatter command was run.
- Phase 4 adds only passive in-memory schema/AST analysis and LSP diagnostics. It introduces no
  OS-specific branches, platform APIs, path comparison, persistence, terminal, or browser behavior;
  cross-OS and higher-tier runs were therefore not required.
- Final integrity checks found no unchecked Phase 4 tasks and no whitespace errors. GitNexus
  `detect-changes --scope all` reported 20 changed indexed files and 65 symbols across the existing
  four-phase worktree, no affected execution processes, no partial/truncated result, and LOW risk.

## Phase 5

- Initialized Phase 5 tracking. The specification already records `implemented: true` and
  `implemented_by: codex/default` as requested.
- Pre-verification requirement-to-test mapping:
  - Directive scanner spans and LF/CRLF behavior: `nullable_target_raw_span_contract` and
    `directive_lines_can_be_removed_end_to_start_without_invalidating_spans`.
  - Typed null, empty-string, concrete, native/quoted YAML, missing/present, malformed, mixed, and
    authored-empty boundaries: `whole_value_evaluation_preserves_nullable_target_types`,
    `unguarded_null_target_is_skipped_with_one_warning`,
    `unguarded_empty_string_target_keeps_its_typed_reason`, and
    `authored_empty_targets_remain_errors_and_mixed_targets_remain_paths`.
  - Condition-aware runtime, condition-blind preflight, sibling discovery, line coordinates, graph
    reuse, and pending-command fail-closed behavior:
    `runtime_page_blocks_suppress_guarded_null_target`,
    `condition_blind_preflight_skips_null_edge_but_keeps_concrete_sibling`,
    `preflight_parse_errors_use_file_relative_lines`,
    `pending_shell_target_is_rejected_before_child_command_approval`, and
    `malformed_directive_in_false_block_has_distinct_runtime_and_preflight_outcomes`.
  - Passive nullability and guard narrowing: the four-test `directive_target_analysis` target,
    including the evaluator-backed guard truth table and no-effect assertions.
  - Real CLI lifecycle and downstream non-execution:
    `compose_guarded_nullable_target_through_real_preflight_lifecycle`,
    `compose_rejects_pending_target_before_child_command_can_execute`, and
    `compose_shipped_transclusion_fixture_through_normal_cli_path`, all through
    `CliProcessFixture`.
  - Real LSP diagnostic output and unchanged concrete-path/cycle behavior:
    `interpolated_transclusion_target_does_not_report_broken_path`,
    `nullable_transclusion_diagnostic_matrix`, and the existing transclusion diagnostic suite.
  - Passive shipped-artifact coverage: `shipped_transclusion_fixture_targets_are_scanned_passively`.
- Phase 5 changes no shipped artifact or persisted value, so it requires no new corpus case or
  read/write/read test. The passive corpus and real shipped-fixture CLI test remain the applicable
  evidence.
- Focused library verification passed all 6,438 Darkmatter L1 tests with seven intentional tier
  skips, including the named scanner, evaluation, preflight, coordinate, security, and passive
  corpus regressions. The dedicated `directive_target_analysis` target separately passed 4/4.
  Phase 1 recorded each runtime/security/coordinate regression failing before implementation;
  Phase 3 recorded the passive-analysis target failing to compile before its API existed.
- The real `compose_transclusion` CLI target passed 10/10 through `CliProcessFixture`, including
  the guarded minimal reproduction, pending-target downstream non-execution, and real shipped
  transclusion fixture.
- Verbatim minimal-probe evidence used the identical eight-line fixture and isolated environment:
  - Before, at `b1bef6839814378420b54b0be6c8bbda98c8ee61`: exit status `1`; stdout was empty
    (`0` bytes); stderr was `678` bytes of the colored `TransclusionError: directive parse failed`
    diagnostic, ranged on frontmatter line 3 (`log: file`) and ending with `Error: Expected value,
    found end of directive` plus `Check syntax: ::file path="..."`.
  - After, from this worktree: exit status `0`; stdout was exactly `"\n"` (`1` byte); stderr was
    empty (`0` bytes).
  - The first after attempt was rejected as evidence because a shared temporary Cargo target reused
    the pre-fix Darkmatter artifact across worktrees with identical package identities. Cleaning
    only `darkmatter` and `darkmatter-cli` from that temporary target and rebuilding both current
    packages produced the accepted after capture above.
- The complete real-LSP `lsp_session` target passed 100/100. The before record from Phase 4 had one
  concrete `broken_path`, no nullable warnings, and the pending nullable oracle. The after matrix
  has exactly one `dm.transclusion.broken_path` ranged on `missing.md`, exactly two
  `dm.transclusion.nullable_target` warnings ranged on `{{optional}}` and `{{defaulted}}`, and no
  warning for the directly or outer-nested guarded forms. The independent original
  `::file {{log}}` regression has no `broken_path`; the surrounding suite preserves concrete-path
  and transclusion-cycle behavior.
- Required package-area gates used the isolated writable target, Homebrew clang, and the Command
  Line Tools SDK. `just build` passed for `darkmatter`, `darkmatter-cli`, `dmls`, and
  `zed-dmls-cli`; `just test` passed all 7,842 executed L1 tests with seven intentional higher-tier
  skips; and `just lint` passed all four packages plus the `zed-dmls` `wasm32-wasip2` compile
  check. No formatter command was run. L2 and browser gates were not applicable because this fix
  does not move behavior into a real-terminal or rendering surface.
- Platform review found no added target-specific branches, native-path comparisons, shell
  selection, or filesystem APIs. Directive target classification and rewriting are byte-span and
  line-ending safe (the scanner regression includes LF and CRLF); concrete targets continue
  through the captured expression-resolution context and the unchanged `FileReference`-based
  transclusion resolver. This host declares `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL`, but the
  fix introduces no OS-specific behavior, so three full remote L1 reruns would not answer a new
  platform question. The clean macOS gates are the local evidence; normal Linux, native Windows,
  and WSL2 CI cells remain the authoritative cross-OS runtime evidence.
- Final GitNexus `detect-changes --scope all` reported 20 indexed changed files and 65 symbols,
  zero affected execution processes, no partial/truncated result, and LOW risk. `git diff --check`
  is clean.
- Scope and comment/document review found no stale behavior descriptions in the nullable-target
  changes. The deliberate line/range assertions are the new three-frontmatter-length
  `preflight_parse_errors_use_file_relative_lines` matrix, the concrete `missing.md` DMLS range,
  and the `{{optional}}`/`{{defaulted}}` expression ranges; no pre-existing unrelated line-number
  expectation was re-cut. A separate pre-existing fix-directory move and local ignore/prompt files
  remain untouched and are not part of this implementation.
- Acceptance criteria are satisfied by named passing evidence:
  1. The guarded minimal fixture passes through `md compose`, and the verbatim after probe emits
     only one newline with no transclusion content.
  2. `preflight_parse_errors_use_file_relative_lines` validates the actual directive line across
     three frontmatter lengths and checks that the selected source line contains `::file`.
  3. `interpolated_transclusion_target_does_not_report_broken_path` executes the real LSP path for
     the exact `::file {{log}}` input and observes no `broken_path`.
  4. `nullable_transclusion_diagnostic_matrix` observes one warning per unguarded nullable root
     and no warning under direct or outer-nested `file_exists(...)` guards.
  5. `runtime_page_blocks_suppress_guarded_null_target`,
     `condition_blind_preflight_skips_null_edge_but_keeps_concrete_sibling`, and
     `malformed_directive_in_false_block_has_distinct_runtime_and_preflight_outcomes` preserve the
     condition-aware runtime and condition-blind approval split.
  6. The 7,842-test area gate is green; only the explicitly recorded new coordinate/range
     assertions changed.
  7. The library and CLI pending-shell target regressions fail closed before approval, and the CLI
     sentinel proves the transcluded child command never executes.
  8. The passive classifier and DMLS matrix treat `default(...)` as metadata: `defaulted` remains
     nullable and receives a warning rather than a runtime fallback.
  9. `unguarded_empty_string_target_keeps_its_typed_reason` proves evaluated-empty skipping, while
     `authored_empty_targets_remain_errors_and_mixed_targets_remain_paths` preserves bare and
     quoted-empty syntax errors.
- Phase 5 is implementation-complete and ready for review. No human decision is required before
  handoff, and the fix directory remains active as required.
