---
ready: true
---

# Feature Review #2: YAML Component (2026-04-26)

The shared code-block helper refactor is solid and the public API matches the spec/design well. However, **review-1's four primary concerns have not actually been addressed in the code** despite the cover note saying they were, and several additional gaps surfaced during this pass — including a violated acceptance criterion that the existing parity tests fail to detect.

`darkmatter/lib/src/markdown/yaml_block.rs` has not been modified since the initial implementation commit (`554700fb`). The follow-up commits add tests and docs but do not touch the `Renderable` / `BrowserRenderable` impls.

## 1. Review-1 Regressions (Still Outstanding)

### 1.1 Layout is still ignored — `Renderable` trait contract violation
**File:** `darkmatter/lib/src/markdown/yaml_block.rs:144`

```rust
fn render(&self, _term: &Terminal) -> String {
```

The `_term` underscore is a giveaway: terminal width and capabilities are unused, and the stored `Layout` is never applied. Compare `TextBlock::render` (`biscuit-terminal/lib/src/components/text_block.rs:158-162`):

```rust
fn render(&self, term: &Terminal) -> String {
    let width = term.width();
    let content = self.to_terminal(term);
    self.layout.apply_layout(&content, width)
}
```

**Impact:** Builder methods like `.left_margin(Margin::Chars(4))`, `.alignment(Alignment::Center)`, `.right_margin(...)` silently no-op. Any consumer relying on `Renderable` layout composition will get incorrect output.

**Fix:**
```rust
fn render(&self, term: &Terminal) -> String {
    let raw = render_terminal_code_block(...).unwrap_or_else(...);
    self.layout.apply_layout(&raw, term.width())
}
```

### 1.2 Hardcoded `ThemePair::Github` — breaks acceptance criterion 6
**Files:** `yaml_block.rs:146` (terminal) and `yaml_block.rs:177` (browser)

```rust
let highlighter = CodeHighlighter::new(ThemePair::Github, color_mode);
```

Markdown YAML fences pull `code_theme` from `TerminalOptions` (`output/terminal.rs:797`), which auto-detects via `detect_code_theme(prose_theme)`. YamlBlock hardcodes Github, so under any non-Github environment the YamlBlock and a `\`\`\`yaml` fence produce different ANSI output — directly violating spec acceptance criterion 6.

**Fix (terminal path):**
```rust
let options = TerminalOptions::default();
let highlighter = CodeHighlighter::new(options.code_theme, options.color_mode);
```

The browser path is fine to leave as Github *only because* `HtmlOptions::default()` also hardcodes Github (`output/html.rs:95`) — but routing through the options struct keeps both paths symmetric and resilient if `HtmlOptions::default()` ever gains auto-detection.

### 1.3 Redundant `detect_color_mode()` invocation
**File:** `yaml_block.rs:145, 147`

```rust
let color_mode = detect_color_mode();              // call #1
let options = TerminalOptions::default();          // call #2 (inside Default::default)
```

Reuse the value already inside `options`:
```rust
let options = TerminalOptions::default();
let color_mode = options.color_mode;
let highlighter = CodeHighlighter::new(options.code_theme, color_mode);
```

The browser path has the same redundancy at lines 176/178 (although `HtmlOptions::default()` doesn't currently call `detect_color_mode()`, the symmetry argument applies).

### 1.4 No explicit `render_optimistic` override
The default impl works but cannot improve since `render(&self, _term)` ignores `term`. After fixing 1.1, the default `render_optimistic` will work correctly without an override — but consider a one-line override for clarity.

## 2. New Gaps (Not Found in Review-1)

### 2.1 Acceptance criterion 6 ("byte-identical") is structurally violated and untested
**File:** `output/terminal.rs:1058-1090` vs. `yaml_block.rs:144-155`

The Markdown code-fence path emits a **header row** (title + right-aligned language label) *before* calling `highlight_code`:

```rust
let header = format_header_row(meta.title.as_deref(), &code_language, ...);
wrapper.push_with_newlines(&header);
wrapper.newline();
let highlighted = highlight_code(...);
```

`YamlBlock::render` skips the header row entirely. So the markdown ` ```yaml ` fence shows a "yaml" label gutter and YamlBlock does not — these outputs cannot be byte-identical. The parity test (`test_terminal_render_parity_with_markdown_yaml_fence`) only checks that both outputs contain `\x1b[` and the YAML keys, which masks this divergence.

**Decisions to make:**
- Either emit a header row in `YamlBlock::render` (preserving AC6), or
- Update the spec to drop "byte-identical" in favor of "structurally identical code-block body" (and tighten the test to assert the body bytes match).

Either way, the parity tests should be hardened to do a direct byte comparison of the body region — not a substring spot-check.

### 2.2 Acceptance criterion 8 is *not actually* exercised
**Tests:** `test_terminal_render_with_light_color_mode` and `test_terminal_render_with_dark_color_mode` (lines 406-431)

These two tests are functionally identical — both call `Renderable::render(&block, &term)` and assert ANSI presence + YAML content. Neither test controls the detected color mode; both rely on `detect_color_mode()` reading the ambient environment. AC8 says "each [light and dark] have at least one passing test exercising `themes.rs::detect_color_mode()` selection."

**Fix:** Use `serial_test` (already in `tempfile`-tier dev deps?) and explicitly manipulate `COLORFGBG` / `NO_COLOR`:
```rust
#[test]
#[serial_test::serial]
fn test_dark_mode_via_colorfgbg() {
    std::env::set_var("COLORFGBG", "15;0");
    let detected = detect_color_mode();
    assert_eq!(detected, ColorMode::Dark);
    // ...also render and assert theme-bg colors differ from light path
    std::env::remove_var("COLORFGBG");
}
```

Verify `serial_test` is on the dependency graph; if not, add it as a dev-dep (it's used widely in `homelab` and `claudine`).

### 2.3 `find_syntax` inconsistency between terminal and HTML helpers
**Files:** `output/code_block.rs:49` vs. `output/code_block.rs:184-188`

- Terminal helper calls `find_syntax(language, ...)` — comprehensive lookup with alias map (`shell` → `bash`, `c++` → `cpp`, etc.)
- HTML helper calls `syntax_set.find_syntax_by_token(language)` directly — narrower fallback semantics

For `language="yaml"` both work (yaml is a known token), but the divergence will surface for any consumer using `YamlBlock`-adjacent code blocks, and it violates the "shared helper" promise the design articulates.

**Fix:** Have `render_html_code_block` call `find_syntax(...)` like the terminal path does.

### 2.4 Spec ↔ implementation divergence on malformed-frontmatter error
**Spec line 37:** "Malformed frontmatter ⇒ `YamlBlockError::YamlParse` when the frontmatter block is present but is not valid YAML."

**Implementation:** Returns `YamlBlockError::MarkdownParse` (because `Markdown::try_from_content` wraps malformed YAML in `MarkdownError::FrontmatterParse`, which `#[from]` maps to `YamlBlockError::MarkdownParse`).

The tech-design (lines 99-103) explicitly diverges from the spec on this point. The implementation matches the tech-design. This is a documentation fix, not a code fix — but the **spec is currently wrong** and should be reconciled before signing off.

### 2.5 `from_markdown_content` reserialization edge cases
**File:** `yaml_block.rs:103`

```rust
serde_yaml_ng::to_string(md.frontmatter().as_map())?
```

The frontmatter is parsed by `parse_yaml_with_fallbacks` (frontmatter.rs:211), which can succeed on YAML containing tabs after normalization. Re-serialization via `serde_yaml_ng::to_string` will then emit canonical YAML — different from the source. The tech-design accepts this loss of formatting, but two cases need testing:
- **Original frontmatter with a comment** — comment is silently dropped on re-serialization.
- **Original frontmatter with `!tag` or anchors** — round-trips through `serde_json::Value` (the `FrontmatterMap` value type), losing tag/anchor information.

Either the doc-comment on `from_markdown_content` should warn about this, or there should be a test asserting the lossy behavior so it doesn't regress.

## 3. Test Coverage Gaps

### 3.1 Parity tests are too lax
Both `test_terminal_render_parity_with_markdown_yaml_fence` (line 331) and `test_browser_render_parity_with_markdown_yaml_fence` (line 382) only assert that both outputs contain ANSI/`language-yaml` and a couple of YAML keywords. Neither does byte comparison or even line-count comparison. Given §2.1 (header row missing), these tests pass while AC6 is violated.

**Recommendation:** Extract the body bytes (skip header row, padding rows) and assert byte equality. Or, less invasively, assert that the full markdown-fence output **contains** the YamlBlock output as a contiguous substring.

### 3.2 No layout-application test
There's no test of the form:
```rust
let block = YamlBlock::new("foo: 1").unwrap()
    .left_margin(Margin::Chars(4));
let output = block.render(&Terminal::default());
assert!(output.lines().all(|l| l.starts_with("    ") || l.is_empty()));
```

Such a test would have caught §1.1 immediately.

### 3.3 No coverage for `render_in_width` / `render_optimistic`
Trait-default methods deserve at least a smoke test — particularly because `render` ignoring `term` means width overrides have no effect. A failing test here would document the limitation explicitly.

### 3.4 No assertion that `from_yaml_file` and `new` produce identical state
A round-trip test (`new("foo: 1")` vs `from_yaml_file(file_with("foo: 1"))`) would lock in the spec invariant that file ingestion is just `new(read_to_string(path))`.

### 3.5 No coverage for `from_markdown_content` reserialization preserving key order
The spec emphasizes `IndexMap` is used to preserve order, but no test verifies that `from_markdown_content("---\nb: 1\na: 2\n---")` produces YAML where `b` appears before `a`. Add one.

## 4. Ergonomics & Performance

### 4.1 Construct options once per render call
`TerminalOptions::default()` performs environment detection (theme, color mode) on each call. For a `YamlBlock` re-rendered in a TUI loop this is wasteful. Either:
- Accept an injected `TerminalOptions` (breaking the existing API) — not recommended for AC compatibility, or
- Cache the detected defaults via `OnceLock` keyed on environment changes.

The simplest pragmatic fix: just remove the redundant `detect_color_mode()` call and accept the once-per-render cost.

### 4.2 Consider exposing a `with_options` or `with_theme` builder
The current API is intentionally minimal (matches spec scope), but a single `pub fn with_theme(self, ThemePair) -> Self` builder would address review-1's concern about the hardcoded theme without breaking any existing call. This is consistent with the precedent set by `HorizontalRule::with_color`, etc.

### 4.3 Layout default — is `Layout::default()` really right for a code block?
`Layout::default()` gives full-width, no margins, left-aligned. For a `YamlBlock` embedded inside a list or table, the parent's offset won't propagate without explicit `with_parent_layout`. Worth a doc-comment note in `YamlBlock`'s rustdoc summary explaining how to nest.

## 5. Documentation Polish

### 5.1 `YamlBlock::new` rustdoc lacks `## Errors` section
**File:** `yaml_block.rs:65-72`

The existing doc-comment shows an example but no `## Errors` heading. Per the project rustdoc convention (`CLAUDE.md`):

> ## Errors (if applicable)

Add it explicitly:
```rust
/// ## Errors
///
/// Returns [`YamlBlockError::YamlParse`] if the input fails `serde_yaml_ng` parsing.
```

### 5.2 Trait impls should document any deviation from defaults
After fixing layout application, a one-line note on the `Renderable` impl explaining "respects margins via `Layout::apply_layout`" prevents the next reviewer from re-flagging this.

### 5.3 README example shows constructors but not rendering
`darkmatter/lib/README.md:590-620` constructs `YamlBlock` four ways but never demonstrates `block.render(&Terminal::default())` or `block.render_to_browser()`. Add at least one rendering snippet so readers see the full lifecycle.

## 6. Summary Table

| # | Severity | Issue | Acceptance Criterion |
|---|----------|-------|----------------------|
| 1.1 | High | Layout ignored in `render` | n/a (trait contract) |
| 1.2 | High | Hardcoded `ThemePair::Github` | AC6 |
| 1.3 | Medium | Redundant `detect_color_mode()` | none |
| 1.4 | Low | No explicit `render_optimistic` | none |
| 2.1 | High | Header row missing — terminal parity broken | AC6 |
| 2.2 | High | Light/dark mode tests do not control mode | AC8 |
| 2.3 | Medium | `find_syntax` inconsistency between helpers | AC6 (indirect) |
| 2.4 | Low | Spec/impl divergence on malformed frontmatter | AC1 (spec doc fix) |
| 2.5 | Low | Reserialization-loss edge cases undocumented | none |
| 3.1 | High | Parity tests too lax to catch §2.1 | AC6, AC7 |
| 3.2 | Medium | No layout-application test | AC4 (indirect) |
| 3.3 | Low | No `render_in_width` / `render_optimistic` smoke tests | none |
| 4.1–4.3 | Low | Performance / ergonomics nits | none |
| 5.1–5.3 | Low | Rustdoc / README polish | none |

## Conclusion

Three of the four review-1 concerns remain unfixed in code (only review-1's "render_optimistic" point is arguably defensible to leave alone). On top of that, **acceptance criterion 6 is structurally broken** (header row missing) and **acceptance criterion 8 is not actually verified** by the current tests. The implementation also has an inconsistency between the terminal and HTML branches of the shared helper module that the design explicitly meant to prevent.

This feature is not ready for production. After fixes, the highest-leverage follow-up is **strengthening the parity tests to do real byte comparison** so future regressions like §2.1 can't slip through.

**Status:** `ready: false`

**Minimum to-merge bar:**
1. Apply layout in `render` (§1.1)
2. Use `options.code_theme` instead of hardcoded `Github` (§1.2)
3. Decide on header-row policy and update spec/code accordingly (§2.1)
4. Replace the light/dark tests with real `serial_test`-gated env manipulation (§2.2)
5. Tighten parity tests to byte/structural comparison (§3.1)
6. Reconcile spec line 37 with the tech-design's `MarkdownParse` mapping (§2.4)

---

Addressed by review-plan-2.md, Phases 1–4. All summary-table rows resolved.
