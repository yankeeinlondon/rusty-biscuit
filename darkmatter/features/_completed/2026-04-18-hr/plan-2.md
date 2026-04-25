---
phases: 5
created: 2026-04-23
---

# Horizontal Rule Correction Plan

This plan corrects the current HR implementation so it matches the intended authoring model: standard Markdown HR markers stay valid, page-level `hr` frontmatter provides defaults, per-rule attributes provide overrides, `alignment` is the canonical positioning key, and capable terminals use image rendering before text fallbacks.

## Phase 1: Reconcile Data Model and Terminology

### Steps

1. Use `RuleAlignment` as the public positioning enum in `biscuit-terminal`.
2. Use `alignment` for builder methods, fields, parsing structs, and docs.
3. Remove legacy positioning terminology from the public API and parser surface.
4. Update tests to assert `alignment` behavior.

### Validation Checkpoints

- `alignment` is the canonical key in code, docs, examples, snapshots, and skill files.
- Unknown positioning keys are treated like any other unknown-key input.

## Phase 2: Add Frontmatter Defaults

### Steps

1. Add an `hr` frontmatter configuration struct to Darkmatter, with optional `style`, `alignment`, `weight`, `width`, and `color` fields.
2. Parse `hr` from page frontmatter before terminal/HTML rendering.
3. Thread the page-level HR configuration into terminal and HTML renderers.
4. Resolve effective options per rule with this precedence: per-rule attribute block, page frontmatter `hr`, component default.
5. Validate frontmatter values with the same validation path used for per-rule overrides.
6. Add tests for bare `---`, `___`, and `***` using frontmatter defaults.

### Validation Checkpoints

- A document with only frontmatter `hr:` and bare CommonMark HR markers renders styled rules.
- A per-rule attribute overrides only the specified keys and inherits the rest from frontmatter.
- Missing `hr` frontmatter preserves current default rendering.

## Phase 3: Restore Image Rendering for Capable Terminals

### Steps

1. Re-enable or implement the Tier 1 path in `biscuit-terminal::HorizontalRule`.
2. Use the existing `resvg -> tiny_skia -> TerminalImage` pattern already used elsewhere in `biscuit-terminal`.
3. Gate Tier 1 on terminal image capability detection, specifically Kitty-compatible image support.
4. Verify WezTerm is detected through the same capability abstraction rather than by hard-coded terminal-name branching where possible.
5. Ensure generated PNG sizing respects resolved `width`, `alignment`, `weight`, and terminal cell dimensions.
6. Add fallback tests proving image failure falls back to Unicode, then ASCII.

### Validation Checkpoints

- WezTerm sessions with Kitty-compatible image support use the image path.
- Unicode output is used only when image rendering is unavailable.
- ASCII output remains the final fallback.
- Image rendering handles all supported styles, especially `waves`.

## Phase 4: Parser and Renderer Integration

### Steps

1. Keep bare `---`, `___`, and `***` on their own line as normal CommonMark HR blocks.
2. Continue supporting the Darkmatter extension form `--- { ... }` for per-rule overrides.
3. Update the HR attribute parser to prefer `alignment`.
4. Map resolved Darkmatter HR settings into `biscuit-terminal::HorizontalRule` once, then use that shared path for terminal and HTML output.
5. Add integration tests for standard HR markers, frontmatter defaults, per-rule overrides, unknown keys, and invalid values.

### Validation Checkpoints

- Standard Markdown HR syntax does not require an attribute block to receive configured styling.
- Attribute-block syntax still works for specific HR overrides.
- Terminal and HTML renderers agree on resolved effective configuration.

## Phase 5: Documentation and Skill Updates

### Steps

1. Update `darkmatter/docs/topics/horizontal-rules.md` to document frontmatter-first styling, per-rule overrides, `alignment`, validation behavior, and terminal rendering tiers.
2. Update `biscuit-terminal/docs/components/horizontal-rule.md` to document `RuleAlignment`, image-first progressive enhancement, and fallback behavior.
3. Update `biscuit-terminal/docs/components/browser-renderable-trait.md` if trait behavior or examples changed while fixing browser rendering.
4. Update `.claude/skills/darkmatter/SKILL.md` so future agents know that `hr` frontmatter is the preferred page-level configuration mechanism.
5. Update `.claude/skills/biscuit-terminal/SKILL.md` so future agents know `HorizontalRule` should use image rendering on capable terminals before Unicode/ASCII fallback.
6. Remove or correct references that describe Tier 1 image rendering as deferred.

### Validation Checkpoints

- Author-facing docs no longer imply that styling requires invalid/non-standard Markdown suffixes.
- Skill files match the implemented architecture and current terminology.
- Examples use `alignment`.

## Suggested Verification Commands

```bash
cargo test -p biscuit-terminal horizontal_rule
cargo test -p darkmatter horizontal_rule
cargo test -p darkmatter hr_frontmatter
cargo fmt -p biscuit-terminal -p darkmatter
cargo clippy -p biscuit-terminal -p darkmatter --all-targets -- -D warnings
```
