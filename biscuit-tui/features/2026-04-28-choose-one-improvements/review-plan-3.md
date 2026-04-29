---
phases: 4
start_phase: 3
source_files_during_phase_3:
  - lib/src/components/choose_many.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
packages:
  - tui-chrome
---
# Review-3 Fix Plan

Addresses all findings from review-3.md for the Choose One Improvements feature.

## Phase 1: Fix Horizontal Layout Hotkey Badge Collision (Finding 1)

The layout engine packs horizontal rows with a vertical increment of 1, but
badges paint at `screen_y + 1`, overwriting the next row's options.

### Root Cause

`ChoiceLayout::horizontal` in `choice_layout.rs:87` uses `y += 1` when
wrapping rows. `render_horizontal` in `choice_render.rs:563` paints badges at
`screen_y + 1`. For multi-row horizontal layouts, row N's badge overwrites
row N+1's option content.

### Fix

Update `ChoiceLayout::horizontal` to accept a `row_height: u16` parameter
(defaulting to `1` when badges are hidden). The caller passes `2` when
`hotkey_display != Hidden`. This reserves a screen row for badge content
between option rows.

### Files Modified

1. **`lib/src/components/choice_layout.rs`**
   - Add `row_height: u16` parameter to `ChoiceLayout::horizontal`
   - Change `y += 1` → `y += row_height` at line 87
   - Set `ChoiceItemRect.height = row_height` instead of hardcoded `1`
   - Update all call sites and tests

2. **`lib/src/components/choice_render.rs`**
   - In `render_horizontal`, compute `row_height` from `self.hotkey_display`:
     - `Hidden` → `1`
     - `CtrlHeld` / `AltHeld` → `2`
   - Pass `row_height` to `ChoiceLayout::horizontal`
   - In `compute_layout`, pass the same `row_height`
   - Add a render-buffer test that verifies a 2-row horizontal layout with
     badges does NOT overwrite row 1's option content

3. **`lib/src/components/choose_one.rs`** (and `choose_many.rs`)
   - Any call sites that invoke `ChoiceLayout::horizontal` directly must
     pass the new parameter; if they go through `ChoiceRenderContext::compute_layout`,
     no change needed

### New Tests

- `choice_layout.rs`: test that `horizontal` with `row_height=2` places
  row 1 at `y=2` instead of `y=1`
- `choice_render.rs`: test that a multi-row horizontal layout with
  `CtrlHeld` badges renders options on even rows and badges on odd rows,
  with no collision

### Existing Tests to Update

- All `ChoiceLayout::horizontal` calls in `choice_layout.rs` tests need the
  extra `row_height` argument (pass `1` for existing behavior)
- `horizontal_layout_navigation_does_not_visit_badge_rows` — update the
  constructor call

---

## Phase 2: Add Discrete `--hotkey-badges` Values (Finding 2)

The spec requires `hidden`, `ctrl`, `alt`, and `auto`. The current implementation
has `auto`, `always`, and `never`.

### Changes

1. **`cli/src/commands/common_choose.rs`**
   - Add `Ctrl` and `Alt` variants to `HotkeyBadgesArg`:
     ```rust
     pub enum HotkeyBadgesArg {
         #[default]
         Auto,
         Always,
         Never,
         Ctrl,
         Alt,
     }
     ```
   - Update `resolve_hotkey_badges`:
     ```rust
     HotkeyBadgesArg::Ctrl => Some(HotkeyDisplayMode::CtrlHeld),
     HotkeyBadgesArg::Alt => Some(HotkeyDisplayMode::AltHeld),
     ```
   - Note: `Always` currently maps to `CtrlHeld` (forced Ctrl badge). This
     remains for backward compatibility but is semantically equivalent to
     `Ctrl`. Consider hiding `always` from help output in a future cleanup.
   - Add `#[clap(alias = "hidden")]` on `Never` for spec compliance if
     needed, but since clap already renders kebab-case `never`, and the spec
     says `hidden`, add an alias:
     `#[clap(alias = "hidden")]` on the `Never` variant

2. **No library changes required** — `HotkeyDisplayMode` already has
   `CtrlHeld` and `AltHeld`, which the CLI maps to correctly.

### New Tests

- `resolve_hotkey_badges_ctrl_forces_ctrl_held` — `Ctrl` →
  `Some(HotkeyDisplayMode::CtrlHeld)`
- `resolve_hotkey_badges_alt_forces_alt_held` — `Alt` →
  `Some(HotkeyDisplayMode::AltHeld)`
- `resolve_hotkey_badges_hidden_alias` — verify clap parses `--hotkey-badges hidden`
  as `Never` (integration-level or clap-derive test)

### Existing Tests to Update

- None — existing tests for `Auto`, `Always`, `Never` remain valid

---

## Phase 3: Document ChooseMany ESC Decision (Finding 3)

### Decision

**No code change.** The spec states: "The default key-bindings for ChooseMany
should stay as they are." The current behavior where `ChooseMany` ESC returns
`EventOutcome::Cancelled` (mapped to exit code 1) is consistent with this
directive.

### Rationale

- `ChooseOne` ESC was explicitly changed to restore + submit (exit code 0) as
  a "Breaking change" noted in the spec
- `ChooseMany` has no such directive; it "stays as they are"
- The existing doc comments in `choose_many.rs` CLI correctly document `Ok(1)`
  on ESC

### Action

Add a clarifying comment in `choose_many.rs` (lib) near the cancel binding
at line 523:
```rust
// Spec: "The default key-bindings for ChooseMany should stay as they
// are." ESC returns Cancelled (exit code 1) — unlike ChooseOne which
// restores and submits.
return EventOutcome::Cancelled;
```

---

## Phase 4: Lint, Test, Verify

Run after all phases are complete:

```bash
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
cargo test -p tui-chrome -p tui-chrome-cli
```

Fix any warnings or failures before marking the feature as ready.

---

## Summary

| Phase | Scope | Risk | Files |
|-------|-------|------|-------|
| 1 | Fix horizontal badge collision | High (functional bug) | `choice_layout.rs`, `choice_render.rs` |
| 2 | Add `ctrl`/`alt` hotkey badge modes | Low (additive) | `common_choose.rs` |
| 3 | Document ChooseMany ESC decision | None (comment only) | `choose_many.rs` |
| 4 | Lint + full test suite | — | — |
