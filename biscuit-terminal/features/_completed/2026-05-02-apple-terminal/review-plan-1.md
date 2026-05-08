---
phases: 4
start_phase: 3
source_files_during_phase_3:
  - biscuit-terminal/lib/src/components/prose.rs
  - biscuit-terminal/lib/examples/discovery_probe.rs
  - biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
  - biscuit-terminal/docs/components/prose.md
  - biscuit-terminal/README.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages:
  - biscuit-terminal
---
# Review-1 Implementation Plan — Apple Terminal Feature

Plan for closing all findings (H1–H4, M1–M5, L1–L3, E1–E3) raised in
`review-1.md` against the Apple Terminal Integration & Prose Graceful
Degradation feature.

## Source documents

- Spec: `biscuit-terminal/features/2026-05-02-apple-terminal/spec.md`
- Review: `biscuit-terminal/features/2026-05-02-apple-terminal/review-1.md`

## Findings → phases overview

| ID | Severity | Phase |
|----|----------|-------|
| H1 | High | Phase 1 |
| H2 | High | Phase 2 |
| H3 | High | Phase 2 |
| H4 | High | Phase 2 (consequence of H2+H3) |
| M1 | Medium | Phase 3 |
| M2 | Medium | Phase 3 |
| M3 | Medium | Phase 1 |
| M4 | Medium | Phase 1 |
| M5 | Medium | Phase 2 |
| L1 | Low | Phase 4 |
| L2 | Low | Phase 3 |
| L3 | Low | Phase 1 |
| E1 | Ergonomics | Phase 3 |
| E2 | Ergonomics | Phase 3 |
| E3 | Ergonomics | Phase 1 |

## Build / lint / test cadence

After **every** phase:

- `cargo build -p biscuit-test-harness -p biscuit-terminal -p biscuit-terminal-cli`
- `cargo test -p biscuit-test-harness`
- `cargo test -p biscuit-terminal -p biscuit-terminal-cli`
- `cargo clippy --all-targets -p biscuit-terminal -p biscuit-terminal-cli -p biscuit-test-harness -- -D warnings`

The plan must end with **zero** clippy warnings in all three crates and
**all** tests green (Level-2 may skip cleanly when Terminal.app or
WezTerm/Kitty is unavailable).

---

## Phase 1 — Foundational fixes & harness wiring

**Goal:** Land the smallest set of changes that make the
`AppleTerminalHarness` module compile, are testable in isolation, and
remove the cheapest medium/low-priority defects (M3, M4, L3, E3).
After Phase 1 the harness module's own unit tests must run as part of
`cargo test -p biscuit-test-harness`.

### Findings addressed

- **H1** — wire `apple_terminal` module into `biscuit-test-harness`.
- **M3** — document & enforce `applescript_escape` byte contract.
- **M4** — replace fixed 800 ms sleep with `wait_for_prompt`.
- **L3** — surface `osascript` cleanup failures via `eprintln!`.
- **E3** — pre-allocate `applescript_escape` buffer for line/tab-heavy strings.

### Files to modify / create

- `biscuit-test-harness/src/lib.rs` — declare `pub mod apple_terminal;`.
- `biscuit-test-harness/src/apple_terminal.rs` — already exists
  (untracked). Edits:
  1. Replace fixed `std::thread::sleep(Duration::from_millis(800))` in
     `spawn_shell` with a call to `super::wait_for_prompt(self)` (M4).
     Keep a small fallback settle (≤200 ms) only if `wait_for_prompt`
     returns without finding a prompt.
  2. In `Drop::drop` / `close_window`, capture and print the
     `osascript` failure: `eprintln!("warning: failed to close
     Terminal.app window {id}: {stderr}")` when status is non-zero
     or spawn errors (L3).
  3. Document `applescript_escape` byte contract in the module-level
     doc comment **and** the function-level `///`:
     "Bytes outside printable UTF-8 plus LF (0x0A) and HT (0x09) are
     not escaped; CR (0x0D), NUL (0x00), BEL (0x07), ESC (0x1B), and
     U+2028 / U+2029 will produce an AppleScript syntax error and are
     rejected via `debug_assert!`." Implement a
     `debug_assert!(is_applescript_safe(ch))` inside the loop (M3).
  4. Pre-allocate the output `String` based on input size to mitigate
     reallocation on multi-line input (E3): use
     `String::with_capacity(s.len() * 2)` when input contains `\n` or
     `\t`, otherwise `s.len() + 8` (current behaviour).
- (No production code changes outside the harness in this phase.)

### Tests to add

Add to `biscuit-test-harness/src/apple_terminal.rs` `#[cfg(test)]`
block (these run on every host because they test pure helpers):

- `applescript_escape_rejects_cr` — `#[should_panic]` (debug builds
  only; gate with `#[cfg(debug_assertions)]`).
- `applescript_escape_rejects_nul` — same gating.
- `applescript_escape_rejects_esc` — same gating.
- `applescript_escape_unicode_separators_panic` — covers U+2028 /
  U+2029, same gating.
- `applescript_escape_preallocates_for_multiline` — invokes the
  function on a 1024-char multi-line string and asserts the result is
  produced (smoke test that allocation policy did not regress).
- `wait_for_prompt_path_is_used` — *not* a runtime test; assert via
  `cargo expand` is excessive. Instead, add a compile-time check by
  constructing a `super::wait_for_prompt(&mut harness)` call inside
  an `#[cfg(test)] fn _typecheck()` private helper that is never
  invoked but ensures the call type-checks.

Also confirm: `cargo test -p biscuit-test-harness` now reports the
existing 8 unit tests **plus** the new ones above (≥ 12 tests in the
`apple_terminal::tests` module).

### Verification commands

- `git status biscuit-test-harness/src/apple_terminal.rs` — file is
  tracked.
- `cargo build -p biscuit-test-harness`.
- `cargo test -p biscuit-test-harness apple_terminal::tests` — all
  pass; off-macOS hosts still skip Terminal-specific behaviour.
- `cargo clippy --all-targets -p biscuit-test-harness -- -D warnings`.

### Definition of done

- `biscuit-test-harness/src/lib.rs` contains `pub mod apple_terminal;`.
- The harness module compiles and its tests run.
- `applescript_escape` documents and enforces its byte contract.
- `spawn_shell` no longer relies on a fixed 800 ms sleep.
- `Drop` cleanup surfaces failures via `eprintln!`.

---

## Phase 2 — Level-2 Terminal.app tests + justfile wiring

**Goal:** Provide real-Terminal.app verification of AC-1, AC-2, AC-5,
and AC-6. After this phase `just test-l2` will exercise the new suite
(skipping cleanly off-macOS / in CI / when Terminal.app is missing).

### Findings addressed

- **H2** — implement
  `cli/tests/level2_apple_terminal_prose.rs`.
- **H3** — append `level2_apple_terminal_prose` to `just test-l2`.
- **H4** — by virtue of H2+H3, AC-1 and AC-2 gain Level-2 coverage.
- **M5** — enforce `serial_test::serial(level2_terminal)` group key
  consistent with `level2_image.rs`.

### Files to modify / create

- `biscuit-terminal/cli/tests/level2_apple_terminal_prose.rs` (new).
- `biscuit-terminal/cli/Cargo.toml` — confirm `serial_test` is a
  `dev-dependencies` entry (it already is for `level2_image.rs`).
- `biscuit-terminal/justfile` — append `--test
  level2_apple_terminal_prose` to `test-l2` recipe at line 63.

### Test file outline (`level2_apple_terminal_prose.rs`)

```rust
//! Level-2 tests for Prose graceful degradation against Apple Terminal.
//!
//! Skip-clean: tests early-return when Terminal.app cannot be addressed
//! (off-macOS, CI=1, osascript unavailable).
//!
//! Serialization: shares the `level2_terminal` serial group with
//! `level2_image.rs` because Terminal.app exposes one global
//! application state to AppleScript.

mod common;

use biscuit_test_harness::apple_terminal::AppleTerminalHarness;
use biscuit_test_harness::{TerminalHarness, skip_with_reason};
use common::send_bt_command;
use serial_test::serial;
use std::time::Duration;
```

Three required tests, each `#[serial(level2_terminal)]`:

1. **`level2_apple_terminal_link_fallback_visible`** (AC-1)
   - Skip when `!AppleTerminalHarness::available()`.
   - `harness.spawn_shell()`; settle.
   - `send_bt_command(&mut harness, "prose '<a href=\"https://example.com\">click here</a>'")`.
   - Capture; assert `frame.plain.contains("click here")` and
     `frame.plain.contains("(https://example.com)")`.
   - Assert `!frame.plain.contains("\x1b]8;;")` and
     `!frame.plain.contains("8;;https://example.com")`.

2. **`level2_apple_terminal_double_underline_plain_text_visible`** (AC-2)
   - Skip-gate identical.
   - `send_bt_command(&mut harness, "prose '<double-underline>important text</double-underline>'")`.
   - Assert `frame.plain.contains("important text")`.
   - Assert `!frame.plain.contains("\x1b[4:2m")` and
     `!frame.plain.contains("[4:2m")` (literal escape garbage).
   - The Terminal.app-stripped capture cannot prove SGR is `\x1b[4m`
     vs nothing; that negative is covered by Level-1. The Level-2
     assertion is **only** about visible text vs visible garbage.

3. **`level2_apple_terminal_harness_lifecycle`** (AC-5, AC-6)
   - First half: with `AppleTerminalHarness::available() == false`,
     test prints `skipping: requires Terminal.app` via
     `skip_with_reason` and returns OK (covers AC-6).
   - Second half (only when available): construct harness inside a
     scope, `spawn_shell`, `send_text("echo HARNESS_LIFECYCLE_OK\n")`,
     `capture` non-empty, drop the harness, then `osascript` query
     "does a window with that id exist" — assert `false` (cleanup
     ran).

### Verification commands

- `cargo build -p biscuit-terminal-cli --test level2_apple_terminal_prose`.
- `cargo test -p biscuit-terminal-cli --test level2_apple_terminal_prose`
  on macOS (developer machine) — all three tests pass.
- Same on Linux / `CI=1` — all three tests skip with
  `skipping: requires Terminal.app` and return OK.
- `just -f biscuit-terminal/justfile test-l2` — the new test target is
  invoked.
- `cargo clippy --all-targets -p biscuit-terminal-cli -- -D warnings`.

### Definition of done

- New test file exists and compiles.
- All three tests pass on macOS, skip cleanly elsewhere.
- `just test-l2` includes `level2_apple_terminal_prose`.
- Each test carries `#[serial_test::serial(level2_terminal)]`.

---

## Phase 3 — Atomic-token degradation + probe policy unification + ergonomics

**Goal:** Close the remaining production-code defects in `Prose` and
`discovery_probe`, plus the small ergonomic wins. This phase keeps
graceful-degradation policy in **one** place (`detection`) and ensures
the `{{double-underline}}` atomic token is no longer a leaky path.

### Findings addressed

- **M1** — capability-aware `{{double-underline}}` atomic token.
- **M2** — probe uses `detection::osc8_link_support()` directly.
- **L2** — Level-1 PTY test exercises `PROBE_FORCE_OSC8=true`.
- **E1** — `<a>` markdown fallback avoids per-tag `format!` allocation
  where reasonable.
- **E2** — replace `(String::new(), String::new())` sentinel with a
  named constant or enum variant inside `block_tag_to_escape` so the
  parser does not re-check empties at line ~1505.

### Files to modify

- `biscuit-terminal/lib/src/components/prose.rs`
    - Introduce a `BlockTagAction` enum returned by `block_tag_to_escape`:

    ```rust
    enum BlockTagAction {
        /// Emit `open` before, `close` after the inner content.
        Wrap { open: Cow<'static, str>, close: Cow<'static, str> },
        /// Emit only the inner content (no escapes).
        Suppress,
    }
    ```

    Update the single call site at line ~1501 to match on the enum
    instead of inspecting `(open, close)` for empties (E2).
    - Replace `format!("]({})", resolved_href)` with a direct push
    pattern when feasible — at minimum ensure the open string `"["`
    is `Cow::Borrowed("[")` (E1). If the enum keeps `Cow<'static>` /
    `String`, document the cost there.
    - Add `atomic_token_to_escape_with_term(token: &str, term:
    Option<&Terminal>) -> Option<Cow<'static, str>>` and route the
    atomic-token branch in `parse_tokens_inner` (line ~1406) through
    it. Only `double-underline` consults `term`; everything else is
    delegated to `atomic_token_to_escape` (M1).
    - Preserve the existing `ATOMIC_TOKEN_TABLE` layout — the new helper
    *wraps* it.

- `biscuit-terminal/lib/examples/discovery_probe.rs`
    - Replace the `match force_osc8 { ... !matches!(...) }` block at
    lines 288–291 with:

    ```rust
    let osc_link_support = match force_osc8 {
        Some(v) => v,
        None => biscuit_terminal::discovery::detection::osc8_link_support(),
    };
    ```

    Update the surrounding doc comment to note that this is the
    canonical detection function and the override is the only
    deviation (M2).

### Tests to add / modify

- `biscuit-terminal/lib/tests/level1_apple_terminal_prose.rs`
    - Add `apple_terminal_double_underline_atomic_token_degrades` —
    sends `PROBE_PROSE_INPUT={{double-underline}}important text` with
    `PROBE_TERM_PROGRAM=Apple_Terminal` and asserts:
        - Output contains `important text`.
        - Output does **not** contain `\x1b[4:2m`.
        - Output **does** contain `\x1b[4m` (degraded to straight) (M1).

    - Add `probe_force_osc8_emits_osc_when_supported` — sends
    `PROBE_TERM_PROGRAM=Apple_Terminal`, `PROBE_FORCE_OSC8=true`, and
    `PROBE_PROSE_INPUT=<a href="https://example.com">click here</a>`.
    Asserts output contains `\x1b]8;;https://example.com` and does
    **not** contain the markdown fallback `[click here](` (L2).

- `biscuit-terminal/lib/src/components/prose.rs` `#[cfg(test)]` block
    - `block_tag_action_suppress_skips_open_close` — uses a `Terminal`
    with both underline supports false; assert that
    `block_tag_to_escape("double-underline", &[], Some(&term))`
    returns `BlockTagAction::Suppress`.
    - `atomic_token_to_escape_with_term_degrades_double_underline` —
    same matrix as the existing block-tag tests but driven through
    the atomic-token helper (M1 unit coverage).

### Verification commands

- `cargo test -p biscuit-terminal --test level1_apple_terminal_prose`.
- `cargo test -p biscuit-terminal --lib`.
- `cargo build -p biscuit-terminal --example discovery_probe`.
- `cargo clippy --all-targets -p biscuit-terminal -- -D warnings`.

### Definition of done

- `{{double-underline}}` is capability-aware; Level-1 PTY proves it.
- `discovery_probe` calls `detection::osc8_link_support()` exactly
  once for the non-override case.
- `BlockTagAction::Suppress` (or equivalent named sentinel) replaces
  the empty-strings check.
- `<a>` open string allocates no per-tag `String` for the bracket.
- `PROBE_FORCE_OSC8=true` has a positive-case Level-1 test.

---

## Phase 4 — Documentation, final test/lint sweep

**Goal:** Update user-facing documentation to describe the new
fallback semantics (L1) and run a final cross-crate
`cargo test` + `cargo clippy --all-targets` sweep.

### Findings addressed

- **L1** — update `docs/components/prose.md` and `README.md`.

### Files to modify

- `biscuit-terminal/docs/components/prose.md`
    - Add a "Graceful degradation" section that lists:
        - OSC8 hyperlink fallback `[desc](url)` when
      `osc_link_support == false`.
        - Double-underline degradation matrix
      (double / straight only / neither).
        - Atomic-token equivalent (`{{double-underline}}`) shares the
      same policy after Phase 3.
- `biscuit-terminal/README.md`
    - Add a sub-bullet under the Prose / capabilities section pointing
    at the new prose.md section. Keep the README delta small —
    reference docs/components/prose.md for the full table.

No production-code changes in this phase.

### Final verification

Run each command from a clean working tree:

- `cargo build --workspace --all-targets`.
- `cargo test -p biscuit-test-harness`.
- `cargo test -p biscuit-terminal -p biscuit-terminal-cli`.
- `just -f biscuit-terminal/justfile test-l2` (macOS dev box; expect
  Apple Terminal tests to actually run).
- `cargo clippy --all-targets -p biscuit-terminal
  -p biscuit-terminal-cli -p biscuit-test-harness -- -D warnings`.
- `cargo doc --no-deps -p biscuit-terminal -p biscuit-test-harness`
  (sanity check that doc-comment additions in M3, M1, M2 compile).

### Definition of done

- `prose.md` and `README.md` describe the fallback semantics.
- All listed verification commands exit 0 with zero warnings.
- All AC-1 .. AC-6 in `spec.md` have at least the verification level
  the spec demanded:
    - AC-1, AC-2: Level-2 (Phase 2) + Level-1 (existing).
    - AC-3: Level-1 (existing, no change).
    - AC-4: Level-1 (existing, plus new tests in Phase 3).
    - AC-5, AC-6: Level-2 (Phase 2 lifecycle test).

---

## Risk register

- **Phase 1 risk:** debug-only `debug_assert!` in `applescript_escape`
  may be reached by a future caller passing a CR — keep the assertion
  inside `#[cfg(debug_assertions)]` so release builds remain
  best-effort.
- **Phase 2 risk:** Terminal.app's `do script` returns immediately,
  but content render lags. The `wait_for_prompt` path from Phase 1
  reduces flakiness; keep the existing `settle()` 400 ms after each
  `send_bt_command` to absorb residual jitter.
- **Phase 3 risk:** Switching `block_tag_to_escape`'s return type
  from `Option<(String, String)>` to `Option<BlockTagAction>` is a
  cross-cutting refactor inside `prose.rs`. The single call site at
  ~1501 is the only consumer, but ensure the change compiles before
  running the test suite to avoid noisy errors.
- **Phase 4 risk:** Documentation drift if Phase 3 lands but the
  prose.md text references an older API. Phase 4 must run *after*
  Phase 3 is merged.
