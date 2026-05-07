---
agent: open_code
model: ""
ready: true
---

# Review 5: Apple Terminal Integration Tests & Prose Graceful Degradation

## Summary

All four findings from review-4 have been resolved:

- **R4-Low (Level-2 link-fallback lacked sentinels)** — fixed. Both Level-2
  prose tests now bracket output with `__BT_START__` / `__BT_END__` and assert
  against the bounded slice (`cli/tests/level2_apple_terminal_prose.rs:75-93`,
  `154-169`).
- **R4-Low (`]` in markdown description unescaped)** — fixed. The parser now
  escapes literal `]` in link descriptions when emitting the markdown fallback
  (`prose.rs:1655-1660`), with a dedicated unit test
  (`link_markdown_fallback_escapes_bracket_in_description`,
  `prose.rs:2241-2255`).
- **R4-Low (`preserve_capabilities` footgun)** — fixed.
  `AppleTerminalHarness::new().preserve_capabilities(true)` omits the
  `FORCE_COLOR=1 CLICOLOR_FORCE=1` exports at spawn time
  (`biscuit-test-harness/src/apple_terminal.rs:115-118`), so future
  capability-degradation tests default to natural detection.
- **R4-Low (double-underline policy duplicated)** — fixed. The four-arm match
  is extracted into `degraded_double_underline_open(term)`
  (`prose.rs:291-298`), called from both `block_tag_to_escape` and
  `atomic_token_to_escape_with_term`.

Mechanical verification:
- `cargo check -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness` — clean.
- `cargo test -p biscuit-terminal --lib` — 1,302 passed, 2 ignored (pre-existing).
- `cargo test -p biscuit-terminal --test level1_apple_terminal_prose` — 6 passed.
- `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose` — 13 passed
  (3 Level-2 Apple Terminal tests, 10 common pane-geometry unit tests).
- `cargo test -p biscuit-test-harness` — 27 passed.
- `just -f biscuit-terminal/justfile test-l2` includes the target.

**Verdict: this feature is ready for production.** The findings below are
informational / cosmetic and do not block shipping.

## Verification Level Matrix

| Requirement | Strongest verification | Required level | Status |
|---|---|---|---|
| AC-1 — `<a href="…">label</a>` renders as `[label](url)` when OSC8 unsupported | Level-1 PTY + Level-2 Terminal.app + unit | Level 2 | ✅ |
| AC-2 — `<double-underline>` falls back to `\x1b[4m` when only straight supported | Level-1 PTY + Level-2 Terminal.app (capture strips ANSI) + unit | Level 2 | ✅ |
| AC-3 — `<double-underline>` emits no underline escapes when neither supported | Level-1 PTY + unit | Level 1 | ✅ |
| AC-4 — Level-1 PTY with `TERM_PROGRAM=Apple_Terminal` asserts exact byte sequences | `level1_apple_terminal_prose.rs` (6 tests) | Level 1 | ✅ |
| AC-5 — AppleScript harness spawns, captures, cleans up on Drop | `level2_apple_terminal_harness_lifecycle` | Level 2 | ✅ |
| AC-6 — Skip-clean off-macOS / in-CI | Harness unit tests + Level-2 early return | Level 1 / Level 2 | ✅ |

Every user-observable requirement has at minimum the verification level
appropriate to it. AC-1 and AC-2 each have both Level-1 byte-level coverage
and Level-2 real-display coverage; AC-3 is purely about the absence of escape
output and is fully covered at Level 1, which is the appropriate maximum given
Terminal.app's plain-text-only capture limitation.

## Findings

### Info — Level-2 double-underline test name is slightly misleading

`level2_apple_terminal_double_underline_plain_text_visible`
(`cli/tests/level2_apple_terminal_prose.rs:139`) is named as if it verifies
"plain text" output. However, Apple Terminal's actual capability profile is
`straight=true, double=false`, so the correct degradation path is to
`\x1b[4m` (straight underline), not plain text. Terminal.app's AppleScript
capture strips all ANSI bytes, so the visible result is indistinguishable from
plain text — the test can only assert "no garbage is visible". The straight-
underline byte-level assertion lives in the Level-1 PTY suite
(`apple_terminal_double_underline_degrades_to_straight`), which is the
appropriate place for it.

**No action required** — the test is correct for what Level-2 can observe,
but the name may confuse future readers into thinking the fallback is plain
text rather than straight underline.

### Info — `<uu>` alias for double-underline lacks dedicated degradation test

The block-tag alias `uu` (`<uu>…</uu>`) shares the exact same match arm as
`double-underline` in `block_tag_to_escape` (`prose.rs:427`) and is therefore
correctly covered by the degradation policy. The optimistic path is tested
(`test_underline_variants_block`, `prose.rs:1764`), but there is no explicit
capability-aware test for `uu`. Given the shared code path, the risk of a
regression is negligible.

**Suggested fix:** add a one-line assertion to
`test_double_underline_degrades_to_straight_when_only_straight_supported` (or
its no-underline counterpart) using `<uu>…</uu>` to lock the alias behaviour.

### Info — No test for nested styling inside `<a>` with markdown fallback

If a user writes `<a href="…"><red>click here</red></a>` on Apple Terminal,
the markdown fallback path emits `[\x1b[31mclick here\x1b[39m](url)`. The `]`
escaping logic (`prose.rs:1660`) runs after `parse_tokens_inner` processes the
inner content, so any SGR sequences are already embedded. The code comment
correctly notes that SGR escapes never contain `]`, making the replace safe.
However, there is no unit or PTY test that exercises this specific combination.

**Severity:** info — the reasoning is sound, but a regression test would
increase confidence.

## Positive Notes

- `degraded_double_underline_open` is a clean, well-documented helper that
  eliminates the duplication review-4 called out. The `TODO(apple-terminal-
  followup)` comment at `prose.rs:269` correctly scopes the follow-up work for
  `curly-underline`, `dotted-underline`, and `dashed-underline`.
- `preserve_capabilities(true)` is an ergonomic builder toggle that prevents
  future test authors from accidentally landing in the `Terminal::new_forced`
  path. The doc example on the method is a nice touch.
- The bracket-escaping implementation correctly targets only `]` (CommonMark
  requires escaping `]` inside link text, but `[` is legal), and the
  fingerprint-based detection (`open == "["` + `close` starts with `](` and
  ends with `)`) is precise enough to avoid false positives on unrelated tags.
- All 6 Level-1 PTY tests use the `---PROSE---` / `---END---` bounded-slice
  pattern with exact `assert_eq!` where appropriate, avoiding the command-echo
  false-positive risk.
- The `AppleTerminalHarness` unit-test suite is comprehensive: 17 tests cover
  escape logic, skip semantics, allocation policy, and the
  `preserve_capabilities` builder.
- Skip-clean discipline is consistently followed across all test files: no
  `#[ignore]` markers, every Level-2 test calls `available()` and returns OK
  with `skip_with_reason` when the harness cannot run.

## Recommendations (in priority order)

1. (Optional, cosmetic) Rename `level2_apple_terminal_double_underline_plain_text_visible`
   to `level2_apple_terminal_double_underline_degrades_visible` or add a
   clarifying doc comment that Apple Terminal's real path is straight-underline
   fallback, and "plain text" in the capture is an artifact of Terminal.app's
   ANSI stripping.
2. (Optional, confidence) Add a one-line `<uu>` alias assertion to an existing
   unit test to lock the alias degradation behaviour.
3. (Optional, confidence) Add a unit test for nested `<red>…</red>` inside an
   `<a>` tag with `osc_link_support=false` to prove the bracket escape is safe
   in the presence of embedded SGR sequences.

None of these are blockers. The feature delivers AC-1 through AC-6 with
appropriate verification at every level and all review-4 findings are resolved.
