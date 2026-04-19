# Review: `start` / `success` / `blocked` / `failure` (Pass 4)

## Findings

### 1. Wrapper-level lifecycle integration still lacks end-to-end coverage

The implementation paths are now in place in [`claudine/cli/src/commands/wrap/composition.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L461) and [`claudine/cli/src/commands/wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L2489), but the CLI test suite still does not exercise the feature. `claudine/cli/tests/wrap_commands.rs` is 3259 lines long, and `rg -n 'start:|success:|blocked:|failure:|speak_first|lifecycle' claudine/cli/tests/wrap_commands.rs` returns no matches. Current coverage is almost entirely library-only in [`claudine/lib/src/composition/lifecycle.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/lifecycle.rs#L969).

That leaves the highest-risk behavior unprotected:

- `start.stderr` ordering relative to provider launch
- `blocked` vs `failure` classification in harness and non-harness paths
- inline closure failure classification
- retry / resume / redirect semantics
- `start` firing exactly once across harness retries

Suggested additions:

- direct compose: `start.stderr` before launch, `success.stderr` on exit `0`, `failure.stderr` on non-zero exit
- inline compose: `success.stderr` after closure, `failure.stderr` on invalid replacement or rewrite failure
- harness: `blocked.stderr` for pre-check / shell-audit exhaustion, `failure.stderr` for agent / inline-closure / post-check exhaustion
- retry semantics: pre-launch recovery should avoid `blocked`; post-launch exhaustion should produce `failure`; `start` should appear once

### 2. Same-change documentation work from the design is still missing

The tech design explicitly called for updating composition-facing docs and the library README, but [`claudine/docs/topics/composition.md`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/docs/topics/composition.md#L57) still documents only `prompt`, `last_updated`, `agent`, `policy`, and `blast_radius`. There is no user-facing schema or timing documentation for `start`, `success`, `blocked`, or `failure`.

That means the runtime feature exists, but it is still effectively undiscoverable unless someone reads the feature folder or source.

Suggested additions:

- frontmatter schema examples for all four lifecycle properties
- fixed stderr state mapping
- audio ordering rules for `speak` vs `speak_first`
- `start`-once semantics and `blocked` vs `failure` timing rules

### 3. The public lifecycle surface still exposes the deprecated foot-gun, and the new guard docs already drifted

[`claudine/lib/src/composition/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/mod.rs#L20) still re-exports deprecated `LifecycleRuntimeState` and `emit_lifecycle_signal`, so new callers can still bypass `LifecycleRunGuard` and recreate the exact classification bugs this feature was meant to prevent.

On top of that, [`claudine/lib/src/composition/lifecycle.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/lifecycle.rs#L225) still says `mark_provider_launched()` should be called only after `execute_harness_attempt(...)` returns `Ok`, but the wrapper now correctly marks launch from `child_spawned` before propagating post-spawn errors in [`claudine/cli/src/commands/wrap/composition.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L681) and [`claudine/cli/src/commands/wrap/mod.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/mod.rs#L2784). That doc comment is now actively misleading.

Suggested cleanup:

- stop re-exporting the deprecated APIs from `composition::mod`, or move them behind a clearly internal compatibility module
- update `mark_provider_launched()` docs to describe the real invariant: mark launch when child spawn succeeds, even if later I/O or rendering fails
- collapse the deprecated emission path onto the guard/emitter internals to remove duplicated lifecycle logic

## Additional Suggestions

- [`claudine/lib/src/messaging/send.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/messaging/send.rs#L103) uses `tokio::spawn` directly for lifecycle messaging, and [`claudine/lib/src/composition/lifecycle.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/composition/lifecycle.rs#L153) calls it without any runtime guard. The wrapper is safe today because it runs inside Tokio, but the public lifecycle API now assumes an async runtime for one output path and not the others. If the goal is that notification failures are never fatal, make the messaging path runtime-safe the same way `speak_blocking()` is.
- Positive-path messaging coverage is still light. [`claudine/lib/src/messaging/send.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/lib/src/messaging/send.rs#L450) only tests no-route / empty-text no-ops for `execute_resolved_message()`, not an actual resolved send path or lifecycle-emitter integration with a route.
- [`claudine/cli/src/commands/wrap/composition.rs`](/Users/ken/.claudine/worktrees/rusty-biscuit/feat-claudine-composition/claudine/cli/src/commands/wrap/composition.rs#L465) clones the full runtime config just to reach TTS + messaging. That is fine now, but if wrapper startup cost grows this is an easy place to narrow to a smaller lifecycle-specific settings load.

## Verification

- `cargo test -p claudine lifecycle --lib` passed locally.
