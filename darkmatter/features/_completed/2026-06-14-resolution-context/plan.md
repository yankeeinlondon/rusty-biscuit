---
agent: codex/
phases: 8
created: 2026-06-15
start_phase: 1
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-06-14-resolution-context/phase-1-baseline.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/doc_namespace.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - darkmatter
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
source_files_during_phase_6:
  - claudine/lib/src/composition/loop_expression.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/cli/tests/level2_prompt_reporting_capture.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/state.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_bare.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_match.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_full_document_schema.snap
docs_updated_during_phase_7:
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/inline/fm-shell-expansion.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/docs/topics/remote-url-references.md
  - prompts/clarify.md
  - prompts/documentation.md
  - claudine/docs/research/usage/_usage.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
packages_during_phase_7:
  - darkmatter
  - claudine
source_files_during_phase_8:
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_during_phase_8:
  - darkmatter
source_code:
  - darkmatter/lib/src/markdown/compose/expression/doc_namespace.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions.rs
  - darkmatter/lib/src/markdown/compose/state.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_bare.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_atom_file_match.snap
  - darkmatter/lib/tests/snapshots/schemas_convert_snapshots__snapshot_full_document_schema.snap
  - darkmatter/cli/tests/cli.rs
  - claudine/lib/src/composition/loop_expression.rs
  - claudine/lib/src/composition/loop_engine.rs
  - claudine/lib/src/dispatch/expression.rs
  - claudine/lib/src/stream/logs/opencode/errors.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/cli/tests/level2_prompt_reporting_capture.rs
documentation:
  - darkmatter/features/2026-06-14-resolution-context/phase-1-baseline.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/inline/fm-shell-expansion.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/docs/topics/remote-url-references.md
  - prompts/clarify.md
  - prompts/documentation.md
  - claudine/docs/research/usage/_usage.md
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
packages:
  - darkmatter
  - claudine
---

# Resolution Context & Token Resolution Execution Plan

Success means every in-scope expression surface has the same read-side function capability, `$()` shell expansion follows the specified token-resolution rules, `doc.*` is available everywhere, optional `file` fields accept empty strings as absent when not required, and the behavior is covered by focused tests and matching documentation.

## Phase 1 — Baseline Orientation

- [x] Run `cargo metadata --no-deps --format-version 1` and identify the exact workspace packages for `darkmatter`, `darkmatter-cli`, and the relevant `claudine` crates.
- [x] Inspect the current implementation around each spec-listed surface: frontmatter interpolation, body interpolation, transclusion `when=`, shell discovery, frontmatter shell expansion, reference graph `when=`, public condition API, claudine loop conditions, and claudine hook conditions.
- [x] Confirm current lookup types and their `EvaluationLookup::resolution_context()` behavior, especially `ResolvingLookup`, `FrontmatterSeedState`, `$()` ternary seed state, `ShortcutLookup`, reference graph lookup, `LoopExpressionLookup`, and event metadata lookups.
- [x] Confirm current expression namespace handling for `ctx.*`, `env.*`, bare variables, and any existing frontmatter-object accessors.
- [x] Confirm schema validation behavior for optional and required `file` fields before changing it.
- [x] Confirm the exact current definitions of context-requiring functions and remote-read discovery functions, including whether `absolute` and `relative` are still present in the remote egress list.
- [x] Validation checkpoint: record the current failing or missing behavior with focused tests or reproducible commands for the motivating `spec: "{{ file_exists(...) ? ... }}"` case and one non-frontmatter surface that already works.

## Phase 2 — Shared Lookup Semantics

- [x] Add or identify a shared helper for resolving the reserved `doc` namespace against a root frontmatter value without changing unrelated variable lookup semantics.
- [x] Implement `doc` as the whole root frontmatter object and `doc.<path>` as dotted traversal through that object.
- [x] Ensure `doc` and `doc.*` are intercepted before normal key lookup and before any legacy fallback to `ctx.*`.
- [x] Preserve existing `ctx.*`, `env.*`, and bare frontmatter property behavior outside the new reserved `doc` namespace.
- [x] Add optional `ResolutionContext` storage and `resolution_context()` overrides to darkmatter lookup types that currently need only seed-state access.
- [x] Keep test-only or explicitly context-free lookups able to return `None` without forcing dummy paths.
- [x] Validation checkpoint: add unit coverage proving `doc.build` resolves a frontmatter property even when bare `build` would resolve differently, bare `doc` returns the object, `doc.doc` reaches a literal property named `doc`, and missing `doc.*` values do not fall back to `ctx.doc`.

## Phase 3 — Darkmatter Resolution Context Wiring

- [x] Thread `options.expression_resolution_context(&runtime.remote_fetch)` into the pre-shell frontmatter interpolation call site.
- [x] Thread the same resolution context into the post-shell frontmatter interpolation call site.
- [x] Update frontmatter interpolation state so read-side functions can access `ResolutionContext` during both passes.
- [x] Update frontmatter interpolation dependency extraction so `doc.<root>` contributes the same dependency root as `<root>`, while bare `doc` contributes no dependency.
- [x] Thread resolution context into frontmatter `$()` ternary condition evaluation during real execution.
- [x] Thread resolution context into `$()` ternary branch interpolation during real execution.
- [x] Thread resolution context into reference-graph `when=` evaluation.
- [x] Update `evaluate_condition_against` and `ShortcutLookup` to build a `ResolutionContext` from the existing `work_dir`.
- [x] Leave shell-command discovery frontmatter preflight context-free and document that it enumerates reachable pipelines without expression selection.
- [x] Validation checkpoint: unit-test `file_exists`, `absolute`, and `relative` in pre-shell frontmatter interpolation, post-shell frontmatter interpolation, `$()` ternary condition or branch interpolation, reference graph `when=`, and `evaluate_condition_against`.

## Phase 4 — `$()` Token Resolution and Diagnostics

- [x] Implement the `$()` token precedence ladder: quoted literal, numeric literal, boolean literal, `name(...)` expression function, path-bearing executable, bare name on `PATH` executable, bare name frontmatter property, absent property as `null`.
- [x] Ensure `true` and `false` are always booleans and never commands or frontmatter properties in `$()` shell expressions.
- [x] Ensure path-bearing tokens such as `/usr/bin/doit` and `./doit` never resolve as frontmatter properties.
- [x] Ensure expression functions with trailing parentheses are classified as safe expression calls and are excluded from shell approval discovery.
- [x] Add detection for `$()` directives that contain no shell command in executed position.
- [x] Replace branch-level parse fallout for all-expression `$()` values with a targeted diagnostic suggesting `{{ ... }}`.
- [x] Preserve support for mixed expressions such as `$( file_exists('Cargo.toml') ? cargo build : make )`, where the condition is expression-engine content and the selected branch is a shell pipeline.
- [x] Validation checkpoint: add unit tests covering every ladder case, no-command diagnostics, mixed expression-plus-command ternaries, both-branch preflight enumeration, and safe-function exclusion from approval.

## Phase 5 — Schema and Remote-Read Corrections

- [x] Update schema validation so an empty string is treated as absent only for non-required `file` fields.
- [x] Ensure required `file` fields still reject empty strings.
- [x] Ensure empty-as-absent behavior preserves file typing and completions for optional fields.
- [x] Split context-requiring read-side functions from remote egress discovery functions.
- [x] Remove `absolute` and `relative` from the remote egress discovery list while keeping them context-requiring.
- [x] Ensure remote URL arguments to frontmatter read-side functions fail loudly because frontmatter resolution context is local-only.
- [x] Preserve remote URL behavior for body and post-shell contexts that already have a remote runtime.
- [x] Validation checkpoint: add regression tests for optional `file` empty success, required `file` empty failure, `absolute("https://...")` not being registered as remote egress, and remote URL frontmatter read-side failure.

## Phase 6 — Claudine Loop and Hook Context

- [x] Locate claudine loop evaluation entry points and confirm `prompt_path` or an equivalent prompt source path is available where lookups are built.
- [x] Thread `prompt_path.parent()` into claudine loop condition lookups.
- [x] Override `resolution_context()` for loop condition lookups with `ResolutionContext::new(base_dir)`.
- [x] Confirm loop probes re-run each iteration while the base directory remains fixed to the prompt parent.
- [x] Locate claudine hook condition lookup construction and identify the hook definition source directory.
- [x] Thread the hook source directory into event metadata condition lookups.
- [x] Override `resolution_context()` for hook condition lookups with `ResolutionContext::new(base_dir)`.
- [x] Add `doc.*` handling to claudine lookups consistently with darkmatter expression lookups.
- [x] Validation checkpoint: add claudine tests proving loop `until` or `while` can use `file_exists` against the prompt parent, hook `when=` can use a read-side function against the hook source directory, and `doc.*` resolves in both surfaces.

## Phase 7 — Migration and Documentation

- [x] Migrate in-repo bare `{{doc}}` or equivalent bare `doc` property references that must now refer to a literal frontmatter property to `doc.doc`.
- [x] Search the repository for remaining bare `{{doc}}`, `{{ doc }}`, and condition or `$()` uses of bare `doc`; classify any historical examples that remain intentionally unchanged.
- [x] Update rustdoc and code comments called out in the spec where behavior flips: `EvaluationLookup::resolution_context()`, fs-gate comments, frontmatter interpolation docs, dependency-root comments, `ResolvingLookup`, body-wrap comments, shell expansion docs, condition API docs, reference graph lookup docs, remote function docs, and claudine lookup docs.
- [x] Update `docs/topics/darkmatter-expressions.md` with read-side functions, `$()` token resolution, `doc.*`/`ctx.*`/`env.*` namespaces, availability across surfaces, and the expression-function authoring guide.
- [x] Update inline and pipeline docs: `docs/inline/fm-interpolation.md`, `docs/inline/interpolation.md`, `docs/inline/fm-shell-expansion.md`, and `docs/darkmatter-compose-pipeline.md`.
- [x] Update `docs/topics/remote-url-references.md` to state that frontmatter read-side URL arguments are local-only and fail loudly, and that `absolute` and `relative` are not remote reads.
- [x] Update `.claude/skills/darkmatter/SKILL.md` and `.claude/skills/darkmatter/compose.md` for two-pass interpolation, read-side functions, `doc.*`, `$()` token resolution, and the remote/frontmatter caveat.
- [x] Regenerate the `.claude/skills/darkmatter/SKILL.md` `hash:` with `md hash .claude/skills/darkmatter/SKILL.md` after editing the skill.
- [x] Add a changelog or release note entry for the public `evaluate_condition_against` capability addition and the `doc` namespace breaking change if this repository has a current changelog convention. (N/A — darkmatter lib/cli have no CHANGELOG.md; no changelog convention exists for this area, so no entry was added.)
- [x] Validation checkpoint: verify documentation examples match implemented behavior and grep confirms migrated `doc` usages.

## Phase 8 — End-to-End Verification

- [x] Add or update an integration fixture for the motivating optional `spec: file` frontmatter ternary using `file_exists(possible_spec) ? possible_spec : ''`.
- [x] Verify the motivating fixture composes when `spec.md` exists and writes or exposes the resolved path.
- [x] Verify the motivating fixture composes when `spec.md` is absent and treats the optional `file` field as absent.
- [x] Verify `::block when="spec"` behaves correctly for both motivating fixture cases.
- [x] Run focused darkmatter unit tests for expression lookup, frontmatter interpolation, frontmatter shell expansion, schema validation, remote discovery, reference graph, and condition API.
- [x] Run focused claudine tests for loop and hook condition behavior.
- [x] Run the narrowest available package-level test commands for changed crates, escalating to broader workspace checks only if failures indicate cross-crate integration risk.
- [x] Review `git diff` for surgical scope: no unrelated formatting, no unrelated comment cleanup, no behavior changes hidden inside documentation-only edits.
- [x] Validation checkpoint: confirm all goals from the spec are satisfied, all non-goals remain untouched, and any deferred or intentionally unchanged historical examples are documented in the implementation notes.

## Parallelization Notes

- [ ] Phase 1 inspection can be split by area: one engineer on darkmatter compose/expression surfaces, one on claudine loop/hook surfaces, and one on schema/remote/docs inventory.
- [ ] Phase 2 must land before most behavior work because `doc.*` and lookup context are shared dependencies.
- [ ] Phase 3 and Phase 4 can proceed in parallel after Phase 2 if both teams coordinate on shared `$()` lookup APIs.
- [ ] Phase 5 can proceed in parallel with Phase 3 and Phase 4 after the current schema and remote-read definitions are confirmed.
- [ ] Phase 6 can proceed in parallel after the public `ResolutionContext` and shared `doc.*` behavior are stable.
- [ ] Phase 7 documentation should start after API and behavior names are stable, but migration searches can begin earlier.
- [x] Phase 8 must run after all behavior, migration, and documentation changes are complete.
