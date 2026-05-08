---
agent: claude
model: ""
ready: true
---

# Review 4: Apple Terminal Integration Tests & Prose Graceful Degradation

## Summary

The four findings raised in review-3 have been resolved:

- **R3-Critical (compile blocker, stray `}` at `prose.rs:2442`)** — fixed.
  `cargo check -p biscuit-terminal` succeeds. The lib test suite is now
  mechanically verifiable: all 1,301 tests pass (2 ignored, both
  pre-existing).
- **R3-Medium (Level-2 double-underline lacked sentinels)** — fixed at
  `cli/tests/level2_apple_terminal_prose.rs:162-189`. The `bt prose`
  output is bracketed by `__BT_START__` / `__BT_END__` printf sentinels
  and the assertion runs over the bounded slice, eliminating the
  command-echo / prompt false-positive risk.
- **R3-Low (atomic-token PTY coverage gap)** — closed by
  `atomic_double_underline_no_underline_support_emits_plain_text` at
  `lib/tests/level1_apple_terminal_prose.rs:223-270`. The atomic form
  with both `straight=false` and `double=false` overrides now has a
  Level-1 PTY round-trip with strict `assert_eq!` on the bounded
  payload (rejects every `\x1b[…m` sequence including `\x1b[0m`).
- **R3-Info (curly/dotted/dashed follow-up TODO)** — present at
  `lib/src/components/prose.rs:375-381`, scoped explicitly to the
  Apple Terminal feature spec.

I verified the Level-2 suite against a real Terminal.app on this host:
all three tests (link fallback, double-underline plain text, harness
lifecycle) pass in 29.37 s.

I think this feature **is ready for production**. The remaining
findings below are cosmetic / ergonomic / edge-case and do not block
shipping.

## Verification Level Matrix

| Requirement | Strongest verification | Required level | Status |
|---|---|---|---|
| AC-1 — `<a href="…">label</a>` renders as `[label](url)` when OSC8 is unsupported | Level-1 PTY (`apple_terminal_link_falls_back_to_markdown`) + Level-2 Terminal.app (`level2_apple_terminal_link_fallback_visible`) | Level 2 | ✅ |
| AC-2 — `<double-underline>` falls back to `\x1b[4m` when only straight underline supported | Level-1 PTY (block + atomic) + Level-2 Terminal.app with sentinel bracketing | Level 2 | ✅ |
| AC-3 — `<double-underline>` emits no underline escapes when neither variant supported | Level-1 unit (`test_double_underline_suppressed…` / `atomic_double_underline_suppressed…`) + Level-1 PTY (block & atomic) | Level 1 | ✅ |
| AC-4 — Level-1 PTY with `TERM_PROGRAM=Apple_Terminal` asserts exact byte sequences | `level1_apple_terminal_prose.rs` (6 tests, all green) | Level 1 | ✅ |
| AC-5 — AppleScript harness spawns, captures, cleans up on Drop | `level2_apple_terminal_harness_lifecycle` — pre-spawn window-id snapshot, post-Drop poll, best-effort manual cleanup with diagnostic | Level 2 | ✅ |
| AC-6 — Skip-clean off-macOS / in-CI | Off-macOS: `available_is_false_off_macos` unit test. CI=1: `available_is_false_in_ci` unit test (macOS-only). All Level-2 tests early-return via `skip_with_reason` | Level 1 + Level 2 | ✅ |

No requirement asserting user-observable behaviour is missing the
verification level appropriate to it. AC-1, AC-2, and AC-5 each have
both Level-1 byte-level coverage (where applicable) and Level-2
real-display coverage; AC-3 is purely about the absence of escape
output and is fully covered at Level 1, which is the appropriate
maximum.

## Findings

### Low — Level-2 link-fallback test does not use sentinels

`level2_apple_terminal_link_fallback_visible`
(`cli/tests/level2_apple_terminal_prose.rs:87-132`) asserts that
`(https://example.com)` and `click here` appear in the captured frame.
Unlike the double-underline test it does not bracket the output with
`__BT_START__` / `__BT_END__` sentinels.

The risk is structurally identical to the one review-3 raised for
double-underline: if the shell echo of the command line survives
quoting (e.g. a future regex-pattern shell config that expands
`(https://example.com)` literally) or if a custom prompt happens to
contain the URL string, the assertion would pass on a degenerate
output. The probability is low because the URL is distinctive, but the
fix is mechanical and consistent with the pattern already used three
lines down for double-underline.

**Suggested fix:** wrap the link command with the same sentinel
pattern and assert against the bounded slice.

### Low — `<a>` markdown fallback does not escape `]` in description content

In `block_tag_to_escape("a", …)` the markdown fallback emits
`Cow::Borrowed("[")` open and `Cow::Owned(format!("]({})", resolved_href))`
close (`prose.rs:454-457`). If `description` contains a literal `]` —
e.g. `<a href="https://example.com">array[0]</a>` — the rendered
output is `[array[0]](https://example.com)`, which most markdown
renderers will parse as `[array[`/`0]](https://example.com)`. The bytes
on screen are still readable, but a downstream consumer that pipes the
output back through a markdown parser would mis-resolve the link.

This is out of scope for the spec's fixture set ("click here" /
"important text") but is a portable correctness papercut. A future
change should `.replace(']', "\\]")` in description content, or escape
both `[` and `]` per the CommonMark inline-link rules.

**Severity:** low — observed only with bracketed descriptions, which
no current test or production caller exercises.

### Low — `disable_color_forcing` is a workaround, not a fix

`AppleTerminalHarness::spawn_shell` unconditionally sets
`FORCE_COLOR=1 CLICOLOR_FORCE=1` in the spawned shell
(`biscuit-test-harness/src/apple_terminal.rs:209`). The Level-2 prose
tests then call `disable_color_forcing(&mut harness)` to undo this so
that `bt`'s `detect_terminal_honoring_force_color` does not collapse
into the `Terminal::new_forced` path which unconditionally enables
`osc_link_support` and `supports_italic` — defeating the very
graceful-degradation paths under test
(`cli/tests/level2_apple_terminal_prose.rs:50-74`).

The current arrangement works, but every future test of "real Apple
Terminal capability profile" must remember to call
`disable_color_forcing` after `spawn_shell`. Forgetting it produces a
silent false negative (the test passes against `Terminal::new_forced`,
not against the actual Apple Terminal capability profile).

**Suggested fix:** add a builder-style toggle to `AppleTerminalHarness`
(e.g. `AppleTerminalHarness::new().preserve_capabilities(true)`) that
omits the `FORCE_COLOR` / `CLICOLOR_FORCE` exports at spawn time.
Existing image tests can opt in to color forcing; capability tests
default to natural detection.

### Low — `block_tag_to_escape` and `atomic_token_to_escape_with_term` duplicate the double-underline policy

`prose.rs:407-416` (block tag) and `prose.rs:287-294` (atomic token)
each carry an independent four-arm match for double-underline
degradation. The arms are identical — `None` and `double=true` →
`\x1b[4:2m`, `straight=true` → `\x1b[4m`, otherwise drop — but they
share no helper. A future capability-aware extension (e.g. when
`curly-underline` becomes capability-aware per the existing TODO) will
have to replicate the same policy in two places again.

**Suggested fix:** extract a `degraded_underline_open(term, kind)`
helper that returns `Option<&'static str>` and call it from both sites.
This is purely a refactor; behaviour is already correct.

### Info — Lifecycle cleanup is best-effort against the "Don't close window" Terminal preference

`level2_apple_terminal_harness_lifecycle`
(`cli/tests/level2_apple_terminal_prose.rs:298-343`) explicitly
documents that Terminal.app's "When the shell exits → Don't close the
window" preference can leave the captured window visible after `Drop`.
The test logs a warning, falls back to a manual close, and continues.
This matches AC-5's contract ("Drop runs without manual intervention
by the test runner"), and the warning provides good diagnostics during
local development.

No action required — flagging only because future readers may
otherwise mistake the warning for a regression.

## Positive Notes

- The probe binary's terminal construction
  (`discovery_probe.rs:280-301`) bypasses `Terminal::new()`'s
  viuer-driven image detection cascade — this is the correct call,
  because that cascade sends terminal queries that block indefinitely
  inside a non-responding test PTY. The comment explains the trap.
- The atomic-token `Suppress` path (`prose.rs:1499-1505`) does not set
  `state.used_styles = true` even when the token is dropped — review-2's
  `\x1b[0m` leak fix from a prior review is preserved.
- The `BlockTagAction` enum (vs the old empty-string sentinel) is a
  clean expression of intent and makes the recursive `Suppress` path
  in `parse_tokens_inner` unambiguous (`prose.rs:1597-1608`).
- `applescript_escape` rejects forbidden bytes (CR, NUL, BEL, ESC,
  U+2028, U+2029) via `debug_assert!` and pre-allocates correctly for
  multi-line inputs. Six dedicated unit tests pin the byte contract.
- The lifecycle test's pre/post window-id diff is robust against
  focus-restoration races — substantially better than relying on
  `front window`.
- The Level-2 suite is genuinely Level-2: `bt` runs in a real
  Terminal.app, capabilities are negotiated through the actual
  detection cascade (after `disable_color_forcing` runs), and capture
  happens via the AppleScript scripting interface. The harness
  acknowledges (in module docs and a per-test comment) that
  Terminal.app cannot expose ANSI bytes — the negative byte-level
  assertions correctly live at Level 1.
- Skip-clean discipline is consistently followed: no `#[ignore]`
  markers, every Level-2 test calls `available()` and returns OK with
  `skip_with_reason("Terminal.app")` when the harness cannot run.

## Recommendations (in priority order)

1. **Add sentinel bracketing to the Level-2 link-fallback test** for
   structural consistency with the double-underline test (cheap,
   eliminates an unlikely but possible false positive).
2. **Add a `preserve_capabilities` opt-out to `AppleTerminalHarness`**
   so future capability-degradation tests cannot accidentally land in
   the `Terminal::new_forced` path.
3. **Extract the double-underline degradation policy into a helper**
   shared between `block_tag_to_escape` and
   `atomic_token_to_escape_with_term`. Pre-emptive groundwork for the
   curly/dotted/dashed follow-up TODO.
4. (Future, post-merge) Implement the curly/dotted/dashed TODO at
   `prose.rs:375-381` against `UnderlineSupport.{curly,dotted,dashed}`.

None of these are blockers. The feature delivers AC-1 through AC-6
with appropriate verification at every level.
