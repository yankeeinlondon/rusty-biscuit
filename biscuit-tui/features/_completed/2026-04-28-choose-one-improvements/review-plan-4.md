---
ready: false
agent: codex
source_review: review-4.md
package_area: biscuit-tui
phases: 5
crates:
  - biscuit-tui
  - biscuit-tui-cli
---

# Review-4 Remediation Plan

This plan addresses every production blocker in `review-4.md` for the
Choose One Improvements feature. It uses five phases: four focused fix phases
matching the review findings, followed by one verification and cleanup phase.

The package area is `biscuit-tui`, so use the area `justfile` where practical:

```bash
cd biscuit-tui
just test
just lint
```

Focused `cargo test -p ...` commands are listed per phase to keep failures
small while implementing. The final phase must pass the full package-area test
and lint commands before the feature is considered ready.

## Phase 1: Wire TerminalStyle Into Component Rendering

Closes review finding 1.

### Problem

`TerminalStyle::from_env()` and the renderer's Nerd Font/light-background logic
exist, but `ChooseOne` and `ChooseMany` construct `ChoiceRenderContext` with
`TerminalStyle::default()`. The real component render path therefore never uses
detected Nerd Font glyphs or light-background active styles.

### Implementation

Files:

- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`
- `biscuit-tui/lib/src/core/terminal_style.rs` only if a small test helper is
  needed

Steps:

1. Add a `terminal_style: TerminalStyle` field to `ChooseOneState` and
   `ChooseManyState`.
2. Initialize that field in each `State::new` with `TerminalStyle::from_env()`.
3. Add builder methods on both states:
   - `with_terminal_style(mut self, terminal_style: TerminalStyle) -> Self`
4. Replace both `TerminalStyle::default()` calls in each component render path
   with `state.terminal_style`, including the layout-computation context and
   the final render context.
5. Keep `TerminalStyle::from_env()` as the only env-reading point. Component
   render tests should primarily use `with_terminal_style(...)` for deterministic
   coverage; if an env-backed component test is added, guard it carefully because
   env mutation is process-global in Rust 2024 tests.

### Required Tests

Add component-level render tests, not only `ChoiceRenderContext` tests:

- `choose_one_render_uses_nerd_font_terminal_style`
  - Build a selected `ChooseOneState` with
    `TerminalStyle { nerd_font: NerdFontStatus::Likely, ..Default::default() }`.
  - Render into a `Buffer`.
  - Assert the selected/unselected symbols are the Nerd Font radio glyphs.
- `choose_many_render_uses_nerd_font_terminal_style`
  - Same coverage for checkbox glyphs.
- `choose_one_render_uses_light_background_active_foreground`
  - Inject `TerminalBackground::Light`, render the active row, and assert the
    active cells use `Color::Black` foreground.
- `choose_many_render_uses_light_background_active_foreground`
  - Same coverage for `ChooseMany`.

Focused verification:

```bash
cargo test -p biscuit-tui terminal_style
cargo test -p biscuit-tui render_uses_nerd_font
cargo test -p biscuit-tui render_uses_light_background
```

## Phase 2: Make TOML File Sources Actually Usable

Closes review finding 2.

### Problem

The CLI documents TOML support for `--file`, but `.toml` files dispatch to
`parse_toml()` and then require the parsed value itself to be an array. Normal
TOML documents are tables, so usable files such as
`options = ["Red", "Green"]` are rejected.

### Contract

Define the supported TOML file shape explicitly:

```toml
options = ["Red", "Green"]
```

and object records:

```toml
[[options]]
label = "Red"
value = "apple"
hotkey = "CTRL+R"

[[options]]
label = "Blue"
value = "sky"
disabled = true
```

Inline-table arrays under `options = [{ label = "...", value = "..." }]` should
also work because they parse to the same `toml::Value::Array`.

### Implementation

Files:

- `biscuit-tui/cli/src/option_sources.rs`
- `biscuit-tui/cli/tests/choose_cli.rs`
- Docs that mention `--file` TOML shape:
  - `biscuit-tui/docs/cli-reference.md`
  - `biscuit-tui/docs/components/choose_one.md`
  - `biscuit-tui/docs/components/choose_many.md`
  - `biscuit-tui/cli/README.md`

Steps:

1. Update `parse_toml()` so a parsed top-level `toml::Value::Table` looks up an
   `options` key and passes that value to the existing array extraction helper.
2. Preserve the existing array extraction behavior for string arrays and table
   arrays.
3. Return `SourceError::NotAnArray` when:
   - there is no top-level `options` key,
   - `options` exists but is not an array,
   - the top-level value is any unsupported shape.
4. Rename `extract_toml_string_array` to a more accurate private name such as
   `extract_toml_options_array`, or keep the name if minimizing churn is
   preferable. If renamed, update all private tests.
5. Update the current rejecting test
   `parse_file_toml_array_top_level_must_be_table` to assert success for
   `options = ["Red", "Green", "Blue"]`.
6. Add `assert_cmd` coverage in `cli/tests/choose_cli.rs` proving
   `question choose-one --file options.toml` reaches the event loop. This
   should follow the existing no-TTY pattern: exit code `1` with
   `stderr` containing `question:` means parsing succeeded and the command got
   as far as terminal startup.

### Required Tests

Unit tests in `option_sources.rs`:

- `parse_file_toml_options_string_array`
- `parse_file_toml_options_inline_table_array_preserves_fields`
- `parse_file_toml_array_of_tables_preserves_fields`
- `parse_file_toml_missing_options_key_is_not_array`
- `parse_file_toml_options_non_array_is_not_array`

Integration test in `cli/tests/choose_cli.rs`:

- `choose_one_file_toml_options_array_reaches_event_loop`

Focused verification:

```bash
cargo test -p biscuit-tui-cli option_sources::tests::parse_file_toml
cargo test -p biscuit-tui-cli --test choose_cli choose_one_file_toml
```

## Phase 3: Preserve Explicit Values Against Value Conventions

Closes review finding 3.

### Problem

`raw_option_to_parsed()` correctly splits `Red Delicious::Apple`, but
`normalize_options()` then applies `--value-convention` unconditionally. The
explicit value is transformed even though the technical design says `::` and
object-supplied fields are escape hatches from convention-derived values.

### Implementation

Files:

- `biscuit-tui/cli/src/choice_normalize.rs`

Steps:

1. Extend `ParsedOption` with provenance flags:
   - `explicit_label: bool`
   - `explicit_value: bool`
2. Set provenance in `raw_option_to_parsed()`:
   - Object `value: Some(...)` -> `explicit_value = true`.
   - `label::value` split -> both sides explicit enough to skip conventions for
     the side supplied by the split. For the reviewed example, both label and
     value should remain exactly `Red Delicious` and `Apple`.
   - Legacy single-character `--delimiter` split should follow the same
     explicit-value rule unless existing tests require otherwise.
   - Plain labels with no explicit value keep both flags false, so conventions
     still derive labels and values.
3. Change convention application:
   - Apply `label_convention` only when `!opt.explicit_label`.
   - Apply `value_convention` only when `!opt.explicit_value`.
4. Keep duplicate hotkey detection and numeric hotkey assignment behavior
   unchanged.
5. Update doc comments in `choice_normalize.rs` so the normalization order
   states that explicit sides are not convention-transformed.

### Required Tests

Unit tests in `choice_normalize.rs`:

- `normalize_options_delimited_value_skips_value_convention`
  - Input: `Red Delicious::Apple`
  - `value_convention = SnakeCase`
  - Expected value: `Apple`
- `normalize_options_delimited_label_skips_label_convention`
  - Input: `red delicious::Apple`
  - `label_convention = TitleCase`
  - Expected label remains `red delicious` if treating `::` label as explicit.
- `normalize_options_object_value_skips_value_convention`
  - Raw object with `value: Some("Apple")` and `value_convention = SnakeCase`
  - Expected value: `Apple`.
- Keep or update `normalize_options_with_conventions` to prove conventions still
  apply to plain string options.

Focused verification:

```bash
cargo test -p biscuit-tui-cli choice_normalize::tests::normalize_options
```

## Phase 4: Fix Horizontal Badge Visible-Row Accounting

Closes review finding 4.

### Problem

When horizontal hotkey badges are visible, each logical option row consumes two
terminal rows. `ChooseOne` and `ChooseMany` pass raw terminal row count into
`adjust_scroll()`, and `ChoiceRenderContext::render()` also passes raw row count
to `render_horizontal()`. A short viewport can try to render logical rows beyond
the buffer height.

### Implementation

Files:

- `biscuit-tui/lib/src/components/choice_render.rs`
- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`

Steps:

1. Add a small helper in `choice_render.rs` so layout, scroll, and render use
   the same math:
   - `row_height()` returns `1` when badges are hidden, otherwise `2`.
   - `visible_logical_rows(body_rows)` returns `body_rows` for vertical mode and
     `body_rows / row_height` for horizontal mode, clamped to `1` when
     `body_rows > 0`.
2. Use that helper inside `ChoiceRenderContext::render()` before dispatching to
   `render_horizontal()`.
3. Use the same helper in `ChooseOne::render()` and `ChooseMany::render()` when
   passing `visible` to `adjust_scroll()`.
4. Keep the existing `screen_y + 1 < area.y + area.height` guard for badge
   drawing. Do not draw partial badge rows outside the render area.
5. Check overflow indicators after the fix. If the bottom overflow marker is
   still based on logical row count rather than terminal row height, adjust it
   to draw on the last occupied option row without overwriting badge text.

### Required Tests

Add render-buffer tests at the component level, because the review calls out
the `ChooseOne` and `ChooseMany` render paths:

- `choose_one_horizontal_badges_short_viewport_does_not_draw_past_area`
  - Horizontal input, badges forced visible, area height `3`, narrow width that
    creates at least three logical rows.
  - Assert only the first logical row plus its badge appears; later option labels
    are absent from rows that would require y `4` or beyond.
  - Assert `state.scroll_offset` remains valid.
- `choose_many_horizontal_badges_short_viewport_does_not_draw_past_area`
  - Same coverage for `ChooseMany`.

Keep the existing renderer-level test
`horizontal_multi_row_badges_do_not_overwrite_next_row_options`, and add one
renderer-level short-viewport test if it makes the helper easier to pin:

- `horizontal_badges_visible_count_uses_logical_rows`

Focused verification:

```bash
cargo test -p biscuit-tui horizontal_badges
cargo test -p biscuit-tui short_viewport
```

## Phase 5: Docs, Full Tests, and Lint

Closes the review's production-readiness concern by proving the package area is
green after all fixes.

### Documentation

Update docs only where the public contract changes or becomes more explicit:

- TOML `--file` docs must say that TOML uses a top-level `options` array.
- If `with_terminal_style` is public, add concise rustdoc on both state builders.
- Do not rewrite unrelated feature docs.

Run a quick docs scan:

```bash
rg -n "TOML|--file|options =" biscuit-tui/docs biscuit-tui/cli/README.md
```

### Full Verification

Run focused tests first while implementing, then the complete package-area
verification:

```bash
cargo test -p biscuit-tui terminal_style
cargo test -p biscuit-tui horizontal_badges
cargo test -p biscuit-tui-cli choice_normalize::tests::normalize_options
cargo test -p biscuit-tui-cli option_sources::tests::parse_file_toml
cargo test -p biscuit-tui-cli --test choose_cli choose_one_file_toml

cd biscuit-tui
just test
just lint
```

If `just lint` does not treat warnings as hard failures, also run:

```bash
cargo clippy -p biscuit-tui -p biscuit-tui-cli --all-targets -- -D warnings
```

The final state must have:

- `cargo test -p biscuit-tui -p biscuit-tui-cli` passing.
- `just test` passing from `biscuit-tui/`.
- `just lint` passing from `biscuit-tui/`.
- No clippy warnings under `-D warnings`.
- Review-specific focused tests present and passing.

## Risk Notes

- Env-backed tests are the main flake risk because Rust 2024 makes environment
  mutation explicitly unsafe. Prefer injecting `TerminalStyle` into state for
  component render tests and leave env parsing tests in `core::terminal_style`.
- TOML support should not loosen JSON/YAML/CSV behavior. Keep the new TOML
  extraction local to `.toml` dispatch.
- The provenance flags in normalization should be private implementation detail;
  they should not leak into the `ChoiceOption` library type.
- Horizontal visible-row math must be shared between scroll adjustment and
  rendering. Duplicating the formula risks reintroducing the same short-viewport
  mismatch later.

## Phase Summary

| Phase | Finding | Scope |
| --- | --- | --- |
| 1 | Terminal detection unused | Library state/render plumbing and component render tests |
| 2 | TOML `--file` rejected | CLI source parsing, docs, CLI smoke test |
| 3 | Explicit values transformed | CLI normalization provenance and unit tests |
| 4 | Horizontal badge viewport overrun | Shared render math and component buffer tests |
| 5 | Production readiness | Docs scan, full tests, clippy/lint |
