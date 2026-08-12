---
status: ready for planning and implementation
reviewed: true
review_iterations: 5
source_review: biscuit-tui/reviews/2026-06-19-comprehensive/review.md
package_area: biscuit-tui
---

# biscuit-tui Review-Findings Remediation Specification

**Status:** Reviewed and ready for planning and implementation. This
specification translates the 2026-06-19 comprehensive review of the
`biscuit-tui` package area into a set of scoped, implementable changes. Each
section maps to a finding in
[`review.md`](../../reviews/2026-06-19-comprehensive/review.md), states the
decision taken, and defines success criteria.

> **Review note:** This inline review fixes the source-review link, tightens
> the Windows captured-stdout contract, makes `InputTableState::try_new`
> responsible for typed `CellValue` compatibility as well as row shape, and
> extends the JSON-boundary validation requirement from row values to column
> configuration values. These are clarifications of the review findings, not
> changes to the underlying remediation goal.

The review rated the package area `medium` risk overall. The architecture is
sound; the changes below close lifecycle-safety, cross-platform, and
API-ergonomics gaps without restructuring the component model.

---

## Scope

In scope — all five review findings plus their associated testing gaps:

1. **F1 (High)** — Terminal setup can return with raw mode still enabled.
2. **F2 (Medium)** — Captured-stdout interactive prompts are Unix-only despite the macOS/Windows/Linux contract.
3. **F3 (Medium)** — `InputTableState::new` panics on recoverable data-shape errors and silently ignores typed cell mismatches.
4. **F4 (Medium)** — `input-table` CLI JSON parsing silently coerces, defaults, or truncates invalid values.
5. **F5 (Low)** — Choice hotkey matching is stricter than typical terminal modifier payloads.

Explicitly out of scope:

- **Next-step #6 (toolchain)** — `cargo fmt --check` could not run during review
  because `cargo-fmt` is not installed for `stable-aarch64-apple-darwin`. This is
  a local toolchain provisioning issue, not a code change, and the monorepo
  policy is to never run `cargo fmt` write-mode. No spec action; resolve by
  `rustup component add rustfmt` on the reviewing host if read-only diagnosis is
  needed.

---

## Decisions

### F1 — Make terminal preparation transactional (High)

**Problem.** `prepare_terminal(fullscreen)` in
`lib/src/core/standalone/terminal_lifecycle.rs` calls `enable_raw_mode()?` and
then, when `fullscreen` is true, `execute!(out, EnterAlternateScreen)?`. The
`TerminalGuard` that restores raw mode is only constructed by the caller
(`run_standalone_with_chrome`, `mod.rs:220`) *after* `prepare_terminal` returns
`Ok`. If `EnterAlternateScreen` (or the subsequent flush) fails, the function
returns `Err` with raw mode still enabled and no guard in place — leaving the
caller's shell in raw mode. This is the highest-impact TUI failure mode: a
corrupted interactive shell after an otherwise recoverable I/O error.

**Decision.** Make `prepare_terminal` transactional: raw mode is the *first*
side effect and must be unwound on any later setup failure within the same
function. Introduce a small internal RAII `PrepareGuard` scoped to
`prepare_terminal` that:

- is armed immediately after `enable_raw_mode()?` succeeds;
- on `Drop`, calls `disable_raw_mode()` **and** (if the alternate screen was
  entered) `LeaveAlternateScreen`, unless it has been explicitly *dismissed*;
- marks the alternate screen as entered only after `EnterAlternateScreen`
  succeeds, so teardown does not emit `LeaveAlternateScreen` for a screen the
  process never entered;
- is dismissed only on the success path, just before `Ok(kbd_pushed)` is
  returned, so ownership transfers cleanly to the caller's `TerminalGuard`.

The keyboard-enhancement push (`PushKeyboardEnhancementFlags`) is already
best-effort (`.is_ok()`), so it does not need to be part of the unwind, but if
it *were* made fallible in future it must be sequenced after the guard is armed.

This keeps the fix localized to `terminal_lifecycle.rs`; the caller contract
(`prepare_terminal -> io::Result<bool>` then `TerminalGuard::new(fullscreen,
kbd_pushed)`) is unchanged.

**Success criteria.**

- After any `Err` return from `prepare_terminal`, raw mode is disabled and the
  alternate screen (if entered) is left.
- No double-disable: the success path dismisses the `PrepareGuard` so only the
  caller's `TerminalGuard` performs teardown.
- Existing standalone behavior on the happy path is byte-for-byte unchanged.

---

### F2 — Full Windows console implementation for captured-stdout prompts (Medium)

**Problem.** When stdout is a pipe (e.g. `FOO=$(question choose-one ...)`), the
prompt must render to the real terminal, not the captured stream — and
crossterm's cursor-position probe (`DSR`) writes to `io::stdout()` and times out
when stdout is a pipe. The Unix `StdoutTtyRedirect`
(`terminal_lifecycle.rs:127-216`) handles this by opening `/dev/tty`, `dup`-ing
the old stdout, and `dup2`-ing the tty fd onto `STDOUT_FILENO`, restoring on
`Drop`. The non-Unix path (`mod.rs:394-402`) is an **empty no-op guard**, so on
Windows the prompt can render into the captured data stream and/or the cursor
probe can hang on the pipe. The monorepo requires macOS/Windows/Linux support.

**Decision.** Implement a real Windows equivalent of `StdoutTtyRedirect` using
the Win32 console API, mirroring the Unix lifecycle:

- **Acquire the console.** Open the active console output via
  `CreateFileW("CONOUT$", GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ |
  FILE_SHARE_WRITE, ..., OPEN_EXISTING, ...)`. `CONOUT$` resolves to the real
  console screen buffer even when the process's standard handle has been
  redirected to a pipe — the Windows analog of `/dev/tty`.
- **Detect "stdout is piped".** Use `GetStdHandle(STD_OUTPUT_HANDLE)` +
  `GetFileType`; treat `FILE_TYPE_DISK` / `FILE_TYPE_PIPE` (i.e. not
  `FILE_TYPE_CHAR`) as captured, matching the `activate_if_piped` intent.
  Activation is a no-op when stdout is already a console.
- **Redirect.** Save the original `STD_OUTPUT_HANDLE` and
  `SetStdHandle(STD_OUTPUT_HANDLE, conout_handle)` so new
  `io::stdout()`-based writes and the DSR probe target the console. On `Drop`,
  flush `io::stdout()` before restoring, `SetStdHandle` the original handle
  back, and close `CONOUT$`.
- **Prove the handle strategy.** The implementation must include a
  Windows-only test or manual CI reproduction showing that Crossterm calls made
  through `io::stdout()` after activation really use `CONOUT$` when the process
  starts with stdout captured. If `SetStdHandle` is insufficient because a
  writer caches the old handle, replace the no-op with a small output
  abstraction for standalone prompts instead of shipping a partial redirect.
- **Symmetry & ownership.** Use the same `Option<...>`-takes-ownership pattern as
  the Unix impl so the restore happens exactly once and partial-activation error
  paths close any handle they opened.

Implementation guidance:

- Prefer the `windows-sys` crate (lightweight, raw FFI) for `CreateFileW`,
  `GetStdHandle`, `SetStdHandle`, `GetFileType`, `CloseHandle`. Add it as a
  `[target.'cfg(windows)'.dependencies]` entry in `lib/Cargo.toml` only — do not
  add it to the default dependency set. Update the root `docs/dependencies.md`
  per the drift rules; there is no current `biscuit-tui/docs/dependencies.md`
  file, so do not create one just to document this dependency unless the area
  gains a dependency document during implementation.
- All FFI calls require `SAFETY:` comments matching the existing Unix block's
  documentation quality (invariants: handle validity, single close, restore
  once). Keep the `unsafe` regions minimal and the type process-global-but-private.
- Keep the existing `#[cfg(unix)]` block untouched. Replace only the
  `#[cfg(not(unix))]` no-op with `#[cfg(windows)]` (real impl) plus a retained
  `#[cfg(all(not(unix), not(windows)))]` no-op for exotic targets, so non-Unix
  non-Windows platforms still compile.

**Success criteria.**

- On Windows, `question` with stdout captured and a console attached renders the
  prompt to the console and the captured stream receives only the submitted
  value — behavioral parity with the Unix command-substitution case.
- The cursor-position probe does not hang when stdout is a pipe on Windows.
- `cargo build` / `cargo test` still succeed on macOS and Linux (Unix path
  unchanged); the crate compiles for `x86_64-pc-windows-msvc` or the active
  Windows target in the CI matrix.
- Restore is exactly-once; no leaked handles on activation-failure paths.
- If a console is unavailable or both stdout and stderr are captured, behavior
  remains the existing explicit `no interactive terminal available` error
  rather than leaking ANSI output into a captured data stream.

**Testing note.** The reviewing host is macOS-only, so Windows behavior cannot
be exercised locally. At minimum, gate Windows-specific assertions behind
`#[cfg(windows)]` and rely on CI matrix coverage (see the
[matrix-testing spec](../../../features/2026-06-07-matrix-testing/spec.md)) to validate on a
`windows-latest` runner. Where the harness permits, add a Windows test for
`question` with stdout captured and a console-like stream attached.

---

### F3 — Add `InputTableState::try_new` returning a typed error (Medium)

**Problem.** `InputTableState::new` (`lib/src/components/input_table/table.rs:101`)
panics when a row's cell count differs from the column count, and `normalize_row`
(same file, ~740-768) panics on duplicate column IDs, unknown column IDs, and
missing column IDs. `apply_cell_value` then silently ignores mismatched
`CellValue` variants (for example, a text value supplied for a boolean column)
and leaves the column's default value in place. These are recoverable validation
errors for any embedder building tables from user/config data. The CLI sidesteps
some of this for its own `--rows` path by pre-validating length, but library
callers have no `Result`-returning constructor and must reimplement private
normalization rules to stay panic-free and avoid silent defaults.

**Decision.**

- Introduce a typed error enum `InputTableError` using the existing
  `thiserror = "2"` dependency in the `input_table` module, with variants
  covering the currently-panicking and currently-silent invalid inputs. Each
  variant carries enough context to diagnose the problem (row index, expected
  vs actual cell count, offending column ID, expected cell kind, found cell
  kind):
  - `RowShapeMismatch { row: usize, expected: usize, found: usize }`
  - `DuplicateColumnId { row: usize, id: String }`
  - `UnknownColumnId { row: usize, id: String }`
  - `MissingColumnId { row: usize, id: String }`
  - `CellTypeMismatch { row: usize, id: String, expected: &'static str, found: &'static str }`
- Add `pub fn try_new(columns: Vec<InputTableColumn>, initial_rows: Vec<Row>)
  -> Result<Self, InputTableError>` that performs all shape/ID validation and
  cell-type compatibility validation and returns `Err` instead of panicking or
  silently defaulting.
- Re-express `new` as an invariant-enforcing convenience wrapper:
  `try_new(columns, initial_rows).expect("InputTableState::new: invalid table
  shape")`, preserving the existing panic-on-misuse contract and signature for
  backward compatibility. Document `new` as "panics on invalid input; use
  `try_new` for caller-provided data."
- Export `InputTableError` from the crate root / prelude alongside the other
  public table types.
- Update the CLI `input-table` path to call `try_new` and surface
  `InputTableError` as an `InvalidInput`-class CLI error (see F4) rather than
  relying on pre-validation, so library and CLI share one validation source of
  truth.
- Keep `with_blank_rows` infallible. It constructs values from the column schema
  itself and does not accept caller-provided cell data.

**Success criteria.**

- `try_new` returns `Err(InputTableError::...)` (never panics) for: row-length
  mismatch, duplicate IDs, unknown IDs, missing IDs, and typed cell mismatches.
- `new` still panics on the same inputs (no behavior change for existing callers).
- CLI `input-table` reports these as readable validation errors with row/column
  context and a non-zero exit, not a panic/backtrace.

---

### F4 — Tighten `input-table` CLI JSON validation (Medium)

**Problem.** At the CLI JSON boundary
(`cli/src/commands/input_table/columns.rs:116-119, 153-162` and
`mod.rs::parse_cell_value:156-203`):

- `max_length` is read via `as_u64().map(|n| n as usize)` and preferred
  dimensions via `as_u64().map(|n| n as u16)` — large values **silently
  truncate** on the `u16` fields.
- Optional column fields such as `initial`, `required`, `scrollbar`,
  `min_selections`, and `max_selections` use `and_then(...).unwrap_or(...)`
  patterns, so a present-but-wrong type is treated as if the field were absent.
- `parse_cell_value` coerces unsupported row JSON values into strings via
  `other.to_string()`, treats unsupported boolean shapes as `false`, and accepts
  comma-splitting for choose-many string values.

Silent coercion at a user-facing JSON boundary makes schema mistakes hard to
diagnose and can produce a table that does not match the user's intent.

**Decision.**

- **Numeric ranges.** Replace `as_u64().map(|n| n as u16)` with `u16::try_from`
  (and equivalent checked conversion for `usize` fields where the source could
  exceed the target on 32-bit). Apply this to `max_length`,
  `preferred_width`, `preferred_height`, `min_selections`, and
  `max_selections`. On overflow, return an `InvalidInput` error that names the
  field, the offending value, and the column context.
- **Column configuration types.** For optional fields, keep absence as the
  defaulting behavior but reject present values of the wrong JSON type. For
  example, `"initial": 1` on a `text-input`, `"required": "yes"` on a choice
  column, and `"scrollbar": "false"` on a text-area column must produce
  `InvalidInput` rather than being ignored.
- **Unexpected cell types.** In `parse_cell_value`, reject JSON values that do
  not match the column's cell type with an `InvalidInput` error including
  column/row context, instead of `other.to_string()` stringification or silent
  `false` for malformed booleans.
- **Documented permissive paths stay.** Keep permissive parsing only where it is
  an intentional, documented compatibility contract. **Decision:** keep these
  existing row-value conveniences and document them: boolean rows may use
  booleans, numbers, or the strings `true`, `on`, `yes`, `1`, `false`, `off`,
  `no`, `0`; text-area rows may use either an array of strings or one string
  split on newlines; choose-many rows may use either an array of strings or one
  comma-separated string. Everything else becomes an error.
- Route these errors through the same `InvalidInput`-class CLI error surface as
  F3 so the user sees one consistent diagnostic style.

**Success criteria.**

- Oversized `preferred_width` / `preferred_height` (> `u16::MAX`) and oversized
  `max_length` / `min_selections` / `max_selections` on 32-bit targets produce
  a clear `InvalidInput` error naming the field and value; they never truncate
  silently.
- Present-but-wrong-type column config fields produce `InvalidInput` errors
  instead of defaulting as if absent.
- A non-string row value for a string-typed cell, and an invalid boolean row
  value, produce `InvalidInput` errors with column/row context rather than being
  coerced.
- Any coercion that remains is documented as an intentional compatibility
  contract.

---

### F5 — Relax choice hotkey modifier matching (Low)

**Problem.** `choose_one.rs:576-595` and `choose_many.rs:582-601` match modifiers
with `match event.modifiers { KeyModifiers::CONTROL => ..., KeyModifiers::ALT =>
... }`. This is exact equality: a terminal that includes a benign extra bit
(e.g. `SHIFT` for an uppercase chord, producing `CONTROL | SHIFT`) fails to
match an otherwise valid hotkey. The rest of the package (Ctrl-C, table
navigation) consistently uses `.contains(...)`.

**Decision.**

- Replace the exact `match` with `.contains(...)` checks:
  `event.modifiers.contains(KeyModifiers::CONTROL)` selects the ctrl-hotkey map;
  `event.modifiers.contains(KeyModifiers::ALT)` selects the alt-hotkey map.
- Define the ambiguous `CONTROL | ALT` case explicitly. **Decision:** when both
  CONTROL and ALT are present, treat it as ambiguous and match **neither** map
  (fall through), because AltGr-style chords on some layouts report `CTRL | ALT`
  and must not be hijacked as a hotkey. Document this choice with a `// WHY`
  comment at the branch.
- Apply identically to both `choose_one` and `choose_many` so their hotkey
  semantics stay in lockstep with `choice_state` helpers.

**Success criteria.**

- `CONTROL | SHIFT` + a mapped ctrl-hotkey char selects and submits that option.
- `ALT | SHIFT` + a mapped alt-hotkey char selects and submits that option.
- `CONTROL | ALT` matches neither hotkey map (no accidental selection).
- Existing bare-`CONTROL` / bare-`ALT` hotkey behavior is unchanged.

---

## Testing Plan

Mirrors the review's "Testing Gaps" section. All tests follow the package
conventions (Writer-seam unit tests, `drive_event_loop` synthetic events,
`TestBackend`) — see the [biscuit-tui skill](../../../.claude/skills/biscuit-tui/SKILL.md)
and the [rust-testing skill](../../../.claude/skills/rust-testing/SKILL.md).

- **F1** — Regression test that terminal preparation failure *after* raw mode is
  enabled leaves raw mode disabled. This needs a small injectable
  terminal-preparation seam (or fault injection on the alternate-screen step)
  rather than real terminal faults; assert raw mode is off after the forced
  `Err`.
- **F2** — `#[cfg(windows)]` test for `question` with stdout captured and a
  console attached (validated primarily via the CI Windows runner). The test
  must fail if prompt bytes enter captured stdout or if the cursor-position
  probe times out. Add a cross-platform compile check that the Windows redirect
  type builds for the active Windows CI target.
- **F3** — `try_new` tests for: row-length mismatch, duplicate IDs, unknown IDs,
  missing IDs, and cell-type mismatches (each asserts the specific
  `InputTableError` variant + context). A test that `new` still panics on the
  same inputs.
- **F4** — CLI `input-table` tests for oversized `preferred_width` /
  `preferred_height`, oversized `min_selections` / `max_selections` on 32-bit
  where practical, present-but-wrong-type column config fields, non-string row
  values for string cells, and invalid boolean row values — each asserting an
  `InvalidInput` error with field/column context, not silent coercion.
- **F5** — Hotkey tests for `CONTROL | SHIFT`, `ALT | SHIFT` (both match), and
  `CONTROL | ALT` (matches neither), in both `choose_one` and `choose_many`.

---

## Risk & Sequencing Notes

- **F1** is the priority: it is the only finding that can corrupt the user's
  shell. It is also self-contained and low-risk to implement.
- **F3 -> F4** share the `InvalidInput` CLI error surface and should be
  implemented together so the CLI routes library validation (`InputTableError`)
  and JSON-boundary validation through one consistent diagnostic path.
- **F2** is the largest effort and the only one that cannot be fully verified on
  the macOS reviewing host; lean on the CI matrix for Windows validation and
  keep the Unix path strictly untouched.
- **F5** is small and isolated; safe to land independently.
- All changes are surgical (Rule 3): no component-model restructuring, no
  formatting sweeps. `new`'s signature and the `prepare_terminal`/`TerminalGuard`
  contract are preserved for backward compatibility.

---

## Quality Gates

Before this work is considered complete, from `biscuit-tui/`:

- `just test` — passes for `biscuit-tui` and `biscuit-tui-cli`.
- `just test-l2` — passes (terminal-lifecycle changes).
- `just lint` — clippy clean for both crates.
- Cross-compile check for the active Windows CI target succeeds (F2).
- Drift updates applied: root `docs/dependencies.md` (windows-sys),
  component/CLI READMEs for documented permissive-parsing contracts (F4), and
  the biscuit-tui skill because the public API surface changes (F3 `try_new` /
  `InputTableError`).

---

## Open Questions

None. The inline review resolves the specification-level design choices above.
Implementation may still uncover platform details in F2; the required outcome
is fixed even if the first Windows handle strategy needs adjustment.
