---
ready: false
agent: codex/default
created: 2026-06-26T14:08:23
implemented: true
---

# Review 2

The implementation closes most of the original remediation work, but I do not
consider the feature production-ready yet. The remaining issues are primarily
verification-level mismatches for user-observable terminal behavior, plus one
public API contract mismatch in `InputTableState::try_new`.

## Findings

### High: Windows captured-stdout behavior is not verified at the required level

Spec requirement: on Windows, `question` with stdout captured and a console
attached must render the prompt to the console while captured stdout receives
only the submitted value.

Strongest verification present: Level 1, Windows-only in-process tests in
`lib/src/core/standalone/tests.rs`. The tests synthesize a pipe by calling
`SetStdHandle` inside the test process and then assert handle/file-type behavior
and sentinel routing. They do not spawn the `question` binary with stdout
captured in an actual console/terminal context, and the active-path test returns
early when stderr is not a terminal (`tests.rs:1094-1099`), which is the normal
nextest shape.

Why this is a gap: F2 is user-observable terminal behavior, not only handle
bookkeeping. The spec explicitly calls for a Windows test or CI reproduction
showing the captured-stdout prompt shape. The current tests are useful, and
`cargo check --target x86_64-pc-windows-msvc -p biscuit-tui` passes, but this
does not prove the CLI boundary or real console rendering contract.

Relevant code/tests:

- `biscuit-tui/lib/src/core/standalone/tests.rs:1080`
- `biscuit-tui/lib/src/core/standalone/tests.rs:1094`
- `biscuit-tui/lib/src/core/standalone/tests.rs:1138`

Recommended fix: add a Windows-gated integration test that spawns `question`
under the captured-stdout shape with stderr/console still interactive, submits a
value, and asserts the captured stream contains only the value and no TUI bytes.
If this must be run outside nextest because stderr capture prevents the shape,
wire it into a Windows CI/manual reproduction recipe and document the command in
the review/feature notes.

### High: Ctrl/Alt+Shift hotkey behavior is only covered by manufactured `KeyEvent`s

Spec requirement: `CONTROL | SHIFT` plus a mapped Ctrl hotkey and `ALT | SHIFT`
plus a mapped Alt hotkey must activate the hotkey in both `choose-one` and
`choose-many`; `CONTROL | ALT` must match neither map.

Strongest verification present: Level 1 unit tests. The tests construct
`KeyEvent::new(..., KeyModifiers::CONTROL | KeyModifiers::SHIFT)` and
`KeyEvent::new(..., KeyModifiers::ALT | KeyModifiers::SHIFT)` directly, then
call `handle_event`.

Why this is a gap: the behavior is a keybinding/user-input requirement. The
unit tests prove reducer logic once crossterm has already produced those
modifiers, but they do not prove that any supported terminal emits the expected
bytes for the physical chord, nor that those bytes decode into the desired
`KeyEvent`. Per the review instructions, byte-manufactured or in-process tests
are insufficient for production readiness of user-observable key behavior.

Relevant tests:

- `biscuit-tui/lib/src/components/choose_one/tests.rs:415`
- `biscuit-tui/lib/src/components/choose_one/tests.rs:433`
- `biscuit-tui/lib/src/components/choose_many/tests.rs:82`
- `biscuit-tui/lib/src/components/choose_many/tests.rs:100`

Recommended fix: add Level 3 OS-keyboard-injection coverage for at least one
supported GUI terminal path that verifies the physical Ctrl+Shift and Alt+Shift
chords select/toggle the intended option. Keep the current Level 1 tests as the
fast reducer contract.

### Medium: `try_new` does not publicly return `MissingColumnId`

Spec requirement: `InputTableState::try_new` must return typed
`InputTableError` variants for row-length mismatch, duplicate IDs, unknown IDs,
missing IDs, and typed cell mismatches.

Current behavior: `try_new` checks row length before calling `validate_row`.
For a public caller using a normal unique-column schema, a missing cell is
reported as `RowShapeMismatch`, not `MissingColumnId`. The test suite documents
this by testing `MissingColumnId` only through the private `validate_row`
helper, not through `InputTableState::try_new`.

Why this is a gap: the exported error variant exists, but the public API
success criterion is not met. The implementation may be defensible if
`RowShapeMismatch` is the intended public diagnostic for under-length rows, but
that is not what the spec says.

Relevant code/tests:

- `biscuit-tui/lib/src/components/input_table/table.rs:130`
- `biscuit-tui/lib/src/components/input_table/table.rs:138`
- `biscuit-tui/lib/src/components/input_table/table/tests.rs:788`

Recommended fix: either change `try_new` validation ordering/semantics so a
missing configured column can surface as `InputTableError::MissingColumnId`
through the public constructor, or amend the spec and public docs to say
under-length rows return `RowShapeMismatch` and `MissingColumnId` is reserved
for lower-level/future normalization paths.

## Verification Level Matrix

| Requirement | Strongest observed verification | Result |
|---|---:|---|
| F1: failed terminal setup unwinds raw mode/alt-screen state | Level 1 fault-injected unit tests | Acceptable for transactional state logic |
| F2: Windows captured stdout renders prompt to console and value to captured stream | Level 1 Windows-only in-process handle tests; macOS cross-check only compiled library | Gap, needs Windows CLI/real-console reproduction |
| F3: `try_new` typed errors for invalid table rows | Level 1 unit tests | Mostly acceptable; `MissingColumnId` public path gap |
| F4: strict `input-table` JSON validation | Level 1 CLI command unit tests | Acceptable for JSON boundary parsing |
| F5: relaxed Ctrl/Alt+Shift hotkey matching | Level 1 manufactured `KeyEvent` unit tests | Gap, needs Level 3 for physical key behavior |

## Notes

The code changes themselves are generally surgical and aligned with the
existing architecture. F1, F3 except the missing-column nuance, and F4 look
substantially implemented. The Windows redirect implementation also compiles
for `x86_64-pc-windows-msvc` from this macOS host.

## Checks Run

- `cargo check --color=never -p biscuit-tui -p biscuit-tui-cli` — passed.
- `cargo check --color=never -p biscuit-tui --target x86_64-pc-windows-msvc` — passed.
- `cargo test --color=never -p biscuit-tui input_table::table::tests::try_new_returns --lib` — passed.
- `cargo test --color=never -p biscuit-tui-cli input_table --bin question` — passed.
- `cargo test --color=never -p biscuit-tui chord_matches --lib` — passed.
- `cargo test --color=never -p biscuit-tui prepare --lib` — passed.

I did not run `just test-l2`, `just test-l3`, or `just lint` during this review.
