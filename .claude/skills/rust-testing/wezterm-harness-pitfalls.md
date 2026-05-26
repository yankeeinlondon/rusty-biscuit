# WezTerm Harness Pitfalls

Notes on L2 tests that use `biscuit_test_harness::wezterm::WezTermHarness` to
capture `frame.raw` from a real WezTerm pane.

## SGR Re-emission Collapses

`harness.capture()` reads the pane via `wezterm cli get-text --escapes`, which
walks the cell grid and emits SGR **only on transitions**. This means:

- Contiguous same-attribute cells collapse into a single SGR span. The leading
  SGR may appear on a previous row and not re-appear on the next row, even
  when the CLI emitted a fresh SGR sequence per line.
- Truecolor SGR re-emits in **either** semicolon form (`\x1b[48;2;R;G;Bm`) **or**
  ITU colon form (`\x1b[48:2::R:G:Bm`) depending on terminfo and WezTerm
  version. The same test on the same machine has been observed switching
  between forms across runs.
- `\x1b[0m` in the source may come back as `\x1b[39m\x1b[49m` (separate fg
  and bg resets) or be elided entirely when the following cell shares the
  prior attributes.

## What This Breaks

**Per-line or per-frame byte-equality assertions across two captures.**

You will see panics where two captures are byte-identical in their CLI
output (verifiable with `script(1)`) but `frame.raw` differs by a leading
SGR span being elided on one capture. The left and right strings look "the
same shape" with only an SGR prefix or suffix missing.

Real example: comparing two `md --code-theme nord` renders where one had
frontmatter and one did not — the underlying `md` output was identical, but
WezTerm captured the leading `\x1b[48:2::46:52:64m` on one and not the other.

## Diagnostic First Step

Before changing the implementation, prove the CLI output is identical:

```bash
script -q /dev/null bash -c "./target/debug/md fixture_a.md --flags" 2>&1 \
  | xxd > /tmp/a.hex
script -q /dev/null bash -c "./target/debug/md fixture_b.md --flags" 2>&1 \
  | xxd > /tmp/b.hex
diff /tmp/a.hex /tmp/b.hex
```

If the hex diffs are identical (or differ only in echo/prompt prefix), the
bug is in the test, not the code.

## Recommended Assertion Pattern

Replace byte equality with semantic checks on the full `frame.raw` stream:

1. **Presence of expected SGR bytes** in both forms:
   ```rust
   let nord_kw_semi = "\x1b[38;2;129;161;193m";
   let nord_kw_colon = "\x1b[38:2::129:161:193m";
   assert!(
       frame.raw.contains(nord_kw_semi) || frame.raw.contains(nord_kw_colon),
       "expected nord keyword fg. raw={:?}", frame.raw
   );
   ```

2. **Absence of disallowed SGR bytes** (the rejected theme's signature color):
   ```rust
   let dracula_kw_semi = "\x1b[38;2;255;121;198m";
   let dracula_kw_colon = "\x1b[38:2::255:121:198m";
   assert!(
       !frame.raw.contains(dracula_kw_semi)
           && !frame.raw.contains(dracula_kw_colon),
       "frontmatter dracula color must not appear when CLI claims the slot. \
        raw={:?}", frame.raw
   );
   ```

3. **Ordering** when you need to prove a span wraps a region — see commit
   `be5d0409e` for the `red_open_at < alpha_at < beta_at` pattern with a
   no-reset-between assertion.

## Pick Sharp Witnesses

When asserting "theme X is in effect", panel background alone is weak —
themes can share `#2e3440` or close-but-not-identical bg colors. Prefer
**signature foreground colors** that differ unambiguously between the two
themes you're trying to discriminate (e.g. nord blue `#81a1c1` vs dracula
pink `#ff79c6` for the `fn`/`let` keywords in a rust snippet).

## Cross-References

- `darkmatter/cli/tests/level2_layout.rs` module-level doc — same pitfall
  documented inline next to the tests it affects.
- Commit `be5d0409e` "fix(darkmatter-cli): handle WezTerm SGR re-emission
  variants in tests" — the canonical fix pattern for ul/hyperlink colors.
