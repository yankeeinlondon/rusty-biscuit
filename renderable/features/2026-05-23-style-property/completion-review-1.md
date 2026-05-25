## spec-1.md - Schema & Parser Review

No completion issues found for sub-spec #1.

The current implementation provides the requested public parser surface in `darkmatter::style`: `from_frontmatter`, `from_json_value`, and `into_strict` are exported from `darkmatter/lib/src/style/mod.rs`, with parser behavior implemented in `darkmatter/lib/src/style/parse.rs:40` and `darkmatter/lib/src/style/parse.rs:258`. The sparse typed schema is present under `darkmatter/lib/src/style/schema/`, with the root buckets in `darkmatter/lib/src/style/schema/mod.rs:18`, page fields in `darkmatter/lib/src/style/schema/page.rs:14`, common component fields in `darkmatter/lib/src/style/schema/common.rs:13`, and list/inline/HR buckets in the sibling schema files.

The core spec requirements are covered: horizontal lengths lower to `renderable::layout::Length` via `darkmatter/lib/src/style/length.rs`, vertical page fields use `u16` row counts in `darkmatter/lib/src/style/schema/page.rs:26`, alignment accepts the documented `centered` alias in `darkmatter/lib/src/style/alignment.rs:11`, colors lower through `renderable::color::Color` with opacity preserved by `StyleColor` in `darkmatter/lib/src/style/color.rs`, and warning categories match the spec in `darkmatter/lib/src/style/warning.rs:20`. Unknown/deprecated key detection is descriptor-backed in `darkmatter/lib/src/style/descriptor.rs:66` and walked in `darkmatter/lib/src/style/walker.rs:19`, including snake_case alias warnings and canonical kebab-case paths. Strict mode correctly promotes only schema warnings in `darkmatter/lib/src/style/parse.rs:398`.

Test coverage is present for parser behavior and drift: unit tests live beside the parser/schema modules, `darkmatter/lib/src/style/coverage_tests.rs` exercises descriptor alias coverage and canonical leaf reachability, and `darkmatter/lib/tests/style_frontmatter.rs:29` checks the fixture path through `from_frontmatter` and strict validation. I attempted targeted `cargo test -p darkmatter ...` runs, but stopped the three review-started Cargo processes because another workspace Cargo job was holding the artifact/package locks; no completed fresh test result is available from this review pass.

## spec-2.md - Page-Level Wiring Review

No completion issues found for sub-spec #2.

The page-level application path is implemented in `darkmatter/lib/src/style/apply.rs:46` and `darkmatter/lib/src/style/apply.rs:284`. `PageStyleOverrides` tracks field-level CLI claims, and `apply_page_style` applies `style.page.*` margins, padding, background, `max-width`, and alignment while skipping CLI-claimed fields. Length lowering matches the spec: horizontal margins/padding resolve percentages against the captured terminal width, `max-width` resolves against post-margin/post-padding content width, CSS lengths return `StyleApplyError::InvalidCssLength`, and zero resolved `max-width` returns `StyleApplyError::InvalidMaxWidth` (`darkmatter/lib/src/style/apply.rs:793`, `darkmatter/lib/src/style/apply.rs:814`).

The CLI wiring is present in both output paths. `render_terminal_output` applies CLI layout flags first and then calls `apply_style_frontmatter` (`darkmatter/cli/src/output.rs:53`), while `html_artifact` does the same before `render_to_browser` (`darkmatter/cli/src/output.rs:424`). `page_style_overrides_from_cli` mirrors margin/padding shorthand expansion plus `--max-width`, `--page-bg`, and alignment claims (`darkmatter/cli/src/output.rs:219`). `--strict-style` is exposed on the CLI (`darkmatter/cli/src/args.rs:662`), and `apply_style_frontmatter` promotes only schema warnings through `into_strict`, leaving `KnownButInactive` informational (`darkmatter/cli/src/output.rs:321`). Non-fatal warnings are emitted through `tracing`, with `KnownButInactive` logged at info level (`darkmatter/cli/src/output.rs:384`).

Warning suppression is covered by the active wiring phase: parser annotation compares descriptor `sub_spec` with `ACTIVE_STYLE_WIRING_SUB_SPEC` (`darkmatter/lib/src/style/parse.rs:22`, `darkmatter/lib/src/style/parse.rs:224`), and the page descriptors remain tagged as sub-spec 2 (`darkmatter/lib/src/style/descriptor.rs:70`). Because the current implementation has advanced through later sub-specs, the active constant is now `7`, which is consistent with spec-2's intent that page fields no longer emit `KnownButInactive`.

Test coverage is substantial. Library integration tests assert fixture parsing, page margin application, no inactive page warnings, terminal/browser render smoke paths, CLI override behavior, percent margin and max-width resolution, and alignment broadcast (`darkmatter/lib/tests/style_frontmatter.rs:32`, `darkmatter/lib/tests/style_frontmatter.rs:108`, `darkmatter/lib/tests/style_frontmatter.rs:137`, `darkmatter/lib/tests/style_frontmatter.rs:196`, `darkmatter/lib/tests/style_frontmatter.rs:224`, `darkmatter/lib/tests/style_frontmatter.rs:237`, `darkmatter/lib/tests/style_frontmatter.rs:257`). CLI tests cover HTML output, strict-style success/failure modes, non-strict unknown keys, and CLI margin override (`darkmatter/cli/tests/cli.rs:3390`, `darkmatter/cli/tests/cli.rs:3410`, `darkmatter/cli/tests/cli.rs:3426`, `darkmatter/cli/tests/cli.rs:3558`, `darkmatter/cli/tests/cli.rs:3584`). The real-terminal Level 2 test verifies the user-visible top and left margins through `md` in a WezTerm pane (`darkmatter/cli/tests/level2_layout.rs:1133`).

Minor suggestion: `darkmatter/docs/rendering/style.md:292` shows the original spec-2 `PageStyleOverrides` fields but omits the later public `align_*` fields now present on the struct. This does not block sub-spec #2 behavior, but the API snippet should be refreshed so the docs match the current public type.

Verification note: I attempted `cargo test -p darkmatter style_frontmatter --test style_frontmatter`, but stopped it after it spent several minutes compiling dependencies without reaching the test binary. No fresh completed Cargo result is available from this review pass.

## spec-3.md - Existing-Component Wiring Review

No completion issues found for sub-spec #3.

The current implementation exposes the requested public override/apply surface in `darkmatter::style`: `ComponentStyleOverrides` has the table/image/block-quote alignment and fill claim bits (`darkmatter/lib/src/style/apply.rs:79`), and `apply_component_style` maps `style.table.*`, `style.images.*`, and `style.block-quote.*` onto `PageComponent::Tables`, `PageComponent::Images`, and `PageComponent::BlockQuotes` respectively (`darkmatter/lib/src/style/apply.rs:407`). The shared lowering path validates width/max-width exclusivity before render, maps `width` to `PageFill::Explicit`, maps `max-width` to `PageFill::Max`, rejects `Length::Css(_)` through `ComponentInvalidCssLength`, and applies alignment only when the matching CLI claim bit is clear (`darkmatter/lib/src/style/apply.rs:756`).

CLI precedence and render-path ordering match the spec. `component_style_overrides_from_cli` treats global `--alignment` / `--fill` as claims for all three buckets and component-specific flags as claims for only their bucket (`darkmatter/cli/src/output.rs:285`). `apply_style_frontmatter` runs `apply_page_style` before `apply_component_style`, then continues through list/HR/color/bespoke style, so component frontmatter can override a page alignment broadcast while CLI values still win (`darkmatter/cli/src/output.rs:353`). The HTML path also calls `apply_style_frontmatter` before `render_to_browser` (`darkmatter/cli/src/output.rs:424`).

Warning lifecycle and docs are in sync with the completed implementation. `ACTIVE_STYLE_WIRING_SUB_SPEC` is now `7`, so sub-spec #3 keys are active, and parser tests explicitly assert table/image/block-quote width, max-width, and alignment no longer emit `KnownButInactive` (`darkmatter/lib/src/style/parse.rs:22`, `darkmatter/lib/src/style/parse.rs:640`). The rendering docs describe the live component buckets, exclusivity rule, kebab-case diagnostics, block-quote width scope, and public API (`darkmatter/docs/rendering/style.md:339`, `darkmatter/docs/rendering/style.md:366`, `darkmatter/docs/rendering/style.md:378`, `darkmatter/docs/rendering/style.md:425`).

Coverage is present at the library, CLI, HTML, and real-terminal levels: `darkmatter/lib/src/style/apply.rs:1193` through `darkmatter/lib/src/style/apply.rs:1414` covers table alignment, width/max-width lowering, image and block-quote fill, CSS-length rejection, CLI suppression, conflict handling, and page-broadcast override behavior; `darkmatter/cli/tests/cli.rs:3641` through `darkmatter/cli/tests/cli.rs:3925` covers CLI override construction, full CLI integration, and emitted component CSS for HTML; `darkmatter/cli/tests/level2_layout.rs:1186` through `darkmatter/cli/tests/level2_layout.rs:1388` covers visible terminal behavior for table max-width/alignment, image fallback alignment, and block-quote wrapping.

Verification note: I attempted `cargo test -p darkmatter component_style --lib`, but it remained blocked on the workspace artifact lock for about a minute and was stopped. No fresh completed Cargo result is available from this review pass.

## spec-4.md - UL/OL/Li Split + Wiring Review

### ✅ PageComponent split + deprecated Lists
**Fully implemented.** `PageComponent::Lists` is retained as a `#[deprecated(note = "use PageComponent::{Ul, Ol, Li}")]` variant, and `Ul`, `Ol`, `Li` are added as concrete variants (`darkmatter/lib/src/layout/types.rs:147-158`).

### ✅ PageComponent::ALL excludes deprecated Lists
**Fully implemented.** `PageComponent::ALL` contains the concrete variants (`Images`, `BlockQuotes`, `Tables`, `CodeBlocks`, `Ul`, `Ol`, `Li`, `Hyperlinks`, `Hr`) and does **not** include deprecated `Lists` (`types.rs:163-173`). Tests verify this at `types.rs:508-518`.

### ✅ --align-lists / --fill-lists broadcast to concrete variants
**Fully implemented.** Both flags iterate `PageComponent::LISTS` and write all three concrete variants (`darkmatter/cli/src/output.rs:150-153` and `:181-184`). `PageComponent::LISTS` is defined as `[Self::Ul, Self::Ol, Self::Li]` (`types.rs:176`).

### ✅ Granular list CLI flags exist
**Fully implemented.** `--align-ul`, `--align-ol`, `--align-li`, `--fill-ul`, `--fill-ol`, `--fill-li` are declared in `darkmatter/cli/src/args.rs:603-612` and `:639-648`, and wired in `output.rs:155-163` and `:186-193`.

### ✅ style.ul.left-margin uses independent indent channel
**Fully implemented.** `DarkmatterPage` stores `list_left_margins: HashMap<PageComponent, WidthUnit>` (`page.rs:84`), with builder methods `with_list_left_margin` and `try_with_list_left_margin` that **only** accept `PageComponent::Ul` (`page.rs:632-657`). The fallible variant returns `PageRenderError::InvalidListLeftMarginComponent` for `Ol`, `Li`, or non-list components. `LayoutContext` carries this map and exposes `list_left_margin(PageComponent)` (`context.rs:368-370`). The terminal renderer resolves it at `terminal.rs:1468-1474`.

### ✅ width / max-width mutually exclusive per list bucket
**Fully implemented.** `apply_list_bucket` (`apply.rs:723-727`) checks `(style.width, style.max_width)` and returns `StyleApplyError::WidthMaxWidthConflict { bucket }` for `ul`, `ol`, and `li`. Tests cover all three buckets at `apply.rs:1582-1624`.

### ✅ LayoutContext falls back from concrete variants to deprecated Lists
**Fully implemented.** `LayoutContext::component_alignment` (`context.rs:376-388`) and `component_fill` (`context.rs:394-406`) both fall back to the deprecated `PageComponent::Lists` entry when the concrete variant is absent. `DarkmatterPage::alignment_for` and `fill_for` mirror this fallback (`page.rs:210-240`).

### ✅ Terminal renderer uses split list variants
**Fully implemented.** The terminal renderer distinguishes:
- `Tag::List(None)` → `PageComponent::Ul` (`terminal.rs:1442-1444`)
- `Tag::List(Some(_))` → `PageComponent::Ol` (`terminal.rs:1442-1444`)
- `Tag::Item` body overrides consult `PageComponent::Li` for width, alignment, and color (`terminal.rs:1511-1518`, `:1565-1579`)

The `li` body alignment logic scopes markers separately from the body: when `li` alignment is non-left, the body starts on a new line so the marker column is governed by the containing `Ul`/`Ol` (`terminal.rs:1568-1578`).

### ✅ Browser CSS selectors split + cascade order correct
**Fully implemented.** `component_selectors` maps:
- `Ul → "ul"`
- `Ol → "ol"`
- `Li → "li"`
- `Lists → "ul, ol"` (`page.rs:1496-1512`)

`build_component_css` emits deprecated `Lists` rules **first**, then concrete variant rules, so granular styles win by normal cascade (`page.rs:1356-1375`).

### ✅ ACTIVE_STYLE_WIRING_SUB_SPEC ≥ 4
**Fully implemented.** The constant is `7` (`parse.rs:22`), well above 4.

### ✅ KnownButInactive { sub_spec: 4 } suppressed for wired keys
**Fully implemented.** All list wiring keys in `SCHEMA` carry `sub_spec: 4` (`descriptor.rs:100-119`):
- `ul.width`, `ul.max-width`, `ul.alignment`, `ul.left-margin`
- `ol.width`, `ol.max-width`, `ol.alignment`
- `li.width`, `li.max-width`, `li.alignment`

Because `ACTIVE_STYLE_WIRING_SUB_SPEC == 7`, these never emit `KnownButInactive`. Tests confirm silence at:
- `coverage_tests.rs:231-247` (descriptor count includes all list keys)
- `parse.rs:697-725` (`no_known_but_inactive_for_any_valid_v1_key`)
- `parse.rs:753-777` (`test_doc_all_known_but_inactive` expects 0 inactive warnings for a document containing `ul`, `ol`, `li` keys)

### 📋 Test Coverage Locations

| Requirement | File | Lines |
|---|---|---|
| Deprecated Lists fallback for alignment/fill | `layout/page.rs` | 1631-1688 |
| `use_alignment_for_all` / `with_fill_for_all` skip `Lists` | `layout/page.rs` | 1640-1670 |
| `list_left_margin` builder accepts only `Ul` | `layout/page.rs` | 1690-1751 |
| `LayoutContext` fallback to deprecated `Lists` | `layout/context.rs` | 376-406 |
| `apply_list_style` lowers ul/ol/li independently | `style/apply.rs` | 1517-1569 |
| width/max-width conflict for ul/ol/li | `style/apply.rs` | 1582-1624 |
| `ul.left-margin` CSS rejected (CSS length) | `style/apply.rs` | 1626-1641 |
| CLI override suppression for list fields | `style/apply.rs` | 1643-1669 |
| `PageComponent::ALL` / `LISTS` invariants | `layout/types.rs` | 508-518 |
| Terminal renderer list component selection | `markdown/output/terminal.rs` | 1442-1445, 1511-1518 |
| Browser selector mapping + CSS order | `layout/page.rs` | 1356-1417, 1496-1512 |
| **Terminal render: ul left-margin** | `layout/page.rs` | 2651-2666 |
| **Terminal render: ul max-width** | `layout/page.rs` | 2668-2685 |
| **Terminal render: ul left-margin + max-width coexist** | `layout/page.rs` | 2687-2731 |
| **Terminal render: ol alignment right** | `layout/page.rs` | 2733-2752 |
| **Terminal render: li body alignment right** | `layout/page.rs` | 2754-2806 |
| **Browser render: split selectors** | `layout/page.rs` | 2808-2827 |
| **Browser render: deprecated Lists selector when set** | `layout/page.rs` | 2829-2843 |
| **Browser render: ul left-margin CSS** | `layout/page.rs` | 2845-2858 |
| **Li independent of Ul/Ol** | `layout/page.rs` | 2860-2889 |
| CLI granular list flags claim their component | `cli/src/output.rs` | 1137-1151 |
| Parser inactive warning suppression | `style/parse.rs` | 697-777 |
| Descriptor round-trip coverage | `style/coverage_tests.rs` | 89-217 |

### ⚠️ Issues / Missing Pieces

1. **Stale comment in `UlStyle`** (`darkmatter/lib/src/style/schema/lists.rs:15`):  
   The doc comment says `ul.left-margin` is "Wired in sub-spec #4 as `PageFill::Indent` on `PageComponent::Ul`." This is **incorrect** — the implementation deliberately uses the independent `list_left_margins` channel, not `PageFill::Indent`. The comment should be updated to match the actual design.

### 💡 Suggestions

- Fix the stale `PageFill::Indent` comment in `lists.rs` to accurately describe the independent indent channel.

**Overall verdict:** The spec is **fully implemented**. All structural requirements, CLI flags, schema wiring, warning suppression, renderer integration, and browser CSS selectors are in place and covered by both unit and integration tests. The only issue is a misleading inline comment in the `UlStyle` schema struct.

## spec-5.md - Color & Background-Color Mutations Review

### Summary

The implementation of sub-spec #5 is **fully complete and well-tested**. Every requirement in the spec is satisfied, with extensive test coverage at unit, integration, and real-terminal levels.

---

### Detailed Findings

#### ✅ `PageComponent::Hyperlinks` added in this phase
**Verified.** `PageComponent::Hyperlinks` exists in the enum at `darkmatter/lib/src/layout/types.rs:153` and is included in `PageComponent::ALL` (`types.rs:163`). The terminal renderer wires hyperlink color via `LayoutContext::hyperlink_color` (`context.rs:288-313`) and `LineWrapper::push_component_color` / `pop_component_color` (`terminal.rs:1316-1428`). Browser CSS maps `Hyperlinks -> a` (`page.rs:1509`).

#### ✅ `DarkmatterPage` stores all four color maps
**Verified.** `DarkmatterPage` holds:
- `page_color: Option<StyleColor>` (`page.rs:85`)
- `page_bg_color: Option<StyleColor>` (`page.rs:86`)
- `component_colors: HashMap<PageComponent, StyleColor>` (`page.rs:87`)
- `component_bg_colors: HashMap<PageComponent, StyleColor>` (`page.rs:88`)

Builder methods `with_page_color`, `with_page_bg_color`, `with_component_color`, `with_component_bg_color` and effective accessors `color_for` / `bg_color_for` are all present (`page.rs:520-275`).

#### ✅ `apply_color_style` exists and wires all required components
**Verified.** `apply_color_style` is defined at `darkmatter/lib/src/style/apply.rs:527-567`. It applies:
- Page-level `color` and `bg-color` as inherited defaults.
- Component-level overrides for `Tables`, `Images`, `BlockQuotes`, `Hyperlinks`, `Ul`, `Ol`, and `Li`.

It is invoked in the CLI pipeline at `darkmatter/cli/src/output.rs:369-370`, placed after `apply_list_style` and before rendering, matching the spec's prescribed ordering.

#### ✅ Page color acts as inherited default
**Verified.** `DarkmatterPage::color_for` / `bg_color_for` fall back from component map to page-level value (`page.rs:261-275`). `LayoutContext::component_color` / `component_bg_color` mirror this inheritance (`context.rs:266-281`). Tests confirm inheritance at `page.rs:2930-2949` and `context.rs:838-868`.

#### ✅ Code blocks participate only through page-level inheritance
**Verified.** `apply_color_style` does **not** wire a `style.code-blocks.*` bucket (there is no such schema bucket). Code blocks receive inherited background color via `LayoutContext::component_bg_color(PageComponent::CodeBlocks)` in the terminal renderer (`terminal.rs:3233-3238`), and the browser CSS selector `.code-block, pre` is present in `component_selectors` (`page.rs:1502`). The spec's deliberate asymmetry is honored: bg-color may change the panel/container, but there is no foreground override that would clobber syntax token colors.

#### ✅ Terminal emits SGR and resets at component boundaries
**Verified.** The shared helper `wrap_with_color` (`color.rs:550-572`) opens foreground (`38;2;r;g;b`) and/or background (`48;2;r;g;b`) SGR sequences and guarantees a `\x1b[0m` reset when any SGR was opened.

Tables are wrapped directly (`terminal.rs:1745-1750`). Images are wrapped directly (`terminal.rs:1837, 1851, 1861`). Blockquotes, lists, and hyperlinks use `LineWrapper`'s scoped color stack (`push_component_color` / `pop_component_color`, `terminal.rs:2627-2633`) so color resets automatically when the component scope ends. Tests verify reset boundaries at `page.rs:3058-3070`.

#### ✅ Browser/HTML emits per-component CSS with `rgba(...)` when opacity is present
**Verified.** `lower_to_css` (`color.rs:508-525`) produces `rgb(r, g, b)` for opaque colors and `rgba(r, g, b, alpha)` when `StyleColor.opacity` is `Some`. `emit_component_color_rules` (`page.rs:1482-1493`) injects `color:` and `background-color:` declarations into the component CSS rule. Tests confirm `rgba` output at `page.rs:3133-3148`.

#### ✅ Terminal drops opacity while browser preserves it
**Verified.** `lower_to_sgr` (`color.rs:536-543`) calls `style_color.color.to_rgb()` and ignores the `opacity` field entirely. Browser tests show `rgba(..., 0.5)` is preserved (`page.rs:3133-3148`), while terminal tests confirm plain `38;2;...` SGR without opacity bytes (`page.rs:3151-3166`).

#### ✅ `ColorDepth::None` emits no color SGR
**Verified.** `lower_to_sgr` returns `None` immediately when `color_depth == ColorDepth::None` (`color.rs:537`). The terminal renderer also performs a post-render `strip_csi_sequences` pass when `ColorDepth::None` is active (`terminal.rs:1945-1947`), ensuring no color escape codes reach the output. Tests at `page.rs:3042-3055` and `page.rs:3277-3297`.

#### ✅ Special CSS colors handled correctly in browser output
**Verified.** `lower_to_css` maps:
- `Tailwind::Transparent` -> `"transparent"`
- `Tailwind::Current` -> `"currentColor"`
- `Tailwind::Inherit` -> `"inherit"`

Non-RGB values (`DefaultForeground`, `DefaultBackground`, `Reset`) return `None` and emit no CSS declaration. Tests at `page.rs:3168-3199` and `color.rs:940-960`.

#### ✅ `build_component_css` includes color/bg-color in the same selector rule
**Verified.** `build_component_css` (`page.rs:1353-1417`) emits alignment, fill, color, and background-color declarations within a single CSS rule for each component. Concrete list selectors are split (`Ul -> ul`, `Ol -> ol`, `Li -> li`) and the deprecated `Lists -> ul, ol` rule is emitted first so concrete rules override it via cascade. The `Hyperlinks -> a` selector is included.

#### ✅ `is_default_layout`, `needs_decoration`, and `has_component_styles` include color maps
**Verified.**
- `DarkmatterPage::is_default_layout` checks `page_color.is_none() && page_bg_color.is_none() && component_colors.is_empty() && component_bg_colors.is_empty()` (`page.rs:975-978`).
- `LayoutContext::from_page` sets `has_layout = true` when any color map is non-empty (`context.rs:178-188`).
- `LayoutContext::has_component_styles` checks `!component_colors.is_empty() || !component_bg_colors.is_empty()` (`context.rs:253-259`).

Tests confirm color-only configuration is not optimized away: `page.rs:2985-3000` and `context.rs:799-835`.

#### ✅ `ACTIVE_STYLE_WIRING_SUB_SPEC` is at least 5
**Verified.** The constant is `7` at `darkmatter/lib/src/style/parse.rs:22`. This exceeds the spec's minimum of `5` because sub-specs #6 (HR) and #7 (bespoke knobs) were subsequently implemented and advanced the constant.

#### ✅ `KnownButInactive { sub_spec: 5 }` warnings are suppressed for wired color keys
**Verified.** Because `ACTIVE_STYLE_WIRING_SUB_SPEC == 7`, any schema key with `sub_spec <= 7` (including all color and bg-color keys) does not emit `KnownButInactive`. Tests explicitly assert silence:
- `darkmatter/lib/tests/style_frontmatter.rs:283-301` (`color_keys_do_not_emit_known_but_inactive`)
- `darkmatter/lib/src/style/coverage_tests.rs:138-165` (alias round-trip check)
- `darkmatter/lib/src/style/coverage_tests.rs:195-217` (canonical leaf round-trip check)

---

### Test Coverage Locations

| Concern | File | Lines |
|---|---|---|
| Color API (setters/getters/inheritance) | `darkmatter/lib/src/layout/page.rs` | 2910-2982 |
| Color-only layout fast-path guards | `darkmatter/lib/src/layout/page.rs` | 2985-3000 |
| Terminal SGR emission & reset boundaries | `darkmatter/lib/src/layout/page.rs` | 3004-3070 |
| Browser CSS emission & `rgba` opacity | `darkmatter/lib/src/layout/page.rs` | 3072-3148 |
| Terminal opacity dropping | `darkmatter/lib/src/layout/page.rs` | 3151-3166 |
| Browser special-color passthrough | `darkmatter/lib/src/layout/page.rs` | 3168-3199 |
| Browser list selectors with color | `darkmatter/lib/src/layout/page.rs` | 3202-3226 |
| Terminal hyperlink OSC8 preservation | `darkmatter/lib/src/layout/page.rs` | 3229-3252 |
| Code-block bg-color without clobbering highlights | `darkmatter/lib/src/layout/page.rs` | 3255-3269 |
| `ColorDepth::None` preserves table/layout | `darkmatter/lib/src/layout/page.rs` | 3277-3297 |
| List color inheritance into li body | `darkmatter/lib/src/layout/page.rs` | 3305-3322 |
| HR color/browser selector targeting | `darkmatter/lib/src/layout/page.rs` | 3328-3368 |
| LayoutContext color inheritance | `darkmatter/lib/src/layout/context.rs` | 779-869 |
| Lowering helper tests (SGR/CSS/wrap) | `darkmatter/lib/src/style/color.rs` | 574-1079 |
| Warning suppression for color keys | `darkmatter/lib/tests/style_frontmatter.rs` | 283-301 |
| Frontmatter -> `apply_color_style` integration | `darkmatter/lib/tests/style_frontmatter.rs` | 303-328 |
| Real-terminal list color inheritance | `darkmatter/cli/tests/level2_layout.rs` | 1788+ |

---

### Issues / Gaps / Suggestions

1. **No `style.code-blocks.*` bucket (intentional).** The spec correctly excluded a dedicated code-block color bucket. Code blocks only inherit page-level color/bg-color. The terminal renderer skips inherited foreground for highlighted code blocks to avoid clobbering syntax highlighting, which matches the spec's documented limitation. The docs at `darkmatter/docs/rendering/style.md:730` accurately describe this.

2. **HR color is already wired (sub-spec #6 completed).** The review notes that `PageComponent::Hr` and `style.hr.color` / `style.hr.bg-color` are already active. This is not a spec-5 gap; it simply means sub-spec #6 landed before this review and the active wiring constant (`7`) reflects that.

3. **All acceptance criteria are met.** No missing pieces were identified during this code review.

## spec-6.md - HR Migration Review

### Canonical Source & Deprecated Aliases

**`style.hr.*` is canonical; top-level `hr:` is a deprecated alias** — **Fully implemented.**
- `StyleFrontmatter::hr: Option<HrStyle>` is the canonical bucket (`darkmatter/lib/src/style/schema/mod.rs:31`).
- `from_frontmatter()` (`parse.rs:258-271`) detects top-level `hr:` and calls `merge_deprecated_top_level_hr()`, which merges legacy values field-by-field into `style.hr` with `style.hr` winning when both are present (`parse.rs:366-386`).

**Top-level `hr:` emits `Deprecated { replacement: "style.hr" }` warnings** — **Fully implemented.**
- `merge_deprecated_top_level_hr()` emits one `Deprecated` warning per recognized legacy key (`parse.rs:306-326`) and a catch-all warning for non-mapping or empty `hr:` values (`parse.rs:289-299`, `331-338`).
- Tests at `parse.rs:838-854` and `parse.rs:928-1004`.

### Inline Attribute Rename

**Inline HR attribute key renamed from `style` to `kind`** — **Fully implemented.**
- `HorizontalRuleAttrs` has `kind: Option<String>` (canonical) and `legacy_style: Option<String>` (deprecated alias) (`darkmatter/lib/src/markdown/inline/types.rs:43-53`).

**Inline `--- { style: waves }` emits `Deprecated { replacement: "kind" }` warnings** — **Fully implemented.**
- `RuleProcessor::process_paragraph_buffer()` pushes a `StyleWarning` with path `hr.inline.style` and replacement `hr.inline.kind` (`rule_processor.rs:420-427`).
- `scan_inline_hr_warnings()` provides a preflight helper for `--strict-style` (`rule_processor.rs:146-158`).
- Tests at `rule_processor.rs:763-783` and `rule_processor.rs:810-820`.

### Typed Enums

**`HrKind` is a typed enum with correct variants** — **Fully implemented.**
- Defined in `darkmatter/lib/src/style/schema/hr.rs:13-23`: `Dashes`, `Dots`, `Waves`, `LineStar`, `LineCircle`, `InsetLine`, `CurtainRod`.
- Maps to `RuleStyle` in `apply.rs:678-688`.

**`HrWeight` is a typed enum with `Thin`, `Medium`, `Thick`** — **Fully implemented.**
- Defined in `darkmatter/lib/src/style/schema/hr.rs:26-32`.
- Maps to `RuleWeight` in `apply.rs:691-697`.

**`HrAlignment` exists and accepts `full | left | center | right`** — **Fully implemented.**
- Defined in `darkmatter/lib/src/style/schema/hr.rs:35-43` with variants `Full`, `Left`, `Center`, `Right`.
- Maps to `RuleAlignment` in `apply.rs:700-707`.

**Note on `centered` alias:** `HrAlignment::Center` has `#[serde(alias = "centered")]` which silently accepts the legacy spelling. However, **no deprecation warning is emitted** and `--strict-style` does **not** reject it. This is a gap — see Issues below.

### `HrStyle` Struct

**`HrStyle` includes `width`, `max-width`, `color`, `bg-color`, `alignment`, `kind`, `weight`** — **Fully implemented.**
- Defined in `darkmatter/lib/src/style/schema/hr.rs:45-65` with all seven fields, custom length/color deserializers, and snake-case aliases for `max_width` and `bg_color`.

### `PageComponent::Hr` & Color Wiring

**`PageComponent::Hr` exists and is honored by color/bg-color handling** — **Fully implemented.**
- `PageComponent::Hr` added in `darkmatter/lib/src/layout/types.rs:155` and included in `PageComponent::ALL` (line 163).
- `apply_hr_style()` calls `with_component_color(PageComponent::Hr, ...)` and `with_component_bg_color(PageComponent::Hr, ...)` (`apply.rs:637-647`).
- Tests at `apply.rs:1755-1791`.

### `apply_hr_style` & Rendering Wiring

**`apply_hr_style` exists and wires HR style to terminal and browser rendering** — **Fully implemented.**
- Defined in `darkmatter/lib/src/style/apply.rs:598-650`.
- Wired into CLI integration order in `darkmatter/cli/src/output.rs:365-367`.
- `DarkmatterPage` stores `hr_kind`, `hr_weight`, `hr_alignment`, `hr_width` and exposes `hr_defaults()` (`layout/page.rs:330-340`).
- Both terminal (`terminal.rs:854`) and HTML (`html.rs:941`) renderers consume `options.hr_defaults` via `build_rule_with_defaults()`.

### Width/Max-Width Exclusivity

**`width` and `max-width` are mutually exclusive for HR** — **Fully implemented.**
- `apply_hr_style()` returns `StyleApplyError::ComponentWidthConflict { bucket: "hr" }` when both are set (`apply.rs:609-611`).
- Test at `apply.rs:1744-1753`.

### Precedence

**Precedence is inline > `style.hr` > top-level `hr` alias > component default** — **Fully implemented.**
- `from_frontmatter` merges top-level `hr` into `style.hr` with `style.hr` winning field-by-field (`parse.rs:366-386`).
- `build_rule_with_defaults()` merges inline `attrs` over `defaults` (`hr_builder.rs:209-233`), with `kind` winning over `legacy_style` in the builder (`hr_builder.rs:107-111`).

### `--strict-style`

**`--strict-style` rejects deprecated HR syntax (top-level `hr:` and inline `style`)** — **Fully implemented.**
- `apply_style_frontmatter()` at `cli/src/output.rs:337-349` partitions schema warnings, extends them with `scan_inline_hr_warnings()`, and passes the combined set to `into_strict()`.
- Tests confirm strict rejection of top-level `hr:` at `parse.rs:928-1004`.

### Active Wiring Sub-Spec

**`ACTIVE_STYLE_WIRING_SUB_SPEC` is at least 6** — **Fully implemented.**
- Set to `7` in `darkmatter/lib/src/style/parse.rs:22`.

**`KnownButInactive { sub_spec: 6 }` warnings are suppressed for wired HR keys** — **Fully implemented.**
- All HR schema leaves have `sub_spec: 6` in `descriptor.rs:146-152`.
- Tests confirm silence at `parse.rs:670-695` and `coverage_tests.rs:251-286`.

### Documentation

**`darkmatter/docs/rendering/hr.md` and `style.md` document `style.hr` as canonical and deprecated aliases** — **Fully implemented.**
- `hr.md` lines 9-21 describe precedence (inline > `style.hr` > top-level `hr` alias > default).
- `hr.md` lines 47-53 document inline `style` as deprecated alias for `kind`.
- `hr.md` lines 120-132 document top-level `hr:` as deprecated alias for `style.hr`.
- `style.md` lines 806-938 contain the full Sub-Spec #6 section including the `--strict-style` rejection table.

### Test Coverage

| File | Lines | Coverage |
|------|-------|----------|
| `darkmatter/lib/src/style/schema/hr.rs` | 67-209 | `HrKind`, `HrWeight`, `HrAlignment`, `HrStyle` deserialization |
| `darkmatter/lib/src/style/descriptor.rs` | 66-153 | Schema catalog with `sub_spec: 6` for all HR leaves |
| `darkmatter/lib/src/style/parse.rs` | 670-1004 | Top-level `hr` alias merge, strict-mode rejection, `KnownButInactive` suppression |
| `darkmatter/lib/src/style/apply.rs` | 1683-1848 | `apply_hr_style`: kind, weight, alignment, width, max-width, color, bg-color, conflict, overrides |
| `darkmatter/lib/src/style/coverage_tests.rs` | 251-286 | Cross-cutting silence for HR keys |
| `darkmatter/lib/src/markdown/block/rule_processor.rs` | 747-834 | Inline `kind` vs `style` parsing and deprecation warnings |
| `darkmatter/lib/src/markdown/block/hr_builder.rs` | 291-454 | `build_rule_with_defaults`, `hr_defaults_from_frontmatter`, scalar coercion |

### Issues / Missing Pieces

**`style.hr.alignment: centered` deprecation warning is missing.**
- `HrAlignment::Center` uses `#[serde(alias = "centered")]` (`schema/hr.rs:40`). Serde silently accepts the alias without surfacing which spelling was used.
- The schema walker (`walker.rs`) only handles snake-case (`_` -> `-`) aliases; it has no mechanism to detect the special-case `centered` -> `center` alias.
- **Consequence:** no `Deprecated` warning is emitted when a document uses `style.hr.alignment: centered`, and `--strict-style` does **not** reject it.
- This contradicts `spec-6.md` Design Decision #10, `style.md` line 937, and `hr.md` line 91.
- **Fix required:** Add a post-parse pass or custom deserializer for `HrAlignment` that emits a `Deprecated` warning when the raw value is `"centered"`. The warning path should be `style.hr.alignment` with replacement `center`.

**Minor note:** `hr_defaults_from_frontmatter()` in `hr_builder.rs` still reads raw top-level `hr:` directly. For the CLI path this is effectively dead code because `DarkmatterPage::hr_defaults()` already supplies merged values, but direct-API callers still hit the legacy path. This is an acceptable compatibility boundary.

**Overall status:** 13 of 14 explicit requirements fully implemented. The `centered` alias warning is the only gap.

## spec-7.md - Bespoke Knobs Review

### `apply_bespoke_style` in the CLI Pipeline
**Implemented.** `apply_bespoke_style` is defined in `darkmatter/lib/src/style/bespoke.rs` (line 90) and is called from `darkmatter/cli/src/output.rs` inside `apply_style_frontmatter` at line 376. The call order matches the spec exactly: `apply_page_style` -> `apply_component_style` -> `apply_list_style` -> `apply_hr_style` -> `apply_color_style` -> `apply_bespoke_style`.

### `style.page.stylesheet`
**Fully implemented.**
- Local relative paths resolve against `source_path` parent or CWD, are read from disk, and inlined as `<style data-darkmatter-source="...">` (`bespoke.rs` lines 207-238; `page.rs` `wrap_browser_html` lines 1262-1285).
- HTTP(S) emitted as `<link rel="stylesheet" href="...">` (`bespoke.rs` lines 201-205).
- `file://` rejected (`bespoke.rs` lines 193-198).
- Errors present in `apply.rs` and used correctly:
  - `StylesheetNotFound` — `bespoke.rs` line 225
  - `StylesheetRead` — `bespoke.rs` line 230
  - `EmptyStylesheet` — `bespoke.rs` line 190
  - `UnsupportedStylesheetScheme` — `bespoke.rs` line 195

### `style.page.meta`
**Fully implemented.**
- Object shape enforced: `parse_page_meta` rejects non-objects with `InvalidMetaShape` (`bespoke.rs` lines 247-253).
- String/number/boolean lowered to `content` via `meta_scalar` (lines 306-316).
- Array accepted only for `keywords`, joined with `, ` (lines 272-287).
- `charset` -> `<meta charset="...">` (lines 267-270).
- `og:*` -> `<meta property="og:...">` (lines 292-296).
- Other keys -> `<meta name="...">` (lines 297-301).
- HTML escaping performed by the tag renderer, not the applicator (`wrap_browser_html` lines 1232-1259 uses `html_escape::encode_text`).
- `InvalidMetaShape` is the documented error.

### `style.page.code.theme`
**Fully implemented.**
- Parses via `ThemePair::try_from` (`bespoke.rs` line 118).
- CLI `--code-theme` wins: `BespokeStyleOverrides::code_theme` suppresses frontmatter when true (`bespoke.rs` lines 115-122; `output.rs` line 315-318).
- Error: `InvalidCodeTheme` defined in `apply.rs` line 215 and raised in `bespoke.rs` line 118.
- **Minor discrepancy:** the spec documents the variant as `InvalidCodeTheme { value }`, but the actual field name is `theme` (`apply.rs` line 215).

### `style.hyperlinks.{color,bg-color}` visual activation
**Fully implemented.**
- **Terminal:** `terminal.rs` pushes color scope around link display text (lines 1314-1345) and wraps styled text inside OSC 8 sequences (lines 1412-1416). SGR reset closes before the OSC 8 end sequence.
- **HTML:** `html.rs` lines 382-404 apply `CommonStyle::to_css_overlay()` to links, merging inline CSS so per-link inline styles win over frontmatter for the same property.

### `style.hyperlinks.{width,max-width,alignment}`
**Fully implemented.**
- Terminal: `apply_inline_text_layout` in `bespoke.rs` (lines 431-476) resolves width/max-width/alignment against the terminal effective width, with truncation/padding.
- HTML: `to_css_overlay` in `schema/common.rs` (lines 46-100) emits `width`, `max-width`, and `text-align` CSS declarations.
- CSS lengths (`Length::Css`) are accepted for HTML but rejected for terminal via `validate_terminal_inline_lengths` (`bespoke.rs` lines 368-395), which returns `PageRenderError::InvalidInlineCssLength`.

### `style.hyperlinks.local-style.*` overrides only local hyperlinks
**Fully implemented.**
- Local detection: `is_local_hyperlink` (`bespoke.rs` line 322) returns true for anything not starting with `http://` or `https://`, matching the spec's definition (relative paths, absolute paths, anchors, `file://`).
- Terminal path: `terminal.rs` lines 1316, 1401 use `is_local_hyperlink`.
- HTML path: `html.rs` line 384 uses `Link::is_file()` (which corresponds to `LinkType::File`).
- Merge: `local-style` merges field-by-field over the outer `hyperlinks` bucket via `merge_common_style` (`bespoke.rs` lines 343-351).

### `style.images.local-style.*` overrides only local images
**Fully implemented.**
- Local detection: `is_local_image` (`bespoke.rs` line 331) returns true for non-HTTP(S) and non-`data:` URLs.
- Terminal path: `terminal.rs` line 1798 uses `is_local_image`.
- HTML path: `html.rs` line 474 uses `is_local_image`.
- Merge: same `merge_common_style` helper.

### Width/max-width conflict detection
**Fully implemented for all three buckets.**
- `hyperlinks`: `bespoke.rs` lines 128-130.
- `hyperlinks.local-style`: `bespoke.rs` lines 133-140.
- `images.local-style`: `bespoke.rs` lines 155-162.
All return `StyleApplyError::ComponentWidthConflict` with canonical kebab-case bucket names.

### `ACTIVE_STYLE_WIRING_SUB_SPEC`
**Is `7`.** Defined at `darkmatter/lib/src/style/parse.rs` line 22.

### `KnownButInactive` emptiness for every valid v1 key
**Verified.**
- `parse.rs` test `no_known_but_inactive_for_any_valid_v1_key` (lines 697-724) exercises a comprehensive document containing every wired key and asserts zero inactive warnings.
- `coverage_tests.rs` tests `every_canonical_leaf_round_trips_without_unknown_key` and `every_alias_round_trips_with_expected_warnings` both assert `expected_inactive == 0` for every leaf whose `sub_spec <= 7`.
- `parse.rs` test `sub_spec_7_keys_no_longer_emit_known_but_inactive` (lines 614-636) spot-checks the new keys explicitly.

### Documentation in `darkmatter/docs/rendering/style.md`
**Fully documented.** The "Bespoke Knobs (Sub-Spec #7)" section (starting around line 981) documents:
- `style.page.stylesheet` behavior (local inline, remote link, `file://` rejection)
- `style.page.meta` conversion rules
- `style.page.code.theme` precedence and error
- `style.hyperlinks.*` terminal SGR / HTML inline CSS behavior
- `style.hyperlinks.local-style.*` merge semantics and local-reference scope
- `style.images.local-style.*` scope and merge semantics
- Width/max-width exclusivity and CSS-length target-specific rejection
- Public API signatures for `DarkmatterPage` builders and `apply_bespoke_style`

### Test Coverage
Key test locations:
- **`darkmatter/lib/src/style/bespoke.rs`** (lines 515-1733): exhaustive unit tests covering:
  - Stylesheet resolution (empty, file://, remote, local relative/absolute, missing) — lines 585-664
  - Meta parsing (description, keywords array, og:, charset, scalar types, invalid shape) — lines 668-775
  - Code theme (apply, CLI override, invalid theme) — lines 808-888
  - Hyperlink style storage, width conflicts, color injection into HTML and terminal, local-style override, per-link inline CSS precedence — lines 890-1255
  - Image local-style storage, width conflicts, color injection into HTML/terminal, remote exclusion — lines 1257-1444
  - Terminal layout (width padding, max-width truncation, alignment, CSS-length rejection) — lines 1446-1672
- **`darkmatter/lib/src/style/parse.rs`** (lines 614-724): `KnownButInactive` suppression tests for sub-specs #3, #6, #7.
- **`darkmatter/lib/src/style/coverage_tests.rs`**: descriptor drift and alias coverage tests ensuring every schema leaf round-trips without unexpected inactive warnings.
- **`darkmatter/lib/src/layout/context.rs`** (lines 510-870): `LayoutContext` tests for color inheritance and hyperlink/image color resolution.

### Issues / Suggestions
1. **Minor API mismatch:** `StyleApplyError::InvalidCodeTheme` uses field name `theme: String` (`apply.rs` line 215) while the spec's Public API section documents `{ value }`. The behavior is correct; only the field name differs.
2. **No issues found** with the core implementation. All bespoke knobs are wired, documented, and tested.
