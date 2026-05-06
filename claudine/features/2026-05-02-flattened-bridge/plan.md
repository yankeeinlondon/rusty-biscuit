---
phases: 5
created: 2026-05-06
start_phase: 1
spec: claudine/features/2026-05-02-flattened-bridge/spec.md
packages:
  - darkmatter
  - claudine
source_files_during_phase_1:
   - darkmatter/lib/src/markdown/compose/expression/ctx.rs
   - darkmatter/lib/src/markdown/compose/conditions.rs
   - darkmatter/lib/src/markdown/compose/expression/mod.rs
 docs_updated_during_phase_1: []
 docs_created_during_phase_1: []
 skills_files_updated_during_phase_1: []
 packages:
   - darkmatter
source_files_during_phase_2:
  - claudine/lib/src/dispatch/expression.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages:
  - claudine
source_files_during_phase_3:
   - claudine/lib/src/dispatch/runner/mod.rs
   - claudine/lib/src/dispatch/runner/meta_json.rs
 docs_updated_during_phase_3: []
 docs_created_during_phase_3: []
 skills_files_updated_during_phase_3: []
 packages:
   - claudine
source_files_during_phase_4:
  - claudine/lib/src/dispatch/runner/mod.rs
  - claudine/lib/src/dispatch/runner/meta_json.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/configuring-actions.md
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/hook-actions.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/architecture.md
  - .claude/skills/claudine/hook-actions.md
---

# Plan — Unify Hook `when` Lookup with `EventMetaExpressionLookup`

This plan converts the spec at
[`spec.md`](./spec.md) into an executable sequence. The five phases are
**strictly sequential** — each depends on the previous one — but the
**steps within most phases are independent** and can be implemented in
parallel where called out below.

## Goal Summary

Eliminate the `flatten_event_meta_aliases` / `event_meta_to_json` mirror
in `dispatch/runner/meta_json.rs` by routing hook `when` evaluation
through a new `EventMetaConditionLookup` composite that delegates
non-`ctx` paths to the existing `EventMetaExpressionLookup` and resolves
`ctx.*` through a new public Darkmatter `CtxLookup`.

## Pre-Flight Verification

Before Phase 1, confirm the Darkmatter API surface assumed by the spec:

- `darkmatter::markdown::compose::expression::evaluate` — public ✓
- `darkmatter::markdown::compose::expression::is_truthy` — public ✓
  (`darkmatter/lib/src/markdown/compose/expression/mod.rs:126`)
- `darkmatter::markdown::compose::expression::EvaluationLookup` — public ✓
- `darkmatter::markdown::compose::conditions::parse_condition` — public ✓
  (`darkmatter/lib/src/markdown/compose/expression/parser.rs:411`,
  re-exported via `conditions::parse_condition`; verify re-export at
  implementation time and add one if absent)
- `ShortcutLookup` is `pub(crate)` and the `ctx.*` body is embedded in
  `ShortcutLookup::get` — confirms Phase 1 work is required.

If `parse_condition` is not re-exported from
`darkmatter::markdown::compose::conditions`, treat the re-export as a
trivial sub-step inside Phase 1 Step 1.

## Dependency Graph

```text
Phase 1 (darkmatter::CtxLookup public)
    └── Phase 2 (claudine::EventMetaConditionLookup) [needs Phase 1 published]
            └── Phase 3 (rewire evaluate_when)        [needs Phase 2 type]
                    └── Phase 4 (delete flatten layer) [needs Phase 3 callers gone]
                            └── Phase 5 (docs sync)
```

Phases 1–4 are mergeable independently per the spec's "Implementation
Plan" preamble. Phase 5 is documentation-only.

---

## Phase 1 — Darkmatter: Expose `CtxLookup`

**Package**: `darkmatter`. **Goal**: lift the `ctx.*` resolution body out
of `ShortcutLookup::get` into a public, composable `CtxLookup` adapter
without changing any observable Darkmatter behavior.

### Steps

1. **[Sequential, foundation]** Create
   `darkmatter/lib/src/markdown/compose/expression/ctx.rs` containing
   `pub struct CtxLookup<'a>` with the API in spec §A:
   - Fields: `work_dir: &'a Path`, `cache: RefCell<HashMap<String, Value>>`,
     `captured: RefCell<HashSet<ContextGroup>>`.
   - `pub fn new(work_dir: &'a Path) -> Self`.
   - `pub fn resolve_ctx(&self, path: &str) -> Option<Value>` — body is the
     `ctx.*` branch from
     `darkmatter/lib/src/markdown/compose/conditions.rs:299-314` lifted
     verbatim, with `path.strip_prefix("ctx.")` retained.
   - `impl EvaluationLookup for CtxLookup<'_>` whose `get` returns
     `self.resolve_ctx(path)` only for `ctx.` prefix or the bare `"ctx"`
     token, `None` otherwise.
   - Re-export `CtxLookup` from
     `darkmatter/lib/src/markdown/compose/expression/mod.rs` (`pub use
     ctx::CtxLookup;`).

2. **[Depends on Step 1]** Refactor `ShortcutLookup` in
   `darkmatter/lib/src/markdown/compose/conditions.rs` to embed
   `CtxLookup`:
   - Replace `ctx_cache` + `captured_groups` fields with a single
     `ctx: CtxLookup<'a>` field.
   - Update `ShortcutLookup::new` to construct `CtxLookup::new(work_dir)`.
   - Replace the `ctx.*` branch in `ShortcutLookup::get` with
     `self.ctx.resolve_ctx(path)`.
   - Keep the `env.*`, plain data, and fallback `ctx.{path}` branches
     untouched so semantics for non-`ctx` paths and the implicit
     fallback are preserved exactly.
   - Update the `#[cfg(test)] fn captured_groups` helper to delegate to
     `ctx.captured` (test-only API).

3. **[Independent of Step 2 once Step 1 lands; can run in parallel]**
   Add unit tests in
   `darkmatter/lib/src/markdown/compose/expression/ctx.rs`:
   - `ctx_lookup_returns_none_for_non_ctx_paths` — `lookup.get("foo")`,
     `lookup.get("env.HOME")`, `lookup.get("git.branch")` all `None`.
   - `ctx_lookup_resolves_today` — `lookup.get("ctx.today")` returns a
     non-empty `Value::String` and triggers exactly one capture.
   - `ctx_lookup_caches_repeated_lookups` — second call to `ctx.today`
     does not re-capture (assert `captured` size unchanged after second
     call).
   - `ctx_lookup_unknown_ctx_key_returns_none` — `lookup.get("ctx.zzz")`
     returns `None`.

4. **[Independent of Steps 2–3]** Verify `parse_condition` and
   `is_truthy` are reachable from
   `darkmatter::markdown::compose::expression::*` and
   `darkmatter::markdown::compose::conditions::*`. If `parse_condition`
   is only available under `expression::parser`, add a `pub use
   super::expression::parse_condition;` re-export to `conditions.rs`.

5. **[Depends on Steps 1–2]** Re-run the existing Darkmatter test suite:
   - `just test darkmatter` (or `cargo test -p darkmatter`).
   - Specifically confirm
     `darkmatter::markdown::compose::conditions::tests::*` passes
     unchanged.

### Validation Checkpoint — End of Phase 1

- `cargo build -p darkmatter` succeeds.
- `cargo test -p darkmatter` passes with zero behavior changes in
  existing `ShortcutLookup` tests.
- `cargo doc -p darkmatter --no-deps` does not warn on the new
  `CtxLookup` rustdoc.
- `rg "pub use ctx::CtxLookup" darkmatter/lib/src/markdown/compose/expression/mod.rs`
  returns one match.

---

## Phase 2 — Claudine: Introduce `EventMetaConditionLookup`

**Package**: `claudine` (lib). **Depends on**: Phase 1 published. **Goal**:
add the composite without yet rewiring the runner, so Phase 2 ships as
a pure additive change.

### Steps

1. **[Sequential, foundation]** Add `EventMetaConditionLookup` to
   `claudine/lib/src/dispatch/expression.rs`:
   - Imports: `use darkmatter::markdown::compose::expression::{CtxLookup,
     EvaluationLookup};` plus `use std::path::Path;`.
   - `pub struct EventMetaConditionLookup<'a> { inner:
     EventMetaExpressionLookup<'a>, ctx: CtxLookup<'a> }`.
   - `pub fn new(meta: &'a EventMeta, work_dir: &'a Path) -> Self`.
   - `impl EvaluationLookup` whose `get` short-circuits on `path == "ctx"`
     or `path.starts_with("ctx.")` to `self.ctx.get(path)` and otherwise
     delegates to `self.inner.get(path)`.

2. **[Independent of Step 1's tests; can run in parallel after the type
   compiles]** Update the module rustdoc in
   `dispatch/expression.rs` to describe the two adapters:
   - `EventMetaExpressionLookup` — used by templates, matchers, harness
     validation; never resolves `ctx.*`.
   - `EventMetaConditionLookup` — used by hook `when`; layers `ctx.*` on
     top of `EventMetaExpressionLookup`.

3. **[Independent of Step 2]** Add unit tests in
   `claudine/lib/src/dispatch/expression.rs` mirroring every relevant
   `EventMetaExpressionLookup` test against the composite, plus:
   - `condition_lookup_resolves_ctx_today` — confirms `ctx.today`
     produces a non-empty string via the composite.
   - `condition_lookup_falls_through_to_inner` — confirms `tool_name`,
     `git.branch`, `hardware.cores`, `extra.*`, `tool_input.*`,
     `tool_response.*`, `env.HOME` all resolve identically to a
     standalone `EventMetaExpressionLookup`.
   - `condition_lookup_ctx_short_circuit` — paths starting with `ctx.`
     never reach the inner lookup (use a fixture meta where the inner
     lookup would produce a value for a fictional `ctx.x` path; verify
     that path returns `None` from the composite when the underlying
     `CtxLookup` does not recognise it).

### Validation Checkpoint — End of Phase 2

- `cargo build -p claudine` succeeds with the new type unused (warning
  allowed at this checkpoint; resolved in Phase 3).
- `cargo test -p claudine dispatch::expression` passes including all
  new composite tests.
- `rg "EventMetaConditionLookup" claudine/lib/src/dispatch/expression.rs`
  returns the type definition and tests.

---

## Phase 3 — Claudine: Rewire `evaluate_when`

**Package**: `claudine` (lib). **Depends on**: Phase 2 type available.
**Goal**: route hook `when` through the composite; the flattening layer
remains in place but its only caller (`event_meta_to_json` →
`evaluate_condition_against`) is removed.

### Steps

1. **[Sequential, foundation]** Replace the body of `evaluate_when` in
   `claudine/lib/src/dispatch/runner/mod.rs:78-101` per spec §C:
   - Drop the `meta_json: &Value` parameter.
   - Compute `work_dir` from `meta.cwd` exactly as today.
   - Use `darkmatter::markdown::compose::conditions::parse_condition` to
     parse `expr`; on parse error emit the same `tracing::warn!` shape
     and return `WhenOutcome::SkipInvalid`.
   - Construct `EventMetaConditionLookup::new(meta, work_dir.as_path())`.
   - Call `darkmatter::markdown::compose::expression::evaluate(&parsed,
     &lookup)`; on evaluation error emit `tracing::warn!` and return
     `SkipInvalid`. Treat truthy via
     `darkmatter::markdown::compose::expression::is_truthy(&value)` —
     `Run` if truthy, `SkipFalse` otherwise.
   - Update the rustdoc on `evaluate_when` to drop the
     `evaluate_condition_against` reference and describe the composite.

2. **[Sequential, depends on Step 1]** Update the call site in
   `execute_actions` (`runner/mod.rs:116`, `:123`):
   - Delete the `let meta_json = event_meta_to_json(meta);` line.
   - Change `evaluate_when(action.when(), meta, &meta_json)` to
     `evaluate_when(action.when(), meta)`.
   - Remove the now-unused `event_meta_to_json` import in
     `runner/mod.rs:27` (keep `strip_nulls` if still used elsewhere —
     verify with `rg "strip_nulls" claudine/lib/src/dispatch/`).

3. **[Independent of Step 2; can run in parallel after Step 1 compiles]**
   Add a regression test in
   `claudine/lib/src/dispatch/runner/mod.rs` (or its `tests` submodule):
   - `when_ctx_today_resolves` — fixture builds an `EventMeta`, action
     has `when: "ctx.today != ''"`, asserts `WhenOutcome::Run` (or asserts
     the action ran via the existing harness pattern used by other
     `when_*` tests).

4. **[Sequential, must follow Steps 1–2]** Run the full existing `when*`
   battery and confirm it passes without behavioral drift:
   - `cargo test -p claudine dispatch::runner::tests::when` — every
     existing test passes.
   - The `when_invalid_expression_skips_action_non_fatally` test may
     need a wording update if its assertion includes the warning string;
     update only the wording if needed, never the outcome shape.

### Validation Checkpoint — End of Phase 3

- `cargo build -p claudine` succeeds with no warnings related to unused
  `event_meta_to_json` at the call sites (the helper itself remains
  defined inside `meta_json.rs`; that is removed in Phase 4 and warnings
  for an unused `pub(super)` item are expected and acceptable here).
- `cargo test -p claudine dispatch::runner` passes all `when*` tests
  plus the new `when_ctx_today_resolves`.
- `rg "event_meta_to_json" claudine/lib/src/dispatch/runner/mod.rs`
  returns no matches outside of the `mod meta_json;` declaration line.

---

## Phase 4 — Claudine: Delete the Flattening Layer

**Package**: `claudine` (lib). **Depends on**: Phase 3 callers removed.
**Goal**: delete dead code and its mirror tests in one focused commit.

### Steps

1. **[Sequential, foundation]** Confirm zero remaining callers:
   - `rg "event_meta_to_json|flatten_event_meta_aliases"
     claudine/` should return only matches inside
     `dispatch/runner/meta_json.rs` itself.
   - If any call site outside `meta_json.rs` survives, return to Phase 3
     and reroute it before continuing.

2. **[Sequential, depends on Step 1]** Delete the helpers and their
   tests from `claudine/lib/src/dispatch/runner/meta_json.rs`:
   - Remove `pub(super) fn event_meta_to_json` (the entire fn body and
     its rustdoc).
   - Remove `pub(super) fn flatten_event_meta_aliases` (entire fn body
     and rustdoc).
   - Remove the `#[cfg(test)] mod tests` block(s) that pin
     `flatten_event_meta_aliases_*` and any test that exists solely to
     guard the alias surface.
   - Keep `strip_nulls` if it has other callers. Verify with `rg
     "strip_nulls" claudine/lib/src/dispatch/`. If `strip_nulls` is the
     only surviving export, consider whether `meta_json.rs` should be
     renamed; defer that to a follow-up rather than expanding scope.
   - Remove now-unused imports (`serde_json::{Map, Value}`, etc.).

3. **[Sequential, depends on Step 2]** Update the `mod.rs` import in
   `claudine/lib/src/dispatch/runner/mod.rs:27`:
   - `use meta_json::{event_meta_to_json, strip_nulls};` becomes either
     `use meta_json::strip_nulls;` (if it survives) or the entire `mod
     meta_json` and `use` line is removed (if Step 2 left the file
     empty).

4. **[Independent of Steps 1–3]** Update the rustdoc at the top of
   `dispatch/runner/mod.rs` (and `meta_json.rs` if it survives) to
   remove any references to alias-flattening or the
   `evaluate_condition_against` round-trip; point readers at
   `dispatch::expression::EventMetaConditionLookup` instead.

### Validation Checkpoint — End of Phase 4

- `cargo build -p claudine` succeeds with zero warnings.
- `cargo test -p claudine dispatch` passes (`expression`, `template`,
  `matcher`, `runner`).
- `cargo test -p claudine harness::validate::tests::render_template`
  passes — confirms harness messaging untouched.
- `rg "flatten_event_meta_aliases|event_meta_to_json" claudine/`
  returns zero matches.
- `cargo clippy -p claudine -- -D warnings` is clean.

---

## Phase 5 — Documentation Sync

**Package**: docs/skills only. **Depends on**: Phase 4 merged. **Goal**:
make the skill docs and any topic doc that mentions the alias surface
match the new architecture.

### Steps (all parallelizable)

1. **[Independent]** Audit
   `claudine/docs/topics/configuring-actions.md` for any mention of
   alias flattening, `event_meta_to_json`, or
   `evaluate_condition_against`. The spec notes this doc likely
   describes user-facing surface only; if so, no edit is required —
   record "no changes needed" in the closing review note. If a mention
   is found, replace it with the composite-adapter description.

2. **[Independent]** Update
   `.claude/skills/claudine/SKILL.md` — find the existing "dispatch
   expression bridge" paragraph (introduced by feature
   `2026-04-29-leverage-dm-parser`) and:
   - Drop wording that implies hook `when` uses a JSON-flattening path.
   - Note that hook `when` now flows through
     `EventMetaConditionLookup` = `EventMetaExpressionLookup` +
     `CtxLookup`, with `ctx.*` resolution exclusive to hook `when`.
   - Add a one-line note that
     `flatten_event_meta_aliases` / `event_meta_to_json` were removed
     by feature `2026-05-02-flattened-bridge`.

3. **[Independent]** Update
   `.claude/skills/claudine/architecture.md`:
   - Replace any diagram or paragraph that depicts the
     "JSON serialize → flatten → ShortcutLookup" path with the spec's
     **Target State** diagram (composite lookup over inner lookup +
     CtxLookup).

4. **[Independent]** Update
   `.claude/skills/claudine/hook-actions.md`:
   - In the `when:` section, explicitly state that path resolution is
     identical to dispatch templates and matchers, with `ctx.*` as the
     only `when`-exclusive surface.

5. **[Independent]** Search the broader repo for stale references
   that the spec did not explicitly call out:
   - `rg "flatten_event_meta_aliases|event_meta_to_json" claudine/docs
     .claude` — anything found here must be either deleted or rewritten.

### Validation Checkpoint — End of Phase 5

- `rg "flatten_event_meta_aliases|event_meta_to_json" .` returns zero
  matches anywhere in the repo (code, docs, skills).
- All four updated skill/topic files render cleanly under whatever the
  team uses for skill verification (e.g. `pnpm check-fixed` if
  applicable; otherwise a manual diff review).

---

## Cross-Phase Validation (after Phase 4, repeat after Phase 5 docs)

Run from the repo root:

```bash
just lint claudine
just lint darkmatter
just test claudine
just test darkmatter
cargo clippy -p claudine -p darkmatter -- -D warnings
```

Confirm:

- All existing `dispatch::runner::tests::when*` pass without changes
  beyond optional warning-text wording.
- All `dispatch::template`, `dispatch::matcher`,
  `dispatch::expression`, `harness::validate::tests::render_template`
  tests pass without modification.
- New tests added in Phase 1 (`CtxLookup`), Phase 2
  (`EventMetaConditionLookup` parity + ctx), and Phase 3
  (`when_ctx_today_resolves`) all pass.
- `rg "flatten_event_meta_aliases|event_meta_to_json" .` is empty.

## Rollback Strategy

Each phase produces a single focused commit. Rollback by reverting the
phase's commit:

- Phase 1 revert is safe and self-contained (additive Darkmatter API).
- Phase 2 revert is safe (additive type, unused at this point).
- Phase 3 revert restores the JSON round-trip and re-introduces
  `meta_json: &Value` parameter; safe but reintroduces the duplication.
- Phase 4 revert restores `flatten_event_meta_aliases` and
  `event_meta_to_json`; only safe if Phase 3 is reverted first or if a
  forward fix repoints `evaluate_when` back at the helpers.
- Phase 5 revert is doc-only; no functional impact.

## Acceptance Criteria (mirrors spec §Acceptance Criteria)

1. `flatten_event_meta_aliases` and `event_meta_to_json` no longer exist
   in the Claudine codebase.
2. Hook `when`, dispatch templates, event binding matchers, and harness
   validation all derive non-`ctx` path resolution from
   `EventMetaExpressionLookup`.
3. `ctx.*` resolves under hook `when` and remains unresolved under
   templates/matchers/harness.
4. Existing `dispatch::runner::tests::when*`, `dispatch::template`,
   `dispatch::matcher`, `harness::validate::tests::render_template`
   tests pass without behavioral drift.
5. New `when_ctx_today_resolves` test passes.
6. Skill docs (SKILL.md, architecture.md, hook-actions.md) describe the
   composite adapter rather than JSON flattening.
