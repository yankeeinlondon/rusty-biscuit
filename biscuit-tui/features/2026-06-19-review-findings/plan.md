---
agent: "open_code/zai-coding-plan/glm-5.2"
phases: 5
created: 2026-06-26
start_phase: 1
yolo: "true"
source_spec: biscuit-tui/features/2026-06-19-review-findings/spec.md
package_area: biscuit-tui
source_files_during_phase_1:
  - biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs
  - biscuit-tui/lib/src/core/standalone/tests.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - biscuit-tui/lib/src/components/input_table/error.rs
  - biscuit-tui/lib/src/components/input_table/mod.rs
  - biscuit-tui/lib/src/components/input_table/table.rs
  - biscuit-tui/lib/src/components/input_table/table/tests.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/cli/src/commands/input_table/columns.rs
  - biscuit-tui/cli/src/commands/input_table/mod.rs
  - biscuit-tui/cli/src/commands/input_table/tests.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_4:
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/components/choose_one/tests.rs
  - biscuit-tui/lib/src/components/choose_many/tests.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - docs/dependencies.md
  - biscuit-tui/docs/components/input_table.md
  - biscuit-tui/lib/README.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/biscuit-tui/SKILL.md
# Phase 6 (execution) implements Phase 3 / F2 (Windows console redirect) —
# the only remaining unchecked plan body. See the Phase 3 checkpoint.
source_files_during_phase_6:
  - biscuit-tui/lib/Cargo.toml
  - biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs
  - biscuit-tui/lib/src/core/standalone/mod.rs
  - biscuit-tui/lib/src/core/standalone/tests.rs
docs_updated_during_phase_6:
  - docs/dependencies.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_code:
  - biscuit-tui/lib/src/core/standalone/terminal_lifecycle.rs
  - biscuit-tui/lib/src/core/standalone/tests.rs
  - biscuit-tui/lib/src/components/input_table/error.rs
  - biscuit-tui/lib/src/components/input_table/mod.rs
  - biscuit-tui/lib/src/components/input_table/table.rs
  - biscuit-tui/lib/src/components/input_table/table/tests.rs
  - biscuit-tui/lib/src/components/mod.rs
  - biscuit-tui/lib/src/prelude.rs
  - biscuit-tui/cli/src/commands/input_table/columns.rs
  - biscuit-tui/cli/src/commands/input_table/mod.rs
  - biscuit-tui/cli/src/commands/input_table/tests.rs
  - biscuit-tui/lib/src/components/choose_one.rs
  - biscuit-tui/lib/src/components/choose_many.rs
  - biscuit-tui/lib/src/components/choose_one/tests.rs
  - biscuit-tui/lib/src/components/choose_many/tests.rs
documentation:
  - docs/dependencies.md
  - biscuit-tui/docs/components/input_table.md
  - biscuit-tui/lib/README.md
packages:
  - biscuit-tui
---

# biscuit-tui Review-Findings Remediation — Execution Plan

This plan implements the five review findings (F1–F5) in the
[spec](./spec.md). Phase order follows the spec's *Risk & Sequencing Notes*:
**F1** (priority, can corrupt the shell) → **F3 + F4** (shared `InvalidInput`
CLI error surface, done together) → **F2** (largest, CI-gated) → **F5**
(small, isolated) → **Closure** (drift docs, skill update, full quality gates).

## Parallelization map (read before sequencing)

- **F1, F2, F5** touch disjoint files and have **no interdependencies**. If
  multiple implementers are available, F2 and F5 can be started in parallel
  with Phase 1/2. The phase order below is the *recommended* single-track
  sequence; it is not a hard dependency chain except where noted.
- **Hard dependency:** F4 depends on F3 (CLI routes library `InputTableError`
  through the shared `InvalidInput` surface). They are fused into one phase.
- **Soft dependency:** Closure (Phase 5) depends on all prior phases landing.

All tasks are scoped to the `biscuit-tui/` package area unless a path says
otherwise. Conventions: US English, no `cargo fmt` write-mode, no comments
that restate code, surgical changes only (Rule 3).

---

## Phase 1 — F1: Transactional terminal preparation

**Goal:** Raw mode can never survive a `prepare_terminal` error return.
Self-contained to `lib/src/core/standalone/terminal_lifecycle.rs`. Highest
priority because it is the only finding that can corrupt the caller's shell.

**Files:** `lib/src/core/standalone/terminal_lifecycle.rs`,
`lib/src/core/standalone/tests.rs` (regression test). The caller contract in
`lib/src/core/standalone/mod.rs:221-222` is **unchanged**.

- [x] Add a private `PrepareGuard` struct in `terminal_lifecycle.rs` that:
  - is armed immediately after `enable_raw_mode()?` succeeds;
  - tracks whether `EnterAlternateScreen` has actually fired (so `Drop` never
    emits `LeaveAlternateScreen` for a screen never entered);
  - on `Drop`, calls `disable_raw_mode()` and (if entered) `LeaveAlternateScreen`
    unless explicitly dismissed;
  - is dismissed only on the success path, just before `Ok(kbd_pushed)` is
    returned, so ownership transfers cleanly to the caller's `TerminalGuard`.
- [x] Rewrite `prepare_terminal` to construct `PrepareGuard` right after
  `enable_raw_mode()?`, mark alt-screen-entered only after `EnterAlternateScreen`
  succeeds, and dismiss the guard immediately before returning `Ok`.
- [x] Confirm the `PushKeyboardEnhancementFlags` step stays best-effort
  (`.is_ok()`) and is **not** part of the unwind (matches spec). Add a one-line
  `// WHY` note that if it ever becomes fallible it must be sequenced after the
  guard is armed.
- [x] Add a regression test that forces `prepare_terminal` to fail *after* raw
  mode is enabled (via a small injectable seam / fault injection on the
  alt-screen step — not real terminal faults) and asserts raw mode is disabled
  and no `LeaveAlternateScreen` is emitted for a screen never entered.
- [x] Add a test asserting the happy path is byte-for-byte unchanged
  (`kbd_pushed` flag flows through; `TerminalGuard::new(fullscreen, kbd_pushed)`
  still receives the same values).

**Phase 1 checkpoint:** `just test` (lib) passes; the new regression test
fails if the `PrepareGuard` is removed (mutation check); `just lint` clean.
Unix happy-path behavior unchanged.

---

## Phase 2 — F3 + F4: Input-table validation surface (fused)

**Goal:** Library callers get a typed, panic-free `try_new`; the CLI JSON
boundary stops silently coercing/truncating. Both route through one
`InvalidInput`-class CLI error surface. **F3 must land before F4 within this
phase** — they share the error type and the CLI switches to `try_new`.

### Phase 2a — F3: `InputTableState::try_new` + `InputTableError`

**Files:** `lib/src/components/input_table/table.rs` (incl.
`normalize_row` ~740-768 and `apply_cell_value`), `lib/src/components/input_table/mod.rs`,
crate root / prelude export site.

- [x] Introduce a typed error enum `InputTableError` (`thiserror = "2"`,
  already a dependency) with variants carrying diagnostic context:
  - `RowShapeMismatch { row, expected, found }`
  - `DuplicateColumnId { row, id }`
  - `UnknownColumnId { row, id }`
  - `MissingColumnId { row, id }`
  - `CellTypeMismatch { row, id, expected: &'static str, found: &'static str }`
- [x] Extract the currently-panicking shape/ID checks and the currently-silent
  cell-type compatibility checks into `Result`-returning helpers consumed by
  `try_new` (no behavior change yet to `new`).
- [x] Add `pub fn try_new(columns, initial_rows) -> Result<Self, InputTableError>`
  that performs shape + ID + typed-cell validation and returns `Err` instead of
  panicking / silently defaulting.
- [x] Re-express `new` as `try_new(...).expect("InputTableState::new: invalid
  table shape")`, preserving the existing panic-on-misuse contract and signature.
- [x] Document `new` as "panics on invalid input; use `try_new` for
  caller-provided data" (no `## Arguments`/`## Returns` duplication per repo
  convention).
- [x] Keep `with_blank_rows` infallible (it seeds from column schema, no caller
  cell data) — verify no accidental coupling to the new validation path.
- [x] Export `InputTableError` from the crate root / prelude alongside the
  other public table types.
- [x] Add `try_new` tests asserting each `InputTableError` variant (with the
  expected context fields) for: row-length mismatch, duplicate IDs, unknown
  IDs, missing IDs, typed cell mismatches.
- [x] Add a test that `new` still panics on the same inputs (no behavior change
  for existing callers).

### Phase 2b — F4: Tighten `input-table` CLI JSON validation

**Files:** `cli/src/commands/input_table/columns.rs` (lines ~116-119, 153-162
and equivalent optional-field reads), `cli/src/commands/input_table/mod.rs`
(`parse_cell_value` ~156-203), `cli/src/commands/input_table/tests.rs`.

- [x] Route the CLI `input-table` build path through `try_new` (from 2a) and
  surface `InputTableError` as an `InvalidInput`-class CLI error with
  row/column context (single validation source of truth for lib + CLI).
- [x] Replace `as_u64().map(|n| n as u16)` with `u16::try_from` and the
  `usize`-target fields with checked conversion. Apply to `max_length`,
  `preferred_width`, `preferred_height`, `min_selections`, `max_selections`.
  On overflow, return `InvalidInput` naming the field, offending value, and
  column context.
- [x] For optional column config fields (`initial`, `required`, `scrollbar`,
  `min_selections`, `max_selections`): keep absence as the defaulting behavior,
  but reject present-but-wrong-type values with `InvalidInput` (e.g.
  `"initial": 1` on a text-input, `"required": "yes"` on a choice column).
- [x] In `parse_cell_value`, reject JSON values that do not match the column's
  cell type with `InvalidInput` (column/row context) instead of
  `other.to_string()` stringification or silent `false` for malformed booleans.
- [x] Keep and **document** the intentional permissive row-value contracts:
  booleans accept bool / number / the strings `true|on|yes|1|false|off|no|0`;
  text-area accepts an array or a newline-split string; choose-many accepts an
  array or a comma-split string. Everything else is an error.
- [x] Add CLI tests asserting `InvalidInput` (not silent coercion) for:
  oversized `preferred_width`/`preferred_height` (> `u16::MAX`), oversized
  `min_selections`/`max_selections` (32-bit where practical), present-but-wrong-
  type column config fields, non-string row values for string cells, invalid
  boolean row values — each with field/column context.

**Phase 2 checkpoint:** `just test` (lib + cli) passes, including the new
`try_new` and CLI validation tests; `just lint` clean; `new`'s signature and
panic contract unchanged. Lib and CLI share one validation code path.

---

## Phase 3 — F2: Windows console redirect for captured-stdout prompts

**Goal:** On Windows, `question` with stdout captured renders the prompt to
the console (not the pipe) and the cursor-position probe no longer hangs on a
pipe — behavioral parity with the Unix `/dev/tty` redirect. The Unix path is
**strictly untouched**. This is the largest effort and the only finding that
cannot be fully verified on the macOS host; lean on CI for Windows.

**Files:** `lib/src/core/standalone/terminal_lifecycle.rs` (new `#[cfg(windows)]`
impl), `lib/src/core/standalone/mod.rs` (replace the `#[cfg(not(unix))]` no-op
at ~396-404 with `#[cfg(windows)]` real impl + retained
`#[cfg(all(not(unix), not(windows)))]` no-op), `lib/Cargo.toml`
(`[target.'cfg(windows)'.dependencies]`), root `docs/dependencies.md`.

- [x] Add `windows-sys` as a `[target.'cfg(windows)'.dependencies]` entry in
  `lib/Cargo.toml` **only** — do not add it to the default dependency set.
- [x] Implement a Windows `StdoutTtyRedirect` mirroring the Unix lifecycle,
  using `windows-sys` FFI: `CreateFileW("CONOUT$", GENERIC_READ|GENERIC_WRITE,
  FILE_SHARE_READ|FILE_SHARE_WRITE, ..., OPEN_EXISTING, ...)` to acquire the
  console; `GetStdHandle(STD_OUTPUT_HANDLE)` + `GetFileType` to detect
  captured (not `FILE_TYPE_CHAR`); save the original `STD_OUTPUT_HANDLE` and
  `SetStdHandle(STD_OUTPUT_HANDLE, conout_handle)` to redirect.
- [x] Implement `Drop`: flush `io::stdout()` before restoring, `SetStdHandle`
  the original handle back, `CloseHandle` on `CONOUT$`. Use the same
  `Option<...>`-takes-ownership pattern as Unix so restore is exactly-once and
  partial-activation error paths close any handle they opened.
- [x] Mirror the Unix activation gating: no-op when stdout is already a
  console; no-op when **both** stdout and stderr are captured (preserve the
  explicit "no interactive terminal available" error from `mod.rs:211-215`
  rather than leaking ANSI into a captured stream).
- [x] Add `SAFETY:` comments on every FFI call matching the Unix block's
  documentation quality (invariants: handle validity, single close, restore
  once). Keep `unsafe` regions minimal; keep the type process-global but private.
- [x] Keep the existing `#[cfg(unix)]` block untouched; replace only the
  `#[cfg(not(unix))]` no-op with `#[cfg(windows)]` (real impl) plus a retained
  `#[cfg(all(not(unix), not(windows)))]` no-op for exotic targets.
- [x] Prove the handle strategy with a `#[cfg(windows)]` test (or documented
  CI reproduction) showing crossterm calls through `io::stdout()` after
  activation really target `CONOUT$` when the process starts with stdout
  captured. If `SetStdHandle` is insufficient (writer caches the old handle),
  fall back to a small output abstraction per the spec rather than shipping a
  partial redirect.
- [x] Add a `#[cfg(windows)]` test that `question` with stdout captured and a
  console attached emits no prompt bytes into captured stdout and the cursor
  probe does not time out.
- [x] Update root `docs/dependencies.md` to record the `windows-sys`
  platform-targeted dependency (per drift rules; do **not** create a
  `biscuit-tui/docs/dependencies.md` just for this).

**Phase 3 checkpoint:** `cargo build` / `just test` still pass on macOS and
Linux (Unix path unchanged); cross-compile check for the active Windows CI
target (`x86_64-pc-windows-msvc` or matrix) succeeds; Windows CI runner passes
the new `#[cfg(windows)]` test. Restore is exactly-once with no leaked handles
on activation-failure paths.

---

## Phase 4 — F5: Relax choice hotkey modifier matching

**Goal:** Benign extra modifier bits (e.g. `SHIFT` for an uppercase chord) no
longer suppress valid hotkeys; `CONTROL | ALT` is treated as ambiguous and
matches neither map. Small, isolated, lockstep across `choose_one` and
`choose_many`.

**Files:** `lib/src/components/choose_one.rs` (~576-595),
`lib/src/components/choose_many.rs` (~582-601).

- [x] In `choose_one.rs`, replace the exact `match event.modifiers` with
  `.contains(KeyModifiers::CONTROL)` / `.contains(KeyModifiers::ALT)`.
- [x] Define the `CONTROL | ALT` case explicitly: when **both** are present,
  match **neither** map (fall through) — AltGr-style chords must not be
  hijacked. Add a `// WHY` comment at the branch documenting the AltGr rationale.
- [x] Apply the identical change to `choose_many.rs` so hotkey semantics stay
  in lockstep with `choice_state` helpers and `choose_one`.
- [x] Add hotkey tests (via the `drive_event_loop` synthetic-event seam) for
  `CONTROL | SHIFT` (matches ctrl map), `ALT | SHIFT` (matches alt map), and
  `CONTROL | ALT` (matches neither), in **both** `choose_one` and `choose_many`.
- [x] Add a test that bare-`CONTROL` and bare-`ALT` hotkey behavior is unchanged.

**Phase 4 checkpoint:** `just test` (lib) passes with the new modifier tests;
`just lint` clean; existing bare-modifier hotkey behavior unchanged.

---

## Phase 5 — Closure: drift docs, skill update, full quality gates

**Goal:** All remediation landed; all drift artifacts updated; every quality
gate green for both crates. No code changes in this phase unless a gate fails.

- [x] Run `just test` from `biscuit-tui/` — passes for `biscuit-tui` and
  `biscuit-tui-cli`.
- [x] Run `just test-l2` from `biscuit-tui/` — passes (covers terminal-lifecycle
  changes from F1/F2).
- [x] Run `just lint` from `biscuit-tui/` — clippy clean for both crates.
- [x] Run the Windows cross-compile check for the active CI target (F2) — succeeds.
- [x] Verify root `docs/dependencies.md` records `windows-sys` (from Phase 3).
- [x] Update component/CLI READMEs to document the F4 permissive-parsing
  contracts (boolean/string-array/comma-split acceptances) as intentional
  compatibility behavior.
- [x] Update the biscuit-tui skill (`.claude/skills/biscuit-tui/SKILL.md` and/or
  `.opencode/skill/biscuit-tui/`) because the public API surface changed:
  `try_new` and `InputTableError` are now exported (F3).
- [x] Verify no `cargo fmt` write-mode was run and no formatting-only commits
  exist in the branch (repo policy).
- [x] Confirm `new`'s signature and the `prepare_terminal`/`TerminalGuard`
  caller contract are unchanged (backward-compatibility verification for F1/F3).

**Phase 5 checkpoint (definition of done):** every gate above is green and
every drift artifact is updated. The branch contains only surgical changes
traceable to F1–F5; no component-model restructuring, no formatting sweeps.

---

## Risk reminders for implementers

- **F1** is the only finding that can corrupt the user's shell — land it first.
- **F3 → F4** share an error surface; never land F4's CLI routing before F3's
  `try_new` + `InputTableError` exist.
- **F2** cannot be fully verified on a macOS host; every Windows assertion must
  be `#[cfg(windows)]`-gated and rely on the CI matrix (see the
  [matrix-testing spec](../../2026-06-07-matrix-testing/spec.md)). Keep the
  Unix path strictly untouched.
- **F5** is safe to land at any point; it has no shared files with F1–F4.
- All changes are surgical (Rule 3): preserve `new`'s signature and the
  `prepare_terminal`/`TerminalGuard` contract for backward compatibility.
