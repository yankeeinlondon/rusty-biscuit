---
phases: 6
created: 2026-05-03
start_phase: 1
source_spec:
  - biscuit-terminal/features/2026-05-02-apple-terminal/spec.md
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - biscuit-terminal
---

# Execution Plan: Apple Terminal Prose Degradation

## Phase 1 - Confirm Current Capability Surface

Goal: verify the existing Apple Terminal profile and identify the exact seams the implementation and tests will use.

1. Inspect `biscuit-terminal/lib/src/discovery/detection.rs` and `biscuit-terminal/lib/src/terminal.rs` for Apple Terminal app detection, `osc_link_support`, and `UnderlineSupport` defaults.
2. Confirm `TERM_PROGRAM=Apple_Terminal` currently resolves to `TerminalApp::AppleTerminal`, `osc_link_support=false`, `underline_support.straight=true`, and `underline_support.double=false`.
3. Inspect `biscuit-terminal/lib/src/components/prose.rs` for all tag handlers that emit OSC8 and double underline sequences.
4. Confirm whether atomic token handling for `{{double-underline}}` can receive `Terminal` capability context. If it cannot, treat atomic token degradation as a follow-up unless the implementation can be made local without refactoring the parser.
5. Inspect `biscuit-terminal/lib/examples/discovery_probe.rs` and `biscuit-terminal/lib/tests/common/pty.rs` to decide whether the Level-1 tests can reuse the existing probe or need one new probe mode.

Validation checkpoint:

- `cargo test -p biscuit-terminal terminal_new_cascade_produces_consistent_fields_in_pty --test level1_terminal_init`
- Manual confirmation notes identify the exact existing fields and any gap between block tags and atomic tokens.

Parallelizable: Phase 1 steps 1-3 can be done independently of step 5.

## Phase 2 - Implement Prose Degradation

Goal: make `Prose::render(&Terminal)` suppress unsupported escape sequences for terminals matching Apple Terminal capabilities.

1. Update `block_tag_to_escape` in `biscuit-terminal/lib/src/components/prose.rs` for `<a href="...">description</a>`:
   - If `term.osc_link_support` is true and href resolves non-empty, keep current OSC8 open/close escapes.
   - If `term.osc_link_support` is false and href resolves non-empty, emit markdown-style visible output as `[description](resolved_href)`.
   - If href is empty, preserve current no-link behavior.
2. Add the smallest parser support needed for the unsupported-OSC8 branch to insert the closing markdown suffix after the linked body without rewriting unrelated tag parsing.
3. Update `block_tag_to_escape` for `<double-underline>` and `<uu>`:
   - If no `Terminal` is supplied, preserve optimistic output `\x1b[4:2m`.
   - If `term.underline_support.double` is true, emit `\x1b[4:2m`.
   - If double is false and straight is true, emit `\x1b[4m`.
   - If both double and straight are false, emit no opening or closing underline escapes.
4. Add a TODO comment near the Prose tag styling logic noting that `Prose` and `Style`/`Stylist` should eventually converge.
5. Add focused unit tests in `prose.rs` for direct `Terminal::builder()` cases:
   - OSC8 supported still emits OSC8.
   - OSC8 unsupported emits `[click here](https://example.com)` and no `\x1b]8;;`.
   - Double unsupported with straight supported emits `\x1b[4mimportant text\x1b[24m` or the existing reset sequence shape used by the parser.
   - Double and straight unsupported emits `important text` with no underline SGR.

Validation checkpoint:

- `cargo test -p biscuit-terminal components::prose`
- `cargo test -p biscuit-terminal --lib prose`

Dependency: Phase 1 must confirm whether the parser can support link suffix insertion locally.

## Phase 3 - Add Level-1 PTY Coverage

Goal: assert exact byte output when a PTY spoofs Apple Terminal.

1. Extend `biscuit-terminal/lib/examples/discovery_probe.rs` with a `PROBE=prose` mode.
2. Add probe environment inputs:
   - `PROBE_PROSE_INPUT` for source prose text.
   - Optional `PROBE_FORCE_OSC8`, `PROBE_FORCE_UNDERLINE_STRAIGHT`, and `PROBE_FORCE_UNDERLINE_DOUBLE` only if direct capability overrides are needed for AC-3.
3. In probe mode, build a `Terminal` from detected values or `Terminal::builder()` overrides and print the rendered `Prose` bytes to stdout.
4. Create `biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs` using `common::pty::spawn_with_env` and `try_read_available`.
5. Add strict assertions for `TERM_PROGRAM=Apple_Terminal`:
   - Link fixture output contains `[click here](https://example.com)`.
   - Link fixture output does not contain `\x1b]8;;`.
   - Double underline fixture output does not contain `\x1b[4:2m`.
   - Double underline fixture output contains `\x1b[4mimportant text`.
6. Add a separate no-underline-supported case using explicit probe overrides or a unit-level fallback if the real detection surface cannot manufacture that profile in a PTY.

Validation checkpoint:

- `cargo build -p biscuit-terminal --example discovery_probe`
- `cargo test -p biscuit-terminal --test level1_apple_terminal_prose`
- Existing Level-1 smoke: `cargo test -p biscuit-terminal --test level1_mode_2027 --test level1_terminal_init`

Dependency: Phase 2 must land first so the PTY tests assert implemented behavior.

Parallelizable: Step 1-3 probe work and step 4 test file scaffolding can be done in parallel after Phase 2 behavior is known.

## Phase 4 - Add AppleScript Terminal.app Harness

Goal: provide a Level-2 harness for Terminal.app that spawns, captures visible text, and cleans up automatically.

1. Add `biscuit-test-harness/src/apple_terminal.rs`.
2. Export the module from `biscuit-test-harness/src/lib.rs`.
3. Implement `AppleTerminalHarness::available()`:
   - Return false unless `cfg!(target_os = "macos")`.
   - Return false when `CI=1`.
   - Return false when `osascript` cannot address `application "Terminal"`.
4. Implement `spawn_shell()` or a Terminal.app-specific `spawn_command()` using `osascript -e 'tell application "Terminal" to do script "..."'`.
5. Ensure spawned commands prepend the cargo target binary directory to `PATH`, matching the existing shell-model harness contract.
6. Capture the spawned window ID and tab identity from AppleScript output in a stable struct field.
7. Minimize the window immediately after spawn with AppleScript so the test does not keep focus.
8. Implement `capture()` by asking Terminal.app for tab contents and returning `CapturedFrame { raw: text.clone(), plain: text }`.
9. Implement `Drop` cleanup that closes the spawned tab/window without saving or requiring manual intervention.
10. Add harness unit tests for shell escaping and skip behavior where possible without opening Terminal.app.

Validation checkpoint:

- `cargo check -p biscuit-test-harness`
- `cargo test -p biscuit-test-harness`
- On macOS outside CI, a manual harness smoke test can spawn `printf apple-terminal-smoke` and capture non-empty contents.

Dependency: independent of Phase 2 after expected command shape is known.

Parallelizable: Phase 4 can run in parallel with Phase 3 once the prose fixtures are settled.

## Phase 5 - Add Level-2 Terminal.app Tests

Goal: validate the real Terminal.app display path while respecting its plain-text-only capture limitation.

1. Create `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs`.
2. Use `AppleTerminalHarness::available()` and `skip_with_reason("Terminal.app via osascript, macOS only, CI disabled")` for clean skips.
3. Add `level2_apple_terminal_link_fallback_visible`:
   - Run `bt prose "<a href=\"https://example.com\">click here</a>"`.
   - Assert captured `plain` contains `click here` and `(https://example.com)`.
   - Assert captured `plain` does not contain visible escape garbage such as `]8;;` or `[4:2m`.
4. Add `level2_apple_terminal_double_underline_plain_text_visible`:
   - Run `bt prose "<double-underline>important text</double-underline>"`.
   - Assert captured `plain` contains `important text`.
   - Assert captured `plain` does not contain `[4:2m` or other literal SGR fragments.
5. Add `level2_apple_terminal_harness_lifecycle`:
   - Spawn a trivial command.
   - Assert capture is non-empty.
   - Let the harness drop and rely on cleanup; avoid manual cleanup in the test body.
6. Mark tests with `serial_test::serial(level2_terminal)` to avoid multiple Terminal.app windows/tabs racing each other.
7. Add the new test target to `biscuit-terminal/justfile` `test-l2`.

Validation checkpoint:

- `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose`
- `just -f biscuit-terminal/justfile test-l2` skips cleanly on non-macOS or CI and includes the new test on macOS.

Dependency: Phase 4 harness must exist. Phase 2 implementation must exist for content assertions.

## Phase 6 - Final Regression and Documentation Pass

Goal: prove acceptance criteria and update adjacent docs only where public behavior changed.

1. Run targeted implementation tests:
   - `cargo test -p biscuit-terminal --lib prose`
   - `cargo test -p biscuit-terminal --test level1_apple_terminal_prose`
2. Run existing Level-1 regression tests touched by probe changes:
   - `cargo test -p biscuit-terminal --test level1_mode_2027 --test level1_terminal_init`
3. Run harness checks:
   - `cargo test -p biscuit-test-harness`
   - `cargo check -p biscuit-terminal-cli --tests`
4. Run Level-2 Terminal.app tests locally on macOS outside CI:
   - `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose`
5. If public `Prose` behavior changed in documented examples, update `biscuit-terminal/docs/components/prose.md` and `biscuit-terminal/README.md` to mention markdown fallback for unsupported OSC8 and underline fallback semantics.
6. If the shared harness gains a new backend, update `.claude/skills/biscuit-terminal/SKILL.md` only if the workflow becomes a reusable package convention.
7. Re-run the relevant docs or skill validation command if any docs or skills were edited.
8. Confirm acceptance criteria:
   - AC-1: unit and Level-1 tests prove markdown link fallback.
   - AC-2: unit and Level-1 tests prove double-to-straight underline fallback.
   - AC-3: unit or explicit probe override proves no underline escapes when none are supported.
   - AC-4: Level-1 Apple Terminal PTY test passes.
   - AC-5: Level-2 harness lifecycle test passes on macOS.
   - AC-6: Level-2 tests skip on CI and when Terminal.app is unavailable.

Validation checkpoint:

- Final green targeted command set from steps 1-4.
- `git diff --check` reports no whitespace errors.
- `git status --short` shows only intentional files.

Dependency: all implementation and test phases complete.
