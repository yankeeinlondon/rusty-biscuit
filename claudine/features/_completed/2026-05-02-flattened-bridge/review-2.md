---
agent: claude
model: ""
ready: true
---

# Review 2: Unify the Hook `when` Lookup with `EventMetaExpressionLookup`

This is the second review on this feature. Review-1 (kimi_code) identified
three blockers: uncommitted Darkmatter changes, missing `when` documentation
in `configuring-actions.md`, and a weak `ctx` regression test plus missing
`tool_response.*` and fallback-syntax runner tests. All three are resolved.

This review focuses on the delta and on remaining risk.

## Summary

The feature delivers exactly what the spec describes:

- `flatten_event_meta_aliases` and `event_meta_to_json` are gone from the
  Claudine source (verified with a project-wide search; remaining hits are
  only in feature/plan/review documents).
- Hook `when` evaluation flows through `EventMetaConditionLookup` =
  `EventMetaExpressionLookup` + Darkmatter `CtxLookup`
  (`claudine/lib/src/dispatch/runner/mod.rs:78-114`).
- `CtxLookup` is publicly exported from
  `darkmatter::markdown::compose::expression::CtxLookup` and
  `ShortcutLookup` is refactored to embed it
  (`darkmatter/lib/src/markdown/compose/conditions.rs:235-273`).
- Skill docs (`SKILL.md`, `architecture.md`, `hook-actions.md`) describe
  the composite adapter and explicitly note the removal of the JSON
  flattening path.
- `cargo test -p claudine --lib dispatch::` passes 187 tests.
- `cargo test -p darkmatter --lib markdown::compose::expression::ctx`
  passes all 5 new tests.
- `cargo clippy -p claudine -p darkmatter --lib -- -D warnings` is clean.

Review-1 follow-ups verified resolved in this pass:

- **Darkmatter commits landed.** `git log` shows `455d9df2` (extract
  `CtxLookup`) and `0c46bad9` (add `Debug` derive + formatting).
- **`configuring-actions.md` documents `when`.** A new "Conditional
  Execution (`when`)" section with full path surface and two examples
  starts at line 234.
- **Weak `ctx` test strengthened.** `when_ctx_fields_do_not_require_precomputed_event_metadata`
  (`claudine/lib/src/dispatch/runner/mod.rs:909`) now asserts both the
  outer `Result` and the inner `Option<HookResponse>` and pins
  `decision == Some(Deny)`.
- **`tool_response.*` and fallback runner tests added.** See
  `when_tool_response_path_resolves` (line 1160) and
  `when_env_fallback_syntax_works` (line 1191).
- **Second `ctx.*` field test added.** `when_ctx_year_resolves` (line 1220)
  proves any captured `ctx.*` key flows through, not just `today`.
- **`EventMetaConditionLookup` derives `Debug`** (line 130, after
  `CtxLookup` was given `#[derive(Debug)]`).
- **`meta_json.rs` has a module doc comment** (line 1).

## Findings

### Low — Per-action `CtxLookup` allocation drops the cache between actions in the same binding

**Severity:** low
**Location:** `claudine/lib/src/dispatch/runner/mod.rs:78-114`,
`claudine/lib/src/dispatch/runner/mod.rs:135`

`evaluate_when` constructs a fresh `EventMetaConditionLookup` (and
therefore a fresh `CtxLookup` with its own `RefCell<HashMap>` cache and
captured-group set) on every call. `execute_actions` calls
`evaluate_when` once per action. If a binding has, say, three actions
that each reference `ctx.today`, the `DateTime` context group is
captured three times instead of once. The `CtxLookup` cache exists
specifically to avoid this, but its scope is wrong: it lives for one
action's evaluation rather than for the whole binding.

This is purely a perf/ergonomic concern — correctness is fine — and
captures are cheap for `DateTime`. But it is the kind of paper cut that
becomes meaningful once a heavier `ctx.*` group is introduced (e.g.
`Git` capture, which currently shells out).

**Recommended:** hoist the lookup creation up one level.

```rust
// in execute_actions, before the loop
let work_dir: PathBuf = meta
    .cwd
    .as_deref()
    .map(PathBuf::from)
    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
let lookup = EventMetaConditionLookup::new(meta, work_dir.as_path());

for (index, action) in actions.iter().enumerate() {
    match evaluate_when_with_lookup(action.when(), &lookup) { ... }
    ...
}
```

`evaluate_when_with_lookup(when, &lookup)` keeps the parse/evaluate
plumbing but reuses the cached context groups. The current
`evaluate_when(when, meta)` can stay as a thin wrapper for any
single-shot caller.

This is a follow-up, not a blocker.

### Low — `meta_json.rs` is now a single-utility module misnamed for its remaining role

**Severity:** low
**Location:** `claudine/lib/src/dispatch/runner/meta_json.rs`

After Phase 4 the file contains only `strip_nulls`, which has nothing
to do with `EventMeta` JSON conversion — it's a generic null-stripping
helper used by `report.rs`. The module name `meta_json` is now
misleading and the doc comment ("retained after flattening removal")
reads as a TODO.

**Recommended:** rename to `null_strip.rs`, or fold `strip_nulls` into
`report.rs` (its only consumer), or move it to a more general
`json_utils` location. Pure housekeeping; can be done in a separate
commit.

### Low — `strip_nulls` lost its tests during the cleanup

**Severity:** low
**Location:** `claudine/lib/src/dispatch/runner/meta_json.rs`

The test block that was deleted in commit `48ec7d69` was titled around
the alias surface, which is correct to remove. But it appears that any
historical `strip_nulls` tests went with it (no tests for that helper
exist anywhere now). `strip_nulls` has a hard depth limit
(`STRIP_NULLS_MAX_DEPTH = 64`) and recursive object/array handling that
is not exercised by any test in the tree.

This is pre-existing risk surface that the feature touched but did not
introduce. Not a blocker; flag it because the helper now lives in a
file with no tests at all.

**Recommended:** add 3-4 small unit tests:

- removes nulls at top level of an object,
- removes nulls inside arrays,
- preserves non-null leaves and nested structure,
- bottoms out cleanly past `STRIP_NULLS_MAX_DEPTH` without panicking.

### Low — `EventMetaExpressionLookup` short-circuits `ctx.*` to `None` in two places

**Severity:** low (informational)
**Location:** `claudine/lib/src/dispatch/expression.rs:92-94`,
`claudine/lib/src/dispatch/expression.rs:152-156`

`EventMetaExpressionLookup::get` explicitly returns `None` for
`ctx`/`ctx.*`, and `EventMetaConditionLookup::get` also short-circuits
the same prefix to its `CtxLookup` before calling `inner.get`. The
explicit short-circuit in the inner lookup is therefore redundant for
the composite path (`inner.get` would never be reached for `ctx.*`
anyway). The spec's §B notes this is a "small optimization, not a
correctness requirement" and keeps it for routing clarity. That's fine.

The minor risk is a future reader assuming the inner lookup will
delegate `ctx.*` somewhere if the composite is removed. The current
rustdoc on `EventMetaExpressionLookup` states the rule, so this is
just a flag — no action required.

## Test Rigor Assessment

This feature is pure expression-evaluation plumbing. The spec correctly
notes: "No Level 2 (terminal) or Level 3 (interactive) tests apply."
This review confirms that classification.

| User-observable requirement | Strongest test | Level | Adequate? |
|---|---|---|---|
| `tool_name == 'Bash'` resolves under `when` | `when_condition_true_executes_action_and_can_block` | L1 | ✓ |
| `git.branch == 'main'` resolves under `when` | `when_git_branch_matches_main_resolves_truthy` | L1 | ✓ |
| `hardware.cores > 8` numeric comparison | `when_hardware_cores_numeric_comparison` | L1 | ✓ |
| `git.is_dirty` boolean usability with `!` | `when_git_is_dirty_resolves_as_boolean` | L1 | ✓ |
| `env.MISSING \|\| "default"` fallback under `when` | `when_env_fallback_syntax_works` | L1 | ✓ |
| `ctx.today` resolves under `when` | `when_ctx_today_resolves` + `condition_lookup_ctx_today_resolves` | L1 | ✓ |
| `ctx.<other>` resolves under `when` | `when_ctx_year_resolves` + `condition_lookup_ctx_year_resolves` | L1 | ✓ |
| `ctx.*` unresolved in templates/matchers/harness | `ctx_namespace_is_unresolved` | L1 | ✓ |
| Falsy result skips action without selecting response | `when_condition_false_skips_call_action_and_no_blocking_response` | L1 | ✓ |
| Invalid expression warns + skips non-fatally | `when_invalid_expression_skips_action_non_fatally` | L1 | ✓ |
| Skipped `Call` cannot replace prior selected response | `when_skipped_call_does_not_replace_prior_selected_response` | L1 | ✓ |
| `tool_input.<nested>` resolves | `when_nested_tool_input_path` + expression unit tests | L1 | ✓ |
| `tool_response.<path>` resolves | `when_tool_response_path_resolves` | L1 | ✓ |
| `extra.<dot.path>` resolves | `when_extra_dot_path_resolves` + expression unit tests | L1 | ✓ |
| Composite parity with inner lookup for non-ctx | `condition_lookup_falls_through_to_inner` | L1 | ✓ |
| `CtxLookup` cache deduplicates captures | `ctx_lookup_caches_repeated_lookups` (darkmatter) | L1 | ✓ |
| `git == None` is falsy under `when` | `when_missing_git_block_is_falsy` | L1 | ✓ |

No requirement in the spec asserts user-observable terminal behaviour,
input-encoder behaviour, or any rendered output, so Level 2 and Level 3
do not apply. The feature is plumbing between two trait implementations
and a parser; Level 1 is the appropriate ceiling.

## Verification Checklist

- [x] `flatten_event_meta_aliases` and `event_meta_to_json` no longer exist in source
- [x] Darkmatter `CtxLookup` is committed (`455d9df2`, `0c46bad9`)
- [x] Hook `when`, templates, matchers, and harness validation derive non-`ctx` paths from `EventMetaExpressionLookup`
- [x] `ctx.*` resolves under hook `when` and remains unresolved elsewhere
- [x] All `dispatch::runner::tests::when*` pass (verified locally)
- [x] All `dispatch::expression::tests`, `dispatch::template::tests`, `dispatch::matcher::tests`, `harness::validate::tests::*` pass
- [x] `when_ctx_today_resolves`, `when_ctx_year_resolves`, `when_tool_response_path_resolves`, `when_env_fallback_syntax_works` are present and pass
- [x] `cargo clippy -p claudine -p darkmatter -- -D warnings` is clean
- [x] `claudine/docs/topics/configuring-actions.md` documents the `when` field with examples
- [x] Skill docs (`SKILL.md`, `architecture.md`, `hook-actions.md`) describe the composite adapter
- [x] `EventMetaConditionLookup` derives `Debug`

## Recommendation

**Mark this feature production-ready.**

All review-1 blockers are resolved, the implementation matches the
spec's target architecture exactly, the test surface covers every
parity invariant in spec §E end-to-end, and clippy is clean. The three
remaining findings are all "low" follow-ups: a perf nit on per-action
`CtxLookup` allocation, a housekeeping rename for `meta_json.rs`, and
adding tests for the orphaned `strip_nulls` helper. None of them gate
shipping; they belong in a future cleanup pass.
