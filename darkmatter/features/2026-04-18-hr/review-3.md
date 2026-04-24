---
review: 3
reviewed: 2026-04-23
reviewer: claude-opus-4-7
feature: horizontal-rule
ready: false
packages:
  - biscuit-terminal
  - darkmatter
test_status: failing
---

# Review 3 — Horizontal Rule Component

Review 2 signed off as "READY" on the premise that all prior review feedback had been implemented. Picking the work up fresh, most of that is true: Tier 1 image rendering, weight-aware Unicode, ANSI color wrap, CSS-variable SVG, prelude exports, YAML flow-mapping attribute parsing, frontmatter `hr:` defaults, bare-marker handling, shared outer `Terminal`, and layout-aware margins are all present with strong tests.

However, the feature as it currently stands is **not production-ready** — the snapshot suite fails, one class of public-facing documentation still advertises the pre-Tier-1 behavior, and a few of the validation guarantees promised by `spec.md` §"Validation Requirements" are not actually enforced.

## `ready` verdict

**Not ready.** Merging in the current state ships a red test (`test_snapshot_complex_document`) and docs that contradict the code. Everything else below is high-quality polish or minor gaps — fixing #1 and #2 alone is enough to move this to READY.

---

## Category A — Blockers

### A1. `test_snapshot_complex_document` fails (stale snapshot after `placement`→`alignment` rename)

`cargo test -p darkmatter --test horizontal_rule_snapshots` fails on `test_snapshot_complex_document`. The test input at `darkmatter/lib/tests/horizontal_rule_snapshots.rs:70` now reads `## Alignment Options`, but the stored `.snap` file at
`darkmatter/lib/tests/snapshots/horizontal_rule_snapshots__tests__terminal_complex_document.snap:15`
still contains `Placement Options` (the pre-rename term). The HTML companion snapshot
`horizontal_rule_snapshots__tests__html_complex_document.snap` has the same drift.

```
-␛[38;2;220;223;228m██ ␛[0m␛[38;2;220;223;228mPlacement␛[0m ␛[38;2;220;223;228mOptions␛[0m
+␛[38;2;220;223;228m██ ␛[0m␛[38;2;220;223;228mAlignment␛[0m ␛[38;2;220;223;228mOptions␛[0m
```

**Impact:** CI red, user trust red.

**Fix:** `cargo insta review` (or delete the `.snap` and re-record). The content under the heading is otherwise unchanged — this is purely the section title rename the spec adopted.

### A2. `biscuit-terminal/docs/components/horizontal-rule.md` says Tier 1 is deferred — it isn't

Lines 173–175 of the public component doc still carry the Review-1 "Deferred" section:

> **Tier 1 (SVG → PNG via `resvg` + `TerminalImage`) is not yet implemented.**

But `horizontal_rule.rs:268-316` now ships `render_image_tier()`, routed from `Renderable::render` at line 178, and the test `test_render_uses_kitty_image_tier_when_supported` confirms it emits Kitty graphics escapes when `image_support == Kitty`. The `SKILL.md` and `darkmatter/docs/topics/horizontal-rules.md` already describe the three-tier progressive enhancement correctly. Only the component doc is stale.

**Fix:** Replace the "Deferred" section with a short "Tier 1 image rendering" section describing the `ImageSupport::Kitty` + `is_tty` gate, the SVG-to-PNG rasterization via `resvg`, and the fallback to Tier 2/3 when rasterization fails or capabilities are absent. Also update the top of the "Renderable (Terminal Rendering)" section (which already lists Tier 1 correctly) so the document is internally consistent.

---

## Category B — Spec-level gaps

### B1. Invalid `width` values are silently accepted (violates spec §"Validation Requirements" and §"Width")

`horizontal_rule.rs:234-266` (`resolve_width`): when the width string is unparseable (e.g., `"abc"`, `"200px"`, `"10em"`, `"full"`, leftover garbage), the function silently returns `term_width` with no `tracing::warn!`. The spec is explicit:

> `spec.md:92` — *Invalid or unsupported widths should fall back to the component default and emit a diagnostic warning.*

Verified empirically: `resolve_width(Some("200px"), 100)` returns `100` and `resolve_width(Some("abc"), 100)` returns `100` with no signal either way. No unit test exercises the invalid-width path.

**Impact:** Two connected problems:
- Authors get no feedback that they typo'd a width, so the rule looks "fine" (full width) instead of flagging the mistake.
- `"200px"` is explicitly listed as a supported width format in both
  `spec.md:62`, `.claude/skills/darkmatter/SKILL.md`, and `biscuit-terminal/docs/components/horizontal-rule.md`
  — so the silent fallback is a documented-vs-actual drift.

**Fix:** Add a fallthrough that emits `tracing::warn!(width = %width_str, "unrecognized horizontal rule width; falling back to terminal width")` when every parse attempt fails. Add a test `test_resolve_width_invalid_warns_and_falls_back` with a `#[tracing_test::traced_test]` assertion.

### B2. `"200px"` / other CSS length units are undocumented no-ops for terminal output

Tied to B1: `"75%"`, `"20ch"`, and bare numbers (`"50"`) are the only terminal-side formats actually implemented. `"200px"`, `"10em"`, `"50vw"` all fall through and yield full terminal width. Browser output on the other hand passes `"200px"` through the SVG `width` attribute verbatim, so a single markdown source renders wildly differently on the two targets (see also Review-2 §2.2).

**Fix:** Pick one of:
- (preferred) Accept `"NNNpx"` by stripping the `px` suffix and using the numeric value as a column count on narrow displays *or* converting through `term.cell_size()` width when available; then add a unit test.
- Narrow the documented width options in `spec.md`, SKILL, and `horizontal-rule.md` to `%`, `ch`, and bare integers; delete `"200px"` from the examples.

### B3. Unquoted numeric frontmatter silently disables every `hr` default

`hr_builder.rs:93-106`:

```rust
match md.frontmatter().get::<HorizontalRuleAttrs>("hr") { … Err(err) => { warn!(…); None } }
```

Because `HorizontalRuleAttrs` fields are all `Option<String>`, a YAML frontmatter like

```yaml
hr:
  width: 50
```

fails to deserialize (number ≠ string), emits a warn, and returns `None` — losing **every** sibling key (`style`, `alignment`, `weight`, `color`). The RuleProcessor attribute path already handles this via `yaml_value_as_string` which coerces numbers/bools to strings (see `rule_processor.rs:211-218`), so the two code paths are inconsistent.

A typical author mistake:

```yaml
hr:
  style: dots
  width: 20    # forgot the quotes
  color: red
```

Current behavior: `style: dots` and `color: red` are both dropped on the floor.

**Fix:** Either (a) write a custom `Deserialize` for `HorizontalRuleAttrs` that coerces numbers/bools to strings, or (b) read the raw `serde_json::Value` and reuse the attribute-parsing string-coercion path. Add a regression test with an unquoted numeric `width` in frontmatter.

### B4. `render()` applies a hidden 10-char minimum width

`horizontal_rule.rs:187`: `let rule_width = rule_width.clamp(10, term_width);`

- `width: "5%"` on an 80-column terminal returns 4 from `resolve_width`, then gets clamped to 10.
- `width: "3"` gets bumped to 10.
- `width: "1ch"` gets bumped to 10.

This is undocumented, not tested at the boundary, and can't be opted out of.

**Fix:** Either delete the 10-char minimum (minimum 1 is already enforced inside `resolve_width` for the percentage branch) or document it and consider a builder method like `min_width()` for overriding it. Either way, document the behavior near the `HorizontalRule` rustdoc and add boundary tests at `width: "1"` / `width: "5%"` on a 100-column terminal.

### B5. `ImageSupport::ITerm` skips Tier 1

`horizontal_rule.rs:269`: `if !term.is_tty || !matches!(term.image_support, ImageSupport::Kitty)` — only `Kitty` triggers the image path. Terminals that biscuit-terminal identifies as `ImageSupport::ITerm` fall through to Unicode despite having image support.

The `spec.md:124` text says "Kitty-compatible image support", and in practice modern iTerm2 is detected as `Kitty` per the biscuit-terminal notes. But the `ImageSupport::ITerm` arm still exists in the enum and in real detection code — this will bite anyone whose terminal gets classified that way.

**Fix (low-risk):** broaden the guard to
`matches!(term.image_support, ImageSupport::Kitty | ImageSupport::ITerm)`
and, if the two protocols emit different escape sequences, add a separate branch that uses `TerminalImage::render_iterm_cells` (or equivalent). Add a test covering the ITerm case. If we deliberately want Kitty-only, update the spec wording and add a rustdoc comment explaining why.

---

## Category C — Test coverage gaps

Most gaps called out in Review-1 have been closed. What's still missing:

### C1. No test for invalid widths (ties to B1)

No test asserts that `width: "200px"` or `width: "garbage"` produces a warning and a default. Adding one naturally covers the fix for B1.

### C2. No test for unquoted-number frontmatter values (ties to B3)

No integration test covers the common `hr: { width: 50 }` footgun. Add a test ensuring all siblings survive a number-valued scalar.

### C3. Image tier failure-path is only covered by "happy / absent" — no rasterization failure test

`render_image_tier` returns `None` if `rasterize_svg_to_png` errors. There's no test that a malformed SVG (e.g., degenerate geometry) triggers the fallback path. Mocking this is fine in biscuit-terminal since the rasterization helper is free-standing.

### C4. CurtainRod + Thick + RuleAlignment::Right is not individually asserted

The 36-tuple exhaustive sweep (`test_all_styles_all_alignments_all_weights_unicode`) only asserts `!result.is_empty()`. The snapshot test (`test_snapshot_render_all_styles`) covers every combination but doesn't fail fast when a specific combination regresses — a broken variant would just update its individual snapshot on `cargo insta accept`. Consider adding one or two "hard" assertions at the boundary combinations.

### C5. No test for `test_horizontal_rule_inside_blockquote_is_currently_transformed` behavior being the intended spec

The test pin (`rule_processor.rs:756-790`) explicitly says "this test is the canary that will force an update to `RuleProcessor` and to the darkmatter skill" if the decision changes. The spec doesn't rule on this case. Either add it to the spec ("HR-like text inside a blockquote becomes a HorizontalRule event") or carve it out of the current pin.

---

## Category D — Ergonomics / correctness polish (non-blocking)

### D1. `HorizontalRule` doesn't implement `PartialEq` even though all three enums do

Small ergonomic loss — callers can't `assert_eq!(rule_a, rule_b)` in tests even when every field is comparable. Layout/Margin support it too, so this should be derivable.

### D2. `use_fancy_chars` takes `_term: &Terminal` but never reads it

`horizontal_rule.rs:605-607`: the parameter is unused. Either drop it or, cleaner, add a `supports_unicode: bool` capability to `Terminal` (mirroring `osc_link_support`, `supports_italic`, etc.) and consult it here. Today's fallback to `env_says_utf8()` is correct but conflates "environment says UTF-8" with "this specific terminal can render `╌`".

### D3. `HtmlOptions.hr_css_variables: Option<HashMap>` is awkward

The option is `None`-vs-`Some(HashMap)` where an empty `HashMap` already means "no overrides" (the html pipeline at `html.rs:464-468` checks `!map.is_empty()`). Consider making it a bare `HashMap<String, String>` with `Default::default() == HashMap::new()`. Avoids two code paths meaning the same thing and simplifies callers.

### D4. `MarginToCss::to_css_value` for `Margin::Offset` is admittedly half-finished

`horizontal_rule.rs:881-898`:

```rust
// This is a simplification - in a real implementation,
// we'd need to parse and combine the values properly
format!("{} + {}ch", base_value, chars)
```

An SVG `style="margin: 2% + 3ch auto 0 auto;"` will be treated as invalid by the browser. Either:
- Use `calc(2% + 3ch)` wrapping so the CSS is actually legal.
- Flag `Margin::Offset` as unsupported in browser output and log a `tracing::warn!`.

No test exercises `Margin::Offset`, so this has not surfaced as a bug — but it will the moment someone sets an offset margin on an HR inside an HTML-rendered document.

### D5. `render_image_tier` fallback behavior on `cell_size()` missing

`horizontal_rule.rs:281-282`: if `term.cell_size()` returns `None`, the code hard-codes 8×16 pixels. Fine as a default, but large terminals with small fonts will get an HR image scaled against the wrong cell dimensions. A brief rustdoc noting the assumption (or deriving the default from `term.width()` and a typical character aspect ratio) would be defensive.

### D6. `apply_terminal_color` retains the raw unrecognized color for the SVG but discards it for the terminal

A user writing `--- { color: "tomato" }` gets an untyped `tomato` passed through to the browser SVG (correct — browsers understand `tomato`) but dropped for the terminal because `parse_basic_color("tomato") == None`. No signal to the author either way. Consider extending `parse_basic_color` to cover the CSS named-color set (≈ 147 names), or at minimum logging the dropped terminal-side color at `info!` alongside the existing `warn!` so users can correlate the missing ANSI wrap with the color string.

### D7. Hidden allocation in color path

`apply_terminal_color` does `.to_ascii_lowercase()` which allocates a fresh `String` for every color comparison. For a rule rendered on every paragraph break in a large document this is measurable. Replace `match raw.to_ascii_lowercase().as_str()` with `.eq_ignore_ascii_case("black")` chained comparisons, or intern the match via `phf`/a static `HashMap`.

---

## Summary

| # | Item | Severity | Recommended action |
|---|------|----------|--------------------|
| A1 | `test_snapshot_complex_document` red (stale `.snap`) | blocker | `cargo insta review` |
| A2 | Component doc says Tier 1 is deferred (it isn't) | blocker | Rewrite the "Deferred" section |
| B1 | Invalid widths silently fall back without warning | should fix | Add `tracing::warn!` + test |
| B2 | `"200px"` documented but unsupported in terminal | should fix | Support `px` or remove from docs |
| B3 | Unquoted numeric frontmatter nukes all hr defaults | should fix | Custom deserializer, coerce numbers to strings |
| B4 | Hidden 10-char minimum width | should fix | Document or remove |
| B5 | `ImageSupport::ITerm` doesn't trigger Tier 1 | should fix | Broaden the guard |
| C1–C5 | Test coverage gaps | should fix | One test per item |
| D1–D7 | Ergonomic / perf polish | optional | |

**Re-run the snapshot suite after A1, then the full `-p biscuit-terminal` and `-p darkmatter` suites plus `cargo clippy -- -D warnings` (clippy is currently clean). When A1 and A2 are addressed, this is a READY feature.**
