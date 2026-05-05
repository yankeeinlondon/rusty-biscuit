# Implementation Plan: Review-3 Fixes for Apple Terminal Integration

This plan addresses every recommendation in
`biscuit-terminal/features/2026-05-02-apple-terminal/review-3.md`.

Review-3 found that the prior review's fixes (review-2) all landed correctly,
but the crate stopped compiling because of a stray `}` at the end of
`prose.rs`. Three smaller follow-ups round out the test matrix:

1. Critical compile blocker (stray `}` at `prose.rs:2442`).
2. Sentinel-bounded Level-2 assertion for AC-2 (close the command-echo loophole).
3. Missing Level-1 PTY case for atomic `{{double-underline}}` with no underline support.
4. Documentation/scoping note for `curly`/`dotted`/`dashed` underline.
5. Final verification (build, test, clippy).

The plan is intentionally split into small phases so each can be executed by a
rust-developer subagent in isolation. Phase 1 is a strict prerequisite for
every other phase (nothing else can be tested until the crate compiles).

Working directory for all commands: the rusty-biscuit worktree root
(`/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/`).

> Workspace gotcha: never run `cargo build` / `cargo test` at the repo root
> without `-p`. All commands below are scoped with `-p`.

---

## Phase 1 — Fix the compile blocker (stray `}` at prose.rs:2442)

**Severity:** Critical. Hard blocker. Nothing else compiles until this is gone.

**Source of finding:** review-3 "Critical: `biscuit-terminal` does not compile —
stray `}` at `prose.rs:2442`" (review-3.md lines 35-52).

**Verified state:** `prose.rs` is currently 2442 lines. Line 2441 is the
matching close of `mod tests`; line 2442 is an extra unmatched `}`.

### Files to modify

- `biscuit-terminal/lib/src/components/prose.rs`

### Exact change

Delete line 2442 (the stray `}`). After the change, the file ends with the
closing brace of `mod tests {` on what is currently line 2441 followed by a
single trailing newline.

The simplest mechanical edit: open the file, confirm the last two lines are
`}\n}\n`, delete the final `}\n` so the file ends with a single `}\n`.

### Acceptance criteria

- `cargo check -p biscuit-terminal` exits 0 with no errors.
- `cargo build -p biscuit-terminal -p biscuit-terminal-cli` exits 0.
- File length decreases by exactly one line.

### Verification command

```sh
cargo check -p biscuit-terminal
cargo build -p biscuit-terminal -p biscuit-terminal-cli
```

---

## Phase 2 — AC-2: sentinel-bounded Level-2 double-underline assertion

**Severity:** Medium. Closes the residual command-echo loophole flagged in
review-3 "Medium: AC-2 Level-2 test does not use sentinels to isolate rendered
output" (review-3.md lines 54-60).

**Why it still matters even after review-2's fixes:** `disable_color_forcing`
+ `wait_for_prompt` shrink the false-positive surface but do not eliminate it.
Sentinels make the assertion robust against future fixture choices and against
shell prompt content.

**Prerequisite:** Phase 1 complete (crate must compile).

### Files to modify

- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`

### Exact changes

Locate `level2_apple_terminal_double_underline_plain_text_visible` (the test
that today sends `bt prose '<double-underline>important text</double-underline>'`
and asserts `frame.plain.contains("important text")`).

1. Replace the `send_bt_command` (or equivalent) call with a sentinel-wrapped
   shell pipeline. Send the bytes directly via `harness.send_text(...)` so
   shell builtins (`printf`) work:

   ```rust
   harness
       .send_text(b"printf '__BT_START__\\n'; bt prose '<double-underline>important text</double-underline>'; printf '\\n__BT_END__\\n'\n")
       .expect("send_text failed");
   harness.settle();
   ```

   Use whatever `settle` / `wait_for_prompt` helper the existing tests use.
   Match the surrounding style (the test file already calls similar helpers).

2. After capture, slice the captured plain text to only the region between the
   sentinels:

   ```rust
   let bounded = frame
       .plain
       .split("__BT_START__\n")
       .nth(1)
       .and_then(|s| s.split("\n__BT_END__").next())
       .unwrap_or("");
   ```

3. Replace the existing positive/negative assertions with sentinel-scoped
   ones, plus a non-empty guard so a `bt` crash fails loudly:

   ```rust
   assert!(
       !bounded.is_empty(),
       "sentinel-bounded output is empty — bt prose likely crashed or emitted nothing.\n\
        full capture:\n{}",
       frame.plain,
   );
   assert!(
       bounded.contains("important text"),
       "expected rendered `important text` between sentinels.\nbounded:\n{}",
       bounded,
   );
   assert!(
       !bounded.contains("[4:2m"),
       "literal `[4:2m` fragment visible in rendered output.\nbounded:\n{}",
       bounded,
   );
   assert!(
       !bounded.contains("\u{1b}[4:2m"),
       "raw double-underline SGR visible in rendered output.\nbounded:\n{}",
       bounded,
   );
   ```

4. Keep all existing harness setup (`disable_color_forcing` was already
   removed in review-2 work; do not re-introduce it).

### Acceptance criteria

- The test compiles and passes on macOS with Terminal.app available.
- The test still skips automatically on non-macOS / `CI=1` (skip path
  unchanged).
- Inserting a deliberate panic into the start of `bt prose` (manual
  smoke-check, not committed) makes the new empty-slice assertion fail —
  i.e., the assertion is sensitive to "command produced nothing".

### Verification command

```sh
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose \
    level2_apple_terminal_double_underline_plain_text_visible -- --nocapture
```

---

## Phase 3 — AC-3: add missing Level-1 PTY test for atomic `{{double-underline}}` with no underline support

**Severity:** Low. Symmetry/coverage gap flagged in review-3 "Low: Missing
Level-1 PTY test for atomic `{{double-underline}}` with no underline support"
(review-3.md lines 62-69).

**Why:** The block-tag form is exercised end-to-end via the PTY probe by
`no_underline_support_emits_plain_text`, but the atomic-token form
(`{{double-underline}}important text`) is only covered by the in-process unit
test `atomic_double_underline_suppressed_when_no_underline_support`. This phase
adds the PTY-level case so the probe path is symmetric across both syntaxes.

**Prerequisite:** Phase 1 complete.

### Files to modify

- `biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs`

### Exact changes

1. Add a new `#[test]` function modeled on
   `no_underline_support_emits_plain_text` and
   `apple_terminal_double_underline_atomic_token_degrades`. Suggested name:

   ```rust
   #[test]
   fn atomic_double_underline_no_underline_support_emits_plain_text() { ... }
   ```

2. Spawn the probe with the no-underline profile. Mirror the env-var pattern
   already used by `no_underline_support_emits_plain_text`:

   - `TERM_PROGRAM=Apple_Terminal` (keep parity with the rest of the file).
   - `PROBE_FORCE_UNDERLINE_STRAIGHT=false`
   - `PROBE_FORCE_UNDERLINE_DOUBLE=false`
   - Any other env vars that `no_underline_support_emits_plain_text`
     currently sets — copy them verbatim so the only delta is the prose
     fixture syntax.

3. Use the atomic-token fixture as the prose input:

   ```text
   {{double-underline}}important text
   ```

   (Match whatever input mechanism the surrounding tests use — the existing
   `apple_terminal_double_underline_atomic_token_degrades` test already feeds
   atomic-token input through the probe; clone its plumbing.)

4. Assertions:

   - The slice between `---PROSE---\n` and `\n---END---` equals exactly
     `"important text"` (use `assert_eq!`, not `contains`).
   - `!output.contains("\x1b[")` — no SGR escape of any kind.
   - Explicitly: `!output.contains("\x1b[4:2m")` and
     `!output.contains("\x1b[4m")` and `!output.contains("\x1b[0m")`.

### Acceptance criteria

- New test compiles and passes.
- Existing tests in `level1_apple_terminal_prose.rs` still pass unchanged.

### Verification command

```sh
cargo test -p biscuit-terminal --test level1_apple_terminal_prose
```

The output must include the new test name and a pass.

---

## Phase 4 — Document the `curly`/`dotted`/`dashed` underline scope gap

**Severity:** Info / forward-looking. Flagged in review-3 "Low:
`curly-underline`, `dotted-underline`, `dashed-underline` are not
capability-aware" (review-3.md lines 71-73).

The spec explicitly scopes graceful degradation to OSC8 + double-underline.
Curly/dotted/dashed are out of scope for this feature, but the gap should be
visible in the source so a future contributor doesn't have to rediscover it.

**Prerequisite:** Phase 1 complete.

### Files to modify

- `biscuit-terminal/lib/src/components/prose.rs`
- `biscuit-terminal/features/2026-05-02-apple-terminal/spec.md` (small
  addendum)

### Exact changes

1. In `prose.rs`, find the existing TODO comment for the Prose/Style
   convergence (around line 370-374, per review-3). Add a sibling TODO
   immediately below it:

   ```rust
   // TODO(apple-terminal-followup): make `curly-underline`, `dotted-underline`,
   // and `dashed-underline` capability-aware in the same way `double-underline`
   // is. `UnderlineSupport` already exposes `curly`, `dotted`, and `dashed`
   // booleans; the atomic and block tag handlers should consult them and fall
   // back to single underline (or plain text) when unsupported. Scoped out of
   // the 2026-05-02 Apple Terminal feature — see
   // features/2026-05-02-apple-terminal/spec.md.
   ```

2. In `spec.md`, append a short subsection at the end of the "Out of scope"
   area (after the existing "refactoring Prose to use the Style/Stylist
   system" note):

   ```markdown
   > **Also out of scope:** `<curly-underline>`, `<dotted-underline>`, and
   > `<dashed-underline>` graceful degradation. These tags currently emit
   > their SGR sequences unconditionally. `UnderlineSupport` already tracks
   > the relevant capability bits; making the tags capability-aware is
   > tracked as a follow-up TODO in `prose.rs`.
   ```

   Do not change any existing acceptance criteria or fixtures.

### Acceptance criteria

- `cargo check -p biscuit-terminal` still passes (TODO is a comment only).
- The new TODO is searchable by `grep -n 'apple-terminal-followup' biscuit-terminal/lib/src/components/prose.rs`.
- The spec addendum is present and does not contradict any existing fixture.

### Verification command

```sh
cargo check -p biscuit-terminal
grep -n 'apple-terminal-followup' biscuit-terminal/lib/src/components/prose.rs
```

---

## Phase 5 — Final verification (build, test, clippy)

**Prerequisite:** Phases 1-4 complete.

This phase is purely mechanical: run the full set of relevant test targets
and lint with warnings-as-errors. It exists as a discrete phase so that, if
anything regresses, the failing command is captured cleanly without rolling
prior phases together.

### Commands to run (all must succeed)

```sh
# 1. Library + CLI unit / integration tests
cargo test -p biscuit-terminal -p biscuit-terminal-cli

# 2. Level-1 PTY tests explicitly (covered by #1, but called out for clarity)
cargo test -p biscuit-terminal --test level1_apple_terminal_prose

# 3. Harness tests
cargo test -p biscuit-test-harness

# 4. Level-2 real-terminal tests (macOS host only; will skip on CI / Linux)
cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose -- --nocapture

# 5. Lint with warnings as errors
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli --all-targets -- -D warnings
```

### Acceptance criteria

- Every command above exits 0.
- Test counts in `biscuit-terminal --test level1_apple_terminal_prose` are
  one higher than before (Phase 3 adds one test).
- No new clippy warnings or errors.
- No new compiler warnings.

### If `cargo clippy` flags new warnings

Before silencing with `#[allow(...)]`, attempt a real fix. Only suppress
when the lint disagrees with project convention or the construct is
intentional and idiomatic for the surrounding code. Document any
suppression with an inline comment explaining the rationale.

---

## Phase Dependency Graph

```
Phase 1 (compile fix)
  ├── Phase 2 (sentinel Level-2 test)
  ├── Phase 3 (atomic Level-1 test)
  └── Phase 4 (TODO + spec addendum)
        └── Phase 5 (full verification)
```

Phases 2, 3, 4 are independent of one another and can be executed in any
order or in parallel by separate subagents, as long as Phase 1 has landed
first. Phase 5 must run last.

---

## Risks & Open Questions

1. **Phase 2 sentinel pipeline depends on shell semantics.** If the harness
   shell is not POSIX-compatible (the harness defaults to the user's login
   shell on macOS, typically `zsh` or `bash`), `printf '__BT_START__\n'; ...`
   should still work, but the implementer should sanity-check by running the
   pipeline interactively in a Terminal.app window once before committing.
2. **Phase 3 env-var names** (`PROBE_FORCE_UNDERLINE_STRAIGHT`,
   `PROBE_FORCE_UNDERLINE_DOUBLE`) are taken from review-3's recommendation 3.
   The implementer should grep the existing PTY probe code to confirm the
   exact names used by `no_underline_support_emits_plain_text` and copy them
   verbatim — do not invent new env-var names.
3. **Phase 5 Level-2 test cannot run in CI.** This is by design (per AC-6 and
   the spec's Risks section). On a non-macOS host or with `CI=1`, the test
   harness early-returns; the assertion that "all commands exit 0" still
   holds because the test is skipped, not failed.
4. **No production code path is changing in this plan.** Every behavior fix
   already landed in review-2's plan. This plan only restores the build,
   tightens one Level-2 assertion, adds one missing Level-1 test, and notes
   a known scope gap. If a reviewer expects new degradation logic, that
   expectation is misplaced — review-3 explicitly confirmed the behavior is
   correct subject to the compile fix.

## Files Touched (summary)

| Phase | File | Nature |
|---|---|---|
| 1 | `biscuit-terminal/lib/src/components/prose.rs` | delete 1 line |
| 2 | `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs` | swap one test body |
| 3 | `biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs` | add one test fn |
| 4 | `biscuit-terminal/lib/src/components/prose.rs` | add TODO comment |
| 4 | `biscuit-terminal/features/2026-05-02-apple-terminal/spec.md` | add scope note |
| 5 | (none — verification only) | n/a |
