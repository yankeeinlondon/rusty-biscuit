# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Breaking

- `run_standalone` now returns `io::Error` with the new
  [`ABORTED_KIND`] sentinel (`io::ErrorKind::ConnectionAborted`) when
  the user presses `Esc`, distinct from the existing
  [`CANCELLED_KIND`] (`io::ErrorKind::Interrupted`) which continues to
  signal `Ctrl+C`. Callers matching on `Interrupted` to treat any
  cancellation uniformly must opt in to the new variant explicitly.

### Added

- `core::fuzzy::FuzzyFilter` — bigram/nucleo-matcher-backed fuzzy
  scorer shared by the choose components.
- `core::sort::SortOrder` — `Natural` / `Reverse` / `Asc` / `Desc`
  projection applied before state construction.
- `core::frame::{BorderStyle, Margin, HeightSpec, FrameChrome,
  FrameChromeConfig}` — border/margin wrapper widget and height spec
  used by the new CLI chrome flags.
- `LoopExit<V>` — three-way result (`Submitted` / `CtrlC` / `Esc`)
  surfaced from `drive_event_loop_with_chrome`, allowing the CLI to
  distinguish aborts from interrupts.
- `ChoiceInput::with_filter_enabled(bool)` — opt-in inline fuzzy
  search (on by default for the CLI, off for legacy library users).
- `ChooseOneState::with_initial_value(&str)` /
  `ChooseManyState::with_initial_values(&[&str])` — pre-select by
  value rather than by id.
- `ChooseManyState::select_all()` / `deselect_all()` — bulk toggles
  for `Ctrl+A` / `Ctrl+D`.
- `KeyBindings::select_all` / `deselect_all` fields (defaulting to
  `Ctrl+A` / `Ctrl+D`).
- `run_standalone_with_chrome` — variant that wraps the component in a
  `FrameChromeConfig` (border + margin) without callers having to
  compose the widget stack themselves.
- `core::split_pane::{SplitPane, SplitDirection, SplitRatio}` —
  geometry-only two-pane layout primitive
  (`SplitPane::split(area) -> (Rect, Rect)`). A container/layout
  primitive like `FrameChrome`, not an input (captures no value, no
  `HandleEvent`). `Auto` direction is resolved from the area's shape
  each split; 50/50 default; ratios clamp on construction
  (`Percent` to `1..=99`, `*Fixed` to `>= 1`) so no pane is
  voluntarily starved.
- `ChooseOneState::active_option` / `active_value` /
  `active_description` — active-item accessors keyed off `hover()`
  (the highlighted row, distinct from the submitted
  `selected_value()`). They return the option as-is — including a
  `disabled` one — and `None` when the list is empty. The entry
  point for a `SplitPane` master/detail pane that derives its
  content from the active highlight each frame; `ChoiceOption` is
  unchanged.
