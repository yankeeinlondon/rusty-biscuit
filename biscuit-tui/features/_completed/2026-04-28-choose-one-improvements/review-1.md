---
ready: true
agent: codex
---

# Review 1

The feature is not ready for production. The focused test suite passes (`cargo test -p biscuit-tui -p biscuit-tui-cli`), but several specification requirements are either unimplemented or only partially scaffolded, and the current tests do not catch those gaps.

## Findings

### 1. `ChoiceInput::with_sort` is a no-op for library consumers

`ChoiceInput` exposes `sort` and a `with_sort` builder, but neither `ChooseOneState::new` nor `ChooseManyState::new` applies it. Both constructors only handle `shuffle_options` before building hotkey maps and cached labels:

- `biscuit-tui/lib/src/components/choose_one.rs:79`
- `biscuit-tui/lib/src/components/choose_many.rs:76`
- `biscuit-tui/lib/src/components/choose.rs:279`

The CLI manually calls `apply_sort`, so CLI paths sort, but embedded/library usage does not. This violates the design that `ChoiceInput` owns the option ordering policy. Add constructor-level sorting before hotkey/cache construction, and add unit tests for `ChooseOneState::new(input.with_sort(...))` and `ChooseManyState::new(input.with_sort(...))`.

### 2. CLI `--sort inverse` is missing

The spec requires `--sort <natural|inverse|asc|desc>` and says `reverse` may only remain as a hidden compatibility alias. The implementation exposes `Reverse` as the clap value:

- `biscuit-tui/cli/src/commands/common_choose.rs:156`

That means `--sort inverse` is rejected while `--sort reverse` is public. This also affects shell completions because they are generated from clap. Rename the CLI enum variant to `Inverse`, map it to `SortOrder::Reverse`, and keep `reverse` hidden only if needed for compatibility. Add CLI and completions tests that assert `inverse` is accepted and `reverse` is not presented as the canonical value.

### 3. File object sources drop `value` and `hotkey`

The design requires `--file` JSON/YAML/TOML arrays to support object records with `label`, `value`, and optional `hotkey`. The current parsers collapse objects into a single string, preferring `label` and discarding `value` and `hotkey`:

- `biscuit-tui/cli/src/option_sources.rs:164`
- `biscuit-tui/cli/src/option_sources.rs:223`
- `biscuit-tui/cli/src/option_sources.rs:246`

For example, `[{"label":"Red","value":"apple","hotkey":"CTRL+R"}]` becomes just `"Red"`, so the CLI returns `Red` instead of `apple` and loses the explicit shortcut. Introduce an intermediate raw option record instead of `Vec<String>`, preserve object fields through normalization, and test JSON, JSONL/NDJSON, YAML, TOML, and CSV object/row cases.

### 4. Hotkey badge display is not implemented

The spec calls for Ctrl/Alt hotkey badges with orange/yellow backgrounds, visible while modifiers are held or via the fallback described in the design. The implementation has `HotkeyDisplayMode`, but neither choice state stores it, and render contexts always set it to `Hidden`:

- `biscuit-tui/lib/src/components/choose_one.rs:54`
- `biscuit-tui/lib/src/components/choose_many.rs:56`
- `biscuit-tui/lib/src/components/choice_render.rs:76`
- `biscuit-tui/lib/src/components/choice_render.rs:100`

There is no badge rendering in vertical or horizontal mode. Add state, event handling for modifier visibility/fallback, renderer support, and buffer tests for Ctrl and Alt badge styling/placement.

### 5. `Padding::default()` contradicts the library-level default requirement

The spec says padding should default to `1` at the library level. `FrameChromeConfig::default()` uses `Padding::uniform(1)`, but `Padding` itself derives `Default`, so `Padding::default()` is all zeroes:

- `biscuit-tui/lib/src/core/frame.rs:168`
- `biscuit-tui/lib/src/core/frame.rs:180`

This creates an API trap and inconsistent internal checks such as `FrameChromeConfig::is_empty()` comparing against zero padding. Implement `Default` manually for `Padding` as `uniform(1)`, add an explicit `Padding::zero()` or `Padding::none()` for no-op chrome, and update tests/docs accordingly.

### 6. Active choice styling is still mostly theme-driven and not spec-driven

The spec requires faint active backgrounds with selectable color variants (`grey`, `green`, `yellow`, `red`) and foreground contrast based on detected terminal background. `ActiveChoiceColor` exists, but it is not connected to `ChoiceInput`, state, rendering, CLI, or docs. Rendering still uses `ComponentTheme::selected_style`, whose default is black-on-cyan:

- `biscuit-tui/lib/src/components/choose.rs:67`
- `biscuit-tui/lib/src/core/theme.rs:104`
- `biscuit-tui/lib/src/components/choice_render.rs:221`
- `biscuit-tui/lib/src/components/choice_render.rs:392`

Wire `ActiveChoiceColor` into the choice configuration/render path, resolve it with `TerminalStyle`, and add render tests for dark/light/unknown backgrounds and the span-width rule.

## Test Coverage Gaps

The existing suite is broad and passes, but it misses important acceptance cases:

- Library sorting via `ChoiceInput::with_sort` on both choice components.
- CLI acceptance/completion of `--sort inverse`.
- File and markdown object records preserving `label`, `value`, and `hotkey`.
- Hotkey badge visibility, badge styling, and horizontal badge placement.
- `Padding::default()` itself, not only `FrameChromeConfig::default()`.
- Active color variants and foreground contrast under dark/light terminal styles.

## Ergonomics And Performance Notes

The strongest ergonomic improvement is to replace the CLI source pipeline's `Vec<String>` with a typed raw option record. That avoids lossy object parsing, centralizes hotkey/value handling, and removes the need to re-encode object data as `label::value` strings.

For rendering, keep the shared layout/render helpers, but pass a resolved per-render style context rather than repeatedly relying on `ComponentTheme::selected_style`. That will make the active color and terminal-contrast logic testable without expanding state complexity.
