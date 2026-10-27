---
total_phases: 5
created: 2026-10-27
phase: 4
agent: opencode/zai-coding-plan/glm-5.3
yolo: 'true'
source_files_during_phase_1:
- biscuit-terminal/lib/src/components/horizontal_rule/browser.rs
- biscuit-terminal/lib/src/components/horizontal_rule/color.rs
- biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
- biscuit-terminal/lib/src/components/horizontal_rule/snapshots/
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- biscuit-terminal
source_files_during_phase_2:
- biscuit-terminal/lib/src/components/horizontal_rule/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
- biscuit-terminal/lib/Cargo.toml
- biscuit-terminal/justfile
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
---

# Plan: HorizontalRule browser parity — remove the legacy SVG, revive the hidden tests

All paths are relative to `biscuit-terminal/` inside this worktree unless stated otherwise. Line numbers were verified against the working tree on 2026-10-27.

## Summary and Definition of Done

`cargo check --workspace --tests --all-features` fails with 28 errors (verified this session), all inside `#[cfg(feature = "serde")] mod serde_roundtrip` in `lib/src/components/horizontal_rule/mod.rs:2797` — eight stale `parity_*` tests that compare a legacy browser SVG (`HorizontalRule::render_browser_svg`) against `renderable::tree::graphics::horizontal_rule_svg`, plus one misplaced terminal test. The legacy SVG drifted from the shared renderer after `8c40526eb` added an `alignment` parameter: it always centers, honors authored width under `Full`, and interpolates `width`/`color` into HTML unvalidated.

The work: **delete** the legacy browser path (`render_browser_svg`, the `MarginToCss` trait, and their tests), **delegate** `impl BrowserRenderable for HorizontalRule` to `BrowserTreeComponent` (the pattern `Prose` already uses), **move** the terminal color helpers into a new `horizontal_rule/color.rs`, **migrate** the surviving ungated browser tests to the public fragment (`render_html_fragment().render()`), **add** fragment-level contract tests for alignment margins, outer layout, and unsafe-value fallback, **relocate** the misplaced terminal test out of the serde gate, **enable** `serde` in the L1 test feature selection (Cargo metadata + justfile), and **align** the docs (`horizontal-rule.md`, `browser-renderable-trait.md`) with the single tree path.

**Definition of Done** (each item is observable):

- `render_browser_svg` and `MarginToCss` do not exist anywhere in the repo; `grep -rn "render_browser_svg\|MarginToCss" biscuit-terminal/lib/src` returns nothing.
- `cargo check -p biscuit-terminal --tests --all-features` compiles clean (run from the repo root or package area).
- `just test` in `biscuit-terminal` selects and passes the four serialization tests (`test_rule_style_serde_roundtrip`, `test_rule_style_serialized_as_kebab_case`, `test_rule_alignment_serde_roundtrip`, `test_rule_weight_serde_roundtrip`) and the relocated `render_text_tier_matches_render_when_image_unavailable` — confirmed by inspecting the run listing, not just a green exit.
- Fragment tests prove: `Left`/`Right`/`Centered` keep authored width with the correct auto margin, `Full` forces `100%` with zero horizontal margins, outer layout wraps exactly once (a browser-specific margin resolves for Browser and appears once), a default layout adds no wrapper, and unsafe `width`/`color` fall back to `100%`/`currentColor` with injection payloads absent.
- `cargo check -p biscuit-terminal --no-default-features` still compiles.
- `just lint` and `just doctest` pass.
- Only the intended browser snapshots change: the 21 `browser_{style}_{weight}` snaps and 2 `browser_custom_attributes*` snaps in `lib/src/components/horizontal_rule/snapshots/`; every `terminal_*` snap is byte-identical.
- Terminal rendering tests pass with unchanged expected output; no terminal code path changes.
- `docs/components/horizontal-rule.md` and `docs/components/browser-renderable-trait.md` describe the delegated tree path, the `Full` width contract, alignment-vs-outer-layout, and the validation fallbacks; they no longer reference `render_to_browser`, `render_to_browser_with_inline_variables`, or a `Renderable` trait.

The Input Robustness Matrix does not apply: this change touches a renderer, its tests, build metadata, and docs — no file-format or configuration reader is added or changed. The `width`/`color` string fields are renderer inputs whose validation behavior is nevertheless fully covered by the new fragment tests (safe values pass; quote, semicolon, and mixed payloads fall back).

## Phase 1 — Legacy path removal and tree delegation

**Wave 1** (single implementer; the tasks share files and are sequential). No other phase may start before this phase's checkpoint passes.

### Necessary Rules

1. **The color helpers move to a new sibling file `lib/src/components/horizontal_rule/color.rs`, keeping `pub(super)` visibility.** Evidence: spec Fix item 3 says "Move the helpers to `color.rs` and keep the browser impl in `browser.rs` … without relocating the impl into the already large parent module" — the "parent module" is `horizontal_rule/mod.rs` (3038 lines), so `color.rs` is a sibling of `browser.rs`, not `crate::utils::color` (which is a directory module for ANSI emission types and is unrelated). Move `parse_basic_color`, `parse_hex_color`, and `nearest_basic_color` verbatim with their doc comments and `use crate::utils::color::{BasicColor, RgbColor};`.
2. **`render_html_fragment` delegates unconditionally:** `BrowserTreeComponent::new(self.clone()).render_html_fragment()` — no default-layout shortcut. Evidence: `render_tree/browser_adapter.rs:100-107` renders with `BrowserRenderOptions::default()` (whose `graphics_mode` is `GraphicsMode::Rich`, `renderable/src/tree/mod.rs:70-78`), and `renderable/src/tree/render/browser.rs:906-909` already adds no wrapper for a default layout, so a shortcut would be dead code. `Prose` uses exactly this delegation for its layout case (`components/prose/prose.rs:450`).
3. **`as_any` stays `self`; `render_html_page` is not overridden.** Evidence: the trait default (`renderable/src/browser/renderable.rs:47-53`) builds an `HtmlPage` from the delegated fragment, which is the "keep page rendering working through the delegated fragment" requirement; the existing test `test_browser_renderable_as_any` (mod.rs:1511) keeps passing unchanged.
4. **Thirteen tests leave `serde_roundtrip` as follows:** delete the eight `parity_*` tests (mod.rs:2861-3016; their sole purpose was comparing the two copies), delete the three `to_css_value` helper tests `test_margin_percent_emits_percent`, `test_margin_zero_emits_default`, `test_margin_chars_emits_ch` (mod.rs:2723-2740; they test only `MarginToCss`, which has no other caller), and move `render_text_tier_matches_render_when_image_unavailable` (mod.rs:3019-3036) into the ungated `mod tests` with `#[serial_test::serial(locale_env)]` added while keeping its `ScopedLcAll::force_utf8()` guard — matching every neighboring test that mutates locale env vars (e.g. mod.rs:1034-1035). Evidence: spec Fix items 1 and 4; `ScopedLcAll` and `text_terminal_with_width` are defined in `mod tests` (mod.rs:855, 914), so the relocated test compiles with no import changes.
5. **The unknown-color warning no longer promises browser pass-through.** Replace the message at mod.rs:736-741 with exactly this text: `"unknown horizontal rule color string; terminal output is uncolored. Browser rendering validates color values independently and substitutes a safe fallback; browser-valid colors the terminal does not recognize (for example `tomato`) still render in the browser. Recognized: CSS basic-16 names, `gray`/`grey`, and `#rrggbb` hex."` Update `test_apply_terminal_color_unknown_name_warns_with_asymmetry_note` (mod.rs:2770) to assert `logs_contain("Browser rendering validates color values independently")`. Evidence: spec Fix item 3; `tomato` passes `is_safe_css_color` (all-letters, `renderable/src/tree/graphics.rs:227`) and fails `parse_basic_color`, so the claim is accurate.
6. **The tree projection doc comment is corrected:** mod.rs:236 "Browser lowers to `<hr>` and Markdown to `---`" becomes wording that browser output is the `ThematicBreak` rendering — an inline SVG under the default rich graphics mode, a plain `<hr>` with `data-hr-*` attributes under `GraphicsMode::Off`. Evidence: `renderable/src/tree/render/browser.rs:870-913`.
7. **The module doc (mod.rs:1-8) is rewritten** to list `color.rs` (terminal color parsing helpers) and `browser.rs` (`impl BrowserRenderable`, delegating to the render tree) and to drop the `MarginToCss` mention. mod.rs's import becomes `use self::color::{parse_basic_color, parse_hex_color};` (mod.rs:27).
8. **Terminal behavior is untouched:** `render`, `render_text_tier`, `render_image_tier`, `render_image_svg`, `resolve_width`, and `apply_terminal_color` (apart from the warning string) are not modified. Evidence: spec "Out of scope". The terminal image tier builds its own SVG (`render_image_svg`, mod.rs:506), independent of the browser path.
9. **No callers outside this package area are affected.** Evidence: repo-wide grep shows `render_browser_svg`/`MarginToCss` references only in `horizontal_rule/browser.rs`, `horizontal_rule/mod.rs`, and docs (the `biscuit-visualized` hit is that crate's own private method); `render_browser_svg` is `pub(crate)` and `MarginToCss` is `pub(super)`. Darkmatter's horizontal rules are its own `HorizontalRuleAttrs` type rendered through the shared tree directly. `lib/tests/l1/html_page_example.rs` asserts only `<svg` presence, which the delegated path still emits.
10. **No spike is needed.** Every question (signatures, output shapes, trait defaults, wrapper CSS) was answered by reading code this session; no runtime measurement is required by the spec.

Tasks:

- [x] **Extract color helpers**
  - Create `lib/src/components/horizontal_rule/color.rs` containing `parse_basic_color`, `parse_hex_color`, `nearest_basic_color` moved verbatim from `browser.rs:10-101` (docs included), visibility `pub(super)`.
  - Delete them from `browser.rs`; change mod.rs:27 to `use self::color::{parse_basic_color, parse_hex_color};`; change the test import at mod.rs:840 to `use super::color::{nearest_basic_color, parse_basic_color, parse_hex_color};`.
  - Risk: none — pure move; the helper tests (`test_parse_basic_color_*`, `test_parse_hex_color_*`, `test_nearest_basic_color_primaries`, `test_parse_basic_color_case_insensitive`) keep passing via the new import path.
- [x] **Delegate BrowserRenderable to the tree**
  - Rewrite `browser.rs` to contain only the impl: `render_html_fragment` returns `BrowserTreeComponent::new(self.clone()).render_html_fragment()` (import `crate::render_tree::BrowserTreeComponent`), `as_any` returns `self`. Delete `render_browser_svg` (browser.rs:103-218) and the `MarginToCss` trait + impl (browser.rs:240-256).
  - The impl's doc comment states the output is the render tree's `ThematicBreak` rendering (shared SVG geometry, alignment-driven margins, validated width/color) and that callers wanting a plain `<hr>` with `data-hr-*` attributes can use the tree renderer with `GraphicsMode::Off`.
  - Update mod.rs module doc (rule 7), tree-projection comment (rule 6), and the warn message + its test (rule 5).
- [x] **Restructure the test module**
  - Delete the eight `parity_*` tests and the `to_css_value` block comment in `serde_roundtrip`; move `render_text_tier_matches_render_when_image_unavailable` into `mod tests` (ungated) with `#[serial_test::serial(locale_env)]`; delete the three `test_margin_*` tests; `serde_roundtrip` then contains exactly the four serialization tests.
  - Remove the now-dead "Phase 2 parity … byte-for-byte" comment block (mod.rs:2856-2859).

**Checkpoint (Phase 1 exit):** from the repo root, `cargo check -p biscuit-terminal --tests --all-features` compiles with zero errors, and `cargo check -p biscuit-terminal --no-default-features` still compiles. Also run `cargo nextest run -p biscuit-terminal --features image -E 'test(render_text_tier_matches_render_when_image_unavailable)'` and confirm it passes ungated.

## Phase 2 — Fragment contract tests and snapshot regeneration

**Wave 2, task A** (may run concurrently with Phases 3 and 4; depends on Phase 1). All new tests live in the ungated `mod tests` in `horizontal_rule/mod.rs`, assert on `hr.render_html_fragment().render()` (the HTML string), and none starts with `browser_` (that prefix selects the headless-browser tier in this repo).

Tasks:

- [x] **Migrate surviving browser tests**
  - `test_render_to_browser` (mod.rs:1447): switch to the fragment; add `.alignment(RuleAlignment::Centered)` so the authored `width("50%")` is honored (default `Full` would force `100%`); keep the existing assertions (`width="50%"`, `var(--hr-color, blue)`, `var(--hr-weight, 4)`, `--hr-weight: 4`, `--hr-color: blue`, `--hr-width: 50%`) — all hold on the shared SVG.
  - `test_render_to_browser_default` (mod.rs:1472): fragment; default rule stays `Full` → assertions (`width="100%"`, currentColor vars, declarations) hold unchanged.
  - `test_render_to_browser_contains_css_variables` (2532), `..._every_style_uses_css_variables` (2561), `..._weight_values_match_fallbacks` (2600), `..._fallbacks_work` (2624), `..._color_declaration_reflects_component_color` (2639): fragment; assertions hold as-is.
  - `test_render_to_browser_width_declaration_reflects_component_width` (2654): fragment; add `.alignment(RuleAlignment::Left)` so `60ch` is authored rather than forced to `100%`; assert `width="60ch"` and `--hr-width: 60ch`.
  - `test_render_to_browser_percent_margin` (2745): fragment; assert the outer wrapper carries `margin-top:2%` (the tree lowers a 2% top margin onto a `<div style="...">` wrapper via `hr_outer_box_css`, `renderable/src/tree/render/browser.rs:3317-3367`) — not the legacy `margin: 2% auto` inside the SVG.
  - `test_snapshot_render_to_browser_all_styles` (1903) and the browser halves of `test_snapshot_render_with_custom_attributes` (2007): switch to the fragment; keep snapshot names; terminal halves and their snapshots unchanged.
- [x] **Regenerate snapshots**
  - Run `cargo insta accept` scoped to this package after running the two snapshot tests; verify `git status` shows exactly the 23 expected browser snapshot files changed (21 `browser_{style}_{weight}` + `browser_custom_attributes` + `browser_custom_attributes_2`) under `lib/src/components/horizontal_rule/snapshots/` and no terminal snapshot touched. Inspect at least the `browser_dashes_medium` and `browser_custom_attributes_2` diffs: Full rules show `margin: 0 0 0 0`; the Right/50ch rule shows `margin: 0 0 0 auto` and `width="50ch"`.
- [x] **Add fragment contract tests** (new; names as below)
  - `test_fragment_routes_through_tree_adapter`: a rule setting all fields plus a nondefault outer layout renders byte-identical HTML via `render_html_fragment().render()` and via `BrowserTreeComponent::new(rule.clone()).render_html_fragment().render()` (`use crate::render_tree::BrowserTreeComponent;`). This is the routing check.
  - `test_fragment_alignment_controls_svg_margins`: for `Left`, `Right`, `Centered` at `width("50%")`: assert `width="50%"` plus the SVG style margin — `Left` → `margin: 0 auto 0 0`, `Right` → `margin: 0 0 0 auto`, `Centered` → `margin: 0 auto 0 auto` (shorthand order top/right/bottom/left, `graphics.rs:91-96`).
  - `test_fragment_full_alignment_forces_full_width`: `Full` with `width("50%")` → `width="100%"`, `--hr-width: 100%`, and `margin: 0 0 0 0` in the SVG style.
  - `test_fragment_outer_layout_wraps_rule`: (a) margins — top `ch(2)`, bottom `ch(1)`, left `TargetValue::PerTarget(BTreeMap::from([(RenderTarget::Terminal, Length::ch(2)), (RenderTarget::Browser, Length::ch(5))]))` (imports: `std::collections::BTreeMap`, `renderable::target::RenderTarget`): assert exactly one `<div style="` wrapper, `margin-left:5ch` appears exactly once, `margin-left:2ch` never, `margin-top:2lh` present (vertical `ch` lowers as `lh`, `css_len`, browser.rs:3297-3306 — do not "fix" this), and the SVG is inside the wrapper; (b) sizing — `Width::Fixed(percent 50%)`, `max_width` `ch(40)`, `Alignment::Center`: assert `width:50%`, `max-width:40ch`, `margin-left:auto;margin-right:auto` on the wrapper, wrapper count 1.
  - `test_fragment_default_layout_adds_no_wrapper`: default rule's fragment HTML contains no `<div`.
  - `test_fragment_unsafe_width_falls_back_to_full`: with `Left` alignment (so `Full`'s forced width cannot mask a validation failure), `width("50%\" onload=\"alert(1)")` and separately `width("50%;color:red")`: assert `width="100%"`, `--hr-width: 100%`, the payload substrings (`onload`, `;color:red`) absent.
  - `test_fragment_unsafe_color_falls_back_to_current_color`: `color("blue\" onmouseover=\"alert(1)")` and separately `color("red;background:url(x)")`: assert `--hr-color: currentColor`, `var(--hr-color, currentColor)`, payloads absent.
  - `test_fragment_safe_authored_values_pass_through`: `Left` + `width("50%")` + `color("tomato")` and `color("#336699")`: assert both colors and `width="50%"` appear — positive proof that rejecting everything cannot pass, and that browser-valid terminal-unknown colors like `tomato` still work.
  - Coverage reuse (no new work): all 7 styles × 3 weights remain covered by the migrated snapshot sweep; page composition stays covered by `lib/tests/l1/html_page_example.rs` (already asserts the fragment promotes to an `<svg>`-bearing page).

**Checkpoint:** `cargo nextest run -p biscuit-terminal --features image -E 'test(fragment_) + test(render_to_browser)'` — all green; `git diff --stat` shows only the 23 expected snapshot changes plus source edits.

## Phase 3 — Test feature selection

**Wave 2, task B** (may run concurrently with Phases 2 and 4; depends on Phase 1 only for the compile fix, but the edits themselves are independent).

Tasks:

- [x] **Enable serde for L1 in Cargo metadata**
  - `lib/Cargo.toml` `[package.metadata.ci.tests]` (line 149): `features = ["image", "serde", "terminal-tests", "browser-tests"]`, `local-features = ["image", "serde"]`. Update the adjacent policy comment's first sentence (line 145) to "CI policy: L1 runs with the `image` and `serde` features; …" leaving the rest of that comment intact.
  - Evidence this is safe: the `serde` feature is a pure cfg switch (`serde = []`, Cargo.toml:24) — `serde`/`serde_json` are already unconditional dependencies (Cargo.toml:44-45) — so no dependency graph or `--no-default-features` build changes.
- [x] **Enable serde in justfile recipes**
  - In `biscuit-terminal/justfile`, change the library entries of `sanity` (line 73), `test` (77), `coverage` (108), and `doctest` (113) from `--features image` to `--features image,serde`. Leave `test-l2` (`terminal-tests`) and `test-browser` (`browser-tests`) untouched; the CLI entries get no features.
  - Rewrite the comment block (lines 60-69): remove the entire "NOT `--all-features`" paragraph; correct the `image` claim so it no longer says `image` gates the "horizontal-rule parity suites" — new text states that `image` gates the resvg rasterization tests and `serde` gates the horizontal-rule serialization tests, without per-feature test counts (they drift; the old "137 of 3004" figure is already stale).
- [x] **No CI workflow changes.** Existing tiers, gates, and feature gates (`terminal-tests`, `browser-tests`) are preserved; no job, environment, or duplicate suite is added. The metadata change alone re-scopes L1 feature selection.

**Checkpoint:** `just test` (from the package area) runs and its listing includes the four serialization tests and the relocated terminal test; `cargo check -p biscuit-terminal --tests --all-features` still compiles (both features and all-features paths).

## Phase 4 — Documentation alignment

**Wave 2, task C** (may run concurrently with Phases 2 and 3; docs only, no compiled code).

Tasks:

- [ ] **Rewrite `docs/components/horizontal-rule.md`**
  - Keep the `blast_radius` frontmatter. Replace `rule.render_to_browser()` in both usage examples with `rule.render_html_fragment().render()` (the fragment is the public API; the trait has no `render_to_browser`).
  - Delete the `render_to_browser_with_inline_variables` subsection (lines 149-160) — that method does not exist in the library.
  - Document the browser contract: rendering flows through the shared render tree (`BrowserTreeComponent` delegation); the SVG keeps the `--hr-weight`/`--hr-color`/`--hr-width` CSS variables (preserve the CSS Variables section — they remain supported by the shared renderer); margins follow alignment (`Left` → `0 auto`, `Right` → `auto 0`, `Full` → `0 0`, `Center` → `auto auto`).
  - Document the `Full` behavior change: `HorizontalRule::new()` defaults to `RuleAlignment::Full`, which spans `100%` with no side margins — a custom `width` under `Full` fills the available outer box; to honor an authored width, select `Centered`, `Left`, or `Right`, or constrain the outer box via `Layout::width`/`max_width`. Both existing usage examples already select `RuleAlignment::Centered`, so their `width("75%")` stays correct; only the method name changes in them.
  - Add a short "Rule alignment vs outer layout" explanation: `.alignment()` is the rule's intrinsic placement inside its box (SVG margins); `with_layout`/`Layout` is the outer block box the tree renderer places (wrapper `<div>` with margins/width/max-width; no wrapper for a default layout).
  - Document invalid width/color fallback: unsafe values fall back to `100%`/`currentColor` in the browser; remove the sentence "the raw string is preserved for the browser target" (line 171) and note that browser-valid terminal-unknown colors such as `tomato` still render.
  - Rename the stale `Renderable` trait references (line 69) to `TerminalRenderable`.
  - In "Tier 1 Image Rendering": remove the claim that "the same SVG produced for browser rendering is rasterized" — the terminal image path builds its own SVG (`render_image_svg`) with pixel geometry and a visibility-safe default color; while touching this section, correct the capability gate to "Kitty or ITerm image support" (matches `render_image_tier`, mod.rs:419-423).
- [ ] **Fix `docs/components/browser-renderable-trait.md`**
  - The page documents a trait shape that never existed (`render_to_browser`/`render_to_browser_with_inline_variables` returning `String`, lines 7-34 and 46-85). Replace the trait definition with the real one (`render_html_fragment() -> BrowserFragment<Ready>`, default `render_html_page`, `as_any`) and replace the `HorizontalRule` example with the actual delegated implementation (a few lines calling `BrowserTreeComponent::new(self.clone()).render_html_fragment()`). Keep the CSS-variable table (lines 89-97) — still accurate for the shared SVG.
- [ ] **Skill check (no edit required)**
  - Decision: no skill edit is part of this change. Verified at planning time: neither `.claude/skills/biscuit-terminal/components.md` nor `.opencode/skill/biscuit-terminal/components.md` describes `HorizontalRule`'s browser output (no match for HorizontalRule/`--hr`/`darkmatter-hr` in any skill page; the two copies are byte-identical), so there is nothing to correct there. As a confirmation step only, re-run `rg -n -i "horizontalrule|darkmatter-hr|--hr" .claude/skills/biscuit-terminal/ .opencode/skill/biscuit-terminal/`; the expected result is no matches. If a match appears it means the skill changed after planning; in that case apply to the matching skill page the same corrections listed for `horizontal-rule.md` above (delegated path, no `render_to_browser`, no raw-color pass-through claim) in both the `.claude` and `.opencode` copies so they stay identical.
  - `docs/dependencies.md` needs no change (no crates added or removed).

**Checkpoint:** `rg -n "render_to_browser|render_browser_svg|MarginToCss" biscuit-terminal/docs/ biscuit-terminal/lib/src/` returns nothing.

## Phase 5 — Verification gate and implementation log

**Wave 3** (single implementer; depends on all prior phases). Run everything from the `biscuit-terminal/` package area unless noted.

Tasks:

- [ ] **Full verification gate**
  - `just test` — all L1 green; explicitly confirm the five previously-hidden tests appear in the results: `cargo nextest run -p biscuit-terminal --features image,serde -E 'test(test_rule_style_serde_roundtrip) + test(test_rule_style_serialized_as_kebab_case) + test(test_rule_alignment_serde_roundtrip) + test(test_rule_weight_serde_roundtrip) + test(render_text_tier_matches_render_when_image_unavailable)'` and check the five names in its output.
  - `cargo check -p biscuit-terminal --tests --all-features` (from repo root or package area) — the originally broken combination compiles, including higher-tier test code without launching it.
  - `cargo check -p biscuit-terminal --no-default-features` — the no-feature library build still compiles.
  - `just lint` and `just doctest` — source, lints, and updated public examples pass.
  - `just sanity` — the fast subset with the new feature selection passes.
  - Do not run `just test-l2` or `just test-browser`: no harness test was changed (verified: the only `browser-tests` file is `tests/l1/prose_cells_parity.rs`, which does not touch HorizontalRule).
- [ ] **Acceptance sweep and drift check**
  - `git status`/`git diff --stat` over the whole change: source files (`horizontal_rule/{mod.rs,browser.rs,color.rs}`, `lib/Cargo.toml`, `justfile`), exactly 23 regenerated browser snapshots, the two docs pages, and the new `implementation-log.md` — nothing else; every terminal snapshot byte-identical.
  - `rg -n "render_browser_svg|MarginToCss|to_css_value|render_to_browser" biscuit-terminal/ --glob '!fixes/**' --glob '!reviews/**'` returns nothing.
- [ ] **Write the implementation log**
  - Create `implementation-log.md` in `fixes/2026-10-27-horizontal-rule-browser-parity/`: a short summary of what each phase changed, any departure from the spec (none expected — every spec item maps to a phase above), the snapshot-diff summary, and the verification commands with their pass results.
  - Do not modify the spec file, move any lifecycle directory, or commit — the session's terminal state is "implementation complete, ready for review".