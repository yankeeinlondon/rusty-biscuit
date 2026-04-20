# Review 2: More Context Variables

## Findings

### 1. `current_dir()` failure is still silently swallowed

Severity: Medium

The technical design explicitly called out a failure mode for process CWD discovery:

- capture should still succeed
- sniff-derived fields should degrade to null
- a warning should be recorded

That is still not implemented.

In [`darkmatter/lib/src/markdown/compose/types.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs), `ComposeContext::capture()` does this:

```rust
let base_dir = std::env::current_dir().unwrap_or_default();
Self::capture_for_dir(&base_dir)
```

Problems:

- `current_dir()` failure does not generate any diagnostic
- the empty fallback path is not semantically the same as “CWD capture failed”
- the warning path added for partial runtime capture never sees this condition

Suggestion:

- handle `current_dir()` explicitly
- if it fails, construct a context with date/time/env populated, sniff-derived fields null, and push a `PartialRuntimeCapture { area: "cwd", ... }` diagnostic

### 2. `ComposeContext` equality no longer reflects its actual contents

Severity: Medium

`ComposeContext` is now a richer, map-backed type with capture diagnostics, but its `PartialEq` implementation still compares only the legacy scalar fields plus `env`.

In [`darkmatter/lib/src/markdown/compose/types.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs):

- `values` is the canonical backing store
- `capture_diagnostics` is part of the object’s observable state
- `PartialEq` ignores both

That means two contexts can compare equal even if they differ in:

- repo metadata
- package/package-area values
- docs-derived values
- OS/hardware values
- capture diagnostics

This was already a little loose before, but now that the map-backed context is the authoritative model and the context module is public, the equality semantics are misleading.

Suggestion:

- either derive `PartialEq`/`Eq` over the full struct
- or document very clearly that equality is only legacy-field compatibility equality

The first option is the better default.

## Coverage Gaps

### 1. The new CLI contract is still not covered end to end

The compose CLI tests still do not cover the feature’s most important behavioral contract:

- invalid scalar/array `ctx` fails by default
- `--allow-ctx-override` downgrades that failure to a warning
- collision warnings are emitted to stderr
- partial runtime capture warnings are emitted to stderr

I checked [`darkmatter/cli/tests/cli.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/cli.rs), and the compose test block still only covers the basic compose/state/set paths. There are no assertions for `allow-ctx-override`, invalid document `ctx`, or context-warning stderr output.

That leaves the user-visible contract under-protected even though the library-side pieces are now better aligned.

Suggestion:

- add dedicated CLI tests for:
  - scalar `ctx` without `--allow-ctx-override` -> non-zero exit, no composed output
  - scalar `ctx` with `--allow-ctx-override` -> success + warning on stderr
  - object `ctx` collision -> success + collision warning on stderr

## Verification Notes

I re-reviewed the follow-up changes in commit `f5b7426c` and confirmed that the earlier issues around:

- hard-failing invalid `ctx`
- preserving capture diagnostics
- excluding `ctx` from `effective_state_hash()`
- deep-merging nested `ctx`
- exact `docs_drift` path matching

have been addressed in code.

I also ran the compose-focused CLI subset:

```bash
cd darkmatter && cargo test -p darkmatter-cli test_compose -- --nocapture
```

Those tests passed, but they still do not cover the new `ctx`-specific CLI behavior described above.
