---
source_review: review-1.md
source_spec: spec.md
created: 2026-05-02
phases: 7
status: ready
---

# Review-Driven Implementation Plan: L1/L2 Test Hardening

This plan addresses every finding in `review-1.md`. Phases are ordered so
that each phase ends in a clean `cargo test` / `cargo clippy` /
`cargo fmt --check` state for the affected crates. Phases 2–6 each replace
weak assertions with the strongest verifiable contract the underlying
terminal CLI exposes; Phase 7 closes the documented trait/spec drift and
sweeps lint/format.

> **Convention.** Every phase ends with a verification block. All
> `cargo` invocations are package-targeted (per repo CLAUDE.md and global
> memory feedback). `cargo test`/`clippy`/`fmt --check` MUST pass for the
> affected crates before declaring the phase complete.

Repo paths are relative to `/Users/ken/.claudine/worktrees/rusty-biscuit/terminal`.

---

## Assumptions and defaults (no clarifying questions per session contract)

1. **WezTerm SGR failure root cause.** Empirical evidence (review §High-1)
   shows `wezterm cli get-text --escapes` returned plain `x` with no SGR
   in the running developer environment. Two non-exclusive hypotheses
   explain this:
   1. The shell spawned via `wezterm cli spawn -- $SHELL -l` inherits an
      env where `bt`'s color detection (`Terminal::new()` →
      `color_depth()`) decides on `ColorDepth::None` because `TERM`/
      `COLORTERM` aren't set, or because stdout is being captured in a
      way that defeats `is_tty()`.
   2. `wezterm cli get-text --escapes` only returns the *terminal-state*
      buffer with SGR re-emitted from cell attributes; if the cell-state
      no longer carries a non-default attribute by the time we capture,
      we will not see the SGR bytes.
   The plan attacks this by **forcing color** in the spawned shell env
   (`FORCE_COLOR=1`, `CLICOLOR_FORCE=1`, and a known-good `TERM` /
   `COLORTERM`), and as a backstop **adding a CLI flag** `bt prose
   --force-color` (or honoring `FORCE_COLOR`) that bypasses detection.
   Default chosen: honor the existing convention `FORCE_COLOR=1` (used
   by `clicolors`, `colored`, and most ecosystem crates) and add one
   small detection-bypass branch in `commands.rs::render_prose`.
   Documented in Phase 2.
2. **Sentinel placement assertion strategy** (review §High-3). We assert
   that the sentinel row index in the captured pane is *strictly greater
   than* the row range occupied by the rendered image, derived from the
   `--debug` output's reported `image_rows` and `cursor BEFORE` row.
   Phase 5 documents the parsing.
3. **Diagram subcommand coverage** (review §High-4). The full subcommand
   list per spec §5.4 is: `flowchart`, `pie-chart`, `bar-chart`,
   `line-chart`, `quadrant`, `git-graph`, `timeline`, `state-diagram`,
   `erd`, `graph-expression`. Phase 4 adds **one** Level-2 test per
   subcommand asserting on rendered protocol bytes (Kitty harness →
   strong APC assertion) plus an `iTerm2`-protocol assertion on the
   WezTerm harness path for those that already have a WezTerm test.
4. **`TerminalHarness::spawn` adaptation** (review §High-5). Default
   chosen: keep the current trait method as a low-level escape hatch
   but rename it on the type implementations so that the *trait method*
   delegates to `spawn_shell` by default. This preserves backward
   compatibility for any direct callers (none expected outside tests
   themselves) while making the trait contract honor the shell model.
   Documented in Phase 7.
5. **Layout and unicode tests** (review §Medium). Phase 6 strengthens
   assertions by computing expected row counts from terminal-side
   capture geometry (`wezterm cli get-text` already returns column-
   aligned text) and comparing exact column boundaries.
6. **NO_COLOR test is currently OK** (review matrix), but its assertion
   harness is brittle. Phase 2 hardens it as a free side-effect.
7. **Lint/format sweep target list:** `biscuit-terminal` (lib + cli),
   `biscuit-test-harness`. `biscuit-tui-cli` is touched only in Phase 7
   if the trait change ripples; otherwise out of scope.

---

## Phase 1 — Stabilize the WezTerm SGR baseline

**Objective.** Make `level2_prose_emits_sgr_in_real_terminal` (and the
NO_COLOR sibling) pass deterministically on a developer host with WezTerm
available. Without this, every later phase is shadowed by a known
failing test.

**Review items addressed.**
- §High-1 "Level-2 suite fails with real WezTerm available"
  (`level2_prose_styling.rs:203` and the SGR check itself).

### Files to modify

- `biscuit-test-harness/src/wezterm.rs` — extend `spawn_shell` to set
  color-forcing env vars on the spawned shell.
- `biscuit-test-harness/src/kitty.rs` — same env propagation for parity.
- `biscuit-test-harness/src/tmux.rs` — same env propagation.
- `biscuit-terminal/cli/src/commands.rs::render_prose` — honor
  `FORCE_COLOR=1` by constructing a `Terminal` whose `color_depth` is
  `ColorDepth::TrueColor` and `is_tty=true`, bypassing detection.
- `biscuit-terminal/lib/src/terminal.rs` — add a small helper
  `Terminal::new_forced()` that returns the forced-color variant
  (used by `commands.rs`). Keep `new()` semantics untouched.
- `biscuit-terminal/cli/tests/level2_prose_styling.rs` — adjust
  `assert_no_sgr_red` to filter on the `bt`-output region using line
  isolation rather than substring proximity.

### Implementation steps

1. **Extend the shell env.** In each harness's `spawn_shell` (and
   `spawn` after Phase 7 unifies them), pass:
   - `FORCE_COLOR=1`
   - `CLICOLOR_FORCE=1`
   - `TERM=xterm-256color` (do not override an explicit caller value)
   - `COLORTERM=truecolor`
   Use `cmd.env(...)` on the *outer* `wezterm cli spawn` Command so the
   spawned shell inherits them. For the WezTerm/Kitty cases where the
   `wezterm cli spawn` invocation forks the shell, env on the outer
   command propagates to the inner shell process.
2. **Honor `FORCE_COLOR`.** In `cli/src/commands.rs::render_prose`,
   replace `let term = Terminal::new();` with:
   ```rust
   let term = if std::env::var_os("FORCE_COLOR").is_some()
       || std::env::var_os("CLICOLOR_FORCE").is_some()
   {
       Terminal::new_forced()
   } else {
       Terminal::new()
   };
   ```
   Apply the same pattern to any *other* prose-style render paths
   (`render_padleft`, `render_padright`, `render_blockquote`,
   `render_unordered_list`, `render_columns`) — these already use
   `Terminal::new()` and will all be exercised by Level-2 tests.
3. **Add `Terminal::new_forced`.** In `lib/src/terminal.rs`, add a
   `pub fn new_forced() -> Terminal` that constructs a terminal with
   detected `app`/`os` but forces `color_depth = TrueColor`, `is_tty
   = true`, `osc_link_support = true`, `supports_italic = true`. This
   keeps it distinct from `new_optimistic` (which hard-codes width).
4. **Harden `assert_no_sgr_red`.** Replace the existing `lines()`
   scan with a regex-free state machine: locate the line whose plain
   form matches `^.*NO_COLOR=1 bt prose .*$`, then walk forward up to
   `N=10` lines or until the next `\$ `/`% `/`# ` prompt suffix. Within
   that window, assert that **no** `\x1b[3[01]m` / `\x1b[9[01]m`
   sequences are present. This precisely targets the `bt` output even
   when the shell prompt itself is colored (e.g. starship).
5. **Add a lib-level unit test** for `Terminal::new_forced` covering
   the forced fields.

### Test additions / changes

- New unit test `lib/src/terminal.rs::tests::new_forced_returns_truecolor_tty`.
- Modified `level2_prose_styling.rs::level2_prose_emits_sgr_in_real_terminal`:
  no signature change; adjust the settle delay to use
  `wait_for_prompt` instead of a flat sleep, then capture twice with a
  100 ms gap and assert SGR is present in *either* capture (defends
  against a one-shot WezTerm get-text race).
- Modified `level2_prose_styling.rs::level2_no_color_strips_sgr_in_real_terminal`:
  rewrite the tail of the assertion to use the new line-window logic.

### Verification

```bash
cargo build -p biscuit-terminal-cli --bin bt
cargo test  -p biscuit-test-harness
cargo test  -p biscuit-terminal --lib terminal
cargo test  -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness --tests -- -D warnings
cargo fmt    -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness -- --check
```

### Risks / open questions

- If the failure mode is hypothesis (1.ii) (cell-state SGR no longer
  rendered by the time we capture), forcing color in the env will not
  help. Mitigation: the double-capture-with-gap logic in step 5
  defends against this; if it still fails, fall back to capturing
  `wezterm cli get-text --escapes` mid-render via a TUI sentinel that
  pauses with `read -n 1`. Document if reached.
- `Terminal::new_forced` must not change behavior of the lib's
  existing detection-based examples.

---

## Phase 2 — Replace debug-text image assertions with protocol-byte assertions

**Objective.** Make every Level-2 image test fail when image bytes are
absent, when CUD math is wrong, or when scroll compensation does not
visibly place text below the image — instead of just checking debug
strings.

**Review items addressed.**
- §High-2 "WezTerm image tests do not verify user-visible image
  rendering" (`level2_image.rs:60`, `:119`, `:155`).

### Files to modify

- `biscuit-terminal/cli/tests/level2_image.rs` — rewrite assertions for
  `level2_image_renders_in_wezterm`,
  `level2_image_scroll_compensation_at_bottom_margin`,
  `level2_warp_uses_floor_rounding`, plus add a Kitty render-row test.
- `biscuit-test-harness/src/wezterm.rs` — add a tiny helper
  `pub fn pane_size(&self) -> io::Result<(u32 /*rows*/, u32 /*cols*/)>`
  that wraps `wezterm cli list --format json` (filtered by pane id) so
  tests can read the actual pane geometry.
- (No CLI/lib changes required.)

### Implementation steps

1. **iTerm2 protocol assertion (WezTerm).** Replace the
   `--- image debug ---` / `Wezterm` substring check with a regex/
   substring assertion that `frame.raw` contains `\x1b]1337;File=`
   *and* a `:` payload separator. WezTerm passes iTerm2 graphics
   through to its scrollback when sourced from the spawned binary, and
   `get-text --escapes` re-emits the OSC. If WezTerm's get-text has
   *converted* the image into spacer cells, fall back to the second
   strategy below.
2. **Pane-row assertion (fallback strategy).** After `bt image`,
   capture twice. Compute `image_rows_observed = post_capture_rows -
   pre_capture_rows - 1` (the −1 accounts for the appended sentinel
   line). Assert `image_rows_observed > 0` and that the row count
   matches `ceil(image_pixel_height / cell_height_px)` derived from
   the pane size. For the WezTerm test we accept either the OSC
   assertion *or* the pane-row assertion succeeding; failure of both
   fails the test with both diagnostic paths included in the error.
3. **Scroll compensation: prove the user-visible effect.** Instead of
   asserting on the `SCROLL needed` debug string:
   - Spawn a small pane: configure WezTerm to launch with `-x 80 -y
     24` via `wezterm cli spawn --new-window -- --config 'ROWS=24'`
     OR position the cursor on row 22 (current behavior) with
     `tput cup 22 0`.
   - Send `bt image fixtures/tiny.png\n`.
   - Send `printf '\\nSENTINEL_AFTER_SCROLL\\n'` immediately after.
   - Capture the pane. Assert that the **row containing
     `SENTINEL_AFTER_SCROLL` is greater than 23 from the top of the
     pre-image cursor**, i.e. the image was placed and the sentinel
     was pushed to the row immediately following the image — not
     overlapping with it. Use the `position_cursor` row (22) and the
     known image height (1px → 1 cell) plus scroll-compensation `\n`
     to compute the expected sentinel row exactly.
4. **Warp `floor` rounding: prove the row count.** Replace the
   `Warp` + `floor=` substring check with: render `tiny.png` at a
   pixel size that yields a non-integer row count (use a 13×13 px
   fixture so 13 / cell_height yields a fractional value). Then
   capture twice in a Warp-spoofed env. Assert that
   `image_rows_observed == floor(13 / cell_height_px)`. Add an
   identical companion test in the default (ceil) path that asserts
   `ceil`. Add the new fixture `fixtures/13x13.png` (an opaque red
   square) to `cli/tests/fixtures/`.
5. **`level2_image_meta_to_stderr` is OK** but tighten: assert the
   stderr JSON in capture has both `cache_hit` and `render_time_ms`
   keys *and* that the same capture contains image-protocol bytes
   (so we know meta did not suppress rendering).
6. **Kitty row math test.** Add `level2_image_kitty_row_advance` that
   asserts the post-image cursor sits exactly `image_rows + 1` below
   the pre-image `tput cup` row. Uses the same pane-row strategy as
   step 2.

### Test additions / changes

- New fixture `biscuit-terminal/cli/tests/fixtures/13x13.png`
  (rendered as a flat-color PNG; documented in a sibling
  `fixtures/README.md`).
- New helper module `biscuit-terminal/cli/tests/common/pane_geometry.rs`
  (local to the cli/tests tree) exposing
  `fn pane_rows(harness: &mut WezTermHarness) -> u32` and
  `fn find_row_of(plain: &str, needle: &str) -> Option<usize>`.
- All four existing tests in `level2_image.rs` keep their names but
  their assertions are rewritten.
- Two new tests added: `level2_image_default_uses_ceil_rounding`,
  `level2_image_kitty_row_advance`.

### Verification

```bash
cargo test  -p biscuit-terminal-cli --test level2_image -- --nocapture
cargo clippy -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt   -p biscuit-terminal-cli -- --check
```

### Risks / open questions

- WezTerm's `get-text --escapes` may or may not preserve OSC-1337
  bytes. If it does not, the test relies on pane-row math; document
  the chosen branch with an inline comment.
- The 13×13 fixture's actual cell rounding depends on the host's font
  metrics. The test computes expected rows from the harness-reported
  cell height, so the assertion is portable.

---

## Phase 3 — Cursor placement: prove sentinel sits below the rendered image

**Objective.** Replace the existence-only sentinel check with a
geometry assertion: `row(sentinel) > row(image_top) + image_rows`.

**Review items addressed.**
- §High-3 "Cursor placement test does not prove the sentinel lands
  below the rendered image" (`level2_cursor_and_hygiene.rs:72`).

### Files to modify

- `biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs` — rewrite
  `level2_cursor_lands_below_rendered_image`.
- `biscuit-terminal/cli/tests/common/pane_geometry.rs` — reuse
  `find_row_of` helper from Phase 2.

### Implementation steps

1. Rewrite `level2_cursor_lands_below_rendered_image`:
   - `clear` the screen, `tput cup 5 0` to put us at row 5.
   - Run `bt image --debug fixtures/tiny.png`. Parse the debug output
     for `image_rows=N` and `cursor BEFORE: row=R`.
   - Send `printf 'SENTINEL_BELOW_IMAGE\\n'`.
   - Capture pane.
   - Compute `expected_min_row = R + N` (image_rows includes the row
     advance the renderer emits).
   - Use `find_row_of(plain, "SENTINEL_BELOW_IMAGE")` and assert it
     is `>= expected_min_row` AND `<= expected_min_row + 2` (allow up
     to two rows of slack for shell echo and prompt redraw).
2. If `--debug` parsing fails (no `image_rows=` line), the test fails
   loudly with the captured debug payload included.
3. Add a negative-control assertion: scan rows `R..R+N` and assert
   `SENTINEL_BELOW_IMAGE` does NOT appear there (would catch the
   off-by-one regression flagged by the reviewer).

### Test additions / changes

- Replace the two existing assertions with the geometric one
  described above. Keep the test name unchanged.
- Add a small parsing helper `parse_debug_image_rows(plain: &str) ->
  Option<u32>` and `parse_debug_cursor_before(plain: &str) ->
  Option<u32>` near the top of the test file.

### Verification

```bash
cargo test  -p biscuit-terminal-cli --test level2_cursor_and_hygiene level2_cursor_lands_below_rendered_image -- --nocapture
cargo clippy -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt   -p biscuit-terminal-cli -- --check
```

### Risks / open questions

- Debug-output format must remain stable for parsing. If a future
  change reformats `image_rows=` / `cursor BEFORE: row=`, the test
  fails and the developer is forced to update both. Acceptable.

---

## Phase 4 — Diagram Level-2 coverage to spec scope, with byte-level assertions

**Objective.** Add Level-2 tests for every diagram subcommand named in
spec §5.4, and replace metadata-JSON / cache-filename assertions with
image-protocol byte assertions (or `\`\`\`mermaid` fallback under tmux).

**Review items addressed.**
- §High-4 "Diagram Level-2 coverage is far below the acceptance
  criteria" (`level2_diagrams.rs:44`, `:100`, `:123`).

### Files to modify

- `biscuit-terminal/cli/tests/level2_diagrams.rs` — major rewrite.

### Implementation steps

1. **Build a parameterized helper** within the test file:
   ```rust
   struct DiagramCase {
       name: &'static str,        // test fn suffix, e.g. "flowchart"
       cmd:  &'static str,        // bt subcommand, e.g. "flowchart"
       arg:  &'static str,        // mermaid source / data
   }

   const CASES: &[DiagramCase] = &[
       DiagramCase { name: "flowchart",        cmd: "flowchart",        arg: "\"A --> B\"" },
       DiagramCase { name: "pie_chart",        cmd: "pie-chart",        arg: "\"A: 1\" \"B: 2\"" },
       DiagramCase { name: "bar_chart",        cmd: "bar-chart",        arg: "\"A,1\" \"B,2\"" },
       DiagramCase { name: "line_chart",       cmd: "line-chart",       arg: "\"1,2,3,4\"" },
       DiagramCase { name: "quadrant",         cmd: "quadrant",         arg: "\"P1: 0.4 0.6\"" },
       DiagramCase { name: "git_graph",        cmd: "git-graph",        arg: "\"main: A B C\"" },
       DiagramCase { name: "timeline",         cmd: "timeline",         arg: "\"2024: launch\"" },
       DiagramCase { name: "state_diagram",    cmd: "state-diagram",    arg: "\"[*] --> Open\"" },
       DiagramCase { name: "erd",              cmd: "erd",              arg: "\"User ||--o{ Order\"" },
       DiagramCase { name: "graph_expression", cmd: "graph-expression", arg: "\"f(x) = x^2\"" },
   ];
   ```
   For each case generate **two** Level-2 test functions via macro:
   - `level2_<name>_renders_in_kitty` — Kitty harness, asserts
     `frame.raw.contains("\x1b_G")`. Kitty preserves APC graphics in
     `kitty @ get-text --extent first_cmd_output`, so this is the
     strong-evidence path.
   - `level2_<name>_renders_in_wezterm` — WezTerm harness, asserts
     `frame.raw.contains("\x1b]1337;File=")` OR (fallback) the
     pane-row math from Phase 2 indicates a non-zero image height.
   Use a `paste!{}`-free pattern: handwritten `#[test]` shells calling
   a helper `assert_diagram_renders(case, harness_kind)`. The cost of
   ~20 explicit `#[test]` items is acceptable and keeps the file
   readable.
2. **`level2_diagram_width_respects_pane_columns`** rewrite. Use
   `kitty @ get-text` to capture the rendered pane, then use the
   `r=R, c=C` fields inside the Kitty APC sequence to verify column
   width. If WezTerm-only, fall back to **pane-column math**: after
   rendering, assert that the topmost image row's spacer-cell width
   equals the pane's col count × 0.5 (within ±1 cell). The
   pane-column count is obtained via `wezterm cli list --format json`.
3. **`level2_inverse_flag_changes_background_in_capture`** rewrite.
   Replace cache-filename comparison with **rendered-byte hash**
   comparison: capture both pane states; for each, slice out the
   image protocol payload (everything between `\x1b_G` and `\x1b\\`
   for Kitty, or between `\x1b]1337;File=` and `\x07` for iTerm2);
   sha256-hash each payload; assert `hash(default) != hash(inverse)`.
   This guarantees the *image bytes themselves* differ, not metadata.
4. **`level2_diagram_fallback_when_no_image_protocol`** is OK.
   Tighten by also asserting that `frame.raw` contains zero
   `\x1b_G` and zero `\x1b]1337;File=` (true negative coverage).
5. **`level2_flowchart_renders_in_wezterm` and
   `level2_pie_chart_renders_in_kitty`** existing tests are subsumed
   by the parameterized matrix — remove or keep as aliases.

### Test additions / changes

- New tests (counting):
  - 10 × `level2_<diagram>_renders_in_kitty`
  - 10 × `level2_<diagram>_renders_in_wezterm`
  - Rewritten `level2_diagram_width_respects_pane_columns`
  - Rewritten `level2_inverse_flag_changes_background_in_capture`
  - Tightened `level2_diagram_fallback_when_no_image_protocol`
- Add `sha2` to `[dev-dependencies]` of `biscuit-terminal-cli` if not
  already present (for the inverse-hash test). Check first; the lib
  may already pull it transitively.

### Verification

```bash
cargo test  -p biscuit-terminal-cli --test level2_diagrams -- --nocapture
cargo clippy -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt   -p biscuit-terminal-cli -- --check
```

### Risks / open questions

- 20 new tests at ~5–10 s each in Kitty/WezTerm → +2 min runtime.
  Spec §8 budgeted for this. CI continues to skip cleanly.
- Some diagrams (`quadrant`, `graph-expression`) may have stricter
  input-format requirements; the `arg` strings above are best-effort
  and will need verification when the test compiles. If a subcommand
  rejects its argument, the test fails fast with the CLI's own error
  message and we adjust the `arg`.
- `kitty @ get-text` output preserves APC sequences only with the
  `--extent` option; ensure `KittyHarness::capture` already requests
  the right extent (review file before relying on it).

---

## Phase 5 — Layout / unicode tests: real geometric assertions

**Objective.** Strengthen `level2_columns_word_wrap_in_pane` and
`level2_dir_command_unicode_widths_in_capture` so that they fail when
columns misalign or wrap rows are wrong, not just when text is absent.

**Review items addressed.**
- §Medium "Layout and unicode-width tests are too weak"
  (`level2_prose_styling.rs:185`, `level2_cursor_and_hygiene.rs:154`).

### Files to modify

- `biscuit-terminal/cli/tests/level2_prose_styling.rs` — rewrite
  `level2_columns_word_wrap_in_pane`, augment
  `level2_pad_columns_respect_actual_pane_width`.
- `biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs` — rewrite
  `level2_dir_command_unicode_widths_in_capture`.
- (No CLI/lib changes required.)

### Implementation steps

1. **`level2_columns_word_wrap_in_pane`.** Spawn a WezTerm pane and
   capture its column count via the `wezterm cli list --format json`
   helper from Phase 2. Construct an input string of length
   `cols + 5` made of one continuous word (no spaces). Assert:
   - The captured plain text contains the input word characters in
     order across at most 2 consecutive rows (catches "wrap at exact
     boundary" off-by-one).
   - The first wrapped row has length ≤ `cols`.
   - The continuation row's first non-space char is the `(cols+1)`-th
     character of the input.
2. **`level2_pad_columns_respect_actual_pane_width`** stays correct
   but harden: also assert that the row containing the `x` has length
   exactly 30 in `frame.plain` (after `trim_end`).
3. **`level2_dir_command_unicode_widths_in_capture`.** Use the
   pre-existing `unicode_dir` fixture. Assert:
   - For each filename row, locate the column index where the
     filename begins (after the tree-glyph prefix).
   - That index must be **identical** across all three rows
     (regular.txt, 中文文件.txt, emoji_🎉.txt).
   - Each row's tree-glyph prefix uses one of `├──`, `└──`, `│`.
   - Use `unicode-width` crate (already a workspace dep — confirm
     in `cli/Cargo.toml`'s `[dev-dependencies]` or add) to compute
     expected column positions of any subsequent metadata.

### Test additions / changes

- Both tests rewritten in place; names preserved.
- Add `unicode-width` to `biscuit-terminal/cli` `[dev-dependencies]`
  if not already present.

### Verification

```bash
cargo test  -p biscuit-terminal-cli --test level2_prose_styling level2_columns_word_wrap_in_pane -- --nocapture
cargo test  -p biscuit-terminal-cli --test level2_prose_styling level2_pad_columns_respect_actual_pane_width -- --nocapture
cargo test  -p biscuit-terminal-cli --test level2_cursor_and_hygiene level2_dir_command_unicode_widths_in_capture -- --nocapture
cargo clippy -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt   -p biscuit-terminal-cli -- --check
```

### Risks / open questions

- WezTerm's `get-text` output may use spaces vs. fullwidth padding
  for CJK cells; the assertion compares character indices, not byte
  indices, and uses `unicode_width::UnicodeWidthStr::width` to
  convert to display columns. If WezTerm pads CJK with one trailing
  space (instead of treating the CJK char as width-2), the test
  documents that and uses display-column math.

---

## Phase 6 — `TerminalHarness::spawn` honors the shell model

**Objective.** Reconcile the trait contract documented at
`biscuit-test-harness/src/lib.rs:56` with the implementations so that
**all callers** of `spawn` end up in a shell with `bt` on PATH.

**Review items addressed.**
- §High-5 "`TerminalHarness::spawn` was not adapted to the shell
  model" (`wezterm.rs:253`, `kitty.rs:110`, `tmux.rs:132`).

### Files to modify

- `biscuit-test-harness/src/lib.rs` — rework the trait.
- `biscuit-test-harness/src/wezterm.rs` — re-route `spawn` impl.
- `biscuit-test-harness/src/kitty.rs` — re-route `spawn` impl.
- `biscuit-test-harness/src/tmux.rs` — re-route `spawn` impl.
- `biscuit-tui/cli/tests/...` — update any direct caller of `spawn`
  that expects to launch a binary directly. **Audit step.**

### Implementation steps

1. **Rename existing direct-spawn methods.** On each implementor type,
   rename the `spawn` method (currently the trait impl) to
   `spawn_program` as an inherent method. This frees the trait method
   name.
2. **Redefine the trait contract.** In `lib.rs`:
   ```rust
   pub trait TerminalHarness {
       /// Spawn a fresh shell. The cargo bin dir is prepended to PATH.
       /// Tests interact with the shell via `send_text("bt ...\n")`.
       fn spawn_shell(&mut self) -> io::Result<()>;
       fn send_text(&mut self, bytes: &[u8]) -> io::Result<()>;
       fn capture(&mut self) -> io::Result<CapturedFrame>;
       fn settle(&self) { ... }

       /// Lower-level escape hatch: spawn a specific program directly.
       /// Default implementation returns ErrorKind::Unsupported. Type
       /// implementations may override to provide direct-spawn.
       fn spawn_program(&mut self, _program: &str, _args: &[&str]) -> io::Result<()> {
           Err(io::Error::new(io::ErrorKind::Unsupported,
               "spawn_program not implemented; use spawn_shell"))
       }
   }
   ```
3. **Update each impl block** to provide both `spawn_shell` (already
   present as inherent; expose via trait) and `spawn_program`
   (renamed from the prior `spawn`).
4. **Audit biscuit-tui callers.** `grep -rn "\.spawn(" biscuit-tui/cli/tests`
   and update any test that relied on the old direct-spawn semantics.
   Replace with either `spawn_shell()` followed by
   `send_text("question ...\n")` (preferred) or `spawn_program("question", &[...])`.
5. **Document the shell-model contract** in the trait doc comment.

### Test additions / changes

- Add a new compile-time check unit test in
  `biscuit-test-harness/src/lib.rs::tests`:
  ```rust
  #[test]
  fn trait_default_spawn_program_is_unsupported() {
      struct Stub;
      impl TerminalHarness for Stub {
          fn spawn_shell(&mut self) -> io::Result<()> { Ok(()) }
          fn send_text(&mut self, _: &[u8]) -> io::Result<()> { Ok(()) }
          fn capture(&mut self) -> io::Result<CapturedFrame> {
              Ok(CapturedFrame::from_raw(String::new()))
          }
      }
      let mut s = Stub;
      assert!(s.spawn_program("x", &[]).is_err());
  }
  ```
- All existing `level2_*` tests continue to call `spawn_shell()`
  unchanged.
- biscuit-tui tests: amend signatures only where required.

### Verification

```bash
cargo test  -p biscuit-test-harness
cargo test  -p biscuit-terminal-cli --tests
cargo test  -p tui-chrome-cli --tests
cargo clippy -p biscuit-test-harness -p biscuit-terminal-cli -p tui-chrome-cli --tests -- -D warnings
cargo fmt   -p biscuit-test-harness -p biscuit-terminal-cli -p tui-chrome-cli -- --check
```

### Risks / open questions

- biscuit-tui's `level2_*` tests already use `spawn_shell` per the
  spec §5.1, so the audit may be a no-op. If any test depends on
  direct-spawn, we accept the trivial rename.
- This is the only phase that touches `biscuit-tui-cli`. Keep
  changes minimal.

---

## Phase 7 — Lint, format, and final acceptance sweep

**Objective.** Resolve every clippy warning and fmt drift across the
affected crates introduced by Phases 1–6, run the full review-defined
test command set, and verify the spec's acceptance criteria.

**Review items addressed.**
- §Recommendations 1–5 (final closure).

### Files touched

- Any file flagged by `cargo clippy --tests -- -D warnings` for the
  three crates.
- Any file flagged by `cargo fmt -- --check` for the three crates.

### Implementation steps

1. **Clippy sweep.**
   ```bash
   cargo clippy -p biscuit-test-harness  --tests -- -D warnings
   cargo clippy -p biscuit-terminal      --tests -- -D warnings
   cargo clippy -p biscuit-terminal-cli  --tests -- -D warnings
   ```
   Resolve every warning. Expected categories:
   - `clippy::needless_borrow` in the new diagram parameterized tests.
   - `clippy::uninlined_format_args` from `format!(\"{}\", x)`.
   - `clippy::collapsible_if` in the new pane-row helpers.
   No `#[allow]` blanket attributes; fix at the source.
2. **fmt sweep.**
   ```bash
   cargo fmt -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli -- --check
   ```
   If drift is reported, run without `--check` once and re-verify.
3. **Re-run the full review command set** verbatim from review §Test
   Commands Run:
   ```bash
   cargo test -p biscuit-test-harness
   cargo test -p biscuit-terminal --test level1_osc_queries --test level1_clipboard --test level1_mode_2027 --test level1_cursor --test level1_terminal_init
   cargo test -p biscuit-terminal-cli --test level2_prose_styling --test level2_image --test level2_diagrams --test level2_cursor_and_hygiene
   cargo check -p tui-chrome-cli --tests
   ```
4. **Verify spec §7 acceptance criteria** by inspection. Append a
   `## Acceptance` table to this plan as part of Phase 7's commit
   record (in the PR description, not in this file) showing each
   criterion's pass evidence.

### Verification

The command set in step 3 plus:

```bash
cargo doc --no-deps -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli
just test 2>/dev/null || true   # only if root-justfile covers these areas
```

### Risks / open questions

- New clippy lints introduced by an upgraded toolchain may surface
  during this phase; resolve at source. No new `#[allow]` lines.
- `cargo doc` may complain about doctests that depend on the running
  WezTerm. Mark such doctests as `ignore` per repo convention.

---

## Cross-phase summary

| Phase | Review items | Crate(s) touched | Estimated runtime impact |
|-------|--------------|------------------|--------------------------|
| 1     | High §1      | harness, lib, cli| Negligible               |
| 2     | High §2      | harness, cli     | +30 s test runtime       |
| 3     | High §3      | cli              | Negligible               |
| 4     | High §4      | cli              | +2 min test runtime      |
| 5     | Medium       | cli              | +20 s test runtime       |
| 6     | High §5      | harness, tui     | Negligible               |
| 7     | All (sweep)  | all              | Negligible               |

## Out-of-scope for this plan

- Adding new harnesses (Ghostty, Alacritty, iTerm2). Spec §8 defers.
- Self-hosted CI runner for Level-2 enforcement. Spec §8 defers.
- Performance benchmarking. Existing 2026-04-08 review owns it.

## Final exit criteria (all phases done)

1. `cargo test -p biscuit-test-harness` passes.
2. `cargo test -p biscuit-terminal --test level1_*` passes (no
   regressions).
3. `cargo test -p biscuit-terminal-cli --test level2_*` passes
   (skips cleanly when terminals are unavailable; passes
   deterministically on the developer host with WezTerm + Kitty
   available).
4. `cargo test -p tui-chrome-cli --tests` passes (no regression from
   Phase 6).
5. `cargo clippy -p {biscuit-test-harness,biscuit-terminal,biscuit-terminal-cli} --tests -- -D warnings` passes.
6. `cargo fmt -p {biscuit-test-harness,biscuit-terminal,biscuit-terminal-cli} -- --check` passes.
7. The `level2_prose_emits_sgr_in_real_terminal` test passes on a
   host with WezTerm available — closing the High-1 finding.
