---
plan_for: review-3.md
feature: horizontal-rule
packages:
  - biscuit-terminal
  - darkmatter
phases: 4
tdd: true
---

# Implementation Plan — Review 3 (Horizontal Rule)

This plan implements every recommendation in
[`review-3.md`](./review-3.md): the two Category A blockers, all five
Category B spec gaps, all five Category C test-coverage gaps, and all
seven Category D ergonomic / correctness polish items.

Target: READY per the review's acceptance criteria, with zero lint
warnings and green tests across both crates.

## Source context

Before any code change, the executing `rust-developer` subagent MUST
have these open:

- `/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-04-18-hr/spec.md`
- `/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-04-18-hr/tech-design.md`
- `/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/features/2026-04-18-hr/review-3.md`

Relevant skills (already in `.claude/skills/`): `biscuit-terminal`,
`darkmatter`, `rust`, `rust-testing`, `nextest`,
`superpowers:test-driven-development`, `superpowers:writing-plans`.

## Execution model

- **Subagent:** `rust-developer`, one phase at a time, sequentially.
- **TDD:** Per phase, write or adjust failing tests **before** the code
  change that makes them pass. Run the failing test to confirm RED,
  then implement, then confirm GREEN.
- **No commits from subagents.** The user commits after each phase
  review.
- **Targeted builds only.** Never `cargo build` / `cargo test` at
  repo root; always use `-p biscuit-terminal` and/or `-p darkmatter`
  (per `MEMORY.md`).
- **Single root `rustfmt.toml`.** Do not introduce per-package copies.
- **Non-interactive.** Never invoke `cargo insta review`. Snapshots are
  re-recorded via `INSTA_UPDATE=always cargo test …` and the resulting
  `.snap` diff is inspected in source control.
- **No `git reset`** on this worktree while uncommitted changes exist.

---

## Dependency graph

```
Phase 1 (A blockers: A1, A2)
       │
       ▼
Phase 2 (B1-B5 + C1, C2, C3)   ── item-level parallelism within phase
       │
       ▼
Phase 3 (C4, C5)
       │
       ▼
Phase 4 (D1-D7)                ── item-level parallelism within phase
       │
       ▼
Final Verification
```

Phases are strictly sequential because:

- Phase 2 tests (especially C3 rasterization-failure and B5 iTerm) will
  fail loudly if Phase 1's snapshot stability work hasn't landed.
- Phase 4's public-API changes (D1, D3) should only land after all
  behavioral changes are green, so breakage is easy to attribute.

Inside each phase, the listed items are independent and can be
implemented in any order, though a single subagent session should do
them in the order written for context locality.

---

## Phase 1 — Category A blockers

**Goal:** Restore a green snapshot suite and remove the stale
"Deferred" section from the public component doc so `-p darkmatter`
and `-p biscuit-terminal` tests pass and the public docs no longer
contradict the code.

### 1.1 Fix `test_snapshot_complex_document` (A1)

**Files:**

- `darkmatter/lib/tests/snapshots/horizontal_rule_snapshots__tests__terminal_complex_document.snap`
- `darkmatter/lib/tests/snapshots/horizontal_rule_snapshots__tests__html_complex_document.snap`

**Tasks:**

1. Confirm the test currently fails with the expected drift:

   ```bash
   cargo test -p darkmatter --test horizontal_rule_snapshots -- \
     test_snapshot_complex_document
   ```

   Expect a failure referencing `Placement Options` vs
   `Alignment Options` (review-3.md lines 29-37).

2. Re-record snapshots **non-interactively**:

   ```bash
   INSTA_UPDATE=always cargo test -p darkmatter \
     --test horizontal_rule_snapshots -- test_snapshot_complex_document
   ```

   (Do **not** use `cargo insta review` — session has no interactive
   TTY.)

3. Inspect the resulting diff manually:

   ```bash
   git diff -- darkmatter/lib/tests/snapshots/
   ```

   Verify:

   - Only the terminal and html `_complex_document.snap` files were
     touched.
   - The only semantic change is `Placement` → `Alignment` in the
     heading around line 15 of the terminal snap, and the
     corresponding `<h2>` text in the html snap.
   - No other content under "## Alignment Options" shifted (the body
     after the heading should be byte-identical).

4. If any other snapshot was rewritten, STOP and investigate — that
   indicates unintended drift outside the review's scope.

**New tests:** None. This is a snapshot refresh only.

### 1.2 Replace the "Deferred" section in the component doc (A2)

**File:** `biscuit-terminal/docs/components/horizontal-rule.md`

**Current state:** Lines 173-175 read "Tier 1 ... is not yet
implemented." This contradicts `horizontal_rule.rs:268-316` and the
passing test `test_render_uses_kitty_image_tier_when_supported`.

**Tasks:**

1. Delete the existing `## Deferred` section.

2. Add a new `## Tier 1 Image Rendering` section near the existing
   "Renderable (Terminal Rendering)" section (same level, appended after
   it) that explains:

   - The component ships Tier 1 rendering via
     `HorizontalRule::render_image_tier`.
   - Tier 1 activates only when **both** `term.is_tty == true` **and**
     `term.image_support == ImageSupport::Kitty` (after Phase 2.5 this
     will also include `ImageSupport::ITerm`).
   - The SVG is rasterized to PNG via `resvg` and emitted through
     `TerminalImage::render_kitty_cells` as a Kitty graphics escape
     sequence.
   - Any rasterization failure logs `tracing::warn!` and falls back to
     Tier 2 / Tier 3.
   - A missing `term.cell_size()` falls back to an 8×16 pixel default
     (see Phase 4 D5 for rustdoc on this assumption).

3. Reconcile the "Renderable (Terminal Rendering)" bullet list
   (currently already marking Tier 1 as implemented, lines 71-75).
   After the rewrite, every mention of Tier 1 in this file must
   consistently say "implemented" — no residual "deferred" or "not yet"
   language.

4. Grep the repo for other stale "not yet implemented" / "deferred"
   language referencing HR Tier 1:

   ```bash
   rg -n 'Tier 1.*(deferred|not.*yet.*implemented)' \
     biscuit-terminal darkmatter
   ```

   Remove or rewrite any surviving hits outside code comments that
   describe intentional fallthrough.

**New tests:** None (docs only).

### Phase 1 — Lint & Test Gate

Run in order; each must pass before moving on:

```bash
cargo test -p darkmatter
cargo test -p biscuit-terminal
cargo clippy -p darkmatter -p biscuit-terminal --all-targets -- -D warnings
```

All darkmatter warnings — even ones unrelated to this phase — must be
fixed before the gate passes. (No `#[allow]` sprinkles; fix the code.)

If `cargo nextest` is available (`cargo nextest --version` succeeds),
also run:

```bash
cargo nextest run -p darkmatter
cargo nextest run -p biscuit-terminal
```

At the end of this phase the review's A-category blockers are
resolved, meaning the feature is already at the review's minimum-READY
bar. Phases 2-4 harden it.

---

## Phase 2 — Category B spec gaps (+ coupled C tests)

**Goal:** Close every spec-vs-implementation gap called out in
review-3.md Category B, with the C-category tests that accompany them
(C1, C2, C3). Each item below is independent inside this phase.

### 2.1 B1 / C1 — Invalid-width warning path

**Files:**

- `biscuit-terminal/lib/src/components/horizontal_rule.rs`
- `biscuit-terminal/lib/Cargo.toml` (add `tracing-test = "0.2"` to
  `[dev-dependencies]`)

**Implementation (`resolve_width` around line 234-266):**

1. Restructure the function so every non-matching branch funnels into a
   single fallthrough that:

   - Emits `tracing::warn!(width = %width_str,
     "unrecognized horizontal rule width; falling back to terminal width")`.
   - Returns `term_width`.

2. Do **not** warn for the `None` (unset-width) path — that's the
   documented default and should stay silent.

3. Inline the ordering:

   - Try `%` suffix → parse as `f32`, return percent-of-term (existing
     behavior).
   - Try `ch` suffix → parse as `usize`, return chars (existing
     behavior).
   - Try bare `usize` → return chars (existing behavior).
   - Try `px` handling (see 2.2 below — B1 and B2 land together).
   - Otherwise: warn + return `term_width`.

**New tests (in `horizontal_rule.rs` inline `#[cfg(test)] mod tests`):**

- `test_resolve_width_invalid_warns_and_falls_back` — uses
  `#[tracing_test::traced_test]`; calls
  `HorizontalRule::new().width("garbage").resolve_width(100)`;
  asserts return value == `100` **and**
  `logs_contain("unrecognized horizontal rule width")`.
- `test_resolve_width_invalid_em_warns` — same pattern for `"10em"`.
- `test_resolve_width_invalid_vw_warns` — same pattern for `"50vw"`.
- `test_resolve_width_none_is_silent` — uses
  `#[tracing_test::traced_test]`; confirms no warning is emitted when
  width is `None`.

Add `use tracing_test::traced_test;` to the tests module.

### 2.2 B2 — `"NNNpx"` handling

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`

**Decision:** Support `"NNNpx"` (per the review's preferred option)
so the spec, skills, and docs remain accurate.

**Implementation:**

1. In `resolve_width`, after the `ch` branch and before the bare-number
   branch, add a `px` branch:

   ```rust
   if let Some(px_str) = trimmed.strip_suffix("px")
       && let Ok(pixels) = px_str.trim().parse::<u32>()
   {
       // Convert pixel width to terminal columns using the terminal's
       // cell width (falling back to 8 pixels/cell when cell_size is
       // unavailable — matches render_image_tier's default).
       let cell_width: u32 = 8; // constant default; callers wanting a
                                // different per-cell assumption should
                                // use "ch" or "%" widths.
       let cols = (pixels / cell_width).max(1) as usize;
       return cols.clamp(1, term_width);
   }
   ```

   Note: `resolve_width` currently has no access to `term.cell_size()`
   because it takes `term_width: usize`, not `&Terminal`. Keep it that
   way to avoid widening the signature; document the 8-px assumption in
   a `///` comment above the function.

2. Update the function-level `///` rustdoc to list all supported
   width forms: `NN%`, `NNch`, `NNpx`, bare `NN`, and explicitly
   mention that everything else emits a warning.

**New tests (inline):**

- `test_resolve_width_px_parses` — `width("160px").resolve_width(100)`
  returns `20` (160 / 8 = 20).
- `test_resolve_width_px_clamps_to_term_width` —
  `width("4000px").resolve_width(100)` returns `100`.
- `test_resolve_width_px_zero_is_one` —
  `width("0px").resolve_width(100)` returns `1` (the floor for any
  positive width).
- `test_resolve_width_px_malformed_warns` — `"abcpx"` emits the invalid
  warning and returns `term_width`.

### 2.3 B3 / C2 — Unquoted numeric frontmatter coercion

**Files:**

- `darkmatter/lib/src/markdown/inline/types.rs` (defines
  `HorizontalRuleAttrs`; check first — may live in `mod.rs`)
- `darkmatter/lib/src/markdown/block/hr_builder.rs`
  (`hr_defaults_from_frontmatter` at lines 93-106)

**Implementation — pick the simpler of the two options:**

Preferred path (no custom deserializer): in
`hr_defaults_from_frontmatter`, stop calling
`frontmatter.get::<HorizontalRuleAttrs>("hr")` directly. Instead:

1. Fetch the raw `serde_yaml_ng::Value` for the `hr` key.
2. If it's a mapping, iterate entries and, for each recognized key
   (`style`, `alignment`, `weight`, `width`, `color`), coerce the
   value through the existing
   `RuleProcessor::yaml_value_as_string` helper (or a sibling helper
   shared between the two sites — extract into
   `hr_builder::yaml_scalar_as_string` if needed).
3. Assemble a `HorizontalRuleAttrs` manually, populating `Some(String)`
   fields per entry. Unknown keys log `tracing::warn!` (matching the
   attribute-path warning text for consistency).
4. Non-mapping `hr` values (e.g., `hr: 42`, `hr: "foo"`) log a warn and
   return `None`.

If refactoring to share the helper is awkward, the fallback is a
custom `Deserialize` on `HorizontalRuleAttrs` that coerces numbers and
bools to strings — but prefer the shared-helper refactor because it
keeps both code paths literally identical.

**New tests (in `darkmatter/lib/tests/horizontal_rule_integration.rs`):**

- `test_hr_frontmatter_unquoted_numeric_width_survives` — frontmatter:

  ```yaml
  hr:
    style: dots
    width: 20
    color: red
  ```

  Render a bare `---` to terminal. Assert:

  - Output contains the dots pattern (not dashes, proving `style: dots`
    survived).
  - Rule width corresponds to 20 chars (proving the numeric width
    coerced).
  - Output contains a red ANSI escape (proving `color: red` survived).

- `test_hr_frontmatter_unquoted_bool_noop_survives` — frontmatter
  with `weight: 2.0` or similar non-string scalar; confirm siblings
  still apply. (Value itself may fall back to component default, but
  siblings must not be dropped.)

- `test_hr_frontmatter_non_mapping_warns` — frontmatter `hr: 42`; use
  `#[tracing_test::traced_test]` and confirm a warn is logged and
  rendering falls back to component defaults.

### 2.4 B4 — Remove or document the hidden 10-char minimum

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`

**Decision:** Remove the 10-char minimum. `resolve_width` already
enforces a minimum of 1 on the percentage path, and `width("1")`
should honor the author's explicit intent. Removing the clamp also
eliminates the undocumented behavior the review flagged.

**Tasks:**

1. Line 187: change

   ```rust
   let rule_width = rule_width.clamp(10, term_width);
   ```

   to

   ```rust
   let rule_width = rule_width.min(term_width).max(1);
   ```

2. Line 274 (inside `render_image_tier`): mirror the same change.

3. Update the public rustdoc on `HorizontalRule::width` to state
   explicitly: "The minimum effective width is 1 column; values
   smaller than 1 are coerced to 1. The maximum is the terminal's
   column count at render time."

**New tests (inline):**

- `test_render_width_one_percent_yields_one_column` — 100-col term,
  `width("1%")`, `alignment(Full)`, style dashes → the rendered
  string's `visible_width` should be 1. Also checks no stray padding.
- `test_render_width_three_yields_three_columns` — 100-col term,
  `width("3")`, alignment Left → 3-column rule, no hidden bump.
- `test_render_width_five_percent_yields_five_columns` — 100-col term,
  `width("5%")`, alignment Full → 5-column rule (previously clamped to
  10).

### 2.5 B5 — Extend Tier 1 to `ImageSupport::ITerm`

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(around line 269, inside `render_image_tier`)

**Tasks:**

1. Read `biscuit-terminal/lib/src/components/terminal_image.rs` to find
   whether a Kitty-vs-iTerm-specific emitter exists (there is a
   `render_kitty_cells`; look for `render_iterm_cells` or similar).

2. If both emitters exist, branch on `term.image_support`:

   ```rust
   let image = match term.image_support {
       ImageSupport::Kitty => TerminalImage::default()
           .render_kitty_cells(&png, rule_width as u32, height_cells),
       ImageSupport::ITerm => TerminalImage::default()
           .render_iterm_cells(&png, rule_width as u32, height_cells),
       _ => return None,
   };
   ```

3. If the iTerm emitter does not exist, update the guard to
   `matches!(term.image_support, ImageSupport::Kitty | ImageSupport::ITerm)`
   and route both through `render_kitty_cells` (iTerm2 accepts the
   Kitty graphics protocol in modern versions, per the biscuit-terminal
   skill notes in `MEMORY.md`). Add a rustdoc comment above
   `render_image_tier` noting the fallthrough behavior and why.

4. Update the early return guard at line 269 accordingly.

**New tests (inline):**

- `test_render_uses_image_tier_when_iterm` — mirror the existing
  `test_render_uses_kitty_image_tier_when_supported` but with
  `term.image_support = ImageSupport::ITerm`. Assert the output
  contains the image escape prefix (`\x1b[s` and either Kitty or
  iTerm2 sequences).
- `test_render_does_not_use_image_tier_when_unsupported` — set
  `image_support = ImageSupport::None`; confirm no image escape is
  emitted (text tier only).

### 2.6 C3 — Rasterization-failure fallback coverage

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(test block)

**Decision:** Cover the failure path without introducing a mocking
crate. Strategy: feed `rasterize_svg_to_png` a known-bad SVG by
introducing a tiny crate-private hook.

**Implementation:**

1. Make `rasterize_svg_to_png` (currently `fn rasterize_svg_to_png`)
   reachable from tests as `pub(crate)`. Already the case? Check and
   expose if needed.

2. Write a test that calls it with bytes that `usvg` rejects (e.g.,
   `b"<not-an-svg>"`). Assert it returns `Err`.

3. Write a test against the full `render_image_tier` path using a
   fabricated SVG input. Because the current implementation constructs
   the SVG internally, direct failure injection requires extracting the
   SVG body through a `pub(crate)` seam. The minimal, non-invasive
   approach:

   - Add `pub(crate) fn render_image_tier_from_svg(&self, term: &Terminal, svg: &str) -> Option<String>`
     that takes a pre-built SVG string, so tests can pass deliberately
     broken SVG and assert the `None` return.
   - Refactor `render_image_tier` to call `render_image_tier_from_svg`
     internally. Keep the public surface unchanged.

4. If the above refactor is considered too invasive, fall back to the
   simpler-but-narrower test that only asserts
   `rasterize_svg_to_png(b"\0\0not svg")` returns `Err`, and add a doc
   comment on `render_image_tier` explaining the fallthrough contract.

**New tests (inline):**

- `test_rasterize_svg_to_png_fails_on_garbage` — asserts `Err`.
- `test_render_image_tier_falls_through_on_rasterization_failure`
  (only if the refactor is done) — uses
  `#[tracing_test::traced_test]`; confirms `None` is returned **and**
  the warn log message "horizontal rule image rendering failed" fires.

### Phase 2 — Lint & Test Gate

```bash
cargo test -p darkmatter
cargo test -p biscuit-terminal
cargo clippy -p darkmatter -p biscuit-terminal --all-targets -- -D warnings
```

If `cargo nextest` available:

```bash
cargo nextest run -p darkmatter
cargo nextest run -p biscuit-terminal
```

After Phase 2, every Category B item is fixed, documented, and
tested, and C1 / C2 / C3 are closed.

---

## Phase 3 — Remaining Category C test gaps

**Goal:** Close C4 and C5 so the suite fails fast on boundary
regressions and the test pin matches the spec's intent.

### 3.1 C4 — Explicit assertion for CurtainRod + Thick + Right

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(test block) **or** `biscuit-terminal/lib/src/components/horizontal_rule_test.rs`

**Tasks:**

1. Add a new test
   `test_render_curtain_rod_thick_right_has_brackets_and_heavy_line`
   that:

   - Builds a rule with
     `style(CurtainRod).weight(Thick).alignment(Right).width("40")`.
   - Renders against a 100-col Terminal with
     `color_depth = ColorDepth::TrueColor`, `image_support = None`,
     `is_tty = false` (so we exercise Tier 2, not Tier 1).
   - Strips ANSI escapes (use the existing helper pattern or
     `regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").replace_all`).
   - **Asserts** all of the following on the stripped string:

     - Starts with exactly 60 leading spaces (right-aligned on 100
       cols, content 40 cols).
     - Contains `┤` as the first non-space character (left bracket).
     - Contains `├` as the last character before EOL (right bracket).
     - Between the brackets, the body is composed entirely of `━`
       (heavy variant, proving `Thick` honored).
     - Total `visible_width` = 40.

2. Add a smaller symmetric test for the Unicode-off path,
   `test_render_curtain_rod_thick_right_ascii_fallback`, which forces
   ASCII (via `ScopedLcAll::new("C")`) and asserts `[` / `]` brackets
   and `-` body.

The intent matches review-3.md §C4: the 36-tuple sweep's
`!result.is_empty()` assertion is weak; these two tests fail fast on
the specific hard combination.

### 3.2 C5 — Decide blockquote-HR behavior, then enforce

**Files:**

- `darkmatter/features/2026-04-18-hr/spec.md`
- `darkmatter/lib/src/markdown/block/rule_processor.rs` (test
  `test_horizontal_rule_inside_blockquote_is_currently_transformed` at
  lines 755-790)

**Decision:** Per spec's "Parsing Requirements" §, bare `---` inside a
blockquote is **still** a horizontal-rule block in CommonMark — the
blockquote context doesn't change that. Codify this in the spec rather
than carving the behavior out of the pin.

**Tasks:**

1. Amend `spec.md` §"Parsing Requirements" with a new bullet:

   > Bare `---`, `___`, or `***` inside a blockquote (`> ---`) remains
   > a horizontal-rule block. Page-level `hr` frontmatter defaults
   > apply inside blockquotes just as they do at the top level. The
   > attribute-block form (`> --- { style: waves }`) is also honored.

2. Add a paragraph in the "Per-rule Attributes" area clarifying that
   the HR remains wrapped by the surrounding blockquote start/end
   tags, not promoted to document level.

3. Rename the test
   `test_horizontal_rule_inside_blockquote_is_currently_transformed` →
   `test_horizontal_rule_inside_blockquote_is_transformed_per_spec`.
   Remove the "canary / force an update" language from the doc
   comment; replace with a pointer to the relevant spec paragraph.

4. Add an integration test in
   `darkmatter/lib/tests/horizontal_rule_integration.rs` named
   `test_blockquote_hr_renders_with_frontmatter_defaults` that:

   - Uses frontmatter `hr: { style: waves }`.
   - Body: `> ---\n`.
   - Asserts the rendered HTML contains the `<svg>` for waves
     **inside** the blockquote element.
   - Asserts the rendered terminal output contains the waves glyph
     at the appropriate column offset.

### Phase 3 — Lint & Test Gate

```bash
cargo test -p darkmatter
cargo test -p biscuit-terminal
cargo clippy -p darkmatter -p biscuit-terminal --all-targets -- -D warnings
```

If `cargo nextest` available, also `nextest run` both packages.

---

## Phase 4 — Category D ergonomic / correctness polish

**Goal:** Land every D-item from the review. Items inside this phase
are independent; the subagent should tackle them in the order below to
keep diffs easy to review.

### 4.1 D1 — Derive `PartialEq` on `HorizontalRule`

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`

**Risk audit (pre-change):**

```bash
rg -n 'HorizontalRule\b' \
   biscuit-terminal darkmatter --type rust
```

Scan every hit for callers that construct a `HorizontalRule` in a
position where adding `PartialEq` could matter (e.g., a `match` on
equality, storage in a `BTreeSet`, etc.). `PartialEq` derivation is
strictly additive, so the only realistic breakage is a dependent
crate intentionally implementing `PartialEq` manually elsewhere —
which the grep will surface.

**Tasks:**

1. Verify `Layout`, `Margin`, `RuleStyle`, `RuleAlignment`, `RuleWeight`
   all already derive `PartialEq` (review-3.md §D1 asserts this —
   confirm by opening `utils/layout.rs`).

2. Add `PartialEq` to the derive list on `HorizontalRule` (line 68).

3. Add `#[derive(Eq)]` if every field is `Eq`; skip if `f32`/`f64`
   lurks anywhere in the layout chain. (Check `Margin` — if it has a
   float, do `PartialEq` only.)

**New tests (inline):**

- `test_horizontal_rule_partial_eq_identity` — two rules built with
  the same builder calls compare equal.
- `test_horizontal_rule_partial_eq_differs_on_style` — differ only in
  style; confirm `!=`.

### 4.2 D2 — `use_fancy_chars` terminal capability hookup

**Files:**

- `biscuit-terminal/lib/src/terminal.rs` (or wherever `Terminal`
  capability flags live — check alongside `image_support`,
  `color_depth`, `osc_link_support`, `supports_italic`).
- `biscuit-terminal/lib/src/components/horizontal_rule.rs` (line
  605-607).

**Decision:** Add a `supports_unicode: bool` capability to `Terminal`
(mirroring existing boolean capabilities) and consult it first. Keep
`env_says_utf8` as the fallback for the `None`/unknown case so the
existing tests still pass.

**Tasks:**

1. Add `pub supports_unicode: bool` to `Terminal`; default to the
   result of `env_says_utf8().unwrap_or(true)` in `Terminal::default`
   and detection paths.

2. Update `use_fancy_chars` to:

   ```rust
   fn use_fancy_chars(&self, term: &Terminal) -> bool {
       term.supports_unicode
   }
   ```

   (The parameter is now used — drop the `_term` prefix.)

3. Update every existing test that builds a `Terminal` manually and
   relies on Unicode output to set `supports_unicode: true` explicitly.

**New tests (inline):**

- `test_use_fancy_chars_honors_terminal_capability` — build a
  `Terminal` with `supports_unicode: false` and assert the rule
  renders ASCII even when `env_says_utf8()` would otherwise return
  `true`. Use the `ScopedLcAll` pattern with `"en_US.UTF-8"` to
  ensure the env is explicitly UTF-8.

### 4.3 D3 — Flatten `HtmlOptions.hr_css_variables`

**Files:**

- `darkmatter/lib/src/markdown/output/html.rs` (field at line 89,
  default at line 101, consumers at lines 170 and 185)
- `darkmatter/lib/tests/horizontal_rule_integration.rs` (consumers
  around lines 452-485)

**Risk audit (pre-change):**

```bash
rg -n 'hr_css_variables' \
   darkmatter biscuit-terminal --type rust
```

Every consumer must be updated in this change. Expect roughly 4-6
hits.

**Tasks:**

1. Change field type:

   ```rust
   pub hr_css_variables: HashMap<String, String>,
   ```

2. Update `Default` impl to use `HashMap::new()` (or drop the explicit
   line since `HashMap` already has a `Default`).

3. Update consumers at html.rs:170 and html.rs:185. Current pattern:

   ```rust
   options.hr_css_variables.as_ref()
   ```

   becomes:

   ```rust
   &options.hr_css_variables
   ```

   And inside the consumer, the `!map.is_empty()` check already maps
   cleanly — empty `HashMap` means "no overrides" by construction.

4. Update the integration tests: replace
   `options.hr_css_variables = Some(vars);` with
   `options.hr_css_variables = vars;`.

5. Sweep docs: `biscuit-terminal/docs/components/horizontal-rule.md`,
   `darkmatter/docs/topics/horizontal-rules.md`, and
   `.claude/skills/darkmatter/SKILL.md` for any example using
   `Some(HashMap::...)` and update to bare `HashMap`.

**New tests:** The existing integration tests (lines 452-485) already
cover the behavior; they just need the type change. No new tests
beyond that unless a test previously asserted `is_some` / `is_none`
semantics — in which case, rewrite to `is_empty`.

### 4.4 D4 — `Margin::Offset` CSS correctness

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(lines 881-898)

**Decision:** Wrap in `calc(...)` so the SVG stays browser-valid.

**Tasks:**

1. Replace the `format!("{} + {}ch", base_value, chars)` arm with
   `format!("calc({} + {}ch)", base_value, chars)`.

2. Handle the `base_value == "0"` case either by:

   - Keeping the existing `format!("{}ch", chars)` fast path, OR
   - Collapsing to `calc(0 + {}ch)` for consistency — prefer the fast
     path (cleaner CSS output).

3. Handle the `chars == 0` case — return `base_value` unchanged
   (avoids pointless `calc(5% + 0ch)`).

**New tests (inline, inside `MarginToCss` test block — add one if
missing):**

- `test_margin_offset_percent_emits_calc` —
  `Margin::Offset(Box::new(Margin::Percent(2.0)), 3).to_css_value("0")`
  returns `"calc(2% + 3ch)"`.
- `test_margin_offset_zero_chars_strips_calc` —
  `Margin::Offset(Box::new(Margin::Percent(2.0)), 0)` returns `"2%"`.
- `test_margin_offset_zero_base_returns_ch` —
  `Margin::Offset(Box::new(Margin::None), 3)` returns `"3ch"`.

### 4.5 D5 — Document `cell_size()` default assumption

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(around line 281-282)

**Tasks:**

1. Add a `///` comment block above `render_image_tier` (or inline
   where the defaults are read) documenting that:

   - When `term.cell_size()` is `None`, the image tier assumes 8×16
     pixels per cell.
   - This matches typical monospace fonts at common DPI; oversized
     fonts will produce a proportionally thicker-looking rule.
   - Callers needing exact sizing should detect and set
     `cell_size` explicitly (e.g., via xterm CSI 16t query).

2. Extract the `8` and `16` constants into `const DEFAULT_CELL_WIDTH: u32 = 8;`
   and `const DEFAULT_CELL_HEIGHT: u32 = 16;` at module scope so the
   doc and code share a single source of truth.

3. Use the same `DEFAULT_CELL_WIDTH` constant in `resolve_width`'s new
   `px` branch (from 2.2) so both paths agree.

**New tests:** None — this is documentation + constant extraction. The
existing `render_image_tier` tests continue to exercise the path.

### 4.6 D6 — Log dropped terminal-side colors at `info!`

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(in `apply_terminal_color`, around line 548-552)

**Tasks:**

1. The current `tracing::warn!` when both `parse_hex_color` and
   `parse_basic_color` return `None` is correct — but the review
   points out that SVG still gets the raw string. The log text should
   make this asymmetry explicit so authors reading logs know why
   they're seeing "no color" in the terminal but a colored rule in
   the browser.

2. Update the existing `warn!` to:

   ```rust
   tracing::warn!(
       color = %raw,
       "unknown horizontal rule color string; browser rendering will \
        pass the value through to the SVG, but the terminal output is \
        uncolored. Recognized: CSS basic-16 names, `gray`/`grey`, and \
        `#rrggbb` hex."
   );
   ```

3. Consider (optional, stretch): extending `parse_basic_color` with a
   small handful of popular CSS named colors (`tomato`, `salmon`,
   `orange`, `violet`, `pink`). If done, map each to the nearest
   `BasicColor` and add a test per new name. If skipped, document in
   the rustdoc why only basic-16 is supported.

**New tests (inline):**

- `test_apply_terminal_color_unknown_name_warns_with_asymmetry_note`
  — uses `#[tracing_test::traced_test]`; calls
  `HorizontalRule::new().color("tomato")` rendered against a
  `ColorDepth::TrueColor` terminal; asserts the warn log contains the
  new "browser rendering will pass" phrase.

### 4.7 D7 — Remove per-call `to_ascii_lowercase` allocation

**File:** `biscuit-terminal/lib/src/components/horizontal_rule.rs`
(`parse_basic_color`, around line 646-666)

**Tasks:**

1. Replace the `match raw.to_ascii_lowercase().as_str() { ... }`
   structure with a chain of `eq_ignore_ascii_case` guards inside the
   function body, OR a lazy-initialized `phf_map!` if D6's scope
   expansion happens.

   Preferred form (no new dep):

   ```rust
   fn parse_basic_color(raw: &str) -> Option<BasicColor> {
       const PAIRS: &[(&str, BasicColor)] = &[
           ("black", BasicColor::Black),
           ("red", BasicColor::Red),
           // ... full table ...
           ("gray", BasicColor::BrightBlack),
           ("grey", BasicColor::BrightBlack),
           ("bright-black", BasicColor::BrightBlack),
           ("brightblack", BasicColor::BrightBlack),
           // ...
       ];
       PAIRS
           .iter()
           .find(|(name, _)| raw.eq_ignore_ascii_case(name))
           .map(|(_, color)| color.clone())
       }
   ```

2. Confirm with `cargo bench -p biscuit-terminal --no-run` that the
   bench harness still builds. If there's a rendering benchmark that
   already runs HR rendering, optionally note the before/after
   timings — but don't block the phase on micro-bench wins.

3. Ensure every prior lowercase match continues to match, especially
   the hyphenated forms (`bright-red`) and the concatenated forms
   (`brightred`). The existing tests already cover these; re-run
   them after the refactor.

**New tests:** None if existing coverage is complete. If any color
test isn't already present for (say) `"BLACK"` (all uppercase),
add a `test_parse_basic_color_case_insensitive` unit test.

### Phase 4 — Lint & Test Gate

```bash
cargo test -p darkmatter
cargo test -p biscuit-terminal
cargo clippy -p darkmatter -p biscuit-terminal --all-targets -- -D warnings
```

If `cargo nextest` available:

```bash
cargo nextest run -p darkmatter
cargo nextest run -p biscuit-terminal
```

All darkmatter warnings — even ones unrelated to this phase — must be
zero before the gate passes.

---

## Final Verification

Before declaring the feature READY, the subagent runs this full
checklist. Every box must pass:

### Test suites

- [ ] `cargo test -p darkmatter` — 0 failures.
- [ ] `cargo test -p biscuit-terminal` — 0 failures.
- [ ] `cargo test -p darkmatter --test horizontal_rule_snapshots` —
      passes without re-recording snapshots
      (i.e., `INSTA_UPDATE` is **not** set and the test passes).
- [ ] `cargo test -p darkmatter --test horizontal_rule_integration` —
      passes; contains every new integration test from Phases 2.3 and
      3.2.
- [ ] If nextest available: `cargo nextest run -p darkmatter` and
      `cargo nextest run -p biscuit-terminal` both pass.

### Lint

- [ ] `cargo clippy -p darkmatter -p biscuit-terminal --all-targets -- -D warnings`
      returns 0 (zero warnings).
- [ ] `cargo fmt --check` at repo root — uses single root
      `rustfmt.toml`.

### Doctests

- [ ] `cargo test -p biscuit-terminal --doc` — every rustdoc example
      compiles and passes (D5's updated rustdoc, D3's example
      updates, Tier 1 section from Phase 1.2).
- [ ] `cargo test -p darkmatter --doc` — same.

### Review-3 acceptance criteria (by item)

- [ ] **A1** — `test_snapshot_complex_document` passes; diff in
      `*_complex_document.snap` is limited to `Placement` → `Alignment`.
- [ ] **A2** — `biscuit-terminal/docs/components/horizontal-rule.md`
      has no "Deferred" section; a new "Tier 1 Image Rendering"
      section describes the Kitty + iTerm capability gate, SVG-to-PNG
      rasterization, and fallback contract.
- [ ] **B1** — `resolve_width` warns on invalid input; covered by
      `test_resolve_width_invalid_warns_and_falls_back`.
- [ ] **B2** — `"NNNpx"` widths resolve to a column count; covered by
      `test_resolve_width_px_*` tests.
- [ ] **B3** — Unquoted numeric frontmatter siblings survive; covered
      by `test_hr_frontmatter_unquoted_numeric_width_survives`.
- [ ] **B4** — Hidden 10-char minimum removed; covered by
      `test_render_width_*` boundary tests; rustdoc on `width()`
      updated.
- [ ] **B5** — `ImageSupport::ITerm` triggers Tier 1; covered by
      `test_render_uses_image_tier_when_iterm`.
- [ ] **C1** — invalid-width test exists (tied to B1).
- [ ] **C2** — unquoted-numeric-frontmatter test exists (tied to B3).
- [ ] **C3** — rasterization-failure test exists (covers `None`
      return; warn log asserted when refactor-to-seam was chosen).
- [ ] **C4** — CurtainRod + Thick + Right asserted explicitly by
      `test_render_curtain_rod_thick_right_has_brackets_and_heavy_line`.
- [ ] **C5** — spec updated; test renamed; new
      `test_blockquote_hr_renders_with_frontmatter_defaults` passes.
- [ ] **D1** — `HorizontalRule: PartialEq` derives land; `assert_eq!`
      compiles with two rules.
- [ ] **D2** — `Terminal::supports_unicode` capability consulted by
      `use_fancy_chars`; `_term` underscore prefix removed.
- [ ] **D3** — `HtmlOptions::hr_css_variables` is bare `HashMap`; all
      consumers and docs updated.
- [ ] **D4** — `Margin::Offset` emits valid `calc(...)` CSS; covered
      by `test_margin_offset_percent_emits_calc`.
- [ ] **D5** — `DEFAULT_CELL_WIDTH` / `_HEIGHT` constants added and
      documented.
- [ ] **D6** — `apply_terminal_color` warn message explains the
      browser-vs-terminal asymmetry; covered by test.
- [ ] **D7** — `parse_basic_color` no longer allocates a lowercase
      `String` per call; benchmarks build.

### Docs consistency sweep

Final grep, all must return zero hits:

```bash
rg -n 'Tier 1.*(deferred|not.*yet.*implemented)' \
   biscuit-terminal darkmatter

rg -n 'Placement Options' darkmatter/lib/tests

rg -n 'hr_css_variables.*Option' darkmatter biscuit-terminal
```

### Skill / memory drift

- [ ] `.claude/skills/biscuit-terminal/SKILL.md` — any HR examples
      still valid? If D2 added a `supports_unicode` field, examples
      constructing `Terminal` manually may need updating.
- [ ] `.claude/skills/darkmatter/SKILL.md` — if D3 changed the
      `hr_css_variables` shape, update any example snippet.
- [ ] `biscuit-terminal/docs/components/horizontal-rule.md` internally
      consistent (Phase 1.2 + Phase 4.3).
- [ ] `darkmatter/docs/topics/horizontal-rules.md` still reads true.

When every box is checked, the feature is READY per review-3.md's
acceptance bar. Commit — but note the `MEMORY.md` constraint:
**subagents do not commit**. Commits are the user's responsibility,
one per phase.

---

## Notes for the executing subagent

- If any phase's Lint & Test Gate fails in a way that looks like
  pre-existing drift unrelated to the current phase's work, STOP and
  report to the user rather than silently expanding the phase's
  scope. The plan's promise is "zero warnings after the phase lands";
  if drift predates the phase, the user decides whether to fix it
  here or defer.
- Do **not** add `#[allow(...)]` to silence clippy warnings unless the
  justification is written in a comment directly above the attribute
  and the user has approved it.
- Do **not** introduce new dependencies beyond `tracing-test` (B1) and
  any optional D7 follow-up (which is explicitly skipped if it would
  require adding `phf`). Every other change uses what's already in
  `Cargo.toml`.
- `INSTA_UPDATE=always` is only used **once**, in Phase 1.1. Under no
  other circumstance does this plan regenerate snapshots — if a later
  phase's change causes a snapshot diff, treat it as a regression to
  investigate, not an update to accept.
