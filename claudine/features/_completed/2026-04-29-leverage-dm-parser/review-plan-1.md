---
phases: 4
created: 2026-05-02
review: claudine/features/2026-04-29-leverage-dm-parser/review-1.md
spec: claudine/features/2026-04-29-leverage-dm-parser/spec.md
prior_plan: claudine/features/2026-04-29-leverage-dm-parser/plan.md
package_areas:
  - claudine
packages:
  - claudine
source_files_during_phase_1:
  - claudine/lib/src/dispatch/runner.rs
  - claudine/lib/src/dispatch/expression.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/dispatch/template.rs
docs_updated_during_phase_2:
  - claudine/docs/topics/unified-events.md
  - claudine/docs/topics/configuring-actions.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/dispatch/loader.rs
  - claudine/lib/src/dispatch/matcher.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
---

# Review 1 — Closure Plan

Source documents:

- Review: `claudine/features/2026-04-29-leverage-dm-parser/review-1.md`
- Spec: `claudine/features/2026-04-29-leverage-dm-parser/spec.md`
- Original plan: `claudine/features/2026-04-29-leverage-dm-parser/plan.md`

## Goal

Close the three findings from review-1 so the feature's central promise — one shared
expression language across templates, hook `when`, matchers, and validation messages —
holds at the implementation level, with documentation that matches behavior and tests
that pin production semantics.

## Decisions Made During Planning

These resolve ambiguities the review left to the planner.

### D-1: How to unify `when` with `EventMetaExpressionLookup` while preserving `ctx.*`

Reviewed both options the review suggested:

- **Composite lookup** would require either reaching into Darkmatter to construct a
  `ShortcutLookup` directly (its constructor is private — see
  `darkmatter/lib/src/markdown/compose/conditions.rs:248`) or duplicating its lazy
  `ctx.*` capture inside Claudine. Both add coupling.
- **Enrich the JSON payload with flattened aliases** is a single, localized change
  inside `runner.rs::event_meta_to_json`. The serialized `EventMeta` already places
  the typed env data under `env.git.*`, `env.os.*`, `env.hardware.*`, and
  `env.repo.*`. Mirroring those subtrees to top-level `git`, `os`, `hardware`, and
  `project` keys makes `evaluate_condition_against` (which uses `ShortcutLookup`)
  resolve `git.branch` etc. without losing `ctx.*` lazy capture.

**Decision:** enrich the JSON payload. It is the minimal change, keeps `ctx.*`
working exactly as today, and matches the resolution order documented in
`dispatch/expression.rs`. The aliases must mirror `EventMetaExpressionLookup`
exactly, including the `git.is_dirty` boolean and `hardware.cores` numeric type
(not stringified), and the `repo.*` -> `project.*` rename.

Out of scope for this review: replacing `evaluate_condition_against` with a path that
takes a custom `EvaluationLookup`. That would be a darkmatter-side API addition.

### D-2: Single-pipe `|` fallback compatibility

The review explicitly says: **"Suggested fix: update all docs plus migration notes."**
Code inspection confirms Darkmatter's interpolation lexer requires `||` (the existing
template tests in `dispatch/template.rs` already cover only `||`). Adding a pre-pass
rewriter would mean re-introducing exactly the kind of ad-hoc syntax bridging the
feature aimed to delete; it would also need to disambiguate `|` inside string
literals from fallback `|`, which is brittle.

**Decision:** drop the legacy single-pipe syntax. Update the three doc references and
add a migration note. Add a single regression test that asserts a stale single-pipe
template is preserved verbatim (current behavior) so the dropped semantics are
deliberately pinned, not accidentally re-introduced.

### D-3: Matcher invalid-input loader assertion

Review wants the production behavior pinned end-to-end so a future contributor doesn't
"fix" `matches_with_pattern()` in the wrong direction. The cleanest place is a new
test in `dispatch/loader.rs` that drives `RuntimeBinding::from_action_binding` (or
equivalent loader entry point) with an invalid matcher string and asserts the
resulting binding has `matcher() == None`, then drives `matcher::matches(None, &meta)`
and asserts `true`. This exercises the same path production uses.

## Phase Index

| Phase | Outcome | Depends on |
| --- | --- | --- |
| 1 | `when` evaluation resolves grouped event paths via flattened JSON aliases | none |
| 2 | Legacy single-pipe fallback is documented as removed; regression pinned | 1 |
| 3 | Matcher invalid-input production semantics pinned by a loader test | 1 |
| 4 | Full clippy/test sweep on `-p claudine` is clean | 1, 2, 3 |

Each phase is self-contained, can be executed by `rust-developer` in one pass, and
ends with focused `cargo test -p claudine` verification.

---

## Phase 1 — `when` evaluation unification

**Outcome:** action `when` clauses resolve `git.*`, `os.*`, `hardware.*`, `project.*`,
nested `tool_input.*`, and `extra.*` against the same data the templates and matchers
see, while continuing to support `ctx.*` lazy capture and `env.*` shell lookups.

### Scope

Implement D-1 by enriching the JSON value passed to
`darkmatter::markdown::compose::conditions::evaluate_condition_against` so that
top-level path lookups in `ShortcutLookup` resolve to the same values that
`EventMetaExpressionLookup` exposes today.

### Files touched

- `claudine/lib/src/dispatch/runner.rs`
  - Modify `event_meta_to_json` (currently lines ~116-126) to produce a flattened
    payload, OR introduce a new helper `event_meta_to_when_json` and call it in
    `execute_actions` (`let meta_json = event_meta_to_when_json(meta);`, line ~141).
  - The new payload is a `serde_json::Value::Object` with:
    - All existing top-level fields from `serde_json::to_value(meta)` preserved
      (so `tool_input.command`, `extra.*`, `provider`, `tool_name`, etc. continue
      to work via `ShortcutLookup` nested resolution).
    - Top-level alias keys mirroring `EventMetaExpressionLookup::resolve_env_path`:
      - `os` -> object copied from `env.os` but reshaped so
        `os.name`, `os.type` (NOT `os.os_type`), `os.version`, `os.hostname`
        match what `EventMetaExpressionLookup` exposes. Concretely insert a new
        `os` object: `{ "name": ..., "type": meta.env.os.os_type, "version": ...,
        "hostname": ... }`.
      - `hardware` -> `{ "arch": ..., "cpu": ..., "cores": <number> }`. Cores must
        be a JSON `Number`, not a string, so `hardware.cores > 8` works.
      - `git` -> when `meta.env.git.is_some()`, an object whose keys match the
        `git.*` paths in `EventMetaExpressionLookup`: `branch`, `is_dirty`
        (JSON bool), `head_sha`, `head_message`, `remote` (from `remote_name`),
        `hosting` (from `hosting_provider`), `repo_name`, `repo_org`. When
        `git` is `None`, omit the alias key entirely so `git.branch` resolves
        to `Null` (matching `EventMetaExpressionLookup` returning `None`).
      - `project` -> object with `language` (from `meta.env.primary_language`),
        `is_monorepo` (JSON bool, from `meta.env.repo.is_monorepo`), and
        `monorepo_tool` (from `meta.env.repo.monorepo_tool`). When both
        `primary_language` and `repo` are `None`, omit the `project` key.
  - Centralize the alias construction in a private helper
    `flatten_event_meta_aliases(meta: &EventMeta) -> serde_json::Map<String, Value>`
    that returns just the alias entries; merge it into the serialized
    `EventMeta` value at the top level. This keeps the change auditable and
    unit-testable.
  - Add a doc comment on `event_meta_to_json` (or the renamed helper) that
    explains *why* the flattening exists: `ShortcutLookup` performs flat
    JSON-path resolution and we need it to see the same paths the
    `EventMetaExpressionLookup` exposes.

- `claudine/lib/src/dispatch/expression.rs`
  - **No code changes.** The doc comment at the top of the file already states
    the resolution order. After this phase, add a one-line note in the module
    doc that says: "Hook action `when` evaluation in `dispatch::runner` mirrors
    these paths via JSON flattening; keep the two layers in sync if you add or
    rename event metadata fields." This guards against future drift.

### New tests

Add to the existing `mod tests` in `claudine/lib/src/dispatch/runner.rs`, in the
`// `when` condition tests` block (after line ~1780). Each test uses
`make_meta_for_when_tests` plus targeted mutations to populate `env.git`,
`env.hardware`, etc. The tests should follow the existing pattern: a single
`HookAction::Call` with a `when` expression, asserted via `execute_actions`
returning either a synthesized `HookResponse::Deny` (truthy condition fired the
failing call) or `None` (skipped).

Test list (one `#[tokio::test]` each):

1. `when_git_branch_matches_main_resolves_truthy` — populate
   `meta.env.git = Some(GitContext { branch: Some("main"), .. })` and use
   `when: "git.branch == 'main'"`. Expect deny (action ran).
2. `when_git_is_dirty_resolves_as_boolean` — `is_dirty: false`,
   `when: "!git.is_dirty"`. Expect deny.
3. `when_hardware_cores_numeric_comparison` — `hardware.cores = 16`,
   `when: "hardware.cores > 8"`. Expect deny. This pins that the alias is a
   JSON number, not a string.
4. `when_project_language_matches` — `primary_language = Some("Rust")`,
   `when: "project.language == 'Rust'"`. Expect deny.
5. `when_nested_tool_input_path` — `tool_input = Some(json!({"command":"npm test"}))`,
   `when: "tool_input.command == 'npm test'"`. Expect deny. (This already works
   via `serde_json::to_value(meta)`; adding the test prevents regression.)
6. `when_extra_dot_path_resolves` — insert `extra` key `attempt = json!(3)`,
   `when: "extra.attempt > 1"`. Expect deny.
7. `when_missing_git_block_is_falsy` — leave `meta.env.git = None`,
   `when: "git.branch == 'main'"`. Expect `None` (skipped).

Also add a focused unit test for the new helper:

8. `flatten_event_meta_aliases_mirrors_expression_lookup` (non-async,
   `#[test]`) — build a `sample_meta` mirroring the one in
   `dispatch/expression.rs::tests`, call the helper, and assert the produced
   `Map` contains `git.branch == "main"`, `git.is_dirty == true` (bool),
   `hardware.cores == 16` (number), `project.is_monorepo == true` (bool),
   `os.type == "macos"`. This is the contract the expression-lookup module
   needs the runner to maintain.

### Verification commands

```bash
cargo test -p claudine dispatch::runner::tests::when
cargo test -p claudine dispatch::runner::tests::flatten_event_meta_aliases
cargo test -p claudine dispatch::expression
cargo test -p claudine dispatch::template
cargo test -p claudine dispatch::matcher
```

### Exit criteria

- [ ] All 8 new tests pass.
- [ ] Existing 7 `when_*` tests still pass unchanged (regression guard).
- [ ] Existing `dispatch::expression`, `dispatch::template`, `dispatch::matcher`
      test counts (per review-1: 28, 17, plus expression-mod tests) match the
      pre-phase baseline.
- [ ] No new `tracing::warn!` is emitted by the seven existing `when_*` tests
      (the new flattening must not break the truthy/falsy paths that already
      work, e.g. `tool_name == 'Bash'`, `env.X == 'yes'`).

---

## Phase 2 — Legacy `|` fallback documentation cleanup

**Outcome:** The three doc references that still advertise single-pipe `|` fallback
are corrected to `||`, a migration note is added, and a regression test pins the
current behavior (single-pipe tokens are preserved verbatim by `interpolate()`).

### Scope

Implement D-2: drop the legacy syntax. No production code change beyond a regression
test, since the parser already requires `||`.

### Files touched

- `claudine/docs/topics/unified-events.md`
  - Line ~632: change `{{env.GREETING | "Hello"}}` to `{{env.GREETING || "Hello"}}`
    in the `Speak` action example block.
  - Line ~959: replace the rule
    `5. The `env.VAR | "default"` fallback syntax uses the default only when the
    variable is not set; an empty-string value is still used as-is.`
    with
    `5. The `env.VAR || "default"` fallback syntax uses the default only when the
    variable is not set or empty (Darkmatter `||` is short-circuit on
    falsy/empty values). The legacy single-pipe `|` form is no longer
    supported — see the migration note below.`
  - Add a new short subsection at the end of section 3.7 (Variables) titled
    `#### Migration: single-pipe fallback removed` containing:
    > Earlier Claudine builds accepted `{{env.VAR | "default"}}` (single pipe)
    > as a fallback. The new template engine uses Darkmatter's expression
    > parser, which only accepts `||`. Any remaining single-pipe templates
    > are preserved verbatim in output (no error, no replacement). Search
    > your config for ` | ` inside `{{...}}` blocks and replace with ` || `.

- `claudine/docs/topics/configuring-actions.md`
  - Line ~282: change `{{env.MY_VAR | "fallback_value"}}` to
    `{{env.MY_VAR || "fallback_value"}}` and append the sentence:
    "The single-pipe `|` form is no longer supported (see the migration note
    in unified-events.md §3.7)."

- `claudine/lib/src/dispatch/template.rs` — **regression test only**, no
  source change. Add to `mod tests` (after `malformed_expression_is_preserved`,
  ~line 815):

  ```rust
  #[test]
  fn legacy_single_pipe_fallback_is_preserved_verbatim() {
      // Single-pipe `|` is no longer recognised as a fallback operator;
      // Darkmatter's interpolation lexer only accepts `||`. The token
      // must therefore round-trip unchanged so operators can spot stale
      // configs in their output.
      let meta = sample_meta();
      let raw = "{{env.CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE | \"fallback\"}}";
      // SAFETY: tests run sequentially in this module.
      unsafe { std::env::remove_var("CLAUDINE_TEMPLATE_TEST_LEGACY_PIPE"); }
      assert_eq!(interpolate(raw, &meta), raw);
  }
  ```

### New tests

Just the one regression test above.

### Verification commands

```bash
cargo test -p claudine dispatch::template::tests::legacy_single_pipe_fallback_is_preserved_verbatim
cargo test -p claudine dispatch::template
# Sanity: docs render. Markdown is plain text; no special tooling needed.
grep -n " | \"" claudine/docs/topics/unified-events.md \
                claudine/docs/topics/configuring-actions.md
# The above grep must return zero matches inside `{{...}}` blocks.
```

### Exit criteria

- [ ] The three doc lines listed above use `||` and the migration note exists.
- [ ] `legacy_single_pipe_fallback_is_preserved_verbatim` passes.
- [ ] The full `dispatch::template` test count increases by exactly 1.
- [ ] A repo-wide grep for `\| \"` inside `{{...}}` braces in
      `claudine/docs/**/*.md` returns no matches.

---

## Phase 3 — Matcher invalid-input loader test

**Outcome:** A loader-level test pins the production behavior: an invalid matcher
string in a config produces a `RuntimeBinding` whose `matcher()` is `None`, and
`matcher::matches(None, &meta)` returns `true`, so the binding fires unconditionally.
Future maintainers reading `matches_with_pattern()` (which returns `false` for
invalid input) will see this test and not "fix" the helper in the wrong direction.

### Scope

Implement D-3 with a new test in `dispatch/loader.rs`. No production code change.

### Files touched

- `claudine/lib/src/dispatch/loader.rs`
  - Identify the existing `mod tests` (or add one if absent — based on the file
    being 1464 lines, a `tests` module almost certainly exists; verify with
    `grep -n '#\[cfg(test)\]\|mod tests' claudine/lib/src/dispatch/loader.rs`).
  - Add a new test that:
    1. Constructs a minimal `ClaudineConfig` (or whatever struct
       `RuntimeBinding::from_action_binding` accepts as input — inspect
       around line 131 and 152 in loader.rs where `RuntimeMatcher::compile`
       is called) with an `actions` event binding whose `matcher` field is
       set to `"[invalid(regex"` (the same string used in
       `dispatch::matcher::tests::invalid_matcher_returns_false` and
       `compile_returns_none_for_invalid_input`).
    2. Invokes the loader path that produces a `RuntimeBinding`.
    3. Asserts that `binding.matcher().is_none()`.
    4. Builds a synthetic `EventMeta` (any tool event with `tool_name = Some("X")`
       is fine) and calls `crate::dispatch::matcher::matches(binding.matcher(), &meta)`.
    5. Asserts the result is `true` (binding fires unconditionally).

  Suggested test name: `invalid_matcher_in_config_compiles_to_unconditional_binding`.

  If the existing loader-tests pattern uses JSON fixtures under
  `lib/tests/fixtures`, prefer the in-code construction path to keep the test
  self-contained (it pins one piece of behavior and shouldn't depend on a
  fixture file).

- `claudine/lib/src/dispatch/matcher.rs`
  - **Optional but recommended**: tighten the doc comment on
    `RuntimeMatcher::compile` (around lines 45-53) to add a one-line cross-
    reference: `/// See `dispatch::loader::tests::invalid_matcher_in_config_compiles_to_unconditional_binding`
    for the end-to-end production-behavior assertion.` This gives a future
    maintainer the link the moment they touch this function.

### New tests

One: `invalid_matcher_in_config_compiles_to_unconditional_binding`.

### Verification commands

```bash
cargo test -p claudine dispatch::loader::tests::invalid_matcher_in_config_compiles_to_unconditional_binding
cargo test -p claudine dispatch::loader
cargo test -p claudine dispatch::matcher
```

### Exit criteria

- [ ] The new loader test exists and passes.
- [ ] Existing `dispatch::matcher` test count (17 per review-1) is unchanged.
- [ ] The doc cross-reference (if added) compiles cleanly under
      `cargo doc -p claudine --no-deps` (run in Phase 4).

---

## Phase 4 — Lint sweep and full verification

**Outcome:** `cargo clippy -p claudine` reports zero warnings, all `claudine`
tests pass, formatting is clean, and rustdoc has no broken links introduced by
the prior phases.

### Scope

This is the cross-cutting requirement from the task brief. The clippy sweep covers
*all* warnings in the `claudine` package area, regardless of whether the prior
three phases caused them — preexisting warnings in `claudine/lib` and `claudine/cli`
must be cleaned up here.

### Files touched

Determined by clippy output. Likely candidates based on the changes:

- `claudine/lib/src/dispatch/runner.rs` — new helper may pick up needless-borrow
  or redundant-clone warnings.
- `claudine/lib/src/dispatch/template.rs` — new test may pick up
  `clippy::uninlined_format_args` or `clippy::needless_raw_string_hashes`.
- `claudine/lib/src/dispatch/loader.rs` — same.
- Any preexisting clippy warnings the rust-developer surfaces during the sweep.

### Steps

1. Run `cargo build -p claudine` and `cargo build -p claudine --tests`. Both must
   succeed with no warnings.
2. Run `cargo clippy -p claudine --all-targets -- -D warnings`. Fix every
   warning, including warnings unrelated to phases 1–3. Do NOT add
   `#[allow(...)]` attributes — fix the underlying issue. The only acceptable
   `#[allow]` is one that already exists on `main` and is not displaced by
   these changes.
3. Run `cargo fmt -p claudine -- --check`. If it fails, run `cargo fmt -p claudine`
   and verify the diff is purely whitespace.
4. Run `cargo test -p claudine`. All tests must pass.
5. Run `cargo doc -p claudine --no-deps`. Must complete with no broken-link
   warnings.

### Verification commands

```bash
# Mandatory order: build first, then clippy, then test, then doc.
cargo build -p claudine --tests
cargo clippy -p claudine --all-targets -- -D warnings
cargo fmt -p claudine -- --check
cargo test -p claudine
cargo doc -p claudine --no-deps
```

### Exit criteria

- [ ] `cargo clippy -p claudine --all-targets -- -D warnings` exits 0.
- [ ] `cargo fmt -p claudine -- --check` exits 0.
- [ ] `cargo test -p claudine` reports 0 failures.
- [ ] `cargo doc -p claudine --no-deps` reports 0 warnings.
- [ ] No `#[allow(clippy::...)]` attributes added in phases 1–3 survive into
      the final tree (re-grep the diff against pre-phase-1 baseline to confirm).

---

## Cross-Phase Notes for the Executor

- **Targeted builds only.** The repo CLAUDE.md states: never `cargo build` at
  repo root; always `-p claudine`.
- **No subagent commits.** The executor implements and tests; commit decisions
  are the user's.
- **No `Co-Authored-By` trailer** on any commit message produced by the executor.
- **Git safety.** Do not `git reset` working tree files. The plan assumes
  the executor edits in place.
- **Test rigor.** All new tests are Level 1 (unit / integration in-crate),
  matching the review's Test Rigor Matrix. No Level 2 or Level 3 tests are
  required by this review.
