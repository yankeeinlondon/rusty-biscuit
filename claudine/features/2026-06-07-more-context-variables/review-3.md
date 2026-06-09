---
ready: false
agent: codex
model: ""
---

# Review: Claudine Context Catalogs - Iteration 3

## Findings

### High: narrow terminals render planner errors instead of catalog content

The shared renderer still passes tables to `Table::render` even when the table cannot fit the available width ([context_render.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context_render.rs:116)). The context fallback only removes pinned widths; it does not handle a failed unpinned plan ([context_render.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context_render.rs:165)). Expression and side-effect tables retain hard first-column widths at every terminal width ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:615), [context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:683)).

Running the current debug binary with `NO_COLOR=1 COLUMNS=40` makes all four reports print `Table could not be rendered...` diagnostics in place of tables. At `COLUMNS=60`, all three side-effect tables fail similarly. This violates the requirements that narrow output retain descriptive content, avoid intentional overflow, and not panic or degrade into renderer errors at supported positive widths.

The Level 2 tests do not catch this because they redefine "narrow" as 78, 100, 120, and 128 columns ([level2_context_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_context_capture.rs:29)). The strongest verification is therefore at the wrong cases and does not substantiate the user-visible narrow-terminal requirement. Add a layout fallback that succeeds at genuinely constrained widths, then add Level 2 captures around 40-60 columns for every report and assert catalog text is present and no planner diagnostic is rendered. Level 3 is not applicable.

### High: expression and effect completeness are still not exact runtime contracts

Expression parity deliberately collapses every overload to its canonical name ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/expression/catalog.rs:364)). The end-to-end test then invokes every descriptor as `name(0, 0)`, regardless of the descriptor's declared signature ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/expression/catalog.rs:391)). Adding or removing an overload descriptor cannot be detected as long as one function with that name remains dispatchable. This does not satisfy exact parity between callable signatures and descriptors.

The effect change compares descriptors against a new, separately maintained `EFFECT_VERBS` list ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/effects/catalog.rs:157)). That list is not the runtime method surface: adding a new public `EffectEngine` method without registering it leaves both compared sets unchanged, so the test still passes. The documentation's claim that the registry is authoritative over dispatchable verbs is therefore stronger than the implementation can enforce.

Define expression arity/overloads in the dispatch registration itself and derive descriptors or exact callable signatures from it. For effects, generate methods and descriptors from one declaration, or narrow the stated runtime surface to an actual dispatcher/registry used by invocation. Tests must fail for both missing names and missing/extra overloads.

### Medium: test-only observability has leaked into the production API and hot paths

The no-probe test adds public hidden counters to Darkmatter and increments atomics on every engine build and every `http_post` call ([mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/effects/mod.rs:20), [verbs.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/effects/verbs.rs:264)). It also publicly exports `EffectVerb` closures that mutate fixed sandbox filenames. These APIs exist only to support tests, add permanent synchronization work to production paths, and expose mutation-capable internals to downstream callers.

Keep instrumentation behind `cfg(test)` or a dedicated test feature, or test the metadata-only renderer through an injected capture/effect boundary. The current network counter also observes only `EffectEngine::http_post`; it would not detect a future direct network client in the report path.

## Verification Levels

- Catalog rendering, flag exclusivity, wording, single context capture, and row inclusion: Level 1 present.
- Styled output, glyphs, margins, wrapping, and width caps: Level 2 present for selected widths.
- Narrow-terminal behavior: Level 2 is present only at widths chosen to avoid the implementation's failure range. Required real-terminal verification at genuinely constrained widths is missing.
- OS keyboard or mouse behavior: out of scope; Level 3 is not required.

## Verification

- Inspected the specification, current working-tree diff, typed catalogs, dispatch registries, renderer, command tests, and Level 2 tmux tests.
- The existing `target/debug/claudine` binary reproduced planner diagnostics at 40 columns for all reports and at 60 columns for `--side-effects`.
- Cargo tests could not run because the host has no installed/default rustup toolchain (`rustup toolchain list` reports `no installed toolchains`).

## Verdict

Not ready for production. Iteration 3 expands the Level 2 matrix and improves runtime-name parity, but narrow terminal rendering is broken in the exact behavior the specification requires, and expression/effect completeness is still not enforced signature-for-signature from the true runtime surface.
