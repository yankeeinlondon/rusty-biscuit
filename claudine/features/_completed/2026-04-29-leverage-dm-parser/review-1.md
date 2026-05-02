---
ready: false
agent: ""
model: ""
---

# Review 1

## Findings

### High: Hook action `when` does not support the shared event expression paths

Spec examples and the new docs promise that action `when` clauses can use the same event paths as templates and matchers, including `git.branch`, `git.is_dirty`, `os.*`, `hardware.*`, and `project.*`. The implementation does not do that. `evaluate_when()` serializes `EventMeta` to JSON and calls Darkmatter's shortcut evaluator directly ([runner.rs:91](../../lib/src/dispatch/runner.rs:91)), while templates and matchers use `EventMetaExpressionLookup` ([template.rs:422](../../lib/src/dispatch/template.rs:422), [matcher.rs:116](../../lib/src/dispatch/matcher.rs:116)). The shared lookup explicitly maps `git.branch`, `hardware.cores`, `project.language`, etc. ([expression.rs:181](../../lib/src/dispatch/expression.rs:181)); serialized `EventMeta` instead stores those under `env.git.branch`, `env.hardware.cores`, and so on.

Consequence: documented configs such as:

```json
{ "type": "notify", "message": "Deploying", "when": "git.branch == 'main' && !git.is_dirty" }
```

will evaluate against missing `git.*` paths and skip incorrectly. Current `when` tests cover `tool_name`, `provider`, `env.*`, and `ctx.*`, but not the grouped event paths required by the spec.

Verification level: strongest present is Level 1 unit coverage, but it is incomplete for this requirement. Level 1 is the appropriate level for expression semantics; add Level 1 tests for `when` with `git.branch`, `git.is_dirty`, `hardware.cores`, `project.language`, nested `tool_input`, and `extra.*`.

Suggested fix: evaluate `when` with the same parsed-expression path and `EventMetaExpressionLookup` used by matchers, while preserving Darkmatter `ctx.*` lazy capture. If `ctx.*` must stay available, use a composite lookup or enrich the JSON payload with the flattened aliases before calling the shortcut evaluator.

### High: Legacy environment fallback syntax appears to have been dropped while still documented

The pre-feature template contract supported `{{env.VAR | "fallback"}}`, and existing docs still advertise it in multiple places ([unified-events.md:632](../../docs/topics/unified-events.md:632), [unified-events.md:959](../../docs/topics/unified-events.md:959), [configuring-actions.md:282](../../docs/topics/configuring-actions.md:282)). The new renderer parses all interpolation through Darkmatter ([template.rs:447](../../lib/src/dispatch/template.rs:447)); Darkmatter's interpolation lexer rejects a single `|` and requires `||` for fallback. The feature added tests only for `||`, so existing configs using the documented pipe fallback will now preserve the whole token unchanged instead of rendering the fallback.

Verification level: strongest present is Level 1 template unit coverage, but it covers only the new `||` spelling. Level 1 is appropriate here; add a regression test for the legacy pipe syntax if compatibility is intended.

Suggested fix: update all docs plus migration notes. G

### Medium: Matcher invalid-input semantics are easy to misread and lack an end-to-end assertion

`RuntimeMatcher::compile()` returns `None` for invalid matcher strings and documents that this makes the binding fire unconditionally ([matcher.rs:45](../../lib/src/dispatch/matcher.rs:45)). That matches the loader path because `matcher::matches(None, meta)` returns true. However the test-only `matches_with_pattern()` helper returns false for the same invalid input, and the matcher unit test asserts that behavior. This split is not wrong by itself, but it obscures the production behavior that a malformed matcher broadens execution rather than disabling it.

Verification level: Level 1 unit tests exist, and Level 1 is appropriate. Add or strengthen a dispatch/loader test that proves an invalid configured matcher actually lets the action binding run, so future maintainers do not "fix" the helper behavior in the wrong direction.

## Test Rigor Matrix

| Requirement | Strongest verification present | Appropriate level | Status |
| --- | --- | --- | --- |
| Template interpolation supports simple fields, fallbacks, ternaries, comparisons, functions, unknown-token preservation, and single-brace rewrite | Level 1 unit tests in `dispatch::template` | Level 1 | Mostly adequate, except legacy pipe fallback regression above |
| Hook action `when` gates action execution and skipped `Call` cannot replace a response | Level 1 runner tests in `dispatch::runner::tests::when*` | Level 1 | Incomplete: missing grouped event-path coverage |
| Event binding matchers support expression mode and regex fallback | Level 1 matcher and loader tests | Level 1 | Adequate for core paths; invalid-input production behavior should be pinned end-to-end |
| Harness validation messages render expressions while preserving malformed/unknown tokens | Level 1 unit tests in `harness::validate` | Level 1 | Adequate |
| Terminal rendering, glyph width, SGR styling, modifier keys, hotkeys, paste, IME, mouse | Not applicable | None | No Level 2/3 requirement for this feature |

## Verification Run

- `cargo test -p claudine dispatch::template` passed: 28 tests.
- `cargo test -p claudine dispatch::runner::tests::when` passed: 7 tests.
- `cargo test -p claudine dispatch::matcher` passed: 17 tests.
- `cargo test -p claudine harness::validate::tests::render_template` passed: 8 tests.
- `cargo test -p claudine leverage-dm-parser -- --nocapture` passed but matched 0 tests, so it is not useful as a feature-level regression command.

## Production Readiness

Not ready. The feature's central promise is a shared expression language across templates, `when`, matchers, and validation messages. Templates and matchers use the shared `EventMeta` lookup, but `when` does not, causing documented `git.*`/context examples to fail. The legacy fallback syntax regression also risks breaking existing hook configs.
