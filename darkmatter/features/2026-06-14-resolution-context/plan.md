---
agent: codex/
phases: 8
created: 2026-06-15
start_phase: 1
yolo: true
---

# Resolution Context & Token Resolution Execution Plan

Success means every in-scope expression surface has the same read-side function capability, `$()` shell expansion follows the specified token-resolution rules, `doc.*` is available everywhere, optional `file` fields accept empty strings as absent when not required, and the behavior is covered by focused tests and matching documentation.

## Phase 1 — Baseline Orientation

- [ ] Run `cargo metadata --no-deps --format-version 1` and identify the exact workspace packages for `darkmatter`, `darkmatter-cli`, and the relevant `claudine` crates.
- [ ] Inspect the current implementation around each spec-listed surface: frontmatter interpolation, body interpolation, transclusion `when=`, shell discovery, frontmatter shell expansion, reference graph `when=`, public condition API, claudine loop conditions, and claudine hook conditions.
- [ ] Confirm current lookup types and their `EvaluationLookup::resolution_context()` behavior, especially `ResolvingLookup`, `FrontmatterSeedState`, `$()` ternary seed state, `ShortcutLookup`, reference graph lookup, `LoopExpressionLookup`, and event metadata lookups.
- [ ] Confirm current expression namespace handling for `ctx.*`, `env.*`, bare variables, and any existing frontmatter-object accessors.
- [ ] Confirm schema validation behavior for optional and required `file` fields before changing it.
- [ ] Confirm the exact current definitions of context-requiring functions and remote-read discovery functions, including whether `absolute` and `relative` are still present in the remote egress list.
- [ ] Validation checkpoint: record the current failing or missing behavior with focused tests or reproducible commands for the motivating `spec: "{{ file_exists(...) ? ... }}"` case and one non-frontmatter surface that already works.

## Phase 2 — Shared Lookup Semantics

- [ ] Add or identify a shared helper for resolving the reserved `doc` namespace against a root frontmatter value without changing unrelated variable lookup semantics.
- [ ] Implement `doc` as the whole root frontmatter object and `doc.<path>` as dotted traversal through that object.
- [ ] Ensure `doc` and `doc.*` are intercepted before normal key lookup and before any legacy fallback to `ctx.*`.
- [ ] Preserve existing `ctx.*`, `env.*`, and bare frontmatter property behavior outside the new reserved `doc` namespace.
- [ ] Add optional `ResolutionContext` storage and `resolution_context()` overrides to darkmatter lookup types that currently need only seed-state access.
- [ ] Keep test-only or explicitly context-free lookups able to return `None` without forcing dummy paths.
- [ ] Validation checkpoint: add unit coverage proving `doc.build` resolves a frontmatter property even when bare `build` would resolve differently, bare `doc` returns the object, `doc.doc` reaches a literal property named `doc`, and missing `doc.*` values do not fall back to `ctx.doc`.

## Phase 3 — Darkmatter Resolution Context Wiring

- [ ] Thread `options.expression_resolution_context(&runtime.remote_fetch)` into the pre-shell frontmatter interpolation call site.
- [ ] Thread the same resolution context into the post-shell frontmatter interpolation call site.
- [ ] Update frontmatter interpolation state so read-side functions can access `ResolutionContext` during both passes.
- [ ] Update frontmatter interpolation dependency extraction so `doc.<root>` contributes the same dependency root as `<root>`, while bare `doc` contributes no dependency.
- [ ] Thread resolution context into frontmatter `$()` ternary condition evaluation during real execution.
- [ ] Thread resolution context into `$()` ternary branch interpolation during real execution.
- [ ] Thread resolution context into reference-graph `when=` evaluation.
- [ ] Update `evaluate_condition_against` and `ShortcutLookup` to build a `ResolutionContext` from the existing `work_dir`.
- [ ] Leave shell-command discovery frontmatter preflight context-free and document that it enumerates reachable pipelines without expression selection.
- [ ] Validation checkpoint: unit-test `file_exists`, `absolute`, and `relative` in pre-shell frontmatter interpolation, post-shell frontmatter interpolation, `$()` ternary condition or branch interpolation, reference graph `when=`, and `evaluate_condition_against`.

## Phase 4 — `$()` Token Resolution and Diagnostics

- [ ] Implement the `$()` token precedence ladder: quoted literal, numeric literal, boolean literal, `name(...)` expression function, path-bearing executable, bare name on `PATH` executable, bare name frontmatter property, absent property as `null`.
- [ ] Ensure `true` and `false` are always booleans and never commands or frontmatter properties in `$()` shell expressions.
- [ ] Ensure path-bearing tokens such as `/usr/bin/doit` and `./doit` never resolve as frontmatter properties.
- [ ] Ensure expression functions with trailing parentheses are classified as safe expression calls and are excluded from shell approval discovery.
- [ ] Add detection for `$()` directives that contain no shell command in executed position.
- [ ] Replace branch-level parse fallout for all-expression `$()` values with a targeted diagnostic suggesting `{{ ... }}`.
- [ ] Preserve support for mixed expressions such as `$( file_exists('Cargo.toml') ? cargo build : make )`, where the condition is expression-engine content and the selected branch is a shell pipeline.
- [ ] Validation checkpoint: add unit tests covering every ladder case, no-command diagnostics, mixed expression-plus-command ternaries, both-branch preflight enumeration, and safe-function exclusion from approval.

## Phase 5 — Schema and Remote-Read Corrections

- [ ] Update schema validation so an empty string is treated as absent only for non-required `file` fields.
- [ ] Ensure required `file` fields still reject empty strings.
- [ ] Ensure empty-as-absent behavior preserves file typing and completions for optional fields.
- [ ] Split context-requiring read-side functions from remote egress discovery functions.
- [ ] Remove `absolute` and `relative` from the remote egress discovery list while keeping them context-requiring.
- [ ] Ensure remote URL arguments to frontmatter read-side functions fail loudly because frontmatter resolution context is local-only.
- [ ] Preserve remote URL behavior for body and post-shell contexts that already have a remote runtime.
- [ ] Validation checkpoint: add regression tests for optional `file` empty success, required `file` empty failure, `absolute("https://...")` not being registered as remote egress, and remote URL frontmatter read-side failure.

## Phase 6 — Claudine Loop and Hook Context

- [ ] Locate claudine loop evaluation entry points and confirm `prompt_path` or an equivalent prompt source path is available where lookups are built.
- [ ] Thread `prompt_path.parent()` into claudine loop condition lookups.
- [ ] Override `resolution_context()` for loop condition lookups with `ResolutionContext::new(base_dir)`.
- [ ] Confirm loop probes re-run each iteration while the base directory remains fixed to the prompt parent.
- [ ] Locate claudine hook condition lookup construction and identify the hook definition source directory.
- [ ] Thread the hook source directory into event metadata condition lookups.
- [ ] Override `resolution_context()` for hook condition lookups with `ResolutionContext::new(base_dir)`.
- [ ] Add `doc.*` handling to claudine lookups consistently with darkmatter expression lookups.
- [ ] Validation checkpoint: add claudine tests proving loop `until` or `while` can use `file_exists` against the prompt parent, hook `when=` can use a read-side function against the hook source directory, and `doc.*` resolves in both surfaces.

## Phase 7 — Migration and Documentation

- [ ] Migrate in-repo bare `{{doc}}` or equivalent bare `doc` property references that must now refer to a literal frontmatter property to `doc.doc`.
- [ ] Search the repository for remaining bare `{{doc}}`, `{{ doc }}`, and condition or `$()` uses of bare `doc`; classify any historical examples that remain intentionally unchanged.
- [ ] Update rustdoc and code comments called out in the spec where behavior flips: `EvaluationLookup::resolution_context()`, fs-gate comments, frontmatter interpolation docs, dependency-root comments, `ResolvingLookup`, body-wrap comments, shell expansion docs, condition API docs, reference graph lookup docs, remote function docs, and claudine lookup docs.
- [ ] Update `docs/topics/darkmatter-expressions.md` with read-side functions, `$()` token resolution, `doc.*`/`ctx.*`/`env.*` namespaces, availability across surfaces, and the expression-function authoring guide.
- [ ] Update inline and pipeline docs: `docs/inline/fm-interpolation.md`, `docs/inline/interpolation.md`, `docs/inline/fm-shell-expansion.md`, and `docs/darkmatter-compose-pipeline.md`.
- [ ] Update `docs/topics/remote-url-references.md` to state that frontmatter read-side URL arguments are local-only and fail loudly, and that `absolute` and `relative` are not remote reads.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` and `.claude/skills/darkmatter/compose.md` for two-pass interpolation, read-side functions, `doc.*`, `$()` token resolution, and the remote/frontmatter caveat.
- [ ] Regenerate the `.claude/skills/darkmatter/SKILL.md` `hash:` with `md hash .claude/skills/darkmatter/SKILL.md` after editing the skill.
- [ ] Add a changelog or release note entry for the public `evaluate_condition_against` capability addition and the `doc` namespace breaking change if this repository has a current changelog convention.
- [ ] Validation checkpoint: verify documentation examples match implemented behavior and grep confirms migrated `doc` usages.

## Phase 8 — End-to-End Verification

- [ ] Add or update an integration fixture for the motivating optional `spec: file` frontmatter ternary using `file_exists(possible_spec) ? possible_spec : ''`.
- [ ] Verify the motivating fixture composes when `spec.md` exists and writes or exposes the resolved path.
- [ ] Verify the motivating fixture composes when `spec.md` is absent and treats the optional `file` field as absent.
- [ ] Verify `::block when="spec"` behaves correctly for both motivating fixture cases.
- [ ] Run focused darkmatter unit tests for expression lookup, frontmatter interpolation, frontmatter shell expansion, schema validation, remote discovery, reference graph, and condition API.
- [ ] Run focused claudine tests for loop and hook condition behavior.
- [ ] Run the narrowest available package-level test commands for changed crates, escalating to broader workspace checks only if failures indicate cross-crate integration risk.
- [ ] Review `git diff` for surgical scope: no unrelated formatting, no unrelated comment cleanup, no behavior changes hidden inside documentation-only edits.
- [ ] Validation checkpoint: confirm all goals from the spec are satisfied, all non-goals remain untouched, and any deferred or intentionally unchanged historical examples are documented in the implementation notes.

## Parallelization Notes

- [ ] Phase 1 inspection can be split by area: one engineer on darkmatter compose/expression surfaces, one on claudine loop/hook surfaces, and one on schema/remote/docs inventory.
- [ ] Phase 2 must land before most behavior work because `doc.*` and lookup context are shared dependencies.
- [ ] Phase 3 and Phase 4 can proceed in parallel after Phase 2 if both teams coordinate on shared `$()` lookup APIs.
- [ ] Phase 5 can proceed in parallel with Phase 3 and Phase 4 after the current schema and remote-read definitions are confirmed.
- [ ] Phase 6 can proceed in parallel after the public `ResolutionContext` and shared `doc.*` behavior are stable.
- [ ] Phase 7 documentation should start after API and behavior names are stable, but migration searches can begin earlier.
- [ ] Phase 8 must run after all behavior, migration, and documentation changes are complete.
