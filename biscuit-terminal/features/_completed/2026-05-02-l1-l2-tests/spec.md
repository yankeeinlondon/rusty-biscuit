# Spec: Strengthen Level 1 / Level 2 Testing in biscuit-terminal

**Status:** Unscheduled
**Author:** Claude (Opus 4.7) for Ken Snyder
**Date:** 2026-05-02
**Skill basis:** `cli` skill — `cli-best-practices.md` §"Test Rigor: Level 1 / Level 2 / Level 3"

## 1. Why this exists

biscuit-terminal is the **authority** in this monorepo for two things:

1. Detecting terminal capabilities across 13+ emulators (Kitty, Ghostty, WezTerm, iTerm2, Warp, Alacritty, …).
2. Producing rich terminal rendering — inline images, Mermaid diagrams, OSC8 hyperlinks, SGR styled prose, cursor-aware image placement, etc.

Both responsibilities are inherently **terminal-mediated**: the code's correctness can only be observed *through* a real (or pseudo) terminal. Yet the current test suite is overwhelmingly **Level 0/1**:

| Layer | Count | Coverage |
|---|---|---|
| In-process unit tests | ~1,146 | Internal helpers, parsers, struct population |
| Library integration tests | ~30 | "doesn't panic" smoke tests |
| `assert_cmd` CLI integration | ~75 | Args, JSON shape, exit codes, NO_COLOR |
| `insta` snapshots | 7 | A handful of styled outputs (NO_COLOR mode only) |
| `expectrl` PTY (Level 1) | **2** | One metadata probe + one `--meta` flag test |
| Real-terminal (Level 2) | **0** | None |
| OS keyboard injection (Level 3) | **0** | N/A — biscuit-terminal has no interactive input |

The 2026-04-08 testing-strategy review (`reviews/2026-04-08-testing-strategy/review.md`) flagged the snapshot/CLI gaps but did **not** apply the Level 1/2/3 vocabulary that the `cli` skill now mandates. This spec addresses that gap directly.

### The structural blindness

Every "render to terminal" code path in `lib/src/components/` and every capability probe in `lib/src/discovery/` has the same property: an in-process test cannot lie about its own input bytes, but it **can** lie about whether those bytes display correctly when a real terminal interprets them. The 5-terminal image rendering work captured in memory (`biscuit-terminal/docs/terminal-images.md`) was debugged by hand-running the binary inside each terminal — there is no automated regression for any of it.

A Level-2 harness is the missing piece. biscuit-tui already maintains the canonical implementation at `biscuit-tui/cli/tests/common/real_terminal/` (WezTerm + Kitty + tmux + cliclick) and the `cli` skill points to it as the reference. **biscuit-terminal should adopt the same harness.**

## 2. Goals

- Establish a Level-1 (PTY) safety net for every code path that *only fires when stdout is a TTY*.
- Establish a Level-2 (real-terminal) safety net for the rendering pipeline so escape-code regressions, glyph-width bugs, scroll-compensation bugs, and OSC8 link breakage are detected on developer machines that have the required terminal emulator installed. These tests skip cleanly in CI and do not gate PRs on GitHub-hosted runners.
- Reuse — not re-invent — the `TerminalHarness` trait and per-terminal harnesses already proven in biscuit-tui.
- Keep the suite **skip-clean** on hosts without the required terminal (no `#[ignore]` markers — print `skipping: requires <X>` and return).

## 3. Non-goals

- Level-3 keyboard-injection tests (biscuit-terminal has no interactive input surface).
- Visual screenshot diffing (impractical for 13 terminals; out of scope).
- Replacing existing unit tests — this spec is **additive**.
- Performance benchmarking (already covered by the 2026-04-08 review's criterion recommendation).

## 4. Reference: `cli` skill rule for level selection

> *"For each user-observable requirement, ask: what is the lowest-fidelity test that could lie about this?"*

Per the `cli` skill's table:

| Requirement shape | Minimum level |
|---|---|
| Internal state transition | Level 1 |
| Argument parsing / output formatting | Level 1 |
| `--json` output is valid JSON | Level 1 |
| **Terminal-rendered glyph / width / colour** | **Level 2** |
| **Scrolling / overflow indicators visible** | **Level 2** |

biscuit-terminal's product surface is dominated by the bottom two rows.

---

## 5. Scope of work

### 5.1 Extract a shared `biscuit-test-harness` crate (foundation)

**Effort:** S (1 day)

Instead of copying the biscuit-tui harness verbatim, extract it into a new workspace member `biscuit-test-harness/` at the repository root.

**What moves:**
- `biscuit-tui/cli/tests/common/real_terminal/` → `biscuit-test-harness/src/`

**What changes:**
- The crate is marked `publish = false` in its `Cargo.toml`.
- Both `biscuit-tui-cli` and `biscuit-terminal-cli` add `biscuit-test-harness` as a `dev-dependency`.
- The duplicate harness code is removed from `biscuit-tui/cli/tests/common/real_terminal/`.
- The harness is adapted to spawn a **login shell** (`bash -l`, or `$SHELL` if available) rather than the target binary directly. Tests interact with the shell by sending text commands.

The harness already gives us:
- `TerminalHarness` trait (`spawn`, `send_text`, `capture`, `settle`).
- `WezTermHarness`, `KittyHarness`, `TmuxHarness` implementations.
- `CapturedFrame { raw, plain }` + a robust ECMA-48 `strip_ansi` helper.
- `available()` probes that check the binary on `$PATH` plus required env (`WEZTERM_UNIX_SOCKET`, `KITTY_LISTEN_ON`, `TMUX`).
- A `skip_with_reason()` helper for clean skips.

**Shell model:** The harness's `spawn` method creates a **new shell instance per call** (`bash -l` or `$SHELL`). Each test spawns its own fresh shell to guarantee isolation — environment variables, working directory, and terminal state do not bleed between tests. Tests use `send_text("bt ...\n")` to type commands into that shell. Before spawning the shell, the harness prepends the cargo target directory (containing `bt`) to PATH so that `send_text("bt ...")` resolves to the compiled binary under test. Tests must not assume any state from previous tests. This is the industry-standard approach for CLI integration testing and supports both one-shot CLIs (biscuit-terminal) and persistent TUIs (biscuit-tui) with minimal adaptation. The harness must include **shell detection** (falling back from `$SHELL` to `bash` to `sh`) and **prompt-matching logic** so that `send_text` does not race the shell's readiness.

**Deliverable:** `biscuit-test-harness/` workspace member with `Cargo.toml`, `src/lib.rs`, and per-terminal modules. Updated `[dev-dependencies]` in both CLI crates. No new external crates required — the harness uses only `std::process::Command` + the `which` probe.

The cliclick module from biscuit-tui can be omitted (biscuit-terminal has no interactive keyboard surface).

#### Background-spawn default (`SpawnVisibility`)

To keep Level-2 tests usable while a developer is actively working on
the same machine, `WezTermHarness` and `KittyHarness` default to
**`SpawnVisibility::Background`**. Concretely:

- **Kitty `Background`** passes `--keep-focus` to `kitty @ launch` so
  the new window is created without stealing focus from whatever the
  developer is doing.
- **WezTerm `Background`** passes `--workspace biscuit-bg` to
  `wezterm cli spawn --new-window`. WezTerm workspaces are independent
  groupings within a single GUI instance, so the spawned window is
  created but is **not visible on the developer's active workspace**.
  The harness still drives it via pane-id (workspace-independent), so
  `send_text` and `get-text` continue to work.

Tests that need the spawned window in the foreground (e.g. biscuit-tui
cliclick tests that subsequently call
`WezTermHarness::focus_spawned_pane`) opt out explicitly:

```rust
let mut harness = WezTermHarness::new()
    .with_spawn_visibility(SpawnVisibility::Foreground);
```

biscuit-terminal Level-2 tests have no OS-level keyboard injection, so
they always use the default Background visibility.

This default is **part of the harness contract**: every new harness
implementation (Ghostty, iTerm2, …) must honor `SpawnVisibility` and
default to `Background`. See `biscuit-test-harness/src/lib.rs` for
the canonical doc-comment.

### 5.2 Level-1 (PTY) — capability-detection round-trips

**Effort:** M (2-3 days)

These must run inside a PTY because the code paths they cover are guarded by `is_tty()` and emit OSC/CSI queries that are silently no-op'd in test environments today. Level-1 tests live in `lib/tests/` and exercise a thin example binary — `lib/examples/discovery_probe.rs` — that links the library, calls the discovery functions directly, and prints results to stdout.

| Test | File | Module under test | What only Level 1 can catch |
|---|---|---|---|
| `bg_color` query returns Some when host responds | `lib/tests/level1_osc_queries.rs` | `discovery/osc_queries.rs` | Today the `_bg = bg_color()` integration test always discards the result — a regression that *always* returns `None` would still pass. PTY + manufactured OSC 11 reply byte stream proves the parser actually extracts RGB. |
| `text_color` / `cursor_color` likewise | `lib/tests/level1_osc_queries.rs` | `osc_queries.rs` | Same. |
| `osc52_support` round-trip | `lib/tests/level1_clipboard.rs` | `discovery/clipboard.rs` | We *build* an OSC 52 sequence and parse base64 in a unit test, but never verify the binary writes the sequence to a TTY at the right moment. PTY captures the byte stream emitted to stdout. |
| `enable_mode_2027` / `disable_mode_2027` | `lib/tests/level1_mode_2027.rs` | `discovery/mode_2027.rs` | Mode 2027 enable writes `\x1b[?2027h` and reads back via DECRQM. The current "doesn't panic" test proves nothing about the actual bytes emitted. |
| `cursor_position` query | `lib/tests/level1_cursor.rs` | `discovery/cursor_position.rs` | Cursor position is read by issuing CSI 6n and parsing CSI R reply. Untested today. |
| `Terminal::new()` capability cascade in PTY | `lib/tests/level1_terminal_init.rs` | `terminal.rs` + all of `discovery/` | Verifies that the optimistic-vs-detected paths produce consistent fields when given a PTY (not just `is_tty()=true`). |

**Pattern (for each test):**

```rust
use expectrl::{Expect, Session};
use std::process::Command;

// Spawn the discovery_probe example binary (links lib, prints discovery results)
let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("discovery_probe"));
cmd.env("CI", "1").env("NO_COLOR", "1").env("TERM_PROGRAM", "Ghostty");
let mut p = Session::spawn(cmd).unwrap();
// Manufacture the terminal's would-be reply, then assert on parsed fields.
p.send(&[0x1b, b']', b'1', b'1', b';' /* … */]).unwrap();
p.expect("expected_field_in_output").unwrap();
```

> The `cargo_bin!("discovery_probe")` macro resolves to the compiled example binary in the cargo target directory, using the same mechanism already used by existing `assert_cmd` integration tests.

Note the `CI=1` / `NO_COLOR=1` / `TERM_PROGRAM=Ghostty` recipe — it is the same anti-hang guard already used at `cli/tests/integration_test.rs:1707-1712` and must be standardized as a helper in `lib/tests/common/pty.rs` (mirroring biscuit-tui's `common/pty.rs`).

### 5.3 Level-2 — image rendering across 5 verified terminals

**Effort:** L (3-5 days)

This is the highest-value gap. `lib/src/components/terminal_image.rs::render_to_terminal` is the most terminal-specific code in the package, with five known divergent code paths captured in memory:

> All terminals use `ceil` except Warp (`floor`). Scroll compensation when `cursor_row + image_rows > term_height` … Ghostty handles natively, Warp never scrolls, etc.

Today, **none** of those rules has a Level-2 test. The only verification is hand-running.

| Test | Harness | Asserts |
|---|---|---|
| `level2_image_renders_in_wezterm` | WezTerm | `send_text("bt image fixture.png\n")` in a WezTerm pane; assert the captured pane's post-image cursor row matches the documented `\x1b[s` + image + `\x1b[u` + `\x1b[{N}B` strategy. Captures contain the iTerm2 image protocol bytes. |
| `level2_image_renders_in_kitty` | Kitty | `send_text("bt image fixture.png\n")` in a Kitty pane; assert capture contains Kitty graphics protocol APC sequence (`\x1b_G…`). Cursor row math is correct. |
| `level2_image_scroll_compensation_at_bottom_margin` | WezTerm | Spawn a small pane (e.g. 24 rows), send `bt image fixture.png\n` when cursor is on row 22 of 24 — verify the `\n` scroll-compensation hack actually advances correctly. |
| `level2_warp_uses_floor_rounding` | WezTerm with `TERM_PROGRAM=WarpTerminal` env | Send `bt image fixture.png\n` and assert row count matches `floor` not `ceil`. (Warp itself is not scriptable, but the rounding branch can be exercised via env-driven detection.) |
| `level2_image_meta_to_stderr` | WezTerm | Send `bt image --meta fixture.png\n` and verify stderr metadata appears separately from stdout image bytes. |

Fixture images live at `cli/tests/fixtures/` (PNG + JPG ≤ 10 KB).

### 5.4 Level-2 — Mermaid / diagram rendering

**Effort:** M (2-3 days)

The diagram subcommands (`flowchart`, `pie-chart`, `bar-chart`, `line-chart`, `quadrant`, `git-graph`, `timeline`, `state-diagram`, `erd`, `graph-expression`) currently have ~50 `assert_cmd` tests that all run `--json`. JSON proves the *generator* works. **Nothing proves the rendered SVG-to-image pipeline emits valid escape codes for each terminal.**

| Test | Asserts |
|---|---|
| `level2_flowchart_renders_in_wezterm` | Send `bt flowchart "A --> B"\n` (no `--json`) in a WezTerm pane; assert captured output contains image-protocol bytes. |
| `level2_pie_chart_renders_in_kitty` | Same in Kitty; assert captured output contains Kitty graphics protocol bytes. |
| `level2_diagram_width_respects_pane_columns` | Spawn pane with known column count, send `bt pie-chart "A,1" --width 50%\n`, assert image-block character width matches. |
| `level2_inverse_flag_changes_background_in_capture` | Send `bt flowchart "A --> B" --inverse\n`; assert capture differs from default in the encoded image bytes (compare hashes). |
| `level2_diagram_fallback_when_no_image_protocol` | Spawn under `TmuxHarness` (where image protocols are commonly disabled), send `bt flowchart "A --> B"\n` — verify the fenced code block fallback fires. |

### 5.5 Level-2 — Prose styling, OSC8, NO_COLOR

**Effort:** S-M (2 days)

| Test | Asserts |
|---|---|
| `level2_prose_emits_sgr_in_real_terminal` | Send `bt prose "<red>x</red>"\n` — `frame.raw` contains `\x1b[31m`/`\x1b[91m` (per palette). Today only NO_COLOR snapshots exist. |
| `level2_prose_osc8_link_renders` | Send `bt prose "<a href=https://example.com>link</a>"\n` — `frame.raw` contains `\x1b]8;;https://example.com\x1b\\` (or BEL terminator), AND `frame.plain` contains the visible text "link". |
| `level2_no_color_strips_sgr_in_real_terminal` | With `NO_COLOR=1`, send `bt prose "<red>x</red>"\n`; assert `frame.raw` contains zero SGR sequences for the same input. (Currently asserted in-process, not in a real terminal.) |
| `level2_pad_columns_respect_actual_pane_width` | Send `bt padleft 30 "x"\n` — captured pane shows 29 spaces + `x` regardless of host's `$COLUMNS`. |
| `level2_columns_word_wrap_in_pane` | Send `bt columns "long…" "long…"\n` in a narrow pane — captured rows match expected wrap. |

### 5.6 Level-2 — Cursor placement & ANSI hygiene

**Effort:** S (1 day)

| Test | Asserts |
|---|---|
| `level2_cursor_lands_below_rendered_image` | Send `bt image fixture.png\n`, then send a sentinel string and assert it appears on the line directly below the image in the capture (catches off-by-one in CUD math). |
| `level2_no_orphan_save_restore_sequences` | `frame.raw` for any rendering command contains balanced `\x1b[s` / `\x1b[u` (no orphan save with no restore). |
| `level2_dir_command_unicode_widths_in_capture` | Send `bt dir\n` against a fixture with CJK/emoji filenames — captured columns align (each row's pre-name padding consistent). |

### 5.7 Tooling & ergonomics

**Effort:** S (0.5 day)

- Add a `just test-l2` recipe in `biscuit-terminal/justfile` that runs only the Level-2 test files, and document in `biscuit-terminal/README.md` how to set `WEZTERM_UNIX_SOCKET` / `KITTY_LISTEN_ON` for local runs.
- Add a top-of-file note in each Level-2 test file (mirroring biscuit-tui's `real_terminal_render.rs:1-14`) explaining the skip-clean contract.
- Document in `.claude/skills/biscuit-terminal/SKILL.md` (or a new `testing.md` page) that biscuit-terminal follows the Level 1/2/3 testing vocabulary, with biscuit-tui as the harness reference.

---

## 6. File layout (proposed)

```
biscuit-test-harness/                    # NEW — shared workspace member (publish = false)
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── wezterm.rs
    ├── kitty.rs
    └── tmux.rs

biscuit-terminal/lib/
├── examples/
│   └── discovery_probe.rs               # NEW (§5.2) — thin binary linking lib
└── tests/
    ├── common/
    │   ├── mod.rs
    │   └── pty.rs                       # standardized PTY helper
    ├── level1_osc_queries.rs            # NEW (§5.2)
    ├── level1_clipboard.rs              # NEW (§5.2)
    ├── level1_mode_2027.rs              # NEW (§5.2)
    ├── level1_cursor.rs                 # NEW (§5.2)
    └── level1_terminal_init.rs          # NEW (§5.2)

biscuit-terminal/cli/tests/
├── integration_test.rs                  # existing — unchanged
├── snapshots/                           # existing — keep growing
├── fixtures/                            # NEW — small PNG/JPG/SVG fixtures
│   ├── tiny.png
│   └── tiny.jpg
├── common/                              # NEW — re-exports harness + CLI helpers
│   ├── mod.rs
│   └── pty.rs
├── level2_image.rs                      # NEW (§5.3)
├── level2_diagrams.rs                   # NEW (§5.4)
├── level2_prose_styling.rs              # NEW (§5.5)
└── level2_cursor_and_hygiene.rs         # NEW (§5.6)
```

Rationale for the `level1_` / `level2_` filename prefix: makes coverage grep-able (`grep -r "level2_" cli/tests/`), groups CI runtime by tier, and matches the vocabulary the `cli` skill teaches.

---

## 7. Acceptance criteria

A reviewer can mark this work complete when:

1. **Foundation:** The `biscuit-test-harness` workspace crate compiles and is referenced as a `dev-dependency` by both `biscuit-tui-cli` and `biscuit-terminal-cli`. The `lib/tests/common/pty.rs` helper exists, used by at least one new Level-1 test.
2. **Skip-clean:** Running `cargo test -p biscuit-terminal-cli` on a CI box with no terminal emulator emits `skipping: requires <X>` lines but exits 0; no `#[ignore]` markers were added.
3. **Capability-detection coverage:** Every public function in `discovery/osc_queries.rs`, `discovery/clipboard.rs`, `discovery/mode_2027.rs`, and `discovery/cursor_position.rs` has at least one Level-1 PTY test that asserts on the *parsed result* (not just "doesn't panic").
4. **Image-rendering coverage:** Every divergent rounding/scroll branch documented in `biscuit-terminal/docs/terminal-images.md` has at least one Level-2 test in WezTerm or Kitty that would fail if the branch were inverted.
5. **Diagram coverage:** At least one Level-2 test per diagram subcommand verifies the rendered output contains image-protocol bytes (or fallback fenced block under tmux).
6. **Prose styling coverage:** Level-2 tests verify SGR, OSC8, and NO_COLOR through a real terminal — not just NO_COLOR snapshots.
7. **Discoverability:** `just test-l2` runs only Level-2 tests; the README explains the env vars; the `cli` and `biscuit-terminal` skills cross-reference each other on the testing tier vocabulary.
8. **Non-disruptive defaults:** Running the Level-2 suite on a machine where the developer is actively working does not pop windows into the foreground or steal keyboard focus. `SpawnVisibility::Background` is the default for both `WezTermHarness` and `KittyHarness`; cliclick-driven biscuit-tui tests explicitly opt in to `Foreground`.

---

## 8. Risks & open questions

- **CI will not run Level-2 tests.** That's by design — the tests are local-developer regression nets and will skip cleanly on GitHub-hosted runners. If we want Level-2 to gate PRs, we need a self-hosted runner with WezTerm/Kitty available; that is a separate spec.
- **Warp scriptability.** Warp does not expose a `wezterm cli`-style IPC. The §5.3 `level2_warp_uses_floor_rounding` test exercises the *detection branch* under WezTerm with spoofed env, not Warp itself. A true Warp Level-2 test would require manual verification or a future Warp scripting API.
- **Ghostty / Alacritty / iTerm2 harnesses.** Out of scope for this spec — adoption can follow the same pattern when their IPC stabilises (Ghostty is closest). The biscuit-tui harness is structured so adding a new emulator is one new file implementing `TerminalHarness`.
- **Shell dependency.** Level-2 tests require a POSIX-compatible login shell (`bash -l` or `$SHELL`) on the host. Non-standard shells or environments without a usable shell may cause spurious failures. The harness includes fallback logic (`$SHELL` → `bash` → `sh`), but exotic environments remain a risk.
- **Test runtime.** Level-2 tests spawn external processes; budget ~5-10s per test. With ~25 new Level-2 tests this adds ~3 minutes to a full local test run on a host that has all terminals. Acceptable; document as such.
- **Test isolation.** Each Level-2 test spawns a fresh shell. This guarantees no state bleed but adds ~1-2s of shell startup overhead per test. If test runtime becomes unacceptable, a future optimization could share shells within a test file with explicit reset logic.
- **Binary resolution.** Tests depend on `assert_cmd::cargo::cargo_bin!()` locating the compiled binary in the cargo target directory. Cross-compilation scenarios or custom target directories may require setting `CARGO_TARGET_DIR` or adjusting the harness.
- **Background-spawn caveat.** With `SpawnVisibility::Background`, WezTerm windows live in the `biscuit-bg` workspace; if a developer happens to switch to that workspace mid-test they will see ephemeral panes appear and disappear. This is cosmetic — the harness still owns and tears down each pane via pane-id. If a future test needs to assert on real terminal *focus* state (e.g. cursor blink behavior), it must opt into `Foreground`.

---

## 9. Sequencing suggestion

1. **§5.1** (harness adoption) — unblocks everything.
2. **§5.2** (Level-1 capability tests) — quickest wins; expands real coverage of code that ships in every terminal interaction.
3. **§5.5** (prose / OSC8 / NO_COLOR) — small, high-confidence Level-2 tests; good way to validate the harness.
4. **§5.3** (image rendering) — highest value, but largest fixture/setup effort.
5. **§5.4** (diagrams) — depends on §5.3 fixtures.
6. **§5.6** (cursor hygiene) and **§5.7** (tooling) — polish.
