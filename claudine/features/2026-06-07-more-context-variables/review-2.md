---
ready: false
agent: codex
model: ""
---

# Review: Claudine Context Catalogs - Iteration 2

## Findings

### High: expression and effect catalog parity is still one-directional

The expression contract test explicitly acknowledges that it cannot detect a new runtime dispatch arm without a descriptor ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/markdown/compose/expression/catalog.rs:387)). The command test only checks that every existing descriptor is rendered, so it does not close that reverse-direction gap. The effect test has the same structure: it iterates `EFFECT_DESCRIPTORS` and invokes the matched verb ([catalog.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/darkmatter/lib/src/effects/catalog.rs:144)); adding a new `EffectEngine` verb without adding a descriptor does not affect the test.

This still violates the source-of-truth requirement and the required exact-parity tests: adding a callable expression function, effect verb, or overload must fail until metadata is added. Define each runtime surface and descriptor from one registration declaration, or expose an authoritative runtime registry that can be compared for exact set equality. The context catalog now has a valid runtime-derived parity test; expressions and effects need the equivalent guarantee.

### High: Level 2 coverage does not verify every report across the required width regimes

The genuine tmux suite is a significant improvement, but it covers the default report at 78, 120, and 160 columns, expressions only at 120, and side effects only at 120 ([level2_context_capture.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/level2_context_capture.rs:141)). There is no Level 2 capture for `--values`, no report is captured at exactly 140 columns, and expressions/side-effects are not captured below or above the cap.

The spec requires every report below, at, and above 140ch and makes user-visible claims about wrapping, margins, styling, and the 40ch signature cap. Their strongest verification is therefore at the wrong level for several report/width combinations. Add Level 2 captures for all four reports at narrow, 140, and wider-than-140 widths, including right-margin accounting, descriptive-content retention, and first-column wrapping. Level 3 is not applicable.

### Medium: the no-side-effect command test does not cover policy or network probes

`context_side_effects_makes_no_filesystem_changes` snapshots only the command's working directory ([context_command.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/tests/context_command.rs:549)). It cannot detect `EffectEngine` construction, allowlist/configuration reads outside that directory, host discovery, or a network request that leaves the filesystem unchanged. Those are explicit prohibited behaviors in the specification.

The current renderer appears to consume only static descriptors ([context.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/claudine/claudine/cli/src/commands/context.rs:641)), but the required regression guard remains incomplete. Introduce an injectable effect/probe boundary or test instrumentation that records engine construction, policy checks, and network attempts, then assert zero calls.

## Verification Levels

- Catalog contents, report selection, flag exclusivity, wording, capture count, and descriptor rendering: Level 1 present. Expression/effect completeness oracles are not exact.
- Default report width, wrapping, glyphs, and inverse styling: Level 2 present at narrow/normal/wide widths, but not exactly 140ch.
- Expressions and side-effects rendering: Level 2 present only at 120ch.
- Values rendering: Level 1 only; Level 2 is absent.
- OS keyboard or mouse behavior: out of scope; Level 3 is not required.

## Verification

- `git diff --check HEAD` passed.
- Cargo, unit, command, and Level 2 test execution could not run because this host has no installed/default rustup toolchain (`rustup toolchain list` reports `no installed toolchains`).

## Verdict

Not ready for production. Iteration 2 resolves the prior alias, narrow-column, inline-code, capture-count, and genuine-terminal-test issues, but exact runtime/catalog parity and the required Level 2 report matrix remain incomplete.
