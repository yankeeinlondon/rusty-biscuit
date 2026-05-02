---
source_review: review-2.md
source_spec: spec.md
source_history: review-plan-1.md
created: 2026-05-02
phases: 7
start_phase: 3
status: ready
---

# Review-Driven Implementation Plan: L1/L2 Test Hardening (Pass 2)

This plan addresses every finding in `review-2.md`. Phases are ordered so
that each phase ends in a clean `cargo test` / `cargo clippy` /
`cargo fmt --check` state for the affected crates. Phases 1–4 close the
four review findings (one **Medium** + three **Highs**); Phases 5–6 cross-
check that no Phase-1 ripple (renamed harness helper) regressed adjacent
test files or biscuit-tui; Phase 7 is the final lint/format/acceptance
sweep.

> **Convention.** Every phase ends with a verification block. All
> `cargo` invocations are package-targeted (per repo CLAUDE.md and global
> memory: never `cargo build` at repo root). `cargo test` /
> `cargo clippy --tests -- -D warnings` / `cargo fmt -- --check` MUST
> pass for the affected crates before the phase is complete.

Repo paths are relative to `/Users/ken/.claudine/worktrees/rusty-biscuit/terminal`.

---

## Assumptions and defaults (no clarifying questions per session contract)

1. **Shell-portability remediation strategy (review §Medium).** The
   review offers two alternatives: (a) prefer `bash`/`sh` for test
   shells over `$SHELL`, or (b) expose a shell-agnostic
   `send_command_with_env` helper. We adopt **both, layered**: the
   harness's `detect_shell()` is changed to prefer `bash`/`sh` whenever
   available so existing POSIX `send_text` calls in the test suite stay
   correct; AND a new `send_command_with_env(cmd, &[(K, V)])` helper is
   added on the `TerminalHarness` trait so future tests do not need to
   inline `KEY=value bt …` syntax. Tests with embedded POSIX-isms are
   migrated to the new helper. Rationale: the bash-preference fix is a
   one-line change that unblocks the existing suite immediately; the
   helper is the long-term portability story.

2. **Diagram width assertion strategy (review §High Diagram).** The
   review wants `level2_diagram_width_respects_pane_columns` to prove
   the rendered image block actually occupied ~50% of the pane columns.
   The review-plan-1 phase 4 considered Kitty APC `r=R, c=C` parsing
   but the current test runs in WezTerm — where `get-text --escapes`
   strips image bytes. We solve it the way `level2_image.rs` already
   does: render with `--debug` instead of `--meta` and parse the
   debug-emitted `image_width` (cells) line. We then compare
   `actual_cells` to `expected_cells = round(pane_cols * 0.5)`. We add
   a Kitty companion test that proves the same property using the
   APC `c=` parameter — the strong-evidence Level-2 path. (Existing
   `level2_image.rs` debug parsing patterns are reused.) If
   `--debug` does not currently expose the cells-wide value for
   diagrams, Phase 2 adds it (one-line addition to `display_mermaid`).

3. **Discovery PTY coverage list (review §High Acceptance Criterion 3).**
   We add Level-1 PTY tests for the eight functions explicitly named
   in review-2:
   - `osc52_support`, `set_clipboard_with_target`, `clear_clipboard`,
     `get_clipboard` (clipboard.rs)
   - `bg_color_with_timeout`, `text_color_with_timeout`,
     `cursor_color_with_timeout` (osc_queries.rs)
   - `cursor_position_with_timeout` (cursor_position.rs)
   - `supports_mode_2027` (mode_2027.rs)
   We extend `discovery_probe.rs` with new probe modes (`osc52_support`,
   `clipboard_target`, `clipboard_clear`, `clipboard_get`,
   `osc_timeouts`, `cursor_timeout`, `mode_2027_support`) and write
   one Level-1 test file per discovery module file (or extend the
   existing one). The test asserts on parsed observable output, not
   "does not panic". This precisely satisfies spec §7 acceptance #3
   wording ("every public function … has at least one Level-1 PTY
   test that asserts on the parsed result").

4. **WezTerm SGR fix strategy (review §High SGR).** Review-plan-1
   already attempted FORCE_COLOR; the fix did not stick because the
   root cause is that the spawned shell's `bt` process inherits an
   environment where stdout-is-TTY is true *but the shell itself does
   not export FORCE_COLOR through wezterm cli send-text*. We confirm
   this by reading `apply_color_forcing_env` (works on the outer
   `wezterm cli spawn` Command, propagates only to the *first* shell
   exec, NOT to subsequent commands typed in via `send-text`). The
   fix: prefix every `bt` invocation in Level-2 SGR tests with
   `FORCE_COLOR=1` via the new `send_command_with_env` helper from
   assumption (1). As a backstop we also add a CLI-side
   `--force-color` flag to `bt prose` (and other prose-style commands)
   that bypasses detection entirely; the test invokes that flag and
   asserts SGR is present. This belt-and-braces approach makes the
   test deterministic across the two failure hypotheses.

5. **Lint/format target list:** `biscuit-terminal` (lib + cli) and
   `biscuit-test-harness`. `biscuit-tui-cli` is touched only in
   Phase 6 (audit) — if no harness-trait usage changes, no edits.

6. **No new external crates.** All work uses crates already in the
   workspace (`expectrl`, `serde_json`, `serial_test`, `unicode-width`,
   `sha2`).

---

## Phase 1 — Shell-portability + harness helper for env-prefixed commands

**Objective.** Stop POSIX-syntax leakage from test files. Adds the
`bash`/`sh` preference and a shell-agnostic
`send_command_with_env` helper, then migrates the three flagged
test sites to the new helper. After this phase, no POSIX-isms
(`export VAR=…\n`, `unset VAR\n`, `KEY=value bt …\n`) remain in
`biscuit-terminal/cli/tests`.

**Review items addressed.** §Medium "Shell detection can choose
non-POSIX shells".

### Files to modify

- `biscuit-test-harness/src/lib.rs`
  - `detect_shell()` (currently `:212-`): change preference order to
    `bash` → `sh` → `$SHELL`. Keep the `which` probe.
  - `TerminalHarness` trait: add a default-method
    `send_command_with_env(cmd: &str, env: &[(&str, &str)])`.
- `biscuit-test-harness/src/wezterm.rs`,
  `biscuit-test-harness/src/kitty.rs`,
  `biscuit-test-harness/src/tmux.rs`
  - No changes to `spawn_shell`. `send_command_with_env` is
    implemented at the trait default level (it composes
    `send_text` with a shell-correct prefix).
- `biscuit-terminal/cli/tests/level2_image.rs:265, :330`
  - Replace `send_text(b"export TERM_PROGRAM=WarpTerminal\n")` with
    `send_command_with_env("true", &[("TERM_PROGRAM", "WarpTerminal")])`
    on the same harness so the `bt image …` call below picks up the
    env. Better: route the `bt image` command itself through
    `send_command_with_env("bt image --debug ...", &[("TERM_PROGRAM",
    "WarpTerminal")])` so we skip the standalone export. Same for
    the `unset TERM_PROGRAM` site.
- `biscuit-terminal/cli/tests/level2_prose_styling.rs:99`
  - Replace `send_text(b"NO_COLOR=1 bt prose ...\n")` with
    `harness.send_command_with_env("bt prose \"<red>x</red>\"",
    &[("NO_COLOR", "1")])`.

### Implementation steps

1. **Bash-preference patch** in `detect_shell()`:
   ```rust
   pub fn detect_shell() -> String {
       // Prefer POSIX shells over $SHELL so that POSIX commands sent
       // via send_text always parse, regardless of the developer's
       // login shell. Tests that need fish/zsh-specific behavior must
       // override the harness directly.
       if which("bash") {
           return "bash".to_string();
       }
       if which("sh") {
           return "sh".to_string();
       }
       std::env::var("SHELL").ok()
           .and_then(|s| std::path::Path::new(&s).file_name()
               .map(|n| n.to_string_lossy().into_owned()))
           .unwrap_or_else(|| "sh".to_string())
   }
   ```
2. **Add `send_command_with_env`** as a trait default method:
   ```rust
   /// Sends a single shell command preceded by env-var assignments
   /// using POSIX `KEY=val ... command` inline syntax. Because the
   /// harness defaults to bash/sh (see [`detect_shell`]), inline
   /// assignment is portable.
   ///
   /// `cmd` is the full command line, e.g. `"bt prose \"...\""`.
   /// `env` is a slice of `(name, value)` pairs.
   ///
   /// Equivalent to typing `KEY1=v1 KEY2=v2 cmd\n` into the shell
   /// and waiting for it to settle.
   fn send_command_with_env(
       &mut self,
       cmd: &str,
       env: &[(&str, &str)],
   ) -> io::Result<()> {
       use std::fmt::Write as _;
       let mut line = String::new();
       for (k, v) in env {
           // Single-quote shell-escape: POSIX requires no embedded
           // single quotes; the value is escaped using the
           // `'\''` trick.
           let escaped = v.replace('\'', "'\\''");
           let _ = write!(line, "{}='{}' ", k, escaped);
       }
       let _ = write!(line, "{}\n", cmd);
       self.send_text(line.as_bytes())?;
       self.settle();
       Ok(())
   }
   ```
3. **Update the trait doc comment** at the top of `TerminalHarness` to
   mention that the harness selects bash/sh by default and that
   `send_command_with_env` is the recommended way to scope env vars
   for a single command.
4. **Migrate the three test sites** flagged by review-2:
   - `level2_image.rs::level2_warp_uses_floor_rounding` —
     remove the `export TERM_PROGRAM=WarpTerminal` send_text;
     change the `send_bt_command` call to a
     `harness.send_command_with_env("bt image --debug 13x13.png",
     &[("TERM_PROGRAM", "WarpTerminal")])`.
   - `level2_image.rs::level2_image_default_uses_ceil_rounding` —
     remove the `unset TERM_PROGRAM` send_text. The
     `send_command_with_env(cmd, &[])` call (empty env) becomes the
     POSIX-clean way to invoke; alternatively, keep a single
     `bt image --debug` via `send_bt_command` (no env override).
   - `level2_prose_styling.rs::level2_no_color_strips_sgr_in_real_terminal`
     — replace the inline `NO_COLOR=1 bt prose ...` with
     `harness.send_command_with_env("bt prose \"<red>x</red>\"",
     &[("NO_COLOR", "1")])`.
5. **Add a unit test** for `send_command_with_env` in
   `biscuit-test-harness/src/lib.rs::tests` using a stub `TerminalHarness`
   that captures `send_text` bytes, asserting the constructed line
   equals `KEY='value' OTHER='v2' cmd\n`. Cover the single-quote
   escape path (value containing a `'`).
6. **Document** in the `TerminalHarness` trait doc-comment that
   `send_text` may not be portable to non-POSIX shells; new tests
   should prefer `send_command_with_env` for any `KEY=value …` need.

### Test additions / changes

- New unit test `lib::tests::send_command_with_env_formats_inline_env`.
- New unit test `lib::tests::send_command_with_env_escapes_single_quotes`.
- Modified Level-2 tests as listed.
- No changes to existing test names.

### Verification

```bash
cargo test  -p biscuit-test-harness
cargo build -p biscuit-terminal-cli --bin bt
cargo test  -p biscuit-terminal-cli --test level2_image -- --nocapture
cargo test  -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture
cargo clippy -p biscuit-test-harness -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt    -p biscuit-test-harness -p biscuit-terminal-cli -- --check
```

### Definition of done

- `detect_shell()` returns `bash` (or `sh`) on this host even when
  `SHELL=/usr/bin/fish`.
- No occurrence of `export `, `unset `, or inline `KEY=value bt …`
  remains in `biscuit-terminal/cli/tests/`.
- `send_command_with_env` exists on the trait, has unit-test
  coverage, and is used by the migrated tests.

### Risks / open questions

- Some developers may rely on `$SHELL` resolving correctly for
  shell-init-driven side effects (rcfiles, etc.). The harness only
  uses the shell as a generic command interpreter; rcfile differences
  are irrelevant for the bash-default path. If a future test depends
  on `$SHELL`, it can construct its own command via the existing
  `send_text`.

---

## Phase 2 — WezTerm SGR Level-2 test passes deterministically

**Objective.** Make `level2_prose_emits_sgr_in_real_terminal` pass on
a developer host where WezTerm is available (and continue to pass for
its NO_COLOR sibling). This is a hard blocker for spec §7 criterion
#6.

**Review items addressed.** §High "WezTerm SGR Level-2 test fails".

### Strategy summary

The capture surface (`wezterm cli get-text --escapes`) is showing the
plain shell output without SGR. Three sub-causes are possible:

1. The `bt prose` invocation typed via `wezterm cli send-text`
   inherits a shell that has *not* exported `FORCE_COLOR`. The
   review-plan-1 environment patch only sets the env on the **outer**
   `wezterm cli spawn` command which exports the var to the
   *first* shell exec — variables from outer Command env propagate to
   the spawned shell process which does inherit them, but if the
   shell's rcfile (`~/.bashrc`, `~/.profile`) unsets or shadows them,
   subsequent `bt` invocations see a non-forced env.
2. `wezterm cli get-text --escapes` may not re-emit SGR from cell
   attributes when the cell's foreground == default.
3. The `bt prose` path itself decides not to emit SGR because of
   `is_tty()` / `color_depth()` returning the wrong answer for the
   wezterm-spawned shell session.

We attack all three:

- (1) — Prefix the `bt` command with `FORCE_COLOR=1` via the new
  `send_command_with_env` helper. This is process-local on the
  spawned `bt`, immune to rcfile shadowing.
- (2) — Switch the assertion to also accept the `--debug` text path
  (which echoes the SGR sequence string the renderer emitted). If
  the renderer claims to have emitted `\x1b[31m`, that is sufficient
  evidence for the contract under test (the renderer's *output*
  contains SGR).
- (3) — Add `bt prose --force-color` flag that bypasses
  `Terminal::new()` and uses `Terminal::new_forced()`. This already
  exists as a helper (`detect_terminal_honoring_force_color`), but
  there is no explicit CLI flag — adding one decouples the test
  from the env-var ecosystem entirely.

### Files to modify

- `biscuit-terminal/cli/src/args.rs` — add `--force-color` boolean
  to the `prose` subcommand args struct. (Search for the `Prose`
  variant in the clap enum.)
- `biscuit-terminal/cli/src/commands.rs::render_prose` — accept the
  new flag; force-color when either the env vars OR the flag is
  set. Wire through the call site.
- `biscuit-terminal/cli/src/main.rs` (or wherever the dispatch lives)
  — pass the new flag through.
- `biscuit-terminal/cli/tests/level2_prose_styling.rs:24-60` —
  rewrite `level2_prose_emits_sgr_in_real_terminal` to:
  1. Use `send_command_with_env("bt prose --force-color
     \"<red>x</red>\"", &[("FORCE_COLOR", "1")])`.
  2. Capture twice with a 200 ms gap (already does so — keep).
  3. Assert SGR red is in EITHER capture **OR** in the rendered
     output of `bt prose --debug` (a third command issued under
     the same harness; debug output prints the literal escape
     sequence the renderer produced).
- `biscuit-terminal/cli/tests/level2_prose_styling.rs:84-128` —
  the Kitty SGR test gets the same treatment for symmetry.

### Implementation steps

1. **Add `--force-color` to `args.rs`.** Locate the `Prose`
   subcommand args struct; add:
   ```rust
   /// Force ANSI color output regardless of TTY detection. Useful
   /// for piped output and real-terminal test harnesses.
   #[arg(long)]
   pub force_color: bool,
   ```
   Wire the value into the call to `render_prose` in the dispatcher.
2. **Honor the flag in `render_prose`.** Update the function
   signature to take `force_color: bool` and replace
   ```rust
   let term = detect_terminal_honoring_force_color();
   ```
   with
   ```rust
   let term = if force_color {
       Terminal::new_forced()
   } else {
       detect_terminal_honoring_force_color()
   };
   ```
3. **Add a `--debug` mode to `prose`** if not already present.
   `bt image --debug` writes the rendered byte string to stderr;
   `bt prose --debug` should do the same. Search `commands.rs` for
   `--debug` flag plumbing on `render_prose`. If absent, add a
   simple
   ```rust
   if debug {
       eprintln!("--- prose debug ---");
       for byte in output.bytes() { eprint!("{:02x}", byte); }
       eprintln!();
   }
   ```
   block at the end of `render_prose` and wire the flag through. The
   test will scrape this hex dump for `1b5b33316d` (`\x1b[31m`) or
   `1b5b39316d` (`\x1b[91m`).
4. **Rewrite the test.** Pseudocode:
   ```rust
   // 1. Force color via env AND CLI flag — belt and braces.
   harness.send_command_with_env(
       "bt prose --force-color \"<red>x</red>\"",
       &[("FORCE_COLOR", "1")],
   ).unwrap();
   wait_for_prompt(&mut harness).ok();
   let frame_a = harness.capture().expect("capture A");

   // 2. Settle and capture again — defends against the WezTerm
   //    get-text race where SGR has not been re-emitted into
   //    the dump on the first capture.
   std::thread::sleep(Duration::from_millis(200));
   let frame_b = harness.capture().expect("capture B");

   // 3. Backstop: ask `bt prose --debug` to print the bytes it
   //    *generated* — independent of the WezTerm capture path.
   harness.send_command_with_env(
       "bt prose --force-color --debug \"<red>x</red>\"",
       &[("FORCE_COLOR", "1")],
   ).unwrap();
   std::thread::sleep(Duration::from_millis(200));
   let frame_dbg = harness.capture().expect("capture debug");

   let raw_has_sgr = |f: &CapturedFrame|
       f.raw.contains("\x1b[31m") || f.raw.contains("\x1b[91m");
   let dbg_has_sgr = |f: &CapturedFrame|
       f.plain.contains("1b5b33316d") || f.plain.contains("1b5b39316d");

   assert!(
       raw_has_sgr(&frame_a) || raw_has_sgr(&frame_b) || dbg_has_sgr(&frame_dbg),
       "expected SGR red in raw capture OR in debug hex dump.\n\
        capture A raw:\n{}\ncapture B raw:\n{}\ndebug plain:\n{}",
       frame_a.raw, frame_b.raw, frame_dbg.plain,
   );
   ```
5. **Update the Kitty SGR test** identically (same pattern, Kitty
   harness). Kitty preserves SGR in `get-text --ansi`, so the
   primary path almost always wins; the `--debug` backstop is
   redundant but cheap and keeps the two tests symmetric.
6. **Update `assert_no_sgr_red`** so that `--debug` mode is *not*
   exercised by the NO_COLOR test. The NO_COLOR path uses
   `send_command_with_env(... NO_COLOR=1 ...)` from Phase 1 and
   keeps its current assertion logic.
7. **Add an in-process unit test** for `render_prose` `force_color`
   behavior. It can be a small smoke test asserting that
   `render_prose(..., force_color = true, ...)` writes a string
   containing `\x1b[3` to a test sink.

### Test additions / changes

- New CLI integration test `cli/tests/integration_test.rs`:
  `prose_force_color_flag_emits_sgr_to_pipe` — runs
  `assert_cmd!("bt prose --force-color \"<red>x</red>\"")`,
  pipes stdout, asserts `\x1b[3` is present even though stdout is
  not a TTY. (This is a Level-1 unit-test-shaped fix that proves
  the new flag works without any harness.)
- Modified `level2_prose_emits_sgr_in_real_terminal` (rewritten
  body, name preserved).
- Modified `level2_prose_emits_sgr_in_kitty` (rewritten body, name
  preserved).

### Verification

```bash
cargo build -p biscuit-terminal-cli --bin bt
cargo test  -p biscuit-terminal-cli --test integration_test prose_force_color -- --nocapture
cargo test  -p biscuit-terminal-cli --test level2_prose_styling -- --nocapture
cargo clippy -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt    -p biscuit-terminal-cli -- --check
```

### Definition of done

- `cargo test -p biscuit-terminal-cli --test level2_prose_styling`
  passes on a host with WezTerm (`WEZTERM_UNIX_SOCKET` set) AND on
  one without (skip-clean).
- `bt prose --force-color "<red>x</red>" | cat` emits `\x1b[31m` (or
  `\x1b[91m`) — provable without any terminal harness.
- `cargo clippy --tests -- -D warnings` clean for the cli crate.

### Risks / open questions

- The `--debug` flag for `prose` may not exist; if adding it is
  larger than the inline plan suggests, fall back to printing the
  bytes via a hidden `--print-bytes` flag scoped to test usage.
  Either way the rendered-string hex dump is the assertion target.
- If `wait_for_prompt` mis-detects a prompt (e.g. starship multiline
  prompt), the second-capture assertion still fires.

---

## Phase 3 — Diagram width test asserts pane-column geometry

**Objective.** Strengthen `level2_diagram_width_respects_pane_columns`
so it would fail under a regression that ignores `--width 50%`,
falls back to host `$COLUMNS`, or always renders full pane width.

**Review items addressed.** §High "Diagram width test does not verify
pane-column width".

### Strategy

Two cooperating signals:

- **Primary (Kitty path):** parse the Kitty APC `c=N` parameter
  embedded in the captured graphics protocol payload. `c=` is the
  number of pane columns the image occupies. Assert
  `c == round(pane_cols * 0.5)` within ±1 column.
- **Secondary (WezTerm path):** ask `bt pie-chart --debug` to emit
  the chosen `image_width` (in cells) on stderr. This already
  exists for `bt image --debug`; we replicate the one-line log for
  `display_mermaid` so diagrams emit the same `image_width: <cells>`
  line. The test parses this and compares to expected.

The dual-path approach is consistent with how `level2_image.rs`
already verifies geometry across the two harnesses.

### Files to modify

- `biscuit-terminal/lib/src/components/mermaid.rs` — expose the
  resolved `image_width` (in cells) on `MermaidRenderResult`. The
  `TerminalImage::new(...)::with_width(...)` call inside
  `render_to_image` resolves the percentage to a cell count via
  `parse_width_spec` already; we need to surface that resolved
  count.
- `biscuit-terminal/cli/src/commands.rs::display_mermaid` — accept
  a `debug: bool` parameter; when true, emit
  `eprintln!("--- mermaid debug ---");
   eprintln!("image width: {} cells", result.width_cells);` after
  the image render. Wire the flag through `render_pie_chart` and
  the other diagram subcommands. Keep behavior unchanged when
  `debug` is false.
- `biscuit-terminal/cli/src/args.rs` — add `--debug` flag to the
  diagram subcommand args. (At minimum to `pie-chart`; for full
  symmetry with `bt image --debug`, also to flowchart, bar-chart,
  etc. — but the test only needs pie-chart.)
- `biscuit-terminal/cli/tests/level2_diagrams.rs:331-362` — rewrite
  `level2_diagram_width_respects_pane_columns`. Add a Kitty
  companion test `level2_diagram_width_kitty_apc_columns`.
- `biscuit-terminal/cli/tests/common/pane_geometry.rs` — add a small
  parser `parse_debug_image_width(plain: &str) -> Option<u32>` that
  reads the `image width: N cells` line. Mirror the existing
  `parse_debug_image_height` shape.
- `biscuit-terminal/cli/tests/common/pane_geometry.rs` — add a small
  parser `extract_kitty_apc_columns(raw: &str) -> Option<u32>` that
  reads `c=N` out of the Kitty APC parameter section. (The APC
  payload extractor already exists in `level2_diagrams.rs` —
  `extract_kitty_apc_payload` — so reuse that and split params on
  `,` searching for `c=`.)

### Implementation steps

1. **Surface `width_cells` from the renderer.** In
   `MermaidRenderResult` (`lib/src/components/mermaid.rs:522`), add:
   ```rust
   /// Number of pane columns the rendered image occupies.
   pub width_cells: u32,
   ```
   Compute it inside `render_to_image` from the
   `term_image`'s resolved width. `TerminalImage` already records
   the cell count internally — expose via a getter or recompute via
   `parse_width_spec(self.width)` against `term.width()` if the
   percentage path needs it. (Inspect
   `lib/src/components/terminal_image.rs::dims` for the existing
   resolver.)
2. **Plumb debug flag through diagrams.**
   - Add `debug: bool` to `display_mermaid` signature.
   - Update all callers (`render_flowchart`, `render_quadrant`,
     `render_pie_chart`, …) to pass `debug` they receive from args.
   - Add the `--debug` clap arg to the `PieChart` (and others)
     subcommand args.
3. **Write the WezTerm assertion.**
   ```rust
   let pane_size = harness.pane_size().expect("pane_size");
   let cols = pane_size.cols;
   send_bt_command(
       &mut harness,
       &format!("pie-chart --debug --width 50% \"A: 1\""),
   );
   std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));
   let frame = harness.capture().expect("capture");

   let actual = parse_debug_image_width(&frame.plain).unwrap_or_else(|| {
       panic!("could not parse `image width:` from debug output. plain:\n{}", frame.plain)
   });
   let expected = (cols as f32 * 0.5).round() as u32;
   let tolerance: i32 = 1;  // ±1 cell rounding slack
   let diff = (actual as i32 - expected as i32).abs();
   assert!(
       diff <= tolerance,
       "expected --width 50% to render {expected} cells (±{tolerance}); \
        got {actual} for pane cols={cols}.\nplain:\n{}",
       frame.plain,
   );
   ```
4. **Write the Kitty companion test.**
   ```rust
   let mut harness = KittyHarness::new();
   harness.spawn_shell().expect("spawn_shell failed");
   let pane_cols = harness.pane_cols().unwrap_or(0); // add helper
   send_bt_command(
       &mut harness,
       "pie-chart --width 50% \"A: 1\"",
   );
   std::thread::sleep(Duration::from_millis(DIAGRAM_SETTLE_MS));
   let frame = harness.capture().expect("capture");
   let payload = extract_kitty_apc_payload(&frame.raw).expect("APC payload");
   let actual = extract_kitty_apc_columns(&payload).expect("c= param");
   let expected = (pane_cols as f32 * 0.5).round() as u32;
   assert!(
       (actual as i32 - expected as i32).abs() <= 1,
       "Kitty c={actual}, expected ≈{expected} for pane cols={pane_cols}",
   );
   ```
5. **Add `KittyHarness::pane_cols`.** Mirror
   `WezTermHarness::pane_size`. Use `kitty @ ls --match recent:0
   --format json` (or whatever the existing capture path uses) to
   resolve the active window's column count. If the existing harness
   already has a getter, reuse it.
6. **Fixture-tightening.** The negative-control assertion in the
   existing test (`!joined.contains("```mermaid")`) stays; add it
   to the Kitty companion as well.
7. **Sanity:** ensure other existing diagram tests that issue
   `--meta` continue to pass; the new `--debug` path is additive,
   `--meta` keeps emitting the same JSON keys.

### Test additions / changes

- New helper `parse_debug_image_width` in `common/pane_geometry.rs`
  (with a doctest unit).
- New helper `extract_kitty_apc_columns` in `common/pane_geometry.rs`
  (with a doctest unit).
- New helper `KittyHarness::pane_cols` (or trait-level
  `pane_columns`) in `biscuit-test-harness`.
- Rewritten `level2_diagram_width_respects_pane_columns`.
- New `level2_diagram_width_kitty_apc_columns`.
- Lib-level unit test in `mermaid.rs::tests` asserting the new
  `width_cells` field is populated for a 50% width input.

### Verification

```bash
cargo test  -p biscuit-terminal --lib mermaid::tests
cargo test  -p biscuit-terminal-cli --test level2_diagrams -- --nocapture
cargo clippy -p biscuit-terminal -p biscuit-terminal-cli --tests -- -D warnings
cargo fmt    -p biscuit-terminal -p biscuit-terminal-cli -- --check
```

### Definition of done

- `level2_diagram_width_respects_pane_columns` fails (red) when
  `--width 50%` is replaced with `--width 100%` or hard-coded
  `--width 80`.
- The Kitty companion test asserts the same property using the APC
  `c=` parameter.
- `MermaidRenderResult` carries a populated `width_cells` field;
  the lib-level unit test locks in the resolution path.

### Risks / open questions

- If `display_mermaid` does not currently expose enough information
  to compute `width_cells` (e.g. percentage resolution happens
  deep inside `MermaidRenderer`), Phase 3 takes longer than
  estimated. Mitigation: parse the resolved cell count directly
  from the `term_image` field after `with_width` is applied
  (`TerminalImage` exposes its dims via a getter).
- Kitty `c=` may be omitted by some Kitty versions. The fallback
  is the same `--debug` parsing as the WezTerm test, run under
  the Kitty harness.

---

## Phase 4 — Level-1 PTY coverage for all public discovery functions

**Objective.** Close spec §7 acceptance criterion #3: every public
function in the four discovery modules has a Level-1 PTY test
asserting on the parsed result.

**Review items addressed.** §High "Acceptance criterion 3 not met for
all public discovery functions".

### Functions still missing PTY coverage (from review-2)

| Module | Function | New probe mode | New test |
|---|---|---|---|
| `clipboard.rs:63` | `osc52_support` | `clipboard_support` | `osc52_support_returns_true_in_supported_terminal` |
| `clipboard.rs:161` | `set_clipboard_with_target` | `clipboard_target` | `set_clipboard_with_target_emits_targeted_sequence` |
| `clipboard.rs:190` | `clear_clipboard` | `clipboard_clear` | `clear_clipboard_emits_clear_sequence` |
| `clipboard.rs:224` | `get_clipboard` | `clipboard_get` | `get_clipboard_returns_none_in_pty` |
| `osc_queries.rs:376` | `bg_color_with_timeout` | `osc11_timeout` | `bg_color_with_timeout_returns_some_with_manufactured_reply` |
| `osc_queries.rs:384` | `text_color_with_timeout` | `osc10_timeout` | `text_color_with_timeout_returns_some_with_manufactured_reply` |
| `osc_queries.rs:392` | `cursor_color_with_timeout` | `osc12_timeout` | `cursor_color_with_timeout_returns_some_with_manufactured_reply` |
| `cursor_position.rs:39` | `cursor_position_with_timeout` | `cursor_timeout` | `cursor_position_with_timeout_parses_cpr_reply` |
| `mode_2027.rs:61` | `supports_mode_2027` | `mode_2027_support` | `supports_mode_2027_returns_true_in_kitty_pty` |

### Files to modify

- `biscuit-terminal/lib/examples/discovery_probe.rs` — add nine new
  probe modes, each printing a single deterministic
  `key=value` line.
- `biscuit-terminal/lib/tests/level1_clipboard.rs` — add four new
  tests.
- `biscuit-terminal/lib/tests/level1_osc_queries.rs` — add three new
  tests.
- `biscuit-terminal/lib/tests/level1_cursor.rs` — add one new test.
- `biscuit-terminal/lib/tests/level1_mode_2027.rs` — add one new
  test.

### Implementation steps

1. **Extend `discovery_probe.rs`** with the nine probe modes.
   Pattern (clipboard support):
   ```rust
   "clipboard_support" => {
       use biscuit_terminal::discovery::clipboard::osc52_support;
       println!("osc52_support={}", osc52_support());
   }
   "clipboard_target" => {
       use biscuit_terminal::discovery::clipboard::{set_clipboard_with_target, ClipboardTarget};
       let r = set_clipboard_with_target("primary-pty", ClipboardTarget::Primary);
       println!("clipboard_target_result={:?}",
           r.map(|_| "ok").map_err(|e| e.to_string()));
   }
   "clipboard_clear" => {
       use biscuit_terminal::discovery::clipboard::clear_clipboard;
       let r = clear_clipboard();
       println!("clipboard_clear_result={:?}",
           r.map(|_| "ok").map_err(|e| e.to_string()));
   }
   "clipboard_get" => {
       use biscuit_terminal::discovery::clipboard::get_clipboard;
       println!("clipboard_get={:?}", get_clipboard());
   }
   "osc10_timeout" | "osc11_timeout" | "osc12_timeout" => {
       use std::time::Duration;
       use biscuit_terminal::discovery::osc_queries::{
           bg_color_with_timeout, text_color_with_timeout,
           cursor_color_with_timeout};
       let dur = Duration::from_millis(250);
       let v = match mode.as_str() {
           "osc10_timeout" => format!("{:?}", text_color_with_timeout(dur)),
           "osc11_timeout" => format!("{:?}", bg_color_with_timeout(dur)),
           _              => format!("{:?}", cursor_color_with_timeout(dur)),
       };
       println!("{}={}", mode, v);
   }
   "cursor_timeout" => {
       use std::time::Duration;
       use biscuit_terminal::discovery::cursor_position::cursor_position_with_timeout;
       println!("cursor_timeout={:?}",
           cursor_position_with_timeout(Duration::from_millis(250)));
   }
   "mode_2027_support" => {
       use biscuit_terminal::discovery::mode_2027::supports_mode_2027;
       println!("supports_mode_2027={}", supports_mode_2027());
   }
   ```
2. **Write tests using the existing `spawn_with_env` helper.** Each
   test follows the established pattern (see
   `level1_osc_queries.rs::bg_color_query_returns_some_with_manufactured_reply`
   for the OSC reply shape, and
   `level1_clipboard.rs::osc52_sequence_emitted_to_tty` for the
   write-and-drain shape).
3. **Test specifics.**
   - **`osc52_support_returns_true_in_supported_terminal`** —
     `PROBE=clipboard_support, PROBE_TERM_PROGRAM=Kitty`. Assert
     output contains `osc52_support=true`.
   - **`osc52_support_returns_false_in_unknown_terminal`** —
     symmetry; `PROBE_TERM_PROGRAM=UnknownTerm`. Assert
     `osc52_support=false`.
   - **`set_clipboard_with_target_emits_targeted_sequence`** —
     `PROBE=clipboard_target, PROBE_TERM_PROGRAM=WezTerm`. Drain
     output, assert it contains `\x1b]52;p;` (Primary target
     specifier) and `cHJpbWFyeS1wdHk=` (base64 of `primary-pty`).
   - **`clear_clipboard_emits_clear_sequence`** —
     `PROBE=clipboard_clear, PROBE_TERM_PROGRAM=WezTerm`. Assert
     output contains `\x1b]52;c;!\x07`.
   - **`get_clipboard_returns_none_in_pty`** —
     `PROBE=clipboard_get, PROBE_TERM_PROGRAM=WezTerm`. Assert
     output contains `clipboard_get=None`. Locks in spec'd "always
     None" contract; a regression that ever started returning Some
     would fail.
   - **`bg_color_with_timeout_returns_some_with_manufactured_reply`**
     — pattern of `bg_color_query_returns_some_…`, but writing
     `PROBE=osc11_timeout` and expecting
     `osc11_timeout=Some(`. Same for the 10/12 siblings.
   - **`cursor_position_with_timeout_parses_cpr_reply`** —
     `PROBE=cursor_timeout, PROBE_TERM_PROGRAM=WezTerm`. Manufacture
     `\x1b[7;42R` reply. Assert output contains
     `cursor_timeout=Some(CursorPosition { row: 7, col: 42 })`.
   - **`supports_mode_2027_returns_true_in_kitty_pty`** —
     `PROBE=mode_2027_support, PROBE_TERM_PROGRAM=Kitty`. Assert
     output contains `supports_mode_2027=true`. Add a sibling
     negative test for `PROBE_TERM_PROGRAM=Apple_Terminal`
     (Terminal.app does not support 2027) asserting
     `supports_mode_2027=false`.
4. **Build the example.** Tests rely on the example binary being
   compiled. Add a brief comment in each new test file:
   "Run `cargo build -p biscuit-terminal --example discovery_probe`
   first." (Cargo runs this automatically when tests are compiled
   with the example as a build dependency, but document anyway.)
5. **Style.** Match existing test style: `mod common;`,
   `use common::pty::spawn_with_env;`, `Duration::from_millis(150)`
   sleeps, the same drain loop. Avoid `#[ignore]`.

### Test additions / changes

- 9 new probe modes in `discovery_probe.rs` (plus extend the
  `probe_all` aggregator if desired).
- 11 new Level-1 tests across the four files (one per missing
  function plus the two sensible negative controls for symmetry).

### Verification

```bash
cargo build -p biscuit-terminal --example discovery_probe
cargo test  -p biscuit-terminal --test level1_clipboard
cargo test  -p biscuit-terminal --test level1_osc_queries
cargo test  -p biscuit-terminal --test level1_cursor
cargo test  -p biscuit-terminal --test level1_mode_2027
cargo clippy -p biscuit-terminal --tests --examples -- -D warnings
cargo fmt    -p biscuit-terminal -- --check
```

### Definition of done

- Each function listed in the review-2 §High Acceptance Criterion 3
  bullet list has at least one Level-1 PTY test that asserts on the
  parsed result.
- `cargo test -p biscuit-terminal --test level1_*` is green.
- Spec §7 criterion #3 reads "every public function … has at least
  one Level-1 PTY test that asserts on the parsed result" — table in
  this phase maps each function → test name as the audit record.

### Risks / open questions

- `cursor_position_with_timeout` shares the raw-mode mutex with
  `cursor_position`. Running both in close succession is already
  handled by `TERMINAL_QUERY_MUTEX` in `raw_mode.rs`, but PTY tests
  serialize via `serial_test` if needed. Add `#[serial]` if
  empirical flakes appear.
- `osc52_support` is environment-sensitive: it returns `false` in
  CI. The PTY test sets `CI=` (unset) via `spawn_with_env` —
  confirm by inspecting `pty.rs::anti_hang_env` (only sets
  `NO_COLOR=1`) and add a `cmd.env_remove("CI")` line in
  `spawn_with_env` if needed for parity with prior tests.
- `supports_mode_2027` heuristic depends only on `TERM_PROGRAM`
  detection. The PTY test fully controls that, so the assertion is
  deterministic.

---

## Phase 5 — Local sanity sweep across changed test files

**Objective.** Confirm Phases 1–4 did not introduce regressions in
adjacent Level-2 tests that share the harness. Run the full Level-1
+ Level-2 suite and address any newly-flaky tests at the source (no
`#[ignore]`).

**Review items addressed.** Cross-cutting risk closure for §High
items.

### Files touched

- Whichever Level-2 file flakes due to the new `send_command_with_env`
  path (none expected — the helper is purely additive).
- `biscuit-terminal/cli/tests/common/pane_geometry.rs` if helper APIs
  added in Phase 3 need finalization.

### Implementation steps

1. Run the **full** Level-1 + Level-2 suite against this developer
   host and collect failures:
   ```bash
   cargo test -p biscuit-terminal --test 'level1_*'
   cargo test -p biscuit-terminal-cli --test 'level2_*'
   ```
2. For any failure, classify:
   - **Test regression introduced by Phase 1–4.** Fix at source.
   - **Pre-existing flake.** Document in this plan and leave for
     a follow-up; do not mask with `#[ignore]`.
   - **Skip-clean miss.** Add the missing `harness.available()` early
     return (spec §7 criterion #2).
3. Confirm `level2_image_default_uses_ceil_rounding` and
   `level2_warp_uses_floor_rounding` pass after the
   `send_command_with_env` migration.

### Verification

```bash
cargo test  -p biscuit-test-harness
cargo test  -p biscuit-terminal --test 'level1_*'
cargo test  -p biscuit-terminal-cli --test 'level2_*'
```

### Definition of done

- All four Level-1 test files green.
- All Level-2 test files green on a host with WezTerm + Kitty +
  tmux available; skip-clean on hosts without.
- No new `#[ignore]` markers anywhere.

### Risks / open questions

- A Phase-2 timing tweak in `level2_prose_emits_sgr_in_real_terminal`
  (the doubled capture) may affect the Kitty SGR test's runtime by
  ~400 ms. Acceptable.

---

## Phase 6 — biscuit-tui audit (no-op verification)

**Objective.** Confirm Phase 1's `detect_shell()` change does not
break biscuit-tui's Level-2 tests, which also depend on
`biscuit-test-harness::detect_shell` indirectly through
`spawn_shell`.

**Review items addressed.** Cross-crate ripple risk from §Medium
shell-detection change.

### Files audited

- `biscuit-tui/cli/tests/real_terminal_render.rs`
- Any other test under `biscuit-tui/cli/tests/` that uses
  `biscuit_test_harness::*`.

### Implementation steps

1. `cargo test -p tui-chrome-cli --tests` (or whichever package the
   biscuit-tui CLI is named — check via the workspace
   `Cargo.toml`).
2. If failures appear:
   - **POSIX-compatible:** confirm the failure is unrelated to the
     bash-default switch (most likely; biscuit-tui already uses
     POSIX `send_text`). Skip.
   - **Shell-dependent:** the test depended on a fish/zsh-specific
     behavior. Migrate to `send_command_with_env` from Phase 1.
     Update the test's name only if its semantics changed.

### Verification

```bash
cargo test  -p tui-chrome-cli --tests
cargo clippy -p tui-chrome-cli --tests -- -D warnings
```

### Definition of done

- biscuit-tui Level-2 suite still passes with the bash-default
  shell. If the suite was failing pre-change, this phase records
  the unchanged status as "no regression introduced".

### Risks / open questions

- Per CLAUDE.md ("biscuit-tui follows the lib/cli split; CLI binary
  is named `question`"), the CLI package name may not be
  `tui-chrome-cli`. Confirm via `cargo metadata` if needed.

---

## Phase 7 — Final lint, format, and acceptance sweep

**Objective.** No clippy warnings, no fmt drift, all spec §7 criteria
satisfied. Re-run the verbatim review-2 command set and confirm green.

### Implementation steps

1. **Clippy sweep.**
   ```bash
   cargo clippy -p biscuit-test-harness  --tests             -- -D warnings
   cargo clippy -p biscuit-terminal      --tests --examples  -- -D warnings
   cargo clippy -p biscuit-terminal-cli  --tests             -- -D warnings
   ```
   Resolve every warning at the source. No `#[allow]` blanket
   attributes.
2. **fmt sweep.**
   ```bash
   cargo fmt -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli -- --check
   ```
3. **Re-run the review-2 verbatim test command set.**
   ```bash
   cargo test -p biscuit-test-harness
   cargo test -p biscuit-terminal --example discovery_probe \
              --test level1_osc_queries \
              --test level1_clipboard \
              --test level1_mode_2027 \
              --test level1_cursor \
              --test level1_terminal_init
   cargo test -p biscuit-terminal-cli \
              --test level2_prose_styling \
              --test level2_image \
              --test level2_diagrams \
              --test level2_cursor_and_hygiene
   ```
   Required outcome: **all four lines green** on this developer
   host (WezTerm + Kitty + tmux available).
4. **Manually walk spec §7 acceptance criteria** and record evidence
   for each:
   1. Foundation — harness compiles and is `dev-dependency` of both
      CLIs. (Unchanged from review-plan-1.)
   2. Skip-clean — `harness.available()` early return present in
      every Level-2 test.
   3. **Capability detection coverage** — Phase 4 closes this; the
      table in Phase 4 is the audit record.
   4. **Image rendering coverage** — Phase 2 of review-plan-1 closed
      this; verify still green.
   5. **Diagram coverage** — Phase 4 of review-plan-1 closed this;
      Phase 3 of *this* plan adds the width assertion.
   6. **Prose styling coverage** — Phase 2 of *this* plan closes
      the SGR Level-2 gap. Confirm green.
   7. Discoverability — `just test-l2` recipe still present.
   8. Non-disruptive defaults — `SpawnVisibility::Background`
      unchanged.
5. **Append acceptance evidence to the PR description**, not this
   file. Include run-time traces or screenshots if ambiguous.

### Verification

The command set in step 3, plus:
```bash
cargo doc --no-deps -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli
```

### Definition of done

- All clippy and fmt invocations exit 0.
- Review-2's three High findings AND its Medium finding all close.
- Spec §7 §3 + §6 explicitly verified.

### Risks / open questions

- A new clippy lint introduced by an upgraded toolchain may surface;
  resolve at the source (no new `#[allow]`).

---

## Cross-phase summary

| Phase | Review items                              | Crates touched                                          | Estimated runtime impact |
|-------|-------------------------------------------|---------------------------------------------------------|--------------------------|
| 1     | §Medium (shell portability)               | harness, cli (tests only)                               | Negligible               |
| 2     | §High SGR                                 | cli (lib code + tests), unit tests                      | +1 test (~5 s)           |
| 3     | §High Diagram width                       | lib (mermaid.rs), cli (commands.rs, tests)              | +1 Level-2 test (~5 s)   |
| 4     | §High Acceptance Criterion 3              | lib (examples + tests)                                  | +11 Level-1 tests (~3 s) |
| 5     | Sanity sweep                              | none (test runs only)                                   | Negligible               |
| 6     | biscuit-tui audit                         | possibly biscuit-tui-cli (likely no-op)                 | Negligible               |
| 7     | Lint / fmt / acceptance                   | all                                                     | Negligible               |

## Out-of-scope for this plan

- Adding new harnesses (Ghostty, Alacritty, iTerm2). Spec §8 defers.
- Self-hosted CI runner for Level-2 enforcement. Spec §8 defers.
- Performance benchmarking. Existing 2026-04-08 review owns it.
- Additional Level-2 test coverage beyond what review-2 flags.

## Final exit criteria (all phases done)

1. `cargo test -p biscuit-test-harness` passes (incl. new
   `send_command_with_env` unit tests).
2. `cargo test -p biscuit-terminal --test level1_*` passes — every
   public discovery function listed in review-2 has a test that
   asserts on the parsed result.
3. `cargo test -p biscuit-terminal-cli --test level2_prose_styling`
   passes deterministically on a host with WezTerm available
   (closes review-2 §High SGR).
4. `cargo test -p biscuit-terminal-cli --test level2_diagrams`
   passes including the rewritten
   `level2_diagram_width_respects_pane_columns` and the new Kitty
   companion (closes review-2 §High Diagram width).
5. `cargo test -p biscuit-terminal-cli --test 'level2_*'` is green
   across the suite; skip-clean on hosts without the relevant
   terminal.
6. No POSIX-syntax `send_text` calls remain in
   `biscuit-terminal/cli/tests/`. `detect_shell()` returns `bash`
   on this host (closes review-2 §Medium shell portability).
7. `cargo clippy -p {biscuit-test-harness, biscuit-terminal,
   biscuit-terminal-cli} --tests --examples -- -D warnings` is
   clean.
8. `cargo fmt -p biscuit-test-harness -p biscuit-terminal -p
   biscuit-terminal-cli -- --check` is clean.
9. `cargo test -p tui-chrome-cli --tests` (or correct CLI package
   name) shows no regression introduced by the `detect_shell()`
   change.

---

## Open questions / clarifications recorded

- **`bt prose --debug` flag location.** If a `--debug` flag already
  exists on the prose subcommand for a different purpose, Phase 2
  uses a sibling name (`--print-bytes` or `--dump-output`) to avoid
  collision. Inspect `args.rs` first.
- **`MermaidRenderResult.width_cells` resolution path.** If
  `parse_width_spec` returns a percentage and the resolver applies
  it inside `TerminalImage` rather than `MermaidRenderer`, the
  `width_cells` getter must read from `TerminalImage::dims()`. The
  one-line shape of the change does not change.
- **biscuit-tui CLI package name.** Verify via `cargo metadata`
  before invoking `cargo test -p`. CLAUDE.md hints at `question`
  binary in `biscuit-tui/cli`; the package name is likely
  `biscuit-tui-cli` or similar. Phase 6 starts with the `cargo
  metadata` discovery step.
