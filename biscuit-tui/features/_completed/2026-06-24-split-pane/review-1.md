---
ready: true
agent: codex/default
created: "2026-06-24T15:06:21"
---

# SplitPane Review 1

## Production Readiness

Ready for production: **yes**.

## Findings

No blocking findings.

The implementation matches the v1 scope in the spec: `SplitPane` is a geometry-only layout primitive in `core`, it exports `SplitPane`, `SplitDirection`, and `SplitRatio` through the expected public surfaces, keeps `ResolvedAxis` private, avoids a render wrapper and CLI command, and adds the required `ChooseOneState` active-row accessors for the master/detail pattern without changing `ChoiceOption`.

## Verification-Level Review

| Requirement | Strongest verification found | Appropriate level? | Notes |
| --- | --- | --- | --- |
| `SplitPane::split(area)` returns two in-bounds, non-overlapping rects with full cross-axis extent | Level 1 | Yes | Pure integer geometry; no terminal renderer or input encoder involved. |
| Default 50/50 behavior, explicit `Horizontal` / `Vertical`, and `Auto` resolution including square tie-break | Level 1 | Yes | Covered by geometry unit tests and doctests. |
| `Percent`, `FirstFixed`, and `SecondFixed` ratio semantics, including constructor and builder clamping | Level 1 | Yes | Covered by unit tests, including raw literal normalization through `with_ratio` and `split()`. |
| Degenerate tiny areas, fixed-length overflow, and `gap >= axis` behavior never panic or overflow | Level 1 | Yes | Covered by named degenerate tests plus the acceptance-invariant sweep. |
| `gap` placement, fixed-pane exact length, flexible-pane gap absorption, and spare-cell behavior | Level 1 | Yes | Covered by unit tests; no real-terminal rendering requirement because `gap` is only returned geometry in v1. |
| Public API availability from documented surfaces | Level 1 | Yes | Covered by compile tests and doctests for the documented `core` paths. |
| `ChooseOneState::active_option`, `active_value`, and `active_description` mirror the active highlight and handle empty/disabled rows | Level 1 | Yes | State/API behavior only; tests cover empty options, initial row, navigation, disabled passthrough, selected-vs-active distinction, and caller-owned description lookup. |
| Real terminal rendering of split panes, divider glyphs, mouse resizing, focus routing, or keyboard activation | Not applicable | Yes | These are explicitly out of v1 scope. No Level 2/Level 3 tests are required for this feature as implemented. |

## Notes

- `just test` passed for the `biscuit-tui` package area.
- `just lint` passed for `biscuit-tui` and `biscuit-tui-cli`.
- `just doctest` passed for `biscuit-tui`; `biscuit-tui-cli` has no library doctests.
