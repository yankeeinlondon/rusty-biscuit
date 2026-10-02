---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/darkmatter/fixes/2026-09-30-file-refs-use-magic/spec.md"
plan: "darkmatter/fixes/2026-09-30-file-refs-use-magic/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/cli/tests/l1/compose_transclusion.rs
docs_updated_during_phase_1:
    - darkmatter/docs/inline/preflight-checks.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
packages:
    - darkmatter
    - darkmatter-cli
    - biscuit-file
    - sniff
    - dmls
    - claudine
    - claudine-gen
    - messenger
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/lib/benches/compose_pipeline.rs
    - darkmatter/lib/benches/compose_schema_transclusion.rs
    - darkmatter/lib/benches/phase6_interpolation.rs
    - darkmatter/lib/benches/reference_graph.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/context/capture/agent.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/checked.rs
    - darkmatter/lib/src/markdown/compose/context/current.rs
    - darkmatter/lib/src/markdown/compose/context/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/tests/icmp.rs
    - darkmatter/lib/src/markdown/compose/tests/identity.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
    - darkmatter/lib/src/markdown/compose/tests/rendering.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/tests/shell.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/array_rendering_json.rs
    - darkmatter/lib/tests/l1/backslash_escape_spans.rs
    - darkmatter/lib/tests/l1/compose_diagnostic_identity.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/compose_reuse_phase5.rs
    - darkmatter/lib/tests/l1/context_functions.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_compose.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/declined_path_transclusion.rs
    - darkmatter/lib/tests/l1/disclosure_transclusion_integration.rs
    - darkmatter/lib/tests/l1/empty_package_area.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/feature_review_incident.rs
    - darkmatter/lib/tests/l1/file_tree_roots.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/frontmatter_surface_projection.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/interpolation_literal_pipeline.rs
    - darkmatter/lib/tests/l1/link_interpolation_integration.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/missing_ctx_capture.rs
    - darkmatter/lib/tests/l1/nested_composition.rs
    - darkmatter/lib/tests/l1/persistent_cache_disabled.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/preflight_child_state_parity.rs
    - darkmatter/lib/tests/l1/preflight_repository_sigils.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/request_context_epoch.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/set_overlay_integration.rs
    - darkmatter/lib/tests/l1/shell_block_integration.rs
    - darkmatter/lib/tests/l1/shell_expansion_coordinates.rs
    - darkmatter/lib/tests/l1/shell_probe_preflight.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/lib/tests/l1/transcluded_shell_failure.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/url_root_identity.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - sniff/lib/src/filesystem/repo/glob.rs
    - sniff/lib/src/filesystem/repo/manifest_index.rs
    - sniff/lib/src/filesystem/repo/mod.rs
docs_updated_during_phase_2:
    - darkmatter/lib/README.md
    - darkmatter/docs/topics/magic-paths.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/inline/text-replacement.md
    - darkmatter/docs/inline/preflight-checks.md
    - biscuit-file/docs/topics/file-references.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_2:
    - darkmatter/docs/topics/compose-requests.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/biscuit-file/references/file-references.md
    - .claude/skills/sniff/remote-and-repository.md
source_files_during_phase_4:
    - darkmatter/dmls/Cargo.toml
    - Cargo.lock
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/lib.rs
    - darkmatter/dmls/src/main.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/bench.rs
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - darkmatter/dmls/src/diagnostics/nested_span/nested_span_tests.rs
    - darkmatter/dmls/src/graph/arena.rs
    - darkmatter/dmls/src/graph/invalidate.rs
    - darkmatter/dmls/src/graph/mod.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/completion.rs
    - darkmatter/dmls/src/providers/diagnostics.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/providers/hover.rs
    - darkmatter/dmls/src/providers/mod.rs
    - darkmatter/dmls/src/providers/semantic_tokens.rs
    - darkmatter/dmls/src/workspace/snapshot.rs
    - darkmatter/dmls/src/workspace/startup.rs
    - darkmatter/dmls/src/workspace/watch.rs
    - darkmatter/dmls/tests/common/mod.rs
    - darkmatter/dmls/tests/fixtures/mapping_only_corpus/baseline.json
    - darkmatter/dmls/tests/l1/level1_graph_index.rs
    - darkmatter/dmls/tests/l1/level1_wiki.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/mapping_only_corpus.rs
    - darkmatter/dmls/tests/l1/repository_contexts.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__reference__file_reference.snap
    - darkmatter/lib/tests/l1/error_snapshots/snapshots/l1__error_snapshots__transclusion__file_reference.snap
    - claudine/lib/src/composition/error/render/mod.rs
docs_updated_during_phase_4:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/features.md
    - darkmatter/dmls/README.md
    - darkmatter/docs/dependencies.md
docs_created_during_phase_4:
    - darkmatter/dmls/docs/file-references.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/os/windows.md
source_files_during_phase_5:
    - claudine/gen/src/agent_errors_check.rs
    - claudine/gen/src/agent_errors_check/review6_tests.rs
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/main.rs
    - claudine/gen/src/signals.rs
    - claudine/gen/src/steering_catalog.rs
    - claudine/gen/src/steering_check.rs
    - claudine/gen/tests/l1/agent_errors_check.rs
    - claudine/gen/tests/l1/drift.rs
    - claudine/gen/tests/l1/signals_validation.rs
    - claudine/gen/tests/l1/steering_activation.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/frontmatter.rs
    - darkmatter/cli/src/commands/graph.rs
    - darkmatter/cli/src/commands/hash.rs
    - darkmatter/cli/src/commands/mod.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/detect.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/io/mod.rs
    - darkmatter/cli/src/request.rs
    - darkmatter/cli/tests/l1/compose_shell.rs
    - darkmatter/cli/tests/l1/compose_state_set.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/lib/benches/clean_hot_paths.rs
    - darkmatter/lib/benches/effective_schema_ownership.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/conditions.rs
    - darkmatter/lib/src/markdown/compose/context/authority.rs
    - darkmatter/lib/src/markdown/compose/context/capture/mod.rs
    - darkmatter/lib/src/markdown/compose/context/effective_state.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/request.rs
    - darkmatter/lib/src/markdown/compose/context/runtime.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/git.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/repository.rs
    - darkmatter/lib/src/markdown/compose/expression/path_projection.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/assignment.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
    - darkmatter/lib/src/markdown/compose/inline/replacement.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
    - darkmatter/lib/src/markdown/compose/link_normalization.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/store.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
    - darkmatter/lib/src/markdown/compose/tests/lazy_roots.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/transclusion/resolver.rs
    - darkmatter/lib/src/markdown/compose/type_tests.rs
    - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/reference/file_tree/mod.rs
    - darkmatter/lib/src/markdown/reference/file_tree/model.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/mod.rs
    - darkmatter/lib/src/markdown/reference/provenance.rs
    - darkmatter/lib/src/markdown/reference/types.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/example.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/lint.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/tests/l1/ambient_ctx_capture.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/clean_counters.rs
    - darkmatter/lib/tests/l1/dasherized_identifier_corpus.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/expression_regression.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs
    - darkmatter/lib/tests/l1/predict_conflicts.rs
    - darkmatter/lib/tests/l1/prelude_exports.rs
    - darkmatter/lib/tests/l1/reference_integration.rs
    - darkmatter/lib/tests/l1/request_context_builder.rs
    - darkmatter/lib/tests/l1/required_context.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_quoting_safety.rs
    - darkmatter/lib/tests/l1/schemas_detect_table.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_required_count_matrix.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_validate_table.rs
    - darkmatter/lib/tests/l1/shell_result_values.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/request_support/mod.rs
    - messenger/lib/src/research/load.rs
    - messenger/lib/tests/research_corpus.rs
    - messenger/lib/tests/research_validation.rs
docs_updated_during_phase_5:
    - darkmatter/dmls/docs/diagnostics.md
    - darkmatter/dmls/docs/file-references.md
    - darkmatter/docs/inline/preflight-checks.md
    - darkmatter/docs/inline/schema-validation.md
    - darkmatter/docs/structs/Markdown.md
    - darkmatter/docs/topics/caching.md
    - darkmatter/docs/topics/darkmatter-expressions.md
    - darkmatter/docs/topics/schemas/definition.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/os/windows.md
---

# Implementation Log for 2026-09-30-file-refs-use-magic (8 phases)

## Phase 1

### Wave 1

#### Reproduce Incident 1

From `claudine/` with the installed `md` (`md 0.1.0`, `~/.cargo/bin/md`):

```text
$ md compose docs/use-claudine/SKILL.md
⤫ TransclusionError: file reference failure
┃
┃ `^` repository reference requires a repository containing reference CWD
┃ `/Volumes/coding/wt/rusty-biscuit/fix-magic-globs/claudine/docs/use-
┃ claudine`
┃
┃ Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped.
```

#### Spike S1: builder-failure inputs (recorded as R11 in the plan)

Ran with a throwaway crate (`/tmp/s1-spike`, path dependencies on
`biscuit-file` and `sniff`; nothing added to the repository).

- (a) `FileResolutionContext::from_snapshot("relative/dir", ..)` fails
  `validate()` with `RelativeContextDirectory { anchor: RequestDirectory }`.
- (b) A request directory inside a repository plus an opening reference
  through `~` or `{{VAR}}` that resolves outside it fails
  `validate()` after `for_source_reference` with
  `RepositoryRootNotContainingSource`. A request directory in `VAULT` with
  the repository root elsewhere fails the same way. With no repository the
  `~` anchor becomes the tree and validation passes.
- (c) A syntactically corrupt `.git/config` (`[core` unterminated) makes
  `GitRepo::discover` and `find_git_root` return `Err`. Rewriting the config
  as valid repairs it. Every other broken state tried (a `.git` file naming
  a missing or empty gitdir, a garbage `.git` file, an empty `.git`
  directory, a missing `HEAD`, an unreadable `.git` on Unix) is reported as
  **no repository**, not an error. A malformed `HEAD`, format version 99, and
  an unknown extension all discover successfully.
- Phase 4 consequence: DMLS must see a `.git/config` change to drop the
  failed entry (see `message_to_agent` in the spec).

#### Spike S2: matrix and guard topology (recorded under R6 in the plan)

Ran with temporary edits to `claudine/cli` (`Cargo.toml`
`source-inputs`, `tests/l1/s2_spike.rs`) and `darkmatter/dmls`
(`tests/l1/s2_spike.rs`), all reverted afterward.

- A `#[path]`-included copy of `darkmatter/cli/tests/common/source_scan.rs`
  compiled and ran in both `dmls::l1` and `claudine-cli::l1`. The darkmatter
  `l1` binary already does this (`semantic_results_never_persist.rs`).
- With only the `#[path]` include, the planner **refused** the declaration:
  `RealWorkspaceTestInputTests.test_every_declared_source_input_is_read_by_its_declarer`
  failed with "claudine-cli declares darkmatter/cli/tests/common/source_scan.rs
  but no L1 test of it names the path". The index walks `#[path]` modules but
  does not count them as references.
- Adding `include_str!("<same path>")` makes the test pass. At module level
  the planned cell is `claudine-cli ubuntu-latest L1 (binary_id(claudine-cli::l1))`.
  Inside the test function it narrows to
  `(binary_id(claudine-cli::l1) & test(=s2_spike::s2_spike_path_include_compiles))`.
  The R6 alternative: each declaring test spells the shared file with an
  in-function `include_str!` next to the `#[path]` include.
- `claudine-cli::l1` holds both completion and composition tests. Several
  `compose_*` modules there are `#[cfg(unix)]`.

### Wave 2

#### Regression tests (written first, confirmed red before the fix)

- `darkmatter/cli/tests/l1/compose_transclusion.rs::test_compose_repository_sigils_from_a_nested_document`.
  This is a git repository built with `CliProcessFixture::initialize_repository_at`.
  `md` is launched from `repo/pkg` (`ambient_context`) with the relative
  argument `docs/guide/doc.md`. The document holds `::file &amp-target.md`,
  `::file ^caret-target.md`, and `::file ^pkg/docs/cli/index.md`, which is
  the shape of the original report. The test asserts exit 0, all three
  bodies in stdout, no leaked `::file`, and no "requires a repository
  containing reference CWD" message. Before the fix it failed with exactly
  that message for `&`.
- `darkmatter/lib/tests/l1/preflight_repository_sigils.rs` uses the same
  layout through `gix::init`. Each test runs two request shapes that carry
  no file-resolution context: `ComposeOptions::for_document(launch_dir, ..)`,
  which is what `md` builds (the source is inside the request repository),
  and `ComposeOptions::new()` (the source is outside the process's
  repository). The tests:
  - `compose_preflight_resolves_repository_sigils_from_a_nested_document`
    checks the approval set holds each target's command and the graph edges
    resolve to the three canonical target paths, in order;
  - `collect_shell_commands_resolves_repository_sigils_from_a_nested_document`
    checks the entries match the targets' commands, in order;
  - `compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document`
    checks the pre-approved set and the discovered count;
  - `compose_preflight_reports_a_missing_repository_target_as_not_found`
    is the negative case: a missing `&` target still fails, names the
    target, and is no longer the missing-repository precondition.

  All four failed before the fix with "`&` repository reference requires a
  repository containing reference CWD".

#### Preflight fix

- `ComposeOptions::prepare_root(&mut self, &Markdown)` (`pub(crate)`,
  `compose/context/options.rs`) runs, in order, `extend_context_for`,
  `establish_repository_observation`, and `ensure_file_resolution_context`.
  `prepared_root(&self, ..)` is the borrowing form, which clones.
- `run_compose_pipeline` calls `prepare_root` in place of the three inline
  calls.
- Pre-flight calls `prepared_root` at the top of `collect_effects` and
  `collect_frontmatter_shell_commands`. **Departure from the plan's wording:**
  the plan lists `compose_preflight`, `compose_preflight_approvals`, and the
  public `collect_*` entries. Every one of those reaches the walk through
  `collect_effects` (`compose_preflight_approvals` goes through
  `compose_preflight`; `collect_shell_commands` goes through
  `collect_shell_commands_with_graph`), except
  `collect_frontmatter_shell_commands`, which is prepared separately. So two
  call sites cover all five entries, plus the pipeline's
  `validate_pre_approved` and `nested.rs`'s use of it.
- The ambient repository observation lives behind an `Arc<OnceLock>` that
  clones share, so preparing a clone in pre-flight does not add a second
  discovery when the pipeline later runs with the caller's options.
- Doc drift fixed: `establish_repository_observation`'s doc said the "root
  pipeline entry" calls it. It now names `prepare_root`.

#### Docs and skill

- `darkmatter/docs/inline/preflight-checks.md`: collection resolves
  transclusion targets as composition does, so `&` and `^` reach the same
  file from any launch directory.
- `.claude/skills/darkmatter/SKILL.md`: pre-flight is a root entry and
  prepares through `prepare_root`.
- `.claude/skills/darkmatter/compose.md`: the "fixed by the root pipeline
  entry" line now names `prepare_root` and both of its callers.

#### Incident check after the fix

Running `target/debug/md compose docs/use-claudine/SKILL.md` from `claudine/`
with the worktree build exits 0 and prints 88 lines with no unresolved
`::file`.

#### Gates

- `just test` (darkmatter): 8749 passed, 12 skipped, exit 0. The four new
  library tests and the new CLI test ran in it.
- `just lint` (darkmatter): exit 0.
- `just test` (claudine, a downstream caller of `compose_preflight`): 8072
  passed, 9 skipped, exit 0.
- No pre-existing failures seen.
- No cross-OS run in this phase. The change adds no path comparison and no
  `#[cfg]` code, and the tests compare canonicalized paths on both sides.
  Windows evidence is gathered in Phase 8 under the plan's Definition of
  Success.
- Not done, per the phase prompt: Commit 1 and `git verify-commit`. The plan's
  Checkpoint asks for them; they are left to the separate commit process.

#### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| `md compose` resolves `&` from a nested directory | `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |
| `md compose` resolves `^` (bare and `^pkg/...`, the incident shape) | same test |
| `compose_preflight` collects `&`/`^` targets with no context (both request shapes) | `preflight_repository_sigils::compose_preflight_resolves_repository_sigils_from_a_nested_document` |
| `collect_shell_commands` likewise | `preflight_repository_sigils::collect_shell_commands_resolves_repository_sigils_from_a_nested_document` |
| `compose_preflight_approvals` likewise | `preflight_repository_sigils::compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document` |
| A missing `&` target still fails, as not-found, not as the repository precondition | `preflight_repository_sigils::compose_preflight_reports_a_missing_repository_target_as_not_found` |

The Input Robustness Matrix does not apply (R15): no file-format reader was
added or changed.

## Phase 2

### Wave 3

#### biscuit-file accessor (subagent)

- `FileReferenceError::resolution_failure()` (`biscuit-file/lib/src/file_reference/resolve.rs`)
  delegates to the private `classify_error`, which stays the one place the
  classification is decided.
- **Departure from the plan:** the plan asks for one test per
  `ResolutionFailure` variant. `classify_error` never returns `NoMatch` (a
  no-match is a result, not an error), so there is no input for it. Four
  variant tests plus `resolution_failure_never_no_match` were added instead:
  `resolution_failure_invalid_reference`, `resolution_failure_missing_context`,
  `resolution_failure_io`, `resolution_failure_unsupported_remote`.
- Also made public (not in the plan): `biscuit_file::capture_env()`, the
  environment reader `FileResolutionContext::new` uses. `RequestSnapshot::from_process()`
  needs it so R2's "same helpers" holds (non-UTF-8 variables skipped,
  `USERPROFILE` via `home_dir()`). Documented in
  `biscuit-file/docs/topics/file-references.md` and the `biscuit-file` skill.
- `just test` (1016 passed) and `just lint` in `biscuit-file/`: exit 0.

#### sniff manifest list (subagent)

- `sniff::filesystem::repo::PACKAGE_MANIFEST_FILE_NAMES` replaces the private
  `MANIFEST_FILES` in `glob.rs`; the membership check in `manifest_index.rs`
  (~691) uses it.
- **Departure:** the two `manifest_index.rs` sites at ~459 and ~501 map each
  name to a `ManifestKind` with a `match` on literals and cannot be written
  over a slice. They were left, and
  `manifest_index::tests::manifest_index_recognizes_every_package_manifest_file_name`
  keeps the ~501 copy in step with the constant (the ~459 walker copy is not
  covered). `glob::tests::package_manifest_file_names_are_pinned` pins the
  contents and order.
- `just test` (3121 passed, 32 skipped) and `just lint` in `sniff/`: exit 0.

#### `RequestSnapshot` and `ContextBuildError`

New module `darkmatter/lib/src/markdown/compose/context/request.rs`,
re-exported from `darkmatter::markdown::compose`:

- `RequestSnapshot::new(dir)` (no home, empty environment), `from_process()`
  (returns `io::Result`; the current-directory read can fail),
  `with_home`, `with_env`, `with_magic_root`, `with_magic_root_tier`
  (Claudine's `~/.claudine` roots need `MagicPathTier::User`; R2 named only
  the inferred form), `with_opening_reference`, `at_request_dir` (keeps home,
  environment, and roots; drops the opening reference, which was resolved for
  the old directory), and accessors.
- `ContextBuildError::{Discovery, Invalid}` with `request_dir()` and
  `resolution_failure()`. **Ruling made here:** a discovery failure classifies
  as `MissingContext`, the class biscuit-file gives its own `Git` errors, so
  the two never disagree.

### Wave 4

#### Builder

`build_resolution_context(&RequestSnapshot)` follows R3: `from_snapshot`,
discovery, scope catalog (which also syncs the launch `@` scope), the
snapshot's extra roots, `for_source_reference` for an opening reference,
`validate()`, then one `tracing::debug!` with `request_dir`, `base_dir`, and
`base_dir_origin`.

- Discovery runs through the same `Repo`-group capture `ctx.repo*` uses
  (`capture_runtime_context_for_groups`), so the observation can be shared.
  With only that group requested, the capture's sole `git` diagnostic is the
  discovery failure, which becomes `ContextBuildError::Discovery`. A relative
  request directory skips discovery (validation rejects it with a typed error
  instead of discovering from the process directory).
- A topology-walk failure under a found repository still degrades to "no
  packages", as before. Only the Git discovery itself is an error.
- `capture_file_resolution_context` and its helper are deleted, with the
  re-exports. Its library callers (`schemas/clean.rs`, `util.rs`
  `source_link_context`'s fallback, tests) and the CLI's
  `schema validate` / `schema triggers` now call the builder.
- Magic paths: `ComposeOptions::magic_paths`, `with_magic_path`, and
  `TransclusionOptions::magic_paths` are deleted (R3). The cache and graph
  identities already encode the context's tier-aware registrations, so their
  explicit `magic_paths` fields were dropped and the identity tests now vary
  the context's roots. `ResolutionContext::magic_paths` (expression side) and
  `document_resolution_context`'s parameter are kept for the no-context
  fallback, which Phase 5 removes; every caller now passes `&[]`.

#### `ComposeRequest`

- `ComposeRequest { options, context }`. `prepare(options, &snapshot)` and
  `with_context(options, context)` (which validates) both return
  `Result<_, ContextBuildError>`. `options()`, `context()`, and
  `map_options()` (for settings decided after preparation, such as
  pre-flight's approval set) are public; `root_options()` (options with the
  context attached) is crate-private.
- `prepare` keeps "one discovery per request" (the `lazy_roots` tests pin
  it): a `ComposeOptions::new()` context that captured only date/time is
  re-anchored on the request directory (`ComposeContext::with_anchor`); the
  request's existing observation is reused when it contains the request
  directory; otherwise the builder's discovery is installed as the request's
  observation (`CurrentAuthority::adopt_ambient_repository`) when the request
  observes its own repository from that directory.
- `ensure_file_resolution_context`, `prepare_root`, and `prepared_root` are
  deleted. `establish_repository_observation` became the module-private
  (`pub(super)`) `establish_request_repository`, called only by
  `ComposeOptions::for_document` and `ComposeRequest`.
- `ComposeOptions::with_file_resolution_context` is now `pub(crate)`: outside
  the crate the only way to attach a context is `ComposeRequest`. The internal
  `Option` field stays until Phase 5, which already lists
  `file_resolution_context()` and `source_file_resolution_context`.
  **Departure from R4's wording** ("`ComposeOptions` loses its `Option`
  context field"): the field is now internal transport between a request and
  its derived child options; Phase 5 makes it required.
- Public entry points on `&ComposeRequest`: `Markdown::compose_with`,
  `compose_preflight`, `compose_preflight_approvals`, `collect_shell_commands`,
  `collect_shell_commands_with_graph`, `collect_frontmatter_shell_commands`,
  `transclusions_with_options`, `ReferenceGraphOptions::with_compose`,
  `shell_expansion::execute_directive`, and `execute_resolved_shell_values`.
  `Markdown::compose()` and `compose_mut()` are deleted.
- Not moved, recorded as departures: `normalize_links` is `pub` inside the
  crate-private `link_normalization` module and `link_resolve` is
  `#[cfg(test)]`; neither is reachable from outside the crate, and both run on
  derived child options inside the pipeline. `validate_pre_approved` is
  `pub(crate)` and also runs on child options (`nested.rs`); it now calls the
  internal `collect_effects` / `frontmatter_shell_commands`.
- Internal inline passes that compose over derived options (pre-flight's
  inline pass, reference-graph content preparation) use the new crate-private
  `Markdown::compose_with_options`, and `ReferenceGraphOptions::from_compose_options`
  replaces their `with_compose` calls. `run_compose_pipeline(&ComposeRequest)`
  delegates to `run_root_pipeline(options)`.
- Added `ComposeOptions::source()` (public getter) so test helpers outside the
  crate can choose a request directory.

#### `ctx.*` from the request

- `ComposeRequest` aligns the `ComposeContext` environment with the request
  context's (`align_context_environment`), so `{{ env.X }}`, `ctx.agent`, and
  `ctx.model` read the snapshot. Extension never overwrites the environment
  (`extend_missing_with` keeps `inner.env`), so later group captures keep it.
- `ResolutionContext::agent()` falls back to the request context's `AGENT`
  before the process.
- Home on the request path already came from the context
  (`options.rs` `expression_resolution_context`), so no change was needed
  there; the `dirs::home_dir()` fallbacks now run only without a context.
- **Residue left for Phase 5** (its "anything left in `runtime.rs`" item):
  `ComposeContext::capture()` / `capture_minimal()` still read the current
  directory and environment, and `capture/mod.rs`'s capture still calls
  `std::env::vars()`. On a request path the environment is replaced and a
  minimal context re-anchored, so neither reaches `ctx.*`; non-request users
  (DMLS validation, `local_expression_resolution_context` without a context)
  still depend on them.
- **Not changed, recorded:** `conditions.rs:379` (`env.*`) belongs to the
  standalone `evaluate_condition_against(data, work_dir)` shortcut API, which
  has no request. It is not on the compose path. Phase 5/6 must classify it.
- `capture/agent.rs`: the two `populate_agent` tests now pass an environment
  map instead of mutating the process environment. The third
  (`capture_runtime_context_includes_agent_group`) still mutates it under
  `serial_test`.

#### Migrating darkmatter's tests

- About 900 call sites in `darkmatter/lib` were moved with a balanced-paren
  rewrite onto two helpers that reproduce the request directory the deleted
  fallback chose (a file source's directory, else the context's absolute
  anchor, else the process directory), with process home and the options'
  own context environment:
  - unit tests: `crate::markdown::compose::test_request(options)`
    (`context/request.rs` `test_support`), which keeps a context the options
    already hold through `with_context`;
  - integration tests: `crate::request_support::{request, request_at}`
    (`lib/tests/request_support/mod.rs`, declared in `l1/main.rs`).
- Tests whose subject is the context moved to `ComposeRequest::with_context`
  (`file_tree_roots`, `link_interpolation_integration`,
  `reference_integration::reference_options`, `unknown_identifier_warning`).
- Behavior surfaced by the migration, now asserted rather than masked: a
  reference graph whose request directory is in this repository rejects a
  document in a temp directory (`RepositoryRootNotContainingSource`) instead
  of silently re-anchoring on the document. Graph tests now anchor the
  request at their fixture (`request_at`).
- `schema_validation::undecided_root_union_resolves_a_caller_file_from_the_launch_area`
  now prepares its request at the launch directory, which is what it tests.
- `transclusion::resolver::resolves_magic_path_prepended` no longer changes
  the process directory; it builds its context from a snapshot root.
- `test_compose_mut_modifies_in_place` was deleted with `compose_mut`.
- Benches and the level2 terminal support were moved to `ComposeRequest`.
- `lib/tests/l1/declined_path_transclusion.rs` is `#[cfg(windows)]`; macOS
  never compiled it. The Windows cross-check caught it.

#### New tests

| Requirement | Test |
|---|---|
| `new()` reads nothing from the process | `context::request::tests::a_new_snapshot_reads_nothing_from_the_process` |
| `at_request_dir` keeps home, env, roots; drops the opening | `context::request::tests::at_request_dir_keeps_home_environment_and_roots_but_not_the_opening` |
| One `debug` event naming request dir and `base_dir` origin (scoped `tracing_test` subscriber) | `context::request::tests::a_successful_build_emits_one_debug_event` |
| A failed build emits none | `context::request::tests::a_failed_build_emits_no_debug_event` |
| AC 6 (a): relative request directory rejected, by builder and `prepare` | `request_context_builder::a_relative_request_directory_is_rejected` |
| AC 6 (b): opening reference through `~` and `{{VAR}}` outside the repository rejected; control builds | `request_context_builder::an_opening_reference_outside_the_request_repository_is_rejected` |
| (b) negative control: no repository, same opening builds | `request_context_builder::an_opening_reference_builds_when_the_request_has_no_repository` |
| R11 (c): corrupt `.git/config` is `Discovery`, `MissingContext`; repair builds | `request_context_builder::a_corrupt_git_config_is_a_discovery_error_until_repaired` |
| Magic-path finding: a snapshot root resolves `@` in the context and through `compose_with`; without it, nothing | `request_context_builder::a_snapshot_magic_root_resolves_an_at_reference` |
| `{{ env.X }}`, `ctx.agent`, and a `{{X}}` file reference read the snapshot while the process lacks `X` | `request_context_builder::an_expression_and_a_file_reference_read_the_same_snapshot_environment` |
| A transclusion's `{{X}}` target reads the snapshot | `request_context_builder::a_transcluded_variable_reference_reads_the_snapshot_environment` |
| `with_context` validates | `request_context_builder::with_context_rejects_an_invalid_context` |
| One discovery per request survives preparation | existing `compose::tests::lazy_roots::ambient_repository::*` (unchanged, passing) |
| Incident 1 stays fixed | existing `preflight_repository_sigils::*` (now through `request_support`) and `compose_transclusion::test_compose_repository_sigils_from_a_nested_document` |

The Input Robustness Matrix does not apply (R15).

#### `md` CLI (compile bridge only; Phase 3 Track A owns the redesign)

- `md compose`: with `--set`, the request is `with_context` over the
  document context it already derived; without `--set` it is `prepare` at the
  document's directory (the launch directory for stdin) with
  `RequestSnapshot::from_process()`. That reproduces the deleted fallback.
  Validation, pre-flight, and compose share the request, and pre-flight's
  approvals are applied with `map_options`.
- `schema validate` / `schema triggers` find their trigger boundary through
  the builder. In `schema validate` a build failure is reported as the file's
  `ParseError` outcome (no better outcome exists yet).
- `from_process()` is called inside `compose.rs`, not once in `main`; Phase 3
  moves it.

#### Docs and skills

- New `darkmatter/docs/topics/compose-requests.md` (snapshot, builder steps,
  failure table, one environment per request, Mermaid flow).
- `darkmatter/docs/topics/magic-paths.md` rewritten for snapshot roots; the
  Claudine example is marked **planned** (Claudine still builds its own
  context until Phase 3).
- Stale `compose_with(options)` / `.compose()` examples fixed in
  `darkmatter/lib/README.md`, `docs/topics/schemas/definition.md`,
  `docs/inline/text-replacement.md`, `docs/inline/preflight-checks.md`.
- `darkmatter` skill (`SKILL.md`, `compose.md`) describes `ComposeRequest`,
  the test helpers, and `compose_with_options`; `prepare_root` is gone.

#### Gates

- `just test` (darkmatter: lib, cli, dmls): 8757 passed, 12 skipped, exit 0.
- `just lint` (darkmatter): exit 0.
- `cargo test --doc -p darkmatter`: 189 passed, 1 failed. The failure,
  `compose::expression::evaluate` (calls an undefined `evaluate_expr`), is
  pre-existing; the file was not touched. Four stale doctests this phase broke
  were fixed. Doctests are not part of `just test`.
- `just cross-check darkmatter --os windows` (filtered to the new and changed
  tests): first run failed to compile `declined_path_transclusion.rs`
  (Windows-only), then three new tests failed on verbatim `\\?\` spellings
  from `canonicalize`. Fixed with `biscuit_file::canonicalize_simplified` and
  portable `{{VAR}}` values; final run 12/12 passed. Linux: 18/18 passed.
  (The `just` wrapper mangles `-E '…(…)…'`; plain substring filters through
  `./scripts/cross-check.sh` work.)
- Not done: the full darkmatter suite on Windows/Linux (Phase 8).

#### Downstream compile errors (Phase 3's input)

messenger, messenger-cli, and claudine-gen still compile. `claudine` does
not, so `claudine-cli` is not checked yet. Per R16, Phase 2 cannot stand
alone, so Phases 2 and 3 land as one commit. `cargo check -p claudine
--all-targets` errors (rustc stops at these; more will follow in
`claudine-cli`):

- lib: `composition/lifecycle/executor.rs:337,344`, `composition/mod.rs:213`,
  `composition/preflight.rs:82,139`,
  `composition/prepare.rs:341,567,678,883,929`,
  `system_prompt/prepare.rs:173,198`. All are
  `with_file_resolution_context` (now crate-private) or an entry point that
  now takes `&ComposeRequest`.
- lib tests: `composition/preflight/tests.rs:324,384,431`,
  `composition/runtime_state/tests.rs:251`, `composition/sequence/tests.rs:761`,
  `invocation_context/tests.rs:306` (`capture_file_resolution_context`).

#### Re-verification (2026-10-01)

A second Phase 2 run found every Phase 2 task already checked and the work
present in the tree, so nothing was re-implemented. Gates re-run on macOS:
`just test` in `darkmatter/` gave 8757 passed and 12 skipped, and `just lint`
exited 0. The plan, log, and spec frontmatter were already complete, so they
were left unchanged.

## Phase 4

Phase 4 ran on top of the committed Phase 2 work and the Phase 3 commits
already on the branch (`14a6bf301` claudine-gen, `b15db8151` md CLI,
`003ffdd3d` messenger). Phase 3's plan tasks are still unchecked and
Claudine (Track B) does not compile yet; nothing in this phase depends on it.

### Wave 7: snapshot and cache

- `RunOptions` gained `snapshot: RequestSnapshot` and lost `Default`
  (`RunOptions::new(snapshot)`). `main.rs` calls
  `RequestSnapshot::from_process()` once, after `--gen-corpus` and before
  `--bench-index` and the server, and passes it to both.
- New `dmls/src/context.rs`: `RepositoryContexts` caches one
  `build_resolution_context` result per key, **failures included**. The key is
  `biscuit_file::find_git_root(folder)`, or the folder itself when there is no
  repository or discovery fails (the builder then reports the same failure
  for that folder). A build uses `snapshot.at_request_dir(key)`, so the request
  directory is the repository root. `for_document(path)` derives with
  `for_source`. A folder → key memo avoids rediscovering per request.
- `DocumentResolution` carries the derived context or a `ContextFailure`
  (`Build(ContextBuildError)` or `UntitledWorkspace`) plus a generation
  number for cache keys. `DocumentContext.resolution` carries it to every
  provider, with `file_context()` and `resolve_reference(raw)` helpers.
- **Departure (counter):** the plan asked for a `work-counters` counter. A
  feature-gated counter would need a self dev-dependency or a new feature in
  every recipe, and a test gated on a feature that `just test` forgets would
  silently vanish. `context_build_count()` is an always-compiled process-wide
  `AtomicUsize`; tests assert deltas (nextest runs one test per process).
- The fixture (`tests/common/mod.rs`) passes `LspFixture::fixture_snapshot`
  (workspace as request dir, `HOME` = `<workspace>/home`, empty environment);
  `start_with_snapshot` lets a test supply its own.

### Wave 8: schema validation, graph, providers

- `overlay/schema.rs::assemble` takes the document's context and calls
  `DarkmatterSchemas::with_file_resolution_context`;
  `with_file_ref_fallback_dir` is gone. The overlay's schema cache key mixes
  in the resolution's generation, so a rebuilt context re-validates.
- **Library fix found here (darkmatter lib):** when a `file(eager)` value
  failed validation, `validate.rs` re-ran resolution *without* the validator's
  request context to build the diagnostic, so a failing `&missing.md` would
  have been reported as a missing-context failure. `FileRefAnchors` now carries
  the context and the re-run uses `resolve_file_reference_in_context`. The
  context-free `resolve_file_reference` is now `#[cfg(test)]` (only its own
  tests call it).
- R5 darkmatter row, schema half: `FileReferenceDiagnostic::ResolutionFailed`
  gained `failure: ResolutionFailure`, and the enum gained
  `resolution_failure()`. All 8 construction sites in
  `compose/schema_validation.rs` pass the class from their error (or the
  fixed class for their non-error cases). One Claudine pattern
  (`composition/error/render/mod.rs:289`) needed `{ raw, .. }`; this was the
  only Claudine edit and it adds no new compile error to Claudine's existing
  Phase 3 list.
- DMLS diagnostics carry R5's `data: {"resolution_failure": "<Class>"}` (the
  `Debug` variant name) on `dm.links.broken_path`,
  `dm.transclusion.broken_path`, `dm.schema.invalid_file_reference`, and
  `dm.context.build_failure`. `LinkDiagnostic::BrokenPath` now carries the
  class (`NoMatch`, or why planning failed).
- Graph: `WorkspaceGraph::build`/`build_with_roots`/`assemble` take
  `&dyn DocumentContexts`, and `WorkspaceIndex::new`/`from_indices` hold an
  `Arc<dyn DocumentContexts>`. Links, transclusions, `$schema`, and file uses
  resolve to the **first `candidate_plan` path that is an indexed document**:
  the graph stays disk-free, and unsaved open buffers count. Contexts are
  fetched once per document folder per rebuild. `diagnose_unresolved` takes
  the document's context. `NoContexts` (resolve nothing) and `FixedContext`
  (one request context for every document) are the non-server sources.
  `WorkspaceIndex::relink()` rebuilds after contexts drop.
- Providers on the context: DSL `resolve_local_path` (hover, definition,
  transclusion links, transclusion diagnostics) now returns a
  `ReferenceTarget` from `resolve_in_context`; frontmatter `nav_targets`
  navigates to the found file, else the first planned candidate (as
  `md compose` absolutizes a link to a missing file); the create-missing-file
  code action creates at the first candidate; anchor completion resolves its
  path through the graph-style candidate plan. `normalize_join` had no
  callers left and was **deleted**, with its test.
- R12: the `trigger_boundary` doc comment claimed the Git root narrows the
  boundary; the code uses the nearest workspace folder only. The comment was
  corrected (code is right).

### Wave 9: invalidation, failure diagnostic, untitled buffers

- Watchers: `watch::context_input_globs()` adds `**/<name>` for each of
  sniff's `PACKAGE_MANIFEST_FILE_NAMES` plus `**/.git/config` (R11's repair
  path); 4 → 9 watchers, 2 → 7 without include globs. `is_context_input`
  keeps those paths out of the document index (before this, every watched
  path was handed to `set_document`).
- Invalidation rule (`RepositoryContexts::invalidate`): drop every entry
  keyed at an ancestor of the changed path, plus, for a path inside `.git/`,
  every entry keyed inside that repository (so a repaired `.git/config` drops
  the failure cached under the document's folder). The folder memo is cleared
  whenever anything drops or a `.git` path changes.
- `apply_watched_changes` invalidates for every event, then re-indexes only
  non-context-input, non-open paths. `reload_config` clears every entry
  (relinking once, folded into the reload's own rebuild).
  `rescan_workspace` (`&mut self` now) returns the changed document paths
  from `reconcile_disk` (which now returns `Vec<PathBuf>`) plus manifests
  whose xxHash changed (`scan_manifests` / `changed_manifests`, baseline taken
  at startup for rescan clients only), and invalidates them.
- **Ordering bug found and fixed:** clearing contexts at the top of
  `reload_config` first published diagnostics with the *old* config, which
  `lsp_session::config_reload_reindexes_wiki_roots` caught. There is now one
  refresh, after the reload.
- Failure diagnostic: `providers/diagnostics.rs` publishes one ERROR at 0:0,
  code `dm.context.build_failure`, source `darkmatter.context` (both new in
  `codes.rs`), message `file references are not resolved in this document
  (<Class>): <error naming the directory>`. The build logs once at `error`
  (`directory`, `failure` fields); a cached failure is not logged again.
  With a failure: graph edges, document links, definition, hover targets,
  transclusion diagnostics, and file-value problems all return nothing.
  **Departure:** the library still evaluates file formats during validation
  without a context; DMLS *suppresses* their problems for that document
  rather than preventing the evaluation. Phase 5 (required context in the
  library) removes the no-context path.
- Untitled buffers: `with_document` accepts `untitled:` URIs. The buffer is
  analyzed at `<repository root>/<buffer name>` and uses that repository's
  built context (cwd = root) when the folders lie in exactly one repository;
  otherwise it carries `ContextFailure::UntitledWorkspace` (class
  `MissingContext`). Untitled buffers are not added to the graph.

### Wave 10: tests

| Requirement | Test |
|---|---|
| AC 8: two documents and the startup index share one build; workspace folder above the repository; `&` anchors at the root | `repository_contexts::documents_in_one_repository_share_one_context_built_at_its_root` |
| AC 8: request dir = repository root, derived cwd = document folder | `context::tests::documents_in_one_repository_share_one_build` |
| AC 8: no repository → own folder; `&` there is `MissingContext` | `repository_contexts::a_document_in_no_repository_resolves_against_its_own_folder` |
| AC 9: `&`, `^`, `@` (snapshot magic root) in schema validation (`file(eager)`), document links (frontmatter, transclusion, Markdown links), definition, code action | `repository_contexts::repository_sigils_resolve_in_every_feature` |
| AC 9 (graph): `&` link, transclusion, and file use resolve; none without a context | `graph::arena::tests::test_repository_sigils_resolve_through_the_document_context` |
| Broken link carries its class | `graph::arena::tests::test_unplannable_link_reports_its_failure_class` |
| AC 12: watched client, only the manifest named | `repository_contexts::a_watched_manifest_change_rebuilds_the_repository_context` |
| AC 12: rescan client, no notification | `repository_contexts::a_rescan_detected_manifest_change_rebuilds_the_repository_context` |
| AC 12: configuration change drops every entry | `repository_contexts::a_configuration_change_drops_every_context` |
| AC 12: snapshot `HOME`, not the process's | `repository_contexts::home_comes_from_the_startup_snapshot` |
| AC 13: one 0:0 error with code, class, directory; no links/definition/validation; clears after repair | `repository_contexts::a_failed_context_is_one_diagnostic_and_no_resolution_until_repaired` |
| AC 13: the log line, once per cached failure | `context::tests::a_failed_build_logs_one_error_naming_the_directory_and_class` |
| Invalidation rule | `context::tests::invalidation_drops_ancestor_and_owned_entries_only`, `a_git_config_change_drops_the_folder_entries_of_its_repository`, `a_corrupt_git_config_is_a_cached_failure_until_invalidated` |
| AC 14: one repository across two folders | `repository_contexts::an_untitled_buffer_uses_the_single_repository_across_the_workspace_folders` |
| AC 14: two repositories / none | `repository_contexts::an_untitled_buffer_fails_with_folders_in_two_repositories`, `…_in_no_repository`, `context::tests::untitled_buffers_need_exactly_one_repository` |
| Watchers and manifest scan | `workspace::watch::tests::test_registration_covers_include_globs`, `test_context_inputs_are_manifests_and_git_files`, `test_manifest_scan_reports_created_changed_and_deleted` |
| `reconcile_disk` reports exact paths | `graph::invalidate::tests::test_reconcile_disk_adopts_created_and_changed_drops_deleted` |
| `untitled:` name parsing | `router::tests::test_untitled_name_reads_only_untitled_uris` |

All new tests are L1 (no tier markers), in `dmls::l1` (declared in
`tests/l1/main.rs`) or the `dmls` lib unit tests; `just check-tier-coverage
darkmatter` reports nothing stranded. The Input Robustness Matrix does not
apply (R15: no file-format reader; manifests are hashed, never parsed).

Test-fixture facts learned (recorded in the `darkmatter` skill): sniff
recognizes a package only under a workspace manifest and counts a declared
member whose directory exists even without its own manifest, so the AC 12
fixture writes the workspace and member manifests together and notifies only
the member's. Inline `$schema` `file` is lazy (syntax only); validation of a
reference needs `file(eager)`.

Existing tests adjusted: unit tests that built graphs or contexts over
rootless `/w` paths now go through `context::test_support::abs` (a drive on
Windows), because a context rejects a relative directory. The first Windows
cross-check caught three frontmatter navigation tests that still used a
rootless path (recorded in the `os` skill, Windows path spelling item 14).
The mapping-only corpus baseline was re-blessed; the only change is the new
`data` payload on 11 diagnostics.

### Gates

- `cargo nextest run -p dmls --features effects-instrumentation`: 774 passed,
  6 skipped.
- `just test` in `darkmatter/`: 8778 run, 8770 passed, 8 failed, 12 skipped.
  **All 8 failures are pre-existing on `HEAD` (`003ffdd3d`), proven by running
  them in a clean `HEAD` worktree**, where the same 8 fail:
  - `darkmatter::l1 error_snapshots::{reference,transclusion}::file_reference_shows_hint`:
    insta snapshots not updated for the `failure:` row `b15db8151` added (R5).
    Both snapshots were accepted in this phase; they now pass.
  - `darkmatter-cli::l1`: `clean_schema::{test_no_trigger_schemas_disables_discovery,
    test_matching_trigger_schema_drives_quoting,
    test_save_shorthand_uses_default_schema_state_and_repairs,
    test_stdin_does_not_discover_repository_trigger_schemas}`,
    `compose_state_set::test_compose_shorthand_numeric_leading_key_is_treated_as_input_path`,
    `compose_transclusion::{test_compose_link_relative_same_repo,
    test_compose_link_transcluded_child}`. These are Phase 3 Track A (`md`
    CLI) behavior changes and are left to Phase 3.
- `just lint` in `darkmatter/`: `darkmatter`, `dmls`, `zed-dmls-cli`, and
  `check-zed` pass. `darkmatter-cli` fails on three pre-existing Phase 3
  clippy errors (`too_many_arguments` in `commands/frontmatter.rs:12` and
  `commands/hash.rs:29`, `ptr_arg` in `io/mod.rs:77`); this phase touched none
  of those files.
- `just test-l2` in `darkmatter/`: 18 + 69 + 3 passed.
- Downstream: `messenger`, `messenger-cli`, and `claudine-gen` compile
  (`--all-targets`). `claudine` fails with exactly Phase 2's recorded list.
- `just cross-check dmls --os windows` (full dmls L1, `effects-instrumentation`):
  first run 773/776, with the 3 rootless-path test failures described above.
  Rerun results follow below.
- Windows rerun after the fix (full dmls L1, `effects-instrumentation`, on
  `build-win-native`): **776 passed, 6 skipped**.
- `just cross-check dmls --os linux` (same scope, `build-linux`): **774
  passed, 6 skipped** (the two-test difference is Windows-only tests).
- Not run here: a Windows cross-check of the darkmatter library for this
  phase's two library edits (a new enum field and passing an existing
  context through). Both are platform-neutral, and Phase 8 runs the full
  per-package Windows cross-checks.

## Phase 5

Phase 5 started on top of the uncommitted Phase 4 work. **Phase 3 Track B
(Claudine) is still not done**: `claudine` does not compile against the
Phase 2 API (13 lib errors at the start of this phase), so Wave 12's Claudine
task cannot be done until it is. Wave 11 (darkmatter lib) and the
darkmatter-cli / dmls / claudine-gen / messenger fallout are done here.

### Wave 11: required context in the darkmatter library

#### Design: the pipeline runs on `ComposeRequest`

- Phase 2 left `ComposeOptions.file_resolution_context: Option<…>` (R4 said
  the field would go). `ComposeOptions` stays the public settings builder,
  and the field is **deleted**. Every pipeline-internal function that took
  `&ComposeOptions` / `ComposeOptions` now takes `&ComposeRequest` /
  `ComposeRequest`, which holds the context by value.
- `ComposeRequest` implements `Deref`/`DerefMut<Target = ComposeOptions>`, so
  stages read settings unchanged. Context-dependent views moved onto the
  request (`transclusion_options`, `source_file_resolution_context`,
  `expression_resolution_context`, `frontmatter_resolution_context`,
  `local_expression_resolution_context`, `with_accepted_source_file`,
  `extended_for`, `compose_cache_fingerprint`); the `ComposeOptions` halves are
  crate-private `*_in(&context)` helpers. Builder chains inside the pipeline
  use the crate-private `ComposeRequest::derive(|options| …)`.
- **Rename:** `ComposeRequest::context()` became `resolution_context()`.
  Through `Deref`, `context()` would have shadowed `ComposeOptions::context()`
  (the captured `ctx.*`), so every stage calling `options.context()` would
  have silently received the wrong type of context.
- `root_options()` is gone; the pipeline consumes the request itself.
  `Markdown::compose_with_options` (internal inline passes) takes a
  `ComposeRequest`.
- `ReferenceGraphOptions.compose` is a `ComposeRequest`; its `Default` (and
  `ReferenceValidationOptions`'s) are deleted, since neither had a context.
  `Markdown::transclusions()` (the ambient compatibility facade) and
  `options_with_reference_resolution_context` are deleted;
  `transclusions_with_options(&request)` remains.
- `FileTree::new(path, &request)` / `from_markdown(md, &request)` replace the
  `std::env::current_dir()` capture (`reference/file_tree/mod.rs:181`); the
  request's `ComposeContext` is extended for the document's `ctx.*` groups.

#### `schemas/`

- `DarkmatterSchemas::new(context)`, `CleanSchemaConfig::new(context)`; no
  `Default`, and `CleanSchemaConfig::with_file_resolution_context` is gone.
  `DarkmatterSchemas::with_file_resolution_context` remains as a
  **replacement** setter (one configuration, such as a CLI baseline, validating
  documents that each carry their own context); it is no longer `Option`.
- `EffectiveSchema`, `ImportEngine`, `rewrite.rs`, `format.rs`,
  `file_match.rs`, `resolve.rs`, and `validate.rs` take the context by value or
  reference. The public `resolve_schema*`, `resolve_yaml_schema*`,
  `rewrite_eager_file_values`, `triggers::scan`, `detect_schema`, and
  `detect_from_document` take it; their `*_in_context` twins were merged in,
  and `detect_from_document_with_context` was removed.
- **Structural validators (new).** A context-free validator used to resolve
  `file` values against the process CWD (`reference.resolve()`). Callers with
  no request — coercion probes, `example(...)` checks, suggestion lint — now
  use `build_structural_validator` / `ValidatorCache::structural_validator_for`:
  a `darkmatter-file` value is judged by `FileReference::new` syntax alone and
  `match()` admits every value, so they never touch the filesystem. The cache
  keeps structural and context-bound entries apart (`JudgedIn`). Diagnostics
  carry `FileRefAnchors::{Syntax, Resolved{..}}`.
- Ambient reads removed: `format.rs` `resolved_from` (now the base directory,
  else the context's `cwd`); `schemas/mod.rs` and `schemas/detect.rs`
  `base_dir_for` (a pathless document uses the context's `cwd`).
- **Departure: `with_file_ref_fallback_dir` is kept.** The plan said it had no
  effect on resolution. It has none on `darkmatter-file`, but the launch-area
  directory is a glob anchor inside `match()`'s `admits`, which the
  glob-reference feature replaces; deleting it now would change `match()`
  results for Claudine and `md compose`. Only the context parameter of
  `admits`/`match_keyword_factory`/`MatchKeyword` changed, as planned, and the
  handed-off `current_dir` fallback in `admits` stays.

#### `compose/` and `expression/`

- `ResolutionContext::new(context)` requires the context (cwd, repository,
  package area come from it). `magic_paths` and the `home_dir` field (with its
  `dirs::home_dir()` fallback) are gone; `agent()` reads only the context's
  `AGENT` (the process fallback is gone). `resolve_document_file_ref*` and
  `resolve_document_directory` take `&FileResolutionContext`;
  `path_projection` (incl. the `dirs::home_dir()` read at `:80`) and
  `path_display_components` too.
- `TransclusionOptions.file_resolution_context` is required.
  `ShellExpansionOptions` (a public settings struct) **lost** its context
  field instead; `resolve_policy_paths(opts, source, &context)` takes it, which
  also removed its two `current_dir` reads (a pathless source uses the request
  directory).
- `util::document_resolution_context` and `source_link_context` were deleted:
  with a required context both were `for_source`/`for_cwd`.
- `PortablePath` task: `link_normalization` always calls `with_ctx` with the
  source's derived request context; the no-context branch is gone.
  `link_resolve` likewise uses the request's derived context only.
- `conditions::evaluate_condition_against(expr, data, &context)`: `env.*`
  reads the context's environment (was `std::env::var`), read-side functions
  resolve through it.
- `runtime.rs`: the CWD-based `ComposeContext::capture()` was deleted (only
  tests called it); `capture_minimal()` is anchored at an empty path instead
  of `current_dir()`. `EffectiveStateBuilder::build()` without a supplied
  context now uses `capture_minimal()` (date/time only) instead of the deleted
  full CWD capture. Production callers all pass a context; Claudine's
  `wrap/sequence/task_run.rs:77` builder passes none and will see only
  date/time `ctx.*` there (recorded for the Claudine track).
- **Residue (not removed this phase):** `context/capture/mod.rs:108`
  `std::env::vars()` still seeds every ambient `ComposeContext` capture
  (`capture_for_dir/content/document`, `CtxLookup` group captures). A request
  replaces that environment with the snapshot's, so it does not reach `ctx.*`
  on a request path, but it is not allowlistable under R7. Removing it means
  the capture API takes the environment (or the request context) from its
  caller; see the message to the next agent.

#### Test fallout

About 690 test compile errors (darkmatter lib unit and `l1` tests, benches)
were fixed by five parallel subagents with disjoint file sets, following one
rule: a test that used the context-free API gets the context the old fallback
built (`FileResolutionContext::new(<document dir>)`: process home and
environment, no repository), or the explicit context it already set, or a
request anchored where its documents live. New helpers:
`compose::test_request_in(options, context)` (unit) and
`request_support::{context_at, cwd_context}` (`l1`). Tests whose premise was
an ambient fallback were rewritten to assert the context-based behavior, never
weakened:

- schemas `CwdGuard` tests (launch-fallback, file-property, `format.rs`
  resolution) now pass a context whose `cwd` is the directory they used to
  `chdir` into; the process directory is no longer changed.
- `context/options.rs` `ambient_context_upgrade_keeps_the_request_launch_anchor`
  became `request_context_upgrade_keeps_the_request_launch_anchor` (prepares
  a request instead of `chdir`).
- `conditions::tests::shortcut_env_lookup` puts its variable in the context's
  environment instead of the process's.
- Reference-graph tests anchor their request at the fixture directory: a
  graph's request context is authoritative, and a document outside the
  request's repository fails with `RepositoryRootNotContainingSource` (the
  Phase 2 behavior), where the deleted default built a per-document context.
- `ambient_ctx_capture.rs` uses `capture_for_dir(current_dir)` for the full
  capture the deleted `capture()` took.

**Structural validators, refined.** The first version judged every `file`
value by syntax and let `match()` admit everything. That broke
`contested_glob_decides_which_arm_coerces_siblings`: root-union coercion picks
an arm through `match()` judging the absolute path a caller value was
projected to. The rule is now: a value whose meaning needs a context (relative,
`&`, `^`, `@`, `~`, vault, `{{VAR}}`) is judged by syntax and admitted by
`match()`; an absolute path needs none, so it must exist and `match()` judges
its full path. Nothing reads the process's directory, home, or environment
(`format::context_free_path`).

### Wave 12: consumers

- **Claudine `Option` contexts: not done (blocked).** Claudine still fails to
  compile against the Phase 2 API (Phase 3 Track B). This phase adds to its
  list: `DarkmatterSchemas::new()`/`with_file_ref_fallback_dir` callers,
  `ResolutionContext::new(cwd)`, `CtxLookup`, `ComposeContext::capture()` in
  tests, `evaluate_condition_against`, `resolve_policy_paths`,
  `ComposeOptions::local_expression_resolution_context` (now only on
  `ComposeRequest`), and `ComposeRequest::context()` (renamed
  `resolution_context()`).
- **darkmatter-cli:** `md schema detect` takes the request (launch context);
  `md schema validate` builds its `DarkmatterSchemas` at the launch context
  and re-anchors per file, and reads `BASELINE_SCHEMA` from the snapshot
  instead of `std::env::var`; `md clean` passes the document's (or, for stdin,
  the launch) context to `CleanSchemaConfig::new`; `md graph` uses
  `FileTree::from_markdown(md, &request)`.
- **Phase 3 Track A residue fixed (7 CLI tests, 3 clippy errors that predated
  Phase 4).** Root cause of 6 tests: `MdRequest::document_context` kept the
  launch derivation whenever it validated, so a document launched from a
  directory in no repository never found its own repository (trigger schemas
  missed, link targets left absolute). It now keeps the launch derivation
  only when the launch context has a repository containing the document, or
  when the document's own build finds none; otherwise it builds at the
  document's directory and derives as a trusted external source (R9). The
  7th (`compose_state_set` numeric shorthand) asserted the removed
  "Failed to load" text; it now asserts R5's `failure: no-match` row. Clippy:
  `#[allow(clippy::too_many_arguments)]` on `run_get`/`run_hash` (repo
  convention) and `&Path` for `resolve_file_path`, `load_markdown_text`,
  `run_graph`.
- **Shell policy for stdin (`md compose -`).** Pre-flight used to drop the
  context, so a stdin document's policy files came from the process
  directory, while execution already used the context's home. With the
  context required, both took the library order (repository root, home,
  request directory), and on Windows the home is the real profile even in a
  fixture (`dirs::home_dir()` ignores `USERPROFILE`), which turned 11 CLI
  tests red on build-win-native. `md` now pins `policy_root` to the launch
  directory for stdin, as it already pinned it beside a file document, so
  pre-flight and execution name the same file and never the user's home.
  `compose_shell::test_compose_stdin_unapproved_command_fails_with_guidance`
  keeps expecting the launch directory; the comment explains why. Recorded in
  the `os` skill (Windows item 2).
- **dmls:** `overlay::schema::assemble` takes `&FileResolutionContext`; a
  document whose context failed gets no schema bundle (its `$schema` and
  trigger payloads cannot resolve without one), replacing Phase 4's
  "suppress file-value problems" workaround. Trigger scanning takes the
  document's context. Docs updated (`file-references.md`, `diagnostics.md`).
- **claudine-gen** (subagent): `load_validated_frontmatter(path, &context)`;
  a context per area built once through the new
  `inputs::area_resolution_context(area, &snapshot)` (the old
  `generator_schemas` body) and threaded into signals, steering catalog and
  check, and agent-errors check. `main` already holds the one snapshot.
- **messenger:** `Loader::new` uses `DarkmatterSchemas::new(context)`; two
  tests build their context with the builder.

### Requirement-to-test mapping

| Requirement | Test |
|---|---|
| A string document's eager `file` value resolves from the request directory; the miss names it (was process CWD, `format.rs` `resolved_from` + `base_dir_for`) | `l1 required_context::a_string_document_resolves_file_values_from_the_request_directory` |
| `env.*` in the condition API reads the context (was `std::env::var`) | `required_context::a_condition_reads_env_from_the_context_not_the_process` |
| Condition read-side functions resolve from the context directory | `required_context::a_condition_resolves_files_from_the_context_directory` |
| Detection infers `file` from the context directory (was `detect.rs` `base_dir_for`) | `required_context::detection_infers_file_values_from_the_context_directory` |
| Structural validators read no process state; absolute paths judged, `match()` on absolute paths | `required_context::structural_validators_resolve_only_context_free_file_values`, `compose::tests::schema::…::contested_glob_decides_which_arm_coerces_siblings` |
| Pathless source's shell policy paths use the request directory (was `store.rs` `current_dir`) | `required_context::a_pathless_source_keeps_shell_policy_in_the_request_directory`, CLI `compose_shell::test_compose_stdin_unapproved_command_fails_with_guidance` |
| `ComposeOptions::new` captures no process directory (`capture_minimal`) | `required_context::new_options_are_anchored_only_by_their_request` |
| `PortablePath` with context: a non-file source's links normalize against the request context | `required_context::a_string_documents_links_normalize_against_the_request_context` |
| Required context (AC2) | compile-time: no `Option<FileResolutionContext>` remains in darkmatter lib/cli, messenger, claudine-gen (Phase 6's guard locks it) |
| `md` documents launched outside any repository use their own repository | CLI `clean_schema::*` (4), `compose_transclusion::test_compose_link_{relative_same_repo,transcluded_child}` |

Each `required_context` test except the link-normalization one fails on the
pre-phase code (it read the process directory or environment); the
link-normalization test pins behavior that already held on the request path.
All are L1 (`darkmatter::l1`, declared in `tests/l1/main.rs`), no tier
markers. The Input Robustness Matrix does not apply (R15).

### Gates

- `just test` in `darkmatter/`: **8786 passed, 12 skipped** (lib, cli, dmls).
- `just test-l2` in `darkmatter/`: 3 + 18 + 69 passed.
- `just lint` in `darkmatter/`: passes.
- `messenger`: `cargo nextest run -p messenger -p messenger-cli` 692 passed,
  2 skipped; `just lint` passes.
- `claudine-gen`: 194 passed; `cargo clippy -p claudine-gen --all-targets -D
  warnings` clean. `just lint`/`just test` in `claudine/` cannot run: the
  `claudine` crate does not compile (Phase 3 Track B).
- Cross-OS (`scripts/cross-check.sh`, local tree):
  - Linux (`build-linux`), darkmatter: **7190 passed**, 67 skipped.
  - Windows (`build-win-native`), darkmatter: first run 7147 passed, 7
    failed. Fixed: `validate_fail_fast` and
    `resolve_file_reference_no_match_for_missing_absolute_path` used `/tmp`
    paths (not absolute on Windows); the four
    `lazy_roots::ambient_repository` tests (file unchanged this phase)
    canonicalized their fixture root with `canonicalize()`, whose verbatim
    `\\?\` spelling never matched the discovered root, now
    `canonicalize_simplified`. Rerun of those plus `required_context`: 17 of
    18 pass. **Remaining, pre-existing:**
    `compose::tests::schema::…::schema_number_increment_survives_quoted_persistence_round_trips`
    builds `dirname(spec) + '/review-2.md'` with `dirname` = `""`, i.e.
    `/review-2.md`, which is a foreign absolute path on Windows. Neither the
    test nor that code path changed this phase; it is left for Phase 8's
    Windows pass.
  - Windows, darkmatter-cli: first run 11 failed (stdin shell policy reached
    the real profile, above); after the `policy_root` fix **794 passed**.
- Final `just test` in `darkmatter/` after those fixes: **8786 passed**,
  12 skipped; `just lint` passes.
