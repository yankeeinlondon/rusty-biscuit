---
review_plan: 1
created: 2026-04-23
targets_review: review-1.md
packages:
  - biscuit-terminal
  - darkmatter
phases: 7
---

# Review-1 Remediation Plan — Horizontal Rule

This plan addresses **every** item called out in `review-1.md`. It is organized into seven phases ordered by dependency — each phase is independently verifiable (`cargo test -p <pkg>` + `cargo clippy -p <pkg> -- -D warnings`) and safe to hand to a `rust-developer` subagent one at a time.

## End-state goal (goal-backward)

The HR feature must:

1. Respect `color` in the terminal (A2), `weight` in the terminal (A3), thread a real `Terminal` through the renderer (B2), honor component `Layout` margins (B3), handle bare `Event::Rule` explicitly (B4), and detect Unicode using an actual Unicode signal (B6).
2. Emit CSS variables in browser SVGs so `render_to_browser_with_inline_variables` has a real override surface (A4).
3. Validate attribute strings and surface warnings when values are unknown (B1).
4. Use a YAML flow-mapping parser so quoted commas/colons work (E2).
5. Ship compiling docs with accurate style/weight tables (D1, D2, D3), re-export the public API through `biscuit_terminal::prelude` (D5), and drop the unused import (D6).
6. Have **all** A1 follow-up framing explicit: Tier 1 (SVG→PNG) is explicitly deferred and documented as such (A1 — the review accepts this as "nice to have").
7. Have tests that would have caught A2, A3, B1, B4, and C1–C4 gaps.
8. Have `cargo test -p biscuit-terminal`, `cargo test -p darkmatter`, `cargo clippy -p biscuit-terminal -p darkmatter -- -D warnings` all green.

The review places A1 (Tier 1 image rendering) under "Nice to have" and allows the tech-design to be amended to defer it. This plan chooses the **defer + document** path — we add a note to `tech-design.md` and to the component doc, and we do **not** implement resvg → PNG here. Every other item in the review is fixed.

---

## Phase 1 — biscuit-terminal component fixes (cosmetic + capability)

**Scope:** code-quality issues in `biscuit-terminal/lib/src/components/horizontal_rule.rs` that do not change public API or rendering semantics. Sets the stage for the behavioral fixes in Phases 2–3.

### 1.1 Run `cargo fmt` and fix brace structure (B5)

- File: `biscuit-terminal/lib/src/components/horizontal_rule.rs`
- Run `cargo fmt -p biscuit-terminal`. Also manually inspect the `resolve_width` function (lines ~183-195) — the review flags broken brace indentation. After fmt, the `None => { match self.alignment { ... } }` block should nest cleanly.
- Confirm `resolve_width` semantics are unchanged (Full → `term_width`, Centered/Left/Right → 80%).

### 1.2 Simplify `InsetLine`/`CurtainRod` halved repeats (B7)

- Same file, inside `generate_terminal_content`.
- Replace:
  ```rust
  format!("  {}{}  ", line.repeat(inner_width / 2), line.repeat(inner_width - inner_width / 2))
  ```
  with:
  ```rust
  format!("  {}  ", line.repeat(inner_width))
  ```
- Do the same for `CurtainRod`'s body (the `format!("{}{}{}{}", ...)` call).

### 1.3 Replace CJK corner brackets in CurtainRod (B8)

- Same file, `RuleStyle::CurtainRod` Unicode arm.
- Replace `「` / `」` (East-Asian wide) with single-width alternatives — use `┤` (U+2524) for left, `├` (U+251C) for right (both are single-width in monospace). Or, equivalently, `⎡` / `⎤` — pick `┤` / `├` since they harmonize visually with `─`.
- Update the `test_render_curtain_rod_unicode` assertion to check for `┤` / `├` instead of `「` / `」`.

### 1.4 Fix `supports_unicode` heuristic (B6)

- Same file. Rename the helper to `use_fancy_chars` and base it on `biscuit_terminal::discovery::locale::env_says_utf8()`:
  ```rust
  fn use_fancy_chars(&self, _term: &Terminal) -> bool {
      crate::discovery::locale::env_says_utf8().unwrap_or(true)
  }
  ```
- The fallback to `true` preserves current behavior for environments that don't set `LC_*`/`LANG`.
- Add a `//` comment summarizing: "We treat missing locale as UTF-8-capable because every modern terminal defaults to UTF-8. Explicit C/POSIX locales fall through to ASCII."
- Update all call sites inside `generate_terminal_content` to use the new name.

### 1.5 Fix `RuleWeight` doc-comment pixel values (D3)

- Same file, lines 41–48.
- Change:
  - `/// Thin line (1px equivalent)` → `/// Thin line (2px stroke in browser, single-line chars in terminal)`
  - `/// Medium line (2px equivalent)` → `/// Medium line (4px stroke in browser, single-line chars in terminal)`
  - `/// Thick line (3px equivalent)` → `/// Thick line (8px stroke in browser, heavy/double chars in terminal)`

### 1.6 Test coverage

- Existing tests in the file exercise structural invariants; after 1.3 update the curtain-rod assertion. All 36 existing snapshots remain valid until Phase 2; **do not** delete snapshots here. `cargo insta review` after Phases 2 and 3.
- Add one new unit test `test_use_fancy_chars_respects_locale` that builds a `Terminal::default()` and asserts `use_fancy_chars` returns `true` when `LANG=en_US.UTF-8` is in the environment (set the env var inside the test via `std::env::set_var`, then remove it at the end). Gate the env manipulation with a serial-test helper if multiple env tests run in parallel — otherwise document the race in a comment.

### Validation

- `cargo fmt -p biscuit-terminal -- --check`
- `cargo test -p biscuit-terminal --lib components::horizontal_rule`
- `cargo clippy -p biscuit-terminal -- -D warnings`

---

## Phase 2 — Terminal `color` + `weight` semantics (A2, A3)

**Scope:** make the HR actually honor its `color` and `weight` attributes in the terminal. This is one of the two main blockers in the review.

### 2.1 Implement `weight`-aware character selection (A3)

- File: `biscuit-terminal/lib/src/components/horizontal_rule.rs`, inside `generate_terminal_content`.
- Introduce a helper `fn heavy(&self) -> bool { matches!(self.weight, RuleWeight::Thick) }`.
- Adjust each Unicode arm:
  - `Dashes`: `╌` (default) vs `╍` (heavy)
  - `Dots`: `·` (default) vs `•` (heavy)
  - `Waves`: keep `≋` for all weights — add a rustdoc note on `RuleStyle::Waves` stating "Waves has no heavy Unicode variant; weight affects only browser rendering."
  - `LineStar`, `LineCircle`, `InsetLine`, `CurtainRod`: swap `─` for `━` when `heavy()`. The curtain-rod bracket characters and the symbol character stay the same.
- ASCII arms are unchanged — ASCII has no heavy variant, document that too.

### 2.2 Implement terminal `color` wrapping (A2)

- Same file, inside `render()` (the `Renderable` impl, lines ~112-141).
- After computing the placed content but before returning, wrap with ANSI escapes **only if** both `self.color.is_some()` and `term.color_depth != ColorDepth::None`.
- Use the existing `biscuit_terminal::utils::color` machinery:
  ```rust
  use crate::utils::color::{BasicColor, RgbColor, TermColor};
  ```
- Parse `self.color`:
  - Named colors (`"red"`, `"green"`, ...): map to `BasicColor` via a small match (cover the CSS basic-16 set: `black, red, green, yellow, blue, magenta, cyan, white` + bright variants; also accept `"gray"`/`"grey"` → `BrightBlack`).
  - Hex (`#rrggbb`): parse to `RgbColor` when `term.color_depth == ColorDepth::TrueColor`. Fall back to nearest `BasicColor` otherwise.
  - Unrecognized: skip wrapping, `tracing::warn!` with the raw string.
- The wrapper goes on the **full** placed string (including centering padding) so the padding is uncolored — use `BasicColor::fg` / `RgbColor::fg` which already return `"\x1b[..m{content}\x1b[39m"`.
- Placement padding should remain outside the color wrap: wrap the content first, then prepend padding spaces.

### 2.3 Decide on tracing dependency

- Check `biscuit-terminal/lib/Cargo.toml` for `tracing`. If absent, add `tracing = { workspace = true }` (workspace pattern used elsewhere in the monorepo — verify with `grep -r 'tracing = ' biscuit-terminal/`). Otherwise no change.

### 2.4 Test coverage

- Add unit tests in the existing `tests` module:
  - `test_render_thick_dashes_is_heavy_unicode`: thick dashes contain `╍` not `╌`.
  - `test_render_thick_dots_is_heavy_unicode`: thick dots contain `•` not `·`.
  - `test_render_thick_line_star_uses_heavy_line`: thick line-star contains `━`.
  - `test_render_thick_waves_same_as_medium_waves_with_note`: thick/medium waves produce the same body (documents the known limitation).
  - `test_render_color_named_wraps_with_ansi`: `color("red")` on a `ColorDepth::Ansi256`/`TrueColor` terminal produces `\x1b[31m...\x1b[39m` in the result.
  - `test_render_color_hex_truecolor_wraps_with_rgb`: `color("#ff0000")` on `TrueColor` produces `\x1b[38;2;255;0;0m`.
  - `test_render_color_no_effect_when_depth_none`: `color("red")` on `ColorDepth::None` produces no escape codes.
  - `test_render_color_invalid_logs_warning_no_wrap`: `color("not-a-color")` yields no ANSI codes. (Asserting the `tracing::warn!` is optional — verify via `logs_contain!` if `tracing-test` is already wired; otherwise just assert no wrapping.)
- Regenerate snapshots: `cargo insta test -p biscuit-terminal` then `cargo insta review` — the thick `{dashes, dots, line_star, line_circle, inset_line, curtain_rod}` snapshots will legitimately change and the custom-attributes snapshot (`terminal_custom_attributes`) will now include ANSI wrapping. Accept those deltas.

### Validation

- `cargo test -p biscuit-terminal --lib components::horizontal_rule`
- `cargo insta review` → accept the legitimate weight/color deltas.
- `cargo clippy -p biscuit-terminal -- -D warnings`

---

## Phase 3 — Browser CSS-variable strategy (A4)

**Scope:** make `render_to_browser` emit a `<style>` block / `var(--...)` expressions so that `render_to_browser_with_inline_variables` has a natural override surface. This is a Category A design gap.

### 3.1 Emit CSS variables in default SVG output

- File: `biscuit-terminal/lib/src/components/horizontal_rule.rs`, `render_to_browser` method.
- Current: hard-codes `stroke-width="4"`, `stroke="blue"`, etc.
- New: emit a small inline `style` attribute on the `<svg>` root that declares:
  ```
  --hr-weight: {current_weight_px};
  --hr-color: {current_color_or_currentColor};
  --hr-width: {current_width_or_100%};
  ```
  Then inside shape primitives use `var(--hr-weight, {fallback})`, `var(--hr-color, currentColor)`, etc. The fallback inside `var()` lets the SVG render correctly even if the inline style is stripped.
- Example:
  ```rust
  format!(
      r#"<svg width="100%" height="40" xmlns="http://www.w3.org/2000/svg" style="display: block; margin: {top} auto {bot} auto; --hr-weight: {weight}; --hr-color: {color}; --hr-width: {width};">
    <line x1="0" y1="50%" x2="100%" y2="50%" stroke="var(--hr-color, currentColor)" stroke-width="var(--hr-weight, 4)" stroke-linecap="round" stroke-dasharray="8,4"/>
  </svg>"#,
      ...)
  ```
- The outer `width` attribute on `<svg>` remains a concrete value (some renderers do not honor `var()` in geometry attributes); we only use `var()` inside style properties.
- Do this for **all seven** styles. Keep numeric positioning (`x1`, `x2`, `cx`, `cy`, etc.) concrete.

### 3.2 Simplify `render_to_browser_with_inline_variables`

- Same file.
- Current: string-replaces `var(--name)` tokens. That's fine to keep, but now `render_to_browser()` actually contains `var(--hr-weight)` etc., so the realignment has real targets.
- Keep the method's signature. Behavior after 3.1 is automatically correct — no code change required beyond updating doc comments to reflect "this now overrides `--hr-weight`, `--hr-color`, `--hr-width` as declared by the default output." Variables passed in override via `var(--name)` string substitution.
- Edge case: `HashMap` key ordering is nondeterministic; since each `--hr-*` variable is substituted independently, order does not matter. Assert this with a test.

### 3.3 Test coverage

- Add:
  - `test_render_to_browser_contains_css_variables`: default output contains `--hr-weight:`, `--hr-color:`, `--hr-width:` and `var(--hr-weight,` / `var(--hr-color,`.
  - `test_render_to_browser_with_inline_variables_overrides_weight`: caller passes `{"hr-weight" => "12"}`, output contains `12` where `var(--hr-weight)` used to be.
  - `test_render_to_browser_with_inline_variables_overrides_color`: similar for `hr-color`.
  - `test_render_to_browser_fallbacks_work`: assert that `var(--hr-weight, 4)` is present — i.e., the fallback is embedded.
- Regenerate `test_snapshot_render_to_browser_all_styles` snapshots (`cargo insta review`) to accept the new SVG shape. **All 21 browser snapshots** (7 styles × 3 weights) will change.

### Validation

- `cargo test -p biscuit-terminal --lib components::horizontal_rule`
- `cargo insta review`
- `cargo clippy -p biscuit-terminal -- -D warnings`

---

## Phase 4 — prelude re-exports + doc fixes (D1, D2, D5, D6)

**Scope:** public API surface polish. Can be done safely any time after Phase 3 (because D2's Unicode table must reflect post-1.3 curtain-rod brackets).

### 4.1 Re-export HR + BrowserRenderable through prelude (D5)

- File: `biscuit-terminal/lib/src/prelude.rs`.
- Add:
  ```rust
  pub use crate::components::horizontal_rule::{HorizontalRule, RuleAlignment, RuleStyle, RuleWeight};
  pub use crate::components::renderable::BrowserRenderable;
  ```
- Place alphabetically with sibling exports (`block_quote`, `compose`, ...).

### 4.2 Fix non-compiling code examples (D1)

- File: `biscuit-terminal/docs/components/horizontal-rule.md`.
- Replace the struct-literal example on lines 87–93 with the builder form:
  ```rust
  use biscuit_terminal::prelude::*;

  let rule = HorizontalRule::new()
      .style(RuleStyle::Waves)
      .alignment(RuleAlignment::Centered)
      .weight(RuleWeight::Medium)
      .width("75%");
  ```
- Fix line 96: `rule.render(&mut terminal)?;` → `let _ = rule.render(&terminal);`
- Remove the `?` and `mut` throughout the doc.

### 4.3 Fix Unicode fallback table (D2)

- Same file, lines 102–111. Correct rows to match source truth **after Phase 1.3** (curtain-rod brackets become `┤` / `├`):
  ```
  | Style       | SVG            | Unicode           | ASCII       |
  |-------------|----------------|-------------------|-------------|
  | Dashes      | dashed line    | `╌` / `╍` (thick) | `-`         |
  | Dots        | dotted line    | `·` / `•` (thick) | `.`         |
  | Waves       | wavy path      | `≋`               | `~`         |
  | LineStar    | line + star    | `─★─` / `━★━`     | `---*---`   |
  | LineCircle  | line + circle  | `─●─` / `━●━`     | `---o---`   |
  | InsetLine   | centered line  | `  ─  ` / `  ━  ` | `  -  `     |
  | CurtainRod  | line + ends    | `┤─┤─...─├`       | `[---]`     |
  ```
- Ensure rendered `|` escapes correctly in markdown tables.

### 4.4 Remove unused import (D6)

- File: `darkmatter/lib/tests/horizontal_rule_integration.rs`, line 4.
- Delete `use biscuit_terminal::terminal::Terminal;`.
- Run the file to confirm no other test needs it; it's a leftover.

### 4.5 Test coverage

- Add an `examples` block (a `#[test] fn prelude_exports_compile()`) in `biscuit-terminal/lib/tests/prelude.rs` (or the existing integration test file for prelude — find with `grep -l 'biscuit_terminal::prelude' biscuit-terminal/lib/tests/`). Test body:
  ```rust
  use biscuit_terminal::prelude::*;
  let _ = HorizontalRule::new().style(RuleStyle::Dashes).alignment(RuleAlignment::Full).weight(RuleWeight::Medium);
  fn takes_browser_renderable<T: BrowserRenderable>(_: T) {}
  takes_browser_renderable(HorizontalRule::new());
  ```
  If no existing prelude test file is present, create `biscuit-terminal/lib/tests/prelude_exports.rs` with this single test.
- Enable `#![doc(test(attr(deny(warnings))))]` is *not* required; instead, we rely on `cargo test --doc -p biscuit-terminal` to compile the docs. Ensure the `horizontal-rule.md` code example in 4.2 is fenced as `rust` and rely on rustdoc's doctest runner; since it's in `/docs/` not `///`, it's not automatically picked up — add a shadow doctest on `HorizontalRule::new()` that mirrors the doc example:
  ```rust
  /// ## Examples
  ///
  /// ```
  /// use biscuit_terminal::prelude::*;
  ///
  /// let rule = HorizontalRule::new()
  ///     .style(RuleStyle::Waves)
  ///     .alignment(RuleAlignment::Centered)
  ///     .weight(RuleWeight::Medium)
  ///     .width("75%");
  ///
  /// let term = Terminal::default();
  /// let _ = rule.render(&term);
  /// ```
  pub fn new() -> Self { ... }
  ```
  This guarantees the example compiles.

### Validation

- `cargo test -p biscuit-terminal`
- `cargo test --doc -p biscuit-terminal` (exercises the new shadow doctest)
- `cargo clippy -p biscuit-terminal -- -D warnings`
- `cargo test -p darkmatter --test horizontal_rule_integration` (confirms D6 fix did not break the suite)

---

## Phase 5 — darkmatter integration: thread Terminal, honor layout, handle `Event::Rule`, validate attrs (B1, B2, B3, B4)

**Scope:** all four Category-B behavioral fixes on the darkmatter side. This is the second blocker phase.

### 5.1 Thread the outer `Terminal` into HR rendering (B2)

- File: `darkmatter/lib/src/markdown/output/terminal.rs`, around line 934.
- Current: `rule.render(&Terminal::new())` — re-detects capabilities and ignores `TerminalOptions`.
- Build a `Terminal` **once** at the top of `write_terminal` (right after `color_depth` is resolved, ~line 770). Options-overridden fields (color_depth, max_width) must flow through:
  ```rust
  let term = Terminal::builder()
      .color_depth(color_depth)
      .width(terminal_width as u32)
      .build();
  ```
  (Check `Terminal::builder()` API — if `.width` takes `u16` or `usize`, convert appropriately. See `biscuit-terminal/lib/src/terminal.rs` for signatures.)
- Pass `&term` into `rule.render(&term)` at line 934.
- Do the same for any other `Terminal::new()` call inside this function (grep shows there is one at line 404 — evaluate whether it should be replaced too. If it's in a different scope, leave it; scope of this fix is the HR renderer only).

### 5.2 Honor HR `Layout` margins instead of hardcoded `\n\n` (B3)

- Same file, around line 935.
- Replace:
  ```rust
  wrapper.push_with_newlines(&rule_output);
  wrapper.push_with_newlines("\n\n"); // Add spacing after rule
  ```
  with:
  ```rust
  wrapper.push_with_newlines(&rule.display(&term));
  ```
- `Renderable::display` already guarantees a single trailing newline. The hardcoded double-newline spacing becomes configurable through the rule's `Layout::bottom_margin`. Default `Layout` has zero margins, which means default rendering will be single-newline-terminated — markdown rendering conventionally wants a blank line before/after rules, so set an explicit default:
  ```rust
  let rule = rule.bottom_margin(Margin::Chars(1)); // keeps a blank line after the rule
  ```
  But `bottom_margin` is a layout concept, not rendered text. The component's `render()` does not itself emit margin newlines — verify this by reading `HorizontalRule::render`. If margins are not currently emitted, we have two choices:
  - **(a)** Emit them in the HR's `render()` itself (apply `layout.top_margin`/`bottom_margin` as newlines when the margin is `Margin::Chars(n)`).
  - **(b)** Keep the current `push_with_newlines("\n")` behavior at the darkmatter layer but gate it on whether the rule's `layout().bottom_margin == Margin::None`.
- **Choose (b)** — least invasive and matches how other markdown block elements are separated in this file. Concretely:
  ```rust
  wrapper.push_with_newlines(&rule.render(&term));
  // Preserve a blank line between this rule and the following block.
  wrapper.push_with_newlines("\n");
  ```
  This drops the **double** newline (removing the hardcoded extra spacing the review called out) while still giving a visible separation. Document the choice in a `//` comment so a future reviewer can find it.
- Pass the original spec/review inspection through: the review's concrete ask was "hardcoded newlines are wrong / should respect layout" — (b) replaces the double with a single and documents why. If end-of-document is a concern, `LineWrapper` should already coalesce trailing whitespace.

### 5.3 Handle bare `Event::Rule` (B4)

- File: `darkmatter/lib/src/markdown/output/terminal.rs`.
- Add an explicit arm before the catch-all:
  ```rust
  InlineEvent::Standard(Event::Rule) => {
      let rule = HorizontalRule::new();
      wrapper.push_with_newlines(&rule.render(&term));
      wrapper.push_with_newlines("\n");
  }
  ```
- File: `darkmatter/lib/src/markdown/output/html.rs`, around line 195 (after the HorizontalRule match arm).
- Add the equivalent:
  ```rust
  InlineEvent::Standard(Event::Rule) => {
      let rule = HorizontalRule::new();
      output.push_str(&rule.render_to_browser());
      output.push('\n');
  }
  ```

### 5.4 Validate HR attribute values (B1)

- File: `darkmatter/lib/src/markdown/block/rule_processor.rs`, inside `parse_attributes`.
- For each of `style`, `alignment`, `weight`: after the string is captured, validate against the allowed set. On mismatch, emit `tracing::warn!(attribute = "style", value = %clean_value, "unknown horizontal rule attribute value")` and **still** store the raw value (so the renderer's downstream match falls through to default — which is the current behavior; this preserves backward compatibility while making the failure visible).
- Allowed sets (factored out as constants near the top of the module):
  ```rust
  const ALLOWED_STYLES: &[&str] = &["dashes","dots","waves","line-star","line-circle","inset-line","curtain-rod"];
  const ALLOWED_PLACEMENTS: &[&str] = &["full","centered","left","right"];
  const ALLOWED_WEIGHTS: &[&str] = &["thin","medium","thick"];
  ```
- For the unknown-key path (e.g., `margin: 4`): today the `_ => {}` arm silently drops the pair. Add a warn-log in that arm too: `tracing::warn!(key = %key, "unknown horizontal rule attribute")`.
- No schema change to `HorizontalRuleAttrs`.

### 5.5 Remove duplicate attribute-mapping blocks (refactor hook)

- Both `terminal.rs:~890-935` and `html.rs:~152-195` repeat the same attribute→builder mapping. Extract a shared helper in `darkmatter/lib/src/markdown/block/mod.rs` (or a new file `hr_builder.rs`):
  ```rust
  pub(crate) fn build_rule(attrs: &HorizontalRuleAttrs) -> biscuit_terminal::prelude::HorizontalRule {
      let mut rule = HorizontalRule::new();
      if let Some(s) = &attrs.style { rule = map_style(rule, s); }
      if let Some(p) = &attrs.alignment { rule = map_alignment(rule, p); }
      if let Some(w) = &attrs.weight { rule = map_weight(rule, w); }
      if let Some(w) = &attrs.width { rule = rule.width(w.clone()); }
      if let Some(c) = &attrs.color { rule = rule.color(c.clone()); }
      rule
  }
  ```
- Terminal + HTML renderers both call `build_rule(attrs)`. This reduces drift, and also means B1 can log unknown enum values from *one* place. Log sites live inside `map_style`/`map_alignment`/`map_weight` (each `_ =>` arm).

### 5.6 Test coverage

- New tests in `darkmatter/lib/src/markdown/block/rule_processor.rs` `#[cfg(test)] mod tests`:
  - `test_parse_attributes_unknown_style_logs_warning`: use `tracing-test`'s `traced_test` or assert via a custom subscriber — if neither is already wired, assert only that the parsed `attrs.style` equals the raw unknown value (so the contract is "store + warn", not "drop").
  - `test_parse_attributes_unknown_alignment_warns`: similar.
  - `test_parse_attributes_unknown_weight_warns`: similar.
  - `test_parse_attributes_unknown_key_is_ignored_with_warning`: feed `--- { margin: 4 }`, assert the result has all fields `None`.
  - `test_horizontal_rule_in_list_item_not_transformed`: `- --- { style: dots }` — must produce list events, not a HR.
  - `test_horizontal_rule_in_blockquote_not_transformed`: `> --- { style: waves }` — must produce blockquote events, not a HR. (If current impl does transform it, the test documents the gap; pin behavior explicitly and update the skill.)
  - `test_horizontal_rule_in_fenced_code_block_passthrough`: `` ```\n--- { style: waves }\n``` `` — HR-like content inside a code block must **not** be transformed (asserts events contain `Event::Text("--- { style: waves }\n")` inside code block start/end, not a `HorizontalRule` event).
  - `test_mixed_markers_rejected`: `-*-` and `-_-` — not transformed.
- New tests in `darkmatter/lib/tests/horizontal_rule_integration.rs`:
  - `test_bare_rule_terminal_output`: `---\n` produces a default dashed rule in terminal output.
  - `test_bare_rule_html_output`: `---\n` produces a default SVG in HTML output.
  - `test_custom_color_produces_ansi_in_terminal`: `--- { color: red }` output contains `\x1b[31m` when terminal color_depth is non-None.
  - `test_custom_weight_thick_differs_from_thin`: compare outputs for `--- { weight: thick }` vs `--- { weight: thin }` and assert they are not byte-equal.
  - `test_terminal_options_width_flows_through_to_rule`: build `TerminalOptions { max_width: Some(40), ... }`, render, assert the rule's visible width is ≤ 40 columns.
  - `test_invalid_style_falls_back_to_default`: `--- { style: bogus }` renders successfully (no panic) and contains the default dashes characters.

### Validation

- `cargo test -p darkmatter`
- `cargo clippy -p darkmatter -- -D warnings`

---

## Phase 6 — parser robustness + remaining test gaps (E2, C1–C4)

**Scope:** replace the hand-rolled comma splitter with a YAML flow-mapping parser (review's preferred option in E2) and fill in the remaining test gaps that Phase 5 didn't cover.

### 6.1 Replace `parse_attributes` with YAML flow-mapping parsing (E2)

- File: `darkmatter/lib/src/markdown/block/rule_processor.rs`.
- Rewrite `parse_attributes`:
  1. Wrap the captured attribute string in `{` ... `}` to form a YAML flow mapping (the braces are already present in the markdown source, so the string passed to `parse_attributes` is the **inner** part — wrap it back).
  2. Deserialize with `serde_yaml_ng` (already in the workspace per `MEMORY.md`: "`serde_yaml` (deprecated) still used in several crates - should migrate to `serde_yaml_ng`"). Check `darkmatter/lib/Cargo.toml`; if absent, add `serde_yaml_ng = { workspace = true }`.
  3. Deserialize into `HashMap<String, serde_yaml_ng::Value>`; for each key, coerce to `String` if scalar, otherwise warn and drop.
  4. On any YAML parse error, fall back to the current ad-hoc splitter (behind a `tracing::warn!`) so malformed-but-previously-accepted inputs continue to work.
- Keep `HorizontalRuleAttrs` as `Option<String>` fields.
- Keep the existing validation from 5.4 — it now runs on the YAML-produced values.

### 6.2 Test coverage (fills C1, C3)

- Add to `rule_processor.rs` tests:
  - `test_parse_attributes_quoted_color_with_comma`: `--- { color: "rgb(255, 0, 0)" }` — `attrs.color == Some("rgb(255, 0, 0)")`.
  - `test_parse_attributes_quoted_value_with_colon`: `--- { prefix: "a:b" }` (unknown key but must parse, then drop with a warn).
  - `test_parse_attributes_malformed_yaml_falls_back_gracefully`: feed `{ style: }` — does not panic, returns default attrs, emits a warn.
- Add to `horizontal_rule_integration.rs`:
  - `test_html_multiple_hrs_each_emits_own_svg`: input with 3 HRs, assert `result.matches("<svg").count() == 3`.
  - `test_html_invalid_attribute_values_render_defaults`: `--- { style: bogus, weight: zzz }` renders a valid default SVG.
  - `test_html_render_to_browser_with_inline_variables_via_pipeline`: **This requires threading `render_to_browser_with_inline_variables` through the HTML renderer**. The current `html.rs` only calls `render_to_browser()`. Add a new entry point `as_html_with_hr_variables(md, options, HashMap<String, String>)` (or an optional field on the HTML options struct) that, when HR events are emitted, calls the inline-variables variant. Minimal implementation: add a `hr_css_variables: Option<HashMap<String, String>>` to whatever HTML-options struct `as_html` uses; if `Some`, call `rule.render_to_browser_with_inline_variables(&vars)`. Test passes a non-empty map and asserts the override shows up in the output.

### 6.3 Test coverage (fills C2 extras that survive Phase 5)

- Phase 5.6 already covers color-produces-ANSI, thick-differs-from-thin, and invalid-style-falls-back. Re-verify those tests still pass after 6.1's parser rewrite.

### 6.4 Test coverage (fills C4 — component)

- Add in `biscuit-terminal/lib/src/components/horizontal_rule.rs` `#[cfg(test)]`:
  - `test_snapshot_render_ascii_all_styles`: mirror `test_snapshot_render_all_styles` but build the terminal with `ColorDepth::None`. Produces Tier-3 snapshots.
  - `test_horizontal_rule_inside_compose`: build a `Compose` with `HorizontalRule::new()` as one child and a `Prose` as another; `compose.render(&term)` must include both and no panic. (If `Compose` is not trivially constructable, skip with a doc comment referencing where the integration happens.)

### Validation

- `cargo test -p biscuit-terminal`
- `cargo test -p darkmatter`
- `cargo insta review` for new ASCII snapshots.
- `cargo clippy -p biscuit-terminal -p darkmatter -- -D warnings`

---

## Phase 7 — documentation, tech-design amendment, skill updates (A1 deferral, D4, E1, E3, E4)

**Scope:** finish the documentation pass and close out the remaining review items.

### 7.1 Defer Tier 1 explicitly in tech-design (A1)

- File: `darkmatter/features/2026-04-18-hr/tech-design.md`.
- Amend §1.2 "Terminal Rendering (Progressive Enhancement)" to reclassify Tier 1:
  > **Tier 1 (Image, deferred):** Rendering the style's SVG to a PNG via `resvg` + `TerminalImage` is planned but not implemented in the initial release. The component currently uses Tier 2 / Tier 3. Tier 1 can be added incrementally (see `biscuit-terminal/lib/src/components/mermaid.rs` for the working `resvg → tiny_skia → TerminalImage` pattern).
- Also update the Review-1 recommended merge path reference in the tech-design's new "Deferred Work" subsection.

### 7.2 Add missing `spec.md` note (D4)

- File: `darkmatter/features/2026-04-18-hr/spec.md` — **exists already** per the glob at the top of this plan (it was referenced in the review but the review claims only `plan.md`/`tech-design.md` are present). Verify with `ls darkmatter/features/2026-04-18-hr/`.
- If `spec.md` is present, no action.
- If absent, create it with a single paragraph that references `tech-design.md` as the authoritative document:
  > See [`tech-design.md`](./tech-design.md). The spec lives there by convention for this feature.

### 7.3 Update component docs to reflect new behavior (D2, D3 tie-in, A4)

- File: `biscuit-terminal/docs/components/horizontal-rule.md`.
- Add a new "## Weight" subsection after the Style Matrix that documents:
  - Unicode thick mappings: `╌`→`╍`, `·`→`•`, `─`→`━`.
  - Waves has no heavy variant (call out as a limitation).
  - Browser: `thin=2px`, `medium=4px`, `thick=8px`.
- Add a "## CSS Variables" subsection documenting `--hr-weight`, `--hr-color`, `--hr-width` and showing a `render_to_browser_with_inline_variables` example that overrides `--hr-weight` to `12`.
- Add a "## Color" subsection showing that terminal color wraps the rule content with ANSI escapes and listing supported color string forms (named CSS colors + `#rrggbb` on truecolor terminals).
- Add a "## Deferred" subsection noting Tier 1 is not yet implemented (mirrors 7.1).
- Verify by `grep -c '```rust' biscuit-terminal/docs/components/horizontal-rule.md` — all code fences must compile when mirrored as rustdoc doctests. Cross-reference 4.5.

### 7.4 Update BrowserRenderable trait doc (A4 tie-in)

- File: `biscuit-terminal/docs/components/browser-renderable-trait.md`.
- The existing "Example Implementation" shows a CSS-variable pattern that roughly matches Phase 3's implementation — verify it aligns. Update if divergent (specifically, the example uses `stroke-width="var(--hr-width, ...)"` where the new implementation uses `stroke-width="var(--hr-weight, ...)"` — fix the variable name to `--hr-weight`).

### 7.5 Update darkmatter user-facing docs (A4 tie-in, B1 tie-in)

- File: `darkmatter/docs/topics/horizontal-rules.md`.
- Add an "Attribute validation" subsection that describes: unknown `style`/`alignment`/`weight` values fall back to the default silently-in-output but emit `tracing::warn!` (visible via `RUST_LOG=darkmatter=warn`). Unknown keys are ignored.
- Update the attribute table to note which attributes are honored in each target (terminal / browser).

### 7.6 Update agent skills (A1 deferral tie-in)

- File: `.claude/skills/darkmatter/SKILL.md`.
  - Add or update a line under "Horizontal rules" that lists supported attributes and notes Tier 1 deferral.
- File: `.claude/skills/biscuit-terminal/SKILL.md` (or the repo's equivalent — confirm path with `ls .claude/skills/`).
  - Add `HorizontalRule` and `BrowserRenderable` to the component/trait catalog.
  - Note the new prelude exports from 4.1.

### 7.7 Optional polish (E1, E3, E4)

These are flagged as "nice to have" in the review. Implement the cheap ones:

- **E1** (serde derives on `RuleStyle`/`RuleAlignment`/`RuleWeight`): add `#[derive(Serialize, Deserialize)]` guarded by a new feature flag `serde` in `biscuit-terminal/lib/Cargo.toml` (pattern: other crates in the monorepo already gate serde this way — verify with `grep -l 'serde = \[' biscuit-terminal/`). Default off.
- **E3** (redundant `to_string()` in `centered_symbol_pattern`): assign `let s = line_char.to_string();` once and reuse.
- **E4** (intermediate `String` allocations): skip. The review itself marks this as "if this becomes hot" — defer until benchmarks justify.

### 7.8 Test coverage

- For E1: add a unit test `test_rule_style_serde_roundtrip` behind `#[cfg(feature = "serde")]` that serializes `RuleStyle::Dashes` to JSON via `serde_json` and deserializes it back.
- For E3: no new test — refactor is behavior-preserving, existing tests cover it.
- For docs: no automated test for the markdown docs, but run `pnpm check-fixed` or `just doctest` (confirm recipe exists in `darkmatter/justfile` and `biscuit-terminal/justfile`) to catch rustdoc regressions.

### Validation

- `cargo test -p biscuit-terminal --all-features`
- `cargo test -p darkmatter`
- `cargo clippy -p biscuit-terminal -p darkmatter --all-features -- -D warnings`
- `cargo test --doc -p biscuit-terminal`
- Manually read the three doc files (`horizontal-rule.md`, `browser-renderable-trait.md`, `horizontal-rules.md`) to confirm examples reflect current code.

---

## Cross-cutting validation (must pass after Phase 7)

All of the following must be clean before this feature is merged:

```bash
cargo test -p biscuit-terminal
cargo test -p darkmatter
cargo test --doc -p biscuit-terminal
cargo test --doc -p darkmatter
cargo clippy -p biscuit-terminal -p darkmatter -- -D warnings
cargo clippy -p biscuit-terminal -p darkmatter --all-features -- -D warnings
```

And the snapshot deltas from Phases 2, 3, and 6 must all be `cargo insta review`'d and committed.

---

## Review items → phase map (traceability)

| Review ID | Item | Phase |
|-----------|------|-------|
| A1 | Tier 1 SVG→PNG | 7.1 (explicitly deferred, documented) |
| A2 | Terminal `color` no effect | 2.2 |
| A3 | Terminal `weight` no effect | 2.1 |
| A4 | CSS-variable strategy | 3.1, 3.2, 7.4 |
| B1 | Silent drop of unknown enum values | 5.4, 5.5 |
| B2 | `Terminal::new()` per rule | 5.1 |
| B3 | Hardcoded double-newline | 5.2 |
| B4 | Bare `Event::Rule` unhandled | 5.3 |
| B5 | `resolve_width` brace structure | 1.1 |
| B6 | `supports_unicode` wrong proxy | 1.4 |
| B7 | Redundant halved repeats | 1.2 |
| B8 | CJK brackets visually wide | 1.3 |
| C1 | RuleProcessor edge cases | 5.6, 6.2 |
| C2 | Terminal renderer integration | 5.6 |
| C3 | HTML renderer edge cases | 6.2 |
| C4 | Component-level gaps | 6.4 |
| D1 | Doc examples don't compile | 4.2, 4.5 |
| D2 | Unicode fallback table wrong | 4.3 |
| D3 | `RuleWeight` doc pixel values | 1.5 |
| D4 | Missing `spec.md` | 7.2 |
| D5 | No prelude exports | 4.1 |
| D6 | Unused import in integration test | 4.4 |
| E1 | Missing serde derives | 7.7 |
| E2 | Hand-rolled attribute parser | 6.1 |
| E3 | Redundant `to_string()` | 7.7 |
| E4 | Intermediate `String`s | deferred (see 7.7) |

---

## Notable risks and assumptions

1. **`tracing` dependency**: assumes `biscuit-terminal` and `darkmatter` already depend on `tracing` (darkmatter does; biscuit-terminal uses it per `tracing::debug!(terminal_width, ...)` found in terminal.rs). If biscuit-terminal does not, Phase 2.3 adds it as a workspace dep.
2. **`serde_yaml_ng`**: per `MEMORY.md` it is the preferred realignment for `serde_yaml`. If darkmatter's Cargo.toml does not yet have it, Phase 6.1 adds it as a workspace dep.
3. **Snapshot churn**: Phases 2, 3, and 6 legitimately change 21+ snapshots. The executor must run `cargo insta review` and inspect each delta to confirm only the expected fields (ANSI codes, heavy chars, CSS variables) change.
4. **Env-var race in 1.6**: `std::env::set_var` is `unsafe` in Rust 2024 edition. If the workspace is on 2024, wrap in `unsafe { ... }` with `// SAFETY: test is single-threaded per `#[serial]`` or skip the test and cover the path via dependency injection.
5. **Tier 1 deferral is accepted by the review**. This plan does not implement it; if a future reviewer insists it must ship, add an eighth phase following the `mermaid.rs` resvg pattern.
6. **HTML pipeline `render_to_browser_with_inline_variables` hookup (6.2)**: requires a new HTML options field. If the HTML options struct is public, this is an additive, non-breaking change. If any downstream code matches exhaustively against that struct, consider using `#[non_exhaustive]` already in place, or introduce a builder.
