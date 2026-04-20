# Review: More Context Variables

## Findings

### 1. Invalid `ctx` does not fail by default

Severity: High

The technical design says a document-defined `ctx` that is not an object must be a hard compose error by default, with `--allow-ctx-override` acting as an explicit escape hatch.

The implementation does not do that.

In [`darkmatter/lib/src/markdown/compose/state.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/state.rs), the `Err` branch from `merge_ctx(...)`:

- replaces the document value with runtime `ctx`
- records `InvalidUserCtxReplaced`
- continues composition successfully

This means the default path behaves the same as `--allow-ctx-override`.

Observed behavior:

- `md compose` succeeds for a document with `ctx: not-an-object`
- it prints `warning[context]: Document ctx was not an object; replaced with runtime context`
- it exits `0`

That is a direct functional mismatch with the design.

### 2. Partial runtime capture diagnostics are dropped

Severity: Medium

The design calls for `PartialRuntimeCapture` diagnostics to surface as compose warnings when git/repo/docs/os/hardware capture only partially succeeds.

However, in [`darkmatter/lib/src/markdown/compose/types.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/types.rs), `ComposeContext::capture_for_dir()` calls `capture_runtime_context(base_dir)` and discards the returned diagnostics:

```rust
let (values, _diagnostics) = super::context::capture::capture_runtime_context(base_dir);
```

So although [`darkmatter/lib/src/markdown/compose/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs) contains logic to translate `PartialRuntimeCapture` into compose warnings, that path never receives those diagnostics.

Result:

- partial sniff failures are silently lost
- the warning behavior described in the design is not actually implemented end to end

### 3. Cache keys still include volatile context

Severity: Medium

The feature updated `context_hash()` to remove volatile values like:

- `now`
- `now_utc`
- `utc`
- `time`
- `timestamp`
- `memory_used`
- `memory_avail`

That change is incomplete.

In [`darkmatter/lib/src/markdown/compose/cache/hashing.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/cache/hashing.rs), `effective_state_hash()` hashes the full `EffectiveState.data()` map. The new implementation materializes `ctx` into `data["ctx"]`, so all of the volatile values are still included there.

That means:

- the persistent compose key still changes across runs because of volatile `ctx` values
- the new `context_hash()` exclusions do not actually stabilize the full cache key
- cache hit rates will be worse than intended

This is both a correctness-of-design issue and a performance issue.

### 4. `docs_drift` is implemented with substring matching instead of blast-radius matching

Severity: Medium

The technical design explicitly points at `sniff::filesystem::blast_radius` for drift detection.

In [`darkmatter/lib/src/markdown/compose/context/capture.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/context/capture.rs), `docs_drift` is computed by checking whether a dirty path string `contains()` each blast-radius pattern string.

That is weaker than the matching already implemented in sniff’s [`find_blast_radius_documents()`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/sniff/lib/src/filesystem/blast_radius.rs#L272), which operates on normalized repo-relative paths.

Risks:

- false negatives when normalized path equality would match but substring logic does not
- future divergence from sniff’s blast-radius semantics
- duplicated logic in Darkmatter where the design expected reuse of sniff helpers

### 5. Merge semantics are shallower than the design specifies

Severity: Low to Medium

The design specifies a deep merge of user `ctx` with runtime `ctx`, with runtime values winning on collisions.

In [`darkmatter/lib/src/markdown/compose/context/merge.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/context/merge.rs), the merge is only top-level:

- start with `user_obj.clone()`
- insert each runtime top-level key over it

Today this is mostly latent because runtime `ctx` is effectively flat. But it does not satisfy the stated design and will become a real bug if nested runtime objects are introduced later.

## Coverage Gaps

### 1. No CLI integration coverage for the new `ctx` failure/warning contract

The CLI tests in [`darkmatter/cli/tests/cli.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/cli/tests/cli.rs) cover basic compose flows, `--state`, and `--set`, but they do not cover:

- scalar/array `ctx` failing by default
- `--allow-ctx-override`
- warning emission for `ctx` collisions
- warning emission for invalid `ctx` replacement

This gap allowed the highest-severity regression above to ship while the full test suite remained green.

### 2. Very limited tests for the new capture surface

The tests in [`darkmatter/lib/src/markdown/compose/context/capture.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/context/capture.rs) only validate the date/time helper population and aliases.

They do not exercise:

- repo detection
- monorepo package/package-area scoping
- dirty/staged/untracked projections
- document scoping
- `docs_drift`
- `docs_skill`
- OS/hardware mapping
- failure/partial-capture diagnostics

Given the size of the new feature, this is light coverage.

### 3. No hash regression tests for the new context model

There are no direct tests showing that:

- `context_hash()` changes when a stable exposed `ctx` value changes
- volatile fields do not affect cache keys
- `effective_state_hash()` and `context_hash()` partition responsibility correctly after materializing `ctx`

That is exactly the sort of test that would have caught the cache instability issue.

### 4. No integration tests for context reuse across the transclusion graph

The design explicitly requires that context be captured once and reused across recursive compose. I did not find integration coverage proving that the same captured context is reused across parent and child transclusions for the new expanded context surface.

## Ergonomics and Performance Suggestions

### 1. Preserve capture diagnostics alongside the captured context

Right now the code splits:

- values into `ComposeContext`
- diagnostics into a separate return path that is immediately discarded

This is awkward and error-prone.

More ergonomic options:

- store diagnostics on `ComposeContext`
- or introduce a `CapturedContext { context, diagnostics }` wrapper

Either approach makes it much harder to silently lose warnings.

### 2. Remove materialized `ctx` from `effective_state_hash()`

If `ctx` remains materialized into `EffectiveState.data`, hashing the full data map is the wrong abstraction boundary.

Better options:

- exclude `ctx` from `effective_state_hash()`
- or hash a pre-materialized state representation
- or keep all context hashing exclusively in `context_hash()`

That would make the cache model easier to reason about and eliminate volatile-key churn.

### 3. Reuse sniff helpers for changed-path and blast-radius projections

The design already pointed at `sniff::filesystem::blast_radius::collect_changed_paths(...)`.

Using sniff helpers more directly would improve:

- correctness
- consistency with the rest of the monorepo
- maintainability

The current code duplicates projection logic for:

- dirty/staged package membership
- `docs_drift`

Those are good candidates for consolidation.

### 4. Implement the documented deep merge now or explicitly narrow the design

The current merge behavior is top-level overwrite. That can be acceptable only if the feature contract says `ctx` is flat and will remain flat.

If nested `ctx` is part of the intended model, the merge should be made truly deep now instead of waiting for nested runtime values to appear.

## Verification

I ran:

```bash
cd darkmatter && just test
```

All library and CLI tests passed.

I also directly verified that a document with scalar `ctx` composes successfully without `--allow-ctx-override`, which confirms the default-behavior regression described above.
