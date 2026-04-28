---
ready: false
---

# Feature Review: YAML Component (2026-04-26)

The implementation of the YAML component is technically sound and follows the technical design closely. The refactoring of the code-block rendering into shared helpers is a significant improvement for maintainability and consistency across the codebase.

However, there are a few architectural gaps and inconsistencies that should be addressed before the feature is considered production-ready.

## Gaps in Functionality & Implementation

### 1. `Renderable` Layout Ignored
`YamlBlock` implements the `Renderable` trait from `biscuit-terminal`, which requires implementors to own and respect a `Layout`. While `YamlBlock` owns a `Layout`, it is completely ignored in the `render` method.

**Recommendation:**
The `render` method should apply the layout to the rendered output:
```rust
fn render(&self, term: &Terminal) -> String {
    // ... rendering logic ...
    let raw = render_terminal_code_block(...).unwrap_or_else(...);
    self.layout.apply_layout(&raw, term.width())
}
```

### 2. Hardcoded Theme (Consistency Issue)
In both `render` and `render_to_browser`, `ThemePair::Github` is hardcoded when creating the `CodeHighlighter`. However, `TerminalOptions::default()` (and `HtmlOptions`) already performs auto-detection of the preferred theme.

**Recommendation:**
Use the theme defined in the options to ensure consistency with the rest of the application's configuration:
```rust
let options = TerminalOptions::default();
let highlighter = CodeHighlighter::new(options.code_theme, color_mode);
```

### 3. Redundant Color Mode Detection
`detect_color_mode()` is called manually in `YamlBlock::render` and then called again internally by `TerminalOptions::default()`.

**Recommendation:**
Reuse the `color_mode` from the options:
```rust
let options = TerminalOptions::default();
let color_mode = options.color_mode;
let highlighter = CodeHighlighter::new(options.code_theme, color_mode);
```

### 4. Missing `render_optimistic` Implementation
While the default implementation of `render_optimistic` works, it is good practice for components in this repository to provide an explicit implementation that matches the behavior of other `Renderable` components, specifically ensuring layout is applied.

## Performance & Ergonomics

### 1. Shared Helper API
The `render_terminal_code_block` and `render_html_code_block` helpers are well-extracted. One minor ergonomic improvement would be to have `render_terminal_code_block` accept `Option<&TerminalOptions>` and use a default if not provided, though the current API is clear for internal use.

### 2. Frontmatter Reserialization
The decision to reserialize frontmatter in `from_markdown_content` is consistent with the design but it does mean that original formatting (comments, whitespace) in the markdown frontmatter is lost. This is acceptable given the "structured data" focus of Darkmatter, but should be kept in mind if users ever request "raw" frontmatter rendering.

## Test Coverage

Test coverage is excellent, including parity tests with Markdown code fences.

**Suggestion:**
To truly fulfill Acceptance Criterion 8 ("Light-mode rendering and dark-mode rendering each have at least one passing test exercising `themes.rs::detect_color_mode()` selection"), the tests should ideally use `serial_test` and temporarily modify environment variables (`NO_COLOR` or `COLORFGBG`) to verify that the detection logic actually picks the correct mode.

## Conclusion

The feature is very close to completion. Once the `Layout` application is fixed and the theme consistency is addressed, it will be ready for production.

**Status:** `ready: false` (Pending fixes for Layout and Theme consistency)
