---
phases: 5
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/layout/types.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/style/apply.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/style/color.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_5:
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/tests/style_frontmatter.rs
docs_updated_during_phase_5:
  - darkmatter/docs/rendering/style.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages:
  - darkmatter
---

# Execution Plan: Sub-Spec #5 - Color & Background-Color Mutations

This execution plan implements sub-spec #5, enabling color and background-color mutations for `DarkmatterPage` components.

## Phase 1: Core Data Structures & API
**Focus:** Extend internal state to hold color definitions and update layout context to prevent color-only configurations from being optimized away.

- [ ] Task: Add `PageComponent::Hyperlinks` to the `PageComponent` enum.
- [ ] Task: Add color maps to `DarkmatterPage` state: `page_color` (Option<StyleColor>), `page_bg_color` (Option<StyleColor>), `component_colors` (HashMap<PageComponent, StyleColor>), and `component_bg_colors` (HashMap<PageComponent, StyleColor>).
- [ ] Task: Implement builder methods on `DarkmatterPage`: `with_page_color`, `with_page_bg_color`, `with_component_color`, `with_component_bg_color`.
- [ ] Task: Implement effective accessors on `DarkmatterPage`: `page_color`, `page_bg_color`, `color_for`, `bg_color_for`. These accessors must resolve component-over-page inheritance.
- [ ] Task: Thread the new color maps into `LayoutContext::from_page` for both `render` and `render_to_browser`.
- [ ] Task: Update `is_default_layout`, `LayoutContext::needs_decoration`, and `LayoutContext::has_component_styles` to check color maps so color-only stylings are not skipped.
- [ ] Checkpoint: Verify `DarkmatterPage` can hold and correctly report inherited colors via the public API.

## Phase 2: Target-Specific Lowering Helpers (Parallelizable)
**Focus:** Implement pure functions that translate `StyleColor` into valid output representations (CSS and Terminal SGR).

- [x] Task: Implement browser lowering helper for `StyleColor` to CSS. Translate RGB-capable to `rgb(...)`/`rgba(...)` (preserving opacity), map `Tailwind` special values (`transparent`, `current`, `inherit`), and return `None` for unsupported values.
- [x] Task: Implement terminal lowering helper that emits SGR sequences based on `StyleColor.color.to_rgb()`. It must emit `38;2;r;g;b` (foreground) or `48;2;r;g;b` (background) depending on the color depth, emit nothing for `ColorDepth::None`, and wrap content to guarantee a reset sequence `\x1b[0m` is printed if any SGR is opened.

## Phase 3: Component CSS & Terminal Output Wiring
**Focus:** Inject lowering helpers into the actual text layout engines.

- [ ] Task: Extend `build_component_css` to include CSS declarations for color, background-color, alignment, fill, and list-indent into a single rule for existing elements. Add mapping for `PageComponent::Hyperlinks` to `a` selector.
- [ ] Task: Wire the terminal SGR wrapper to component rendering phases: Tables, Images, BlockQuotes, Hyperlinks, Ul, Ol, Li. Ensure SGR for Hyperlinks wraps link label text without breaking OSC8 sequences.
- [ ] Task: Apply `style.page.bg-color` inheritance for Code Blocks to the panel/container fill. Ensure token highlighting from the syntax highlighter is untouched. 
- [ ] Task: Apply `style.page.color` inheritance for Code Blocks as fallback foreground (skip highlighting if no safe fallback exists and document the limitation).
- [ ] Checkpoint: Validate terminal renders verify SGR application with precise scope boundaries. Verify browser output holds correct CSS declarations.

## Phase 4: Frontmatter Application Pipeline
**Focus:** Apply parsed configurations onto the `DarkmatterPage` state and manage active wiring properties.

- [ ] Task: Implement `darkmatter::style::apply_color_style(page: DarkmatterPage, style: &StyleFrontmatter) -> Result<DarkmatterPage, StyleApplyError>`.
- [ ] Task: Add `apply_color_style` into the CLI render pipeline just after `apply_list_style` (before rendering).
- [ ] Task: Update the warning lifecycle logic to suppress `KnownButInactive { sub_spec: 5 }` warnings for the wired properties. Ensure `style.hr.*` and `style.hyperlinks.local-style.*` still emit the warning since they apply in future sub-specs.
- [ ] Task: Advance `ACTIVE_STYLE_WIRING_SUB_SPEC` to 5.
- [ ] Checkpoint: Verify passing style configurations correctly maps to `DarkmatterPage` builders without generating inactive warnings for applied keys.

## Phase 5: Tests & Documentation
**Focus:** Cover acceptance criteria, behavior verification, and document user-facing updates.

- [ ] Task: Write test: Page color inherited by components safely handles fallback text.
- [ ] Task: Write test: Component color overrides page color.
- [ ] Task: Write test: Component bg-color overrides page bg-color.
- [ ] Task: Write test: Opacity dropped on terminal but preserved in browser CSS (`rgba()`).
- [ ] Task: Write test: Color depth none behaves as expected, omitting SGR.
- [ ] Task: Write test: Terminal reset boundary properly scopes colors.
- [ ] Task: Write test: Inherited code-block bg-color panel override and inherited foreground don't clobber highlighting.
- [ ] Task: Write test: Browser CSS special colors behavior.
- [ ] Task: Write test: List selectors properly emit separate target rules.
- [ ] Task: Write test: Hyperlink color routing works without harming OSC8 tags.
- [ ] Task: Write test: Validate active wiring warnings are correct.
- [ ] Task: Update `darkmatter/docs/rendering/style.md` moving the components from pending to live, update opacity logic explanation for browser vs terminal, detail CSS special colors, and document lack of reset parsing.