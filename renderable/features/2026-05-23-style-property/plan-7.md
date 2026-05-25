---
phases: 6
created: 2026-05-23
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/style/apply.rs
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/style/mod.rs
  - darkmatter/lib/src/style/schema/common.rs
  - darkmatter/lib/src/layout/page.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_3:
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/tests/style_frontmatter.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/style/bespoke.rs
  - darkmatter/lib/src/layout/context.rs
  - darkmatter/lib/src/layout/page.rs
  - darkmatter/lib/src/markdown/output/html.rs
  - darkmatter/lib/src/markdown/output/terminal.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/style/parse.rs
  - darkmatter/cli/src/output.rs
docs_updated_during_phase_6:
  - darkmatter/docs/rendering/style.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
  - darkmatter-cli
---

# Execution Plan: Sub-Spec #7 - Bespoke Knobs

This execution plan implements the final Sub-Spec #7 for `style:` frontmatter. It covers the unwired bespoke keys, error handling, terminal/HTML rendering paths, and final active wiring bump.

## Phase 1: API Surface and Error Types

Establish the public interface and error types for applying bespoke styles.

- [x] Add `StyleApplyError` variants: `StylesheetNotFound`, `StylesheetRead`, `EmptyStylesheet`, `UnsupportedStylesheetScheme`, `InvalidMetaShape`, `InvalidCodeTheme`.
- [x] Create data types `PageStylesheet` (Inline/Remote), `PageMeta`, and `MetaTag` (Charset/Name/Property).
- [x] Extend `DarkmatterPage` with builder methods: `with_stylesheet`, `with_page_meta`, `with_page_code_theme`, `with_hyperlink_style`, `with_local_hyperlink_style`, and `with_local_image_style`.
- [x] Add `apply_bespoke_style` function signature and `BespokeStyleOverrides` struct.
- [x] Add helper function for `CommonStyle` to convert to a CSS declaration overlay (handles `width`, `max-width`, `text-align`, `color`, and `background-color`).
- [x] **Validation Checkpoint**: Project compiles successfully with all new types and public signatures available.

## Phase 2: Page Stylesheet and Meta Tags (HTML)

Implement HTML head injections for custom CSS and metadata.

- [ ] Implement parsing and validation for `style.page.stylesheet`. Reject `file://` scheme, allow local relative/absolute paths, and allow HTTP(S).
- [ ] Read local stylesheet files from disk (relative to source document or CWD) and inline them into `PageStylesheet::Inline`.
- [ ] Implement `style.page.meta` object parsing into `MetaTag` entries (mapping `description`, arrays to `keywords`, `og:` prefixes to `property`, `charset`, etc.).
- [ ] Update HTML rendering pipeline to emit `<style>` blocks or `<link rel="stylesheet">` for `PageStylesheet`, and `<meta>` tags for `PageMeta`. Ensure proper HTML escaping.
- [ ] **Validation Checkpoint**: Tests pass for local stylesheet inline, remote stylesheet link, missing stylesheet error, and page meta (including invalid shape error).

## Phase 3: Page Code Theme

Implement code block theme defaults and overrides.

- [x] Update `apply_bespoke_style` to parse `style.page.code.theme` into `ThemePair::try_from`.
- [x] Apply CLI precedence: return `BespokeStyleOverrides::code_theme` values over frontmatter values.
- [x] Emit `StyleApplyError::InvalidCodeTheme` if an unknown theme is requested.
- [x] Integrate theme choice before generating terminal and HTML output, respecting inverted color modes.
- [x] **Validation Checkpoint**: Tests pass for code theme frontmatter setting, CLI code theme override, and invalid code theme error.

## Phase 4: Hyperlinks and Local Style [Parallelizable]

Wire `style.hyperlinks.*` and `style.hyperlinks.local-style`.

- [x] Implement local reference detection for hyperlinks: `LinkType::File` (relative, absolute, anchor, `file://`) vs remote (`http/s`).
- [x] Implement conflict detection for `width` and `max-width` under `hyperlinks` and `hyperlinks.local-style`, returning a documented error if both are present.
- [x] Wire `style.hyperlinks.color` and `bg-color` to terminal SGR (wrapping display text while preserving OSC 8 boundaries) and HTML `<a>` inline CSS.
- [x] Implement local-style field-by-field merge: `style.hyperlinks.local-style` properties override `style.hyperlinks.*` properties for local references.
- [x] **Validation Checkpoint**: Tests pass for hyperlink color (terminal and HTML boundaries), local hyperlink override, and width/max-width conflict.

## Phase 5: Local Image Style [Parallelizable]

Wire `style.images.local-style` for local image references.

- [x] Implement local reference detection for images: primary `src`/`srcset` is not HTTP(S) and not `data:`.
- [x] Implement conflict detection for `width` and `max-width` under `images.local-style`, returning error if both are present.
- [x] Wire `local-style` into inline CSS for HTML output (`ImageRef::with_style`).
- [x] Wire `local-style` into terminal fallback output (color/bg-color for alt text; width/max-width/alignment for rendered fallback text layout).
- [x] **Validation Checkpoint**: Tests pass for local image override (HTML and terminal) and width/max-width conflict.

## Phase 6: Final Wiring and Cleanup

Integrate bespoke styles into the render pipeline and promote the active wiring phase.

- [x] Wire `apply_bespoke_style` into the CLI render pipeline (`DarkmatterPage` sequence).
- [x] Update `ACTIVE_STYLE_WIRING_SUB_SPEC` to `7`.
- [x] Suppress `KnownButInactive { sub_spec: 7 }` for all keys wired in this sub-spec (`style.page.stylesheet`, `style.page.meta`, `style.page.code.theme`, `style.hyperlinks.*`, `style.hyperlinks.local-style`, `style.images.local-style`).
- [x] Update `darkmatter/docs/rendering/style.md` to document that #7 is live, detailing stylesheet, meta, and code-theme behaviors.
- [x] **Validation Checkpoint**: 0 `KnownButInactive` warnings emitted for any valid v1 schema key. All previous sub-spec tests and new Phase 1-5 tests pass.