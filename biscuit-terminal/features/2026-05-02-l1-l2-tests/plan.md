---
phases: 5
created: 2026-05-02
start_phase: 1
source_files_during_phase_1:
  - biscuit-test-harness/Cargo.toml
  - biscuit-test-harness/src/lib.rs
  - biscuit-test-harness/src/wezterm.rs
  - biscuit-test-harness/src/kitty.rs
  - biscuit-test-harness/src/tmux.rs
  - biscuit-test-harness/src/cliclick.rs
  - Cargo.toml
  - biscuit-tui/cli/Cargo.toml
  - biscuit-terminal/cli/Cargo.toml
  - biscuit-tui/cli/tests/common/real_terminal/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-terminal/lib/tests/common/mod.rs
  - biscuit-terminal/lib/tests/common/pty.rs
  - biscuit-terminal/lib/examples/discovery_probe.rs
  - biscuit-terminal/lib/tests/level1_osc_queries.rs
  - biscuit-terminal/lib/tests/level1_clipboard.rs
  - biscuit-terminal/lib/tests/level1_mode_2027.rs
  - biscuit-terminal/lib/tests/level1_cursor.rs
  - biscuit-terminal/lib/tests/level1_terminal_init.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - biscuit-terminal/cli/tests/common/mod.rs
  - biscuit-terminal/cli/tests/level2_prose_styling.rs
  - biscuit-terminal/cli/src/commands.rs
  - biscuit-terminal/cli/tests/snapshots/integration_test__prose_snapshot.snap
  - biscuit-terminal/cli/tests/snapshots/integration_test__prose_styled_snapshot.snap
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - biscuit-terminal/cli/tests/level2_image.rs
  - biscuit-terminal/cli/tests/level2_diagrams.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/biscuit-terminal/SKILL.md
packages:
  - biscuit-test-harness
  - biscuit-tui
  - biscuit-terminal
---

# Execution Plan: Strengthen Level 1 / Level 2 Testing

**Source spec:** `spec.md` (this directory)
**Total estimated effort:** 9–13 days

---

## Phase 1 — Shared Test Harness Foundation

> Unblocks all subsequent phases. Must land first.

### Step 1.1 — Scaffold `biscuit-test-harness` workspace crate

- Create `biscuit-test-harness/Cargo.toml` with `publish = false`, no external deps beyond `std`.
- Create `biscuit-test-harness/src/lib.rs` re-exporting submodules.
- Add `"biscuit-test-harness"` to `Cargo.toml` workspace `members` array.
- **Verify:** `cargo check -p biscuit-test-harness` succeeds.

### Step 1.2 — Port harness types and trait from biscuit-tui

- Copy `CapturedFrame`, `strip_ansi`, `skip_with_reason` into `biscuit-test-harness/src/lib.rs` (from `biscuit-tui/cli/tests/common/real_terminal/mod.rs`).
- Copy `TerminalHarness` trait into the same file.
- Keep the `#[cfg(test)] mod tests` block for `strip_ansi`.
- **Verify:** `cargo test -p biscuit-test-harness` passes (strip_ansi unit tests).

### Step 1.3 — Port `WezTermHarness`

- Create `biscuit-test-harness/src/wezterm.rs` from `biscuit-tui/cli/tests/common/real_terminal/wezterm.rs`.
- Remove `focus_spawned_pane` / `cliclick` related code (not needed for biscuit-terminal).
- Convert `spawn` to the **shell model**: spawn `bash -l` (or `$SHELL`), not the target binary directly. Prepend the cargo target dir to PATH so `bt` resolves.
- Add **prompt-matching logic**: after spawning the shell, wait for a shell prompt (e.g., match `$ ` or `# ` or `% `) before returning, so callers don't race shell readiness.
- Add **shell detection**: `$SHELL` → `bash` → `sh` fallback chain.
- **Verify:** `cargo check -p biscuit-test-harness` succeeds.

### Step 1.4 — Port `KittyHarness`

- Create `biscuit-test-harness/src/kitty.rs` from `biscuit-tui/cli/tests/common/real_terminal/kitty.rs`.
- Apply the same shell-model changes as Step 1.3 (spawn shell, not binary directly).
- **Verify:** `cargo check -p biscuit-test-harness` succeeds.

### Step 1.5 — Port `TmuxHarness`

- Create `biscuit-test-harness/src/tmux.rs` from `biscuit-tui/cli/tests/common/real_terminal/tmux.rs`.
- Apply shell-model changes. tmux's `-x 120 -y 40` size args stay; the inner process becomes the shell.
- Remove `send_key` method (not needed without keyboard injection).
- **Verify:** `cargo check -p biscuit-test-harness` succeeds.

### Step 1.6 — Deduplicate: update biscuit-tui to consume new crate

- Add `biscuit-test-harness` as a dev-dependency in `biscuit-tui/cli/Cargo.toml`.
- Replace `biscuit-tui/cli/tests/common/real_terminal/mod.rs` with a re-export from `biscuit_test_harness`.
- Remove the now-redundant `wezterm.rs`, `kitty.rs`, `tmux.rs`, `cliclick.rs` files from biscuit-tui's tree.
- **Verify:** `cargo test -p tui-chrome-cli` still passes (existing Level-2 tests still work).

### Step 1.7 — Add dev-dependency in biscuit-terminal-cli

- Add `biscuit-test-harness` as a dev-dependency in `biscuit-terminal/cli/Cargo.toml`.
- **Verify:** `cargo check -p biscuit-terminal-cli --tests` succeeds.

**Checkpoint 1:** `cargo test -p biscuit-test-harness` passes. Both `tui-chrome-cli` and `biscuit-terminal-cli` compile with `--tests`. The workspace member is registered.

---

## Phase 2 — Level-1 PTY Tests (Capability Detection)

> Tests in `biscuit-terminal/lib/tests/`. These exercise library code via a thin example binary inside a PTY (using `expectrl`). Quick wins — no real terminal required.

### Step 2.1 — Create PTY test helpers

- Create `biscuit-terminal/lib/tests/common/mod.rs` with `#![allow(dead_code)]`.
- Create `biscuit-terminal/lib/tests/common/pty.rs` providing:
  - `fn spawn_with_env(envs: &[(&str, &str)]) -> expectrl::Session` — spawns `cargo_bin!("discovery_probe")` with standard anti-hang guards (`CI=1`, `NO_COLOR=1`) plus caller-supplied env.
  - `fn anti_hang_env() -> Vec<(&'static str, &'static str)>` returning the base env map.
- **Verify:** File compiles.

### Step 2.2 — Create `discovery_probe.rs` example binary

- Create `biscuit-terminal/lib/examples/discovery_probe.rs`.
- This binary links the library, calls discovery functions, and prints results as key=value lines to stdout.
- Must accept env-var overrides for `TERM_PROGRAM` etc.
- Output format must be machine-parseable (e.g., `bg_color=rgb:128/128/128` or `bg_color=None`).
- **Verify:** `cargo run -p biscuit-terminal --example discovery_probe` exits without error (results will be `None` in piped mode, that's fine).

### Step 2.3 — `level1_osc_queries.rs`

- Create `biscuit-terminal/lib/tests/level1_osc_queries.rs`.
- Tests:
  - `bg_color_query_returns_some_with_manufactured_reply` — spawn probe in PTY, manufacture OSC 11 reply (`\x1b]11;rgb:80/80/80\x07`), assert output contains `bg_color=Some(...)`.
  - `text_color_query_returns_some_with_manufactured_reply` — same for OSC 10.
  - `cursor_color_query_returns_some_with_manufactured_reply` — same for OSC 12.
- Each test spawns its own PTY session; no shared state.
- **Verify:** `cargo test -p biscuit-terminal --test level1_osc_queries` passes.

### Step 2.4 — `level1_clipboard.rs`

- Create `biscuit-terminal/lib/tests/level1_clipboard.rs`.
- Tests:
  - `osc52_sequence_emitted_to_tty` — spawn probe with env triggering OSC52 write, capture PTY output, assert bytes contain `\x1b]52;c;` + base64 payload + `\x07`.
- **Verify:** `cargo test -p biscuit-terminal --test level1_clipboard` passes.

### Step 2.5 — `level1_mode_2027.rs`

- Create `biscuit-terminal/lib/tests/level1_mode_2027.rs`.
- Tests:
  - `enable_mode_2027_emits_escape_sequence` — spawn probe in PTY, trigger enable, capture raw bytes, assert contains `\x1b[?2027h`.
  - `disable_mode_2027_emits_escape_sequence` — same for `\x1b[?2027l`.
- **Verify:** `cargo test -p biscuit-terminal --test level1_mode_2027` passes.

### Step 2.6 — `level1_cursor.rs`

- Create `biscuit-terminal/lib/tests/level1_cursor.rs`.
- Tests:
  - `cursor_position_query_emits_csi_6n` — spawn probe, trigger cursor query, capture raw bytes, assert output contains `\x1b[6n`.
  - `cursor_position_parses_csi_r_reply` — manufacture a `\x1b[12;34R` reply, assert probe reports `cursor_position=Some(row=12,col=34)`.
- **Verify:** `cargo test -p biscuit-terminal --test level1_cursor` passes.

### Step 2.7 — `level1_terminal_init.rs`

- Create `biscuit-terminal/lib/tests/level1_terminal_init.rs`.
- Tests:
  - `terminal_new_cascade_produces_consistent_fields_in_pty` — spawn probe in PTY with `TERM_PROGRAM=Ghostty`, verify output shows consistent capability fields (e.g., `is_tty=true`, `app=Ghostty`).
- **Verify:** `cargo test -p biscuit-terminal --test level1_terminal_init` passes.

**Checkpoint 2:** All Level-1 tests pass: `cargo test -p biscuit-terminal --test level1_` (all 5 test files). Each public function in `osc_queries`, `clipboard`, `mode_2027`, `cursor_position` has at least one PTY test asserting on parsed results.

---

## Phase 3 — Level-2 Prose, OSC8, and NO_COLOR Tests

> Small, high-confidence Level-2 tests. Good validation that the shell-model harness works end-to-end before tackling the more complex image rendering tests.

### Step 3.1 — Create CLI test helpers

- Create `biscuit-terminal/cli/tests/common/mod.rs` with `#![allow(dead_code)]`.
- Re-export `biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason}`.
- Create a helper `fn send_bt_command(harness: &mut impl TerminalHarness, args: &str)` that sends `bt {args}\n` and calls `settle()`.
- **Verify:** File compiles.

### Step 3.2 — Create test fixtures

- Create `biscuit-terminal/cli/tests/fixtures/` directory.
- Add `tiny.png` (1×1 pixel, <1 KB) and `tiny.jpg` (1×1 pixel, <1 KB) fixtures.
- These are used by Phase 4 image tests too, so create them here.
- **Verify:** Fixtures exist and are valid images.

### Step 3.3 — `level2_prose_styling.rs`

- Create `biscuit-terminal/cli/tests/level2_prose_styling.rs`.
- Add top-of-file comment block explaining the skip-clean contract.
- Tests:
  - `level2_prose_emits_sgr_in_real_terminal` — WezTerm harness, `send_text("bt prose \"<red>x</red>\"\n")`, assert `frame.raw` contains `\x1b[31m` or `\x1b[91m`.
  - `level2_prose_osc8_link_renders` — WezTerm, send `bt prose "<a href=https://example.com>link</a>"\n`, assert `frame.raw` contains `\x1b]8;;https://example.com` and `frame.plain` contains `link`.
  - `level2_no_color_strips_sgr_in_real_terminal` — WezTerm with `NO_COLOR=1` in shell env, send `bt prose "<red>x</red>"\n`, assert `frame.raw` has zero SGR sequences for the color.
- Each test early-returns with `skip_with_reason` when harness unavailable.
- **Verify:** `cargo test -p biscuit-terminal-cli --test level2_prose_styling` — skips cleanly on hosts without WezTerm.

### Step 3.4 — `level2_prose_styling.rs` — Kitty variant

- Add Kitty versions of the SGR and OSC8 tests (same assertions, `KittyHarness`).
- **Verify:** Tests skip cleanly without Kitty; pass locally with Kitty.

### Step 3.5 — `level2_prose_styling.rs` — layout tests

- Tests:
  - `level2_pad_columns_respect_actual_pane_width` — WezTerm, `send_text("bt padleft 30 \"x\"\n")`, captured pane shows 29 spaces + `x`.
  - `level2_columns_word_wrap_in_pane` — WezTerm with narrow pane, `bt columns "long…" "long…"\n`, captured rows match expected wrap.
- **Verify:** `cargo test -p biscuit-terminal-cli --test level2_prose_styling` passes (or skips).

**Checkpoint 3:** `level2_prose_styling.rs` is complete. SGR, OSC8, NO_COLOR, and layout assertions all pass through real terminal. Harness shell model validated end-to-end.

---

## Phase 4 — Level-2 Image and Diagram Rendering Tests

> Highest-value gap. Requires fixture images and validates the most terminal-specific code paths.

### Step 4.1 — `level2_image.rs` — basic image rendering

- Create `biscuit-terminal/cli/tests/level2_image.rs`.
- Add top-of-file comment block explaining the skip-clean contract.
- Tests:
  - `level2_image_renders_in_wezterm` — WezTerm harness, `send_text("bt image fixtures/tiny.png\n")`, assert capture contains iTerm2 image protocol bytes (`\x1b]1337;File=...`).
  - `level2_image_renders_in_kitty` — Kitty harness, `send_text("bt image fixtures/tiny.png\n")`, assert capture contains Kitty graphics protocol APC sequence (`\x1b_G`).
- **Verify:** Tests skip cleanly without terminals; pass locally.

### Step 4.2 — `level2_image.rs` — scroll and rounding

- Tests:
  - `level2_image_scroll_compensation_at_bottom_margin` — WezTerm with small pane (24 rows), position cursor near bottom, send `bt image fixtures/tiny.png\n`, verify scroll compensation.
  - `level2_warp_uses_floor_rounding` — WezTerm with `TERM_PROGRAM=WarpTerminal` env, send `bt image fixtures/tiny.png\n`, assert row math uses `floor` not `ceil`.
  - `level2_image_meta_to_stderr` — WezTerm, `bt image --meta fixtures/tiny.png\n`, verify stderr metadata separate from stdout image bytes.
- **Verify:** Tests pass locally with WezTerm.

### Step 4.3 — `level2_diagrams.rs` — Mermaid rendering

- Create `biscuit-terminal/cli/tests/level2_diagrams.rs`.
- Tests:
  - `level2_flowchart_renders_in_wezterm` — WezTerm, `bt flowchart "A --> B"\n`, assert capture contains image-protocol bytes.
  - `level2_pie_chart_renders_in_kitty` — Kitty, `bt pie-chart "A,1"\n`, assert capture contains Kitty graphics protocol bytes.
  - `level2_diagram_width_respects_pane_columns` — WezTerm with known column count, `bt pie-chart "A,1" --width 50%\n`, assert image-block width matches.
  - `level2_inverse_flag_changes_background_in_capture` — WezTerm, `bt flowchart "A --> B" --inverse\n`, assert capture differs from default (compare raw hashes).
  - `level2_diagram_fallback_when_no_image_protocol` — Tmux (no image protocols), `bt flowchart "A --> B"\n`, verify fenced code block fallback fires.
- **Verify:** All tests pass with respective terminals; skip cleanly otherwise.

**Checkpoint 4:** All image and diagram rendering paths have Level-2 coverage. Every divergent rounding/scroll branch has a dedicated test.

---

## Phase 5 — Cursor Hygiene, Tooling, and Documentation

> Polish phase. Can be parallelized across three independent tracks.

### Step 5.1 — `level2_cursor_and_hygiene.rs`

- Create `biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs`.
- Tests:
  - `level2_cursor_lands_below_rendered_image` — WezTerm, `bt image fixtures/tiny.png\n`, then send sentinel string, assert it appears on line directly below image in capture.
  - `level2_no_orphan_save_restore_sequences` — For any rendering command, `frame.raw` contains balanced `\x1b[s` / `\x1b[u` pairs.
  - `level2_dir_command_unicode_widths_in_capture` — `bt dir\n` against fixture with CJK/emoji filenames, captured columns align.
- **Verify:** Tests pass with WezTerm; skip cleanly otherwise.

### Step 5.2 — Justfile recipe

- Add `test-l2` recipe to `biscuit-terminal/justfile`:
  ```just
  # Run only Level-2 real-terminal tests
  test-l2 *args="":
      @cargo test -p biscuit-terminal-cli --test level2_ {{ args }}
  ```
- **Verify:** `just test-l2` runs Level-2 test files only.

### Step 5.3 — README documentation

- Add a section to `biscuit-terminal/README.md` explaining:
  - How to set `WEZTERM_UNIX_SOCKET` / `KITTY_LISTEN_ON` for local Level-2 runs.
  - That Level-2 tests skip cleanly in CI.
  - The Level 1/2/3 testing vocabulary.
- **Verify:** README renders correctly.

### Step 5.4 — Skill documentation update

- Update `.claude/skills/biscuit-terminal/SKILL.md` (or add a new `testing.md` page) documenting:
  - biscuit-terminal follows the Level 1/2/3 testing vocabulary.
  - biscuit-test-harness is the shared harness crate.
  - How to add new Level-1 or Level-2 tests.
  - Cross-reference to the `cli` skill's testing tier vocabulary.
- **Verify:** Skill file is well-formed.

**Checkpoint 5:** All acceptance criteria from spec §7 are met. `just test-l2` works. README and skill docs are updated. Full suite passes (or skips cleanly).

---

## Dependency graph

```
Phase 1 (harness)
  ├── Phase 2 (Level-1 PTY) ── uses expectrl, not harness
  ├── Phase 3 (Level-2 prose) ── uses harness
  │     └── Phase 4 (Level-2 images) ── uses harness + fixtures from Phase 3
  └── Phase 5 (hygiene + tooling) ── uses harness
        ├── Step 5.1 depends on Phase 4 (image fixtures)
        ├── Step 5.2 independent
        ├── Step 5.3 independent
        └── Step 5.4 independent
```

## Parallelizable work

| Tracks | Can run in parallel |
|---|---|
| Phase 2 + Phase 1 (Steps 2.1–2.2 can start after Step 1.7) | Level-1 tests use `expectrl` directly, not the shared harness |
| Steps 5.2, 5.3, 5.4 | All independent of each other |
| Step 4.1 + 4.2 + 4.3 | Image and diagram test files are independent |

## Risk mitigations

- **Shell model complexity:** Phase 3 Step 3.3 is the first end-to-end test of the shell model. If prompt-matching proves unreliable, fall back to the binary-direct model for Level-1 tests (they use `expectrl` directly anyway).
- **Test runtime:** Budget ~5–10s per Level-2 test. ~25 tests = ~3 min full local run. Acceptable.
- **CI skip-clean:** Every Level-2 test checks `harness.available()` and early-returns with `skip_with_reason`. No `#[ignore]` markers.
