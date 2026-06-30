---
ready: false
agent: "codex/default"
created: "2026-06-26T11:08:02"
implemented: true
---

# Review 2

Not ready for production. The iteration fixes the prior concrete implementation issues: operation-file over-cap now counts mode-valid candidates, the detail block is built from structured terminal components, and the `file`/`file[]` missing-property chooser has stronger L2 and L3 tests. One spec surface is still under-verified: the operation-file ENTER autocomplete UI itself, especially the single-match confirmation path.

## Findings

### High: operation-file autocomplete presentations are not verified at the required terminal levels

Requirement: `claudine compose|inline-compose|sequence <partial>` must use runtime autocomplete for the operation file. A single match must render the lightweight confirmation dialog ending in `Use this file? (Y/n)` with no chooser, while multiple matches must render a `ChooseOne` two-pane chooser with `SplitPane::Auto` layout and active-item detail updates ([spec.md](spec.md:130), [spec.md](spec.md:137), [spec.md](spec.md:160), [spec.md](spec.md:208), [spec.md](spec.md:210)).

Implementation/test coverage: production routes one candidate to `confirm_one_file` and multiple candidates to `choose_one_file` in [operation_file.rs](../../cli/src/completion/operation_file.rs:126). However, the new L2 tests explicitly drive a missing schema `file` / `file[]` property, not an operation-file partial ([level2_auto_complete_chooser.rs](../../cli/tests/level2_auto_complete_chooser.rs:1), [level2_auto_complete_chooser.rs](../../cli/tests/level2_auto_complete_chooser.rs:380)). The new L3 tests likewise run `compose --goose <resolved plan.md>` and then exercise the missing-property chooser ([level3_auto_complete_chooser.rs](../../cli/tests/level3_auto_complete_chooser.rs:3), [level3_auto_complete_chooser.rs](../../cli/tests/level3_auto_complete_chooser.rs:238), [level3_auto_complete_chooser.rs](../../cli/tests/level3_auto_complete_chooser.rs:260)). The only operation-file PTY path I found is the ignored performance helper, which waits for a multi-match chooser marker and sends a raw Esc byte ([completion_perf.rs](../../cli/tests/completion_perf.rs:335)).

Impact: the spec's operation-file UI can regress independently of the missing-property chooser and still pass the current L2/L3 suite. Examples: a single valid `plan` match could accidentally show the chooser instead of the confirmation dialog; the confirmation dialog could stop accepting `Y`/Enter or `n`/Esc; or the operation-file chooser could lose its mode badge/detail wiring while the file-property chooser stays green. Under the review rubric, operation-file chooser rendering/layout needs L2 capture, and operation-file key behavior needs L3 OS keyboard coverage.

Fix direction: add focused terminal tests that start from an unresolved operation-file partial. Cover at least:

- L2 single-match: capture the confirmation dialog, assert `Use this file? (Y/n)`, assert no chooser markers/SplitPane list.
- L3 single-match: OS-inject Enter/Y to accept and Esc/n to cancel, proving provider launch vs no launch.
- L2 multi-match: capture the operation-file `ChooseOne` two-pane layout in wide and tall terminals, with active detail derived from the highlighted item.
- L3 multi-match: OS-inject Down/Up and Enter through the operation-file chooser.

## Verification Level Summary

- Shared bounded walker, mode-valid over-cap counting, no-match/over-cap/non-TTY errors, YAML sequence contract, no config override, `__complete` contract, bare `file` fallback, and comma continuation: Level 1 source/unit/subprocess coverage present.
- Missing-property `file` and `file[]` chooser behavior: Level 2 capture now checks widget-specific markers and value shape; Level 3 OS keyboard injection now covers Enter, Esc, arrow navigation, and Space toggling.
- Detail block rendering contract: unit coverage now checks BlockQuote border, OSC8, no-schema styling, and inline path layout; L2 captures broad detail presence through the chooser.
- Operation-file autocomplete presentations: strongest direct coverage is Level 1/unit plus an ignored PTY performance helper; required L2/L3 coverage is missing.

## Notes

I did not run the full test suite for this review. The findings above come from source and test inspection.
