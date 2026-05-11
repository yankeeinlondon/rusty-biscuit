---
ready: true
---

# Feature Review: Horizontal Rule (HR) Component

This review evaluates the implementation of the Horizontal Rule feature defined in `darkmatter/features/2026-04-18-hr/`.

## Status: Ready for Production

The implementation is exceptionally solid and meets all criteria established in the specification and technical design.

## Review Highlights

### 1. Functional Completeness
- **All Styles Implemented**: All seven styles (`dashes`, `dots`, `waves`, `line-star`, `line-circle`, `inset-line`, `curtain-rod`) are present and correctly rendered in both terminal and browser targets.
- **Attributes & Customization**: Full support for `alignment`, `weight`, `width`, and `color` is implemented.
- **Precedence Logic**: The resolution order (Per-rule attributes > Page frontmatter > Component defaults) is correctly implemented via `build_rule_with_defaults` in `hr_builder.rs`.
- **Parsing**: `RuleProcessor` robustly handles the `--- { attrs }` extension while maintaining standard Markdown compatibility. It correctly ignores paragraphs with mixed content and honors HRs inside blockquotes.

### 2. Rendering Quality
- **Three-Tier Progressive Enhancement**:
    - **Tier 1 (Image)**: Successfully implemented using `resvg` and `TerminalImage`. It supports Kitty and iTerm2 protocols with proper cell-size detection and alignment.
    - **Tier 2 (Unicode)**: High-quality fallback using appropriate box-drawing and decorative characters.
    - **Tier 3 (ASCII)**: Reliable fallback for legacy environments.
- **HTML/SVG Output**: The browser target generates clean SVG with CSS variable support (`--hr-weight`, `--hr-color`, `--hr-width`), allowing for easy downstream theming.

### 3. Test Coverage
- **Integration Tests**: `darkmatter/lib/tests/horizontal_rule_integration.rs` provides exhaustive coverage for complex documents, frontmatter merging, and specific regressions (e.g., blockquote nesting, width clamping).
- **Unit Tests**: `biscuit-terminal` and `darkmatter` have strong internal tests for component logic and attribute parsing.
- **Snapshot Tests**: Snapshot testing ensures visual consistency across updates.

### 4. Code Quality & Ergonomics
- **Shared Builder**: Moving the HR assembly logic to `darkmatter/lib/src/markdown/block/hr_builder.rs` was a great architectural choice, preventing drift between the terminal and HTML renderers.
- **Validation**: Proper use of `tracing::warn!` for unknown attributes and values ensures that authors get feedback without breaking the render.
- **YAML Parsing**: Transitioning to `serde_yaml_ng` for attribute blocks provides a more robust and standards-compliant authoring experience.

## Suggestions for Future Iterations
- While the current implementation is very performant, if the number of horizontal rules in a document becomes extremely large, the on-the-fly SVG rasterization for Tier 1 might benefit from caching (though currently, this is not a bottleneck).
- Consider adding support for `RuleWeight::Thick` in the `Waves` style for Tier 2 (Unicode) if suitable characters are identified in the future.

## Conclusion
The feature is complete, well-tested, and idiomatically implemented. It is ready for production use.
