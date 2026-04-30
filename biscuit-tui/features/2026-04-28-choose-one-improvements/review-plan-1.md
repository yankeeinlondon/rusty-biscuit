---
ready: false
agent: codex
source_review: review-1.md
package_area: biscuit-tui
crates:
  - tui-chrome (biscuit-tui/lib)
  - tui-chrome-cli (biscuit-tui/cli)
---

# Review-1 Remediation Plan

This plan addresses every finding from `review-1.md` for the
`2026-04-28-choose-one-improvements` feature. It is broken into
**six implementation phases plus a final verification phase**. Each phase
is independently shippable: it ends with a green `cargo test -p tui-chrome
-p tui-chrome-cli` and a clean `cargo clippy -p tui-chrome -p tui-chrome-cli
--all-targets -- -D warnings`.

Phases are ordered to minimize churn:

1. Pure library invariants first (sort wiring, `Padding::default`).
2. Cross-cutting CLI refactor (raw option record) before features that
   depend on hotkey/value preservation.
3. CLI surface (`--sort inverse`) once the underlying library naming is
   already correct.
4. New rendering features (active-color wiring, hotkey badges) last,
   because they depend on the typed source records and the stable
   `HotkeyDisplayMode` plumbing.

Findings → phase mapping:

| Finding (review-1.md) | Phase |
| --- | --- |
| 1. `ChoiceInput::with_sort` no-op in state ctors | Phase 1 |
| 5. `Padding::default()` should be `uniform(1)` | Phase 2 |
| 3. File-object sources drop `value` / `hotkey` (raw record) | Phase 3 |
| 2. CLI `--sort inverse` rename | Phase 4 |
| 6. `ActiveChoiceColor` not wired through render path | Phase 5 |
| 4. Hotkey badge display unimplemented | Phase 6 |
| All test-coverage gaps | distributed per phase + Phase 7 |

---

## Phase 1 — Apply `ChoiceInput::with_sort` in state constructors

**Closes review finding 1.**

### Goal

`ChooseOneState::new` and `ChooseManyState::new` must apply
`input.sort` (via the same logic the CLI's `apply_sort` uses) before
building the hotkey map and `cached_labels`. Library consumers must see
the same ordering as CLI consumers.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/lib/src/components/choose.rs` | Move `apply_sort` logic into a pure `pub(crate)` function on `ChoiceInput<V>` (e.g. `ChoiceInput::sort_options_in_place`). It must sort by `option.label` for `Asc`/`Desc`, reverse for `Inverse`/`Reverse`, and no-op for `Natural`. Keep the existing `with_sort` builder. |
| `biscuit-tui/lib/src/components/choose_one.rs` | In `ChooseOneState::new`, after the `selection_mode` assignment and before the existing `shuffle_options` block (or merged with it), call `input.sort_options_in_place()`. Order matters: sort first, then optionally shuffle, then build hotkeys/labels. The existing CLI `apply_sort` becomes a thin re-export or is removed (see Phase 4). |
| `biscuit-tui/lib/src/components/choose_many.rs` | Same insertion in `ChooseManyState::new`. |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Remove (or convert to a deprecated thin shim that delegates to the library) the duplicated `apply_sort` helper. Update `commands/choose_one.rs` and `commands/choose_many.rs` callers to use `input.with_sort(...)` instead of mutating `input.options` post-hoc. |
| `biscuit-tui/cli/src/commands/choose_one.rs` | Replace `apply_sort(&mut input.options, args.chrome.sort.into())` with `input = input.with_sort(args.chrome.sort.into())` *before* calling `ChooseOneState::new(input)`. |
| `biscuit-tui/cli/src/commands/choose_many.rs` | Same as above for `ChooseManyState::new`. |

### New tests

Library unit tests in `lib/src/components/choose_one.rs` and
`lib/src/components/choose_many.rs`:

- `state_new_applies_with_sort_asc_orders_by_label`
- `state_new_applies_with_sort_desc_orders_by_label`
- `state_new_applies_with_sort_inverse_reverses_natural`
- `state_new_with_sort_natural_is_no_op`
- `state_new_sort_then_shuffle_does_not_panic` (sort + shuffle interplay)
- Existing `apply_sort_*` CLI tests in `common_choose.rs` either move to
  the library next to the new helper, or are kept as integration-style
  asserts on `state.visible_options()` order.

### Verification

```bash
cargo test -p tui-chrome --lib -- choose_one choose_many
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

### Risks / open questions

- `ChoiceInput` currently exposes `pub options` so external callers may
  read it pre-sort. Sorting inside `state::new` is correct per the
  tech-design ("`ChoiceInput` owns the option ordering policy"), but we
  should keep `with_sort` as the configuration entry point — do **not**
  silently sort inside builder mutators that touch `options`.

---

## Phase 2 — `Padding::default()` = `uniform(1)`, plus `Padding::zero()`

**Closes review finding 5.**

### Goal

Match the spec: padding defaults to `1` cell on all sides at the
**library** level. Provide an explicit zero-padding constructor for
chrome that wants no interior spacing. Replace any
`Padding == Padding::default()` "is empty" checks with
`Padding == Padding::zero()`.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/lib/src/core/frame.rs` | Remove `#[derive(Default)]` from `Padding`. Add `impl Default for Padding { fn default() -> Self { Self::uniform(1) } }`. Add `pub const fn zero() -> Self` returning all zeros, and a `pub const fn none() -> Self` alias. Update `FrameChromeConfig::default()` so it can either drop its explicit `padding: Padding::uniform(1)` (now redundant) or keep it as a self-documenting value — pick one and document the choice in the rustdoc. Audit `FrameChromeConfig::is_empty()` (and any `==` comparison against `Padding::default()`) and replace with `Padding::zero()` where the intent is "no padding". |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Audit `--padding` handling: if `--padding 0` was previously equivalent to "no flag passed" via `Padding::default()`, that semantics now flips. Treat unset flags as "use library default" by passing `Option<u16>` and only overriding when present. Confirm `--pt`/`--pb`/`--pl`/`--pr` still merge the way the spec describes (per-side override on top of `--padding`). |
| `biscuit-tui/lib/src/core/frame.rs` (rustdoc) | Update the rustdoc on `Padding` to call out: "Defaults to 1 cell on all sides (matches the spec). Use `Padding::zero()` for chrome with no interior spacing." |
| `biscuit-tui/docs/components/frame_chrome.md` | Reflect the new default and document `Padding::zero()` / `Padding::none()`. |

### New tests

In `lib/src/core/frame.rs`:

- `padding_default_is_uniform_one`
- `padding_zero_is_all_zeros`
- `padding_none_alias_matches_zero`
- `padding_default_shrinks_rect_by_one_on_each_side`
- Update `FrameChromeConfig::default` test to assert
  `config.padding == Padding::uniform(1)` and that
  `FrameChromeConfig::is_empty()` only returns true when the user has
  explicitly set `Padding::zero()` (or equivalent).

In `cli/src/commands/common_choose.rs`:

- Test that `question choose-one --no-border` (or whatever the minimal
  chrome path is) without `--padding` produces a `FrameChromeConfig`
  with `padding == Padding::uniform(1)`.
- Test that explicit `--padding 0` produces `Padding::zero()`.

### Verification

```bash
cargo test -p tui-chrome --lib -- core::frame
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

### Risks / open questions

- This is a **behavior change** for any caller that constructs
  `Padding::default()` expecting zeros. The repo is the only consumer
  (no external crates), so a direct change is acceptable per the
  tech-design's "no deprecation baggage" guidance. Grep for
  `Padding::default()` and `Padding {` literal constructions before
  shipping to confirm we haven't missed a call site.
- Existing CLI snapshot tests that pin border output may need their
  fixtures regenerated because the inner widget area shrinks by one
  cell on each side.

---

## Phase 3 — Typed raw option record (CLI source pipeline refactor)

**Closes review finding 3 and the "ergonomic improvement" recommendation.**
Required before Phase 6 so that hotkey badges have hotkeys to render
when sourced from `--file`.

### Goal

Replace the `Vec<String>` plumbing in `cli/src/option_sources.rs` with a
typed `RawOption` record carrying `label`, optional `value`, optional
`hotkey`, and optional `disabled`. Preserve all three fields end-to-end
through JSON, JSONL, NDJSON, YAML, TOML, CSV, and markdown frontmatter
sources. Delete the lossy
`format!("{}::{}", ...)` re-encoding currently in `parse_csv_file`.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/cli/src/option_sources.rs` | Introduce `pub struct RawOption { pub label: String, pub value: Option<String>, pub hotkey: Option<String>, pub disabled: Option<bool> }`. Change all internal helpers (`parse_csv`, `parse_list`, `parse_rows`, `parse_file`, `parse_json`, `parse_jsonl`, `parse_yaml`, `parse_toml`, `parse_csv_file`, `parse_md`, `parse_dictionary`, `parse_markdown_list`) to return `Vec<RawOption>` instead of `Vec<String>`. Object inputs keep all three keys; string inputs map `label = s; value = None; hotkey = None`. CSV rows of length ≥ 2 set `label = col0; value = Some(col1); hotkey = col2 if present`. The public `resolve_raw_options` returns `Vec<RawOption>`. |
| `biscuit-tui/cli/src/option_sources.rs` | Add a `From<String>` for `RawOption` to simplify positional/stdin string paths. |
| `biscuit-tui/cli/src/choice_normalize.rs` | Update normalization signatures to consume `Vec<RawOption>`. The pipeline (per tech-design §"Label and Value Normalization") now becomes: (1) parse to `RawOption`, (2) strip `[CTRL+X]`/`[ALT+X]`/`[OPT+X]` prefixes from `label` (only when `hotkey` is `None`), (3) split `Label::Value` on `::` only when `value` is `None`, (4) apply `--label` / `--value` conventions, (5) numeric hotkey fallback only when `hotkey` is `None`, (6) build `ChoiceOption` with `hotkey: Option<HotkeySpec>` populated. Hotkey strings such as `"CTRL+R"` from object sources parse via the same parser used for the `[CTRL+X]` prefix. |
| `biscuit-tui/cli/src/commands/choose_one.rs` | Update call sites to feed `Vec<RawOption>` into normalization. |
| `biscuit-tui/cli/src/commands/choose_many.rs` | Same as above. |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Update `build_chrome` / source dispatch to use the new pipeline. |

### New tests

In `cli/src/option_sources.rs` (or a new `cli/src/option_sources_tests.rs`):

- `json_array_of_objects_preserves_label_value_hotkey`
  (input: `[{"label":"Red","value":"apple","hotkey":"CTRL+R"}]`, asserts
  `RawOption { label: "Red", value: Some("apple"), hotkey: Some("CTRL+R"), .. }`).
- `jsonl_object_preserves_value_and_hotkey`
- `ndjson_object_preserves_value_and_hotkey`
- `yaml_array_of_mappings_preserves_value_and_hotkey`
- `toml_table_preserves_value_and_hotkey`
- `csv_three_columns_preserves_value_and_hotkey`
- `csv_two_columns_preserves_value_no_hotkey`
- `json_string_array_yields_label_only_options`
- `non_array_top_level_returns_invalid_source_shape` (per
  `ChoiceCliError::InvalidSourceShape`).
- `markdown_frontmatter_object_array_preserves_value_and_hotkey`

In `cli/src/choice_normalize.rs`:

- `prefix_hotkey_does_not_overwrite_explicit_object_hotkey`
- `delimiter_split_does_not_overwrite_explicit_object_value`
- `numeric_hot_keys_skips_options_with_explicit_hotkey`

In `cli/tests/cli.rs` (assert_cmd integration):

- `choose_one_file_object_returns_value_not_label`
  (asserts `question choose-one --file fixture.json` echoes `apple` not
  `Red` when an object source supplies `value: "apple"`).
- Same for YAML, TOML, JSONL, NDJSON, CSV.

### Verification

```bash
cargo test -p tui-chrome-cli --lib -- option_sources choice_normalize
cargo test -p tui-chrome-cli --test cli
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

### Risks / open questions

- The existing tests that assert `Vec<String>` output will all break.
  Plan to update them in lockstep, not as a follow-up.
- `parse_dictionary` for `--options-from-dictionary` is an older code
  path; preserve its current behavior by mapping to `RawOption` with
  `label` only.
- Hotkey strings from object sources need a single canonical parser.
  Promote the existing prefix parser in `choice_normalize.rs` so that
  `"CTRL+R"`, `"ALT+B"`, and `"OPT+B"` (no surrounding brackets) all
  parse to the same `HotkeySpec`.

---

## Phase 4 — CLI `--sort inverse` rename

**Closes review finding 2.**
Comes after Phase 3 because both touch CLI surface, and Phase 3
significantly reshapes `commands/common_choose.rs`.

### Goal

The clap value enum on `--sort` exposes `natural`, `inverse`, `asc`,
`desc`. `reverse` may remain only as a hidden compatibility alias on
the same variant; it is **not** the canonical value and must not appear
in `--help` or shell completions.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Rename `SortOrderArg::Reverse` → `SortOrderArg::Inverse`. Update the `From<SortOrderArg> for SortOrder` mapping accordingly. Add `#[clap(alias = "reverse")]` on the `Inverse` variant only if we want backward compatibility (decision: yes, with `hide = true` so it does not appear in `--help`). Rustdoc on the variant should call out the alias. |
| `biscuit-tui/lib/src/core/sort.rs` | Confirm `SortOrder` and `OptionSort` already use `Inverse` as the canonical name. If `OptionSort` is the spec-aligned vocabulary and `SortOrder` is the legacy name, this phase is the right time to rename `SortOrder::Reverse` → `SortOrder::Inverse` (with a `pub use Inverse as Reverse` deprecated alias only if needed). The SKILL.md already lists both, so update it to mark `Inverse` canonical. |
| `biscuit-tui/cli/src/commands/common_choose.rs` (tests) | Update `apply_sort_*` tests (or their successors after Phase 1) to reference `SortOrder::Inverse`. |
| `biscuit-tui/docs/cli-reference.md` | Update `--sort <natural\|inverse\|asc\|desc>`. |
| `biscuit-tui/docs/components/choose_one.md`, `choose_many.md` | Same. |
| `biscuit-tui/cli/README.md` | Same. |
| `.claude/skills/biscuit-tui/SKILL.md` | Update the `--sort` example line and the `SortOrder` row. |

### New tests

In `cli/tests/cli.rs` (assert_cmd):

- `sort_inverse_is_accepted_and_reverses_natural_order`
  (run `question choose-one --sort inverse Alpha Beta Gamma`, assert
  the help/output ordering).
- `sort_reverse_is_a_hidden_alias`
  (run `question choose-one --sort reverse Alpha`, assert it succeeds
  but `--help` output does NOT contain `reverse`).
- `sort_help_lists_inverse_not_reverse`
  (`question choose-one --help` contains `inverse`, does not contain a
  visible `reverse`).

In `cli/tests/cli.rs` (completions):

- `completions_zsh_contains_inverse_for_sort`
- `completions_zsh_does_not_present_reverse_as_canonical`
- Same for `bash` and `fish`.

### Verification

```bash
cargo test -p tui-chrome-cli --test cli
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
question completions zsh | grep -F inverse
question completions zsh | grep -F reverse  # should be empty if alias hidden
```

### Risks / open questions

- Renaming the library `SortOrder::Reverse` variant will ripple into
  any downstream callers (currently only inside this workspace). Do
  this as one atomic rename plus a `cargo check --workspace` sweep
  before opening a PR.

---

## Phase 5 — Wire `ActiveChoiceColor` through state, render, CLI, docs

**Closes review finding 6.**

### Goal

`ActiveChoiceColor` becomes a real configuration knob: it lives on
`ChoiceInput`, propagates into `ChooseOneState`/`ChooseManyState`, and
is read by `choice_render.rs` to compute the active row's background
and foreground using `TerminalStyle`. The active highlight must cover
**only** the rendered item width plus one trailing blank cell — never a
full-row `buf.set_style(area, …)`.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/lib/src/components/choose.rs` | Add `pub active_color: ActiveChoiceColor` field on `ChoiceInput<V>` (default `Grey`). Add builder `pub fn with_active_color(mut self, color: ActiveChoiceColor) -> Self`. |
| `biscuit-tui/lib/src/components/choose_one.rs` | Carry `active_color` into `ChooseOneState`. Add `pub fn with_active_color(self, ActiveChoiceColor) -> Self` (sugar that mutates `self.input.active_color`). |
| `biscuit-tui/lib/src/components/choose_many.rs` | Same. |
| `biscuit-tui/lib/src/core/terminal_style.rs` | Add `pub fn resolve_active_style(color: ActiveChoiceColor, bg: TerminalBackground) -> ratatui::style::Style`. The function returns: faint background of the chosen color (Grey/Green/Yellow/Red palette tuned per `Dark`/`Light`/`Unknown`), and a foreground of `White` on `Dark`/`Unknown`, `Black` on `Light`. Bold modifier on, underline modifier off. Document the chosen RGB or 256-color values inline. |
| `biscuit-tui/lib/src/components/choice_render.rs` | Replace the three `theme.selected_style` reads at lines ~231, ~367, ~665 with `resolve_active_style(ctx.active_color, ctx.terminal_style.background)`. Extend `ChoiceRenderContext` with `pub active_color: ActiveChoiceColor`. Ensure the styled span group (indicator + space + label + one trailing space) is the only thing receiving the active style — confirm with a buffer test that the cell *after* the trailing blank is unstyled. |
| `biscuit-tui/lib/src/core/theme.rs` | Leave `selected_style` in place for non-choice components (`BooleanSwitch`, `InputTable`); only the choice render path migrates to `resolve_active_style`. Add a rustdoc note that `selected_style` is no longer used by `ChooseOne`/`ChooseMany`. |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Add `--active-color <grey\|green\|yellow\|red>` (default `grey`) to `ChooseChromeArgs`. Add a `From<ActiveColorArg> for ActiveChoiceColor`. Wire it into `build_chrome` so it ends up on the `ChoiceInput`. |
| `biscuit-tui/docs/components/choose_one.md`, `choose_many.md` | Document `--active-color` and the dark/light/unknown contrast rules. Cross-link to the spec. |
| `biscuit-tui/cli/README.md`, `biscuit-tui/lib/README.md` | Same. |
| `.claude/skills/biscuit-tui/SKILL.md` | Add `--active-color` to the choose-specific flags list. |

### New tests

Library buffer tests using `TestBackend` in `choice_render.rs`:

- `active_row_uses_grey_default_palette_on_dark_background`
- `active_row_uses_green_palette_when_configured`
- `active_row_uses_white_fg_on_dark_bg`
- `active_row_uses_black_fg_on_light_bg`
- `active_row_uses_dark_palette_on_unknown_background` (safer default)
- `active_row_style_covers_only_label_plus_one_blank` (assert that
  `area.x + label_width + 1` is the last styled cell)
- `active_row_style_does_not_underline`

CLI tests:

- `cli_accepts_active_color_grey_green_yellow_red`
- `cli_active_color_default_is_grey`
- `completions_include_active_color_values`

### Verification

```bash
cargo test -p tui-chrome --lib -- choice_render terminal_style
cargo test -p tui-chrome-cli --test cli
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

### Risks / open questions

- Choosing concrete RGB values for "faint" backgrounds across
  Grey/Green/Yellow/Red on dark vs light terminals is a design
  decision. Use 256-color indices that look acceptable on both
  iTerm2 dark and Terminal.app light (e.g. dark: 238, 22, 58, 52;
  light: 252, 194, 230, 224). Confirm with a manual pass after
  Phase 5; document the chosen indices in `terminal_style.rs`.
- `boolean_switch.rs` and `input_table/table.rs` still rely on
  `theme.selected_style`. They are intentionally out of scope —
  call this out in the docs note.

---

## Phase 6 — Hotkey badge display (Ctrl/Alt orange/yellow)

**Closes review finding 4.**
Final feature phase. Depends on Phases 3 and 5 (typed hotkeys and
`ChoiceRenderContext`).

### Goal

Implement the spec's hotkey badges:

- A new state field `hotkey_display: HotkeyDisplayMode` on both
  `ChooseOneState` and `ChooseManyState` (currently absent — the
  enum exists but is never stored).
- Event handling: when a `KeyEventKind::Press`/`Release` modifier-only
  event for `Ctrl` or `Alt` is observed, transition
  `Hidden → CtrlHeld`/`AltHeld` → `Hidden`. Where pure modifier
  events are unavailable (most terminals), fall back to a brief
  display window after any Ctrl/Alt chord — implemented with a
  monotonic deadline stored alongside the mode.
- Render: `choice_render.rs` draws a badge after the label in vertical
  mode and below the row in horizontal mode. Ctrl badges are orange
  background with white text; Alt badges are yellow background with
  white text. The currently-held modifier renders bold; the other
  renders dim/light.

### Code changes

| File | Change |
| --- | --- |
| `biscuit-tui/lib/src/components/choose_one.rs` | Add `hotkey_display: HotkeyDisplayMode` and `hotkey_display_deadline: Option<std::time::Instant>` to `ChooseOneState`. Initialize as `Hidden`/`None`. In `handle_event`, intercept modifier-only key events to set the mode; on chord events, set the fallback deadline. Expose `pub fn hotkey_display(&self) -> HotkeyDisplayMode` for tests. |
| `biscuit-tui/lib/src/components/choose_many.rs` | Same fields and event handling. |
| `biscuit-tui/lib/src/components/choice_render.rs` | Both `for_single` and `for_multiple` already accept a `HotkeyDisplayMode` field on `ChoiceRenderContext` — currently always set to `Hidden`. Replace the hard-coded `Hidden` at lines 76 and 100 with the value passed in by the state. Add badge rendering helpers: `render_ctrl_badge`, `render_alt_badge`. Vertical layout reserves badge width via `unicode_width::UnicodeWidthStr` and places badges immediately after the trailing blank space. Horizontal layout adds a "below row" badge band per the spec ("in horizontal mode this text will be placed below"). |
| `biscuit-tui/lib/src/components/choice_layout.rs` | Extend `ChoiceItemRect` (or add a sibling `BadgePlacement`) so navigation logic can account for badges in horizontal mode without clipping. |
| `biscuit-tui/lib/src/components/choose_one.rs` & `choose_many.rs` | When constructing the `ChoiceRenderContext`, pass `state.current_hotkey_display(now)` (a method that resolves the deadline against `Instant::now()` and returns `Hidden` when expired). |
| `biscuit-tui/cli/src/commands/common_choose.rs` | Optional: add `--hotkey-badges <auto\|always\|never>` flag (default `auto`). `auto` uses the modifier-detection + deadline fallback; `always` forces `CtrlHeld` style; `never` forces `Hidden`. This is only added if it is a small ergonomic win — flag as optional. |

### New tests

Library tests using `drive_event_loop` and `TestBackend`:

- `hotkey_display_initially_hidden`
- `hotkey_display_transitions_to_ctrl_held_on_ctrl_modifier_press`
- `hotkey_display_returns_to_hidden_on_ctrl_modifier_release`
- `hotkey_display_briefly_visible_after_ctrl_chord_via_deadline`
- `hotkey_display_alt_path_is_symmetric`
- `vertical_render_draws_ctrl_badge_with_orange_bg_and_bold_text`
- `vertical_render_draws_alt_badge_with_yellow_bg_and_bold_text`
- `vertical_render_when_ctrl_held_renders_alt_badges_dim`
- `horizontal_render_places_badge_below_row_not_inline`
- `badge_only_appears_for_options_with_explicit_hotkey`

CLI integration (only if `--hotkey-badges` is added):

- `cli_hotkey_badges_auto_is_default`
- `cli_hotkey_badges_never_hides_badges_in_render_test`

### Verification

```bash
cargo test -p tui-chrome --lib -- choose_one choose_many choice_render
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
```

### Risks / open questions

- Crossterm's modifier-only key events (`KeyEventKind::Press` with
  `KeyCode::Modifier`) are not reliable across all terminals. The
  deadline-based fallback (≈300 ms) is the production path on
  unsupported terminals. Make the deadline duration a private
  constant initially; promote to configuration if user feedback
  asks for it.
- Horizontal "below row" badges interact with the layout cache
  invariant ("`ChoiceItemRect` is rebuilt during render"). Make sure
  `compute_layout` accounts for badge rows so navigation Up/Down does
  not jump into a badge row.
- `unicode_width` 0.2 is already a workspace dep — no new deps needed.

---

## Phase 7 — Final verification and documentation sweep

### Goal

End-to-end gate: every finding closed, every test passing, clippy clean,
docs up to date, skill catalog refreshed.

### Tasks

- Run the full focused suite: `cargo test -p tui-chrome -p
  tui-chrome-cli` and confirm zero failures.
- Run `cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets --
  -D warnings` and confirm zero warnings.
- Run `cargo doc -p tui-chrome -p tui-chrome-cli --no-deps` and skim
  for broken intra-doc links (especially after the `Padding` and
  `SortOrder` renames).
- Run the per-area `just test` and `just lint` from
  `biscuit-tui/justfile` to confirm the wrappers still work (the
  area justfile is the one source of truth for biscuit-tui tasks per
  CLAUDE.md).
- Manually exercise the CLI:
  - `question choose-one --sort inverse Alpha Beta Gamma`
    (asserts ordering Gamma, Beta, Alpha).
  - `question choose-one --file fixture.json` with a JSON object
    array containing `value` and `hotkey` keys; assert the emitted
    value is the object's `value`, not its `label`.
  - `question choose-one --active-color green Alpha Beta` and
    verify a green active background.
  - Press and hold Ctrl in a Nerd-Font terminal and visually
    confirm orange/yellow badges appear next to options whose
    hotkeys are bound to Ctrl/Alt.
- Update docs that the previous phases queued for change:
  - `biscuit-tui/docs/components/choose_one.md`
  - `biscuit-tui/docs/components/choose_many.md`
  - `biscuit-tui/docs/components/frame_chrome.md`
  - `biscuit-tui/docs/cli-reference.md`
  - `biscuit-tui/lib/README.md`
  - `biscuit-tui/cli/README.md`
  - `.claude/skills/biscuit-tui/SKILL.md` (Components, Core
    Primitives, and CLI sections).
- Mark `review-1.md` `ready: true` once all six findings are closed.
- Update `biscuit-tui/features/2026-04-28-choose-one-improvements/plan.md`
  status if it tracks per-phase progress.

### Verification

```bash
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
cargo doc -p tui-chrome -p tui-chrome-cli --no-deps
cd biscuit-tui && just test && just lint
```

### Risks / open questions

- None expected; this is a verification phase. If clippy flags
  something new under Phase 6's badge code (commonly `clippy::too_many_arguments`
  on render helpers), fix in-phase rather than deferring.

---

## Cross-Cutting Notes

### Test coverage gap → phase mapping

The review's "Test Coverage Gaps" list maps to:

| Gap | Phase that adds the test |
| --- | --- |
| Library sorting via `ChoiceInput::with_sort` on both choice components | Phase 1 |
| CLI acceptance/completion of `--sort inverse` | Phase 4 |
| File and markdown object records preserving `label`, `value`, `hotkey` | Phase 3 |
| Hotkey badge visibility, badge styling, horizontal placement | Phase 6 |
| `Padding::default()` itself, not only `FrameChromeConfig::default()` | Phase 2 |
| Active color variants and foreground contrast under dark/light | Phase 5 |

### Files most touched

- `biscuit-tui/lib/src/components/choose.rs`
- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`
- `biscuit-tui/lib/src/components/choice_render.rs`
- `biscuit-tui/lib/src/components/choice_layout.rs`
- `biscuit-tui/lib/src/core/frame.rs`
- `biscuit-tui/lib/src/core/terminal_style.rs`
- `biscuit-tui/lib/src/core/sort.rs`
- `biscuit-tui/cli/src/option_sources.rs`
- `biscuit-tui/cli/src/choice_normalize.rs`
- `biscuit-tui/cli/src/commands/common_choose.rs`
- `biscuit-tui/cli/src/commands/choose_one.rs`
- `biscuit-tui/cli/src/commands/choose_many.rs`

### Aggregate risks

1. **Renames ripple**: `SortOrder::Reverse → Inverse` (Phase 4) and
   `Padding::default()` semantics change (Phase 2) both touch many call
   sites. Schedule them as their own commits so a bisect stays useful.
2. **CLI snapshot churn**: any assert_cmd test that pins exact output
   width may need fixture regeneration after Phase 2 (padding default
   shrinks inner area) and Phase 5 (active background span change).
3. **Modifier event portability** (Phase 6): the deadline-based
   fallback is the production path. Document this clearly in the
   rustdoc on `HotkeyDisplayMode` so users know what to expect across
   terminals.
4. **Badge layout in horizontal mode** (Phase 6): "below row" badges
   interact with the layout cache; navigation logic must skip badge
   rows. Reserve a small geometry-only test fixture in `choice_layout.rs`.
