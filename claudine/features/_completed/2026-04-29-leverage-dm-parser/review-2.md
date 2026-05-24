---
ready: true
agent: ""
model: ""
---

# Review 2

## Findings

No production-blocking findings.

The prior review's issues appear addressed:

- Hook action `when` now evaluates the documented grouped paths (`git.*`, `hardware.*`, `project.*`, `tool_input.*`, `extra.*`) via flattened aliases before calling Darkmatter's condition shortcut.
- Legacy single-pipe env fallback is now explicitly documented as removed, and stale tokens are preserved verbatim instead of silently changing behavior.
- Invalid matcher production behavior is pinned through the loader/runtime path: invalid configured matchers compile to `None`, and a `None` matcher fires unconditionally.

## Non-Blocking Suggestions

1. Add one dispatch-level integration smoke test that loads a `ClaudineConfig` with a `when`-gated action, runs through `dispatch_canonical_with_runtime`, and proves the action is skipped or executed from the configured runtime binding. The current runner tests are strong for expression semantics, and loader tests cover compilation, but a single full pipeline test would guard the handoff between loader, matcher, dispatch, and runner.

2. Consider eventually sharing one lookup adapter between hook `when` and template/matcher evaluation. The current flattened-alias bridge is well tested, but it duplicates the path surface from `EventMetaExpressionLookup`; a composite lookup that preserves `ctx.*` support while delegating event paths to the shared adapter would reduce future drift risk.

3. The original spec's harness-message example mentions broader values such as `source_file`, `cwd`, frontmatter, and context variables. The implemented scope intentionally uses the existing narrow `build_vars` map. If those broader variables are still desired, track them as a follow-up rather than treating the current implementation as incomplete.

## Test Rigor Matrix

| Requirement | Strongest verification present | Appropriate level | Status |
| --- | --- | --- | --- |
| Dispatch template interpolation supports simple fields, env fallbacks with `||`, ternaries, comparisons, helper functions, unknown-token preservation, malformed-token preservation, and legacy single-brace rewrite | Level 1 unit tests in `dispatch::template` | Level 1 | Adequate |
| Legacy single-pipe env fallback is no longer supported and remains visible as an unchanged token | Level 1 unit test in `dispatch::template` plus docs updates | Level 1 | Adequate |
| Hook action `when` gates action execution, invalid/falsy conditions skip non-fatally, skipped `Call` cannot replace a prior response, and grouped event paths resolve consistently | Level 1 runner tests in `dispatch::runner::tests::when*` | Level 1 | Adequate |
| Event binding matchers support expression mode, regex fallback for legacy patterns, invalid matcher drop-to-unconditional production behavior, and helper functions | Level 1 matcher and loader tests | Level 1 | Adequate |
| Harness validation messages render Darkmatter interpolation expressions while preserving unknown and malformed tokens | Level 1 unit tests in `harness::validate::tests::render_template*` | Level 1 | Adequate |
| Terminal rendering, glyph widths, SGR styling, modifier keys, hotkeys, paste, IME, mouse, or terminal input encoder behavior | Not applicable | None | This feature does not add terminal-emulator or OS-keyboard behavior, so Level 2/3 tests are not required |

## Verification Run

- `cargo test -p claudine dispatch::template -- --nocapture` passed: 29 tests.
- `cargo test -p claudine dispatch::matcher -- --nocapture` passed: 17 tests.
- `cargo test -p claudine harness::validate::tests::render_template -- --nocapture` passed: 8 tests.
- `cargo test -p claudine dispatch::runner::tests::when -- --nocapture` passed: 14 tests.

## Production Readiness

Ready. Each user-observable requirement in this feature is expression evaluation or dispatch gating behavior, and Level 1 verification is the appropriate level for those semantics. No Level 2 or Level 3 terminal verification is required for this feature.
