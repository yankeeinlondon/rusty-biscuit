---
ready: true
---

# Feature Review #3: YAML Component (2026-04-26)

This is the third pass on `YamlBlock` after review-2's findings were addressed
via `review-plan-2.md`. The implementation is now in good shape: all 34
`yaml_block` unit tests pass, clippy is clean, and every high-severity item
from review-2 has either been fixed in code or reconciled in the spec.

The remaining items below are **minor** — documentation hygiene, performance
nits, and one residual spec/implementation drift that is informational rather
than blocking. None of them should hold the feature back from production.

## 1. Verification of Prior Review Fixes

| Review-2 Item | Severity | Status | Evidence |
|---|---|---|---|
| 1.1 Layout ignored | High | **Fixed** | `yaml_block.rs:211` — `self.layout.apply_layout(&raw, terminal_width)` |
| 1.2 Hardcoded `ThemePair::Github` (terminal) | High | **Fixed** | `yaml_block.rs:173-176` — `TerminalOptions::default()` → `options.code_theme` |
| 1.3 Redundant `detect_color_mode()` | Medium | **Fixed** | `yaml_block.rs:175` — single `options.color_mode` reuse |
| 1.4 No `render_optimistic` override | Low | **Fixed** | `yaml_block.rs:218-222` — explicit override |
| 2.1 Missing header row | High | **Fixed** | `yaml_block.rs:187-193` — `format_header_row` emitted before body |
| 2.2 Light/dark tests do not control mode | High | **Fixed** | `yaml_block.rs:611-679` — three `#[serial]` env-driven tests with `EnvVarGuard` RAII |
| 2.3 `find_syntax` inconsistency between helpers | Medium | **Fixed** | `output/code_block.rs:181-182` — HTML helper now also uses shared `find_syntax` |
| 2.4 Spec/impl divergence on malformed FM error | Low | **Fixed** | `spec.md:80` AC1 now references `MarkdownParse` |
| 2.5 Reserialization-loss undocumented | Low | **Fixed** | `yaml_block.rs:103-120` rustdoc + tests at `:776-805` |
| 3.1 Parity tests too lax | High | **Improved** | `yaml_block.rs:457-514` and `:556-604` — now extract body slice and assert verbatim substring presence; see §3.1 below |
| 3.2 No layout-application test | Medium | **Fixed** | `yaml_block.rs:688-701` — `test_left_margin_is_applied` |
| 3.3 No `render_optimistic` / width smoke tests | Low | **Fixed** | `yaml_block.rs:709-738` |
| 3.4 No `from_yaml_file` ↔ `new` round-trip | Low | **Fixed** | `yaml_block.rs:742-751` |
| 3.5 No key-order preservation test | Low | **Fixed** | `yaml_block.rs:759-770` |
| 5.1 Missing `## Errors` rustdoc | Low | **Fixed** | `yaml_block.rs:66-68` and elsewhere |
| 5.2 Trait-impl docs | Low | **Fixed** | `yaml_block.rs:166-168` and `:241-245` |
| 5.3 README rendering snippet | Low | **Fixed** | `darkmatter/lib/README.md:612-630` |

All 16 review-2 summary-table rows have been addressed. The plan's Phase 4
audit was performed implicitly by the test/clippy runs.

## 2. Residual Concerns

### 2.1 Spec AC6 still says "byte-identical" — Low

**File:** `spec.md:85`

> Rendering a `YamlBlock` with content `X` (terminal) is byte-identical to
> rendering a markdown document containing only a ` ```yaml `-fenced block
> with content `X`, under the same theme.

The implementation **does not** deliver byte-identity, and structurally
cannot. The Markdown path wraps the highlighted body with `wrapper.newline()`
spacing and a trailing `\n\n`, while `YamlBlock` joins with a single `\n` and
applies `Layout`. The parity tests at `yaml_block.rs:457` (terminal) and
`:556` (browser) correctly use **substring containment**, not byte-equality.

This was explicitly flagged in review-2 §2.1 with two options ("emit header"
or "relax the spec"). Phase 1 implemented the header but the spec wording
was never relaxed. AC6 should be reworded along the lines of:

> Rendering a `YamlBlock` with content `X` (terminal) produces the same
> highlighted body bytes as rendering a markdown document containing only a
> ` ```yaml `-fenced block with content `X` under the same theme. The
> Markdown wrapper may add additional outer spacing.

This is a documentation-only fix. The current behaviour is correct and
desirable — only the criterion wording is misleading.

### 2.2 Width source diverges from `as_terminal` — Low (informational)

**Files:** `yaml_block.rs:178`, `output/terminal.rs:792-794`

`YamlBlock::render` uses `term.width()` (a `u32`). Markdown's `as_terminal`
uses `options.max_width.unwrap_or(detected)` (a `u16`). When a caller renders
a `YamlBlock` next to a Markdown YAML fence with `TerminalOptions { max_width: Some(60), .. }`,
the two outputs use different widths. There is no shared knob today.

This is a `Renderable` trait/`as_terminal` API divergence, not a `YamlBlock`
bug — the same divergence applies to every `Renderable` implementor in
`darkmatter` and `biscuit-terminal`. Worth noting in the rustdoc (one line
under the `Renderable` impl) but no code change is required.

### 2.3 `terminal_width as u16` truncation — Low

**File:** `yaml_block.rs:192`

```rust
terminal_width as u16,
```

If a (silly) consumer passes `Terminal::default().width(99999)` the cast
silently wraps to `u16::MAX % 65536`. Same pattern is used inside `terminal.rs`
already, so this is a codebase-wide convention, not a `YamlBlock` regression.
Consider switching to `u16::try_from(terminal_width).unwrap_or(u16::MAX)`
*globally* as a separate cleanup, or leave as-is. Not blocking.

### 2.4 Minor README typo — Low

**File:** `darkmatter/lib/README.md:606`

```rust
/ block.yaml() contains "title: Hello" only
```

Single `/` instead of `//` — this would not compile if executed. Pure
documentation polish.

### 2.5 Redundant validation in `from_markdown_content` — Low (perf)

**File:** `yaml_block.rs:121-133`

```rust
let yaml = if md.frontmatter().is_empty() {
    "{}".to_string()
} else {
    serde_yaml_ng::to_string(md.frontmatter().as_map())?  // (1) serialize
};
validate_yaml(&yaml)?;                                     // (2) re-parse
```

`md.frontmatter()` was already parsed by `Markdown::try_from_content` (via
`parse_yaml_with_fallbacks` in `frontmatter.rs:238`). After step (1) we have
known-valid YAML produced by `serde_yaml_ng::to_string`, so step (2) is
guaranteed to succeed. The double-validation is defensive but wasteful.

Removing the trailing `validate_yaml(&yaml)?;` would shave one parse pass per
call. Acceptable as-is; the cost is small.

### 2.6 `render` constructs `TerminalOptions::default()` per call — Low (perf)

**File:** `yaml_block.rs:173`

`TerminalOptions::default()` performs env-var lookups (NO_COLOR, COLORFGBG,
COLORTERM, etc.) for theme + color-mode detection. In a TUI loop that
re-renders a `YamlBlock` every frame, this is wasted work.

A pragmatic fix would be a `OnceLock<TerminalOptions>` cache or, better, a
future builder (`YamlBlock::with_options(self, TerminalOptions) -> Self`)
that lets callers inject pre-resolved options. This was correctly deferred
from spec scope; it remains future work.

## 3. Test Coverage

### 3.1 Parity tests are now substring-based, not byte-equal

**Files:** `yaml_block.rs:457-514` (terminal), `:556-604` (browser)

The terminal parity test extracts the slice from `"foo: 1"` to end-of-`"bar: 2"`
in the `YamlBlock` plain output and asserts that exact slice appears in the
Markdown plain output. This is strictly stronger than the previous "both
outputs contain ANSI" check and catches the §2.1 header-row class of bug.
However, it doesn't enforce the *full* highlighted body — only the plain
characters. The ANSI escape sequences inside the body could differ silently.

A stronger test would be:
1. Locate the start-of-body padding row in both outputs.
2. Locate the end-of-body padding row in both outputs.
3. Slice the body region (with ANSI) and assert byte-equality.

This is not blocking — the current test is a meaningful improvement and
catches structural drift. Future enhancement.

### 3.2 Missing test gaps

These are nice-to-haves, not gaps that would block release:

- **`from_markdown_file` without frontmatter** — file equivalent of
  `test_from_markdown_content_no_frontmatter`. The file-loading code path
  is trivial (delegates to `from_markdown_content`), so risk of regression
  is low.
- **Empty frontmatter delimiters (`---\n---\n`)** — exercises the
  `parse_frontmatter` empty-yaml path that returns an empty `FrontmatterMap`,
  distinct from "no frontmatter delimiters at all." Probably yields the same
  `{}` payload, but worth one explicit assertion.
- **YAML with diverse data types** (lists, nulls, booleans, anchors) — the
  current tests only cover scalar maps. `serde_yaml_ng` handles the rest, so
  marginal value.
- **Multi-byte UTF-8 in YAML keys/values** — not currently exercised; the
  `format_header_row` byte/char-count math could surprise on multi-byte
  content. Low risk because the body itself is rendered through `syntect`.

### 3.3 Test isolation is good

The three env-driven tests (`test_dark_mode_via_colorfgbg`,
`test_light_mode_via_colorfgbg`, `test_dark_and_light_render_differ`) all
use `#[serial]` and an `EnvVarGuard` RAII helper for deterministic restore
on failure. This is the correct pattern.

## 4. Ergonomics & Performance

### 4.1 Public API surface is appropriate

Constructors split by argument kind (`Into<String>` for content,
`AsRef<Path>` for files), validation at construction time, and no leakage
of `serde_yaml_ng::Value`. Matches monorepo precedent.

### 4.2 Trait integration is clean

`Renderable::render` honours layout and chains into the shared helpers.
`BrowserRenderable::render_to_browser` does the same on the HTML side.
`as_any` is implemented for both traits. `is_block_level()` returns `true`,
which is correct.

### 4.3 Future builder hooks

The spec explicitly defers theme/title overrides. The current API is
intentionally minimal — no `with_theme`, `with_title`, etc. This is the
right call for v1; future additions are non-breaking.

## 5. Documentation

### 5.1 Rustdoc on public API is thorough

Every public constructor has `## Examples` and `## Errors` sections.
`from_markdown_content` has a detailed `## Notes` block calling out the
reserialization caveats. The trait-impl headers explain what each `render`
method emits.

### 5.2 README has a complete lifecycle example

`darkmatter/lib/README.md:614-630` shows construction → terminal render →
browser render. Good for onboarding.

### 5.3 SKILL.md has a `YamlBlock` section

Visible in the loaded `darkmatter` skill output — covers constructors,
validation, rendering, and limitations. Aligned with the implementation.

## 6. Summary Table

| # | Severity | Issue | Blocking? |
|---|----------|-------|-----------|
| 2.1 | Low | Spec AC6 says "byte-identical" but implementation isn't | No (doc-only fix) |
| 2.2 | Low | Width source diverges from `as_terminal` (architectural) | No |
| 2.3 | Low | `terminal_width as u16` silent truncation | No (codebase-wide) |
| 2.4 | Low | README typo: single `/` on line 606 | No |
| 2.5 | Low | Redundant `validate_yaml` after `to_string` | No (perf) |
| 2.6 | Low | `TerminalOptions::default()` re-detected per render call | No (perf) |
| 3.1 | Low | Parity tests are substring-based, not full byte-equal | No |
| 3.2 | Low | A few "nice-to-have" test gaps | No |

No high or medium severity issues remain.

## 7. Conclusion

The third review pass confirms that **all of review-2's high-severity items
have been resolved in code**. The implementation:

- Honours the stored `Layout` (margins, alignment).
- Routes theme and color-mode through `TerminalOptions::default()`.
- Emits the same header row Markdown YAML fences emit.
- Has real env-driven light/dark tests gated by `serial_test`.
- Uses the shared `find_syntax` helper consistently across terminal and HTML.
- Preserves frontmatter key order via `IndexMap` round-trip and documents the
  reserialization caveats with both rustdoc and tests.

What remains is a small punch-list of low-severity items: a one-line spec
rewording for AC6, a single README typo, and a handful of optional
performance/perfection nits that can be deferred.

The feature is ready for production.

**Status:** `ready: true`

**Suggested follow-ups (non-blocking, in priority order):**
1. Reword spec AC6 to drop "byte-identical" in favor of "highlighted-body
   bytes appear verbatim, modulo Markdown wrapper spacing." (`spec.md:85`)
2. Fix the single-slash comment typo in `darkmatter/lib/README.md:606`.
3. Drop the redundant `validate_yaml(&yaml)?;` after `to_string` in
   `from_markdown_content` (`yaml_block.rs:128`).
4. Add a `from_markdown_file` no-frontmatter test for symmetry with the
   content-based test (`yaml_block.rs:345`).
5. (Future) Consider a `YamlBlock::with_options(TerminalOptions) -> Self`
   builder so TUI loops can avoid re-detecting on every render.
