---
agent: open_code
model: ""
ready: false
---

# Review 3: Apple Terminal Integration Tests & Prose Graceful Degradation

## Summary

The prior review's (review-2) four findings have been addressed:

- AC-3's `\x1b[0m` leak is fixed: both the atomic-token suppress path (lines 1499-1507) and the block-tag `Suppress` path (lines 1590-1601) no longer set `state.used_styles = true`.
- AC-2's Level-2 test now uses `disable_color_forcing` + `wait_for_prompt` to prevent the command-echo false positive (though it still does not use sentinels — see M1).
- AC-5's lifecycle test now snapshots pre-spawn window IDs, diffs to find the harness window, and polls for post-Drop disappearance.
- The `FORCE_COLOR` / `CLICOLOR_FORCE` footgun in the harness is documented and callers manually unset the vars.

However, **the crate does not compile** due to a stray closing brace at `prose.rs:2442`. This is a hard blocker that prevents any test from running, which means none of the implementation or test changes in this worktree have been mechanically verified by this review.

I do **not** think this is production-ready.

## Verification Level Matrix

| Requirement | Strongest present verification | Required level | Status |
|---|---|---|---|
| AC-1: OSC8 unsupported renders `[description](url)` | Level 1 PTY + Level 2 Terminal.app | Level 2 | OK (subject to compile fix) |
| AC-2: double underline falls back to straight in Apple Terminal | Level 1 PTY + Level 2 Terminal.app (no sentinels) | Level 2 | OK with caveat (M1) |
| AC-3: no underline support emits plain text with zero escapes | Level 1 unit + Level 1 PTY | Level 1 | OK |
| AC-4: Level-1 PTY with `TERM_PROGRAM=Apple_Terminal` | Level 1 PTY | Level 1 | OK |
| AC-5: AppleScript harness spawns, captures, and cleans up on Drop | Level 2 Terminal.app (window-id diff + poll) | Level 2 | OK |
| AC-6: skip on CI / Terminal.app unavailable | Harness unit test + Level 2 early return | Level 1 / Level 2 | OK |

## Findings

### Critical: `biscuit-terminal` does not compile — stray `}` at `prose.rs:2442`

`prose.rs:2442` has an extra closing brace that has no matching opener:

```
error: unexpected closing delimiter: `}`
  --> biscuit-terminal/lib/src/components/prose.rs:2442:1
   |
1651 | mod tests {
   |           - this opening brace...
...
2441 | }
   | - ...matches this closing brace
2442 | }
   | ^ unexpected closing delimiter
```

`cargo check -p biscuit-terminal` fails. **No test in the entire crate can run until this is fixed.** The last line of the file (2442) must be deleted.

### Medium: AC-2 Level-2 test does not use sentinels to isolate rendered output

Review-2 recommended wrapping the `bt prose` output with unique sentinels (e.g. `printf '__BT_START__\n'; bt prose '...'; printf '\n__BT_END__\n'`) to prove the captured `important text` comes from `bt`'s stdout rather than the shell's echo of the typed command line.

The current implementation adds a `disable_color_forcing` helper and a `wait_for_prompt` call, which reduces the risk of the false positive but does not eliminate it: if the shell's prompt contains `important text` (however unlikely), or if `bt` crashes and the shell re-echoes the command line containing the literal string `important text`, the assertion still passes. The sentinel approach from review-2 is the robust solution.

Severity is medium rather than high because the double-underline command line itself does not contain the literal substring `important text` in a form that would survive shell quoting — the assertion is checking the rendered output, not the command echo. But a future test with a simpler fixture (e.g. `<double-underline>x</double-underline>`) would be vulnerable.

### Low: Missing Level-1 PTY test for atomic `{{double-underline}}` with no underline support

The Level-1 PTY suite (`level1_apple_terminal_prose.rs`) has:

- `apple_terminal_double_underline_atomic_token_degrades` — tests the Apple Terminal profile (straight supported, double unsupported)
- `no_underline_support_emits_plain_text` — tests the block tag `<double-underline>` with no underline support

But there is no Level-1 PTY test for `{{double-underline}}important text` with both straight and double underline disabled. The unit test `atomic_double_underline_suppressed_when_no_underline_support` covers this at the `parse_tokens` level, but the probe's PTY path is never exercised for this combination. The gap is minor because the underlying function (`atomic_token_to_escape_with_term`) is shared and unit-tested, but the Level-1 probe coverage is asymmetric.

### Low: `curly-underline`, `dotted-underline`, `dashed-underline` are not capability-aware

The spec focused on double-underline degradation, but the implementation also supports `<curly-underline>`, `<dotted-underline>`, and `<dashed-underline>` (both block and atomic forms). These always emit their respective SGR sequences (`\x1b[4:3m`, `\x1b[4:4m`, `\x1b[4:5m`) regardless of terminal capability. Apple Terminal's profile shows only `straight: yes` — all extended underline styles are unsupported. This is out of scope per the spec ("Out of scope: refactoring Prose to use the Style/Stylist system"), but worth noting as a future improvement since the `UnderlineSupport` struct already tracks `curly`, `dotted`, `dashed` fields.

### Info: TODO comment for Prose/Style convergence is present

The review-1 recommendation to add a TODO for the Prose/Style convergence is addressed at `prose.rs:370-374`. Good.

## Positive Notes

- The `BlockTagAction` enum introduced in review-2 (replacing the empty-string sentinel pattern) is clean and makes the suppress/wrap intent explicit.
- The harness unit test suite is comprehensive: 25 tests pass in `biscuit-test-harness`, including escape, quote, CI gate, off-macOS gate, Unicode rejection, and allocation policy checks.
- The `AppleTerminalHarness` avoids the Dock miniaturize animation by snapshotting and restoring the frontmost app — a thoughtful UX choice for developers running tests locally.
- The lifecycle test's window-ID diffing approach is more robust than relying on `front window` which is focus-dependent.
- The `applescript_escape` byte contract is well-documented and enforced with `debug_assert!` in debug builds.
- The `PROBE_FORCE_OSC8` override from review-1 now has a Level-1 PTY test (`probe_force_osc8_emits_osc_when_forced_on`), closing review-1 L2.

## Recommendations (in priority order)

1. **Delete the stray `}` at line 2442** — this is the only compile blocker.
2. **Add sentinels to the Level-2 double-underline test** to fully close the review-2 command-echo gap.
3. **Add a Level-1 PTY test** for `{{double-underline}}important text` with `PROBE_FORCE_UNDERLINE_STRAIGHT=false` and `PROBE_FORCE_UNDERLINE_DOUBLE=false` to round out the atomic-token no-underline coverage.
4. After the compile fix, run `cargo test -p biscuit-terminal --lib -- prose` and `cargo test -p biscuit-terminal --test level1_apple_terminal_prose` to verify the complete test suite passes.
