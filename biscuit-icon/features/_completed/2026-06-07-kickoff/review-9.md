---
ready: false
agent: codex
model: ""
---

# Biscuit Icon Design Review

Iteration 9 closes the multi-prefix Iconify contract, visible truncation,
partial-fetch exit status, and completion-test retry findings from iteration 8.
The feature is not ready for production because the image Level 2 test can
report success without performing its assertion, and the new concurrent fetch
path has no reliable SQLite write-concurrency contract or test.

## Findings

### 1. High: The image Level 2 test silently passes when screenshot capture is unavailable

After confirming that WezTerm is available, the test treats either failed
window capture as a successful early return. Under the canonical
`just test-l2` recipe, missing screen-recording permission therefore produces a
passing test instead of the required hard failure. This recreates the core
verification gap from iteration 8 in a less visible form: nextest can report the
image fallback as passed even though no pixels were inspected.

The assertion did execute and pass on this review host, but the test's behavior
on a host without capture permission is still invalid for a required Level 2
gate.

Evidence:

- `cli/tests/level2_terminal.rs:178-183` returns successfully when the baseline
  capture is unavailable.
- `cli/tests/level2_terminal.rs:194-199` does the same when the post-render
  capture is unavailable.
- `cli/tests/level2_terminal.rs:146-147` gates only on WezTerm availability, not
  on the screenshot capability needed by the assertion.

Make screenshot capture part of the required resource check. When
`BISCUIT_TEST_LEVEL_REQUIRED=2`, an unavailable capture must fail the test; an
ordinary optional run may skip through the repository's standard level-gating
mechanism.

### 2. High: Concurrent body fetches race independent SQLite connections without lock handling

The CLI now runs ten `Icon::iconify_with` calls concurrently. Every call opens a
new connection for its cache read, executes `PRAGMA journal_mode = WAL`, then
opens another new connection for its write. The cache configures neither a busy
timeout nor serialized/batched writes. Concurrent successful HTTP responses can
therefore turn into `database is locked` cache failures even though the icons
were fetched correctly.

The large-catalog test does not exercise this contract: it mocks only three of
the 100 body requests and intentionally lets the other 97 fail. It asserts two
rows and the truncation notice but not successful completion or that every
successful response was cached. As a result, the test cannot distinguish the
intended network failures from cache-lock failures and does not verify the new
concurrency path.

Evidence:

- `cli/src/commands.rs:110-130` launches up to ten cache-writing tasks at once.
- `lib/src/icon.rs:102-121` opens separate connections for each read and write.
- `lib/src/cache/store.rs:44-48` changes journal mode on every `open_at` call.
- `lib/src/cache/store.rs:144-145` opens connections without a busy timeout.
- `cli/tests/cli.rs:148-196` provides 120 search hits but successful body mocks
  for only three of them and does not assert a successful exit.

Use one serialized cache writer, a transaction/batch API, or a documented busy
timeout and retry policy. Add an L1 test where at least one full concurrency
window returns successful bodies, the command exits zero, and every body is
present in the cache afterward.

### 3. Medium: The shared `test` recipe still includes non-Level-1 test tiers

The fix for running Level 2 tests in ordinary package tests was made in the
repo-wide `_test` recipe, but its filter excludes only `level2_`. The canonical
testing contract defines `test` as the full Level 1 suite, so `level3_`,
`browser_`, `real_`, and `slow_` tests must also be excluded. The `cargo test`
fallback has the same incomplete filter.

This change affects every package area using the shared recipe, not only
`biscuit-icon`, and can run resource-dependent or slow tests from an ordinary
`just test` invocation.

Evidence:

- `just/devops.just:165` excludes only `test(/level2_/)` from nextest.
- `just/devops.just:168` skips only names beginning with `level2_` in the cargo
  fallback.
- `.claude/skills/rust-testing/SKILL.md` defines the Level 1 filter as excluding
  Level 2, Level 3, browser, real-resource, and slow tests.

Apply the complete canonical Level 1 filter in the shared nextest recipe and
equivalent skips in the cargo fallback, or keep the change package-local if a
repo-wide correction is outside this feature's scope.

## Verification Levels

| Requirement | Strongest verification observed | Assessment |
|---|---|---|
| Domain enums, SVG assembly, styling, cache, and HTTP contracts | Level 1 | Passed |
| Default command, direct lookup, cache clear, completions, online merge, prefix filtering | Level 1 CLI subprocess | Passed |
| Bounded online listing and truncation notice | Level 1 CLI subprocess | Notice passed; successful concurrent fetch/cache behavior is not covered |
| Unicode glyph rendering | Level 2 tmux | Passed |
| Nerd Font glyph rendering | Level 2 tmux | Passed |
| Text fallback and multi-row listing | Level 2 tmux | Passed |
| Styled CLI errors | Level 2 tmux | Passed |
| Image-protocol fallback | Level 2 WezTerm | Passed on this host; gate can falsely pass without capture permission |
| OS keyboard/mouse behavior | Not applicable | No Level 3 requirement |

## Commands Run

- `just test`: library 95/95 passed; CLI 22/22 Level 1 tests passed and five
  Level 2 tests were excluded.
- `just test-l2`: all six Level 2 tests passed, including the image pixel
  assertion on WezTerm.
- `just lint`: passed for both crates.

The required `biscuit-icon` skill is absent from the authoritative local skill
catalog and repository skill directory. Package-specific guidance was therefore
derived from the specification, implementation, and repository instructions.
The `rust-testing` skill was used for the verification-level audit.
