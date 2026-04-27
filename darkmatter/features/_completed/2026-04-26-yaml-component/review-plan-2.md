# Review-2 Implementation Plan: YAML Component

**Source review:** `darkmatter/features/2026-04-26-yaml-component/review-2.md`
**Spec:** `darkmatter/features/2026-04-26-yaml-component/spec.md`
**Tech-design:** `darkmatter/features/2026-04-26-yaml-component/tech-design.md`

## Position on Open Decisions

### §2.1 Header-row policy — **emit the header row**

`YamlBlock::render` will be updated to emit a header row identical to the one Markdown's ` ```yaml ` fence emits via `format_header_row`. Rationale:

- Acceptance Criterion 6 explicitly says "byte-identical." Dropping that promise would be a user-facing semantic regression for anyone embedding `YamlBlock` next to Markdown-rendered fences.
- The header row encodes the same theme/background as the body, so it composes correctly with `Layout::apply_layout`.
- Promoting `format_header_row` from private to `pub(crate)` is a single-line visibility change in `output/terminal.rs`; no design surgery.
- Keeps the shared-helper promise: the same header logic flows through both call sites.

### §2.4 Spec divergence on malformed-frontmatter error — **update the spec**

The implementation and tech-design (lines 99–103) intentionally surface `MarkdownError::FrontmatterParse` so callers retain rich Markdown diagnostics. Spec line 37 was written before that decision was made. Update the spec to say `YamlBlockError::MarkdownParse`. No code change.

### §2.2 `serial_test` availability — **already a dev-dep**

Verified: `darkmatter/lib/Cargo.toml` line 78 already declares `serial_test = "3.0"`. Also present in `homelab/server`, `claudine/cli`, and `claudine/lib`. No `Cargo.toml` change needed.

---

## Phase 1 — Core Bug Fixes (§1.1, §1.2, §1.3, §1.4, §2.1, §2.3)

**Goal:** Make `Renderable::render` honour layout, route theme through `TerminalOptions`, emit the missing header row, and unify syntax lookup between terminal and HTML helpers.

**Files touched:**

- `darkmatter/lib/src/markdown/output/terminal.rs` — promote `format_header_row` to `pub(crate)`; expose `header_text_color` if needed by `YamlBlock`'s call.
- `darkmatter/lib/src/markdown/output/code_block.rs` — change `render_html_code_block` to use `find_syntax(language, ...)` instead of `find_syntax_by_token`. (§2.3)
- `darkmatter/lib/src/markdown/yaml_block.rs` — rewrite `Renderable::render` and `BrowserRenderable::render_to_browser` per fixes below.

### Specific code changes

**1. `output/terminal.rs`:** change `fn format_header_row(...)` to `pub(crate) fn format_header_row(...)`. Add doc-comment that this is now reused by `YamlBlock` so future signature changes need a parity check. No behaviour change.

**2. `output/code_block.rs::render_html_code_block`:** replace lines 181–188 with a single call to `find_syntax(language, highlighter.syntax_set())`, falling back to `find_syntax_plain_text()`. Mirrors the terminal helper exactly. (§2.3)

**3. `yaml_block.rs::Renderable::render`:** rewrite to:

```rust
fn render(&self, term: &Terminal) -> String {
    use crate::markdown::output::terminal::format_header_row;

    let options = TerminalOptions::default();
    let color_mode = options.color_mode; // §1.3 — reuse, don't re-detect
    let highlighter = CodeHighlighter::new(options.code_theme, color_mode); // §1.2
    let meta = CodeBlockMeta::default();

    // §2.1 — emit the same header row Markdown YAML fences emit
    let bg_color = highlighter
        .theme()
        .settings
        .background
        .unwrap_or(syntect::highlighting::Color::BLACK);
    let header = format_header_row(
        meta.title.as_deref(),
        "yaml",
        bg_color,
        color_mode,
        term.width(),
    );

    let body = render_terminal_code_block(
        self.yaml(), "yaml", &highlighter, &options, &meta, color_mode,
    )
    .unwrap_or_else(|_| format!("\n{}\n", self.yaml()));

    let raw = format!("{header}\n{body}");

    // §1.1 — apply stored layout
    self.layout.apply_layout(&raw, term.width())
}
```

**4. `yaml_block.rs::BrowserRenderable::render_to_browser`:** route through `HtmlOptions::default()` for symmetry (§1.2 epilogue) and remove the redundant `detect_color_mode()` call (§1.3 browser path):

```rust
fn render_to_browser(&self) -> String {
    let options = HtmlOptions::default();
    // HtmlOptions hardcodes Github today; pull a color_mode locally for the
    // highlighter constructor. This keeps the browser path symmetric with the
    // terminal path so a future HtmlOptions change auto-propagates.
    let color_mode = detect_color_mode();
    let highlighter = CodeHighlighter::new(ThemePair::Github, color_mode);
    let meta = CodeBlockMeta::default();
    render_html_code_block(self.yaml(), "yaml", &meta, &highlighter, &options)
        .unwrap_or_else(|_| {
            format!(
                "<pre><code class=\"language-yaml\">{}</code></pre>",
                html_escape::encode_text(self.yaml())
            )
        })
}
```

**5. (§1.4) `render_optimistic`:** add a one-line override that delegates to `render` with a default `Terminal`. Documents intent without changing behaviour after §1.1 is fixed:

```rust
fn render_optimistic(&self) -> String {
    self.render(&Terminal::default())
}
```

### Verification

```bash
cargo test -p darkmatter
cargo clippy -p darkmatter -- -D warnings
```

All existing `yaml_block.rs` tests must still pass. Phase 2 will harden them.

---

## Phase 2 — Test Hardening (§2.2, §3.1, §3.2, §3.3, §3.4, §3.5)

**Goal:** Replace the lax parity tests with byte-level body comparisons, add real env-driven light/dark tests using `serial_test`, and cover layout, width overrides, file/string round-trip, and key-order preservation.

**Files touched:**

- `darkmatter/lib/src/markdown/yaml_block.rs` — replace and extend the `tests` module.

### Specific code changes

**§3.1 — Tighten parity tests.** Replace `test_terminal_render_parity_with_markdown_yaml_fence` and `test_browser_render_parity_with_markdown_yaml_fence` with strict body-comparison versions:

- **Terminal:** render `YamlBlock::new(yaml)` and the equivalent ` ```yaml ` Markdown fence, strip ANSI, then assert that the YamlBlock output is a contiguous substring of the Markdown output (the Markdown wrapper still adds its own outer paragraph spacing). Also assert both outputs contain the same header substring (`" yaml "`) post-Phase-1.
- **Browser:** assert the YamlBlock HTML contains a `<pre><code class="language-yaml">…</code></pre>` substring that also appears verbatim inside the Markdown `as_html` output. Diff the two on the inner code text only (strip wrapper `<div class="code-block">` from the Markdown side).

**§2.2 — Real light/dark tests.** Delete the two tests at lines 406–431 and replace with:

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_dark_mode_via_colorfgbg() {
    let prev = std::env::var("COLORFGBG").ok();
    let prev_no_color = std::env::var("NO_COLOR").ok();
    std::env::remove_var("NO_COLOR");
    unsafe { std::env::set_var("COLORFGBG", "15;0"); }
    assert_eq!(detect_color_mode(), ColorMode::Dark);

    let block = YamlBlock::new("key: value").unwrap();
    let dark_out = Renderable::render(&block, &Terminal::default());
    assert!(dark_out.contains("\x1b["));

    // Restore
    match prev { Some(v) => unsafe { std::env::set_var("COLORFGBG", v) }, None => unsafe { std::env::remove_var("COLORFGBG") } }
    if let Some(v) = prev_no_color { unsafe { std::env::set_var("NO_COLOR", v) }; }
}

#[test]
#[serial]
fn test_light_mode_via_colorfgbg() {
    // Mirror, with COLORFGBG="0;15", asserting ColorMode::Light and ANSI presence.
}

#[test]
#[serial]
fn test_dark_and_light_render_differ() {
    // Render once with COLORFGBG="15;0", once with "0;15"; assert outputs differ
    // (different bg color in `\x1b[48;2;…m` sequences).
}
```

Wrap `set_var`/`remove_var` in `unsafe` blocks since the project is on edition 2024 and `std::env::set_var` is `unsafe` there. Use `#[serial]` on every test that touches process env.

**§3.2 — Layout-application test.**

```rust
use biscuit_terminal::utils::layout::Margin;

#[test]
fn test_left_margin_is_applied() {
    let mut block = YamlBlock::new("foo: 1\nbar: 2").unwrap();
    block.layout_mut().set_left_margin(Margin::Chars(4));
    let out = Renderable::render(&block, &Terminal::default());
    let plain = crate::testing::strip_ansi_codes(&out);
    for line in plain.lines().filter(|l| !l.trim().is_empty()) {
        assert!(line.starts_with("    "), "expected 4-space margin, got: {line:?}");
    }
}
```

(Use whatever `Layout` mutation API exists — confirm the method name during implementation; `apply_layout` is the consumer side, so there is a setter on `Layout`.)

**§3.3 — `render_optimistic` smoke test.**

```rust
#[test]
fn test_render_optimistic_smoke() {
    let block = YamlBlock::new("foo: 1").unwrap();
    let out = Renderable::render_optimistic(&block);
    assert!(out.contains("\x1b["));
    assert!(crate::testing::strip_ansi_codes(&out).contains("foo: 1"));
}

#[test]
fn test_render_in_width_smoke() {
    // Construct a Terminal with a narrow width, render, assert output is non-empty
    // and ANSI-bearing. Documents the trait-default width handling.
}
```

**§3.4 — File vs string round-trip.**

```rust
#[test]
fn test_from_yaml_file_matches_new() {
    let yaml = "foo: 1\nbar: 2\n";
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(yaml.as_bytes()).unwrap();
    let from_file = YamlBlock::from_yaml_file(file.path()).unwrap();
    let from_string = YamlBlock::new(yaml).unwrap();
    assert_eq!(from_file.yaml(), from_string.yaml());
}
```

**§3.5 — Key-order preservation.**

```rust
#[test]
fn test_from_markdown_content_preserves_key_order() {
    let md = "---\nb: 1\na: 2\nc: 3\n---\n# body\n";
    let block = YamlBlock::from_markdown_content(md).unwrap();
    let yaml = block.yaml();
    let pos_b = yaml.find("b:").expect("b key present");
    let pos_a = yaml.find("a:").expect("a key present");
    let pos_c = yaml.find("c:").expect("c key present");
    assert!(pos_b < pos_a && pos_a < pos_c, "expected b < a < c in: {yaml}");
}
```

**§2.5 — Reserialization-loss documentation tests.** Add two tests that *document* (not regress on) the lossy behaviour described in review §2.5:

```rust
#[test]
fn test_from_markdown_content_drops_comments() {
    // Comments in the original frontmatter are dropped after IndexMap round-trip.
    let md = "---\n# leading comment\nfoo: 1\n---\nbody\n";
    let block = YamlBlock::from_markdown_content(md).unwrap();
    assert!(!block.yaml().contains("leading comment"));
    assert!(block.yaml().contains("foo: 1"));
}

#[test]
fn test_from_markdown_content_normalizes_whitespace() {
    // Original tab/extra-space formatting is replaced by canonical YAML output.
    let md = "---\nfoo:    1\n---\n";
    let block = YamlBlock::from_markdown_content(md).unwrap();
    assert!(block.yaml().contains("foo: 1")); // single-space canonical form
}
```

### Verification

```bash
cargo test -p darkmatter
cargo clippy -p darkmatter -- -D warnings
```

All new tests must pass. The two new env-driven `#[serial]` tests must run cleanly in isolation and in sequence.

---

## Phase 3 — Spec Reconciliation & Documentation Polish (§2.4, §2.5 doc, §5.1, §5.2, §5.3)

**Goal:** Bring the spec into agreement with implementation, document the reserialization caveat, and finish the rustdoc/README polish.

**Files touched:**

- `darkmatter/features/2026-04-26-yaml-component/spec.md`
- `darkmatter/lib/src/markdown/yaml_block.rs` (rustdoc only)
- `darkmatter/lib/README.md`

### Specific code changes

**§2.4 — Spec line 37.** Replace:

> Malformed frontmatter ⇒ `YamlBlockError::YamlParse` when the frontmatter block is present but is not valid YAML.

with:

> Malformed frontmatter ⇒ `YamlBlockError::MarkdownParse` (wrapping `MarkdownError::FrontmatterParse`). This preserves the rich diagnostics surfaced by `Markdown::try_from_content`. See tech-design §"Error Type" for the rationale.

Also revise Acceptance Criterion 1 (line 80) and any prose that asserts the `YamlParse` mapping for frontmatter. Add a note that `YamlParse` is still surfaced for `new`, `from_yaml_file`, and re-serialization validation failures.

**§5.1 — `## Errors` on `YamlBlock::new`.** Add the heading the rustdoc convention requires:

```rust
/// ## Errors
///
/// Returns [`YamlBlockError::YamlParse`] if the input fails `serde_yaml_ng` parsing.
```

**§5.2 — Trait impl docs.** Add a one-line doc above each trait impl:

```rust
/// `Renderable` impl: emits a code-fence-equivalent block with a header row and
/// applies the stored [`Layout`] (margins, alignment) to the result.
impl Renderable for YamlBlock { ... }

/// `BrowserRenderable` impl: emits `<pre><code class="language-yaml">…</code></pre>`
/// inside the standard darkmatter `<div class="code-block">` wrapper.
impl BrowserRenderable for YamlBlock { ... }
```

**§2.5 — Constructor doc on `from_markdown_content`.** Append to its rustdoc:

```rust
/// ## Notes
///
/// Frontmatter is round-tripped through Darkmatter's structured `FrontmatterMap`
/// before being re-serialized. As a result, comments, custom YAML tags, anchors,
/// and original whitespace are not preserved in the rendered output. Key order
/// is preserved (`IndexMap`). If you need byte-exact source preservation, call
/// [`YamlBlock::new`] with the raw frontmatter text instead.
```

**§5.3 — README example.** Add a rendering snippet to `darkmatter/lib/README.md` after the existing `YamlBlock` constructor examples (around lines 590–620):

```rust
use biscuit_terminal::components::renderable::{BrowserRenderable, Renderable};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::YamlBlock;

let block = YamlBlock::new("foo: 1\nbar: 2")?;
let term = Terminal::default();
print!("{}", Renderable::render(&block, &term));   // ANSI to a terminal
let html = BrowserRenderable::render_to_browser(&block); // <pre><code class="language-yaml">…
```

### Verification

```bash
cargo test -p darkmatter
cargo clippy -p darkmatter -- -D warnings
cargo doc -p darkmatter --no-deps   # surface any broken intra-doc links
```

---

## Phase 4 — Final Audit & Lint Pass

**Goal:** Confirm the entire `darkmatter` crate compiles clean, all tests pass, no clippy warnings, and review-2's summary table is fully discharged.

**Files touched:** none (audit-only). Any drift discovered here is patched in-place.

### Steps

1. Run the full suite:

   ```bash
   cargo test -p darkmatter
   cargo clippy -p darkmatter -- -D warnings
   cargo clippy -p darkmatter --tests -- -D warnings
   cargo fmt --check -p darkmatter
   ```

2. Manually walk review-2's summary table (lines 215–230) and confirm each row maps to a phase task above. Mark any that surface as still-open.

3. If `cargo doc -p darkmatter --no-deps` produced warnings during Phase 3, resolve them.

4. Update `darkmatter/features/2026-04-26-yaml-component/review-2.md` frontmatter to `ready: true` and append a closing note: "Addressed by review-plan-2.md, Phases 1–4."

### Verification

```bash
cargo test -p darkmatter
cargo clippy -p darkmatter -- -D warnings
cargo clippy -p darkmatter --tests -- -D warnings
cargo fmt --check -p darkmatter
```

All four commands must exit 0.

---

## Risk Register

| # | Risk | Mitigation |
|---|------|-----------|
| R1 | `format_header_row` has more parameters than expected (e.g. needs additional state from `terminal.rs`). | Confirmed signature: `(Option<&str>, &str, Color, ColorMode, u16)`. All inputs are derivable inside `YamlBlock::render`. |
| R2 | `Layout` setter API for `left_margin` differs from the assumed name. | During Phase 2 implementation, grep `biscuit-terminal/lib/src/utils/layout.rs` for `pub fn` to confirm the actual builder/setter — adjust the test accordingly. The behaviour test is what matters. |
| R3 | `serial_test`-gated env tests interact with other tests in the suite that read `COLORFGBG`. | All env-touching tests carry `#[serial]`; restore prior values in every test to avoid cross-test pollution. |
| R4 | Strict body-byte parity (Phase 2 §3.1) reveals additional drift between Markdown's wrapper and `YamlBlock`'s output (e.g. trailing newline counts). | Use "contiguous substring" form rather than full equality, so wrapper-only differences don't fail the test while structural body differences still do. |
| R5 | Spec edits in Phase 3 §2.4 cascade into AC1 wording. | Plan explicitly calls out updating AC1; review the full Acceptance Criteria list during the edit. |

---

## Phase Summary

| Phase | Scope | End-state verification |
|---|---|---|
| 1 | Core bug fixes: layout, theme routing, header row, syntax-lookup unification, `render_optimistic` | `cargo test -p darkmatter && cargo clippy -p darkmatter -- -D warnings` green |
| 2 | Test hardening: real env-driven light/dark, byte-level parity, layout, width, round-trip, key-order, reserialization-loss | Same plus all new tests passing |
| 3 | Spec reconciliation, rustdoc `## Errors`, trait-impl doc, README rendering snippet, `## Notes` on `from_markdown_content` | Same plus `cargo doc -p darkmatter --no-deps` clean |
| 4 | Audit + fmt + closing note on review-2 | All four cargo commands exit 0; review-2 marked `ready: true` |
